//! RO:WHAT — Internal ROC Beta Phase 3 Round 2 approved payout DTO boundary tests.
//! RO:WHY — Proves existing operation/receipt DTOs can represent approved payout execution receipts without adding client, rewarder, policy, accounting, bridge, staking, liquidity, or external-settlement authority.
//! RO:INTERACTS — QuickChainOperationIntentV1, QuickChainReceiptV1, QuickChainRewardPlanReferenceV1.
//! RO:INVARIANTS — DTO-only; svc-wallet remains mutation front-door; ron-ledger remains durable truth; reward plans are not receipts.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — rejects authority-smuggling fields and fake receipt/balance/finality claims.
//! RO:TEST — cargo test -p ron-proto --test internal_roc_beta_phase3_approved_payout_dto.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use ron_proto::{
    ContentId, QuickChainEventClassV1, QuickChainOperationClassV1, QuickChainOperationIntentV1,
    QuickChainReceiptStatusV1, QuickChainReceiptV1, QuickChainRewardPlanReferenceV1,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_OPERATION_INTENT_SCHEMA, QUICKCHAIN_RECEIPT_ASSET_ROC,
    QUICKCHAIN_RECEIPT_SCHEMA, QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";
const PAYOUT_ACCOUNT: &str = "account:creator-payout-phase3";
const PAYOUT_AMOUNT: &str = "37";
const PAYOUT_OPERATION_ID: &str = "op_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PAYOUT_IDEMPOTENCY_KEY: &str = "idem:phase3:payout:plan-a:creator-a";
const PAYOUT_TXID: &str = "tx:roc:phase3:payout:creator-a";

const FORBIDDEN_AUTHORITY_FIELDS: &[&str] = &[
    "reward_plan_id",
    "reward_plan_root",
    "accounting_snapshot_id",
    "accounting_snapshot_root",
    "policy_decision_receipt",
    "policy_receipt",
    "rewarder_receipt",
    "accounting_receipt",
    "client_receipt",
    "balance",
    "balance_minor",
    "available_balance",
    "wallet_balance",
    "ledger_balance",
    "payout_execution_truth",
    "payout_finality",
    "client_finality_claim",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
    "outside_settlement_claim",
];

fn cid(ch: char) -> ContentId {
    format!("b3:{}", ch.to_string().repeat(64))
        .parse()
        .expect("test CID")
}

fn approved_payout_intent() -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        operation_id: PAYOUT_OPERATION_ID.to_owned(),
        idempotency_key: PAYOUT_IDEMPOTENCY_KEY.to_owned(),
        op_class: QuickChainOperationClassV1::Issue,
        actor_account_id: PAYOUT_ACCOUNT.to_owned(),
        counterparty_account_id: None,
        amount_minor: Some(PAYOUT_AMOUNT.to_owned()),
        hold_id: None,

        // Ledger-assigned only after backend wallet/ledger acceptance.
        account_sequence: None,

        produced_at_ms: 1_900_100_000_000,
    }
}

fn approved_payout_receipt() -> QuickChainReceiptV1 {
    QuickChainReceiptV1 {
        schema: QUICKCHAIN_RECEIPT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        txid: PAYOUT_TXID.to_owned(),
        operation_id: PAYOUT_OPERATION_ID.to_owned(),
        op: "approved_payout".to_owned(),
        op_class: QuickChainOperationClassV1::Issue,
        status: QuickChainReceiptStatusV1::Accepted,
        from_account_id: None,
        to_account_id: Some(PAYOUT_ACCOUNT.to_owned()),
        asset: QUICKCHAIN_RECEIPT_ASSET_ROC.to_owned(),
        amount_minor: PAYOUT_AMOUNT.to_owned(),
        account_sequence: Some(7),
        hold_id: None,
        session_budget_id: None,
        idempotency_key: PAYOUT_IDEMPOTENCY_KEY.to_owned(),
        operation_hash: None,
        receipt_hash: None,
        receipt_root: None,
        checkpoint_hash: None,
        ledger_seq_start: Some(41),
        ledger_seq_end: Some(41),
        previous_ledger_root: Some(cid('1')),
        new_ledger_root: Some(cid('2')),
        memo: Some("backend wallet approved payout execution receipt".to_owned()),
        produced_at_ms: 1_900_100_000_001,
    }
}

fn reward_plan_reference() -> QuickChainRewardPlanReferenceV1 {
    QuickChainRewardPlanReferenceV1 {
        schema: QUICKCHAIN_REWARD_PLAN_REFERENCE_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        plan_id: "reward-plan-phase3-approved-payout-a".to_owned(),
        plan_root: cid('a'),
        snapshot_id: "accounting-snapshot-phase3-a".to_owned(),
        snapshot_root: cid('b'),
        source_event_class: QuickChainEventClassV1::ProofEligible,
        planned_total_minor: PAYOUT_AMOUNT.to_owned(),
        payout_candidate_count: 1,
        capped_by_policy: true,
        verification_ref: Some("verification-phase3-plan-a".to_owned()),
        funding_budget_ref: None,
        produced_at_ms: 1_900_099_999_999,
    }
}

