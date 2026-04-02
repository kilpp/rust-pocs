use hyper::{Method, StatusCode, Uri};

pub struct RequestLog {
    pub method: Method,
    pub path: String,
    pub backend: Uri,
    pub status: StatusCode,
    pub duration_ms: u64,
}

pub fn log_request(entry: &RequestLog) {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    println!(
        "[{}] {} {} -> {} {} {}ms",
        now,
        entry.method,
        entry.path,
        entry.backend,
        entry.status,
        entry.duration_ms,
    );
}
