use hyper::Uri;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub struct Backend {
    pub uri: Uri,
    pub active_connections: AtomicUsize,
}

impl Backend {
    pub fn new(url: &str) -> anyhow::Result<Arc<Self>> {
        let uri: Uri = url
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid backend URL '{}': {}", url, e))?;
        Ok(Arc::new(Backend {
            uri,
            active_connections: AtomicUsize::new(0),
        }))
    }
}
