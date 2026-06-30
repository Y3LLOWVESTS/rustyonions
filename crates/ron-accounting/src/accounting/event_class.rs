//! RO:WHAT — Internal ROC event-class decisions for accounting anti-farming boundaries.
//! RO:WHY — ECON/SEC: raw usage must be classified before it can become reward-planning material; raw engagement must never directly mint or allocate ROC.
//! RO:INTERACTS — `MetricKind`, reward projection, `svc-rewarder` capped planning inputs.
//! RO:INVARIANTS — classification is report/planning metadata only; no balance truth, no receipt truth, no wallet or ledger side effect.
//! RO:METRICS — callers may count rejected/isolated classes.
//! RO:CONFIG — no runtime config; later policies may tune which metering facts become proof candidates.
//! RO:SECURITY — analytics and unfunded ad-style events are isolated from protocol rewards.
//! RO:TEST — `tests/internal_roc_beta_phase5_event_class_antifarming.rs`.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::MetricKind,
    errors::{Error, Result},
    normalize::normalize_component,
};

/// Internal ROC event classes used for beta accounting/reward boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalRocEventClass {
    /// Backend wallet/ledger accepted economic truth only.
    EconomicReceipt,
    /// Usage/metering facts only; not payout truth.
    Metering,
    /// Candidate material that requires verification/caps/policy before reward planning.
    ProofEligible,
    /// Explicit payer-authorized budget lane; not protocol emission from engagement.
    AdBudgeted,
    /// Product analytics only; never protocol payout material.
    AnalyticsOnly,
}

impl InternalRocEventClass {
    /// Stable lowercase label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EconomicReceipt => "economic_receipt",
            Self::Metering => "metering",
            Self::ProofEligible => "proof_eligible",
            Self::AdBudgeted => "ad_budgeted",
            Self::AnalyticsOnly => "analytics_only",
        }
    }
}

/// Read-only classification result for one accounting event kind.
///
/// This is not a receipt, balance, payout, entitlement, wallet command, or ledger command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocEventClassDecision {
    /// Classification.
    pub event_class: InternalRocEventClass,
    /// Whether this may become reward-planning material after downstream gates.
    pub reward_planning_candidate: bool,
    /// Whether verification/challenge/caps are required before reward planning.
    pub requires_verification: bool,
    /// Whether an explicit payer/sponsor budget is required.
    pub requires_explicit_budget: bool,
    /// Whether only backend wallet/ledger sources may create this class.
    pub requires_wallet_ledger_source: bool,
    /// Must remain false: accounting classification never directly allocates ROC.
    pub direct_protocol_roc_allocation: bool,
    /// Must remain false: accounting classification never mutates wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting classification never mutates ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting classification never creates receipt truth.
    pub receipt_truth: bool,
    /// Must remain false: accounting classification never creates balance truth.
    pub balance_truth: bool,
}

