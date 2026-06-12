#![cfg(feature = "quickchain-preflight")]

//! RO:WHAT — Integration tests for QuickChain operation identity, scoped idempotency, and ledger-assigned replay sequences.
//! RO:WHY — ECON/RES: freeze duplicate/retry behavior before balance, hold, persistence, or root implementation.
//! RO:INTERACTS — ron_ledger::quickchain, ron_proto::quickchain operation DTOs.
//! RO:INVARIANTS — exact retry returns original evidence; conflicts never mutate; sequence assignment is ledger-owned.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no capabilities, secrets, wallet mutation, or receipt fabrication.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainCommittedOperationRecord, QuickChainReplayError, QuickChainReplayIndex,
    QuickChainSubmissionDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_string(),
        op_class,
        actor_account_id: actor.to_string(),
        counterparty_account_id: counterparty.map(str::to_string),
        amount_minor: Some(amount_minor.to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms,
    }
}

fn committed(
    intent: QuickChainOperationIntentV1,
    receipt_txid: &str,
    account_sequence: u64,
    ledger_sequence_start: u64,
    ledger_sequence_end: u64,
) -> QuickChainCommittedOperationRecord {
    QuickChainCommittedOperationRecord::new(
        intent,
        receipt_txid,
        account_sequence,
        ledger_sequence_start,
        ledger_sequence_end,
    )
    .expect("valid committed record")
}

#[test]
fn identical_scoped_retry_returns_original_without_mutation() {
    let original_intent = intent(
        '1',
        "idem:issue:one",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        1_000,
    );
    let original = committed(original_intent.clone(), "tx:roc:0001", 1, 1, 1);

    let mut index = QuickChainReplayIndex::new();
    index
        .record_committed(original.clone())
        .expect("record accepted");
    let snapshot = index.clone();

    let decision = index
        .classify_submission(&original_intent)
        .expect("retry classified");

    assert_eq!(
        decision,
        QuickChainSubmissionDecision::ReturnOriginal(Box::new(original))
    );
    assert_eq!(index, snapshot);
}

#[test]
fn conflicting_reuse_in_same_scope_rejects_without_mutation() {
    let original_intent = intent(
        '2',
        "idem:transfer:one",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "10",
        2_000,
    );

    let mut index = QuickChainReplayIndex::new();
    index
        .record_committed(committed(original_intent, "tx:roc:0002", 1, 1, 2))
        .expect("record accepted");
    let snapshot = index.clone();

    let conflict = intent(
        '3',
        "idem:transfer:one",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:carol"),
        "11",
        2_001,
    );

    let error = index
        .classify_submission(&conflict)
        .expect_err("same scoped key with different intent must reject");

    assert_eq!(error, QuickChainReplayError::IdempotencyConflict);
    assert_eq!(index, snapshot);
}

#[test]
fn same_operation_id_under_another_retry_key_rejects() {
    let original_intent = intent(
        '4',
        "idem:issue:original",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "25",
        3_000,
    );

    let mut index = QuickChainReplayIndex::new();
    index
        .record_committed(committed(original_intent.clone(), "tx:roc:0003", 1, 1, 1))
        .expect("record accepted");
    let snapshot = index.clone();

    let mut duplicate = original_intent;
    duplicate.idempotency_key = "idem:issue:another".to_string();
    duplicate.produced_at_ms = 3_001;

    let error = index
        .classify_submission(&duplicate)
        .expect_err("distinct submission cannot reuse operation id");

    assert_eq!(error, QuickChainReplayError::DuplicateOperationId);
    assert_eq!(index, snapshot);
}

#[test]
fn idempotency_scope_allows_same_key_for_other_account_or_family() {
    let mut index = QuickChainReplayIndex::new();
    index
        .record_committed(committed(
            intent(
                '5',
                "retry-key",
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "50",
                4_000,
            ),
            "tx:roc:0004",
            1,
            1,
            1,
        ))
        .expect("record accepted");

    let other_account = intent(
        '6',
        "retry-key",
        QuickChainOperationClassV1::Issue,
        "account:bob",
        None,
        "50",
        4_001,
    );
    let other_family = intent(
        '7',
        "retry-key",
        QuickChainOperationClassV1::Burn,
        "account:alice",
        None,
        "5",
        4_002,
    );

    assert_eq!(
        index.classify_submission(&other_account).unwrap(),
        QuickChainSubmissionDecision::Fresh
    );
    assert_eq!(
        index.classify_submission(&other_family).unwrap(),
        QuickChainSubmissionDecision::Fresh
    );
}

