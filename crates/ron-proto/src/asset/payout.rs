//! RO:WHAT — Payout target DTOs for WEB3_2 asset/site/content flows.
//! RO:WHY — Cross-service payout shapes must be strict without executing wallet mutations.
//! RO:INTERACTS — asset manifests, asset pages, svc-wallet receipts by tx_id/account references only.
//! RO:INVARIANTS — integer bps only; no floating point; no direct wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — payout policies are loaded elsewhere, not here.
//! RO:SECURITY — accounts are identifiers only; no spend authority is represented here.
//! RO:TEST — asset_manifest.rs validates split totals and required accounts.

use serde::{Deserialize, Serialize};

use super::{require_non_empty_bounded, AssetValidationError, BPS_DENOMINATOR, MAX_REF_BYTES};

/// Role of a recipient in a payout split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PayoutRole {
    /// Original creator or creator-owned wallet.
    Creator,
    /// Asset owner.
    AssetOwner,
    /// Site owner.
    SiteOwner,
    /// Curator/referrer/collection maintainer.
    Curator,
    /// Storage provider.
    StorageProvider,
    /// Gateway/edge provider.
    EdgeProvider,
    /// Protocol treasury.
    Treasury,
    /// Reward pool.
    RewardPool,
    /// Referrer/affiliate.
    Referrer,
}

/// Individual payout split line.
///
/// `bps` is integer basis points. No floating point is allowed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PayoutSplitV1 {
    /// Recipient role.
    pub role: PayoutRole,
    /// Recipient wallet/account ID.
    pub account: String,
    /// Basis points for this recipient.
    pub bps: u16,
}

impl PayoutSplitV1 {
    /// Validate split line fields.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded("payout.splits[].account", &self.account, MAX_REF_BYTES)?;
        Ok(())
    }
}

/// Default payout target for an asset/page action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PayoutTarget {
    /// Action that triggers payout, e.g. `asset_publish`, `paid_content_view`.
    pub default_action: String,
    /// Primary recipient account.
    pub recipient_account: String,
    /// Optional deterministic splits. If present, total must equal 10_000 bps.
    #[serde(default)]
    pub splits: Vec<PayoutSplitV1>,
}

impl PayoutTarget {
    /// Validate payout fields and basis-point totals.
    pub fn validate(&self) -> Result<(), AssetValidationError> {
        require_non_empty_bounded("payout.default_action", &self.default_action, MAX_REF_BYTES)?;
        require_non_empty_bounded(
            "payout.recipient_account",
            &self.recipient_account,
            MAX_REF_BYTES,
        )?;

        if self.splits.is_empty() {
            return Ok(());
        }

        let mut total_bps = 0_u32;
        for split in &self.splits {
            split.validate()?;
            total_bps = total_bps.saturating_add(u32::from(split.bps));
        }

        if total_bps != BPS_DENOMINATOR {
            return Err(AssetValidationError::InvalidBpsTotal {
                expected_bps: BPS_DENOMINATOR,
                actual_bps: total_bps,
            });
        }

        Ok(())
    }
}
