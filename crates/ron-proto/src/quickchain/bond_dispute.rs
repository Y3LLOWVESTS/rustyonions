//! RO:WHAT — Strict Phase 4 Round 2 bond dispute/challenge DTOs.
//! RO:WHY — ECON/GOV: simulate slash consequences with challenge/appeal windows before live enforcement.
//! RO:INTERACTS — bond DTOs, slash evidence DTOs, ron-ledger replayable dispute simulation.
//! RO:INVARIANTS — DTO/validation only; epoch windows explicit; no one-step irreversible slash; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — dispute events grant no spend, slash, finality, bridge, staking, liquidity, or settlement authority.
//! RO:TEST — tests/quickchain_phase4_bond_dispute.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    validate_chain_id, validate_epoch_id, validate_money_minor_units, validate_ref,
    validate_schema, validate_version, QuickChainResult, QuickChainValidationError,
};

/// Schema tag for one disputed-bond state snapshot.
pub const QUICKCHAIN_BOND_DISPUTE_SCHEMA: &str = "quickchain.bond-dispute.v1";

/// Schema tag for one replayable disputed-bond event.
pub const QUICKCHAIN_BOND_DISPUTE_EVENT_SCHEMA: &str = "quickchain.bond-dispute-event.v1";

/// Disputed-bond lifecycle status.
///
/// These states simulate adjudication flow only. They do not authorize a live
/// slash, burn, capture, release, or settlement mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondDisputeStatusV1 {
    /// Challenge is open and still inside its explicit challenge window.
    ChallengeOpen,
    /// Bond amount is frozen for review while appeal is pending.
    FrozenPendingAppeal,
    /// Appeal has been submitted inside the appeal window.
    AppealOpen,
    /// Dispute resolved without simulated slash.
    ResolvedNoSlash,
    /// A requested irreversible slash was rejected by simulation policy.
    ResolvedSlashRejected,
}

/// Replayable disputed-bond event kind.
///
/// Every event is simulation-only in Phase 4 Round 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondDisputeEventKindV1 {
    /// Move disputed funds into a hold-like frozen amount for appeal review.
    FreezePendingAppeal,
    /// Submit an appeal inside the appeal window.
    SubmitAppeal,
    /// Resolve the dispute without slash.
    ResolveNoSlash,
    /// Deterministically reject a one-step irreversible slash attempt.
    RejectIrreversibleSlash,
}

/// Deterministic dispute rejection reason used by simulation artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondDisputeRejectionCodeV1 {
    ChallengeWindowClosed,
    AppealWindowClosed,
    AppealWindowRequired,
    DisputeAlreadyTerminal,
    EvidenceInvalid,
    AmountExceedsDisputedBond,
    OneStepIrreversibleSlashForbidden,
    GovernanceApprovalRequired,
    SequenceMustIncrease,
}

/// Inclusive epoch window for challenge or appeal simulation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondDisputeWindowV1 {
    pub start_epoch: u64,
    pub end_epoch: u64,
}

impl QuickChainBondDisputeWindowV1 {
    /// True when `epoch` falls inside this inclusive window.
    #[must_use]
    pub const fn contains_epoch(&self, epoch: u64) -> bool {
        self.start_epoch <= epoch && epoch <= self.end_epoch
    }
}

/// Disputed-bond state snapshot.
///
/// This is a replayable simulation state, not bond truth and not slash truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondDisputeV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub dispute_id: String,
    pub bond_account_id: String,
    pub validator_id: String,
    pub validator_set_hash: ContentId,
    pub evidence_id: String,
    pub challenger_ref: String,
    pub status: QuickChainBondDisputeStatusV1,
    pub challenge_window: QuickChainBondDisputeWindowV1,
    pub appeal_window: Option<QuickChainBondDisputeWindowV1>,
    pub disputed_amount_minor: String,
    pub frozen_amount_minor: String,
    pub last_dispute_sequence: u64,
}

