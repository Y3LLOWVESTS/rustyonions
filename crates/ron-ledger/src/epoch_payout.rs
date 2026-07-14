//! RO:WHAT — Durable Phase 16 ROC epoch-payout operation and receipt evidence.
//! RO:WHY — ECON/RES: quorum-approved rewards must remain bound to policy,
//! economics, registry, reward-binding, accounting, evidence, and ledger truth.
//! RO:INTERACTS — ron-proto Phase 15 quorum DTOs, ledger entries, svc-wallet.
//! RO:INVARIANTS — integer money, strict roots, unique replay identities,
//! receipt hashes, deterministic replay, and supply conservation.
//! RO:METRICS — none; service wrappers may count validation and replay outcomes.
//! RO:CONFIG — no mutable reward rates are owned here.
//! RO:SECURITY — this module records accepted evidence but does not verify keys
//! or authorize wallet mutation by itself.
//! RO:TEST — svc-wallet Phase 16 quorum execution integration tests.

use std::collections::{BTreeMap, BTreeSet};

use ron_proto::{ContentId, ServiceNodeQuorumV1};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current Phase 16 payout operation and receipt version.
pub const EPOCH_PAYOUT_VERSION: u16 = 1;

/// Schema for one quorum-approved ROC payout operation.
pub const EPOCH_PAYOUT_OPERATION_SCHEMA: &str = "ron.ledger.epoch-payout-operation.v1";

/// Schema for one accepted quorum-approved ROC payout receipt.
pub const EPOCH_PAYOUT_RECEIPT_SCHEMA: &str = "ron.ledger.epoch-payout-receipt.v1";

const MAX_TOKEN_BYTES: usize = 256;
const MAX_SOURCE_POOL_BYTES: usize = 128;
const MAX_LEDGER_ROOT_BYTES: usize = 64;
const MAX_MINOR_UNIT_DIGITS: usize = 20;

/// Deterministic validation failures for Phase 16 payout evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EpochPayoutValidationError {
    /// Schema did not match the required value.
    #[error("invalid schema for {field}: expected {expected}, got {actual}")]
    InvalidSchema {
        /// Field being validated.
        field: &'static str,
        /// Required schema.
        expected: &'static str,
        /// Supplied schema.
        actual: String,
    },

    /// Version did not match the current version.
    #[error("invalid version for {field}: expected {expected}, got {actual}")]
    InvalidVersion {
        /// Field being validated.
        field: &'static str,
        /// Required version.
        expected: u16,
        /// Supplied version.
        actual: u16,
    },

    /// Required field was empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Field being validated.
        field: &'static str,
    },

    /// Field exceeded its byte bound.
    #[error("{field} exceeds maximum bytes: max={max}, actual={actual}")]
    FieldTooLong {
        /// Field being validated.
        field: &'static str,
        /// Maximum bytes.
        max: usize,
        /// Actual bytes.
        actual: usize,
    },

    /// Identifier contained unsupported characters.
    #[error("{field} contains unsupported characters")]
    InvalidToken {
        /// Field being validated.
        field: &'static str,
    },

    /// Decimal minor-unit amount was malformed.
    #[error("invalid minor-unit amount for {field}: {reason}")]
    InvalidMoney {
        /// Field being validated.
        field: &'static str,
        /// Stable failure reason.
        reason: &'static str,
    },

    /// Quorum structure failed validation.
    #[error("invalid quorum signature set: {reason}")]
    InvalidQuorum {
        /// Bounded quorum validation message.
        reason: String,
    },

    /// One operation field did not match its quorum or ledger binding.
    #[error("epoch payout binding mismatch: {field}")]
    BindingMismatch {
        /// Field or relationship that mismatched.
        field: &'static str,
    },

    /// Duplicate replay identity was observed.
    #[error("duplicate epoch payout identity: {field}")]
    Duplicate {
        /// Duplicate field.
        field: &'static str,
    },

    /// Receipt sequence order was invalid.
    #[error("receipt sequence order is not strictly increasing")]
    InvalidSequenceOrder,

    /// Checked arithmetic overflowed.
    #[error("epoch payout arithmetic overflow")]
    ArithmeticOverflow,

    /// Replayed balances did not conserve against issued supply.
    #[error("epoch payout replay conservation failure")]
    ConservationFailure,

    /// Receipt hash could not be encoded or did not match.
    #[error("epoch payout receipt hash failure: {reason}")]
    ReceiptHash {
        /// Stable hash failure reason.
        reason: String,
    },
}

