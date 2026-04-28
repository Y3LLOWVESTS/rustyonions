//! RO:WHAT — Ledger engine surface: storage backends, observer hooks, accumulator, replay, checkpoints, and the core writer.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/GOV. Keep append-only truth in small modules with explicit seams.
//! RO:INTERACTS — crate::api, crate::config, crate::types, crate::error.
//! RO:INVARIANTS — single-writer mutation path; deterministic replay; storage-agnostic engine; no service/runtime coupling.
//! RO:METRICS — observer events are the only outward hook for service metrics.
//! RO:CONFIG — LedgerConfig drives batching/checkpoints/engine posture.
//! RO:SECURITY — this layer never verifies caps or holds secrets; it only stores identifiers.
//! RO:TEST — replay_recovery.rs, idempotency_prop.rs, interop_vectors.rs, benches/micro.rs.

pub mod accumulator;
pub mod checkpoint;
pub mod ledger;
pub mod observer;
pub mod replay;
pub mod storage;

pub use crate::api::RootItem;
pub use crate::types::CheckpointRecord;
pub use ledger::Ledger;
pub use observer::{LedgerEvent, NoopObserver, Observer};
pub use storage::{FileStorage, MemoryStorage, Storage};
