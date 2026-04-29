//! RO:WHAT — AssetManifestV1 and metadata DTOs for WEB3_2 typed b3 asset pages.
//! RO:WHY — Asset manifests bind immutable bytes to owner, payout, metadata, provenance, storage refs, and receipts.
//! RO:INTERACTS — id::ContentId, asset::ownership, asset::payout, asset::page, svc-index, omnigate.
//! RO:INVARIANTS — asset bytes stay immutable by CID; manifests are metadata; no storage/wallet mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — owner/payout fields are declarations only; authorization is enforced elsewhere.
//! RO:TEST — asset_manifest.rs.

use serde::{Deserialize, Serialize};

use super::{
    page::{ReceiptRefV1, StorageAvailabilityV1},
    payout::PayoutTarget,
    require_non_empty_bounded, validate_optional_bounded, validate_tags, AssetKind,
    AssetValidationError, MAX_ASSET_DESCRIPTION_BYTES, MAX_ASSET_LICENSE_BYTES,
    MAX_ASSET_TITLE_BYTES, MAX_CONTENT_TYPE_BYTES, MAX_REF_BYTES,
};

/// Current asset manifest DTO version.
pub const ASSET_MANIFEST_VERSION: u16 = 1;

/// Owner fields embedded in asset manifests and hydrated pages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetOwnerV1 {
    /// Passport subject that owns or controls the asset manifest.
    pub passport_subject: String,
    /// Wallet account associated with the owner.
    pub wallet_account: String,
}

impl AssetOwnerV1 {
    /// Validate owner fields.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded(
            "owner.passport_subject",
            &self.passport_subject,
            MAX_REF_BYTES,
        )?;
        require_non_empty_bounded("owner.wallet_account", &self.wallet_account, MAX_REF_BYTES)?;
        Ok(())
    }
}

/// User/product metadata for an asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetMetadataV1 {
    /// Human title.
    pub title: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Tags in deterministic order supplied by caller/index.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional license string.
    #[serde(default)]
    pub license: Option<String>,
    /// Optional media/content type.
    #[serde(default)]
    pub content_type: Option<String>,
}

impl AssetMetadataV1 {
    /// Validate bounded metadata fields.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded("metadata.title", &self.title, MAX_ASSET_TITLE_BYTES)?;
        validate_optional_bounded(
            "metadata.description",
            self.description.as_deref(),
            MAX_ASSET_DESCRIPTION_BYTES,
        )?;
        validate_optional_bounded(
            "metadata.license",
            self.license.as_deref(),
            MAX_ASSET_LICENSE_BYTES,
        )?;
        validate_optional_bounded(
            "metadata.content_type",
            self.content_type.as_deref(),
            MAX_CONTENT_TYPE_BYTES,
        )?;
        validate_tags(&self.tags)?;
        Ok(())
    }
}

/// Provenance metadata for an asset manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetProvenanceV1 {
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at_ms: u64,
    /// Optional source string or source ref.
    #[serde(default)]
    pub source: Option<String>,
    /// Optional parent/derived-from CIDs.
    #[serde(default)]
    pub parent_cids: Vec<crate::id::ContentId>,
}

impl AssetProvenanceV1 {
    /// Validate provenance fields.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        if self.created_at_ms == 0 {
            return Err(AssetValidationError::EmptyField {
                field: "provenance.created_at_ms",
            });
        }

        validate_optional_bounded("provenance.source", self.source.as_deref(), MAX_REF_BYTES)?;
        Ok(())
    }
}

/// Optional curator metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CuratorMetadataV1 {
    /// Curator passport subject.
    pub curator_passport_subject: String,
    /// Optional curator notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Curator tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl CuratorMetadataV1 {
    /// Validate curator metadata.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded(
            "curator.curator_passport_subject",
            &self.curator_passport_subject,
            MAX_REF_BYTES,
        )?;
        validate_optional_bounded(
            "curator.notes",
            self.notes.as_deref(),
            MAX_ASSET_DESCRIPTION_BYTES,
        )?;
        validate_tags(&self.tags)?;
        Ok(())
    }
}

/// Versioned asset manifest.
///
/// This metadata does not mutate raw bytes. Raw bytes are addressed by `asset_cid`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetManifestV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Immutable asset content ID.
    pub asset_cid: crate::id::ContentId,
    /// Typed asset kind.
    pub asset_kind: AssetKind,
    /// Optional manifest CID if this manifest itself was stored as content-addressed bytes.
    #[serde(default)]
    pub manifest_cid: Option<crate::id::ContentId>,
    /// Owner declaration.
    pub owner: AssetOwnerV1,
    /// Default payout declaration.
    pub payout: PayoutTarget,
    /// Product metadata.
    pub metadata: AssetMetadataV1,
    /// Provenance metadata.
    pub provenance: AssetProvenanceV1,
    /// Optional storage/provider availability metadata.
    #[serde(default)]
    pub storage: Option<StorageAvailabilityV1>,
    /// Optional receipt references.
    #[serde(default)]
    pub receipts: Vec<ReceiptRefV1>,
    /// Optional curator metadata.
    #[serde(default)]
    pub curator: Option<CuratorMetadataV1>,
}

impl AssetManifestV1 {
    /// Validate the manifest shape.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        if self.version != ASSET_MANIFEST_VERSION {
            return Err(AssetValidationError::InvalidVersion {
                ty: "AssetManifestV1",
                expected: ASSET_MANIFEST_VERSION,
                actual: self.version,
            });
        }

        self.owner.validate()?;
        self.payout.validate()?;
        self.metadata.validate()?;
        self.provenance.validate()?;

        if let Some(storage) = &self.storage {
            storage.validate()?;
        }

        for receipt in &self.receipts {
            receipt.validate()?;
        }

        if let Some(curator) = &self.curator {
            curator.validate()?;
        }

        Ok(())
    }

    /// Validate that this manifest matches a typed crab suffix.
    pub fn validate_for_crab_suffix(&self, suffix: &str) -> Result<(), AssetValidationError> {
        self.validate()?;

        if !self.asset_kind.matches_suffix(suffix) {
            return Err(AssetValidationError::AssetKindMismatch {
                expected: suffix.trim().to_ascii_lowercase(),
                actual: self.asset_kind.suffix().to_owned(),
            });
        }

        Ok(())
    }
}
