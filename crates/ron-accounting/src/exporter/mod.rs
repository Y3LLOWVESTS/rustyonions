//! RO:WHAT — Ordered/idempotent exporter support for sealed accounting slices.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/PERF. Export keeps order without mutating ledger truth.
//! RO:INTERACTS — accounting::SealedSlice, AckLru, lanes, router, worker, future ledger adapter.
//! RO:INVARIANTS — one stream per `(tenant,dimension)`; no N+1 before N; bounded queues.
//! RO:METRICS — worker/router callers update export latency/failure/backlog metrics.
//! RO:CONFIG — exporter.ordered_buffer_cap and backoff knobs.
//! RO:SECURITY — no ambient authority; downstream adapters enforce capabilities.
//! RO:TEST — unit: exporter_ordering_tests; loom models in later batch.

pub mod ack_lru;
pub mod lane;
pub mod router;
pub mod trait_;
#[path = "trait.rs"]
pub mod trait_mod;
pub mod worker;

pub use ack_lru::{AckKey, AckLru};
pub use lane::{ExportLane, StreamKey};
pub use router::ExporterRouter;
pub use trait_::{Ack, BoxExportFuture, Exporter};
pub use worker::{export_next, export_one, export_until_blocked, ExportReport, ExportRetryPolicy};
