// crates/macronode/src/facets/mod.rs

//! RO:WHAT — Facet helpers for Macronode (permits, quotas, etc.).
//! RO:WHY  — Provide a small, crate-local façade around admission/quotas so
//!           higher layers can talk in terms of `PermitRequest` / `QuotaRequest`
//!           without depending on the eventual policy engine wiring.
//!
//! RO:INVARIANTS —
//!   - This module is intentionally tiny and mostly type re-exports.
//!   - It is OK for these types to be "unused" inside this crate for now;
//!     they are part of the public surface we are shaping for future use.
//!   - No I/O, no global state: pure data types and helpers only.
//!
//! RO:STATUS —
//!   - `permits` and `quotas` are currently simple data containers.
//!   - Enforcement is NOT yet wired into admin handlers or services; that
//!     will come when we introduce real admission control.

// These re-exports are part of the shaped API surface for macronode, and
// clippy will flag them as "unused" until we start plumbing them through
// the admin handlers and services. We explicitly allow that here so
// `cargo clippy -D warnings` stays green during the incremental build.
#![allow(unused_imports)]

pub mod permits;
pub mod quotas;

// Re-export the core facet types so callers can use
// `macronode::facets::{PermitRequest, PermitDecision, ...}`.
pub use permits::{PermitDecision, PermitKind, PermitRequest};
pub use quotas::{QuotaDecision, QuotaKey, QuotaRequest, QuotaWindow};
