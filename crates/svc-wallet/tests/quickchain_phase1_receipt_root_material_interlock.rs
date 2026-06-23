#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 1 Round 2 interlock proving wallet receipts can feed receipt-root material safely.
//! RO:WHY — svc-wallet is mutation front-door only; Phase 1 roots/proofs must consume backend-derived receipt evidence without making wallet a root authority.
//! RO:INTERACTS — svc_wallet::quickchain projection, ron-ledger tree material/proof helpers, ron-proto QuickChain receipt/root DTOs.
//! RO:INVARIANTS — wallet receipts are accepted-only evidence; root/proof material is deterministic; no wallet-side finality, validator, anchor, bridge, or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — client/idempotency data never becomes operation authority; roots/proofs are evidence only and not spend authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_phase1_receipt_root_material_interlock.

use ron_ledger::quickchain::{
    build_tree_inclusion_proof_from_batch, build_tree_material_batch, compute_tree_root_from_batch,
    project_receipt_hash_payload, verify_tree_inclusion_proof, QuickChainCommittedOperationRecord,
    QuickChainReceiptHashProjectionContext, QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        quickchain_tree_root_domain_for_tree, to_canonical_json_vec, QuickChainOperationClassV1,
        QuickChainOperationIntentV1, QuickChainTreeMaterialKindV1, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_OPERATION_INTENT_SCHEMA, QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1,
    },
    ContentId,
};
use serde::Serialize;
use serde_json::{json, Value};
use svc_wallet::{
    dto::{
        requests::AmountMinor,
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjection,
        QuickChainWalletReceiptProjectionContext, QuickChainWalletReceiptStatus,
    },
    util::blake3_receipt::finalize_receipt,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0001";

#[derive(Debug, Clone)]
struct ReceiptMaterialFixture {
    projection: QuickChainWalletReceiptProjection,
    sort_key_hex: String,
    payload_hash: ContentId,
    item: QuickChainTreeMaterialProjectionItem,
}

fn repeated_hex(ch: char, len: usize) -> String {
    ch.to_string().repeat(len)
}

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", repeated_hex(hex_digit, 32))
}

fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("test BLAKE3 content ID should parse")
}

fn hash_quickchain_payload<T>(domain: &str, payload: &T) -> ContentId
where
    T: Serialize,
{
    let canonical_payload_bytes =
        to_canonical_json_vec(payload).expect("QuickChain payload should serialize canonically");

    let mut preimage = Vec::with_capacity(domain.len() + 1 + canonical_payload_bytes.len());
    preimage.extend_from_slice(domain.as_bytes());
    preimage.push(0x00);
    preimage.extend_from_slice(&canonical_payload_bytes);

    let digest = blake3::hash(&preimage).to_hex().to_string();
    format!("b3:{digest}")
        .parse()
        .expect("domain-separated QuickChain payload hash should parse")
}

fn receipt_sort_key_hex(operation_id: &str) -> String {
    let mut bytes = b"receipt\0".to_vec();
    bytes.extend_from_slice(operation_id.as_bytes());
    hex::encode(bytes)
}

