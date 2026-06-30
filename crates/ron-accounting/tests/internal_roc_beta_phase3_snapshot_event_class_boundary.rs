//! RO:WHAT — Internal ROC Beta Phase 3 accounting snapshot/event-class boundary tests.
//! RO:WHY — Phase 3 Round 1 proves classified events can become deterministic accounting snapshot material without wallet mutation, payout execution, balance truth, fake receipts, or raw-engagement direct ROC allocation.
//! RO:INTERACTS — UsageEvent, EventIngestPolicy, Recorder, SealedSlice, RewardSnapshotExport.
//! RO:INVARIANTS — accounting remains derivative; economic receipts are observed only after wallet/ledger truth; metering/proof/ad/analytics lanes do not become direct payout authority.
//! RO:METRICS — none.
//! RO:CONFIG — EventIngestPolicy::default, fixed test window.
//! RO:SECURITY — rejects client-supplied event_class/reward/payout authority fields; no bridge/staking/liquidity/external-settlement behavior.
//! RO:TEST — cargo test -p ron-accounting --test internal_roc_beta_phase3_snapshot_event_class_boundary.

use ron_accounting::{
    record_usage_events, CounterRow, Dimension, EventIngestPolicy, MetricKind, Recorder,
    RewardContributionExport, RewardSnapshotExport, SealedSlice, SliceId, UsageEvent, Window,
};
use serde_json::{json, Value};

const TENANT: u128 = 303;
const VIEWER: &str = "acct_phase3_viewer";
const CREATOR: &str = "acct_phase3_creator";

const FORBIDDEN_AUTHORITY_KEYS: &[&str] = &[
    "balance",
    "balance_minor",
    "available_balance",
    "wallet_balance",
    "ledger_balance",
    "wallet_receipt",
    "ledger_receipt",
    "receipt_txid",
    "payout_receipt_txid",
    "reward_plan_id",
    "reward_plan_root",
    "accounting_snapshot_root",
    "payout_execution",
    "wallet_mutation",
    "ledger_mutation",
    "issue",
    "burn",
    "transfer",
    "hold",
    "capture",
    "release",
    "entitlement",
    "unlock",
    "finality",
    "client_finality_claim",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
];

fn accepted_wallet_receipt_event(timestamp_ms: u64, action: &str, amount_minor: u64) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        CREATOR,
        MetricKind::Custom("econ_ok".to_string()),
        amount_minor,
    )
    .with_source_service("svc-wallet")
    .with_region("internal")
    .with_route(format!("/internal-roc/{action}/wallet-receipt"))
}

fn metering_event(timestamp_ms: u64, subject: &str, amount: u64) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        subject,
        MetricKind::BytesStored,
        amount,
    )
    .with_source_service("svc-storage")
    .with_region("internal")
}

fn analytics_only_event(timestamp_ms: u64) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        VIEWER,
        MetricKind::Custom("analytics".to_string()),
        1,
    )
    .with_source_service("omnigate")
    .with_region("internal")
    .with_route("/analytics/raw-view")
}

fn proof_eligible_event(timestamp_ms: u64) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        VIEWER,
        MetricKind::Custom("proof_eligible".to_string()),
        30,
    )
    .with_source_service("svc-gateway")
    .with_region("internal")
    .with_route("/usage/proof-eligible/watch-time")
}

fn ad_budgeted_event(timestamp_ms: u64) -> UsageEvent {
    UsageEvent::new(
        timestamp_ms,
        TENANT,
        VIEWER,
        MetricKind::Custom("ad_budget".to_string()),
        1,
    )
    .with_source_service("svc-gateway")
    .with_region("internal")
    .with_route("/usage/ad-budgeted/impression")
}

