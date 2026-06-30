//! RO:WHAT — Internal ROC Beta paid-content DTO coverage for post/comment/article/content_view labels.
//! RO:WHY — Phase 1 proves repeatable paid content flows can use existing strict receipt/event DTOs.
//! RO:INTERACTS — ron_proto::quickchain receipts, usage events, money strings, event classes.
//! RO:INVARIANTS — DTO-only; backend-derived receipt labels only; no bridge/staking/liquidity fields.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — validation does not create receipt, balance, entitlement, finality, or wallet authority.
//! RO:TEST — cargo test -p ron-proto --test internal_roc_beta_paid_content_dto.

use ron_proto::{
    ContentId, QuickChainEventClassV1, QuickChainOperationClassV1, QuickChainReceiptStatusV1,
    QuickChainReceiptV1, QuickChainUsageEventV1, QuickChainValidationError, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_RECEIPT_SCHEMA, QUICKCHAIN_USAGE_EVENT_SCHEMA,
};
use serde_json::json;
use std::collections::BTreeMap;

const CHAIN_ID: &str = "ron-devnet";

fn content_cid(hex_digit: char) -> ContentId {
    let hex = hex_digit.to_string().repeat(64);
    format!("b3:{hex}").parse().expect("valid test CID")
}

fn operation_id(index: u8) -> String {
    format!("op_{index:032x}")
}

fn paid_receipt(action: &str, operation_index: u8, amount_minor: &str) -> QuickChainReceiptV1 {
    QuickChainReceiptV1 {
        schema: QUICKCHAIN_RECEIPT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        txid: format!("tx:roc:{action}:accepted:{operation_index}"),
        operation_id: operation_id(operation_index),
        op: action.to_string(),
        op_class: QuickChainOperationClassV1::Transfer,
        status: QuickChainReceiptStatusV1::Accepted,
        from_account_id: Some("account:visitor".to_string()),
        to_account_id: Some("account:creator".to_string()),
        asset: "roc".to_string(),
        amount_minor: amount_minor.to_string(),
        account_sequence: Some(u64::from(operation_index)),
        hold_id: None,
        session_budget_id: None,
        idempotency_key: format!("idem:{action}:visitor:creator:{operation_index}"),
        operation_hash: None,
        receipt_hash: None,
        receipt_root: None,
        checkpoint_hash: None,
        ledger_seq_start: None,
        ledger_seq_end: None,
        previous_ledger_root: None,
        new_ledger_root: None,
        memo: Some("backend-derived paid content receipt".to_string()),
        produced_at_ms: 1_820_000_000_000 + u64::from(operation_index),
    }
}

fn paid_usage_event(
    action: &str,
    event_class: QuickChainEventClassV1,
    amount_minor: Option<&str>,
    labels: BTreeMap<String, String>,
) -> QuickChainUsageEventV1 {
    QuickChainUsageEventV1 {
        schema: QUICKCHAIN_USAGE_EVENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        event_id: format!("event:{action}:{}", labels.len() + 1),
        action: action.to_string(),
        event_class,
        account_id: Some("account:visitor".to_string()),
        counterparty_account_id: Some("account:creator".to_string()),
        object_cid: Some(content_cid('a')),
        site_name: Some("crab:paid-demo-site".to_string()),
        amount_minor: amount_minor.map(str::to_string),
        units: 1,
        labels,
        produced_at_ms: 1_820_000_000_500,
        idempotency_key: format!("idem:event:{action}:{}", 42),
    }
}

fn assert_invalid_money(receipt: QuickChainReceiptV1) {
    let error = receipt
        .validate()
        .expect_err("receipt should reject invalid money");

    match error {
        QuickChainValidationError::InvalidMoney { field, .. } => {
            assert_eq!(field, "amount_minor");
        }
        other => panic!("expected InvalidMoney for amount_minor, got {other:?}"),
    }
}

#[test]
fn paid_content_receipts_validate_for_beta_action_labels() {
    for (action, operation_index, amount_minor) in [
        ("paid_post", 1, "10"),
        ("paid_comment", 2, "15"),
        ("paid_article", 3, "20"),
        ("content_view", 4, "25"),
    ] {
        let receipt = paid_receipt(action, operation_index, amount_minor);

        receipt
            .validate()
            .expect("accepted paid-content receipt DTO should validate");

        assert_eq!(receipt.asset, "roc");
        assert_eq!(receipt.op, action);
        assert_eq!(receipt.amount_minor, amount_minor);
        assert_eq!(receipt.op_class, QuickChainOperationClassV1::Transfer);
        assert!(receipt.status.is_backend_accepted());
        assert!(!receipt.status.is_epoch_included_or_stronger());

        let encoded = serde_json::to_string(&receipt).expect("receipt should serialize");
        assert!(encoded.contains("\"status\":\"accepted\""));
        assert!(encoded.contains("\"asset\":\"roc\""));
        assert!(encoded.contains(action));

        let decoded: QuickChainReceiptV1 =
            serde_json::from_str(&encoded).expect("receipt should roundtrip");
        assert_eq!(decoded, receipt);
        decoded
            .validate()
            .expect("roundtripped receipt should validate");
    }
}

