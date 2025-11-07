

# ron-kms — Benchmark Playbook (God-Tier Verify Locked)

## 0) TL;DR (what’s “best” and why)

* **Best overall verify path for production:**
  **Ed25519 (dalek v2) + `verify_strict` + true multiscalar (`dalek-batch`) + parallel chunking (`parallel-batch`)**.
  Yields ~**1.6–1.7×** faster batches vs serial multiscalar on your 2019 i5 laptop; per-op verify down to ~**9–12 µs** at realistic batch sizes (64–512).

* **Best sign path (optional lane):**
  Enable **`fast` (ring)** if you need the last few µs for *signing*. Keep **dalek** for verifying.

* **Security posture:**
  We verify with **`verify_strict`** everywhere (non-malleable). Single-verify is ~33 µs on your box (a deliberate ~10 µs cost vs non-strict).

---

## 1) Repro commands (exact)

### 1.1 Serial multiscalar baseline (for reference)

```
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms \
  --features dalek-batch \
  --bench batch_verify -- --measurement-time 14 --sample-size 90
```

**Representative results (your box):**

* `verify_batch/8`  ≈ **173 µs**
* `verify_batch/32` ≈ **636 µs**
* `verify_batch/64` ≈ **1.251 ms**

### 1.2 Parallel multiscalar (throughput mode, production-preferred)

```
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms \
  --features "dalek-batch,parallel-batch" \
  --bench batch_verify -- --measurement-time 14 --sample-size 90
```

**Representative results (your box):**

* `verify_batch/8`  ≈ **170–173 µs** (flat vs serial; overhead amortizes at larger N)
* `verify_batch/32` ≈ **391 µs** (**~1.63×** faster than serial)
* `verify_batch/64` ≈ **744 µs** (**~1.68×** faster than serial)

### 1.3 Sustained throughput benchmark (msgs/sec)

Ensure this `[[bench]]` exists in `crates/ron-kms/Cargo.toml`:

```toml
[[bench]]
name = "throughput_batch"
harness = false
```

Run:

```
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=4 \
cargo bench -p ron-kms --features "dalek-batch,parallel-batch" \
  --bench throughput_batch -- --measurement-time 14 --sample-size 90
```

**Your latest medians:**

| Batch |     Median time | Throughput (Elements/s) | Effective per-verify |
| ----: | --------------: | ----------------------: | -------------------: |
|    64 | ~**738–742 µs** |        **86.2–87.1k/s** |      **~11.5 µs/op** |
|   128 |   ~**1.283 ms** |      **~99.1–100.3k/s** |      **~10.0 µs/op** |
|   256 |   ~**2.356 ms** |         **~108–109k/s** |       **~9.2 µs/op** |
|   512 |   ~**4.617 ms** |           **~110.9k/s** |       **~9.0 µs/op** |

> Takeaway: as batch size grows, total throughput climbs and per-op cost trends to ~9 µs—**God-tier** for software KMS.

---

## 2) What each benchmark measures (and how to read it)

* **`batch_verify` bench (N=8/32/64)**
  Measures **batch latency** (time to verify N signatures together). With `dalek-batch` we use **true multiscalar** (amortizes curve ops across the set). With `parallel-batch`, we **split into chunks** and run multiscalar in parallel, then AND results.
  **Use when:** you care about how a single “batch window” affects request latency.

* **`throughput_batch` bench (N=64/128/256/512)**
  Reports **time/iteration** and **Elements/s**. This is your **sustained verify rate** at steady state.
  **Use when:** you want to size hardware, set SLOs, or compare deployment profiles (cores/threads).

* **`verify_bench` (steady single verify)**
  Measures a single strict verify with a prebuilt verifying key: ~**32.9 µs** median on your laptop.
  **Why it’s higher than earlier 20–22 µs claims:** we moved to **`verify_strict`** (more checks, safer). Keep strict in prod.

* **`sign_bench`**
  Measures sign with dalek (~**16.7 µs**) and can measure ring (faster sign, slower verify).
  **Use when:** request path involves server-side signing (attestations, rotations, receipts).

---

## 3) Ideal real-world usage patterns (how to **translate** benches to prod)

### 3.1 Batch formation (ingress verify hot path)

* **Collect requests into micro-batches** by either a **count** (e.g., 64/128) or a **time window** (e.g., 250–750 µs), whichever comes first.
  This keeps **latency bounded** while unlocking multiscalar + parallel speedups.
* **Heuristic we like:**

  * `target_batch = 64` (floor), `max_batch = 256` (cap), `max_wait = 500 µs`.
  * If traffic is heavy, you’ll routinely hit 128–256 and ride the **~9–10 µs/op** curve.
  * If traffic is light, you still batch 8–32 quickly and stay near serial latency.

### 3.2 Parallel chunking knobs

* The library auto-sizes chunks to ensure **≥8 items per chunk** and to use **~2× available threads**.
* In services with many CPU-bound tasks, set `RAYON_NUM_THREADS` (or use a shared rayon pool sized to your CPU) to avoid oversubscription.

### 3.3 When to stick to serial multiscalar

