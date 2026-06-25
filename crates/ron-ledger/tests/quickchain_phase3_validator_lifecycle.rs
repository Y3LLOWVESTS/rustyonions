#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 3 Round 2 tests for read-only validator lifecycle hardening evaluation.
//! RO:WHY — ECON/GOV: ron-ledger can inspect rotation, revocation, and equivocation evidence without granting balance/staking/slashing authority.
//! RO:INTERACTS — ron-ledger validator_lifecycle and ron-proto Phase 3 lifecycle DTOs.
//! RO:INVARIANTS — deterministic inputs only; no wallet mutation; no staking/slashing; no finality/settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — fixture evidence is inert and grants no spend, bridge, paid unlock, slashing, staking, or finality authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    evaluate_validator_equivocation_evidence_read_only, evaluate_validator_revocation_read_only,
    evaluate_validator_rotation_read_only,
};
use ron_proto::{
    quantum::SignatureAlg,
    quickchain::{
        QuickChainValidatorEquivocationEvidenceV1, QuickChainValidatorIdentityV1,
        QuickChainValidatorLifecycleDecisionStatusV1, QuickChainValidatorLifecycleOperationV1,
        QuickChainValidatorLifecycleRejectionCodeV1, QuickChainValidatorLifecycleStatusV1,
        QuickChainValidatorRevocationReasonV1, QuickChainValidatorRevocationV1,
        QuickChainValidatorRotationV1, QuickChainValidatorSetV1, QuickChainVerifierReplayStatusV1,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VALIDATOR_EQUIVOCATION_EVIDENCE_SCHEMA,
        QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA, QUICKCHAIN_VALIDATOR_REVOCATION_SCHEMA,
        QUICKCHAIN_VALIDATOR_ROTATION_SCHEMA,
        QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1, QUICKCHAIN_VALIDATOR_SET_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0005";
const VALIDATOR_ID: &str = "validator-alpha";
const PASSPORT_SUBJECT: &str = "@validator-alpha";
const REGISTRY_ENTRY_ID: &str = "registry:validator-alpha";
const OLD_KEY_ID: &str = "key:validator-alpha:001";
const NEW_KEY_ID: &str = "key:validator-alpha:002";
const OLD_CAPABILITY_ID: &str = "cap:validator-alpha:verify:001";
const NEW_CAPABILITY_ID: &str = "cap:validator-alpha:verify:002";
const NOT_BEFORE_MS: u64 = 1_800_000_000_000;
const EXPIRES_AT_MS: u64 = 1_800_172_800_000;
const EVENT_MS: u64 = 1_800_086_460_000;

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn identity(status: QuickChainValidatorLifecycleStatusV1) -> QuickChainValidatorIdentityV1 {
    QuickChainValidatorIdentityV1 {
        schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_id: VALIDATOR_ID.to_string(),
        passport_subject: PASSPORT_SUBJECT.to_string(),
        registry_entry_id: REGISTRY_ENTRY_ID.to_string(),
        key_id: OLD_KEY_ID.to_string(),
        capability_id: OLD_CAPABILITY_ID.to_string(),
        signature_algorithm: SignatureAlg::Ed25519,
        lifecycle_status: status,
        not_before_ms: NOT_BEFORE_MS,
        expires_at_ms: EXPIRES_AT_MS,
    }
}

fn validator_set(status: QuickChainValidatorLifecycleStatusV1) -> QuickChainValidatorSetV1 {
    QuickChainValidatorSetV1 {
        schema: QUICKCHAIN_VALIDATOR_SET_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set-r2"),
        policy_hash: cid("policy-r2"),
        registry_snapshot_hash: cid("registry-r2"),
        passport_required: true,
        bond_required: false,
        validator_set_algorithm: QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1
            .to_string(),
        members: vec![identity(status)],
    }
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
fn active_validator_rotation_is_accepted_read_only() {
    let set = validator_set(QuickChainValidatorLifecycleStatusV1::Active);
    let decision = evaluate_validator_rotation_read_only(&set, &rotation()).unwrap();

    assert_eq!(
        decision.status,
        QuickChainValidatorLifecycleDecisionStatusV1::Accepted
    );
    assert_eq!(
        decision.operation,
        QuickChainValidatorLifecycleOperationV1::RotateValidator
    );
    assert_eq!(decision.rejection_code, None);
    decision.validate().unwrap();
}

#[test]
fn active_validator_revocation_is_accepted_read_only_without_slashing() {
    let set = validator_set(QuickChainValidatorLifecycleStatusV1::Active);
    let decision = evaluate_validator_revocation_read_only(&set, &revocation()).unwrap();

    assert_eq!(
        decision.status,
        QuickChainValidatorLifecycleDecisionStatusV1::Accepted
    );
    assert_eq!(
        decision.operation,
        QuickChainValidatorLifecycleOperationV1::RevokeValidator
    );
    assert_eq!(decision.rejection_code, None);
    decision.validate().unwrap();
}

#[test]
fn equivocation_evidence_is_accepted_as_evidence_only() {
    let set = validator_set(QuickChainValidatorLifecycleStatusV1::Active);
    let decision =
        evaluate_validator_equivocation_evidence_read_only(&set, &equivocation()).unwrap();

    assert_eq!(
        decision.status,
        QuickChainValidatorLifecycleDecisionStatusV1::Accepted
    );
    assert_eq!(
        decision.operation,
        QuickChainValidatorLifecycleOperationV1::RecordEquivocationEvidence
    );
    assert_eq!(decision.rejection_code, None);
    decision.validate().unwrap();
}

#[test]
fn rotation_rejects_key_or_capability_mismatch_without_mutation() {
    let set = validator_set(QuickChainValidatorLifecycleStatusV1::Active);
    let mut rotation = rotation();
    rotation.old_key_id = "key:validator-alpha:wrong".to_string();

    let decision = evaluate_validator_rotation_read_only(&set, &rotation).unwrap();

    assert_eq!(
        decision.status,
        QuickChainValidatorLifecycleDecisionStatusV1::Rejected
    );
    assert_eq!(
        decision.rejection_code,
        Some(QuickChainValidatorLifecycleRejectionCodeV1::KeyMismatch)
    );
}

#[test]
fn revoked_validator_cannot_rotate_or_submit_equivocation_as_authorized_member() {
    let set = validator_set(QuickChainValidatorLifecycleStatusV1::Revoked);

    let rotation_decision = evaluate_validator_rotation_read_only(&set, &rotation()).unwrap();
    assert_eq!(
        rotation_decision.rejection_code,
        Some(QuickChainValidatorLifecycleRejectionCodeV1::RevokedValidator)
    );

    let evidence_decision =
        evaluate_validator_equivocation_evidence_read_only(&set, &equivocation()).unwrap();
    assert_eq!(
        evidence_decision.rejection_code,
        Some(QuickChainValidatorLifecycleRejectionCodeV1::RevokedValidator)
    );
}

#[test]
fn validator_set_hash_mismatch_rejects_lifecycle_artifacts() {
    let set = validator_set(QuickChainValidatorLifecycleStatusV1::Active);
    let mut revocation = revocation();
    revocation.validator_set_hash = cid("different-validator-set");

    let decision = evaluate_validator_revocation_read_only(&set, &revocation).unwrap();

    assert_eq!(
        decision.status,
        QuickChainValidatorLifecycleDecisionStatusV1::Rejected
    );
    assert_eq!(
        decision.rejection_code,
        Some(QuickChainValidatorLifecycleRejectionCodeV1::ValidatorSetHashMismatch)
    );
}

#[test]
fn lifecycle_hardening_source_stays_out_of_staking_slashing_and_balance_authority() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/quickchain/validator_lifecycle.rs"),
    )
    .expect("validator_lifecycle.rs should be readable");

    for forbidden in [
        "stake_validator(",
        "slash_validator(",
        "staking_power:",
        "slash_amount_minor",
        "wallet_mutation",
        "balance_mutation",
        "settlement_authority:true",
        "bridge_authority:true",
    ] {
        assert!(
            !source.contains(forbidden),
            "validator lifecycle evaluator must not contain forbidden authority marker: {forbidden}"
        );
    }
}
