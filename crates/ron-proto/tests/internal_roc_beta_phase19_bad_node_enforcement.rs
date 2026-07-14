//! RO:WHAT — Phase 19A canonical violation, containment, and appeal tests.
//!
//! RO:WHY — Bad-node evidence and operator-visible appeals must share one strict
//! protocol shape while reusing the Phase 18 lifecycle.
//!
//! RO:INVARIANTS — no second quarantine state machine; reports are evidence
//! only; contained nodes have no quorum or reward posture; private-network and
//! economic-authority fields reject.

use ron_proto::{
    ContentId, ServiceNodeAppealStateV1, ServiceNodeAppealStatusV1, ServiceNodeEligibilityStateV1,
    ServiceNodeEnforcementStatusV1, ServiceNodeEnforcementValidationError,
    ServiceNodeViolationKindV1, ServiceNodeViolationReportV1,
    SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA, SERVICE_NODE_ENFORCEMENT_VERSION,
    SERVICE_NODE_VIOLATION_REPORT_SCHEMA,
};
use serde_json::Value;

fn cid(label: &str) -> ContentId {
    format!("b3:{}", blake3::hash(label.as_bytes()).to_hex())
        .parse()
        .expect("fixture ContentId should parse")
}

fn report(violation: ServiceNodeViolationKindV1) -> ServiceNodeViolationReportV1 {
    ServiceNodeViolationReportV1 {
        schema: SERVICE_NODE_VIOLATION_REPORT_SCHEMA.to_string(),
        version: SERVICE_NODE_ENFORCEMENT_VERSION,
        report_id: "violation:report_001".to_string(),
        service_node_id: "service_node:bad_node_01".to_string(),
        reporter_id: "user_node:verifier_01".to_string(),
        violation,
        evidence_root: cid("phase19-violation-evidence"),
        detected_epoch: 19,
        observed_count: 1,
    }
}

fn status(
    state: ServiceNodeEligibilityStateV1,
    appeal: ServiceNodeAppealStatusV1,
) -> ServiceNodeEnforcementStatusV1 {
    ServiceNodeEnforcementStatusV1 {
        schema: SERVICE_NODE_ENFORCEMENT_STATUS_SCHEMA.to_string(),
        version: SERVICE_NODE_ENFORCEMENT_VERSION,
        status_id: "enforcement:status_001".to_string(),
        service_node_id: "service_node:bad_node_01".to_string(),
        state,
        reason: ServiceNodeViolationKindV1::HashMismatch,
        evidence_root: cid("phase19-containment-evidence"),
        effective_epoch: 20,
        appeal,
    }
}

#[test]
fn every_required_phase19_signal_has_stable_wire_shape() {
    let cases = [
        (ServiceNodeViolationKindV1::HashMismatch, "hash_mismatch"),
        (
            ServiceNodeViolationKindV1::DenylistViolation,
            "denylist_violation",
        ),
        (
            ServiceNodeViolationKindV1::TombstoneViolation,
            "tombstone_violation",
        ),
        (
            ServiceNodeViolationKindV1::FakeDeliveryProof,
            "fake_delivery_proof",
        ),
        (ServiceNodeViolationKindV1::ProviderSpam, "provider_spam"),
        (ServiceNodeViolationKindV1::ReplayAttempt, "replay_attempt"),
        (
            ServiceNodeViolationKindV1::SelfTrafficLoop,
            "self_traffic_loop",
        ),
        (
            ServiceNodeViolationKindV1::ChallengeFailure,
            "challenge_failure",
        ),
        (
            ServiceNodeViolationKindV1::InvalidEpochProposal,
            "invalid_epoch_proposal",
        ),
        (
            ServiceNodeViolationKindV1::InvalidEpochSignature,
            "invalid_epoch_signature",
        ),
        (
            ServiceNodeViolationKindV1::UnilateralMintAttempt,
            "unilateral_mint_attempt",
        ),
        (ServiceNodeViolationKindV1::PrivacyLeak, "privacy_leak"),
        (
            ServiceNodeViolationKindV1::RewardRecipientBindingAbuse,
            "reward_recipient_binding_abuse",
        ),
    ];

    for (signal, wire) in cases {
        assert_eq!(
            serde_json::to_value(signal).expect("signal should serialize"),
            Value::String(wire.to_string())
        );

        assert_eq!(
            serde_json::from_value::<ServiceNodeViolationKindV1>(Value::String(wire.to_string()))
                .expect("wire signal should deserialize"),
            signal
        );
    }
}

#[test]
fn violation_report_is_strict_evidence_only_shape() {
    let report = report(ServiceNodeViolationKindV1::HashMismatch);

    report.validate().expect("valid report should validate");

    assert!(!report.authorizes_containment_mutation());
    assert!(!report.authorizes_economic_mutation());

    let mut encoded = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "residential_ip",
        "socket_addr",
        "payout_amount",
        "reward_recipient_account_id",
        "wallet_mutation",
        "ledger_mutation",
        "mint_authority",
        "finality",
    ] {
        encoded
            .as_object_mut()
            .expect("report should be an object")
            .insert(forbidden.to_string(), Value::Bool(true));

        assert!(
            serde_json::from_value::<ServiceNodeViolationReportV1>(encoded.clone()).is_err(),
            "forbidden field {forbidden} must reject"
        );

        encoded
            .as_object_mut()
            .expect("report should be an object")
            .remove(forbidden);
    }
}

