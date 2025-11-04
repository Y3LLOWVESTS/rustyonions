//! Tiny readiness/degradation flag exposed to /readyz.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Default)]
struct ReadyStateInner {
    degraded: AtomicBool,
}

#[derive(Clone)]
pub struct ReadyState(Arc<ReadyStateInner>);

impl Default for ReadyState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadyState {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(ReadyStateInner::default()))
    }

    pub fn set_degraded(&self, d: bool) {
        self.0.degraded.store(d, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.0.degraded.load(Ordering::Relaxed)
    }

    /// Consider "ready" when not degraded (you can enrich later with more gates).
    #[must_use]
    pub fn ready(&self) -> bool {
        !self.is_degraded()
    }
}
