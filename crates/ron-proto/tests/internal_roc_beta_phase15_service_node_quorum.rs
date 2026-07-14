//! RO:WHAT — Phase 15A tests for Service Node eligibility, thresholds, signatures, and quorum.
//! RO:WHY — Proves a single node cannot advance an epoch and malformed/duplicate/out-of-set signature material fails closed.
//! RO:INTERACTS — ron-proto service_node quorum DTOs.
//! RO:INVARIANTS — structural quorum is not cryptographic verification or wallet/ledger execution.
//! RO:TEST — this file.

use ron_proto::{
    quantum::SignatureAlg, ContentId, EpochEligibilityStatusV1, EpochEligibilityV1,
    EpochQuorumThresholdV1, ServiceNodeQuorumV1, ServiceNodeQuorumValidationError,
    ServiceNodeSignatureV1, SERVICE_NODE_QUORUM_SCHEMA, SERVICE_NODE_QUORUM_VERSION,
};
use serde_json::Value;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0015";

fn cid(label: &str) -> ContentId {
    format!("b3:{}", blake3::hash(label.as_bytes()).to_hex())
        .parse()
        .expect("fixture CID should parse")
}

fn eligibility(index: u8) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: SERVICE_NODE_QUORUM_VERSION,
        service_node_id: format!("service_node:node_{index:02}"),
        registry_entry_id: format!("registry:node_{index:02}"),
        reward_binding_id: format!("binding:node_{index:02}"),
        key_id: format!("key:node_{index:02}"),
        status: EpochEligibilityStatusV1::Eligible,
    }
}

fn threshold() -> EpochQuorumThresholdV1 {
    EpochQuorumThresholdV1 {
        version: SERVICE_NODE_QUORUM_VERSION,
        eligible_service_nodes: 3,
        quorum_bps: 6_666,
        minimum_signatures: 2,
        required_signatures: 2,
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

fn quorum() -> ServiceNodeQuorumV1 {
    let transition_hash = cid("epoch-transition");

    ServiceNodeQuorumV1 {
        schema: SERVICE_NODE_QUORUM_SCHEMA.to_string(),
        version: SERVICE_NODE_QUORUM_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        transition_hash: transition_hash.clone(),
        threshold: threshold(),
        eligibilities: vec![eligibility(1), eligibility(2), eligibility(3)],
        signatures: vec![
            signature(1, &transition_hash),
            signature(2, &transition_hash),
        ],
    }
}

#[test]
fn valid_service_node_quorum_reaches_required_structural_threshold() {
    quorum()
        .validate()
        .expect("valid structural quorum should validate");
}

#[test]
fn single_service_node_cannot_form_quorum() {
    let threshold = EpochQuorumThresholdV1 {
        version: SERVICE_NODE_QUORUM_VERSION,
        eligible_service_nodes: 1,
        quorum_bps: 10_000,
        minimum_signatures: 1,
        required_signatures: 1,
    };

    let error = threshold
        .validate()
        .expect_err("single-node quorum must reject");

    assert!(matches!(
        error,
        ServiceNodeQuorumValidationError::InvalidField {
            field: "eligible_service_nodes",
            ..
        }
    ));
}

#[test]
fn insufficient_signature_count_is_rejected() {
    let mut quorum = quorum();
    quorum.signatures.pop();

    assert_eq!(
        quorum.validate(),
        Err(ServiceNodeQuorumValidationError::InsufficientQuorum {
            required: 2,
            actual: 1,
        })
    );
}

#[test]
fn duplicate_service_node_signature_is_rejected() {
    let mut quorum = quorum();
    let duplicate = quorum.signatures[0].clone();
    quorum.signatures.insert(1, duplicate);

    assert_eq!(
        quorum.validate(),
        Err(ServiceNodeQuorumValidationError::Duplicate {
            field: "signatures.service_node_id",
        })
    );
}

#[test]
fn signature_from_outside_eligibility_set_is_rejected() {
    let mut quorum = quorum();
    let transition_hash = quorum.transition_hash.clone();
    quorum.signatures[1] = signature(9, &transition_hash);

    let error = quorum
        .validate()
        .expect_err("out-of-set signature must reject");

    assert!(matches!(
        error,
        ServiceNodeQuorumValidationError::InvalidField {
            field: "signatures.service_node_id",
            ..
        }
    ));
}

#[test]
fn signature_for_different_transition_is_rejected() {
    let mut quorum = quorum();
    quorum.signatures[0].transition_hash = cid("different-transition");

    assert_eq!(
        quorum.validate(),
        Err(ServiceNodeQuorumValidationError::Mismatch {
            field: "signatures.transition_hash",
        })
    );
}

#[test]
fn ineligible_member_cannot_appear_in_quorum_set() {
    let mut quorum = quorum();
    quorum.eligibilities[1].status = EpochEligibilityStatusV1::Ineligible;

    let error = quorum
        .validate()
        .expect_err("ineligible member must reject");

    assert!(matches!(
        error,
        ServiceNodeQuorumValidationError::InvalidField {
            field: "eligibilities.status",
            ..
        }
    ));
}

#[test]
fn noncanonical_signature_order_is_rejected() {
    let mut quorum = quorum();
    quorum.signatures.swap(0, 1);

    let error = quorum
        .validate()
        .expect_err("noncanonical signature order must reject");

    assert!(matches!(
        error,
        ServiceNodeQuorumValidationError::InvalidField {
            field: "signatures.service_node_id",
            ..
        }
    ));
}

#[test]
fn manual_trusted_issuer_flag_does_not_exist() {
    let mut value = serde_json::to_value(quorum()).expect("quorum should serialize");

    value
        .as_object_mut()
        .expect("quorum should serialize as object")
        .insert("manual_trusted_issuer".to_string(), Value::Bool(true));

    let error = serde_json::from_value::<ServiceNodeQuorumV1>(value)
        .expect_err("unknown trusted-issuer field must reject");

    assert!(error.to_string().contains("unknown field"));
}
