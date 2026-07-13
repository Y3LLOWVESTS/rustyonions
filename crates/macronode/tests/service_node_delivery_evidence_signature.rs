//! RO:WHAT — Phase 13 real requester acknowledgment signature tests.
//! RO:WHY — Provider claims must not become candidates without requester verification.
//! RO:INTERACTS — macronode reviewer, ron-proto evidence, ron-kms Ed25519.
//! RO:INVARIANTS — signature before replay; trusted key comes from resolver.
//! RO:SECURITY — forged corruption cannot create a verified provider finding.
//! RO:TEST — cargo test -p macronode --test service_node_delivery_evidence_signature.

#[path = "../src/services/delivery_evidence.rs"]
mod delivery_evidence;

use delivery_evidence::{DeliveryEvidenceReview, DeliveryEvidenceReviewer, RequesterKeyResolver};
use ron_kms::backends::ed25519;
use ron_proto::{
    ContentId, DeliveryProofV1, DeliveryRequesterAckV1, DeliveryRequesterAckValidationError,
    DELIVERY_PROOF_SCHEMA, DELIVERY_REQUESTER_ACK_SCHEMA, SERVICE_EVIDENCE_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const REQUESTER_NODE: &str = "user_node:bravo";
const REQUESTER_KEY: &str = "requester_key:0001";

struct StaticResolver {
    public_key: [u8; 32],
}

impl RequesterKeyResolver for StaticResolver {
    fn resolve_ed25519_key(
        &self,
        requester_node_id: &str,
        requester_key_id: &str,
    ) -> Option<[u8; 32]> {
        if requester_node_id == REQUESTER_NODE && requester_key_id == REQUESTER_KEY {
            Some(self.public_key)
        } else {
            None
        }
    }
}

fn content_id() -> ContentId {
    CID.parse().expect("canonical test CID")
}

fn proof() -> DeliveryProofV1 {
    DeliveryProofV1 {
        schema: DELIVERY_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "delivery:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        requester_node_id: REQUESTER_NODE.to_owned(),
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

fn signed_ack(proof: &DeliveryProofV1, secret_key: &[u8; 32]) -> DeliveryRequesterAckV1 {
    let signing_bytes = proof.requester_ack_signing_bytes(REQUESTER_KEY);

    let signature = ed25519::sign(secret_key, &signing_bytes);

    DeliveryRequesterAckV1 {
        schema: DELIVERY_REQUESTER_ACK_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        ack_id: proof.requester_ack_ref.clone(),
        requester_node_id: proof.requester_node_id.clone(),
        requester_key_id: REQUESTER_KEY.to_owned(),
        signature_hex: signature.iter().map(|byte| format!("{byte:02x}")).collect(),
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

#[test]
fn valid_requester_signature_creates_verified_candidate() {
    let (public_key, secret_key) = ed25519::generate();

    let resolver = StaticResolver { public_key };
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let proof = proof();
    let ack = signed_ack(&proof, &secret_key);

    let review = reviewer.review_signed(proof.clone(), ack.clone(), &resolver);

    let DeliveryEvidenceReview::Candidate(candidate) = review else {
        panic!("valid signature must create candidate");
    };

    assert_eq!(candidate.proof, proof);
    assert_eq!(candidate.requester_ack, Some(ack));
    assert!(candidate.requester_signature_verified);

    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_truth);
    assert!(!candidate.payout_authority);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn tampered_proof_rejects_before_replay_insertion() {
    let (public_key, secret_key) = ed25519::generate();

    let resolver = StaticResolver { public_key };
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let original = proof();
    let ack = signed_ack(&original, &secret_key);

    let mut tampered = original;
    tampered.bytes_delivered += 1;

    assert!(matches!(
        reviewer.review_signed(tampered, ack, &resolver,),
        DeliveryEvidenceReview::RequesterSignatureInvalid { .. }
    ));

    assert_eq!(
        reviewer.observed_key_count(),
        0,
        "invalid signature must not poison replay state"
    );
}

#[test]
fn unknown_requester_key_rejects_without_replay() {
    let (_public_key, secret_key) = ed25519::generate();

    let different_resolver = StaticResolver {
        public_key: [0u8; 32],
    };

    let proof = proof();
    let mut ack = signed_ack(&proof, &secret_key);
    ack.requester_key_id = "requester_key:unknown".to_owned();

    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    assert!(matches!(
        reviewer.review_signed(proof, ack, &different_resolver,),
        DeliveryEvidenceReview::RequesterKeyUnavailable { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn signed_corrupt_delivery_creates_verified_finding() {
    let (public_key, secret_key) = ed25519::generate();

    let resolver = StaticResolver { public_key };
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let mut corrupt = proof();
    corrupt.content_verified = false;

    let ack = signed_ack(&corrupt, &secret_key);

    let review = reviewer.review_signed(corrupt, ack, &resolver);

    let DeliveryEvidenceReview::CorruptDelivery(finding) = review else {
        panic!("signed corrupt delivery must create finding");
    };

    assert!(finding.provider_integrity_failure);
    assert!(finding.requester_signature_verified);
    assert_eq!(finding.requester_key_id.as_deref(), Some(REQUESTER_KEY));

    assert!(!finding.reward_eligible);
    assert!(!finding.automatic_economic_penalty);
    assert!(!finding.reward_truth);
    assert!(!finding.wallet_mutation);
    assert!(!finding.ledger_mutation);
}

#[test]
fn forged_corruption_cannot_create_verified_finding() {
    let (public_key, _secret_key) = ed25519::generate();
    let (_other_public_key, other_secret_key) = ed25519::generate();

    let resolver = StaticResolver { public_key };
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let mut corrupt = proof();
    corrupt.content_verified = false;

    let forged_ack = signed_ack(&corrupt, &other_secret_key);

    assert!(matches!(
        reviewer.review_signed(corrupt, forged_ack, &resolver,),
        DeliveryEvidenceReview::RequesterSignatureInvalid { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn acknowledgment_identity_mismatch_rejects_before_crypto() {
    let (public_key, secret_key) = ed25519::generate();

    let resolver = StaticResolver { public_key };
    let reviewer = DeliveryEvidenceReviewer::new(8).expect("bounded reviewer");

    let proof = proof();
    let mut ack = signed_ack(&proof, &secret_key);
    ack.requester_node_id = "user_node:charlie".to_owned();

    assert_eq!(
        reviewer.review_signed(proof, ack, &resolver,),
        DeliveryEvidenceReview::AcknowledgmentInvalid {
            error: DeliveryRequesterAckValidationError::BindingMismatch {
                field: "requester_node_id",
            },
        }
    );

    assert_eq!(reviewer.observed_key_count(), 0);
}
