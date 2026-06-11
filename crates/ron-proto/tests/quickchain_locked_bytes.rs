use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, QuickChainHoldStateV1,
    QuickChainHoldStatusV1, QuickChainOperationClassV1, QuickChainOperationIntentV1,
    QuickChainReceiptStatusV1, QuickChainReceiptV1, QuickChainTestVectorV1,
    QuickChainVectorCanonicalEncodingV1, QuickChainVectorHashAlgorithmV1,
    QuickChainVectorPreimageFramingV1, QuickChainVectorStatusV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1, QUICKCHAIN_HOLD_STATE_SCHEMA,
    QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1, QUICKCHAIN_OPERATION_INTENT_SCHEMA,
    QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, QUICKCHAIN_RECEIPT_SCHEMA, QUICKCHAIN_TEST_VECTOR_SCHEMA,
};
use serde::Serialize;

const OP_A: &str = "op_0123456789abcdef0123456789abcdef";
const HOLD_A: &str = "hold_0123456789abcdef0123456789abcdef";

fn operation_intent() -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        operation_id: OP_A.to_string(),
        idempotency_key: "idem:hold-open:0001".to_string(),
        op_class: QuickChainOperationClassV1::HoldOpen,
        actor_account_id: "account:viewer-a".to_string(),
        counterparty_account_id: Some("account:creator-b".to_string()),
        amount_minor: Some("100".to_string()),
        hold_id: Some(HOLD_A.to_string()),

        // account_sequence is assigned by the ledger after acceptance.
        account_sequence: None,

        produced_at_ms: 1_777_000_000_000,
    }
}

fn open_hold_state() -> QuickChainHoldStateV1 {
    QuickChainHoldStateV1 {
        schema: QUICKCHAIN_HOLD_STATE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        hold_id: HOLD_A.to_string(),
        account_id: "account:viewer-a".to_string(),
        counterparty_account_id: Some("account:creator-b".to_string()),
        amount_minor: "100".to_string(),
        status: QuickChainHoldStatusV1::Open,
        opened_operation_id: OP_A.to_string(),
        terminal_operation_id: None,
        opened_at_ms: 1_777_000_000_000,
        expires_at_ms: 1_777_003_600_000,
        terminal_at_ms: None,
        account_sequence_opened: 7,
        account_sequence_terminal: None,
    }
}

fn accepted_receipt() -> QuickChainReceiptV1 {
    QuickChainReceiptV1 {
        schema: QUICKCHAIN_RECEIPT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        txid: "tx:roc:000000000001".to_string(),
        operation_id: OP_A.to_string(),
        op: "paid_site_visit".to_string(),
        op_class: QuickChainOperationClassV1::Transfer,
        status: QuickChainReceiptStatusV1::Accepted,
        from_account_id: Some("account:visitor-b".to_string()),
        to_account_id: Some("account:creator-a".to_string()),
        asset: "roc".to_string(),
        amount_minor: "10".to_string(),
        account_sequence: Some(7),
        hold_id: None,
        session_budget_id: None,
        idempotency_key: "visit-2026-06-10T18:40:00Z-0001".to_string(),
        operation_hash: None,
        receipt_hash: None,
        receipt_root: None,
        checkpoint_hash: None,
        ledger_seq_start: None,
        ledger_seq_end: None,
        previous_ledger_root: None,
        new_ledger_root: None,
        memo: Some("backend-derived receipt reference".to_string()),
        produced_at_ms: 1_800_000_000_000,
    }
}

