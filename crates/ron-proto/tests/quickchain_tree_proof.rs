//! RO:WHAT — Tests for strict QuickChain tree inclusion-proof DTOs.
//! RO:WHY — ECON/GOV: Phase 1 proof artifacts need strict wire shape before ledger/verifier logic consumes them.
//! RO:INTERACTS — ron_proto::quickchain::root_material proof DTOs.
//! RO:INVARIANTS — DTO-only; no hashing; no finality; no validators; no anchors; no settlement.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture hashes are inert commitments, not spend authority or validator attestations.
//! RO:TEST — this file.

use ron_proto::{
    quickchain::{
        QuickChainCanonicalEncodingV1, QuickChainTreeInclusionProofStepV1,
        QuickChainTreeInclusionProofV1, QuickChainTreeLeafNodeV1, QuickChainTreeMaterialKindV1,
        QuickChainTreeProofSiblingPositionV1, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA, QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA,
        QUICKCHAIN_TREE_LEAF_NODE_SCHEMA, QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1,
    },
    ContentId,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0001";
const ALICE_SORT_KEY_HEX: &str = "6163636f756e743a616c69636500726f63";

fn content_id(hex_digit: char) -> ContentId {
    format!("b3:{}", hex_digit.to_string().repeat(64))
        .parse()
        .expect("fixture content id should parse")
}

fn leaf() -> QuickChainTreeLeafNodeV1 {
    QuickChainTreeLeafNodeV1 {
        schema: QUICKCHAIN_TREE_LEAF_NODE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree: QuickChainTreeMaterialKindV1::State,
        sort_key_hex: ALICE_SORT_KEY_HEX.to_string(),
        payload_schema: QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA.to_string(),
        payload_hash: content_id('a'),
    }
}

fn proof_step() -> QuickChainTreeInclusionProofStepV1 {
    QuickChainTreeInclusionProofStepV1 {
        schema: QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree: QuickChainTreeMaterialKindV1::State,
        layer_index: 0,
        sibling_position: QuickChainTreeProofSiblingPositionV1::Right,
        sibling_node_hash: content_id('b'),
    }
}

fn proof() -> QuickChainTreeInclusionProofV1 {
    QuickChainTreeInclusionProofV1 {
        schema: QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        tree: QuickChainTreeMaterialKindV1::State,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1.to_string(),
        algorithm: QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1
            .to_string(),
        source_items_count: 2,
        tree_height: 1,
        root_hash: content_id('c'),
        leaf_index: 0,
        leaf: leaf(),
        steps: vec![proof_step()],
    }
}

#[test]
fn inclusion_proof_dto_validates() {
    let proof = proof();

    proof
        .validate()
        .expect("well-formed inclusion proof should validate");

    assert_eq!(proof.schema, QUICKCHAIN_TREE_INCLUSION_PROOF_SCHEMA);
    assert_eq!(
        proof.steps[0].schema,
        QUICKCHAIN_TREE_INCLUSION_PROOF_STEP_SCHEMA
    );
    assert_eq!(
        proof.steps[0].sibling_position,
        QuickChainTreeProofSiblingPositionV1::Right
    );
}

#[test]
fn inclusion_proof_rejects_empty_tree_or_out_of_range_leaf_index() {
    let mut empty = proof();
    empty.source_items_count = 0;
    empty.tree_height = 0;
    empty.leaf_index = 0;
    empty.steps.clear();

    empty
        .validate()
        .expect_err("inclusion proof for an empty tree must reject");

    let mut out_of_range = proof();
    out_of_range.leaf_index = out_of_range.source_items_count;

    out_of_range
        .validate()
        .expect_err("leaf index outside source count must reject");
}

#[test]
fn inclusion_proof_rejects_domain_or_leaf_tree_mismatch() {
    let mut wrong_domain = proof();
    wrong_domain.tree = QuickChainTreeMaterialKindV1::Receipts;

    wrong_domain
        .validate()
        .expect_err("proof domain must match proof tree");

    let mut wrong_leaf = proof();
    wrong_leaf.leaf.tree = QuickChainTreeMaterialKindV1::Receipts;

    wrong_leaf
        .validate()
        .expect_err("proof leaf tree must match proof tree");
}

#[test]
fn inclusion_proof_rejects_duplicate_or_overheight_step_layers() {
    let mut duplicate_layer = proof();
    duplicate_layer.tree_height = 2;
    duplicate_layer
        .steps
        .push(QuickChainTreeInclusionProofStepV1 {
            layer_index: 0,
            ..proof_step()
        });

    duplicate_layer
        .validate()
        .expect_err("duplicate proof step layers must reject");

    let mut overheight = proof();
    overheight.steps[0].layer_index = overheight.tree_height;

    overheight
        .validate()
        .expect_err("proof step layer at tree height must reject");
}

#[test]
fn inclusion_proof_rejects_bad_single_item_height_shape() {
    let mut proof = proof();
    proof.source_items_count = 1;
    proof.tree_height = 1;
    proof.steps.clear();

    proof
        .validate()
        .expect_err("single-item proof must have height zero");
}

#[test]
fn inclusion_proof_dtos_reject_unknown_fields() {
    let mut step_value = serde_json::to_value(proof_step()).unwrap();
    step_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeInclusionProofStepV1>(step_value)
        .expect_err("proof step must reject unknown fields");

    let mut proof_value = serde_json::to_value(proof()).unwrap();
    proof_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeInclusionProofV1>(proof_value)
        .expect_err("proof must reject unknown fields");
}
