//! RO:WHAT — Produces and verifies one real Ed25519 signature for one deterministic QuickChain checkpoint candidate.
//! RO:WHY — FINAL_BETA Phase 19 requires independently cryptographic checkpoint-validator signatures before committee threshold/finality.
//! RO:INTERACTS — ron-proto checkpoint signature contract, QuickChainValidatorIdentityV1, ron-kms Signer/Verifier/KeyId.
//! RO:INVARIANTS — reviewed active identity only; Ed25519 only; exact canonical message bytes; one participant creates one signature.
//! RO:METRICS — none.
//! RO:CONFIG — none; concrete signing key is injected by the caller.
//! RO:SECURITY — owns no private-key export, quorum aggregation, finality, wallet mutation, ledger mutation, payout, receipt, or paid-unlock authority.
//! RO:TEST — focused unit tests in this module.

#![forbid(unsafe_code)]

use std::{error::Error as StdError, fmt, sync::Arc};

use ron_kms::{Alg, KeyId, Signer, Verifier};
use ron_proto::{
    quantum::SignatureAlg,
    quickchain::{
        checkpoint_validator_signature_message_bytes, QuickChainCheckpointValidatorSignatureV1,
        QuickChainCheckpointValidatorSigningPayloadV1, QuickChainValidatorIdentityV1,
        QuickChainValidatorLifecycleStatusV1, QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA,
        QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};

const ED25519_SIGNATURE_BYTES: usize = 64;
const ED25519_SIGNATURE_HEX_LEN: usize = ED25519_SIGNATURE_BYTES * 2;

/// One locally configured checkpoint validator backed by one concrete KMS key.
///
/// This type is intentionally narrower than committee membership. Construction
/// proves only that the supplied identity is structurally valid, active, and
/// compatible with the concrete Ed25519 signing key. It does not prove that the
/// validator belongs to the final reviewed committee used for threshold
/// acceptance; that is a later Phase 19 layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointValidatorParticipant {
    chain_id: String,
    epoch_id: String,
    validator_id: String,
    logical_key_id: String,
    algorithm: SignatureAlg,
    concrete_key: KeyId,
}

impl CheckpointValidatorParticipant {
    /// Construct a local signing participant from one reviewed validator
    /// identity and its injected concrete KMS key.
    ///
    /// # Errors
    ///
    /// Rejects malformed or non-active identities, unsupported signature
    /// algorithms, or a concrete key whose algorithm is not Ed25519.
    pub fn from_reviewed_identity(
        identity: &QuickChainValidatorIdentityV1,
        concrete_key: KeyId,
    ) -> Result<Self, CheckpointValidatorSigningError> {
        identity.validate().map_err(|error| {
            CheckpointValidatorSigningError::InvalidIdentity(format!("{error:?}"))
        })?;

        if identity.lifecycle_status != QuickChainValidatorLifecycleStatusV1::Active {
            return Err(CheckpointValidatorSigningError::ValidatorNotActive {
                validator_id: identity.validator_id.clone(),
            });
        }

        if identity.signature_algorithm != SignatureAlg::Ed25519 {
            return Err(CheckpointValidatorSigningError::UnsupportedAlgorithm {
                algorithm: identity.signature_algorithm,
            });
        }

        if concrete_key.alg != Alg::Ed25519 {
            return Err(CheckpointValidatorSigningError::ConcreteKeyAlgorithmMismatch);
        }

        Ok(Self {
            chain_id: identity.chain_id.clone(),
            epoch_id: identity.epoch_id.clone(),
            validator_id: identity.validator_id.clone(),
            logical_key_id: identity.key_id.clone(),
            algorithm: identity.signature_algorithm,
            concrete_key,
        })
    }

    /// Chain binding inherited from the reviewed validator identity.
    #[must_use]
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Epoch binding inherited from the reviewed validator identity.
    #[must_use]
    pub fn epoch_id(&self) -> &str {
        &self.epoch_id
    }

    /// Validator identity bound into every signature.
    #[must_use]
    pub fn validator_id(&self) -> &str {
        &self.validator_id
    }

    /// Logical reviewed key identity bound into every signature.
    #[must_use]
    pub fn logical_key_id(&self) -> &str {
        &self.logical_key_id
    }

