//! RO:WHAT — Builds one finalized QuickChain checkpoint only after deterministic hash reproduction and cryptographic committee threshold review.
//! RO:WHY — FINAL_BETA Phase 19 requires finality to derive from verified committee evidence rather than one process, signer, HTTP response, or client.
//! RO:INTERACTS — ron-proto committee/finality DTOs, checkpoint committee validation, checkpoint validator signing, ron-kms Verifier.
//! RO:INVARIANTS — candidate hash is independently recomputed; reviewed validator-set commitment must match; threshold must be satisfied; signatures are sorted deterministically before finality construction.
//! RO:METRICS — none.
//! RO:CONFIG — none; candidate, validator set, verifier bindings, and signatures are explicit inputs.
//! RO:SECURITY — local threshold-backed finality evidence only; no fork choice, persistence authority, wallet/ledger mutation, payout, receipt, bridge, staking, settlement, or CrabLink finality authority.
//! RO:TEST — focused unit tests in this module.

#![forbid(unsafe_code)]

use std::{error::Error as StdError, fmt};

use ron_kms::Verifier;
use ron_proto::{
    to_canonical_json_vec, ContentId, QuickChainCheckpointFinalityStateV1,
    QuickChainCheckpointValidatorSignatureV1, QuickChainCommitteeCheckpointPayloadV1,
    QuickChainFinalizedCheckpointV1, QuickChainValidatorSetV1,
    QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA,
};

use super::{
    checkpoint_committee_validation::{
        review_checkpoint_committee_signatures, CheckpointCommitteeReviewError,
        CheckpointCommitteeTarget,
    },
    checkpoint_validator_signing::CheckpointValidatorParticipant,
};

/// Build one threshold-backed finalized checkpoint artifact.
///
/// Finality construction proceeds in this exact order:
///
/// 1. validate the unsigned committee candidate;
/// 2. independently recompute its checkpoint hash;
/// 3. reject any caller-supplied hash mismatch;
/// 4. perform the full cryptographic committee review;
/// 5. require the deterministic committee threshold;
/// 6. sort the verified signatures by validator identity;
/// 7. build and validate the finalized-checkpoint protocol artifact.
///
/// The returned artifact carries committee-derived finality evidence. This
/// function does not grant finality authority to the local process itself.
///
/// # Errors
///
/// Returns the first candidate, hash, committee, threshold, count-conversion,
/// or finality-artifact validation failure.
pub fn build_finalized_checkpoint_from_committee(
    candidate: &QuickChainCommitteeCheckpointPayloadV1,
    checkpoint_hash: &ContentId,
    validator_set: &QuickChainValidatorSetV1,
    participants: &[CheckpointValidatorParticipant],
    verifier: &dyn Verifier,
    signatures: &[QuickChainCheckpointValidatorSignatureV1],
) -> Result<QuickChainFinalizedCheckpointV1, CheckpointFinalityError> {
    candidate
        .validate()
        .map_err(|error| CheckpointFinalityError::InvalidCandidate(format!("{error:?}")))?;

    let recomputed_hash = recompute_checkpoint_candidate_hash(candidate)?;

    if &recomputed_hash != checkpoint_hash {
        return Err(CheckpointFinalityError::CheckpointHashMismatch);
    }

    let target = CheckpointCommitteeTarget {
        chain_id: candidate.chain_id.clone(),
        height: candidate.height,
        epoch_id: candidate.epoch_id.clone(),
        checkpoint_hash: checkpoint_hash.clone(),
        validator_set_hash: candidate.validator_set_hash.clone(),
    };

    let review = review_checkpoint_committee_signatures(
        &target,
        validator_set,
        participants,
        verifier,
        signatures,
    )
    .map_err(CheckpointFinalityError::CommitteeReview)?;

    if !review.threshold_satisfied {
        return Err(CheckpointFinalityError::ThresholdNotSatisfied {
            verified: review.verified_unique_signatures,
            required: review.required_signatures,
        });
    }

    let required_signatures = u32::try_from(review.required_signatures)
        .map_err(|_| CheckpointFinalityError::CountConversion)?;

    let verified_unique_signatures = u32::try_from(review.verified_unique_signatures)
        .map_err(|_| CheckpointFinalityError::CountConversion)?;

    let mut ordered_signatures = signatures.to_vec();

    ordered_signatures.sort_by(|left, right| left.validator_id.cmp(&right.validator_id));

    let artifact = QuickChainFinalizedCheckpointV1 {
        schema: QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        candidate: candidate.clone(),
        checkpoint_hash: checkpoint_hash.clone(),
        validator_set: validator_set.clone(),
        required_signatures,
        verified_unique_signatures,
        signatures: ordered_signatures,
        finalization_state: QuickChainCheckpointFinalityStateV1::Finalized,
    };

    artifact
        .validate()
        .map_err(|error| CheckpointFinalityError::InvalidFinalityArtifact(format!("{error:?}")))?;

    Ok(artifact)
}

