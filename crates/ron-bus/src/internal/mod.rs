//! RO:WHAT — Small internal helpers to keep public files tiny.
//! RO:WHY  — Encapsulate wrappers/heuristics and document invariants once.
//! RO:INTERACTS — `channel` (Tokio broadcast wrapper); optional `depth_estimator`.
//! RO:INVARIANTS — No locks across `.await`; bounded channels only; no bg tasks.

pub mod channel;
pub mod depth_estimator;

pub use channel::bounded as bounded_channel;
pub use depth_estimator::{DepthEstimator, DepthSnapshot};

#[cfg(test)]
mod _doc_pattern {
    use tokio::sync::broadcast;

    // Ensure our wrapper returns the same types we expect from Tokio.
    #[test]
    fn channel_types_match() {
        let (tx, rx): (broadcast::Sender<u8>, broadcast::Receiver<u8>) = super::channel::bounded(8);
        drop((tx, rx));
    }
}
