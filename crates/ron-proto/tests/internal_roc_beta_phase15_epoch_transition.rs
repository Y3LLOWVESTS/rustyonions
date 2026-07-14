//! RO:WHAT — Phase 15 ROC epoch-transition root, cap, duplicate, and challenge tests.
//! RO:WHY — Prove that quorum-reviewed transitions cannot select unchecked economic inputs.
//! RO:INTERACTS — ron_proto::service_node epoch-transition and quorum DTOs.
//! RO:INVARIANTS — exact roots; canonical allocations; no recipient override; no unilateral mint.
//! RO:SECURITY — strict serde rejects invented recipient, authority, and trusted-issuer fields.
//! RO:TEST — cargo test -p ron-proto --test internal_roc_beta_phase15_epoch_transition.

use ron_proto::{
    service_node_signature_message_bytes, ContentId, EpochEligibilityStatusV1, EpochEligibilityV1,
    EpochQuorumThresholdV1, EpochRewardAllocationV1, InvalidEpochChallengeKindV1,
    InvalidEpochChallengeV1, RocEpochTransitionExpectationV1, RocEpochTransitionV1,
    RocEpochTransitionValidationError, ServiceNodeQuorumV1, ServiceNodeSignatureV1, SignatureAlg,
    EPOCH_REWARD_ALLOCATION_SCHEMA, INVALID_EPOCH_CHALLENGE_SCHEMA,
    ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA, ROC_EPOCH_TRANSITION_SCHEMA,
    ROC_EPOCH_TRANSITION_VERSION,
};

fn cid(character: char) -> ContentId {
    format!("b3:{}", character.to_string().repeat(64))
        .parse()
        .expect("test content id must parse")
}

fn eligibility(
    service_node_id: &str,
    registry_entry_id: &str,
    reward_binding_id: &str,
    key_id: &str,
) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: 1,
        service_node_id: service_node_id.to_owned(),
        registry_entry_id: registry_entry_id.to_owned(),
        reward_binding_id: reward_binding_id.to_owned(),
        key_id: key_id.to_owned(),
        status: EpochEligibilityStatusV1::Eligible,
    }
}

fn threshold() -> EpochQuorumThresholdV1 {
    EpochQuorumThresholdV1 {
        version: 1,
        eligible_service_nodes: 3,
        quorum_bps: 6_666,
        minimum_signatures: 2,
        required_signatures: 2,
    }
}

fn signature(
    service_node_id: &str,
    key_id: &str,
    transition_hash: &ContentId,
) -> ServiceNodeSignatureV1 {
    ServiceNodeSignatureV1 {
        version: 1,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:15".to_owned(),
        service_node_id: service_node_id.to_owned(),
        key_id: key_id.to_owned(),
        algorithm: SignatureAlg::Ed25519,
        transition_hash: transition_hash.clone(),
        signature_wire: format!("sig:{service_node_id}"),
    }
}

fn quorum(transition_hash: &ContentId) -> ServiceNodeQuorumV1 {
    let eligibilities = vec![
        eligibility(
            "service_node:alpha",
            "registry:alpha",
            "binding:alpha",
            "key:alpha",
        ),
        eligibility(
            "service_node:beta",
            "registry:beta",
            "binding:beta",
            "key:beta",
        ),
        eligibility(
            "service_node:gamma",
            "registry:gamma",
            "binding:gamma",
            "key:gamma",
        ),
    ];

    ServiceNodeQuorumV1 {
        schema: "ron.service_node.quorum.v1".to_owned(),
        version: 1,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:15".to_owned(),
        transition_hash: transition_hash.clone(),
        threshold: threshold(),
        eligibilities,
        signatures: vec![
            signature("service_node:alpha", "key:alpha", transition_hash),
            signature("service_node:beta", "key:beta", transition_hash),
        ],
    }
}

fn allocation(
    allocation_id: &str,
    reward_plan_allocation_id: &str,
    service_node_id: &str,
    amount_minor_units: &str,
) -> EpochRewardAllocationV1 {
    EpochRewardAllocationV1 {
        schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        allocation_id: allocation_id.to_owned(),
        reward_plan_allocation_id: reward_plan_allocation_id.to_owned(),
        service_node_id: service_node_id.to_owned(),
        source_pool: "node_delivery".to_owned(),
        amount_minor_units: amount_minor_units.to_owned(),
    }
}

