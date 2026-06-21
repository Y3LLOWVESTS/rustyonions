#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — QC-1A tests for deterministic tree-material assembly without root production.
//! RO:WHY — ECON/GOV: Phase 1 begins by proving sorted material inputs before reducing them.
//! RO:INTERACTS — ron-ledger quickchain tree-material projection and ron-proto tree-material DTOs.
//! RO:INVARIANTS — unordered input becomes sorted output; duplicates reject; no hashing, roots, checkpoints, or proof verification.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — fixture b3 commitments are inert test data, not authoritative receipts or roots.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_tree_material_batch, QuickChainTreeMaterialProjectionError,
    QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        quickchain_tree_material_json_v1_encoding, QuickChainTreeMaterialKindV1,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
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