/// One wallet/ledger operation derived from an accepted epoch transition.
///
/// The full quorum signature set is retained so the ledger record remains bound
/// to the exact transition authorization material used by `svc-wallet`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochPayoutOperationV1 {
    /// Strict schema identifier.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Deterministic backend operation identity.
    pub operation_id: String,
    /// Retry identity. It is not supply authority.
    pub idempotency_key: String,
    /// Accepted epoch identity.
    pub epoch_id: String,
    /// Reward pool/category that funded the payout.
    pub source_pool: String,
    /// Service node that earned the allocation, when applicable.
    pub service_node_id: Option<String>,
    /// Registry-resolved canonical recipient account.
    pub recipient_account_id: String,
    /// Positive ROC minor-unit amount encoded as a canonical decimal string.
    pub amount_minor: String,
    /// Accepted Phase 15 transition hash.
    pub transition_hash: ContentId,
    /// Policy approval hash accepted by quorum.
    pub policy_approval_hash: ContentId,
    /// Canonical economics configuration hash accepted by quorum.
    pub economics_config_hash: ContentId,
    /// Reward-plan hash accepted by quorum.
    pub reward_plan_hash: ContentId,
    /// Accounting snapshot hash accepted by quorum.
    pub accounting_snapshot_hash: ContentId,
    /// Registry root used for recipient resolution.
    pub registry_hash: ContentId,
    /// Reward-binding root used for recipient resolution.
    pub reward_binding_hash: ContentId,
    /// Evidence root supporting the allocation.
    pub evidence_root: ContentId,
    /// Exact structural and cryptographically verified quorum set.
    pub quorum_signature_set: ServiceNodeQuorumV1,
    /// Deterministic transition-supplied timestamp.
    pub submitted_at_ms: u64,
}

impl EpochPayoutOperationV1 {
    /// Validate operation shape and its binding to the supplied quorum.
    pub fn validate(&self) -> Result<(), EpochPayoutValidationError> {
        validate_schema(
            "EpochPayoutOperationV1.schema",
            &self.schema,
            EPOCH_PAYOUT_OPERATION_SCHEMA,
        )?;
        validate_version("EpochPayoutOperationV1.version", self.version)?;

        validate_token("operation_id", &self.operation_id, MAX_TOKEN_BYTES)?;
        validate_token("idempotency_key", &self.idempotency_key, MAX_TOKEN_BYTES)?;
        validate_token("epoch_id", &self.epoch_id, MAX_TOKEN_BYTES)?;
        validate_token("source_pool", &self.source_pool, MAX_SOURCE_POOL_BYTES)?;
        validate_token(
            "recipient_account_id",
            &self.recipient_account_id,
            MAX_TOKEN_BYTES,
        )?;

        if let Some(service_node_id) = &self.service_node_id {
            validate_token("service_node_id", service_node_id, MAX_TOKEN_BYTES)?;
        }

        let _ = self.amount_u64()?;

        if self.submitted_at_ms == 0 {
            return Err(EpochPayoutValidationError::BindingMismatch {
                field: "submitted_at_ms",
            });
        }

        self.quorum_signature_set.validate().map_err(|error| {
            EpochPayoutValidationError::InvalidQuorum {
                reason: error.to_string(),
            }
        })?;

        if self.quorum_signature_set.epoch_id != self.epoch_id {
            return Err(EpochPayoutValidationError::BindingMismatch {
                field: "quorum_signature_set.epoch_id",
            });
        }

        if self.quorum_signature_set.transition_hash != self.transition_hash {
            return Err(EpochPayoutValidationError::BindingMismatch {
                field: "quorum_signature_set.transition_hash",
            });
        }

        if let Some(service_node_id) = &self.service_node_id {
            let eligible = self
                .quorum_signature_set
                .eligibilities
                .iter()
                .any(|eligibility| eligibility.service_node_id == *service_node_id);

            if !eligible {
                return Err(EpochPayoutValidationError::BindingMismatch {
                    field: "service_node_id eligibility",
                });
            }
        }

        Ok(())
    }

    /// Parse the canonical amount into the current primitive ledger ceiling.
    pub fn amount_u64(&self) -> Result<u64, EpochPayoutValidationError> {
        parse_minor_units_u64("amount_minor", &self.amount_minor)
    }
}

