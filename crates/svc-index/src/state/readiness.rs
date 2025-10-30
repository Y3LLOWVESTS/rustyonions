//! RO:WHAT — Health/Readiness gate with truthful signals.

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct HealthState {
    ready: std::sync::Arc<AtomicBool>,
}

impl Default for HealthState {
    fn default() -> Self {
        Self {
            ready: std::sync::Arc::new(AtomicBool::new(false)),
        }
    }
}

impl HealthState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn mark_ready(&self) {
        self.ready.store(true, Ordering::SeqCst);
    }
    pub fn mark_not_ready(&self) {
        self.ready.store(false, Ordering::SeqCst);
    }
    pub fn all_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::SeqCst)
    }
}
