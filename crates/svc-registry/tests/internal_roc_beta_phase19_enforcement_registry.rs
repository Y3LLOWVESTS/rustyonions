//! RO:WHAT — Phase 19 registry containment, replay, and appeal tests.
//!
//! RO:WHY — Policy decisions must become canonical lifecycle state exactly once,
//! while evidence and pending appeals remain visible.
//!
//! RO:INVARIANTS — reports/evidence/epochs do not replay; containment does not
//! downgrade; appeals do not restore quorum or rewards; no economic authority.

#![allow(clippy::missing_panics_doc)]

use ron_policy::{
    ServiceNodeEligibilityObservationV1, ServiceNodeEligibilityPolicyV1,
    ServiceNodeHistoryThresholdsV1, ServiceNodeViolationPolicyV1,
    SERVICE_NODE_ELIGIBILITY_POLICY_VERSION, SERVICE_NODE_VIOLATION_POLICY_VERSION,
};
use ron_proto::{
    ContentId, RewardBindingSignatureRefV1, ServiceNodeAppealStateV1,
    ServiceNodeEligibilityStateV1, ServiceNodeIdentityDescriptorV1, ServiceNodeRewardBindingV1,
    ServiceNodeViolationKindV1, ServiceNodeViolationReportV1, SERVICE_NODE_ENFORCEMENT_VERSION,
    SERVICE_NODE_REWARD_BINDING_VERSION, SERVICE_NODE_VIOLATION_REPORT_SCHEMA,
};
use svc_registry::{
    eligibility::{ServiceNodeEligibilityRegistry, ServiceNodeHistoryRootsUpdate},
    enforcement::{ServiceNodeEnforcementRegistry, ServiceNodeEnforcementRegistryError},
    rewards::RewardBindingRegistry,
};

use svc_rewarder::{
    core::{
        compute_service_node_reward_review_with_enforcement, AmountMinor,
        ServiceNodeRewardCandidate, ServiceNodeRewardEvidenceClass, ServiceNodeRewardPlanInput,
    },
    inputs::{load_internal_roc_planning_economics_toml, InternalRocRewardPlanningEconomics},
};

const ECONOMICS: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

const NODE_ID: &str = "service_node:node_01";
const REGISTERED_EPOCH: u64 = 10;
const ELIGIBLE_EPOCH: u64 = 13;

fn cid(label: &str) -> ContentId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in label.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("b3:{hash:016x}{hash:016x}{hash:016x}{hash:016x}")
        .parse()
        .expect("fixture ContentId should parse")
}

fn signature(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_string(),
        public_key_ref: format!("key:{label}"),
        signature: format!("signature-{label}"),
    }
}

fn reward_binding() -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: "binding:node_01:1".to_string(),
        service_node_id: NODE_ID.to_string(),
        operator_account_id: "acct_operator".to_string(),
        operator_display_address: "@operator".to_string(),
        operator_passport_id: Some("passport:main:operator".to_string()),
        reward_recipient_account_id: "acct_operator".to_string(),
        reward_recipient_display_address: "@operator".to_string(),
        node_public_key: "key:node_01".to_string(),
        operator_signature: signature("operator"),
        node_signature: signature("node_01"),
        created_at_ms: 1_000,
        effective_epoch: REGISTERED_EPOCH,
        expires_at_ms: None,
        rotation_nonce: "nonce_node_01_1".to_string(),
        policy_hash: cid("reward-binding-policy"),
    }
}

fn candidate_descriptor() -> ServiceNodeIdentityDescriptorV1 {
    ServiceNodeIdentityDescriptorV1::new_candidate(
        NODE_ID.to_string(),
        "registry:node_01".to_string(),
        "binding:node_01:1".to_string(),
        "key:node_01".to_string(),
        REGISTERED_EPOCH,
        cid("service-history-0"),
        cid("challenge-history-0"),
    )
    .expect("candidate descriptor should validate")
}

fn lifecycle_policy() -> ServiceNodeEligibilityPolicyV1 {
    ServiceNodeEligibilityPolicyV1 {
        version: SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
        candidate_to_probation: ServiceNodeHistoryThresholdsV1 {
            minimum_state_age_epochs: 1,
            minimum_successful_service_events: 1,
            minimum_distinct_requesters: 1,
            minimum_distinct_provider_peers: 1,
        },
        probation_to_eligible: ServiceNodeHistoryThresholdsV1 {
            minimum_state_age_epochs: 1,
            minimum_successful_service_events: 2,
            minimum_distinct_requesters: 2,
            minimum_distinct_provider_peers: 2,
        },
        maximum_failure_rate_bps: 2_000,
        degrade_upheld_challenge_count: 1,
        quarantine_upheld_challenge_count: 2,
        block_upheld_challenge_count: 4,
    }
}

