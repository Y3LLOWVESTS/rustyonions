//! RO:WHAT — Phase 13 range-request and repair DTO tests.
//! RO:WHY — Lock exact ranges, repair actors, privacy, replay, and authority.
//! RO:TEST — cargo test -p ron-proto --test
//!   service_node_range_repair_evidence.

use ron_proto::{
    ChallengeEvidenceKindV1, ContentId, RangeRequestProofV1, RangeRequestProofValidationError,
    RepairProofV1, RepairProofValidationError, ServiceChallengeAckV1,
    ServiceChallengeAckValidationError, RANGE_REQUEST_PROOF_SCHEMA, REPAIR_PROOF_SCHEMA,
    SERVICE_CHALLENGE_ACK_SCHEMA, SERVICE_EVIDENCE_VERSION,
};
use serde_json::json;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const RANGE_B3: &str = "b3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";

fn content_id() -> ContentId {
    CID.parse().expect("canonical content ID")
}

fn range_proof() -> RangeRequestProofV1 {
    RangeRequestProofV1 {
        schema: RANGE_REQUEST_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "range:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        request_id: "range:request:0001".to_owned(),
        request_nonce: "nonce:range:0001".to_owned(),
        content_id: content_id(),
        service_observation_ref: "service:range:observation:0001".to_owned(),
        witness_ack_ref: "witness:range:ack:0001".to_owned(),
        range_start: 65_536,
        range_length: 4_096,
        bytes_returned: 4_096,
        expected_range_b3: RANGE_B3.to_owned(),
        observed_range_b3: RANGE_B3.to_owned(),
        request_started_at_ms: 1_900_000_000_000,
        response_completed_at_ms: 1_900_000_000_020,
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn repair_proof() -> RepairProofV1 {
    RepairProofV1 {
        schema: REPAIR_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "repair:proof:0001".to_owned(),
        repairing_service_node_id: "service_node:target".to_owned(),
        source_service_node_id: "service_node:source".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        repair_id: "repair:0001".to_owned(),
        repair_nonce: "nonce:repair:0001".to_owned(),
        content_id: content_id(),
        source_observation_ref: "repair:source:observation:0001".to_owned(),
        target_observation_ref: "repair:target:observation:0001".to_owned(),
        witness_ack_ref: "witness:repair:ack:0001".to_owned(),
        bytes_repaired: 131_072,
        repair_started_at_ms: 1_900_000_000_000,
        repair_completed_at_ms: 1_900_000_000_250,
        source_content_verified: true,
        target_content_verified: true,
        local_object_restored: true,
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn acknowledgment(kind: ChallengeEvidenceKindV1, ack_id: &str) -> ServiceChallengeAckV1 {
    ServiceChallengeAckV1 {
        schema: SERVICE_CHALLENGE_ACK_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        evidence_kind: kind,
        ack_id: ack_id.to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        witness_key_id: "witness_key:0001".to_owned(),
        signature_hex: "00".repeat(64),
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

#[test]
fn valid_range_and_repair_proofs_roundtrip() {
    let range = range_proof();
    range.validate().expect("valid range proof");

    let encoded = serde_json::to_value(&range).expect("range JSON");
    let decoded: RangeRequestProofV1 = serde_json::from_value(encoded).expect("range decode");

    assert_eq!(decoded, range);

    let repair = repair_proof();
    repair.validate().expect("valid repair proof");

    let encoded = serde_json::to_value(&repair).expect("repair JSON");
    let decoded: RepairProofV1 = serde_json::from_value(encoded).expect("repair decode");

    assert_eq!(decoded, repair);
}

#[test]
fn range_requires_exact_length_and_digest() {
    let mut short = range_proof();
    short.bytes_returned -= 1;

    assert_eq!(
        short.validate(),
        Err(RangeRequestProofValidationError::RangeLengthMismatch {
            requested: 4_096,
            returned: 4_095,
        })
    );

    let mut digest_mismatch = range_proof();
    digest_mismatch.observed_range_b3 =
        "b3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned();

    assert_eq!(
        digest_mismatch.validate(),
        Err(RangeRequestProofValidationError::RangeDigestMismatch)
    );
}

#[test]
fn range_rejects_provider_only_self_traffic_and_overflow() {
    let mut provider_only = range_proof();
    provider_only.witness_ack_ref.clear();

    assert_eq!(
        provider_only.validate(),
        Err(RangeRequestProofValidationError::ProviderOnlyClaim)
    );

    let mut self_traffic = range_proof();
    self_traffic.witness_node_id = self_traffic.service_node_id.clone();

    assert_eq!(
        self_traffic.validate(),
        Err(RangeRequestProofValidationError::SelfTraffic)
    );

    let mut overflow = range_proof();
    overflow.range_start = u64::MAX;
    overflow.range_length = 2;
    overflow.bytes_returned = 2;

    assert_eq!(
        overflow.validate(),
        Err(RangeRequestProofValidationError::RangeOverflow)
    );
}

#[test]
fn repair_requires_three_distinct_actors() {
    let mut source_is_target = repair_proof();
    source_is_target.source_service_node_id = source_is_target.repairing_service_node_id.clone();

    assert_eq!(
        source_is_target.validate(),
        Err(RepairProofValidationError::ActorCollision {
            left: "repairing_service_node_id",
            right: "source_service_node_id",
        })
    );

    let mut witness_is_source = repair_proof();
    witness_is_source.witness_node_id = witness_is_source.source_service_node_id.clone();

    assert_eq!(
        witness_is_source.validate(),
        Err(RepairProofValidationError::ActorCollision {
            left: "source_service_node_id",
            right: "witness_node_id",
        })
    );
}

#[test]
fn repair_requires_verified_source_and_restored_target() {
    let mut bad_source = repair_proof();
    bad_source.source_content_verified = false;

    assert_eq!(
        bad_source.validate(),
        Err(RepairProofValidationError::UnverifiedSource)
    );

    let mut bad_target = repair_proof();
    bad_target.target_content_verified = false;

    assert_eq!(
        bad_target.validate(),
        Err(RepairProofValidationError::UnverifiedTarget)
    );

    let mut not_restored = repair_proof();
    not_restored.local_object_restored = false;

    assert_eq!(
        not_restored.validate(),
        Err(RepairProofValidationError::LocalObjectNotRestored)
    );
}

#[test]
fn replay_keys_ignore_proof_ids() {
    let original = range_proof();
    let key = original.replay_key();

    let mut renamed = original;
    renamed.proof_id = "range:proof:9999".to_owned();

    assert_eq!(renamed.replay_key(), key);

    let original = repair_proof();
    let key = original.replay_key();

    let mut renamed = original;
    renamed.proof_id = "repair:proof:9999".to_owned();

    assert_eq!(renamed.replay_key(), key);
}

#[test]
fn acknowledgments_bind_range_and_repair_kinds() {
    let range = range_proof();
    let range_ack = acknowledgment(
        ChallengeEvidenceKindV1::RangeRequest,
        &range.witness_ack_ref,
    );

    assert_eq!(
        range_ack
            .validate_for_range_request(&range)
            .expect("range ack"),
        [0u8; 64]
    );

    let repair = repair_proof();
    let mut wrong_kind = acknowledgment(
        ChallengeEvidenceKindV1::RangeRequest,
        &repair.witness_ack_ref,
    );

    assert_eq!(
        wrong_kind.validate_for_repair(&repair),
        Err(ServiceChallengeAckValidationError::KindMismatch {
            expected: ChallengeEvidenceKindV1::Repair,
            actual: ChallengeEvidenceKindV1::RangeRequest,
        })
    );

    wrong_kind.evidence_kind = ChallengeEvidenceKindV1::Repair;

    assert!(wrong_kind.validate_for_repair(&repair).is_ok());
}

#[test]
fn signing_bytes_bind_material_fields() {
    let range = range_proof();

    let first = range.witness_ack_signing_bytes("witness_key:0001");

    let mut changed = range;
    changed.range_start += 1;

    assert_ne!(
        first,
        changed.witness_ack_signing_bytes("witness_key:0001",)
    );

    let repair = repair_proof();

    let first = repair.witness_ack_signing_bytes("witness_key:0001");

    let mut changed = repair;
    changed.bytes_repaired += 1;

    assert_ne!(
        first,
        changed.witness_ack_signing_bytes("witness_key:0001",)
    );
}

#[test]
fn wire_shape_rejects_ip_and_authority_smuggling() {
    let mut range = serde_json::to_value(range_proof()).expect("range JSON");

    range
        .as_object_mut()
        .expect("range object")
        .insert("requester_ip".to_owned(), json!("192.0.2.10"));

    assert!(serde_json::from_value::<RangeRequestProofV1>(range).is_err());

    let mut repair = repair_proof();
    repair.reward_truth = true;

    assert_eq!(
        repair.validate(),
        Err(RepairProofValidationError::AuthorityBoundary {
            field: "reward_truth",
        })
    );
}
