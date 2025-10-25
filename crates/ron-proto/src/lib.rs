//! RO:WHAT — Flat public facade for RustyOnions canonical DTOs (pure types).
//! RO:WHY  — Pillar 7 (SDK/Interop); Concerns: DX/RES. Deterministic, strict schemas for cross-SDK parity.
//! RO:INTERACTS — oap (frames), id::ContentId, manifest::*, mailbox::*, cap::*, error::*, version::*, naming::*, econ::*, gov::*, quantum::*
//! RO:INVARIANTS — DTO-only (no I/O/crypto); #[serde(deny_unknown_fields)] on externals; OAP max_frame=1MiB; storage chunk≈64KiB.
//! RO:METRICS — N/A (library types only; reason strings stable for host metrics).
//! RO:CONFIG — None (schema helpers only).
//! RO:SECURITY — No secrets/PII; capability types are headers only (no verification).
//! RO:TEST — See tests/ in crate (vectors, cross-version, property tests).

#![forbid(unsafe_code)]
#![deny(warnings)]

pub mod cap;
pub mod config;
pub mod econ;
pub mod error;
pub mod gov;
pub mod id;
pub mod mailbox;
pub mod manifest;
pub mod naming;
pub mod oap;
pub mod quantum;
pub mod trace;
pub mod version; // <— export config helpers/traits

pub use cap::*;
pub use config::{Limits, Validate};
pub use econ::*;
pub use error::*;
pub use gov::*;
pub use id::*;
pub use mailbox::*;
pub use manifest::*;
pub use naming::*;
pub use oap::*;
pub use quantum::*;
pub use trace::*;
pub use version::*; // <— re-export trait + limits for ergonomics