fn observation(
    descriptor: &ServiceNodeIdentityDescriptorV1,
    evaluation_epoch: u64,
) -> ServiceNodeEligibilityObservationV1 {
    ServiceNodeEligibilityObservationV1 {
        service_node_id: descriptor.service_node_id.clone(),
        evaluation_epoch,
        service_history_root: descriptor.service_history_root.clone(),
        challenge_history_root: descriptor.challenge_history_root.clone(),
        successful_service_events_in_state: 4,
        failed_service_events_in_state: 0,
        distinct_requesters_in_state: 3,
        distinct_provider_peers_in_state: 2,
        upheld_challenges_in_state: 0,
        rejected_challenges_in_state: 0,
        pending_challenges_in_state: 0,
    }
}

fn eligible_registry() -> ServiceNodeEligibilityRegistry {
    let mut bindings = RewardBindingRegistry::new(cid("registry-root"), cid("binding-root"));

    bindings
        .insert_binding(reward_binding())
        .expect("reward binding should insert");

    let mut registry = ServiceNodeEligibilityRegistry::new();

    registry
        .register_candidate(candidate_descriptor(), &bindings, 2_000)
        .expect("candidate should register");

    let candidate = registry
        .descriptor(NODE_ID)
        .expect("candidate should exist")
        .clone();

    registry
        .evaluate_and_apply_policy(lifecycle_policy(), &observation(&candidate, 11))
        .expect("candidate should enter probation");

    let probation = registry
        .descriptor(NODE_ID)
        .expect("probation should exist")
        .clone();

    registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: NODE_ID.to_string(),
            expected_service_history_root: probation.service_history_root,
            service_history_root: cid("service-history-1"),
            expected_challenge_history_root: probation.challenge_history_root,
            challenge_history_root: cid("challenge-history-1"),
            effective_epoch: 12,
        })
        .expect("history roots should advance");

    let probation = registry
        .descriptor(NODE_ID)
        .expect("probation should remain")
        .clone();

    registry
        .evaluate_and_apply_policy(lifecycle_policy(), &observation(&probation, ELIGIBLE_EPOCH))
        .expect("probation should become eligible");

    registry
}

fn violation_policy() -> ServiceNodeViolationPolicyV1 {
    ServiceNodeViolationPolicyV1 {
        version: SERVICE_NODE_VIOLATION_POLICY_VERSION,
        repeated_violation_quarantine_count: 3,
        repeated_violation_block_count: 5,
    }
}

fn report(
    report_id: &str,
    evidence_label: &str,
    violation: ServiceNodeViolationKindV1,
    detected_epoch: u64,
    observed_count: u64,
) -> ServiceNodeViolationReportV1 {
    ServiceNodeViolationReportV1 {
        schema: SERVICE_NODE_VIOLATION_REPORT_SCHEMA.to_string(),
        version: SERVICE_NODE_ENFORCEMENT_VERSION,
        report_id: report_id.to_string(),
        service_node_id: NODE_ID.to_string(),
        reporter_id: "user_node:verifier_01".to_string(),
        violation,
        evidence_root: cid(evidence_label),
        detected_epoch,
        observed_count,
    }
}

fn reward_economics() -> InternalRocRewardPlanningEconomics {
    load_internal_roc_planning_economics_toml(ECONOMICS).expect("canonical economics should load")
}

fn reward_candidate(service_node_id: &str, content_label: &str) -> ServiceNodeRewardCandidate {
    ServiceNodeRewardCandidate {
        service_node_id: service_node_id.to_string(),
        evidence_class: ServiceNodeRewardEvidenceClass::Delivery,
        content_id: cid(content_label).to_string(),
        evidence_count: 1,
        eligible_score: 100,
        evidence_verified: true,
        accounting_accepted: true,
        policy_gate_passed: true,
        challenge_required: false,
        challenge_accepted: false,
    }
}

fn reward_input(
    economics: &InternalRocRewardPlanningEconomics,
    service_node_id: &str,
    content_label: &str,
) -> ServiceNodeRewardPlanInput {
    ServiceNodeRewardPlanInput {
        epoch_id: "epoch:phase19:cross-crate-containment".to_string(),
        accounting_snapshot_cid: cid("phase19-cross-crate-accounting").to_string(),
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: cid("phase19-cross-crate-policy").to_string(),
        pool_minor_units: AmountMinor(1_000_000),
        candidates: vec![reward_candidate(service_node_id, content_label)],
    }
}

