//! RO:WHAT — QuickChain operation-intent DTO shapes for replay/idempotency preflight.
//! RO:WHY — ECON/RES: replay must distinguish durable operations from retry idempotency keys.
//! RO:INTERACTS — future svc-wallet receipts, ron-ledger replay, hold DTOs, QC-0A vectors.
//! RO:INVARIANTS — DTO-only; strict class matrix; account_sequence is ledger-assigned; no mutation or roots.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — idempotency_key is retry metadata, not authority or a secret.
//! RO:TEST — tests/quickchain_operation_hold.rs.

use serde::{Deserialize, Serialize};

use super::{
    ids::{validate_hold_id_v1, validate_idempotency_key_v1, validate_operation_id_v1},
    money::validate_quickchain_minor_units,
    validate_chain_id, validate_ref, validate_schema, validate_version, QuickChainResult,
    QuickChainValidationError,
};

pub const QUICKCHAIN_OPERATION_INTENT_SCHEMA: &str = "quickchain.operation-intent.v1";

/// Operation class vocabulary for future wallet/ledger receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainOperationClassV1 {
    Issue,
    Transfer,
    Burn,
    HoldOpen,
    HoldCapture,
    HoldRelease,
    HoldExpire,
}

/// Operation-intent DTO for QC-0B shape tests.
///
/// This is not an executable transaction. It carries enough shape to test that
/// operation_id, idempotency_key, hold_id, and account_sequence semantics do
/// not get blurred before replay/root work begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainOperationIntentV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub operation_id: String,
    pub idempotency_key: String,
    pub op_class: QuickChainOperationClassV1,

    /// Primary affected account.
    ///
    /// This is the credited account for issue and the source/holder account for
    /// transfer, burn, and hold lifecycle operations.
    pub actor_account_id: String,

    /// Destination or beneficiary account when the operation class uses one.
    #[serde(default)]
    pub counterparty_account_id: Option<String>,

    /// Integer ROC minor units committed by the operation intent.
    ///
    /// Phase 0 requires an explicit amount for every operation class, including
    /// capture, release, and expiry, so retry/conflict identity is value-complete.
    #[serde(default)]
    pub amount_minor: Option<String>,

    /// One durable hold lifecycle identifier for hold operations.
    #[serde(default)]
    pub hold_id: Option<String>,

    /// Reserved post-acceptance sequence field.
    ///
    /// Operation intents must leave this absent because account_sequence is
    /// assigned by the wallet/ledger commit path after acceptance.
    #[serde(default)]
    pub account_sequence: Option<u64>,

    pub produced_at_ms: u64,
}

impl QuickChainOperationIntentV1 {
    /// Validate DTO shape only. This does not execute the operation.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainOperationIntentV1.schema",
            &self.schema,
            QUICKCHAIN_OPERATION_INTENT_SCHEMA,
        )?;
        validate_version("QuickChainOperationIntentV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_operation_id_v1("operation_id", &self.operation_id)?;
        validate_idempotency_key_v1("idempotency_key", &self.idempotency_key)?;
        validate_ref("actor_account_id", &self.actor_account_id)?;

        if let Some(counterparty_account_id) = &self.counterparty_account_id {
            validate_ref("counterparty_account_id", counterparty_account_id)?;
        }

        if let Some(amount_minor) = &self.amount_minor {
            validate_quickchain_minor_units("amount_minor", amount_minor)?;
        }

        if let Some(hold_id) = &self.hold_id {
            validate_hold_id_v1("hold_id", hold_id)?;
        }

        if self.account_sequence.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "account_sequence",
                reason: "must be absent because the ledger assigns it after acceptance",
            });
        }

        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "must be greater than zero",
            });
        }

        self.validate_class_requirements()
    }

    /// Validate the Phase 0 operation-intent field matrix.
    ///
    /// - issue: actor is credited account; amount required; no counterparty/hold
    /// - transfer: actor is source; counterparty and amount required; no hold
    /// - burn: actor is source; amount required; no counterparty/hold
    /// - hold_open: actor is holder; amount and hold required; counterparty optional
    /// - hold_capture: actor is holder; counterparty, amount, and hold required
    /// - hold_release/hold_expire: actor is holder; amount and hold required; no counterparty
    fn validate_class_requirements(&self) -> QuickChainResult<()> {
        match self.op_class {
            QuickChainOperationClassV1::Issue => {
                require_amount(self.amount_minor.as_deref())?;
                forbid_optional_field(
                    "counterparty_account_id",
                    self.counterparty_account_id.is_some(),
                )?;
                forbid_optional_field("hold_id", self.hold_id.is_some())
            }

            QuickChainOperationClassV1::Transfer => {
                require_amount(self.amount_minor.as_deref())?;
                require_counterparty(self.counterparty_account_id.as_deref())?;
                forbid_optional_field("hold_id", self.hold_id.is_some())
            }

            QuickChainOperationClassV1::Burn => {
                require_amount(self.amount_minor.as_deref())?;
                forbid_optional_field(
                    "counterparty_account_id",
                    self.counterparty_account_id.is_some(),
                )?;
                forbid_optional_field("hold_id", self.hold_id.is_some())
            }

            QuickChainOperationClassV1::HoldOpen => {
                require_amount(self.amount_minor.as_deref())?;
                require_hold(self.hold_id.as_deref())
            }

            QuickChainOperationClassV1::HoldCapture => {
                require_amount(self.amount_minor.as_deref())?;
                require_counterparty(self.counterparty_account_id.as_deref())?;
                require_hold(self.hold_id.as_deref())
            }

            QuickChainOperationClassV1::HoldRelease | QuickChainOperationClassV1::HoldExpire => {
                require_amount(self.amount_minor.as_deref())?;
                forbid_optional_field(
                    "counterparty_account_id",
                    self.counterparty_account_id.is_some(),
                )?;
                require_hold(self.hold_id.as_deref())
            }
        }
    }
}

fn require_amount(value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "amount_minor",
        reason: "required for this operation class",
    })
}

fn require_counterparty(value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "counterparty_account_id",
        reason: "required for this operation class",
    })
}

fn require_hold(value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "hold_id",
        reason: "required for this operation class",
    })
}

fn forbid_optional_field(field: &'static str, present: bool) -> QuickChainResult<()> {
    if !present {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field,
        reason: "must be absent for this operation class",
    })
}
