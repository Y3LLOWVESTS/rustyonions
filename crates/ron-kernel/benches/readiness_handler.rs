/*!
RO: benches/readiness_handler.rs
WHAT: Measure axum handler overhead for the readiness gate.
WHY : Ensure /readyz is microseconds-fast in both states.
*/

use axum::http::StatusCode;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use ron_kernel::metrics::health::HealthState;
use ron_kernel::metrics::readiness::{readyz_handler, Readiness};
use std::time::Duration;
use tokio::runtime::Builder;

fn bench_readyz(c: &mut Criterion) {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();

    let mut group = c.benchmark_group("readyz");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(60);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));

    // not_ready
    group.bench_with_input(BenchmarkId::new("not_ready", "handler()"), &(), |b, _| {
        b.iter(|| {
            rt.block_on(async {
                let health = HealthState::new();
                let ready = Readiness::new(health.clone());
                let resp = readyz_handler(ready).await;
                assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
            });
        });
    });

    // ready
    group.bench_with_input(BenchmarkId::new("ready", "handler()"), &(), |b, _| {
        b.iter(|| {
            rt.block_on(async {
                let health = HealthState::new();
                let ready = Readiness::new(health.clone());
                ready.set_config_loaded(true);
                health.set("kernel", true);
                let resp = readyz_handler(ready).await;
                assert_eq!(resp.status(), StatusCode::OK);
            });
        });
    });

    group.finish();
}

criterion_group!(benches, bench_readyz);
criterion_main!(benches);
