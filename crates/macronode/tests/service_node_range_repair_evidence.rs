//! RO:WHAT — Signed range-request and repair reviewer tests.
//! RO:WHY — Prove exact ranges, repair actors, signatures, replay, and non-authority.
//! RO:TEST — cargo test -p macronode --test
//!   service_node_range_repair_evidence.

#[path = "../src/services/challenge_evidence.rs"]
mod challenge_evidence;

#[path = "../src/services/range_repair_evidence.rs"]
mod range_repair_evidence;

use challenge_evidence::WitnessKeyResolver;
use range_repair_evidence::{
    RangeRepairEvidenceCandidateV1, RangeRepairEvidenceReview, RangeRepairEvidenceReviewer,
    RangeRepairReviewerConfigError,
};
use ron_kms::backends::ed25519;
use ron_proto::{
    ChallengeEvidenceKindV1, ContentId, RangeRequestProofV1, RangeRequestProofValidationError,
    RepairProofV1, RepairProofValidationError, ServiceChallengeAckV1, RANGE_REQUEST_PROOF_SCHEMA,
    REPAIR_PROOF_SCHEMA, SERVICE_CHALLENGE_ACK_SCHEMA, SERVICE_EVIDENCE_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const RANGE_B3: &str = "b3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";

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
    CID.parse().expect("canonical content ID")
}

fn range_proof() -> RangeRequestProofV1 {
    RangeRequestProofV1 {
        schema: RANGE_REQUEST_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "range:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: WITNESS_NODE.to_owned(),
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
        witness_node_id: WITNESS_NODE.to_owned(),
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

fn range_ack(proof: &RangeRequestProofV1, secret_key: &[u8; 32]) -> ServiceChallengeAckV1 {
    signed_ack(
        ChallengeEvidenceKindV1::RangeRequest,
        &proof.witness_ack_ref,
        &proof.witness_ack_signing_bytes(WITNESS_KEY),
        secret_key,
    )
}

fn repair_ack(proof: &RepairProofV1, secret_key: &[u8; 32]) -> ServiceChallengeAckV1 {
    signed_ack(
        ChallengeEvidenceKindV1::Repair,
        &proof.witness_ack_ref,
        &proof.witness_ack_signing_bytes(WITNESS_KEY),
        secret_key,
    )
}

#[test]
fn signed_range_becomes_partial_delivery_candidate() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = RangeRepairEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = range_proof();
    let ack = range_ack(&proof, &secret_key);

    let review = reviewer.review_range_request_signed(proof.clone(), ack.clone(), &resolver);

    let RangeRepairEvidenceReview::Candidate(candidate) = review else {
        panic!("signed range must be candidate");
    };

    let RangeRepairEvidenceCandidateV1::RangeRequest(candidate) = candidate.as_ref() else {
        panic!("candidate kind mismatch");
    };

    assert_eq!(candidate.proof, proof);
    assert_eq!(candidate.witness_ack, ack);
    assert!(candidate.witness_signature_verified);
    assert!(!candidate.full_object_delivery_proven);
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_truth);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
}

#[test]
fn signed_repair_remains_local_non_economic_evidence() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = RangeRepairEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = repair_proof();
    let ack = repair_ack(&proof, &secret_key);

    let review = reviewer.review_repair_signed(proof.clone(), ack, &resolver);

    let RangeRepairEvidenceReview::Candidate(candidate) = review else {
        panic!("signed repair must be candidate");
    };

    let RangeRepairEvidenceCandidateV1::Repair(candidate) = candidate.as_ref() else {
        panic!("candidate kind mismatch");
    };

    assert_eq!(candidate.proof, proof);
    assert!(candidate.witness_signature_verified);
    assert!(!candidate.network_repair_proven);
    assert!(!candidate.provider_publication_proven);
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_truth);
    assert!(!candidate.payout_authority);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
}

#[test]
fn invalid_range_does_not_consume_replay_state() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = RangeRepairEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = range_proof();
    let ack = range_ack(&original, &secret_key);

    let mut invalid = original;
    invalid.bytes_returned -= 1;

    assert_eq!(
        reviewer.review_range_request_signed(invalid, ack, &resolver,),
        RangeRepairEvidenceReview::RangeRequestInvalid {
            error: RangeRequestProofValidationError::RangeLengthMismatch {
                requested: 4_096,
                returned: 4_095,
            },
        }
    );

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn invalid_repair_actor_set_rejects_before_crypto() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = RangeRepairEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = repair_proof();
    let ack = repair_ack(&original, &secret_key);

    let mut invalid = original;
    invalid.witness_node_id = invalid.source_service_node_id.clone();

    assert_eq!(
        reviewer.review_repair_signed(invalid, ack, &resolver,),
        RangeRepairEvidenceReview::RepairInvalid {
            error: RepairProofValidationError::ActorCollision {
                left: "source_service_node_id",
                right: "witness_node_id",
            },
        }
    );

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn changed_repair_proof_id_cannot_evade_duplicate() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = RangeRepairEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = repair_proof();
    let original_ack = repair_ack(&original, &secret_key);

    assert!(matches!(
        reviewer.review_repair_signed(original.clone(), original_ack, &resolver,),
        RangeRepairEvidenceReview::Candidate(_)
    ));

    let mut renamed = original;
    renamed.proof_id = "repair:proof:9999".to_owned();

    let renamed_ack = repair_ack(&renamed, &secret_key);

    assert!(matches!(
        reviewer.review_repair_signed(renamed, renamed_ack, &resolver,),
        RangeRepairEvidenceReview::Duplicate { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn unknown_key_and_forged_signature_do_not_poison_replay() {
    let (public_key, secret_key) = ed25519::generate();
    let (_other_public, other_secret) = ed25519::generate();

    let reviewer = RangeRepairEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = range_proof();

    let mut unknown = range_ack(&proof, &secret_key);
    unknown.witness_key_id = "witness_key:unknown".to_owned();

    assert!(matches!(
        reviewer.review_range_request_signed(proof.clone(), unknown, &resolver,),
        RangeRepairEvidenceReview::WitnessKeyUnavailable { .. }
    ));

    let forged = range_ack(&proof, &other_secret);

    assert!(matches!(
        reviewer.review_range_request_signed(proof, forged, &resolver,),
        RangeRepairEvidenceReview::WitnessSignatureInvalid { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn zero_replay_capacity_rejects() {
    assert!(matches!(
        RangeRepairEvidenceReviewer::new(0),
        Err(RangeRepairReviewerConfigError::ZeroReplayCapacity)
    ));
}
