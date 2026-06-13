// crates/ron-ledger/tests/quickchain_accepted_replay_sequence_evidence.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for accepted-replay rejection of corrupted hold sequence evidence.
//! RO:WHY — ECON/RES: accepted history must preserve ledger-owned account and primitive sequence evidence exactly.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainCommittedOperationRecord, replay errors, and hold execution.
//! RO:INVARIANTS — replay inputs may drive execution, but committed sequence evidence must reproduce exactly or reconstruction rejects.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixtures use inert receipt references only; no roots, proofs, signatures, settlement, or wallet authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainCommittedOperationRecord,
    QuickChainExecutionError, QuickChainHoldEpochInput, QuickChainReplayError,
    QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

#[allow(clippy::too_many_arguments)]
fn intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: Option<&str>,
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
        hold_id: hold_id.map(str::to_string),
        account_sequence: None,
        produced_at_ms,
    }
}

#[allow(clippy::too_many_arguments)]
fn accept_balance(
    state: &mut QuickChainAtomicState,
    history: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    actor: &str,
    amount_minor: &str,
    supply_decision: QuickChainSupplyDecision,
    txid: &str,
    produced_at_ms: u64,
) {
    let operation = intent(
        operation_hex_digit,
        idempotency_key,
        QuickChainOperationClassV1::Issue,
        actor,
        None,
        amount_minor,
        None,
        produced_at_ms,
    );

    let outcome = state
        .execute_balance_operation(&operation, supply_decision, txid)
        .expect("balance operation should commit");

    history.push(QuickChainAcceptedOperation::balance(
        outcome.record().clone(),
        supply_decision,
    ));
}

#[allow(clippy::too_many_arguments)]
fn accept_hold(
    state: &mut QuickChainAtomicState,
    history: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: &str,
    epoch_input: QuickChainHoldEpochInput,
    txid: &str,
    produced_at_ms: u64,
) {
    let operation = intent(
        operation_hex_digit,
        idempotency_key,
        op_class,
        actor,
        counterparty,
        amount_minor,
        Some(hold_id),
        produced_at_ms,
    );

    let outcome = state
        .execute_hold_operation(&operation, epoch_input, txid)
        .expect("hold operation should commit");

    history.push(QuickChainAcceptedOperation::hold(
        outcome.record().clone(),
        epoch_input,
    ));
}

fn tampered_record(
    original: &QuickChainCommittedOperationRecord,
    account_sequence: u64,
    ledger_sequence_start: u64,
    ledger_sequence_end: u64,
) -> QuickChainCommittedOperationRecord {
    QuickChainCommittedOperationRecord::new(
        original.intent().clone(),
        original.receipt_txid().to_string(),
        account_sequence,
        ledger_sequence_start,
        ledger_sequence_end,
    )
    .expect("tampered test record should still be structurally valid")
}

fn assert_accepted_record_mismatch(error: QuickChainExecutionError, expected_operation_id: &str) {
    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::AcceptedRecordMismatch {
            operation_id: expected_operation_id.to_string(),
        })
    );
}

#[test]
fn accepted_replay_rejects_hold_open_account_sequence_mismatch() {
    let hold = hold_id('a');

    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_balance(
        &mut live,
        &mut history,
        '1',
        "idem:accepted-sequence:issue",
        "account:alice",
        "100",
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:accepted-sequence:issue",
        1_000,
    );

    accept_hold(
        &mut live,
        &mut history,
        '2',
        "idem:accepted-sequence:hold-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "40",
        &hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:accepted-sequence:hold-open",
        1_001,
    );

    let original_hold_record = history[1].record().clone();

    assert_eq!(original_hold_record.account_sequence(), 2);
    assert_eq!(original_hold_record.ledger_sequence_start(), 2);
    assert_eq!(original_hold_record.ledger_sequence_end(), 2);

    history[1] = QuickChainAcceptedOperation::hold(
        tampered_record(
            &original_hold_record,
            original_hold_record.account_sequence() + 1,
            original_hold_record.ledger_sequence_start(),
            original_hold_record.ledger_sequence_end(),
        ),
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
    );

    let error = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect_err("accepted replay must reject corrupted hold account sequence");

    assert_accepted_record_mismatch(error, &original_hold_record.intent().operation_id);
}

#[test]
fn accepted_replay_rejects_hold_capture_primitive_ledger_range_mismatch() {
    let hold = hold_id('b');

    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_balance(
        &mut live,
        &mut history,
        '1',
        "idem:accepted-range:issue",
        "account:alice",
        "100",
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:accepted-range:issue",
        1_000,
    );

    accept_hold(
        &mut live,
        &mut history,
        '2',
        "idem:accepted-range:hold-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "40",
        &hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:accepted-range:hold-open",
        1_001,
    );

    accept_hold(
        &mut live,
        &mut history,
        '3',
        "idem:accepted-range:hold-capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "25",
        &hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:accepted-range:hold-capture",
        1_002,
    );

    let original_capture_record = history[2].record().clone();

    assert_eq!(original_capture_record.account_sequence(), 3);
    assert_eq!(original_capture_record.ledger_sequence_start(), 3);
    assert_eq!(original_capture_record.ledger_sequence_end(), 4);

    // HoldCapture occupies two primitive ledger postings. The persisted record
    // below is structurally valid, but it lies about the deterministic end
    // sequence. Accepted replay must detect that exact evidence mismatch.
    history[2] = QuickChainAcceptedOperation::hold(
        tampered_record(
            &original_capture_record,
            original_capture_record.account_sequence(),
            original_capture_record.ledger_sequence_start(),
            original_capture_record.ledger_sequence_start(),
        ),
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
    );

    let error = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect_err("accepted replay must reject corrupted hold capture ledger range");

    assert_accepted_record_mismatch(error, &original_capture_record.intent().operation_id);
}
