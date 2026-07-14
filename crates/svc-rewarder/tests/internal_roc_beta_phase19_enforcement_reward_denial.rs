//! RO:WHAT — Phase 19 enforcement-aware reward-denial tests.
//!
//! RO:WHY — Contained nodes must earn zero while honest eligible candidates
//! continue through deterministic planning.
//!
//! RO:INVARIANTS — blocked/quarantined/degraded candidates receive no
//! allocations; pending appeals do not restore rewards; exact status matching;
//! no wallet, ledger, receipt, payout, balance, or finality authority.

#![allow(clippy::missing_panics_doc)]

use ron_proto::{
    ContentId, ServiceNodeAppealStateV1, ServiceNodeAppealStatusV1, ServiceNodeEligibilityStateV1,
    ServiceNodeEnforcementStatusV1, ServiceNodeIdentityDescriptorV1, ServiceNodeViolationKindV1,
    SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA, SERVICE_NODE_ENFORCEMENT_VERSION,
};
use svc_rewarder::{
    core::{
        compute_service_node_reward_review_with_enforcement, AmountMinor,
        ServiceNodeRewardCandidate, ServiceNodeRewardEvidenceClass, ServiceNodeRewardPlanInput,
    },
    inputs::{load_internal_roc_planning_economics_toml, InternalRocRewardPlanningEconomics},
};

const ECONOMICS: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

fn economics() -> InternalRocRewardPlanningEconomics {
    load_internal_roc_planning_economics_toml(ECONOMICS).expect("canonical economics should load")
}

fn cid(label: &str) -> ContentId {
    format!("b3:{}", blake3::hash(label.as_bytes()).to_hex())
        .parse()
        .expect("fixture ContentId should parse")
}

fn b3(label: &str) -> String {
    cid(label).to_string()
}

fn candidate(service_node_id: &str, content_label: &str) -> ServiceNodeRewardCandidate {
    ServiceNodeRewardCandidate {
        service_node_id: service_node_id.to_string(),
        evidence_class: ServiceNodeRewardEvidenceClass::Delivery,
        content_id: b3(content_label),
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
        epoch_id: "epoch:phase19:reward-denial".to_string(),
        accounting_snapshot_cid: b3("accounting-snapshot"),
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: b3("policy"),
        pool_minor_units: AmountMinor(1_000_000),
        candidates,
    }
}

fn descriptor(
    service_node_id: &str,
    state: ServiceNodeEligibilityStateV1,
    seed: &str,
) -> ServiceNodeIdentityDescriptorV1 {
    let mut descriptor = ServiceNodeIdentityDescriptorV1::new_candidate(
        service_node_id.to_string(),
        format!("registry:{seed}"),
        format!("binding:{seed}:1"),
        format!("key:{seed}"),
        1,
        cid(&format!("service-history-{seed}")),
        cid(&format!("challenge-history-{seed}")),
    )
    .expect("candidate descriptor should validate");

    descriptor.state = state;

    if state != ServiceNodeEligibilityStateV1::Candidate {
        descriptor.state_effective_epoch = 2;
    }

    descriptor
        .validate()
        .expect("fixture descriptor should validate");

    descriptor
}

fn status(
    service_node_id: &str,
    state: ServiceNodeEligibilityStateV1,
    violation: ServiceNodeViolationKindV1,
    appeal: ServiceNodeAppealStatusV1,
    seed: &str,
) -> ServiceNodeEnforcementStatusV1 {
    let status = ServiceNodeEnforcementStatusV1 {
        schema: SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA.to_string(),
        version: SERVICE_NODE_ENFORCEMENT_VERSION,
        status_id: format!("enforcement:{seed}"),
        service_node_id: service_node_id.to_string(),
        state,
        reason: violation,
        evidence_root: cid(&format!("evidence-{seed}")),
        effective_epoch: 2,
        appeal,
    };

    status
        .validate()
        .expect("fixture enforcement status should validate");

    status
}

fn allocation_nodes(
    review: &svc_rewarder::core::ServiceNodeEnforcementRewardReview,
) -> Vec<String> {
    review.plan.as_ref().map_or_else(Vec::new, |plan| {
        plan.plan
            .allocations
            .iter()
            .map(|allocation| allocation.service_node_id.clone())
            .collect()
    })
}

#[test]
fn blocked_node_gets_no_allocation_while_eligible_node_plans() {
    let economics = economics();

    let review = compute_service_node_reward_review_with_enforcement(
        input(
            &economics,
            vec![
                candidate("service_node:blocked", "blocked-content"),
                candidate("service_node:eligible", "eligible-content"),
            ],
        ),
        vec![
            descriptor(
                "service_node:blocked",
                ServiceNodeEligibilityStateV1::Blocked,
                "blocked",
            ),
            descriptor(
                "service_node:eligible",
                ServiceNodeEligibilityStateV1::Eligible,
                "eligible",
            ),
        ],
        vec![status(
            "service_node:blocked",
            ServiceNodeEligibilityStateV1::Blocked,
            ServiceNodeViolationKindV1::DenylistViolation,
            ServiceNodeAppealStatusV1::not_appealed(),
            "blocked",
        )],
        &economics,
    )
    .expect("mixed enforcement review should compute");

    assert_eq!(review.denied_service_nodes.len(), 1);
    assert_eq!(
        review.denied_service_nodes[0].service_node_id,
        "service_node:blocked"
    );
    assert_eq!(
        review.denied_service_nodes[0].state,
        ServiceNodeEligibilityStateV1::Blocked
    );
    assert_eq!(
        review.denied_service_nodes[0].violation,
        ServiceNodeViolationKindV1::DenylistViolation
    );
    assert_eq!(
        allocation_nodes(&review),
        vec!["service_node:eligible".to_string()]
    );

    assert!(!review.all_candidates_denied());
    assert!(!review.authorizes_economic_mutation());
    assert!(!review.payout_authority);
    assert!(!review.wallet_mutation);
    assert!(!review.ledger_mutation);
    assert!(!review.receipt_created);
    assert!(!review.balance_truth);

    review.validate().expect("review should validate");
}