fn record_rows(events: &[UsageEvent]) -> Vec<CounterRow> {
    let recorder = Recorder::default();
    let report = record_usage_events(&recorder, events, &EventIngestPolicy::default())
        .expect("phase3 accounting events should record");
    assert_eq!(report.inspected, events.len());
    assert_eq!(report.recorded, events.len());
    assert_eq!(report.skipped_zero, 0);
    recorder.snapshot()
}

fn sealed_request_slice(events: &[UsageEvent]) -> SealedSlice {
    let recorder = Recorder::default();
    record_usage_events(&recorder, events, &EventIngestPolicy::default())
        .expect("events should record before seal");

    let window = Window::for_timestamp_ms(1_900_000_000_000, 300)
        .expect("fixed test window should construct");

    recorder
        .seal_slice(
            SliceId {
                tenant: TENANT,
                dimension: Dimension::Requests,
                seq: 1,
            },
            window,
            None,
            true,
        )
        .expect("request stream should seal")
}

fn assert_no_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    !FORBIDDEN_AUTHORITY_KEYS.contains(&key.as_str()),
                    "accounting artifact must not expose authority key `{key}`"
                );
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

#[test]
fn paid_wallet_receipt_events_become_snapshot_input_only() {
    let events = vec![
        accepted_wallet_receipt_event(1_900_000_000_001, "paid-post", 10),
        accepted_wallet_receipt_event(1_900_000_000_002, "paid-comment", 15),
        accepted_wallet_receipt_event(1_900_000_000_003, "content-view", 25),
    ];

    let rows = record_rows(&events);
    assert_eq!(rows.len(), 3);

    let total: u64 = rows.iter().map(|row| row.value).sum();
    assert_eq!(total, 50);

    for row in &rows {
        assert_eq!(row.key.labels.tenant, TENANT);
        assert_eq!(row.key.dimension, Dimension::Requests);
        assert!(
            row.key.labels.route.starts_with("/internal-roc/"),
            "economic_receipt accounting input must retain internal ROC observation route context"
        );
        assert_eq!(
            row.key.labels.method, "ECON_OK",
            "economic_receipt accounting input must use a bounded stable method label"
        );
    }

    let slice = sealed_request_slice(&events);
    assert_eq!(slice.id.tenant, TENANT);
    assert_eq!(slice.id.dimension, Dimension::Requests);
    assert_eq!(slice.rows.len(), 3);
    assert_eq!(slice.rows.iter().map(|row| row.value).sum::<u64>(), 50);

    let json = serde_json::to_value(&slice).expect("sealed slice should serialize");
    assert_no_authority_keys(&json);
}

#[test]
fn accounting_snapshot_rows_are_deterministic_for_same_events_regardless_input_order() {
    let forward = vec![
        accepted_wallet_receipt_event(1_900_000_000_001, "paid-post", 10),
        accepted_wallet_receipt_event(1_900_000_000_002, "paid-comment", 15),
        analytics_only_event(1_900_000_000_003),
        proof_eligible_event(1_900_000_000_004),
        ad_budgeted_event(1_900_000_000_005),
    ];

    let mut reversed = forward.clone();
    reversed.reverse();

    let left = record_rows(&forward);
    let right = record_rows(&reversed);

    assert_eq!(
        left, right,
        "accounting recorder snapshots must sort normalized rows deterministically"
    );

    let left_slice = sealed_request_slice(&forward);
    let right_slice = sealed_request_slice(&reversed);

    assert_eq!(left_slice.rows, right_slice.rows);
    assert_eq!(left_slice.id, right_slice.id);
    assert_eq!(
        left_slice.meta.window_start_ms,
        right_slice.meta.window_start_ms
    );
    assert_eq!(
        left_slice.meta.window_end_ms,
        right_slice.meta.window_end_ms
    );
}

