

# ron-kernel Bench Suite

Fast, reproducible microbenches for the in-process bus backends (classic bounded + new SoA engine), including edge-coalesced wakeups and batched publishing.

> If you only read one section: run the **Quickstart** and then **FORTRAN Trick – What Changed**.

---

## TL;DR — Quickstart

```bash
cargo bench -p ron-kernel --bench bus_publish -- \
  --warm-up-time 3 --measurement-time 10
```

Enable the Structure-of-Arrays backend and edge-coalesced wakes:

```bash
cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
  --warm-up-time 3 --measurement-time 10
```

Compare vs a saved baseline (uses Criterion’s baselines):

```bash
# Save a classic baseline
cargo bench -p ron-kernel --bench bus_publish -- \
  --warm-up-time 3 --measurement-time 10 --save-baseline classic

# Compare SoA+edge against that baseline
cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
  --warm-up-time 3 --measurement-time 10 --baseline classic
```

---

## What you’ll see (wow-factor snapshots)

From our latest runs on this repo:

* **Steady publish (1 sub):** ~190 ns → **~96 ns** (≈ **1.95× faster**)
* **Lagged (cap=1) path:** ~188 ns → **~94 ns** (≈ **2× faster**)
* **Bursty classic fanout (burst=256, fanout=4, cap=2048):** ~5.1 Melem/s → **~10.34 Melem/s** (≈ **2× throughput**)
* **Bursty edge fanout:** ~0.99 Melem/s → **~1.64 Melem/s** (≈ **1.6×**)

Your silicon may vary, but the shape of the wins should be obvious.

---

## The “FORTRAN trick” — What changed and why it wins

We re-ordered the SoA receiver hot path like an old-school Fortran inner loop: **do the minimal thing once, in the cache-friendly order, with just-enough atomics**.

* **Claim → clone once → last reader drops:** We `fetch_and` the per-subscriber bitmask, clone the payload once, and only the last reader `take()`s the slot. No double-lock, no speculative loads, no ping-pong.
* **Tight atomics:** `Acquire/Release` where correctness needs it, `Relaxed` only for benign checks. This removes unnecessary fences from the hot path.
* **Edge-notify drain loop:** Coalesces wakes to **one notify per batch** and performs a race check after draining to avoid spurious awaits.

Result: halves single-sub latency and doubles classic burst throughput.

---

## Bench matrix

The `bus_publish` bench exercises four families:

1. **Steady / no_subscribers** — cost of a publish when nobody listens.
2. **Steady / one_subscriber** — cost with one live reader.
3. **Steady / lagged_subscriber_cap1** — the gnarly cap=1 lag path.
4. **Bursty** (classic and edge) — burst=256 into fanout=4 at various caps.

### Classic (bounded) backend

```bash
cargo bench -p ron-kernel --bench bus_publish -- \
  --warm-up-time 3 --measurement-time 10
```

### SoA backend (Structure-of-Arrays)

```bash
cargo bench -p ron-kernel --bench bus_publish --features bus_soa -- \
  --warm-up-time 3 --measurement-time 10
```

### SoA + Edge-coalesced wakeups

```bash
cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
  --warm-up-time 3 --measurement-time 10
```

### SoA + Edge + Batched publishing

```bash
cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify,bus_batch" -- \
  --warm-up-time 3 --measurement-time 10
```

---

## Environment knobs (used by the benches)

These control the synthetic workload (the bench titles will echo what you set):

```bash
# Minimal queue to stress the lag path (dramatically shows SoA gains)
RON_BENCH_BURST=1 RON_BENCH_FANOUT=1 RON_BENCH_CAP=1 \
cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
  --warm-up-time 3 --measurement-time 10
```

Available variables:

* `RON_BENCH_BURST` — elements per burst (default 256).
* `RON_BENCH_FANOUT` — subscriber count (default 4).
* `RON_BENCH_CAP` — queue capacity (default 2048).
* `RON_TLS_FLUSH_THRESHOLD` — threshold for hot-path metric buffer flush (bench prints the value it used).

---

## Baselines 101 (Criterion)

Save a named baseline and compare future runs to it:

```bash
cargo bench -p ron-kernel --bench bus_publish -- \
  --warm-up-time 3 --measurement-time 10 --save-baseline core-2025-10-19

cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
  --warm-up-time 3 --measurement-time 10 --baseline core-2025-10-19
```

If Criterion errors with “Baseline must exist,” re-run the first command to create it (make sure `--save-baseline` is passed **after** `--` so Criterion sees it).

---

## Interpreting the output (quick guide)

* **time:** median and CI for op time (ns) or batch time (µs).
* **thrpt:** elements per second (for bursty tests).
* **change:** percent vs baseline (needs `--baseline NAME`).

Rules of thumb:

* Negative time change (or positive thrpt change) = good.
* Look at steady + lagged numbers for latency, bursty for throughput.

---

## Reproducibility & tooling notes

* Criterion handles runtime variance better than custom loops. We still recommend:

  * Close heavy apps/browsers.
  * Run on AC power, performance mode if available.
  * Pin to a quiet machine if you want pretty plots.

* If you see `Gnuplot not found, using plotters backend`, that’s fine; plots are still generated inline by `plotters`.

---

## Troubleshooting

* **“Unrecognized option: 'baseline' / 'save-baseline'”**
  Ensure options come **after** the double-dash that separates Cargo from Criterion:

  * Correct: `cargo bench … -- --baseline NAME`
  * Incorrect: `cargo bench … --baseline NAME`

* **Feature not found / method missing**
  Match features to benches:

  * `bus_soa` enables the SoA backend.
  * `bus_edge_notify` enables coalesced wakes + edge receivers.
  * `bus_batch` enables batched publish API/benches.

* **Huge regressions only on bursty edge**
  Re-run with default env (no overrides), then try:

  ```bash
  RON_BENCH_BURST=256 RON_BENCH_FANOUT=4 RON_BENCH_CAP=2048 \
  cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
    --warm-up-time 3 --measurement-time 10
  ```

---

## Example: full classic vs SoA+edge showdown

```bash
# 1) Save classic baseline
cargo bench -p ron-kernel --bench bus_publish -- \
  --warm-up-time 3 --measurement-time 10 --save-baseline classic

# 2) SoA + Edge (coalesced wakes) vs baseline
cargo bench -p ron-kernel --bench bus_publish --features "bus_soa,bus_edge_notify" -- \
  --warm-up-time 3 --measurement-time 10 --baseline classic
```

Expected shape (based on our runs):

* Steady/no_subscribers: ~68 ns (unchanged-ish)
* Steady/one_subscriber: **~96 ns** (≈1.9× faster)
* Steady/lagged_subscriber_cap1: **~94 ns** (≈2× faster)
* Bursty classic fanout: **~10.34 Melem/s** (≈2× throughput)
* Bursty edge fanout: **~1.64 Melem/s** (≈1.6×)

---

## FAQ

**Q: Do I need nightly or special flags?**
A: No. Stable Rust + `tokio` + Criterion is all you need.

**Q: Where are metrics?**
A: The benches print the effective config (including `RON_TLS_FLUSH_THRESHOLD`). For runtime metrics (Prometheus), run the example:

```bash
cargo run -p ron-kernel --example kernel_demo --features metrics_buf
```

Then visit `http://127.0.0.1:9600/metrics`.

---

**Have fun benchmarking — and if you beat these numbers on your machine, tell us.**
