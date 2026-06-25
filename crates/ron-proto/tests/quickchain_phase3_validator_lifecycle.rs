//! RO:WHAT — Phase 3 Round 2 DTO tests for validator lifecycle hardening artifacts.
//! RO:WHY — ECON/GOV: lifecycle artifacts must cover rotation, revocation, downtime, evidence, and governance updates without enabling staking/slashing.
//! RO:INTERACTS — ron-proto quickchain validator_lifecycle DTOs and Phase 3 validator-set DTOs.
//! RO:INVARIANTS — unknown fields reject; lifecycle evidence is inert; governance approval is explicit; no live validator economy.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture hashes and references are test artifacts and grant no spend, bridge, slashing, staking, finality, or settlement authority.
//! RO:TEST — this file.

use ron_proto::{
    quickchain::{
        QuickChainReplayChallengeEvidenceV1, QuickChainReplayChallengeKindV1,
        QuickChainValidatorDowntimeReportV1, QuickChainValidatorDowntimeStatusV1,
        QuickChainValidatorEquivocationEvidenceV1, QuickChainValidatorGovernanceParameterV1,
        QuickChainValidatorLifecycleDecisionStatusV1, QuickChainValidatorLifecycleDecisionV1,
        QuickChainValidatorLifecycleOperationV1, QuickChainValidatorLifecycleRejectionCodeV1,
        QuickChainValidatorParameterUpdateV1, QuickChainValidatorRevocationReasonV1,
        QuickChainValidatorRevocationV1, QuickChainValidatorRotationV1,
        QuickChainVerifierReplayStatusV1, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_REPLAY_CHALLENGE_EVIDENCE_SCHEMA, QUICKCHAIN_VALIDATOR_DOWNTIME_REPORT_SCHEMA,
        QUICKCHAIN_VALIDATOR_EQUIVOCATION_EVIDENCE_SCHEMA,
        QUICKCHAIN_VALIDATOR_LIFECYCLE_DECISION_SCHEMA,
        QUICKCHAIN_VALIDATOR_PARAMETER_UPDATE_SCHEMA, QUICKCHAIN_VALIDATOR_REVOCATION_SCHEMA,
        QUICKCHAIN_VALIDATOR_ROTATION_SCHEMA,
    },
    ContentId,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0005";
const EFFECTIVE_EPOCH_ID: &str = "epoch_0006";
const VALIDATOR_ID: &str = "validator-alpha";
const PASSPORT_SUBJECT: &str = "@validator-alpha";
const OLD_KEY_ID: &str = "key:validator-alpha:001";
const NEW_KEY_ID: &str = "key:validator-alpha:002";
const OLD_CAPABILITY_ID: &str = "cap:validator-alpha:verify:001";
const NEW_CAPABILITY_ID: &str = "cap:validator-alpha:verify:002";
const EVENT_MS: u64 = 1_800_086_460_000;

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn rotation() -> QuickChainValidatorRotationV1 {
    QuickChainValidatorRotationV1 {
        schema: QUICKCHAIN_VALIDATOR_ROTATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        validator_id: VALIDATOR_ID.to_string(),
        passport_subject: PASSPORT_SUBJECT.to_string(),
        old_key_id: OLD_KEY_ID.to_string(),
        new_key_id: NEW_KEY_ID.to_string(),
        old_capability_id: OLD_CAPABILITY_ID.to_string(),
        new_capability_id: NEW_CAPABILITY_ID.to_string(),
        effective_at_ms: EVENT_MS,
        governance_approval_ref: "governance:validator-rotation:001".to_string(),
    }
}

fn revocation() -> QuickChainValidatorRevocationV1 {
    QuickChainValidatorRevocationV1 {
        schema: QUICKCHAIN_VALIDATOR_REVOCATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        validator_id: VALIDATOR_ID.to_string(),
        passport_subject: PASSPORT_SUBJECT.to_string(),
        key_id: OLD_KEY_ID.to_string(),
        capability_id: OLD_CAPABILITY_ID.to_string(),
        reason: QuickChainValidatorRevocationReasonV1::GovernanceAction,
        revoked_at_ms: EVENT_MS,
        governance_approval_ref: "governance:validator-revocation:001".to_string(),
    }
}

fn equivocation() -> QuickChainValidatorEquivocationEvidenceV1 {
    QuickChainValidatorEquivocationEvidenceV1 {
        schema: QUICKCHAIN_VALIDATOR_EQUIVOCATION_EVIDENCE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        validator_id: VALIDATOR_ID.to_string(),
        key_id: OLD_KEY_ID.to_string(),
        first_attestation_hash: cid("attestation-one"),
        second_attestation_hash: cid("attestation-two"),
        first_replay_result_hash: cid("replay-result-one"),
        second_replay_result_hash: cid("replay-result-two"),
        first_replay_status: QuickChainVerifierReplayStatusV1::Verified,
        second_replay_status: QuickChainVerifierReplayStatusV1::Mismatch,
        evidence_ref: "evidence:equivocation:001".to_string(),
        observed_at_ms: EVENT_MS,
    }
}

#[test]
fn rotation_revocation_downtime_challenge_and_parameter_update_shapes_validate() {
    rotation().validate().unwrap();
    revocation().validate().unwrap();

    QuickChainValidatorDowntimeReportV1 {
        schema: QUICKCHAIN_VALIDATOR_DOWNTIME_REPORT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        validator_id: VALIDATOR_ID.to_string(),
        downtime_status: QuickChainValidatorDowntimeStatusV1::Degraded,
        observed_at_ms: EVENT_MS,
        evidence_ref: "evidence:downtime:001".to_string(),
    }
    .validate()
    .unwrap();

    QuickChainReplayChallengeEvidenceV1 {
        schema: QUICKCHAIN_REPLAY_CHALLENGE_EVIDENCE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        challenge_kind: QuickChainReplayChallengeKindV1::DoubleAttestation,
        replay_bundle_hash: cid("replay-bundle"),
        disputed_replay_result_hash: cid("disputed-replay-result"),
        challenger_ref: "passport:@challenger".to_string(),
        evidence_ref: "evidence:replay-challenge:001".to_string(),
        submitted_at_ms: EVENT_MS,
    }
    .validate()
    .unwrap();

    QuickChainValidatorParameterUpdateV1 {
        schema: QUICKCHAIN_VALIDATOR_PARAMETER_UPDATE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        parameter: QuickChainValidatorGovernanceParameterV1::ChallengeWindowMs,
        previous_value_ref: "param:challenge-window-ms:86400000".to_string(),
        new_value_ref: "param:challenge-window-ms:172800000".to_string(),
        effective_epoch_id: EFFECTIVE_EPOCH_ID.to_string(),
        governance_approval_ref: "governance:parameter-update:001".to_string(),
    }
    .validate()
    .unwrap();
}

#[test]
fn rotation_requires_key_and_capability_change() {
    let mut same_key = rotation();
    same_key.new_key_id = same_key.old_key_id.clone();

    let error = same_key.validate().unwrap_err();
    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "new_key_id");
        }
        other => panic!("expected new_key_id InvalidField, got {other:?}"),
    }

    let mut same_capability = rotation();
    same_capability.new_capability_id = same_capability.old_capability_id.clone();

    let error = same_capability.validate().unwrap_err();
    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "new_capability_id");
        }
        other => panic!("expected new_capability_id InvalidField, got {other:?}"),
    }
}

