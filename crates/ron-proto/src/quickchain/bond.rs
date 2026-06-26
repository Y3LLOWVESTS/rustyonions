//! RO:WHAT — Strict Phase 4 Round 1 bond DTOs for future bonded validator modeling.
//! RO:WHY — ECON/GOV: model internal bond intent, bond account, slash evidence, and lifecycle decisions without live slashing.
//! RO:INTERACTS — validator_set, validator_lifecycle, future svc-wallet explicit confirmation, ron-ledger no-op accounting model.
//! RO:INVARIANTS — DTO/validation only; integer minor-unit strings; no wallet mutation; no public staking/liquidity; no automatic slashing.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — bond/slash fields are shape data only and grant no spend, unlock, settlement, bridge, staking, or slashing authority.
//! RO:TEST — tests/quickchain_phase4_bond_dto.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    validate_chain_id, validate_epoch_id, validate_money_minor_units, validate_ref,
    validate_schema, validate_version, QuickChainResult, QuickChainValidationError,
};

/// Schema tag for one explicit validator bond intent artifact.
pub const QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA: &str = "quickchain.validator-bond-intent.v1";

/// Schema tag for one validator bond account snapshot DTO.
pub const QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA: &str = "quickchain.validator-bond-account.v1";

/// Schema tag for one slash evidence artifact.
///
/// This is evidence-only in Phase 4 Round 1. It does not authorize automatic
/// slash/capture/burn/release behavior.
pub const QUICKCHAIN_SLASH_EVIDENCE_SCHEMA: &str = "quickchain.slash-evidence.v1";

/// Schema tag for one read-only bond lifecycle decision.
pub const QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA: &str = "quickchain.bond-lifecycle-decision.v1";

/// The only asset accepted by Phase 4 Round 1 bond DTOs.
pub const QUICKCHAIN_BOND_ASSET_ROC: &str = "roc";

/// Explicit internal bond intent kind.
///
/// These intent classes are modeling inputs only. They do not create a public
/// staking market and do not bypass wallet confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondIntentKindV1 {
    /// Open a new internal validator bond account after explicit authorization.
    OpenBond,
    /// Add more explicitly authorized ROC to an existing bond account.
    IncreaseBond,
    /// Request a future unlock window for already-bonded ROC.
    RequestUnlock,
    /// Cancel a pending unlock request before it is released.
    CancelUnlockRequest,
}

/// Bond account lifecycle status.
///
/// This status is descriptive in Phase 4 Round 1. It is not spend authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondAccountStatusV1 {
    Active,
    UnlockPending,
    FrozenEvidenceOnly,
    Closed,
}

/// Evidence category for potential future bond challenge handling.
///
/// Evidence is inert in Phase 4 Round 1. It may be validated and displayed, but
/// it must not automatically slash, burn, capture, release, or move ROC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainSlashEvidenceKindV1 {
    ValidatorEquivocation,
    ReplayMismatch,
    Downtime,
    PolicyViolation,
    GovernanceFinding,
}

/// Bond lifecycle operation evaluated by a read-only/no-op model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondLifecycleOperationV1 {
    OpenBond,
    IncreaseBond,
    RequestUnlock,
    CancelUnlockRequest,
    EvaluateSlashEvidenceNoop,
}

/// Read-only bond lifecycle decision status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondLifecycleDecisionStatusV1 {
    Accepted,
    Rejected,
}

/// Deterministic bond lifecycle rejection code.
///
/// These codes are diagnostics. They do not grant finality, wallet mutation,
/// settlement, bridge, staking, liquidity, or slashing authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondLifecycleRejectionCodeV1 {
    ChainEpochMismatch,
    UnknownBondAccount,
    BondAccountAlreadyExists,
    ValidatorMismatch,
    OwnerAccountMismatch,
    AssetMismatch,
    AmountRequired,
    AmountMustBePositive,
    InsufficientOwnerAvailable,
    InsufficientBondAvailable,
    InsufficientPendingUnlock,
    BondComponentMismatch,
    AutomaticSlashingForbidden,
    PublicStakingForbidden,
    LiquidityForbidden,
    SilentMutationForbidden,
    EvidenceInvalid,
}

