//! RO:WHAT — WEB3_2 asset DTO entrypoint: kinds, manifests, ownership, payout, and asset pages.
//! RO:WHY — Product-proof layer needs stable cross-service message shapes without service logic.
//! RO:INTERACTS — id::ContentId, svc-index, omnigate, svc-storage, svc-wallet receipts by reference only.
//! RO:INVARIANTS — DTO-only; no IO; no async; internal CIDs stay canonical `b3:<64 lowercase hex>`.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no signing/verification here; signatures and capabilities are represented only.
//! RO:TEST — tests/asset_manifest.rs and tests/asset_page_wire.rs.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt, str::FromStr};
use thiserror::Error;

pub mod manifest;
pub mod ownership;
pub mod page;
pub mod payout;

pub use manifest::{
    AssetManifestV1, AssetMetadataV1, AssetOwnerV1, AssetProvenanceV1, CuratorMetadataV1,
    ASSET_MANIFEST_VERSION,
};
pub use ownership::{
    AssetOwnershipClaimV1, OwnershipClaimType, SignatureRefV1, ASSET_OWNERSHIP_CLAIM_VERSION,
};
pub use page::{
    AssetPageLinksV1, AssetPageV1, ReceiptKind, ReceiptRefV1, StorageAvailabilityV1,
    ASSET_PAGE_VERSION,
};
pub use payout::{PayoutRole, PayoutSplitV1, PayoutTarget};

/// Maximum title length in bytes.
pub const MAX_ASSET_TITLE_BYTES: usize = 160;
/// Maximum description length in bytes.
pub const MAX_ASSET_DESCRIPTION_BYTES: usize = 2_048;
/// Maximum license string length in bytes.
pub const MAX_ASSET_LICENSE_BYTES: usize = 128;
/// Maximum content type string length in bytes.
pub const MAX_CONTENT_TYPE_BYTES: usize = 128;
/// Maximum generic URI/link/account/ref string length in bytes.
pub const MAX_REF_BYTES: usize = 512;
/// Maximum tags per asset metadata block.
pub const MAX_ASSET_TAGS: usize = 32;
/// Maximum single tag length in bytes.
pub const MAX_ASSET_TAG_BYTES: usize = 64;
/// Basis-point denominator for payout splits.
pub const BPS_DENOMINATOR: u32 = 10_000;

/// Typed WEB3_2 asset kind vocabulary.
///
/// The public crab URL format is:
///
/// `crab://<64 lowercase hex>.<asset_kind>`
///
/// Internally the content ID is still `b3:<64 lowercase hex>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AssetKind {
    /// Image or visual artwork.
    Image,
    /// Video asset.
    Video,
    /// Music/audio asset.
    Music,
    /// Song/audio asset.
    Song,
    /// Article or long-form text.
    Article,
    /// Post/feed object.
    Post,
    /// Comment/discussion object.
    Comment,
    /// Web page.
    Page,
    /// Site manifest or site root object.
    Site,
    /// Application/app manifest object.
    App,
    /// Manifest metadata object.
    Manifest,
}

impl AssetKind {
    /// Return the canonical suffix used in public crab asset URLs.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            AssetKind::Image => "image",
            AssetKind::Video => "video",
            AssetKind::Music => "music",
            AssetKind::Song => "song",
            AssetKind::Article => "article",
            AssetKind::Post => "post",
            AssetKind::Comment => "comment",
            AssetKind::Page => "page",
            AssetKind::Site => "site",
            AssetKind::App => "app",
            AssetKind::Manifest => "manifest",
        }
    }

    /// Return true when the suffix matches this kind after ASCII lowercase normalization.
    #[must_use]
    pub fn matches_suffix(self, suffix: &str) -> bool {
        self.suffix() == suffix.trim().to_ascii_lowercase()
    }
}

impl fmt::Display for AssetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.suffix())
    }
}

impl FromStr for AssetKind {
    type Err = AssetValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "image" => Ok(Self::Image),
            "video" => Ok(Self::Video),
            "music" => Ok(Self::Music),
            "song" => Ok(Self::Song),
            "article" => Ok(Self::Article),
            "post" => Ok(Self::Post),
            "comment" => Ok(Self::Comment),
            "page" => Ok(Self::Page),
            "site" => Ok(Self::Site),
            "app" => Ok(Self::App),
            "manifest" => Ok(Self::Manifest),
            other => Err(AssetValidationError::UnsupportedAssetKind {
                kind: other.to_owned(),
            }),
        }
    }
}

