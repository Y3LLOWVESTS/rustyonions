//! RO:WHAT — Tests for strict QuickChain tree-material and tree-root DTOs.
//! RO:WHY — ECON/GOV: Phase 1 roots need sorted, reviewable material manifests and root payload contracts.
//! RO:INTERACTS — ron_proto::quickchain::root_material and existing hash-payload/domain schema constants.
//! RO:INVARIANTS — ron-proto validates DTO/hash-domain shape only; no hashing implementation; no checkpoints, validators, anchors, or settlement.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture b3 values are opaque data, not spend authority, proof verification, or finality.
//! RO:TEST — this file.

use ron_proto::{
    quickchain::{
        quickchain_tree_root_domain_for_tree, QuickChainCanonicalEncodingV1,
        QuickChainTreeBranchNodeV1, QuickChainTreeLeafNodeV1, QuickChainTreeMaterialBatchV1,
        QuickChainTreeMaterialItemV1, QuickChainTreeMaterialKindV1, QuickChainTreeReductionPairV1,
        QuickChainTreeReductionPlanV1, QuickChainTreeRootPayloadV1, QuickChainTreeRootV1,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA, QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA, QUICKCHAIN_TREE_LEAF_NODE_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA, QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA, QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1,
        QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA, QUICKCHAIN_TREE_ROOT_SCHEMA,
    },
    ContentId,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0001";
const ALICE_SORT_KEY_HEX: &str = "6163636f756e743a616c69636500726f63";
const BOB_SORT_KEY_HEX: &str = "6163636f756e743a626f6200726f63";

fn content_id(hex_digit: char) -> ContentId {
    format!("b3:{}", hex_digit.to_string().repeat(64))
        .parse()
        .expect("fixture content id should parse")
}

fn item(
    tree: QuickChainTreeMaterialKindV1,
    sort_key_hex: &str,
    payload_schema: &str,
    payload_hash_digit: char,
) -> QuickChainTreeMaterialItemV1 {
    QuickChainTreeMaterialItemV1 {
        schema: QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree,
        sort_key_hex: sort_key_hex.to_string(),
        payload_schema: payload_schema.to_string(),
        payload_hash: content_id(payload_hash_digit),
    }
}

fn valid_state_batch() -> QuickChainTreeMaterialBatchV1 {
    QuickChainTreeMaterialBatchV1 {
        schema: QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        tree: QuickChainTreeMaterialKindV1::State,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        items: vec![
            item(
                QuickChainTreeMaterialKindV1::State,
                ALICE_SORT_KEY_HEX,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                'a',
            ),
            item(
                QuickChainTreeMaterialKindV1::State,
                BOB_SORT_KEY_HEX,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                'b',
            ),
        ],
    }
}

fn reduction_pair(
    pair_index: u32,
    left_sort_key_hex: &str,
    left_payload_digit: char,
    right_sort_key_hex: Option<&str>,
    right_payload_digit: Option<char>,
) -> QuickChainTreeReductionPairV1 {
    QuickChainTreeReductionPairV1 {
        schema: QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree: QuickChainTreeMaterialKindV1::State,
        layer_index: 0,
        pair_index,
        left_sort_key_hex: left_sort_key_hex.to_string(),
        left_payload_hash: content_id(left_payload_digit),
        right_sort_key_hex: right_sort_key_hex.map(str::to_string),
        right_payload_hash: right_payload_digit.map(content_id),
    }
}

fn valid_reduction_plan() -> QuickChainTreeReductionPlanV1 {
    QuickChainTreeReductionPlanV1 {
        schema: QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        tree: QuickChainTreeMaterialKindV1::State,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        source_items_count: 3,
        pairs: vec![
            reduction_pair(
                0,
                ALICE_SORT_KEY_HEX,
                'a',
                Some(BOB_SORT_KEY_HEX),
                Some('b'),
            ),
            reduction_pair(1, "6163636f756e743a6361726f6c00726f63", 'c', None, None),
        ],
    }
}

fn valid_root_payload() -> QuickChainTreeRootPayloadV1 {
    QuickChainTreeRootPayloadV1 {
        schema: QUICKCHAIN_TREE_ROOT_PAYLOAD_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        tree: QuickChainTreeMaterialKindV1::State,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1.to_string(),
        source_items_count: 2,
        tree_height: 1,
        root_node_hash: Some(content_id('c')),
    }
}

fn valid_root() -> QuickChainTreeRootV1 {
    QuickChainTreeRootV1 {
        schema: QUICKCHAIN_TREE_ROOT_SCHEMA.to_string(),
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
        root_hash: content_id('d'),
    }
}

#[test]
fn sorted_state_material_batch_validates() {
    let batch = valid_state_batch();

    batch
        .validate()
        .expect("sorted state material batch should validate");

    assert_eq!(batch.schema, QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA);
    assert_eq!(batch.version, QUICKCHAIN_DTO_VERSION);
    assert_eq!(batch.chain_id, CHAIN_ID);
    assert_eq!(batch.epoch_id, EPOCH_ID);
    assert_eq!(batch.tree, QuickChainTreeMaterialKindV1::State);
    assert_eq!(
        batch.canonical_encoding,
        QuickChainCanonicalEncodingV1::JsonV1
    );
    assert_eq!(
        batch.item_sort_rule,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1
    );
}

#[test]
fn material_batch_rejects_unsorted_or_duplicate_sort_keys() {
    let mut unsorted = valid_state_batch();
    unsorted.items.reverse();

    unsorted
        .validate()
        .expect_err("unsorted material items must reject");

    let mut duplicate = valid_state_batch();
    duplicate.items[1].sort_key_hex = duplicate.items[0].sort_key_hex.clone();

    duplicate
        .validate()
        .expect_err("duplicate material sort keys must reject");
}

#[test]
fn material_item_rejects_uppercase_or_odd_hex_sort_key() {
    let uppercase = item(
        QuickChainTreeMaterialKindV1::State,
        "AA",
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        'a',
    );

    uppercase
        .validate()
        .expect_err("uppercase hex sort key must reject");

    let odd = item(
        QuickChainTreeMaterialKindV1::State,
        "abc",
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        'a',
    );

    odd.validate()
        .expect_err("odd-length hex sort key must reject");
}

#[test]
fn material_batch_rejects_payload_schema_tree_mismatch() {
    let mut batch = valid_state_batch();
    batch.items[0].payload_schema = QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA.to_string();

    batch
        .validate()
        .expect_err("state material cannot carry receipt payload schema");
}

#[test]
fn reduction_plan_validates_adjacent_pairs_with_final_odd_carry() {
    let plan = valid_reduction_plan();

    plan.validate()
        .expect("well-formed material reduction plan should validate");

    assert_eq!(plan.schema, QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA);
    assert_eq!(
        plan.pair_rule,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1
    );
    assert_eq!(plan.source_items_count, 3);
    assert_eq!(plan.pairs.len(), 2);
    assert!(plan.pairs[0].has_right_member());
    assert!(!plan.pairs[1].has_right_member());
}

#[test]
fn reduction_pair_rejects_partial_or_reversed_right_member() {
    let partial = reduction_pair(0, ALICE_SORT_KEY_HEX, 'a', Some(BOB_SORT_KEY_HEX), None);

    partial
        .validate()
        .expect_err("right sort key without right payload hash must reject");

    let reversed = reduction_pair(
        0,
        BOB_SORT_KEY_HEX,
        'b',
        Some(ALICE_SORT_KEY_HEX),
        Some('a'),
    );

    reversed
        .validate()
        .expect_err("right sort key must be greater than left sort key");
}

#[test]
fn reduction_plan_rejects_nonfinal_carry_or_pair_count_mismatch() {
    let mut nonfinal_carry = valid_reduction_plan();
    nonfinal_carry.pairs[0].right_sort_key_hex = None;
    nonfinal_carry.pairs[0].right_payload_hash = None;

    nonfinal_carry
        .validate()
        .expect_err("only final odd pair may omit a right member");

    let mut bad_pair_count = valid_reduction_plan();
    bad_pair_count.source_items_count = 4;

    bad_pair_count
        .validate()
        .expect_err("declared source item count must match pair count");
}

#[test]
fn tree_leaf_branch_root_payload_and_root_dtos_validate() {
    let leaf = QuickChainTreeLeafNodeV1 {
        schema: QUICKCHAIN_TREE_LEAF_NODE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree: QuickChainTreeMaterialKindV1::State,
        sort_key_hex: ALICE_SORT_KEY_HEX.to_string(),
        payload_schema: QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA.to_string(),
        payload_hash: content_id('a'),
    };
    leaf.validate()
        .expect("state leaf node with account payload schema should validate");

    let branch = QuickChainTreeBranchNodeV1 {
        schema: QUICKCHAIN_TREE_BRANCH_NODE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree: QuickChainTreeMaterialKindV1::State,
        layer_index: 0,
        pair_index: 0,
        left_node_hash: content_id('a'),
        right_node_hash: content_id('b'),
    };
    branch
        .validate()
        .expect("branch node with two child hashes should validate");

    let payload = valid_root_payload();
    payload
        .validate()
        .expect("well-formed tree-root payload should validate");

    let root = valid_root();
    root.validate()
        .expect("well-formed tree-root result should validate");

    assert_eq!(
        root.hash_domain,
        quickchain_tree_root_domain_for_tree(QuickChainTreeMaterialKindV1::State)
    );
    assert_eq!(
        root.algorithm,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1
    );
}

#[test]
fn root_payload_rejects_domain_tree_mismatch_and_bad_empty_shape() {
    let mut wrong_domain = valid_root_payload();
    wrong_domain.tree = QuickChainTreeMaterialKindV1::Receipts;

    wrong_domain
        .validate()
        .expect_err("root payload hash domain must match tree kind");

    let mut empty_with_hash = valid_root_payload();
    empty_with_hash.source_items_count = 0;
    empty_with_hash.tree_height = 0;

    empty_with_hash
        .validate()
        .expect_err("empty root payload must use null root_node_hash");

    let mut nonempty_without_hash = valid_root_payload();
    nonempty_without_hash.root_node_hash = None;

    nonempty_without_hash
        .validate()
        .expect_err("non-empty root payload must carry a final node hash");
}

#[test]
fn tree_root_rejects_unsupported_algorithm_and_height_shape() {
    let mut bad_algorithm = valid_root();
    bad_algorithm.algorithm = "sorted_binary_merkle_map_sha256_json_v1".to_string();

    bad_algorithm
        .validate()
        .expect_err("unsupported root algorithm must reject");

    let mut bad_height = valid_root();
    bad_height.source_items_count = 2;
    bad_height.tree_height = 0;

    bad_height
        .validate()
        .expect_err("multi-item tree root must have non-zero height");
}

#[test]
fn material_and_reduction_dtos_reject_unknown_fields() {
    let mut batch_value = serde_json::to_value(valid_state_batch()).unwrap();
    batch_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeMaterialBatchV1>(batch_value)
        .expect_err("material batch must reject unknown fields");

    let mut item_value = serde_json::to_value(item(
        QuickChainTreeMaterialKindV1::State,
        ALICE_SORT_KEY_HEX,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        'a',
    ))
    .unwrap();
    item_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeMaterialItemV1>(item_value)
        .expect_err("material item must reject unknown fields");

    let mut plan_value = serde_json::to_value(valid_reduction_plan()).unwrap();
    plan_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeReductionPlanV1>(plan_value)
        .expect_err("reduction plan must reject unknown fields");
}

#[test]
fn tree_root_dtos_reject_unknown_fields() {
    let mut leaf_value = serde_json::to_value(QuickChainTreeLeafNodeV1 {
        schema: QUICKCHAIN_TREE_LEAF_NODE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree: QuickChainTreeMaterialKindV1::State,
        sort_key_hex: ALICE_SORT_KEY_HEX.to_string(),
        payload_schema: QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA.to_string(),
        payload_hash: content_id('a'),
    })
    .unwrap();
    leaf_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeLeafNodeV1>(leaf_value)
        .expect_err("tree leaf node must reject unknown fields");

    let mut root_payload_value = serde_json::to_value(valid_root_payload()).unwrap();
    root_payload_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeRootPayloadV1>(root_payload_value)
        .expect_err("tree root payload must reject unknown fields");

    let mut root_value = serde_json::to_value(valid_root()).unwrap();
    root_value["unexpected"] = json!(true);

    serde_json::from_value::<QuickChainTreeRootV1>(root_value)
        .expect_err("tree root result must reject unknown fields");
}
