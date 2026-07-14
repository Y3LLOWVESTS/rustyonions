//! RO:WHAT — Phase 18 probation reward-cap enforcement tests.
//!
//! RO:WHY — Service Nodes may participate during probation only under the
//! economics-owned ceiling, while unsafe lifecycle states fail closed.
//!
//! RO:INTERACTS — `ron-proto` lifecycle descriptors, canonical ROC economics,
//! and the existing identity-bound Service Node reward planner.
//!
//! RO:INVARIANTS — exact descriptor set; deterministic ordering; probation cap;
//! eligible normal cap; no wallet, ledger, receipt, balance, or payout authority.

#![allow(clippy::missing_panics_doc)]

use ron_proto::{ContentId, ServiceNodeEligibilityStateV1, ServiceNodeIdentityDescriptorV1};
use svc_rewarder::{
    core::{
        compute_service_node_reward_plan_with_eligibility, AmountMinor,
        ServiceNodeEligibilityRewardPlan, ServiceNodeRewardCandidate,
        ServiceNodeRewardEvidenceClass, ServiceNodeRewardPlanInput,
    },
    inputs::{load_internal_roc_planning_economics_toml, InternalRocRewardPlanningEconomics},
};

const CANONICAL: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

fn cid(value: u8) -> String {
    format!("b3:{value:064x}")
}

fn content_id(value: u8) -> ContentId {
    cid(value).parse().expect("fixture ContentId should parse")
}

fn economics() -> InternalRocRewardPlanningEconomics {
    load_internal_roc_planning_economics_toml(CANONICAL).expect("canonical economics should load")
}

fn changed_economics(old_cap: AmountMinor, new_cap: u128) -> InternalRocRewardPlanningEconomics {
    let old = format!(
        "probation_reward_cap_minor_per_node_per_epoch = \"{}\"",
        old_cap.get()
    );

    let new = format!("probation_reward_cap_minor_per_node_per_epoch = \"{new_cap}\"");

    let raw = std::str::from_utf8(CANONICAL).expect("canonical economics should be UTF-8");

    assert_eq!(raw.matches(&old).count(), 1);

    load_internal_roc_planning_economics_toml(raw.replacen(&old, &new, 1).as_bytes())
        .expect("changed complete economics should validate")
}

fn candidate(service_node_id: &str, content_seed: u8) -> ServiceNodeRewardCandidate {
    ServiceNodeRewardCandidate {
        service_node_id: service_node_id.to_owned(),
        evidence_class: ServiceNodeRewardEvidenceClass::Delivery,
        content_id: cid(content_seed),
        evidence_count: 1,
        eligible_score: 100,
        evidence_verified: true,
        accounting_accepted: true,
        policy_gate_passed: true,
        challenge_required: false,
        challenge_accepted: false,
    }
}

fn input(
    economics: &InternalRocRewardPlanningEconomics,
    candidates: Vec<ServiceNodeRewardCandidate>,
) -> ServiceNodeRewardPlanInput {
    ServiceNodeRewardPlanInput {
        epoch_id: "epoch:phase18:probation-cap".to_string(),
        accounting_snapshot_cid: cid(10),
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: cid(11),
        pool_minor_units: AmountMinor(1_000_000),
        candidates,
    }
}

fn descriptor(
    service_node_id: &str,
    state: ServiceNodeEligibilityStateV1,
    seed: u8,
) -> ServiceNodeIdentityDescriptorV1 {
    let mut descriptor = ServiceNodeIdentityDescriptorV1::new_candidate(
        service_node_id.to_string(),
        format!("registry:{service_node_id}"),
        format!("binding:{service_node_id}"),
        format!("key:{service_node_id}"),
        1,
        content_id(seed),
        content_id(seed.saturating_add(1)),
    )
    .expect("candidate descriptor should validate");

    descriptor.state = state;

    if descriptor.state != ServiceNodeEligibilityStateV1::Candidate {
        descriptor.state_effective_epoch = 2;
    }

    descriptor
        .validate()
        .expect("fixture descriptor should validate");

    descriptor
}

fn node_total(result: &ServiceNodeEligibilityRewardPlan, service_node_id: &str) -> AmountMinor {
    result
        .plan
        .allocations
        .iter()
        .filter(|allocation| allocation.service_node_id == service_node_id)
        .fold(AmountMinor::ZERO, |total, allocation| {
            total
                .checked_add(allocation.amount_minor_units)
                .expect("allocation total should not overflow")
        })
}

#[test]
fn probation_node_is_capped_while_eligible_node_uses_normal_cap() {
    let economics = economics();

    let result = compute_service_node_reward_plan_with_eligibility(
        input(
            &economics,
            vec![
                candidate("service_node:probation", 1),
                candidate("service_node:eligible", 2),
            ],
        ),
        vec![
            descriptor(
                "service_node:probation",
                ServiceNodeEligibilityStateV1::Probation,
                20,
            ),
            descriptor(
                "service_node:eligible",
                ServiceNodeEligibilityStateV1::Eligible,
                30,
            ),
        ],
        &economics,
    )
    .expect("eligibility-aware plan should compute");

    assert_eq!(
        node_total(&result, "service_node:probation"),
        economics.probation_reward_cap_minor_per_node_per_epoch
    );

    assert_eq!(
        node_total(&result, "service_node:eligible"),
        economics.max_reward_minor_per_account_per_epoch
    );

    assert_eq!(
        result.probation_service_node_ids,
        vec!["service_node:probation".to_string()]
    );

    assert!(result.planning_only);
    assert!(result.registry_attestation_required);
    assert!(!result.payout_authority);
    assert!(!result.payout_executed);
    assert!(!result.wallet_mutation);
    assert!(!result.ledger_mutation);
    assert!(!result.receipt_created);
    assert!(!result.balance_truth);

    result.validate().expect("result should validate");
}