/// Deterministic validation errors for WEB3_2 asset DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssetValidationError {
    /// Version field did not match the DTO version.
    #[error("invalid {ty} version: expected {expected}, got {actual}")]
    InvalidVersion {
        /// DTO type name.
        ty: &'static str,
        /// Expected version.
        expected: u16,
        /// Actual version.
        actual: u16,
    },
    /// Required field was empty or all whitespace.
    #[error("empty field: {field}")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },
    /// Bounded field exceeded its max length.
    #[error("field too long: {field} max={max} actual={actual}")]
    FieldTooLong {
        /// Field name.
        field: &'static str,
        /// Max bytes.
        max: usize,
        /// Actual bytes.
        actual: usize,
    },
    /// Unsupported asset kind suffix.
    #[error("unsupported asset kind: {kind}")]
    UnsupportedAssetKind {
        /// Unsupported kind.
        kind: String,
    },
    /// Manifest/page asset kind did not match the expected suffix/kind.
    #[error("asset kind mismatch: expected {expected}, got {actual}")]
    AssetKindMismatch {
        /// Expected suffix.
        expected: String,
        /// Actual suffix.
        actual: String,
    },
    /// Too many tags.
    #[error("too many tags: max={max} actual={actual}")]
    TooManyTags {
        /// Max tag count.
        max: usize,
        /// Actual tag count.
        actual: usize,
    },
    /// Duplicate tag.
    #[error("duplicate tag: {tag}")]
    DuplicateTag {
        /// Duplicate tag value.
        tag: String,
    },
    /// Payout splits did not sum to exactly 10_000 basis points.
    #[error("invalid payout split total: expected {expected_bps} bps, got {actual_bps} bps")]
    InvalidBpsTotal {
        /// Expected bps total.
        expected_bps: u32,
        /// Actual bps total.
        actual_bps: u32,
    },
    /// Timestamp ordering was invalid.
    #[error("invalid time order: {field}")]
    InvalidTimeOrder {
        /// Field/relation name.
        field: &'static str,
    },
}

/// Build the canonical public crab URL for a typed asset page.
///
/// The input CID remains internal canonical `b3:<64 lowercase hex>`, while the public
/// crab URL omits the `b3/` path prefix.
#[must_use]
pub fn canonical_crab_asset_url(asset_cid: &crate::id::ContentId, asset_kind: AssetKind) -> String {
    let raw_hash = asset_cid
        .as_str()
        .strip_prefix(crate::id::CONTENT_ID_PREFIX)
        .unwrap_or(asset_cid.as_str());

    format!("crab://{raw_hash}.{}", asset_kind.suffix())
}

pub(crate) fn require_non_empty_bounded(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), AssetValidationError> {
    if value.trim().is_empty() {
        return Err(AssetValidationError::EmptyField { field });
    }
    validate_bounded(field, value, max)
}

pub(crate) fn validate_optional_bounded(
    field: &'static str,
    value: Option<&str>,
    max: usize,
) -> Result<(), AssetValidationError> {
    if let Some(value) = value {
        validate_bounded(field, value, max)?;
    }
    Ok(())
}

pub(crate) fn validate_bounded(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), AssetValidationError> {
    let actual = value.len();
    if actual > max {
        return Err(AssetValidationError::FieldTooLong { field, max, actual });
    }
    Ok(())
}

pub(crate) fn validate_tags(tags: &[String]) -> Result<(), AssetValidationError> {
    if tags.len() > MAX_ASSET_TAGS {
        return Err(AssetValidationError::TooManyTags {
            max: MAX_ASSET_TAGS,
            actual: tags.len(),
        });
    }

    let mut seen = BTreeSet::new();
    for tag in tags {
        require_non_empty_bounded("metadata.tags[]", tag, MAX_ASSET_TAG_BYTES)?;

        let canonical = tag.trim().to_ascii_lowercase();
        if !seen.insert(canonical.clone()) {
            return Err(AssetValidationError::DuplicateTag { tag: canonical });
        }
    }

    Ok(())
}
