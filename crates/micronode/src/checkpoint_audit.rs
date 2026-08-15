//! RO:WHAT — Independent User Node review of finalized QuickChain checkpoint evidence.
//! RO:WHY — FINAL_BETA Phase 19 requires ordinary User Nodes to detect and
//! challenge invalid checkpoint roots, validator sets, signatures, and evidence.
//! RO:INTERACTS — ron-proto finalized checkpoint/challenge DTOs and ron-kms
//! Ed25519 verification.
//! RO:INVARIANTS — independently reproduce candidate hash; compare locally
//! observed roots/set commitment; verify every counted signature against an
//! explicitly reviewed public key; challenge only observed protocol faults.
//! RO:SECURITY — read-only verification/evidence construction only. No wallet,
//! ledger, quorum-signing, checkpoint production, slashing, rollback, or
//! finality authority.
//! RO:TEST — focused unit tests in this module.

#![forbid(unsafe_code)]

use std::{collections::HashSet, error::Error as StdError, fmt};

use ron_kms::backends::ed25519;
use ron_proto::{
    checkpoint_validator_signature_message_bytes, to_canonical_json_vec, ContentId,
    QuickChainChallengeTypeV1, QuickChainChallengeV1, QuickChainCheckpointValidatorSignatureV1,
    QuickChainFinalizedCheckpointV1, QuickChainValidatorLifecycleStatusV1, SignatureAlg,
    QUICKCHAIN_CHALLENGE_SCHEMA, QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1, QUICKCHAIN_DTO_VERSION,
};
use serde::Serialize;

const CHECKPOINT_AUDIT_EVIDENCE_DOMAIN: &str = "micronode.user-node-checkpoint-audit-evidence.v1";

const ED25519_PUBLIC_KEY_BYTES: usize = 32;
const ED25519_SIGNATURE_BYTES: usize = 64;

/// Explicit reviewed public key for one checkpoint validator.
///
/// The User Node receives this material from its independently reviewed
/// validator-set/key view. The finalized checkpoint does not get to choose a
/// replacement public key for itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointValidatorPublicKey {
    /// Reviewed validator identity.
    pub validator_id: String,

    /// Reviewed logical key identity.
    pub key_id: String,

    /// Exact Ed25519 public key bytes.
    pub public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

/// User Node's independently observed checkpoint expectations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserNodeCheckpointObservation {
    /// Local challenger identity used only for challenge evidence.
    pub challenger_id: String,

    /// Non-zero local observation/challenge time.
    pub observed_at_ms: u64,

    /// State root independently obtained/replayed by the User Node.
    pub expected_new_state_root: ContentId,

    /// Receipt root independently obtained/replayed by the User Node.
    pub expected_receipt_root: ContentId,

    /// Validator-set commitment independently reviewed by the User Node.
    pub expected_validator_set_hash: ContentId,

    /// Reviewed validator public keys.
    pub validator_public_keys: Vec<CheckpointValidatorPublicKey>,
}

/// One deterministic local checkpoint audit result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserNodeCheckpointAudit {
    /// Checkpoint evidence passed every local review performed here.
    Accepted,

    /// Checkpoint evidence failed and produced a canonical descriptive
    /// QuickChain challenge.
    Challenged {
        /// Specific local finding.
        finding: CheckpointAuditFinding,

        /// Canonical protocol challenge evidence.
        challenge: QuickChainChallengeV1,
    },
}

/// Exact checkpoint fault identified by the User Node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckpointAuditFinding {
    /// Artifact checkpoint hash differs from the independently reproduced
    /// candidate hash.
    InvalidCheckpointHash { claimed: ContentId, reproduced: ContentId },

    /// Candidate state root differs from the User Node replay.
    InvalidStateRoot { claimed: ContentId, expected: ContentId },

    /// Candidate receipt root differs from the User Node replay.
    InvalidReceiptRoot { claimed: ContentId, expected: ContentId },

    /// Candidate/finalized artifact carries the wrong reviewed validator-set
    /// commitment.
    InvalidValidatorSet { claimed: ContentId, expected: ContentId },

    /// A counted validator signature is malformed, misbound, or fails real
    /// Ed25519 verification.
    InvalidValidatorSignature { validator_id: String },

    /// The artifact is otherwise inconsistent with the finalized-checkpoint
    /// protocol contract.
    InvalidCheckpointEvidence,
}

