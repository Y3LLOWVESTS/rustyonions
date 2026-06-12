#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests proving one QuickChain replay/atomic state cannot mix chain identities.
//! RO:WHY — ECON/RES: operation, account, hold, receipt, and future root evidence must belong to exactly one declared chain.
//! RO:INTERACTS — QuickChainReplayIndex, QuickChainAtomicState, committed-operation evidence, and ron-proto intents.
//! RO:INVARIANTS — first commit binds chain_id; mismatched chains reject before mutation; empty indexes remain unbound.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — chain_id is public domain identity, not authorization, custody, or settlement proof.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainCommittedOperationRecord, QuickChainExecutionError,
    QuickChainReplayError, QuickChainReplayIndex, QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const DEV_CHAIN_ID: &str = "ron-devnet";
const TEST_CHAIN_ID: &str = "ron-testnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn issue_intent(
    chain_id: &str,
    operation_hex_digit: char,
    idempotency_key: &str,
    amount_minor: &str,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: chain_id.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_string(),
        op_class: QuickChainOperationClassV1::Issue,
        actor_account_id: "account:alice".to_string(),
        counterparty_account_id: None,
        amount_minor: Some(amount_minor.to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms: 1_000,
    }
}

fn committed(
    intent: QuickChainOperationIntentV1,
    receipt_txid: &str,
    account_sequence: u64,
    ledger_sequence: u64,
) -> QuickChainCommittedOperationRecord {
    QuickChainCommittedOperationRecord::new(
        intent,
        receipt_txid,
        account_sequence,
        ledger_sequence,
        ledger_sequence,
    )
    .expect("test committed record should be valid")
}

#[test]
fn empty_index_binds_to_first_committed_chain() {
    let submitted = issue_intent(DEV_CHAIN_ID, '1', "idem:chain-binding:first", "100");

    let mut index = QuickChainReplayIndex::new();

    assert_eq!(index.chain_id(), None);

    index
        .record_committed(committed(submitted, "tx:roc:chain-binding:first", 1, 1))
        .expect("first commit should bind chain identity");

    assert_eq!(index.chain_id(), Some(DEV_CHAIN_ID));
    assert_eq!(index.operation_count(), 1);
    assert_eq!(index.next_ledger_sequence(), 2);
}

#[test]
fn replay_index_rejects_other_chain_without_mutation() {
    let first = issue_intent(DEV_CHAIN_ID, '2', "idem:chain-binding:dev", "100");

    let mut index = QuickChainReplayIndex::new();
    index
        .record_committed(committed(first, "tx:roc:chain-binding:dev", 1, 1))
        .expect("development-chain operation should commit");

    let snapshot = index.clone();

    let other_chain = issue_intent(TEST_CHAIN_ID, '3', "idem:chain-binding:test", "25");

    let classify_error = index
        .classify_submission(&other_chain)
        .expect_err("another chain must not share this replay index");

    assert_eq!(
        classify_error,
        QuickChainReplayError::ChainIdMismatch {
            expected: DEV_CHAIN_ID.to_string(),
            actual: TEST_CHAIN_ID.to_string(),
        }
    );
    assert_eq!(index, snapshot);

    let commit_error = index
        .record_committed(committed(other_chain, "tx:roc:chain-binding:test", 2, 2))
        .expect_err("another chain must not commit into this index");

    assert_eq!(
        commit_error,
        QuickChainReplayError::ChainIdMismatch {
            expected: DEV_CHAIN_ID.to_string(),
            actual: TEST_CHAIN_ID.to_string(),
        }
    );
    assert_eq!(index, snapshot);
}

#[test]
fn atomic_state_rejects_cross_chain_issue_before_economic_mutation() {
    let first = issue_intent(DEV_CHAIN_ID, '4', "idem:atomic-chain:dev", "100");

    let mut state = QuickChainAtomicState::new();

    state
        .execute_balance_operation(
            &first,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:atomic-chain:dev",
        )
        .expect("first-chain issue should commit");

    assert_eq!(state.replay_index().chain_id(), Some(DEV_CHAIN_ID));

    let snapshot = state.clone();

    let other_chain = issue_intent(TEST_CHAIN_ID, '5', "idem:atomic-chain:test", "50");

    let error = state
        .execute_balance_operation(
            &other_chain,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:atomic-chain:test",
        )
        .expect_err("cross-chain issue must reject before mutation");

    assert_eq!(
        error,
        QuickChainExecutionError::Replay(QuickChainReplayError::ChainIdMismatch {
            expected: DEV_CHAIN_ID.to_string(),
            actual: TEST_CHAIN_ID.to_string(),
        })
    );

    assert_eq!(state, snapshot);
    assert_eq!(state.balance_minor("account:alice"), 100);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 1);
    assert_eq!(state.next_ledger_sequence(), 2);
}
