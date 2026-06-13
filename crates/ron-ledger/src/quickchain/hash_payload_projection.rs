//! RO:WHAT — Pure projection of validated QuickChain operation and committed-receipt evidence into frozen ron-proto hash-payload DTOs.
//! RO:WHY — ECON/RES: immutable intent, committed sequences, and reviewed external commitments must agree before canonical bytes, hashes, receipts, or roots exist.
//! RO:INTERACTS — ron_proto::quickchain operation/receipt hash payload contracts and QuickChainCommittedOperationRecord.
//! RO:INVARIANTS — explicit immutable context; exact receipt direction; no serialization, hashing, root calculation, clocks, IO, persistence, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through the quickchain-preflight feature.
//! RO:SECURITY — supplied hashes and roots are opaque reviewed inputs; they grant no wallet, settlement, proof, or spend authority.
//! RO:TEST — tests/quickchain_operation_hash_projection.rs and tests/quickchain_receipt_hash_projection.rs.

use ron_proto::{
    quickchain::{
        QuickChainOperationClassV1, QuickChainOperationHashPayloadV1, QuickChainOperationIntentV1,
        QuickChainReceiptHashPayloadV1, QUICKCHAIN_DTO_VERSION, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC,
        QUICKCHAIN_OPERATION_HASH_PAYLOAD_SCHEMA, QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    ContentId,
};
use thiserror::Error;

use super::types::QuickChainCommittedOperationRecord;

/// Explicit immutable context absent from the preflight operation-intent DTO.
///
/// The intent already owns economic identity, accounts, amount, hold identity,
/// and idempotency identity. This context supplies reviewed policy information
/// that `ron-ledger` cannot truthfully derive or invent.
///
/// This type contains no canonical bytes and does not claim that either content
/// identifier was produced by this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainOperationHashProjectionContext {
    operation_id: String,
    purpose: String,
    session_budget_id: Option<String>,
    policy_hash: ContentId,
    chain_params_hash: ContentId,
}

impl QuickChainOperationHashProjectionContext {
    /// Create explicit immutable context for one operation-hash payload.
    #[must_use]
    pub fn new(
        operation_id: impl Into<String>,
        purpose: impl Into<String>,
        session_budget_id: Option<String>,
        policy_hash: ContentId,
        chain_params_hash: ContentId,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            purpose: purpose.into(),
            session_budget_id,
            policy_hash,
            chain_params_hash,
        }
    }

    /// Durable operation identity to which this context is bound.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Reviewed immutable purpose token.
    #[must_use]
    pub fn purpose(&self) -> &str {
        &self.purpose
    }

    /// Optional reviewed session-budget identifier.
    #[must_use]
    pub fn session_budget_id(&self) -> Option<&str> {
        self.session_budget_id.as_deref()
    }

    /// Reviewed policy content identifier.
    #[must_use]
    pub const fn policy_hash(&self) -> &ContentId {
        &self.policy_hash
    }

    /// Reviewed chain-parameter content identifier.
    #[must_use]
    pub const fn chain_params_hash(&self) -> &ContentId {
        &self.chain_params_hash
    }
}

/// Explicit immutable context required to project one accepted receipt payload.
///
/// `previous_ledger_root` and `new_ledger_root` are deliberately opaque inputs.
/// This adapter does not compute them and must not be supplied with legacy
/// rolling-accumulator roots unless a future reviewed design explicitly makes
/// that relationship valid.
///
/// `produced_at_ms` is also explicit. The client-facing operation-intent
/// timestamp is not silently reused as the backend receipt-production time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainReceiptHashProjectionContext {
    operation_id: String,
    operation_hash: ContentId,
    op: String,
    session_budget_id: Option<String>,
    previous_ledger_root: ContentId,
    new_ledger_root: ContentId,
    produced_at_ms: u64,
}

