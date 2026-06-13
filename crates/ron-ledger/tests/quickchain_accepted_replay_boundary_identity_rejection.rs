// crates/ron-ledger/tests/quickchain_accepted_replay_boundary_identity_rejection.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Tests that identity-layer rejections do not advance accepted-history replay boundaries.
//! RO:WHY — ECON/RES: duplicate durable operation IDs and scoped idempotency conflicts must reject before economic or sequence mutation.
//! RO:INTERACTS — QuickChainAtomicState, QuickChainAcceptedReplayBoundary, replay identity index, and ron-proto operation intents.
//! RO:INVARIANTS — identity rejections leave balances, operation count, account sequence, primitive ledger sequence, and chain binding unchanged.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture txids are inert public test values; no wallet authority, receipt fabrication, roots, checkpoints, signatures, finality, or settlement.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionError, QuickChainReplayError,
    QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const SEED_IDEMPOTENCY_KEY: &str = "idem:boundary-identity-reject:seed-issue";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn issue_intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    amount_minor: &str,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_string(),
        op_class: QuickChainOperationClassV1::Issue,
        actor_account_id: "account:alice".to_string(),
        counterparty_account_id: None,
        amount_minor: Some(amount_minor.to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms,
    }
}

fn seed_alice_issue(state: &mut QuickChainAtomicState) {
    let seed = issue_intent('1', SEED_IDEMPOTENCY_KEY, "100", 1_000);

    state
        .execute_balance_operation(
            &seed,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:boundary-identity-reject:seed-issue",
        )
        .expect("seed issue should commit");
}

#[test]
fn duplicate_operation_id_rejects_without_advancing_accepted_replay_boundary() {
    let mut state = QuickChainAtomicState::new();
    seed_alice_issue(&mut state);

    let boundary_before_reject = state.accepted_replay_boundary();
    let balance_before_reject = state.balance_minor("account:alice");
    let account_sequence_before_reject = state.last_account_sequence("account:alice");

    let duplicate_operation_id = issue_intent(
        '1',
        "idem:boundary-identity-reject:duplicate-operation-id",
        "7",
        1_001,
    );

    let error = state
        .execute_balance_operation(
            &duplicate_operation_id,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:boundary-identity-reject:duplicate-operation-id",
        )
        .expect_err("duplicate operation_id under a new retry key must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::DuplicateOperationId)
    );

    assert_eq!(state.accepted_replay_boundary(), boundary_before_reject);
    assert_eq!(state.balance_minor("account:alice"), balance_before_reject);
    assert_eq!(
        state.last_account_sequence("account:alice"),
        account_sequence_before_reject
    );
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.next_ledger_sequence(), 2);
    assert_eq!(state.replay_index().chain_id(), Some(CHAIN_ID));
}

#[test]
fn idempotency_conflict_rejects_without_advancing_accepted_replay_boundary() {
    let mut state = QuickChainAtomicState::new();
    seed_alice_issue(&mut state);

    let boundary_before_reject = state.accepted_replay_boundary();
    let balance_before_reject = state.balance_minor("account:alice");
    let account_sequence_before_reject = state.last_account_sequence("account:alice");

    let conflicting_retry_scope = issue_intent('2', SEED_IDEMPOTENCY_KEY, "50", 1_001);

    let error = state
        .execute_balance_operation(
            &conflicting_retry_scope,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:boundary-identity-reject:idempotency-conflict",
        )
        .expect_err("same scoped idempotency key with a different intent must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::IdempotencyConflict)
    );

    assert_eq!(state.accepted_replay_boundary(), boundary_before_reject);
    assert_eq!(state.balance_minor("account:alice"), balance_before_reject);
    assert_eq!(
        state.last_account_sequence("account:alice"),
        account_sequence_before_reject
    );
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.next_ledger_sequence(), 2);
    assert_eq!(state.replay_index().chain_id(), Some(CHAIN_ID));
}
