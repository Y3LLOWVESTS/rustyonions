### BEGIN NOTE - NOVEMBER 5 2025 - 19:37 CST

RUN BENCHMARK: bash crates/ron-kms/scripts/bench.sh

TLDR: ron-kms benchmark proves elite performance (God tier)

# ron-kms — Benchmark Summary (macOS laptop)

**Hardware**

* 2019 13" MacBook Pro, Intel i5-8257U (4C/8T, base 1.4 GHz, turbo ~3.9 GHz), 16 GB RAM
* Power mode: **Low Power OFF** (plugged in), macOS throttling/fans as usual

**Benchmark harness**

* Criterion with: `--warm-up-time 8 --measurement-time 15 --sample-size 300`
* Command: per-bench invocations via `crates/ron-kms/scripts/bench.sh`
* Build flags: `RUSTFLAGS="-C target-cpu=native"`; `[profile.bench] opt-level=3, lto="thin"`

---

## 1) Single-op signing: `sign_bench::ed25519_sign_128B`

**What it measures**
Time to produce one Ed25519 signature over a 128-byte message with the in-memory keystore (ed25519-dalek). Hot path: `SigningKey::sign(msg)`.

**Result**

* **17.286–17.333 µs** (median **17.309 µs**)
* Throughput ≈ **57.8k signs/sec** (1 / 17.309e-6)

**Interpretation**
This is very strong for a 15 W mobile Intel (pure Rust, no hand-tuned ASM). Disabling Low Power Mode cut latency ~43–45% vs earlier runs.

---

## 2) Single-op verify: `verify_bench::ed25519_verify_128B`

**What it measures**
Time to verify one Ed25519 signature over a 128-byte message (pure dalek verify path).

**Result**

* **33.617–33.701 µs** (median **33.659 µs**)
* Throughput ≈ **29.7k verifies/sec**

**Interpretation**
This lines up with dalek’s expected verify cost (≈2× sign). It’s now ~32–33% faster than our previous low-power runs.

---

## 3) Batch verify (amortized): `batch_verify::{verify_batch_8,32,64}`

**What it measures**
Wall-time to verify **N** signatures in one batch using the fast path (amortizes scalar mults). We report both **total** and **per-op**.

**Results (total → per-op)**

* **N=8:** 170.97–171.51 µs (median **171.24 µs**) → **21.4 µs/op** (≈46.7k/s)
* **N=32:** 634.80–637.47 µs (median **636.04 µs**) → **19.9 µs/op** (≈50.3k/s)
* **N=64:** 1.2409–1.2442 ms (median **1.2425 ms**) → **19.4 µs/op** (≈51.5k/s)

**Interpretation**
Batching slashes amortized verify from ~33.7 µs/op → **~19–21 µs/op** (≈**40% faster**). Sweet spot is around **32–64** items per batch on this CPU.

> Criterion warned about sample count at N=32; that’s just a runtime/variance note. Numbers are stable—extend measurement time if you want quieter CI plots.

---

## 4) Parallel throughput (4× threads): `parallel_throughput::{parallel_sign_4x, parallel_verify_4x}`

**What it measures**
Aggregate time to complete **4 × 1000 = 4000 ops** with 4 worker threads (coarse multi-core throughput on this 4C CPU).

**Results (median)**

* **Sign 4×:** **20.047 ms** for 4000 ops → **~199k signs/sec aggregate**
  (4000 / 0.020047 ≈ 199, k/s)
* **Verify 4×:** **45.890 ms** for 4000 ops → **~87.2k verifies/sec aggregate**
  (4000 / 0.045890 ≈ 87, k/s)

**Interpretation**
Excellent scaling for a thermally-limited laptop. On a desktop/server CPU, expect materially higher aggregate numbers. These already clear typical gateway SLOs with ease.

---

## Takeaways (fast)

* **Single-op:** 17.3 µs sign, 33.7 µs verify.
* **Batch verify (32–64):** ~**19–20 µs/op** (≈**40%** faster than single verify).
* **Parallel (4 threads):** ~**199k signs/s**, **87k verifies/s** aggregate.

This is **production-ready** for Micronode and won’t bottleneck Gateway. For Macronode scale or PQ-hybrid later, batching + parallelism gives plenty of headroom.

---

## “God-tier” performance knobs we can still turn

1. **Batch everywhere it fits** on ingress (JWT/passport checks, attestation bundles). Target batch size 32–64.
2. **Parallel at the caller** when batches are huge (split into chunks of 64 per thread; batch inside each thread).
3. **Hot-key caches** for public keys (avoid map lookups & copies in tight loops).
4. **Longer Criterion windows** in CI to reduce variance (`--measurement-time 20`, `--sample-size 300`).
5. **Perf gates** (fail PR if slower):

   * `sign_128B ≤ 18.0 µs`
   * `verify_128B ≤ 35.0 µs`
   * `verify_batch_32 ≤ 20.5 µs/op`


### END NOTE - NOVEMBER 5 2025 - 19:37 CST