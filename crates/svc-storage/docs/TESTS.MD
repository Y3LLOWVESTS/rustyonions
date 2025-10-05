

---

````markdown
---
title: 🧪 TESTS — svc-storage
audience: developers, auditors, CI maintainers
msrv: 1.80.0 (Tokio/loom compatible)
last-updated: 2025-10-04
---

# TESTS.md — svc-storage

## 0) Purpose

Define the **test contract** for `svc-storage`:

- Unit, integration, property, fuzz, chaos, performance.
- Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
- Reproducible invocation commands for devs & CI.

**Canon invariants under test (non-exhaustive):**
- **Content addressing:** BLAKE3 (`b3:<hex>`), strong ETag.
- **HTTP contract:** `GET/HEAD/Range` and `POST /put` (digest-on-write).
- **Input safety:** max body **1 MiB**, decompression ratio **≤ 10×** (+ absolute output cap).
- **Streaming I/O:** ~**64 KiB** chunking on the hot read path.
- **Backpressure:** `max_inflight` / `max_rps` shedding; structured 429/503.
- **Readiness DAG:** listeners + data dirs + pacer/cache → `/readyz = 200`.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100 ms)

**Scope:** Pure logic and small modules.

| ID | Area | What it asserts |
|----|------|------------------|
| UT-ADDR-01 | Address parser | `b3:<hex>` validation (length, hex set), rejects bad prefixes/case |
| UT-ETAG-02 | ETag | `ETag: "b3:<hex>"` is stable and derived from digest |
| UT-RANGE-03 | Range math | Normalize `(start,end)`; clamp negative/overflow; 416 rules |
| UT-CHUNK-04 | Chunker | Reader yields ~64 KiB chunks; boundary conditions at EOF |
| UT-DECOMP-05 | Decompress guard | Stops at ratio and absolute caps; returns structured error |
| UT-HEADERS-06 | Headers | `Accept-Ranges: bytes`, `Content-Length`, `Content-Range` formation |

**Location:** `src/*` (with `#[cfg(test)]`).

