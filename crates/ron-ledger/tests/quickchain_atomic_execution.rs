#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for atomic composition of QuickChain balances, replay identity, and ledger-owned sequences.
//! RO:WHY — ECON/RES: a fresh operation must commit all substates together while retries and every rejection mutate nothing.
//! RO:INTERACTS — ron_ledger::quickchain, ron_proto::quickchain operation DTOs.
//! RO:INVARIANTS — exact retry is a no-op; sequence ranges are deterministic; arithmetic/index failures fully roll back.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — receipt txids and supply decisions are bounded test inputs, not real capabilities.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionDisposition, QuickChainExecutionError,
    QuickChainReplayError, QuickChainSupplyDecision, QuickChainTransitionError,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
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
        hold_id: None,
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
                1_000,
            ),
            QuickChainSupplyDecision::IssueApproved,
            receipt_txid,
        )
        .expect("test issue should commit");
}

#[test]
fn fresh_issue_commits_balance_identity_and_sequences_together() {
    let submitted = intent(
        '1',
        "idem:atomic:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        1_000,
    );

    let mut state = QuickChainAtomicState::new();

    let outcome = state
        .execute_balance_operation(
            &submitted,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:0001",
        )
        .expect("authorized issue should commit");

    assert_eq!(
        outcome.disposition(),
        QuickChainExecutionDisposition::Committed
    );
    assert!(outcome.is_committed());
    assert!(!outcome.is_retry());
    assert!(outcome.transition().is_some());

    assert_eq!(outcome.record().receipt_txid(), "tx:roc:0001");
    assert_eq!(outcome.record().account_sequence(), 1);
    assert_eq!(outcome.record().ledger_sequence_start(), 1);
    assert_eq!(outcome.record().ledger_sequence_end(), 1);

    assert_eq!(state.balance_minor("account:alice"), 100);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.last_account_sequence("account:alice"), 1);
    assert_eq!(state.next_ledger_sequence(), 2);

    assert_eq!(
        state
            .committed_operation(&submitted.operation_id)
            .expect("operation must be indexed")
            .receipt_txid(),
        "tx:roc:0001"
    );
}

#[test]
fn transfer_advances_both_account_state_sequences_and_returns_actor_sequence() {
    let mut state = QuickChainAtomicState::new();

    commit_issue(
        &mut state,
        '1',
        "idem:seed:alice",
        "account:alice",
        "100",
        "tx:roc:0010",
    );

    let transfer = intent(
        '2',
        "idem:atomic:transfer",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "40",
        2_000,
    );

    let outcome = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:0011",
        )
        .expect("funded transfer should commit");

    // The committed receipt evidence carries the actor's sequence.
    assert_eq!(outcome.record().account_sequence(), 2);
    assert_eq!(outcome.record().ledger_sequence_start(), 2);
    assert_eq!(outcome.record().ledger_sequence_end(), 3);

    assert_eq!(state.balance_minor("account:alice"), 60);
    assert_eq!(state.balance_minor("account:bob"), 40);
    assert_eq!(state.current_supply_minor(), 100);

    // Both distinct account leaves changed and therefore advance once.
    assert_eq!(state.last_account_sequence("account:alice"), 2);
    assert_eq!(state.last_account_sequence("account:bob"), 1);
    assert_eq!(state.next_ledger_sequence(), 4);
    assert_eq!(state.operation_count(), 2);
}

#[test]
fn self_transfer_advances_one_account_state_sequence_once() {
    let mut state = QuickChainAtomicState::new();

    commit_issue(
        &mut state,
        '1',
        "idem:self-transfer:seed",
        "account:alice",
        "100",
        "tx:roc:self-transfer:seed",
    );

    let transfer = intent(
        '2',
        "idem:self-transfer:commit",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:alice"),
        "40",
        2_100,
    );

    let outcome = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:self-transfer:commit",
        )
        .expect("funded self-transfer should commit deterministically");

    assert_eq!(outcome.record().account_sequence(), 2);
    assert_eq!(outcome.record().ledger_sequence_start(), 2);
    assert_eq!(outcome.record().ledger_sequence_end(), 3);
    assert_eq!(state.balance_minor("account:alice"), 100);
    assert_eq!(state.last_account_sequence("account:alice"), 2);
    assert_eq!(state.next_ledger_sequence(), 4);
    assert_eq!(state.operation_count(), 2);
}

#[test]
fn burn_receives_one_posting_sequence_and_reduces_supply() {
    let mut state = QuickChainAtomicState::new();

    commit_issue(
        &mut state,
        '1',
        "idem:seed:bob",
        "account:bob",
        "75",
        "tx:roc:0020",
    );

    let burn = intent(
        '2',
        "idem:atomic:burn",
        QuickChainOperationClassV1::Burn,
        "account:bob",
        None,
        "25",
        2_000,
    );

    let outcome = state
        .execute_balance_operation(&burn, QuickChainSupplyDecision::BurnApproved, "tx:roc:0021")
        .expect("authorized funded burn should commit");

    assert_eq!(outcome.record().account_sequence(), 2);
    assert_eq!(outcome.record().ledger_sequence_start(), 2);
    assert_eq!(outcome.record().ledger_sequence_end(), 2);

    assert_eq!(state.balance_minor("account:bob"), 50);
    assert_eq!(state.current_supply_minor(), 50);
    assert_eq!(state.next_ledger_sequence(), 3);
    assert_eq!(state.operation_count(), 2);
}

