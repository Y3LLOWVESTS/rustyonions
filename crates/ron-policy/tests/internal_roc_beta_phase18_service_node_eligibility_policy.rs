//! RO:WHAT — Phase 18C tests for deterministic protocol-earned Service Node eligibility policy.
//! RO:WHY — Proves objective history, challenge escalation, staged promotion, and non-authority boundaries.
//! RO:INTERACTS — ron-policy eligibility evaluator and ron-proto canonical Service Node descriptors.
//! RO:INVARIANTS — candidate cannot skip probation; probation requires external reward cap; false/pending challenges do not punish.
//! RO:TEST — this file.

use ron_policy::{
    ServiceNodeEligibilityObservationV1, ServiceNodeEligibilityPolicyError,
    ServiceNodeEligibilityPolicyV1, ServiceNodeEligibilityReasonCodeV1,
    ServiceNodeHistoryThresholdsV1, ServiceNodeRewardReviewPostureV1,
    SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
};
use ron_proto::{ContentId, ServiceNodeEligibilityStateV1, ServiceNodeIdentityDescriptorV1};

const REGISTERED_EPOCH: u64 = 10;

fn cid(label: &str) -> ContentId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in label.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("b3:{hash:016x}{hash:016x}{hash:016x}{hash:016x}")
        .parse()
        .expect("fixture CID should parse")
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

fn descriptor(
    state: ServiceNodeEligibilityStateV1,
    state_effective_epoch: u64,
) -> ServiceNodeIdentityDescriptorV1 {
    let mut descriptor = ServiceNodeIdentityDescriptorV1::new_candidate(
        "service_node:node_01".to_string(),
        "registry:node_01".to_string(),
        "binding:node_01:1".to_string(),
        "key:node_01".to_string(),
        REGISTERED_EPOCH,
        cid("service-history"),
        cid("challenge-history"),
    )
    .expect("candidate descriptor should validate");

    descriptor.state = state;
    descriptor.state_effective_epoch = state_effective_epoch;
    descriptor
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
) -> ServiceNodeEligibilityObservationV1 {
    let mut observation = observation(descriptor, REGISTERED_EPOCH + 2);
    observation.successful_service_events_in_state = 4;
    observation.distinct_requesters_in_state = 3;
    observation.distinct_provider_peers_in_state = 2;
    observation
}

fn qualifying_probation_observation(
    descriptor: &ServiceNodeIdentityDescriptorV1,
) -> ServiceNodeEligibilityObservationV1 {
    let mut observation = observation(descriptor, descriptor.state_effective_epoch + 3);
    observation.successful_service_events_in_state = 10;
    observation.failed_service_events_in_state = 1;
    observation.distinct_requesters_in_state = 6;
    observation.distinct_provider_peers_in_state = 3;
    observation
}

#[test]
fn candidate_with_insufficient_history_remains_non_authoritative() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Candidate, REGISTERED_EPOCH);
    let observation = observation(&descriptor, REGISTERED_EPOCH + 1);

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("valid pending candidate evaluation should succeed");

    assert_eq!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Candidate
    );
    assert_eq!(
        decision.reason,
        ServiceNodeEligibilityReasonCodeV1::CandidateHistoryPending
    );
    assert_eq!(
        decision.reward_review,
        ServiceNodeRewardReviewPostureV1::Denied
    );
    assert!(!decision.counts_toward_quorum());
    assert!(!decision.authorizes_registry_mutation());
    assert!(!decision.authorizes_economic_mutation());
}

#[test]
fn qualifying_candidate_enters_probation_but_cannot_skip_to_eligible() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Candidate, REGISTERED_EPOCH);
    let observation = qualifying_candidate_observation(&descriptor);

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("qualifying candidate should enter probation");

    assert!(decision.is_transition());
    assert_eq!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Probation
    );
    assert_eq!(
        decision.reason,
        ServiceNodeEligibilityReasonCodeV1::CandidateEnteredProbation
    );
    assert_eq!(
        decision.reward_review,
        ServiceNodeRewardReviewPostureV1::ProbationCapRequired
    );
    assert!(decision.requires_probation_reward_cap());
    assert!(!decision.counts_toward_quorum());
}

#[test]
fn probation_reward_posture_requires_external_cap_without_defining_amount() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Probation, 12);
    let mut observation = observation(&descriptor, 13);
    observation.successful_service_events_in_state = 2;
    observation.distinct_requesters_in_state = 2;
    observation.distinct_provider_peers_in_state = 1;

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("pending probation evaluation should succeed");

    assert_eq!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Probation
    );
    assert_eq!(
        decision.reward_review,
        ServiceNodeRewardReviewPostureV1::ProbationCapRequired
    );
    assert!(decision.requires_probation_reward_cap());

    let encoded = serde_json::to_value(&decision).expect("decision should serialize");
    assert!(encoded.get("reward_cap_minor").is_none());
    assert!(encoded.get("reward_rate").is_none());
}

#[test]
fn qualifying_probation_history_promotes_eligibility() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Probation, 12);
    let observation = qualifying_probation_observation(&descriptor);

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("qualifying probation should promote");

    assert_eq!(decision.next_state, ServiceNodeEligibilityStateV1::Eligible);
    assert_eq!(
        decision.reason,
        ServiceNodeEligibilityReasonCodeV1::ProbationPromoted
    );
    assert_eq!(
        decision.reward_review,
        ServiceNodeRewardReviewPostureV1::StandardReview
    );
    assert!(decision.counts_toward_quorum());
    assert!(!decision.requires_probation_reward_cap());
}