Run:
```bash
cargo test -p svc-storage --lib
````

---

### 1.2 Integration Tests (end-to-end, real HTTP)

**Scope:** Public surface in `tests/*.rs` with a booted instance (ephemeral data dir).

**Must cover:**

1. **Happy paths**

   * IT-HAPPY-GET-01: `POST /put` (random 1 MiB) → 201 Created + `Location: /o/b3:...` + strong ETag → `GET /o/...` 200 with identical bytes.
   * IT-HAPPY-HEAD-02: `HEAD /o/...` returns headers only; `Content-Length` and ETag consistent with GET.
   * IT-HAPPY-RANGE-03: Single range (`bytes=0-65535`) → 206 with correct `Content-Range`, payload length 65,536.

2. **Conditional requests**

   * IT-COND-ETAG-04: `If-None-Match: "b3:..."` on GET returns **304** (no body).
   * IT-COND-ETAG-05: Mismatched ETag returns **200** with body.

3. **Idempotency & dedupe**

   * IT-IDEMP-06: Re-`POST /put` same bytes → **200/201** (implementation-defined) but **must** point to same address; store is not duplicated.

4. **Safety & limits**

   * IT-LIMIT-07: Body > 1 MiB → **413** with reason label (metrics `rejected_total{reason="body_cap"}` increments).
   * IT-LIMIT-08: Compressed payload that would inflate > 10× or over absolute cap → **413** (`rejected_total{reason="decompress_cap"}`).

5. **Range edge cases**

   * IT-RANGE-09: `bytes=0-` (open-ended) → 206, full length.
   * IT-RANGE-10: Suffix range `bytes=-65536` returns last 64 KiB.
   * IT-RANGE-11: Invalid range past EOF → **416** with `Content-Range: bytes */<len>`.

6. **Readiness & gating**

   * IT-READY-12: `/healthz` 200 from process start; `/readyz` stays 503 until data dirs + pacer ready; flips to **200** within expected window.

7. **Backpressure & errors**

   * IT-SHED-13: Saturate `max_inflight` → proportionate **429/503**; no deadlocks; recovery when load drops.

8. **Observability**

   * IT-METRICS-14: `requests_total`, `latency_seconds{route,code}`, `bytes_{in,out}_total` increment as expected; labels stable.

Run:

```bash
cargo test -p svc-storage --test '*'
```

---

### 1.3 Property-Based Tests (`proptest`)

**Scope:** Parsers, state machines, and chunk boundaries.

* PT-RANGE-01: Arbitrary valid/invalid Range headers → parser never panics; valid inputs round-trip to canonical form; invalid → 416 path.
* PT-ADDR-02: Arbitrary hex strings (0–1KiB) fed into `b3:<hex>` parser → only correct lengths pass; no allocations unbounded.
* PT-CHUNK-03: Any byte slice split by chunker → concatenation of chunks equals input; last chunk ≤ 64 KiB.
* PT-DECOMP-04: Any compressed frame with adversarial headers → guard never exceeds caps; returns error without OOM.

Optional seed snapshot committed under `testing/proptest-regressions/`.

---

### 1.4 Fuzz Tests (`cargo fuzz`)

**Scope:** Wire-adjacent surfaces that touch untrusted input.

**Targets (mandatory):**

* FZ-RANGE: Range header parser (bytes → parse → serialize).
* FZ-ADDR: Address/ETag parser (“b3:” prefix + hex).
* FZ-DECOMP: Decompression guard (tiny constrained decompressor wrapper).

Corpus:

* Seed with real Range headers from integration logs + synthetic extremes.
* Keep crashers and interesting inputs in `fuzz/corpus/*`.

Acceptance:

* CI (nightly): **≥ 1h** per target, **zero** crashes (Silver).
* Scheduled (weekly): **≥ 4h** per target, **zero** crashes (Gold).

Run examples:

```bash
cargo fuzz run range_fuzz -- -max_total_time=3600
cargo fuzz run addr_fuzz  -- -max_total_time=3600
cargo fuzz run decomp_fuzz -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak

**Scope:** Service resilience under faults and long duration.

Scenarios:

* CH-CRASH-01: Kill the process mid-write (SIGKILL) → clean restart; no partials served; integrity holds.
* CH-BUS-02: Simulate bus lag/drops → no deadlocks; health stays green; shed if needed.
* CH-DISK-03: Disk **full** and **slow I/O** → `/readyz` degrades (writes refused), reads still OK; fast 413/503 on writes.
* CH-SLOWLORIS-04: Many half-open connections → backpressure works; no FD leak.
* CH-SOAK-24H: 24-hour mixed workload (70% reads, 20% HEAD, 10% 1MiB writes) → **zero FD/memory leak**, stable p95s.

Artifacts:

* Flamegraphs at T+1h, T+12h, T+24h.
* Heap snapshots or RSS trend.

---

### 1.6 Performance / Load

**Scope:** Validate SLOs and guard against regressions.

Targets (steady state, intra-region):

* **GET p95 < 80 ms**; Range start p95 < 100 ms.
* **PUT p95 < 250 ms** for 1 MiB bodies.
* **Throughput:** ≥ **500 req/s** on 4c/8GiB reference profile.
* **Error budgets:** 5xx < 0.1%, 429/503 < 1% at target load.

Tools:

* `criterion` micro-bench (digest, chunker, decomp guard).
* `testing/performance/*` HTTP rigs (`wrk`, `bombardier`, `gwsmoke`).

Runs:

```bash
cargo bench -p svc-storage
testing/performance/run_load.sh                    # repository script
testing/performance/compare_baselines.sh          # fails if >10% latency regression
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Line coverage **≥ 70%** (`grcov`/`tarpaulin`).
* Fuzz harness builds; at least one seed corpus present.
* Perf smoke: 60s `wrk` run shows stable operation; no panics.

### 2.2 Silver (Useful Substrate)

* Property tests present for Range, address, chunker.
* Fuzz run **≥ 1h**/target in CI (nightly).
* Coverage **≥ 85%** on core modules (addressing, range, io).
* Chaos scripts checked in; 1h soak in CI weekly.
* Perf gate: p95/throughput thresholds enforced vs baselines.

### 2.3 Gold (Ops-Ready)

* Fuzz **≥ 4h**/target (scheduled); zero crashes.
* Soak **24h** in CI (scheduled), no FD/mem leak; flamegraphs uploaded.
* Coverage **≥ 90%** on core; mutation testing (optional) clean on parsers.
* Release trains must include perf delta report; regressions require waivers.

---

## 3) Invocation Examples

### 3.1 Run everything (dev)

```bash
cargo test -p svc-storage --all-targets -- --nocapture
```

### 3.2 Specific suites

```bash
cargo test -p svc-storage --lib
cargo test -p svc-storage --test '*'
```

### 3.3 Fuzz (local quick)

