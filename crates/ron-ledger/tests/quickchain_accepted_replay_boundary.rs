// crates/ron-ledger/tests/quickchain_accepted_replay_boundary.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for accepted-history replay boundary validation.
//! RO:WHY — ECON/RES: durable accepted-history adapters must detect truncated, overextended, or wrong-chain history even when records are individually valid.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAcceptedReplayBoundary, QuickChainAtomicState, replay errors, and hold execution.
//! RO:INVARIANTS — boundaries are replay obligations only; they are not roots, signatures, checkpoints, finality, or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture txids and boundaries are inert public test values; no spend authority, wallet truth, or receipt fabrication.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAcceptedReplayBoundary, QuickChainAtomicState,
    QuickChainExecutionError, QuickChainHoldEpochInput, QuickChainReplayError,
    QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const ALT_CHAIN_ID: &str = "ron-altchain";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

#[allow(clippy::too_many_arguments)]
fn intent_for_chain(
    chain_id: &str,
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
        chain_id: chain_id.to_string(),
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
fn accept_issue_for_chain(
    chain_id: &str,
    state: &mut QuickChainAtomicState,
    history: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    actor: &str,
    amount_minor: &str,
    txid: &str,
    produced_at_ms: u64,
) {
    let operation = intent_for_chain(
        chain_id,
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
fn accept_hold_for_chain(
    chain_id: &str,
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
    let operation = intent_for_chain(
        chain_id,
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

fn accepted_terminal_history_for_chain(
    chain_id: &str,
) -> (
    QuickChainAtomicState,
    Vec<QuickChainAcceptedOperation>,
    QuickChainAcceptedReplayBoundary,
) {
    let hold = hold_id('a');

    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_issue_for_chain(
        chain_id,
        &mut live,
        &mut history,
        '1',
        "idem:accepted-boundary:issue",
        "account:alice",
        "100",
        "tx:roc:accepted-boundary:issue",
        1_000,
    );

    accept_hold_for_chain(
        chain_id,
        &mut live,
        &mut history,
        '2',
        "idem:accepted-boundary:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "60",
        &hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:accepted-boundary:open",
        1_001,
    );

    accept_hold_for_chain(
        chain_id,
        &mut live,
        &mut history,
        '3',
        "idem:accepted-boundary:capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "45",
        &hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:accepted-boundary:capture",
        1_002,
    );

    let boundary = live.accepted_replay_boundary();

    (live, history, boundary)
}

fn accepted_terminal_history() -> (
    QuickChainAtomicState,
    Vec<QuickChainAcceptedOperation>,
    QuickChainAcceptedReplayBoundary,
) {
    accepted_terminal_history_for_chain(CHAIN_ID)
}

#[test]
fn accepted_replay_boundary_round_trips_live_terminal_history() {
    let (live, history, boundary) = accepted_terminal_history();

    assert_eq!(boundary.operation_count(), 3);
    assert_eq!(boundary.next_ledger_sequence(), live.next_ledger_sequence());
    assert_eq!(boundary.chain_id(), Some(CHAIN_ID));

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &history,
        boundary.clone(),
    )
    .expect("accepted replay should satisfy exact live boundary");

    assert_eq!(rebuilt, live);
    assert_eq!(rebuilt.accepted_replay_boundary(), boundary);
}

#[test]
fn accepted_replay_boundary_rejects_truncated_valid_history() {
    let (_live, mut history, boundary) = accepted_terminal_history();

    let removed = history
        .pop()
        .expect("fixture should contain a terminal operation to truncate");
    assert_eq!(
        removed.record().intent().operation_id,
        operation_id('3'),
        "fixture should truncate the terminal capture operation"
    );

    let error =
        QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(&history, boundary)
            .expect_err("boundary must reject valid but truncated accepted history");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(
            QuickChainReplayError::AcceptedHistoryOperationCountMismatch {
                expected: 3,
                actual: 2,
            }
        )
    );
}

#[test]
fn accepted_replay_boundary_rejects_next_ledger_sequence_mismatch() {
    let (_live, history, boundary) = accepted_terminal_history();

    let wrong_boundary = QuickChainAcceptedReplayBoundary::with_chain_id(
        boundary.operation_count(),
        boundary
            .next_ledger_sequence()
            .checked_add(1)
            .expect("test boundary should not overflow"),
        CHAIN_ID,
    );

    let error = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &history,
        wrong_boundary.clone(),
    )
    .expect_err("boundary must reject expected next-ledger-sequence mismatch");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(
            QuickChainReplayError::AcceptedHistoryNextLedgerSequenceMismatch {
                expected: wrong_boundary.next_ledger_sequence(),
                actual: boundary.next_ledger_sequence(),
            }
        )
    );
}

#[test]
fn accepted_replay_boundary_rejects_same_shape_wrong_chain_history() {
    let (_live, _history, boundary) = accepted_terminal_history();
    let (_alt_live, alt_history, alt_boundary) = accepted_terminal_history_for_chain(ALT_CHAIN_ID);

    assert_eq!(boundary.operation_count(), alt_boundary.operation_count());
    assert_eq!(
        boundary.next_ledger_sequence(),
        alt_boundary.next_ledger_sequence()
    );
    assert_eq!(boundary.chain_id(), Some(CHAIN_ID));
    assert_eq!(alt_boundary.chain_id(), Some(ALT_CHAIN_ID));

    let error = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &alt_history,
        boundary,
    )
    .expect_err("boundary must reject same-shape accepted history from another chain");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::AcceptedHistoryChainIdMismatch {
            expected: CHAIN_ID.to_string(),
            actual: Some(ALT_CHAIN_ID.to_string()),
        })
    );
}

#[test]
fn numeric_boundary_without_chain_id_remains_legacy_shape_only_check() {
    let (_live, history, boundary) = accepted_terminal_history_for_chain(ALT_CHAIN_ID);

    let numeric_only_boundary = QuickChainAcceptedReplayBoundary::new(
        boundary.operation_count(),
        boundary.next_ledger_sequence(),
    );

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &history,
        numeric_only_boundary,
    )
    .expect("numeric-only boundary should remain shape-only for legacy adapters");

    assert_eq!(rebuilt.replay_index().chain_id(), Some(ALT_CHAIN_ID));
}

#[test]
fn empty_accepted_replay_boundary_accepts_empty_history() {
    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &[],
        QuickChainAcceptedReplayBoundary::empty(),
    )
    .expect("empty history should satisfy empty accepted-replay boundary");

    assert_eq!(rebuilt.operation_count(), 0);
    assert_eq!(rebuilt.next_ledger_sequence(), 1);
    assert_eq!(rebuilt.replay_index().chain_id(), None);
    assert_eq!(
        rebuilt.accepted_replay_boundary(),
        QuickChainAcceptedReplayBoundary::empty()
    );
}
