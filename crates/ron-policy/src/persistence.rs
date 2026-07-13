//! RO:WHAT — Declarative exact-B3 persistence eligibility policy.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 11 permits durable storage only after
//! moderation, category, review-threshold, and optional operator-pin checks
//! succeed.
//!
//! RO:INTERACTS — moderation policy, `ron-proto::asset::AssetKind`, and
//! `svc-storage` persistence state.
//!
//! RO:INVARIANTS — moderation refusal wins; defaults fail closed; policy
//! performs no storage I/O.
//!
//! RO:SECURITY — no byte movement, disk writes, provider mutation, rewards,
//! wallet authority, or ledger authority.
//!
//! RO:TEST — `tests/persistence_policy.rs`.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use ron_proto::asset::AssetKind;
use serde::{Deserialize, Serialize};

use crate::moderation::{B3Id, Policy as ModerationPolicy, ReasonCode as ModerationReasonCode};

/// Review strength established for an exact object.
///
/// Variant order is intentional and allows deterministic threshold comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewLevel {
    /// The object has not completed a persistence review.
    Unreviewed,
    /// The object's bytes and exact content identity were verified.
    IntegrityVerified,
    /// The object completed the required moderation approval.
    #[default]
    ModerationApproved,
}

impl ReviewLevel {
    /// Return the canonical low-cardinality label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::IntegrityVerified => "integrity_verified",
            Self::ModerationApproved => "moderation_approved",
        }
    }

    /// Whether this review level meets the required threshold.
    #[must_use]
    pub const fn meets(self, required: Self) -> bool {
        self as u8 >= required as u8
    }
}

/// Requested policy operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// Review eligibility for ordinary durable persistence.
    Persist,
    /// Review eligibility for explicit operator pinning.
    Pin,
}

impl Intent {
    /// Return the canonical low-cardinality label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Persist => "persist",
            Self::Pin => "pin",
        }
    }
}

/// Resulting persistence posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Policy permits a later persistence-state transition or backend action.
    Eligible,
    /// Policy refuses persistence or pinning.
    Ineligible,
}

impl Effect {
    /// Whether this effect permits later persistence behavior.
    #[must_use]
    pub const fn permits_persistence(self) -> bool {
        matches!(self, Self::Eligible)
    }
}

/// Stable persistence-policy reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    /// All configured persistence checks passed.
    Eligible,
    /// Canonical moderation selected the global deny state.
    GlobalDeny,
    /// Canonical moderation selected the owner tombstone state.
    OwnerTombstone,
    /// Canonical moderation selected the operator-local block state.
    LocalBlock,
    /// Canonical moderation selected quarantine.
    Quarantined,
    /// The object's canonical asset category is not enabled for persistence.
    AssetKindNotAllowed,
    /// The established review level does not meet the configured threshold.
    ReviewThresholdNotMet,
    /// Ordinary persistence may be eligible, but operator pinning is disabled.
    OperatorPinningDisabled,
}

impl ReasonCode {
    /// Return the canonical low-cardinality label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::GlobalDeny => "global_deny",
            Self::OwnerTombstone => "owner_tombstone",
            Self::LocalBlock => "local_block",
            Self::Quarantined => "quarantined",
            Self::AssetKindNotAllowed => "asset_kind_not_allowed",
            Self::ReviewThresholdNotMet => "review_threshold_not_met",
            Self::OperatorPinningDisabled => "operator_pinning_disabled",
        }
    }
}

/// Deterministic persistence-policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// Resulting policy posture.
    pub effect: Effect,
    /// Stable reason for the selected posture.
    pub reason: ReasonCode,
}

impl Decision {
    /// Whether the decision permits the requested persistence behavior.
    #[must_use]
    pub const fn permits_persistence(self) -> bool {
        self.effect.permits_persistence()
    }

    const fn eligible() -> Self {
        Self {
            effect: Effect::Eligible,
            reason: ReasonCode::Eligible,
        }
    }

    const fn ineligible(reason: ReasonCode) -> Self {
        Self {
            effect: Effect::Ineligible,
            reason,
        }
    }
}

/// Declarative persistence eligibility policy.
///
/// Safe defaults are deliberately fail-closed:
///
/// - no asset categories are enabled;
/// - moderation approval is required;
/// - operator pinning is disabled.
///
/// This type decides eligibility only. It does not assert that durable bytes
/// exist and does not perform a persistence-state transition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    required_review: ReviewLevel,
    allowed_asset_kinds: BTreeSet<AssetKind>,
    allow_operator_pinning: bool,
}

impl Policy {
    /// Evaluate persistence eligibility for one exact object.
    ///
    /// Canonical moderation is evaluated first. Global deny, owner tombstone,
    /// local block, and quarantine therefore defeat category approval, review
    /// approval, and operator pinning.
    #[must_use]
    pub fn evaluate(
        &self,
        moderation: &ModerationPolicy,
        object: &B3Id,
        asset_kind: AssetKind,
        review: ReviewLevel,
        intent: Intent,
    ) -> Decision {
        match moderation.evaluate(object).reason {
            ModerationReasonCode::GlobalDeny => {
                return Decision::ineligible(ReasonCode::GlobalDeny);
            }
            ModerationReasonCode::OwnerTombstone => {
                return Decision::ineligible(ReasonCode::OwnerTombstone);
            }
            ModerationReasonCode::LocalBlock => {
                return Decision::ineligible(ReasonCode::LocalBlock);
            }
            ModerationReasonCode::Quarantined => {
                return Decision::ineligible(ReasonCode::Quarantined);
            }
            ModerationReasonCode::NoRule | ModerationReasonCode::LocalAllow => {}
        }

        if !self.allowed_asset_kinds.contains(&asset_kind) {
            return Decision::ineligible(ReasonCode::AssetKindNotAllowed);
        }

        if !review.meets(self.required_review) {
            return Decision::ineligible(ReasonCode::ReviewThresholdNotMet);
        }

        if matches!(intent, Intent::Pin) && !self.allow_operator_pinning {
            return Decision::ineligible(ReasonCode::OperatorPinningDisabled);
        }

        Decision::eligible()
    }

    /// Configured minimum review level.
    #[must_use]
    pub const fn required_review(&self) -> ReviewLevel {
        self.required_review
    }

    /// Change the minimum review level.
    pub fn set_required_review(&mut self, required: ReviewLevel) {
        self.required_review = required;
    }

    /// Enable one canonical asset category.
    #[must_use]
    pub fn allow_asset_kind(&mut self, asset_kind: AssetKind) -> bool {
        self.allowed_asset_kinds.insert(asset_kind)
    }

    /// Disable one canonical asset category.
    #[must_use]
    pub fn disallow_asset_kind(&mut self, asset_kind: &AssetKind) -> bool {
        self.allowed_asset_kinds.remove(asset_kind)
    }

    /// Whether one canonical asset category is enabled.
    #[must_use]
    pub fn permits_asset_kind(&self, asset_kind: AssetKind) -> bool {
        self.allowed_asset_kinds.contains(&asset_kind)
    }

    /// Number of enabled canonical asset categories.
    #[must_use]
    pub fn allowed_asset_kind_count(&self) -> usize {
        self.allowed_asset_kinds.len()
    }

    /// Whether policy permits explicit operator pinning.
    #[must_use]
    pub const fn operator_pinning_allowed(&self) -> bool {
        self.allow_operator_pinning
    }

    /// Enable or disable explicit operator pinning.
    pub fn set_operator_pinning_allowed(&mut self, allowed: bool) {
        self.allow_operator_pinning = allowed;
    }
}
