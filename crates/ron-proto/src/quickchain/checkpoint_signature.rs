//! RO:WHAT — FINAL_BETA Phase 19 checkpoint-validator signing payload and signature DTO contract.
//! RO:WHY — ECON/GOV: validator signatures must bind one exact unsigned checkpoint candidate before committee validation or finality.
//! RO:INTERACTS — committee checkpoint payloads, checkpoint candidate hashes, validator-set identities, SignatureAlg, canonical JSON.
//! RO:INVARIANTS — fixed signing domain; chain/height/epoch/checkpoint/validator/key/algorithm all signed; signature wire excluded from its own preimage.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — DTO/canonical-message construction only; no private keys, crypto execution, quorum acceptance, finality, wallet mutation, or ledger mutation.
//! RO:TEST — tests/final_beta_phase19_checkpoint_validator_signature_contract.rs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{id::ContentId, quantum::SignatureAlg};

use super::{
    to_canonical_json_vec, validate_bounded_nonempty, validate_chain_id, validate_epoch_id,
    validate_ref, validate_schema, validate_version, QuickChainCanonicalError, QuickChainResult,
    QuickChainValidationError, MAX_QUICKCHAIN_SIGNATURE_BYTES, QUICKCHAIN_DTO_VERSION,
};

/// Schema for the exact checkpoint-validator signing payload.
pub const QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA: &str =
    "quickchain.checkpoint-validator-signing-payload.v1";

/// Schema for one checkpoint-validator signature artifact.
pub const QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA: &str =
    "quickchain.checkpoint-validator-signature.v1";

/// Fixed domain embedded in every checkpoint-validator signing message.
///
/// Callers cannot substitute another signing domain through the public DTO.
pub const QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_DOMAIN: &str =
    "quickchain.checkpoint-validator-signature.v1";

/// Exact pre-signature facts one validator must sign for a checkpoint candidate.
///
/// This payload deliberately contains no signature bytes and no finality state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainCheckpointValidatorSigningPayloadV1 {
    /// Exact signing-payload schema.
    pub schema: String,

    /// QuickChain DTO version.
    pub version: u16,

    /// Chain whose checkpoint candidate is being signed.
    pub chain_id: String,

    /// Non-zero checkpoint height.
    pub height: u64,

    /// Epoch whose checkpoint candidate is being signed.
    pub epoch_id: String,

    /// Exact deterministic unsigned checkpoint candidate hash.
    pub checkpoint_hash: ContentId,

    /// Validator identity producing the signature.
    pub validator_id: String,

    /// Reviewed validator key identity.
    pub key_id: String,

    /// Signature algorithm expected for this validator key.
    pub algorithm: SignatureAlg,
}

impl QuickChainCheckpointValidatorSigningPayloadV1 {
    /// Validate signing-payload shape before canonical message construction.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainCheckpointValidatorSigningPayloadV1.schema",
            &self.schema,
            QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA,
        )?;

        validate_version(
            "QuickChainCheckpointValidatorSigningPayloadV1.version",
            self.version,
        )?;

        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("validator_id", &self.validator_id)?;
        validate_ref("key_id", &self.key_id)?;

        if self.height == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "height",
                reason: "checkpoint height must be greater than zero",
            });
        }

        Ok(())
    }
}

/// One validator signature bound to one exact checkpoint candidate context.
///
/// This is signature evidence only. It does not state that a validator is
/// eligible, that the signature has been cryptographically verified, that a
/// quorum exists, or that the checkpoint is final.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainCheckpointValidatorSignatureV1 {
    /// Exact signature artifact schema.
    pub schema: String,

    /// QuickChain DTO version.
    pub version: u16,

    /// Chain whose checkpoint candidate was signed.
    pub chain_id: String,

    /// Checkpoint height whose candidate was signed.
    pub height: u64,

    /// Epoch whose checkpoint candidate was signed.
    pub epoch_id: String,

    /// Exact unsigned checkpoint candidate hash that was signed.
    pub checkpoint_hash: ContentId,

    /// Validator identity that claims this signature.
    pub validator_id: String,

    /// Reviewed signing-key identity.
    pub key_id: String,

    /// Signature algorithm used by the signing key.
    pub algorithm: SignatureAlg,

    /// Bounded wire representation of the resulting signature.
    pub signature_wire: String,
}

impl QuickChainCheckpointValidatorSignatureV1 {
    /// Validate signature artifact shape only.
    ///
    /// Cryptographic verification and validator-set eligibility are deliberately
    /// deferred to the committee-validation layer.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainCheckpointValidatorSignatureV1.schema",
            &self.schema,
            QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA,
        )?;

        self.signing_payload().validate()?;

        validate_bounded_nonempty(
            "signature_wire",
            &self.signature_wire,
            MAX_QUICKCHAIN_SIGNATURE_BYTES,
        )
    }

    /// Reconstruct the exact signature-free payload this signature must cover.
    #[must_use]
    pub fn signing_payload(&self) -> QuickChainCheckpointValidatorSigningPayloadV1 {
        QuickChainCheckpointValidatorSigningPayloadV1 {
            schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA.to_owned(),
            version: self.version,
            chain_id: self.chain_id.clone(),
            height: self.height,
            epoch_id: self.epoch_id.clone(),
            checkpoint_hash: self.checkpoint_hash.clone(),
            validator_id: self.validator_id.clone(),
            key_id: self.key_id.clone(),
            algorithm: self.algorithm,
        }
    }
}

/// Errors returned while constructing canonical checkpoint-validator signing bytes.
#[derive(Debug, Error)]
pub enum QuickChainCheckpointValidatorSigningError {
    /// The typed signing payload itself is invalid.
    #[error("invalid checkpoint-validator signing payload: {0}")]
    InvalidPayload(#[from] QuickChainValidationError),

    /// Canonical JSON serialization failed.
    #[error("checkpoint-validator canonical signing message failed: {0}")]
    Canonical(#[from] QuickChainCanonicalError),
}

/// Construct the exact canonical bytes a validator signs.
///
/// The message contains a fixed protocol domain followed by every binding that
/// prevents a valid signature from being replayed as a signature for a
/// different chain, height, epoch, checkpoint candidate, validator identity,
/// key identity, or algorithm.
///
/// Signature bytes are structurally absent from this message.
///
/// # Errors
///
/// Returns an error if the typed payload is invalid or canonical JSON encoding
/// fails.
pub fn checkpoint_validator_signature_message_bytes(
    payload: &QuickChainCheckpointValidatorSigningPayloadV1,
) -> Result<Vec<u8>, QuickChainCheckpointValidatorSigningError> {
    payload.validate()?;

    #[derive(Serialize)]
    struct SigningMessage<'a> {
        domain: &'static str,
        version: u16,
        chain_id: &'a str,
        height: u64,
        epoch_id: &'a str,
        checkpoint_hash: &'a ContentId,
        validator_id: &'a str,
        key_id: &'a str,
        algorithm: SignatureAlg,
    }

    let message = SigningMessage {
        domain: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_DOMAIN,
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: &payload.chain_id,
        height: payload.height,
        epoch_id: &payload.epoch_id,
        checkpoint_hash: &payload.checkpoint_hash,
        validator_id: &payload.validator_id,
        key_id: &payload.key_id,
        algorithm: payload.algorithm,
    };

    Ok(to_canonical_json_vec(&message)?)
}