impl QuickChainBondDisputeV1 {
    /// Validate dispute snapshot shape and status/window consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainBondDisputeV1.schema",
            &self.schema,
            QUICKCHAIN_BOND_DISPUTE_SCHEMA,
        )?;
        validate_version("QuickChainBondDisputeV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("dispute_id", &self.dispute_id)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("evidence_id", &self.evidence_id)?;
        validate_ref("challenger_ref", &self.challenger_ref)?;
        validate_challenge_window(&self.challenge_window)?;

        if let Some(window) = &self.appeal_window {
            validate_appeal_window(window)?;
        }

        let disputed = parse_positive_minor("disputed_amount_minor", &self.disputed_amount_minor)?;
        let frozen = parse_minor("frozen_amount_minor", &self.frozen_amount_minor)?;

        if frozen > disputed {
            return Err(QuickChainValidationError::InvalidField {
                field: "frozen_amount_minor",
                reason: "frozen amount cannot exceed disputed amount",
            });
        }

        if self.last_dispute_sequence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "last_dispute_sequence",
                reason: "dispute sequence must be greater than zero",
            });
        }

        match self.status {
            QuickChainBondDisputeStatusV1::ChallengeOpen => {
                reject_appeal_window(self.appeal_window.as_ref())?;
                require_zero("frozen_amount_minor", frozen)?;
            }
            QuickChainBondDisputeStatusV1::FrozenPendingAppeal
            | QuickChainBondDisputeStatusV1::AppealOpen => {
                require_appeal_window(self.appeal_window.as_ref())?;
                require_positive_value("frozen_amount_minor", frozen)?;
            }
            QuickChainBondDisputeStatusV1::ResolvedNoSlash
            | QuickChainBondDisputeStatusV1::ResolvedSlashRejected => {
                require_zero("frozen_amount_minor", frozen)?;
            }
        }

        Ok(())
    }

    /// True when this dispute is terminal and must reject further events.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            QuickChainBondDisputeStatusV1::ResolvedNoSlash
                | QuickChainBondDisputeStatusV1::ResolvedSlashRejected
        )
    }
}

/// Replayable disputed-bond event.
///
/// This event is simulation input only. It does not mutate wallet/ledger truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondDisputeEventV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub dispute_id: String,
    pub bond_account_id: String,
    pub validator_id: String,
    pub event_sequence: u64,
    pub event_kind: QuickChainBondDisputeEventKindV1,
    pub actor_ref: String,
    pub occurred_epoch: u64,
    pub amount_minor: Option<String>,
    pub appeal_window: Option<QuickChainBondDisputeWindowV1>,
    pub rejection_code: Option<QuickChainBondDisputeRejectionCodeV1>,
}

impl QuickChainBondDisputeEventV1 {
    /// Validate event shape and kind-specific constraints.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainBondDisputeEventV1.schema",
            &self.schema,
            QUICKCHAIN_BOND_DISPUTE_EVENT_SCHEMA,
        )?;
        validate_version("QuickChainBondDisputeEventV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("dispute_id", &self.dispute_id)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("actor_ref", &self.actor_ref)?;

        if self.event_sequence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "event_sequence",
                reason: "event sequence must be greater than zero",
            });
        }

        if self.occurred_epoch == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "occurred_epoch",
                reason: "occurred epoch must be greater than zero",
            });
        }

        if let Some(window) = &self.appeal_window {
            validate_appeal_window(window)?;
        }

        match self.event_kind {
            QuickChainBondDisputeEventKindV1::FreezePendingAppeal => {
                require_positive_amount(self.amount_minor.as_deref())?;
                require_appeal_window(self.appeal_window.as_ref())?;
                reject_rejection_code(self.rejection_code)?;
            }
            QuickChainBondDisputeEventKindV1::SubmitAppeal => {
                reject_amount(self.amount_minor.as_deref())?;
                reject_appeal_window(self.appeal_window.as_ref())?;
                reject_rejection_code(self.rejection_code)?;
            }
            QuickChainBondDisputeEventKindV1::ResolveNoSlash => {
                reject_amount(self.amount_minor.as_deref())?;
                reject_appeal_window(self.appeal_window.as_ref())?;
                reject_rejection_code(self.rejection_code)?;
            }
            QuickChainBondDisputeEventKindV1::RejectIrreversibleSlash => {
                reject_amount(self.amount_minor.as_deref())?;
                reject_appeal_window(self.appeal_window.as_ref())?;

                if self.rejection_code
                    != Some(QuickChainBondDisputeRejectionCodeV1::OneStepIrreversibleSlashForbidden)
                {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "irreversible slash rejection requires explicit forbidden code",
                    });
                }
            }
        }

        Ok(())
    }
}

