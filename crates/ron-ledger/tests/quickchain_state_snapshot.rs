#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for deterministic, ordered, non-hashing QuickChain state snapshots.
//! RO:WHY — ECON/RES: account and active-hold projections must be stable before any leaf, hash, root, or checkpoint implementation.
//! RO:INTERACTS — QuickChainAtomicState, state_snapshot, hold execution, balance execution, and ron-proto operation intents.
//! RO:INVARIANTS — capture is read-only; accounts/holds sort by ID; terminal holds are excluded; balance equals held plus available.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — test receipt references, epochs, and idempotency keys grant no authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainHoldEpochInput, QuickChainSupplyDecision,
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

fn commit_issue(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    idempotency_key: &str,
    account_id: &str,
    amount_minor: &str,
    receipt_txid: &str,
) {
    state
        .execute_balance_operation(
            &intent(
                operation_hex_digit,
                idempotency_key,
                QuickChainOperationClassV1::Issue,
                account_id,
                None,
                amount_minor,
                None,
                1_000,
            ),
            QuickChainSupplyDecision::IssueApproved,
            receipt_txid,
        )
        .expect("test issue should commit");
}

#[allow(clippy::too_many_arguments)]
fn commit_open_hold(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    idempotency_key: &str,
    account_id: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: &str,
    created_at_epoch: u64,
    expires_at_epoch: u64,
    receipt_txid: &str,
) {
    state
        .execute_hold_operation(
            &intent(
                operation_hex_digit,
                idempotency_key,
                QuickChainOperationClassV1::HoldOpen,
                account_id,
                counterparty,
                amount_minor,
                Some(hold_id),
                2_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch,
                expires_at_epoch,
            },
            receipt_txid,
        )
        .expect("test hold should open");
}

fn commit_release(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    account_id: &str,
    amount_minor: &str,
    hold_id: &str,
    current_epoch: u64,
    receipt_txid: &str,
) {
    state
        .execute_hold_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:snapshot:release:{operation_hex_digit}"),
                QuickChainOperationClassV1::HoldRelease,
                account_id,
                None,
                amount_minor,
                Some(hold_id),
                3_000,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch },
            receipt_txid,
        )
        .expect("test hold should release");
}

fn build_equivalent_state(bob_first: bool) -> QuickChainAtomicState {
    let alice_hold = hold_id('3');
    let bob_hold = hold_id('4');
    let mut state = QuickChainAtomicState::new();

    let issue_alice = |state: &mut QuickChainAtomicState| {
        commit_issue(
            state,
            '1',
            "idem:snapshot:equivalent:alice:issue",
            "account:alice",
            "100",
            "tx:roc:equivalent:alice:issue",
        );
    };
    let issue_bob = |state: &mut QuickChainAtomicState| {
        commit_issue(
            state,
            '2',
            "idem:snapshot:equivalent:bob:issue",
            "account:bob",
            "200",
            "tx:roc:equivalent:bob:issue",
        );
    };
    let open_alice = |state: &mut QuickChainAtomicState| {
        commit_open_hold(
            state,
            '3',
            "idem:snapshot:equivalent:alice:hold",
            "account:alice",
            None,
            "30",
            &alice_hold,
            1,
            10,
            "tx:roc:equivalent:alice:hold",
        );
    };
    let open_bob = |state: &mut QuickChainAtomicState| {
        commit_open_hold(
            state,
            '4',
            "idem:snapshot:equivalent:bob:hold",
            "account:bob",
            None,
            "40",
            &bob_hold,
            1,
            10,
            "tx:roc:equivalent:bob:hold",
        );
    };

    if bob_first {
        issue_bob(&mut state);
        issue_alice(&mut state);
        open_bob(&mut state);
        open_alice(&mut state);
    } else {
        issue_alice(&mut state);
        issue_bob(&mut state);
        open_alice(&mut state);
        open_bob(&mut state);
    }

    state
}

#[test]
fn empty_state_snapshot_is_explicit_and_read_only() {
    let state = QuickChainAtomicState::new();
    let before = state.clone();

    let snapshot = state
        .state_snapshot()
        .expect("empty valid state should snapshot");

    assert_eq!(snapshot.chain_id(), None);
    assert!(snapshot.accounts().is_empty());
    assert!(snapshot.active_holds().is_empty());
    assert_eq!(snapshot.current_supply_minor(), 0);
    assert_eq!(snapshot.next_ledger_sequence(), 1);
    assert_eq!(snapshot.operation_count(), 0);
    assert_eq!(state, before);
}

