//! RO:WHAT — In-process event bus for svc-rewarder lifecycle events.
//! RO:WHY — Pillar 12; Concerns: RES/GOV. Keeps run events broadcastable without coupling to audit.
//! RO:INTERACTS — bus::events, http handlers, future ron-bus bridge.
//! RO:INVARIANTS — bounded tokio broadcast; no blocking on slow consumers.
//! RO:METRICS — lag metrics are future work in batch 2.
//! RO:CONFIG — capacity currently fixed by constructor caller.
//! RO:SECURITY — events carry identifiers only.
//! RO:TEST — compile/integration coverage.

use tokio::sync::broadcast;

use crate::bus::events::RewarderEvent;

pub mod events;

/// Small cloneable broadcast bus.
#[derive(Debug, Clone)]
pub struct RewarderBus {
    tx: broadcast::Sender<RewarderEvent>,
}

impl RewarderBus {
    /// Create a bounded bus.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    /// Publish event; no receivers is not fatal.
    pub fn publish(&self, event: RewarderEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> broadcast::Receiver<RewarderEvent> {
        self.tx.subscribe()
    }
}
