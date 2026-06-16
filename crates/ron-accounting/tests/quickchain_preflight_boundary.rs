//! RO:WHAT — QuickChain Phase-0 boundary tests for ron-accounting.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Proves accounting artifacts remain derivative.
//! RO:INTERACTS — UsageEvent, HTTP ingest DTO, reward projection, reward snapshot export.
//! RO:INVARIANTS — no balance truth; no wallet/ledger mutation; snapshot CID is not a root.
//! RO:METRICS — none.
//! RO:CONFIG — RewardProjectionConfig only.
//! RO:SECURITY — rejects authority-looking fields at strict usage/ingest boundaries.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_boundary.

use std::collections::BTreeSet;

use ron_accounting::{
    canonical_snapshot_cid,
    http_ingest::{UsageEventsIngestRequest, STORAGE_USAGE_EVENTS_SCHEMA},
    project_reward_snapshot_from_slices, Dimension, LabelSet, MetricKind, RewardContributionExport,
    RewardProjectionConfig, RewardSnapshotExport, SealedSlice, SliceId, SliceMeta, SliceRow,
    UsageEvent, Window,
};
use serde_json::{json, Value};

const B3_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const AUTHORITY_KEYS: &[&str] = &[
    "balance",
    "balance_minor",
    "available_balance",
    "spendable_balance",
    "operation_id",
    "account_sequence",
    "settlement_status",
    "finalized",
    "finality",
    "chain_id",
    "state_root",
    "receipt_root",
    "accounting_root",
    "reward_root",
    "checkpoint",
    "checkpoint_root",
    "checkpoint_hash",
    "anchor",
    "validator",
    "bridge",
    "staking",
    "liquidity",
    "payout_authorized",
    "mint_authorized",
    "ledger_mutation",
    "wallet_mutation",
    "event_class",
];

fn meta() -> SliceMeta {
    SliceMeta::new(
        Window::for_timestamp_ms(300_000, 300).expect("window"),
        300_001,
        None,
        true,
    )
}

fn row(
    tenant: u128,
    service: &str,
    region: &str,
    method: &str,
    route: &str,
    dimension: Dimension,
    value: u64,
) -> SliceRow {
    SliceRow {
        labels: LabelSet::new(tenant, service, region, method, route),
        dimension,
        value,
    }
}

fn slice(seq: u64, rows: Vec<SliceRow>) -> SealedSlice {
    SealedSlice::new(
        SliceId {
            tenant: 1,
            dimension: Dimension::Bytes,
            seq,
        },
        meta(),
        rows,
    )
    .expect("sealed slice")
}

fn assert_b3_cid(value: &str) {
    assert_eq!(value.len(), 67, "b3 CID must be b3:<64 lowercase hex>");
    assert!(
        value.starts_with("b3:"),
        "b3 CID must start with canonical b3: prefix"
    );
    assert!(
        value.as_bytes()[3..]
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "b3 CID must use lowercase hex only"
    );
}

fn assert_no_authority_keys(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let normalized = key.to_ascii_lowercase();

                assert!(
                    !AUTHORITY_KEYS.contains(&normalized.as_str()),
                    "authority-looking key leaked into accounting artifact: {key}"
                );
                assert!(
                    !normalized.ends_with("_root"),
                    "root-looking key leaked into accounting artifact: {key}"
                );

                assert_no_authority_keys(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_authority_keys(item);
            }
        }
        _ => {}
    }
}

#[test]
fn usage_event_dto_rejects_smuggled_quickchain_authority_fields() {
    for field in AUTHORITY_KEYS {
        let mut value = json!({
            "timestamp_ms": 1,
            "tenant": 7,
            "subject": "provider_a",
            "metric_kind": "bytes_stored",
            "value": 128
        });

        value
            .as_object_mut()
            .expect("usage event JSON object")
            .insert((*field).to_string(), json!("client-supplied-authority"));

        let result = serde_json::from_value::<UsageEvent>(value);

        assert!(
            result.is_err(),
            "UsageEvent must reject authority-looking unknown field: {field}"
        );
    }
}