#[test]
fn report_requires_nonzero_epoch_and_observation_count() {
    let mut invalid_epoch = report(ServiceNodeViolationKindV1::ReplayAttempt);
    invalid_epoch.detected_epoch = 0;

    assert_eq!(
        invalid_epoch.validate(),
        Err(ServiceNodeEnforcementValidationError::InvalidField {
            field: "detected_epoch",
            reason: "must be greater than zero",
        })
    );

    let mut invalid_count = report(ServiceNodeViolationKindV1::ProviderSpam);
    invalid_count.observed_count = 0;

    assert_eq!(
        invalid_count.validate(),
        Err(ServiceNodeEnforcementValidationError::InvalidField {
            field: "observed_count",
            reason: "must be greater than zero",
        })
    );
}

#[test]
fn containment_reuses_only_phase18_lifecycle_states() {
    for allowed in [
        ServiceNodeEligibilityStateV1::Degraded,
        ServiceNodeEligibilityStateV1::Quarantined,
        ServiceNodeEligibilityStateV1::Blocked,
    ] {
        let status = status(allowed, ServiceNodeAppealStatusV1::not_appealed());

        status
            .validate()
            .expect("containment lifecycle state should validate");

        assert!(!status.counts_toward_quorum());
        assert!(!status.permits_reward_planning());
        assert!(!status.authorizes_economic_mutation());
    }

    for forbidden in [
        ServiceNodeEligibilityStateV1::Candidate,
        ServiceNodeEligibilityStateV1::Probation,
        ServiceNodeEligibilityStateV1::Eligible,
    ] {
        let invalid = status(forbidden, ServiceNodeAppealStatusV1::not_appealed());

        assert!(matches!(
            invalid.validate(),
            Err(ServiceNodeEnforcementValidationError::InvalidField { field: "state", .. })
        ));
    }
}

#[test]
fn pending_appeal_status_is_visible_and_non_authoritative() {
    let appeal = ServiceNodeAppealStatusV1 {
        state: ServiceNodeAppealStateV1::Pending,
        appeal_id: Some("appeal:node_01:0001".to_string()),
        submitted_epoch: Some(21),
        resolved_epoch: None,
        resolution_evidence_root: None,
    };

    appeal.validate().expect("pending appeal should validate");

    assert!(appeal.is_pending());
    assert!(!appeal.authorizes_state_change());

    let containment = status(ServiceNodeEligibilityStateV1::Quarantined, appeal);

    containment
        .validate()
        .expect("status with pending appeal should validate");

    let encoded = serde_json::to_value(containment).expect("status should serialize");

    assert_eq!(encoded["appeal"]["state"], "pending");
    assert_eq!(encoded["appeal"]["appeal_id"], "appeal:node_01:0001");
    assert_eq!(encoded["appeal"]["submitted_epoch"], 21);
}

#[test]
fn resolved_appeal_requires_later_epoch_and_evidence() {
    let missing_evidence = ServiceNodeAppealStatusV1 {
        state: ServiceNodeAppealStateV1::Accepted,
        appeal_id: Some("appeal:node_01:0001".to_string()),
        submitted_epoch: Some(21),
        resolved_epoch: Some(22),
        resolution_evidence_root: None,
    };

    assert!(matches!(
        missing_evidence.validate(),
        Err(ServiceNodeEnforcementValidationError::InvalidField {
            field: "appeal.resolution_evidence_root",
            ..
        })
    ));

    let reversed_epoch = ServiceNodeAppealStatusV1 {
        state: ServiceNodeAppealStateV1::Rejected,
        appeal_id: Some("appeal:node_01:0002".to_string()),
        submitted_epoch: Some(22),
        resolved_epoch: Some(22),
        resolution_evidence_root: Some(cid("appeal-resolution")),
    };

    assert!(matches!(
        reversed_epoch.validate(),
        Err(ServiceNodeEnforcementValidationError::InvalidField {
            field: "appeal.resolved_epoch",
            ..
        })
    ));
}

#[test]
fn accepted_appeal_does_not_silently_restore_eligibility() {
    let appeal = ServiceNodeAppealStatusV1 {
        state: ServiceNodeAppealStateV1::Accepted,
        appeal_id: Some("appeal:node_01:0003".to_string()),
        submitted_epoch: Some(21),
        resolved_epoch: Some(23),
        resolution_evidence_root: Some(cid("accepted-appeal-review")),
    };

    let containment = status(ServiceNodeEligibilityStateV1::Quarantined, appeal);

    containment
        .validate()
        .expect("accepted appeal remains visible on containment status");

    assert_eq!(
        containment.state,
        ServiceNodeEligibilityStateV1::Quarantined
    );
    assert!(!containment.counts_toward_quorum());
    assert!(!containment.permits_reward_planning());
}

#[test]
fn signal_classification_helpers_are_conservative() {
    assert!(ServiceNodeViolationKindV1::PrivacyLeak.is_privacy_violation());

    assert!(ServiceNodeViolationKindV1::InvalidEpochSignature.is_epoch_or_issuance_violation());

    assert!(ServiceNodeViolationKindV1::UnilateralMintAttempt.is_epoch_or_issuance_violation());

    assert!(ServiceNodeViolationKindV1::RewardRecipientBindingAbuse.is_reward_path_violation());

    assert!(ServiceNodeViolationKindV1::FakeDeliveryProof.is_reward_path_violation());

    assert!(!ServiceNodeViolationKindV1::ProviderSpam.is_reward_path_violation());
}
