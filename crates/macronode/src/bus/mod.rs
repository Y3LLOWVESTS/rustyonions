// crates/macronode/src/bus/mod.rs

//! RO:WHAT — Lightweight broadcast bus wrapper for Macronode.
//! RO:WHY  — Provide bounded, lag-aware pub/sub over `NodeEvent` without
//!           leaking `tokio::sync::broadcast` details across the crate.
//!
//! RO:INVARIANTS —
//!   - Bus is bounded; senders see an error if subscribers lag too far.
//!   - Consumers must handle `Lagged` by reconciling from a snapshot
//!     (supervisor/status endpoints can always provide an up-to-date view).
//!   - The API is intentionally tiny: `publish` + `subscribe`.
//!
//! RO:INTERACTS —
//!   - Will be threaded into `Supervisor` in a later step so that
//!     supervisor, services, and admin handlers can exchange events.
//!   - Event type is `NodeEvent` (an alias for `ron_kernel::KernelEvent`).

// This module intentionally shapes a future-facing API (NodeBus/NodeEvent)
// that is not wired anywhere *yet*. We allow dead_code here so that
// `cargo clippy -D warnings` stays green while we incrementally integrate
// the bus into supervisor/services in later slices.
#![allow(dead_code)]

use std::fmt;

use tokio::sync::broadcast;

/// Canonical event type carried by the Macronode bus.
///
/// For now this is *exactly* the kernel’s `KernelEvent` so there is a
/// single, shared event taxonomy across the project.
pub type NodeEvent = ron_kernel::KernelEvent;

/// Default channel capacity for the node bus.
///
/// This is deliberately modest; we want backpressure via `Lagged` errors
/// instead of unbounded growth. We can tune this later if needed.
const DEFAULT_CAPACITY: usize = 1024;

/// Cloneable handle to the Macronode event bus.
///
/// Internally, this wraps a `tokio::sync::broadcast::Sender<NodeEvent>`.
/// Subscribers obtain a `broadcast::Receiver<NodeEvent>` via `subscribe()`.
#[derive(Clone)]
pub struct NodeBus {
    tx: broadcast::Sender<NodeEvent>,
}

impl NodeBus {
    /// Create a new bus with the given bounded capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Create a new bus with a sensible default capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns `Ok(())` on success or a `SendError` if there were no
    /// active subscribers or if the channel was otherwise unable to
    /// accept the event.
    pub fn publish(&self, event: NodeEvent) -> Result<(), broadcast::error::SendError<NodeEvent>> {
        self.tx.send(event)?;
        Ok(())
    }

    /// Subscribe to the stream of node events.
    ///
    /// Callers **must** be prepared to handle `RecvError::Lagged(_)` on
    /// the returned receiver by re-syncing from a snapshot (e.g. via
    /// `/api/v1/status`) before continuing.
    pub fn subscribe(&self) -> broadcast::Receiver<NodeEvent> {
        self.tx.subscribe()
    }

    /// Access the underlying sender for advanced integrations.
    ///
    /// Most code should prefer `publish()` instead of using this directly.
    pub fn sender(&self) -> broadcast::Sender<NodeEvent> {
        self.tx.clone()
    }
}

impl fmt::Debug for NodeBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // We intentionally don’t expose internal channel state here.
        f.debug_struct("NodeBus").finish_non_exhaustive()
    }
}
