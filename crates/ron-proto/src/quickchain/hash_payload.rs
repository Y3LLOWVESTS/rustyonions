//! RO:WHAT — Immutable QuickChain Phase 0 hash-payload DTO boundaries for operation, receipt, account, active-hold, and unsigned-checkpoint vectors.
//! RO:WHY — ECON/RES: hashes must commit only to reviewed immutable fields and must not include self-hashes, mutable status, signatures, or later proof evidence.
//! RO:INTERACTS — canonical JSON helpers, domain separators, operation/receipt DTO planning, future ron-ledger replay and root modules.
//! RO:INVARIANTS — DTO and validation only; no hashing, roots, persistence, service calls, signatures, settlement, or ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — payload schemas are explicit; unknown fields reject; receipt/checkpoint self-reference is structurally impossible.
//! RO:TEST — tests/quickchain_hash_payloads.rs and tests/vectors/quickchain/hash_payloads/.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    ids::{validate_hold_id_v1, validate_idempotency_key_v1, validate_operation_id_v1},
    money::validate_quickchain_minor_units,
    operation::QuickChainOperationClassV1,
    validate_chain_id, validate_epoch_id, validate_ref, validate_schema, validate_timestamp_order,
    validate_token, validate_version, QuickChainCanonicalEncodingV1, QuickChainReceiptRootSchemeV1,
    QuickChainResult, QuickChainStateRootSchemeV1, QuickChainValidationError,
};

/// Schema tag for immutable canonical operation-hash payloads.
pub const QUICKCHAIN_OPERATION_HASH_PAYLOAD_SCHEMA: &str = "quickchain.operation-hash-payload.v1";

/// Schema tag for immutable canonical receipt-hash payloads.
pub const QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA: &str = "quickchain.receipt-hash-payload.v1";

/// Schema tag for canonical account-leaf payloads.
pub const QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA: &str = "quickchain.account-leaf-payload.v1";

/// Schema tag for canonical open-hold leaf payloads.
pub const QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA: &str =
    "quickchain.active-hold-leaf-payload.v1";

/// Schema tag for canonical unsigned-checkpoint hash payloads.
pub const QUICKCHAIN_UNSIGNED_CHECKPOINT_PAYLOAD_SCHEMA: &str =
    "quickchain.unsigned-checkpoint-payload.v1";

/// Internal ROC asset token committed by Phase 0 hash payloads.
pub const QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC: &str = "roc";

/// Maximum operation purpose token length.
pub const MAX_QUICKCHAIN_OPERATION_PURPOSE_BYTES: usize = 96;

/// Maximum operation-family token length.
pub const MAX_QUICKCHAIN_OPERATION_FAMILY_BYTES: usize = 64;

/// Maximum execution-spec version token length.
pub const MAX_QUICKCHAIN_EXECUTION_SPEC_VERSION_BYTES: usize = 64;

/// Maximum session-budget identifier length.
pub const MAX_QUICKCHAIN_HASH_PAYLOAD_SESSION_BUDGET_ID_BYTES: usize = 128;

/// Immutable operation intent committed before receipt fields exist.
///
/// This payload intentionally excludes receipt status, ledger results,
/// checkpoint evidence, signatures, and wall-clock fields not required by the
/// accepted operation contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainOperationHashPayloadV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub operation_id: String,
    pub op_class: QuickChainOperationClassV1,
    pub actor_account_id: String,
    #[serde(default)]
    pub counterparty_account_id: Option<String>,
    pub asset: String,
    pub amount_minor: String,
    pub purpose: String,
    #[serde(default)]
    pub hold_id: Option<String>,
    #[serde(default)]
    pub session_budget_id: Option<String>,
    pub policy_hash: ContentId,
    pub chain_params_hash: ContentId,
    pub idempotency_scope_account_id: String,
    pub idempotency_scope_operation_family: String,
    pub idempotency_key: String,
}