#[test]
fn ingest_request_dto_rejects_smuggled_quickchain_authority_fields() {
    for field in AUTHORITY_KEYS {
        let mut value = json!({
            "schema": STORAGE_USAGE_EVENTS_SCHEMA,
            "cid": B3_A,
            "wallet_txid": "wallet_tx_dev_1",
            "source_service": "svc-storage",
            "events": []
        });

        value
            .as_object_mut()
            .expect("ingest request JSON object")
            .insert((*field).to_string(), json!("client-supplied-authority"));

        let result = serde_json::from_value::<UsageEventsIngestRequest>(value);

        assert!(
            result.is_err(),
            "UsageEventsIngestRequest must reject authority-looking unknown field: {field}"
        );
    }
}

#[test]
fn reward_snapshot_serializes_as_planning_artifact_not_balance_or_root() {
    let snapshot = RewardSnapshotExport::new(
        1,
        "1000",
        vec![
            RewardContributionExport::new("acct_b", 200, 0, 20),
            RewardContributionExport::new("acct_a", 100, 50, 10),
        ],
    )
    .expect("reward snapshot");

    let cid = canonical_snapshot_cid(&snapshot).expect("snapshot artifact cid");
    assert_b3_cid(&cid);

    let value = serde_json::to_value(&snapshot).expect("snapshot JSON");
    assert_no_authority_keys(&value);

    let object = value.as_object().expect("snapshot object");
    let keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = BTreeSet::from(["contributions", "pool_minor_units", "produced_at_millis"]);

    assert_eq!(
        keys, expected,
        "reward snapshot export must remain the svc-rewarder planning shape"
    );
}

#[test]
fn projected_reward_snapshot_is_artifact_cid_not_quickchain_root() {
    let slices = vec![slice(
        1,
        vec![
            row(
                1,
                "svc-storage",
                "local",
                "PUT",
                "/objects",
                Dimension::Bytes,
                100,
            ),
            row(
                1,
                "svc-storage",
                "local",
                "VIEW",
                "/analytics/views",
                Dimension::Requests,
                999,
            ),
        ],
    )];

    let projected =
        project_reward_snapshot_from_slices(42, &RewardProjectionConfig::new("1000"), &slices)
            .expect("projected reward snapshot");

    assert_b3_cid(&projected.snapshot_cid);

    assert_eq!(projected.report.input_slices, 1);
    assert_eq!(projected.report.input_rows, 2);
    assert_eq!(projected.report.ignored_rows, 1);
    assert_eq!(projected.report.bytes_stored, 100);
    assert_eq!(projected.report.bytes_served, 0);
    assert_eq!(projected.report.uptime_seconds, 0);

    assert_eq!(projected.snapshot.contributions.len(), 1);
    assert_eq!(projected.snapshot.contributions[0].bytes_stored, 100);
    assert_eq!(projected.snapshot.contributions[0].bytes_served, 0);
    assert_eq!(projected.snapshot.contributions[0].uptime_seconds, 0);

    let value = serde_json::to_value(&projected).expect("projected snapshot JSON");
    assert_no_authority_keys(&value);

    let object = value.as_object().expect("projected snapshot object");
    assert!(
        object.contains_key("snapshot_cid"),
        "projection may expose an artifact CID"
    );
    assert!(
        !object.keys().any(|key| key.ends_with("_root")),
        "projection must not expose root fields in Phase 0"
    );
}

#[test]
fn raw_engagement_usage_event_is_metering_not_protocol_roc_authority() {
    let event = UsageEvent::new(
        1,
        7,
        "creator_a",
        MetricKind::Custom("views".to_string()),
        1,
    )
    .with_source_service("omnigate")
    .with_route("/analytics/views");

    let counter = event
        .to_counter(&ron_accounting::EventIngestPolicy::default())
        .expect("usage event converts to counter");

    assert_eq!(counter.dimension, Dimension::Requests);
    assert_eq!(counter.labels.method, "VIEWS");
    assert_eq!(counter.labels.route, "/analytics/views");

    let slices = vec![slice(
        1,
        vec![SliceRow {
            labels: counter.labels,
            dimension: counter.dimension,
            value: counter.value,
        }],
    )];

    let result =
        project_reward_snapshot_from_slices(42, &RewardProjectionConfig::new("1000"), &slices);

    assert!(
        result.is_err(),
        "raw analytics-style engagement alone must not become reward projection output"
    );
}
