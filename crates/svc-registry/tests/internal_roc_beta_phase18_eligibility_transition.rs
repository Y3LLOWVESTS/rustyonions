//! RO:WHAT — Phase 18 registry application tests for Service Node eligibility decisions.
//!
//! RO:WHY — Objective policy output must become canonical state only through root-bound,
//! replay-protected registry application.
//!
//! RO:INTERACTS — `svc-registry` eligibility/history custody, `ron-policy` lifecycle
//! evaluation, and `ron-proto` Service Node/reward-binding DTOs.
//!
//! RO:INVARIANTS — registry derives policy decisions itself; stale roots and replay reject;
//! candidate cannot skip probation; transition application grants no economic authority.

use ron_policy::{
    ServiceNodeEligibilityObservationV1, ServiceNodeEligibilityPolicyError,
    ServiceNodeEligibilityPolicyV1, ServiceNodeHistoryThresholdsV1,
    SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
};
use ron_proto::{
    ContentId, RewardBindingSignatureRefV1, ServiceNodeEligibilityStateV1,
    ServiceNodeIdentityDescriptorV1, ServiceNodeRewardBindingV1,
    SERVICE_NODE_REWARD_BINDING_VERSION,
};
use svc_registry::{
    eligibility::{
        ServiceNodeEligibilityRegistry, ServiceNodeEligibilityTransitionError,
        ServiceNodeHistoryRootsUpdate,
    },
    rewards::RewardBindingRegistry,
};

const NODE_ID: &str = "service_node:node_01";
const BINDING_ID: &str = "binding:node_01:1";
const REGISTERED_EPOCH: u64 = 18;

fn cid(byte: char) -> ContentId {
    let raw = format!("b3:{}", byte.to_string().repeat(64));
    ContentId::parse(&raw).expect("fixture CID should parse")
}

fn signature(label: &str) -> RewardBindingSignatureRefV1 {
    RewardBindingSignatureRefV1 {
        alg: "ed25519".to_string(),
        public_key_ref: format!("key:{label}"),
        signature: format!("sig-{label}-bytes"),
    }
}

fn reward_binding() -> ServiceNodeRewardBindingV1 {
    ServiceNodeRewardBindingV1 {
        version: SERVICE_NODE_REWARD_BINDING_VERSION,
        binding_id: BINDING_ID.to_string(),
        service_node_id: NODE_ID.to_string(),
        operator_account_id: "acct_operator".to_string(),
        operator_display_address: "@operator".to_string(),
        operator_passport_id: Some("passport:main:operator".to_string()),
        reward_recipient_account_id: "acct_operator".to_string(),
        reward_recipient_display_address: "@operator".to_string(),
        node_public_key: "node_public_key_01".to_string(),
        operator_signature: signature("operator"),
        node_signature: signature("node"),
        created_at_ms: 1_000,
        effective_epoch: REGISTERED_EPOCH,
        expires_at_ms: Some(100_000),
        rotation_nonce: "nonce_node_01_1".to_string(),
        policy_hash: cid('a'),
    }
}

fn candidate_descriptor() -> ServiceNodeIdentityDescriptorV1 {
    ServiceNodeIdentityDescriptorV1::new_candidate(
        NODE_ID.to_string(),
        "registry:node_01".to_string(),
        BINDING_ID.to_string(),
        "key:node_01".to_string(),
        REGISTERED_EPOCH,
        cid('b'),
        cid('c'),
    )
    .expect("candidate descriptor should validate")
}

fn policy() -> ServiceNodeEligibilityPolicyV1 {
    ServiceNodeEligibilityPolicyV1 {
        version: SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
        candidate_to_probation: ServiceNodeHistoryThresholdsV1 {
            minimum_state_age_epochs: 2,
            minimum_successful_service_events: 4,
            minimum_distinct_requesters: 3,
            minimum_distinct_provider_peers: 2,
        },
        probation_to_eligible: ServiceNodeHistoryThresholdsV1 {
            minimum_state_age_epochs: 3,
            minimum_successful_service_events: 10,
            minimum_distinct_requesters: 6,
            minimum_distinct_provider_peers: 3,
        },
        maximum_failure_rate_bps: 2_000,
        degrade_upheld_challenge_count: 1,
        quarantine_upheld_challenge_count: 2,
        block_upheld_challenge_count: 4,
    }
}

