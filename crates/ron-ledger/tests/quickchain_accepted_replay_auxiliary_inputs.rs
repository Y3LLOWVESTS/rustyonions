// crates/ron-ledger/tests/quickchain_accepted_replay_auxiliary_inputs.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests proving accepted replay auxiliary inputs are explicit deterministic replay data.
//! RO:WHY — ECON/RES: supply decisions and hold epoch inputs affect state and must not be invented, defaulted, or treated as authority.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAtomicState, balance execution, hold execution, and accepted replay.
//! RO:INVARIANTS — replay auxiliary inputs are explicit; wrong supply decisions reject; wrong hold epochs change state and therefore must be persisted exactly.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture decisions and receipt references are inert test values, not capabilities or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainExecutionError,
    QuickChainHoldEpochInput, QuickChainSupplyDecision, QuickChainTransitionError,
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
fn accepted_replay_supply_decision_must_match_original_execution() {
    let issue = intent(
        '1',
        "idem:aux:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    let mut live = QuickChainAtomicState::new();
    let issue_outcome = live
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:aux:issue",
        )
        .expect("issue should commit with explicit approval");

    let wrong_history = [QuickChainAcceptedOperation::balance(
        issue_outcome.record().clone(),
        QuickChainSupplyDecision::NoSupplyChange,
    )];

    let error = QuickChainAtomicState::rebuild_from_accepted_operations(&wrong_history)
        .expect_err("accepted replay may not silently change supply authorization input");

    assert_eq!(
        error,
        QuickChainExecutionError::Transition(QuickChainTransitionError::UnauthorizedIssue)
    );

    assert_eq!(live.balance_minor("account:alice"), 100);
    assert_eq!(live.current_supply_minor(), 100);
    assert_eq!(live.operation_count(), 1);
}

#[test]
fn accepted_replay_hold_epoch_input_must_be_preserved_exactly() {
    let hold = hold_id('a');

    let issue = intent(
        '2',
        "idem:aux:issue-for-hold",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        2_000,
    );

    let open = intent(
        '3',
        "idem:aux:hold-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:bob"),
        "70",
        Some(&hold),
        2_001,
    );

    let original_epochs = QuickChainHoldEpochInput::Open {
        created_at_epoch: 1,
        expires_at_epoch: 10,
    };

    let drifted_epochs = QuickChainHoldEpochInput::Open {
        created_at_epoch: 2,
        expires_at_epoch: 10,
    };

    let mut live = QuickChainAtomicState::new();

    let issue_outcome = live
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:aux:issue-for-hold",
        )
        .expect("issue should commit");

    let open_outcome = live
        .execute_hold_operation(&open, original_epochs, "tx:roc:aux:hold-open")
        .expect("hold should open with explicit epochs");

    let correct_history = [
        QuickChainAcceptedOperation::balance(
            issue_outcome.record().clone(),
            QuickChainSupplyDecision::IssueApproved,
        ),
        QuickChainAcceptedOperation::hold(open_outcome.record().clone(), original_epochs),
    ];

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&correct_history)
        .expect("exact accepted history should rebuild");
    assert_eq!(rebuilt, live);

    let drifted_history = [
        QuickChainAcceptedOperation::balance(
            issue_outcome.record().clone(),
            QuickChainSupplyDecision::IssueApproved,
        ),
        QuickChainAcceptedOperation::hold(open_outcome.record().clone(), drifted_epochs),
    ];

    let drifted = QuickChainAtomicState::rebuild_from_accepted_operations(&drifted_history)
        .expect("drifted auxiliary input is still executable but represents different state");

    assert_ne!(drifted, live);

    let live_hold = live
        .active_hold(&hold)
        .expect("original live state should retain active hold");
    let drifted_hold = drifted
        .active_hold(&hold)
        .expect("drifted replay should retain active hold");

    assert_eq!(live_hold.created_at_epoch(), 1);
    assert_eq!(drifted_hold.created_at_epoch(), 2);

    assert_eq!(live_hold.expires_at_epoch(), 10);
    assert_eq!(drifted_hold.expires_at_epoch(), 10);

    assert_eq!(
        live.balance_minor("account:alice"),
        drifted.balance_minor("account:alice")
    );
    assert_eq!(
        live.held_minor("account:alice"),
        drifted.held_minor("account:alice")
    );
    assert_eq!(
        live.available_minor("account:alice").unwrap(),
        drifted.available_minor("account:alice").unwrap()
    );
}
