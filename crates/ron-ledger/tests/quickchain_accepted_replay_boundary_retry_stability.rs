// crates/ron-ledger/tests/quickchain_accepted_replay_boundary_retry_stability.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Tests that exact retries do not advance accepted-history replay boundaries.
//! RO:WHY — ECON/RES: retries must return original evidence without extending durable accepted history, sequences, or boundary obligations.
//! RO:INTERACTS — QuickChainAtomicState, QuickChainAcceptedReplayBoundary, balance execution, hold execution, and ron-proto intents.
//! RO:INVARIANTS — exact retries do not mutate balances, holds, operation count, primitive ledger sequence, account sequence, or chain binding.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture txids are inert public test values; no roots, checkpoints, signatures, settlement, or wallet authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionDisposition, QuickChainHoldEpochInput,
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

#[test]
fn accepted_replay_boundary_is_stable_across_exact_balance_retry() {
    let mut state = QuickChainAtomicState::new();

    let issue = intent(
        '1',
        "idem:retry-boundary:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    let committed = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:retry-boundary:issue",
        )
        .expect("fresh issue should commit");

    assert_eq!(
        committed.disposition(),
        QuickChainExecutionDisposition::Committed
    );

    let boundary_before_retry = state.accepted_replay_boundary();
    let account_sequence_before_retry = state.last_account_sequence("account:alice");
    let balance_before_retry = state.balance_minor("account:alice");

    let retried = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:retry-boundary:issue-mutated",
        )
        .expect("exact retry should return original before reauthorization or txid validation");

    assert_eq!(
        retried.disposition(),
        QuickChainExecutionDisposition::Retried
    );
    assert_eq!(retried.record(), committed.record());

    assert_eq!(state.accepted_replay_boundary(), boundary_before_retry);
    assert_eq!(
        state.last_account_sequence("account:alice"),
        account_sequence_before_retry
    );
    assert_eq!(state.balance_minor("account:alice"), balance_before_retry);
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.next_ledger_sequence(), 2);
    assert_eq!(state.replay_index().chain_id(), Some(CHAIN_ID));
}

#[test]
fn accepted_replay_boundary_is_stable_across_exact_hold_retry() {
    let mut state = QuickChainAtomicState::new();
    let hold = hold_id('a');

    let issue = intent(
        '1',
        "idem:retry-boundary:hold-issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:retry-boundary:hold-issue",
        )
        .expect("issue should commit before hold");

    let open = intent(
        '2',
        "idem:retry-boundary:hold-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "60",
        Some(&hold),
        1_001,
    );

    let epoch_input = QuickChainHoldEpochInput::Open {
        created_at_epoch: 1,
        expires_at_epoch: 10,
    };

    let committed = state
        .execute_hold_operation(&open, epoch_input, "tx:roc:retry-boundary:hold-open")
        .expect("fresh hold open should commit");

    assert_eq!(
        committed.disposition(),
        QuickChainExecutionDisposition::Committed
    );

    let boundary_before_retry = state.accepted_replay_boundary();
    let account_sequence_before_retry = state.last_account_sequence("account:alice");
    let balance_before_retry = state.balance_minor("account:alice");
    let held_before_retry = state.held_minor("account:alice");
    let available_before_retry = state
        .available_minor("account:alice")
        .expect("available balance should derive");

    let retried = state
        .execute_hold_operation(
            &open,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 999,
                expires_at_epoch: 1_000,
            },
            "tx:roc:retry-boundary:hold-open-mutated",
        )
        .expect("exact hold retry should return original before new epoch or txid validation");

    assert_eq!(
        retried.disposition(),
        QuickChainExecutionDisposition::Retried
    );
    assert_eq!(retried.record(), committed.record());

    assert_eq!(state.accepted_replay_boundary(), boundary_before_retry);
    assert_eq!(
        state.last_account_sequence("account:alice"),
        account_sequence_before_retry
    );
    assert_eq!(state.balance_minor("account:alice"), balance_before_retry);
    assert_eq!(state.held_minor("account:alice"), held_before_retry);
    assert_eq!(
        state
            .available_minor("account:alice")
            .expect("available balance should still derive"),
        available_before_retry
    );
    assert_eq!(state.operation_count(), 2);
    assert_eq!(state.next_ledger_sequence(), 3);
    assert_eq!(state.replay_index().chain_id(), Some(CHAIN_ID));
}
