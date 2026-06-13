// crates/ron-ledger/tests/quickchain_retry_after_accepted_rebuild.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for exact retry behavior after accepted QuickChain state rebuild.
//! RO:WHY — ECON/RES: replayed accepted history must restore retry indexes so duplicates return original evidence without reauthorization.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAtomicState, replay index, balance execution, and hold execution.
//! RO:INVARIANTS — exact retries after rebuild do not mutate state, consume sequence, reauthorize supply, validate new txids, or re-evaluate new epochs.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — invalid retry txids are inert test inputs proving they are ignored only for exact accepted retries.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainHoldEpochInput,
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
fn balance_exact_retry_after_accepted_rebuild_returns_original_before_new_authorization_or_txid() {
    let issue = intent(
        '1',
        "idem:retry-after-rebuild:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    let mut live = QuickChainAtomicState::new();

    let original = live
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:retry-after-rebuild:issue",
        )
        .expect("original issue should commit");

    assert!(original.is_committed());
    assert_eq!(live.balance_minor("account:alice"), 100);
    assert_eq!(live.current_supply_minor(), 100);
    assert_eq!(live.operation_count(), 1);
    assert_eq!(live.next_ledger_sequence(), 2);

    let accepted = [QuickChainAcceptedOperation::balance(
        original.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
    )];

    let mut rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&accepted)
        .expect("accepted issue history should rebuild exactly");

    assert_eq!(rebuilt, live);

    let before_retry = rebuilt.clone();

    let retry = rebuilt
        .execute_balance_operation(
            &issue,
            // This would reject for a fresh issue, but exact retry must return
            // original evidence before reauthorization is consulted.
            QuickChainSupplyDecision::NoSupplyChange,
            // This would reject if a new committed record were being built.
            "invalid retry receipt txid with spaces",
        )
        .expect("exact retry after accepted rebuild should return original evidence");

    assert!(retry.is_retry());
    assert_eq!(retry.record(), original.record());
    assert!(retry.transition().is_none());

    assert_eq!(rebuilt, before_retry);
    assert_eq!(rebuilt.balance_minor("account:alice"), 100);
    assert_eq!(rebuilt.current_supply_minor(), 100);
    assert_eq!(rebuilt.operation_count(), 1);
    assert_eq!(rebuilt.next_ledger_sequence(), 2);
}

#[test]
fn hold_exact_retry_after_accepted_rebuild_returns_original_before_new_epoch_or_txid() {
    let hold = hold_id('a');

    let issue = intent(
        '1',
        "idem:retry-after-rebuild:hold-issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    let open = intent(
        '2',
        "idem:retry-after-rebuild:hold-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "40",
        Some(&hold),
        1_001,
    );

    let mut live = QuickChainAtomicState::new();

    let issue_outcome = live
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:retry-after-rebuild:hold-issue",
        )
        .expect("funding issue should commit");

    let open_epoch_input = QuickChainHoldEpochInput::Open {
        created_at_epoch: 1,
        expires_at_epoch: 10,
    };

    let open_outcome = live
        .execute_hold_operation(
            &open,
            open_epoch_input,
            "tx:roc:retry-after-rebuild:hold-open",
        )
        .expect("hold open should commit");

    assert!(open_outcome.is_committed());
    assert_eq!(live.balance_minor("account:alice"), 100);
    assert_eq!(live.held_minor("account:alice"), 40);
    assert_eq!(live.available_minor("account:alice").unwrap(), 60);
    assert_eq!(live.operation_count(), 2);
    assert_eq!(live.next_ledger_sequence(), 3);
    assert!(live.active_hold(&hold).is_some());

    let accepted = [
        QuickChainAcceptedOperation::balance(
            issue_outcome.record().clone(),
            QuickChainSupplyDecision::IssueApproved,
        ),
        QuickChainAcceptedOperation::hold(open_outcome.record().clone(), open_epoch_input),
    ];

    let mut rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&accepted)
        .expect("accepted issue + hold history should rebuild exactly");

    assert_eq!(rebuilt, live);

    let before_retry = rebuilt.clone();

    let retry = rebuilt
        .execute_hold_operation(
            &open,
            // This mismatches HoldOpen and would reject for a fresh operation,
            // but exact retry must return original evidence first.
            QuickChainHoldEpochInput::Terminal { current_epoch: 999 },
            // This would reject if new committed evidence were being created.
            "invalid retry receipt txid with spaces",
        )
        .expect("exact hold retry after accepted rebuild should return original evidence");

    assert!(retry.is_retry());
    assert_eq!(retry.record(), open_outcome.record());
    assert!(retry.transition().is_none());

    assert_eq!(rebuilt, before_retry);
    assert_eq!(rebuilt.balance_minor("account:alice"), 100);
    assert_eq!(rebuilt.held_minor("account:alice"), 40);
    assert_eq!(rebuilt.available_minor("account:alice").unwrap(), 60);
    assert_eq!(rebuilt.operation_count(), 2);
    assert_eq!(rebuilt.next_ledger_sequence(), 3);
    assert!(rebuilt.active_hold(&hold).is_some());
    assert!(rebuilt.terminal_hold(&hold).is_none());
}
