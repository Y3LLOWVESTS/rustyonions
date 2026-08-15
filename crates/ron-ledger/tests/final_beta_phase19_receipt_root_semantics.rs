#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — FINAL_BETA Phase 19 tests for typed ledger-sequence receipt-root provenance.
//! RO:WHY — A checkpoint must not label an arbitrary generic receipt root as LedgerSequenceMerkleV1.
//! RO:INTERACTS — ron-proto receipt ordering/hash payloads and ron-ledger audited tree reducer.
//! RO:INVARIANTS — receipt order is ledger sequence then txid bytes; payloads validate first;
//!                 input permutation cannot change the root; duplicate receipt keys reject.
//! RO:SECURITY — deterministic local root only; no checkpoint, validator, wallet, ledger,
//!               payout, settlement, or finality mutation.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_tree_material_batch, compute_ledger_sequence_receipt_root, compute_tree_root_from_batch,
    QuickChainTreeMaterialProjectionError, QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        quickchain_receipt_sort_key_v1, to_canonical_json_vec, QuickChainOperationClassV1,
        QuickChainReceiptHashPayloadV1, QuickChainTreeMaterialKindV1, QuickChainTreeRootV1,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_phase19_receipts";

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture ContentId must parse")
}

fn receipt(label: char, ledger_seq_start: u64, txid: &str) -> QuickChainReceiptHashPayloadV1 {
    let payload = QuickChainReceiptHashPayloadV1 {
        schema: QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        txid: txid.to_owned(),

        operation_id: format!("op_{}", label.to_string().repeat(32)),

        operation_hash: cid(&format!("operation-{label}")),

        op: "issue".to_owned(),

        op_class: QuickChainOperationClassV1::Issue,

        from_account_id: None,

        to_account_id: Some(format!("acct_phase19_{label}")),

        asset: "roc".to_owned(),

        amount_minor: "10".to_owned(),

        account_sequence: ledger_seq_start,

        hold_id: None,

        session_budget_id: None,

        idempotency_key: format!("idem-phase19-{label}"),

        ledger_seq_start,

        ledger_seq_end: ledger_seq_start,

        previous_ledger_root: cid(&format!("previous-{label}")),

        new_ledger_root: cid(&format!("new-{label}")),

        produced_at_ms: 1_800_000_000_000 + ledger_seq_start,
    };

    payload
        .validate()
        .expect("Phase 19 receipt fixture must validate");

    payload
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[usize::from(byte >> 4)] as char);

        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }

    encoded
}

fn independent_receipt_hash(payload: &QuickChainReceiptHashPayloadV1) -> ContentId {
    let canonical = to_canonical_json_vec(payload).expect("receipt fixture must canonicalize");

    let mut framed =
        Vec::with_capacity(QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1.len() + 1 + canonical.len());

    framed.extend_from_slice(QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1.as_bytes());

    framed.push(0);

    framed.extend_from_slice(&canonical);

    let digest = blake3::hash(&framed).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("independent receipt hash must parse")
}

fn expected_generic_root(receipts: &[QuickChainReceiptHashPayloadV1]) -> QuickChainTreeRootV1 {
    let items = receipts
        .iter()
        .map(|receipt| {
            let sort_key = quickchain_receipt_sort_key_v1(receipt.ledger_seq_start, &receipt.txid)
                .expect("fixture receipt sort key must derive");

            QuickChainTreeMaterialProjectionItem::new(
                encode_lower_hex(&sort_key),
                QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
                independent_receipt_hash(receipt),
            )
        })
        .collect::<Vec<_>>();

    let batch = build_tree_material_batch(
        CHAIN_ID,
        EPOCH_ID,
        QuickChainTreeMaterialKindV1::Receipts,
        items,
    )
    .expect("independent generic receipt material must build");

    compute_tree_root_from_batch(&batch).expect("independent generic receipt root must compute")
}

#[test]
fn empty_typed_receipt_root_matches_existing_empty_receipts_root() {
    let typed = compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[])
        .expect("empty typed receipt root must compute");

    let expected = expected_generic_root(&[]);

    assert_eq!(typed.root(), &expected,);

    assert_eq!(typed.root().tree, QuickChainTreeMaterialKindV1::Receipts,);

    assert_eq!(typed.root().source_items_count, 0,);
}