* **Very small bursts** (e.g., <= 8) *and* ultra-tight tail latency budgets (sub-millisecond).
  Parallel overhead is already small, but you can compile **without** `parallel-batch` if you want fewer moving parts. You’ll land at ~**170 µs** for N=8 on your box.

### 3.4 Signing lanes

* **Default:** dalek.
* **If you need maximum sign throughput:** expose a **“fast sign”** lane using **ring** behind a feature. Keep all **verify** on dalek.

---

## 4) Environment & runner tips

* Always use `-C target-cpu=native` locally:

  ```
  RUSTFLAGS="-C target-cpu=native"
  ```
* For throughput: set `RAYON_NUM_THREADS` to **physical cores** (or a bit higher if the work is compute-bound and memory behavior is friendly):

  ```
  RAYON_NUM_THREADS=4  # adjust per machine
  ```
* Keep the machine cool and on AC; close heavy apps. On Linux, **performance governor** and core pinning (`taskset`) reduce jitter.

---

## 5) CI / local perf gates (recommended)

Save baselines and guard them:

```
# Serial multiscalar baseline
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms \
  --features dalek-batch \
  --bench batch_verify -- --measurement-time 14 --sample-size 90 \
  --save-baseline kms-serial-YYYY-MM-DD

# Parallel baseline
RUSTFLAGS="-C target-cpu=native" cargo bench -p ron-kms \
  --features "dalek-batch,parallel-batch" \
  --bench batch_verify -- --measurement-time 14 --sample-size 90 \
  --save-baseline kms-par-YYYY-MM-DD
```

**Suggested local gates (medians):**

* Parallel multiscalar:

  * `verify_batch/32` ≤ **0.42 ms**
  * `verify_batch/64` ≤ **0.80 ms**
* Serial multiscalar (reference):

  * `verify_batch/32` ≤ **0.68 ms**
  * `verify_batch/64` ≤ **1.33 ms**
* Single strict verify (info only): ~**33 µs** (± jitter)
* Sign (dalek steady): ~**16–17 µs**; **ring fast**: ~**14–15 µs**

> In CI VMs, widen thresholds ~10–20% or run perf jobs on dedicated runners.

---

## 6) Why these benches matter to the server

* **Batch verify** mirrors the most common server hot path: “verify many independent signatures per unit time.”
  True multiscalar **reduces total curve work**; parallel chunking **utilizes all cores**. This combo is why we’re seeing **>100k verifies/sec** sustained on a **2019 laptop**.
* **Latency remains bounded**: even a batch of 512 completes in ~4.6 ms on your box. With **64–256**, you’re in the **0.74–2.36 ms** window—very comfortable for microservice RPC SLOs.
* **Strict verification** is a non-negotiable security invariant; our numbers already include it.

---

## 7) Tuning & advanced options (when you want the last few %)

* **Small-N fast path:** skip parallel path for `n < ~24`. Gains are modest (~1–3%) but can shave a few µs at N=8.
* **Profile settings:** `lto = "thin"`, `codegen-units = 1` in `[profile.release]` and `[profile.bench]` help inlining; always test.
* **Scratch reuse (serial path):** we already reuse allocations to cut allocator noise (helpful when `parallel-batch` is off).
* **Prehash mode (large payloads only):** Ed25519ph semantics (e.g., BLAKE3 prehash) reduce copying costs for KB–MB messages. Keep feature-gated; use only when messages are large.

---

## 8) Numbers you can quote (your hardware, Nov 6 2025)

* **Serial multiscalar:** N=32 **~636 µs**, N=64 **~1.251 ms**
* **Parallel multiscalar:** N=32 **~391 µs**, N=64 **~744 µs**
* **Throughput (parallel):**
  **64→ ~86.6k/s**, **128→ ~100k/s**, **256→ ~109k/s**, **512→ ~111k/s**
  (Per-op ≈ **9–12 µs** at scale)
* **Single strict verify:** **~32.9 µs**
* **Sign (dalek steady):** **~16.7 µs**
  **Sign (ring “fast”):** **~14.3 µs**

---

## 9) Production checklist (how to **utilize** this)

* [ ] Deploy with **dalek verify** + **`verify_strict`**.
* [ ] Enable **`dalek-batch`** and **`parallel-batch`** features.
* [ ] In the verify service: **accumulate requests** using `max_wait ≈ 500 µs`, `target_batch=64`, `max_batch=256`.
* [ ] Run with **`RAYON_NUM_THREADS = (physical cores)`** for the verify pool.
* [ ] Keep the **sign** lane configurable: dalek default; **ring fast** optional for sign-heavy flows.
* [ ] Add **perf gates** on the medians above; save Criterion baselines on release tags.
* [ ] Monitor **ops metrics**: `kms_ops_total{op,alg}`, `kms_op_latency_seconds{op}`, and **batch size histograms** (to confirm batching behavior matches expectations).

---

## 10) Bottom line

You’re running a **God-tier software KMS**: strict, multiscalar, multi-core. On modest hardware, you’re already clearing **~100k verifications/sec** with sub-ms batch latencies. The playbook above is exactly how to keep those wins in real traffic while guarding regressions over time.


