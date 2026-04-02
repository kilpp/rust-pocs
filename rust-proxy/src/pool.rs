use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::Uri;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::collections::HashMap;

pub type BoxedBody = BoxBody<Bytes, hyper::Error>;

/// One hyper-util client per backend origin. Each client maintains its own
/// internal connection pool for that upstream.
pub struct Pool {
    clients: HashMap<String, Client<hyper_util::client::legacy::connect::HttpConnector, BoxedBody>>,
}

impl Pool {
    pub fn new(uris: &[Uri]) -> Self {
        let mut clients = HashMap::new();
        for uri in uris {
            let origin = origin_key(uri);
            clients
                .entry(origin)
                .or_insert_with(|| Client::builder(TokioExecutor::new()).build_http());
        }
        Pool { clients }
    }

    pub fn get(
        &self,
        uri: &Uri,
    ) -> Option<&Client<hyper_util::client::legacy::connect::HttpConnector, BoxedBody>> {
        self.clients.get(&origin_key(uri))
    }
}

fn origin_key(uri: &Uri) -> String {
    format!(
        "{}://{}",
        uri.scheme_str().unwrap_or("http"),
        uri.authority().map(|a| a.as_str()).unwrap_or("")
    )
}
