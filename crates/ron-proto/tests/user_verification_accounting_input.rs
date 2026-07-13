//! RO:WHAT — Phase 14 User Node verification accounting-input tests.
//! RO:WHY — Lock result, attestation, privacy, and non-authority boundaries.

use ron_proto::{
    ContentId, UserVerificationAccountingInputV1, UserVerificationAccountingInputValidationError,
    UserVerificationEvidenceKindV1, UserVerificationFailureReasonV1, UserVerificationResultV1,
    USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA, USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
};
use serde_json::json;

const DIGEST: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn input(kind: UserVerificationEvidenceKindV1) -> UserVerificationAccountingInputV1 {
    UserVerificationAccountingInputV1 {
        schema: USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA.to_owned(),
        version: USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
        sequence: 1,
        evidence_id: "verification:evidence:0001".to_owned(),
        user_node_id: "user_node:alpha".to_owned(),
        subject_ref: "receipt:subject:0001".to_owned(),
        verification_kind: kind,
        observed_at_ms: 1_900_000_000_000,
        input_digest: DIGEST.parse::<ContentId>().expect("canonical input digest"),
        result: UserVerificationResultV1::VerifiedValid,
        failure_reason: None,
        nonce: "nonce:verification:0001".to_owned(),
        idempotency_key: "idempotency:verification:0001".to_owned(),
        capability_id: "capability:verification:0001".to_owned(),
        privacy_route_id: Some("privacy_route:0001".to_owned()),
        attestation_ref: "attestation:user_node:0001".to_owned(),
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
fn all_verification_kinds_validate_and_roundtrip() {
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
        let input = input(kind);
        input.validate().expect("valid input");

        let encoded = serde_json::to_value(&input).expect("verification input JSON");

        let decoded: UserVerificationAccountingInputV1 =
            serde_json::from_value(encoded).expect("verification input decode");

        assert_eq!(decoded, input);
    }
}

#[test]
fn verified_valid_forbids_failure_reason() {
    let mut input = input(UserVerificationEvidenceKindV1::MicrotxReceiptVerification);

    input.failure_reason = Some(UserVerificationFailureReasonV1::DigestMismatch);

    assert_eq!(
        input.validate(),
        Err(UserVerificationAccountingInputValidationError::UnexpectedFailureReason)
    );
}

#[test]
fn invalid_and_challenge_results_require_reason() {
    for result in [
        UserVerificationResultV1::VerifiedInvalid,
        UserVerificationResultV1::ChallengeRaised,
    ] {
        let mut missing = input(UserVerificationEvidenceKindV1::LedgerReceiptReplaySample);
        missing.result = result;

        assert_eq!(
            missing.validate(),
            Err(UserVerificationAccountingInputValidationError::MissingFailureReason)
        );

        missing.failure_reason = Some(UserVerificationFailureReasonV1::ReplayDetected);

        missing.validate().expect("reason-bound invalid result");
    }
}

#[test]
fn attestation_and_evidence_only_are_required() {
    let mut unsigned = input(UserVerificationEvidenceKindV1::RewardPlanVerification);
    unsigned.local_attestation_verified = false;

    assert_eq!(
        unsigned.validate(),
        Err(UserVerificationAccountingInputValidationError::AttestationNotVerified)
    );

    let mut authoritative = input(UserVerificationEvidenceKindV1::RewardPlanVerification);
    authoritative.evidence_only = false;

    assert_eq!(
        authoritative.validate(),
        Err(UserVerificationAccountingInputValidationError::NotEvidenceOnly)
    );
}

#[test]
fn economic_authority_flags_reject() {
    for field in [
        "accounting_accepted",
        "reward_eligible",
        "reward_truth",
        "payout_authority",
        "wallet_mutation",
        "ledger_mutation",
    ] {
        let mut value = serde_json::to_value(input(
            UserVerificationEvidenceKindV1::RewardPoolCapVerification,
        ))
        .expect("verification JSON");

        value
            .as_object_mut()
            .expect("verification object")
            .insert(field.to_owned(), json!(true));

        let poisoned: UserVerificationAccountingInputV1 =
            serde_json::from_value(value).expect("known field must decode");

        assert_eq!(
            poisoned.validate(),
            Err(UserVerificationAccountingInputValidationError::AuthorityBoundary { field })
        );
    }
}

#[test]
fn sequence_timestamp_and_tokens_are_strict() {
    let mut zero_sequence = input(UserVerificationEvidenceKindV1::ReplayRejectionProof);
    zero_sequence.sequence = 0;

    assert_eq!(
        zero_sequence.validate(),
        Err(UserVerificationAccountingInputValidationError::ZeroValue { field: "sequence" })
    );

    let mut zero_timestamp = input(UserVerificationEvidenceKindV1::ReplayRejectionProof);
    zero_timestamp.observed_at_ms = 0;

    assert_eq!(
        zero_timestamp.validate(),
        Err(UserVerificationAccountingInputValidationError::ZeroValue {
            field: "observed_at_ms",
        })
    );

    for invalid_value in [
        "192.0.2.10",
        "127.0.0.1:5301",
        "2001:db8::1",
        "tcp://192.0.2.10:5301",
        ":missing_namespace",
        "missing_value:",
    ] {
        let mut invalid_route = input(UserVerificationEvidenceKindV1::ReplayRejectionProof);

        invalid_route.privacy_route_id = Some(invalid_value.to_owned());

        assert_eq!(
            invalid_route.validate(),
            Err(
                UserVerificationAccountingInputValidationError::InvalidToken {
                    field: "privacy_route_id",
                }
            ),
            "transport-shaped privacy route must reject: {invalid_value}"
        );
    }

    for valid_value in ["privacy_route:0001", "route_0001", "relay-token-0001"] {
        let mut valid_route = input(UserVerificationEvidenceKindV1::ReplayRejectionProof);

        valid_route.privacy_route_id = Some(valid_value.to_owned());

        valid_route
            .validate()
            .expect("opaque privacy route must validate");
    }
}

#[test]
fn wire_rejects_ip_raw_engagement_amount_and_payout() {
    for (field, value) in [
        ("source_ip", json!("192.0.2.10")),
        ("viewer_ip", json!("198.51.100.10")),
        ("raw_engagement_count", json!(1)),
        ("reward_amount_minor", json!("100")),
        ("payout_account", json!("account:attacker")),
        ("receipt_created", json!(true)),
    ] {
        let mut input = serde_json::to_value(input(
            UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
        ))
        .expect("verification JSON");

        input
            .as_object_mut()
            .expect("verification object")
            .insert(field.to_owned(), value);

        assert!(
            serde_json::from_value::<UserVerificationAccountingInputV1>(input).is_err(),
            "unknown field must reject: {field}"
        );
    }
}
