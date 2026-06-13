// crates/ron-ledger/tests/quickchain_accepted_replay_boundary_extremes.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Edge-case tests for accepted-history replay boundary validation.
//! RO:WHY — ECON/RES: accepted-history adapters must reject valid-but-overextended history and impossible chain-bound empty restores.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAcceptedReplayBoundary, QuickChainAtomicState, replay errors, and hold execution.
//! RO:INVARIANTS — boundaries are replay obligations only; they are not roots, signatures, checkpoints, finality, or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture txids and chain IDs are inert public test values; no spend authority, wallet truth, or receipt fabrication.
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

fn accepted_terminal_history() -> (
    QuickChainAtomicState,
    Vec<QuickChainAcceptedOperation>,
    QuickChainAcceptedReplayBoundary,
) {
    let hold = hold_id('a');

    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_issue(
        &mut live,
        &mut history,
        '1',
        "idem:accepted-boundary-extreme:issue",
        "account:alice",
        "100",
        "tx:roc:accepted-boundary-extreme:issue",
        1_000,
    );

    accept_hold(
        &mut live,
        &mut history,
        '2',
        "idem:accepted-boundary-extreme:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "60",
        &hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:accepted-boundary-extreme:open",
        1_001,
    );

    accept_hold(
        &mut live,
        &mut history,
        '3',
        "idem:accepted-boundary-extreme:capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "45",
        &hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:accepted-boundary-extreme:capture",
        1_002,
    );

    let boundary = live.accepted_replay_boundary();

    (live, history, boundary)
}

#[test]
fn accepted_replay_boundary_rejects_overextended_valid_history() {
    let (mut live, mut history, boundary) = accepted_terminal_history();

    assert_eq!(boundary.operation_count(), 3);
    assert_eq!(boundary.chain_id(), Some(CHAIN_ID));

    accept_issue(
        &mut live,
        &mut history,
        '4',
        "idem:accepted-boundary-extreme:extra-issue",
        "account:bob",
        "7",
        "tx:roc:accepted-boundary-extreme:extra-issue",
        1_003,
    );

    assert_eq!(history.len(), 4);
    assert_eq!(live.operation_count(), 4);

    let error =
        QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(&history, boundary)
            .expect_err("boundary must reject valid but overextended accepted history");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(
            QuickChainReplayError::AcceptedHistoryOperationCountMismatch {
                expected: 3,
                actual: 4,
            }
        )
    );
}

#[test]
fn chain_bound_empty_boundary_rejects_empty_history() {
    let boundary = QuickChainAcceptedReplayBoundary::with_chain_id(0, 1, CHAIN_ID);

    let error =
        QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(&[], boundary)
            .expect_err("chain-bound empty boundary must reject when no chain was rebuilt");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::AcceptedHistoryChainIdMismatch {
            expected: CHAIN_ID.to_string(),
            actual: None,
        })
    );
}
