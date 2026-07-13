//! RO:WHAT — Phase 14 deterministic service-evidence classification tests.
//! RO:WHY — Prove service evidence is classified before capped reward planning.

use ron_accounting::{
    classify_service_evidence, classify_service_evidence_batch, Error,
    ServiceEvidenceAccountingClassV1,
};
use ron_accounting::{
    EvidenceContentId, ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingKindV1,
    SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA, SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn input(sequence: u64, kind: ServiceEvidenceAccountingKindV1) -> ServiceEvidenceAccountingInputV1 {
    ServiceEvidenceAccountingInputV1 {
        schema: SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
        sequence,
        kind,
        proof_id: format!("evidence:proof:{sequence:04}"),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        related_actor_ids: Vec::new(),
        content_id: CID
            .parse::<EvidenceContentId>()
            .expect("canonical content ID"),
        observed_at_ms: 1_900_000_000_000 + sequence,
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

#[test]
fn five_service_kinds_become_capped_planning_candidates() {
    for kind in [
        ServiceEvidenceAccountingKindV1::Delivery,
        ServiceEvidenceAccountingKindV1::Availability,
        ServiceEvidenceAccountingKindV1::RangeRequest,
        ServiceEvidenceAccountingKindV1::Repair,
        ServiceEvidenceAccountingKindV1::HotCache,
    ] {
        let decision =
            classify_service_evidence(&input(1, kind)).expect("classified service evidence");

        assert_eq!(
            decision.class,
            ServiceEvidenceAccountingClassV1::ProofEligibleService
        );

        assert!(decision.verified_evidence);
        assert!(decision.requires_policy_review);
        assert!(decision.requires_economics_config);
        assert!(decision.requires_cap);
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
fn refusal_and_moderation_remain_policy_evidence_only() {
    for kind in [
        ServiceEvidenceAccountingKindV1::PolicyRefusal,
        ServiceEvidenceAccountingKindV1::ModerationAction,
    ] {
        let decision =
            classify_service_evidence(&input(1, kind)).expect("classified policy evidence");

        assert_eq!(
            decision.class,
            ServiceEvidenceAccountingClassV1::PolicyEvidenceOnly
        );

        assert!(decision.verified_evidence);
        assert!(decision.requires_policy_review);
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
    let first = input(1, ServiceEvidenceAccountingKindV1::Delivery);

    let second = input(2, ServiceEvidenceAccountingKindV1::PolicyRefusal);

    let ordered = classify_service_evidence_batch(&[first.clone(), second.clone()])
        .expect("ordered classification");

    let reversed =
        classify_service_evidence_batch(&[second, first]).expect("reversed classification");

    assert_eq!(ordered, reversed);
    assert_eq!(ordered.input_count, 2);
    assert_eq!(ordered.proof_eligible_count, 1);
    assert_eq!(ordered.policy_evidence_count, 1);
    assert!(ordered.deterministic_order);

    assert!(!ordered.accounting_snapshot_created);
    assert!(!ordered.reward_plan_created);
    assert!(!ordered.payout_authority);
    assert!(!ordered.wallet_mutation);
    assert!(!ordered.ledger_mutation);
}

#[test]
fn duplicate_sequence_rejects_whole_batch() {
    let first = input(1, ServiceEvidenceAccountingKindV1::Delivery);

    let mut second = input(1, ServiceEvidenceAccountingKindV1::Availability);
    second.proof_id = "evidence:proof:different".to_owned();

    let error = classify_service_evidence_batch(&[first, second])
        .expect_err("duplicate sequence must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));
    assert!(error
        .to_string()
        .contains("duplicate service evidence sequence"));
}

#[test]
fn duplicate_proof_identity_rejects_whole_batch() {
    let first = input(1, ServiceEvidenceAccountingKindV1::Delivery);

    let mut second = input(2, ServiceEvidenceAccountingKindV1::Delivery);
    second.proof_id = first.proof_id.clone();

    let error = classify_service_evidence_batch(&[first, second])
        .expect_err("duplicate proof identity must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));
    assert!(error
        .to_string()
        .contains("duplicate service evidence proof identity"));
}

#[test]
fn authority_poison_rejects_before_classification() {
    let mut poisoned = input(1, ServiceEvidenceAccountingKindV1::Delivery);
    poisoned.reward_truth = true;

    let error = classify_service_evidence(&poisoned).expect_err("reward truth must reject");

    assert!(matches!(error, Error::SchemaViolation(_)));
    assert!(error.to_string().contains("reward_truth"));
}

#[test]
fn classification_contains_no_amount_balance_or_receipt_truth() {
    let batch = classify_service_evidence_batch(&[
        input(1, ServiceEvidenceAccountingKindV1::Delivery),
        input(2, ServiceEvidenceAccountingKindV1::ModerationAction),
    ])
    .expect("classification batch");

    let json = serde_json::to_string(&batch).expect("classification JSON");

    for forbidden in [
        "reward_amount",
        "amount_minor",
        "balance",
        "receipt",
        "payout_account",
        "confirmed_roc",
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
