//! RO:WHAT — Phase 13 DeliveryProofV1 DTO and validation tests.
//! RO:WHY — Lock corroboration, replay identity, privacy, and non-authority boundaries.
//! RO:INTERACTS — ron_proto::service_node evidence DTOs.
//! RO:INVARIANTS — provider-only/self traffic reject; no IP; no reward truth.
//! RO:SECURITY — strict serde rejects requester network and payout fields.
//! RO:TEST — cargo test -p ron-proto --test service_node_delivery_evidence.

use ron_proto::{
    ContentId, DeliveryProofV1, DeliveryProofValidationError, DELIVERY_PROOF_SCHEMA,
    SERVICE_EVIDENCE_VERSION,
};
use serde_json::json;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn content_id() -> ContentId {
    CID.parse().expect("test CID must be canonical")
}

fn valid_proof() -> DeliveryProofV1 {
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

#[test]
fn valid_delivery_proof_roundtrips_as_evidence_only() {
    let proof = valid_proof();

    proof
        .validate()
        .expect("valid requester-corroborated proof");

    assert!(proof.content_verified);
    assert!(proof.evidence_only);
    assert!(!proof.reward_truth);
    assert!(!proof.payout_authority);
    assert!(!proof.wallet_mutation);
    assert!(!proof.ledger_mutation);

    let encoded = serde_json::to_value(&proof).expect("serialize proof");
    let decoded: DeliveryProofV1 =
        serde_json::from_value(encoded.clone()).expect("deserialize proof");

    assert_eq!(decoded, proof);

    let object = encoded.as_object().expect("proof object");

    for forbidden in [
        "reward_recipient_account_id",
        "reward_recipient_display_address",
        "amount_minor",
        "planned_points",
        "balance_minor",
        "receipt_txid",
        "ledger_sequence",
        "payout_executed",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "delivery proof must not expose {forbidden}"
        );
    }
}

#[test]
fn provider_only_delivery_claim_is_rejected() {
    let mut missing_ack = valid_proof();
    missing_ack.requester_ack_ref.clear();

    assert_eq!(
        missing_ack.validate(),
        Err(DeliveryProofValidationError::ProviderOnlyClaim)
    );

    let mut missing_requester = valid_proof();
    missing_requester.requester_node_id.clear();

    assert_eq!(
        missing_requester.validate(),
        Err(DeliveryProofValidationError::ProviderOnlyClaim)
    );
}

#[test]
fn self_traffic_and_collapsed_witnesses_are_rejected() {
    let mut self_traffic = valid_proof();
    self_traffic.requester_node_id = self_traffic.service_node_id.clone();

    assert_eq!(
        self_traffic.validate(),
        Err(DeliveryProofValidationError::SelfTraffic)
    );

    let mut same_reference = valid_proof();
    same_reference.requester_ack_ref = same_reference.provider_observation_ref.clone();

    assert_eq!(
        same_reference.validate(),
        Err(DeliveryProofValidationError::WitnessReferenceCollision)
    );
}

#[test]
fn requester_ip_route_and_unknown_authority_fields_reject() {
    let mut ip_actor = valid_proof();
    ip_actor.requester_node_id = "192.0.2.10".to_owned();

    assert_eq!(
        ip_actor.validate(),
        Err(DeliveryProofValidationError::InvalidToken {
            field: "requester_node_id",
        })
    );

    let encoded = serde_json::to_string(&valid_proof()).expect("serialize proof");

    for forbidden_fragment in [
        "requester_ip",
        "residential_ip",
        "socket_addr",
        "remote_addr",
        "tcp://",
        "http://",
        "crab://",
    ] {
        assert!(
            !encoded.contains(forbidden_fragment),
            "delivery proof leaked {forbidden_fragment}"
        );
    }

    let mut unknown_ip = serde_json::to_value(valid_proof()).expect("proof JSON");

    unknown_ip
        .as_object_mut()
        .expect("proof object")
        .insert("requester_ip".to_owned(), json!("192.0.2.10"));

    assert!(
        serde_json::from_value::<DeliveryProofV1>(unknown_ip).is_err(),
        "strict DTO must reject requester IP fields"
    );

    let mut payout_field = serde_json::to_value(valid_proof()).expect("proof JSON");

    payout_field.as_object_mut().expect("proof object").insert(
        "reward_recipient_account_id".to_owned(),
        json!("account:attacker"),
    );

    assert!(
        serde_json::from_value::<DeliveryProofV1>(payout_field).is_err(),
        "service evidence must reject payout-recipient smuggling"
    );
}

#[test]
fn verified_bytes_time_and_authority_boundaries_are_strict() {
    let mut zero_bytes = valid_proof();
    zero_bytes.bytes_delivered = 0;

    assert_eq!(
        zero_bytes.validate(),
        Err(DeliveryProofValidationError::ZeroBytes)
    );

    let mut unverified = valid_proof();
    unverified.content_verified = false;

    assert_eq!(
        unverified.validate(),
        Err(DeliveryProofValidationError::UnverifiedContent)
    );

    let mut bad_order = valid_proof();
    bad_order.completed_at_ms = bad_order.started_at_ms - 1;

    assert_eq!(
        bad_order.validate(),
        Err(DeliveryProofValidationError::InvalidTimeOrder)
    );

    for field in [
        "reward_truth",
        "payout_authority",
        "wallet_mutation",
        "ledger_mutation",
    ] {
        let mut proof = valid_proof();

        match field {
            "reward_truth" => proof.reward_truth = true,
            "payout_authority" => {
                proof.payout_authority = true;
            }
            "wallet_mutation" => {
                proof.wallet_mutation = true;
            }
            "ledger_mutation" => {
                proof.ledger_mutation = true;
            }
            _ => unreachable!(),
        }

        assert_eq!(
            proof.validate(),
            Err(DeliveryProofValidationError::AuthorityBoundary { field })
        );
    }

    let mut not_evidence = valid_proof();
    not_evidence.evidence_only = false;

    assert_eq!(
        not_evidence.validate(),
        Err(DeliveryProofValidationError::AuthorityBoundary {
            field: "evidence_only",
        })
    );
}

#[test]
fn replay_key_cannot_be_evaded_by_changing_proof_id() {
    let original = valid_proof();
    let original_key = original.replay_key();

    let mut renamed = original.clone();
    renamed.proof_id = "delivery:proof:9999".to_owned();

    assert_eq!(
        renamed.replay_key(),
        original_key,
        "new proof ID must not change duplicate identity"
    );

    let mut different_request = original;
    different_request.request_id = "request:0002".to_owned();

    assert_ne!(
        different_request.replay_key(),
        original_key,
        "a distinct requester request needs a distinct replay key"
    );
}