#[test]
fn snapshot_sorts_accounts_and_active_holds_and_excludes_terminal_holds() {
    let hold_f = hold_id('f');
    let hold_1 = hold_id('1');
    let hold_8 = hold_id('8');
    let mut state = QuickChainAtomicState::new();

    // Deliberately commit accounts and holds out of lexical order.
    commit_issue(
        &mut state,
        'b',
        "idem:snapshot:bob:issue",
        "account:bob",
        "300",
        "tx:roc:snapshot:bob",
    );
    commit_issue(
        &mut state,
        'a',
        "idem:snapshot:alice:issue",
        "account:alice",
        "500",
        "tx:roc:snapshot:alice",
    );
    state
        .execute_balance_operation(
            &intent(
                'c',
                "idem:snapshot:transfer",
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:carol"),
                "50",
                None,
                1_500,
            ),
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:snapshot:transfer",
        )
        .expect("test transfer should commit");

    commit_open_hold(
        &mut state,
        'd',
        "idem:snapshot:hold:f",
        "account:alice",
        Some("account:merchant"),
        "100",
        &hold_f,
        3,
        30,
        "tx:roc:snapshot:hold:f",
    );
    commit_open_hold(
        &mut state,
        'e',
        "idem:snapshot:hold:1",
        "account:bob",
        None,
        "70",
        &hold_1,
        2,
        20,
        "tx:roc:snapshot:hold:1",
    );
    commit_open_hold(
        &mut state,
        'f',
        "idem:snapshot:hold:8",
        "account:alice",
        None,
        "40",
        &hold_8,
        4,
        40,
        "tx:roc:snapshot:hold:8",
    );
    commit_release(
        &mut state,
        '0',
        "account:alice",
        "40",
        &hold_8,
        4,
        "tx:roc:snapshot:release:8",
    );

    let before = state.clone();
    let snapshot = state
        .state_snapshot()
        .expect("valid populated state should snapshot");

    assert_eq!(snapshot.chain_id(), Some(CHAIN_ID));
    assert_eq!(snapshot.current_supply_minor(), 800);
    assert_eq!(snapshot.operation_count(), 7);
    assert_eq!(snapshot.next_ledger_sequence(), 9);

    let account_ids: Vec<_> = snapshot
        .accounts()
        .iter()
        .map(|account| account.account_id())
        .collect();
    assert_eq!(
        account_ids,
        vec!["account:alice", "account:bob", "account:carol"]
    );

    let alice = &snapshot.accounts()[0];
    assert_eq!(alice.balance_minor(), 450);
    assert_eq!(alice.held_minor(), 100);
    assert_eq!(alice.available_minor(), 350);
    assert_eq!(alice.account_sequence(), 5);

    let bob = &snapshot.accounts()[1];
    assert_eq!(bob.balance_minor(), 300);
    assert_eq!(bob.held_minor(), 70);
    assert_eq!(bob.available_minor(), 230);
    assert_eq!(bob.account_sequence(), 2);

    let carol = &snapshot.accounts()[2];
    assert_eq!(carol.balance_minor(), 50);
    assert_eq!(carol.held_minor(), 0);
    assert_eq!(carol.available_minor(), 50);

    // Carol's balance changed as the transfer destination, so her account-state
    // sequence advances even though the receipt carries Alice's actor sequence.
    assert_eq!(carol.account_sequence(), 1);

    for account in snapshot.accounts() {
        assert_eq!(
            account.balance_minor(),
            account
                .held_minor()
                .checked_add(account.available_minor())
                .expect("test account arithmetic should fit")
        );
    }

    let summed_supply: u128 = snapshot
        .accounts()
        .iter()
        .map(|account| account.balance_minor())
        .sum();
    assert_eq!(summed_supply, snapshot.current_supply_minor());

    let active_hold_ids: Vec<_> = snapshot
        .active_holds()
        .iter()
        .map(|hold| hold.hold_id())
        .collect();
    assert_eq!(active_hold_ids, vec![hold_1.as_str(), hold_f.as_str()]);
    assert!(!active_hold_ids.contains(&hold_8.as_str()));
    assert_eq!(state.hold_state().terminal_hold_count(), 1);

    let bob_hold = &snapshot.active_holds()[0];
    assert_eq!(bob_hold.account_id(), "account:bob");
    assert_eq!(bob_hold.counterparty_account_id(), None);
    assert_eq!(bob_hold.amount_minor(), 70);
    assert_eq!(bob_hold.created_at_epoch_number(), 2);
    assert_eq!(bob_hold.expires_at_epoch_number(), 20);

    let expected_open_operation_id = operation_id('e');
    assert_eq!(
        bob_hold.opened_operation_id(),
        expected_open_operation_id.as_str()
    );
    assert_eq!(bob_hold.opened_idempotency_key(), "idem:snapshot:hold:1");

    let alice_hold = &snapshot.active_holds()[1];
    assert_eq!(alice_hold.account_id(), "account:alice");
    assert_eq!(
        alice_hold.counterparty_account_id(),
        Some("account:merchant")
    );
    assert_eq!(alice_hold.amount_minor(), 100);
    assert_eq!(alice_hold.created_at_epoch_number(), 3);
    assert_eq!(alice_hold.expires_at_epoch_number(), 30);

    let held_from_rows: u128 = snapshot
        .active_holds()
        .iter()
        .filter(|hold| hold.account_id() == "account:alice")
        .map(|hold| hold.amount_minor())
        .sum();
    assert_eq!(held_from_rows, alice.held_minor());

    assert_eq!(state, before);
}

#[test]
fn equivalent_final_state_has_identical_snapshot_despite_cross_account_commit_order() {
    let first = build_equivalent_state(false);
    let second = build_equivalent_state(true);

    // The accepted-history indexes differ because primitive ledger sequences
    // were assigned in different order. The state projection is still equal.
    assert_ne!(first.replay_index(), second.replay_index());

    let first_snapshot = first
        .state_snapshot()
        .expect("first final state should snapshot");
    let second_snapshot = second
        .state_snapshot()
        .expect("second final state should snapshot");

    assert_eq!(first_snapshot, second_snapshot);
    assert_eq!(first_snapshot.chain_id(), Some(CHAIN_ID));
    assert_eq!(first_snapshot.current_supply_minor(), 300);
    assert_eq!(first_snapshot.operation_count(), 4);
    assert_eq!(first_snapshot.next_ledger_sequence(), 5);
}
