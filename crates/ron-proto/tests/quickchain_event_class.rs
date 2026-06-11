use ron_proto::{
    ContentId, QuickChainEventClassV1, QuickChainUsageEventV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_USAGE_EVENT_SCHEMA,
};
use serde_json::json;
use std::collections::BTreeMap;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn valid_usage_event(event_class: QuickChainEventClassV1) -> QuickChainUsageEventV1 {
    let mut labels = BTreeMap::new();
    labels.insert("route".to_string(), "site-view".to_string());

    QuickChainUsageEventV1 {
        schema: QUICKCHAIN_USAGE_EVENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        event_id: "event:site-view:0001".to_string(),
        action: "site_visit".to_string(),
        event_class,
        account_id: Some("account:viewer-a".to_string()),
        counterparty_account_id: Some("account:creator-b".to_string()),
        object_cid: Some(cid('a')),
        site_name: Some("site:demo".to_string()),
        amount_minor: Some("10".to_string()),
        units: 1,
        labels,
        produced_at_ms: 1_777_000_000_000,
        idempotency_key: "event:site-view:0001".to_string(),
    }
}

#[test]
fn event_class_wire_names_are_strict_and_unknown_rejects() {
    let cases = [
        (QuickChainEventClassV1::EconomicReceipt, "economic_receipt"),
        (QuickChainEventClassV1::Metering, "metering"),
        (QuickChainEventClassV1::ProofEligible, "proof_eligible"),
        (QuickChainEventClassV1::AdBudgeted, "ad_budgeted"),
        (QuickChainEventClassV1::AnalyticsOnly, "analytics_only"),
    ];

    for (variant, wire) in cases {
        assert_eq!(
            serde_json::to_string(&variant).unwrap(),
            format!("\"{wire}\"")
        );
        let decoded: QuickChainEventClassV1 = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(decoded, variant);
    }

    serde_json::from_value::<QuickChainEventClassV1>(json!("raw_engagement_reward"))
        .expect_err("unknown event class must reject");
}

#[test]
fn event_class_policy_helpers_are_conservative() {
    assert!(QuickChainEventClassV1::EconomicReceipt.may_represent_balance_truth());
    assert!(!QuickChainEventClassV1::Metering.may_represent_balance_truth());
    assert!(!QuickChainEventClassV1::ProofEligible.may_represent_balance_truth());
    assert!(!QuickChainEventClassV1::AdBudgeted.may_represent_balance_truth());
    assert!(!QuickChainEventClassV1::AnalyticsOnly.may_represent_balance_truth());

    assert!(QuickChainEventClassV1::EconomicReceipt.may_enter_reward_manifest_without_extra_proof());
    assert!(QuickChainEventClassV1::AdBudgeted.may_enter_reward_manifest_without_extra_proof());
    assert!(!QuickChainEventClassV1::Metering.may_enter_reward_manifest_without_extra_proof());
    assert!(!QuickChainEventClassV1::ProofEligible.may_enter_reward_manifest_without_extra_proof());
    assert!(!QuickChainEventClassV1::AnalyticsOnly.may_enter_reward_manifest_without_extra_proof());
}

#[test]
fn usage_event_validates_and_rejects_unknown_fields() {
    let event = valid_usage_event(QuickChainEventClassV1::AnalyticsOnly);
    event.validate().unwrap();

    let mut value = serde_json::to_value(event).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_string(), json!(true));

    serde_json::from_value::<QuickChainUsageEventV1>(value)
        .expect_err("unknown usage event field must reject");
}

#[test]
fn usage_event_rejects_bad_money_and_bad_idempotency() {
    let mut event = valid_usage_event(QuickChainEventClassV1::EconomicReceipt);
    event.amount_minor = Some("01".to_string());
    event
        .validate()
        .expect_err("non-canonical money must reject");

    let mut event = valid_usage_event(QuickChainEventClassV1::Metering);
    event.idempotency_key = "contains space".to_string();
    event
        .validate()
        .expect_err("bad idempotency key must reject");
}
