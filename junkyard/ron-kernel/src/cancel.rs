#![forbid(unsafe_code)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Notify;

/// Simple cancellation token with parent/child semantics.
/// - `cancel()` notifies all waiters.
/// - `cancelled().await` resolves once cancelled.
/// - `child()` returns another handle to the same token (shared root).
#[derive(Clone)]
pub struct Shutdown {
    inner: Arc<Inner>,
}

struct Inner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl Shutdown {
    /// Create a new, not-yet-cancelled token.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    /// Request cancellation (idempotent).
    pub fn cancel(&self) {
        // Only notify once on the first transition to true.
        if !self.inner.cancelled.swap(true, Ordering::SeqCst) {
            self.inner.notify.notify_waiters();
        }
    }

    /// Wait until cancellation is requested.
    pub async fn cancelled(&self) {
        // Fast path
        if self.inner.cancelled.load(Ordering::SeqCst) {
            return;
        }
        // Slow path: wait for a notification, with a loop to avoid missed wakes.
        loop {
            if self.inner.cancelled.load(Ordering::SeqCst) {
                break;
            }
            self.inner.notify.notified().await;
            if self.inner.cancelled.load(Ordering::SeqCst) {
                break;
            }
        }
    }

    /// Create a child handle. (Here children share the same root signal.)
    pub fn child(&self) -> Self {
        self.clone()
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}
