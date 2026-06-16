//! RO:WHAT — QuickChain Phase-0 ingest poisoning tests for ron-accounting.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Proves ingest rejects client-supplied authority.
//! RO:INTERACTS — UsageEventsIngestRequest, UsageEvent, EventIngestPolicy.
//! RO:INVARIANTS — idempotency is header retry safety; request body cannot smuggle roots/finality.
//! RO:METRICS — none.
//! RO:CONFIG — MAX_USAGE_EVENTS_PER_REQUEST.
//! RO:SECURITY — rejects malformed b3, schema drift, body idempotency, and authority-looking fields.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_ingest_poisoning.

use ron_accounting::{
    http_ingest::{
        is_b3_cid, UsageEventsIngestRequest, MAX_USAGE_EVENTS_PER_REQUEST,
        STORAGE_USAGE_EVENTS_SCHEMA,
    },
    MetricKind, UsageEvent,
};
use serde_json::json;

const B3_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn valid_event() -> UsageEvent {
    UsageEvent::new(1_000, 7, "provider-a", MetricKind::BytesStored, 1)
        .with_source_service("svc-storage")
        .with_region("local")
}

fn valid_request() -> UsageEventsIngestRequest {
    UsageEventsIngestRequest {
        schema: STORAGE_USAGE_EVENTS_SCHEMA.to_string(),
        cid: B3_A.to_string(),
        wallet_txid: "wallet-tx-dev-1".to_string(),
        source_service: "svc-storage".to_string(),
        events: vec![valid_event()],
    }
}

#[test]
fn strict_ingest_request_validates_minimal_storage_export() {
    let request = valid_request();

    request
        .validate_for_ingest()
        .expect("valid storage usage export should pass");

    assert!(is_b3_cid(&request.cid));
}

#[test]
fn ingest_request_rejects_noncanonical_cid_shapes_before_dedupe_authority() {
    for cid in [
        "",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "b3:",
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "b3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "b3:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
    ] {
        let mut request = valid_request();
        request.cid = cid.to_string();

        assert!(
            request.validate_for_ingest().is_err(),
            "malformed cid must reject before ingest state is consumed: {cid:?}"
        );
    }
}

#[test]
fn ingest_request_rejects_schema_drift_and_empty_required_fields() {
    let mut wrong_schema = valid_request();
    wrong_schema.schema = "quickchain.settlement.v1".to_string();
    assert!(wrong_schema.validate_for_ingest().is_err());

    let mut empty_wallet_txid = valid_request();
    empty_wallet_txid.wallet_txid = "   ".to_string();
    assert!(empty_wallet_txid.validate_for_ingest().is_err());

    let mut empty_source = valid_request();
    empty_source.source_service = "   ".to_string();
    assert!(empty_source.validate_for_ingest().is_err());
}

#[test]
fn ingest_request_rejects_oversized_batches() {
    let mut request = valid_request();
    request.events = vec![valid_event(); MAX_USAGE_EVENTS_PER_REQUEST + 1];

    assert!(
        request.validate_for_ingest().is_err(),
        "oversized ingest batches must reject deterministically"
    );
}

#[test]
fn ingest_request_rejects_invalid_nested_usage_events_before_recording() {
    let mut zero_timestamp = valid_request();
    zero_timestamp.events = vec![UsageEvent::new(
        0,
        7,
        "provider-a",
        MetricKind::BytesStored,
        1,
    )];
    assert!(zero_timestamp.validate_for_ingest().is_err());

    let mut empty_subject = valid_request();
    empty_subject.events = vec![UsageEvent::new(1_000, 7, "   ", MetricKind::BytesStored, 1)];
    assert!(empty_subject.validate_for_ingest().is_err());
}

#[test]
fn ingest_request_body_cannot_smuggle_idempotency_or_operation_identity() {
    let top_level_idempotency = json!({
        "schema": STORAGE_USAGE_EVENTS_SCHEMA,
        "cid": B3_A,
        "wallet_txid": "wallet-tx-dev-1",
        "source_service": "svc-storage",
        "idempotency_key": "retry-key-is-header-only",
        "events": []
    });

    assert!(
        serde_json::from_value::<UsageEventsIngestRequest>(top_level_idempotency).is_err(),
        "idempotency_key must remain HTTP retry metadata, not body authority"
    );

    let nested_operation_identity = json!({
        "schema": STORAGE_USAGE_EVENTS_SCHEMA,
        "cid": B3_A,
        "wallet_txid": "wallet-tx-dev-1",
        "source_service": "svc-storage",
        "events": [{
            "timestamp_ms": 1,
            "tenant": 7,
            "subject": "provider-a",
            "metric_kind": "bytes_stored",
            "value": 1,
            "operation_id": "client-supplied-ledger-operation"
        }]
    });

    assert!(
        serde_json::from_value::<UsageEventsIngestRequest>(nested_operation_identity).is_err(),
        "operation_id must not be accepted from ron-accounting ingest bodies"
    );
}

#[test]
fn ingest_request_rejects_quickchain_root_and_finality_poison_fields() {
    for field in [
        "state_root",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "checkpoint_root",
        "validator",
        "finalized",
        "finality",
        "settlement_status",
        "bridge",
        "staking",
        "liquidity",
        "payout_authorized",
        "ledger_mutation",
        "wallet_mutation",
    ] {
        let mut value = json!({
            "schema": STORAGE_USAGE_EVENTS_SCHEMA,
            "cid": B3_A,
            "wallet_txid": "wallet-tx-dev-1",
            "source_service": "svc-storage",
            "events": []
        });

        value
            .as_object_mut()
            .expect("request object")
            .insert(field.to_string(), json!("client-smuggled-authority"));

        assert!(
            serde_json::from_value::<UsageEventsIngestRequest>(value).is_err(),
            "ingest body must reject authority-looking unknown field: {field}"
        );
    }
}
