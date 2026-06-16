//! RO:WHAT — QuickChain Phase-0 event-class boundary tests for ron-accounting.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Prevents raw engagement/event-class drift from becoming ROC authority.
//! RO:INTERACTS — UsageEvent, UsageEventsIngestRequest, MetricKind, reward projection.
//! RO:INVARIANTS — event classes are doctrine here, not wallet/ledger authority; raw engagement cannot allocate ROC.
//! RO:METRICS — none.
//! RO:CONFIG — EventIngestPolicy only.
//! RO:SECURITY — rejects event_class/body authority smuggling and proves attribution does not imply payout authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_event_class_boundary.

use ron_accounting::{
    http_ingest::{UsageEventsIngestRequest, STORAGE_USAGE_EVENTS_SCHEMA},
    project_reward_snapshot_from_slices, Dimension, EventIngestPolicy, EventSubjectMode, LabelSet,
    MetricKind, RewardProjectionConfig, SealedSlice, SliceId, SliceMeta, SliceRow, UsageEvent,
    Window,
};
use serde_json::{json, Value};

const B3_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn meta() -> SliceMeta {
    SliceMeta::new(
        Window::for_timestamp_ms(300_000, 300).expect("window"),
        300_001,
        None,
        true,
    )
}

fn slice(seq: u64, rows: Vec<SliceRow>) -> SealedSlice {
    SealedSlice::new(
        SliceId {
            tenant: 7,
            dimension: Dimension::Bytes,
            seq,
        },
        meta(),
        rows,
    )
    .expect("sealed slice")
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

fn row_from_event(event: UsageEvent) -> SliceRow {
    let counter = event
        .to_counter(&EventIngestPolicy::default())
        .expect("event should convert to counter");

    SliceRow {
        labels: counter.labels,
        dimension: counter.dimension,
        value: counter.value,
    }
}

fn project_one(row: SliceRow) -> ron_accounting::Result<ron_accounting::ProjectedRewardSnapshot> {
    project_reward_snapshot_from_slices(
        42,
        &RewardProjectionConfig::new("1000"),
        &[slice(1, vec![row])],
    )
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).expect("JSON string")
}

#[test]
fn usage_event_rejects_client_supplied_event_class_and_reward_authority() {
    for field in [
        "event_class",
        "reward_class",
        "proof_eligible",
        "ad_budgeted",
        "analytics_only",
        "reward_weight",
        "payout_authorized",
        "protocol_roc",
        "ledger_mutation",
        "wallet_mutation",
    ] {
        let mut value = json!({
            "timestamp_ms": 1,
            "tenant": 7,
            "subject": "creator-a",
            "metric_kind": {
                "custom": "views"
            },
            "value": 1,
            "source_service": "omnigate",
            "route": "/analytics/views"
        });

        value
            .as_object_mut()
            .expect("usage event object")
            .insert(field.to_string(), json!("client-smuggled-authority"));

        assert!(
            serde_json::from_value::<UsageEvent>(value).is_err(),
            "UsageEvent must reject client-supplied event-class/authority field: {field}"
        );
    }
}

#[test]
fn ingest_request_rejects_top_level_and_nested_event_class_fields() {
    let top_level = json!({
        "schema": STORAGE_USAGE_EVENTS_SCHEMA,
        "cid": B3_A,
        "wallet_txid": "wallet-tx-dev-1",
        "source_service": "svc-storage",
        "event_class": "proof_eligible",
        "events": []
    });

    assert!(
        serde_json::from_value::<UsageEventsIngestRequest>(top_level).is_err(),
        "ingest request body must not accept top-level event_class"
    );

    let nested = json!({
        "schema": STORAGE_USAGE_EVENTS_SCHEMA,
        "cid": B3_A,
        "wallet_txid": "wallet-tx-dev-1",
        "source_service": "svc-storage",
        "events": [{
            "timestamp_ms": 1,
            "tenant": 7,
            "subject": "creator-a",
            "metric_kind": {
                "custom": "views"
            },
            "value": 1,
            "source_service": "omnigate",
            "route": "/analytics/views",
            "event_class": "analytics_only"
        }]
    });

    assert!(
        serde_json::from_value::<UsageEventsIngestRequest>(nested).is_err(),
        "nested usage event must reject event_class drift"
    );
}

#[test]
fn only_explicit_reward_projection_metric_kinds_are_currently_eligible() {
    assert!(MetricKind::BytesStored.is_reward_projection_input());
    assert!(MetricKind::BytesServed.is_reward_projection_input());
    assert!(MetricKind::UptimeSeconds.is_reward_projection_input());

    for metric in [
        MetricKind::RequestOk,
        MetricKind::PinSeconds,
        MetricKind::CpuUnits,
        MetricKind::Custom("views".to_string()),
        MetricKind::Custom("likes".to_string()),
        MetricKind::Custom("comments".to_string()),
        MetricKind::Custom("impressions".to_string()),
        MetricKind::Custom("watch_seconds".to_string()),
        MetricKind::Custom("ad_impression".to_string()),
    ] {
        assert!(
            !metric.is_reward_projection_input(),
            "metric must not be reward-projection eligible by doctrine: {metric:?}"
        );
    }
}