fn valid_transition() -> RocEpochTransitionV1 {
    let transition_hash = cid('8');

    RocEpochTransitionV1 {
        schema: ROC_EPOCH_TRANSITION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:15".to_owned(),
        transition_hash: transition_hash.clone(),
        accounting_snapshot_hash: cid('1'),
        reward_plan_hash: cid('2'),
        policy_hash: cid('3'),
        economics_config_hash: cid('4'),
        registry_root: cid('5'),
        reward_binding_root: cid('6'),
        evidence_root: cid('7'),
        reward_cap_minor_units: "1000".to_owned(),
        reward_total_minor_units: "1000".to_owned(),
        allocations: vec![
            allocation(
                "allocation:alpha",
                "reward_plan_allocation:alpha",
                "service_node:alpha",
                "400",
            ),
            allocation(
                "allocation:beta",
                "reward_plan_allocation:beta",
                "service_node:beta",
                "600",
            ),
        ],
        quorum: quorum(&transition_hash),
        produced_at_ms: 1_789_000_000_000,
    }
}

fn valid_expectation(transition: &RocEpochTransitionV1) -> RocEpochTransitionExpectationV1 {
    RocEpochTransitionExpectationV1 {
        schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        chain_id: transition.chain_id.clone(),
        epoch_id: transition.epoch_id.clone(),
        accounting_snapshot_hash: transition.accounting_snapshot_hash.clone(),
        reward_plan_hash: transition.reward_plan_hash.clone(),
        policy_hash: transition.policy_hash.clone(),
        economics_config_hash: transition.economics_config_hash.clone(),
        registry_root: transition.registry_root.clone(),
        reward_binding_root: transition.reward_binding_root.clone(),
        evidence_root: transition.evidence_root.clone(),
        reward_cap_minor_units: transition.reward_cap_minor_units.clone(),
        threshold: transition.quorum.threshold.clone(),
        eligibilities: transition.quorum.eligibilities.clone(),
    }
}

#[test]
fn valid_epoch_transition_reaches_structural_quorum() {
    let transition = valid_transition();
    let expectation = valid_expectation(&transition);

    transition
        .validate_against(&expectation)
        .expect("valid transition must pass exact reviewed context");
}

#[test]
fn invalid_reward_plan_root_is_rejected() {
    let mut transition = valid_transition();
    let expectation = valid_expectation(&transition);

    transition.reward_plan_hash = cid('9');

    assert_eq!(
        transition.validate_against(&expectation),
        Err(RocEpochTransitionValidationError::Mismatch {
            field: "reward_plan_hash",
        })
    );
}

#[test]
fn invalid_policy_hash_is_rejected() {
    let mut transition = valid_transition();
    let expectation = valid_expectation(&transition);

    transition.policy_hash = cid('9');

    assert_eq!(
        transition.validate_against(&expectation),
        Err(RocEpochTransitionValidationError::Mismatch {
            field: "policy_hash",
        })
    );
}

#[test]
fn missing_economics_config_hash_is_rejected_by_strict_wire_shape() {
    let transition = valid_transition();
    let mut encoded = serde_json::to_value(transition).expect("transition must serialize");

    encoded
        .as_object_mut()
        .expect("transition must encode as object")
        .remove("economics_config_hash");

    assert!(
        serde_json::from_value::<RocEpochTransitionV1>(encoded).is_err(),
        "economics_config_hash must be required"
    );
}

#[test]
fn invalid_economics_config_hash_is_rejected() {
    let mut transition = valid_transition();
    let expectation = valid_expectation(&transition);

    transition.economics_config_hash = cid('9');

    assert_eq!(
        transition.validate_against(&expectation),
        Err(RocEpochTransitionValidationError::Mismatch {
            field: "economics_config_hash",
        })
    );
}

