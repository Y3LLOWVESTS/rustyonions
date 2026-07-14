//! RO:WHAT — Phase 19 deterministic bad-node policy tests.
//!
//! RO:WHY — Every required signal must select conservative containment and
//! reward denial without granting policy mutation authority.
//!
//! RO:INVARIANTS — bad hashes quarantine; denylist violations block; privacy
//! and invalid epoch signatures quarantine; binding abuse denies rewards;
//! repeated lower-severity behavior escalates; containment never downgrades.

#![allow(clippy::missing_panics_doc)]

use ron_policy::{
    ServiceNodeRewardReviewPostureV1, ServiceNodeViolationDecisionReasonV1,
    ServiceNodeViolationPolicyError, ServiceNodeViolationPolicyV1,
    SERVICE_NODE_VIOLATION_POLICY_VERSION,
};
use ron_proto::{
    ContentId, ServiceNodeAppealStateV1, ServiceNodeEligibilityStateV1,
    ServiceNodeIdentityDescriptorV1, ServiceNodeViolationKindV1, ServiceNodeViolationReportV1,
    SERVICE_NODE_ENFORCEMENT_VERSION, SERVICE_NODE_VIOLATION_REPORT_SCHEMA,
};
use serde_json::json;

const REGISTERED_EPOCH: u64 = 10;
const STATE_EPOCH: u64 = 12;
const DETECTED_EPOCH: u64 = 20;

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

fn policy() -> ServiceNodeViolationPolicyV1 {
    ServiceNodeViolationPolicyV1 {
        version: SERVICE_NODE_VIOLATION_POLICY_VERSION,
        repeated_violation_quarantine_count: 3,
        repeated_violation_block_count: 5,
    }
}

fn descriptor(state: ServiceNodeEligibilityStateV1) -> ServiceNodeIdentityDescriptorV1 {
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
    descriptor.state_effective_epoch = STATE_EPOCH;

    descriptor
        .validate()
        .expect("fixture descriptor should validate");

    descriptor
}

fn report(
    violation: ServiceNodeViolationKindV1,
    observed_count: u64,
) -> ServiceNodeViolationReportV1 {
    ServiceNodeViolationReportV1 {
        schema: SERVICE_NODE_VIOLATION_REPORT_SCHEMA.to_string(),
        version: SERVICE_NODE_ENFORCEMENT_VERSION,
        report_id: format!("violation:report_{observed_count:03}"),
        service_node_id: "service_node:node_01".to_string(),
        reporter_id: "user_node:verifier_01".to_string(),
        violation,
        evidence_root: cid("phase19-policy-evidence"),
        detected_epoch: DETECTED_EPOCH,
        observed_count,
    }
}

fn evaluate(
    state: ServiceNodeEligibilityStateV1,
    violation: ServiceNodeViolationKindV1,
    observed_count: u64,
) -> ron_policy::ServiceNodeViolationDecisionV1 {
    policy()
        .evaluate(&descriptor(state), &report(violation, observed_count))
        .expect("violation policy review should succeed")
}