    /// Produce exactly one real signature over the canonical checkpoint
    /// validator message.
    ///
    /// The checkpoint hash is an already-reproduced deterministic candidate
    /// hash. This method does not accept alternate message bytes or an
    /// alternate caller-provided signing preimage.
    ///
    /// # Errors
    ///
    /// Rejects invalid signing payloads, canonicalization failures, KMS signing
    /// failures, or an invalid resulting signature artifact.
    pub fn sign_checkpoint(
        &self,
        signer: &dyn Signer,
        height: u64,
        checkpoint_hash: ContentId,
    ) -> Result<QuickChainCheckpointValidatorSignatureV1, CheckpointValidatorSigningError> {
        let payload = QuickChainCheckpointValidatorSigningPayloadV1 {
            schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA.to_owned(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: self.chain_id.clone(),
            height,
            epoch_id: self.epoch_id.clone(),
            checkpoint_hash,
            validator_id: self.validator_id.clone(),
            key_id: self.logical_key_id.clone(),
            algorithm: self.algorithm,
        };

        let message = checkpoint_validator_signature_message_bytes(&payload)
            .map_err(|error| CheckpointValidatorSigningError::SigningMessage(error.to_string()))?;

        let raw_signature = signer
            .sign(&self.concrete_key, &message)
            .map_err(|error| CheckpointValidatorSigningError::Signing(error.to_string()))?;

        if raw_signature.len() != ED25519_SIGNATURE_BYTES {
            return Err(CheckpointValidatorSigningError::UnexpectedSignatureLength {
                actual: raw_signature.len(),
            });
        }

        let signature = QuickChainCheckpointValidatorSignatureV1 {
            schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA.to_owned(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: payload.chain_id,
            height: payload.height,
            epoch_id: payload.epoch_id,
            checkpoint_hash: payload.checkpoint_hash,
            validator_id: payload.validator_id,
            key_id: payload.key_id,
            algorithm: payload.algorithm,
            signature_wire: encode_lower_hex(&raw_signature),
        };

        signature.validate().map_err(|error| {
            CheckpointValidatorSigningError::InvalidSignatureArtifact(format!("{error:?}"))
        })?;

        Ok(signature)
    }

    /// Verify one signature with this participant's concrete KMS key.
    ///
    /// This checks only the cryptographic signature and this participant's
    /// exact identity/key bindings. It does not perform committee membership,
    /// duplicate-signer, threshold, quorum, or finality review.
    ///
    /// A well-formed signature that is cryptographically wrong returns
    /// `Ok(false)`. Malformed wire data returns an error.
    ///
    /// # Errors
    ///
    /// Rejects malformed signature artifacts, identity/key/context mismatch,
    /// malformed signature hex, or KMS verification failures.
    pub fn verify_signature(
        &self,
        verifier: &dyn Verifier,
        signature: &QuickChainCheckpointValidatorSignatureV1,
    ) -> Result<bool, CheckpointValidatorSigningError> {
        signature.validate().map_err(|error| {
            CheckpointValidatorSigningError::InvalidSignatureArtifact(format!("{error:?}"))
        })?;

        if signature.chain_id != self.chain_id {
            return Err(CheckpointValidatorSigningError::ChainMismatch);
        }

        if signature.epoch_id != self.epoch_id {
            return Err(CheckpointValidatorSigningError::EpochMismatch);
        }

        if signature.validator_id != self.validator_id {
            return Err(CheckpointValidatorSigningError::ValidatorMismatch);
        }

        if signature.key_id != self.logical_key_id {
            return Err(CheckpointValidatorSigningError::KeyIdentityMismatch);
        }

        if signature.algorithm != self.algorithm {
            return Err(CheckpointValidatorSigningError::SignatureAlgorithmMismatch);
        }

        let message = checkpoint_validator_signature_message_bytes(&signature.signing_payload())
            .map_err(|error| CheckpointValidatorSigningError::SigningMessage(error.to_string()))?;

        let raw_signature = decode_lower_hex_ed25519_signature(&signature.signature_wire)?;

        verifier
            .verify(&self.concrete_key, &message, &raw_signature)
            .map_err(|error| CheckpointValidatorSigningError::Verification(error.to_string()))
    }
}

/// Process-local runtime wrapper for exactly one checkpoint validator signer.
///
/// The signer backend remains opaque and is never exposed by `Debug`. This
/// wrapper creates one validator signature at a time and has no committee,
/// quorum, or finality authority.
pub struct CheckpointValidatorSigningRuntime {
    participant: CheckpointValidatorParticipant,
    signer: Arc<dyn Signer>,
}

impl CheckpointValidatorSigningRuntime {
    /// Construct one process-local checkpoint validator signing runtime.
    #[must_use]
    pub fn new(participant: CheckpointValidatorParticipant, signer: Arc<dyn Signer>) -> Self {
        Self {
            participant,
            signer,
        }
    }

    /// Reviewed chain identity of this checkpoint validator.
    #[must_use]
    pub fn chain_id(&self) -> &str {
        self.participant.chain_id()
    }

    /// Reviewed epoch identity of this checkpoint validator.
    #[must_use]
    pub fn epoch_id(&self) -> &str {
        self.participant.epoch_id()
    }

    /// Reviewed validator identifier.
    #[must_use]
    pub fn validator_id(&self) -> &str {
        self.participant.validator_id()
    }

    /// Reviewed logical signing-key identifier.
    #[must_use]
    pub fn logical_key_id(&self) -> &str {
        self.participant.logical_key_id()
    }

    /// Produce exactly one checkpoint-validator signature.
    ///
    /// This delegates to the already-reviewed participant and injected KMS
    /// signer. It does not aggregate signatures or create finality.
    ///
    /// # Errors
    ///
    /// Returns the participant/KMS signing failure without manufacturing a
    /// signature.
    pub fn sign_checkpoint(
        &self,
        height: u64,
        checkpoint_hash: ContentId,
    ) -> Result<QuickChainCheckpointValidatorSignatureV1, CheckpointValidatorSigningRuntimeError>
    {
        self.participant
            .sign_checkpoint(self.signer.as_ref(), height, checkpoint_hash)
            .map_err(CheckpointValidatorSigningRuntimeError::Signing)
    }
}

impl fmt::Debug for CheckpointValidatorSigningRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CheckpointValidatorSigningRuntime")
            .field("chain_id", &self.participant.chain_id())
            .field("epoch_id", &self.participant.epoch_id())
            .field("validator_id", &self.participant.validator_id())
            .field("logical_key_id", &self.participant.logical_key_id())
            .field("signer", &"<redacted>")
            .finish()
    }
}

