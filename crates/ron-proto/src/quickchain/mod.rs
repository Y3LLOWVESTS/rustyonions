//! RO:WHAT — Strict QuickChain Phase 0 DTOs for future ROC checkpoint/proof contracts.
//! RO:WHY — Pillar 12 / ECON+GOV: define wire-safe shapes before roots, validators, pruning, or anchors exist.
//! RO:INTERACTS — id::ContentId, quantum::SignatureAlg, future ron-ledger roots, svc-wallet receipts, ron-accounting snapshots.
//! RO:INVARIANTS — DTO-only; no IO; no crypto; no wallet/ledger mutation; no consensus; integer minor units as canonical strings.
//! RO:METRICS — none.
//! RO:CONFIG — none; runtime QuickChain remains disabled elsewhere.
//! RO:SECURITY — no private keys, no fake proofs, no external anchors, no signature verification shortcuts.
//! RO:TEST — tests/quickchain_dto_strict.rs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{id::ContentId, quantum::SignatureAlg};

pub mod canonical;
pub mod domain;
pub mod empty_tree;
pub mod event_class;
pub mod hash_payload;
pub mod hold;
pub mod hold_scenario;
pub mod ids;
pub mod money;
pub mod operation;
pub mod receipt;
pub mod receipt_order;
pub mod replay;
pub mod root_material;
pub mod sort_key;
pub mod vector;
pub use canonical::*;
pub use domain::*;
pub use empty_tree::*;
pub use event_class::*;
pub use hash_payload::*;
pub use hold::*;
pub use hold_scenario::*;
pub use ids::*;
pub use money::*;
pub use operation::*;
pub use receipt::*;
pub use receipt_order::*;
pub use replay::*;
pub use root_material::*;
pub use sort_key::*;
pub use vector::*;

/// Current QuickChain DTO version.
pub const QUICKCHAIN_DTO_VERSION: u16 = 1;

/// Schema tag for checkpoint headers.
pub const QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA: &str = "quickchain.checkpoint_header.v1";

/// Schema tag for validator signature DTOs.
pub const QUICKCHAIN_VALIDATOR_SIGNATURE_SCHEMA: &str = "quickchain.validator_signature.v1";

/// Schema tag for account state DTOs.
pub const QUICKCHAIN_ACCOUNT_STATE_SCHEMA: &str = "quickchain.account_state.v1";

/// Schema tag for receipt inclusion proof DTOs.
pub const QUICKCHAIN_RECEIPT_PROOF_SCHEMA: &str = "quickchain.receipt_proof.v1";

/// Schema tag for chain parameter DTOs.
pub const QUICKCHAIN_CHAIN_PARAMS_SCHEMA: &str = "quickchain.chain_params.v1";

/// Schema tag for challenge DTOs.
pub const QUICKCHAIN_CHALLENGE_SCHEMA: &str = "quickchain.challenge.v1";

/// Maximum chain id length.
pub const MAX_QUICKCHAIN_CHAIN_ID_BYTES: usize = 64;

/// Maximum epoch id length.
pub const MAX_QUICKCHAIN_EPOCH_ID_BYTES: usize = 96;

/// Maximum public validator/account/key reference length.
pub const MAX_QUICKCHAIN_REF_BYTES: usize = 256;

/// Maximum signature wire string length.
pub const MAX_QUICKCHAIN_SIGNATURE_BYTES: usize = 2048;

/// Maximum validator signatures carried by one DTO.
pub const MAX_QUICKCHAIN_SIGNATURES: usize = 128;

/// Maximum Merkle proof siblings carried by one proof DTO.
pub const MAX_QUICKCHAIN_MERKLE_PATH: usize = 256;

/// One basis-point denominator.
pub const QUICKCHAIN_BPS_DENOMINATOR: u16 = 10_000;

/// Result type used by QuickChain DTO validation.
pub type QuickChainResult<T> = Result<T, QuickChainValidationError>;

/// Canonical encoding tag for hashed QuickChain payloads.
///
/// This is descriptive in Phase 0. Actual canonical byte generation belongs in a
/// later root/canonicalizer module, not in this DTO module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QuickChainCanonicalEncodingV1 {
    /// Human-reviewable canonical JSON experiment.
    #[serde(rename = "json-v1")]
    JsonV1,

    /// Reserved future CBOR option.
    #[serde(rename = "cbor-v1")]
    CborV1,

    /// Reserved future fixed binary option.
    #[serde(rename = "fixed-binary-v1")]
    FixedBinaryV1,
}

