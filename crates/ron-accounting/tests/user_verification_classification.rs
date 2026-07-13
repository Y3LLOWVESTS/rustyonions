//! RO:WHAT — Phase 14 deterministic User Node classification tests.
//! RO:WHY — Prove valid verification work is separated from challenge findings.

use ron_accounting::{
    classify_user_verification, classify_user_verification_batch, Error,
    UserVerificationAccountingClassV1,
};
use ron_accounting::{
    EvidenceContentId, UserVerificationAccountingInputV1, UserVerificationEvidenceKindV1,
    UserVerificationFailureReasonV1, UserVerificationResultV1,
    USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA, USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
};

const DIGEST: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn input(sequence: u64, kind: UserVerificationEvidenceKindV1) -> UserVerificationAccountingInputV1 {
    UserVerificationAccountingInputV1 {
        schema: USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA.to_owned(),
        version: USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
        sequence,
        evidence_id: format!("verification:evidence:{sequence:04}"),
        user_node_id: "user_node:alpha".to_owned(),
        subject_ref: "receipt:subject:0001".to_owned(),
        verification_kind: kind,
        observed_at_ms: 1_900_000_000_000 + sequence,
        input_digest: DIGEST
            .parse::<EvidenceContentId>()
            .expect("canonical input digest"),
        result: UserVerificationResultV1::VerifiedValid,
        failure_reason: None,
        nonce: format!("nonce:verification:{sequence:04}"),
        idempotency_key: format!("idempotency:verification:{sequence:04}"),
        capability_id: "capability:verification:0001".to_owned(),
        privacy_route_id: Some("privacy_route:0001".to_owned()),
        attestation_ref: format!("attestation:user_node:{sequence:04}"),
        local_attestation_verified: true,
        evidence_only: true,
        accounting_accepted: false,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

#[test]
fn verified_valid_work_becomes_capped_planning_candidate() {
    for kind in [
        UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
        UserVerificationEvidenceKindV1::LedgerReceiptReplaySample,
        UserVerificationEvidenceKindV1::EpochTransitionReplaySample,
        UserVerificationEvidenceKindV1::RewardPlanVerification,
        UserVerificationEvidenceKindV1::RewardPoolCapVerification,
        UserVerificationEvidenceKindV1::DeliveryReceiptChallengeReview,
        UserVerificationEvidenceKindV1::B3IntegrityChallenge,
        UserVerificationEvidenceKindV1::AvailabilityChallengeReview,
        UserVerificationEvidenceKindV1::ReplayRejectionProof,
        UserVerificationEvidenceKindV1::DuplicateEvidenceRejection,
        UserVerificationEvidenceKindV1::InvalidEpochChallenge,
    ] {
        let decision =
            classify_user_verification(&input(1, kind)).expect("classified verification");

        assert_eq!(
            decision.class,
            UserVerificationAccountingClassV1::ProofEligibleVerification
        );

        assert!(decision.verified_evidence);
        assert!(decision.requires_policy_review);
        assert!(decision.requires_economics_config);
        assert!(decision.requires_cap);
        assert!(!decision.requires_challenge_acceptance);
        assert!(decision.reward_planning_candidate);

        assert!(!decision.accounting_snapshot_member);
        assert!(!decision.reward_plan_created);
        assert!(!decision.direct_protocol_roc_allocation);
        assert!(!decision.reward_truth);
        assert!(!decision.payout_authority);
        assert!(!decision.wallet_mutation);
        assert!(!decision.ledger_mutation);
    }
}

#[test]
fn invalid_and_challenge_findings_require_acceptance() {
    for result in [
        UserVerificationResultV1::VerifiedInvalid,
        UserVerificationResultV1::ChallengeRaised,
    ] {
        let mut input = input(1, UserVerificationEvidenceKindV1::InvalidEpochChallenge);

        input.result = result;
        input.failure_reason = Some(UserVerificationFailureReasonV1::SupplyMismatch);

        let decision = classify_user_verification(&input).expect("classified challenge evidence");

        assert_eq!(
            decision.class,
            UserVerificationAccountingClassV1::ChallengeEvidenceOnly
        );

        assert!(decision.verified_evidence);
        assert!(decision.requires_policy_review);
        assert!(decision.requires_challenge_acceptance);

        assert!(!decision.requires_economics_config);
        assert!(!decision.requires_cap);
        assert!(!decision.reward_planning_candidate);
        assert!(!decision.reward_plan_created);
        assert!(!decision.reward_truth);
        assert!(!decision.payout_authority);
    }
}

#[test]
fn batch_output_is_independent_of_input_order() {
    let first = input(
        1,
        UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
    );

    let mut second = input(2, UserVerificationEvidenceKindV1::InvalidEpochChallenge);
    second.result = UserVerificationResultV1::ChallengeRaised;
    second.failure_reason = Some(UserVerificationFailureReasonV1::PolicyMismatch);

    let ordered =
        classify_user_verification_batch(&[first.clone(), second.clone()]).expect("ordered batch");

    let reversed = classify_user_verification_batch(&[second, first]).expect("reversed batch");

    assert_eq!(ordered, reversed);
    assert_eq!(ordered.input_count, 2);
    assert_eq!(ordered.proof_eligible_count, 1);
    assert_eq!(ordered.challenge_evidence_count, 1);
    assert!(ordered.deterministic_order);

    assert!(!ordered.accounting_snapshot_created);
    assert!(!ordered.reward_plan_created);
    assert!(!ordered.payout_authority);
    assert!(!ordered.wallet_mutation);
    assert!(!ordered.ledger_mutation);
}

#[test]
fn duplicate_sequence_rejects_batch() {
    let first = input(
        1,
        UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
    );

    let mut second = input(1, UserVerificationEvidenceKindV1::RewardPlanVerification);
    second.evidence_id = "verification:evidence:different".to_owned();

    let error = classify_user_verification_batch(&[first, second])
        .expect_err("duplicate sequence must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));

    assert!(error
        .to_string()
        .contains("duplicate user verification sequence"));
}

#[test]
fn duplicate_evidence_identity_rejects_batch() {
    let first = input(1, UserVerificationEvidenceKindV1::LedgerReceiptReplaySample);

    let mut second = input(2, UserVerificationEvidenceKindV1::LedgerReceiptReplaySample);
    second.evidence_id = first.evidence_id.clone();

    let error = classify_user_verification_batch(&[first, second])
        .expect_err("duplicate evidence identity must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));

    assert!(error
        .to_string()
        .contains("duplicate user verification evidence identity"));
}

#[test]
fn unattested_or_authority_poisoned_input_rejects() {
    let mut unattested = input(1, UserVerificationEvidenceKindV1::RewardPoolCapVerification);
    unattested.local_attestation_verified = false;

    assert!(classify_user_verification(&unattested)
        .expect_err("unattested input must reject")
        .to_string()
        .contains("verified attestation"));

    let mut poisoned = input(2, UserVerificationEvidenceKindV1::RewardPoolCapVerification);
    poisoned.reward_truth = true;

    assert!(classify_user_verification(&poisoned)
        .expect_err("reward truth must reject")
        .to_string()
        .contains("reward_truth"));
}

#[test]
fn classification_has_no_amount_balance_receipt_or_ip_truth() {
    let batch = classify_user_verification_batch(&[
        input(
            1,
            UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
        ),
        input(2, UserVerificationEvidenceKindV1::ReplayRejectionProof),
    ])
    .expect("classification batch");

    let json = serde_json::to_string(&batch).expect("classification JSON");

    for forbidden in [
        "reward_amount",
        "amount_minor",
        "balance",
        "receipt_created",
        "payout_account",
        "confirmed_roc",
        "source_ip",
        "viewer_ip",
        "socket_addr",
        "raw_engagement",
    ] {
        assert!(
            !json.contains(forbidden),
            "classification must not contain {forbidden}"
        );
    }

    assert!(json.contains(r#""reward_plan_created":false"#));
    assert!(json.contains(r#""wallet_mutation":false"#));
    assert!(json.contains(r#""ledger_mutation":false"#));
}
