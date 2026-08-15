//! RO:WHAT — Deterministically reviews cryptographically signed QuickChain checkpoint candidates against one reviewed validator set.
//! RO:WHY — FINAL_BETA Phase 19 requires membership, key, signature, duplicate, and threshold review before any finality artifact exists.
//! RO:INTERACTS — checkpoint_validator_signing, ron-proto validator-set/signature DTOs, ron-kms Verifier.
//! RO:INVARIANTS — reviewed set only; active unique validators; exact chain/height/epoch/hash/set bindings; ceil(2N/3) threshold; duplicates never increase count.
//! RO:METRICS — none.
//! RO:CONFIG — none; target checkpoint context and reviewed validator set are explicit inputs.
//! RO:SECURITY — committee threshold review only; no finality, fork choice, wallet/ledger mutation, payout, receipt, paid unlock, bridge, staking, or settlement.
//! RO:TEST — focused unit tests in this module.

#![forbid(unsafe_code)]

use std::{collections::HashSet, error::Error as StdError, fmt};

use ron_kms::Verifier;
use ron_proto::{
    quickchain::{
        QuickChainCheckpointValidatorSignatureV1, QuickChainValidatorLifecycleStatusV1,
        QuickChainValidatorSetV1,
    },
    ContentId,
};

use super::checkpoint_validator_signing::{
    CheckpointValidatorParticipant, CheckpointValidatorSigningError,
};

/// Exact unsigned-checkpoint context against which committee signatures are
/// reviewed.
///
/// `validator_set_hash` must be the validator-set commitment already contained
/// in the deterministic unsigned checkpoint candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointCommitteeTarget {
    /// Chain whose checkpoint candidate is under review.
    pub chain_id: String,

    /// Non-zero checkpoint height.
    pub height: u64,

    /// Epoch whose checkpoint candidate is under review.
    pub epoch_id: String,

    /// Exact deterministic unsigned checkpoint candidate hash.
    pub checkpoint_hash: ContentId,

    /// Validator-set commitment contained by the candidate payload.
    pub validator_set_hash: ContentId,
}

/// Deterministic committee-signature review result.
///
/// `threshold_satisfied` means only that enough unique, eligible,
/// cryptographically verified validator signatures were observed. It does not
/// mean the checkpoint is final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointCommitteeReview {
    /// Validator-set commitment reviewed.
    pub validator_set_hash: ContentId,

    /// Checkpoint candidate hash reviewed.
    pub checkpoint_hash: ContentId,

    /// Checkpoint height reviewed.
    pub height: u64,

    /// Number of active members in the reviewed validator set.
    pub active_members: usize,

    /// Deterministic number of signatures required.
    pub required_signatures: usize,

    /// Number of unique cryptographically verified signatures.
    pub verified_unique_signatures: usize,

    /// Whether the deterministic committee signature threshold was satisfied.
    pub threshold_satisfied: bool,
}

/// Deterministic ceil(2N/3) checkpoint-committee threshold.
///
/// A checkpoint committee must contain at least two active validators. This
/// threshold is committee-readiness evidence only and does not create finality.
///
/// # Errors
///
/// Rejects committees smaller than two active validators or impossible integer
/// overflow.
pub fn checkpoint_committee_required_signatures(
    active_members: usize,
) -> Result<usize, CheckpointCommitteeReviewError> {
    if active_members < 2 {
        return Err(CheckpointCommitteeReviewError::CommitteeTooSmall { active_members });
    }

    let doubled = active_members
        .checked_mul(2)
        .ok_or(CheckpointCommitteeReviewError::ThresholdArithmeticOverflow)?;

    let numerator = doubled
        .checked_add(2)
        .ok_or(CheckpointCommitteeReviewError::ThresholdArithmeticOverflow)?;

    Ok(numerator / 3)
}

