#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for pure committed-record projection into frozen ron-proto receipt-hash payload DTOs.
//! RO:WHY — ECON/RES: receipt identity, account direction, sequences, operation commitment, and explicit continuity context must agree before receipt hashing or roots.
//! RO:INTERACTS — QuickChainCommittedOperationRecord, receipt projection context, and ron-proto receipt-hash payload validation.
//! RO:INVARIANTS — all current operation classes; ledger-owned evidence preserved; roots/timestamps explicit; no serialization, hashing, root calculation, clocks, IO, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture content IDs are inert reviewed inputs and are not production roots, receipts, proofs, or authorization.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    project_receipt_hash_payload, QuickChainCommittedOperationRecord,
    QuickChainHashPayloadProjectionError, QuickChainReceiptHashProjectionContext,
};
use ron_proto::{
    quickchain::{
        QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC, QUICKCHAIN_OPERATION_INTENT_SCHEMA,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

/// Produce a genuine BLAKE3 content identifier for an inert test label.
///
/// These IDs test explicit projection plumbing only. They are not production
/// operation hashes, ledger roots, receipt roots, state roots, or vectors.
fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("BLAKE3 test content ID should parse")
}

fn operation_for_class(
    operation_hex_digit: char,
    op_class: QuickChainOperationClassV1,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    let mut operation = QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: format!("idem:receipt-projection:{operation_hex_digit}"),
        op_class,
        actor_account_id: "account:alice".to_string(),
        counterparty_account_id: None,
        amount_minor: Some("125".to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms,
    };

    match op_class {
        QuickChainOperationClassV1::Issue => {
            operation.actor_account_id = "account:recipient".to_string();
        }

        QuickChainOperationClassV1::Transfer => {
            operation.counterparty_account_id = Some("account:bob".to_string());
        }

        QuickChainOperationClassV1::Burn => {}

        QuickChainOperationClassV1::HoldOpen => {
            operation.counterparty_account_id = Some("account:merchant".to_string());
            operation.hold_id = Some(hold_id('1'));
        }

        QuickChainOperationClassV1::HoldCapture => {
            operation.counterparty_account_id = Some("account:merchant".to_string());
            operation.hold_id = Some(hold_id('2'));
        }

        QuickChainOperationClassV1::HoldRelease => {
            operation.hold_id = Some(hold_id('3'));
        }

        QuickChainOperationClassV1::HoldExpire => {
            operation.hold_id = Some(hold_id('4'));
        }

        _ => panic!("unsupported future QuickChain operation class in test fixture"),
    }

    operation
}

fn committed_record(
    operation_hex_digit: char,
    op_class: QuickChainOperationClassV1,
    produced_at_ms: u64,
    account_sequence: u64,
    ledger_seq_start: u64,
    ledger_seq_end: u64,
) -> QuickChainCommittedOperationRecord {
    QuickChainCommittedOperationRecord::new(
        operation_for_class(operation_hex_digit, op_class, produced_at_ms),
        format!("tx:roc:receipt-projection:{operation_hex_digit}"),
        account_sequence,
        ledger_seq_start,
        ledger_seq_end,
    )
    .expect("test committed record should be valid")
}

fn projection_context(
    operation_id: &str,
    op: &str,
    session_budget_id: Option<&str>,
    produced_at_ms: u64,
) -> QuickChainReceiptHashProjectionContext {
    QuickChainReceiptHashProjectionContext::new(
        operation_id,
        test_content_id(&format!("{operation_id}/operation-hash")),
        op,
        session_budget_id.map(str::to_owned),
        test_content_id(&format!("{operation_id}/previous-ledger-root")),
        test_content_id(&format!("{operation_id}/new-ledger-root")),
        produced_at_ms,
    )
}

#[test]
fn all_current_operation_classes_project_exact_receipt_direction() {
    let cases = [
        (
            QuickChainOperationClassV1::Issue,
            '1',
            "issue",
            None,
            Some("account:recipient"),
            1,
            1,
        ),
        (
            QuickChainOperationClassV1::Transfer,
            '2',
            "paid_transfer",
            Some("account:alice"),
            Some("account:bob"),
            2,
            3,
        ),
        (
            QuickChainOperationClassV1::Burn,
            '3',
            "burn",
            Some("account:alice"),
            None,
            4,
            4,
        ),
        (
            QuickChainOperationClassV1::HoldOpen,
            '4',
            "paid_site_visit",
            Some("account:alice"),
            Some("account:merchant"),
            5,
            5,
        ),
        (
            QuickChainOperationClassV1::HoldCapture,
            '5',
            "hold_capture",
            Some("account:alice"),
            Some("account:merchant"),
            6,
            7,
        ),
        (
            QuickChainOperationClassV1::HoldRelease,
            '6',
            "hold_release",
            Some("account:alice"),
            None,
            8,
            8,
        ),
        (
            QuickChainOperationClassV1::HoldExpire,
            '7',
            "hold_expire",
            Some("account:alice"),
            None,
            9,
            9,
        ),
    ];

    for (op_class, hex_digit, op, expected_from, expected_to, ledger_seq_start, ledger_seq_end) in
        cases
    {
        let record = committed_record(
            hex_digit,
            op_class,
            1_777_000_000_000,
            7,
            ledger_seq_start,
            ledger_seq_end,
        );

        let session_budget_id = if op_class == QuickChainOperationClassV1::Transfer {
            Some("budget_transfer_001")
        } else {
            None
        };

        let context = projection_context(
            &record.intent().operation_id,
            op,
            session_budget_id,
            1_888_000_000_000,
        );

        let payload = project_receipt_hash_payload(&record, &context)
            .expect("valid committed evidence should project");

        payload
            .validate()
            .expect("projected receipt payload must satisfy ron-proto");

        assert_eq!(payload.schema, QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA);
        assert_eq!(payload.version, QUICKCHAIN_DTO_VERSION);
        assert_eq!(payload.chain_id, CHAIN_ID);
        assert_eq!(payload.txid, record.receipt_txid());
        assert_eq!(payload.operation_id, record.intent().operation_id);
        assert_eq!(payload.operation_hash, context.operation_hash().clone());
        assert_eq!(payload.op, op);
        assert_eq!(payload.op_class, op_class);
        assert_eq!(payload.from_account_id.as_deref(), expected_from);
        assert_eq!(payload.to_account_id.as_deref(), expected_to);
        assert_eq!(payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
        assert_eq!(payload.amount_minor, "125");
        assert_eq!(payload.account_sequence, 7);
        assert_eq!(payload.hold_id, record.intent().hold_id);
        assert_eq!(payload.session_budget_id.as_deref(), session_budget_id);
        assert_eq!(payload.idempotency_key, record.intent().idempotency_key);
        assert_eq!(payload.ledger_seq_start, ledger_seq_start);
        assert_eq!(payload.ledger_seq_end, ledger_seq_end);
        assert_eq!(
            payload.previous_ledger_root,
            context.previous_ledger_root().clone()
        );
        assert_eq!(payload.new_ledger_root, context.new_ledger_root().clone());
        assert_eq!(payload.produced_at_ms, 1_888_000_000_000);
    }
}

#[test]
fn receipt_context_cannot_be_rebound_to_another_operation() {
    let record = committed_record(
        '1',
        QuickChainOperationClassV1::Transfer,
        1_777_000_000_000,
        1,
        1,
        2,
    );

    let context = projection_context(&operation_id('2'), "paid_transfer", None, 1_888_000_000_000);

    let error = project_receipt_hash_payload(&record, &context)
        .expect_err("context for another operation must reject");

    assert_eq!(
        error,
        QuickChainHashPayloadProjectionError::ReceiptContextMismatch {
            context_operation_id: operation_id('2'),
            record_operation_id: operation_id('1'),
        }
    );
}

#[test]
fn ron_proto_rejects_invalid_explicit_receipt_action() {
    let record = committed_record(
        '3',
        QuickChainOperationClassV1::HoldOpen,
        1_777_000_000_000,
        1,
        1,
        1,
    );

    // Receipt action names use the frozen bounded-token contract.
    let context = projection_context(
        &record.intent().operation_id,
        "paid site visit",
        Some("budget_hold_001"),
        1_888_000_000_000,
    );

    let error = project_receipt_hash_payload(&record, &context)
        .expect_err("invalid explicit receipt action must reject");

    match error {
        QuickChainHashPayloadProjectionError::InvalidReceiptHashPayload {
            operation_id,
            reason,
        } => {
            assert_eq!(operation_id, record.intent().operation_id);
            assert!(!reason.is_empty());
        }

        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn receipt_production_timestamp_is_explicit_not_client_derived() {
    let first = committed_record(
        '4',
        QuickChainOperationClassV1::Transfer,
        1_700_000_000_000,
        3,
        10,
        11,
    );

    let second = committed_record(
        '4',
        QuickChainOperationClassV1::Transfer,
        1_799_000_000_000,
        3,
        10,
        11,
    );

    let context = projection_context(
        &first.intent().operation_id,
        "paid_transfer",
        Some("budget_transfer_002"),
        1_900_000_000_000,
    );

    let first_payload = project_receipt_hash_payload(&first, &context)
        .expect("first receipt projection should succeed");

    let second_payload = project_receipt_hash_payload(&second, &context)
        .expect("second receipt projection should succeed");

    // Operation-intent production time is not reused as receipt production
    // time. The backend receipt time comes only from explicit reviewed context.
    assert_eq!(first_payload, second_payload);
    assert_eq!(first_payload.produced_at_ms, 1_900_000_000_000);
}

#[test]
fn projection_is_read_only_and_zero_receipt_timestamp_rejects() {
    let record = committed_record(
        '5',
        QuickChainOperationClassV1::Burn,
        1_777_000_000_000,
        9,
        20,
        20,
    );

    let context = projection_context(&record.intent().operation_id, "burn", None, 0);

    let original_record = record.clone();
    let original_context = context.clone();

    let error = project_receipt_hash_payload(&record, &context)
        .expect_err("zero backend receipt timestamp must reject");

    match error {
        QuickChainHashPayloadProjectionError::InvalidReceiptHashPayload {
            operation_id,
            reason,
        } => {
            assert_eq!(operation_id, record.intent().operation_id);
            assert!(!reason.is_empty());
        }

        other => panic!("unexpected projection error: {other:?}"),
    }

    assert_eq!(record, original_record);
    assert_eq!(context, original_context);
}
