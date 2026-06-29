//! RO:WHAT — Strict Phase 4 Round 3 controlled internal bond enforcement DTOs.
//! RO:WHY — ECON/GOV: represent policy-gated internal reserve/release/capture without public staking or bridge scope.
//! RO:INTERACTS — bond DTOs, bond dispute DTOs, ron-ledger internal bond accounting, future svc-wallet explicit confirmation.
//! RO:INVARIANTS — DTO/validation only; ROC integer minor units; no wallet mutation; no one-step irreversible slash.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — enforcement DTOs grant no spend, staking, bridge, settlement, finality, liquidity, or client authority.
//! RO:TEST — tests/quickchain_phase4_bond_enforcement.rs.

use serde::{Deserialize, Serialize};

use super::{
    validate_chain_id, validate_epoch_id, validate_money_minor_units, validate_ref,
    validate_schema, validate_version, QuickChainResult, QuickChainValidationError,
};

/// Schema tag for one controlled internal bond enforcement intent.
pub const QUICKCHAIN_BOND_ENFORCEMENT_INTENT_SCHEMA: &str = "quickchain.bond-enforcement-intent.v1";

/// Schema tag for one controlled internal bond enforcement decision.
pub const QUICKCHAIN_BOND_ENFORCEMENT_DECISION_SCHEMA: &str =
    "quickchain.bond-enforcement-decision.v1";

/// Controlled internal-only bond enforcement kind.
///
/// These kinds are not public staking operations. They are narrow internal
/// transitions over already-bonded ROC after explicit policy/operator gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondEnforcementKindV1 {
    /// Move available bonded ROC into slash-reserved review state.
    ReserveSlash,
    /// Return slash-reserved ROC back to available bonded state.
    ReleaseSlashReserve,
    /// Capture only ROC that is already slash-reserved.
    CaptureSlashReserve,
}

/// Controlled internal bond enforcement decision status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondEnforcementDecisionStatusV1 {
    Accepted,
    Rejected,
}

/// Deterministic rejection code for controlled internal bond enforcement.
///
/// These codes are diagnostics and do not create public staking, liquidity,
/// bridge, settlement, finality, wallet, gateway, policy, index, cache, or
/// client authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondEnforcementRejectionCodeV1 {
    ChainEpochMismatch,
    UnknownBondAccount,
    ValidatorMismatch,
    OwnerAccountMismatch,
    AmountRequired,
    AmountMustBePositive,
    InsufficientBondAvailable,
    InsufficientSlashReserved,
    PolicyDecisionRequired,
    GovernanceApprovalRequired,
    OperatorConfirmationRequired,
    DisputeEvidenceRequired,
    OneStepIrreversibleSlashForbidden,
    PublicStakingForbidden,
    LiquidityForbidden,
    SilentMutationForbidden,
}

/// Controlled internal-only bond enforcement intent.
///
/// This is strict wire shape only. A valid DTO does not itself move ROC. The
/// ledger model must still enforce conservation, and service-facing mutation
/// must still flow through explicit wallet/ledger authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondEnforcementIntentV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub enforcement_id: String,
    pub idempotency_key: String,
    pub bond_account_id: String,
    pub validator_id: String,
    pub actor_ref: String,
    pub kind: QuickChainBondEnforcementKindV1,
    pub amount_minor: String,
    pub dispute_id: String,
    pub evidence_id: String,
    pub policy_decision_ref: String,
    pub governance_approval_ref: Option<String>,
    pub operator_confirmation_ref: String,
}

