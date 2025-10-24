//! RO:WHAT — Simple recv-side latency microbench.
//! RO:WHY  — Complements throughput bench to sanity check service time for subscribers.
//! RO:INTERACTS — Bus, BusConfig, Event.
//! RO:NOTES — Coarse; for deep dives, use project-wide harness.

use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use ron_bus::{Bus, BusConfig, Event};
use tokio::runtime::Builder;

fn recv_latency_one_publisher(c: &mut Criterion) {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("recv_latency_one_publisher");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(8));
    group.noise_threshold(0.02);
    group.significance_level(0.01);

    group.bench_function("recv_latency_one_publisher", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async move {
                let bus = Bus::new(BusConfig::new().with_capacity(1024)).unwrap();
                let tx = bus.sender();
                let mut rx = bus.subscribe();

                let pubber = tokio::spawn({
                    let tx = tx.clone();
                    async move {
                        for i in 0..iters {
                            let _ = tx.send(Event::ConfigUpdated { version: i });
                        }
                        let _ = tx.send(Event::Shutdown);
                    }
                });

                // Measure the time to consume `iters` events
                let start = std::time::Instant::now();
                let mut seen = 0u64;
                loop {
                    match rx.recv().await {
                        Ok(Event::ConfigUpdated { .. }) => {
                            seen += 1;
                            if seen == iters {
                                break;
                            }
                        }
                        Ok(Event::Shutdown) => break,
                        Err(_) => break,
                        _ => {}
                    }
                }
                let _ = pubber.await;
                start.elapsed()
            })
        });
    });

    group.finish();
}

criterion_group!(benches, recv_latency_one_publisher);
criterion_main!(benches);
