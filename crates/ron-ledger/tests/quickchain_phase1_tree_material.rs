#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 1 tests for deterministic tree-material assembly and local root projection.
//! RO:WHY — ECON/GOV: Phase 1 needs sorted material, domain-separated BLAKE3 roots, and locked vector hashes before downstream display.
//! RO:INTERACTS — ron-ledger quickchain tree-material projection and ron-proto tree-material/root DTOs.
//! RO:INVARIANTS — unordered input becomes same root; duplicates reject; no DB order; no wall-clock; no checkpoints, validators, anchors, or finality.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — fixture b3 commitments and roots are inert test artifacts, not authoritative receipts, spend authority, or settlement finality.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_tree_material_batch, build_tree_reduction_plan_from_batch, compute_tree_root_from_batch,
    QuickChainTreeMaterialProjectionError, QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        quickchain_tree_material_json_v1_encoding, quickchain_tree_root_domain_for_tree,
        QuickChainTreeMaterialKindV1, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA, QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1,
        QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1,
        QUICKCHAIN_TREE_ROOT_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0001";

fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn material_item(
    sort_key_hex: &str,
    payload_schema: &str,
    label: &str,
) -> QuickChainTreeMaterialProjectionItem {
    QuickChainTreeMaterialProjectionItem::new(sort_key_hex, payload_schema, test_content_id(label))
}

#[test]
fn unordered_account_material_is_sorted_into_phase1_input_order() {
    let bob_sort_key = "6163636f756e743a626f6200726f63";
    let alice_sort_key = "6163636f756e743a616c69636500726f63";

    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
        ],
    )
    .expect("unordered explicit material should assemble into sorted batch");

    batch
        .validate()
        .expect("assembled tree material batch must satisfy ron-proto");

    assert_eq!(batch.schema, QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA);
    assert_eq!(batch.chain_id, CHAIN_ID);
    assert_eq!(batch.epoch_id, EPOCH_ID);
    assert_eq!(batch.tree, QuickChainTreeMaterialKindV1::State);
    assert_eq!(
        batch.canonical_encoding,
        quickchain_tree_material_json_v1_encoding()
    );
    assert_eq!(
        batch.item_sort_rule,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1
    );

    assert_eq!(batch.items.len(), 2);
    assert_eq!(batch.items[0].sort_key_hex, alice_sort_key);
    assert_eq!(batch.items[1].sort_key_hex, bob_sort_key);
    assert_eq!(
        batch.items[0].payload_schema,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA
    );
    assert_eq!(
        batch.items[1].payload_schema,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA
    );
}

#[test]
fn duplicate_material_sort_keys_reject_before_batch_is_returned() {
    let duplicate = "6163636f756e743a616c69636500726f63";

    let error = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                duplicate,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf-a",
            ),
            material_item(
                duplicate,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf-b",
            ),
        ],
    )
    .expect_err("duplicate material keys must reject");

    assert_eq!(
        error,
        QuickChainTreeMaterialProjectionError::DuplicateSortKey {
            sort_key_hex: duplicate.to_string(),
        }
    );
}

