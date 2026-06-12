//! RO:WHAT — Library-first ROC ledger crate exposing DTOs, config, errors, the legacy append engine, and gated QuickChain preflight boundaries.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/GOV. Keep ledger truth separate from HTTP, wallet UX, and reward logic.
//! RO:INTERACTS — crate::{api,config,engine,error,types}; gated crate::quickchain; future svc-wallet adapters.
//! RO:INVARIANTS — append-only truth; deterministic replay; checked integer money; non-negative balances; QuickChain roots remain out of scope.
//! RO:METRICS — none directly; observer hooks allow service wrappers to emit latency and rejection metrics.
//! RO:CONFIG — LedgerConfig controls legacy batching/checkpoints; quickchain-preflight is compile-time gated.
//! RO:SECURITY — KID/cap refs are identifiers only; no custody, token verification, or client-assigned economic authority.
//! RO:TEST — tests/* plus fuzz/bench/example scaffolds; QuickChain tests require quickchain-preflight.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod api;
pub mod config;
pub mod engine;
pub mod error;
#[cfg(feature = "quickchain-preflight")]
pub mod quickchain;
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
