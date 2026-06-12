#![cfg(feature = "quickchain-preflight")]

//! RO:WHAT — Integration tests for pure checked QuickChain issue, transfer, and burn transitions.
//! RO:WHY — ECON/RES: freeze DTO validation boundaries, arithmetic, authorization, conservation, ordering, and failure atomicity.
//! RO:INTERACTS — ron_ledger::quickchain, ron_proto::quickchain operation DTOs.
//! RO:INVARIANTS — ron-proto rejects malformed shape; no saturation; positive u128 execution; transfer conserved; failures leave state unchanged.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — supply decisions are test inputs only; no real capabilities, receipts, or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainBalanceState, QuickChainSupplyDecision, QuickChainTransitionError,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const HOLD_ID: &str = "hold_11111111111111111111111111111111";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn intent(
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hex_digit: char,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(hex_digit),
        idempotency_key: format!("idem:test:{hex_digit}"),
        op_class,
        actor_account_id: actor.to_string(),
        counterparty_account_id: counterparty.map(str::to_string),
        amount_minor: Some(amount_minor.to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms: 1_000 + u64::from(hex_digit.to_digit(16).unwrap_or(1)),
    }
}

fn issue(
    state: &mut QuickChainBalanceState,
    account_id: &str,
    amount_minor: &str,
    hex_digit: char,
) {
    state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                account_id,
                None,
                amount_minor,
                hex_digit,
            ),
            QuickChainSupplyDecision::IssueApproved,
        )
        .expect("test issue should succeed");
}

#[test]
fn authorized_issue_increases_balance_and_supply() {
    let mut state = QuickChainBalanceState::new();

    let transition = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "100",
                '1',
            ),
            QuickChainSupplyDecision::IssueApproved,
        )
        .expect("authorized issue should succeed");

    assert_eq!(state.balance_minor("account:alice"), 100);
    assert_eq!(state.total_issued_minor(), 100);
    assert_eq!(state.total_burned_minor(), 0);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(transition.actor_balance_before, 0);
    assert_eq!(transition.actor_balance_after, 100);
    assert_eq!(transition.supply_before, 0);
    assert_eq!(transition.supply_after, 100);
}

#[test]
fn unauthorized_issue_rejects_without_state_change() {
    let mut state = QuickChainBalanceState::new();
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "100",
                '2',
            ),
            QuickChainSupplyDecision::NoSupplyChange,
        )
        .expect_err("issue without approval must reject");

    assert_eq!(error, QuickChainTransitionError::UnauthorizedIssue);
    assert_eq!(state, before);
}

#[test]
fn transfer_conserves_participant_balances_and_supply() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "100", '3');

    let supply_before = state.current_supply_minor();
    let participant_total_before =
        state.balance_minor("account:alice") + state.balance_minor("account:bob");

    let transition = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:bob"),
                "40",
                '4',
            ),
            QuickChainSupplyDecision::NoSupplyChange,
        )
        .expect("funded transfer should succeed");

    let participant_total_after =
        state.balance_minor("account:alice") + state.balance_minor("account:bob");

    assert_eq!(state.balance_minor("account:alice"), 60);
    assert_eq!(state.balance_minor("account:bob"), 40);
    assert_eq!(participant_total_before, participant_total_after);
    assert_eq!(state.current_supply_minor(), supply_before);
    assert_eq!(state.total_issued_minor(), 100);
    assert_eq!(state.total_burned_minor(), 0);
    assert_eq!(transition.supply_before, transition.supply_after);
}

#[test]
fn transfer_underflow_rejects_without_crediting_destination() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "10", '5');
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:bob"),
                "11",
                '6',
            ),
            QuickChainSupplyDecision::NoSupplyChange,
        )
        .expect_err("overdraft transfer must reject");

    assert!(matches!(
        error,
        QuickChainTransitionError::InsufficientFunds {
            available_minor: 10,
            required_minor: 11,
            ..
        }
    ));
    assert_eq!(state, before);
    assert_eq!(state.balance_minor("account:bob"), 0);
}

#[test]
fn authorized_burn_reduces_balance_and_supply() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "100", '7');

    let transition = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Burn,
                "account:alice",
                None,
                "25",
                '8',
            ),
            QuickChainSupplyDecision::BurnApproved,
        )
        .expect("authorized funded burn should succeed");

    assert_eq!(state.balance_minor("account:alice"), 75);
    assert_eq!(state.total_issued_minor(), 100);
    assert_eq!(state.total_burned_minor(), 25);
    assert_eq!(state.current_supply_minor(), 75);
    assert_eq!(transition.actor_balance_before, 100);
    assert_eq!(transition.actor_balance_after, 75);
    assert_eq!(transition.supply_before, 100);
    assert_eq!(transition.supply_after, 75);
}

#[test]
fn unauthorized_burn_rejects_without_state_change() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "100", '9');
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Burn,
                "account:alice",
                None,
                "25",
                'a',
            ),
            QuickChainSupplyDecision::NoSupplyChange,
        )
        .expect_err("burn without approval must reject");

    assert_eq!(error, QuickChainTransitionError::UnauthorizedBurn);
    assert_eq!(state, before);
}