#[test]
fn excessive_failure_rate_degrades_an_active_node() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Eligible, 15);
    let mut observation = observation(&descriptor, 18);
    observation.successful_service_events_in_state = 5;
    observation.failed_service_events_in_state = 5;

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("failure-rate evaluation should succeed");

    assert_eq!(decision.next_state, ServiceNodeEligibilityStateV1::Degraded);
    assert_eq!(
        decision.reason,
        ServiceNodeEligibilityReasonCodeV1::FailureRateDegraded
    );
    assert_eq!(decision.observed_failure_rate_bps, 5_000);
    assert!(!decision.counts_toward_quorum());
    assert_eq!(
        decision.reward_review,
        ServiceNodeRewardReviewPostureV1::Denied
    );
}

#[test]
fn upheld_challenges_escalate_deterministically() {
    let eligible = descriptor(ServiceNodeEligibilityStateV1::Eligible, 15);

    let mut degraded_observation = observation(&eligible, 18);
    degraded_observation.upheld_challenges_in_state = 1;

    let degraded = policy()
        .evaluate(&eligible, &degraded_observation)
        .expect("degradation evaluation should succeed");

    assert_eq!(degraded.next_state, ServiceNodeEligibilityStateV1::Degraded);
    assert_eq!(
        degraded.reason,
        ServiceNodeEligibilityReasonCodeV1::ChallengeHistoryDegraded
    );

    let mut quarantine_observation = observation(&eligible, 18);
    quarantine_observation.upheld_challenges_in_state = 2;

    let quarantined = policy()
        .evaluate(&eligible, &quarantine_observation)
        .expect("quarantine evaluation should succeed");

    assert_eq!(
        quarantined.next_state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert_eq!(
        quarantined.reason,
        ServiceNodeEligibilityReasonCodeV1::ChallengeHistoryQuarantined
    );
    assert!(!quarantined.counts_toward_quorum());

    let mut block_observation = observation(&eligible, 18);
    block_observation.upheld_challenges_in_state = 4;

    let blocked = policy()
        .evaluate(&eligible, &block_observation)
        .expect("block evaluation should succeed");

    assert_eq!(blocked.next_state, ServiceNodeEligibilityStateV1::Blocked);
    assert_eq!(
        blocked.reason,
        ServiceNodeEligibilityReasonCodeV1::ChallengeHistoryBlocked
    );
}

#[test]
fn rejected_and_pending_challenges_do_not_punish_an_honest_node() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Eligible, 15);
    let mut observation = observation(&descriptor, 18);
    observation.successful_service_events_in_state = 10;
    observation.rejected_challenges_in_state = 100;
    observation.pending_challenges_in_state = 100;

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("false and pending challenge history should remain non-punitive");

    assert_eq!(decision.next_state, ServiceNodeEligibilityStateV1::Eligible);
    assert_eq!(
        decision.reason,
        ServiceNodeEligibilityReasonCodeV1::EligibleMaintained
    );
    assert!(decision.counts_toward_quorum());
}

#[test]
fn degraded_node_recovers_only_to_probation() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Degraded, 20);
    let observation = qualifying_probation_observation(&descriptor);

    let decision = policy()
        .evaluate(&descriptor, &observation)
        .expect("objective degraded recovery should succeed");

    assert_eq!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Probation
    );
    assert_eq!(
        decision.reason,
        ServiceNodeEligibilityReasonCodeV1::DegradedReturnedToProbation
    );
    assert!(decision.requires_probation_reward_cap());
    assert!(!decision.counts_toward_quorum());
}

#[test]
fn stale_or_mismatched_history_roots_are_rejected() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Candidate, REGISTERED_EPOCH);
    let mut observation = qualifying_candidate_observation(&descriptor);
    observation.service_history_root = cid("stale-service-history");

    let error = policy()
        .evaluate(&descriptor, &observation)
        .expect_err("stale service-history root must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityPolicyError::ServiceHistoryRootMismatch
    ));

    let mut observation = qualifying_candidate_observation(&descriptor);
    observation.challenge_history_root = cid("stale-challenge-history");

    let error = policy()
        .evaluate(&descriptor, &observation)
        .expect_err("stale challenge-history root must reject");

    assert!(matches!(
        error,
        ServiceNodeEligibilityPolicyError::ChallengeHistoryRootMismatch
    ));
}

#[test]
fn unsafe_policy_thresholds_are_rejected() {
    let mut invalid = policy();
    invalid.maximum_failure_rate_bps = 10_001;

    assert!(matches!(
        invalid.validate(),
        Err(ServiceNodeEligibilityPolicyError::InvalidPolicy {
            field: "maximum_failure_rate_bps",
            ..
        })
    ));

    let mut invalid = policy();
    invalid.quarantine_upheld_challenge_count = invalid.degrade_upheld_challenge_count;

    assert!(matches!(
        invalid.validate(),
        Err(ServiceNodeEligibilityPolicyError::InvalidPolicy {
            field: "quarantine_upheld_challenge_count",
            ..
        })
    ));
}

#[test]
fn manual_founder_or_trusted_node_policy_fields_do_not_exist() {
    let mut encoded = serde_json::to_value(policy()).expect("policy should serialize");
    let object = encoded
        .as_object_mut()
        .expect("serialized policy should be an object");

    object.insert("founder_approved".to_string(), serde_json::json!(true));
    object.insert("manual_trusted".to_string(), serde_json::json!(true));
    object.insert("trusted_node".to_string(), serde_json::json!(true));

    assert!(
        serde_json::from_value::<ServiceNodeEligibilityPolicyV1>(encoded).is_err(),
        "strict policy DTO must reject manual trust fields"
    );
}
