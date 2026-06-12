#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for deterministic reconstruction from ordered accepted QuickChain operations.
//! RO:WHY — ECON/RES: replay must use the live transition path and reproduce balances, holds, identities, and sequences exactly.
//! RO:INTERACTS — ron_ledger::quickchain accepted replay API and ron_proto operation intents.
//! RO:INVARIANTS — accepted duplicates reject; expected committed evidence must match; replay failure exposes no partial state.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — test decisions and receipt references are inert bounded values, not real capabilities.
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

#[test]
fn mixed_accepted_history_rebuilds_identical_complete_state() {
    let hold = hold_id('a');
    let mut live = QuickChainAtomicState::new();
    let mut accepted = Vec::new();

    let issue = intent(
        '1',
        "idem:rebuild:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        None,
        1_000,
    );

    let issue_outcome = live
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:rebuild:issue",
        )
        .expect("issue should commit");

    accepted.push(QuickChainAcceptedOperation::balance(
        issue_outcome.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
    ));

    let open = intent(
        '2',
        "idem:rebuild:hold-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:bob"),
        "60",
        Some(&hold),
        1_001,
    );

    let open_epochs = QuickChainHoldEpochInput::Open {
        created_at_epoch: 1,
        expires_at_epoch: 10,
    };

    let open_outcome = live
        .execute_hold_operation(&open, open_epochs, "tx:roc:rebuild:hold-open")
        .expect("hold should open");

    accepted.push(QuickChainAcceptedOperation::hold(
        open_outcome.record().clone(),
        open_epochs,
    ));

    let transfer = intent(
        '3',
        "idem:rebuild:transfer",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "20",
        None,
        1_002,
    );

    let transfer_outcome = live
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:rebuild:transfer",
        )
        .expect("available funds should transfer");

    accepted.push(QuickChainAcceptedOperation::balance(
        transfer_outcome.record().clone(),
        QuickChainSupplyDecision::NoSupplyChange,
    ));

    let capture = intent(
        '4',
        "idem:rebuild:hold-capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:bob"),
        "50",
        Some(&hold),
        1_003,
    );

    let capture_epoch = QuickChainHoldEpochInput::Terminal { current_epoch: 2 };

    let capture_outcome = live
        .execute_hold_operation(&capture, capture_epoch, "tx:roc:rebuild:hold-capture")
        .expect("hold should capture");

    accepted.push(QuickChainAcceptedOperation::hold(
        capture_outcome.record().clone(),
        capture_epoch,
    ));

    let burn = intent(
        '5',
        "idem:rebuild:burn",
        QuickChainOperationClassV1::Burn,
        "account:bob",
        None,
        "10",
        None,
        1_004,
    );

    let burn_outcome = live
        .execute_balance_operation(
            &burn,
            QuickChainSupplyDecision::BurnApproved,
            "tx:roc:rebuild:burn",
        )
        .expect("burn should commit");

    accepted.push(QuickChainAcceptedOperation::balance(
        burn_outcome.record().clone(),
        QuickChainSupplyDecision::BurnApproved,
    ));

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&accepted)
        .expect("accepted history should rebuild");

    assert_eq!(rebuilt, live);
    assert_eq!(rebuilt.balance_minor("account:alice"), 30);
    assert_eq!(rebuilt.balance_minor("account:bob"), 60);
    assert_eq!(rebuilt.current_supply_minor(), 90);
    assert_eq!(rebuilt.held_minor("account:alice"), 0);
    assert!(rebuilt.active_hold(&hold).is_none());
    assert!(rebuilt.terminal_hold(&hold).is_some());
    assert_eq!(rebuilt.operation_count(), 5);
    assert_eq!(rebuilt.last_account_sequence("account:alice"), 4);

    // Bob's account state changes three times:
    // transfer credit -> 1, hold-capture credit -> 2, burn -> 3.
    assert_eq!(rebuilt.last_account_sequence("account:bob"), 3);

    assert_eq!(rebuilt.next_ledger_sequence(), 8);
}

#[test]
fn duplicate_record_in_accepted_history_rejects() {
    let submitted = intent(
        '6',
        "idem:rebuild:duplicate",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "25",
        None,
        2_000,
    );

    let mut live = QuickChainAtomicState::new();
    let outcome = live
        .execute_balance_operation(
            &submitted,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:rebuild:duplicate",
        )
        .expect("issue should commit");

    let accepted = QuickChainAcceptedOperation::balance(
        outcome.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
    );

    let error =
        QuickChainAtomicState::rebuild_from_accepted_operations(&[accepted.clone(), accepted])
            .expect_err("accepted history may not contain a duplicate commit");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::DuplicateAcceptedOperation {
            operation_id: submitted.operation_id,
        })
    );
}

#[test]
fn mismatched_committed_sequences_reject_reconstruction() {
    let submitted = intent(
        '7',
        "idem:rebuild:mismatch",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "10",
        None,
        3_000,
    );

    let mismatched_record = ron_ledger::quickchain::QuickChainCommittedOperationRecord::new(
        submitted.clone(),
        "tx:roc:rebuild:mismatch",
        2,
        1,
        1,
    )
    .expect("record shape itself is valid");

    let history = [QuickChainAcceptedOperation::balance(
        mismatched_record,
        QuickChainSupplyDecision::IssueApproved,
    )];

    let error = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect_err("replay must detect an incorrect accepted account sequence");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::AcceptedRecordMismatch {
            operation_id: submitted.operation_id,
        })
    );
}

#[test]
fn contradictory_accepted_history_rejects_through_live_transition_rules() {
    let unfunded_transfer = intent(
        '8',
        "idem:rebuild:unfunded",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "1",
        None,
        4_000,
    );

    let claimed_record = ron_ledger::quickchain::QuickChainCommittedOperationRecord::new(
        unfunded_transfer,
        "tx:roc:rebuild:unfunded",
        1,
        1,
        2,
    )
    .expect("claimed record shape is valid");

    let history = [QuickChainAcceptedOperation::balance(
        claimed_record,
        QuickChainSupplyDecision::NoSupplyChange,
    )];

    QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect_err("accepted history cannot bypass ordinary balance execution rules");
}
