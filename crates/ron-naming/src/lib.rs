//! RO:WHAT — Public entry for RON naming/addressing types and wire helpers.
//! RO:WHY  — Pillar 9 (Content & Naming). This crate defines schemas & hygiene only;
//!           runtime lookups live in svc-index (DHT/overlay are elsewhere).
//! RO:INTERACTS — crate::types, crate::normalize, crate::address, crate::version, crate::wire::*
//! RO:INVARIANTS — DTOs are pure (serde, deny_unknown_fields); content ids are "b3:<hex>"; no locks across .await.
//! RO:SECURITY — No ambient I/O or network; pure value types; amnesia posture is N/A here.
//! RO:TEST — unit tests in module files; round-trip vectors in tests/ (JSON/CBOR).

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, missing_docs, clippy::all)]

pub mod address;
pub mod normalize;
pub mod types;
pub mod version;

/// Wire-encoding helpers (JSON/CBOR) for DTO round-trips.
///
/// These are thin serde wrappers used by tests/examples/SDKs. Transport/runtime
/// concerns live in services (e.g., svc-index); this module is schema-focused.
pub mod wire {
    /// CBOR helpers.
    pub mod cbor;
    /// JSON helpers.
    pub mod json;
}

#[cfg(feature = "verify")]
pub mod verify;

pub use address::{Address, ParseAddressError};
pub use normalize::{normalize_fqdn_ascii, NormalizedFqdn};
pub use types::{ContentId, Fqdn, NameRecord};
pub use version::{NameVersion, VersionParseError};
