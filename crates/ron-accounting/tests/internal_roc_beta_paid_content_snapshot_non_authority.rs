//! RO:WHAT — Internal ROC Beta paid-content accounting snapshot non-authority tests.
//! RO:WHY — Internal ROC Beta Phase 1; Concerns: ECON/GOV/SEC. Accounting may observe paid receipts after wallet/ledger truth but must not become balance, receipt, entitlement, or payout authority.
//! RO:INTERACTS — UsageEvent, Recorder, SealedSlice, RewardProjectionConfig.
//! RO:INVARIANTS — accounting snapshots are derivative; no wallet/ledger mutation; no paid unlock authority; raw engagement cannot directly mint ROC.
//! RO:METRICS — none.
//! RO:CONFIG — EventIngestPolicy::default and RewardProjectionConfig::new.
//! RO:SECURITY — rejects authority poison fields and keeps snapshot artifacts display/planning only.
//! RO:TEST — cargo test -p ron-accounting --test internal_roc_beta_paid_content_snapshot_non_authority.

use ron_accounting::{
    project_reward_snapshot_from_slices, record_usage_events, Dimension, EventIngestPolicy,
    MetricKind, Recorder, RewardProjectionConfig, SliceId, UsageEvent, Window,
};
use serde_json::{json, Value};

const TENANT: u128 = 77;

#[derive(Debug, Clone, Copy)]
struct PaidObservation {
    action: &'static str,
    amount_minor: u64,
}

const PAID_OBSERVATIONS: &[PaidObservation] = &[
    PaidObservation {
        action: "paid_post",
        amount_minor: 10,
    },
    PaidObservation {
        action: "paid_comment",
        amount_minor: 15,
    },
    PaidObservation {
        action: "paid_article",
        amount_minor: 20,
    },
    PaidObservation {
        action: "content_view",
        amount_minor: 25,
    },
];

#[test]
fn wallet_receipt_observations_seal_as_derivative_snapshot_not_balance_truth() {
    let recorder = Recorder::default();

    let events = PAID_OBSERVATIONS
        .iter()
        .enumerate()
        .map(|(idx, observation)| {
            UsageEvent::new(
                1_800_000 + idx as u64,
                TENANT,
                "svc-wallet",
                MetricKind::Custom(format!("economic_receipt_{}", observation.action)),
                observation.amount_minor,
            )
            .with_source_service("svc-wallet")
            .with_region("local")
            .with_route(format!(
                "/internal-roc/{}/accepted-wallet-receipt",
                observation.action
            ))
        })
        .collect::<Vec<_>>();

    let report = record_usage_events(&recorder, &events, &EventIngestPolicy::default())
        .expect("paid wallet receipt observations should record");

    assert_eq!(report.inspected, PAID_OBSERVATIONS.len());
    assert_eq!(report.recorded, PAID_OBSERVATIONS.len());
    assert_eq!(report.skipped_zero, 0);

    let slice = recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 1,
            },
            Window::for_timestamp_ms(1_800_000, 300).expect("valid beta accounting window"),
            None,
            true,
        )
        .expect("paid content accounting slice should seal");

    assert_eq!(slice.rows.len(), PAID_OBSERVATIONS.len());
    assert_eq!(slice.id.tenant, TENANT);
    assert_eq!(slice.id.dimension, Dimension::Requests);
    assert_eq!(slice.rows.iter().map(|row| row.value).sum::<u64>(), 70);
    assert_b3_artifact(&slice.digest);

    for row in &slice.rows {
        assert_eq!(
            row.labels.service, "svc-wallet",
            "paid economic receipt observations must remain wallet-sourced"
        );
        assert_eq!(
            row.dimension,
            Dimension::Requests,
            "paid receipt observation is an accounting counter, not a balance"
        );
    }

    let value = serde_json::to_value(&slice).expect("slice JSON");
    assert_no_authority_keys(&value);
}

#[test]
fn raw_paid_content_engagement_cannot_project_rewards_without_verification_or_receipt_truth() {
    let recorder = Recorder::default();

    let raw_events = [
        UsageEvent::new(
            1_900_000,
            TENANT,
            "creator-a",
            MetricKind::Custom("views".to_string()),
            1_000,
        )
        .with_source_service("omnigate")
        .with_region("local")
        .with_route("/analytics/post/views"),
        UsageEvent::new(
            1_900_001,
            TENANT,
            "creator-a",
            MetricKind::Custom("comment_likes".to_string()),
            100,
        )
        .with_source_service("omnigate")
        .with_region("local")
        .with_route("/analytics/comment/likes"),
    ];

    let report = record_usage_events(&recorder, &raw_events, &EventIngestPolicy::default())
        .expect("raw analytics events can be metered");

    assert_eq!(report.recorded, raw_events.len());

    let slice = recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 2,
            },
            Window::for_timestamp_ms(1_900_000, 300).expect("valid beta accounting window"),
            None,
            true,
        )
        .expect("raw engagement accounting slice should seal");

    assert_eq!(slice.rows.iter().map(|row| row.value).sum::<u64>(), 1_100);

    let err = project_reward_snapshot_from_slices(
        1_900_500,
        &RewardProjectionConfig::new("1000"),
        &[slice],
    )
    .expect_err("raw analytics engagement alone must not create reward snapshot material");

    let message = err.to_string();
    assert!(
        message.contains("reward snapshot must contain at least one contribution"),
        "unexpected projection rejection message: {message}"
    );
}

#[test]
fn accounting_usage_events_reject_paid_content_authority_poison_fields() {
    for forbidden_field in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_side_effect",
        "rewarder_direct_payout",
        "client_finality_claim",
        "cache_unlock_authority",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
    ] {
        let mut value = json!({
            "timestamp_ms": 2_000_000,
            "tenant": TENANT,
            "subject": "svc-wallet",
            "metric_kind": { "custom": "economic_receipt_paid_post" },
            "value": 10,
            "source_service": "svc-wallet",
            "region": "local",
            "route": "/internal-roc/paid_post/accepted-wallet-receipt"
        });

        value
            .as_object_mut()
            .expect("test event object")
            .insert(forbidden_field.to_string(), json!(true));

        assert!(
            serde_json::from_value::<UsageEvent>(value).is_err(),
            "UsageEvent must reject forbidden paid-content authority field `{forbidden_field}`"
        );
    }
}

fn assert_b3_artifact(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected b3:<64 lowercase hex>, got {value}"
    );

    let hex = &value[3..];
    assert_eq!(hex.len(), 64, "expected 64 hex chars, got {value}");
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase b3 artifact CID, got {value}"
    );
}

fn assert_no_authority_keys(value: &Value) {
    let forbidden_keys = [
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_side_effect",
        "rewarder_direct_payout",
        "client_finality_claim",
        "cache_unlock_authority",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
        "accounting_root",
        "reward_root",
        "checkpoint_root",
        "finality_truth",
        "settlement_truth",
    ];

    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !forbidden_keys.iter().any(|forbidden| key == forbidden),
                    "accounting snapshot artifact must not expose authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_no_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_authority_keys(nested);
            }
        }
        _ => {}
    }
}
