//! RO:WHAT — Phase 13 requester acknowledgment DTO and signing-byte tests.
//! RO:WHY — Lock exact proof binding before runtime cryptographic verification.
//! RO:INTERACTS — DeliveryProofV1 and DeliveryRequesterAckV1.
//! RO:INVARIANTS — strict wire shape; exact identity/ack binding; stable bytes.
//! RO:SECURITY — no public key supplied by provider; no economic authority.
//! RO:TEST — cargo test -p ron-proto --test service_node_delivery_ack.

use ron_proto::{
    ContentId, DeliveryProofV1, DeliveryRequesterAckV1, DeliveryRequesterAckValidationError,
    DELIVERY_PROOF_SCHEMA, DELIVERY_REQUESTER_ACK_SCHEMA, SERVICE_EVIDENCE_VERSION,
};
use serde_json::json;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn content_id() -> ContentId {
    CID.parse().expect("canonical test CID")
}

fn proof() -> DeliveryProofV1 {
    DeliveryProofV1 {
        schema: DELIVERY_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "delivery:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        requester_node_id: "user_node:bravo".to_owned(),
        request_id: "request:0001".to_owned(),
        content_id: content_id(),
        provider_observation_ref: "provider:observation:0001".to_owned(),
        requester_ack_ref: "requester:ack:0001".to_owned(),
        bytes_delivered: 131_072,
        started_at_ms: 1_900_000_000_000,
        completed_at_ms: 1_900_000_000_125,
        content_verified: true,
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn acknowledgment() -> DeliveryRequesterAckV1 {
    DeliveryRequesterAckV1 {
        schema: DELIVERY_REQUESTER_ACK_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        ack_id: "requester:ack:0001".to_owned(),
        requester_node_id: "user_node:bravo".to_owned(),
        requester_key_id: "requester_key:0001".to_owned(),
        signature_hex: "00".repeat(64),
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

#[test]
fn acknowledgment_binds_exactly_to_delivery_proof() {
    let proof = proof();
    let ack = acknowledgment();

    assert_eq!(
        ack.validate_for(&proof)
            .expect("valid acknowledgment shape"),
        [0u8; 64]
    );

    let encoded = serde_json::to_value(&ack).expect("ack JSON");

    assert!(
        !encoded
            .as_object()
            .expect("ack object")
            .contains_key("public_key"),
        "provider-facing acknowledgment must not supply a trusted key"
    );
}

#[test]
fn acknowledgment_rejects_identity_and_reference_mismatch() {
    let proof = proof();

    let mut wrong_identity = acknowledgment();
    wrong_identity.requester_node_id = "user_node:charlie".to_owned();

    assert_eq!(
        wrong_identity.validate_for(&proof),
        Err(DeliveryRequesterAckValidationError::BindingMismatch {
            field: "requester_node_id",
        })
    );

    let mut wrong_ack = acknowledgment();
    wrong_ack.ack_id = "requester:ack:9999".to_owned();

    assert_eq!(
        wrong_ack.validate_for(&proof),
        Err(DeliveryRequesterAckValidationError::BindingMismatch { field: "ack_id" })
    );
}

#[test]
fn signature_encoding_is_exact_and_lowercase() {
    let proof = proof();

    let mut short = acknowledgment();
    short.signature_hex = "00".repeat(63);

    assert_eq!(
        short.validate_for(&proof),
        Err(DeliveryRequesterAckValidationError::InvalidSignatureLength { actual: 126 })
    );

    let mut uppercase = acknowledgment();
    uppercase.signature_hex = "AA".repeat(64);

    assert_eq!(
        uppercase.validate_for(&proof),
        Err(DeliveryRequesterAckValidationError::InvalidSignatureHex)
    );
}

#[test]
fn signing_bytes_are_deterministic_and_materially_bound() {
    let proof = proof();

    let first = proof.requester_ack_signing_bytes("requester_key:0001");

    let second = proof.requester_ack_signing_bytes("requester_key:0001");

    assert_eq!(first, second);

    let mut changed_request = proof.clone();
    changed_request.request_id = "request:0002".to_owned();

    assert_ne!(
        first,
        changed_request.requester_ack_signing_bytes("requester_key:0001",)
    );

    assert_ne!(
        first,
        proof.requester_ack_signing_bytes("requester_key:0002",)
    );
}

#[test]
fn acknowledgment_wire_rejects_unknown_and_authority_fields() {
    let proof = proof();

    let mut value = serde_json::to_value(acknowledgment()).expect("ack JSON");

    value
        .as_object_mut()
        .expect("ack object")
        .insert("requester_ip".to_owned(), json!("192.0.2.10"));

    assert!(serde_json::from_value::<DeliveryRequesterAckV1>(value).is_err());

    let mut authority = acknowledgment();
    authority.reward_truth = true;

    assert_eq!(
        authority.validate_for(&proof),
        Err(DeliveryRequesterAckValidationError::AuthorityBoundary {
            field: "reward_truth",
        })
    );
}
