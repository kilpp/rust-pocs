use std::time::{Duration, Instant};

use chrono::Local;
use tokio::sync::mpsc;

pub struct EndpointStatus {
    pub url: String,
    pub status: Option<u16>,
    pub response_time_ms: Option<u128>,
    pub last_check: Option<String>,
    pub error: Option<String>,
    pub history: Vec<u64>,
    pub up_count: u64,
    pub total_count: u64,
}

impl EndpointStatus {
    pub fn new(url: String) -> Self {
        Self {
            url,
            status: None,
            response_time_ms: None,
            last_check: None,
            error: None,
            history: Vec::new(),
            up_count: 0,
            total_count: 0,
        }
    }

    pub fn uptime_pct(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.up_count as f64 / self.total_count as f64) * 100.0
        }
    }

    pub fn apply_result(&mut self, result: CheckResult) {
        self.status = result.status;
        self.response_time_ms = result.response_time_ms;
        self.error = result.error;
        self.last_check = Some(Local::now().format("%H:%M:%S").to_string());
        self.total_count += 1;
        if self.status.is_some_and(|s| (200..400).contains(&s)) {
            self.up_count += 1;
        }
        if let Some(ms) = result.response_time_ms {
            self.history.push(ms as u64);
            if self.history.len() > 60 {
                self.history.remove(0);
            }
        }
    }
}

pub struct CheckResult {
    pub index: usize,
    pub status: Option<u16>,
    pub response_time_ms: Option<u128>,
    pub error: Option<String>,
}

pub fn poll_endpoints(
    endpoints: &[EndpointStatus],
    client: &reqwest::Client,
    tx: &mpsc::UnboundedSender<CheckResult>,
) {
    for (i, ep) in endpoints.iter().enumerate() {
        let tx = tx.clone();
        let client = client.clone();
        let url = ep.url.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            match client.get(&url).send().await {
                Ok(resp) => {
                    let _ = tx.send(CheckResult {
                        index: i,
                        status: Some(resp.status().as_u16()),
                        response_time_ms: Some(start.elapsed().as_millis()),
                        error: None,
                    });
                }
                Err(e) => {
                    let _ = tx.send(CheckResult {
                        index: i,
                        status: None,
                        response_time_ms: Some(start.elapsed().as_millis()),
                        error: Some(e.to_string()),
                    });
                }
            }
        });
    }
}

pub fn build_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
}