#[test]
fn reward_eligible_metering_events_project_but_remain_planning_only() {
    let rows = vec![
        row_from_event(
            UsageEvent::new(1_000, 7, "provider-a", MetricKind::BytesStored, 100)
                .with_source_service("svc-storage"),
        ),
        row_from_event(
            UsageEvent::new(1_001, 7, "provider-a", MetricKind::BytesServed, 40)
                .with_source_service("svc-gateway"),
        ),
        row_from_event(
            UsageEvent::new(1_002, 7, "provider-a", MetricKind::UptimeSeconds, 30)
                .with_source_service("svc-overlay"),
        ),
    ];

    let projected = project_reward_snapshot_from_slices(
        42,
        &RewardProjectionConfig::new("1000"),
        &[slice(1, rows)],
    )
    .expect("reward-eligible metering events should project");

    assert_eq!(projected.report.projected_accounts, 1);
    assert_eq!(projected.report.ignored_rows, 0);
    assert_eq!(projected.report.bytes_stored, 100);
    assert_eq!(projected.report.bytes_served, 40);
    assert_eq!(projected.report.uptime_seconds, 30);

    let value = serde_json::to_value(&projected).expect("projected JSON");
    let body = json_string(&value);

    for forbidden in [
        "event_class",
        "payout_authorized",
        "ledger_mutation",
        "wallet_mutation",
        "state_root",
        "reward_root",
        "checkpoint_root",
        "finality",
        "validator",
    ] {
        assert!(
            !body.contains(forbidden),
            "projection artifact must not contain authority/event-class field: {forbidden}"
        );
    }
}

#[test]
fn analytics_engagement_metrics_remain_requests_and_do_not_project_rewards() {
    for (name, route) in [
        ("views", "/analytics/views"),
        ("likes", "/analytics/likes"),
        ("comments", "/analytics/comments"),
        ("impressions", "/analytics/impressions"),
        ("watch_seconds", "/analytics/watch-seconds"),
    ] {
        let event = UsageEvent::new(
            1_000,
            7,
            "creator-a",
            MetricKind::Custom(name.to_string()),
            1,
        )
        .with_source_service("omnigate")
        .with_route(route);

        let counter = event
            .to_counter(&EventIngestPolicy::default())
            .expect("custom event should convert to counter");

        assert_eq!(counter.dimension, Dimension::Requests);
        assert_eq!(
            counter.labels.route,
            ron_accounting::normalize::normalize_route(route)
        );

        assert!(
            project_one(SliceRow {
                labels: counter.labels,
                dimension: counter.dimension,
                value: counter.value,
            })
            .is_err(),
            "analytics metric must not project into reward snapshot: {name}"
        );
    }
}

#[test]
fn ad_budgeted_style_events_do_not_become_protocol_reward_authority() {
    for (metric, route) in [
        ("ad_impression", "/ads/impression"),
        ("ad_click", "/ads/click"),
        ("sponsor_view", "/sponsor/view"),
        ("campaign_view", "/campaign/view"),
    ] {
        let row = row_from_event(
            UsageEvent::new(
                1_000,
                7,
                "creator-a",
                MetricKind::Custom(metric.to_string()),
                1,
            )
            .with_source_service("omnigate")
            .with_route(route),
        );

        assert_eq!(row.dimension, Dimension::Requests);

        assert!(
            project_one(row).is_err(),
            "ad-budgeted style event must not allocate protocol reward directly: {metric}"
        );
    }
}

#[test]
fn pin_cpu_and_plain_request_metrics_do_not_project_rewards() {
    for event in [
        UsageEvent::new(1_000, 7, "provider-a", MetricKind::PinSeconds, 60)
            .with_source_service("svc-storage"),
        UsageEvent::new(1_001, 7, "provider-a", MetricKind::CpuUnits, 10)
            .with_source_service("svc-worker"),
        UsageEvent::new(1_002, 7, "provider-a", MetricKind::RequestOk, 1)
            .with_source_service("svc-gateway")
            .with_route("/site/view"),
    ] {
        let metric = event.metric_kind.clone();
        let row = row_from_event(event);

        assert!(
            project_one(row).is_err(),
            "non-reward metric must not produce reward projection output: {metric:?}"
        );
    }
}

#[test]
fn attribution_mode_does_not_turn_raw_engagement_into_reward_authority() {
    let policy = EventIngestPolicy {
        subject_mode: EventSubjectMode::SourceService,
        ..EventIngestPolicy::default()
    };

    let event = UsageEvent::new(
        1_000,
        7,
        "creator-a",
        MetricKind::Custom("views".to_string()),
        10,
    )
    .with_source_service("omnigate")
    .with_route("/analytics/views");

    let counter = event
        .to_counter(&policy)
        .expect("event should convert with source attribution");

    assert_eq!(counter.dimension, Dimension::Requests);
    assert_eq!(counter.labels.service, "omnigate");

    assert!(
        project_one(SliceRow {
            labels: counter.labels,
            dimension: counter.dimension,
            value: counter.value,
        })
        .is_err(),
        "changing attribution mode must not turn raw engagement into protocol payout authority"
    );
}

#[test]
fn route_wording_cannot_smuggle_reward_authority_into_request_metrics() {
    for route in [
        "/analytics/reward",
        "/analytics/payout",
        "/analytics/claim",
        "/engagement/proof-eligible",
        "/ad-budgeted/reward",
    ] {
        let request_row = row(
            7,
            "creator-a",
            "local",
            "VIEW",
            route,
            Dimension::Requests,
            1,
        );

        assert!(
            project_one(request_row).is_err(),
            "reward-looking request route must not project without an eligible metric: {route}"
        );
    }
}