/// Review checkpoint-validator signatures against one reviewed validator set.
///
/// The function:
///
/// - validates the validator set;
/// - binds it to the candidate's validator-set commitment;
/// - counts active members deterministically;
/// - rejects duplicate active signing keys;
/// - rejects duplicate validator signatures;
/// - rejects unknown or inactive validators;
/// - requires exact validator/key/algorithm bindings;
/// - requires exact chain/height/epoch/checkpoint bindings;
/// - cryptographically verifies every accepted signature;
/// - computes ceil(2N/3) over active members.
///
/// It does not create a finality artifact.
///
/// # Errors
///
/// Returns the first deterministic validator-set, target, signer, binding, or
/// cryptographic verification failure.
pub fn review_checkpoint_committee_signatures(
    target: &CheckpointCommitteeTarget,
    validator_set: &QuickChainValidatorSetV1,
    participants: &[CheckpointValidatorParticipant],
    verifier: &dyn Verifier,
    signatures: &[QuickChainCheckpointValidatorSignatureV1],
) -> Result<CheckpointCommitteeReview, CheckpointCommitteeReviewError> {
    if target.height == 0 {
        return Err(CheckpointCommitteeReviewError::ZeroCheckpointHeight);
    }

    validator_set.validate().map_err(|error| {
        CheckpointCommitteeReviewError::InvalidValidatorSet(format!("{error:?}"))
    })?;

    if validator_set.validator_set_hash != target.validator_set_hash {
        return Err(CheckpointCommitteeReviewError::ValidatorSetHashMismatch);
    }

    if validator_set.chain_id != target.chain_id {
        return Err(CheckpointCommitteeReviewError::ValidatorSetChainMismatch);
    }

    if validator_set.epoch_id != target.epoch_id {
        return Err(CheckpointCommitteeReviewError::ValidatorSetEpochMismatch);
    }

    let active_members = validator_set
        .members
        .iter()
        .filter(|member| member.lifecycle_status == QuickChainValidatorLifecycleStatusV1::Active)
        .collect::<Vec<_>>();

    let required_signatures = checkpoint_committee_required_signatures(active_members.len())?;

    let mut active_key_ids = HashSet::<&str>::with_capacity(active_members.len());

    for member in &active_members {
        if !active_key_ids.insert(member.key_id.as_str()) {
            return Err(CheckpointCommitteeReviewError::DuplicateActiveKeyId {
                key_id: member.key_id.clone(),
            });
        }
    }

    let mut participant_ids = HashSet::<&str>::with_capacity(participants.len());

    for participant in participants {
        if !participant_ids.insert(participant.validator_id()) {
            return Err(CheckpointCommitteeReviewError::DuplicateParticipant {
                validator_id: participant.validator_id().to_owned(),
            });
        }
    }

    let mut reviewed_signers = HashSet::<&str>::with_capacity(signatures.len());

    let mut verified_unique_signatures = 0usize;

    for signature in signatures {
        signature.validate().map_err(|error| {
            CheckpointCommitteeReviewError::InvalidSignatureArtifact(format!("{error:?}"))
        })?;

        if signature.chain_id != target.chain_id {
            return Err(CheckpointCommitteeReviewError::SignatureChainMismatch {
                validator_id: signature.validator_id.clone(),
            });
        }

        if signature.height != target.height {
            return Err(CheckpointCommitteeReviewError::SignatureHeightMismatch {
                validator_id: signature.validator_id.clone(),
            });
        }

        if signature.epoch_id != target.epoch_id {
            return Err(CheckpointCommitteeReviewError::SignatureEpochMismatch {
                validator_id: signature.validator_id.clone(),
            });
        }

        if signature.checkpoint_hash != target.checkpoint_hash {
            return Err(
                CheckpointCommitteeReviewError::SignatureCheckpointHashMismatch {
                    validator_id: signature.validator_id.clone(),
                },
            );
        }

        if !reviewed_signers.insert(signature.validator_id.as_str()) {
            return Err(
                CheckpointCommitteeReviewError::DuplicateValidatorSignature {
                    validator_id: signature.validator_id.clone(),
                },
            );
        }

        let member = validator_set
            .members
            .iter()
            .find(|member| member.validator_id == signature.validator_id)
            .ok_or_else(|| CheckpointCommitteeReviewError::UnknownValidator {
                validator_id: signature.validator_id.clone(),
            })?;

        if member.lifecycle_status != QuickChainValidatorLifecycleStatusV1::Active {
            return Err(CheckpointCommitteeReviewError::InactiveValidator {
                validator_id: signature.validator_id.clone(),
            });
        }

        if member.key_id != signature.key_id {
            return Err(CheckpointCommitteeReviewError::ValidatorKeyMismatch {
                validator_id: signature.validator_id.clone(),
            });
        }

        if member.signature_algorithm != signature.algorithm {
            return Err(CheckpointCommitteeReviewError::ValidatorAlgorithmMismatch {
                validator_id: signature.validator_id.clone(),
            });
        }

        let participant = participants
            .iter()
            .find(|participant| participant.validator_id() == signature.validator_id)
            .ok_or_else(
                || CheckpointCommitteeReviewError::MissingVerificationParticipant {
                    validator_id: signature.validator_id.clone(),
                },
            )?;

        if participant.chain_id() != member.chain_id
            || participant.epoch_id() != member.epoch_id
            || participant.logical_key_id() != member.key_id
        {
            return Err(CheckpointCommitteeReviewError::ParticipantBindingMismatch {
                validator_id: signature.validator_id.clone(),
            });
        }

        let verifies = participant
            .verify_signature(verifier, signature)
            .map_err(
                |error| CheckpointCommitteeReviewError::CryptographicReviewFailed {
                    validator_id: signature.validator_id.clone(),
                    reason: error.to_string(),
                },
            )?;

        if !verifies {
            return Err(
                CheckpointCommitteeReviewError::CryptographicSignatureInvalid {
                    validator_id: signature.validator_id.clone(),
                },
            );
        }

        verified_unique_signatures = verified_unique_signatures
            .checked_add(1)
            .ok_or(CheckpointCommitteeReviewError::ThresholdArithmeticOverflow)?;
    }

    Ok(CheckpointCommitteeReview {
        validator_set_hash: target.validator_set_hash.clone(),
        checkpoint_hash: target.checkpoint_hash.clone(),
        height: target.height,
        active_members: active_members.len(),
        required_signatures,
        verified_unique_signatures,
        threshold_satisfied: verified_unique_signatures >= required_signatures,
    })
}