/// Future account state root scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainStateRootSchemeV1 {
    /// Sorted Merkle map over canonical account-state leaves.
    SortedMerkleMapV1,
}

/// Future receipt root scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainReceiptRootSchemeV1 {
    /// Ledger-sequence-ordered Merkle root over receipt hashes.
    LedgerSequenceMerkleV1,
}

/// Merkle proof sibling side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainMerkleSideV1 {
    /// Sibling hash is on the left side.
    Left,

    /// Sibling hash is on the right side.
    Right,
}

/// Future challenge category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainChallengeTypeV1 {
    InvalidStateRoot,
    InvalidReceiptRoot,
    MissingData,
    InvalidRewardRoot,
    InvalidPolicyHash,
    InvalidChainParamsHash,
    UnauthorizedIssue,
    UnauthorizedBurn,
    DoubleSpend,
    DuplicateOperationCommit,
    ValidatorEquivocation,
    CarrierFailure,
    RawEngagementRewardAbuse,
}

/// Validator signature DTO.
///
/// This carries signature material as data only. `ron-proto` does not verify
/// signatures and must not own private key custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainValidatorSignatureV1 {
    pub schema: String,
    pub version: u16,
    pub validator_id: String,
    pub key_id: String,
    pub algorithm: SignatureAlg,
    pub checkpoint_hash: ContentId,
    pub signature_wire: String,
}

impl QuickChainValidatorSignatureV1 {
    /// Validate DTO shape only. This does not verify cryptographic signatures.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainValidatorSignatureV1.schema",
            &self.schema,
            QUICKCHAIN_VALIDATOR_SIGNATURE_SCHEMA,
        )?;
        validate_version("QuickChainValidatorSignatureV1.version", self.version)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("key_id", &self.key_id)?;
        validate_bounded_nonempty(
            "signature_wire",
            &self.signature_wire,
            MAX_QUICKCHAIN_SIGNATURE_BYTES,
        )
    }
}

/// Compact future checkpoint header DTO.
///
/// This is not a live checkpoint and does not create chain state. It is the
/// future wire shape that root/signature code can target once internal ROC is
/// proven.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainCheckpointHeaderV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub height: u64,
    pub epoch_id: String,
    pub previous_checkpoint_hash: ContentId,
    pub previous_state_root: ContentId,
    pub new_state_root: ContentId,
    pub receipt_root: ContentId,
    pub accounting_snapshot_root: ContentId,
    pub reward_manifest_root: ContentId,
    pub data_availability_root: ContentId,
    pub policy_hash: ContentId,
    pub validator_set_hash: ContentId,
    pub chain_params_hash: ContentId,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub state_root_scheme: QuickChainStateRootSchemeV1,
    pub receipt_root_scheme: QuickChainReceiptRootSchemeV1,
    pub supply_delta_minor_units: String,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
    pub produced_at_ms: u64,
    #[serde(default)]
    pub signatures: Vec<QuickChainValidatorSignatureV1>,
}

impl QuickChainCheckpointHeaderV1 {
    /// Validate DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainCheckpointHeaderV1.schema",
            &self.schema,
            QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA,
        )?;
        validate_version("QuickChainCheckpointHeaderV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_money_minor_units("supply_delta_minor_units", &self.supply_delta_minor_units)?;
        validate_timestamp_order(
            "checkpoint_time_range",
            self.started_at_ms,
            self.ended_at_ms,
            self.produced_at_ms,
        )?;
        validate_signature_vec(&self.signatures)
    }
}

/// Account state DTO used by future local state-root experiments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainAccountStateV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub account_id: String,
    pub available_minor_units: String,
    pub held_minor_units: String,
    pub nonce: u64,
    pub last_ledger_seq: u64,
}

impl QuickChainAccountStateV1 {
    /// Validate DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainAccountStateV1.schema",
            &self.schema,
            QUICKCHAIN_ACCOUNT_STATE_SCHEMA,
        )?;
        validate_version("QuickChainAccountStateV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("account_id", &self.account_id)?;
        validate_money_minor_units("available_minor_units", &self.available_minor_units)?;
        validate_money_minor_units("held_minor_units", &self.held_minor_units)
    }
}

/// One Merkle path sibling for future inclusion proofs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainMerkleSiblingV1 {
    pub side: QuickChainMerkleSideV1,
    pub hash: ContentId,
}

