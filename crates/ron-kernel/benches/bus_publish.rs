//! RO:WHAT — Bus publish perf: steady-state + bursty (classic vs edge, fanout, tunable caps).
//! RO:WHY  — Show real-world wins by:
//!           • steady-state apples-to-apples,
//!           • burst benches with draining + configurable fanout,
//!           • optional publisher epoch (yield between bursts),
//!           • configurable bus cap to avoid queue backpressure (critical).
//! RO:NOTE — Only `publish()`/`publish_many()` are timed; setup/drain outside hot loops.

use std::{env, time::Duration};

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
#[cfg(feature = "bus_batch")]
use criterion::BatchSize;

use ron_kernel::{KernelEvent, Metrics};
use tokio::sync::broadcast::error::RecvError;

// ----------------------------- Env toggles -----------------------------

fn getenv_usize(keys: &[&str], default_: usize) -> usize {
    for k in keys {
        if let Ok(v) = env::var(k) {
            if let Ok(n) = v.parse::<usize>() {
                if n > 0 {
                    return n;
                }
            }
        }
    }
    default_
}

fn getenv_bool(keys: &[&str]) -> bool {
    for k in keys {
        if let Ok(v) = env::var(k) {
            let s = v.to_ascii_lowercase();
            if s == "1" || s == "true" || s == "yes" || s == "on" {
                return true;
            }
            if s == "0" || s == "false" || s == "no" || s == "off" {
                return false;
            }
        }
    }
    false
}

// Accept both RON_* and plain keys (your earlier runs used BURST/CAP).
fn burst_size() -> usize {
    getenv_usize(&["RON_BURST", "BURST", "RON_BENCH_BURST"], 256)
}
fn fanout() -> usize {
    getenv_usize(&["RON_FANOUT", "FANOUT", "RON_BENCH_FANOUT"], 4)
}
fn pub_epoch_yield() -> bool {
    getenv_bool(&["RON_BENCH_PUB_YIELD", "PUB_YIELD"])
}
fn burst_cap() -> usize {
    getenv_usize(&["RON_CAP", "CAP", "RON_BENCH_CAP"], 2048)
}
fn tls_flush_threshold() -> usize {
    getenv_usize(&["RON_TLS_FLUSH_THRESHOLD", "TLS_THRESH"], 64)
}

// ------------------------------ Utilities -----------------------------

#[inline(always)]
fn publish_burst<B: Publisher<KernelEvent>>(bus: &B, n: usize) {
    for _ in 0..n {
        let _ = black_box(bus.publish(KernelEvent::Shutdown));
    }
}

trait Publisher<T> {
    fn publish(&self, t: T) -> usize;
}
impl Publisher<KernelEvent> for ron_kernel::bus::bounded::Bus<KernelEvent> {
    #[inline(always)]
    fn publish(&self, t: KernelEvent) -> usize {
        self.publish(t)
    }
}

fn spawn_classic_drains(
    rt: &tokio::runtime::Runtime,
    bus: &ron_kernel::bus::bounded::Bus<KernelEvent>,
    n: usize,
) {
    for _ in 0..n {
        let mut rx = bus.subscribe();
        rt.spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_msg) => {}
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}

#[cfg(feature = "bus_edge_notify")]
fn spawn_edge_drains(
    rt: &tokio::runtime::Runtime,
    bus: &ron_kernel::bus::bounded::Bus<KernelEvent>,
    n: usize,
) {
    use ron_kernel::bus::bounded::EdgeReceiver;
    for idx in 0..n {
        let mut sub: EdgeReceiver<KernelEvent> = bus.subscribe_edge();
        rt.spawn(async move { sub.run_drain_loop(idx).await; });
    }
}

// -------------------------------- Benches -----------------------------