/// Checkpoint committee validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointCommitteeReviewError {
    /// The reviewed validator-set DTO itself is invalid.
    InvalidValidatorSet(String),

    /// Candidate checkpoint height is zero.
    ZeroCheckpointHeight,

    /// Candidate validator-set commitment differs from the reviewed set.
    ValidatorSetHashMismatch,

    /// Candidate chain differs from the reviewed validator set.
    ValidatorSetChainMismatch,

    /// Candidate epoch differs from the reviewed validator set.
    ValidatorSetEpochMismatch,

    /// Fewer than two active validators exist.
    CommitteeTooSmall {
        /// Number of active validators.
        active_members: usize,
    },

    /// Threshold arithmetic overflowed.
    ThresholdArithmeticOverflow,

    /// Two active validators share one reviewed logical signing key.
    DuplicateActiveKeyId {
        /// Duplicate logical signing key.
        key_id: String,
    },

    /// Local verifier configuration contains the same validator twice.
    DuplicateParticipant {
        /// Duplicate validator identifier.
        validator_id: String,
    },

    /// Signature artifact failed DTO validation.
    InvalidSignatureArtifact(String),

    /// Signature targets another chain.
    SignatureChainMismatch {
        /// Validator claiming the signature.
        validator_id: String,
    },

    /// Signature targets another checkpoint height.
    SignatureHeightMismatch {
        /// Validator claiming the signature.
        validator_id: String,
    },

    /// Signature targets another epoch.
    SignatureEpochMismatch {
        /// Validator claiming the signature.
        validator_id: String,
    },

    /// Signature targets another checkpoint hash.
    SignatureCheckpointHashMismatch {
        /// Validator claiming the signature.
        validator_id: String,
    },

    /// Same validator signature was supplied more than once.
    DuplicateValidatorSignature {
        /// Duplicate validator identifier.
        validator_id: String,
    },

    /// Signature claims a validator absent from the reviewed set.
    UnknownValidator {
        /// Unknown validator identifier.
        validator_id: String,
    },

    /// Validator is present but not active.
    InactiveValidator {
        /// Inactive validator identifier.
        validator_id: String,
    },

    /// Signature key identity differs from reviewed membership.
    ValidatorKeyMismatch {
        /// Validator whose key binding mismatched.
        validator_id: String,
    },

    /// Signature algorithm differs from reviewed membership.
    ValidatorAlgorithmMismatch {
        /// Validator whose algorithm binding mismatched.
        validator_id: String,
    },

    /// No local verification participant exists for this reviewed validator.
    MissingVerificationParticipant {
        /// Validator missing a verifier binding.
        validator_id: String,
    },

    /// Local KMS participant does not match reviewed chain/epoch/key bindings.
    ParticipantBindingMismatch {
        /// Validator with mismatched local verifier configuration.
        validator_id: String,
    },

    /// KMS verification could not complete.
    CryptographicReviewFailed {
        /// Validator whose verification operation failed.
        validator_id: String,

        /// Redacted/local failure description.
        reason: String,
    },

    /// Signature was well-formed but cryptographically invalid.
    CryptographicSignatureInvalid {
        /// Validator whose signature failed verification.
        validator_id: String,
    },
}