#[test]
fn schema_tree_mismatch_rejects_at_ron_proto_boundary() {
    let error = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![material_item(
            "6163636f756e743a616c69636500726f63",
            QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
            "receipt-as-state-leaf",
        )],
    )
    .expect_err("state material cannot carry receipt payload schema");

    match error {
        QuickChainTreeMaterialProjectionError::InvalidBatch { reason } => {
            assert!(
                reason.contains("payload_schema") || reason.contains("invalid field"),
                "unexpected validation reason: {reason}"
            );
        }
        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn material_batch_builds_first_adjacent_reduction_plan_with_odd_carry() {
    let alice_sort_key = "6163636f756e743a616c69636500726f63";
    let bob_sort_key = "6163636f756e743a626f6200726f63";
    let carol_sort_key = "6163636f756e743a6361726f6c00726f63";

    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                carol_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "carol-leaf",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
        ],
    )
    .expect("unordered explicit material should assemble into sorted batch");

    let plan = build_tree_reduction_plan_from_batch(&batch)
        .expect("validated sorted material should assemble into a reduction plan");

    plan.validate()
        .expect("assembled reduction plan must satisfy ron-proto");

    assert_eq!(plan.schema, QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA);
    assert_eq!(plan.chain_id, CHAIN_ID);
    assert_eq!(plan.epoch_id, EPOCH_ID);
    assert_eq!(plan.tree, QuickChainTreeMaterialKindV1::State);
    assert_eq!(plan.source_items_count, 3);
    assert_eq!(
        plan.pair_rule,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1
    );
    assert_eq!(plan.pairs.len(), 2);

    assert_eq!(plan.pairs[0].layer_index, 0);
    assert_eq!(plan.pairs[0].pair_index, 0);
    assert_eq!(plan.pairs[0].left_sort_key_hex, alice_sort_key);
    assert_eq!(
        plan.pairs[0].right_sort_key_hex.as_deref(),
        Some(bob_sort_key)
    );
    assert!(plan.pairs[0].has_right_member());

    assert_eq!(plan.pairs[1].layer_index, 0);
    assert_eq!(plan.pairs[1].pair_index, 1);
    assert_eq!(plan.pairs[1].left_sort_key_hex, carol_sort_key);
    assert_eq!(plan.pairs[1].right_sort_key_hex, None);
    assert_eq!(plan.pairs[1].right_payload_hash, None);
    assert!(!plan.pairs[1].has_right_member());
}

#[test]
fn empty_material_batch_builds_empty_reduction_plan() {
    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::Receipts,
        Vec::<QuickChainTreeMaterialProjectionItem>::new(),
    )
    .expect("empty explicit material is a valid empty vector fixture");

    let plan = build_tree_reduction_plan_from_batch(&batch)
        .expect("empty material should assemble into empty reduction plan");

    assert_eq!(plan.source_items_count, 0);
    assert!(plan.pairs.is_empty());
    assert_eq!(plan.tree, QuickChainTreeMaterialKindV1::Receipts);
}

#[test]
fn invalid_material_batch_rejects_before_reduction_plan_is_returned() {
    let alice_sort_key = "6163636f756e743a616c69636500726f63";
    let bob_sort_key = "6163636f756e743a626f6200726f63";

    let mut batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
        ],
    )
    .expect("valid material batch should build first");

    batch.items[1].sort_key_hex = batch.items[0].sort_key_hex.clone();

    let error = build_tree_reduction_plan_from_batch(&batch)
        .expect_err("invalid material must reject before reduction plan is returned");

    match error {
        QuickChainTreeMaterialProjectionError::InvalidBatch { reason } => {
            assert!(
                reason.contains("sort_key") || reason.contains("invalid field"),
                "unexpected validation reason: {reason}"
            );
        }
        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn deterministic_state_root_ignores_caller_input_order() {
    let alice_sort_key = "6163636f756e743a616c69636500726f63";
    let bob_sort_key = "6163636f756e743a626f6200726f63";
    let carol_sort_key = "6163636f756e743a6361726f6c00726f63";

    let forward = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
            material_item(
                carol_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "carol-leaf",
            ),
        ],
    )
    .expect("forward material should build");

    let shuffled = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                carol_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "carol-leaf",
            ),
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
        ],
    )
    .expect("shuffled material should build");

    let forward_root = compute_tree_root_from_batch(&forward).expect("forward root should compute");
    let shuffled_root =
        compute_tree_root_from_batch(&shuffled).expect("shuffled root should compute");

    forward_root
        .validate()
        .expect("computed root must satisfy ron-proto");
    shuffled_root
        .validate()
        .expect("computed root must satisfy ron-proto");

    assert_eq!(forward_root, shuffled_root);
    assert_eq!(forward_root.schema, QUICKCHAIN_TREE_ROOT_SCHEMA);
    assert_eq!(forward_root.chain_id, CHAIN_ID);
    assert_eq!(forward_root.epoch_id, EPOCH_ID);
    assert_eq!(forward_root.tree, QuickChainTreeMaterialKindV1::State);
    assert_eq!(forward_root.source_items_count, 3);
    assert_eq!(forward_root.tree_height, 2);
    assert_eq!(
        forward_root.hash_domain,
        quickchain_tree_root_domain_for_tree(QuickChainTreeMaterialKindV1::State)
    );
    assert_eq!(
        forward_root.algorithm,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1
    );
    assert_eq!(
        forward_root.root_hash.to_string(),
        "b3:fb534c250d95f3d0474e3129af481eaefd4f983b7f485047c55bdda532115bfc"
    );
}

