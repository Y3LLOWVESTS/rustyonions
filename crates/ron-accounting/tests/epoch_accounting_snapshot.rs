//! RO:WHAT — Phase 14 deterministic epoch accounting snapshot tests.
//! RO:WHY — Prove canonical epoch commitments remain non-authoritative.

use ron_accounting::{
    build_accounting_epoch_snapshot, canonical_accounting_epoch_snapshot_artifact_cid,
    classify_service_evidence_batch, classify_user_verification_batch,
    AccountingEconomicsConfigBindingV1, AccountingEconomicsProfileV1, AccountingEpochSnapshotV1,
    AccountingEpochWindowV1, Error,
};
use ron_accounting::{
    EvidenceContentId, ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingKindV1,
    UserVerificationAccountingInputV1, UserVerificationEvidenceKindV1,
    UserVerificationFailureReasonV1, UserVerificationResultV1,
    SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA, SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
    USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA, USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ECONOMICS_SCHEMA: &str = "internal_roc.economics-config.v1";
const ECONOMICS_HASH_A: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ECONOMICS_HASH_B: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn economics_binding(hash: &str) -> AccountingEconomicsConfigBindingV1 {
    AccountingEconomicsConfigBindingV1::from_validated_model(
        ECONOMICS_SCHEMA,
        1,
        AccountingEconomicsProfileV1::Canonical,
        hash,
    )
    .expect("canonical normalized economics binding")
}

fn content_id() -> EvidenceContentId {
    CID.parse().expect("canonical content ID")
}

fn window() -> AccountingEpochWindowV1 {
    AccountingEpochWindowV1 {
        epoch_id: "epoch:0001".to_owned(),
        starts_at_ms: 1_000,
        ends_at_ms: 2_000,
        produced_at_ms: 2_100,
    }
}

fn service_input(
    sequence: u64,
    kind: ServiceEvidenceAccountingKindV1,
    observed_at_ms: u64,
) -> ServiceEvidenceAccountingInputV1 {
    ServiceEvidenceAccountingInputV1 {
        schema: SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
        sequence,
        kind,
        proof_id: format!("service:evidence:{sequence:04}"),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        related_actor_ids: Vec::new(),
        content_id: content_id(),
        observed_at_ms,
        signature_verified: true,
        evidence_only: true,
        accounting_accepted: false,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn user_input(
    sequence: u64,
    kind: UserVerificationEvidenceKindV1,
    observed_at_ms: u64,
) -> UserVerificationAccountingInputV1 {
    UserVerificationAccountingInputV1 {
        schema: USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA.to_owned(),
        version: USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
        sequence,
        evidence_id: format!("user:verification:{sequence:04}"),
        user_node_id: "user_node:charlie".to_owned(),
        subject_ref: "receipt:subject:0001".to_owned(),
        verification_kind: kind,
        observed_at_ms,
        input_digest: content_id(),
        result: UserVerificationResultV1::VerifiedValid,
        failure_reason: None,
        nonce: format!("nonce:user:{sequence:04}"),
        idempotency_key: format!("idempotency:user:{sequence:04}"),
        capability_id: "capability:verification:0001".to_owned(),
        privacy_route_id: Some("privacy_route:0001".to_owned()),
        attestation_ref: format!("attestation:user:{sequence:04}"),
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

fn mixed_snapshot_with_hash(economics_config_hash: &str) -> AccountingEpochSnapshotV1 {
    let service_batch = classify_service_evidence_batch(&[
        service_input(2, ServiceEvidenceAccountingKindV1::PolicyRefusal, 1_200),
        service_input(1, ServiceEvidenceAccountingKindV1::Delivery, 1_100),
    ])
    .expect("service classification");

    let mut challenge = user_input(
        2,
        UserVerificationEvidenceKindV1::InvalidEpochChallenge,
        1_400,
    );

    challenge.result = UserVerificationResultV1::ChallengeRaised;

    challenge.failure_reason = Some(UserVerificationFailureReasonV1::SupplyMismatch);

    let user_batch = classify_user_verification_batch(&[
        challenge,
        user_input(
            1,
            UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
            1_300,
        ),
    ])
    .expect("user classification");

    build_accounting_epoch_snapshot(
        window(),
        &economics_binding(economics_config_hash),
        Some(&service_batch),
        Some(&user_batch),
    )
    .expect("epoch snapshot")
}

fn mixed_snapshot() -> AccountingEpochSnapshotV1 {
    mixed_snapshot_with_hash(ECONOMICS_HASH_A)
}

#[test]
fn snapshot_filters_policy_and_challenge_evidence() {
    let snapshot = mixed_snapshot();

    assert_eq!(snapshot.source_service_input_count, 2);
    assert_eq!(snapshot.source_user_verification_input_count, 2);

    assert_eq!(snapshot.eligible_service_count, 1);
    assert_eq!(snapshot.eligible_user_verification_count, 1);

    assert_eq!(snapshot.policy_evidence_count, 1);
    assert_eq!(snapshot.challenge_evidence_count, 1);

    assert_eq!(snapshot.service_rows.len(), 1);
    assert_eq!(snapshot.user_verification_rows.len(), 1);

    assert!(snapshot.deterministic_order);
    assert!(snapshot.snapshot_artifact_created);

    assert_eq!(snapshot.economics_config.config_schema, ECONOMICS_SCHEMA);
    assert_eq!(snapshot.economics_config.config_version, 1);
    assert_eq!(
        snapshot.economics_config.economics_config_hash.as_str(),
        ECONOMICS_HASH_A
    );
    assert_eq!(
        snapshot.economics_config.profile,
        AccountingEconomicsProfileV1::Canonical
    );

    assert!(!snapshot.consensus_root_claimed);
    assert!(!snapshot.reward_plan_created);
    assert!(!snapshot.payout_authority);
    assert!(!snapshot.wallet_mutation);
    assert!(!snapshot.ledger_mutation);
}

#[test]
fn snapshot_and_artifact_cid_are_replay_deterministic() {
    let first = mixed_snapshot();
    let second = mixed_snapshot();

    assert_eq!(first, second);

    let first_cid =
        canonical_accounting_epoch_snapshot_artifact_cid(&first).expect("first artifact CID");

    let second_cid =
        canonical_accounting_epoch_snapshot_artifact_cid(&second).expect("second artifact CID");

    assert_eq!(first_cid, second_cid);
    assert!(first_cid.starts_with("b3:"));
    assert_eq!(first_cid.len(), 67);
}

#[test]
fn classification_input_order_does_not_change_snapshot() {
    let first_service = classify_service_evidence_batch(&[
        service_input(2, ServiceEvidenceAccountingKindV1::Availability, 1_200),
        service_input(1, ServiceEvidenceAccountingKindV1::Delivery, 1_100),
    ])
    .expect("first service batch");

    let second_service = classify_service_evidence_batch(&[
        service_input(1, ServiceEvidenceAccountingKindV1::Delivery, 1_100),
        service_input(2, ServiceEvidenceAccountingKindV1::Availability, 1_200),
    ])
    .expect("second service batch");

    let first_user = classify_user_verification_batch(&[
        user_input(
            2,
            UserVerificationEvidenceKindV1::RewardPlanVerification,
            1_400,
        ),
        user_input(
            1,
            UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
            1_300,
        ),
    ])
    .expect("first user batch");

    let second_user = classify_user_verification_batch(&[
        user_input(
            1,
            UserVerificationEvidenceKindV1::MicrotxReceiptVerification,
            1_300,
        ),
        user_input(
            2,
            UserVerificationEvidenceKindV1::RewardPlanVerification,
            1_400,
        ),
    ])
    .expect("second user batch");

    let first = build_accounting_epoch_snapshot(
        window(),
        &economics_binding(ECONOMICS_HASH_A),
        Some(&first_service),
        Some(&first_user),
    )
    .expect("first snapshot");

    let second = build_accounting_epoch_snapshot(
        window(),
        &economics_binding(ECONOMICS_HASH_A),
        Some(&second_service),
        Some(&second_user),
    )
    .expect("second snapshot");

    assert_eq!(first, second);
    assert_eq!(
        first.canonical_artifact_cid().expect("first CID"),
        second.canonical_artifact_cid().expect("second CID")
    );
}

#[test]
fn out_of_window_evidence_rejects() {
    let service_batch = classify_service_evidence_batch(&[service_input(
        1,
        ServiceEvidenceAccountingKindV1::Delivery,
        999,
    )])
    .expect("classification succeeds before epoch binding");

    let error = build_accounting_epoch_snapshot(
        window(),
        &economics_binding(ECONOMICS_HASH_A),
        Some(&service_batch),
        None,
    )
    .expect_err("out-of-window evidence must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));

    assert!(error.to_string().contains("outside epoch window"));
}

#[test]
fn fabricated_classification_authority_rejects() {
    let mut service_batch = classify_service_evidence_batch(&[service_input(
        1,
        ServiceEvidenceAccountingKindV1::Delivery,
        1_100,
    )])
    .expect("service classification");

    service_batch.decisions[0].reward_plan_created = true;

    let error = build_accounting_epoch_snapshot(
        window(),
        &economics_binding(ECONOMICS_HASH_A),
        Some(&service_batch),
        None,
    )
    .expect_err("fabricated authority must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));
}

#[test]
fn snapshot_contains_no_amount_recipient_balance_or_receipt() {
    let snapshot = mixed_snapshot();

    let json = serde_json::to_string(&snapshot).expect("snapshot JSON");

    for forbidden in [
        "amount_minor",
        "reward_amount",
        "recipient_account",
        "payout_account",
        "confirmed_roc",
        "wallet_receipt",
        "ledger_receipt",
        "balance",
    ] {
        assert!(
            !json.contains(forbidden),
            "snapshot must not contain {forbidden}"
        );
    }

    assert!(json.contains(r#""consensus_root_claimed":false"#));
    assert!(json.contains(r#""reward_plan_created":false"#));
    assert!(json.contains(r#""wallet_mutation":false"#));
    assert!(json.contains(r#""ledger_mutation":false"#));
}

#[test]
fn snapshot_wire_requires_economics_config_binding() {
    let snapshot = mixed_snapshot();

    let mut value = serde_json::to_value(snapshot).expect("snapshot JSON");
    value
        .as_object_mut()
        .expect("snapshot object")
        .remove("economics_config");

    assert!(serde_json::from_value::<AccountingEpochSnapshotV1>(value).is_err());
}

#[test]
fn economics_config_binding_rejects_invalid_hash() {
    let error = AccountingEconomicsConfigBindingV1::from_validated_model(
        ECONOMICS_SCHEMA,
        1,
        AccountingEconomicsProfileV1::Canonical,
        "b3:not-a-full-digest",
    )
    .expect_err("invalid economics config hash must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));
}

#[test]
fn economics_config_binding_rejects_authority_smuggling() {
    let snapshot = mixed_snapshot();
    let mut value = serde_json::to_value(snapshot).expect("snapshot JSON");

    value["economics_config"]["payout_authority"] = serde_json::json!(true);

    assert!(serde_json::from_value::<AccountingEpochSnapshotV1>(value).is_err());
}

#[test]
fn changing_economics_config_hash_changes_snapshot_artifact_deterministically() {
    let first_a = mixed_snapshot_with_hash(ECONOMICS_HASH_A);
    let second_a = mixed_snapshot_with_hash(ECONOMICS_HASH_A);
    let snapshot_b = mixed_snapshot_with_hash(ECONOMICS_HASH_B);

    let first_a_cid = first_a.canonical_artifact_cid().expect("first A CID");
    let second_a_cid = second_a.canonical_artifact_cid().expect("second A CID");
    let snapshot_b_cid = snapshot_b.canonical_artifact_cid().expect("B CID");

    assert_eq!(first_a_cid, second_a_cid);
    assert_ne!(first_a_cid, snapshot_b_cid);

    assert_eq!(first_a.service_rows, snapshot_b.service_rows);
    assert_eq!(
        first_a.user_verification_rows,
        snapshot_b.user_verification_rows
    );
}

#[test]
fn snapshot_wire_rejects_raw_engagement_smuggling() {
    let snapshot = mixed_snapshot();

    let mut value = serde_json::to_value(snapshot).expect("snapshot JSON");

    value
        .as_object_mut()
        .expect("snapshot object")
        .insert("raw_engagement_count".to_owned(), serde_json::json!(1));

    assert!(serde_json::from_value::<AccountingEpochSnapshotV1>(value).is_err());
}

#[test]
fn snapshot_requires_at_least_one_source_batch() {
    let error =
        build_accounting_epoch_snapshot(window(), &economics_binding(ECONOMICS_HASH_A), None, None)
            .expect_err("empty source set must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));
}
