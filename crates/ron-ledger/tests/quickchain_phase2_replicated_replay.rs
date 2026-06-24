#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 2 Round 1 tests for read-only replicated replay over Phase 1 artifacts.
//! RO:WHY — ECON/GOV: two replay runs must reproduce the same roots from the same artifacts before committee work.
//! RO:INTERACTS — ron-ledger replicated_replay and tree_material_projection; ron-proto verifier DTOs.
//! RO:INVARIANTS — read-only replay only; no clocks, DB order, service IO, signing, quorum, bridge, staking, or finality.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — replay results are diagnostic artifacts and grant no spend or settlement authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_tree_inclusion_proof_from_batch, build_tree_material_batch, compute_tree_root_from_batch,
    verify_replay_bundle_read_only, QuickChainTreeMaterialProjectionError,
    QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        QuickChainTreeMaterialKindV1, QuickChainVerifierCheckStatusV1,
        QuickChainVerifierReplayBundleV1, QuickChainVerifierReplayStatusV1,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1,
        QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0002";
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

#[test]
fn two_read_only_replay_runs_reproduce_identical_results() {
    let bundle = replay_bundle();

    let first = verify_replay_bundle_read_only(&bundle).expect("first replay should verify");
    let second = verify_replay_bundle_read_only(&bundle).expect("second replay should verify");

    assert_eq!(first, second);
    assert_eq!(first.status, QuickChainVerifierReplayStatusV1::Verified);
    assert_eq!(first.root_checks.len(), 1);
    assert_eq!(first.proof_checks.len(), 1);
    assert_eq!(
        first.root_checks[0].status,
        QuickChainVerifierCheckStatusV1::Verified
    );
    assert_eq!(
        first.proof_checks[0].status,
        QuickChainVerifierCheckStatusV1::Verified
    );

    first
        .validate()
        .expect("replay result should satisfy ron-proto DTO validation");
}

#[test]
fn root_mismatch_returns_diagnostic_mismatch_not_authority() {
    let mut bundle = replay_bundle();
    bundle.inclusion_proofs.clear();
    bundle.expected_roots[0].root_hash = test_content_id("wrong-state-root");

    let result = verify_replay_bundle_read_only(&bundle)
        .expect("valid bundle with wrong expected root should report mismatch");

    assert_eq!(result.status, QuickChainVerifierReplayStatusV1::Mismatch);
    assert_eq!(result.root_checks.len(), 1);
    assert_eq!(
        result.root_checks[0].status,
        QuickChainVerifierCheckStatusV1::Mismatch
    );
    assert_eq!(result.proof_checks.len(), 0);
}

#[test]
fn missing_expected_root_rejects_as_invalid_replay_bundle() {
    let mut bundle = replay_bundle();
    bundle.expected_roots.clear();

    let error = verify_replay_bundle_read_only(&bundle)
        .expect_err("missing expected root should reject before replay");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::InvalidBatch { .. }
    ));
}
