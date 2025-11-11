//! Process supervisor scaffold for svc-edge.
//!
//! RO:WHAT
//! - Placeholder for listener/task lifecycle management and orderly shutdown.
//!
//! RO:NEXT
//! - Add accept loops, task JoinHandles, and a `cancel()` to request drain.

/// No-op supervisor placeholder.
#[derive(Debug, Clone, Default)]
pub struct Supervisor;

impl Supervisor {
    /// Create a new no-op supervisor.
    pub fn new() -> Self {
        Self
    }
    /// Request shutdown (no-op for now).
    pub async fn shutdown(&self) {
        // Future: signal tasks and await joins with a timeout.
    }
}
