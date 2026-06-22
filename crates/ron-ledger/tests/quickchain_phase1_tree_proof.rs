#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 1 tests for deterministic QuickChain tree inclusion proof construction and verification.
//! RO:WHY — ECON/GOV: roots must be accompanied by reproducible proof artifacts before downstream display.
//! RO:INTERACTS — ron-ledger quickchain tree-material projection and ron-proto proof DTOs.
//! RO:INVARIANTS — no DB order; no wall-clock; no validators; no anchors; no settlement; tampered proofs reject.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — fixture commitments are inert test artifacts, not wallet receipts or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_tree_inclusion_proof_from_batch, build_tree_material_batch, compute_tree_root_from_batch,
    verify_tree_inclusion_proof, QuickChainTreeMaterialProjectionError,
    QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        QuickChainTreeMaterialKindV1, QuickChainTreeProofSiblingPositionV1,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0001";
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

fn three_leaf_state_batch() -> ron_proto::quickchain::QuickChainTreeMaterialBatchV1 {
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
    .expect("three-leaf state material should build")
}

#[test]
fn two_leaf_inclusion_proof_verifies_against_locked_state_root() {
    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(BOB_SORT_KEY, "bob-leaf"),
            material_item(ALICE_SORT_KEY, "alice-leaf"),
        ],
    )
    .expect("two-leaf state material should build");

    let root = compute_tree_root_from_batch(&batch).expect("root should compute");
    let proof = build_tree_inclusion_proof_from_batch(&batch, ALICE_SORT_KEY)
        .expect("alice proof should build");

    proof
        .validate()
        .expect("proof should satisfy ron-proto validation");
    verify_tree_inclusion_proof(&root, &proof).expect("proof should verify");

    assert_eq!(proof.schema, QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA);
    assert_eq!(proof.leaf_index, 0);
    assert_eq!(proof.source_items_count, 2);
    assert_eq!(proof.tree_height, 1);
    assert_eq!(proof.steps.len(), 1);
    assert_eq!(
        proof.steps[0].sibling_position,
        QuickChainTreeProofSiblingPositionV1::Right
    );
    assert_eq!(
        root.root_hash.to_string(),
        "b3:f1810151cc24d5865bdb5692d919588ef4d3f822f50952fa62b41b40d2a73a3f"
    );
}

#[test]
fn odd_carry_leaf_proof_verifies_with_implicit_carry_layer() {
    let batch = three_leaf_state_batch();
    let root = compute_tree_root_from_batch(&batch).expect("root should compute");
    let proof = build_tree_inclusion_proof_from_batch(&batch, CAROL_SORT_KEY)
        .expect("carol proof should build");

    verify_tree_inclusion_proof(&root, &proof).expect("odd-carry proof should verify");

    assert_eq!(proof.leaf_index, 2);
    assert_eq!(proof.source_items_count, 3);
    assert_eq!(proof.tree_height, 2);

    // Carol is carried at layer 0, then paired with the alice+bob branch at layer 1.
    assert_eq!(proof.steps.len(), 1);
    assert_eq!(proof.steps[0].layer_index, 1);
    assert_eq!(
        proof.steps[0].sibling_position,
        QuickChainTreeProofSiblingPositionV1::Left
    );
    assert_eq!(
        root.root_hash.to_string(),
        "b3:fb534c250d95f3d0474e3129af481eaefd4f983b7f485047c55bdda532115bfc"
    );
}

#[test]
fn proof_for_middle_leaf_verifies_with_left_then_right_sibling_path() {
    let batch = three_leaf_state_batch();
    let root = compute_tree_root_from_batch(&batch).expect("root should compute");
    let proof = build_tree_inclusion_proof_from_batch(&batch, BOB_SORT_KEY)
        .expect("bob proof should build");

    verify_tree_inclusion_proof(&root, &proof).expect("bob proof should verify");

    assert_eq!(proof.leaf_index, 1);
    assert_eq!(proof.source_items_count, 3);
    assert_eq!(proof.tree_height, 2);
    assert_eq!(proof.steps.len(), 2);
    assert_eq!(
        proof.steps[0].sibling_position,
        QuickChainTreeProofSiblingPositionV1::Left
    );
    assert_eq!(
        proof.steps[1].sibling_position,
        QuickChainTreeProofSiblingPositionV1::Right
    );
}

#[test]
fn missing_material_sort_key_rejects_before_proof_is_returned() {
    let batch = three_leaf_state_batch();

    let error =
        build_tree_inclusion_proof_from_batch(&batch, "6163636f756e743a6d697373696e6700726f63")
            .expect_err("missing material sort key must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::MaterialSortKeyNotFound { .. }
    ));
}

#[test]
fn tampered_sibling_hash_rejects_verification() {
    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(BOB_SORT_KEY, "bob-leaf"),
            material_item(ALICE_SORT_KEY, "alice-leaf"),
        ],
    )
    .expect("two-leaf state material should build");

    let root = compute_tree_root_from_batch(&batch).expect("root should compute");
    let mut proof = build_tree_inclusion_proof_from_batch(&batch, ALICE_SORT_KEY)
        .expect("alice proof should build");

    proof.steps[0].sibling_node_hash = test_content_id("tampered-sibling");

    let error =
        verify_tree_inclusion_proof(&root, &proof).expect_err("tampered sibling proof must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::ProofVerificationFailed { .. }
    ));
}

#[test]
fn mismatched_root_metadata_rejects_verification() {
    let batch = three_leaf_state_batch();
    let mut root = compute_tree_root_from_batch(&batch).expect("root should compute");
    let proof = build_tree_inclusion_proof_from_batch(&batch, ALICE_SORT_KEY)
        .expect("alice proof should build");

    root.epoch_id = "epoch_9999".to_string();

    let error = verify_tree_inclusion_proof(&root, &proof)
        .expect_err("root/proof metadata mismatch must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::ProofVerificationFailed { .. }
    ));
}

#[test]
fn tampered_leaf_commitment_rejects_verification() {
    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(BOB_SORT_KEY, "bob-leaf"),
            material_item(ALICE_SORT_KEY, "alice-leaf"),
        ],
    )
    .expect("two-leaf state material should build");

    let root = compute_tree_root_from_batch(&batch).expect("root should compute");
    let mut proof = build_tree_inclusion_proof_from_batch(&batch, ALICE_SORT_KEY)
        .expect("alice proof should build");

    proof.leaf.payload_hash = test_content_id("tampered-leaf");

    let error =
        verify_tree_inclusion_proof(&root, &proof).expect_err("tampered leaf proof must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::ProofVerificationFailed { .. }
    ));
}
