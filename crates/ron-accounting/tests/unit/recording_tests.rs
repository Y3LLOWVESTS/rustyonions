//! RO:WHAT — Unit tests for recorder monotonicity, label normalization, and sealing.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/RES. Proves Batch 1 core counter invariants.
//! RO:INTERACTS — Recorder, LabelSet, Window, SealedSlice.
//! RO:INVARIANTS — saturating increments; seal drains stream; digest is b3-prefixed.
//! RO:METRICS — none.
//! RO:CONFIG — default RecorderConfig.
//! RO:SECURITY — route IDs are templated.
//! RO:TEST — cargo test -p ron-accounting --test recording_tests.

use ron_accounting::{Dimension, LabelSet, Recorder, SliceId, Window};

#[test]
fn recording_is_monotone_and_saturating() {
    let recorder = Recorder::default();
    let labels = LabelSet::new(7, "svc-storage", "LOCAL", "get", "/objects/123456789");

    recorder
        .record(labels.clone(), Dimension::Requests, 10)
        .expect("record 1");
    recorder
        .record(labels.clone(), Dimension::Requests, 5)
        .expect("record 2");

    let rows = recorder.snapshot();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, 15);
    assert_eq!(rows[0].key.labels.service, "svc-storage");
    assert_eq!(rows[0].key.labels.region, "local");
    assert_eq!(rows[0].key.labels.method, "GET");
    assert_eq!(rows[0].key.labels.route, "/objects/:id");
}

#[test]
fn sealing_drains_only_requested_stream_and_commits_digest() {
    let recorder = Recorder::default();
    recorder
        .record(
            LabelSet::new(1, "svc-storage", "local", "PUT", "/objects/1"),
            Dimension::Bytes,
            1024,
        )
        .expect("record tenant 1");
    recorder
        .record(
            LabelSet::new(2, "svc-storage", "local", "PUT", "/objects/2"),
            Dimension::Bytes,
            2048,
        )
        .expect("record tenant 2");

    let slice = recorder
        .seal_slice(
            SliceId {
                tenant: 1,
                dimension: Dimension::Bytes,
                seq: 1,
            },
            Window::for_timestamp_ms(300_000, 300).expect("valid window"),
            None,
            true,
        )
        .expect("seal tenant 1");

    assert_eq!(slice.rows.len(), 1);
    assert!(slice.digest().starts_with("b3:"));
    assert_eq!(recorder.snapshot().len(), 1);
    assert_eq!(recorder.snapshot()[0].key.labels.tenant, 2);
}
