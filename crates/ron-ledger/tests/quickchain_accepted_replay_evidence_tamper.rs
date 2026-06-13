// crates/ron-ledger/tests/quickchain_accepted_replay_evidence_tamper.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for accepted-replay rejection of split-field receipt evidence tampering.
//! RO:WHY — ECON/RES: durable accepted history must restore committed evidence and replay inputs without silently accepting disagreement.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAtomicState, committed records, replay errors, balance and hold execution.
//! RO:INVARIANTS — receipt references are evidence, not authority; split replay receipt disagreement rejects; no roots, hashes, IO, or settlement.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture txids are inert backend references and never wallet receipts, signatures, proofs, or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainExecutionError,
    QuickChainHoldEpochInput, QuickChainReplayError, QuickChainSupplyDecision,
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
fn accept_issue(
    state: &mut QuickChainAtomicState,
    history: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    actor: &str,
    amount_minor: &str,
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
        .execute_balance_operation(&operation, QuickChainSupplyDecision::IssueApproved, txid)
        .expect("issue should commit");

    history.push(QuickChainAcceptedOperation::balance(
        outcome.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
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

fn assert_accepted_record_mismatch(error: QuickChainExecutionError, expected_operation_id: &str) {
    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::AcceptedRecordMismatch {
            operation_id: expected_operation_id.to_string(),
        })
    );
}

#[test]
fn accepted_replay_rejects_balance_replay_receipt_txid_mismatch() {
    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_issue(
        &mut live,
        &mut history,
        '1',
        "idem:accepted-receipt-tamper:issue",
        "account:alice",
        "100",
        "tx:roc:accepted-receipt-tamper:issue",
        1_000,
    );

    let original_record = history[0].record().clone();

    assert_eq!(
        original_record.receipt_txid(),
        "tx:roc:accepted-receipt-tamper:issue"
    );

    history[0] = QuickChainAcceptedOperation::balance_with_replay_receipt_txid(
        original_record.clone(),
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:accepted-receipt-tamper:issue-mutated",
    );

    let error = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect_err("accepted replay must reject split-field balance receipt mismatch");

    assert_accepted_record_mismatch(error, &original_record.intent().operation_id);
}

#[test]
fn accepted_replay_rejects_terminal_hold_replay_receipt_txid_mismatch() {
    let hold = hold_id('a');

    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_issue(
        &mut live,
        &mut history,
        '1',
        "idem:accepted-terminal-receipt:issue",
        "account:alice",
        "100",
        "tx:roc:accepted-terminal-receipt:issue",
        1_000,
    );

    accept_hold(
        &mut live,
        &mut history,
        '2',
        "idem:accepted-terminal-receipt:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "60",
        &hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:accepted-terminal-receipt:open",
        1_001,
    );

    accept_hold(
        &mut live,
        &mut history,
        '3',
        "idem:accepted-terminal-receipt:capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "45",
        &hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:accepted-terminal-receipt:capture",
        1_002,
    );

    let original_capture_record = history[2].record().clone();

    assert_eq!(
        original_capture_record.receipt_txid(),
        "tx:roc:accepted-terminal-receipt:capture"
    );
    assert!(
        live.terminal_hold(&hold).is_some(),
        "live state should retain terminal hold evidence before tamper replay"
    );

    history[2] = QuickChainAcceptedOperation::hold_with_replay_receipt_txid(
        original_capture_record.clone(),
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:accepted-terminal-receipt:capture-mutated",
    );

    let error = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect_err("accepted replay must reject split-field terminal hold receipt mismatch");

    assert_accepted_record_mismatch(error, &original_capture_record.intent().operation_id);
}
