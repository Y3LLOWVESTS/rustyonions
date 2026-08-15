#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — FINAL_BETA Phase 19 Step 7B operation and executed-transition replay proof.
//! RO:WHY — Durable operations must commit once; exact retries may return prior
//! evidence but must never execute a second economic transition.
//! RO:INTERACTS — QuickChainAtomicState, accepted-operation replay boundaries,
//! replay index, durable operation IDs, idempotency keys, and supply decisions.
//! RO:INVARIANTS — exact retry is non-mutating; changed-body idempotency reuse
//! rejects; duplicate durable operation ID rejects; accepted-history rebuild
//! preserves retry identity; duplicate accepted history cannot manufacture a
//! second committed transition.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight test feature only.
//! RO:SECURITY — local deterministic ledger proof only; no live wallet,
//! network submission, payout, bridge, staking, settlement, or finality.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAcceptedReplayBoundary, QuickChainAtomicState,
    QuickChainExecutionError, QuickChainReplayError, QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32),)
}

fn issue_intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    amount_minor: &str,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        operation_id: operation_id(operation_hex_digit),

        idempotency_key: idempotency_key.to_owned(),

        op_class: QuickChainOperationClassV1::Issue,

        actor_account_id: "account:phase19-alice".to_owned(),

        counterparty_account_id: None,

        amount_minor: Some(amount_minor.to_owned()),

        hold_id: None,

        account_sequence: None,

        produced_at_ms,
    }
}

#[test]
fn exact_duplicate_operation_retry_returns_original_evidence_without_second_transition() {
    let mut state = QuickChainAtomicState::new();

    let operation = issue_intent('1', "idem:phase19:exact-retry", "100", 19_001);

    let first = state
        .execute_balance_operation(
            &operation,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:exact-retry",
        )
        .expect("first Phase 19 operation must commit");

    assert!(
        first.is_committed(),
        "first submission must be a fresh commit",
    );

    assert!(
        first.transition().is_some(),
        "fresh commit must carry its economic transition",
    );

    let original_record = first.record().clone();

    let state_after_first = state.clone();

    let retry = state
        .execute_balance_operation(
            &operation,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:exact-retry",
        )
        .expect("exact retry must return original evidence");

    assert!(
        retry.is_retry(),
        "same durable operation and retry identity must be classified as retry",
    );

    assert!(
        retry.transition().is_none(),
        "exact retry must not manufacture a second economic transition",
    );

    assert_eq!(
        retry.record(),
        &original_record,
        "exact retry must return the original committed record",
    );

    assert_eq!(
        state, state_after_first,
        "exact retry must leave every atomic ledger component unchanged",
    );
}

#[test]
fn changed_body_under_same_idempotency_key_rejects_without_mutation() {
    let mut state = QuickChainAtomicState::new();

    let original = issue_intent('2', "idem:phase19:shared", "100", 19_010);

    state
        .execute_balance_operation(
            &original,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:shared-original",
        )
        .expect("original operation must commit");

    let state_after_commit = state.clone();

    let conflicting = issue_intent('3', "idem:phase19:shared", "101", 19_011);

    let error = state
        .execute_balance_operation(
            &conflicting,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:shared-conflict",
        )
        .expect_err("same idempotency key with changed operation body must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::IdempotencyConflict,),
    );

    assert_eq!(
        state, state_after_commit,
        "idempotency conflict must not mutate ledger or replay state",
    );
}

#[test]
fn duplicate_durable_operation_id_under_new_retry_key_rejects_without_mutation() {
    let mut state = QuickChainAtomicState::new();

    let original = issue_intent('4', "idem:phase19:operation-original", "25", 19_020);

    state
        .execute_balance_operation(
            &original,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:operation-original",
        )
        .expect("original durable operation must commit");

    let state_after_commit = state.clone();

    let mut duplicate = original.clone();

    duplicate.idempotency_key = "idem:phase19:operation-duplicate".to_owned();

    duplicate.produced_at_ms = 19_021;

    let error = state
        .execute_balance_operation(
            &duplicate,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:operation-duplicate",
        )
        .expect_err("one durable operation ID must never commit twice");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::DuplicateOperationId,),
    );

    assert_eq!(
        state, state_after_commit,
        "duplicate durable operation ID rejection must leave state unchanged",
    );
}

#[test]
fn accepted_history_rebuild_preserves_exact_retry_identity_without_reexecuting_transition() {
    let mut live = QuickChainAtomicState::new();

    let operation = issue_intent('5', "idem:phase19:rebuild-retry", "75", 19_030);

    let committed = live
        .execute_balance_operation(
            &operation,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:rebuild-retry",
        )
        .expect("live operation must commit before accepted-history rebuild");

    assert!(committed.is_committed(),);

    let accepted = vec![QuickChainAcceptedOperation::balance(
        committed.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
    )];

    let boundary = live.accepted_replay_boundary();

    let mut rebuilt =
        QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(&accepted, boundary)
            .expect("accepted history must reproduce live state");

    assert_eq!(
        rebuilt, live,
        "accepted replay must reconstruct exact committed ledger state",
    );

    let rebuilt_before_retry = rebuilt.clone();

    let retry = rebuilt
        .execute_balance_operation(
            &operation,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:rebuild-retry",
        )
        .expect("same operation after rebuild must remain an exact retry");

    assert!(
        retry.is_retry(),
        "accepted-history reconstruction must retain durable retry identity",
    );

    assert!(
        retry.transition().is_none(),
        "post-rebuild retry must not execute the accepted transition again",
    );

    assert_eq!(
        retry.record(),
        committed.record(),
        "post-rebuild retry must return the original accepted record",
    );

    assert_eq!(
        rebuilt, rebuilt_before_retry,
        "post-rebuild retry must not alter reconstructed state",
    );
}

#[test]
fn duplicated_accepted_transition_cannot_masquerade_as_two_committed_operations() {
    let mut live = QuickChainAtomicState::new();

    let operation = issue_intent('6', "idem:phase19:accepted-duplicate", "50", 19_040);

    let committed = live
        .execute_balance_operation(
            &operation,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase19:accepted-duplicate",
        )
        .expect("fixture operation must commit");

    let accepted = QuickChainAcceptedOperation::balance(
        committed.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
    );

    let duplicated_history = vec![accepted.clone(), accepted];

    // Deliberately claim the duplicated history represents two committed
    // operations. Replay must not let one accepted transition satisfy that
    // boundary twice.
    let false_two_operation_boundary =
        QuickChainAcceptedReplayBoundary::with_chain_id(2, 3, CHAIN_ID);

    let error = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &duplicated_history,
        false_two_operation_boundary,
    )
    .expect_err("duplicate accepted transition must not reconstruct as two commits");

    assert!(
        matches!(error, QuickChainExecutionError::Replay(_)),
        "duplicate accepted transition must fail through replay validation",
    );

    assert_eq!(
        live.operation_count(),
        1,
        "source ledger must still contain exactly one committed operation",
    );

    assert_eq!(
        live.next_ledger_sequence(),
        2,
        "duplicate replay evidence must not consume another ledger sequence",
    );
}
