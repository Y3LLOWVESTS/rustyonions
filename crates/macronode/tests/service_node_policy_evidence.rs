//! RO:WHAT — Signed policy-refusal and moderation-action reviewer tests.
//! RO:WHY — Prove signatures, replay rejection, and non-economic classification.

#[path = "../src/services/challenge_evidence.rs"]
mod challenge_evidence;

#[path = "../src/services/policy_evidence.rs"]
mod policy_evidence;

use challenge_evidence::WitnessKeyResolver;
use policy_evidence::{
    PolicyEvidenceCandidateV1, PolicyEvidenceReview, PolicyEvidenceReviewer,
    PolicyEvidenceReviewerConfigError,
};
use ron_kms::backends::ed25519;
use ron_proto::{
    ChallengeEvidenceKindV1, ContentId, ModerationActionKindV1, ModerationActionProofV1,
    ModerationActionProofValidationError, PolicyRefusalProofV1, PolicyRefusalProofValidationError,
    PolicyRefusalReasonV1, ServiceChallengeAckV1, MODERATION_ACTION_PROOF_SCHEMA,
    POLICY_REFUSAL_PROOF_SCHEMA, SERVICE_CHALLENGE_ACK_SCHEMA, SERVICE_EVIDENCE_VERSION,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const REQUESTER_NODE: &str = "user_node:bravo";
const MODERATION_WITNESS: &str = "audit_node:delta";
const WITNESS_KEY: &str = "witness_key:0001";

struct StaticResolver {
    public_key: [u8; 32],
}

impl WitnessKeyResolver for StaticResolver {
    fn resolve_ed25519_key(&self, witness_node_id: &str, witness_key_id: &str) -> Option<[u8; 32]> {
        let known_witness =
            witness_node_id == REQUESTER_NODE || witness_node_id == MODERATION_WITNESS;

        if known_witness && witness_key_id == WITNESS_KEY {
            Some(self.public_key)
        } else {
            None
        }
    }
}

fn content_id() -> ContentId {
    CID.parse().expect("canonical content ID")
}

fn refusal() -> PolicyRefusalProofV1 {
    PolicyRefusalProofV1 {
        schema: POLICY_REFUSAL_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "policy_refusal:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        requester_node_id: REQUESTER_NODE.to_owned(),
        request_id: "policy_request:0001".to_owned(),
        request_nonce: "nonce:policy:0001".to_owned(),
        content_id: content_id(),
        policy_snapshot_ref: "policy:snapshot:0001".to_owned(),
        service_observation_ref: "policy:refusal:observation:0001".to_owned(),
        requester_ack_ref: "requester:policy:ack:0001".to_owned(),
        refusal_reason: PolicyRefusalReasonV1::GlobalDeny,
        requested_at_ms: 1_900_000_000_000,
        refused_at_ms: 1_900_000_000_005,
        policy_enforced: true,
        content_served: false,
        bytes_served: 0,
        evidence_only: true,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn moderation() -> ModerationActionProofV1 {
    ModerationActionProofV1 {
        schema: MODERATION_ACTION_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "moderation:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        operator_subject_id: "operator:charlie".to_owned(),
        witness_node_id: MODERATION_WITNESS.to_owned(),
        action_id: "moderation:action:0001".to_owned(),
        action_nonce: "nonce:moderation:0001".to_owned(),
        content_id: content_id(),
        policy_snapshot_before_ref: "policy:snapshot:before:0001".to_owned(),
        policy_snapshot_after_ref: "policy:snapshot:after:0001".to_owned(),
        action_observation_ref: "moderation:observation:0001".to_owned(),
        witness_ack_ref: "moderation:witness:ack:0001".to_owned(),
        action: ModerationActionKindV1::LocalBlock,
        action_recorded_at_ms: 1_900_000_000_100,
        changed: true,
        runtime_hot_reload: false,
        storage_delete: false,
        provider_withdrawal: false,
        network_propagation: false,
        evidence_only: true,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn signed_ack(
    kind: ChallengeEvidenceKindV1,
    ack_id: &str,
    witness_node_id: &str,
    signing_bytes: &[u8],
    secret_key: &[u8; 32],
) -> ServiceChallengeAckV1 {
    let signature = ed25519::sign(secret_key, signing_bytes);

    ServiceChallengeAckV1 {
        schema: SERVICE_CHALLENGE_ACK_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        evidence_kind: kind,
        ack_id: ack_id.to_owned(),
        witness_node_id: witness_node_id.to_owned(),
        witness_key_id: WITNESS_KEY.to_owned(),
        signature_hex: signature.iter().map(|byte| format!("{byte:02x}")).collect(),
        evidence_only: true,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

fn refusal_ack(proof: &PolicyRefusalProofV1, secret_key: &[u8; 32]) -> ServiceChallengeAckV1 {
    signed_ack(
        ChallengeEvidenceKindV1::PolicyRefusal,
        &proof.requester_ack_ref,
        &proof.requester_node_id,
        &proof.requester_ack_signing_bytes(WITNESS_KEY),
        secret_key,
    )
}

fn moderation_ack(proof: &ModerationActionProofV1, secret_key: &[u8; 32]) -> ServiceChallengeAckV1 {
    signed_ack(
        ChallengeEvidenceKindV1::ModerationAction,
        &proof.witness_ack_ref,
        &proof.witness_node_id,
        &proof.witness_ack_signing_bytes(WITNESS_KEY),
        secret_key,
    )
}

#[test]
fn signed_refusal_becomes_non_reward_candidate() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = PolicyEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = refusal();
    let ack = refusal_ack(&proof, &secret_key);

    let review = reviewer.review_policy_refusal_signed(proof.clone(), ack.clone(), &resolver);

    let PolicyEvidenceReview::Candidate(candidate) = review else {
        panic!("signed refusal must become candidate");
    };

    let PolicyEvidenceCandidateV1::PolicyRefusal(candidate) = candidate.as_ref() else {
        panic!("candidate kind mismatch");
    };

    assert_eq!(candidate.proof, proof);
    assert_eq!(candidate.requester_ack, ack);
    assert!(candidate.requester_signature_verified);
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_eligible);
    assert!(!candidate.reward_truth);
    assert!(!candidate.payout_authority);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
}

#[test]
fn signed_moderation_action_does_not_claim_follow_on_effects() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = PolicyEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = moderation();
    let ack = moderation_ack(&proof, &secret_key);

    let review = reviewer.review_moderation_action_signed(proof.clone(), ack, &resolver);

    let PolicyEvidenceReview::Candidate(candidate) = review else {
        panic!("signed moderation action must become candidate");
    };

    let PolicyEvidenceCandidateV1::ModerationAction(candidate) = candidate.as_ref() else {
        panic!("candidate kind mismatch");
    };

    assert_eq!(candidate.proof, proof);
    assert!(candidate.witness_signature_verified);
    assert!(!candidate.runtime_activation_proven);
    assert!(!candidate.storage_delete_proven);
    assert!(!candidate.provider_withdrawal_proven);
    assert!(!candidate.network_propagation_proven);
    assert!(!candidate.accounting_accepted);
    assert!(!candidate.reward_eligible);
    assert!(!candidate.reward_truth);
    assert!(!candidate.wallet_mutation);
    assert!(!candidate.ledger_mutation);
}

#[test]
fn invalid_refusal_does_not_consume_replay() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = PolicyEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = refusal();
    let ack = refusal_ack(&original, &secret_key);

    let mut invalid = original;
    invalid.bytes_served = 1;

    assert_eq!(
        reviewer.review_policy_refusal_signed(invalid, ack, &resolver,),
        PolicyEvidenceReview::PolicyRefusalInvalid {
            error: PolicyRefusalProofValidationError::BytesServed { bytes_served: 1 },
        }
    );

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn invalid_moderation_action_rejects_before_crypto() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = PolicyEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = moderation();
    let ack = moderation_ack(&original, &secret_key);

    let mut invalid = original;
    invalid.provider_withdrawal = true;

    assert_eq!(
        reviewer.review_moderation_action_signed(invalid, ack, &resolver,),
        PolicyEvidenceReview::ModerationActionInvalid {
            error: ModerationActionProofValidationError::ProviderWithdrawalClaim,
        }
    );

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn changed_proof_id_cannot_evade_duplicate_detection() {
    let (public_key, secret_key) = ed25519::generate();

    let reviewer = PolicyEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let original = refusal();
    let original_ack = refusal_ack(&original, &secret_key);

    assert!(matches!(
        reviewer.review_policy_refusal_signed(original.clone(), original_ack, &resolver,),
        PolicyEvidenceReview::Candidate(_)
    ));

    let mut renamed = original;
    renamed.proof_id = "policy_refusal:proof:9999".to_owned();

    let renamed_ack = refusal_ack(&renamed, &secret_key);

    assert!(matches!(
        reviewer.review_policy_refusal_signed(renamed, renamed_ack, &resolver,),
        PolicyEvidenceReview::Duplicate { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 1);
}

#[test]
fn unknown_key_and_forged_signature_do_not_poison_replay() {
    let (public_key, secret_key) = ed25519::generate();
    let (_other_public, other_secret) = ed25519::generate();

    let reviewer = PolicyEvidenceReviewer::new(8).expect("bounded reviewer");
    let resolver = StaticResolver { public_key };

    let proof = moderation();

    let mut unknown = moderation_ack(&proof, &secret_key);
    unknown.witness_key_id = "witness_key:unknown".to_owned();

    assert!(matches!(
        reviewer.review_moderation_action_signed(proof.clone(), unknown, &resolver,),
        PolicyEvidenceReview::WitnessKeyUnavailable { .. }
    ));

    let forged = moderation_ack(&proof, &other_secret);

    assert!(matches!(
        reviewer.review_moderation_action_signed(proof, forged, &resolver,),
        PolicyEvidenceReview::WitnessSignatureInvalid { .. }
    ));

    assert_eq!(reviewer.observed_key_count(), 0);
}

#[test]
fn zero_replay_capacity_rejects() {
    assert!(matches!(
        PolicyEvidenceReviewer::new(0),
        Err(PolicyEvidenceReviewerConfigError::ZeroReplayCapacity)
    ));
}