fn registry() -> ServiceNodeEligibilityRegistry {
    let mut rewards = RewardBindingRegistry::new(cid('d'), cid('e'));
    rewards
        .insert_binding(reward_binding())
        .expect("reward binding should insert");

    let mut registry = ServiceNodeEligibilityRegistry::new();
    registry
        .register_candidate(candidate_descriptor(), &rewards, 2_000)
        .expect("candidate should register");

    registry
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
        successful_service_events_in_state: 0,
        failed_service_events_in_state: 0,
        distinct_requesters_in_state: 0,
        distinct_provider_peers_in_state: 0,
        upheld_challenges_in_state: 0,
        rejected_challenges_in_state: 0,
        pending_challenges_in_state: 0,
    }
}

fn qualifying_candidate_observation(
    descriptor: &ServiceNodeIdentityDescriptorV1,
    evaluation_epoch: u64,
) -> ServiceNodeEligibilityObservationV1 {
    let mut observation = observation(descriptor, evaluation_epoch);
    observation.successful_service_events_in_state = 4;
    observation.distinct_requesters_in_state = 3;
    observation.distinct_provider_peers_in_state = 2;
    observation
}

fn qualifying_probation_observation(
    descriptor: &ServiceNodeIdentityDescriptorV1,
    evaluation_epoch: u64,
) -> ServiceNodeEligibilityObservationV1 {
    let mut observation = observation(descriptor, evaluation_epoch);
    observation.successful_service_events_in_state = 10;
    observation.failed_service_events_in_state = 1;
    observation.distinct_requesters_in_state = 6;
    observation.distinct_provider_peers_in_state = 3;
    observation
}

fn advance_history(
    registry: &mut ServiceNodeEligibilityRegistry,
    effective_epoch: u64,
    service_root: ContentId,
    challenge_root: ContentId,
) {
    let current = registry
        .descriptor(NODE_ID)
        .expect("descriptor should exist")
        .clone();

    registry
        .update_history_roots(ServiceNodeHistoryRootsUpdate {
            service_node_id: NODE_ID.to_string(),
            expected_service_history_root: current.service_history_root,
            service_history_root: service_root,
            expected_challenge_history_root: current.challenge_history_root,
            challenge_history_root: challenge_root,
            effective_epoch,
        })
        .expect("history roots should advance");
}

fn promote_to_eligible(registry: &mut ServiceNodeEligibilityRegistry) {
    let candidate = registry
        .descriptor(NODE_ID)
        .expect("candidate descriptor should exist")
        .clone();

    registry
        .evaluate_and_apply_policy(policy(), &qualifying_candidate_observation(&candidate, 20))
        .expect("candidate should enter probation");

    advance_history(registry, 21, cid('f'), cid('1'));

    let probation = registry
        .descriptor(NODE_ID)
        .expect("probation descriptor should exist")
        .clone();

    registry
        .evaluate_and_apply_policy(policy(), &qualifying_probation_observation(&probation, 23))
        .expect("probation should become eligible");
}

#[test]
fn qualifying_candidate_is_atomically_applied_as_probation() {
    let mut registry = registry();
    let descriptor = registry
        .descriptor(NODE_ID)
        .expect("candidate should exist")
        .clone();

    let transition = registry
        .evaluate_and_apply_policy(policy(), &qualifying_candidate_observation(&descriptor, 20))
        .expect("qualifying candidate should transition");

    assert!(transition.state_changed);
    assert_eq!(
        transition.decision.previous_state,
        ServiceNodeEligibilityStateV1::Candidate
    );
    assert_eq!(
        transition.descriptor.state,
        ServiceNodeEligibilityStateV1::Probation
    );
    assert_eq!(transition.descriptor.state_effective_epoch, 20);
    assert!(transition.requires_probation_reward_cap());
    assert!(!transition.counts_toward_quorum());
    assert!(!transition.authorizes_economic_mutation());
    assert_eq!(registry.last_policy_evaluation_epoch(NODE_ID), Some(20));
}