#[test]
fn burn_underflow_rejects_without_state_change() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "10", 'b');
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Burn,
                "account:alice",
                None,
                "11",
                'c',
            ),
            QuickChainSupplyDecision::BurnApproved,
        )
        .expect_err("over-balance burn must reject");

    assert!(matches!(
        error,
        QuickChainTransitionError::InsufficientFunds {
            available_minor: 10,
            required_minor: 11,
            ..
        }
    ));
    assert_eq!(state, before);
}

#[test]
fn account_credit_overflow_rejects_without_state_change() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", &u128::MAX.to_string(), 'd');
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "1",
                'e',
            ),
            QuickChainSupplyDecision::IssueApproved,
        )
        .expect_err("account credit overflow must reject");

    assert!(matches!(
        error,
        QuickChainTransitionError::BalanceOverflow { .. }
    ));
    assert_eq!(state, before);
}

#[test]
fn total_supply_overflow_rejects_without_state_change() {
    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", &u128::MAX.to_string(), '1');
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                "account:bob",
                None,
                "1",
                '2',
            ),
            QuickChainSupplyDecision::IssueApproved,
        )
        .expect_err("supply overflow must reject");

    assert_eq!(error, QuickChainTransitionError::SupplyOverflow);
    assert_eq!(state, before);
    assert_eq!(state.balance_minor("account:bob"), 0);
}

#[test]
fn proto_rejects_amount_above_u128_before_transition() {
    let mut state = QuickChainBalanceState::new();
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "340282366920938463463374607431768211456",
                '3',
            ),
            QuickChainSupplyDecision::IssueApproved,
        )
        .expect_err("u128 max plus one must fail DTO validation");

    assert!(matches!(
        error,
        QuickChainTransitionError::InvalidIntent(ref message)
            if message.contains("amount_minor")
                && message.contains("must fit in u128 minor units")
    ));
    assert_eq!(state, before);
}

#[test]
fn proto_rejects_missing_amount_before_transition() {
    let mut submitted = intent(
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "1",
        '4',
    );
    submitted.amount_minor = None;

    let mut state = QuickChainBalanceState::new();
    let before = state.clone();

    let error = state
        .apply_balance_operation(&submitted, QuickChainSupplyDecision::IssueApproved)
        .expect_err("issue without amount must fail DTO validation");

    assert!(matches!(
        error,
        QuickChainTransitionError::InvalidIntent(ref message)
            if message.contains("amount_minor") && message.contains("required")
    ));
    assert_eq!(state, before);
}

#[test]
fn proto_rejects_missing_transfer_counterparty_before_transition() {
    let submitted = intent(
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        None,
        "1",
        '5',
    );

    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "10", '6');
    let before = state.clone();

    let error = state
        .apply_balance_operation(&submitted, QuickChainSupplyDecision::NoSupplyChange)
        .expect_err("transfer without counterparty must fail DTO validation");

    assert!(matches!(
        error,
        QuickChainTransitionError::InvalidIntent(ref message)
            if message.contains("counterparty_account_id") && message.contains("required")
    ));
    assert_eq!(state, before);
}

#[test]
fn zero_amount_rejects_as_ledger_execution_rule() {
    let mut state = QuickChainBalanceState::new();
    let before = state.clone();

    let error = state
        .apply_balance_operation(
            &intent(
                QuickChainOperationClassV1::Issue,
                "account:alice",
                None,
                "0",
                '7',
            ),
            QuickChainSupplyDecision::IssueApproved,
        )
        .expect_err("zero-value mutation must reject");

    assert_eq!(error, QuickChainTransitionError::ZeroAmount);
    assert_eq!(state, before);
}

#[test]
fn hold_operations_remain_out_of_scope_and_do_not_mutate() {
    let mut hold_open = intent(
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:bob"),
        "10",
        '8',
    );
    hold_open.hold_id = Some(HOLD_ID.to_string());

    let mut state = QuickChainBalanceState::new();
    issue(&mut state, "account:alice", "100", '9');
    let before = state.clone();

    let error = state
        .apply_balance_operation(&hold_open, QuickChainSupplyDecision::NoSupplyChange)
        .expect_err("holds are not part of this arithmetic slice");

    assert_eq!(error, QuickChainTransitionError::UnsupportedOperationClass);
    assert_eq!(state, before);
}

#[test]
fn different_application_orders_produce_identical_ordered_state() {
    let mut first = QuickChainBalanceState::new();
    issue(&mut first, "account:charlie", "30", 'a');
    issue(&mut first, "account:alice", "10", 'b');
    issue(&mut first, "account:bob", "20", 'c');

    let mut second = QuickChainBalanceState::new();
    issue(&mut second, "account:bob", "20", 'd');
    issue(&mut second, "account:charlie", "30", 'e');
    issue(&mut second, "account:alice", "10", 'f');

    assert_eq!(first, second);

    let ordered: Vec<_> = first.ordered_balances().collect();
    assert_eq!(
        ordered,
        vec![
            ("account:alice", 10),
            ("account:bob", 20),
            ("account:charlie", 30),
        ]
    );
}
