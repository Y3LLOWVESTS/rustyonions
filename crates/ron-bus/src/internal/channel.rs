//! RO:WHAT — Narrow wrapper for creating a bounded Tokio broadcast channel
//! RO:WHY  — Centralize invariants and future tweaks (e.g., debug asserts)
//! RO:INTERACTS — used by Bus::new()
//! RO:INVARIANTS — capacity >= 2; bounded queue; one receiver per task pattern
//! RO:TEST — Indirect via Bus integration tests

use tokio::sync::broadcast;

/// A simple constructor wrapper to emphasize bounded semantics.
pub fn bounded<T: Clone>(capacity: usize) -> (broadcast::Sender<T>, broadcast::Receiver<T>) {
    // Tokio ensures capacity >= 1 produces a bounded channel; we pre-validate in BusConfig.
    broadcast::channel(capacity)
}
