//! RO:WHAT — Receive-side latency microbench (deterministic runtime).
//! RO:WHY  — Reduce scheduler variance while preserving original iter_custom style.
//! RO:INTERACTS — Bus, BusConfig, Event.
//! RO:NOTES — Single-thread runtime; measures time to recv `iters` events; clean shutdown.

use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use ron_bus::{Bus, BusConfig, Event};
use tokio::runtime::Builder;

fn recv_latency_one_publisher(c: &mut Criterion) {
    // Use a single-thread runtime to reduce scheduler jitter vs multi-thread.
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio rt");

    let mut group = c.benchmark_group("recv_latency_one_publisher");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(8));
    group.noise_threshold(0.02);
    group.significance_level(0.01);

    group.bench_function("recv_latency_one_publisher", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async move {
                // Fresh bus per measurement to avoid cross-iter state bleed.
                let bus = Bus::new(BusConfig::new().with_capacity(1024)).unwrap();
                let tx = bus.sender();
                let mut rx = bus.subscribe();

                // Publisher: send exactly `iters` events, then Shutdown.
                let pubber = tokio::spawn({
                    let tx = tx.clone();
                    async move {
                        for i in 0..iters {
                            // Small POD payload path; minimal branching.
                            let _ = tx.send(Event::ConfigUpdated { version: i });
                        }
                        let _ = tx.send(Event::Shutdown);
                        // Drop to ensure receivers can observe close if needed.
                        drop(tx);
                    }
                });

                // Measure time to consume exactly `iters` relevant events.
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
                        Ok(Event::Shutdown) => break, // belt-and-suspenders
                        Err(_) => break,              // sender dropped
                        _ => {}
                    }
                }
                // Ensure publisher task is done before returning elapsed time.
                let _ = pubber.await;
                start.elapsed()
            })
        });
    });

    group.finish();
}

criterion_group!(benches, recv_latency_one_publisher);
criterion_main!(benches);
