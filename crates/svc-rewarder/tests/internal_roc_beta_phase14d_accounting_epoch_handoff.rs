//! RO:WHAT — Phase 14D canonical accounting snapshot reward-planning handoff tests.
//! RO:WHY — Proves Service and User Node accepted evidence reaches deterministic,
//! capped, recipient-free, non-authoritative planning.

#![allow(clippy::missing_panics_doc)]

use ron_accounting::accounting::epoch_snapshot::{
    canonical_accounting_epoch_snapshot_artifact_cid, AccountingEconomicsConfigBindingV1,
    AccountingEconomicsProfileV1, AccountingEpochSnapshotV1, ServiceAccountingSnapshotRowV1,
    UserVerificationAccountingSnapshotRowV1, ACCOUNTING_EPOCH_SNAPSHOT_SCHEMA,
    ACCOUNTING_EPOCH_SNAPSHOT_VERSION,
};
use ron_accounting::accounting::node_evidence_wire::{
    EvidenceContentId, ServiceEvidenceAccountingKindV1, UserVerificationEvidenceKindV1,
};
use serde_json::json;

use svc_rewarder::{
    core::{compute_service_node_reward_plan, AmountMinor},
    inputs::{
        accounting_epoch_reward_planning_handoff, load_internal_roc_planning_economics_toml,
        AccountingEpochRewardPlanningHandoffV1, InternalRocRewardPlanningEconomics,
    },
};

const CANONICAL: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

fn cid(value: u8) -> String {
    format!("b3:{value:064x}")
}

fn economics() -> InternalRocRewardPlanningEconomics {
    load_internal_roc_planning_economics_toml(CANONICAL).expect("canonical economics should load")
}

fn policy(economics: &InternalRocRewardPlanningEconomics) -> svc_rewarder::inputs::RewardPolicy {
    economics
        .to_reward_policy("policy:phase14:accounting-handoff", &cid(90))
        .expect("signed protocol policy")
}

fn service_row(
    sequence: u64,
    node: &str,
    content: u8,
    kind: ServiceEvidenceAccountingKindV1,
) -> ServiceAccountingSnapshotRowV1 {
    ServiceAccountingSnapshotRowV1 {
        sequence,
        kind,
        proof_id: format!("evidence:service:{sequence:04}"),
        service_node_id: node.to_owned(),
        content_id: cid(content)
            .parse::<EvidenceContentId>()
            .expect("service content CID"),
        observed_at_ms: 150 + sequence,
    }
}

fn user_row(sequence: u64, node: &str, digest: u8) -> UserVerificationAccountingSnapshotRowV1 {
    UserVerificationAccountingSnapshotRowV1 {
        sequence,
        verification_kind: UserVerificationEvidenceKindV1::RewardPlanVerification,
        evidence_id: format!("evidence:user-verification:{sequence:04}"),
        user_node_id: node.to_owned(),
        subject_ref: format!("reward-plan:subject:{sequence:04}"),
        input_digest: cid(digest)
            .parse::<EvidenceContentId>()
            .expect("verification input digest"),
        observed_at_ms: 150 + sequence,
    }
}