#[test]
fn no_op_candidate_review_is_recorded_and_replay_rejected() {
    let mut registry = registry();
    let descriptor = registry
        .descriptor(NODE_ID)
        .expect("candidate should exist")
        .clone();
    let observation = observation(&descriptor, 19);

    let transition = registry
        .evaluate_and_apply_policy(policy(), &observation)
        .expect("pending candidate review should apply");

    assert!(!transition.state_changed);
    assert_eq!(
        transition.descriptor.state,
        ServiceNodeEligibilityStateV1::Candidate
    );
    assert_eq!(registry.last_policy_evaluation_epoch(NODE_ID), Some(19));

    let error = registry
        .evaluate_and_apply_policy(policy(), &observation)
        .expect_err("same evaluation epoch must reject as replay");

    assert!(matches!(
        error,
        ServiceNodeEligibilityTransitionError::PolicyEvaluationReplay {
            evaluation_epoch: 19,
            last_applied_epoch: 19,
        }
    ));
}

#[test]
fn stale_history_root_rejects_without_mutating_registry() {
    let mut registry = registry();
    let descriptor = registry
        .descriptor(NODE_ID)
        .expect("candidate should exist")
        .clone();

    let mut stale = qualifying_candidate_observation(&descriptor, 20);
    stale.service_history_root = cid('9');

    let error = registry
        .evaluate_and_apply_policy(policy(), &stale)
        .expect_err("stale service-history root must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityTransitionError::Policy(
            ServiceNodeEligibilityPolicyError::ServiceHistoryRootMismatch
        )
    ));

    assert_eq!(
        registry
            .descriptor(NODE_ID)
            .expect("descriptor should remain")
            .state,
        ServiceNodeEligibilityStateV1::Candidate
    );
    assert_eq!(registry.last_policy_evaluation_epoch(NODE_ID), None);
}

#[test]
fn evaluation_before_latest_committed_history_epoch_rejects() {
    let mut registry = registry();

    advance_history(&mut registry, 25, cid('6'), cid('7'));

    let descriptor = registry
        .descriptor(NODE_ID)
        .expect("descriptor should exist")
        .clone();
    let observation = qualifying_candidate_observation(&descriptor, 24);

    let error = registry
        .evaluate_and_apply_policy(policy(), &observation)
        .expect_err("evaluation before committed history must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityTransitionError::EvaluationPredatesHistory {
            history_epoch: 25,
            evaluation_epoch: 24,
        }
    ));
}

#[test]
fn committed_service_history_promotes_probation_to_eligible() {
    let mut registry = registry();
    promote_to_eligible(&mut registry);

    let descriptor = registry
        .descriptor(NODE_ID)
        .expect("eligible descriptor should exist");

    assert_eq!(descriptor.state, ServiceNodeEligibilityStateV1::Eligible);
    assert_eq!(descriptor.state_effective_epoch, 23);
    assert_eq!(registry.last_policy_evaluation_epoch(NODE_ID), Some(23));
    assert!(descriptor.state.counts_toward_quorum());
}

#[test]
fn upheld_challenges_apply_quarantine_and_remove_quorum_posture() {
    let mut registry = registry();
    promote_to_eligible(&mut registry);

    advance_history(&mut registry, 24, cid('2'), cid('3'));

    let eligible = registry
        .descriptor(NODE_ID)
        .expect("eligible descriptor should exist")
        .clone();

    let mut challenged = observation(&eligible, 25);
    challenged.successful_service_events_in_state = 10;
    challenged.upheld_challenges_in_state = 2;

    let transition = registry
        .evaluate_and_apply_policy(policy(), &challenged)
        .expect("upheld challenge threshold should quarantine");

    assert_eq!(
        transition.descriptor.state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert!(!transition.counts_toward_quorum());
    assert!(!transition.requires_probation_reward_cap());
    assert!(!transition.authorizes_economic_mutation());
}

#[test]
fn invalid_policy_rejects_without_consuming_evaluation_epoch() {
    let mut registry = registry();
    let descriptor = registry
        .descriptor(NODE_ID)
        .expect("candidate should exist")
        .clone();
    let observation = qualifying_candidate_observation(&descriptor, 20);

    let mut invalid_policy = policy();
    invalid_policy.maximum_failure_rate_bps = 10_001;

    let error = registry
        .evaluate_and_apply_policy(invalid_policy, &observation)
        .expect_err("invalid policy must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityTransitionError::Policy(
            ServiceNodeEligibilityPolicyError::InvalidPolicy {
                field: "maximum_failure_rate_bps",
                ..
            }
        )
    ));

    assert_eq!(registry.last_policy_evaluation_epoch(NODE_ID), None);
    assert_eq!(
        registry
            .descriptor(NODE_ID)
            .expect("descriptor should remain")
            .state,
        ServiceNodeEligibilityStateV1::Candidate
    );
}