/// Local threshold-gated checkpoint finality construction failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointFinalityError {
    /// Unsigned checkpoint candidate failed protocol validation.
    InvalidCandidate(String),

    /// Canonical candidate serialization failed.
    CandidateCanonicalization(String),

    /// Recomputed BLAKE3 content ID could not be represented as a protocol ID.
    InvalidComputedCheckpointHash,

    /// Caller-supplied checkpoint hash differs from independently recomputed
    /// canonical candidate hash.
    CheckpointHashMismatch,

    /// Committee membership, binding, or cryptographic review failed.
    CommitteeReview(CheckpointCommitteeReviewError),

    /// Valid signatures were present but did not satisfy the deterministic
    /// committee threshold.
    ThresholdNotSatisfied {
        /// Number of unique cryptographically verified signatures.
        verified: usize,

        /// Number required by the reviewed active committee.
        required: usize,
    },

    /// Internal committee count could not fit the finalized-checkpoint wire
    /// representation.
    CountConversion,

    /// Constructed finalized-checkpoint DTO failed structural validation.
    InvalidFinalityArtifact(String),
}

impl fmt::Display for CheckpointFinalityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCandidate(reason) => {
                write!(formatter, "invalid checkpoint candidate: {reason}")
            }
            Self::CandidateCanonicalization(reason) => {
                write!(
                    formatter,
                    "checkpoint candidate canonicalization failed: {reason}"
                )
            }
            Self::InvalidComputedCheckpointHash => formatter
                .write_str("recomputed checkpoint candidate hash is not a valid content id"),
            Self::CheckpointHashMismatch => formatter.write_str(
                "supplied checkpoint hash does not match canonical checkpoint candidate",
            ),
            Self::CommitteeReview(error) => {
                write!(formatter, "checkpoint committee review failed: {error}")
            }
            Self::ThresholdNotSatisfied { verified, required } => {
                write!(
                    formatter,
                    "checkpoint committee threshold not satisfied: verified {verified}, required {required}"
                )
            }
            Self::CountConversion => formatter
                .write_str("checkpoint committee count exceeds finalized-checkpoint wire range"),
            Self::InvalidFinalityArtifact(reason) => {
                write!(
                    formatter,
                    "constructed finalized checkpoint is invalid: {reason}"
                )
            }
        }
    }
}

impl StdError for CheckpointFinalityError {}

