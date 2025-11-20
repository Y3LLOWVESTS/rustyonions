//! RO:WHAT — Macronode lifecycle states.
//! RO:WHY  — Give supervisor and admin plane a shared vocabulary for the
//!           node's coarse-grained lifecycle (starting, running, draining, stopped).
//! RO:INVARIANTS —
//!   - States form a simple DAG; we do not model every possible edge case.

#![allow(dead_code)]

/// Coarse-grained lifecycle state of the Macronode process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Process is booting, reading config, and binding listeners.
    Starting,
    /// Listeners are up and services are running.
    Running,
    /// Node is draining: rejecting new work and attempting graceful shutdown.
    Draining,
    /// Node has completed shutdown. In practice we usually exit before
    /// exposing this state, but it is useful for tests.
    Stopped,
}

impl LifecycleState {
    /// Returns true if the node should be considered "ready" to serve.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, LifecycleState::Running)
    }

    /// Returns true if the node is in a shutdown path.
    #[must_use]
    pub const fn is_draining(self) -> bool {
        matches!(self, LifecycleState::Draining | LifecycleState::Stopped)
    }
}
