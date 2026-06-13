// crates/ron-ledger/tests/quickchain_terminal_hold_replay_evidence.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests proving terminal hold evidence survives accepted replay and remains terminal.
//! RO:WHY — ECON/RES: captured, released, and expired hold lifecycles must replay exactly before persistence, roots, proofs, or pruning exist.
//! RO:INTERACTS — QuickChainAtomicState, QuickChainAcceptedOperation, terminal hold records, state snapshots, and active-hold projection.
//! RO:INVARIANTS — terminal holds are durable replay evidence; active-hold projection excludes terminal history; no roots, hashes, clocks, IO, or settlement.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture receipt references and operation IDs are inert test values, not wallet authority or proofs.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainExecutionError,
    QuickChainHoldEpochInput, QuickChainHoldError, QuickChainHoldTerminalStatus,
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
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    supply_decision: QuickChainSupplyDecision,
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

fn build_terminal_hold_history() -> (
    QuickChainAtomicState,
    Vec<QuickChainAcceptedOperation>,
    String,
    String,
    String,
) {
    let captured_hold = hold_id('a');
    let released_hold = hold_id('b');
    let expired_hold = hold_id('c');

    let mut state = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_balance(
        &mut state,
        &mut history,
        '1',
        "idem:terminal-replay:issue-alice",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "200",
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:terminal-replay:issue-alice",
        1_000,
    );

    accept_hold(
        &mut state,
        &mut history,
        '2',
        "idem:terminal-replay:open-capture",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "60",
        &captured_hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:terminal-replay:open-capture",
        1_001,
    );

    accept_hold(
        &mut state,
        &mut history,
        '3',
        "idem:terminal-replay:capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "25",
        &captured_hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:terminal-replay:capture",
        1_002,
    );

    accept_hold(
        &mut state,
        &mut history,
        '4',
        "idem:terminal-replay:open-release",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        None,
        "30",
        &released_hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 2,
            expires_at_epoch: 8,
        },
        "tx:roc:terminal-replay:open-release",
        1_003,
    );

    accept_hold(
        &mut state,
        &mut history,
        '5',
        "idem:terminal-replay:release",
        QuickChainOperationClassV1::HoldRelease,
        "account:alice",
        None,
        "30",
        &released_hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 3 },
        "tx:roc:terminal-replay:release",
        1_004,
    );

    accept_hold(
        &mut state,
        &mut history,
        '6',
        "idem:terminal-replay:open-expire",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        None,
        "20",
        &expired_hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 4,
            expires_at_epoch: 5,
        },
        "tx:roc:terminal-replay:open-expire",
        1_005,
    );

    accept_hold(
        &mut state,
        &mut history,
        '7',
        "idem:terminal-replay:expire",
        QuickChainOperationClassV1::HoldExpire,
        "account:alice",
        None,
        "20",
        &expired_hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 5 },
        "tx:roc:terminal-replay:expire",
        1_006,
    );

    (state, history, captured_hold, released_hold, expired_hold)
}