#[test]
fn exact_retry_returns_original_before_reauthorization_or_new_txid_validation() {
    let submitted = intent(
        '3',
        "idem:atomic:retry",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        3_000,
    );

    let mut state = QuickChainAtomicState::new();

    let first = state
        .execute_balance_operation(
            &submitted,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:0030",
        )
        .expect("first issue should commit");

    let snapshot = state.clone();

    let retry = state
        .execute_balance_operation(
            &submitted,
            QuickChainSupplyDecision::NoSupplyChange,
            "invalid receipt txid with spaces",
        )
        .expect("exact retry must return original evidence");

    assert_eq!(retry.disposition(), QuickChainExecutionDisposition::Retried);
    assert!(retry.is_retry());
    assert!(!retry.is_committed());
    assert!(retry.transition().is_none());

    assert_eq!(retry.record(), first.record());
    assert_eq!(retry.record().receipt_txid(), "tx:roc:0030");
    assert_eq!(state, snapshot);
}

#[test]
fn idempotency_conflict_leaves_every_state_component_unchanged() {
    let mut state = QuickChainAtomicState::new();

    commit_issue(
        &mut state,
        '4',
        "idem:atomic:shared",
        "account:alice",
        "100",
        "tx:roc:0040",
    );

    let snapshot = state.clone();

    let conflict = intent(
        '5',
        "idem:atomic:shared",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "101",
        4_001,
    );

    let error = state
        .execute_balance_operation(
            &conflict,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:0041",
        )
        .expect_err("conflicting scoped retry key must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::IdempotencyConflict)
    );
    assert_eq!(state, snapshot);
}

#[test]
fn duplicate_operation_id_under_new_retry_key_leaves_state_unchanged() {
    let original = intent(
        '6',
        "idem:atomic:original",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "25",
        5_000,
    );

    let mut state = QuickChainAtomicState::new();

    state
        .execute_balance_operation(
            &original,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:0050",
        )
        .expect("original operation should commit");

    let snapshot = state.clone();

    let mut duplicate = original;
    duplicate.idempotency_key = "idem:atomic:duplicate".to_string();
    duplicate.produced_at_ms = 5_001;

    let error = state
        .execute_balance_operation(
            &duplicate,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:0051",
        )
        .expect_err("durable operation id may not commit twice");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::DuplicateOperationId)
    );
    assert_eq!(state, snapshot);
}

#[test]
fn arithmetic_rejection_does_not_create_identity_or_consume_sequences() {
    let mut state = QuickChainAtomicState::new();

    commit_issue(
        &mut state,
        '7',
        "idem:seed:small",
        "account:alice",
        "10",
        "tx:roc:0060",
    );

    let snapshot = state.clone();

    let transfer = intent(
        '8',
        "idem:atomic:overdraft",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "11",
        6_001,
    );

    let error = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:0061",
        )
        .expect_err("overdraft must reject atomically");

    assert!(matches!(
        error,
        QuickChainExecutionError::Transition(QuickChainTransitionError::InsufficientFunds {
            available_minor: 10,
            required_minor: 11,
            ..
        })
    ));

    assert_eq!(state, snapshot);
    assert!(state.committed_operation(&transfer.operation_id).is_none());
}

#[test]
fn invalid_receipt_reference_rolls_back_an_already_computed_transition() {
    let mut state = QuickChainAtomicState::new();

    commit_issue(
        &mut state,
        '9',
        "idem:seed:receipt",
        "account:alice",
        "100",
        "tx:roc:0070",
    );

    let snapshot = state.clone();

    let transfer = intent(
        'a',
        "idem:atomic:bad-receipt",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "10",
        7_001,
    );

    let error = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx receipt with spaces",
        )
        .expect_err("invalid receipt reference must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::InvalidReceiptTxid)
    );

    assert_eq!(state, snapshot);
    assert_eq!(state.balance_minor("account:bob"), 0);
    assert!(state.committed_operation(&transfer.operation_id).is_none());
}

#[test]
fn identical_ordered_commands_rebuild_identical_atomic_state() {
    let issue = intent(
        'b',
        "idem:replay:issue",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "100",
        8_000,
    );
    let transfer = intent(
        'c',
        "idem:replay:transfer",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "40",
        8_001,
    );
    let burn = intent(
        'd',
        "idem:replay:burn",
        QuickChainOperationClassV1::Burn,
        "account:bob",
        None,
        "10",
        8_002,
    );

    let mut first = QuickChainAtomicState::new();
    let mut replay = QuickChainAtomicState::new();

    for state in [&mut first, &mut replay] {
        state
            .execute_balance_operation(
                &issue,
                QuickChainSupplyDecision::IssueApproved,
                "tx:roc:0080",
            )
            .expect("issue should replay");

        state
            .execute_balance_operation(
                &transfer,
                QuickChainSupplyDecision::NoSupplyChange,
                "tx:roc:0081",
            )
            .expect("transfer should replay");

        state
            .execute_balance_operation(&burn, QuickChainSupplyDecision::BurnApproved, "tx:roc:0082")
            .expect("burn should replay");
    }

    assert_eq!(first, replay);
    assert_eq!(first.balance_minor("account:alice"), 60);
    assert_eq!(first.balance_minor("account:bob"), 30);
    assert_eq!(first.current_supply_minor(), 90);
    assert_eq!(first.operation_count(), 3);
    assert_eq!(first.next_ledger_sequence(), 5);
}

#[test]
fn client_assigned_account_sequence_rejects_before_any_mutation() {
    let mut submitted = intent(
        'e',
        "idem:atomic:client-sequence",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "10",
        9_000,
    );
    submitted.account_sequence = Some(1);

    let mut state = QuickChainAtomicState::new();
    let snapshot = state.clone();

    let error = state
        .execute_balance_operation(
            &submitted,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:0090",
        )
        .expect_err("client-owned account sequence must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::ClientAssignedAccountSequence)
    );
    assert_eq!(state, snapshot);
}
