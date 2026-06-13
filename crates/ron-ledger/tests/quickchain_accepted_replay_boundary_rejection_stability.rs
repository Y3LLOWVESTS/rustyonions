// crates/ron-ledger/tests/quickchain_accepted_replay_boundary_rejection_stability.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Tests that rejected fresh operations do not advance accepted-history replay boundaries.
//! RO:WHY — ECON/RES: failed submissions must not consume durable operation count, primitive ledger sequence, account sequence, or chain boundary.
//! RO:INTERACTS — QuickChainAtomicState, QuickChainAcceptedReplayBoundary, balance execution, hold execution, and ron-proto intents.
//! RO:INVARIANTS — rejected operations leave accepted replay boundary and economic state unchanged; no roots, checkpoints, signatures, finality, or settlement.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture txids are inert public test values; no spend authority, wallet truth, receipt fabrication, or chain authority.
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

fn seed_alice_with_roc(state: &mut QuickChainAtomicState) {
    let issue = intent(
        '1',
        "idem:boundary-reject-stability:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    let outcome = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:boundary-reject-stability:issue",
        )
        .expect("seed issue should commit");

    assert_eq!(
        outcome.disposition(),
        QuickChainExecutionDisposition::Committed
    );
}

#[test]
fn accepted_replay_boundary_is_stable_after_rejected_balance_operation() {
    let mut state = QuickChainAtomicState::new();
    seed_alice_with_roc(&mut state);

    let boundary_before_reject = state.accepted_replay_boundary();
    let alice_sequence_before_reject = state.last_account_sequence("account:alice");
    let bob_sequence_before_reject = state.last_account_sequence("account:bob");
    let alice_balance_before_reject = state.balance_minor("account:alice");
    let bob_balance_before_reject = state.balance_minor("account:bob");

    let overdraft = intent(
        '2',
        "idem:boundary-reject-stability:overdraft-transfer",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "101",
        None,
        1_001,
    );

    let error = state
        .execute_balance_operation(
            &overdraft,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:boundary-reject-stability:overdraft-transfer",
        )
        .expect_err("overdraft transfer must reject without boundary mutation");

    assert!(
        error.to_string().contains("underflow")
            || error.to_string().contains("insufficient")
            || error.to_string().contains("available"),
        "unexpected rejection shape: {error}"
    );

    assert_eq!(state.accepted_replay_boundary(), boundary_before_reject);
    assert_eq!(
        state.last_account_sequence("account:alice"),
        alice_sequence_before_reject
    );
    assert_eq!(
        state.last_account_sequence("account:bob"),
        bob_sequence_before_reject
    );
    assert_eq!(
        state.balance_minor("account:alice"),
        alice_balance_before_reject
    );
    assert_eq!(
        state.balance_minor("account:bob"),
        bob_balance_before_reject
    );
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.next_ledger_sequence(), 2);
    assert_eq!(state.replay_index().chain_id(), Some(CHAIN_ID));
}

#[test]
fn accepted_replay_boundary_is_stable_after_rejected_hold_operation() {
    let mut state = QuickChainAtomicState::new();
    seed_alice_with_roc(&mut state);

    let boundary_before_reject = state.accepted_replay_boundary();
    let alice_sequence_before_reject = state.last_account_sequence("account:alice");
    let alice_balance_before_reject = state.balance_minor("account:alice");
    let alice_held_before_reject = state.held_minor("account:alice");
    let alice_available_before_reject = state
        .available_minor("account:alice")
        .expect("available balance should derive before rejected hold");

    let hold = hold_id('a');

    let over_reserve = intent(
        '2',
        "idem:boundary-reject-stability:over-reserve",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "101",
        Some(&hold),
        1_001,
    );

    let error = state
        .execute_hold_operation(
            &over_reserve,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "tx:roc:boundary-reject-stability:over-reserve",
        )
        .expect_err("over-reserved hold must reject without boundary mutation");

    assert!(
        error.to_string().contains("insufficient available funds")
            || error.to_string().contains("available"),
        "unexpected rejection shape: {error}"
    );

    assert_eq!(state.accepted_replay_boundary(), boundary_before_reject);
    assert_eq!(
        state.last_account_sequence("account:alice"),
        alice_sequence_before_reject
    );
    assert_eq!(
        state.balance_minor("account:alice"),
        alice_balance_before_reject
    );
    assert_eq!(state.held_minor("account:alice"), alice_held_before_reject);
    assert_eq!(
        state
            .available_minor("account:alice")
            .expect("available balance should still derive after rejected hold"),
        alice_available_before_reject
    );
    assert!(state.active_hold(&hold).is_none());
    assert!(state.terminal_hold(&hold).is_none());
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.next_ledger_sequence(), 2);
    assert_eq!(state.replay_index().chain_id(), Some(CHAIN_ID));
}