impl QuickChainReceiptHashProjectionContext {
    /// Create explicit reviewed context for one immutable receipt payload.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        operation_id: impl Into<String>,
        operation_hash: ContentId,
        op: impl Into<String>,
        session_budget_id: Option<String>,
        previous_ledger_root: ContentId,
        new_ledger_root: ContentId,
        produced_at_ms: u64,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            operation_hash,
            op: op.into(),
            session_budget_id,
            previous_ledger_root,
            new_ledger_root,
            produced_at_ms,
        }
    }

    /// Durable operation identity to which this receipt context belongs.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Reviewed hash of the corresponding immutable operation payload.
    #[must_use]
    pub const fn operation_hash(&self) -> &ContentId {
        &self.operation_hash
    }

    /// Reviewed action token such as `paid_site_visit` or `hold_capture`.
    #[must_use]
    pub fn op(&self) -> &str {
        &self.op
    }

    /// Optional immutable session-budget identifier.
    #[must_use]
    pub fn session_budget_id(&self) -> Option<&str> {
        self.session_budget_id.as_deref()
    }

    /// Reviewed QuickChain continuity commitment before the operation.
    #[must_use]
    pub const fn previous_ledger_root(&self) -> &ContentId {
        &self.previous_ledger_root
    }

    /// Reviewed QuickChain continuity commitment after the operation.
    #[must_use]
    pub const fn new_ledger_root(&self) -> &ContentId {
        &self.new_ledger_root
    }

    /// Explicit backend receipt-production timestamp.
    #[must_use]
    pub const fn produced_at_ms(&self) -> u64 {
        self.produced_at_ms
    }
}

/// Deterministic failure while projecting immutable QuickChain hash payloads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainHashPayloadProjectionError {
    /// Operation context for one durable operation was applied to another.
    #[error(
        "operation projection context mismatch: \
         context={context_operation_id}, intent={intent_operation_id}"
    )]
    OperationContextMismatch {
        /// Operation identity to which the supplied context belongs.
        context_operation_id: String,

        /// Operation identity carried by the submitted intent.
        intent_operation_id: String,
    },

    /// Receipt context for one durable operation was applied to another.
    #[error(
        "receipt projection context mismatch: \
         context={context_operation_id}, record={record_operation_id}"
    )]
    ReceiptContextMismatch {
        /// Operation identity to which the supplied context belongs.
        context_operation_id: String,

        /// Operation identity carried by committed ledger evidence.
        record_operation_id: String,
    },

    /// The source operation intent failed the frozen ron-proto contract.
    #[error("invalid operation intent for {operation_id}: {reason}")]
    InvalidOperationIntent {
        /// Durable operation identity carried by the invalid intent.
        operation_id: String,

        /// Bounded validation explanation from ron-proto.
        reason: String,
    },

    /// The linked ron-proto version exposed an operation class not reviewed by
    /// this projection adapter.
    #[error("unsupported operation class while projecting {operation_id}")]
    UnsupportedOperationClass {
        /// Durable operation identity using the unsupported class.
        operation_id: String,
    },

    /// The assembled operation-hash DTO failed its frozen ron-proto contract.
    #[error("invalid operation-hash payload for {operation_id}: {reason}")]
    InvalidOperationHashPayload {
        /// Durable operation identity whose payload was invalid.
        operation_id: String,

        /// Bounded validation explanation from ron-proto.
        reason: String,
    },

    /// The assembled receipt-hash DTO failed its frozen ron-proto contract.
    #[error("invalid receipt-hash payload for {operation_id}: {reason}")]
    InvalidReceiptHashPayload {
        /// Durable operation identity whose receipt payload was invalid.
        operation_id: String,

        /// Bounded validation explanation from ron-proto.
        reason: String,
    },
}

