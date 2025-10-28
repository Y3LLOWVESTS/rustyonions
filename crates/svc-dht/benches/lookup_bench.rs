//! Criterion + custom stats: lookup baseline and optional hedge tail-rescue sim.
//! RO:WHAT — (1) Baseline lookup path with β sweep and zero stagger (fast).
//!           (2) Optional tail-rescue sim showing P50/P95/P99 (env-gated).
//! RO:RUN  — Fast baseline only (default):
//!            cargo bench -p svc-dht --bench lookup_bench
//!           Include tail-rescue sim (tunable):
//!            DHT_SIM=1 DHT_TRIALS=600 DHT_PSLOW=0.05 DHT_STAGGER_MS=2 cargo bench -p svc-dht --bench lookup_bench

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use svc_dht::pipeline::hedging::race_hedged;
use svc_dht::pipeline::lookup::{LookupCtx, LookupRequest};
use svc_dht::provider::Store;

// ---------- tiny helpers ----------
fn percentiles(mut xs: Vec<f64>) -> (f64, f64, f64) {
    if xs.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = |p: f64| -> usize {
        let n = xs.len() as f64;
        let k = (p * (n - 1.0)).round() as usize;
        k.min(xs.len() - 1)
    };
    (xs[idx(0.50)], xs[idx(0.95)], xs[idx(0.99)])
}

// SplitMix-like deterministic mixer (no non-Send RNG).
#[inline]
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}
#[inline]
fn mix_range_inc(x: u64, min: u64, max: u64) -> u64 {
    let span = max.saturating_sub(min) + 1;
    min + (mix64(x) % span)
}

// ---------- optional tail-rescue sim (env-gated) ----------
fn maybe_print_hedge_tail_rescue(rt: &tokio::runtime::Runtime) {
    let sim = std::env::var("DHT_SIM").ok().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    if sim == 0 {
        // Keep benches fast unless explicitly enabled.
        return;
    }

    let trials: usize =
        std::env::var("DHT_TRIALS").ok().and_then(|s| s.parse().ok()).unwrap_or(400);
    let p_slow: f64 = std::env::var("DHT_PSLOW").ok().and_then(|s| s.parse().ok()).unwrap_or(0.05);
    let hedge_stagger_ms: u64 =
        std::env::var("DHT_STAGGER_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
    let slow_min: u64 =
        std::env::var("DHT_SLOW_MIN_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(80);
    let slow_max: u64 =
        std::env::var("DHT_SLOW_MAX_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(120);
    let fast_min: u64 =
        std::env::var("DHT_FAST_MIN_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let fast_max: u64 =
        std::env::var("DHT_FAST_MAX_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
    let leg_budget_ms: u64 =
        std::env::var("DHT_LEG_BUDGET_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(150);

    let leg_budget = Duration::from_millis(leg_budget_ms);
    let ctr = Arc::new(AtomicU64::new(1));

    let run_beta = |beta: usize| -> Vec<f64> {
        let mut out = Vec::with_capacity(trials);
        rt.block_on(async {
            for _ in 0..trials {
                let t0 = Instant::now();
                let _ = race_hedged::<_, _, (), ()>(
                    beta,
                    Duration::from_millis(hedge_stagger_ms),
                    leg_budget,
                    {
                        let ctr = ctr.clone();
                        move |leg_idx| {
                            let seed =
                                ctr.fetch_add(1, Ordering::Relaxed).wrapping_add(leg_idx as u64);
                            async move {
                                // primary slow with prob p_slow; hedges fast
                                let slow_roll = (mix64(seed) as f64) / (u64::MAX as f64);
                                let is_slow_primary = leg_idx == 0 && slow_roll < p_slow;
                                let delay_ms = if is_slow_primary {
                                    mix_range_inc(seed ^ 0xA5A5, slow_min, slow_max)
                                } else {
                                    mix_range_inc(seed ^ 0x5A5A, fast_min, fast_max)
                                };
                                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                                Ok(())
                            }
                        }
                    },
                )
                .await;
                out.push(t0.elapsed().as_secs_f64() * 1_000.0);
            }
        });
        out
    };

    let b0 = run_beta(0);
    let b1 = run_beta(1);
    let b2 = run_beta(2);
    let b3 = run_beta(3);
    let (b0_p50, b0_p95, b0_p99) = percentiles(b0);
    let (b1_p50, b1_p95, b1_p99) = percentiles(b1);
    let (b2_p50, b2_p95, b2_p99) = percentiles(b2);
    let (b3_p50, b3_p95, b3_p99) = percentiles(b3);

    println!(
        "\n=== Hedge Tail Rescue (trials={} p_slow={:.1}% stagger={}ms budget={}ms) ===",
        trials,
        p_slow * 100.0,
        hedge_stagger_ms,
        leg_budget_ms
    );
    println!("β=0  P50={:.2}ms  P95={:.2}ms  P99={:.2}ms", b0_p50, b0_p95, b0_p99);
    println!("β=1  P50={:.2}ms  P95={:.2}ms  P99={:.2}ms", b1_p50, b1_p95, b1_p99);
    println!("β=2  P50={:.2}ms  P95={:.2}ms  P99={:.2}ms", b2_p50, b2_p95, b2_p99);
    println!("β=3  P50={:.2}ms  P95={:.2}ms  P99={:.2}ms", b3_p50, b3_p95, b3_p99);
    println!("(sim is env-gated; default bench does not run it)");
}

// ---------- (1) Baseline: lookup over in-memory store (no stagger) ----------
fn bench_lookup_baseline(c: &mut Criterion) {
    let store = Arc::new(Store::new(Duration::from_secs(60)));
    let cid = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

    // Warm providers so the lookup path returns immediately.
    for i in 0..8 {
        store.add(cid.clone(), format!("local://node{i}"), Some(Duration::from_secs(60)));
    }
    let ctx = LookupCtx::new(store, 64);

    // Current-thread RT for stable measurements.
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("rt");

    // Print the optional tail simulation (fast baseline remains unaffected if DHT_SIM=0).
    maybe_print_hedge_tail_rescue(&rt);

    let mut group = c.benchmark_group("lookup_baseline");
    for beta in [0usize, 1, 2, 3] {
        group.bench_with_input(BenchmarkId::new("beta", beta), &beta, |b, &bval| {
            b.iter(|| {
                rt.block_on(async {
                    let req = LookupRequest {
                        cid: cid.clone(),
                        alpha: 1,
                        beta: bval,
                        hop_budget: 6,
                        deadline: Duration::from_millis(200),
                        hedge_stagger: Duration::from_millis(0), // ← zero to measure orchestration
                        min_leg_budget: Duration::from_millis(5),
                    };
                    // Don’t panic in benches; rare timing hiccups shouldn’t fail the run.
                    let _ = ctx.run(req).await;
                });
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_lookup_baseline);
criterion_main!(benches);