fn validate_chain_epoch(chain_id: &str, epoch_id: &str) -> QuickChainResult<()> {
    validate_chain_id(chain_id)?;
    validate_epoch_id(epoch_id)
}

fn validate_challenge_window(window: &QuickChainBondDisputeWindowV1) -> QuickChainResult<()> {
    validate_window("challenge_window", window)
}

fn validate_appeal_window(window: &QuickChainBondDisputeWindowV1) -> QuickChainResult<()> {
    validate_window("appeal_window", window)
}

fn validate_window(
    field: &'static str,
    window: &QuickChainBondDisputeWindowV1,
) -> QuickChainResult<()> {
    if window.start_epoch == 0 || window.end_epoch == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "window epochs must be greater than zero",
        });
    }

    if window.end_epoch < window.start_epoch {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "window end must be greater than or equal to start",
        });
    }

    Ok(())
}

fn require_appeal_window(value: Option<&QuickChainBondDisputeWindowV1>) -> QuickChainResult<()> {
    if value.is_none() {
        return Err(QuickChainValidationError::InvalidField {
            field: "appeal_window",
            reason: "appeal window is required for frozen/appeal dispute states",
        });
    }

    Ok(())
}

fn reject_appeal_window(value: Option<&QuickChainBondDisputeWindowV1>) -> QuickChainResult<()> {
    if value.is_some() {
        return Err(QuickChainValidationError::InvalidField {
            field: "appeal_window",
            reason: "appeal window is not valid for this dispute/event state",
        });
    }

    Ok(())
}

fn reject_rejection_code(
    value: Option<QuickChainBondDisputeRejectionCodeV1>,
) -> QuickChainResult<()> {
    if value.is_some() {
        return Err(QuickChainValidationError::InvalidField {
            field: "rejection_code",
            reason: "rejection code is not valid for this event kind",
        });
    }

    Ok(())
}

fn require_positive_amount(value: Option<&str>) -> QuickChainResult<u128> {
    let value = value.ok_or(QuickChainValidationError::InvalidField {
        field: "amount_minor",
        reason: "amount is required for this event kind",
    })?;

    parse_positive_minor("amount_minor", value)
}

fn reject_amount(value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Err(QuickChainValidationError::InvalidField {
            field: "amount_minor",
            reason: "amount is not valid for this event kind",
        });
    }

    Ok(())
}

fn parse_positive_minor(field: &'static str, value: &str) -> QuickChainResult<u128> {
    let parsed = parse_minor(field, value)?;

    if parsed == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "amount must be greater than zero",
        });
    }

    Ok(parsed)
}

fn parse_minor(field: &'static str, value: &str) -> QuickChainResult<u128> {
    validate_money_minor_units(field, value)?;

    value
        .parse::<u128>()
        .map_err(|_| QuickChainValidationError::InvalidMoney {
            field,
            reason: "must fit u128",
        })
}

fn require_zero(field: &'static str, value: u128) -> QuickChainResult<()> {
    if value != 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "amount must be zero for this state",
        });
    }

    Ok(())
}

fn require_positive_value(field: &'static str, value: u128) -> QuickChainResult<()> {
    if value == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "amount must be greater than zero for this state",
        });
    }

    Ok(())
}