/// Project one operation intent into the frozen immutable operation-hash DTO.
///
/// The function first validates the complete source intent. This is important
/// because fields deliberately excluded from the hash payload, such as the
/// reserved client-facing `account_sequence`, must not be silently discarded
/// when invalid.
///
/// The returned value is only a typed payload. This function does not:
///
/// - serialize canonical JSON;
/// - construct domain-separated preimage bytes;
/// - calculate BLAKE3;
/// - assert an operation hash;
/// - construct a receipt;
/// - read a clock;
/// - perform IO or persistence;
/// - mutate ledger state.
pub fn project_operation_hash_payload(
    intent: &QuickChainOperationIntentV1,
    context: &QuickChainOperationHashProjectionContext,
) -> Result<QuickChainOperationHashPayloadV1, QuickChainHashPayloadProjectionError> {
    if context.operation_id() != intent.operation_id {
        return Err(
            QuickChainHashPayloadProjectionError::OperationContextMismatch {
                context_operation_id: context.operation_id().to_string(),
                intent_operation_id: intent.operation_id.clone(),
            },
        );
    }

    validate_intent_for_projection(intent)?;

    let amount_minor = intent.amount_minor.as_ref().ok_or_else(|| {
        QuickChainHashPayloadProjectionError::InvalidOperationIntent {
            operation_id: intent.operation_id.clone(),
            reason: "validated operation intent omitted amount_minor".to_string(),
        }
    })?;

    let operation_family = canonical_operation_family(intent.op_class).ok_or_else(|| {
        QuickChainHashPayloadProjectionError::UnsupportedOperationClass {
            operation_id: intent.operation_id.clone(),
        }
    })?;

    let payload = QuickChainOperationHashPayloadV1 {
        schema: QUICKCHAIN_OPERATION_HASH_PAYLOAD_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: intent.chain_id.clone(),
        operation_id: intent.operation_id.clone(),
        op_class: intent.op_class,
        actor_account_id: intent.actor_account_id.clone(),
        counterparty_account_id: intent.counterparty_account_id.clone(),
        asset: QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC.to_string(),
        amount_minor: amount_minor.clone(),
        purpose: context.purpose().to_string(),
        hold_id: intent.hold_id.clone(),
        session_budget_id: context.session_budget_id().map(str::to_owned),
        policy_hash: context.policy_hash().clone(),
        chain_params_hash: context.chain_params_hash().clone(),
        idempotency_scope_account_id: intent.actor_account_id.clone(),
        idempotency_scope_operation_family: operation_family.to_string(),
        idempotency_key: intent.idempotency_key.clone(),
    };

    payload.validate().map_err(|error| {
        QuickChainHashPayloadProjectionError::InvalidOperationHashPayload {
            operation_id: intent.operation_id.clone(),
            reason: error.to_string(),
        }
    })?;

    Ok(payload)
}

