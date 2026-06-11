//! RO:WHAT — Proves exact receipt sort-key bytes and deterministic receipt ordering.
//! RO:WHY — ECON/RES: future receipt roots must not inherit DB, arrival, map, or scheduler order.
//! RO:INVARIANTS — u64 big-endian prefix; txid byte tie-break; duplicates reject; no hashing or roots.
//! RO:TEST — file-backed Phase 0 receipt ordering vectors and negative input tests.

use ron_proto::{
    quickchain_receipt_sort_key_v1, sort_quickchain_keys_v1, QuickChainValidationError,
    QUICKCHAIN_RECEIPT_SORT_KEY_LEDGER_SEQ_BYTES_V1, QUICKCHAIN_RECEIPT_SORT_KEY_RULE_V1,
};
use serde::Deserialize;
use serde_json::{json, Value};

const RECEIPT_ORDER_VECTOR: &str =
    include_str!("vectors/quickchain/receipt_order/receipt_sort_keys_locked_bytes_v1.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptOrderInput {
    ledger_seq_start: u64,
    txid: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptKeyExample {
    ledger_seq_start: u64,
    txid: String,
    expected_sort_key_hex: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptOrderVectorSet {
    schema: String,
    version: u16,
    status: String,
    rule: String,
    key_example: ReceiptKeyExample,
    unordered_inputs: Vec<ReceiptOrderInput>,
    expected_order_hex: Vec<String>,
    duplicate_inputs: Vec<ReceiptOrderInput>,
    expected_duplicate_error: String,
    notes: Vec<String>,
}

fn load_vector() -> ReceiptOrderVectorSet {
    let vector: ReceiptOrderVectorSet = serde_json::from_str(RECEIPT_ORDER_VECTOR).unwrap();

    assert_eq!(vector.schema, "quickchain.receipt-sort-key-vector-set.v1");
    assert_eq!(vector.version, 1);
    assert_eq!(vector.status, "locked_bytes");
    assert_eq!(vector.rule, QUICKCHAIN_RECEIPT_SORT_KEY_RULE_V1);
    assert!(!vector.notes.is_empty());
    assert!(vector.notes.iter().all(|note| !note.is_empty()));

    vector
}

fn key_for(input: &ReceiptOrderInput) -> Vec<u8> {
    quickchain_receipt_sort_key_v1(input.ledger_seq_start, &input.txid).unwrap()
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

#[test]
fn receipt_sort_key_matches_exact_locked_bytes() {
    let vector = load_vector();

    assert_eq!(QUICKCHAIN_RECEIPT_SORT_KEY_LEDGER_SEQ_BYTES_V1, 8);

    let key = quickchain_receipt_sort_key_v1(
        vector.key_example.ledger_seq_start,
        &vector.key_example.txid,
    )
    .unwrap();

    assert_eq!(lower_hex(&key), vector.key_example.expected_sort_key_hex);

    assert_eq!(
        &key[..QUICKCHAIN_RECEIPT_SORT_KEY_LEDGER_SEQ_BYTES_V1],
        &1_u64.to_be_bytes()
    );

    assert_eq!(
        &key[QUICKCHAIN_RECEIPT_SORT_KEY_LEDGER_SEQ_BYTES_V1..],
        b"tx:roc:0001"
    );
}

#[test]
fn receipt_order_is_numeric_then_txid_bytewise() {
    let vector = load_vector();

    let mut keys: Vec<Vec<u8>> = vector.unordered_inputs.iter().map(key_for).collect();

    sort_quickchain_keys_v1(&mut keys).unwrap();

    let actual_hex: Vec<String> = keys.iter().map(|key| lower_hex(key)).collect();

    assert_eq!(actual_hex.as_slice(), vector.expected_order_hex.as_slice());

    let seq_ten_a = quickchain_receipt_sort_key_v1(10, "tx:roc:a").unwrap();
    let seq_ten_b = quickchain_receipt_sort_key_v1(10, "tx:roc:b").unwrap();
    let seq_two = quickchain_receipt_sort_key_v1(2, "tx:roc:z").unwrap();
    let seq_two_fifty_six = quickchain_receipt_sort_key_v1(256, "tx:roc:a").unwrap();

    assert!(seq_two < seq_ten_a);
    assert!(seq_ten_a < seq_ten_b);
    assert!(seq_ten_b < seq_two_fifty_six);
}

#[test]
fn input_permutations_produce_identical_receipt_order() {
    let vector = load_vector();

    let mut forward: Vec<Vec<u8>> = vector.unordered_inputs.iter().map(key_for).collect();

    let mut reverse = forward.clone();
    reverse.reverse();

    let mut rotated = forward.clone();
    rotated.rotate_left(1);

    sort_quickchain_keys_v1(&mut forward).unwrap();
    sort_quickchain_keys_v1(&mut reverse).unwrap();
    sort_quickchain_keys_v1(&mut rotated).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward, rotated);
}

#[test]
fn duplicate_receipt_sort_keys_reject() {
    let vector = load_vector();

    let mut keys: Vec<Vec<u8>> = vector.duplicate_inputs.iter().map(key_for).collect();

    let error = sort_quickchain_keys_v1(&mut keys).unwrap_err();

    match error {
        QuickChainValidationError::InvalidField { field, reason } => {
            assert_eq!(field, "sort_keys");
            assert_eq!(reason, vector.expected_duplicate_error.as_str());
        }
        other => {
            panic!("expected duplicate receipt sort-key error, got {other:?}");
        }
    }
}

#[test]
fn receipt_sort_key_rejects_zero_sequence_and_bad_txid() {
    let zero_error = quickchain_receipt_sort_key_v1(0, "tx:roc:0001").unwrap_err();

    assert!(matches!(
        zero_error,
        QuickChainValidationError::InvalidField {
            field: "ledger_seq_start",
            reason: "must be greater than zero for receipt ordering"
        }
    ));

    quickchain_receipt_sort_key_v1(1, "TX:ROC:0001").expect_err("uppercase txid must reject");

    quickchain_receipt_sort_key_v1(1, "tx roc 0001").expect_err("txid spaces must reject");

    quickchain_receipt_sort_key_v1(1, "tx:roc:0001\0evil").expect_err("embedded NUL must reject");
}

#[test]
fn receipt_order_vector_rejects_unknown_fields() {
    let mut value: Value = serde_json::from_str(RECEIPT_ORDER_VECTOR).unwrap();

    value
        .as_object_mut()
        .unwrap()
        .insert("database_order".to_string(), json!(true));

    serde_json::from_value::<ReceiptOrderVectorSet>(value)
        .expect_err("unknown receipt-order fields must reject");
}
