use crate::backend::Backend;
use crate::balancer::LoadBalancer;
use crate::logging::{log_request, RequestLog};
use crate::pool::{BoxedBody, Pool};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderName, HeaderValue};
use hyper::service::Service;
use hyper::{Request, Response, StatusCode, Uri};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

/// Hop-by-hop headers that must not be forwarded to the upstream.
static HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

pub struct ProxyState {
    pub backends: Vec<Arc<Backend>>,
    pub balancer: Box<dyn LoadBalancer>,
    pub pool: Pool,
}

#[derive(Clone)]
pub struct ProxyService {
    pub state: Arc<ProxyState>,
}

impl Service<Request<Incoming>> for ProxyService {
    type Response = Response<BoxedBody>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let state = self.state.clone();
        Box::pin(async move { forward(state, req).await })
    }
}

async fn forward(
    state: Arc<ProxyState>,
    req: Request<Incoming>,
) -> anyhow::Result<Response<BoxedBody>> {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "/".to_owned());

    let idx = match state.balancer.select(&state.backends) {
        Some(i) => i,
        None => return Ok(error_response(StatusCode::BAD_GATEWAY, "No backends available")),
    };

    let backend = &state.backends[idx];
    backend.active_connections.fetch_add(1, Ordering::Relaxed);
    let _guard = scopeguard::guard(backend.clone(), |b| {
        b.active_connections.fetch_sub(1, Ordering::Relaxed);
    });

    let upstream_uri = build_upstream_uri(&backend.uri, &path)?;

    let client = match state.pool.get(&backend.uri) {
        Some(c) => c,
        None => return Ok(error_response(StatusCode::BAD_GATEWAY, "No pool for backend")),
    };

    let mut upstream_req = Request::builder()
        .method(method.clone())
        .uri(upstream_uri);

    // Copy headers, filtering hop-by-hop
    for (name, value) in req.headers() {
        if !is_hop_by_hop(name) {
            upstream_req = upstream_req.header(name, value);
        }
    }

    // Ensure Host header points to the backend
    let host_value = backend
        .uri
        .authority()
        .map(|a| a.as_str().to_owned())
        .unwrap_or_default();
    if let Ok(hv) = HeaderValue::from_str(&host_value) {
        upstream_req = upstream_req.header(hyper::header::HOST, hv);
    }

    let body: BoxedBody = req.into_body().map_err(|e| e).boxed();
    let upstream_req = upstream_req
        .body(body)
        .map_err(|e| anyhow::anyhow!("Failed to build upstream request: {}", e))?;

    match client.request(upstream_req).await {
        Ok(resp) => {
            let status = resp.status();
            log_request(&RequestLog {
                method,
                path,
                backend: backend.uri.clone(),
                status,
                duration_ms: start.elapsed().as_millis() as u64,
            });

            // Convert response body to BoxedBody
            let (parts, body) = resp.into_parts();
            let boxed_body: BoxedBody = body.map_err(|e| e).boxed();
            Ok(Response::from_parts(parts, boxed_body))
        }
        Err(e) => {
            log_request(&RequestLog {
                method,
                path,
                backend: backend.uri.clone(),
                status: StatusCode::BAD_GATEWAY,
                duration_ms: start.elapsed().as_millis() as u64,
            });
            Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Upstream error: {}", e),
            ))
        }
    }
}

fn build_upstream_uri(backend: &Uri, path_and_query: &str) -> anyhow::Result<Uri> {
    let mut parts = backend.clone().into_parts();
    parts.path_and_query = Some(
        path_and_query
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?,
    );
    Uri::from_parts(parts).map_err(|e| anyhow::anyhow!("Failed to build URI: {}", e))
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    HOP_BY_HOP.iter().any(|h| *h == name.as_str())
}

fn error_response(status: StatusCode, msg: &str) -> Response<BoxedBody> {
    let body: BoxedBody = Full::new(Bytes::from(msg.to_owned()))
        .map_err(|never| match never {})
        .boxed();
    Response::builder()
        .status(status)
        .body(body)
        .expect("static error response is always valid")
}
