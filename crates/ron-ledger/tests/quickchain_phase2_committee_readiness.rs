#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 2 Round 2 tests for bounded committee attestation readiness over replay results.
//! RO:WHY — ECON/GOV: small replicated verification needs deterministic agreement semantics before validator economy.
//! RO:INTERACTS — ron-ledger replicated_replay; ron-proto committee attestation/readiness DTOs.
//! RO:INVARIANTS — read-only; deterministic; no clocks, DB order, service IO, fork choice, finality, staking, slashing, bridge, or pruning.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — fixture signatures are inert shape data and grant no spend or settlement authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_tree_inclusion_proof_from_batch, build_tree_material_batch, compute_tree_root_from_batch,
    evaluate_committee_readiness_from_attestations, verify_replay_bundle_read_only,
    QuickChainTreeMaterialProjectionError, QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quantum::SignatureAlg,
    quickchain::{
        QuickChainCommitteeAttestationSetV1, QuickChainCommitteeDisagreementCodeV1,
        QuickChainCommitteeMemberV1, QuickChainCommitteeReadinessStatusV1,
        QuickChainTreeMaterialKindV1, QuickChainVerifierAttestationV1,
        QuickChainVerifierReplayBundleV1, QuickChainVerifierReplayResultV1,
        QuickChainVerifierReplayStatusV1, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1,
        QUICKCHAIN_COMMITTEE_ATTESTATION_SET_SCHEMA,
        QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA, QUICKCHAIN_COMMITTEE_MEMBER_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA,
        QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1,
        QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0003";
const ALICE_SORT_KEY: &str = "6163636f756e743a616c69636500726f63";
const BOB_SORT_KEY: &str = "6163636f756e743a626f6200726f63";
const CAROL_SORT_KEY: &str = "6163636f756e743a6361726f6c00726f63";

fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn material_item(sort_key_hex: &str, label: &str) -> QuickChainTreeMaterialProjectionItem {
    QuickChainTreeMaterialProjectionItem::new(
        sort_key_hex,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        test_content_id(label),
    )
}

fn state_batch() -> ron_proto::quickchain::QuickChainTreeMaterialBatchV1 {
    build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(CAROL_SORT_KEY, "carol-leaf"),
            material_item(ALICE_SORT_KEY, "alice-leaf"),
            material_item(BOB_SORT_KEY, "bob-leaf"),
        ],
    )
    .expect("state material batch should build deterministically")
}

fn replay_bundle() -> QuickChainVerifierReplayBundleV1 {
    let batch = state_batch();
    let root = compute_tree_root_from_batch(&batch).expect("state root should compute");
    let proof = build_tree_inclusion_proof_from_batch(&batch, ALICE_SORT_KEY)
        .expect("alice inclusion proof should build");

    QuickChainVerifierReplayBundleV1 {
        schema: QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        replay_algorithm: QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1.to_string(),
        material_batches: vec![batch],
        expected_roots: vec![root],
        inclusion_proofs: vec![proof],
    }
}

fn replay_result_hash(result: &QuickChainVerifierReplayResultV1) -> ContentId {
    let bytes = serde_json::to_vec(result).expect("replay result should serialize");
    let digest = blake3::hash(&bytes).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture replay result hash should parse")
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

fn attestation(member_id: &str, result_hash: ContentId) -> QuickChainVerifierAttestationV1 {
    QuickChainVerifierAttestationV1 {
        schema: QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        committee_member_id: member_id.to_string(),
        key_id: format!("{member_id}/key_01"),
        signature_algorithm: SignatureAlg::Ed25519,
        signed_payload_schema: QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA.to_string(),
        replay_result_hash: result_hash,
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        signature_wire: format!("test-signature-wire-for-{member_id}"),
    }
}

fn attestation_set(
    result_hash: ContentId,
    attesting_members: &[&str],
) -> QuickChainCommitteeAttestationSetV1 {
    QuickChainCommitteeAttestationSetV1 {
        schema: QUICKCHAIN_COMMITTEE_ATTESTATION_SET_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        committee_algorithm: QUICKCHAIN_COMMITTEE_AGREEMENT_ALGORITHM_BOUNDED_V1.to_string(),
        replay_result_hash: result_hash.clone(),
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        required_attestations: 2,
        committee_members: vec![
            committee_member("committee_member_01"),
            committee_member("committee_member_02"),
            committee_member("committee_member_03"),
        ],
        attestations: attesting_members
            .iter()
            .map(|member_id| attestation(member_id, result_hash.clone()))
            .collect(),
    }
}

#[test]
fn committee_readiness_becomes_ready_after_bounded_attestations() {
    let replay_result =
        verify_replay_bundle_read_only(&replay_bundle()).expect("replay should verify");
    let result_hash = replay_result_hash(&replay_result);
    let set = attestation_set(result_hash, &["committee_member_01", "committee_member_02"]);

    let readiness = evaluate_committee_readiness_from_attestations(&set)
        .expect("valid attestation set should evaluate");

    assert_eq!(
        readiness.status,
        QuickChainCommitteeReadinessStatusV1::Ready
    );
    assert_eq!(readiness.required_attestations, 2);
    assert_eq!(readiness.committee_members_count, 3);
    assert_eq!(readiness.accepted_attestations_count, 2);
    assert_eq!(readiness.disagreement_code, None);

    readiness
        .validate()
        .expect("readiness result should satisfy ron-proto validation");
}

#[test]
fn committee_readiness_stays_not_ready_below_threshold() {
    let replay_result =
        verify_replay_bundle_read_only(&replay_bundle()).expect("replay should verify");
    let result_hash = replay_result_hash(&replay_result);
    let set = attestation_set(result_hash, &["committee_member_01"]);

    let readiness = evaluate_committee_readiness_from_attestations(&set)
        .expect("valid below-threshold attestation set should evaluate");

    assert_eq!(
        readiness.status,
        QuickChainCommitteeReadinessStatusV1::NotReady
    );
    assert_eq!(readiness.accepted_attestations_count, 1);
    assert_eq!(
        readiness.disagreement_code,
        Some(QuickChainCommitteeDisagreementCodeV1::InsufficientAttestations)
    );

    readiness
        .validate()
        .expect("not-ready result should satisfy ron-proto validation");
}

#[test]
fn duplicate_member_attestation_rejects_before_readiness() {
    let replay_result =
        verify_replay_bundle_read_only(&replay_bundle()).expect("replay should verify");
    let result_hash = replay_result_hash(&replay_result);
    let set = attestation_set(result_hash, &["committee_member_01", "committee_member_01"]);

    let error = evaluate_committee_readiness_from_attestations(&set)
        .expect_err("double attestation by one member must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::InvalidBatch { .. }
    ));
}

#[test]
fn mismatched_attestation_result_hash_rejects_before_readiness() {
    let replay_result =
        verify_replay_bundle_read_only(&replay_bundle()).expect("replay should verify");
    let result_hash = replay_result_hash(&replay_result);
    let mut set = attestation_set(result_hash, &["committee_member_01", "committee_member_02"]);
    set.attestations[1].replay_result_hash = test_content_id("different-replay-result");

    let error = evaluate_committee_readiness_from_attestations(&set)
        .expect_err("mismatched replay result hash must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::InvalidBatch { .. }
    ));
}