/// Explicit internal validator bond intent.
///
/// A valid intent is not enough to move money. The wallet front-door must still
/// provide explicit confirmation and the ledger must remain economic truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorBondIntentV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_id: String,
    pub bond_account_id: String,
    pub actor_account_id: String,
    pub intent_id: String,
    pub idempotency_key: String,
    pub kind: QuickChainBondIntentKindV1,
    pub asset: String,
    pub amount_minor: Option<String>,
    pub unlock_epoch_id: Option<String>,
    pub governance_approval_ref: Option<String>,
}

impl QuickChainValidatorBondIntentV1 {
    /// Validate explicit bond intent shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorBondIntentV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA,
        )?;
        validate_version("QuickChainValidatorBondIntentV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;
        validate_ref("actor_account_id", &self.actor_account_id)?;
        validate_ref("intent_id", &self.intent_id)?;
        validate_ref("idempotency_key", &self.idempotency_key)?;
        validate_bond_asset(&self.asset)?;
        validate_optional_ref(
            "governance_approval_ref",
            self.governance_approval_ref.as_deref(),
        )?;

        match self.kind {
            QuickChainBondIntentKindV1::OpenBond | QuickChainBondIntentKindV1::IncreaseBond => {
                require_positive_minor("amount_minor", self.amount_minor.as_deref())?;
                reject_unlock_epoch("unlock_epoch_id", self.unlock_epoch_id.as_deref())?;
            }
            QuickChainBondIntentKindV1::RequestUnlock => {
                require_positive_minor("amount_minor", self.amount_minor.as_deref())?;
                let unlock_epoch_id = self.unlock_epoch_id.as_deref().ok_or(
                    QuickChainValidationError::InvalidField {
                        field: "unlock_epoch_id",
                        reason: "unlock request requires an explicit unlock epoch",
                    },
                )?;
                validate_epoch_id(unlock_epoch_id)?;
            }
            QuickChainBondIntentKindV1::CancelUnlockRequest => {
                require_positive_minor("amount_minor", self.amount_minor.as_deref())?;
                reject_unlock_epoch("unlock_epoch_id", self.unlock_epoch_id.as_deref())?;
            }
        }

        Ok(())
    }
}

/// Validator bond account snapshot DTO.
///
/// This is a deterministic model snapshot, not wallet authority and not a public
/// staking account. Components must conserve exactly:
///
/// `locked = available_to_unlock + pending_unlock + slash_reserved`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorBondAccountV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_id: String,
    pub bond_account_id: String,
    pub owner_account_id: String,
    pub asset: String,
    pub locked_minor: String,
    pub available_to_unlock_minor: String,
    pub pending_unlock_minor: String,
    pub slash_reserved_minor: String,
    pub status: QuickChainBondAccountStatusV1,
    pub account_sequence: u64,
}

impl QuickChainValidatorBondAccountV1 {
    /// Validate bond account snapshot shape and conservative component math.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorBondAccountV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA,
        )?;
        validate_version("QuickChainValidatorBondAccountV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;
        validate_ref("owner_account_id", &self.owner_account_id)?;
        validate_bond_asset(&self.asset)?;

        let locked = parse_minor("locked_minor", &self.locked_minor)?;
        let available = parse_minor("available_to_unlock_minor", &self.available_to_unlock_minor)?;
        let pending = parse_minor("pending_unlock_minor", &self.pending_unlock_minor)?;
        let slash_reserved = parse_minor("slash_reserved_minor", &self.slash_reserved_minor)?;

        let components = available
            .checked_add(pending)
            .and_then(|value| value.checked_add(slash_reserved))
            .ok_or(QuickChainValidationError::InvalidField {
                field: "locked_minor",
                reason: "bond components overflowed u128",
            })?;

        if components != locked {
            return Err(QuickChainValidationError::InvalidField {
                field: "locked_minor",
                reason: "locked amount must equal available + pending_unlock + slash_reserved",
            });
        }

        if self.account_sequence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "account_sequence",
                reason: "bond account sequence must be greater than zero",
            });
        }

        Ok(())
    }
}

