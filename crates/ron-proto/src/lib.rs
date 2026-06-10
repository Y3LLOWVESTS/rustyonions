//! RO:WHAT — Flat public facade for RustyOnions canonical DTOs (pure types).
//! RO:WHY  — Pillar 7 (SDK/Interop); Concerns: DX/RES. Deterministic, strict schemas for cross-SDK parity.
//! RO:INTERACTS — asset::*, identity::*, quickchain::*, oap frames, id::ContentId, manifest::*, mailbox::*, cap::*, error::*, naming::*, econ::*
//! RO:INVARIANTS — DTO-only; no I/O/crypto; #[serde(deny_unknown_fields)] on externals; OAP max_frame=1MiB.
//! RO:METRICS — N/A; library types only.
//! RO:CONFIG — None.
//! RO:SECURITY — No secrets; no verification; no wallet/ledger/storage mutation.
//! RO:TEST — See tests/ in crate: vectors, cross-version, asset manifest/page wire tests, identity profile wire tests, QuickChain DTO strictness tests.

#![forbid(unsafe_code)]
#![deny(warnings)]

pub mod asset;
pub mod cap;
pub mod config;
pub mod econ;
pub mod error;
pub mod gov;
pub mod id;
pub mod identity;
pub mod mailbox;
pub mod manifest;
pub mod naming;
pub mod oap;
pub mod quantum;
pub mod quickchain;
pub mod trace;
pub mod version;

pub use asset::*;
pub use cap::*;
pub use config::{Limits, Validate};
pub use econ::*;
pub use error::*;
pub use gov::*;
pub use id::*;
pub use identity::*;
pub use mailbox::*;
pub use manifest::*;
pub use naming::*;
pub use oap::*;
pub use quantum::*;
pub use quickchain::*;
pub use trace::*;
pub use version::*;