/// Accepted ledger evidence for one epoch payout entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpochPayoutReceiptV1 {
    /// Strict schema identifier.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Exact accepted operation.
    pub operation: EpochPayoutOperationV1,
    /// Primitive ledger sequence assigned to this payout entry.
    pub ledger_seq: u64,
    /// Ledger accumulator root after the complete atomic payout batch.
    pub ledger_root: String,
    /// Deterministic accepted timestamp copied from transition material.
    pub accepted_at_ms: u64,
    /// BLAKE3 hash over all preceding receipt fields.
    pub receipt_hash: ContentId,
}

impl EpochPayoutReceiptV1 {
    /// Construct and hash an accepted receipt.
    pub fn new(
        operation: EpochPayoutOperationV1,
        ledger_seq: u64,
        ledger_root: String,
        accepted_at_ms: u64,
    ) -> Result<Self, EpochPayoutValidationError> {
        let mut receipt = Self {
            schema: EPOCH_PAYOUT_RECEIPT_SCHEMA.to_owned(),
            version: EPOCH_PAYOUT_VERSION,
            operation,
            ledger_seq,
            ledger_root,
            accepted_at_ms,
            receipt_hash: zero_content_id()?,
        };

        receipt.receipt_hash = receipt.compute_receipt_hash()?;
        receipt.validate()?;
        Ok(receipt)
    }

    /// Validate operation, ledger fields, and receipt hash.
    pub fn validate(&self) -> Result<(), EpochPayoutValidationError> {
        validate_schema(
            "EpochPayoutReceiptV1.schema",
            &self.schema,
            EPOCH_PAYOUT_RECEIPT_SCHEMA,
        )?;
        validate_version("EpochPayoutReceiptV1.version", self.version)?;
        self.operation.validate()?;

        if self.ledger_seq == 0 {
            return Err(EpochPayoutValidationError::BindingMismatch {
                field: "ledger_seq",
            });
        }

        validate_ledger_root(&self.ledger_root)?;

        if self.accepted_at_ms == 0 || self.accepted_at_ms != self.operation.submitted_at_ms {
            return Err(EpochPayoutValidationError::BindingMismatch {
                field: "accepted_at_ms",
            });
        }

        let expected = self.compute_receipt_hash()?;
        if expected != self.receipt_hash {
            return Err(EpochPayoutValidationError::ReceiptHash {
                reason: "receipt_hash mismatch".to_owned(),
            });
        }

        Ok(())
    }

    fn compute_receipt_hash(&self) -> Result<ContentId, EpochPayoutValidationError> {
        #[derive(Serialize)]
        struct ReceiptPreimage<'a> {
            schema: &'a str,
            version: u16,
            operation: &'a EpochPayoutOperationV1,
            ledger_seq: u64,
            ledger_root: &'a str,
            accepted_at_ms: u64,
        }

        let encoded = serde_json::to_vec(&ReceiptPreimage {
            schema: &self.schema,
            version: self.version,
            operation: &self.operation,
            ledger_seq: self.ledger_seq,
            ledger_root: &self.ledger_root,
            accepted_at_ms: self.accepted_at_ms,
        })
        .map_err(|error| EpochPayoutValidationError::ReceiptHash {
            reason: error.to_string(),
        })?;

        ContentId::parse(&format!("b3:{}", blake3::hash(&encoded).to_hex())).map_err(|error| {
            EpochPayoutValidationError::ReceiptHash {
                reason: error.to_string(),
            }
        })
    }
}

/// Deterministic replay summary for accepted epoch payout receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochPayoutReplaySummaryV1 {
    /// Number of replayed receipts.
    pub receipt_count: usize,
    /// Total ROC issued by the replayed receipts.
    pub total_issued_minor: u128,
    /// Reconstructed recipient balances in canonical account order.
    pub balances: BTreeMap<String, u128>,
    /// Last primitive ledger sequence in the replay.
    pub last_ledger_seq: u64,
    /// Ledger root reported by the final receipt.
    pub last_ledger_root: String,
}