/// Evidence for a future slash/challenge process.
///
/// In Phase 4 Round 1, this DTO is inert evidence. It must not directly slash,
/// burn, capture, release, or transfer ROC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainSlashEvidenceV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_set_hash: ContentId,
    pub evidence_id: String,
    pub validator_id: String,
    pub evidence_kind: QuickChainSlashEvidenceKindV1,
    pub evidence_ref: String,
    pub submitter_ref: String,
    pub observed_at_ms: u64,
    pub recommended_freeze: bool,
    pub recommended_amount_minor: Option<String>,
}

impl QuickChainSlashEvidenceV1 {
    /// Validate slash evidence shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainSlashEvidenceV1.schema",
            &self.schema,
            QUICKCHAIN_SLASH_EVIDENCE_SCHEMA,
        )?;
        validate_version("QuickChainSlashEvidenceV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("evidence_id", &self.evidence_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("evidence_ref", &self.evidence_ref)?;
        validate_ref("submitter_ref", &self.submitter_ref)?;

        if self.observed_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "observed_at_ms",
                reason: "must be greater than zero",
            });
        }

        if let Some(amount) = self.recommended_amount_minor.as_deref() {
            parse_minor("recommended_amount_minor", amount)?;
        }

        Ok(())
    }
}

/// Read-only/no-op bond lifecycle decision DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondLifecycleDecisionV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub epoch_id: String,
    pub validator_id: String,
    pub bond_account_id: String,
    pub operation: QuickChainBondLifecycleOperationV1,
    pub status: QuickChainBondLifecycleDecisionStatusV1,
    pub rejection_code: Option<QuickChainBondLifecycleRejectionCodeV1>,
    pub amount_minor: Option<String>,
}

impl QuickChainBondLifecycleDecisionV1 {
    /// Validate lifecycle decision shape and accepted/rejected consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainBondLifecycleDecisionV1.schema",
            &self.schema,
            QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA,
        )?;
        validate_version("QuickChainBondLifecycleDecisionV1.version", self.version)?;
        validate_chain_epoch(&self.chain_id, &self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("bond_account_id", &self.bond_account_id)?;

        if let Some(amount) = self.amount_minor.as_deref() {
            parse_minor("amount_minor", amount)?;
        }

        match self.status {
            QuickChainBondLifecycleDecisionStatusV1::Accepted => {
                if self.rejection_code.is_some() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "accepted bond lifecycle decision cannot carry a rejection code",
                    });
                }
            }
            QuickChainBondLifecycleDecisionStatusV1::Rejected => {
                if self.rejection_code.is_none() {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "rejection_code",
                        reason: "rejected bond lifecycle decision requires a deterministic rejection code",
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

fn validate_bond_asset(asset: &str) -> QuickChainResult<()> {
    if asset == QUICKCHAIN_BOND_ASSET_ROC {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "asset",
        reason: "Phase 4 Round 1 bond DTOs only support internal roc",
    })
}

fn validate_optional_ref(field: &'static str, value: Option<&str>) -> QuickChainResult<()> {
    if let Some(value) = value {
        validate_ref(field, value)?;
    }

    Ok(())
}

fn reject_unlock_epoch(field: &'static str, value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "unlock epoch is only valid for unlock requests",
        });
    }

    Ok(())
}

fn require_positive_minor(field: &'static str, value: Option<&str>) -> QuickChainResult<u128> {
    let value = value.ok_or(QuickChainValidationError::InvalidField {
        field,
        reason: "amount is required for this bond intent",
    })?;

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