#[test]
fn bad_hash_report_atomically_quarantines_canonical_descriptor() {
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let transition = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:hash_001",
                "hash-evidence-001",
                ServiceNodeViolationKindV1::HashMismatch,
                20,
                1,
            ),
        )
        .expect("hash violation should apply");

    assert!(transition.state_changed);
    assert!(transition.status_changed);
    assert_eq!(
        transition.descriptor.state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert_eq!(
        transition.status.state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert_eq!(
        transition.status.reason,
        ServiceNodeViolationKindV1::HashMismatch
    );
    assert_eq!(
        transition.status.appeal.state,
        ServiceNodeAppealStateV1::NotAppealed
    );
    assert!(!transition.counts_toward_quorum());
    assert!(transition.rewards_denied());
    assert!(!transition.authorizes_economic_mutation());
    assert!(!transition.authorizes_appeal_resolution());

    assert_eq!(
        eligibility
            .descriptor(NODE_ID)
            .expect("descriptor should remain")
            .state,
        ServiceNodeEligibilityStateV1::Quarantined
    );

    assert!(enforcement.report("violation:hash_001").is_some());

    assert_eq!(enforcement.report_count(), 1);
}

#[test]
fn denylist_violation_blocks_node_immediately() {
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let transition = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:denylist_001",
                "denylist-evidence-001",
                ServiceNodeViolationKindV1::DenylistViolation,
                20,
                1,
            ),
        )
        .expect("denylist violation should apply");

    assert_eq!(
        transition.descriptor.state,
        ServiceNodeEligibilityStateV1::Blocked
    );
    assert!(!transition.counts_toward_quorum());
    assert!(transition.rewards_denied());
}

#[test]
fn report_id_evidence_and_node_epoch_replays_are_rejected() {
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let accepted = report(
        "violation:hash_001",
        "hash-evidence-001",
        ServiceNodeViolationKindV1::HashMismatch,
        20,
        1,
    );

    enforcement
        .evaluate_and_apply_violation(&mut eligibility, violation_policy(), accepted.clone())
        .expect("first report should apply");

    let descriptor_before = eligibility
        .descriptor(NODE_ID)
        .expect("descriptor should exist")
        .clone();

    let status_before = enforcement
        .status(NODE_ID)
        .expect("status should exist")
        .clone();

    let report_replay = enforcement
        .evaluate_and_apply_violation(&mut eligibility, violation_policy(), accepted)
        .expect_err("report ID replay must reject");

    assert!(matches!(
        report_replay,
        ServiceNodeEnforcementRegistryError::ReportReplay { .. }
    ));

    let evidence_replay = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:hash_002",
                "hash-evidence-001",
                ServiceNodeViolationKindV1::HashMismatch,
                21,
                1,
            ),
        )
        .expect_err("evidence root replay must reject");

    assert!(matches!(
        evidence_replay,
        ServiceNodeEnforcementRegistryError::EvidenceReplay { .. }
    ));

    let epoch_replay = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:spam_001",
                "spam-evidence-001",
                ServiceNodeViolationKindV1::ProviderSpam,
                20,
                1,
            ),
        )
        .expect_err("same node epoch must reject");

    assert!(matches!(
        epoch_replay,
        ServiceNodeEnforcementRegistryError::ViolationEpochReplay {
            detected_epoch: 20,
            last_applied_epoch: 20,
            ..
        }
    ));

    assert_eq!(
        eligibility
            .descriptor(NODE_ID)
            .expect("descriptor should remain"),
        &descriptor_before
    );

    assert_eq!(
        enforcement.status(NODE_ID).expect("status should remain"),
        &status_before
    );

    assert_eq!(enforcement.report_count(), 1);
}

#[test]
fn rejected_policy_input_does_not_consume_report_identity() {
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let mut mismatched = report(
        "violation:recoverable_001",
        "recoverable-evidence-001",
        ServiceNodeViolationKindV1::HashMismatch,
        20,
        1,
    );

    mismatched.service_node_id = "service_node:other_node".to_string();

    let rejected = enforcement
        .evaluate_and_apply_violation(&mut eligibility, violation_policy(), mismatched)
        .expect_err("missing descriptor must reject");

    assert!(matches!(
        rejected,
        ServiceNodeEnforcementRegistryError::DescriptorNotFound { .. }
    ));

    assert_eq!(enforcement.report_count(), 0);

    enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:recoverable_001",
                "recoverable-evidence-001",
                ServiceNodeViolationKindV1::HashMismatch,
                20,
                1,
            ),
        )
        .expect("corrected report should not look replayed");

    assert_eq!(enforcement.report_count(), 1);
}