/// Future receipt inclusion proof DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainReceiptProofV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub checkpoint_hash: ContentId,
    pub receipt_hash: ContentId,
    pub receipt_root: ContentId,
    pub ledger_seq: u64,
    #[serde(default)]
    pub merkle_path: Vec<QuickChainMerkleSiblingV1>,
}

impl QuickChainReceiptProofV1 {
    /// Validate DTO shape only. This does not verify the proof.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainReceiptProofV1.schema",
            &self.schema,
            QUICKCHAIN_RECEIPT_PROOF_SCHEMA,
        )?;
        validate_version("QuickChainReceiptProofV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_merkle_path(&self.merkle_path)
    }
}

/// Future chain params DTO.
///
/// This is descriptive data only. Runtime enabling and governance live outside
/// `ron-proto`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainChainParamsV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub enabled: bool,
    pub epoch_duration_ms: u64,
    pub checkpoint_cadence: u32,
    pub challenge_window_ms: u64,
    pub max_receipts_per_batch: u64,
    pub max_accounting_snapshot_bytes: u64,
    pub max_reward_manifest_bytes: u64,
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    pub state_root_scheme: QuickChainStateRootSchemeV1,
    pub receipt_root_scheme: QuickChainReceiptRootSchemeV1,
    pub quorum_bps: u16,
    pub min_validators: u16,
    pub max_validators: u16,
    pub passport_required: bool,
    pub bond_required: bool,
    pub rox_anchor_enabled: bool,
}

impl QuickChainChainParamsV1 {
    /// Validate DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainChainParamsV1.schema",
            &self.schema,
            QUICKCHAIN_CHAIN_PARAMS_SCHEMA,
        )?;
        validate_version("QuickChainChainParamsV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;

        if self.epoch_duration_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "epoch_duration_ms",
                reason: "must be greater than zero",
            });
        }

        if self.checkpoint_cadence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "checkpoint_cadence",
                reason: "must be greater than zero",
            });
        }

        if self.challenge_window_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "challenge_window_ms",
                reason: "must be greater than zero",
            });
        }

        if self.max_receipts_per_batch == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "max_receipts_per_batch",
                reason: "must be greater than zero",
            });
        }

        if self.max_accounting_snapshot_bytes == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "max_accounting_snapshot_bytes",
                reason: "must be greater than zero",
            });
        }

        if self.max_reward_manifest_bytes == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "max_reward_manifest_bytes",
                reason: "must be greater than zero",
            });
        }

        if self.quorum_bps == 0 || self.quorum_bps > QUICKCHAIN_BPS_DENOMINATOR {
            return Err(QuickChainValidationError::InvalidBps {
                field: "quorum_bps",
                actual: self.quorum_bps,
            });
        }

        // Zero/zero is the explicit no-committee posture used during
        // QuickChain Phase 0. Mixed zero/nonzero bounds are ambiguous.
        match (self.min_validators, self.max_validators) {
            (0, 0) => {}
            (0, _) | (_, 0) => {
                return Err(QuickChainValidationError::InvalidField {
                    field: "validator_bounds",
                    reason: "must both be zero or both be greater than zero",
                });
            }
            (min_validators, max_validators) if max_validators < min_validators => {
                return Err(QuickChainValidationError::InvalidField {
                    field: "max_validators",
                    reason: "must be greater than or equal to min_validators",
                });
            }
            _ => {}
        }

        Ok(())
    }

    /// Phase 0 safety gate: QuickChain and external anchors must be disabled.
    pub fn validate_phase0_disabled(&self) -> QuickChainResult<()> {
        self.validate()?;

        if self.enabled {
            return Err(QuickChainValidationError::InvalidField {
                field: "enabled",
                reason: "must remain false during Phase 0",
            });
        }

        if self.rox_anchor_enabled {
            return Err(QuickChainValidationError::InvalidField {
                field: "rox_anchor_enabled",
                reason: "must remain false during Phase 0",
            });
        }

        Ok(())
    }
}

/// Future challenge DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainChallengeV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub checkpoint_hash: ContentId,
    pub challenger_id: String,
    pub challenge_type: QuickChainChallengeTypeV1,
    pub evidence_cid: ContentId,
    pub submitted_at_ms: u64,
}

impl QuickChainChallengeV1 {
    /// Validate DTO shape only.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainChallengeV1.schema",
            &self.schema,
            QUICKCHAIN_CHALLENGE_SCHEMA,
        )?;
        validate_version("QuickChainChallengeV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("challenger_id", &self.challenger_id)?;

        if self.submitted_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_at_ms",
                reason: "must be greater than zero",
            });
        }

        Ok(())
    }
}

