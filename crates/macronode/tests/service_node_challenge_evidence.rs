//! RO:WHAT — Signed availability and hot-cache reviewer tests.
//! RO:WHY — Prove witness verification, replay rejection, and non-authority.
//! RO:TEST — cargo test -p macronode --test
//!   service_node_challenge_evidence.

#[path = "../src/services/challenge_evidence.rs"]
mod challenge_evidence;

use challenge_evidence::{
    ChallengeEvidenceCandidateV1, ChallengeEvidenceReview, ChallengeEvidenceReviewer,
    ChallengeEvidenceReviewerConfigError, WitnessKeyResolver,
};
use ron_kms::backends::ed25519;
use ron_proto::{
    AvailabilityProofV1, ChallengeEvidenceKindV1, ChallengeEvidenceValidationError, ContentId,
    HotCacheProofV1, HotCacheTierV1, ServiceChallengeAckV1, AVAILABILITY_PROOF_SCHEMA,
    HOT_CACHE_PROOF_SCHEMA, SERVICE_CHALLENGE_ACK_SCHEMA, SERVICE_EVIDENCE_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const WITNESS_NODE: &str = "user_node:bravo";
const WITNESS_KEY: &str = "witness_key:0001";

struct StaticResolver {
    public_key: [u8; 32],
}

impl WitnessKeyResolver for StaticResolver {
    fn resolve_ed25519_key(&self, witness_node_id: &str, witness_key_id: &str) -> Option<[u8; 32]> {
        if witness_node_id == WITNESS_NODE && witness_key_id == WITNESS_KEY {
            Some(self.public_key)
        } else {
            None
        }
    }
}

fn content_id() -> ContentId {
    CID.parse().expect("canonical test CID")
}

fn availability() -> AvailabilityProofV1 {
    AvailabilityProofV1 {
        schema: AVAILABILITY_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "availability:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: WITNESS_NODE.to_owned(),
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
        witness_node_id: WITNESS_NODE.to_owned(),
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

fn signed_ack(
    kind: ChallengeEvidenceKindV1,
    ack_id: &str,
    signing_bytes: &[u8],
    secret_key: &[u8; 32],
) -> ServiceChallengeAckV1 {
    let signature = ed25519::sign(secret_key, signing_bytes);

    ServiceChallengeAckV1 {
        schema: SERVICE_CHALLENGE_ACK_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        evidence_kind: kind,
        ack_id: ack_id.to_owned(),
        witness_node_id: WITNESS_NODE.to_owned(),
        witness_key_id: WITNESS_KEY.to_owned(),
        signature_hex: signature.iter().map(|byte| format!("{byte:02x}")).collect(),
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn availability_ack(proof: &AvailabilityProofV1, secret_key: &[u8; 32]) -> ServiceChallengeAckV1 {
    signed_ack(
        ChallengeEvidenceKindV1::Availability,
        &proof.witness_ack_ref,
        &proof.witness_ack_signing_bytes(WITNESS_KEY),
        secret_key,
    )
}

fn hot_cache_ack(proof: &HotCacheProofV1, secret_key: &[u8; 32]) -> ServiceChallengeAckV1 {
    signed_ack(
        ChallengeEvidenceKindV1::HotCache,
        &proof.witness_ack_ref,
        &proof.witness_ack_signing_bytes(WITNESS_KEY),
        secret_key,
    )
}

#[test]
fn signed_availability_becomes_non_economic_candidate() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = ChallengeEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = availability();
    let ack = availability_ack(&proof, &secret_key);

    let review = reviewer.review_availability_signed(proof.clone(), ack.clone(), &resolver);

    let ChallengeEvidenceReview::Candidate(candidate) = review else {
        panic!("signed availability must be a candidate");
    };

    let ChallengeEvidenceCandidateV1::Availability(candidate) = candidate.as_ref() else {
        panic!("candidate kind mismatch");
    };

    assert_eq!(candidate.proof, proof);
    assert_eq!(candidate.witness_ack, ack);
    assert!(candidate.witness_signature_verified);
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_truth);
    assert!(!candidate.payout_authority);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
}

#[test]
fn signed_hot_cache_does_not_claim_durability() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = ChallengeEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = hot_cache();
    let ack = hot_cache_ack(&proof, &secret_key);

    let review = reviewer.review_hot_cache_signed(proof.clone(), ack, &resolver);

    let ChallengeEvidenceReview::Candidate(candidate) = review else {
        panic!("signed hot-cache proof must be candidate");
    };

    let ChallengeEvidenceCandidateV1::HotCache(candidate) = candidate.as_ref() else {
        panic!("candidate kind mismatch");
    };

    assert_eq!(candidate.proof, proof);
    assert!(candidate.witness_signature_verified);
    assert!(!candidate.durable_storage_proven);
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_truth);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
}

#[test]
fn tampering_rejects_before_replay_insertion() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = ChallengeEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = availability();
    let ack = availability_ack(&original, &secret_key);

    let mut tampered = original;
    tampered.bytes_returned = tampered.requested_sample_length - 1;

    assert_eq!(
        reviewer.review_availability_signed(tampered, ack, &resolver,),
        ChallengeEvidenceReview::AvailabilityInvalid {
            error: ChallengeEvidenceValidationError::SampleLengthMismatch {
                requested: 4_096,
                returned: 4_095,
            },
        }
    );

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn changed_proof_id_cannot_evade_duplicate_detection() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = ChallengeEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = availability();
    let original_ack = availability_ack(&original, &secret_key);

    assert!(matches!(
        reviewer.review_availability_signed(original.clone(), original_ack, &resolver,),
        ChallengeEvidenceReview::Candidate(_)
    ));

    let mut renamed = original;
    renamed.proof_id = "availability:proof:9999".to_owned();

    let renamed_ack = availability_ack(&renamed, &secret_key);

    assert!(matches!(
        reviewer.review_availability_signed(renamed, renamed_ack, &resolver,),
        ChallengeEvidenceReview::Duplicate { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn unknown_key_and_forged_signature_do_not_poison_replay() {
    let (public_key, secret_key) = ed25519::generate();
    let (_other_public, other_secret) = ed25519::generate();

    let reviewer = ChallengeEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = hot_cache();

    let mut unknown = hot_cache_ack(&proof, &secret_key);
    unknown.witness_key_id = "witness_key:unknown".to_owned();

    assert!(matches!(
        reviewer.review_hot_cache_signed(proof.clone(), unknown, &resolver,),
        ChallengeEvidenceReview::WitnessKeyUnavailable { .. }
    ));

    let forged = hot_cache_ack(&proof, &other_secret);

    assert!(matches!(
        reviewer.review_hot_cache_signed(proof, forged, &resolver,),
        ChallengeEvidenceReview::WitnessSignatureInvalid { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn replay_window_is_bounded() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = ChallengeEvidenceReviewer::new(1).expect("capacity-one reviewer");
    let resolver = StaticResolver { public_key };

    let availability = availability();
    let availability_ack = availability_ack(&availability, &secret_key);

    assert!(matches!(
        reviewer.review_availability_signed(availability, availability_ack, &resolver,),
        ChallengeEvidenceReview::Candidate(_)
    ));

    let hot_cache = hot_cache();
    let hot_cache_ack = hot_cache_ack(&hot_cache, &secret_key);

    assert!(matches!(
        reviewer.review_hot_cache_signed(hot_cache, hot_cache_ack, &resolver,),
        ChallengeEvidenceReview::Candidate(_)
    ));

    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn zero_replay_capacity_rejects() {
    assert!(matches!(
        ChallengeEvidenceReviewer::new(0),
        Err(ChallengeEvidenceReviewerConfigError::ZeroReplayCapacity)
    ));
}
