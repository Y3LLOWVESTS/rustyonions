//! RO:WHAT — Internal ROC Beta Phase 3 Round 2 approved payout replay/receipt boundary tests.
//! RO:WHY — Proves approved payout execution appears in ledger truth only as accepted wallet/ledger issue operations with durable receipt references, replay equality, and duplicate payout prevention.
//! RO:INTERACTS — QuickChainAtomicState, QuickChainAcceptedOperation, QuickChainOperationIntentV1.
//! RO:INVARIANTS — svc-wallet remains mutation front-door; ron-ledger remains durable truth; reward plans are not receipts; duplicate operation commits do not double issue.
//! RO:METRICS — none.
//! RO:CONFIG — requires ron-ledger quickchain-preflight feature.
//! RO:SECURITY — no rewarder/accounting/policy direct ledger mutation; no bridge/staking/liquidity/external settlement.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_approved_payout_replay.

#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const CREATOR_A: &str = "account:creator-payout-a";
const CREATOR_B: &str = "account:creator-payout-b";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

#[allow(clippy::too_many_arguments)]
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
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_owned(),
        op_class,
        actor_account_id: actor.to_owned(),
        counterparty_account_id: counterparty.map(str::to_owned),
        amount_minor: Some(amount_minor.to_owned()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms,
    }
}

fn approved_payout_intent(
    operation_hex_digit: char,
    account: &str,
    amount_minor: &str,
) -> QuickChainOperationIntentV1 {
    intent(
        operation_hex_digit,
        &format!("idem:internal-roc-beta-phase3:payout:{operation_hex_digit}:{account}"),
        QuickChainOperationClassV1::Issue,
        account,
        None,
        amount_minor,
        1_900_200_000_000 + u64::from(operation_hex_digit.to_digit(16).expect("hex digit")),
    )
}

fn commit_approved_payout(
    state: &mut QuickChainAtomicState,
    submitted: &QuickChainOperationIntentV1,
    receipt_txid: &str,
) -> QuickChainAcceptedOperation {
    let outcome = state
        .execute_balance_operation(
            submitted,
            QuickChainSupplyDecision::IssueApproved,
            receipt_txid,
        )
        .expect("approved payout issue should commit");

    assert!(outcome.is_committed());

    QuickChainAcceptedOperation::balance(
        outcome.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
    )
}

fn assert_replay_matches(
    live: &QuickChainAtomicState,
    accepted: &[QuickChainAcceptedOperation],
) -> QuickChainAtomicState {
    let replayed = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        accepted,
        live.accepted_replay_boundary(),
    )
    .expect("accepted approved payout history should replay to live boundary");

    assert_eq!(&replayed, live);
    assert_eq!(
        replayed.state_snapshot().expect("snapshot should capture"),
        live.state_snapshot().expect("snapshot should capture")
    );

    replayed
}

#[test]
fn approved_payout_issues_create_durable_receipts_and_replay_equally() {
    let mut live = QuickChainAtomicState::new();
    let mut accepted = Vec::new();

    let payout_a = approved_payout_intent('1', CREATOR_A, "37");
    let payout_b = approved_payout_intent('2', CREATOR_B, "19");

    accepted.push(commit_approved_payout(
        &mut live,
        &payout_a,
        "tx:roc:phase3:approved-payout:a",
    ));
    accepted.push(commit_approved_payout(
        &mut live,
        &payout_b,
        "tx:roc:phase3:approved-payout:b",
    ));

    assert_eq!(live.balance_minor(CREATOR_A), 37);
    assert_eq!(live.balance_minor(CREATOR_B), 19);
    assert_eq!(live.current_supply_minor(), 56);
    assert_eq!(live.balance_state().total_issued_minor(), 56);

    let replayed = assert_replay_matches(&live, &accepted);

    assert_eq!(replayed.balance_minor(CREATOR_A), 37);
    assert_eq!(replayed.balance_minor(CREATOR_B), 19);
    assert_eq!(replayed.current_supply_minor(), 56);

    for accepted_operation in &accepted {
        let operation_id = &accepted_operation.record().intent().operation_id;
        let replayed_record = replayed
            .committed_operation(operation_id)
            .expect("approved payout operation should be indexed after replay");

        assert_eq!(replayed_record, accepted_operation.record());
        assert_eq!(
            replayed_record.receipt_txid(),
            accepted_operation.trusted_receipt_txid()
        );
        assert!(
            replayed_record
                .receipt_txid()
                .starts_with("tx:roc:phase3:approved-payout:"),
            "approved payout receipt must be backend wallet/ledger tx evidence"
        );
    }
}

#[test]
fn duplicate_approved_payout_operation_id_does_not_double_issue() {
    let mut state = QuickChainAtomicState::new();

    let original = approved_payout_intent('3', CREATOR_A, "41");
    let original_outcome = state
        .execute_balance_operation(
            &original,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase3:approved-payout:duplicate-original",
        )
        .expect("original approved payout should commit");

    assert!(original_outcome.is_committed());
    assert_eq!(state.balance_minor(CREATOR_A), 41);
    assert_eq!(state.current_supply_minor(), 41);

    let mut duplicate = approved_payout_intent('4', CREATOR_A, "41");
    duplicate.operation_id = original.operation_id.clone();

    let duplicate_result = state.execute_balance_operation(
        &duplicate,
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:phase3:approved-payout:duplicate-second",
    );

    assert!(
        duplicate_result.is_err(),
        "duplicate approved payout operation_id must not commit twice"
    );
    assert_eq!(
        state.balance_minor(CREATOR_A),
        41,
        "duplicate payout attempt must not credit creator again"
    );
    assert_eq!(state.current_supply_minor(), 41);
}

#[test]
fn reward_plan_evidence_prefix_cannot_be_used_as_payout_receipt_txid() {
    let mut state = QuickChainAtomicState::new();
    let payout = approved_payout_intent('5', CREATOR_A, "23");

    let result = state.execute_balance_operation(
        &payout,
        QuickChainSupplyDecision::IssueApproved,
        "reward_plan:b3aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    assert!(
        result.is_err(),
        "reward-plan evidence must not masquerade as payout receipt txid"
    );
    assert_eq!(state.balance_minor(CREATOR_A), 0);
    assert_eq!(state.current_supply_minor(), 0);
}

#[test]
fn replay_rejects_tampered_approved_payout_receipt_reference() {
    let mut live = QuickChainAtomicState::new();

    let payout = approved_payout_intent('6', CREATOR_B, "29");
    let accepted = commit_approved_payout(
        &mut live,
        &payout,
        "tx:roc:phase3:approved-payout:tamper-original",
    );

    let tampered = QuickChainAcceptedOperation::balance_with_replay_receipt_txid(
        accepted.record().clone(),
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:phase3:approved-payout:tampered",
    );

    let replay = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &[tampered],
        live.accepted_replay_boundary(),
    );

    assert!(
        replay.is_err(),
        "accepted replay must reject receipt references that disagree with committed evidence"
    );
    assert_eq!(live.balance_minor(CREATOR_B), 29);
}