/// Project committed ledger evidence into the frozen immutable receipt DTO.
///
/// Account direction is derived from the validated operation class:
///
/// - issue: `None -> actor`;
/// - transfer: `actor -> counterparty`;
/// - burn: `actor -> None`;
/// - hold open: `actor -> optional fixed counterparty`;
/// - hold capture: `actor -> counterparty`;
/// - hold release/expiry: `actor -> None`.
///
/// Ledger-owned sequence evidence and transaction identity come from the
/// committed record. The operation hash, action token, optional session budget,
/// continuity-root references, and production timestamp must be supplied
/// explicitly by the reviewed caller.
///
/// The returned value is not a hydrated receipt and contains no status,
/// receipt hash, receipt root, checkpoint hash, memo, signatures, or finality
/// claim.
///
/// This function performs no canonical serialization, hashing, root calculation,
/// clock access, IO, persistence, or state mutation.
pub fn project_receipt_hash_payload(
    record: &QuickChainCommittedOperationRecord,
    context: &QuickChainReceiptHashProjectionContext,
) -> Result<QuickChainReceiptHashPayloadV1, QuickChainHashPayloadProjectionError> {
    let intent = record.intent();

    if context.operation_id() != intent.operation_id {
        return Err(
            QuickChainHashPayloadProjectionError::ReceiptContextMismatch {
                context_operation_id: context.operation_id().to_string(),
                record_operation_id: intent.operation_id.clone(),
            },
        );
    }

    validate_intent_for_projection(intent)?;

    let amount_minor = intent.amount_minor.as_ref().ok_or_else(|| {
        QuickChainHashPayloadProjectionError::InvalidOperationIntent {
            operation_id: intent.operation_id.clone(),
            reason: "validated operation intent omitted amount_minor".to_string(),
        }
    })?;

    let (from_account_id, to_account_id) = receipt_account_direction(intent).ok_or_else(|| {
        QuickChainHashPayloadProjectionError::UnsupportedOperationClass {
            operation_id: intent.operation_id.clone(),
        }
    })?;

    let payload = QuickChainReceiptHashPayloadV1 {
        schema: QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: intent.chain_id.clone(),
        txid: record.receipt_txid().to_string(),
        operation_id: intent.operation_id.clone(),
        operation_hash: context.operation_hash().clone(),
        op: context.op().to_string(),
        op_class: intent.op_class,
        from_account_id,
        to_account_id,
        asset: QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC.to_string(),
        amount_minor: amount_minor.clone(),
        account_sequence: record.account_sequence(),
        hold_id: intent.hold_id.clone(),
        session_budget_id: context.session_budget_id().map(str::to_owned),
        idempotency_key: intent.idempotency_key.clone(),
        ledger_seq_start: record.ledger_sequence_start(),
        ledger_seq_end: record.ledger_sequence_end(),
        previous_ledger_root: context.previous_ledger_root().clone(),
        new_ledger_root: context.new_ledger_root().clone(),
        produced_at_ms: context.produced_at_ms(),
    };

    payload.validate().map_err(|error| {
        QuickChainHashPayloadProjectionError::InvalidReceiptHashPayload {
            operation_id: intent.operation_id.clone(),
            reason: error.to_string(),
        }
    })?;

    Ok(payload)
}

fn validate_intent_for_projection(
    intent: &QuickChainOperationIntentV1,
) -> Result<(), QuickChainHashPayloadProjectionError> {
    intent.validate().map_err(|error| {
        QuickChainHashPayloadProjectionError::InvalidOperationIntent {
            operation_id: intent.operation_id.clone(),
            reason: error.to_string(),
        }
    })
}

fn canonical_operation_family(op_class: QuickChainOperationClassV1) -> Option<&'static str> {
    match op_class {
        QuickChainOperationClassV1::Issue => Some("issue"),
        QuickChainOperationClassV1::Transfer => Some("transfer"),
        QuickChainOperationClassV1::Burn => Some("burn"),
        QuickChainOperationClassV1::HoldOpen => Some("hold_open"),
        QuickChainOperationClassV1::HoldCapture => Some("hold_capture"),
        QuickChainOperationClassV1::HoldRelease => Some("hold_release"),
        QuickChainOperationClassV1::HoldExpire => Some("hold_expire"),

        // ron-proto marks this vocabulary non-exhaustive. Future classes must
        // receive an explicit reviewed mapping instead of silently inheriting
        // an accidental token.
        _ => None,
    }
}

fn receipt_account_direction(
    intent: &QuickChainOperationIntentV1,
) -> Option<(Option<String>, Option<String>)> {
    match intent.op_class {
        QuickChainOperationClassV1::Issue => Some((None, Some(intent.actor_account_id.clone()))),

        QuickChainOperationClassV1::Transfer => Some((
            Some(intent.actor_account_id.clone()),
            intent.counterparty_account_id.clone(),
        )),

        QuickChainOperationClassV1::Burn => Some((Some(intent.actor_account_id.clone()), None)),

        QuickChainOperationClassV1::HoldOpen => Some((
            Some(intent.actor_account_id.clone()),
            intent.counterparty_account_id.clone(),
        )),

        QuickChainOperationClassV1::HoldCapture => Some((
            Some(intent.actor_account_id.clone()),
            intent.counterparty_account_id.clone(),
        )),

        QuickChainOperationClassV1::HoldRelease | QuickChainOperationClassV1::HoldExpire => {
            Some((Some(intent.actor_account_id.clone()), None))
        }

        _ => None,
    }
}