impl QuickChainOperationHashPayloadV1 {
    /// Validate immutable operation-hash payload shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainOperationHashPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_OPERATION_HASH_PAYLOAD_SCHEMA,
        )?;
        validate_version("QuickChainOperationHashPayloadV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_operation_id_v1("operation_id", &self.operation_id)?;
        validate_ref("actor_account_id", &self.actor_account_id)?;

        if let Some(counterparty_account_id) = &self.counterparty_account_id {
            validate_ref("counterparty_account_id", counterparty_account_id)?;
        }

        validate_asset_roc(&self.asset)?;
        validate_quickchain_minor_units("amount_minor", &self.amount_minor)?;
        validate_token(
            "purpose",
            &self.purpose,
            MAX_QUICKCHAIN_OPERATION_PURPOSE_BYTES,
        )?;

        if let Some(hold_id) = &self.hold_id {
            validate_hold_id_v1("hold_id", hold_id)?;
        }

        if let Some(session_budget_id) = &self.session_budget_id {
            validate_token(
                "session_budget_id",
                session_budget_id,
                MAX_QUICKCHAIN_HASH_PAYLOAD_SESSION_BUDGET_ID_BYTES,
            )?;
        }

        validate_ref(
            "idempotency_scope_account_id",
            &self.idempotency_scope_account_id,
        )?;
        validate_token(
            "idempotency_scope_operation_family",
            &self.idempotency_scope_operation_family,
            MAX_QUICKCHAIN_OPERATION_FAMILY_BYTES,
        )?;
        validate_idempotency_key_v1("idempotency_key", &self.idempotency_key)?;

        if self.idempotency_scope_account_id != self.actor_account_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "idempotency_scope_account_id",
                reason: "must match actor_account_id",
            });
        }

        if self.idempotency_scope_operation_family != operation_class_token(self.op_class) {
            return Err(QuickChainValidationError::InvalidField {
                field: "idempotency_scope_operation_family",
                reason: "must match the canonical operation class token",
            });
        }

        validate_operation_payload_shape(
            self.op_class,
            self.counterparty_account_id.as_deref(),
            self.hold_id.as_deref(),
        )
    }
}

/// Immutable accepted receipt fields committed by the receipt hash.
///
/// This payload intentionally excludes `receipt_hash`, receipt roots,
/// checkpoint hashes, mutable settlement status, display memo text, and
/// validator signatures. Those fields are later evidence or presentation data
/// and cannot be part of the receipt's own hash preimage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainReceiptHashPayloadV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub txid: String,
    pub operation_id: String,
    pub operation_hash: ContentId,
    pub op: String,
    pub op_class: QuickChainOperationClassV1,
    #[serde(default)]
    pub from_account_id: Option<String>,
    #[serde(default)]
    pub to_account_id: Option<String>,
    pub asset: String,
    pub amount_minor: String,
    pub account_sequence: u64,
    #[serde(default)]
    pub hold_id: Option<String>,
    #[serde(default)]
    pub session_budget_id: Option<String>,
    pub idempotency_key: String,
    pub ledger_seq_start: u64,
    pub ledger_seq_end: u64,
    pub previous_ledger_root: ContentId,
    pub new_ledger_root: ContentId,
    pub produced_at_ms: u64,
}

impl QuickChainReceiptHashPayloadV1 {
    /// Validate immutable receipt-hash payload shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainReceiptHashPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
        )?;
        validate_version("QuickChainReceiptHashPayloadV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("txid", &self.txid)?;
        validate_operation_id_v1("operation_id", &self.operation_id)?;
        validate_token("op", &self.op, MAX_QUICKCHAIN_OPERATION_PURPOSE_BYTES)?;

        if let Some(from_account_id) = &self.from_account_id {
            validate_ref("from_account_id", from_account_id)?;
        }

        if let Some(to_account_id) = &self.to_account_id {
            validate_ref("to_account_id", to_account_id)?;
        }

        validate_asset_roc(&self.asset)?;
        validate_quickchain_minor_units("amount_minor", &self.amount_minor)?;

        if self.account_sequence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "account_sequence",
                reason: "must be greater than zero",
            });
        }

        if let Some(hold_id) = &self.hold_id {
            validate_hold_id_v1("hold_id", hold_id)?;
        }

        if let Some(session_budget_id) = &self.session_budget_id {
            validate_token(
                "session_budget_id",
                session_budget_id,
                MAX_QUICKCHAIN_HASH_PAYLOAD_SESSION_BUDGET_ID_BYTES,
            )?;
        }

        validate_idempotency_key_v1("idempotency_key", &self.idempotency_key)?;

        if self.ledger_seq_start == 0
            || self.ledger_seq_end == 0
            || self.ledger_seq_end < self.ledger_seq_start
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "ledger_seq_range",
                reason: "must be non-zero and ordered",
            });
        }

        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "must be greater than zero",
            });
        }

        validate_receipt_payload_shape(
            self.op_class,
            self.from_account_id.as_deref(),
            self.to_account_id.as_deref(),
            self.hold_id.as_deref(),
        )
    }
}

