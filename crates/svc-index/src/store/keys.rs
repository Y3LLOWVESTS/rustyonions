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

/// Key prefix for creator publication projection records.
pub const CREATOR_PUBLICATION_PREFIX: &str = "creator_publication:";

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

/// Build the storage prefix for one creator's publications.
#[must_use]
pub fn creator_publication_prefix(
    canonical_username: &str,
) -> String {
    format!(
        "{CREATOR_PUBLICATION_PREFIX}{canonical_username}:",
    )
}

/// Build the storage key for one creator publication.
#[must_use]
pub fn creator_publication_key(
    canonical_username: &str,
    canonical_publication_id: &str,
) -> String {
    format!(
        "{}{canonical_publication_id}",
        creator_publication_prefix(
            canonical_username,
        ),
    )
}


// FINAL_BETA_PHASE14A6A_PUBLICATION_RELATION_KEYS_V1

/// Key prefix for durable publication relation projections.
///
/// Relation values contain display-safe metadata only. Raw Comment/Image bytes
/// remain immutable CAS objects owned by svc-storage.
pub const PUBLICATION_RELATION_PREFIX: &str =
    "publication_relation:";

/// Build the prefix for every publication relation whose exact parent is the
/// supplied canonical typed crab URL.
#[must_use]
pub fn publication_relation_prefix(
    canonical_parent_crab_url: &str,
) -> String {
    format!(
        "{PUBLICATION_RELATION_PREFIX}{canonical_parent_crab_url}:",
    )
}

/// Build the storage key for one publication relation.
///
/// The publication identifier is already validated by the relation's embedded
/// PublicationSummaryV1 before the Store writes this key.
#[must_use]
pub fn publication_relation_key(
    canonical_parent_crab_url: &str,
    canonical_publication_id: &str,
) -> String {
    format!(
        "{}{canonical_publication_id}",
        publication_relation_prefix(
            canonical_parent_crab_url,
        ),
    )
}


// FINAL_BETA_PHASE15A4A2B1_SITE_PUBLICATION_KEYS_V1

/// Key prefix for durable named-Site root-publication projections.
///
/// Values are display/index metadata only. Raw publication bytes remain in
/// immutable CAS storage and Comment reply topology remains in relation keys.
pub const SITE_PUBLICATION_PREFIX: &str =
    "site_publication:";

/// Build the prefix for every root publication attached to one exact named Site.
#[must_use]
pub fn site_publication_prefix(
    canonical_site_crab_url:
        &str,
) -> String {
    format!(
        "{SITE_PUBLICATION_PREFIX}{canonical_site_crab_url}:",
    )
}

/// Build the key for one root publication attached to one exact named Site.
#[must_use]
pub fn site_publication_key(
    canonical_site_crab_url:
        &str,

    canonical_publication_id:
        &str,
) -> String {
    format!(
        "{}{canonical_publication_id}",
        site_publication_prefix(
            canonical_site_crab_url,
        ),
    )
}