#[test]
fn receipt_input_permutation_cannot_change_ledger_sequence_root() {
    let first = receipt('a', 10, "txid_z");

    let second = receipt('b', 20, "txid_a");

    let third = receipt('c', 30, "txid_m");

    let forward = compute_ledger_sequence_receipt_root(
        CHAIN_ID,
        EPOCH_ID,
        &[first.clone(), second.clone(), third.clone()],
    )
    .expect("forward receipt root must compute");

    let shuffled =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[third, first, second])
            .expect("shuffled receipt root must compute");

    assert_eq!(forward, shuffled,);
}

#[test]
fn typed_root_matches_protocol_ledger_sequence_then_txid_material() {
    // txid lexical order intentionally opposes ledger order.
    // Sequence 10 must still precede sequence 20.
    let later = receipt('a', 20, "txid_a");

    let earlier = receipt('b', 10, "txid_z");

    let typed =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[later.clone(), earlier.clone()])
            .expect("typed receipt root must compute");

    let expected = expected_generic_root(&[later, earlier]);

    assert_eq!(typed.root(), &expected,);
}

#[test]
fn equal_sequence_receipts_use_txid_tie_break_deterministically() {
    let txid_b = receipt('a', 40, "txid_b");

    let txid_a = receipt('b', 40, "txid_a");

    let forward =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[txid_b.clone(), txid_a.clone()])
            .expect("same-sequence receipt root must compute");

    let reversed =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[txid_a.clone(), txid_b.clone()])
            .expect("reversed same-sequence receipt root must compute");

    let expected = expected_generic_root(&[txid_b, txid_a]);

    assert_eq!(forward, reversed,);

    assert_eq!(forward.root(), &expected,);
}

#[test]
fn receipt_chain_mismatch_rejects_before_root_is_returned() {
    let mut wrong_chain = receipt('a', 10, "txid_a");

    wrong_chain.chain_id = "another-chain".to_owned();

    let error = compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[wrong_chain])
        .expect_err("cross-chain receipt must reject");

    assert_eq!(
        error,
        QuickChainTreeMaterialProjectionError::ReceiptChainMismatch {
            index: 0,
            expected: CHAIN_ID.to_owned(),
            actual: "another-chain".to_owned(),
        },
    );
}

#[test]
fn invalid_receipt_payload_rejects_before_hash_or_root_is_returned() {
    let mut invalid = receipt('a', 10, "txid_a");

    invalid.account_sequence = 0;

    let error = compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[invalid])
        .expect_err("invalid receipt payload must reject");

    assert!(matches!(
        error,
        QuickChainTreeMaterialProjectionError::InvalidReceiptPayload { index: 0, .. }
    ));
}

#[test]
fn duplicate_ledger_sequence_and_txid_sort_key_rejects() {
    let first = receipt('a', 50, "txid_duplicate");

    let second = receipt('b', 50, "txid_duplicate");

    let expected_sort_key = encode_lower_hex(
        &quickchain_receipt_sort_key_v1(50, "txid_duplicate")
            .expect("duplicate fixture sort key must derive"),
    );

    let error = compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[first, second])
        .expect_err("duplicate receipt sort key must reject");

    assert_eq!(
        error,
        QuickChainTreeMaterialProjectionError::DuplicateSortKey {
            sort_key_hex: expected_sort_key,
        },
    );
}

#[test]
fn immutable_receipt_payload_change_changes_typed_root() {
    let original = receipt('a', 60, "txid_a");

    let original_root =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, std::slice::from_ref(&original))
            .expect("original receipt root must compute");

    let mut changed = original;

    changed.amount_minor = "11".to_owned();

    changed
        .validate()
        .expect("changed immutable receipt fixture remains valid");

    let changed_root = compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[changed])
        .expect("changed receipt root must compute");

    assert_ne!(original_root, changed_root,);
}

#[test]
fn typed_wrapper_preserves_receipt_provenance_through_owned_conversion() {
    let typed =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[receipt('a', 70, "txid_a")])
            .expect("typed receipt root must compute");

    assert_eq!(typed.root().tree, QuickChainTreeMaterialKindV1::Receipts,);

    let borrowed = typed.root().clone();

    let owned = typed.into_root();

    assert_eq!(borrowed, owned,);

    assert_eq!(owned.tree, QuickChainTreeMaterialKindV1::Receipts,);
}