fn bench_publish(c: &mut Criterion) {
    // Log config once per run so threshold sweeps are easy to map in output.
    let tls_thresh = tls_flush_threshold();
    let burst = burst_size();
    let fanout_n = fanout();
    let cap = burst_cap();
    eprintln!(
        "[bench cfg] RON_TLS_FLUSH_THRESHOLD={}, burst={}, fanout={}, cap={}",
        tls_thresh, burst, fanout_n, cap
    );

    // ============ Group 1: steady-state (classic, idle subscriber) ============
    let mut steady = c.benchmark_group(format!("bus_publish_steady (tls_thresh={})", tls_thresh));
    steady.sampling_mode(SamplingMode::Flat);
    steady.sample_size(80);
    steady.warm_up_time(Duration::from_secs(1));
    steady.measurement_time(Duration::from_secs(6));

    // (A) no subscribers — cap=64 (small, stable)
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(64);
        steady.bench_with_input(BenchmarkId::new("no_subscribers", "publish()"), &(), |b, _| {
            b.iter(|| {
                let _ = black_box(bus.publish(KernelEvent::Shutdown));
            });
        });
    }

    // (B) one subscriber (idle; no recv) — cap=64
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(64);
        let _rx = bus.subscribe(); // keep alive; no recv()
        steady.bench_with_input(BenchmarkId::new("one_subscriber", "publish()"), &(), |b, _| {
            b.iter(|| {
                let _ = black_box(bus.publish(KernelEvent::Shutdown));
            });
        });
    }

    // (C) lagged subscriber (cap=1; no recv)
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(1);
        let _rx = bus.subscribe(); // keep alive; no recv()
        steady.bench_with_input(
            BenchmarkId::new("lagged_subscriber_cap1", "publish()"),
            &(),
            |b, _| {
                b.iter(|| {
                    let _ = black_box(bus.publish(KernelEvent::Shutdown));
                });
            },
        );
    }
    steady.finish();

    // Runtime for burst groups
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio rt");

    // ============ Group 2: Bursty — CLASSIC recv drain (fanout) ============
    let mut bursty_classic =
        c.benchmark_group(format!("bus_publish_bursty_classic (tls_thresh={})", tls_thresh));
    bursty_classic.sampling_mode(SamplingMode::Flat);
    bursty_classic.sample_size(60);
    bursty_classic.warm_up_time(Duration::from_secs(1));
    bursty_classic.measurement_time(Duration::from_secs(6));

    let label = format!("burst{}_fanout{}_cap{}", burst, fanout_n, cap);

    // (C1) classic fanout; cap=CAP (avoid backpressure)
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(cap);
        spawn_classic_drains(&rt, &bus, fanout_n);

        bursty_classic.throughput(Throughput::Elements(burst as u64));
        bursty_classic.bench_with_input(
            BenchmarkId::new("classic_fanout", &label),
            &(),
            |b, _| {
                b.iter(|| {
                    publish_burst(&bus, burst);
                    if pub_epoch_yield() {
                        rt.block_on(async { tokio::task::yield_now().await });
                    }
                });
            },
        );

        rt.block_on(async { tokio::task::yield_now().await });
    }

    // (C2) classic lagged fanout; cap=1 (pressure path)
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(1);
        spawn_classic_drains(&rt, &bus, fanout_n);

        let label_lag = format!("burst{}_fanout{}_cap{}", burst, fanout_n, 1);
        bursty_classic.throughput(Throughput::Elements(burst as u64));
        bursty_classic.bench_with_input(
            BenchmarkId::new("classic_lagged_fanout", &label_lag),
            &(),
            |b, _| {
                b.iter(|| {
                    publish_burst(&bus, burst);
                    if pub_epoch_yield() {
                        rt.block_on(async { tokio::task::yield_now().await });
                    }
                });
            },
        );

        rt.block_on(async { tokio::task::yield_now().await });
    }

    bursty_classic.finish();

    // ============ Group 3: Bursty — EDGE recv drain (fanout, gated) ============
    #[cfg(feature = "bus_edge_notify")]
    {
        let mut bursty_edge =
            c.benchmark_group(format!("bus_publish_bursty_edge (tls_thresh={})", tls_thresh));
        bursty_edge.sampling_mode(SamplingMode::Flat);
        bursty_edge.sample_size(60);
        bursty_edge.warm_up_time(Duration::from_secs(1));
        bursty_edge.measurement_time(Duration::from_secs(6));

        let burst = burst_size();
        let fanout_n = fanout();
        let cap = burst_cap();
        let label = format!("burst{}_fanout{}_cap{}", burst, fanout_n, cap);

        // (E1) edge fanout; cap=CAP
        {
            let metrics = Metrics::new(true);
            let bus = metrics.make_bus::<KernelEvent>(cap);
            spawn_edge_drains(&rt, &bus, fanout_n);

            bursty_edge.throughput(Throughput::Elements(burst as u64));
            bursty_edge.bench_with_input(BenchmarkId::new("edge_fanout", &label), &(), |b, _| {
                b.iter(|| {
                    publish_burst(&bus, burst);
                    if pub_epoch_yield() {
                        rt.block_on(async { tokio::task::yield_now().await });
                    }
                });
            });

            rt.block_on(async { tokio::task::yield_now().await });
        }

        // (E2) edge lagged fanout; cap=1
        {
            let metrics = Metrics::new(true);
            let bus = metrics.make_bus::<KernelEvent>(1);
            spawn_edge_drains(&rt, &bus, fanout_n);

            let label_lag = format!("burst{}_fanout{}_cap{}", burst, fanout_n, 1);
            bursty_edge.throughput(Throughput::Elements(burst as u64));
            bursty_edge.bench_with_input(
                BenchmarkId::new("edge_lagged_fanout", &label_lag),
                &(),
                |b, _| {
                    b.iter(|| {
                        publish_burst(&bus, burst);
                        if pub_epoch_yield() {
                            rt.block_on(async { tokio::task::yield_now().await });
                        }
                    });
                },
            );

            rt.block_on(async { tokio::task::yield_now().await });
        }

        bursty_edge.finish();
    }
}

