//! RO:WHAT — Internal ROC accounting snapshot and reward-plan reference DTOs.
//! RO:WHY — ECON/GOV: Phase 3 needs deterministic planning references without wallet or ledger mutation.
//! RO:INTERACTS — event_class.rs, ron-accounting snapshots, svc-rewarder plans, ron-policy gates.
//! RO:INVARIANTS — references only; no balances, receipts, finality, payout execution, bridge, or staking runtime.
//! RO:METRICS — none.
//! RO:CONFIG — downstream economics config version may be referenced by policy, not executed here.
//! RO:SECURITY — raw engagement classes cannot become payout authority or receipt truth through these DTOs.
//! RO:TEST — tests/internal_roc_beta_phase3_accounting_reward_plan_dto.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    event_class::QuickChainEventClassV1, money::validate_quickchain_minor_units, validate_chain_id,
    validate_ref, validate_schema, validate_version, QuickChainResult, QuickChainValidationError,
};

pub const QUICKCHAIN_ACCOUNTING_SNAPSHOT_REFERENCE_SCHEMA: &str =
    "quickchain.accounting-snapshot-reference.v1";
pub const QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA: &str = "quickchain.reward-plan-reference.v1";

/// Deterministic reference to a sealed accounting snapshot.
///
/// This is not balance truth. It is a compact reference that can be handed to
/// reward planning after accounting has sealed and counted classified events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainAccountingSnapshotReferenceV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub snapshot_id: String,
    pub snapshot_root: ContentId,
    pub window_started_at_ms: u64,
    pub window_ended_at_ms: u64,
    pub sealed_at_ms: u64,
    pub source_event_count: u64,
    pub economic_receipt_count: u64,
    pub metering_count: u64,
    pub proof_eligible_count: u64,
    pub ad_budgeted_count: u64,
    pub analytics_only_count: u64,
}

impl QuickChainAccountingSnapshotReferenceV1 {
    /// Validate reference shape and deterministic event-class count accounting.
    ///
    /// Validation does not ingest events, create balances, create receipts, or
    /// authorize payouts.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainAccountingSnapshotReferenceV1.schema",
            &self.schema,
            QUICKCHAIN_ACCOUNTING_SNAPSHOT_REFERENCE_SCHEMA,
        )?;
        validate_version(
            "QuickChainAccountingSnapshotReferenceV1.version",
            self.version,
        )?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("snapshot_id", &self.snapshot_id)?;

        if self.window_started_at_ms == 0
            || self.window_ended_at_ms == 0
            || self.sealed_at_ms == 0
            || self.window_ended_at_ms < self.window_started_at_ms
            || self.sealed_at_ms < self.window_ended_at_ms
        {
            return Err(QuickChainValidationError::InvalidTimestampOrder {
                field: "snapshot_window",
            });
        }

        let class_count = checked_event_count_sum(self)?;
        if class_count != self.source_event_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "source_event_count",
                reason: "must equal the sum of all event-class counts",
            });
        }

        Ok(())
    }
}

/// Deterministic reference to a non-mutating reward plan.
///
/// This is planning material only. It is not a receipt, balance, entitlement,
/// finality marker, payout execution, bridge mint/burn, staking position, or
/// exchange-facing artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainRewardPlanReferenceV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub plan_id: String,
    pub plan_root: ContentId,
    pub snapshot_id: String,
    pub snapshot_root: ContentId,
    pub source_event_class: QuickChainEventClassV1,
    pub planned_total_minor: String,
    pub payout_candidate_count: u64,
    pub capped_by_policy: bool,
    #[serde(default)]
    pub verification_ref: Option<String>,
    #[serde(default)]
    pub funding_budget_ref: Option<String>,
    pub produced_at_ms: u64,
}

impl QuickChainRewardPlanReferenceV1 {
    /// Validate reward-plan reference shape and Phase 3 event-class boundaries.
    ///
    /// Validation does not execute the plan and does not create wallet or ledger
    /// receipt truth.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainRewardPlanReferenceV1.schema",
            &self.schema,
            QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA,
        )?;
        validate_version("QuickChainRewardPlanReferenceV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("plan_id", &self.plan_id)?;
        validate_ref("snapshot_id", &self.snapshot_id)?;
        validate_quickchain_minor_units("planned_total_minor", &self.planned_total_minor)?;

        if let Some(verification_ref) = &self.verification_ref {
            validate_ref("verification_ref", verification_ref)?;
        }

        if let Some(funding_budget_ref) = &self.funding_budget_ref {
            validate_ref("funding_budget_ref", funding_budget_ref)?;
        }

        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "must be greater than zero",
            });
        }

        self.validate_planning_material()
    }

    fn validate_planning_material(&self) -> QuickChainResult<()> {
        let has_planned_value = self.planned_total_minor != "0";
        let has_candidates = self.payout_candidate_count > 0;

        if has_planned_value != has_candidates {
            return Err(QuickChainValidationError::InvalidField {
                field: "payout_candidate_count",
                reason: "must be nonzero exactly when planned_total_minor is nonzero",
            });
        }

        if has_planned_value && !self.capped_by_policy {
            return Err(QuickChainValidationError::InvalidField {
                field: "capped_by_policy",
                reason: "must be true for any nonzero reward plan",
            });
        }

        match self.source_event_class {
            QuickChainEventClassV1::EconomicReceipt => Ok(()),

            QuickChainEventClassV1::Metering | QuickChainEventClassV1::AnalyticsOnly => {
                if has_planned_value || has_candidates {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "source_event_class",
                        reason:
                            "metering and analytics_only cannot directly become payout material",
                    });
                }

                Ok(())
            }

            QuickChainEventClassV1::ProofEligible => {
                if (has_planned_value || has_candidates) && self.verification_ref.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "verification_ref",
                        reason: "proof_eligible payout planning requires verification evidence",
                    });
                }

                Ok(())
            }

            QuickChainEventClassV1::AdBudgeted => {
                if (has_planned_value || has_candidates) && self.funding_budget_ref.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "funding_budget_ref",
                        reason: "ad_budgeted payout planning requires explicit budget evidence",
                    });
                }

                Ok(())
            }
        }
    }
}

fn checked_event_count_sum(
    snapshot: &QuickChainAccountingSnapshotReferenceV1,
) -> QuickChainResult<u64> {
    snapshot
        .economic_receipt_count
        .checked_add(snapshot.metering_count)
        .and_then(|value| value.checked_add(snapshot.proof_eligible_count))
        .and_then(|value| value.checked_add(snapshot.ad_budgeted_count))
        .and_then(|value| value.checked_add(snapshot.analytics_only_count))
        .ok_or(QuickChainValidationError::InvalidField {
            field: "source_event_count",
            reason: "event-class count sum overflowed",
        })
}
