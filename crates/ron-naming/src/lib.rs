//! RO:WHAT — Public entry for RON naming/addressing, crab links, asset kinds, and wire helpers.
//! RO:WHY  — Pillar 9 (Content & Naming). This crate defines schemas & hygiene only;
//!           runtime lookups live in svc-index (DHT/overlay are elsewhere).
//! RO:INTERACTS — crate::{address, asset, crab, normalize, types, username, version, wire::*}
//! RO:INVARIANTS — DTOs are pure; content ids are canonical "b3:<64 lowercase hex>"; no IO or async.
//! RO:SECURITY — No ambient authority, no network, no storage, no wallet/ledger mutation.
//! RO:TEST — unit tests in module files; round-trip vectors in tests/; crab tests in tests/crab_links.rs.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, missing_docs, clippy::all)]

pub mod address;
pub mod asset;
pub mod crab;
pub mod normalize;
pub mod types;
pub mod username;
pub mod version;

/// Wire-encoding helpers (JSON/CBOR) for DTO round-trips.
///
/// These are thin serde wrappers used by tests/examples/SDKs. Transport/runtime
/// concerns live in services such as `svc-index`; this module is schema-focused.
pub mod wire {
    /// CBOR helpers.
    pub mod cbor;
    /// JSON helpers.
    pub mod json;
}

#[cfg(feature = "verify")]
pub mod verify;

pub use address::{Address, ParseAddressError};
pub use asset::{AssetKind, AssetKindParseError};
pub use crab::{CrabLink, CrabNamespace, CrabParseError, CrabRoute, CRAB_SCHEME};
pub use normalize::{normalize_fqdn_ascii, NormalizedFqdn};
pub use types::{ContentId, Fqdn, NameRecord};
pub use username::{normalize_handle, normalize_username, RonUsername, UsernameParseError};
pub use version::{NameVersion, VersionParseError};