impl InternalRocEventClassDecision {
    /// Validate no-authority flags and class-specific requirements.
    ///
    /// # Errors
    ///
    /// Returns `Error::SchemaViolation` if a decision claims economic authority.
    pub fn validate(&self) -> Result<()> {
        if self.direct_protocol_roc_allocation {
            return Err(Error::schema(
                "event classification must not directly allocate protocol ROC",
            ));
        }
        if self.wallet_side_effect {
            return Err(Error::schema(
                "event classification must not claim wallet side effect",
            ));
        }
        if self.ledger_side_effect {
            return Err(Error::schema(
                "event classification must not claim ledger side effect",
            ));
        }
        if self.receipt_truth {
            return Err(Error::schema(
                "event classification must not claim receipt truth",
            ));
        }
        if self.balance_truth {
            return Err(Error::schema(
                "event classification must not claim balance truth",
            ));
        }

        match self.event_class {
            InternalRocEventClass::EconomicReceipt => {
                if !self.requires_wallet_ledger_source
                    || self.reward_planning_candidate
                    || !self.requires_verification
                    || self.requires_explicit_budget
                {
                    return Err(Error::schema(
                        "economic_receipt class must require wallet/ledger source and remain outside raw reward planning",
                    ));
                }
            }
            InternalRocEventClass::Metering => {
                if self.reward_planning_candidate
                    || self.requires_wallet_ledger_source
                    || self.requires_explicit_budget
                {
                    return Err(Error::schema(
                        "metering class must not become direct reward-planning material",
                    ));
                }
            }
            InternalRocEventClass::ProofEligible => {
                if !self.reward_planning_candidate
                    || !self.requires_verification
                    || self.requires_wallet_ledger_source
                    || self.requires_explicit_budget
                {
                    return Err(Error::schema(
                        "proof_eligible class must require verification/caps before reward planning",
                    ));
                }
            }
            InternalRocEventClass::AdBudgeted => {
                if !self.requires_explicit_budget
                    || !self.requires_verification
                    || self.requires_wallet_ledger_source
                {
                    return Err(Error::schema(
                        "ad_budgeted class must require explicit payer budget and verification",
                    ));
                }
            }
            InternalRocEventClass::AnalyticsOnly => {
                if self.reward_planning_candidate
                    || self.requires_verification
                    || self.requires_explicit_budget
                    || self.requires_wallet_ledger_source
                {
                    return Err(Error::schema(
                        "analytics_only class must remain isolated from reward planning",
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Classify a metering `MetricKind` for Internal ROC beta reward boundaries.
///
/// `BytesStored`, `BytesServed`, and `UptimeSeconds` are the existing narrow
/// proof-candidate service metrics. They still require verification/caps/policy
/// before rewarder planning and never allocate ROC directly.
#[must_use]
pub fn classify_metric_for_internal_roc(metric: &MetricKind) -> InternalRocEventClassDecision {
    let decision = match metric {
        MetricKind::BytesStored | MetricKind::BytesServed | MetricKind::UptimeSeconds => {
            InternalRocEventClassDecision {
                event_class: InternalRocEventClass::ProofEligible,
                reward_planning_candidate: true,
                requires_verification: true,
                requires_explicit_budget: false,
                requires_wallet_ledger_source: false,
                direct_protocol_roc_allocation: false,
                wallet_side_effect: false,
                ledger_side_effect: false,
                receipt_truth: false,
                balance_truth: false,
            }
        }
        MetricKind::PinSeconds | MetricKind::RequestOk | MetricKind::CpuUnits => {
            InternalRocEventClassDecision {
                event_class: InternalRocEventClass::Metering,
                reward_planning_candidate: false,
                requires_verification: false,
                requires_explicit_budget: false,
                requires_wallet_ledger_source: false,
                direct_protocol_roc_allocation: false,
                wallet_side_effect: false,
                ledger_side_effect: false,
                receipt_truth: false,
                balance_truth: false,
            }
        }
        MetricKind::Custom(value) => classify_custom_metric(value),
    };

    debug_assert!(decision.validate().is_ok());
    decision
}

/// Build a valid `economic_receipt` decision only for backend wallet/ledger source labels.
///
/// # Errors
///
/// Returns `Error::SchemaViolation` when a raw usage source attempts to claim
/// economic receipt classification.
pub fn economic_receipt_decision_from_source(
    source: &str,
) -> Result<InternalRocEventClassDecision> {
    let source = source.trim();

    if !(source.starts_with("svc-wallet:")
        || source.starts_with("ron-ledger:")
        || source.starts_with("wallet:")
        || source.starts_with("ledger:"))
    {
        return Err(Error::schema(
            "economic_receipt classification requires wallet/ledger source",
        ));
    }

    let decision = InternalRocEventClassDecision {
        event_class: InternalRocEventClass::EconomicReceipt,
        reward_planning_candidate: false,
        requires_verification: true,
        requires_explicit_budget: false,
        requires_wallet_ledger_source: true,
        direct_protocol_roc_allocation: false,
        wallet_side_effect: false,
        ledger_side_effect: false,
        receipt_truth: false,
        balance_truth: false,
    };
    decision.validate()?;
    Ok(decision)
}

fn classify_custom_metric(value: &str) -> InternalRocEventClassDecision {
    let normalized = normalize_component(value).replace('-', "_");

    if looks_ad_budgeted(&normalized) {
        return InternalRocEventClassDecision {
            event_class: InternalRocEventClass::AdBudgeted,
            reward_planning_candidate: false,
            requires_verification: true,
            requires_explicit_budget: true,
            requires_wallet_ledger_source: false,
            direct_protocol_roc_allocation: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            receipt_truth: false,
            balance_truth: false,
        };
    }

    if looks_raw_engagement(&normalized) {
        return InternalRocEventClassDecision {
            event_class: InternalRocEventClass::AnalyticsOnly,
            reward_planning_candidate: false,
            requires_verification: false,
            requires_explicit_budget: false,
            requires_wallet_ledger_source: false,
            direct_protocol_roc_allocation: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            receipt_truth: false,
            balance_truth: false,
        };
    }

    InternalRocEventClassDecision {
        event_class: InternalRocEventClass::AnalyticsOnly,
        reward_planning_candidate: false,
        requires_verification: false,
        requires_explicit_budget: false,
        requires_wallet_ledger_source: false,
        direct_protocol_roc_allocation: false,
        wallet_side_effect: false,
        ledger_side_effect: false,
        receipt_truth: false,
        balance_truth: false,
    }
}

fn looks_ad_budgeted(value: &str) -> bool {
    value.contains("ad_")
        || value.contains("ads_")
        || value.contains("sponsored")
        || value.contains("campaign")
}

fn looks_raw_engagement(value: &str) -> bool {
    value.contains("view")
        || value.contains("click")
        || value.contains("like")
        || value.contains("share")
        || value.contains("scroll")
        || value.contains("hover")
        || value.contains("impression")
        || value.contains("engagement")
        || value.contains("comment")
        || value.contains("watch")
        || value.contains("play")
}