fn locked_bytes_vector<T>(
    vector_id: &str,
    purpose: &str,
    domain_separator: &str,
    payload: &T,
    expected_payload_utf8: &str,
) -> QuickChainTestVectorV1
where
    T: Serialize,
{
    let canonical_payload_utf8 = to_canonical_json_string(payload).unwrap();

    assert_eq!(canonical_payload_utf8, expected_payload_utf8);

    let vector = QuickChainTestVectorV1 {
        schema: QUICKCHAIN_TEST_VECTOR_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        vector_id: vector_id.to_string(),
        status: QuickChainVectorStatusV1::LockedBytes,
        purpose: purpose.to_string(),
        domain_separator: domain_separator.to_string(),
        canonical_encoding: QuickChainVectorCanonicalEncodingV1::CanonicalJsonV1,
        preimage_framing: QuickChainVectorPreimageFramingV1::DomainSeparatorNulPayload,
        hash_algorithm: QuickChainVectorHashAlgorithmV1::Blake3_256,
        human_readable_json: serde_json::to_value(payload).unwrap(),
        canonical_payload_utf8: Some(canonical_payload_utf8.clone()),
        canonical_payload_hex: Some(lower_hex(canonical_payload_utf8.as_bytes())),

        // locked_bytes deliberately does not claim a preimage or hash.
        preimage_hex: None,
        expected_b3: None,

        notes: vec!["locked canonical bytes only; no preimage or expected hash".to_string()],
    };

    vector.validate().unwrap();

    assert!(vector.preimage_hex.is_none());
    assert!(vector.expected_b3.is_none());

    vector
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
fn operation_intent_has_exact_locked_canonical_bytes() {
    let payload = operation_intent();
    payload.validate().unwrap();

    let expected = concat!(
        r#"{"schema":"quickchain.operation-intent.v1","#,
        r#""version":1,"#,
        r#""chain_id":"roc-dev","#,
        r#""operation_id":"op_0123456789abcdef0123456789abcdef","#,
        r#""idempotency_key":"idem:hold-open:0001","#,
        r#""op_class":"hold_open","#,
        r#""actor_account_id":"account:viewer-a","#,
        r#""counterparty_account_id":"account:creator-b","#,
        r#""amount_minor":"100","#,
        r#""hold_id":"hold_0123456789abcdef0123456789abcdef","#,
        r#""account_sequence":null,"#,
        r#""produced_at_ms":1777000000000}"#
    );

    let vector = locked_bytes_vector(
        "canonical_operation_intent_vector_001",
        "operation_intent_canonical_bytes",
        QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
        &payload,
        expected,
    );

    assert_eq!(vector.status, QuickChainVectorStatusV1::LockedBytes);

    let decoded: QuickChainOperationIntentV1 =
        from_canonical_json_slice(expected.as_bytes()).unwrap();

    assert_eq!(decoded, payload);
    decoded.validate().unwrap();
}

#[test]
fn open_hold_state_has_exact_locked_canonical_bytes() {
    let payload = open_hold_state();
    payload.validate().unwrap();

    let expected = concat!(
        r#"{"schema":"quickchain.hold-state.v1","#,
        r#""version":1,"#,
        r#""chain_id":"roc-dev","#,
        r#""hold_id":"hold_0123456789abcdef0123456789abcdef","#,
        r#""account_id":"account:viewer-a","#,
        r#""counterparty_account_id":"account:creator-b","#,
        r#""amount_minor":"100","#,
        r#""status":"open","#,
        r#""opened_operation_id":"op_0123456789abcdef0123456789abcdef","#,
        r#""terminal_operation_id":null,"#,
        r#""opened_at_ms":1777000000000,"#,
        r#""expires_at_ms":1777003600000,"#,
        r#""terminal_at_ms":null,"#,
        r#""account_sequence_opened":7,"#,
        r#""account_sequence_terminal":null}"#
    );

    let vector = locked_bytes_vector(
        "canonical_open_hold_state_vector_001",
        "hold_state_canonical_bytes",
        QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
        &payload,
        expected,
    );

    assert_eq!(vector.status, QuickChainVectorStatusV1::LockedBytes);

    let decoded: QuickChainHoldStateV1 = from_canonical_json_slice(expected.as_bytes()).unwrap();

    assert_eq!(decoded, payload);
    decoded.validate().unwrap();
}

#[test]
fn accepted_receipt_has_exact_locked_canonical_bytes() {
    let payload = accepted_receipt();
    payload.validate().unwrap();

    let expected = concat!(
        r#"{"schema":"quickchain.receipt.v1","#,
        r#""version":1,"#,
        r#""chain_id":"roc-dev","#,
        r#""txid":"tx:roc:000000000001","#,
        r#""operation_id":"op_0123456789abcdef0123456789abcdef","#,
        r#""op":"paid_site_visit","#,
        r#""op_class":"transfer","#,
        r#""status":"accepted","#,
        r#""from_account_id":"account:visitor-b","#,
        r#""to_account_id":"account:creator-a","#,
        r#""asset":"roc","#,
        r#""amount_minor":"10","#,
        r#""account_sequence":7,"#,
        r#""hold_id":null,"#,
        r#""session_budget_id":null,"#,
        r#""idempotency_key":"visit-2026-06-10T18:40:00Z-0001","#,
        r#""operation_hash":null,"#,
        r#""receipt_hash":null,"#,
        r#""receipt_root":null,"#,
        r#""checkpoint_hash":null,"#,
        r#""ledger_seq_start":null,"#,
        r#""ledger_seq_end":null,"#,
        r#""previous_ledger_root":null,"#,
        r#""new_ledger_root":null,"#,
        r#""memo":"backend-derived receipt reference","#,
        r#""produced_at_ms":1800000000000}"#
    );

    let vector = locked_bytes_vector(
        "canonical_accepted_receipt_vector_001",
        "receipt_canonical_bytes",
        QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        &payload,
        expected,
    );

    assert_eq!(vector.status, QuickChainVectorStatusV1::LockedBytes);

    let decoded: QuickChainReceiptV1 = from_canonical_json_slice(expected.as_bytes()).unwrap();

    assert_eq!(decoded, payload);
    decoded.validate().unwrap();
}
