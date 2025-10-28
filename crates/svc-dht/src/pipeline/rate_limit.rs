//! RO:WHAT — Simple global rate limiter for in-flight lookup legs
//! RO:WHY — Backpressure to avoid overload; Concerns: RES/PERF

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct Limiter {
    sem: std::sync::Arc<Semaphore>,
}

impl Limiter {
    /// new: max concurrent legs (global)
    pub fn new(max_legs: usize) -> Self {
        Self { sem: std::sync::Arc::new(Semaphore::new(max_legs)) }
    }
    pub async fn acquire(&self) -> OwnedSemaphorePermit {
        self.sem.clone().acquire_owned().await.expect("semaphore closed")
    }
}