#[test]
fn descriptor_order_does_not_change_eligibility_plan() {
    let economics = economics();

    let reward_input = input(
        &economics,
        vec![
            candidate("service_node:alpha", 1),
            candidate("service_node:bravo", 2),
        ],
    );

    let descriptors = vec![
        descriptor(
            "service_node:alpha",
            ServiceNodeEligibilityStateV1::Probation,
            20,
        ),
        descriptor(
            "service_node:bravo",
            ServiceNodeEligibilityStateV1::Eligible,
            30,
        ),
    ];

    let first = compute_service_node_reward_plan_with_eligibility(
        reward_input.clone(),
        descriptors.clone(),
        &economics,
    )
    .expect("first eligibility plan");

    let mut reversed = descriptors;
    reversed.reverse();

    let second =
        compute_service_node_reward_plan_with_eligibility(reward_input, reversed, &economics)
            .expect("reordered eligibility plan");

    assert_eq!(first, second);
}

#[test]
fn candidate_degraded_quarantined_and_blocked_states_fail_closed() {
    let economics = economics();

    for (index, state) in [
        ServiceNodeEligibilityStateV1::Candidate,
        ServiceNodeEligibilityStateV1::Degraded,
        ServiceNodeEligibilityStateV1::Quarantined,
        ServiceNodeEligibilityStateV1::Blocked,
    ]
    .into_iter()
    .enumerate()
    {
        let service_node_id = format!("service_node:denied_{index}");

        let error = compute_service_node_reward_plan_with_eligibility(
            input(
                &economics,
                vec![candidate(&service_node_id, index as u8 + 1)],
            ),
            vec![descriptor(&service_node_id, state, index as u8 + 40)],
            &economics,
        )
        .expect_err("unsafe lifecycle state must reject");

        assert!(
            error.to_string().contains("cannot enter reward planning"),
            "unexpected rejection: {error}"
        );
    }
}

#[test]
fn descriptor_set_must_exactly_match_candidate_nodes() {
    let economics = economics();

    let reward_input = input(&economics, vec![candidate("service_node:alpha", 1)]);

    let missing = compute_service_node_reward_plan_with_eligibility(
        reward_input.clone(),
        Vec::new(),
        &economics,
    )
    .expect_err("missing descriptor must reject");

    assert!(missing.to_string().contains("descriptor set mismatch"));

    let alpha = descriptor(
        "service_node:alpha",
        ServiceNodeEligibilityStateV1::Eligible,
        20,
    );

    let duplicate = compute_service_node_reward_plan_with_eligibility(
        reward_input.clone(),
        vec![alpha.clone(), alpha],
        &economics,
    )
    .expect_err("duplicate descriptor must reject");

    assert!(duplicate
        .to_string()
        .contains("duplicate Service Node eligibility descriptor"));

    let extra = compute_service_node_reward_plan_with_eligibility(
        reward_input,
        vec![
            descriptor(
                "service_node:alpha",
                ServiceNodeEligibilityStateV1::Eligible,
                20,
            ),
            descriptor(
                "service_node:extra",
                ServiceNodeEligibilityStateV1::Eligible,
                30,
            ),
        ],
        &economics,
    )
    .expect_err("extra descriptor must reject");

    assert!(extra.to_string().contains("descriptor set mismatch"));
}

#[test]
fn economics_probation_cap_changes_allocation_and_plan_identity() {
    let original = economics();
    let changed = changed_economics(
        original.probation_reward_cap_minor_per_node_per_epoch,
        1_500,
    );

    let original_result = compute_service_node_reward_plan_with_eligibility(
        input(&original, vec![candidate("service_node:probation", 1)]),
        vec![descriptor(
            "service_node:probation",
            ServiceNodeEligibilityStateV1::Probation,
            20,
        )],
        &original,
    )
    .expect("original probation plan");

    let changed_result = compute_service_node_reward_plan_with_eligibility(
        input(&changed, vec![candidate("service_node:probation", 1)]),
        vec![descriptor(
            "service_node:probation",
            ServiceNodeEligibilityStateV1::Probation,
            20,
        )],
        &changed,
    )
    .expect("changed probation plan");

    assert_eq!(
        node_total(&changed_result, "service_node:probation"),
        AmountMinor(1_500)
    );

    assert_ne!(
        original_result.plan.economics_config_hash,
        changed_result.plan.economics_config_hash
    );

    assert_ne!(
        original_result.eligibility_plan_id,
        changed_result.eligibility_plan_id
    );

    assert_ne!(original_result.plan.plan_id, changed_result.plan.plan_id);
}

#[test]
fn eligibility_review_hash_binds_lifecycle_history() {
    let economics = economics();
    let reward_input = input(&economics, vec![candidate("service_node:alpha", 1)]);

    let first_descriptor = descriptor(
        "service_node:alpha",
        ServiceNodeEligibilityStateV1::Eligible,
        20,
    );

    let first = compute_service_node_reward_plan_with_eligibility(
        reward_input.clone(),
        vec![first_descriptor.clone()],
        &economics,
    )
    .expect("first review");

    let mut changed_descriptor = first_descriptor;
    changed_descriptor.service_history_root = content_id(99);
    changed_descriptor
        .validate()
        .expect("changed descriptor should validate");

    let second = compute_service_node_reward_plan_with_eligibility(
        reward_input,
        vec![changed_descriptor],
        &economics,
    )
    .expect("changed review");

    assert_ne!(
        first.eligibility_review_hash,
        second.eligibility_review_hash
    );

    assert_ne!(first.eligibility_plan_id, second.eligibility_plan_id);
}