#[test]
fn approved_payout_uses_existing_issue_intent_shape_without_client_sequence() {
    let intent = approved_payout_intent();

    intent
        .validate()
        .expect("approved payout issue intent should validate");

    assert_eq!(intent.op_class, QuickChainOperationClassV1::Issue);
    assert_eq!(intent.actor_account_id, PAYOUT_ACCOUNT);
    assert_eq!(intent.amount_minor.as_deref(), Some(PAYOUT_AMOUNT));
    assert_eq!(intent.account_sequence, None);
    assert_eq!(intent.counterparty_account_id, None);
    assert_eq!(intent.hold_id, None);

    let mut client_sequence_poison = serde_json::to_value(&intent).expect("intent serializes");
    client_sequence_poison["account_sequence"] = json!(99_u64);

    let poisoned = serde_json::from_value::<QuickChainOperationIntentV1>(client_sequence_poison)
        .expect("known field still deserializes");
    assert!(
        poisoned.validate().is_err(),
        "client must not assign account_sequence before wallet/ledger acceptance"
    );
}

#[test]
fn accepted_payout_receipt_is_backend_ledger_truth_not_reward_plan_truth() {
    let plan = reward_plan_reference();
    let receipt = approved_payout_receipt();

    plan.validate()
        .expect("reward plan reference remains valid planning material");
    receipt
        .validate()
        .expect("approved payout receipt should validate as accepted backend receipt");

    assert_eq!(receipt.status, QuickChainReceiptStatusV1::Accepted);
    assert!(receipt.status.is_backend_accepted());
    assert!(
        !receipt.status.is_epoch_included_or_stronger(),
        "accepted payout receipt must not overstate epoch/finality/anchor status"
    );
    assert_eq!(receipt.op_class, QuickChainOperationClassV1::Issue);
    assert_eq!(receipt.from_account_id, None);
    assert_eq!(receipt.to_account_id.as_deref(), Some(PAYOUT_ACCOUNT));
    assert_eq!(receipt.amount_minor, PAYOUT_AMOUNT);
    assert_ne!(
        receipt.txid, plan.plan_id,
        "reward plan id must not be reused as backend payout txid"
    );
}

#[test]
fn accepted_payout_receipt_requires_backend_assigned_sequence_and_ordering_context_when_claimed() {
    let mut missing_sequence = approved_payout_receipt();
    missing_sequence.account_sequence = None;
    missing_sequence
        .validate()
        .expect("accepted receipt may omit account_sequence until backend exposes it");

    let mut zero_sequence = approved_payout_receipt();
    zero_sequence.account_sequence = Some(0);
    assert!(
        zero_sequence.validate().is_err(),
        "account_sequence must be non-zero if present"
    );

    let mut half_range = approved_payout_receipt();
    half_range.ledger_seq_end = None;
    assert!(
        half_range.validate().is_err(),
        "ledger_seq_start and ledger_seq_end must be present together"
    );

    let mut half_roots = approved_payout_receipt();
    half_roots.new_ledger_root = None;
    assert!(
        half_roots.validate().is_err(),
        "previous_ledger_root and new_ledger_root must be present together"
    );
}

#[test]
fn future_finality_status_requires_receipt_root_checkpoint_and_sequence_evidence() {
    let mut finalized_without_evidence = approved_payout_receipt();
    finalized_without_evidence.status = QuickChainReceiptStatusV1::Finalized;

    assert!(
        finalized_without_evidence.validate().is_err(),
        "finalized payout receipt status requires receipt/root/checkpoint evidence"
    );

    finalized_without_evidence.receipt_hash = Some(cid('3'));
    finalized_without_evidence.receipt_root = Some(cid('4'));
    finalized_without_evidence.checkpoint_hash = Some(cid('5'));

    finalized_without_evidence
        .validate()
        .expect("finalized status validates only when future proof references are present");
}

#[test]
fn payout_intent_and_receipt_reject_authority_poison_fields() {
    for forbidden in FORBIDDEN_AUTHORITY_FIELDS {
        let mut intent_json = serde_json::to_value(approved_payout_intent()).expect("intent JSON");
        intent_json
            .as_object_mut()
            .expect("intent JSON object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<QuickChainOperationIntentV1>(intent_json).is_err(),
            "payout operation intent must reject authority-smuggling field `{forbidden}`"
        );

        let mut receipt_json =
            serde_json::to_value(approved_payout_receipt()).expect("receipt JSON");
        receipt_json
            .as_object_mut()
            .expect("receipt JSON object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<QuickChainReceiptV1>(receipt_json).is_err(),
            "payout receipt DTO must reject authority-smuggling field `{forbidden}`"
        );
    }
}