#[test]
fn equivocation_evidence_requires_distinct_and_conflicting_attestations() {
    equivocation().validate().unwrap();

    let mut same_attestation = equivocation();
    same_attestation.second_attestation_hash = same_attestation.first_attestation_hash.clone();
    assert!(same_attestation.validate().is_err());

    let mut not_conflicting = equivocation();
    not_conflicting.second_replay_result_hash = not_conflicting.first_replay_result_hash.clone();
    not_conflicting.second_replay_status = not_conflicting.first_replay_status;
    assert!(not_conflicting.validate().is_err());
}

#[test]
fn lifecycle_decision_status_consistency_is_strict() {
    let accepted = QuickChainValidatorLifecycleDecisionV1 {
        schema: QUICKCHAIN_VALIDATOR_LIFECYCLE_DECISION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        validator_id: VALIDATOR_ID.to_string(),
        operation: QuickChainValidatorLifecycleOperationV1::RotateValidator,
        status: QuickChainValidatorLifecycleDecisionStatusV1::Accepted,
        rejection_code: None,
    };

    accepted.validate().unwrap();

    let mut rejected = accepted.clone();
    rejected.status = QuickChainValidatorLifecycleDecisionStatusV1::Rejected;
    rejected.rejection_code = Some(QuickChainValidatorLifecycleRejectionCodeV1::KeyMismatch);
    rejected.validate().unwrap();

    let mut invalid = accepted;
    invalid.rejection_code = Some(QuickChainValidatorLifecycleRejectionCodeV1::KeyMismatch);
    assert!(invalid.validate().is_err());
}

#[test]
fn phase3_round2_lifecycle_dtos_reject_unknown_economics_and_slashing_fields() {
    let mut rotation = serde_json::to_value(rotation()).unwrap();
    rotation
        .as_object_mut()
        .unwrap()
        .insert("staking_power".to_string(), json!("1000000"));
    assert!(serde_json::from_value::<QuickChainValidatorRotationV1>(rotation).is_err());

    let mut revocation = serde_json::to_value(revocation()).unwrap();
    revocation
        .as_object_mut()
        .unwrap()
        .insert("slash_amount_minor".to_string(), json!("500"));
    assert!(serde_json::from_value::<QuickChainValidatorRevocationV1>(revocation).is_err());

    let mut evidence = serde_json::to_value(equivocation()).unwrap();
    evidence
        .as_object_mut()
        .unwrap()
        .insert("automatic_slash".to_string(), json!(true));
    assert!(serde_json::from_value::<QuickChainValidatorEquivocationEvidenceV1>(evidence).is_err());
}