/// Replay accepted payout receipts and prove duplicate rejection and conservation.
pub fn replay_epoch_payout_receipts(
    receipts: &[EpochPayoutReceiptV1],
) -> Result<EpochPayoutReplaySummaryV1, EpochPayoutValidationError> {
    if receipts.is_empty() {
        return Err(EpochPayoutValidationError::EmptyField { field: "receipts" });
    }

    let mut operation_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();
    let mut balances = BTreeMap::<String, u128>::new();
    let mut total_issued_minor = 0_u128;
    let mut last_ledger_seq = 0_u64;
    let mut last_ledger_root = String::new();

    for receipt in receipts {
        receipt.validate()?;

        if receipt.ledger_seq <= last_ledger_seq {
            return Err(EpochPayoutValidationError::InvalidSequenceOrder);
        }

        if !operation_ids.insert(receipt.operation.operation_id.as_str()) {
            return Err(EpochPayoutValidationError::Duplicate {
                field: "operation_id",
            });
        }

        if !idempotency_keys.insert(receipt.operation.idempotency_key.as_str()) {
            return Err(EpochPayoutValidationError::Duplicate {
                field: "idempotency_key",
            });
        }

        let amount = u128::from(receipt.operation.amount_u64()?);

        total_issued_minor = total_issued_minor
            .checked_add(amount)
            .ok_or(EpochPayoutValidationError::ArithmeticOverflow)?;

        let balance = balances
            .entry(receipt.operation.recipient_account_id.clone())
            .or_default();

        *balance = balance
            .checked_add(amount)
            .ok_or(EpochPayoutValidationError::ArithmeticOverflow)?;

        last_ledger_seq = receipt.ledger_seq;
        last_ledger_root = receipt.ledger_root.clone();
    }

    let reconstructed_total = balances.values().try_fold(0_u128, |sum, balance| {
        sum.checked_add(*balance)
            .ok_or(EpochPayoutValidationError::ArithmeticOverflow)
    })?;

    if reconstructed_total != total_issued_minor {
        return Err(EpochPayoutValidationError::ConservationFailure);
    }

    Ok(EpochPayoutReplaySummaryV1 {
        receipt_count: receipts.len(),
        total_issued_minor,
        balances,
        last_ledger_seq,
        last_ledger_root,
    })
}

fn validate_schema(
    field: &'static str,
    actual: &str,
    expected: &'static str,
) -> Result<(), EpochPayoutValidationError> {
    if actual == expected {
        return Ok(());
    }

    Err(EpochPayoutValidationError::InvalidSchema {
        field,
        expected,
        actual: actual.to_owned(),
    })
}

fn validate_version(field: &'static str, actual: u16) -> Result<(), EpochPayoutValidationError> {
    if actual == EPOCH_PAYOUT_VERSION {
        return Ok(());
    }

    Err(EpochPayoutValidationError::InvalidVersion {
        field,
        expected: EPOCH_PAYOUT_VERSION,
        actual,
    })
}

fn validate_token(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), EpochPayoutValidationError> {
    if value.trim().is_empty() {
        return Err(EpochPayoutValidationError::EmptyField { field });
    }

    if value.len() > max {
        return Err(EpochPayoutValidationError::FieldTooLong {
            field,
            max,
            actual: value.len(),
        });
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return Err(EpochPayoutValidationError::InvalidToken { field });
    }

    Ok(())
}

fn parse_minor_units_u64(
    field: &'static str,
    value: &str,
) -> Result<u64, EpochPayoutValidationError> {
    if value.is_empty() {
        return Err(EpochPayoutValidationError::InvalidMoney {
            field,
            reason: "must not be empty",
        });
    }

    if value.len() > MAX_MINOR_UNIT_DIGITS {
        return Err(EpochPayoutValidationError::InvalidMoney {
            field,
            reason: "exceeds current primitive ledger u64 width",
        });
    }

    if value.len() > 1 && value.starts_with('0') {
        return Err(EpochPayoutValidationError::InvalidMoney {
            field,
            reason: "must not contain leading zeroes",
        });
    }

    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(EpochPayoutValidationError::InvalidMoney {
            field,
            reason: "must contain decimal digits only",
        });
    }

    let amount = value
        .parse::<u64>()
        .map_err(|_| EpochPayoutValidationError::InvalidMoney {
            field,
            reason: "must fit in the current primitive ledger u64 type",
        })?;

    if amount == 0 {
        return Err(EpochPayoutValidationError::InvalidMoney {
            field,
            reason: "must be greater than zero",
        });
    }

    Ok(amount)
}

fn validate_ledger_root(root: &str) -> Result<(), EpochPayoutValidationError> {
    if root.len() != MAX_LEDGER_ROOT_BYTES
        || !root
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(EpochPayoutValidationError::BindingMismatch {
            field: "ledger_root",
        });
    }

    Ok(())
}

fn zero_content_id() -> Result<ContentId, EpochPayoutValidationError> {
    ContentId::parse(&format!("b3:{}", "0".repeat(64))).map_err(|error| {
        EpochPayoutValidationError::ReceiptHash {
            reason: error.to_string(),
        }
    })
}
