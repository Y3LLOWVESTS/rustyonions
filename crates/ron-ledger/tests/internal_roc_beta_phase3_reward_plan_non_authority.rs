#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Internal ROC Beta Phase 3 ledger boundary for reward-plan non-authority.
//! RO:WHY — ECON/GOV: accounting snapshots and reward plans must not become ledger receipt or balance truth.
//! RO:INTERACTS — ron_ledger::quickchain atomic state and ron_proto Phase 3 planning DTOs.
//! RO:INVARIANTS — svc-wallet remains mutation front-door; reward plans/events are not receipts; raw engagement cannot mint.
//! RO:METRICS — none.
//! RO:CONFIG — requires ron-ledger quickchain-preflight feature.
//! RO:SECURITY — no bridge, staking, liquidity, external settlement, or direct rewarder/accounting mutation.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase3_reward_plan_non_authority.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionError, QuickChainReplayError,
    QuickChainSupplyDecision,
};
use ron_proto::{
    ContentId, QuickChainEventClassV1, QuickChainOperationClassV1, QuickChainOperationIntentV1,
    QuickChainRewardPlanReferenceV1, QuickChainUsageEventV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA, QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA,
    QUICKCHAIN_USAGE_EVENT_SCHEMA,
};
use std::collections::BTreeMap;

const CHAIN_ID: &str = "ron-devnet";
const VIEWER: &str = "account:viewer-phase3";
const CREATOR: &str = "account:creator-phase3";

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().expect("test cid")
}

fn operation_id(index: u8) -> String {
    format!("op_{index:032x}")
}

#[allow(clippy::too_many_arguments)]
fn intent(
    operation_index: u8,
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
        operation_id: operation_id(operation_index),
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

fn usage_event(event_class: QuickChainEventClassV1, index: u8) -> QuickChainUsageEventV1 {
    let mut labels = BTreeMap::new();
    labels.insert("phase".to_string(), "internal-roc-beta-phase3".to_string());

    QuickChainUsageEventV1 {
        schema: QUICKCHAIN_USAGE_EVENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        event_id: format!("event:phase3:{index:04}"),
        action: "content_view".to_string(),
        event_class,
        account_id: Some(VIEWER.to_string()),
        counterparty_account_id: Some(CREATOR.to_string()),
        object_cid: Some(cid('c')),
        site_name: Some("site:phase3-demo".to_string()),
        amount_minor: Some("1".to_string()),
        units: 1,
        labels,
        produced_at_ms: 1_910_000_000_000 + u64::from(index),
        idempotency_key: format!("idem:phase3:event:{index:04}"),
    }
}

fn reward_plan_ref() -> QuickChainRewardPlanReferenceV1 {
    QuickChainRewardPlanReferenceV1 {
        schema: QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        plan_id: "reward_plan:phase3:window-0001".to_string(),
        plan_root: cid('d'),
        snapshot_id: "snapshot:phase3:window-0001".to_string(),
        snapshot_root: cid('e'),
        source_event_class: QuickChainEventClassV1::EconomicReceipt,
        planned_total_minor: "25".to_string(),
        payout_candidate_count: 1,
        capped_by_policy: true,
        verification_ref: None,
        funding_budget_ref: None,
        produced_at_ms: 1_910_000_100_000,
    }
}

fn seed_viewer_balance(state: &mut QuickChainAtomicState) {
    let issue = intent(
        1,
        "idem:phase3:seed-viewer",
        QuickChainOperationClassV1::Issue,
        VIEWER,
        None,
        "100",
        1_910_000_000_100,
    );

    state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase3:seed_viewer",
        )
        .expect("seed issue should commit through wallet-like tx evidence");

    assert_eq!(state.balance_minor(VIEWER), 100);
    assert_eq!(state.current_supply_minor(), 100);
}

fn expect_non_receipt_evidence(error: QuickChainExecutionError) {
    assert!(
        matches!(
            error,
            QuickChainExecutionError::Replay(QuickChainReplayError::InvalidReceiptEvidenceKind)
        ),
        "unexpected error: {error}"
    );
}

#[test]
fn reward_plan_reference_cannot_be_committed_as_ledger_receipt() {
    let mut state = QuickChainAtomicState::new();
    seed_viewer_balance(&mut state);

    let plan = reward_plan_ref();
    plan.validate()
        .expect("reward plan reference should validate as planning material");

    let before = state.clone();

    let payout_like_issue = intent(
        2,
        "idem:phase3:reward-plan-direct-issue",
        QuickChainOperationClassV1::Issue,
        CREATOR,
        None,
        "25",
        1_910_000_000_200,
    );

    let error = state
        .execute_balance_operation(
            &payout_like_issue,
            QuickChainSupplyDecision::IssueApproved,
            plan.plan_id,
        )
        .expect_err("reward plan reference must not be accepted as receipt evidence");

    expect_non_receipt_evidence(error);
    assert_eq!(
        state, before,
        "rejected reward plan receipt poison must not mutate ledger state"
    );
    assert_eq!(state.balance_minor(CREATOR), 0);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 1);
}

#[test]
fn raw_engagement_events_cannot_mutate_balance_or_supply() {
    let mut state = QuickChainAtomicState::new();
    seed_viewer_balance(&mut state);

    let cases = [
        QuickChainEventClassV1::Metering,
        QuickChainEventClassV1::AnalyticsOnly,
        QuickChainEventClassV1::ProofEligible,
        QuickChainEventClassV1::AdBudgeted,
    ];

    for (offset, event_class) in cases.into_iter().enumerate() {
        let event = usage_event(event_class, offset as u8 + 1);
        event
            .validate()
            .expect("usage event should validate as accounting input");

        assert!(
            !event.event_class.may_represent_balance_truth(),
            "only backend wallet/ledger economic receipts may represent balance truth"
        );

        let before = state.clone();

        let attempted_transfer = intent(
            10 + offset as u8,
            &format!("idem:phase3:raw-event-direct-transfer:{offset}"),
            QuickChainOperationClassV1::Transfer,
            VIEWER,
            Some(CREATOR),
            "1",
            1_910_000_001_000 + offset as u64,
        );

        let error = state
            .execute_balance_operation(
                &attempted_transfer,
                QuickChainSupplyDecision::NoSupplyChange,
                event.event_id,
            )
            .expect_err("raw usage event id must not be accepted as receipt evidence");

        expect_non_receipt_evidence(error);
        assert_eq!(
            state, before,
            "raw engagement/event reference must not mutate ledger state"
        );
    }

    assert_eq!(state.balance_minor(VIEWER), 100);
    assert_eq!(state.balance_minor(CREATOR), 0);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 1);
}

#[test]
fn accepted_wallet_operation_remains_receipt_source() {
    let mut state = QuickChainAtomicState::new();
    seed_viewer_balance(&mut state);

    let transfer = intent(
        40,
        "idem:phase3:wallet-transfer",
        QuickChainOperationClassV1::Transfer,
        VIEWER,
        Some(CREATOR),
        "7",
        1_910_000_002_000,
    );

    let outcome = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:phase3:wallet_transfer",
        )
        .expect("wallet-like tx evidence should remain accepted");

    assert!(outcome.is_committed());
    assert_eq!(
        outcome.record().receipt_txid(),
        "tx:roc:phase3:wallet_transfer"
    );
    assert_eq!(state.balance_minor(VIEWER), 93);
    assert_eq!(state.balance_minor(CREATOR), 7);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 2);
}
