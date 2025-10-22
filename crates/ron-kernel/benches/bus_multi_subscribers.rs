/*!
RO: benches/bus_multi_subscribers.rs
WHAT: Publish throughput with 0, 1, and 16 draining subscribers.
WHY : Show fan-out cost growth as subscriber count rises.
*/

use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode, BenchmarkId};
use ron_kernel::{Bus, KernelEvent, Metrics};
use tokio::runtime::Builder;

const INNER_PUBLISHES: usize = 10_000;

fn bench_bus_publish(c: &mut Criterion) {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();

    let mut group = c.benchmark_group("bus_publish");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(60);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));

    // 0 subscribers
    group.bench_with_input(BenchmarkId::new("0_subscribers", INNER_PUBLISHES), &(), |b, _| {
        b.iter(|| {
            rt.block_on(async {
                let metrics = Metrics::new(false);
                let bus: Bus<KernelEvent> = metrics.make_bus(1024);
                for i in 0..black_box(INNER_PUBLISHES) {
                    let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                }
            });
        });
    });

    // 1 subscriber (draining)
    group.bench_with_input(BenchmarkId::new("1_subscriber", INNER_PUBLISHES), &(), |b, _| {
        b.iter(|| {
            rt.block_on(async {
                let metrics = Metrics::new(false);
                let bus: Bus<KernelEvent> = metrics.make_bus(1024);

                let mut rx = bus.subscribe();
                let drain = tokio::spawn(async move {
                    while let Ok(_ev) = rx.recv().await {}
                });

                for i in 0..black_box(INNER_PUBLISHES) {
                    let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                }

                drop(bus);
                let _ = drain.await;
            });
        });
    });

    // 16 subscribers (draining)
    group.bench_with_input(BenchmarkId::new("16_subscribers", INNER_PUBLISHES), &(), |b, _| {
        b.iter(|| {
            rt.block_on(async {
                let metrics = Metrics::new(false);
                let bus: Bus<KernelEvent> = metrics.make_bus(2048);

                let mut joins = Vec::new();
                for _ in 0..16 {
                    let mut rx = bus.subscribe();
                    joins.push(tokio::spawn(async move {
                        while let Ok(_ev) = rx.recv().await {}
                    }));
                }

                for i in 0..black_box(INNER_PUBLISHES) {
                    let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                }

                drop(bus);
                for j in joins {
                    let _ = j.await;
                }
            });
        });
    });

    group.finish();
}

criterion_group!(benches, bench_bus_publish);
criterion_main!(benches);
