//! RO:WHAT — Hydrated b3 asset page response DTOs.
//! RO:WHY — Omnigate/gateway need a stable JSON shape for typed asset pages.
//! RO:INTERACTS — asset manifests, svc-index manifest pointers, svc-storage HEAD metadata, wallet receipts.
//! RO:INVARIANTS — DTO-only; no resolver logic; no direct storage/wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — receipt and account fields are references only; verification belongs upstream.
//! RO:TEST — asset_page_wire.rs covers stable wire shape, unknown-field rejection, and validation.

use serde::{Deserialize, Serialize};

use super::{
    manifest::{AssetManifestV1, AssetOwnerV1},
    payout::PayoutTarget,
    require_non_empty_bounded, validate_optional_bounded, AssetKind, AssetValidationError,
    MAX_CONTENT_TYPE_BYTES, MAX_REF_BYTES,
};

/// Current asset page response DTO version.
pub const ASSET_PAGE_VERSION: u16 = 1;

/// Availability and metadata returned by storage/index hydration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StorageAvailabilityV1 {
    /// Whether bytes are currently available.
    pub available: bool,
    /// Optional known size.
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Optional content type.
    #[serde(default)]
    pub content_type: Option<String>,
    /// Optional storage/provider reference.
    #[serde(default)]
    pub provider_ref: Option<String>,
    /// Optional raw object URL/path.
    #[serde(default)]
    pub raw_url: Option<String>,
}

impl StorageAvailabilityV1 {
    /// Validate storage metadata shape.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        validate_optional_bounded(
            "storage.content_type",
            self.content_type.as_deref(),
            MAX_CONTENT_TYPE_BYTES,
        )?;
        validate_optional_bounded(
            "storage.provider_ref",
            self.provider_ref.as_deref(),
            MAX_REF_BYTES,
        )?;
        validate_optional_bounded("storage.raw_url", self.raw_url.as_deref(), MAX_REF_BYTES)?;
        Ok(())
    }
}

/// Receipt kind shown on asset pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReceiptKind {
    /// Wallet hold receipt.
    Hold,
    /// Wallet capture receipt.
    Capture,
    /// Wallet release receipt.
    Release,
    /// Wallet issue/mint receipt.
    Issue,
    /// Wallet transfer receipt.
    Transfer,
    /// Wallet burn receipt.
    Burn,
    /// Paid storage receipt.
    PaidStorage,
    /// Paid content view receipt.
    PaidView,
    /// Paid site visit receipt.
    SiteVisit,
}

/// Lightweight receipt reference for display/hydration.
///
/// This is not a receipt verifier. Services fetch/verify receipts through `svc-wallet`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptRefV1 {
    /// Wallet transaction or receipt ID.
    pub tx_id: String,
    /// Receipt kind.
    pub receipt_kind: ReceiptKind,
    /// Optional ROC amount in integer minor units.
    #[serde(default)]
    pub amount_minor_units: Option<u64>,
    /// Optional account associated with the receipt.
    #[serde(default)]
    pub account: Option<String>,
    /// Optional timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub created_at_ms: Option<u64>,
}

impl ReceiptRefV1 {
    /// Validate receipt reference shape.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded("receipts[].tx_id", &self.tx_id, MAX_REF_BYTES)?;
        validate_optional_bounded("receipts[].account", self.account.as_deref(), MAX_REF_BYTES)?;
        Ok(())
    }
}

/// Links included on hydrated asset pages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetPageLinksV1 {
    /// Canonical public crab URL, e.g. `crab://<64hex>.image`.
    pub canonical_crab: String,
    /// Raw bytes URL/path if exposed.
    #[serde(default)]
    pub raw: Option<String>,
    /// Manifest URL/path if exposed.
    #[serde(default)]
    pub manifest: Option<String>,
    /// Paid view URL/path if exposed.
    #[serde(default)]
    pub paid_view: Option<String>,
}

impl AssetPageLinksV1 {
    /// Validate page link shape.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded("links.canonical_crab", &self.canonical_crab, MAX_REF_BYTES)?;
        if !self.canonical_crab.starts_with("crab://") {
            return Err(AssetValidationError::EmptyField {
                field: "links.canonical_crab",
            });
        }

        validate_optional_bounded("links.raw", self.raw.as_deref(), MAX_REF_BYTES)?;
        validate_optional_bounded("links.manifest", self.manifest.as_deref(), MAX_REF_BYTES)?;
        validate_optional_bounded("links.paid_view", self.paid_view.as_deref(), MAX_REF_BYTES)?;
        Ok(())
    }
}

/// Hydrated asset page response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetPageV1 {
    /// DTO version. Must be `1`.
    pub version: u16,
    /// Asset content ID.
    pub asset_cid: crate::id::ContentId,
    /// Asset kind.
    pub asset_kind: AssetKind,
    /// Optional manifest data if discovered.
    #[serde(default)]
    pub manifest: Option<AssetManifestV1>,
    /// Optional owner data if known.
    #[serde(default)]
    pub owner: Option<AssetOwnerV1>,
    /// Optional payout data if known.
    #[serde(default)]
    pub payout: Option<PayoutTarget>,
    /// Storage availability.
    pub storage: StorageAvailabilityV1,
    /// Receipt references.
    #[serde(default)]
    pub receipts: Vec<ReceiptRefV1>,
    /// Page links.
    pub links: AssetPageLinksV1,
    /// Non-fatal hydration warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl AssetPageV1 {
    /// Validate hydrated asset page shape.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        if self.version != ASSET_PAGE_VERSION {
            return Err(AssetValidationError::InvalidVersion {
                ty: "AssetPageV1",
                expected: ASSET_PAGE_VERSION,
                actual: self.version,
            });
        }

        if let Some(manifest) = &self.manifest {
            manifest.validate()?;

            if manifest.asset_cid != self.asset_cid {
                return Err(AssetValidationError::AssetKindMismatch {
                    expected: self.asset_cid.to_string(),
                    actual: manifest.asset_cid.to_string(),
                });
            }

            if manifest.asset_kind != self.asset_kind {
                return Err(AssetValidationError::AssetKindMismatch {
                    expected: self.asset_kind.suffix().to_owned(),
                    actual: manifest.asset_kind.suffix().to_owned(),
                });
            }
        }

        if let Some(owner) = &self.owner {
            owner.validate()?;
        }

        if let Some(payout) = &self.payout {
            payout.validate()?;
        }

        self.storage.validate()?;

        for receipt in &self.receipts {
            receipt.validate()?;
        }

        for warning in &self.warnings {
            require_non_empty_bounded("warnings[]", warning, MAX_REF_BYTES)?;
        }

        self.links.validate()
    }
}
