//! RO:WHAT — Simple cooperative shutdown token for Macronode.
//! RO:WHY  — Give the supervisor and services a shared, cheap way to
//!           coordinate graceful shutdown without pulling in extra deps.
//! RO:INVARIANTS —
//!   - `trigger()` is idempotent.
//!   - `is_triggered()` is lock-free and wait-free.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Cheap, cloneable shutdown token.
///
/// This does not provide async notification; workers are expected to
/// periodically call `is_triggered()` inside their own loops.
#[derive(Clone, Debug)]
pub struct ShutdownToken {
    inner: Arc<AtomicBool>,
}

impl ShutdownToken {
    /// Construct a new token in the "not triggered" state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal shutdown to all holders of this token.
    pub fn trigger(&self) {
        self.inner.store(true, Ordering::Release);
    }

    /// Check whether shutdown has been requested.
    pub fn is_triggered(&self) -> bool {
        self.inner.load(Ordering::Acquire)
    }
}

impl Default for ShutdownToken {
    fn default() -> Self {
        Self::new()
    }
}
