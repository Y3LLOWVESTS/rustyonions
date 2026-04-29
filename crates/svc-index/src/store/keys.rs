//! RO:WHAT — Stable svc-index keyspace helpers for manifest pointer records.
//! RO:WHY — Pillar 9; Concerns: RES/GOV/DX. Keeps mutable pointers separate from immutable bytes.
//! RO:INTERACTS — store::Store, types::{AssetManifestPointer, SiteManifestPointer}.
//! RO:INVARIANTS — raw bytes are never stored here; values are manifest pointers only.
//! RO:METRICS — none.
//! RO:CONFIG — honors store backend selected by Config.
//! RO:SECURITY — key normalization happens before these helpers are called.
//! RO:TEST — integration.rs and prop_index.rs.

/// Key prefix for immutable asset CID → mutable asset manifest pointer records.
pub const ASSET_MANIFEST_PREFIX: &str = "asset_manifest:";

/// Key prefix for site/name → mutable site manifest pointer records.
pub const SITE_MANIFEST_PREFIX: &str = "site_manifest:";

/// Build the storage key for an asset manifest pointer.
///
/// `canonical_asset_cid` must already be normalized as `b3:<64 lowercase hex>`.
#[must_use]
pub fn asset_manifest_key(canonical_asset_cid: &str) -> String {
    format!("{ASSET_MANIFEST_PREFIX}{canonical_asset_cid}")
}

/// Build the storage key for a site manifest pointer.
///
/// `canonical_name` must already be normalized by svc-index validation.
#[must_use]
pub fn site_manifest_key(canonical_name: &str) -> String {
    format!("{SITE_MANIFEST_PREFIX}{canonical_name}")
}
