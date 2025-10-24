//! RO:WHAT — Criterion throughput bench for ron-bus (sync harness + async runtime inside)
//! RO:WHY  — Avoid Criterion's async adapters; drive Tokio ourselves via block_on()
//! RO:INTERACTS — Bus, BusConfig, Event; isolated Tokio runtimes per bench
//! RO:INVARIANTS — bounded channel; no background tasks from the library
//! RO:NOTES — Coarse microbench; for deep dives, use repo-wide harness & baselines.

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_bus::{Bus, BusConfig, Event};
use tokio::runtime::Builder;

/// Busy-work to simulate a "slow" subscriber without OS sleep jitter.
/// Tune `ns` once on your machine if needed.
#[inline]
fn burn_cycles(ns: u64) {
    // The body is intentionally simple integer math; adjust divisor to match ~ns cost.
    let iters = ns / 10;
    let mut x = 0u64;
    for i in 0..iters {
        // LCG-ish mixing; keep it opaque to optimizer.
        x = x
            .wrapping_mul(1664525)
            .wrapping_add(i ^ 1013904223u64);
        std::hint::black_box(x);
    }
}

/// Publish cost with zero subscribers draining.
fn bench_publish_zero_subs(c: &mut Criterion) {
    // Current-thread runtime is enough here.
    let rt = Builder::new_current_thread().enable_all().build().unwrap();

    let mut group = c.benchmark_group("publish_zero_subs");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(8));
    group.noise_threshold(0.02);
    group.significance_level(0.01);

    group.bench_function("publish_zero_subs", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async move {
                // Setup is outside the measured window.
                let bus = Bus::new(BusConfig::new().with_capacity(1024)).unwrap();
                let tx = bus.sender();

                let start = std::time::Instant::now();
                for _ in 0..iters {
                    // Measure only the send loop.
                    let _ = black_box(&tx).send(Event::ConfigUpdated { version: 1 });
                }
                start.elapsed()
            })
        });
    });

    group.finish();
}

/// Publish throughput with 8 draining subscribers (fast consumers).
fn bench_publish_eight_subs(c: &mut Criterion) {
    let rt = Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("publish_eight_subs");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(8));
    group.noise_threshold(0.02);
    group.significance_level(0.01);

    group.bench_function("publish_eight_subs", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async move {
                let bus = Bus::new(BusConfig::new().with_capacity(2048)).unwrap();
                let tx = bus.sender();

                // Spawn fast draining subscribers.
                let mut rxs = Vec::new();
                for _ in 0..8 {
                    rxs.push(bus.subscribe());
                }
                for mut rx in rxs {
                    tokio::spawn(async move {
                        while let Ok(ev) = rx.recv().await {
                            criterion::black_box(ev);
                        }
                    });
                }

                // Measure only the publish loop.
                let start = std::time::Instant::now();
                for i in 0..iters {
                    let _ = tx.send(Event::ConfigUpdated { version: i });
                }
                // Ensure convergence.
                let _ = tx.send(Event::Shutdown);
                start.elapsed()
            })
        });
    });

    group.finish();
}

/// Publish while one subscriber is intentionally slow to induce Lagged(n).
fn bench_publish_with_one_slow_subscriber(c: &mut Criterion) {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("publish_with_one_slow_subscriber");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(8));
    group.noise_threshold(0.02);
    group.significance_level(0.01);

    group.bench_function("publish_with_one_slow_subscriber", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async move {
                let bus = Bus::new(BusConfig::new().with_capacity(64)).unwrap();
                let tx = bus.sender();

                // Slow consumer to create pressure — use CPU burn instead of sleep.
                let mut rx = bus.subscribe();
                let slow = tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(Event::Shutdown) => break,
                            Ok(_) => {
                                // ~0.1ms CPU burn; adjust if you want the same wall time as the old sleep.
                                burn_cycles(100_000);
                            }
                            Err(_) => break,
                        }
                    }
                });

                // Measure the publish loop.
                let start = std::time::Instant::now();
                for i in 0..iters {
                    let _ = tx.send(Event::ConfigUpdated { version: i });
                }
                let _ = tx.send(Event::Shutdown);
                let _ = slow.await;
                start.elapsed()
            })
        });
    });

    group.finish();
}

pub fn criterion_benches(c: &mut Criterion) {
    bench_publish_zero_subs(c);
    bench_publish_eight_subs(c);
    bench_publish_with_one_slow_subscriber(c);
}

criterion_group!(benches, criterion_benches);
criterion_main!(benches);