impl QuickChainBondEnforcementIntentV1 {
    /// Validate controlled internal bond enforcement shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainBondEnforcementIntentV1.schema",
            &self.schema,
            QUICKCHAIN_BOND_ENFORCEMENT_INTENT_SCHEMA,
        )?;
        validate_version("QuickChainBondEnforcementIntentV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("enforcement_id", &self.enforcement_id)?;
        validate_ref("idempotency_key", &self.idempotency_key)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("actor_ref", &self.actor_ref)?;
        validate_ref("dispute_id", &self.dispute_id)?;
        validate_ref("evidence_id", &self.evidence_id)?;
        validate_ref("policy_decision_ref", &self.policy_decision_ref)?;
        validate_ref("operator_confirmation_ref", &self.operator_confirmation_ref)?;

        if let Some(governance_approval_ref) = self.governance_approval_ref.as_deref() {
            validate_ref("governance_approval_ref", governance_approval_ref)?;
        }

        require_positive_minor("amount_minor", &self.amount_minor)?;

        match self.kind {
            QuickChainBondEnforcementKindV1::ReserveSlash
            | QuickChainBondEnforcementKindV1::CaptureSlashReserve => {
                if self.governance_approval_ref.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "governance_approval_ref",
                        reason: "reserve/capture enforcement requires explicit governance approval",
                    });
                }
            }
            QuickChainBondEnforcementKindV1::ReleaseSlashReserve => {}
        }

        Ok(())
    }
}

/// Controlled internal bond enforcement decision snapshot.
///
/// The resulting component fields must conserve exactly:
///
/// `resulting_locked = resulting_available + resulting_pending_unlock + resulting_slash_reserved`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondEnforcementDecisionV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub enforcement_id: String,
    pub bond_account_id: String,
    pub validator_id: String,
    pub kind: QuickChainBondEnforcementKindV1,
    pub status: QuickChainBondEnforcementDecisionStatusV1,
    pub rejection_code: Option<QuickChainBondEnforcementRejectionCodeV1>,
    pub amount_minor: String,
    pub resulting_locked_minor: String,
    pub resulting_available_to_unlock_minor: String,
    pub resulting_pending_unlock_minor: String,
    pub resulting_slash_reserved_minor: String,
    pub account_sequence: u64,
}

impl QuickChainBondEnforcementDecisionV1 {
    /// Validate enforcement decision shape and conservative component math.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainBondEnforcementDecisionV1.schema",
            &self.schema,
            QUICKCHAIN_BOND_ENFORCEMENT_DECISION_SCHEMA,
        )?;
        validate_version("QuickChainBondEnforcementDecisionV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("enforcement_id", &self.enforcement_id)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        require_positive_minor("amount_minor", &self.amount_minor)?;

        let locked = parse_minor("resulting_locked_minor", &self.resulting_locked_minor)?;
        let available = parse_minor(
            "resulting_available_to_unlock_minor",
            &self.resulting_available_to_unlock_minor,
        )?;
        let pending = parse_minor(
            "resulting_pending_unlock_minor",
            &self.resulting_pending_unlock_minor,
        )?;
        let reserved = parse_minor(
            "resulting_slash_reserved_minor",
            &self.resulting_slash_reserved_minor,
        )?;

        let components = available
            .checked_add(pending)
            .and_then(|value| value.checked_add(reserved))
            .ok_or(QuickChainValidationError::InvalidField {
                field: "resulting_locked_minor",
                reason: "resulting bond components overflowed u128",
            })?;

        if components != locked {
            return Err(QuickChainValidationError::InvalidField {
                field: "resulting_locked_minor",
                reason:
                    "resulting locked amount must equal available + pending_unlock + slash_reserved",
            });
        }

        if self.account_sequence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "account_sequence",
                reason: "enforcement decision account sequence must be greater than zero",
            });
        }

        match self.status {
            QuickChainBondEnforcementDecisionStatusV1::Accepted => {
                if self.rejection_code.is_some() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "accepted enforcement decision cannot carry a rejection code",
                    });
                }
            }
            QuickChainBondEnforcementDecisionStatusV1::Rejected => {
                if self.rejection_code.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason:
                            "rejected enforcement decision requires deterministic rejection code",
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

fn require_positive_minor(field: &'static str, value: &str) -> QuickChainResult<u128> {
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
