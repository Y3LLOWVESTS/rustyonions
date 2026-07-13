//! RO:WHAT — Phase 14D identity-bound Service Node reward-plan tests.
//! RO:WHY — Proves real category/account/content/event caps without arbitrary recipients.

#![allow(clippy::missing_panics_doc)]

use serde_json::json;

use svc_rewarder::{
    core::{
        compute_service_node_reward_plan, AmountMinor, ServiceNodeRewardCandidate,
        ServiceNodeRewardEvidenceClass, ServiceNodeRewardPlanInput,
    },
    inputs::{load_internal_roc_planning_economics_toml, InternalRocRewardPlanningEconomics},
};

const CANONICAL: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

fn cid(value: u8) -> String {
    format!("b3:{value:064x}")
}

fn candidate(
    node: &str,
    content: u8,
    evidence_count: u64,
    eligible_score: u128,
) -> ServiceNodeRewardCandidate {
    ServiceNodeRewardCandidate {
        service_node_id: node.to_owned(),
        evidence_class: ServiceNodeRewardEvidenceClass::Delivery,
        content_id: cid(content),
        evidence_count,
        eligible_score,
        evidence_verified: true,
        accounting_accepted: true,
        policy_gate_passed: true,
        challenge_required: false,
        challenge_accepted: false,
    }
}

fn canonical_economics() -> InternalRocRewardPlanningEconomics {
    load_internal_roc_planning_economics_toml(CANONICAL).expect("canonical economics should load")
}

fn changed_economics(replacements: &[(&str, &str)]) -> InternalRocRewardPlanningEconomics {
    let mut raw = std::str::from_utf8(CANONICAL)
        .expect("canonical economics should be UTF-8")
        .to_owned();

    for (old, new) in replacements {
        assert_eq!(
            raw.matches(old).count(),
            1,
            "economics test replacement must match once: {old}"
        );
        raw = raw.replacen(old, new, 1);
    }

    load_internal_roc_planning_economics_toml(raw.as_bytes())
        .expect("changed complete economics should validate")
}

fn input(
    economics: &InternalRocRewardPlanningEconomics,
    candidates: Vec<ServiceNodeRewardCandidate>,
) -> ServiceNodeRewardPlanInput {
    ServiceNodeRewardPlanInput {
        epoch_id: "epoch:phase14d:service-node".into(),
        accounting_snapshot_cid: cid(10),
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: cid(11),
        pool_minor_units: AmountMinor(1_000_000),
        candidates,
    }
}

#[test]
fn reordered_candidates_produce_identical_service_node_plan() {
    let economics = canonical_economics();

    let candidates = vec![
        candidate("service_node:bravo", 2, 1, 100),
        candidate("service_node:alpha", 1, 1, 200),
    ];

    let first = compute_service_node_reward_plan(input(&economics, candidates.clone()), &economics)
        .expect("first service-node plan");

    let mut reversed = candidates;
    reversed.reverse();

    let second = compute_service_node_reward_plan(input(&economics, reversed), &economics)
        .expect("reordered service-node plan");

    assert_eq!(first, second);
    assert!(first.planning_only);
    assert!(first.registry_resolution_required);
    assert!(!first.payout_authority);
    assert!(!first.payout_executed);
    assert!(!first.wallet_mutation);
    assert!(!first.ledger_mutation);
    assert!(!first.receipt_created);
    assert!(!first.balance_truth);
}

#[test]
fn node_delivery_category_cap_changes_actual_allocation() {
    let economics = changed_economics(&[
        (
            r#"max_reward_minor_per_account_per_epoch = "10000""#,
            r#"max_reward_minor_per_account_per_epoch = "1000000""#,
        ),
        (
            r#"max_reward_minor_per_content_per_epoch = "50000""#,
            r#"max_reward_minor_per_content_per_epoch = "1000000""#,
        ),
    ]);

    let plan = compute_service_node_reward_plan(
        input(&economics, vec![candidate("service_node:alpha", 1, 1, 100)]),
        &economics,
    )
    .expect("category-capped plan");

    assert_eq!(plan.allocations.len(), 1);
    assert_eq!(plan.allocations[0].reward_category, "node_delivery");
    assert_eq!(plan.allocations[0].amount_minor_units, AmountMinor(200_000));
}

#[test]
fn per_node_cap_applies_across_multiple_content_rows() {
    let economics = canonical_economics();

    let plan = compute_service_node_reward_plan(
        input(
            &economics,
            vec![
                candidate("service_node:alpha", 1, 1, 100),
                candidate("service_node:alpha", 2, 1, 100),
            ],
        ),
        &economics,
    )
    .expect("node-capped plan");

    let node_total = plan
        .allocations
        .iter()
        .fold(AmountMinor::ZERO, |total, allocation| {
            total
                .checked_add(allocation.amount_minor_units)
                .expect("node allocation sum")
        });

    assert_eq!(node_total, economics.max_reward_minor_per_account_per_epoch);
}