```bash
cargo fuzz run range_fuzz -- -max_total_time=60
```

### 3.4 Loom (concurrency model)

Loom tests live under `tests/loom_*` or `src/*` with `cfg(loom)`.

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-storage --test loom_*
```

### 3.5 Benches

```bash
cargo bench -p svc-storage
```

---

## 4) Observability Hooks

* Tests emit **structured JSON logs** on failure (include `corr_id`, `route`, `latency_ms`, `reason`).
* Integration tests assert key **Prometheus metrics** deltas:

  * `requests_total{route,code}`, `latency_seconds{route}`, `bytes_{in,out}_total`
  * `rejected_total{reason}`, `integrity_fail_total{reason}`
* Every test assigns a **Correlation ID**; `corr_id` is propagated in logs.

---

## 5) CI Enforcement (GitHub Actions outline)

* Build & test:

  * `cargo test --workspace --all-targets`
  * `cargo fmt --all -- --check`
  * `cargo clippy --workspace -- -D warnings`
  * `cargo deny check advisories bans licenses sources`
* Coverage:

  * `grcov` or `tarpaulin` job; fail if below gate.
* Fuzz (nightly):

  * `cargo fuzz run range_fuzz/addr_fuzz/decomp_fuzz -- -max_total_time=3600`
* Perf:

  * Run `testing/performance/run_load.sh`; compare to `testing/performance/baselines/`; fail if p95 ↑ >10% or throughput ↓ >10%.
* Chaos (scheduled):

  * Slow-loris + disk-full scenarios; attach graphs; fail on FD/mem leak or readiness-gate violations.

---

## 6) Test Data & Fixtures

* `testing/fixtures/1MiB.bin` — exactly 1,048,576 bytes; deterministic content (generated in repo).
* `testing/fixtures/range_vectors.json` — canonical header/response tuples.
* `testing/fixtures/zip_bomb.bin` — safe repro of ratio-cap violation (small file that inflates >10×).
* `testing/proptest-regressions/*` — pinned counterexamples.
* `fuzz/corpus/*` — curated inputs per target.

Rebuild 1 MiB fixture:

```bash
dd if=/dev/zero of=testing/fixtures/1MiB.bin bs=1M count=1
```

Compute BLAKE3 (for expected ETag/address in tests):

```bash
b3sum testing/fixtures/1MiB.bin
```

---

## 7) Concurrency/Loom Scope (svc-storage specifics)

**Loom-checked invariants:**

* LM-PACER-01: Pacer + readiness ordering: pacer must not publish ready before caches/dirs OK.
* LM-BACKPRESSURE-02: `max_inflight` semaphore never deadlocks; permits restored after cancellation.
* LM-IO-03: Reader/writer chunk handoff (64 KiB) has no lost or duplicated bytes under interleavings.
* LM-SHUTDOWN-04: Graceful shutdown drains inflight up to idle timeout; no tasks left waiting on poisoned locks.

Execution:

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-storage --test loom_*
```

---

## 8) Fuzz Targets (mandatory list)

* `range_fuzz` — bytes → Range header parser → serialize; assert determinism/no panic.
* `addr_fuzz` — strings → address/ETag parser; reject non-hex/length drift.
* `decomp_fuzz` — adversarial compressed frames → guard; assert bounded output & time; no OOM.

Artifacts: crashes minimized and committed; CI replays on PRs.

---

## 9) Performance SLOs Measured

* p95 GET (intra-region), p95 Range start, p95 PUT (1 MiB).
* Throughput at 4c/8GiB ref.
* Error budget: 5xx and shed 429/503 within bounds.
* Hedge efficacy (if enabled): tail latency reduction vs. baseline.

---

## 10) Bronze→Silver→Gold Checklist (per release)

* [ ] Bronze: unit/integration pass; ≥70% coverage; fuzz builds; perf smoke green.
* [ ] Silver: property tests; fuzz ≥1h; ≥85% coverage; chaos scripts run weekly; perf gates enforced.
* [ ] Gold: fuzz ≥4h (scheduled); 24h soak; ≥90% coverage (core); perf deltas reported; no leaks.

---

## 11) Open Items & Ownership

* Owners: svc-storage maintainers (test triage rotation posted in CODEOWNERS).
* Escalation: SLO breach → open “perf-regression” issue with flamegraph + metrics diff; assign within 24h.
* Waivers: Any gate failure needs a signed waiver with root cause and rollback plan; expires in 14 days.

```

---