fn snapshot(
    economics: &InternalRocRewardPlanningEconomics,
    service_rows: Vec<ServiceAccountingSnapshotRowV1>,
    user_rows: Vec<UserVerificationAccountingSnapshotRowV1>,
) -> AccountingEpochSnapshotV1 {
    let service_count = service_rows.len();
    let user_count = user_rows.len();

    let economics_config = AccountingEconomicsConfigBindingV1::from_validated_model(
        economics.schema.clone(),
        economics.version,
        AccountingEconomicsProfileV1::Canonical,
        &economics.economics_config_hash,
    )
    .expect("accounting economics binding");

    AccountingEpochSnapshotV1 {
        schema: ACCOUNTING_EPOCH_SNAPSHOT_SCHEMA.to_owned(),
        version: ACCOUNTING_EPOCH_SNAPSHOT_VERSION,
        epoch_id: "epoch:phase14:handoff".into(),
        starts_at_ms: 100,
        ends_at_ms: 200,
        produced_at_ms: 250,
        economics_config,
        service_rows,
        user_verification_rows: user_rows,
        source_service_input_count: service_count,
        source_user_verification_input_count: user_count,
        eligible_service_count: service_count,
        eligible_user_verification_count: user_count,
        policy_evidence_count: 0,
        challenge_evidence_count: 0,
        deterministic_order: true,
        snapshot_artifact_created: true,
        consensus_root_claimed: false,
        reward_plan_created: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
    .canonicalized()
    .expect("canonical accounting snapshot")
}

#[test]
fn canonical_snapshot_produces_service_plan_and_user_points() {
    let economics = economics();
    let policy = policy(&economics);

    let snapshot = snapshot(
        &economics,
        vec![
            service_row(
                1,
                "service_node:alpha",
                1,
                ServiceEvidenceAccountingKindV1::Delivery,
            ),
            service_row(
                2,
                "service_node:alpha",
                1,
                ServiceEvidenceAccountingKindV1::Delivery,
            ),
        ],
        vec![user_row(3, "user_node:verifier", 3)],
    );

    let expected_cid = canonical_accounting_epoch_snapshot_artifact_cid(&snapshot)
        .expect("accounting snapshot CID");

    let handoff = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect("accounting reward handoff");

    assert_eq!(handoff.accounting_snapshot_cid, expected_cid);
    assert!(handoff.planning_only);
    assert!(!handoff.payout_authority);
    assert!(!handoff.wallet_mutation);
    assert!(!handoff.ledger_mutation);

    let service_input = handoff
        .service_node_plan_input
        .as_ref()
        .expect("service planning input");

    assert_eq!(service_input.candidates.len(), 1);
    assert_eq!(
        service_input.candidates[0].service_node_id,
        "service_node:alpha"
    );
    assert_eq!(service_input.candidates[0].evidence_count, 2);
    assert_eq!(service_input.candidates[0].eligible_score, 2);

    assert_eq!(handoff.user_verification_point_plan.total_planned_points, 1);
    assert_eq!(
        handoff.user_verification_point_plan.points[0].user_node_id,
        "user_node:verifier"
    );
    assert!(!handoff.user_verification_point_plan.reward_amount_assigned);

    let plan = compute_service_node_reward_plan(service_input.clone(), &economics)
        .expect("identity-bound service reward plan");

    assert_eq!(plan.allocations.len(), 1);
    assert_eq!(plan.allocations[0].service_node_id, "service_node:alpha");
    assert!(plan.registry_resolution_required);
    assert!(!plan.payout_authority);
}

#[test]
fn reordered_accounting_rows_produce_identical_handoff_and_plan() {
    let economics = economics();
    let policy = policy(&economics);

    let service_rows = vec![
        service_row(
            1,
            "service_node:bravo",
            2,
            ServiceEvidenceAccountingKindV1::Availability,
        ),
        service_row(
            2,
            "service_node:alpha",
            1,
            ServiceEvidenceAccountingKindV1::Delivery,
        ),
    ];

    let user_rows = vec![
        user_row(3, "user_node:bravo", 4),
        user_row(4, "user_node:alpha", 5),
    ];

    let first_snapshot = snapshot(&economics, service_rows.clone(), user_rows.clone());

    let mut reversed_service = service_rows;
    reversed_service.reverse();

    let mut reversed_user = user_rows;
    reversed_user.reverse();

    let second_snapshot = snapshot(&economics, reversed_service, reversed_user);

    let first = accounting_epoch_reward_planning_handoff(
        &first_snapshot,
        &policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect("first handoff");

    let second = accounting_epoch_reward_planning_handoff(
        &second_snapshot,
        &policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect("reordered handoff");

    assert_eq!(first, second);

    let first_plan = compute_service_node_reward_plan(
        first.service_node_plan_input.expect("first service input"),
        &economics,
    )
    .expect("first service plan");

    let second_plan = compute_service_node_reward_plan(
        second
            .service_node_plan_input
            .expect("second service input"),
        &economics,
    )
    .expect("second service plan");

    assert_eq!(first_plan, second_plan);
}

#[test]
fn accounting_economics_binding_mismatch_fails_closed() {
    let economics = economics();
    let policy = policy(&economics);

    let snapshot = snapshot(
        &economics,
        vec![service_row(
            1,
            "service_node:alpha",
            1,
            ServiceEvidenceAccountingKindV1::Delivery,
        )],
        Vec::new(),
    );

    let mut mismatched = economics.clone();
    mismatched.economics_config_hash = cid(99);

    let error = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &policy,
        AmountMinor(1_000_000),
        &mismatched,
    )
    .expect_err("economics mismatch must reject");

    assert_eq!(error.reason(), "bad_request");
    assert!(error.to_string().contains("economics binding"));
}

#[test]
fn policy_only_service_evidence_cannot_enter_reward_planning() {
    let economics = economics();
    let policy = policy(&economics);

    let mut snapshot = snapshot(
        &economics,
        vec![service_row(
            1,
            "service_node:alpha",
            1,
            ServiceEvidenceAccountingKindV1::Delivery,
        )],
        Vec::new(),
    );

    snapshot.service_rows[0].kind = ServiceEvidenceAccountingKindV1::PolicyRefusal;

    let error = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect_err("policy-only service row must reject");

    assert_eq!(error.reason(), "bad_request");
}

#[test]
fn user_verification_points_obey_economics_event_cap() {
    let raw = std::str::from_utf8(CANONICAL).expect("canonical economics UTF-8");

    let changed = raw.replacen(
        "max_events_per_account_per_epoch = 1000",
        "max_events_per_account_per_epoch = 1",
        1,
    );

    assert_ne!(raw, changed);

    let economics = load_internal_roc_planning_economics_toml(changed.as_bytes())
        .expect("event-capped economics");

    let policy = policy(&economics);

    let snapshot = snapshot(
        &economics,
        Vec::new(),
        vec![
            user_row(1, "user_node:farmer", 1),
            user_row(2, "user_node:farmer", 2),
        ],
    );

    let error = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect_err("second user event must exceed cap");

    assert_eq!(error.reason(), "bad_request");
    assert!(error
        .to_string()
        .contains("exceeds economics max events per account per epoch"));
}

#[test]
fn arbitrary_recipient_and_reward_amount_fields_reject() {
    let economics = economics();
    let policy = policy(&economics);

    let snapshot = snapshot(
        &economics,
        vec![service_row(
            1,
            "service_node:alpha",
            1,
            ServiceEvidenceAccountingKindV1::Delivery,
        )],
        vec![user_row(2, "user_node:verifier", 2)],
    );

    let handoff = accounting_epoch_reward_planning_handoff(
        &snapshot,
        &policy,
        AmountMinor(1_000_000),
        &economics,
    )
    .expect("valid handoff");

    let mut value = serde_json::to_value(&handoff).expect("handoff JSON");

    value
        .as_object_mut()
        .expect("handoff object")
        .insert("payout_recipient".into(), json!("@attacker"));

    assert!(serde_json::from_value::<AccountingEpochRewardPlanningHandoffV1>(value).is_err());

    let mut value = serde_json::to_value(&handoff).expect("handoff JSON");

    value["user_verification_point_plan"]["points"][0]
        .as_object_mut()
        .expect("user point object")
        .insert("reward_amount_minor".into(), json!("999999"));

    assert!(serde_json::from_value::<AccountingEpochRewardPlanningHandoffV1>(value).is_err());
}