#[test]
fn per_content_cap_applies_across_multiple_service_nodes() {
    let economics = changed_economics(&[(
        r#"max_reward_minor_per_account_per_epoch = "10000""#,
        r#"max_reward_minor_per_account_per_epoch = "1000000""#,
    )]);

    let shared_content = 9;

    let plan = compute_service_node_reward_plan(
        input(
            &economics,
            vec![
                candidate("service_node:alpha", shared_content, 1, 100),
                candidate("service_node:bravo", shared_content, 1, 100),
            ],
        ),
        &economics,
    )
    .expect("content-capped plan");

    let content_total = plan
        .allocations
        .iter()
        .fold(AmountMinor::ZERO, |total, allocation| {
            total
                .checked_add(allocation.amount_minor_units)
                .expect("content allocation sum")
        });

    assert_eq!(
        content_total,
        economics.max_reward_minor_per_content_per_epoch
    );
}

#[test]
fn economics_event_limit_rejects_service_node_overage() {
    let economics = changed_economics(&[(
        "max_events_per_account_per_epoch = 1000",
        "max_events_per_account_per_epoch = 2",
    )]);

    let error = compute_service_node_reward_plan(
        input(
            &economics,
            vec![candidate("service_node:farmer", 1, 3, 100)],
        ),
        &economics,
    )
    .expect_err("event overage must reject");

    assert_eq!(error.reason(), "bad_request");
    assert!(error
        .to_string()
        .contains("exceeds economics max events per account per epoch"));
}

#[test]
fn policy_and_challenge_posture_fail_closed() {
    let economics = canonical_economics();

    let mut missing_policy = candidate("service_node:alpha", 1, 1, 100);
    missing_policy.policy_gate_passed = false;

    let error =
        compute_service_node_reward_plan(input(&economics, vec![missing_policy]), &economics)
            .expect_err("missing policy gate must reject");

    assert!(error.to_string().contains("ron-policy gate"));

    let mut unresolved_challenge = candidate("service_node:alpha", 1, 1, 100);
    unresolved_challenge.challenge_required = true;

    let error =
        compute_service_node_reward_plan(input(&economics, vec![unresolved_challenge]), &economics)
            .expect_err("unresolved challenge must reject");

    assert!(error.to_string().contains("requires accepted challenge"));
}

#[test]
fn arbitrary_recipient_and_amount_fields_reject_at_input_boundary() {
    let mut value =
        serde_json::to_value(candidate("service_node:alpha", 1, 1, 100)).expect("candidate JSON");

    let object = value
        .as_object_mut()
        .expect("candidate should serialize as object");

    object.insert("payout_recipient".into(), json!("@attacker"));

    assert!(serde_json::from_value::<ServiceNodeRewardCandidate>(value).is_err());

    let mut value =
        serde_json::to_value(candidate("service_node:alpha", 1, 1, 100)).expect("candidate JSON");

    value
        .as_object_mut()
        .expect("candidate should serialize as object")
        .insert("requested_reward_minor".into(), json!("999999"));

    assert!(serde_json::from_value::<ServiceNodeRewardCandidate>(value).is_err());
}

#[test]
fn changing_category_cap_changes_plan_identity_and_amount() {
    let high_caps = [
        (
            r#"max_reward_minor_per_account_per_epoch = "10000""#,
            r#"max_reward_minor_per_account_per_epoch = "1000000""#,
        ),
        (
            r#"max_reward_minor_per_content_per_epoch = "50000""#,
            r#"max_reward_minor_per_content_per_epoch = "1000000""#,
        ),
    ];

    let canonical_cap = changed_economics(&high_caps);

    let changed_cap = changed_economics(&[
        high_caps[0],
        high_caps[1],
        (
            r#"category = "node_delivery"
pool_bps = 2000
category_cap_minor = "200000""#,
            r#"category = "node_delivery"
pool_bps = 2000
category_cap_minor = "150000""#,
        ),
    ]);

    let candidate = candidate("service_node:alpha", 1, 1, 100);

    let first = compute_service_node_reward_plan(
        input(&canonical_cap, vec![candidate.clone()]),
        &canonical_cap,
    )
    .expect("canonical category plan");

    let second =
        compute_service_node_reward_plan(input(&changed_cap, vec![candidate]), &changed_cap)
            .expect("changed category plan");

    assert_ne!(first.economics_config_hash, second.economics_config_hash);
    assert_ne!(first.plan_id, second.plan_id);
    assert_eq!(
        first.allocations[0].amount_minor_units,
        AmountMinor(200_000)
    );
    assert_eq!(
        second.allocations[0].amount_minor_units,
        AmountMinor(150_000)
    );
}