/// Canonical account leaf committed under the account-state hash domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainAccountLeafPayloadV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub account_id: String,
    pub asset: String,
    pub balance_minor: String,
    pub held_minor: String,
    pub available_minor: String,
    pub account_sequence: u64,
    pub receipt_root: ContentId,
    pub holds_root: ContentId,
    #[serde(default)]
    pub permissions_root: Option<ContentId>,
    pub updated_at_epoch: String,
}

impl QuickChainAccountLeafPayloadV1 {
    /// Validate canonical account-leaf shape and arithmetic consistency.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainAccountLeafPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        )?;
        validate_version("QuickChainAccountLeafPayloadV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("account_id", &self.account_id)?;
        validate_asset_roc(&self.asset)?;

        let balance = parse_unsigned_minor("balance_minor", &self.balance_minor)?;
        let held = parse_unsigned_minor("held_minor", &self.held_minor)?;
        let available = parse_unsigned_minor("available_minor", &self.available_minor)?;

        let recomposed =
            available
                .checked_add(held)
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "account_balance",
                    reason: "available_minor plus held_minor must fit in u128",
                })?;

        if balance != recomposed {
            return Err(QuickChainValidationError::InvalidField {
                field: "account_balance",
                reason: "balance_minor must equal available_minor plus held_minor",
            });
        }

        validate_epoch_id(&self.updated_at_epoch)
    }
}

/// Wire value for the only status allowed in the active hold leaf set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainActiveHoldStatusV1 {
    Open,
}

/// Canonical open-hold leaf committed under the hold-state hash domain.
///
/// Terminal holds are deliberately unrepresentable here. Their lifecycle
/// remains provable through immutable receipts after active-state compaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainActiveHoldLeafPayloadV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub hold_id: String,
    pub account_id: String,
    #[serde(default)]
    pub counterparty_account_id: Option<String>,
    pub amount_minor: String,
    pub purpose: String,
    pub created_at_epoch: String,
    pub expires_at_epoch: String,
    pub status: QuickChainActiveHoldStatusV1,
    pub operation_id: String,
    pub idempotency_key: String,
    pub policy_hash: ContentId,
}

impl QuickChainActiveHoldLeafPayloadV1 {
    /// Validate canonical active-hold leaf shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainActiveHoldLeafPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA,
        )?;
        validate_version("QuickChainActiveHoldLeafPayloadV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_hold_id_v1("hold_id", &self.hold_id)?;
        validate_ref("account_id", &self.account_id)?;

        if let Some(counterparty_account_id) = &self.counterparty_account_id {
            validate_ref("counterparty_account_id", counterparty_account_id)?;
        }

        validate_quickchain_minor_units("amount_minor", &self.amount_minor)?;
        validate_token(
            "purpose",
            &self.purpose,
            MAX_QUICKCHAIN_OPERATION_PURPOSE_BYTES,
        )?;
        validate_epoch_id(&self.created_at_epoch)?;
        validate_epoch_id(&self.expires_at_epoch)?;
        validate_operation_id_v1("operation_id", &self.operation_id)?;
        validate_idempotency_key_v1("idempotency_key", &self.idempotency_key)
    }
}

/// Canonical checkpoint settlement mode tag.
///
/// Phase 0 permits the local-root vector only. Later modes require a new
/// reviewed schema/version and explicit authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainSettlementModeV1 {
    LocalRoot,
}

/// Canonical supply delta embedded in an unsigned checkpoint payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainSupplyDeltaV1 {
    pub issued_minor: String,
    pub burned_minor: String,
    pub net_minor: String,
}