impl CheckpointAuditFinding {
    fn challenge_type(&self) -> QuickChainChallengeTypeV1 {
        match self {
            Self::InvalidCheckpointHash { .. } => QuickChainChallengeTypeV1::InvalidCheckpointHash,

            Self::InvalidStateRoot { .. } => QuickChainChallengeTypeV1::InvalidStateRoot,

            Self::InvalidReceiptRoot { .. } => QuickChainChallengeTypeV1::InvalidReceiptRoot,

            Self::InvalidValidatorSet { .. } => QuickChainChallengeTypeV1::InvalidValidatorSet,

            Self::InvalidValidatorSignature { .. } => {
                QuickChainChallengeTypeV1::InvalidValidatorSignature
            }

            Self::InvalidCheckpointEvidence => QuickChainChallengeTypeV1::InvalidCheckpointEvidence,
        }
    }
}

/// Local User Node checkpoint-audit setup failure.
///
/// These errors mean the User Node does not yet have enough valid local
/// material to accuse the remote checkpoint. They are deliberately distinct
/// from challenge findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointAuditError {
    /// Local observation is malformed.
    InvalidObservation(&'static str),

    /// User Node lacks a reviewed public key for a counted validator.
    MissingVerificationKey { validator_id: String, key_id: String },

    /// Local canonicalization failed.
    Canonicalization(String),

    /// Locally computed BLAKE3 ID could not be represented as ContentId.
    InvalidComputedContentId,
}

impl fmt::Display for CheckpointAuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidObservation(reason) => {
                write!(formatter, "invalid User Node checkpoint observation: {reason}")
            }

            Self::MissingVerificationKey { validator_id, key_id } => {
                write!(
                    formatter,
                    "missing reviewed checkpoint verification key for validator {validator_id} key {key_id}"
                )
            }

            Self::Canonicalization(reason) => {
                write!(formatter, "checkpoint audit canonicalization failed: {reason}")
            }

            Self::InvalidComputedContentId => {
                formatter.write_str("checkpoint audit computed an invalid content id")
            }
        }
    }
}

impl StdError for CheckpointAuditError {}

/// Independently review one finalized checkpoint.
///
/// A malformed **local observation** returns `Err` and does not fabricate a
/// challenge. A fault in the supplied checkpoint evidence returns
/// `Ok(UserNodeCheckpointAudit::Challenged { .. })`.
///
/// # Errors
///
/// Returns an error only when local User Node observation/key material is
/// insufficient or local canonicalization cannot complete.
pub fn audit_finalized_checkpoint(
    artifact: &QuickChainFinalizedCheckpointV1,
    observation: &UserNodeCheckpointObservation,
) -> Result<UserNodeCheckpointAudit, CheckpointAuditError> {
    validate_observation(observation)?;

    if artifact.candidate.validate().is_err() {
        return challenge(artifact, observation, CheckpointAuditFinding::InvalidCheckpointEvidence);
    }

    let reproduced_hash = recompute_checkpoint_hash(&artifact.candidate)?;

    if reproduced_hash != artifact.checkpoint_hash {
        return challenge(
            artifact,
            observation,
            CheckpointAuditFinding::InvalidCheckpointHash {
                claimed: artifact.checkpoint_hash.clone(),

                reproduced: reproduced_hash,
            },
        );
    }

    if artifact.candidate.new_state_root != observation.expected_new_state_root {
        return challenge(
            artifact,
            observation,
            CheckpointAuditFinding::InvalidStateRoot {
                claimed: artifact.candidate.new_state_root.clone(),

                expected: observation.expected_new_state_root.clone(),
            },
        );
    }

    if artifact.candidate.receipt_root != observation.expected_receipt_root {
        return challenge(
            artifact,
            observation,
            CheckpointAuditFinding::InvalidReceiptRoot {
                claimed: artifact.candidate.receipt_root.clone(),

                expected: observation.expected_receipt_root.clone(),
            },
        );
    }

    if artifact.candidate.validator_set_hash != observation.expected_validator_set_hash
        || artifact.validator_set.validator_set_hash != observation.expected_validator_set_hash
    {
        return challenge(
            artifact,
            observation,
            CheckpointAuditFinding::InvalidValidatorSet {
                claimed: artifact.candidate.validator_set_hash.clone(),

                expected: observation.expected_validator_set_hash.clone(),
            },
        );
    }

    if artifact.validator_set.validate().is_err()
        || artifact.validator_set.chain_id != artifact.candidate.chain_id
        || artifact.validator_set.epoch_id != artifact.candidate.epoch_id
    {
        return challenge(
            artifact,
            observation,
            CheckpointAuditFinding::InvalidValidatorSet {
                claimed: artifact.validator_set.validator_set_hash.clone(),

                expected: observation.expected_validator_set_hash.clone(),
            },
        );
    }

    for signature in &artifact.signatures {
        match verify_checkpoint_signature(artifact, observation, signature)? {
            true => {}

            false => {
                return challenge(
                    artifact,
                    observation,
                    CheckpointAuditFinding::InvalidValidatorSignature {
                        validator_id: signature.validator_id.clone(),
                    },
                );
            }
        }
    }

    if artifact.validate().is_err() {
        return challenge(artifact, observation, CheckpointAuditFinding::InvalidCheckpointEvidence);
    }

    Ok(UserNodeCheckpointAudit::Accepted)
}