/// Process-local checkpoint validator runtime failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointValidatorSigningRuntimeError {
    /// A signer is already registered and cannot be silently replaced.
    AlreadyConfigured,

    /// No checkpoint validator signer has been explicitly registered.
    NotConfigured,

    /// The registered participant or KMS signer rejected the operation.
    Signing(CheckpointValidatorSigningError),
}

impl fmt::Display for CheckpointValidatorSigningRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConfigured => {
                formatter.write_str("checkpoint validator signer is already configured")
            }
            Self::NotConfigured => {
                formatter.write_str("checkpoint validator signer is not configured")
            }
            Self::Signing(error) => {
                write!(formatter, "checkpoint validator signing failed: {error}")
            }
        }
    }
}

impl StdError for CheckpointValidatorSigningRuntimeError {}

/// Local checkpoint-validator signing or verification failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointValidatorSigningError {
    /// Reviewed validator identity failed DTO validation.
    InvalidIdentity(String),

    /// Validator is not in the active lifecycle state.
    ValidatorNotActive {
        /// Rejected validator identifier.
        validator_id: String,
    },

    /// Validator requests a signature algorithm not supported by this runtime.
    UnsupportedAlgorithm {
        /// Unsupported protocol algorithm.
        algorithm: SignatureAlg,
    },

    /// Injected concrete KMS key is not an Ed25519 key.
    ConcreteKeyAlgorithmMismatch,

    /// Canonical signing-message construction failed.
    SigningMessage(String),

    /// KMS signing failed.
    Signing(String),

    /// KMS returned a signature with an unexpected byte length.
    UnexpectedSignatureLength {
        /// Actual signature byte count.
        actual: usize,
    },

    /// Resulting protocol signature artifact failed validation.
    InvalidSignatureArtifact(String),

    /// Signature targets a different chain.
    ChainMismatch,

    /// Signature targets a different epoch.
    EpochMismatch,

    /// Signature claims another validator identity.
    ValidatorMismatch,

    /// Signature claims another reviewed key identity.
    KeyIdentityMismatch,

    /// Signature claims another signature algorithm.
    SignatureAlgorithmMismatch,

    /// Signature wire data is not exactly one canonical lowercase Ed25519
    /// signature.
    InvalidSignatureWire,

    /// KMS verification operation failed.
    Verification(String),
}

