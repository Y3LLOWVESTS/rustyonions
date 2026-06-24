//! RO:WHAT — Phase 2 Round 2 DTO tests for bounded committee attestation/readiness artifacts.
//! RO:WHY — ECON/GOV: committee agreement must be shape-checked before any validator economy, staking, slashing, or bridge scope.
//! RO:INTERACTS — ron-proto quickchain verifier DTOs.
//! RO:INVARIANTS — DTO-only; unknown fields reject; duplicate attestations reject; readiness is not finality.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture signatures/hashes are inert test artifacts.
//! RO:TEST — this file.

use ron_proto::{
    quantum::SignatureAlg,
    quickchain::{
        QuickChainCommitteeAttestationSetV1, QuickChainCommitteeDisagreementCodeV1,
        QuickChainCommitteeMemberV1, QuickChainCommitteeReadinessResultV1,
        QuickChainCommitteeReadinessStatusV1, QuickChainVerifierAttestationV1,
        QuickChainVerifierReplayStatusV1, QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1,
        QUICKCHAIN_COMMITTEE_ATTESTATION_SET_SCHEMA,
        QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA, QUICKCHAIN_COMMITTEE_MEMBER_SCHEMA,
        QUICKCHAIN_COMMITTEE_READINESS_RESULT_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0003";

fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn committee_member(id: &str) -> QuickChainCommitteeMemberV1 {
    QuickChainCommitteeMemberV1 {
        schema: QUICKCHAIN_COMMITTEE_MEMBER_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        member_id: id.to_string(),
        key_id: format!("{id}/key_01"),
    }
}

fn attestation(member_id: &str, replay_result_hash: ContentId) -> QuickChainVerifierAttestationV1 {
    QuickChainVerifierAttestationV1 {
        schema: QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        committee_member_id: member_id.to_string(),
        key_id: format!("{member_id}/key_01"),
        signature_algorithm: SignatureAlg::Ed25519,
        signed_payload_schema: QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA.to_string(),
        replay_result_hash,
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        signature_wire: format!("test-signature-wire-for-{member_id}"),
    }
}

fn attestation_set() -> QuickChainCommitteeAttestationSetV1 {
    let replay_result_hash = test_content_id("phase2-round2-replay-result");

    QuickChainCommitteeAttestationSetV1 {
        schema: QUICKCHAIN_COMMITTEE_ATTESTATION_SET_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        committee_algorithm: QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1.to_string(),
        replay_result_hash: replay_result_hash.clone(),
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        required_attestations: 2,
        committee_members: vec![
            committee_member("committee_member_01"),
            committee_member("committee_member_02"),
            committee_member("committee_member_03"),
        ],
        attestations: vec![
            attestation("committee_member_01", replay_result_hash.clone()),
            attestation("committee_member_02", replay_result_hash),
        ],
    }
}

#[test]
fn committee_attestation_set_accepts_bounded_count_based_readiness_shape() {
    let set = attestation_set();

    set.validate()
        .expect("valid committee attestation set should validate");

    assert_eq!(set.required_attestations, 2);
    assert_eq!(set.committee_members.len(), 3);
    assert_eq!(set.attestations.len(), 2);
}

#[test]
fn committee_attestation_set_rejects_duplicate_member_attestation() {
    let mut set = attestation_set();
    let replay_result_hash = set.replay_result_hash.clone();
    set.attestations
        .push(attestation("committee_member_01", replay_result_hash));

    let error = set
        .validate()
        .expect_err("duplicate member attestation must reject")
        .to_string();

    assert!(
        error.contains("attestations.committee_member_id"),
        "unexpected error: {error}"
    );
}

#[test]
fn committee_attestation_set_rejects_attestation_from_outside_committee() {
    let mut set = attestation_set();
    let replay_result_hash = set.replay_result_hash.clone();
    set.attestations
        .push(attestation("committee_member_99", replay_result_hash));

    let error = set
        .validate()
        .expect_err("outside-committee attestation must reject")
        .to_string();

    assert!(
        error.contains("attestations.committee_member_id"),
        "unexpected error: {error}"
    );
}

#[test]
fn committee_attestation_set_rejects_mismatched_replay_result_hash() {
    let mut set = attestation_set();
    set.attestations[0].replay_result_hash = test_content_id("different-result");

    let error = set
        .validate()
        .expect_err("mismatched replay result hash must reject")
        .to_string();

    assert!(
        error.contains("attestations.replay_result_hash"),
        "unexpected error: {error}"
    );
}

#[test]
fn committee_readiness_result_status_must_match_counts() {
    let result = QuickChainCommitteeReadinessResultV1 {
        schema: QUICKCHAIN_COMMITTEE_READINESS_RESULT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        committee_algorithm: QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1.to_string(),
        replay_result_hash: test_content_id("ready-result"),
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        required_attestations: 2,
        committee_members_count: 3,
        accepted_attestations_count: 2,
        status: QuickChainCommitteeReadinessStatusV1::Ready,
        disagreement_code: None,
    };

    result
        .validate()
        .expect("ready result with enough attestations should validate");

    let mut invalid = result;
    invalid.accepted_attestations_count = 1;

    let error = invalid
        .validate()
        .expect_err("ready result without enough attestations must reject")
        .to_string();

    assert!(error.contains("status"), "unexpected error: {error}");

    let not_ready = QuickChainCommitteeReadinessResultV1 {
        status: QuickChainCommitteeReadinessStatusV1::NotReady,
        accepted_attestations_count: 1,
        disagreement_code: Some(QuickChainCommitteeDisagreementCodeV1::InsufficientAttestations),
        ..invalid
    };

    not_ready
        .validate()
        .expect("not_ready result with insufficient_attestations should validate");
}

#[test]
fn committee_attestation_set_rejects_unknown_fields() {
    let set = attestation_set();
    let mut value = serde_json::to_value(set).expect("set should serialize");

    value
        .as_object_mut()
        .expect("set should serialize as object")
        .insert("finality_claim".to_string(), serde_json::json!(true));

    let error = serde_json::from_value::<QuickChainCommitteeAttestationSetV1>(value)
        .expect_err("unknown fields must reject");

    assert!(
        error.to_string().contains("unknown field"),
        "unexpected error: {error}"
    );
}
