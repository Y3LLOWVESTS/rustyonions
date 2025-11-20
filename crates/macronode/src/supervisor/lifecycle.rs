//! RO:WHAT — Lifecycle primitives for supervised service tasks.
//! RO:WHY  — Provide a small abstraction for representing a running worker
//!           (JoinHandle + metadata) without yet implementing crash detection.
//! RO:INVARIANTS —
//!   - Zero side effects today.
//!   - No panics; noop watch() until the next slice.
//!   - Forward-compatible with crash detection and restart loops.

#![allow(dead_code)]

use std::fmt;
use tokio::task::JoinHandle;

/// High-level state of a supervised service.
///
/// This is intentionally simple; we'll expand once restart flows land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Starting,
    Running,
    Stopping,
    Stopped,
}

/// A managed worker task.
///
/// Right now this is just a placeholder so that Supervisor can begin to
/// track per-service join handles without altering behavior.
pub struct ManagedTask {
    pub service_name: &'static str,
    pub handle: JoinHandle<()>,
}

impl ManagedTask {
    /// Construct a new managed worker.
    #[must_use]
    pub fn new(service_name: &'static str, handle: JoinHandle<()>) -> Self {
        Self {
            service_name,
            handle,
        }
    }

    /// Placeholder: later this will become the crash detection hook.
    ///
    /// For now it simply awaits the handle and swallows any JoinError so
    /// existing test behavior is unaffected.
    pub async fn watch(self) {
        let _ = self.handle.await;
        // In a future step:
        //   - detect crash vs clean exit
        //   - update probes
        //   - notify supervisor restart loop
    }
}

impl fmt::Debug for ManagedTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManagedTask")
            .field("service_name", &self.service_name)
            .finish()
    }
}
