//! RO:WHAT — Library-first ROC ledger crate exposing DTOs, config, errors, and the deterministic engine.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/GOV. Keep ledger truth separate from HTTP, wallet UX, and reward logic.
//! RO:INTERACTS — crate::api, crate::types, crate::config, crate::error, crate::engine; future svc-wallet / svc-ledger wrappers.
//! RO:INVARIANTS — append-only truth; deterministic replay/root; no floats; non-negative balances; amnesia honored by storage profile.
//! RO:METRICS — none directly; observer hooks allow service wrappers to emit latency/reject/root metrics.
//! RO:CONFIG — LedgerConfig controls batching, checkpoints, engine mode, and size limits.
//! RO:SECURITY — KID/cap refs are identifiers only; no custody, no token verification, no PII logging in this crate.
//! RO:TEST — unit/integration/property tests in tests/* plus fuzz/bench/example scaffolds.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod api;
pub mod config;
pub mod engine;
pub mod error;
pub mod types;

pub use crate::config::{AccumulatorKind, EngineMode, LedgerConfig, Limits, PqMode};
pub use crate::engine::{
    CheckpointRecord, FileStorage, Ledger, LedgerEvent, MemoryStorage, NoopObserver, Observer,
    RootItem, Storage,
};
pub use crate::error::{LedgerError, RejectReason};
pub use crate::types::{
    AccountId, CapabilityRef, Entry, EntryKind, EntryRecord, Kid, Nonce, Root, Seq,
};