fn wallet_receipt(
    hex_digit: char,
    from: &str,
    to: &str,
    amount_minor: u128,
    ledger_seq_start: u64,
    ledger_seq_end: u64,
) -> Receipt {
    let short = repeated_hex(hex_digit, 8);

    finalize_receipt(Receipt {
        txid: format!("tx:qc1r2:{short}"),
        op: WalletOp::Transfer,
        from: Some(from.to_string()),
        to: Some(to.to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(amount_minor),
        nonce: Some(1),
        idem: format!("idem:qc1r2:{short}"),
        ts: 1_777_309_851_000 + ledger_seq_start,
        ledger_seq_start: Some(ledger_seq_start),
        ledger_seq_end: Some(ledger_seq_end),
        ledger_root: repeated_hex(hex_digit, 64),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("wallet receipt fixture should finalize")
}

fn receipt_material_fixture(
    hex_digit: char,
    from: &str,
    to: &str,
    amount_minor: u128,
    account_sequence: u64,
    ledger_seq_start: u64,
    ledger_seq_end: u64,
) -> ReceiptMaterialFixture {
    let operation_id = operation_id(hex_digit);
    let wallet_receipt = wallet_receipt(
        hex_digit,
        from,
        to,
        amount_minor,
        ledger_seq_start,
        ledger_seq_end,
    );

    let wallet_context =
        QuickChainWalletReceiptProjectionContext::accepted(CHAIN_ID, operation_id.clone())
            .expect("wallet projection context should validate");

    let projection =
        project_wallet_receipt_for_quickchain_preflight(&wallet_receipt, &wallet_context)
            .expect("backend-derived wallet receipt should project");

    let intent = QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: projection.operation_id.clone(),
        idempotency_key: projection.idempotency_key.clone(),
        op_class: QuickChainOperationClassV1::Transfer,
        actor_account_id: projection
            .from
            .clone()
            .expect("transfer receipt should carry debit account"),
        counterparty_account_id: projection.to.clone(),
        amount_minor: Some(projection.amount_minor.get().to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms: projection.produced_at_ms,
    };

    let committed = QuickChainCommittedOperationRecord::new(
        intent,
        projection.txid.clone(),
        account_sequence,
        projection.ledger_seq_start,
        projection.ledger_seq_end,
    )
    .expect("committed QuickChain receipt evidence should validate");

    let receipt_context = QuickChainReceiptHashProjectionContext::new(
        projection.operation_id.clone(),
        test_content_id(&format!("{}:operation-hash", projection.operation_id)),
        projection.op.as_str(),
        None,
        test_content_id(&format!("{}:previous-ledger-root", projection.operation_id)),
        test_content_id(&format!("{}:new-ledger-root", projection.operation_id)),
        projection.produced_at_ms,
    );

    let receipt_hash_payload = project_receipt_hash_payload(&committed, &receipt_context)
        .expect("committed wallet evidence should project into receipt-hash payload");

    assert_eq!(receipt_hash_payload.txid, projection.txid);
    assert_eq!(receipt_hash_payload.operation_id, projection.operation_id);
    assert_eq!(
        receipt_hash_payload.idempotency_key,
        projection.idempotency_key
    );
    assert_eq!(
        receipt_hash_payload.amount_minor,
        projection.amount_minor.get().to_string()
    );
    assert_eq!(
        receipt_hash_payload.ledger_seq_start,
        projection.ledger_seq_start
    );
    assert_eq!(
        receipt_hash_payload.ledger_seq_end,
        projection.ledger_seq_end
    );

    let payload_hash =
        hash_quickchain_payload(QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, &receipt_hash_payload);
    let sort_key_hex = receipt_sort_key_hex(&projection.operation_id);

    let item = QuickChainTreeMaterialProjectionItem::new(
        sort_key_hex.clone(),
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
        payload_hash.clone(),
    );

    ReceiptMaterialFixture {
        projection,
        sort_key_hex,
        payload_hash,
        item,
    }
}

fn assert_field_absent(value: &Value, field: &str) {
    let object = value
        .as_object()
        .expect("serialized wallet projection should be an object");

    assert!(
        !object.contains_key(field),
        "wallet projection must not expose root/proof/finality authority field: {field}"
    );
}

#[test]
fn wallet_receipts_project_into_receipt_root_material_without_wallet_root_authority() {
    let first = receipt_material_fixture('1', "acct_qc1r2_alice", "acct_qc1r2_bob", 125, 7, 20, 21);
    let second =
        receipt_material_fixture('2', "acct_qc1r2_carol", "acct_qc1r2_dave", 250, 8, 22, 23);

    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::Receipts,
        vec![second.item.clone(), first.item.clone()],
    )
    .expect("unordered wallet receipt material should assemble into a sorted batch");

    batch
        .validate()
        .expect("wallet receipt material batch must satisfy ron-proto");

    assert_eq!(batch.chain_id, CHAIN_ID);
    assert_eq!(batch.epoch_id, EPOCH_ID);
    assert_eq!(batch.tree, QuickChainTreeMaterialKindV1::Receipts);
    assert_eq!(
        batch.item_sort_rule,
        QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1
    );
    assert_eq!(batch.items.len(), 2);

    assert_eq!(batch.items[0].sort_key_hex, first.sort_key_hex);
    assert_eq!(batch.items[1].sort_key_hex, second.sort_key_hex);
    assert_eq!(batch.items[0].payload_hash, first.payload_hash);
    assert_eq!(batch.items[1].payload_hash, second.payload_hash);
    assert_eq!(
        batch.items[0].payload_schema,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA
    );
    assert_eq!(
        batch.items[1].payload_schema,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA
    );

    let root = compute_tree_root_from_batch(&batch)
        .expect("wallet receipt material should compute a deterministic receipt root");

    root.validate()
        .expect("computed receipt root must satisfy ron-proto");

    assert_eq!(root.chain_id, CHAIN_ID);
    assert_eq!(root.epoch_id, EPOCH_ID);
    assert_eq!(root.tree, QuickChainTreeMaterialKindV1::Receipts);
    assert_eq!(root.source_items_count, 2);
    assert_eq!(root.tree_height, 1);
    assert_eq!(
        root.hash_domain,
        quickchain_tree_root_domain_for_tree(QuickChainTreeMaterialKindV1::Receipts)
    );
    assert_eq!(
        root.algorithm,
        QUICKCHAIN_TREE_ROOT_ALGORITHM_SORTED_BINARY_MERKLE_MAP_BLAKE3_JSON_V1
    );
    assert!(root.root_hash.to_string().starts_with("b3:"));

    let proof = build_tree_inclusion_proof_from_batch(&batch, &second.sort_key_hex)
        .expect("receipt material inclusion proof should build");

    proof
        .validate()
        .expect("receipt material inclusion proof should satisfy ron-proto");

    verify_tree_inclusion_proof(&root, &proof)
        .expect("receipt material inclusion proof should verify");

    assert_eq!(
        first.projection.settlement_status,
        QuickChainWalletReceiptStatus::Accepted,
        "wallet projection may honestly claim only accepted, not finalized/anchored"
    );
}

#[test]
fn wallet_projection_rejects_root_proof_or_finality_poison_after_material_exists() {
    let fixture =
        receipt_material_fixture('3', "acct_qc1r2_eve", "acct_qc1r2_frank", 375, 9, 24, 25);

    let clean =
        serde_json::to_value(&fixture.projection).expect("wallet projection should serialize");

    for forbidden in [
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "root_hash",
        "root_node_hash",
        "inclusion_proof",
        "proof_steps",
        "validator_signature",
        "finality",
        "finalized",
        "anchored",
        "external_settlement_status",
    ] {
        assert_field_absent(&clean, forbidden);

        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("projection JSON should be an object")
            .insert(forbidden.to_string(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<QuickChainWalletReceiptProjection>(poisoned).is_err(),
            "wallet projection DTO must reject injected root/proof/finality field: {forbidden}"
        );
    }
}
