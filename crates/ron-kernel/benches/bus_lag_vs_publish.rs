/*!
RO: benches/bus_lag_vs_publish.rs
WHAT: Compare publish throughput under no-subscriber vs single slow subscriber.
WHY : Validate non-blocking publish w/ slow receiver (bounded cost, drops on recv side).
NOTE: Group config tuned to avoid "unable to complete samples" warnings on many machines.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use ron_kernel::{Bus, KernelEvent, Metrics};
use std::time::Duration;
use tokio::runtime::Builder;

const INNER_PUBLISHES: usize = 25_000;

fn bench_bus_lag_vs_publish(c: &mut Criterion) {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();

    let mut group = c.benchmark_group("bus_lag_vs_publish");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(60);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));

    // no_subscribers (upper-bound publish cost)
    group.bench_with_input(
        BenchmarkId::new("no_subscribers", INNER_PUBLISHES),
        &(),
        |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let metrics = Metrics::new(false);
                    let bus: Bus<KernelEvent> = metrics.make_bus(1024);
                    for i in 0..black_box(INNER_PUBLISHES) {
                        let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                    }
                });
            });
        },
    );

    // one_slow_subscriber (non-blocking publish should remain bounded)
    group.bench_with_input(
        BenchmarkId::new("one_slow_subscriber", INNER_PUBLISHES),
        &(),
        |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let metrics = Metrics::new(false);
                    let bus: Bus<KernelEvent> = metrics.make_bus(64);

                    let mut rx = bus.subscribe();
                    let slow = tokio::spawn(async move {
                        loop {
                            match rx.recv().await {
                                Ok(_e) => {
                                    tokio::time::sleep(Duration::from_micros(black_box(20))).await
                                }
                                Err(_) => break,
                            }
                        }
                    });

                    for i in 0..black_box(INNER_PUBLISHES) {
                        let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                    }

                    drop(bus);
                    let _ = slow.await;
                });
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench_bus_lag_vs_publish);
criterion_main!(benches);
