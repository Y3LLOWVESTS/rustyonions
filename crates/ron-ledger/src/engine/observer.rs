//! RO:WHAT — Observer hook trait and typed ledger events for service-layer metrics, traces, and audit fan-out.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/DX. The library emits typed events instead of baking in Prometheus/OTEL.
//! RO:INTERACTS — crate::engine::ledger, future svc-ledger / svc-wallet wrappers.
//! RO:INVARIANTS — hooks are side-effect optional; failures in observers must not corrupt ledger truth.
//! RO:METRICS — future wrappers can map committed/rejected/replayed/checkpointed events into counters/histograms.
//! RO:CONFIG — none.
//! RO:SECURITY — events carry IDs and reasons only, not secrets.
//! RO:TEST — minimal smoke via example/test engine flows.

use crate::{
    error::RejectReason,
    types::{Root, Seq},
};

/// Typed observer events emitted by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerEvent {
    /// A batch was committed.
    BatchCommitted {
        /// First sequence in the batch.
        seq_start: Seq,
        /// Last sequence in the batch.
        seq_end: Seq,
        /// New head root.
        new_root: Root,
        /// Number of entries committed.
        entries: usize,
    },
    /// A request or entry was rejected.
    Rejected {
        /// Reject reason.
        reason: RejectReason,
        /// Optional failing entry index.
        idx: Option<usize>,
    },
    /// Replay completed on startup.
    Replayed {
        /// Number of entries replayed.
        entries: usize,
        /// Resulting head root.
        new_root: Root,
    },
    /// A checkpoint was written.
    Checkpointed {
        /// Sequence of the checkpoint.
        seq: Seq,
        /// Root at checkpoint.
        root: Root,
    },
}

/// Hook trait implemented by service wrappers or tests.
pub trait Observer: Send + Sync + 'static {
    /// Receive an event.
    fn on_event(&self, event: &LedgerEvent);
}

/// Observer that ignores all events.
#[derive(Debug, Default)]
pub struct NoopObserver;

impl Observer for NoopObserver {
    fn on_event(&self, _event: &LedgerEvent) {}
}