#[test]
fn client_assigned_account_sequence_rejects_before_lookup() {
    let mut submitted = intent(
        '8',
        "idem:client-sequence",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "1",
        5_000,
    );
    submitted.account_sequence = Some(9);

    let index = QuickChainReplayIndex::new();
    let error = index
        .classify_submission(&submitted)
        .expect_err("client sequence must reject");

    assert_eq!(error, QuickChainReplayError::ClientAssignedAccountSequence);
    assert_eq!(index.operation_count(), 0);
}

#[test]
fn account_sequence_must_advance_exactly_once() {
    let mut index = QuickChainReplayIndex::new();
    index
        .record_committed(committed(
            intent(
                '9',
                "idem:seq:one",
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "1",
                6_000,
            ),
            "tx:roc:0005",
            1,
            1,
            1,
        ))
        .expect("first record accepted");
    let snapshot = index.clone();

    let error = index
        .record_committed(committed(
            intent(
                'a',
                "idem:seq:two",
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:bob"),
                "1",
                6_001,
            ),
            "tx:roc:0006",
            3,
            2,
            3,
        ))
        .expect_err("sequence two may not be skipped");

    assert!(matches!(
        error,
        QuickChainReplayError::AccountSequenceMismatch {
            expected: 2,
            actual: 3,
            ..
        }
    ));
    assert_eq!(index, snapshot);
}

#[test]
fn ledger_sequence_range_must_append_contiguously() {
    let mut index = QuickChainReplayIndex::new();
    let snapshot = index.clone();

    let error = index
        .record_committed(committed(
            intent(
                'b',
                "idem:ledger-gap",
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "1",
                7_000,
            ),
            "tx:roc:0007",
            1,
            2,
            2,
        ))
        .expect_err("first record must begin at ledger sequence one");

    assert_eq!(
        error,
        QuickChainReplayError::LedgerSequenceMismatch {
            expected: 1,
            actual: 2,
        }
    );
    assert_eq!(index, snapshot);
}

#[test]
fn rejected_record_does_not_consume_any_sequence() {
    let mut index = QuickChainReplayIndex::new();
    let bad = committed(
        intent(
            'c',
            "idem:bad-sequence",
            QuickChainOperationClassV1::Issue,
            "account:alice",
            None,
            "1",
            8_000,
        ),
        "tx:roc:0008",
        2,
        1,
        1,
    );

    index
        .record_committed(bad)
        .expect_err("wrong account sequence rejects");

    assert_eq!(index.operation_count(), 0);
    assert_eq!(index.last_account_sequence("account:alice"), 0);
    assert_eq!(index.next_ledger_sequence(), 1);
}

#[test]
fn same_ordered_records_rebuild_identical_index_state() {
    let records = vec![
        committed(
            intent(
                'd',
                "idem:replay:one",
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "100",
                9_000,
            ),
            "tx:roc:0009",
            1,
            1,
            1,
        ),
        committed(
            intent(
                'e',
                "idem:replay:two",
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:bob"),
                "25",
                9_001,
            ),
            "tx:roc:0010",
            2,
            2,
            3,
        ),
    ];

    let mut first = QuickChainReplayIndex::new();
    let mut replay = QuickChainReplayIndex::new();

    for record in &records {
        first
            .record_committed(record.clone())
            .expect("first replay accepts record");
    }

    for record in records {
        replay
            .record_committed(record)
            .expect("second replay accepts record");
    }

    assert_eq!(first, replay);
    assert_eq!(first.operation_count(), 2);
    assert_eq!(first.next_ledger_sequence(), 4);
    assert_eq!(first.last_account_sequence("account:alice"), 2);
}

#[test]
fn invalid_receipt_reference_rejects_record_construction() {
    let error = QuickChainCommittedOperationRecord::new(
        intent(
            'f',
            "idem:bad-receipt",
            QuickChainOperationClassV1::Issue,
            "account:alice",
            None,
            "1",
            10_000,
        ),
        "tx receipt with spaces",
        1,
        1,
        1,
    )
    .expect_err("invalid receipt reference must reject");

    assert_eq!(error, QuickChainReplayError::InvalidReceiptTxid);
}