#[test]
fn later_weaker_report_cannot_downgrade_or_hide_block_reason() {
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let blocked = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:denylist_001",
                "denylist-evidence-001",
                ServiceNodeViolationKindV1::DenylistViolation,
                20,
                1,
            ),
        )
        .expect("denylist violation should block");

    let original_status_id = blocked.status.status_id;

    let maintained = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:spam_001",
                "spam-evidence-001",
                ServiceNodeViolationKindV1::ProviderSpam,
                21,
                1,
            ),
        )
        .expect("later report should be recorded");

    assert!(!maintained.state_changed);
    assert!(!maintained.status_changed);
    assert_eq!(
        maintained.descriptor.state,
        ServiceNodeEligibilityStateV1::Blocked
    );
    assert_eq!(
        maintained.status.reason,
        ServiceNodeViolationKindV1::DenylistViolation
    );
    assert_eq!(maintained.status.status_id, original_status_id);
    assert_eq!(enforcement.report_count(), 2);
}

#[test]
fn pending_appeal_is_visible_but_does_not_restore_eligibility() {
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:privacy_001",
                "privacy-evidence-001",
                ServiceNodeViolationKindV1::PrivacyLeak,
                20,
                1,
            ),
        )
        .expect("privacy violation should quarantine");

    let appealed = enforcement
        .submit_appeal(NODE_ID, "appeal:node_01:0001".to_string(), 21)
        .expect("pending appeal should be recorded");

    assert_eq!(appealed.appeal.state, ServiceNodeAppealStateV1::Pending);
    assert_eq!(
        appealed.appeal.appeal_id.as_deref(),
        Some("appeal:node_01:0001")
    );
    assert!(!appealed.counts_toward_quorum());
    assert!(!appealed.permits_reward_planning());
    assert!(!appealed.authorizes_economic_mutation());

    assert_eq!(
        eligibility
            .descriptor(NODE_ID)
            .expect("descriptor should remain")
            .state,
        ServiceNodeEligibilityStateV1::Quarantined
    );

    let replay = enforcement
        .submit_appeal(NODE_ID, "appeal:node_01:0001".to_string(), 22)
        .expect_err("appeal ID replay must reject");

    assert!(matches!(
        replay,
        ServiceNodeEnforcementRegistryError::AppealIdReplay { .. }
    ));

    let second = enforcement
        .submit_appeal(NODE_ID, "appeal:node_01:0002".to_string(), 22)
        .expect_err("second pending appeal must reject");

    assert!(matches!(
        second,
        ServiceNodeEnforcementRegistryError::AppealAlreadyExists {
            state: ServiceNodeAppealStateV1::Pending,
            ..
        }
    ));
}

#[test]
fn appeal_requires_existing_containment_and_valid_epoch() {
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let missing = enforcement
        .submit_appeal(NODE_ID, "appeal:missing:0001".to_string(), 20)
        .expect_err("appeal without containment must reject");

    assert!(matches!(
        missing,
        ServiceNodeEnforcementRegistryError::EnforcementStatusNotFound { .. }
    ));

    let mut eligibility = eligible_registry();

    enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:hash_001",
                "hash-evidence-001",
                ServiceNodeViolationKindV1::HashMismatch,
                20,
                1,
            ),
        )
        .expect("hash violation should quarantine");

    let stale = enforcement
        .submit_appeal(NODE_ID, "appeal:stale:0001".to_string(), 19)
        .expect_err("appeal before containment must reject");

    assert!(matches!(
        stale,
        ServiceNodeEnforcementRegistryError::AppealPredatesContainment {
            submitted_epoch: 19,
            containment_epoch: 20,
        }
    ));
}