impl fmt::Display for CheckpointCommitteeReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValidatorSet(reason) => {
                write!(formatter, "invalid checkpoint validator set: {reason}")
            }
            Self::ZeroCheckpointHeight => {
                formatter.write_str("checkpoint committee target height must be greater than zero")
            }
            Self::ValidatorSetHashMismatch => formatter.write_str(
                "checkpoint candidate validator-set hash does not match reviewed validator set",
            ),
            Self::ValidatorSetChainMismatch => formatter
                .write_str("checkpoint candidate chain does not match reviewed validator set"),
            Self::ValidatorSetEpochMismatch => formatter
                .write_str("checkpoint candidate epoch does not match reviewed validator set"),
            Self::CommitteeTooSmall { active_members } => {
                write!(
                    formatter,
                    "checkpoint committee requires at least two active validators; found {active_members}"
                )
            }
            Self::ThresholdArithmeticOverflow => {
                formatter.write_str("checkpoint committee threshold arithmetic overflow")
            }
            Self::DuplicateActiveKeyId { key_id } => {
                write!(
                    formatter,
                    "checkpoint committee active validators share key id {key_id}"
                )
            }
            Self::DuplicateParticipant { validator_id } => {
                write!(
                    formatter,
                    "duplicate checkpoint verification participant {validator_id}"
                )
            }
            Self::InvalidSignatureArtifact(reason) => {
                write!(
                    formatter,
                    "invalid checkpoint validator signature artifact: {reason}"
                )
            }
            Self::SignatureChainMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature chain mismatch for {validator_id}"
                )
            }
            Self::SignatureHeightMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature height mismatch for {validator_id}"
                )
            }
            Self::SignatureEpochMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature epoch mismatch for {validator_id}"
                )
            }
            Self::SignatureCheckpointHashMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature hash mismatch for {validator_id}"
                )
            }
            Self::DuplicateValidatorSignature { validator_id } => {
                write!(
                    formatter,
                    "duplicate checkpoint validator signature from {validator_id}"
                )
            }
            Self::UnknownValidator { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature validator {validator_id} is not in reviewed set"
                )
            }
            Self::InactiveValidator { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature validator {validator_id} is not active"
                )
            }
            Self::ValidatorKeyMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature key does not match reviewed validator {validator_id}"
                )
            }
            Self::ValidatorAlgorithmMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature algorithm does not match reviewed validator {validator_id}"
                )
            }
            Self::MissingVerificationParticipant { validator_id } => {
                write!(
                    formatter,
                    "missing checkpoint verification participant for {validator_id}"
                )
            }
            Self::ParticipantBindingMismatch { validator_id } => {
                write!(
                    formatter,
                    "checkpoint verification participant bindings do not match reviewed validator {validator_id}"
                )
            }
            Self::CryptographicReviewFailed {
                validator_id,
                reason,
            } => {
                write!(
                    formatter,
                    "checkpoint signature verification failed for {validator_id}: {reason}"
                )
            }
            Self::CryptographicSignatureInvalid { validator_id } => {
                write!(
                    formatter,
                    "checkpoint signature is cryptographically invalid for {validator_id}"
                )
            }
        }
    }
}

impl StdError for CheckpointCommitteeReviewError {}