#[test]
fn every_phase19_signal_has_reviewed_containment() {
    let cases = [
        (
            ServiceNodeViolationKindV1::HashMismatch,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::DenylistViolation,
            ServiceNodeEligibilityStateV1::Blocked,
        ),
        (
            ServiceNodeViolationKindV1::TombstoneViolation,
            ServiceNodeEligibilityStateV1::Blocked,
        ),
        (
            ServiceNodeViolationKindV1::FakeDeliveryProof,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::ProviderSpam,
            ServiceNodeEligibilityStateV1::Degraded,
        ),
        (
            ServiceNodeViolationKindV1::ReplayAttempt,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::SelfTrafficLoop,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::ChallengeFailure,
            ServiceNodeEligibilityStateV1::Degraded,
        ),
        (
            ServiceNodeViolationKindV1::InvalidEpochProposal,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::InvalidEpochSignature,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::UnilateralMintAttempt,
            ServiceNodeEligibilityStateV1::Blocked,
        ),
        (
            ServiceNodeViolationKindV1::PrivacyLeak,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
        (
            ServiceNodeViolationKindV1::RewardRecipientBindingAbuse,
            ServiceNodeEligibilityStateV1::Quarantined,
        ),
    ];

    for (violation, expected_state) in cases {
        let decision = evaluate(ServiceNodeEligibilityStateV1::Eligible, violation, 1);

        assert_eq!(decision.next_state, expected_state);
        assert_eq!(
            decision.reward_review,
            ServiceNodeRewardReviewPostureV1::Denied
        );
        assert!(!decision.counts_toward_quorum());
        assert!(decision.rewards_denied());
        assert!(!decision.authorizes_registry_mutation());
        assert!(!decision.authorizes_economic_mutation());
        assert!(!decision.authorizes_appeal_resolution());
    }
}

#[test]
fn bad_hash_provider_is_quarantined() {
    let decision = evaluate(
        ServiceNodeEligibilityStateV1::Eligible,
        ServiceNodeViolationKindV1::HashMismatch,
        1,
    );

    assert_eq!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert_eq!(
        decision.reason,
        ServiceNodeViolationDecisionReasonV1::BaselineContainment
    );
    assert!(decision.is_transition());
}

#[test]
fn denylist_violator_is_blocked() {
    let decision = evaluate(
        ServiceNodeEligibilityStateV1::Eligible,
        ServiceNodeViolationKindV1::DenylistViolation,
        1,
    );

    assert_eq!(decision.next_state, ServiceNodeEligibilityStateV1::Blocked);
    assert!(decision.rewards_denied());
}

#[test]
fn privacy_leak_and_invalid_epoch_signature_are_quarantined() {
    for violation in [
        ServiceNodeViolationKindV1::PrivacyLeak,
        ServiceNodeViolationKindV1::InvalidEpochSignature,
    ] {
        let decision = evaluate(ServiceNodeEligibilityStateV1::Eligible, violation, 1);

        assert_eq!(
            decision.next_state,
            ServiceNodeEligibilityStateV1::Quarantined
        );
        assert!(!decision.counts_toward_quorum());
    }
}

#[test]
fn reward_binding_abuse_blocks_rewards() {
    let decision = evaluate(
        ServiceNodeEligibilityStateV1::Probation,
        ServiceNodeViolationKindV1::RewardRecipientBindingAbuse,
        1,
    );

    assert_eq!(
        decision.next_state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert_eq!(
        decision.reward_review,
        ServiceNodeRewardReviewPostureV1::Denied
    );
    assert!(decision.rewards_denied());
}

#[test]
fn repeated_provider_spam_escalates_deterministically() {
    let first = evaluate(
        ServiceNodeEligibilityStateV1::Eligible,
        ServiceNodeViolationKindV1::ProviderSpam,
        1,
    );

    assert_eq!(first.next_state, ServiceNodeEligibilityStateV1::Degraded);

    let quarantined = evaluate(
        ServiceNodeEligibilityStateV1::Eligible,
        ServiceNodeViolationKindV1::ProviderSpam,
        3,
    );

    assert_eq!(
        quarantined.next_state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert_eq!(
        quarantined.reason,
        ServiceNodeViolationDecisionReasonV1::RepeatedViolationQuarantined
    );

    let blocked = evaluate(
        ServiceNodeEligibilityStateV1::Eligible,
        ServiceNodeViolationKindV1::ProviderSpam,
        5,
    );

    assert_eq!(blocked.next_state, ServiceNodeEligibilityStateV1::Blocked);
    assert_eq!(
        blocked.reason,
        ServiceNodeViolationDecisionReasonV1::RepeatedViolationBlocked
    );
}

#[test]
fn existing_stronger_containment_is_never_downgraded() {
    let decision = evaluate(
        ServiceNodeEligibilityStateV1::Blocked,
        ServiceNodeViolationKindV1::ProviderSpam,
        1,
    );

    assert_eq!(
        decision.previous_state,
        ServiceNodeEligibilityStateV1::Blocked
    );
    assert_eq!(decision.next_state, ServiceNodeEligibilityStateV1::Blocked);
    assert_eq!(
        decision.reason,
        ServiceNodeViolationDecisionReasonV1::ExistingContainmentMaintained
    );
    assert_eq!(decision.state_effective_epoch, STATE_EPOCH);
    assert!(!decision.is_transition());
}

#[test]
fn appeal_status_is_visible_but_not_resolved_by_policy() {
    let decision = evaluate(
        ServiceNodeEligibilityStateV1::Eligible,
        ServiceNodeViolationKindV1::HashMismatch,
        1,
    );

    assert_eq!(decision.appeal.state, ServiceNodeAppealStateV1::NotAppealed);
    assert!(!decision.appeal.authorizes_state_change());

    let encoded = serde_json::to_value(decision).expect("decision should serialize");

    assert_eq!(encoded["appeal"]["state"], "not_appealed");
}

#[test]
fn stale_or_mismatched_reports_are_rejected() {
    let descriptor = descriptor(ServiceNodeEligibilityStateV1::Eligible);

    let mut mismatched = report(ServiceNodeViolationKindV1::HashMismatch, 1);

    mismatched.service_node_id = "service_node:other_node".to_string();

    assert!(matches!(
        policy().evaluate(&descriptor, &mismatched),
        Err(ServiceNodeViolationPolicyError::ServiceNodeIdMismatch { .. })
    ));

    let mut stale = report(ServiceNodeViolationKindV1::HashMismatch, 1);

    stale.detected_epoch = STATE_EPOCH - 1;

    assert!(matches!(
        policy().evaluate(&descriptor, &stale),
        Err(ServiceNodeViolationPolicyError::StaleViolationReport { .. })
    ));
}

#[test]
fn unsafe_thresholds_and_founder_fields_are_rejected() {
    let invalid = ServiceNodeViolationPolicyV1 {
        version: SERVICE_NODE_VIOLATION_POLICY_VERSION,
        repeated_violation_quarantine_count: 1,
        repeated_violation_block_count: 1,
    };

    assert!(matches!(
        invalid.validate(),
        Err(ServiceNodeViolationPolicyError::InvalidPolicy { .. })
    ));

    let founder_override = json!({
        "version": SERVICE_NODE_VIOLATION_POLICY_VERSION,
        "repeated_violation_quarantine_count": 3,
        "repeated_violation_block_count": 5,
        "trusted_founder_override": true
    });

    assert!(serde_json::from_value::<ServiceNodeViolationPolicyV1>(founder_override).is_err());

    let manual_trust = json!({
        "version": SERVICE_NODE_VIOLATION_POLICY_VERSION,
        "repeated_violation_quarantine_count": 3,
        "repeated_violation_block_count": 5,
        "manual_trusted_node": "service_node:node_01"
    });

    assert!(serde_json::from_value::<ServiceNodeViolationPolicyV1>(manual_trust).is_err());
}
