//! RO:WHAT — Phase 13 macronode delivery-evidence reviewer tests.
//! RO:WHY — Prove bounded duplicate rejection and non-economic corruption findings.
//! RO:INTERACTS — ron-proto DeliveryProofV1 and macronode delivery reviewer.
//! RO:INVARIANTS — no fake acceptance; no reward truth; bounded replay memory.
//! RO:SECURITY — invalid claims do not poison replay; findings expose no IP.
//! RO:TEST — cargo test -p macronode --test service_node_delivery_evidence_review.

#[path = "../src/services/delivery_evidence.rs"]
mod delivery_evidence;

use delivery_evidence::{
    DeliveryEvidenceReview, DeliveryEvidenceReviewer, DeliveryEvidenceReviewerConfigError,
    CORRUPT_DELIVERY_FINDING_SCHEMA,
};
use ron_proto::{
    ContentId, DeliveryProofV1, DeliveryProofValidationError, DELIVERY_PROOF_SCHEMA,
    SERVICE_EVIDENCE_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn content_id() -> ContentId {
    CID.parse().expect("test CID must be canonical")
}

fn valid_proof(proof_id: &str, request_id: &str) -> DeliveryProofV1 {
    DeliveryProofV1 {
        schema: DELIVERY_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: proof_id.to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        requester_node_id: "user_node:bravo".to_owned(),
        request_id: request_id.to_owned(),
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
fn valid_proof_becomes_candidate_not_reward_truth() {
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let proof = valid_proof("delivery:proof:0001", "request:0001");

    let review = reviewer.review(proof.clone());

    let DeliveryEvidenceReview::Candidate(candidate) = review else {
        panic!("valid proof must become a review candidate");
    };

    assert_eq!(candidate.proof, proof);
    assert_eq!(candidate.replay_key, candidate.proof.replay_key());

    assert!(
        !candidate.requester_signature_verified,
        "DTO validation must not fake signature verification"
    );
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_truth);
    assert!(!candidate.payout_authority);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn duplicate_key_rejects_even_when_proof_id_changes() {
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let original = valid_proof("delivery:proof:0001", "request:0001");

    assert!(matches!(
        reviewer.review(original.clone()),
        DeliveryEvidenceReview::Candidate(_)
    ));

    let mut renamed = original;
    renamed.proof_id = "delivery:proof:9999".to_owned();

    let expected_key = renamed.replay_key();

    assert_eq!(
        reviewer.review(renamed),
        DeliveryEvidenceReview::Duplicate {
            replay_key: expected_key,
        }
    );

    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn corrupt_delivery_creates_non_economic_integrity_finding() {
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let mut proof = valid_proof("delivery:proof:0001", "request:0001");
    proof.content_verified = false;

    let review = reviewer.review(proof.clone());

    let DeliveryEvidenceReview::CorruptDelivery(finding) = review else {
        panic!("unverified delivery must create an integrity finding");
    };

    assert_eq!(finding.schema, CORRUPT_DELIVERY_FINDING_SCHEMA);
    assert_eq!(finding.service_node_id, proof.service_node_id);
    assert_eq!(finding.requester_node_id, proof.requester_node_id);
    assert_eq!(finding.expected_content_id, proof.content_id);
    assert_eq!(finding.finding, "content_address_verification_failed");

    assert!(finding.provider_integrity_failure);
    assert!(!finding.reward_eligible);
    assert!(!finding.automatic_economic_penalty);
    assert!(finding.evidence_only);
    assert!(!finding.reward_truth);
    assert!(!finding.payout_authority);
    assert!(!finding.wallet_mutation);
    assert!(!finding.ledger_mutation);

    let json = serde_json::to_string(&finding).expect("serialize integrity finding");

    for forbidden in [
        "requester_ip",
        "residential_ip",
        "socket_addr",
        "remote_addr",
        "reward_recipient",
        "payout_recipient",
        "192.168.",
        "127.0.0.1",
        "tcp://",
        "http://",
    ] {
        assert!(
            !json.contains(forbidden),
            "integrity finding leaked forbidden value {forbidden}"
        );
    }

    assert_eq!(
        reviewer.review(proof.clone()),
        DeliveryEvidenceReview::Duplicate {
            replay_key: proof.replay_key(),
        },
        "repeated corrupt claim must not create unlimited findings"
    );
}

#[test]
fn malformed_provider_only_claim_does_not_poison_replay() {
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let mut malformed = valid_proof("delivery:proof:0001", "request:0001");
    malformed.requester_ack_ref.clear();

    assert_eq!(
        reviewer.review(malformed),
        DeliveryEvidenceReview::Invalid {
            error: DeliveryProofValidationError::ProviderOnlyClaim,
        }
    );

    assert_eq!(
        reviewer.observed_key_count(),
        0,
        "invalid proof must not reserve a replay identity"
    );

    assert!(matches!(
        reviewer.review(valid_proof("delivery:proof:0002", "request:0001",)),
        DeliveryEvidenceReview::Candidate(_)
    ));
}

#[test]
fn replay_window_is_bounded_and_evicts_oldest_key() {
    let reviewer = DeliveryEvidenceReviewer::new(1).expect("capacity-one reviewer");

    assert!(matches!(
        reviewer.review(valid_proof("delivery:proof:0001", "request:0001",)),
        DeliveryEvidenceReview::Candidate(_)
    ));

    assert!(matches!(
        reviewer.review(valid_proof("delivery:proof:0002", "request:0002",)),
        DeliveryEvidenceReview::Candidate(_)
    ));

    assert_eq!(reviewer.observed_key_count(), 1);

    // The first key fell outside this explicitly bounded local window.
    assert!(matches!(
        reviewer.review(valid_proof("delivery:proof:0003", "request:0001",)),
        DeliveryEvidenceReview::Candidate(_)
    ));
}

#[test]
fn zero_replay_capacity_is_rejected() {
    assert!(matches!(
        DeliveryEvidenceReviewer::new(0),
        Err(DeliveryEvidenceReviewerConfigError::ZeroReplayCapacity)
    ));
}
