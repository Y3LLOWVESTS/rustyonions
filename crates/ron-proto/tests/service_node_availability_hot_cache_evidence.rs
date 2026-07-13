//! RO:WHAT — Phase 13 availability/hot-cache DTO tests.
//! RO:WHY — Lock witness, replay, privacy, and authority boundaries.
//! RO:TEST — cargo test -p ron-proto --test
//!   service_node_availability_hot_cache_evidence.

use ron_proto::{
    AvailabilityProofV1, ChallengeEvidenceKindV1, ChallengeEvidenceValidationError, ContentId,
    HotCacheProofV1, HotCacheTierV1, ServiceChallengeAckV1, ServiceChallengeAckValidationError,
    AVAILABILITY_PROOF_SCHEMA, HOT_CACHE_PROOF_SCHEMA, SERVICE_CHALLENGE_ACK_SCHEMA,
    SERVICE_EVIDENCE_VERSION,
};
use serde_json::json;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn content_id() -> ContentId {
    CID.parse().expect("canonical test CID")
}

fn availability() -> AvailabilityProofV1 {
    AvailabilityProofV1 {
        schema: AVAILABILITY_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "availability:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        challenge_id: "availability:challenge:0001".to_owned(),
        challenge_nonce: "nonce:0001".to_owned(),
        content_id: content_id(),
        service_observation_ref: "service:observation:0001".to_owned(),
        witness_ack_ref: "witness:ack:0001".to_owned(),
        requested_sample_offset: 65_536,
        requested_sample_length: 4_096,
        bytes_returned: 4_096,
        challenge_issued_at_ms: 1_900_000_000_000,
        response_completed_at_ms: 1_900_000_000_025,
        content_verified: true,
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn hot_cache() -> HotCacheProofV1 {
    HotCacheProofV1 {
        schema: HOT_CACHE_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "hot_cache:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        challenge_id: "hot_cache:challenge:0001".to_owned(),
        challenge_nonce: "nonce:0002".to_owned(),
        content_id: content_id(),
        service_observation_ref: "service:observation:0002".to_owned(),
        witness_ack_ref: "witness:ack:0002".to_owned(),
        bytes_returned: 16_384,
        challenge_issued_at_ms: 1_900_000_000_000,
        response_completed_at_ms: 1_900_000_000_010,
        response_latency_micros: 8_500,
        cache_tier: HotCacheTierV1::Memory,
        cache_hit: true,
        origin_fetch_performed: false,
        content_verified: true,
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
fn valid_availability_and_hot_cache_proofs_roundtrip() {
    let availability = availability();
    availability.validate().expect("availability proof");

    let encoded = serde_json::to_value(&availability).expect("availability JSON");
    let decoded: AvailabilityProofV1 =
        serde_json::from_value(encoded).expect("availability decode");

    assert_eq!(decoded, availability);

    let hot_cache = hot_cache();
    hot_cache.validate().expect("hot-cache proof");

    let encoded = serde_json::to_value(&hot_cache).expect("hot-cache JSON");
    let decoded: HotCacheProofV1 = serde_json::from_value(encoded).expect("hot-cache decode");

    assert_eq!(decoded, hot_cache);
}

#[test]
fn provider_only_and_self_traffic_claims_reject() {
    let mut provider_only = availability();
    provider_only.witness_ack_ref.clear();

    assert_eq!(
        provider_only.validate(),
        Err(ChallengeEvidenceValidationError::ProviderOnlyClaim)
    );

    let mut self_traffic = hot_cache();
    self_traffic.witness_node_id = self_traffic.service_node_id.clone();

    assert_eq!(
        self_traffic.validate(),
        Err(ChallengeEvidenceValidationError::SelfTraffic)
    );
}

#[test]
fn availability_sample_and_integrity_are_exact() {
    let mut short = availability();
    short.bytes_returned -= 1;

    assert_eq!(
        short.validate(),
        Err(ChallengeEvidenceValidationError::SampleLengthMismatch {
            requested: 4_096,
            returned: 4_095,
        })
    );

    let mut unverified = availability();
    unverified.content_verified = false;

    assert_eq!(
        unverified.validate(),
        Err(ChallengeEvidenceValidationError::UnverifiedContent)
    );
}

#[test]
fn hot_cache_rejects_miss_origin_fetch_and_fake_latency() {
    let mut miss = hot_cache();
    miss.cache_hit = false;

    assert_eq!(
        miss.validate(),
        Err(ChallengeEvidenceValidationError::HotCacheMiss)
    );

    let mut origin = hot_cache();
    origin.origin_fetch_performed = true;

    assert_eq!(
        origin.validate(),
        Err(ChallengeEvidenceValidationError::OriginFetchPerformed)
    );

    let mut impossible_latency = hot_cache();
    impossible_latency.response_latency_micros = 10_001;

    assert_eq!(
        impossible_latency.validate(),
        Err(ChallengeEvidenceValidationError::LatencyExceedsWindow {
            latency_us: 10_001,
            window_us: 10_000,
        })
    );
}

#[test]
fn replay_keys_ignore_proof_id_but_bind_challenge() {
    let original = availability();
    let key = original.replay_key();

    let mut renamed = original.clone();
    renamed.proof_id = "availability:proof:9999".to_owned();

    assert_eq!(renamed.replay_key(), key);

    let mut new_challenge = original;
    new_challenge.challenge_nonce = "nonce:9999".to_owned();

    assert_ne!(new_challenge.replay_key(), key);
}

#[test]
fn acknowledgments_bind_kind_identity_and_reference() {
    let availability = availability();
    let ack = acknowledgment(
        ChallengeEvidenceKindV1::Availability,
        &availability.witness_ack_ref,
    );

    assert_eq!(
        ack.validate_for_availability(&availability)
            .expect("valid ack shape"),
        [0u8; 64]
    );

    let mut wrong_kind = ack.clone();
    wrong_kind.evidence_kind = ChallengeEvidenceKindV1::HotCache;

    assert_eq!(
        wrong_kind.validate_for_availability(&availability),
        Err(ServiceChallengeAckValidationError::KindMismatch {
            expected: ChallengeEvidenceKindV1::Availability,
            actual: ChallengeEvidenceKindV1::HotCache,
        })
    );

    let mut wrong_identity = ack;
    wrong_identity.witness_node_id = "user_node:charlie".to_owned();

    assert_eq!(
        wrong_identity.validate_for_availability(&availability),
        Err(ServiceChallengeAckValidationError::BindingMismatch {
            field: "witness_node_id",
        })
    );
}

#[test]
fn signing_bytes_are_deterministic_and_materially_bound() {
    let availability = availability();

    let first = availability.witness_ack_signing_bytes("witness_key:0001");

    assert_eq!(
        first,
        availability.witness_ack_signing_bytes("witness_key:0001",)
    );

    let mut changed = availability;
    changed.bytes_returned += 1;

    assert_ne!(
        first,
        changed.witness_ack_signing_bytes("witness_key:0001",)
    );

    let hot_cache = hot_cache();

    assert_ne!(
        hot_cache.witness_ack_signing_bytes("witness_key:0001",),
        hot_cache.witness_ack_signing_bytes("witness_key:0002",)
    );
}

#[test]
fn wire_shape_rejects_ip_and_authority_smuggling() {
    let mut value = serde_json::to_value(availability()).expect("availability JSON");

    value
        .as_object_mut()
        .expect("availability object")
        .insert("requester_ip".to_owned(), json!("192.0.2.10"));

    assert!(serde_json::from_value::<AvailabilityProofV1>(value).is_err());

    let mut ack = serde_json::to_value(acknowledgment(
        ChallengeEvidenceKindV1::HotCache,
        "witness:ack:0002",
    ))
    .expect("ack JSON");

    ack.as_object_mut()
        .expect("ack object")
        .insert("public_key".to_owned(), json!("provider-supplied-key"));

    assert!(serde_json::from_value::<ServiceChallengeAckV1>(ack).is_err());

    let mut authority = hot_cache();
    authority.reward_truth = true;

    assert_eq!(
        authority.validate(),
        Err(ChallengeEvidenceValidationError::AuthorityBoundary {
            field: "reward_truth",
        })
    );
}