fn validate_observation(
    observation: &UserNodeCheckpointObservation,
) -> Result<(), CheckpointAuditError> {
    if observation.challenger_id.trim().is_empty() {
        return Err(CheckpointAuditError::InvalidObservation("challenger_id must not be empty"));
    }

    if observation.observed_at_ms == 0 {
        return Err(CheckpointAuditError::InvalidObservation(
            "observed_at_ms must be greater than zero",
        ));
    }

    let mut validators = HashSet::new();

    let mut keys = HashSet::new();

    for key in &observation.validator_public_keys {
        if key.validator_id.trim().is_empty() || key.key_id.trim().is_empty() {
            return Err(CheckpointAuditError::InvalidObservation(
                "validator verification identities must not be empty",
            ));
        }

        if !validators.insert(key.validator_id.clone()) {
            return Err(CheckpointAuditError::InvalidObservation(
                "duplicate validator verification identity",
            ));
        }

        if !keys.insert(key.key_id.clone()) {
            return Err(CheckpointAuditError::InvalidObservation(
                "duplicate validator verification key identity",
            ));
        }
    }

    Ok(())
}

fn verify_checkpoint_signature(
    artifact: &QuickChainFinalizedCheckpointV1,
    observation: &UserNodeCheckpointObservation,
    signature: &QuickChainCheckpointValidatorSignatureV1,
) -> Result<bool, CheckpointAuditError> {
    if signature.validate().is_err()
        || signature.chain_id != artifact.candidate.chain_id
        || signature.epoch_id != artifact.candidate.epoch_id
        || signature.height != artifact.candidate.height
        || signature.checkpoint_hash != artifact.checkpoint_hash
        || signature.algorithm != SignatureAlg::Ed25519
    {
        return Ok(false);
    }

    let Some(member) = artifact
        .validator_set
        .members
        .iter()
        .find(|member| member.validator_id == signature.validator_id)
    else {
        return Ok(false);
    };

    if member.lifecycle_status != QuickChainValidatorLifecycleStatusV1::Active
        || member.key_id != signature.key_id
        || member.signature_algorithm != signature.algorithm
    {
        return Ok(false);
    }

    let reviewed_key = observation
        .validator_public_keys
        .iter()
        .find(|key| key.validator_id == signature.validator_id && key.key_id == signature.key_id)
        .ok_or_else(|| CheckpointAuditError::MissingVerificationKey {
            validator_id: signature.validator_id.clone(),

            key_id: signature.key_id.clone(),
        })?;

    let message = checkpoint_validator_signature_message_bytes(&signature.signing_payload())
        .map_err(|error| CheckpointAuditError::Canonicalization(error.to_string()))?;

    let raw_signature = decode_signature_wire(&signature.signature_wire);

    let Some(raw_signature) = raw_signature else {
        return Ok(false);
    };

    Ok(ed25519::verify(&reviewed_key.public_key, &message, &raw_signature))
}

