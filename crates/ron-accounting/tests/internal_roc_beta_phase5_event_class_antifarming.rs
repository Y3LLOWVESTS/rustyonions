#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — Internal ROC Beta Phase 5 Round 2 event-class anti-farming tests for ron-accounting.
//! RO:WHY — Proves analytics, metering, proof-eligible, ad-budgeted, and economic-receipt lanes remain isolated before reward planning.
//! RO:INTERACTS — `InternalRocEventClassDecision`, `MetricKind`, reward-projection boundaries.
//! RO:INVARIANTS — raw usage cannot directly mint/allocate ROC; accounting remains non-authoritative.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects client/raw attempts to claim economic receipt or reward authority.
//! RO:TEST — `cargo test -p ron-accounting --test internal_roc_beta_phase5_event_class_antifarming`.

use ron_accounting::{
    classify_metric_for_internal_roc, economic_receipt_decision_from_source, InternalRocEventClass,
    InternalRocEventClassDecision, MetricKind,
};
use serde_json::json;

fn assert_no_economic_authority(decision: &InternalRocEventClassDecision) {
    assert!(!decision.direct_protocol_roc_allocation);
    assert!(!decision.wallet_side_effect);
    assert!(!decision.ledger_side_effect);
    assert!(!decision.receipt_truth);
    assert!(!decision.balance_truth);
    decision
        .validate()
        .expect("valid classification decisions must remain non-authoritative");
}

#[test]
fn analytics_only_quarantines_raw_engagement_from_reward_material() {
    for metric in [
        MetricKind::Custom("post_view".to_owned()),
        MetricKind::Custom("article_click".to_owned()),
        MetricKind::Custom("like".to_owned()),
        MetricKind::Custom("scroll_depth".to_owned()),
        MetricKind::Custom("watch_start".to_owned()),
        MetricKind::Custom("comment_impression".to_owned()),
    ] {
        let decision = classify_metric_for_internal_roc(&metric);

        assert_eq!(decision.event_class, InternalRocEventClass::AnalyticsOnly);
        assert!(!decision.reward_planning_candidate);
        assert!(!decision.requires_verification);
        assert!(!decision.requires_explicit_budget);
        assert!(!decision.requires_wallet_ledger_source);
        assert_no_economic_authority(&decision);
    }
}

#[test]
fn metering_never_directly_becomes_payout_or_receipt_truth() {
    for metric in [
        MetricKind::RequestOk,
        MetricKind::PinSeconds,
        MetricKind::CpuUnits,
    ] {
        let decision = classify_metric_for_internal_roc(&metric);

        assert_eq!(decision.event_class, InternalRocEventClass::Metering);
        assert!(!decision.reward_planning_candidate);
        assert!(!decision.requires_explicit_budget);
        assert!(!decision.requires_wallet_ledger_source);
        assert_no_economic_authority(&decision);
    }
}

#[test]
fn proof_eligible_service_metrics_require_verification_before_planning() {
    for metric in [
        MetricKind::BytesStored,
        MetricKind::BytesServed,
        MetricKind::UptimeSeconds,
    ] {
        let decision = classify_metric_for_internal_roc(&metric);

        assert_eq!(decision.event_class, InternalRocEventClass::ProofEligible);
        assert!(decision.reward_planning_candidate);
        assert!(decision.requires_verification);
        assert!(!decision.requires_explicit_budget);
        assert!(!decision.requires_wallet_ledger_source);
        assert_no_economic_authority(&decision);
    }
}

#[test]
fn ad_budgeted_events_require_explicit_budget_and_do_not_mint_protocol_roc() {
    for metric in [
        MetricKind::Custom("ad_impression".to_owned()),
        MetricKind::Custom("sponsored_view".to_owned()),
        MetricKind::Custom("campaign_click".to_owned()),
    ] {
        let decision = classify_metric_for_internal_roc(&metric);

        assert_eq!(decision.event_class, InternalRocEventClass::AdBudgeted);
        assert!(!decision.reward_planning_candidate);
        assert!(decision.requires_verification);
        assert!(decision.requires_explicit_budget);
        assert_no_economic_authority(&decision);
    }
}

#[test]
fn economic_receipt_class_requires_backend_wallet_or_ledger_source() {
    assert!(economic_receipt_decision_from_source("browser:raw-click").is_err());
    assert!(economic_receipt_decision_from_source("analytics:post-view").is_err());

    for source in ["svc-wallet:receipt", "ron-ledger:accepted"] {
        let decision = economic_receipt_decision_from_source(source)
            .expect("backend wallet/ledger source may be classified as economic receipt");

        assert_eq!(decision.event_class, InternalRocEventClass::EconomicReceipt);
        assert!(!decision.reward_planning_candidate);
        assert!(decision.requires_verification);
        assert!(decision.requires_wallet_ledger_source);
        assert_no_economic_authority(&decision);
    }
}

#[test]
fn event_class_decisions_reject_authority_poisoning_and_unknown_fields() {
    let decision = classify_metric_for_internal_roc(&MetricKind::BytesServed);

    for authority_flag in [
        "direct_protocol_roc_allocation",
        "wallet_side_effect",
        "ledger_side_effect",
        "receipt_truth",
        "balance_truth",
    ] {
        let mut value = serde_json::to_value(&decision).expect("decision should serialize");
        value
            .as_object_mut()
            .expect("decision JSON should be an object")
            .insert(authority_flag.to_owned(), json!(true));

        let poisoned = serde_json::from_value::<InternalRocEventClassDecision>(value)
            .expect("known flag should parse before validation");
        assert!(
            poisoned.validate().is_err(),
            "authority flag `{authority_flag}` must fail validation"
        );
    }

    let mut value = serde_json::to_value(decision).expect("decision should serialize");
    value
        .as_object_mut()
        .expect("decision JSON should be an object")
        .insert("wallet_issue_request".to_owned(), json!("forbidden"));

    assert!(
        serde_json::from_value::<InternalRocEventClassDecision>(value).is_err(),
        "unknown wallet-authority field must be rejected"
    );
}