impl QuickChainSupplyDeltaV1 {
    /// Validate supply delta strings and exact net arithmetic.
    pub fn validate(&self) -> QuickChainResult<()> {
        let issued = parse_unsigned_minor("supply_delta.issued_minor", &self.issued_minor)?;
        let burned = parse_unsigned_minor("supply_delta.burned_minor", &self.burned_minor)?;
        validate_signed_minor_units("supply_delta.net_minor", &self.net_minor)?;

        let expected_net = if issued >= burned {
            (issued - burned).to_string()
        } else {
            format!("-{}", burned - issued)
        };

        if self.net_minor != expected_net {
            return Err(QuickChainValidationError::InvalidField {
                field: "supply_delta.net_minor",
                reason: "must equal issued_minor minus burned_minor",
            });
        }

        Ok(())
    }
}

/// Canonical conservation summary embedded in an unsigned checkpoint payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainConservationV1 {
    pub debits_minor: String,
    pub credits_minor: String,
    pub issue_exceptions_minor: String,
    pub burn_exceptions_minor: String,
    pub valid: bool,
}

impl QuickChainConservationV1 {
    /// Validate the Phase 0 conservation equation.
    pub fn validate(&self) -> QuickChainResult<()> {
        let debits = parse_unsigned_minor("conservation.debits_minor", &self.debits_minor)?;
        let credits = parse_unsigned_minor("conservation.credits_minor", &self.credits_minor)?;
        let issues = parse_unsigned_minor(
            "conservation.issue_exceptions_minor",
            &self.issue_exceptions_minor,
        )?;
        let burns = parse_unsigned_minor(
            "conservation.burn_exceptions_minor",
            &self.burn_exceptions_minor,
        )?;

        let debit_side =
            debits
                .checked_add(issues)
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "conservation",
                    reason: "debits plus issue exceptions must fit in u128",
                })?;
        let credit_side =
            credits
                .checked_add(burns)
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "conservation",
                    reason: "credits plus burn exceptions must fit in u128",
                })?;

        if !self.valid || debit_side != credit_side {
            return Err(QuickChainValidationError::InvalidField {
                field: "conservation",
                reason: "must be marked valid and satisfy debits + issues = credits + burns",
            });
        }

        Ok(())
    }
}

/// Canonical unsigned checkpoint hash payload.
///
/// Validator signatures are structurally excluded. Signatures may attest to
/// the resulting checkpoint hash later, but can never be part of the hash they
/// sign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainUnsignedCheckpointPayloadV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub height: u64,
    pub epoch_id: String,
    pub execution_spec_version: String,
    pub previous_checkpoint_hash: ContentId,
    pub previous_state_root: ContentId,
    pub new_state_root: ContentId,
    pub receipt_root: ContentId,
    pub accounting_snapshot_root: ContentId,
    pub reward_manifest_root: ContentId,
    #[serde(default)]
    pub data_availability_root: Option<ContentId>,
    pub policy_hash: ContentId,
    #[serde(default)]
    pub validator_set_hash: Option<ContentId>,
    pub chain_params_hash: ContentId,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub state_root_scheme: QuickChainStateRootSchemeV1,
    pub receipt_root_scheme: QuickChainReceiptRootSchemeV1,
    pub supply_delta: QuickChainSupplyDeltaV1,
    pub conservation: QuickChainConservationV1,
    pub settlement_mode: QuickChainSettlementModeV1,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
    pub produced_at_ms: u64,
}

impl QuickChainUnsignedCheckpointPayloadV1 {
    /// Validate unsigned checkpoint hash-payload shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainUnsignedCheckpointPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_UNSIGNED_CHECKPOINT_PAYLOAD_SCHEMA,
        )?;
        validate_version(
            "QuickChainUnsignedCheckpointPayloadV1.version",
            self.version,
        )?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_token(
            "execution_spec_version",
            &self.execution_spec_version,
            MAX_QUICKCHAIN_EXECUTION_SPEC_VERSION_BYTES,
        )?;

        if self.canonical_encoding != QuickChainCanonicalEncodingV1::JsonV1 {
            return Err(QuickChainValidationError::InvalidField {
                field: "canonical_encoding",
                reason: "must be json-v1 for the Phase 0 locked hash corpus",
            });
        }

        if self.data_availability_root.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "data_availability_root",
                reason: "must be absent for the Phase 0 local-root vector",
            });
        }

        if self.validator_set_hash.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "validator_set_hash",
                reason: "must be absent before validator-set authorization",
            });
        }

        self.supply_delta.validate()?;
        self.conservation.validate()?;
        validate_timestamp_order(
            "checkpoint_time_range",
            self.started_at_ms,
            self.ended_at_ms,
            self.produced_at_ms,
        )
    }
}