fn recompute_checkpoint_hash(
    candidate: &ron_proto::QuickChainCommitteeCheckpointPayloadV1,
) -> Result<ContentId, CheckpointAuditError> {
    let canonical = to_canonical_json_vec(candidate)
        .map_err(|error| CheckpointAuditError::Canonicalization(error.to_string()))?;

    let mut hasher = blake3::Hasher::new();

    hasher.update(QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());

    hasher.update(&[0]);

    hasher.update(&canonical);

    format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .map_err(|_| CheckpointAuditError::InvalidComputedContentId)
}

fn challenge(
    artifact: &QuickChainFinalizedCheckpointV1,
    observation: &UserNodeCheckpointObservation,
    finding: CheckpointAuditFinding,
) -> Result<UserNodeCheckpointAudit, CheckpointAuditError> {
    #[derive(Serialize)]
    struct EvidencePreimage<'a> {
        schema: &'static str,

        chain_id: &'a str,

        checkpoint_hash: &'a ContentId,

        challenger_id: &'a str,

        finding: &'a CheckpointAuditFinding,

        observed_at_ms: u64,
    }

    let preimage = EvidencePreimage {
        schema: CHECKPOINT_AUDIT_EVIDENCE_DOMAIN,

        chain_id: &artifact.candidate.chain_id,

        checkpoint_hash: &artifact.checkpoint_hash,

        challenger_id: &observation.challenger_id,

        finding: &finding,

        observed_at_ms: observation.observed_at_ms,
    };

    let canonical = to_canonical_json_vec(&preimage)
        .map_err(|error| CheckpointAuditError::Canonicalization(error.to_string()))?;

    let mut hasher = blake3::Hasher::new();

    hasher.update(CHECKPOINT_AUDIT_EVIDENCE_DOMAIN.as_bytes());

    hasher.update(&[0]);

    hasher.update(&canonical);

    let evidence_cid: ContentId = format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .map_err(|_| CheckpointAuditError::InvalidComputedContentId)?;

    let challenge = QuickChainChallengeV1 {
        schema: QUICKCHAIN_CHALLENGE_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: artifact.candidate.chain_id.clone(),

        checkpoint_hash: artifact.checkpoint_hash.clone(),

        challenger_id: observation.challenger_id.clone(),

        challenge_type: finding.challenge_type(),

        evidence_cid,

        submitted_at_ms: observation.observed_at_ms,
    };

    challenge.validate().map_err(|_| {
        CheckpointAuditError::InvalidObservation("constructed challenge failed protocol validation")
    })?;

    Ok(UserNodeCheckpointAudit::Challenged { finding, challenge })
}