#[test]
fn event_class_isolation_keeps_non_receipt_lanes_from_direct_payout_authority() {
    let events = vec![
        metering_event(1_900_000_000_010, "provider-a", 512),
        analytics_only_event(1_900_000_000_011),
        proof_eligible_event(1_900_000_000_012),
        ad_budgeted_event(1_900_000_000_013),
        UsageEvent::new(
            1_900_000_000_014,
            TENANT,
            VIEWER,
            MetricKind::Custom("raw-like".to_string()),
            1,
        )
        .with_source_service("omnigate")
        .with_region("internal")
        .with_route("/engagement/like"),
    ];

    let rows = record_rows(&events);
    assert_eq!(rows.len(), 5);

    let mut saw_metering = false;
    let mut saw_analytics = false;
    let mut saw_proof_eligible = false;
    let mut saw_ad_budgeted = false;
    let mut saw_raw_like = false;

    for row in rows {
        let method = row.key.labels.method.as_str();
        let route = row.key.labels.route.as_str();

        match method {
            "PUT" => {
                saw_metering = true;
                assert_eq!(row.key.dimension, Dimension::Bytes);
            }
            "ANALYTICS" => {
                saw_analytics = true;
                assert_eq!(row.key.dimension, Dimension::Requests);
                assert!(route.contains("analytics"));
            }
            "PROOF_ELIGIBLE" => {
                saw_proof_eligible = true;
                assert_eq!(row.key.dimension, Dimension::Requests);
                assert!(route.contains("proof-eligible"));
            }
            "AD_BUDGET" => {
                saw_ad_budgeted = true;
                assert_eq!(row.key.dimension, Dimension::Requests);
                assert!(route.contains("ad-budgeted"));
            }
            "RAW_LIKE" => {
                saw_raw_like = true;
                assert_eq!(row.key.dimension, Dimension::Requests);
                assert!(route.contains("engagement"));
            }
            other => panic!("unexpected method label in phase3 event-class test: {other}"),
        }
    }

    assert!(saw_metering);
    assert!(saw_analytics);
    assert!(saw_proof_eligible);
    assert!(saw_ad_budgeted);
    assert!(saw_raw_like);
}

#[test]
fn usage_events_reject_client_supplied_classification_and_reward_authority() {
    let clean = serde_json::to_value(UsageEvent::new(
        1_900_000_000_020,
        TENANT,
        VIEWER,
        MetricKind::Custom("raw-view".to_string()),
        1,
    ))
    .expect("usage event should serialize");

    for forbidden in FORBIDDEN_AUTHORITY_KEYS.iter().copied().chain([
        "event_class",
        "source_event_class",
        "reward_material",
    ]) {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("usage event JSON should be object")
            .insert(forbidden.to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<UsageEvent>(poisoned).is_err(),
            "UsageEvent must reject client-supplied authority field `{forbidden}`"
        );
    }
}

#[test]
fn reward_snapshot_exports_remain_planning_material_not_wallet_authority() {
    let snapshot = RewardSnapshotExport::new(
        1_900_000_000_030,
        "1000",
        vec![
            RewardContributionExport::new("acct_phase3_a", 100, 50, 10),
            RewardContributionExport::new("acct_phase3_b", 200, 0, 20),
        ],
    )
    .expect("valid reward snapshot export should construct");

    let canonical = snapshot
        .canonicalized()
        .expect("reward snapshot should canonicalize");
    assert_eq!(canonical.contribution_count(), 2);
    assert_eq!(
        canonical
            .pool_minor_units_as_u128()
            .expect("pool must parse as integer minor units"),
        1000
    );

    let cid = canonical
        .canonical_cid()
        .expect("snapshot CID should compute");
    assert!(cid.starts_with("b3:"));
    assert_eq!(cid.len(), 67);

    let json = serde_json::to_value(&canonical).expect("snapshot should serialize");
    assert_no_authority_keys(&json);

    for forbidden in FORBIDDEN_AUTHORITY_KEYS {
        let mut poisoned = json.clone();
        poisoned
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned).is_err(),
            "RewardSnapshotExport must reject authority field `{forbidden}`"
        );
    }
}