fn validate_asset_roc(asset: &str) -> QuickChainResult<()> {
    if asset == QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "asset",
        reason: "must be roc during the internal ROC phase",
    })
}

fn operation_class_token(op_class: QuickChainOperationClassV1) -> &'static str {
    match op_class {
        QuickChainOperationClassV1::Issue => "issue",
        QuickChainOperationClassV1::Transfer => "transfer",
        QuickChainOperationClassV1::Burn => "burn",
        QuickChainOperationClassV1::HoldOpen => "hold_open",
        QuickChainOperationClassV1::HoldCapture => "hold_capture",
        QuickChainOperationClassV1::HoldRelease => "hold_release",
        QuickChainOperationClassV1::HoldExpire => "hold_expire",
    }
}

fn validate_operation_payload_shape(
    op_class: QuickChainOperationClassV1,
    counterparty_account_id: Option<&str>,
    hold_id: Option<&str>,
) -> QuickChainResult<()> {
    match op_class {
        QuickChainOperationClassV1::Issue => {
            forbid_present("counterparty_account_id", counterparty_account_id.is_some())?;
            forbid_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::Transfer => {
            require_present("counterparty_account_id", counterparty_account_id.is_some())?;
            forbid_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::Burn => {
            forbid_present("counterparty_account_id", counterparty_account_id.is_some())?;
            forbid_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::HoldOpen => require_present("hold_id", hold_id.is_some()),
        QuickChainOperationClassV1::HoldCapture => {
            require_present("counterparty_account_id", counterparty_account_id.is_some())?;
            require_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::HoldRelease | QuickChainOperationClassV1::HoldExpire => {
            forbid_present("counterparty_account_id", counterparty_account_id.is_some())?;
            require_present("hold_id", hold_id.is_some())
        }
    }
}

fn validate_receipt_payload_shape(
    op_class: QuickChainOperationClassV1,
    from_account_id: Option<&str>,
    to_account_id: Option<&str>,
    hold_id: Option<&str>,
) -> QuickChainResult<()> {
    match op_class {
        QuickChainOperationClassV1::Issue => {
            forbid_present("from_account_id", from_account_id.is_some())?;
            require_present("to_account_id", to_account_id.is_some())?;
            forbid_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::Transfer => {
            require_present("from_account_id", from_account_id.is_some())?;
            require_present("to_account_id", to_account_id.is_some())?;
            forbid_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::Burn => {
            require_present("from_account_id", from_account_id.is_some())?;
            forbid_present("to_account_id", to_account_id.is_some())?;
            forbid_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::HoldOpen => {
            require_present("from_account_id", from_account_id.is_some())?;
            require_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::HoldCapture => {
            require_present("from_account_id", from_account_id.is_some())?;
            require_present("to_account_id", to_account_id.is_some())?;
            require_present("hold_id", hold_id.is_some())
        }
        QuickChainOperationClassV1::HoldRelease | QuickChainOperationClassV1::HoldExpire => {
            require_present("from_account_id", from_account_id.is_some())?;
            forbid_present("to_account_id", to_account_id.is_some())?;
            require_present("hold_id", hold_id.is_some())
        }
    }
}

fn require_present(field: &'static str, present: bool) -> QuickChainResult<()> {
    if present {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field,
        reason: "required for this operation class",
    })
}

fn forbid_present(field: &'static str, present: bool) -> QuickChainResult<()> {
    if !present {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field,
        reason: "must be absent for this operation class",
    })
}

fn parse_unsigned_minor(field: &'static str, value: &str) -> QuickChainResult<u128> {
    validate_quickchain_minor_units(field, value)?;

    value
        .parse::<u128>()
        .map_err(|_| QuickChainValidationError::InvalidMoney {
            field,
            reason: "must fit in u128 minor units",
        })
}

fn validate_signed_minor_units(field: &'static str, value: &str) -> QuickChainResult<()> {
    if let Some(magnitude) = value.strip_prefix('-') {
        if magnitude == "0" {
            return Err(QuickChainValidationError::InvalidMoney {
                field,
                reason: "negative zero is not canonical",
            });
        }

        return validate_quickchain_minor_units(field, magnitude);
    }

    validate_quickchain_minor_units(field, value)
}
