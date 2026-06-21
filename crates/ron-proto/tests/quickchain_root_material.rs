//! RO:WHAT — Tests for strict QuickChain tree-material DTOs.
//! RO:WHY — ECON/GOV: Phase 1 root preparation needs sorted, reviewable material manifests before any root producer exists.
//! RO:INTERACTS — ron_proto::quickchain::root_material and existing hash-payload schema constants.
//! RO:INVARIANTS — no hashes are computed; no roots are produced; unknown fields reject; sorted keys are strict.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture b3 values are opaque data, not spend authority or proof verification.
//! RO:TEST — this file.

use ron_proto::{
    quickchain::{
        QuickChainCanonicalEncodingV1, QuickChainTreeMaterialBatchV1, QuickChainTreeMaterialItemV1,
        QuickChainTreeMaterialKindV1, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA, QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
    },
    ContentId,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0001";

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
                "6163636f756e743a616c69636500726f63",
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                'a',
            ),
            item(
                QuickChainTreeMaterialKindV1::State,
                "6163636f756e743a626f6200726f63",
                QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
                'b',
            ),
        ],
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
fn material_dtos_reject_unknown_fields() {
    let mut batch_value = serde_json::to_value(valid_state_batch()).unwrap();
    batch_value
        .as_object_mut()
        .unwrap()
        .insert("invented_root".to_string(), json!("b3:placeholder"));

    serde_json::from_value::<QuickChainTreeMaterialBatchV1>(batch_value)
        .expect_err("batch DTO must reject unknown fields");

    let mut item_value = serde_json::to_value(item(
        QuickChainTreeMaterialKindV1::State,
        "00",
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        'a',
    ))
    .unwrap();

    item_value
        .as_object_mut()
        .unwrap()
        .insert("proof".to_string(), json!([]));

    serde_json::from_value::<QuickChainTreeMaterialItemV1>(item_value)
        .expect_err("item DTO must reject unknown fields");
}
