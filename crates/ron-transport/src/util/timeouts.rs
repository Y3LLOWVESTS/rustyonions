//! RO:WHAT — Timeout helpers (read/write/idle).
use std::time::Duration;
use tokio::time::{timeout, Instant};

pub async fn with_timeout<F, T>(dur: Duration, f: F) -> Result<T, tokio::time::error::Elapsed>
where
    F: std::future::Future<Output = T>,
{
    timeout(dur, f).await
}

pub struct IdleGuard {
    last: Instant,
    idle: Duration,
}
impl IdleGuard {
    pub fn new(idle: Duration) -> Self {
        Self {
            last: Instant::now(),
            idle,
        }
    }
    pub fn bump(&mut self) {
        self.last = Instant::now();
    }
    pub fn expired(&self) -> bool {
        self.last.elapsed() > self.idle
    }
}
