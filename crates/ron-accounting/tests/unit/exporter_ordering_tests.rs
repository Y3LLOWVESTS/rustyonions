//! RO:WHAT — Unit tests for ordered exporter lanes, router leasing, and ACK cache behavior.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Ensures no N+1 before N export drift.
//! RO:INTERACTS — ExportLane, ExporterRouter, AckLru, SealedSlice.
//! RO:INVARIANTS — stream matching; contiguous sequence acceptance; bounded queues; retry-safe lease.
//! RO:METRICS — none.
//! RO:CONFIG — lane cap set directly in tests.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo test -p ron-accounting --test unit.

use ron_accounting::{
    accounting::{SliceMeta, SliceRow},
    config::schema::ExporterConfig,
    exporter::{AckKey, AckLru, ExportLane, ExportRetryPolicy, ExporterRouter, StreamKey},
    Dimension, LabelSet, SealedSlice, SliceId, Window,
};

fn slice(seq: u64) -> SealedSlice {
    slice_for(1, Dimension::Requests, seq)
}

fn slice_for(tenant: u128, dimension: Dimension, seq: u64) -> SealedSlice {
    let id = SliceId {
        tenant,
        dimension,
        seq,
    };
    let meta = SliceMeta::new(
        Window::for_timestamp_ms(300_000, 300).expect("window"),
        300_001,
        None,
        true,
    );
    let row = SliceRow {
        labels: LabelSet::new(tenant, "svc-gateway", "local", "GET", "/v1/ping"),
        dimension,
        value: 1,
    };
    SealedSlice::new(id, meta, vec![row]).expect("slice")
}

#[test]
fn lane_rejects_sequence_gap() {
    let key = StreamKey::new(1, Dimension::Requests);
    let mut lane = ExportLane::new(key, 8, 1);

    lane.push(slice(1)).expect("seq1 ok");
    assert!(lane.push(slice(3)).is_err());
    lane.push(slice(2)).expect("seq2 ok after gap rejection");
    assert_eq!(lane.len(), 2);
}

#[test]
fn router_keeps_deterministic_backlog() {
    let mut router = ExporterRouter::new(8);
    router.route(slice(1)).expect("route seq1");
    router.route(slice(2)).expect("route seq2");
    assert_eq!(router.backlog_len(), 2);

    let first = router.pop_next().expect("first lease");
    assert_eq!(first.id.seq, 1);
    assert_eq!(router.backlog_len(), 2);
    assert_eq!(router.inflight_len(), 1);
}

#[test]
fn lane_lease_ack_and_nack_are_retry_safe() {
    let key = StreamKey::new(1, Dimension::Requests);
    let mut lane = ExportLane::new(key, 8, 1);

    lane.push(slice(1)).expect("seq1");
    lane.push(slice(2)).expect("seq2");

    let leased = lane.lease_next().expect("lease seq1");
    assert_eq!(leased.id.seq, 1);
    assert_eq!(lane.inflight_seq(), Some(1));
    assert_eq!(lane.len(), 2);

    assert!(lane.lease_next().is_none());

    lane.nack(1).expect("nack seq1");
    assert_eq!(lane.inflight_seq(), None);

    let retry = lane.lease_next().expect("retry seq1");
    assert_eq!(retry.id.seq, 1);

    lane.ack(1).expect("ack seq1");
    assert_eq!(lane.next_seq(), 2);
    assert_eq!(lane.len(), 1);

    let second = lane.lease_next().expect("lease seq2");
    assert_eq!(second.id.seq, 2);
}

#[test]
fn router_retains_empty_lane_sequence_continuity() {
    let mut router = ExporterRouter::new(8);
    let key = StreamKey::new(1, Dimension::Requests);

    router.route(slice(1)).expect("route seq1");
    let leased = router.lease_next().expect("lease seq1");
    assert_eq!(leased.id.seq, 1);
    router.ack(key, 1).expect("ack seq1");

    assert_eq!(router.backlog_len(), 0);
    assert_eq!(router.lane_count(), 1);

    assert!(router.route(slice(3)).is_err());
    router.route(slice(2)).expect("seq2 still required");
}

#[test]
fn router_can_lease_other_stream_when_one_stream_is_inflight() {
    let mut router = ExporterRouter::new(8);

    router
        .route(slice_for(1, Dimension::Requests, 1))
        .expect("tenant1");
    router
        .route(slice_for(2, Dimension::Requests, 1))
        .expect("tenant2");

    let first = router.lease_next().expect("first lease");
    let second = router.lease_next().expect("second stream lease");

    assert_ne!(first.id.tenant, second.id.tenant);
    assert_eq!(router.inflight_len(), 2);
}

#[test]
fn ack_lru_dedupes_and_evicts_oldest() {
    let first = slice(1);
    let second = slice(2);
    let third = slice(3);

    let mut lru = AckLru::new(2);

    assert!(lru.insert_slice(&first));
    assert!(!lru.insert(AckKey::from_slice(&first)));
    assert!(lru.insert_slice(&second));
    assert_eq!(lru.len(), 2);

    assert!(lru.insert_slice(&third));
    assert_eq!(lru.len(), 2);
    assert!(!lru.contains_slice(&first));
    assert!(lru.contains_slice(&second));
    assert!(lru.contains_slice(&third));
}

#[test]
fn retry_policy_caps_exponential_backoff() {
    let cfg = ExporterConfig {
        backoff_base_ms: 10,
        backoff_cap_ms: 25,
        ..ExporterConfig::default()
    };
    let policy = ExportRetryPolicy::from_config(&cfg, 4);

    assert_eq!(policy.retry_delay_ms(0), None);
    assert_eq!(policy.retry_delay_ms(1), Some(10));
    assert_eq!(policy.retry_delay_ms(2), Some(20));
    assert_eq!(policy.retry_delay_ms(3), Some(25));
    assert_eq!(policy.retry_delay_ms(4), None);
}
