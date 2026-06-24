//! RO:WHAT — Phase 2 Round 1 DTO tests for verifier replay bundles and replay results.
//! RO:WHY — ECON/GOV: replicated verification starts with strict artifacts before committee signing or quorum semantics.
//! RO:INTERACTS — ron-proto quickchain verifier and tree-material DTOs.
//! RO:INVARIANTS — unknown fields reject; one root per material tree; no fake finality, staking, bridge, or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture hashes are inert test artifacts.
//! RO:TEST — this file.

use ron_proto::{
    quickchain::{
        quickchain_tree_material_json_v1_encoding, quickchain_tree_root_domain_for_tree,
        QuickChainTreeMaterialBatchV1, QuickChainTreeMaterialItemV1, QuickChainTreeMaterialKindV1,
        QuickChainTreeRootV1, QuickChainVerifierCheckStatusV1, QuickChainVerifierReplayBundleV1,
        QuickChainVerifierReplayResultV1, QuickChainVerifierReplayStatusV1,
        QuickChainVerifierRootCheckV1, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
        QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1,
        QUICKCHAIN_TREE_ROOT_SCHEMA, QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1,
        QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA, QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA,
        QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0002";
const ALICE_SORT_KEY: &str = "6163636f756e743a616c69636500726f63";
const BOB_SORT_KEY: &str = "6163636f756e743a626f6200726f63";

fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn material_batch() -> QuickChainTreeMaterialBatchV1 {
    QuickChainTreeMaterialBatchV1 {
        schema: QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        tree: QuickChainTreeMaterialKindV1::State,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        items: vec![
            QuickChainTreeMaterialItemV1 {
                schema: QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA.to_string(),
                version: QUICKCHAIN_DTO_VERSION,
                tree: QuickChainTreeMaterialKindV1::State,
                sort_key_hex: ALICE_SORT_KEY.to_string(),
                payload_schema: QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA.to_string(),
                payload_hash: test_content_id("alice-leaf"),
            },
            QuickChainTreeMaterialItemV1 {
                schema: QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA.to_string(),
                version: QUICKCHAIN_DTO_VERSION,
                tree: QuickChainTreeMaterialKindV1::State,
                sort_key_hex: BOB_SORT_KEY.to_string(),
                payload_schema: QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA.to_string(),
                payload_hash: test_content_id("bob-leaf"),
            },
        ],
    }
}

fn expected_root(label: &str) -> QuickChainTreeRootV1 {
    QuickChainTreeRootV1 {
        schema: QUICKCHAIN_TREE_ROOT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        tree: QuickChainTreeMaterialKindV1::State,
        canonical_encoding: quickchain_tree_material_json_v1_encoding(),
        item_sort_rule: QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1.to_string(),
        pair_rule: QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1.to_string(),
        hash_domain: quickchain_tree_root_domain_for_tree(QuickChainTreeMaterialKindV1::State)
            .to_string(),
        algorithm: QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1
            .to_string(),
        source_items_count: 2,
        tree_height: 1,
        root_hash: test_content_id(label),
    }
}

fn replay_bundle() -> QuickChainVerifierReplayBundleV1 {
    QuickChainVerifierReplayBundleV1 {
        schema: QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        replay_algorithm: QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1.to_string(),
        material_batches: vec![material_batch()],
        expected_roots: vec![expected_root("expected-state-root")],
        inclusion_proofs: Vec::new(),
    }
}

#[test]
fn verifier_replay_bundle_accepts_one_material_tree_and_expected_root() {
    let bundle = replay_bundle();

    bundle
        .validate()
        .expect("valid replay bundle should satisfy strict DTO validation");

    assert_eq!(bundle.schema, QUICKCHAIN_VERIFIER_REPLAY_BUNDLE_SCHEMA);
    assert_eq!(bundle.material_batches.len(), 1);
    assert_eq!(bundle.expected_roots.len(), 1);
    assert_eq!(bundle.inclusion_proofs.len(), 0);
}

#[test]
fn verifier_replay_bundle_rejects_missing_expected_root() {
    let mut bundle = replay_bundle();
    bundle.expected_roots.clear();

    let error = bundle
        .validate()
        .expect_err("bundle without root for material must reject")
        .to_string();

    assert!(
        error.contains("expected_roots"),
        "unexpected error: {error}"
    );
}

#[test]
fn verifier_replay_bundle_rejects_duplicate_material_tree() {
    let mut bundle = replay_bundle();
    bundle.material_batches.push(material_batch());
    bundle.expected_roots.push(expected_root("duplicate-root"));

    let error = bundle
        .validate()
        .expect_err("duplicate material trees must reject")
        .to_string();

    assert!(
        error.contains("material_batches.tree"),
        "unexpected error: {error}"
    );
}

#[test]
fn verifier_replay_result_status_must_match_checks() {
    let result = QuickChainVerifierReplayResultV1 {
        schema: QUICKCHAIN_VERIFIER_REPLAY_RESULT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        replay_algorithm: QUICKCHAIN_VERIFIER_REPLAY_ALGORITHM_READ_ONLY_ROOT_PROOF_V1.to_string(),
        status: QuickChainVerifierReplayStatusV1::Verified,
        material_batches_count: 1,
        expected_roots_count: 1,
        inclusion_proofs_count: 0,
        root_checks: vec![QuickChainVerifierRootCheckV1 {
            schema: QUICKCHAIN_VERIFIER_ROOT_CHECK_SCHEMA.to_string(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: CHAIN_ID.to_string(),
            epoch_id: EPOCH_ID.to_string(),
            tree: QuickChainTreeMaterialKindV1::State,
            expected_root_hash: test_content_id("same-root"),
            recomputed_root_hash: test_content_id("same-root"),
            status: QuickChainVerifierCheckStatusV1::Verified,
            detail: None,
        }],
        proof_checks: Vec::new(),
    };

    result
        .validate()
        .expect("verified result with matching root check should validate");

    let mut invalid = result;
    invalid.status = QuickChainVerifierReplayStatusV1::Mismatch;

    let error = invalid
        .validate()
        .expect_err("mismatch result without mismatched checks must reject")
        .to_string();

    assert!(error.contains("status"), "unexpected error: {error}");
}

#[test]
fn verifier_replay_bundle_rejects_unknown_fields() {
    let bundle = replay_bundle();
    let mut value = serde_json::to_value(bundle).expect("bundle should serialize");
    value
        .as_object_mut()
        .expect("bundle should serialize as object")
        .insert("unexpected".to_string(), serde_json::json!(true));

    let error = serde_json::from_value::<QuickChainVerifierReplayBundleV1>(value)
        .expect_err("unknown fields must reject");

    assert!(
        error.to_string().contains("unknown field"),
        "unexpected error: {error}"
    );
}