/// Deterministic validation errors for QuickChain DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QuickChainValidationError {
    #[error("invalid schema for {field}: expected {expected}, got {actual}")]
    InvalidSchema {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },

    #[error("invalid version for {field}: expected {expected}, got {actual}")]
    InvalidVersion {
        field: &'static str,
        expected: u16,
        actual: u16,
    },

    #[error("empty field: {field}")]
    EmptyField { field: &'static str },

    #[error("field too long: {field} max={max} actual={actual}")]
    FieldTooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    #[error("invalid token field: {field}")]
    InvalidToken { field: &'static str },

    #[error("invalid money field: {field} reason={reason}")]
    InvalidMoney {
        field: &'static str,
        reason: &'static str,
    },

    #[error("invalid timestamp order: {field}")]
    InvalidTimestampOrder { field: &'static str },

    #[error("too many items: {field} max={max} actual={actual}")]
    TooManyItems {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    #[error("invalid basis points: {field} actual={actual}")]
    InvalidBps { field: &'static str, actual: u16 },

    #[error("invalid field: {field} reason={reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
}

fn validate_schema(
    field: &'static str,
    actual: &str,
    expected: &'static str,
) -> QuickChainResult<()> {
    if actual == expected {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidSchema {
        field,
        expected,
        actual: actual.to_string(),
    })
}

fn validate_version(field: &'static str, actual: u16) -> QuickChainResult<()> {
    if actual == QUICKCHAIN_DTO_VERSION {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidVersion {
        field,
        expected: QUICKCHAIN_DTO_VERSION,
        actual,
    })
}

fn validate_chain_id(value: &str) -> QuickChainResult<()> {
    validate_token("chain_id", value, MAX_QUICKCHAIN_CHAIN_ID_BYTES)
}

fn validate_epoch_id(value: &str) -> QuickChainResult<()> {
    validate_token("epoch_id", value, MAX_QUICKCHAIN_EPOCH_ID_BYTES)
}

fn validate_ref(field: &'static str, value: &str) -> QuickChainResult<()> {
    validate_token(field, value, MAX_QUICKCHAIN_REF_BYTES)
}

fn validate_token(field: &'static str, value: &str, max: usize) -> QuickChainResult<()> {
    validate_bounded_nonempty(field, value, max)?;

    if !value.bytes().all(is_allowed_token_byte) {
        return Err(QuickChainValidationError::InvalidToken { field });
    }

    Ok(())
}

fn is_allowed_token_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase()
        || byte.is_ascii_digit()
        || matches!(byte, b'_' | b'-' | b'.' | b':' | b'@' | b'/')
}

fn validate_bounded_nonempty(field: &'static str, value: &str, max: usize) -> QuickChainResult<()> {
    if value.trim().is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    let actual = value.len();
    if actual > max {
        return Err(QuickChainValidationError::FieldTooLong { field, max, actual });
    }

    Ok(())
}

fn validate_money_minor_units(field: &'static str, value: &str) -> QuickChainResult<()> {
    money::validate_quickchain_minor_units(field, value)
}

fn validate_timestamp_order(
    field: &'static str,
    started_at_ms: u64,
    ended_at_ms: u64,
    produced_at_ms: u64,
) -> QuickChainResult<()> {
    if started_at_ms == 0 || ended_at_ms == 0 || produced_at_ms == 0 {
        return Err(QuickChainValidationError::InvalidTimestampOrder { field });
    }

    if ended_at_ms < started_at_ms || produced_at_ms < ended_at_ms {
        return Err(QuickChainValidationError::InvalidTimestampOrder { field });
    }

    Ok(())
}

fn validate_signature_vec(signatures: &[QuickChainValidatorSignatureV1]) -> QuickChainResult<()> {
    if signatures.len() > MAX_QUICKCHAIN_SIGNATURES {
        return Err(QuickChainValidationError::TooManyItems {
            field: "signatures",
            max: MAX_QUICKCHAIN_SIGNATURES,
            actual: signatures.len(),
        });
    }

    for signature in signatures {
        signature.validate()?;
    }

    Ok(())
}

fn validate_merkle_path(path: &[QuickChainMerkleSiblingV1]) -> QuickChainResult<()> {
    if path.len() > MAX_QUICKCHAIN_MERKLE_PATH {
        return Err(QuickChainValidationError::TooManyItems {
            field: "merkle_path",
            max: MAX_QUICKCHAIN_MERKLE_PATH,
            actual: path.len(),
        });
    }

    Ok(())
}