impl From<CheckpointValidatorSigningError> for CheckpointCommitteeReviewError {
    fn from(error: CheckpointValidatorSigningError) -> Self {
        Self::CryptographicReviewFailed {
            validator_id: "unknown".to_owned(),
            reason: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_kms::{backends::memory::MemoryKeystore, memory_keystore, Keystore};
    use ron_proto::{
        quantum::SignatureAlg,
        quickchain::{
            QuickChainValidatorIdentityV1, QUICKCHAIN_DTO_VERSION,
            QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
            QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1,
            QUICKCHAIN_VALIDATOR_SET_SCHEMA,
        },
    };

    const CHAIN_ID: &str = "ron-devnet";
    const EPOCH_ID: &str = "epoch_phase19_checkpoint_committee";

    fn cid(label: &str) -> ContentId {
        let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

        format!("b3:{digest}")
            .parse()
            .expect("fixture content ID must parse")
    }

    fn identity(
        suffix: &str,
        status: QuickChainValidatorLifecycleStatusV1,
    ) -> QuickChainValidatorIdentityV1 {
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
            lifecycle_status: status,
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
            validator_set_hash: cid("checkpoint-validator-set"),
            policy_hash: cid("checkpoint-policy"),
            registry_snapshot_hash: cid("checkpoint-registry"),
            passport_required: true,
            bond_required: false,
            validator_set_algorithm: QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1
                .to_owned(),
            members: vec![
                identity("alpha", QuickChainValidatorLifecycleStatusV1::Active),
                identity("beta", QuickChainValidatorLifecycleStatusV1::Active),
                identity("gamma", QuickChainValidatorLifecycleStatusV1::Active),
            ],
        }
    }

    fn target(set: &QuickChainValidatorSetV1) -> CheckpointCommitteeTarget {
        CheckpointCommitteeTarget {
            chain_id: CHAIN_ID.to_owned(),
            height: 19,
            epoch_id: EPOCH_ID.to_owned(),
            checkpoint_hash: cid("phase19-checkpoint-candidate"),
            validator_set_hash: set.validator_set_hash.clone(),
        }
    }

    fn participants(
        kms: &MemoryKeystore,
        set: &QuickChainValidatorSetV1,
    ) -> Vec<CheckpointValidatorParticipant> {
        set.members
            .iter()
            .map(|member| {
                let key = kms
                    .create_ed25519("checkpoint-validator", &member.validator_id)
                    .expect("fixture Ed25519 key must be created");

                CheckpointValidatorParticipant::from_reviewed_identity(member, key)
                    .expect("active checkpoint validator participant must build")
            })
            .collect()
    }

    fn sign(
        kms: &MemoryKeystore,
        participant: &CheckpointValidatorParticipant,
        target: &CheckpointCommitteeTarget,
    ) -> QuickChainCheckpointValidatorSignatureV1 {
        participant
            .sign_checkpoint(kms, target.height, target.checkpoint_hash.clone())
            .expect("fixture checkpoint signature must be created")
    }

    #[test]
    fn checkpoint_threshold_is_deterministic_ceil_two_thirds() {
        assert_eq!(
            checkpoint_committee_required_signatures(2).expect("two-member threshold"),
            2,
        );
        assert_eq!(
            checkpoint_committee_required_signatures(3).expect("three-member threshold"),
            2,
        );
        assert_eq!(
            checkpoint_committee_required_signatures(4).expect("four-member threshold"),
            3,
        );
        assert_eq!(
            checkpoint_committee_required_signatures(5).expect("five-member threshold"),
            4,
        );
        assert_eq!(
            checkpoint_committee_required_signatures(6).expect("six-member threshold"),
            4,
        );

        assert!(matches!(
            checkpoint_committee_required_signatures(1),
            Err(CheckpointCommitteeReviewError::CommitteeTooSmall { active_members: 1 }),
        ));
    }

    #[test]
    fn one_of_three_is_insufficient_without_becoming_an_error() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let signatures = vec![sign(&kms, &participants[0], &target)];

        let review =
            review_checkpoint_committee_signatures(&target, &set, &participants, &kms, &signatures)
                .expect("valid one-of-three review must complete");

        assert_eq!(review.active_members, 3,);
        assert_eq!(review.required_signatures, 2,);
        assert_eq!(review.verified_unique_signatures, 1,);
        assert!(!review.threshold_satisfied,);
    }

    #[test]
    fn two_of_three_satisfies_threshold_independent_of_signature_order() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let alpha = sign(&kms, &participants[0], &target);
        let beta = sign(&kms, &participants[1], &target);