fn recompute_checkpoint_candidate_hash(
    candidate: &QuickChainCommitteeCheckpointPayloadV1,
) -> Result<ContentId, CheckpointFinalityError> {
    let canonical = to_canonical_json_vec(candidate)
        .map_err(|error| CheckpointFinalityError::CandidateCanonicalization(error.to_string()))?;

    let mut hasher = blake3::Hasher::new();

    hasher.update(QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());

    hasher.update(&[0]);

    hasher.update(&canonical);

    format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .map_err(|_| CheckpointFinalityError::InvalidComputedCheckpointHash)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_kms::{backends::memory::MemoryKeystore, memory_keystore, Keystore};
    use ron_proto::{
        QuickChainCanonicalEncodingV1, QuickChainConservationV1, QuickChainReceiptRootSchemeV1,
        QuickChainSettlementModeV1, QuickChainStateRootSchemeV1, QuickChainSupplyDeltaV1,
        QuickChainValidatorIdentityV1, QuickChainValidatorLifecycleStatusV1, SignatureAlg,
        QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA, QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1, QUICKCHAIN_VALIDATOR_SET_SCHEMA,
    };

    const CHAIN_ID: &str = "ron-devnet";

    const EPOCH_ID: &str = "epoch_phase19_local_finality";

    fn cid(label: &str) -> ContentId {
        let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

        format!("b3:{digest}")
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
            validator_set_hash: cid("phase19-local-finality-validator-set"),
            policy_hash: cid("phase19-local-finality-policy"),
            registry_snapshot_hash: cid("phase19-local-finality-registry"),
            passport_required: true,
            bond_required: false,
            validator_set_algorithm: QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1
                .to_owned(),
            members: vec![member("alpha"), member("beta"), member("gamma")],
        }
    }

    fn candidate(set: &QuickChainValidatorSetV1) -> QuickChainCommitteeCheckpointPayloadV1 {
        QuickChainCommitteeCheckpointPayloadV1 {
            schema: QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: CHAIN_ID.to_owned(),
            height: 19,
            epoch_id: EPOCH_ID.to_owned(),
            execution_spec_version: "quickchain-execution-v1".to_owned(),
            previous_checkpoint_hash: cid("phase19-finality-previous-checkpoint"),
            previous_state_root: cid("phase19-finality-previous-state"),
            new_state_root: cid("phase19-finality-new-state"),
            receipt_root: cid("phase19-finality-receipt-root"),
            accounting_snapshot_root: cid("phase19-finality-accounting-root"),
            reward_manifest_root: cid("phase19-finality-reward-root"),
            data_availability_root: cid("phase19-finality-da-root"),
            policy_hash: cid("phase19-local-finality-policy"),
            validator_set_hash: set.validator_set_hash.clone(),
            chain_params_hash: cid("phase19-finality-chain-params"),
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

    fn participants(
        kms: &MemoryKeystore,
        set: &QuickChainValidatorSetV1,
    ) -> Vec<CheckpointValidatorParticipant> {
        set.members
            .iter()
            .map(|member| {
                let key = kms
                    .create_ed25519("checkpoint-validator", &member.validator_id)
                    .expect("fixture validator key must be created");

                CheckpointValidatorParticipant::from_reviewed_identity(member, key)
                    .expect("fixture validator participant must build")
            })
            .collect()
    }

    fn sign(
        kms: &MemoryKeystore,
        participant: &CheckpointValidatorParticipant,
        checkpoint_hash: &ContentId,
    ) -> QuickChainCheckpointValidatorSignatureV1 {
        participant
            .sign_checkpoint(kms, 19, checkpoint_hash.clone())
            .expect("fixture checkpoint signature must build")
    }

    #[test]
    fn two_of_three_verified_signatures_build_finalized_checkpoint() {
        let kms = memory_keystore();

        let set = validator_set();

        let candidate = candidate(&set);

        let checkpoint_hash =
            recompute_checkpoint_candidate_hash(&candidate).expect("candidate hash must recompute");

        let participants = participants(&kms, &set);

        let signatures = vec![
            sign(&kms, &participants[0], &checkpoint_hash),
            sign(&kms, &participants[1], &checkpoint_hash),
        ];

        let finalized = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &signatures,
        )
        .expect("two-of-three verified committee must produce finality artifact");

        assert_eq!(finalized.required_signatures, 2,);

        assert_eq!(finalized.verified_unique_signatures, 2,);

        assert_eq!(finalized.checkpoint_hash, checkpoint_hash,);

        assert_eq!(
            finalized.finalization_state,
            QuickChainCheckpointFinalityStateV1::Finalized,
        );

        finalized
            .validate()
            .expect("constructed finalized checkpoint must validate");
    }

    #[test]
    fn one_of_three_cannot_produce_finality_artifact() {
        let kms = memory_keystore();

        let set = validator_set();

        let candidate = candidate(&set);

        let checkpoint_hash =
            recompute_checkpoint_candidate_hash(&candidate).expect("candidate hash must recompute");

        let participants = participants(&kms, &set);

        let signatures = vec![sign(&kms, &participants[0], &checkpoint_hash)];

        let error = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &signatures,
        )
        .expect_err("one-of-three committee must not produce finality");

        assert_eq!(
            error,
            CheckpointFinalityError::ThresholdNotSatisfied {
                verified: 1,
                required: 2,
            },
        );
    }

    #[test]
    fn supplied_checkpoint_hash_must_match_canonical_candidate() {
        let kms = memory_keystore();

        let set = validator_set();

        let candidate = candidate(&set);

        let participants = participants(&kms, &set);

        let error = build_finalized_checkpoint_from_committee(
            &candidate,
            &cid("caller-supplied-wrong-checkpoint-hash"),
            &set,
            &participants,
            &kms,
            &[],
        )
        .expect_err("caller-supplied wrong checkpoint hash must reject");

        assert_eq!(error, CheckpointFinalityError::CheckpointHashMismatch,);
    }

    #[test]
    fn signature_input_order_cannot_change_finalized_artifact() {
        let kms = memory_keystore();

        let set = validator_set();

        let candidate = candidate(&set);

        let checkpoint_hash =
            recompute_checkpoint_candidate_hash(&candidate).expect("candidate hash must recompute");

        let participants = participants(&kms, &set);

        let alpha = sign(&kms, &participants[0], &checkpoint_hash);

        let beta = sign(&kms, &participants[1], &checkpoint_hash);

        let forward = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &[alpha.clone(), beta.clone()],
        )
        .expect("forward signature order must finalize");

        let reverse = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &[beta, alpha],
        )
        .expect("reverse signature order must finalize");

        assert_eq!(forward, reverse,);

        assert_eq!(forward.signatures[0].validator_id, "validator-alpha",);

        assert_eq!(forward.signatures[1].validator_id, "validator-beta",);
    }

    #[test]
    fn tampered_signature_cannot_produce_finality() {
        let kms = memory_keystore();

        let set = validator_set();

        let candidate = candidate(&set);

        let checkpoint_hash =
            recompute_checkpoint_candidate_hash(&candidate).expect("candidate hash must recompute");

        let participants = participants(&kms, &set);

        let alpha = sign(&kms, &participants[0], &checkpoint_hash);

        let mut beta = sign(&kms, &participants[1], &checkpoint_hash);

        let replacement = if beta.signature_wire.starts_with('0') {
            "1"
        } else {
            "0"
        };

        beta.signature_wire.replace_range(0..1, replacement);

        let error = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &[alpha, beta],
        )
        .expect_err("tampered committee signature must prevent finality");

        assert!(matches!(error, CheckpointFinalityError::CommitteeReview(_)),);
    }

    #[test]
    fn candidate_mutation_after_signing_cannot_reuse_old_checkpoint_hash() {
        let kms = memory_keystore();

        let set = validator_set();

        let mut candidate = candidate(&set);

        let checkpoint_hash =
            recompute_checkpoint_candidate_hash(&candidate).expect("candidate hash must recompute");

        let participants = participants(&kms, &set);

        let signatures = vec![
            sign(&kms, &participants[0], &checkpoint_hash),
            sign(&kms, &participants[1], &checkpoint_hash),
        ];

        candidate.new_state_root = cid("mutated-state-after-signing");

        let error = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &signatures,
        )
        .expect_err("mutated candidate cannot reuse old signed hash");

        assert_eq!(error, CheckpointFinalityError::CheckpointHashMismatch,);
    }

    #[test]
    fn finality_artifact_contains_no_process_wallet_or_ledger_authority() {
        let kms = memory_keystore();

        let set = validator_set();

        let candidate = candidate(&set);

        let checkpoint_hash =
            recompute_checkpoint_candidate_hash(&candidate).expect("candidate hash must recompute");

        let participants = participants(&kms, &set);

        let signatures = vec![
            sign(&kms, &participants[0], &checkpoint_hash),
            sign(&kms, &participants[1], &checkpoint_hash),
        ];

        let finalized = build_finalized_checkpoint_from_committee(
            &candidate,
            &checkpoint_hash,
            &set,
            &participants,
            &kms,
            &signatures,
        )
        .expect("valid threshold evidence must produce artifact");

        let value = serde_json::to_value(&finalized).expect("finality artifact must serialize");

        let object = value
            .as_object()
            .expect("finality artifact must serialize as object");

        for forbidden in [
            "finalized_by_process",
            "finalized_by_node",
            "finalized_by_http",
            "crablink_finality",
            "wallet_mutation",
            "ledger_mutation",
            "payout_executed",
            "receipt_created",
            "bridge_settled",
        ] {
            assert!(
                !object.contains_key(
                    forbidden,
                ),
                "finality artifact must not contain process/client/economic authority field {forbidden}",
            );
        }
    }
}