#[test]
fn invalid_registry_root_is_rejected() {
    let mut transition = valid_transition();
    let expectation = valid_expectation(&transition);

    transition.registry_root = cid('9');

    assert_eq!(
        transition.validate_against(&expectation),
        Err(RocEpochTransitionValidationError::Mismatch {
            field: "registry_root",
        })
    );
}

#[test]
fn invalid_reward_binding_root_is_rejected() {
    let mut transition = valid_transition();
    let expectation = valid_expectation(&transition);

    transition.reward_binding_root = cid('9');

    assert_eq!(
        transition.validate_against(&expectation),
        Err(RocEpochTransitionValidationError::Mismatch {
            field: "reward_binding_root",
        })
    );
}

#[test]
fn reward_cap_overflow_is_rejected() {
    let mut transition = valid_transition();
    transition.reward_cap_minor_units = "999".to_owned();

    assert_eq!(
        transition.validate(),
        Err(RocEpochTransitionValidationError::CapOverflow {
            cap: 999,
            total: 1000,
        })
    );
}

#[test]
fn duplicate_reward_plan_allocation_is_rejected() {
    let mut transition = valid_transition();

    transition.allocations.push(allocation(
        "allocation:duplicate",
        "reward_plan_allocation:alpha",
        "service_node:alpha",
        "1",
    ));

    assert_eq!(
        transition.validate(),
        Err(RocEpochTransitionValidationError::Duplicate {
            field: "allocations.reward_plan_allocation_id",
        })
    );
}

#[test]
fn arbitrary_recipient_account_cannot_be_added_to_transition() {
    let transition = valid_transition();
    let mut encoded = serde_json::to_value(transition).expect("transition must serialize");

    encoded
        .as_object_mut()
        .expect("transition must encode as object")
        .insert(
            "recipient_account_id".to_owned(),
            serde_json::Value::String("@attacker".to_owned()),
        );

    assert!(
        serde_json::from_value::<RocEpochTransitionV1>(encoded).is_err(),
        "Phase 15 transition must not accept a caller-selected recipient"
    );
}

#[test]
fn invalid_epoch_challenge_has_strict_non_authoritative_shape() {
    let challenge = InvalidEpochChallengeV1 {
        schema: INVALID_EPOCH_CHALLENGE_SCHEMA.to_owned(),
        version: ROC_EPOCH_TRANSITION_VERSION,
        challenge_id: "challenge:epoch:15:policy".to_owned(),
        chain_id: "rustyonions-dev".to_owned(),
        epoch_id: "epoch:15".to_owned(),
        transition_hash: cid('8'),
        challenger_id: "user_node:auditor".to_owned(),
        challenge_kind: InvalidEpochChallengeKindV1::InvalidPolicyHash,
        evidence_hash: cid('a'),
        submitted_at_ms: 1_789_000_000_001,
    };

    challenge
        .validate()
        .expect("well-formed challenge evidence must validate");

    let mut encoded = serde_json::to_value(challenge).expect("challenge must serialize");

    encoded
        .as_object_mut()
        .expect("challenge must encode as object")
        .insert(
            "challenge_accepted".to_owned(),
            serde_json::Value::Bool(true),
        );

    assert!(
        serde_json::from_value::<InvalidEpochChallengeV1>(encoded).is_err(),
        "challenge DTO must not claim its own acceptance"
    );
}

#[test]
fn service_node_signature_message_is_canonical_and_excludes_wire() {
    let transition_hash = cid('8');

    let mut first = signature("service_node:alpha", "key:alpha", &transition_hash);
    first.signature_wire = "11".repeat(64);

    let mut second = first.clone();
    second.signature_wire = "22".repeat(64);

    let first_bytes =
        service_node_signature_message_bytes(&first).expect("canonical signature message");
    let second_bytes =
        service_node_signature_message_bytes(&second).expect("canonical signature message");

    assert_eq!(first_bytes, second_bytes);

    let encoded: serde_json::Value = serde_json::from_slice(&first_bytes).expect("message JSON");

    assert_eq!(
        encoded["domain"],
        "rustyonions.service-node-epoch-signature.v1"
    );
    assert!(encoded.get("signature_wire").is_none());
}
