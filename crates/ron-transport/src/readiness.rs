//! RO:WHAT — Minimal readiness gate for listeners.
//! RO:WHY  — Truthful /readyz for services consuming transport.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ReadyGate {
    listeners_bound: Arc<AtomicBool>,
}

impl ReadyGate {
    pub fn new() -> Self {
        Self {
            listeners_bound: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn set_listeners_bound(&self, v: bool) {
        self.listeners_bound.store(v, Ordering::SeqCst);
    }
    pub fn listeners_bound(&self) -> bool {
        self.listeners_bound.load(Ordering::SeqCst)
    }
}
