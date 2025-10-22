/*!
RO: benches/metrics_encode.rs
WHAT: Measure Prometheus registry gather+encode cost (drift guard).
WHY : Catch accidental cardinality/registry growth; not a throughput contest.
*/

use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode, BenchmarkId};
use prometheus::Encoder;
use ron_kernel::Metrics;

fn bench_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(60);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));

    group.bench_with_input(BenchmarkId::new("gather+encode_text", "registry"), &(), |b, _| {
        b.iter(|| {
            let metrics = Metrics::new(false);
            metrics.set_amnesia(true); // ensure non-empty registry
            let families = (*metrics).registry.gather();
            let mut buf = Vec::new();
            prometheus::TextEncoder::new().encode(&families, &mut buf).unwrap();
            black_box(buf.len());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_metrics);
criterion_main!(benches);