#[test]
fn paid_content_usage_events_keep_receipt_truth_separate_from_metering() {
    let mut receipt_labels = BTreeMap::new();
    receipt_labels.insert("paid_action".to_string(), "paid_post".to_string());
    receipt_labels.insert("source".to_string(), "svc-wallet".to_string());

    let economic_event = paid_usage_event(
        "paid_post",
        QuickChainEventClassV1::EconomicReceipt,
        Some("10"),
        receipt_labels,
    );

    economic_event
        .validate()
        .expect("wallet-derived economic receipt event should validate");
    assert!(economic_event.event_class.may_represent_balance_truth());
    assert!(economic_event
        .event_class
        .may_enter_reward_manifest_without_extra_proof());

    let mut metering_labels = BTreeMap::new();
    metering_labels.insert("paid_action".to_string(), "content_view".to_string());
    metering_labels.insert("source".to_string(), "omnigate".to_string());

    let metering_event = paid_usage_event(
        "content_view",
        QuickChainEventClassV1::Metering,
        None,
        metering_labels,
    );

    metering_event
        .validate()
        .expect("metering event should validate but remain non-authority");
    assert!(!metering_event.event_class.may_represent_balance_truth());
    assert!(!metering_event
        .event_class
        .may_enter_reward_manifest_without_extra_proof());

    let mut analytics_labels = BTreeMap::new();
    analytics_labels.insert("paid_action".to_string(), "paid_article".to_string());
    analytics_labels.insert("source".to_string(), "crablink-display".to_string());

    let analytics_event = paid_usage_event(
        "paid_article",
        QuickChainEventClassV1::AnalyticsOnly,
        None,
        analytics_labels,
    );

    analytics_event
        .validate()
        .expect("analytics-only event should validate as display/reporting data");
    assert!(!analytics_event.event_class.may_represent_balance_truth());
    assert!(!analytics_event
        .event_class
        .may_enter_reward_manifest_without_extra_proof());
}

#[test]
fn paid_content_dtos_reject_unknown_bridge_staking_liquidity_fields() {
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
    ] {
        let mut value = serde_json::to_value(paid_receipt("paid_article", 9, "20"))
            .expect("receipt should convert to JSON value");

        value
            .as_object_mut()
            .expect("receipt JSON should be an object")
            .insert(forbidden_field.to_string(), json!("forbidden"));

        serde_json::from_value::<QuickChainReceiptV1>(value)
            .expect_err("strict receipt DTO must reject forbidden active external-scope fields");
    }

    let mut event_labels = BTreeMap::new();
    event_labels.insert("paid_action".to_string(), "paid_comment".to_string());

    let mut value = serde_json::to_value(paid_usage_event(
        "paid_comment",
        QuickChainEventClassV1::Metering,
        None,
        event_labels,
    ))
    .expect("usage event should convert to JSON value");

    value
        .as_object_mut()
        .expect("event JSON should be an object")
        .insert("raw_engagement_mints_roc".to_string(), json!(true));

    serde_json::from_value::<QuickChainUsageEventV1>(value)
        .expect_err("strict usage DTO must reject raw-engagement mint shortcuts");
}

#[test]
fn paid_content_money_stays_integer_minor_unit_string_only() {
    let valid = paid_receipt("content_view", 7, "25");
    valid
        .validate()
        .expect("integer string amount should validate");

    let mut decimal = valid.clone();
    decimal.amount_minor = "1.0".to_string();
    assert_invalid_money(decimal);

    let mut leading_zero = valid.clone();
    leading_zero.amount_minor = "025".to_string();
    assert_invalid_money(leading_zero);

    let mut numeric_json = serde_json::to_value(valid).expect("receipt should encode");
    numeric_json["amount_minor"] = json!(25);

    serde_json::from_value::<QuickChainReceiptV1>(numeric_json)
        .expect_err("numeric JSON amount must reject before validation");
}