#[cfg(feature = "bus_batch")]
fn bench_publish_batched(c: &mut Criterion) {
    // ========= Group 4: Bursty — **BATCHED** publish_many (fanout), real A2 path =========
    let tls_thresh = tls_flush_threshold();
    let mut bursty_batched =
        c.benchmark_group(format!("bus_publish_bursty_batched (tls_thresh={})", tls_thresh));
    bursty_batched.sampling_mode(SamplingMode::Flat);
    bursty_batched.sample_size(60);
    bursty_batched.warm_up_time(Duration::from_secs(1));
    bursty_batched.measurement_time(Duration::from_secs(6));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio rt");

    let burst = burst_size();
    let fanout_n = fanout();
    let cap = burst_cap();
    let label = format!("burst{}_fanout{}_cap{}", burst, fanout_n, cap);

    // (B1) batched fanout; cap=CAP (no backpressure)
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(cap);
        spawn_classic_drains(&rt, &bus, fanout_n);

        bursty_batched.throughput(Throughput::Elements(burst as u64));
        bursty_batched.bench_with_input(BenchmarkId::new("batched_fanout", &label), &(), |b, _| {
            // Prepare per-iter batch without timing the setup (A2 hot path only):
            b.iter_batched_ref(
                || {
                    let mut v = Vec::with_capacity(burst);
                    v.resize(burst, KernelEvent::Shutdown);
                    v
                },
                |batch| {
                    #[allow(unused_must_use)]
                    {
                        bus.publish_many(black_box(&mut batch[..]));
                    }
                    if pub_epoch_yield() {
                        rt.block_on(async { tokio::task::yield_now().await });
                    }
                },
                BatchSize::SmallInput,
            );
        });

        rt.block_on(async { tokio::task::yield_now().await });
    }

    // (B2) batched **lagged** fanout; cap=1 (pressure path)
    {
        let metrics = Metrics::new(true);
        let bus = metrics.make_bus::<KernelEvent>(1);
        spawn_classic_drains(&rt, &bus, fanout_n);

        let label_lag = format!("burst{}_fanout{}_cap{}", burst, fanout_n, 1);
        bursty_batched.throughput(Throughput::Elements(burst as u64));
        bursty_batched.bench_with_input(
            BenchmarkId::new("batched_lagged_fanout", &label_lag),
            &(),
            |b, _| {
                b.iter_batched_ref(
                    || {
                        let mut v = Vec::with_capacity(burst);
                        v.resize(burst, KernelEvent::Shutdown);
                        v
                    },
                    |batch| {
                        #[allow(unused_must_use)]
                        {
                            bus.publish_many(black_box(&mut batch[..]));
                        }
                        if pub_epoch_yield() {
                            rt.block_on(async { tokio::task::yield_now().await });
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        rt.block_on(async { tokio::task::yield_now().await });
    }

    bursty_batched.finish();
}

// ---- Criterion mains (feature-gated so you can `--features bus_batch`) ----

#[cfg(feature = "bus_batch")]
criterion_group!(benches, bench_publish, bench_publish_batched);

#[cfg(not(feature = "bus_batch"))]
criterion_group!(benches, bench_publish);

criterion_main!(benches);
