//! RO:WHAT — Criterion benchmark for sealing one accounting slice.
//! RO:WHY — Pillar 12; Concerns: PERF/ECON. Keeps digest/encoding overhead visible.
//! RO:INTERACTS — Recorder, Window, SliceId, SealedSlice.
//! RO:INVARIANTS — benchmark reseeds data each iteration; correctness remains in tests.
//! RO:METRICS — Criterion output only.
//! RO:CONFIG — default RecorderConfig, 5m window.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo bench -p ron-accounting --bench seal.

use criterion::{criterion_group, criterion_main, Criterion};
use ron_accounting::{Dimension, LabelSet, Recorder, SliceId, Window};

fn bench_seal(c: &mut Criterion) {
    c.bench_function("accounting_seal_one_slice", |b| {
        b.iter(|| {
            let recorder = Recorder::default();
            recorder
                .record(
                    LabelSet::new(1, "svc-storage", "local", "PUT", "/objects/123"),
                    Dimension::Bytes,
                    1024,
                )
                .expect("record should succeed");
            recorder
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
                .expect("seal should succeed");
        });
    });
}

criterion_group!(benches, bench_seal);
criterion_main!(benches);
