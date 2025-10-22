//! RO:WHAT
//! Bench the bounded Bus<T> publish paths with and without batching.
//!
//! RO:WHY
//! A2 (bus_batch) should reduce notify/wake amplification and fence costs by
//! batching multiple publishes into one sweep with <=1 notify. This bench *must*
//! be env-configurable so we can sweep fanout/cap/burst from the shell.
//!
//! RO:INTERACTS
//! - Uses `ron_kernel::bus::bounded::Bus` directly.
//! - Feature flag `bus_batch` enables the batch path.
//! - Criterion for timing.
//!
//! RO:INVARIANTS
//! - Public API untouched (bench only).
//! - No panics under capacity pressure; drops are handled inside Bus.
//! - Single-threaded Tokio runtime for stability.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::env;
use std::time::Duration;

// We bench the kernel Bus<T> directly.
use ron_kernel::bus::bounded::Bus;

// A small POD event for hot-path measurement.
#[derive(Clone)]
#[allow(dead_code)]
struct Ev(u64);

fn getenv<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)
}

// Drain helper: non-blocking, like our EdgeReceiver::try_recv_now_or_never().
fn drain_now<T: Clone + Send + 'static>(rx: &mut tokio::sync::broadcast::Receiver<T>) -> usize {
    use tokio::sync::broadcast::error::TryRecvError::*;
    let mut n = 0usize;
    loop {
        match rx.try_recv() {
            Ok(_) => {
                n += 1;
            }
            Err(Empty) => break,
            Err(Lagged(_)) => continue,
            Err(Closed) => break,
        }
    }
    n
}

fn bench_bus_batch(c: &mut Criterion) {
    // Env-driven configuration (defaults match previous behavior).
    let subs = getenv::<usize>("RON_BENCH_FANOUT", 4);
    let cap = getenv::<usize>("RON_BENCH_CAP", 64);
    let burst = getenv::<usize>("RON_BENCH_BURST", 128);

    eprintln!(
        "[bench cfg] subs={}, cap={}, burst={}  (set RON_BENCH_FANOUT/CAP/BURST to override)",
        subs, cap, burst
    );

    // Single-threaded runtime for stable numbers.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    let mut g = c.benchmark_group("bus_batch");

    // --- Single publish baseline ---------------------------------------------------------
    g.throughput(Throughput::Elements(10_000));
    g.measurement_time(Duration::from_secs(10));
    g.warm_up_time(Duration::from_secs(3));

    g.bench_with_input(
        BenchmarkId::new(
            "publish_single",
            format!("subs={subs},cap={cap},burst={burst}"),
        ),
        &(),
        |b, _| {
            rt.block_on(async {
                let bus: Bus<Ev> = Bus::with_capacity(cap);
                // spawn subscribers
                let mut rxs: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();
                b.iter(|| {
                    for i in 0..10_000u64 {
                        let _ = bus.publish(Ev(i));
                    }
                    // drain
                    for rx in &mut rxs {
                        let _ = drain_now(rx);
                    }
                    black_box(());
                });
            });
        },
    );

    // --- Batch publish (A2) --------------------------------------------------------------
    #[cfg(feature = "bus_batch")]
    {
        g.throughput(Throughput::Elements(10_000));
        g.bench_with_input(
            BenchmarkId::new(
                "publish_many",
                format!("subs={subs},cap={cap},burst={burst}"),
            ),
            &(),
            |b, _| {
                rt.block_on(async {
                    let bus: Bus<Ev> = Bus::with_capacity(cap);
                    let mut rxs: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();

                    let mut batch = Vec::with_capacity(burst.max(1));
                    b.iter(|| {
                        batch.clear();
                        // total elements = 10_000 per iter (≈ 10_000 / burst batches)
                        for i in 0..10_000u64 {
                            batch.push(Ev(i));
                            if batch.len() == burst {
                                let _ = bus.publish_many(&batch);
                                batch.clear();
                            }
                        }
                        if !batch.is_empty() {
                            let _ = bus.publish_many(&batch);
                            batch.clear();
                        }
                        // drain
                        for rx in &mut rxs {
                            let _ = drain_now(rx);
                        }
                        black_box(());
                    });
                });
            },
        );
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(6))
        .warm_up_time(Duration::from_secs(2))
        .sample_size(40);
    targets = bench_bus_batch
}
criterion_main!(benches);