#[test]
fn phase19_cross_crate_containment_denies_hash_and_epoch_signer_rewards() {
    let economics = reward_economics();

    for (violation, report_id, evidence_label, content_label) in [
        (
            ServiceNodeViolationKindV1::HashMismatch,
            "violation:cross_hash_001",
            "cross-hash-evidence",
            "cross-hash-content",
        ),
        (
            ServiceNodeViolationKindV1::InvalidEpochSignature,
            "violation:cross_epoch_signature_001",
            "cross-epoch-signature-evidence",
            "cross-epoch-signature-content",
        ),
    ] {
        let mut eligibility = eligible_registry();
        let mut enforcement = ServiceNodeEnforcementRegistry::new();

        let transition = enforcement
            .evaluate_and_apply_violation(
                &mut eligibility,
                violation_policy(),
                report(report_id, evidence_label, violation, 20, 1),
            )
            .expect("violation should apply through policy and registry");

        assert_eq!(
            transition.descriptor.state,
            ServiceNodeEligibilityStateV1::Quarantined
        );
        assert!(!transition.descriptor.state.counts_toward_quorum());

        let review = compute_service_node_reward_review_with_enforcement(
            reward_input(&economics, NODE_ID, content_label),
            vec![transition.descriptor],
            vec![transition.status],
            &economics,
        )
        .expect("contained node should produce denial review");

        assert!(review.all_candidates_denied());
        assert!(review.plan.is_none());
        assert_eq!(review.denied_service_nodes.len(), 1);
        assert_eq!(
            review.denied_service_nodes[0].state,
            ServiceNodeEligibilityStateV1::Quarantined
        );
        assert_eq!(review.denied_service_nodes[0].violation, violation);
        assert!(!review.denied_service_nodes[0].permits_reward_planning());
        assert!(!review.authorizes_economic_mutation());
    }
}

#[test]
fn phase19_cross_crate_denylist_block_produces_no_reward_plan() {
    let economics = reward_economics();
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    let transition = enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:cross_denylist_001",
                "cross-denylist-evidence",
                ServiceNodeViolationKindV1::DenylistViolation,
                20,
                1,
            ),
        )
        .expect("denylist violation should block");

    assert_eq!(
        transition.descriptor.state,
        ServiceNodeEligibilityStateV1::Blocked
    );
    assert!(!transition.descriptor.state.counts_toward_quorum());

    let review = compute_service_node_reward_review_with_enforcement(
        reward_input(&economics, NODE_ID, "cross-denylist-content"),
        vec![transition.descriptor],
        vec![transition.status],
        &economics,
    )
    .expect("blocked node should produce denial-only review");

    assert!(review.all_candidates_denied());
    assert!(review.plan.is_none());
    assert_eq!(
        review.denied_service_nodes[0].state,
        ServiceNodeEligibilityStateV1::Blocked
    );
    assert_eq!(
        review.denied_service_nodes[0].violation,
        ServiceNodeViolationKindV1::DenylistViolation
    );
    assert!(!review.denied_service_nodes[0].permits_reward_planning());
}

#[test]
fn phase19_cross_crate_pending_appeal_stays_visible_and_reward_denied() {
    let economics = reward_economics();
    let mut eligibility = eligible_registry();
    let mut enforcement = ServiceNodeEnforcementRegistry::new();

    enforcement
        .evaluate_and_apply_violation(
            &mut eligibility,
            violation_policy(),
            report(
                "violation:cross_privacy_001",
                "cross-privacy-evidence",
                ServiceNodeViolationKindV1::PrivacyLeak,
                20,
                1,
            ),
        )
        .expect("privacy violation should quarantine");

    let appealed_status = enforcement
        .submit_appeal(NODE_ID, "appeal:cross_privacy:0001".to_string(), 21)
        .expect("pending appeal should be visible");

    let descriptor = eligibility
        .descriptor(NODE_ID)
        .expect("contained descriptor should remain")
        .clone();

    assert_eq!(descriptor.state, ServiceNodeEligibilityStateV1::Quarantined);
    assert_eq!(
        appealed_status.appeal.state,
        ServiceNodeAppealStateV1::Pending
    );

    let review = compute_service_node_reward_review_with_enforcement(
        reward_input(&economics, NODE_ID, "cross-privacy-content"),
        vec![descriptor],
        vec![appealed_status],
        &economics,
    )
    .expect("pending appeal must remain reward-denied");

    assert!(review.all_candidates_denied());
    assert!(review.plan.is_none());
    assert_eq!(
        review.denied_service_nodes[0].appeal.state,
        ServiceNodeAppealStateV1::Pending
    );
    assert_eq!(
        review.denied_service_nodes[0].violation,
        ServiceNodeViolationKindV1::PrivacyLeak
    );
    assert!(!review.denied_service_nodes[0].permits_reward_planning());
    assert!(!review.authorizes_economic_mutation());
}
