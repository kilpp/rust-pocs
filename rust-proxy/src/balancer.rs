use crate::backend::Backend;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub trait LoadBalancer: Send + Sync {
    fn select(&self, backends: &[Arc<Backend>]) -> Option<usize>;
}

pub struct RoundRobin {
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new() -> Self {
        RoundRobin {
            counter: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn select(&self, backends: &[Arc<Backend>]) -> Option<usize> {
        if backends.is_empty() {
            return None;
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % backends.len();
        Some(idx)
    }
}

pub struct LeastConnections;

impl LoadBalancer for LeastConnections {
    fn select(&self, backends: &[Arc<Backend>]) -> Option<usize> {
        backends
            .iter()
            .enumerate()
            .min_by_key(|(_, b)| b.active_connections.load(Ordering::Relaxed))
            .map(|(i, _)| i)
    }
}