#[test]
fn accepted_replay_preserves_captured_released_and_expired_terminal_holds() {
    let (live, history, captured_hold, released_hold, expired_hold) = build_terminal_hold_history();

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect("accepted terminal hold history should replay exactly");

    assert_eq!(rebuilt, live);

    assert!(rebuilt.active_hold(&captured_hold).is_none());
    assert!(rebuilt.active_hold(&released_hold).is_none());
    assert!(rebuilt.active_hold(&expired_hold).is_none());

    let captured = rebuilt
        .terminal_hold(&captured_hold)
        .expect("capture should leave terminal evidence");
    assert_eq!(captured.status(), QuickChainHoldTerminalStatus::Captured);
    assert_eq!(captured.account_id(), "account:alice");
    assert_eq!(
        captured.opened_counterparty_account_id(),
        Some("account:merchant")
    );
    assert_eq!(
        captured.terminal_counterparty_account_id(),
        Some("account:merchant")
    );
    assert_eq!(captured.original_amount_minor(), 60);
    assert_eq!(captured.terminal_amount_minor(), 25);
    assert_eq!(captured.uncaptured_remainder_minor(), 35);
    assert_eq!(captured.created_at_epoch(), 1);
    assert_eq!(captured.expires_at_epoch(), 10);
    assert_eq!(captured.terminal_at_epoch(), 2);

    let released = rebuilt
        .terminal_hold(&released_hold)
        .expect("release should leave terminal evidence");
    assert_eq!(released.status(), QuickChainHoldTerminalStatus::Released);
    assert_eq!(released.account_id(), "account:alice");
    assert_eq!(released.opened_counterparty_account_id(), None);
    assert_eq!(released.terminal_counterparty_account_id(), None);
    assert_eq!(released.original_amount_minor(), 30);
    assert_eq!(released.terminal_amount_minor(), 30);
    assert_eq!(released.uncaptured_remainder_minor(), 0);
    assert_eq!(released.created_at_epoch(), 2);
    assert_eq!(released.expires_at_epoch(), 8);
    assert_eq!(released.terminal_at_epoch(), 3);

    let expired = rebuilt
        .terminal_hold(&expired_hold)
        .expect("expiry should leave terminal evidence");
    assert_eq!(expired.status(), QuickChainHoldTerminalStatus::Expired);
    assert_eq!(expired.account_id(), "account:alice");
    assert_eq!(expired.opened_counterparty_account_id(), None);
    assert_eq!(expired.terminal_counterparty_account_id(), None);
    assert_eq!(expired.original_amount_minor(), 20);
    assert_eq!(expired.terminal_amount_minor(), 20);
    assert_eq!(expired.uncaptured_remainder_minor(), 0);
    assert_eq!(expired.created_at_epoch(), 4);
    assert_eq!(expired.expires_at_epoch(), 5);
    assert_eq!(expired.terminal_at_epoch(), 5);

    assert_eq!(rebuilt.balance_minor("account:alice"), 175);
    assert_eq!(rebuilt.balance_minor("account:merchant"), 25);
    assert_eq!(rebuilt.current_supply_minor(), 200);
    assert_eq!(rebuilt.held_minor("account:alice"), 0);
    assert_eq!(rebuilt.available_minor("account:alice").unwrap(), 175);
    assert_eq!(rebuilt.operation_count(), 7);

    let snapshot = rebuilt
        .state_snapshot()
        .expect("terminal-only hold state should snapshot");

    assert!(snapshot.active_holds().is_empty());

    let active_hold_payloads = snapshot
        .project_active_hold_leaf_payloads(&[])
        .expect("terminal holds are not active-hold leaves");

    assert!(active_hold_payloads.is_empty());
}

#[test]
fn terminal_hold_history_rebuilt_from_accepted_replay_blocks_reopen_and_retransition() {
    let (_live, history, captured_hold, _released_hold, _expired_hold) =
        build_terminal_hold_history();

    let mut rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect("accepted terminal hold history should replay exactly");

    let before_reopen_attempt = rebuilt.clone();

    let reopen = intent(
        '8',
        "idem:terminal-replay:reopen-captured",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "5",
        Some(&captured_hold),
        2_000,
    );

    let error = rebuilt
        .execute_hold_operation(
            &reopen,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 6,
                expires_at_epoch: 12,
            },
            "tx:roc:terminal-replay:reopen-captured",
        )
        .expect_err("terminal hold IDs cannot be reopened after accepted replay");

    assert_eq!(
        error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldIdAlreadyUsed {
            hold_id: captured_hold.clone(),
        })
    );
    assert_eq!(rebuilt, before_reopen_attempt);

    let before_second_terminal_attempt = rebuilt.clone();

    let release_again = intent(
        '9',
        "idem:terminal-replay:release-captured-again",
        QuickChainOperationClassV1::HoldRelease,
        "account:alice",
        None,
        "60",
        Some(&captured_hold),
        2_001,
    );

    let error = rebuilt
        .execute_hold_operation(
            &release_again,
            QuickChainHoldEpochInput::Terminal { current_epoch: 6 },
            "tx:roc:terminal-replay:release-captured-again",
        )
        .expect_err("terminal holds cannot transition again after accepted replay");

    assert_eq!(
        error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldAlreadyTerminal {
            hold_id: captured_hold,
        })
    );
    assert_eq!(rebuilt, before_second_terminal_attempt);
}
