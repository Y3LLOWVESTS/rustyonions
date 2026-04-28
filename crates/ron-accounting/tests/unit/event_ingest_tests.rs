//! RO:WHAT — Unit tests for usage-event ingest into the Recorder.
//! RO:WHY — Pillar 12; Concerns: ECON/DX. Proves WEB3 usage events become deterministic counters.
//! RO:INTERACTS — UsageEvent, MetricKind, EventIngestPolicy, Recorder.
//! RO:INVARIANTS — zero events are no-ops; subject attribution is deterministic; bad schema rejected.
//! RO:METRICS — none.
//! RO:CONFIG — EventIngestPolicy.
//! RO:SECURITY — subject/source/route are normalized.
//! RO:TEST — cargo test -p ron-accounting --test unit.

use ron_accounting::{
    record_usage_event, record_usage_events, Dimension, EventIngestPolicy, EventSubjectMode,
    MetricKind, Recorder, UsageEvent,
};

#[test]
fn usage_event_records_bytes_stored_counter() {
    let recorder = Recorder::default();
    let event = UsageEvent::new(1_000, 7, "Provider-A", MetricKind::BytesStored, 1_024)
        .with_region("us-central")
        .with_source_service("svc-storage");

    let recorded =
        record_usage_event(&recorder, &event, &EventIngestPolicy::default()).expect("record");

    assert!(recorded);

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.labels.tenant, 7);
    assert_eq!(rows[0].key.labels.service, "provider-a");
    assert_eq!(rows[0].key.labels.region, "us-central");
    assert_eq!(rows[0].key.labels.method, "PUT");
    assert_eq!(rows[0].key.labels.route, "/usage/bytes-stored");
    assert_eq!(rows[0].key.dimension, Dimension::Bytes);
    assert_eq!(rows[0].value, 1_024);
}

#[test]
fn usage_event_records_bytes_served_as_get_bytes() {
    let recorder = Recorder::default();
    let event = UsageEvent::new(1_000, 7, "provider-a", MetricKind::BytesServed, 512)
        .with_route("/objects/b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    record_usage_event(&recorder, &event, &EventIngestPolicy::default()).expect("record");

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.labels.method, "GET");
    assert_eq!(rows[0].key.labels.route, "/objects/:cid");
    assert_eq!(rows[0].key.dimension, Dimension::Bytes);
    assert_eq!(rows[0].value, 512);
}

#[test]
fn usage_event_can_attribute_to_source_service() {
    let recorder = Recorder::default();
    let policy = EventIngestPolicy {
        subject_mode: EventSubjectMode::SourceService,
        ..EventIngestPolicy::default()
    };

    let event = UsageEvent::new(1_000, 1, "provider-a", MetricKind::RequestOk, 1)
        .with_source_service("svc-gateway");

    record_usage_event(&recorder, &event, &policy).expect("record");

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.labels.service, "svc-gateway");
    assert_eq!(rows[0].key.labels.method, "REQ_OK");
    assert_eq!(rows[0].key.dimension, Dimension::Requests);
}

#[test]
fn zero_value_event_is_noop() {
    let recorder = Recorder::default();
    let event = UsageEvent::new(1_000, 1, "provider-a", MetricKind::BytesStored, 0);

    let recorded =
        record_usage_event(&recorder, &event, &EventIngestPolicy::default()).expect("record");

    assert!(!recorded);
    assert!(recorder.snapshot().is_empty());
}

#[test]
fn batch_ingest_reports_recorded_and_zero_skipped() {
    let recorder = Recorder::default();
    let events = vec![
        UsageEvent::new(1_000, 1, "provider-a", MetricKind::BytesStored, 10),
        UsageEvent::new(1_001, 1, "provider-a", MetricKind::BytesStored, 0),
        UsageEvent::new(1_002, 1, "provider-a", MetricKind::UptimeSeconds, 60),
    ];

    let report =
        record_usage_events(&recorder, &events, &EventIngestPolicy::default()).expect("batch");

    assert_eq!(report.inspected, 3);
    assert_eq!(report.recorded, 2);
    assert_eq!(report.skipped_zero, 1);
    assert_eq!(recorder.snapshot().len(), 2);
}

#[test]
fn usage_event_rejects_empty_subject() {
    let recorder = Recorder::default();
    let event = UsageEvent::new(1_000, 1, "   ", MetricKind::BytesStored, 10);

    assert!(record_usage_event(&recorder, &event, &EventIngestPolicy::default()).is_err());
}

#[test]
fn usage_event_rejects_zero_timestamp() {
    let recorder = Recorder::default();
    let event = UsageEvent::new(0, 1, "provider-a", MetricKind::BytesStored, 10);

    assert!(record_usage_event(&recorder, &event, &EventIngestPolicy::default()).is_err());
}

#[test]
fn custom_metric_kind_is_normalized_to_requests_dimension() {
    let recorder = Recorder::default();
    let event = UsageEvent::new(
        1_000,
        1,
        "provider-a",
        MetricKind::Custom("cache-hit".to_string()),
        3,
    );

    record_usage_event(&recorder, &event, &EventIngestPolicy::default()).expect("record");

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.dimension, Dimension::Requests);
    assert_eq!(rows[0].key.labels.method, "CACHE_HIT");
    assert_eq!(rows[0].key.labels.route, "/usage/custom/cache-hit");
}