impl fmt::Display for CheckpointValidatorSigningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity(reason) => {
                write!(formatter, "invalid checkpoint validator identity: {reason}")
            }
            Self::ValidatorNotActive { validator_id } => {
                write!(
                    formatter,
                    "checkpoint validator is not active: {validator_id}"
                )
            }
            Self::UnsupportedAlgorithm { algorithm } => {
                write!(
                    formatter,
                    "unsupported checkpoint validator signature algorithm: {algorithm:?}"
                )
            }
            Self::ConcreteKeyAlgorithmMismatch => {
                formatter.write_str("checkpoint validator concrete KMS key is not Ed25519")
            }
            Self::SigningMessage(reason) => {
                write!(
                    formatter,
                    "checkpoint validator signing message failed: {reason}"
                )
            }
            Self::Signing(reason) => {
                write!(formatter, "checkpoint validator signing failed: {reason}")
            }
            Self::UnexpectedSignatureLength { actual } => {
                write!(
                    formatter,
                    "checkpoint validator Ed25519 signature length was {actual} bytes"
                )
            }
            Self::InvalidSignatureArtifact(reason) => {
                write!(
                    formatter,
                    "invalid checkpoint validator signature artifact: {reason}"
                )
            }
            Self::ChainMismatch => {
                formatter.write_str("checkpoint validator signature chain mismatch")
            }
            Self::EpochMismatch => {
                formatter.write_str("checkpoint validator signature epoch mismatch")
            }
            Self::ValidatorMismatch => {
                formatter.write_str("checkpoint validator signature validator mismatch")
            }
            Self::KeyIdentityMismatch => {
                formatter.write_str("checkpoint validator signature key identity mismatch")
            }
            Self::SignatureAlgorithmMismatch => {
                formatter.write_str("checkpoint validator signature algorithm mismatch")
            }
            Self::InvalidSignatureWire => formatter.write_str(
                "checkpoint validator signature wire is not canonical lowercase Ed25519 hex",
            ),
            Self::Verification(reason) => {
                write!(
                    formatter,
                    "checkpoint validator signature verification failed: {reason}"
                )
            }
        }
    }
}

impl StdError for CheckpointValidatorSigningError {}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }

    encoded
}

fn decode_lower_hex_ed25519_signature(
    encoded: &str,
) -> Result<Vec<u8>, CheckpointValidatorSigningError> {
    if encoded.len() != ED25519_SIGNATURE_HEX_LEN {
        return Err(CheckpointValidatorSigningError::InvalidSignatureWire);
    }

    let bytes = encoded.as_bytes();

    let mut decoded = Vec::with_capacity(ED25519_SIGNATURE_BYTES);

    let mut index = 0usize;

    while index < bytes.len() {
        let high = decode_lower_hex_nibble(bytes[index])
            .ok_or(CheckpointValidatorSigningError::InvalidSignatureWire)?;

        let low = decode_lower_hex_nibble(bytes[index + 1])
            .ok_or(CheckpointValidatorSigningError::InvalidSignatureWire)?;

        decoded.push((high << 4) | low);

        index += 2;
    }

    Ok(decoded)
}

