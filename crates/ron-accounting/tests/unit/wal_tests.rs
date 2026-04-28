//! RO:WHAT — Feature-gated unit tests for the Batch 1 bounded WAL handle.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. WAL must remain bounded and replay-safe.
//! RO:INTERACTS — wal::Wal, wal::segment, wal::replay.
//! RO:INVARIANTS — quota failure is typed; corrupt records are skipped.
//! RO:METRICS — none.
//! RO:CONFIG — wal feature and WalConfig.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo test -p ron-accounting --features wal --test unit.

#[cfg(feature = "wal")]
use ron_accounting::{
    accounting::{SliceMeta, SliceRow},
    config::WalConfig,
    wal::{replay::verified_records, segment::SegmentRecord, Wal},
    Dimension, LabelSet, SealedSlice, SliceId, Window,
};

#[cfg(feature = "wal")]
fn slice() -> SealedSlice {
    let row = SliceRow {
        labels: LabelSet::new(1, "svc-storage", "local", "PUT", "/objects/1"),
        dimension: Dimension::Bytes,
        value: 1024,
    };

    SealedSlice::new(
        SliceId {
            tenant: 1,
            dimension: Dimension::Bytes,
            seq: 1,
        },
        SliceMeta::new(
            Window::for_timestamp_ms(300_000, 300).expect("window"),
            300_001,
            None,
            false,
        ),
        vec![row],
    )
    .expect("slice")
}

#[cfg(feature = "wal")]
#[test]
fn wal_appends_and_drains() {
    let cfg = WalConfig {
        max_entries: 4,
        max_bytes: 1_048_576,
        ..WalConfig::default()
    };

    let wal = Wal::new(cfg).expect("wal");
    wal.append(slice()).expect("append");

    assert_eq!(wal.stats().entries, 1);
    assert_eq!(wal.drain().len(), 1);
    assert_eq!(wal.stats().entries, 0);
}

#[cfg(feature = "wal")]
#[test]
fn replay_skips_corrupt_records() {
    let good = SegmentRecord::new(vec![1, 2, 3]);
    let mut bad = SegmentRecord::new(vec![9, 9, 9]);
    bad.bytes.push(0);

    let verified = verified_records(&[good.clone(), bad]);

    assert_eq!(verified, vec![good]);
}

#[cfg(not(feature = "wal"))]
#[test]
fn wal_feature_disabled_placeholder() {
    let marker = "wal-disabled";
    assert_eq!(marker.split('-').count(), 2);
}
