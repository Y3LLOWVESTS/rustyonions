//! RO:WHAT — Criterion benchmark for the recorder hot path.
//! RO:WHY — Pillar 12; Concerns: PERF/RES. Detects regressions in usage increment cost.
//! RO:INTERACTS — Recorder, LabelSet, Dimension.
//! RO:INVARIANTS — benchmark does not prove correctness; tests do.
//! RO:METRICS — Criterion output only.
//! RO:CONFIG — default RecorderConfig.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo bench -p ron-accounting --bench record.

use criterion::{criterion_group, criterion_main, Criterion};
use ron_accounting::{Dimension, LabelSet, Recorder};

fn bench_record(c: &mut Criterion) {
    c.bench_function("accounting_record_one", |b| {
        let recorder = Recorder::default();
        let labels = LabelSet::new(1, "svc-storage", "local", "GET", "/objects/123");
        b.iter(|| {
            recorder
                .record(labels.clone(), Dimension::Requests, 1)
                .expect("record benchmark should accept default row");
        });
    });
}

criterion_group!(benches, bench_record);
criterion_main!(benches);