fn decode_lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_kms::{memory_keystore, Keystore};
    use ron_proto::quickchain::QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA;
    use serde_json::Value;

    const CHAIN_ID: &str = "ron-devnet";
    const EPOCH_ID: &str = "epoch_phase19_checkpoint_crypto";

    fn cid(label: &str) -> ContentId {
        let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

        format!("b3:{digest}")
            .parse()
            .expect("fixture content ID must parse")
    }

    fn identity(validator_id: &str, key_id: &str) -> QuickChainValidatorIdentityV1 {
        QuickChainValidatorIdentityV1 {
            schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_owned(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: CHAIN_ID.to_owned(),
            epoch_id: EPOCH_ID.to_owned(),
            validator_id: validator_id.to_owned(),
            passport_subject: format!("@{validator_id}"),
            registry_entry_id: format!("registry:{validator_id}"),
            key_id: key_id.to_owned(),
            capability_id: format!("cap:{validator_id}:verify:001"),
            signature_algorithm: SignatureAlg::Ed25519,
            lifecycle_status: QuickChainValidatorLifecycleStatusV1::Active,
            not_before_ms: 1_800_000_000_000,
            expires_at_ms: 1_800_086_400_000,
        }
    }

    fn assert_no_authority_fields(signature: &QuickChainCheckpointValidatorSignatureV1) {
        let value = serde_json::to_value(signature).expect("signature must serialize");

        let object = value
            .as_object()
            .expect("signature must serialize as object");

        for forbidden in [
            "quorum",
            "quorum_reached",
            "quorum_finalized",
            "threshold",
            "finality",
            "finalized",
            "checkpoint_finalized",
            "wallet_mutation",
            "ledger_mutation",
            "payout_executed",
            "receipt_created",
            "paid_unlock",
        ] {
            assert!(
                !object.contains_key(forbidden),
                "checkpoint validator signature must not contain authority field {forbidden}",
            );
        }
    }

    #[test]
    fn two_independent_validators_sign_same_checkpoint_and_verify_independently() {
        let alpha_kms = memory_keystore();
        let beta_kms = memory_keystore();

        let alpha_key = alpha_kms
            .create_ed25519("checkpoint-validator", "phase19-alpha")
            .expect("alpha key must be created");

        let beta_key = beta_kms
            .create_ed25519("checkpoint-validator", "phase19-beta")
            .expect("beta key must be created");

        let alpha = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-alpha", "key:validator-alpha:001"),
            alpha_key,
        )
        .expect("alpha participant must build");

        let beta = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-beta", "key:validator-beta:001"),
            beta_key,
        )
        .expect("beta participant must build");

        let checkpoint_hash = cid("phase19-checkpoint");

        let alpha_signature = alpha
            .sign_checkpoint(&alpha_kms, 19, checkpoint_hash.clone())
            .expect("alpha signature must be created");

        let beta_signature = beta
            .sign_checkpoint(&beta_kms, 19, checkpoint_hash)
            .expect("beta signature must be created");

        assert_ne!(alpha_signature.validator_id, beta_signature.validator_id,);

        assert_ne!(alpha_signature.key_id, beta_signature.key_id,);

        assert_ne!(
            alpha_signature.signature_wire,
            beta_signature.signature_wire,
        );

        assert!(alpha
            .verify_signature(&alpha_kms, &alpha_signature,)
            .expect("alpha verification must complete",));

        assert!(beta
            .verify_signature(&beta_kms, &beta_signature,)
            .expect("beta verification must complete",));
    }

    #[test]
    fn checkpoint_signature_wire_is_real_lowercase_ed25519_hex() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-wire")
            .expect("fixture key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-wire", "key:validator-wire:001"),
            key,
        )
        .expect("participant must build");

        let signature = participant
            .sign_checkpoint(&kms, 19, cid("wire-checkpoint"))
            .expect("signature must be created");

        assert_eq!(signature.signature_wire.len(), ED25519_SIGNATURE_HEX_LEN,);

        assert!(signature
            .signature_wire
            .bytes()
            .all(|byte| { byte.is_ascii_digit() || matches!(byte, b'a'..=b'f') }),);

        assert!(participant
            .verify_signature(&kms, &signature,)
            .expect("signature verification must complete",));
    }

    #[test]
    fn signature_cannot_be_reused_for_changed_checkpoint_hash() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-hash-reuse")
            .expect("fixture key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-hash", "key:validator-hash:001"),
            key,
        )
        .expect("participant must build");

        let signature = participant
            .sign_checkpoint(&kms, 19, cid("original-checkpoint"))
            .expect("signature must be created");

        let mut changed = signature.clone();

        changed.checkpoint_hash = cid("different-checkpoint");

        let verifies = participant
            .verify_signature(&kms, &changed)
            .expect("well-formed changed-hash verification must complete");

        assert!(
            !verifies,
            "original signature must not verify after checkpoint hash changes",
        );
    }

    #[test]
    fn signature_cannot_be_reused_for_changed_height() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-height-reuse")
            .expect("fixture key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-height", "key:validator-height:001"),
            key,
        )
        .expect("participant must build");

        let signature = participant
            .sign_checkpoint(&kms, 19, cid("height-checkpoint"))
            .expect("signature must be created");

        let mut changed = signature.clone();

        changed.height = 20;

        let verifies = participant
            .verify_signature(&kms, &changed)
            .expect("well-formed changed-height verification must complete");

        assert!(
            !verifies,
            "original signature must not verify after checkpoint height changes",
        );
    }

    #[test]
    fn signature_does_not_verify_under_another_validator_key() {
        let alpha_kms = memory_keystore();
        let beta_kms = memory_keystore();

        let alpha_key = alpha_kms
            .create_ed25519("checkpoint-validator", "phase19-cross-key-alpha")
            .expect("alpha key must be created");

        let beta_key = beta_kms
            .create_ed25519("checkpoint-validator", "phase19-cross-key-beta")
            .expect("beta key must be created");

        let alpha = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-alpha", "key:validator-alpha:001"),
            alpha_key,
        )
        .expect("alpha participant must build");

        let beta = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-beta", "key:validator-beta:001"),
            beta_key,
        )
        .expect("beta participant must build");

        let alpha_signature = alpha
            .sign_checkpoint(&alpha_kms, 19, cid("cross-key-checkpoint"))
            .expect("alpha signature must be created");

        let mut beta_bound = alpha_signature.clone();

        beta_bound.validator_id = beta.validator_id().to_owned();

        beta_bound.key_id = beta.logical_key_id().to_owned();

        let verifies = beta
            .verify_signature(&beta_kms, &beta_bound)
            .expect("cross-key verification must complete");

        assert!(
            !verifies,
            "alpha signature must not verify after rebinding to beta identity/key",
        );
    }

    #[test]
    fn revoked_validator_identity_cannot_become_checkpoint_signer() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-revoked")
            .expect("fixture key must be created");

        let mut revoked = identity("validator-revoked", "key:validator-revoked:001");

        revoked.lifecycle_status = QuickChainValidatorLifecycleStatusV1::Revoked;

        let error = CheckpointValidatorParticipant::from_reviewed_identity(&revoked, key)
            .expect_err("revoked validator must not become local checkpoint signer");

        assert_eq!(
            error,
            CheckpointValidatorSigningError::ValidatorNotActive {
                validator_id: "validator-revoked".to_owned(),
            },
        );
    }

    #[test]
    fn unsupported_validator_algorithm_rejects_before_signing() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-algorithm")
            .expect("fixture key must be created");

        let mut unsupported = identity("validator-algorithm", "key:validator-algorithm:001");

        unsupported.signature_algorithm = SignatureAlg::Dilithium3;

        let error = CheckpointValidatorParticipant::from_reviewed_identity(&unsupported, key)
            .expect_err("unsupported algorithm must reject before signing");

        assert_eq!(
            error,
            CheckpointValidatorSigningError::UnsupportedAlgorithm {
                algorithm: SignatureAlg::Dilithium3,
            },
        );
    }

    #[test]
    fn zero_height_rejects_before_kms_signature_is_created() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-zero-height")
            .expect("fixture key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-zero", "key:validator-zero:001"),
            key,
        )
        .expect("participant must build");

        let error = participant
            .sign_checkpoint(&kms, 0, cid("zero-height-checkpoint"))
            .expect_err("zero-height checkpoint must reject before signing");

        assert!(matches!(
            error,
            CheckpointValidatorSigningError::SigningMessage(_)
        ),);
    }

    #[test]
    fn signature_artifact_contains_no_quorum_finality_or_economic_authority() {
        let kms = memory_keystore();

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-authority")
            .expect("fixture key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-authority", "key:validator-authority:001"),
            key,
        )
        .expect("participant must build");

        let signature = participant
            .sign_checkpoint(&kms, 19, cid("authority-checkpoint"))
            .expect("signature must be created");

        assert_no_authority_fields(&signature);

        let value = serde_json::to_value(&signature).expect("signature must serialize");

        assert_eq!(value["algorithm"], Value::String("ed25519".to_owned(),),);
    }

    #[test]
    fn runtime_status_starts_without_checkpoint_validator_signer_and_fails_closed() {
        let runtime = crate::types::RuntimeStatus::new();

        assert!(!runtime.checkpoint_validator_signing_active(),);

        let error = runtime
            .sign_checkpoint_validator(19, &cid("runtime-unconfigured-checkpoint"))
            .expect_err("unconfigured runtime must not manufacture checkpoint signature");

        assert_eq!(error, CheckpointValidatorSigningRuntimeError::NotConfigured,);
    }

    #[test]
    fn runtime_status_registers_one_checkpoint_validator_and_signs() {
        let runtime = crate::types::RuntimeStatus::new();

        let kms = std::sync::Arc::new(memory_keystore());

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-runtime-checkpoint-alpha")
            .expect("fixture checkpoint key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-runtime-alpha", "key:validator-runtime-alpha:001"),
            key.clone(),
        )
        .expect("runtime checkpoint participant must build");

        let signer: std::sync::Arc<dyn ron_kms::Signer> = kms.clone();

        runtime
            .register_checkpoint_validator_signer(std::sync::Arc::new(
                CheckpointValidatorSigningRuntime::new(participant, signer),
            ))
            .expect("first checkpoint validator signer must register");

        assert!(runtime.checkpoint_validator_signing_active(),);

        let checkpoint_hash = cid("runtime-checkpoint-alpha");

        let signature = runtime
            .sign_checkpoint_validator(19, &checkpoint_hash)
            .expect("registered checkpoint validator must sign");

        assert_eq!(signature.validator_id, "validator-runtime-alpha",);

        assert_eq!(signature.key_id, "key:validator-runtime-alpha:001",);

        assert_eq!(signature.checkpoint_hash, checkpoint_hash,);

        let message = checkpoint_validator_signature_message_bytes(&signature.signing_payload())
            .expect("runtime signature message must encode");

        let raw_signature = decode_lower_hex_ed25519_signature(&signature.signature_wire)
            .expect("runtime signature wire must decode");

        let verifies = ron_kms::Verifier::verify(kms.as_ref(), &key, &message, &raw_signature)
            .expect("runtime signature verification must complete");

        assert!(
            verifies,
            "runtime-produced checkpoint signature must be real Ed25519 evidence",
        );
    }

    #[test]
    fn runtime_status_rejects_second_checkpoint_validator_registration() {
        let runtime = crate::types::RuntimeStatus::new();

        let first_kms = std::sync::Arc::new(memory_keystore());

        let first_key = first_kms
            .create_ed25519("checkpoint-validator", "phase19-runtime-checkpoint-first")
            .expect("first runtime key must be created");

        let first = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-runtime-first", "key:validator-runtime-first:001"),
            first_key,
        )
        .expect("first runtime participant must build");

        let first_signer: std::sync::Arc<dyn ron_kms::Signer> = first_kms;

        runtime
            .register_checkpoint_validator_signer(std::sync::Arc::new(
                CheckpointValidatorSigningRuntime::new(first, first_signer),
            ))
            .expect("first runtime checkpoint signer must register");

        let second_kms = std::sync::Arc::new(memory_keystore());

        let second_key = second_kms
            .create_ed25519("checkpoint-validator", "phase19-runtime-checkpoint-second")
            .expect("second runtime key must be created");

        let second = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity(
                "validator-runtime-second",
                "key:validator-runtime-second:001",
            ),
            second_key,
        )
        .expect("second runtime participant must build");

        let second_signer: std::sync::Arc<dyn ron_kms::Signer> = second_kms;

        let error = runtime
            .register_checkpoint_validator_signer(std::sync::Arc::new(
                CheckpointValidatorSigningRuntime::new(second, second_signer),
            ))
            .expect_err("second runtime checkpoint signer must reject");

        assert_eq!(
            error,
            CheckpointValidatorSigningRuntimeError::AlreadyConfigured,
        );
    }

    #[test]
    fn checkpoint_validator_runtime_debug_redacts_signing_backend() {
        let kms = std::sync::Arc::new(memory_keystore());

        let key = kms
            .create_ed25519("checkpoint-validator", "phase19-runtime-debug")
            .expect("runtime debug key must be created");

        let participant = CheckpointValidatorParticipant::from_reviewed_identity(
            &identity("validator-runtime-debug", "key:validator-runtime-debug:001"),
            key,
        )
        .expect("runtime debug participant must build");

        let signer: std::sync::Arc<dyn ron_kms::Signer> = kms;

        let runtime = CheckpointValidatorSigningRuntime::new(participant, signer);

        let debug = format!("{runtime:?}");

        assert!(debug.contains("validator-runtime-debug",),);

        assert!(debug.contains("<redacted>",),);

        assert!(!debug.contains("MemoryKeystore",),);
    }
}
