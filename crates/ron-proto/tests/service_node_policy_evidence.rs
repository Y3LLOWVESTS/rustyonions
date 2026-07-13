//! RO:WHAT — Phase 13 policy-refusal and moderation-action DTO tests.
//! RO:WHY — Lock privacy, policy enforcement, replay, and non-economic boundaries.

use ron_proto::{
    ChallengeEvidenceKindV1, ContentId, ModerationActionKindV1, ModerationActionProofV1,
    ModerationActionProofValidationError, PolicyRefusalProofV1, PolicyRefusalProofValidationError,
    PolicyRefusalReasonV1, ServiceChallengeAckV1, ServiceChallengeAckValidationError,
    MODERATION_ACTION_PROOF_SCHEMA, POLICY_REFUSAL_PROOF_SCHEMA, SERVICE_CHALLENGE_ACK_SCHEMA,
    SERVICE_EVIDENCE_VERSION,
};
use serde_json::json;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn content_id() -> ContentId {
    CID.parse().expect("canonical content ID")
}

fn refusal() -> PolicyRefusalProofV1 {
    PolicyRefusalProofV1 {
        schema: POLICY_REFUSAL_PROOF_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        proof_id: "policy_refusal:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        requester_node_id: "user_node:bravo".to_owned(),
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
        witness_node_id: "audit_node:delta".to_owned(),
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

fn acknowledgment(
    kind: ChallengeEvidenceKindV1,
    ack_id: &str,
    witness_node_id: &str,
) -> ServiceChallengeAckV1 {
    ServiceChallengeAckV1 {
        schema: SERVICE_CHALLENGE_ACK_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_VERSION,
        evidence_kind: kind,
        ack_id: ack_id.to_owned(),
        witness_node_id: witness_node_id.to_owned(),
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
fn valid_policy_evidence_roundtrips() {
    let refusal = refusal();
    refusal.validate().expect("valid refusal");

    let encoded = serde_json::to_value(&refusal).expect("refusal JSON");
    let decoded: PolicyRefusalProofV1 = serde_json::from_value(encoded).expect("refusal decode");

    assert_eq!(decoded, refusal);

    let moderation = moderation();
    moderation.validate().expect("valid moderation action");

    let encoded = serde_json::to_value(&moderation).expect("moderation JSON");
    let decoded: ModerationActionProofV1 =
        serde_json::from_value(encoded).expect("moderation decode");

    assert_eq!(decoded, moderation);
}

#[test]
fn refusal_requires_independent_requester() {
    let mut provider_only = refusal();
    provider_only.requester_ack_ref.clear();

    assert_eq!(
        provider_only.validate(),
        Err(PolicyRefusalProofValidationError::ProviderOnlyClaim)
    );

    let mut self_traffic = refusal();
    self_traffic.requester_node_id = self_traffic.service_node_id.clone();

    assert_eq!(
        self_traffic.validate(),
        Err(PolicyRefusalProofValidationError::SelfTraffic)
    );
}

#[test]
fn refusal_cannot_hide_served_content() {
    let mut served = refusal();
    served.content_served = true;

    assert_eq!(
        served.validate(),
        Err(PolicyRefusalProofValidationError::ContentServed)
    );

    let mut bytes = refusal();
    bytes.bytes_served = 1;

    assert_eq!(
        bytes.validate(),
        Err(PolicyRefusalProofValidationError::BytesServed { bytes_served: 1 })
    );

    let mut not_enforced = refusal();
    not_enforced.policy_enforced = false;

    assert_eq!(
        not_enforced.validate(),
        Err(PolicyRefusalProofValidationError::PolicyNotEnforced)
    );
}

#[test]
fn policy_refusal_is_never_direct_reward_truth() {
    let mut proof = refusal();
    proof.reward_eligible = true;

    assert_eq!(
        proof.validate(),
        Err(PolicyRefusalProofValidationError::RewardEligible)
    );

    let mut proof = refusal();
    proof.reward_truth = true;

    assert_eq!(
        proof.validate(),
        Err(PolicyRefusalProofValidationError::AuthorityBoundary {
            field: "reward_truth",
        })
    );
}

#[test]
fn moderation_requires_distinct_actors_and_real_change() {
    let mut collision = moderation();
    collision.witness_node_id = collision.operator_subject_id.clone();

    assert_eq!(
        collision.validate(),
        Err(ModerationActionProofValidationError::ActorCollision {
            left: "operator_subject_id",
            right: "witness_node_id",
        })
    );

    let mut no_change = moderation();
    no_change.changed = false;

    assert_eq!(
        no_change.validate(),
        Err(ModerationActionProofValidationError::NoChange)
    );
}

#[test]
fn moderation_cannot_claim_follow_on_effects() {
    let mut runtime = moderation();
    runtime.runtime_hot_reload = true;

    assert_eq!(
        runtime.validate(),
        Err(ModerationActionProofValidationError::RuntimeHotReloadClaim)
    );

    let mut storage = moderation();
    storage.storage_delete = true;

    assert_eq!(
        storage.validate(),
        Err(ModerationActionProofValidationError::StorageDeleteClaim)
    );

    let mut provider = moderation();
    provider.provider_withdrawal = true;

    assert_eq!(
        provider.validate(),
        Err(ModerationActionProofValidationError::ProviderWithdrawalClaim)
    );
}

#[test]
fn replay_keys_ignore_proof_ids() {
    let original = refusal();
    let key = original.replay_key();

    let mut renamed = original;
    renamed.proof_id = "policy_refusal:proof:9999".to_owned();

    assert_eq!(renamed.replay_key(), key);

    let original = moderation();
    let key = original.replay_key();

    let mut renamed = original;
    renamed.proof_id = "moderation:proof:9999".to_owned();

    assert_eq!(renamed.replay_key(), key);
}

#[test]
fn acknowledgments_bind_kind_identity_and_reference() {
    let refusal = refusal();

    let refusal_ack = acknowledgment(
        ChallengeEvidenceKindV1::PolicyRefusal,
        &refusal.requester_ack_ref,
        &refusal.requester_node_id,
    );

    assert_eq!(
        refusal_ack
            .validate_for_policy_refusal(&refusal)
            .expect("valid refusal acknowledgment"),
        [0u8; 64]
    );

    let moderation = moderation();

    let wrong_kind = acknowledgment(
        ChallengeEvidenceKindV1::PolicyRefusal,
        &moderation.witness_ack_ref,
        &moderation.witness_node_id,
    );

    assert_eq!(
        wrong_kind.validate_for_moderation_action(&moderation),
        Err(ServiceChallengeAckValidationError::KindMismatch {
            expected: ChallengeEvidenceKindV1::ModerationAction,
            actual: ChallengeEvidenceKindV1::PolicyRefusal,
        })
    );
}

#[test]
fn signing_bytes_bind_material_fields() {
    let refusal = refusal();

    let first = refusal.requester_ack_signing_bytes("witness_key:0001");

    let mut changed = refusal;
    changed.refusal_reason = PolicyRefusalReasonV1::Quarantine;

    assert_ne!(
        first,
        changed.requester_ack_signing_bytes("witness_key:0001",)
    );

    let moderation = moderation();

    let first = moderation.witness_ack_signing_bytes("witness_key:0001");

    let mut changed = moderation;
    changed.action = ModerationActionKindV1::Quarantine;

    assert_ne!(
        first,
        changed.witness_ack_signing_bytes("witness_key:0001",)
    );
}

#[test]
fn wire_shape_rejects_ip_and_authority_smuggling() {
    let mut refusal = serde_json::to_value(refusal()).expect("refusal JSON");

    refusal
        .as_object_mut()
        .expect("refusal object")
        .insert("requester_ip".to_owned(), json!("192.0.2.10"));

    assert!(serde_json::from_value::<PolicyRefusalProofV1>(refusal).is_err());

    let mut moderation = moderation();
    moderation.payout_authority = true;

    assert_eq!(
        moderation.validate(),
        Err(ModerationActionProofValidationError::AuthorityBoundary {
            field: "payout_authority",
        })
    );
}
