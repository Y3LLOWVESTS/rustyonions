//! RO:WHAT — Phase 18A tests for the canonical Service Node identity and protocol-earned eligibility lifecycle.
//! RO:WHY — Proves fresh, probationary, degraded, quarantined, and blocked identities cannot masquerade as quorum-eligible nodes.
//! RO:INTERACTS — ron-proto service_node eligibility and existing Phase 15 quorum DTOs.
//! RO:INVARIANTS — no manual trust field; probation cap amount remains external config; DTO validation grants no economic authority.
//! RO:TEST — this file.

use ron_proto::{
    quantum::SignatureAlg, ContentId, EpochEligibilityStatusV1, EpochQuorumThresholdV1,
    ServiceNodeEligibilityStateV1, ServiceNodeEligibilityValidationError,
    ServiceNodeIdentityDescriptorV1, ServiceNodeQuorumV1, ServiceNodeQuorumValidationError,
    ServiceNodeSignatureV1, SERVICE_NODE_ELIGIBILITY_VERSION,
    SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA, SERVICE_NODE_QUORUM_SCHEMA,
    SERVICE_NODE_QUORUM_VERSION,
};
use serde_json::Value;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0018";

fn cid(label: &str) -> ContentId {
    format!("b3:{}", blake3::hash(label.as_bytes()).to_hex())
        .parse()
        .expect("fixture CID should parse")
}

fn descriptor(index: u8, state: ServiceNodeEligibilityStateV1) -> ServiceNodeIdentityDescriptorV1 {
    ServiceNodeIdentityDescriptorV1 {
        schema: SERVICE_NODE_IDENTITY_DESCRIPTOR_SCHEMA.to_string(),
        version: SERVICE_NODE_ELIGIBILITY_VERSION,
        service_node_id: format!("service_node:node_{index:02}"),
        registry_entry_id: format!("registry:node_{index:02}"),
        reward_binding_id: format!("binding:node_{index:02}"),
        key_id: format!("key:node_{index:02}"),
        registered_at_epoch: 18,
        state,
        state_effective_epoch: 18,
        service_history_root: cid(&format!("service-history-{index:02}")),
        challenge_history_root: cid(&format!("challenge-history-{index:02}")),
    }
}

fn signature(index: u8, transition_hash: &ContentId) -> ServiceNodeSignatureV1 {
    ServiceNodeSignatureV1 {
        version: SERVICE_NODE_QUORUM_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        service_node_id: format!("service_node:node_{index:02}"),
        key_id: format!("key:node_{index:02}"),
        algorithm: SignatureAlg::Ed25519,
        transition_hash: transition_hash.clone(),
        signature_wire: format!("test-signature-node-{index:02}"),
    }
}

#[test]
fn newly_registered_service_node_starts_as_candidate() {
    let descriptor = ServiceNodeIdentityDescriptorV1::new_candidate(
        "service_node:node_01".to_string(),
        "registry:node_01".to_string(),
        "binding:node_01".to_string(),
        "key:node_01".to_string(),
        18,
        cid("service-history-new"),
        cid("challenge-history-new"),
    )
    .expect("valid new descriptor should validate");

    assert_eq!(descriptor.state, ServiceNodeEligibilityStateV1::Candidate);
    assert_eq!(
        descriptor.state_effective_epoch,
        descriptor.registered_at_epoch
    );
    assert!(!descriptor.state.counts_toward_quorum());
}

#[test]
fn only_protocol_eligible_state_projects_into_epoch_quorum() {
    let states = [
        (
            ServiceNodeEligibilityStateV1::Candidate,
            EpochEligibilityStatusV1::Ineligible,
        ),
        (
            ServiceNodeEligibilityStateV1::Probation,
            EpochEligibilityStatusV1::Ineligible,
        ),
        (
            ServiceNodeEligibilityStateV1::Eligible,
            EpochEligibilityStatusV1::Eligible,
        ),
        (
            ServiceNodeEligibilityStateV1::Degraded,
            EpochEligibilityStatusV1::Ineligible,
        ),
        (
            ServiceNodeEligibilityStateV1::Quarantined,
            EpochEligibilityStatusV1::Ineligible,
        ),
        (
            ServiceNodeEligibilityStateV1::Blocked,
            EpochEligibilityStatusV1::Ineligible,
        ),
    ];

    for (index, (state, expected)) in states.into_iter().enumerate() {
        let projected = descriptor(index as u8 + 1, state)
            .to_epoch_eligibility()
            .expect("valid descriptor should project");

        assert_eq!(projected.status, expected);
    }
}