fn decode_signature_wire(wire: &str) -> Option<[u8; ED25519_SIGNATURE_BYTES]> {
    if wire.len() != ED25519_SIGNATURE_BYTES * 2
        || !wire.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }

    let decoded = hex::decode(wire).ok()?;

    let signature: [u8; ED25519_SIGNATURE_BYTES] = decoded.try_into().ok()?;

    Some(signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_proto::{
        QuickChainCanonicalEncodingV1, QuickChainCheckpointFinalityStateV1,
        QuickChainCheckpointValidatorSignatureV1, QuickChainCommitteeCheckpointPayloadV1,
        QuickChainConservationV1, QuickChainFinalizedCheckpointV1, QuickChainReceiptRootSchemeV1,
        QuickChainSettlementModeV1, QuickChainStateRootSchemeV1, QuickChainSupplyDeltaV1,
        QuickChainValidatorIdentityV1, QuickChainValidatorSetV1,
        QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA,
        QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA, QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA,
        QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1, QUICKCHAIN_VALIDATOR_SET_SCHEMA,
    };

    const CHAIN_ID: &str = "ron-devnet";

    const EPOCH_ID: &str = "epoch_phase19_user_node_checkpoint";

    fn cid(label: &str) -> ContentId {
        format!("b3:{}", blake3::hash(label.as_bytes(),).to_hex(),)
            .parse()
            .expect("fixture content id must parse")
    }

    fn member(suffix: &str) -> QuickChainValidatorIdentityV1 {
        QuickChainValidatorIdentityV1 {
            schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            epoch_id: EPOCH_ID.to_owned(),

            validator_id: format!("validator-{suffix}"),

            passport_subject: format!("@validator-{suffix}"),

            registry_entry_id: format!("registry:validator-{suffix}"),

            key_id: format!("key:validator-{suffix}:001"),

            capability_id: format!("cap:validator-{suffix}:verify:001"),

            signature_algorithm: SignatureAlg::Ed25519,

            lifecycle_status: QuickChainValidatorLifecycleStatusV1::Active,

            not_before_ms: 1_800_000_000_000,

            expires_at_ms: 1_800_086_400_000,
        }
    }

    fn validator_set() -> QuickChainValidatorSetV1 {
        QuickChainValidatorSetV1 {
            schema: QUICKCHAIN_VALIDATOR_SET_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            epoch_id: EPOCH_ID.to_owned(),

            validator_set_hash: cid("validator-set"),

            policy_hash: cid("validator-policy"),

            registry_snapshot_hash: cid("validator-registry"),

            passport_required: true,

            bond_required: false,

            validator_set_algorithm: QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1
                .to_owned(),

            members: vec![member("alpha"), member("beta"), member("gamma")],
        }
    }

    fn candidate(
        set: &QuickChainValidatorSetV1,
        new_state_root: ContentId,
        receipt_root: ContentId,
    ) -> QuickChainCommitteeCheckpointPayloadV1 {
        QuickChainCommitteeCheckpointPayloadV1 {
            schema: QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            height: 19,

            epoch_id: EPOCH_ID.to_owned(),

            execution_spec_version: "quickchain-execution-v1".to_owned(),

            previous_checkpoint_hash: cid("previous-checkpoint"),

            previous_state_root: cid("previous-state"),

            new_state_root,

            receipt_root,

            accounting_snapshot_root: cid("accounting-root"),

            reward_manifest_root: cid("reward-root"),

            data_availability_root: cid("da-root"),

            policy_hash: set.policy_hash.clone(),

            validator_set_hash: set.validator_set_hash.clone(),

            chain_params_hash: cid("chain-params"),

            canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,

            state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,

            receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,

            supply_delta: QuickChainSupplyDeltaV1 {
                issued_minor: "0".to_owned(),

                burned_minor: "0".to_owned(),

                net_minor: "0".to_owned(),
            },

            conservation: QuickChainConservationV1 {
                debits_minor: "100".to_owned(),

                credits_minor: "100".to_owned(),

                issue_exceptions_minor: "0".to_owned(),

                burn_exceptions_minor: "0".to_owned(),

                valid: true,
            },

            settlement_mode: QuickChainSettlementModeV1::LocalRoot,

            started_at_ms: 1_800_000_000_000,

            ended_at_ms: 1_800_000_060_000,

            produced_at_ms: 1_800_000_061_000,
        }
    }

    fn sign(
        candidate_hash: &ContentId,
        member: &QuickChainValidatorIdentityV1,
        seed: &[u8; 32],
    ) -> QuickChainCheckpointValidatorSignatureV1 {
        let mut signature = QuickChainCheckpointValidatorSignatureV1 {
            schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            height: 19,

            epoch_id: EPOCH_ID.to_owned(),

            checkpoint_hash: candidate_hash.clone(),

            validator_id: member.validator_id.clone(),

            key_id: member.key_id.clone(),

            algorithm: SignatureAlg::Ed25519,

            signature_wire: String::new(),
        };

        let message = checkpoint_validator_signature_message_bytes(&signature.signing_payload())
            .expect("fixture checkpoint signature message");

        signature.signature_wire = hex::encode(ed25519::sign(seed, &message));

        signature
    }

    fn finalized_for_candidate(
        set: &QuickChainValidatorSetV1,
        candidate: QuickChainCommitteeCheckpointPayloadV1,
        alpha_seed: &[u8; 32],
        beta_seed: &[u8; 32],
    ) -> QuickChainFinalizedCheckpointV1 {
        let checkpoint_hash =
            recompute_checkpoint_hash(&candidate).expect("fixture checkpoint hash");

        let mut signatures = vec![
            sign(&checkpoint_hash, &set.members[0], alpha_seed),
            sign(&checkpoint_hash, &set.members[1], beta_seed),
        ];

        signatures.sort_by(|left, right| left.validator_id.cmp(&right.validator_id));

        QuickChainFinalizedCheckpointV1 {
            schema: QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            candidate,

            checkpoint_hash,

            validator_set: set.clone(),

            required_signatures: 2,

            verified_unique_signatures: 2,

            signatures,

            finalization_state: QuickChainCheckpointFinalityStateV1::Finalized,
        }
    }

    fn observation(
        set: &QuickChainValidatorSetV1,
        expected_state: ContentId,
        expected_receipt: ContentId,
        alpha_seed: &[u8; 32],
        beta_seed: &[u8; 32],
    ) -> UserNodeCheckpointObservation {
        UserNodeCheckpointObservation {
            challenger_id: "user_node:phase19-auditor".to_owned(),

            observed_at_ms: 1_800_000_100_000,

            expected_new_state_root: expected_state,

            expected_receipt_root: expected_receipt,

            expected_validator_set_hash: set.validator_set_hash.clone(),

            validator_public_keys: vec![
                CheckpointValidatorPublicKey {
                    validator_id: set.members[0].validator_id.clone(),

                    key_id: set.members[0].key_id.clone(),

                    public_key: ed25519::public_key(alpha_seed),
                },
                CheckpointValidatorPublicKey {
                    validator_id: set.members[1].validator_id.clone(),

                    key_id: set.members[1].key_id.clone(),

                    public_key: ed25519::public_key(beta_seed),
                },
            ],
        }
    }

    fn challenge_type(review: UserNodeCheckpointAudit) -> QuickChainChallengeTypeV1 {
        match review {
            UserNodeCheckpointAudit::Accepted => {
                panic!("fixture expected a challenge")
            }

            UserNodeCheckpointAudit::Challenged { challenge, .. } => challenge.challenge_type,
        }
    }

    #[test]
    fn valid_checkpoint_is_accepted_after_independent_hash_root_set_and_signature_review() {
        let set = validator_set();

        let expected_state = cid("state-good");

        let expected_receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let artifact = finalized_for_candidate(
            &set,
            candidate(&set, expected_state.clone(), expected_receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        artifact.validate().expect("fixture finalized checkpoint must validate structurally");

        let review = audit_finalized_checkpoint(
            &artifact,
            &observation(&set, expected_state, expected_receipt, &alpha_seed, &beta_seed),
        )
        .expect("valid local review must complete");

        assert_eq!(review, UserNodeCheckpointAudit::Accepted,);
    }

    #[test]
    fn independently_observed_wrong_state_root_builds_invalid_state_root_challenge() {
        let set = validator_set();

        let actual_state = cid("state-wrong");

        let expected_state = cid("state-expected");

        let receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let artifact = finalized_for_candidate(
            &set,
            candidate(&set, actual_state, receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        let review = audit_finalized_checkpoint(
            &artifact,
            &observation(&set, expected_state, receipt, &alpha_seed, &beta_seed),
        )
        .expect("wrong-root review must complete");

        assert_eq!(challenge_type(review,), QuickChainChallengeTypeV1::InvalidStateRoot,);
    }

    #[test]
    fn wrong_validator_set_commitment_builds_invalid_validator_set_challenge() {
        let set = validator_set();

        let state = cid("state-good");

        let receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let artifact = finalized_for_candidate(
            &set,
            candidate(&set, state.clone(), receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        let mut observation = observation(&set, state, receipt, &alpha_seed, &beta_seed);

        observation.expected_validator_set_hash = cid("independently-reviewed-other-set");

        let review = audit_finalized_checkpoint(&artifact, &observation)
            .expect("wrong-set review must complete");

        assert_eq!(challenge_type(review,), QuickChainChallengeTypeV1::InvalidValidatorSet,);
    }

    #[test]
    fn cryptographically_tampered_checkpoint_signature_builds_invalid_signature_challenge() {
        let set = validator_set();

        let state = cid("state-good");

        let receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let mut artifact = finalized_for_candidate(
            &set,
            candidate(&set, state.clone(), receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        let replacement =
            if artifact.signatures[1].signature_wire.starts_with('0') { "1" } else { "0" };

        artifact.signatures[1].signature_wire.replace_range(0..1, replacement);

        let review = audit_finalized_checkpoint(
            &artifact,
            &observation(&set, state, receipt, &alpha_seed, &beta_seed),
        )
        .expect("tampered-signature review must complete");

        assert_eq!(challenge_type(review,), QuickChainChallengeTypeV1::InvalidValidatorSignature,);
    }

    #[test]
    fn candidate_tampering_without_hash_update_builds_invalid_checkpoint_hash_challenge() {
        let set = validator_set();

        let state = cid("state-good");

        let receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let mut artifact = finalized_for_candidate(
            &set,
            candidate(&set, state.clone(), receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        artifact.candidate.data_availability_root = cid("tampered-da-root");

        let review = audit_finalized_checkpoint(
            &artifact,
            &observation(&set, state, receipt, &alpha_seed, &beta_seed),
        )
        .expect("tampered checkpoint review must complete");

        assert_eq!(challenge_type(review,), QuickChainChallengeTypeV1::InvalidCheckpointHash,);
    }

    #[test]
    fn structurally_tampered_finality_evidence_builds_invalid_checkpoint_evidence_challenge() {
        let set = validator_set();

        let state = cid("state-good");

        let receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let mut artifact = finalized_for_candidate(
            &set,
            candidate(&set, state.clone(), receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        artifact.required_signatures = 1;

        let review = audit_finalized_checkpoint(
            &artifact,
            &observation(&set, state, receipt, &alpha_seed, &beta_seed),
        )
        .expect("tampered finality evidence review must complete");

        assert_eq!(challenge_type(review,), QuickChainChallengeTypeV1::InvalidCheckpointEvidence,);
    }

    #[test]
    fn missing_local_verification_key_does_not_fabricate_network_challenge() {
        let set = validator_set();

        let state = cid("state-good");

        let receipt = cid("receipt-good");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let artifact = finalized_for_candidate(
            &set,
            candidate(&set, state.clone(), receipt.clone()),
            &alpha_seed,
            &beta_seed,
        );

        let mut observation = observation(&set, state, receipt, &alpha_seed, &beta_seed);

        observation.validator_public_keys.retain(|key| key.validator_id != "validator-beta");

        let error = audit_finalized_checkpoint(&artifact, &observation).expect_err(
            "missing local public key must fail local review rather than accuse network",
        );

        assert!(matches!(error, CheckpointAuditError::MissingVerificationKey { .. }),);
    }

    #[test]
    fn tampered_receipt_replay_root_builds_invalid_receipt_root_challenge() {
        let set = validator_set();

        let state = cid("state-good");

        let checkpoint_receipt = cid("receipt-root-from-checkpoint");

        let independently_replayed_receipt = cid("receipt-root-after-tampered-receipt-replay");

        let alpha_seed = [0x11; 32];

        let beta_seed = [0x22; 32];

        let artifact = finalized_for_candidate(
            &set,
            candidate(&set, state.clone(), checkpoint_receipt),
            &alpha_seed,
            &beta_seed,
        );

        let review = audit_finalized_checkpoint(
            &artifact,
            &observation(&set, state, independently_replayed_receipt, &alpha_seed, &beta_seed),
        )
        .expect("tampered receipt-root review must complete");

        assert_eq!(challenge_type(review,), QuickChainChallengeTypeV1::InvalidReceiptRoot,);
    }
}