        let forward = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[alpha.clone(), beta.clone()],
        )
        .expect("forward threshold review must complete");

        let reverse = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[beta, alpha],
        )
        .expect("reverse threshold review must complete");

        assert_eq!(forward, reverse,);
        assert_eq!(forward.required_signatures, 2,);
        assert_eq!(forward.verified_unique_signatures, 2,);
        assert!(forward.threshold_satisfied,);
    }

    #[test]
    fn duplicate_validator_signature_rejects_instead_of_increasing_count() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let signature = sign(&kms, &participants[0], &target);

        let error = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[signature.clone(), signature],
        )
        .expect_err("duplicate signer must reject");

        assert!(matches!(
            error,
            CheckpointCommitteeReviewError::DuplicateValidatorSignature { .. }
        ));
    }

    #[test]
    fn unknown_validator_rejects_before_crypto_counting() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let mut signature = sign(&kms, &participants[0], &target);

        signature.validator_id = "validator-unknown".to_owned();

        let error = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[signature],
        )
        .expect_err("unknown validator must reject");

        assert!(matches!(
            error,
            CheckpointCommitteeReviewError::UnknownValidator { .. }
        ));
    }

    #[test]
    fn reviewed_validator_key_mismatch_rejects() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let mut signature = sign(&kms, &participants[0], &target);

        signature.key_id = set.members[1].key_id.clone();

        let error = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[signature],
        )
        .expect_err("wrong reviewed key must reject");

        assert!(matches!(
            error,
            CheckpointCommitteeReviewError::ValidatorKeyMismatch { .. }
        ));
    }

    #[test]
    fn wrong_chain_height_epoch_and_checkpoint_hash_each_reject() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let original = sign(&kms, &participants[0], &target);

        let mut wrong_chain = original.clone();
        wrong_chain.chain_id = "ron-other".to_owned();

        assert!(matches!(
            review_checkpoint_committee_signatures(
                &target,
                &set,
                &participants,
                &kms,
                &[wrong_chain],
            ),
            Err(CheckpointCommitteeReviewError::SignatureChainMismatch { .. }),
        ));

        let mut wrong_height = original.clone();
        wrong_height.height += 1;

        assert!(matches!(
            review_checkpoint_committee_signatures(
                &target,
                &set,
                &participants,
                &kms,
                &[wrong_height],
            ),
            Err(CheckpointCommitteeReviewError::SignatureHeightMismatch { .. }),
        ));

        let mut wrong_epoch = original.clone();
        wrong_epoch.epoch_id = "epoch-other".to_owned();

        assert!(matches!(
            review_checkpoint_committee_signatures(
                &target,
                &set,
                &participants,
                &kms,
                &[wrong_epoch],
            ),
            Err(CheckpointCommitteeReviewError::SignatureEpochMismatch { .. }),
        ));

        let mut wrong_hash = original;
        wrong_hash.checkpoint_hash = cid("different-checkpoint");

        assert!(matches!(
            review_checkpoint_committee_signatures(
                &target,
                &set,
                &participants,
                &kms,
                &[wrong_hash],
            ),
            Err(CheckpointCommitteeReviewError::SignatureCheckpointHashMismatch { .. }),
        ));
    }

    #[test]
    fn cryptographically_tampered_signature_rejects() {
        let kms = memory_keystore();
        let set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let mut signature = sign(&kms, &participants[0], &target);

        let replacement = if signature.signature_wire.starts_with('0') {
            "1"
        } else {
            "0"
        };

        signature.signature_wire.replace_range(0..1, replacement);

        let error = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[signature],
        )
        .expect_err("tampered signature must reject");

        assert!(matches!(
            error,
            CheckpointCommitteeReviewError::CryptographicSignatureInvalid { .. }
        ));
    }

    #[test]
    fn inactive_reviewed_validator_signature_rejects() {
        let kms = memory_keystore();
        let mut set = validator_set();
        let target = target(&set);
        let participants = participants(&kms, &set);

        let signature = sign(&kms, &participants[0], &target);

        set.members[0].lifecycle_status = QuickChainValidatorLifecycleStatusV1::Revoked;

        let error = review_checkpoint_committee_signatures(
            &target,
            &set,
            &participants,
            &kms,
            &[signature],
        )
        .expect_err("inactive signer must reject");

        assert!(matches!(
            error,
            CheckpointCommitteeReviewError::InactiveValidator { .. }
        ));
    }

    #[test]
    fn candidate_validator_set_hash_mismatch_rejects() {
        let kms = memory_keystore();
        let set = validator_set();
        let mut target = target(&set);
        let participants = participants(&kms, &set);

        target.validator_set_hash = cid("wrong-validator-set");

        let error = review_checkpoint_committee_signatures(&target, &set, &participants, &kms, &[])
            .expect_err("candidate/set commitment mismatch must reject");

        assert_eq!(
            error,
            CheckpointCommitteeReviewError::ValidatorSetHashMismatch,
        );
    }

    #[test]
    fn active_validators_cannot_share_one_logical_signing_key() {
        let kms = memory_keystore();
        let mut set = validator_set();

        set.members[1].key_id = set.members[0].key_id.clone();

        let target = target(&set);

        let error = review_checkpoint_committee_signatures(&target, &set, &[], &kms, &[])
            .expect_err("shared active validator key must reject");

        assert!(matches!(
            error,
            CheckpointCommitteeReviewError::DuplicateActiveKeyId { .. }
        ));
    }
}