#[test]
fn candidate_cannot_control_existing_quorum_validation() {
    let transition_hash = cid("phase18-transition");

    let candidate = descriptor(1, ServiceNodeEligibilityStateV1::Candidate)
        .to_epoch_eligibility()
        .expect("candidate descriptor should project as ineligible");

    let eligible_two = descriptor(2, ServiceNodeEligibilityStateV1::Eligible)
        .to_epoch_eligibility()
        .expect("eligible descriptor should project");

    let eligible_three = descriptor(3, ServiceNodeEligibilityStateV1::Eligible)
        .to_epoch_eligibility()
        .expect("eligible descriptor should project");

    let quorum = ServiceNodeQuorumV1 {
        schema: SERVICE_NODE_QUORUM_SCHEMA.to_string(),
        version: SERVICE_NODE_QUORUM_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        transition_hash: transition_hash.clone(),
        threshold: EpochQuorumThresholdV1 {
            version: SERVICE_NODE_QUORUM_VERSION,
            eligible_service_nodes: 3,
            quorum_bps: 6_666,
            minimum_signatures: 2,
            required_signatures: 2,
        },
        eligibilities: vec![candidate, eligible_two, eligible_three],
        signatures: vec![
            signature(1, &transition_hash),
            signature(2, &transition_hash),
        ],
    };

    let error = quorum
        .validate()
        .expect_err("candidate quorum member must reject");

    assert!(matches!(
        error,
        ServiceNodeQuorumValidationError::InvalidField {
            field: "eligibilities.status",
            ..
        }
    ));
}

#[test]
fn probation_state_requires_external_reward_cap_without_defining_amount() {
    assert!(ServiceNodeEligibilityStateV1::Probation.requires_probation_reward_cap());
    assert!(!ServiceNodeEligibilityStateV1::Eligible.requires_probation_reward_cap());
}

#[test]
fn quarantined_node_projects_as_ineligible_for_quorum() {
    let projected = descriptor(1, ServiceNodeEligibilityStateV1::Quarantined)
        .to_epoch_eligibility()
        .expect("quarantined descriptor should project as ineligible");

    assert_eq!(projected.status, EpochEligibilityStatusV1::Ineligible);
}

#[test]
fn descriptor_rejects_state_epoch_before_registration() {
    let mut descriptor = descriptor(1, ServiceNodeEligibilityStateV1::Candidate);
    descriptor.registered_at_epoch = 19;
    descriptor.state_effective_epoch = 18;

    assert_eq!(
        descriptor.validate(),
        Err(ServiceNodeEligibilityValidationError::InvalidField {
            field: "state_effective_epoch",
            reason: "must not precede registered_at_epoch",
        })
    );
}

#[test]
fn descriptor_requires_reward_binding_reference() {
    let mut descriptor = descriptor(1, ServiceNodeEligibilityStateV1::Candidate);
    descriptor.reward_binding_id.clear();

    assert_eq!(
        descriptor.validate(),
        Err(ServiceNodeEligibilityValidationError::InvalidField {
            field: "reward_binding_id",
            reason: "must not be empty",
        })
    );
}

#[test]
fn manual_founder_or_trusted_node_flags_do_not_exist() {
    for forbidden_field in [
        "manual_trusted",
        "founder_approved",
        "trusted_node",
        "manual_quorum_weight",
    ] {
        let mut value =
            serde_json::to_value(descriptor(1, ServiceNodeEligibilityStateV1::Candidate))
                .expect("descriptor should serialize");

        value
            .as_object_mut()
            .expect("descriptor should serialize as object")
            .insert(forbidden_field.to_string(), Value::Bool(true));

        let error = serde_json::from_value::<ServiceNodeIdentityDescriptorV1>(value)
            .expect_err("manual trust fields must reject");

        assert!(error.to_string().contains("unknown field"));
    }
}