#[test]
fn all_blocked_candidates_produce_denial_only_review() {
    let economics = economics();

    let review = compute_service_node_reward_review_with_enforcement(
        input(
            &economics,
            vec![candidate("service_node:blocked", "blocked-content")],
        ),
        vec![descriptor(
            "service_node:blocked",
            ServiceNodeEligibilityStateV1::Blocked,
            "blocked",
        )],
        vec![status(
            "service_node:blocked",
            ServiceNodeEligibilityStateV1::Blocked,
            ServiceNodeViolationKindV1::UnilateralMintAttempt,
            ServiceNodeAppealStatusV1::not_appealed(),
            "blocked",
        )],
        &economics,
    )
    .expect("all-contained review should succeed");

    assert!(review.all_candidates_denied());
    assert!(review.plan.is_none());
    assert_eq!(review.denied_service_nodes[0].candidate_rows, 1);
    assert!(!review.denied_service_nodes[0].permits_reward_planning());
}

#[test]
fn reward_binding_abuse_denies_rewards_during_pending_appeal() {
    let economics = economics();

    let appeal = ServiceNodeAppealStatusV1 {
        state: ServiceNodeAppealStateV1::Pending,
        appeal_id: Some("appeal:binding:0001".to_string()),
        submitted_epoch: Some(3),
        resolved_epoch: None,
        resolution_evidence_root: None,
    };

    let review = compute_service_node_reward_review_with_enforcement(
        input(
            &economics,
            vec![candidate("service_node:abusive", "abusive-content")],
        ),
        vec![descriptor(
            "service_node:abusive",
            ServiceNodeEligibilityStateV1::Quarantined,
            "abusive",
        )],
        vec![status(
            "service_node:abusive",
            ServiceNodeEligibilityStateV1::Quarantined,
            ServiceNodeViolationKindV1::RewardRecipientBindingAbuse,
            appeal,
            "abusive",
        )],
        &economics,
    )
    .expect("binding-abuse denial should compute");

    assert!(review.all_candidates_denied());
    assert_eq!(
        review.denied_service_nodes[0].violation,
        ServiceNodeViolationKindV1::RewardRecipientBindingAbuse
    );
    assert_eq!(
        review.denied_service_nodes[0].appeal.state,
        ServiceNodeAppealStateV1::Pending
    );
}

#[test]
fn missing_extra_or_mismatched_statuses_fail_closed() {
    let economics = economics();

    let reward_input = input(
        &economics,
        vec![candidate("service_node:blocked", "blocked-content")],
    );

    let blocked_descriptor = descriptor(
        "service_node:blocked",
        ServiceNodeEligibilityStateV1::Blocked,
        "blocked",
    );

    let missing = compute_service_node_reward_review_with_enforcement(
        reward_input.clone(),
        vec![blocked_descriptor.clone()],
        Vec::new(),
        &economics,
    )
    .expect_err("missing status must reject");

    assert!(missing.to_string().contains("status set mismatch"));

    let mismatched = compute_service_node_reward_review_with_enforcement(
        reward_input,
        vec![blocked_descriptor],
        vec![status(
            "service_node:blocked",
            ServiceNodeEligibilityStateV1::Quarantined,
            ServiceNodeViolationKindV1::HashMismatch,
            ServiceNodeAppealStatusV1::not_appealed(),
            "blocked",
        )],
        &economics,
    )
    .expect_err("state mismatch must reject");

    assert!(mismatched.to_string().contains("does not match descriptor"));
}

#[test]
fn input_descriptor_and_status_order_do_not_change_review() {
    let economics = economics();

    let candidates = vec![
        candidate("service_node:eligible", "eligible-content"),
        candidate("service_node:blocked", "blocked-content"),
    ];

    let descriptors = vec![
        descriptor(
            "service_node:eligible",
            ServiceNodeEligibilityStateV1::Eligible,
            "eligible",
        ),
        descriptor(
            "service_node:blocked",
            ServiceNodeEligibilityStateV1::Blocked,
            "blocked",
        ),
    ];

    let statuses = vec![status(
        "service_node:blocked",
        ServiceNodeEligibilityStateV1::Blocked,
        ServiceNodeViolationKindV1::DenylistViolation,
        ServiceNodeAppealStatusV1::not_appealed(),
        "blocked",
    )];

    let first = compute_service_node_reward_review_with_enforcement(
        input(&economics, candidates.clone()),
        descriptors.clone(),
        statuses.clone(),
        &economics,
    )
    .expect("first review should compute");

    let second = compute_service_node_reward_review_with_enforcement(
        input(&economics, candidates.into_iter().rev().collect()),
        descriptors.into_iter().rev().collect(),
        statuses.into_iter().rev().collect(),
        &economics,
    )
    .expect("second review should compute");

    assert_eq!(first, second);
}
