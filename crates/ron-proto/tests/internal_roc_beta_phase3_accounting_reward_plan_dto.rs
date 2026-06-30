//! RO:WHAT — Internal ROC Beta Phase 3 DTO proof for accounting snapshot and reward-plan references.
//! RO:WHY — ECON/GOV: reward planning must be deterministic without becoming receipt, balance, or payout truth.
//! RO:INTERACTS — ron_proto::quickchain event classes, accounting snapshot refs, and reward-plan refs.
//! RO:INVARIANTS — references only; integer money strings; no raw engagement minting; bridge/staking fields reject.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no wallet authority, payout execution, bridge, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p ron-proto --test internal_roc_beta_phase3_accounting_reward_plan_dto.

use ron_proto::{
    ContentId, QuickChainAccountingSnapshotReferenceV1, QuickChainEventClassV1,
    QuickChainRewardPlanReferenceV1, QUICKCHAIN_ACCOUNTING_SNAPSHOT_REFERENCE_SCHEMA,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().expect("test cid")
}

fn snapshot_ref() -> QuickChainAccountingSnapshotReferenceV1 {
    QuickChainAccountingSnapshotReferenceV1 {
        schema: QUICKCHAIN_ACCOUNTING_SNAPSHOT_REFERENCE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        snapshot_id: "snapshot:phase3:window-0001".to_string(),
        snapshot_root: cid('a'),
        window_started_at_ms: 1_900_000_000_000,
        window_ended_at_ms: 1_900_003_600_000,
        sealed_at_ms: 1_900_003_700_000,
        source_event_count: 5,
        economic_receipt_count: 1,
        metering_count: 1,
        proof_eligible_count: 1,
        ad_budgeted_count: 1,
        analytics_only_count: 1,
    }
}

fn reward_plan_ref(source_event_class: QuickChainEventClassV1) -> QuickChainRewardPlanReferenceV1 {
    QuickChainRewardPlanReferenceV1 {
        schema: QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        plan_id: "reward_plan:phase3:window-0001".to_string(),
        plan_root: cid('b'),
        snapshot_id: "snapshot:phase3:window-0001".to_string(),
        snapshot_root: cid('a'),
        source_event_class,
        planned_total_minor: "25".to_string(),
        payout_candidate_count: 1,
        capped_by_policy: true,
        verification_ref: None,
        funding_budget_ref: None,
        produced_at_ms: 1_900_003_800_000,
    }
}

#[test]
fn accounting_snapshot_reference_keeps_event_class_counts_deterministic() {
    let snapshot = snapshot_ref();
    snapshot
        .validate()
        .expect("valid accounting snapshot reference should pass");

    let mut mismatched = snapshot;
    mismatched.analytics_only_count = 2;

    let error = mismatched
        .validate()
        .expect_err("mismatched event-class count total must reject");

    assert!(
        error.to_string().contains("source_event_count"),
        "unexpected error: {error}"
    );
}

#[test]
fn reward_plan_reference_is_not_receipt_or_balance_truth() {
    let plan = reward_plan_ref(QuickChainEventClassV1::EconomicReceipt);

    plan.validate()
        .expect("economic receipt derived plan reference should validate");

    let encoded = serde_json::to_value(&plan).expect("plan ref should encode");
    let object = encoded.as_object().expect("plan ref encodes as object");

    for forbidden_truth_field in [
        "receipt_txid",
        "receipt_root",
        "balance_minor",
        "available_minor",
        "held_minor",
        "account_sequence",
        "ledger_sequence",
        "finality",
        "entitlement",
        "payout_executed",
    ] {
        assert!(
            !object.contains_key(forbidden_truth_field),
            "reward-plan reference must not expose {forbidden_truth_field}"
        );
    }
}

#[test]
fn event_class_dto_enforces_phase3_reward_material_boundaries() {
    for event_class in [
        QuickChainEventClassV1::Metering,
        QuickChainEventClassV1::AnalyticsOnly,
    ] {
        let plan = reward_plan_ref(event_class);
        plan.validate()
            .expect_err("metering and analytics_only cannot directly become payout material");
    }

    let proof_plan = reward_plan_ref(QuickChainEventClassV1::ProofEligible);
    proof_plan
        .validate()
        .expect_err("proof_eligible payout planning requires verification evidence");

    let mut verified_proof_plan = reward_plan_ref(QuickChainEventClassV1::ProofEligible);
    verified_proof_plan.verification_ref = Some("proof:phase3:verified-0001".to_string());
    verified_proof_plan
        .validate()
        .expect("verified proof_eligible plan should validate as planning material");

    let ad_plan = reward_plan_ref(QuickChainEventClassV1::AdBudgeted);
    ad_plan
        .validate()
        .expect_err("ad_budgeted payout planning requires explicit budget evidence");

    let mut funded_ad_plan = reward_plan_ref(QuickChainEventClassV1::AdBudgeted);
    funded_ad_plan.funding_budget_ref = Some("ad_budget:phase3:funded-0001".to_string());
    funded_ad_plan
        .validate()
        .expect("funded ad_budgeted plan should validate as planning material");
}

#[test]
fn zero_value_metering_plan_can_reference_snapshot_without_payout() {
    let mut plan = reward_plan_ref(QuickChainEventClassV1::Metering);
    plan.planned_total_minor = "0".to_string();
    plan.payout_candidate_count = 0;
    plan.capped_by_policy = false;

    plan.validate()
        .expect("zero-value metering plan reference is allowed as non-payout planning data");
}

#[test]
fn money_fields_remain_integer_minor_unit_strings() {
    let mut plan = reward_plan_ref(QuickChainEventClassV1::EconomicReceipt);
    plan.planned_total_minor = "1.0".to_string();

    plan.validate()
        .expect_err("float-like planned_total_minor must reject");

    let mut value = serde_json::to_value(reward_plan_ref(QuickChainEventClassV1::EconomicReceipt))
        .expect("plan should encode");
    value["planned_total_minor"] = json!(25);

    serde_json::from_value::<QuickChainRewardPlanReferenceV1>(value)
        .expect_err("numeric JSON money must reject because money is string-only");
}

#[test]
fn future_bridge_and_staking_fields_remain_absent_or_inert() {
    for forbidden_field in [
        "bridge_txid",
        "bridge_mint",
        "solana_signature",
        "rox_settlement_id",
        "staking_yield_bps",
        "staking_position_id",
        "liquidity_pool_id",
        "exchange_order_id",
        "client_finality_claim",
        "payout_receipt_txid",
    ] {
        let mut plan =
            serde_json::to_value(reward_plan_ref(QuickChainEventClassV1::EconomicReceipt))
                .expect("plan should encode");
        plan.as_object_mut()
            .expect("plan JSON should be object")
            .insert(forbidden_field.to_string(), json!("forbidden"));

        assert!(
            serde_json::from_value::<QuickChainRewardPlanReferenceV1>(plan).is_err(),
            "forbidden field {forbidden_field} must reject"
        );

        let mut snapshot = serde_json::to_value(snapshot_ref()).expect("snapshot should encode");
        snapshot
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(forbidden_field.to_string(), json!("forbidden"));

        assert!(
            serde_json::from_value::<QuickChainAccountingSnapshotReferenceV1>(snapshot).is_err(),
            "forbidden field {forbidden_field} must reject"
        );
    }
}
