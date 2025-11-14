//! ron-audit — audit-chain helpers for `AuditRecord`.
//!
//! RO:WHAT — Library for canonicalization, hashing, verification and sink traits
//!           over `AuditRecord`.
//! RO:WHY  — Give Micronode/Macronode hosts one precise place to agree on how
//!           audit chains are formed, hashed and checked.
//! RO:INTERACTS — Intended to be wired by svc-edge, svc-registry, svc-storage,
//!           micronode, macronode, etc. DTOs currently live locally; they can
//!           be migrated into `ron-proto` later without changing this crate's
//!           outward API.
//!
//! NOTE: The original IDB sketched `pub use ron_proto::audit::AuditRecord;`,
//! but `ron_proto::audit` does not exist yet. For now we define the DTOs
//! locally (in `dto`) and re-export from there.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::await_holding_lock)]

mod errors;

pub mod bounds;
pub mod canon;
pub mod dto;
pub mod hash;
pub mod metrics;
pub mod prelude;
pub mod privacy;
pub mod sink;
pub mod stream;
pub mod verify;

/// Primary DTO used by ron-audit — currently defined locally in `dto`.
pub use crate::dto::AuditRecord;

pub use crate::canon::CanonError;
pub use crate::errors::{AppendError, BoundsError, VerifyError};
pub use crate::sink::{AuditSink, AuditStream, ChainState};