#[test]
fn locked_two_leaf_state_root_vector_matches_real_b3_hash() {
    let alice_sort_key = "6163636f756e743a616c69636500726f63";
    let bob_sort_key = "6163636f756e743a626f6200726f63";

    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
        ],
    )
    .expect("unordered two-leaf material should build");

    let root = compute_tree_root_from_batch(&batch).expect("state root should compute");

    assert_eq!(root.schema, QUICKCHAIN_TREE_ROOT_SCHEMA);
    assert_eq!(root.source_items_count, 2);
    assert_eq!(root.tree_height, 1);
    assert_eq!(
        root.root_hash.to_string(),
        "b3:f1810151cc24d5865bdb5692d919588ef4d3f822f50952fa62b41b40d2a73a3f"
    );
}

#[test]
fn empty_receipt_root_vector_uses_explicit_empty_payload_hash() {
    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::Receipts,
        Vec::<QuickChainTreeMaterialProjectionItem>::new(),
    )
    .expect("empty receipts material should build");

    let root = compute_tree_root_from_batch(&batch).expect("empty receipt root should compute");

    assert_eq!(root.schema, QUICKCHAIN_TREE_ROOT_SCHEMA);
    assert_eq!(root.tree, QuickChainTreeMaterialKindV1::Receipts);
    assert_eq!(root.source_items_count, 0);
    assert_eq!(root.tree_height, 0);
    assert_eq!(
        root.hash_domain,
        quickchain_tree_root_domain_for_tree(QuickChainTreeMaterialKindV1::Receipts)
    );
    assert_eq!(
        root.root_hash.to_string(),
        "b3:e0fe50b12d2dc0ea1e694a411d0968ae540c300ead7de5155b4ea0d8085e9af9"
    );
}

#[test]
fn root_changes_when_leaf_commitment_changes() {
    let alice_sort_key = "6163636f756e743a616c69636500726f63";
    let bob_sort_key = "6163636f756e743a626f6200726f63";

    let original = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
        ],
    )
    .expect("original material should build");

    let changed = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf-mutated",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
        ],
    )
    .expect("changed material should build");

    let original_root =
        compute_tree_root_from_batch(&original).expect("original root should compute");
    let changed_root = compute_tree_root_from_batch(&changed).expect("changed root should compute");

    assert_ne!(original_root.root_hash, changed_root.root_hash);
}

#[test]
fn invalid_material_batch_rejects_before_root_is_returned() {
    let alice_sort_key = "6163636f756e743a616c69636500726f63";
    let bob_sort_key = "6163636f756e743a626f6200726f63";

    let mut batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::State,
        vec![
            material_item(
                alice_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "alice-leaf",
            ),
            material_item(
                bob_sort_key,
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                "bob-leaf",
            ),
        ],
    )
    .expect("valid material batch should build first");

    batch.items.reverse();

    let error = compute_tree_root_from_batch(&batch)
        .expect_err("invalid material must reject before root is returned");

    match error {
        QuickChainTreeMaterialProjectionError::InvalidBatch { reason } => {
            assert!(
                reason.contains("sort_key") || reason.contains("invalid field"),
                "unexpected validation reason: {reason}"
            );
        }
        other => panic!("unexpected projection error: {other:?}"),
    }
}
