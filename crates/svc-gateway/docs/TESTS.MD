

---

````markdown
# 🧪 TESTS.md — `svc-gateway`

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/Loom compatible)*

---

## 0) Purpose

Define the **test contract** for `svc-gateway`:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals and **Bronze → Silver → Gold** acceptance gates.
* Reproducible invocation commands for devs & CI, with stable baselines.

This test plan enforces:
- **Ingress safety invariants:** bounded body (≤ 1 MiB), decode ratio (≤ 10×), early shed (429/503) under pressure.
- **Control surface invariants:** `/healthz`, `/readyz`, `/metrics`, `/version` present and correct.
- **Degrade-first semantics:** write paths shed before reads when overloaded.
- **Error taxonomy stability:** 400/401/403/405/411/413/415/416/429/503 mapped and JSON-shaped.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100ms)

**Scope**
- Header parsing (Accept-Encoding, Range, Content-Length consistency).
- Capability token envelope parsing (structure only; no crypto semantics here).
- Config parsing/validation (caps/timeouts/“danger_ok” guards).
- Small helper modules (DRR weight math, limiter arithmetic, reason mapping).

**Location**
- `src/**` with `#[cfg(test)]`.

**Run**
```bash
cargo test -p svc-gateway --lib
````

**Key invariants**

* Never panic on malformed headers; return correct taxonomy code.
* Content-Length vs chunked ambiguity → 400.
* `danger_ok=false` in non-dev profiles blocks unsafe caps at startup.

---

### 1.2 Integration Tests (end-to-end)

**Scope**

* Full HTTP ingress → handlers → (mocked) downstream calls.
* Readiness lifecycle, backpressure, and degrade-first behavior.
* Error taxonomy and JSON error body shape (stable fields).
* Config reload semantics (SIGHUP/`/config/reload` if applicable).

**Location**

* `tests/*.rs`

**Must cover**

1. **Control endpoints**

   * `/healthz` → 200 (process alive).
   * `/readyz` → 503 until bus/deps ready; 200 after.
   * `/metrics` exposes golden metrics.
2. **Routing & Happy paths**

   * `GET /o/{addr}`: 200 with small bodies (16/64/256 KiB).
   * Range requests: valid `206` for satisfiable; `416` on invalid ranges.
3. **Safety caps**

   * Body > 1 MiB → `413 body-cap` with structured JSON (`reason`, `cap_bytes`).
   * Encoded body exceeding decode ratio (e.g., gz ~12 MiB from 100 KiB) → `413 decoded-cap`.
4. **Backpressure & Quotas**

   * Spike to 600 rps at default caps → early 429 with `Retry-After` header; sustained reject rate <1% once load returns to steady-state.
5. **Error taxonomy**

   * 400 double `Content-Length`.
   * 401/403 capability missing/invalid.
   * 405 wrong method to route.
   * 411 missing length where required.
   * 415 unsupported `Content-Encoding`.
   * 416 unsatisfiable ranges.
   * 503 when `/readyz` not ready or degrading due to downstream.
6. **Config reload**

   * Flip rate limit and inflight caps; verify new limits applied without process restart.
7. **Degrade-first**

   * Under induced downstream slowness, writes shed (429/503) while reads continue, then `/readyz` flips to 503 when guardrails engage.

**Run**

```bash
cargo test -p svc-gateway --test '*'
```

---

### 1.3 Property-Based Tests

**Tooling**

* `proptest` (preferred) or `quickcheck`.

**Targets & invariants**

* **Header/Path parser**

  * Arbitrary header maps (bounded size) never panic.
  * Normalization is idempotent (normalize(normalize(h)) == normalize(h)).
  * Range parser: for any content length `L`, producing ranges via generator:

    * Valid ranges yield `Some(Satisfiable)` where `start < end ≤ L`.
    * Invalid yield `None` and map to 416.
* **Error JSON envelope**

  * For any taxonomy variant, JSON → struct → JSON round-trips byte-for-byte (stable fields).
* **Limiter arithmetic**

  * Token bucket under any burst pattern respects ceiling; total accepted ≤ configured cap over window.

**Run**

```bash
cargo test -p svc-gateway --features proptest
```

---

### 1.4 Fuzz Tests

**Tooling**

* `cargo fuzz` (libFuzzer, nightly).

**Fuzz targets (examples under `fuzz/fuzz_targets/`)**

* `headers_fuzz.rs` — arbitrary HTTP/1.1 header sequences → parser (no panics, bounded allocations).
* `range_header_fuzz.rs` — byte-range inputs → parser invariants hold.
* `path_fuzz.rs` — percent-encoding, UTF-8 edges; no traversal or UB.
* `gzip_bomb_fuzz.rs` — random compressed blobs → decode guard refuses > ratio/abs-cap and never OOMs.
* `cap_token_fuzz.rs` — capability envelope (structure-only) must not panic; rejects unbounded size.

**Acceptance**

* CI: 1h per-target (Silver).
* Nightly: 4h per-target (Gold).
* **Zero crashes**, **no OOM**, ASan clean.

**Run examples**

```bash
cargo fuzz run headers_fuzz -- -max_total_time=3600
cargo fuzz run gzip_bomb_fuzz -- -rss_limit_mb=4096 -timeout=30
```

---

### 1.5 Chaos / Soak Tests

**Scope**

* Process crashes, restart storms, LB health-eject, bus lag/drops, downstream slowness, and network jitter.

**Inject**

* Kill gateway process randomly during load; verify clean restart, `/readyz` recovery < 15s.
* Add downstream latency/jitter (e.g., 50/200/500 ms P99) and packet loss (1–3%).
* Slow-loris connections (incomplete headers/bodies) to assert timeouts and FD boundedness.
* Decompression bombs directed to decode guard.

**Acceptance**

* **24h soak** at 80% of RPS cap with zero FD leak (fds stable), bounded memory, and no rising reject trend.

**Harness**

* Scripts in `testing/chaos/` using `toxiproxy` or `tc netem`.
* Assertions scrape `/metrics` + logs for regressions.

---

### 1.6 Performance / Load Tests

**Scope**

* Throughput, latency (p50/p95/p99), backpressure behavior, and reject taxonomy under spikes.

**Targets**

* Intra-region GET p95 < 80 ms; inter-region p95 < 200 ms (at Bronze host).
* Spike 600 rps for 60s: early 429s with `Retry-After`, no collapse.
* 64 KiB write p95 ≤ 150 ms @ 400 rps baseline.

**Tools**

* `criterion` for micro benches.
* `bombardier`, `wrk`, `hey` under `testing/performance/`.

**Run examples**

```bash
bombardier -c 128 -d 3m -l -m GET http://$BIND/o/$ADDR
bombardier -c 256 -r 600 -n 30000 -m POST http://$BIND/put --body ./payload_64k.bin
cargo bench -p svc-gateway
```

---

## 2) Coverage & Gates

We use `cargo-llvm-cov` (preferred) or `grcov` to compute coverage.
**Lines/branches** exclude generated code and vendor stubs.

### 2.1 Bronze (MVP)

* Unit + integration tests pass on Linux/macOS.
* Coverage **≥ 70%** (lines).
* Fuzz harness builds (no targets required to run in PR).
* Control endpoints verified (`/healthz`, `/readyz`, `/metrics`, `/version`).

### 2.2 Silver (Useful Substrate)

* Property tests enabled; key invariants under proptest.
* Fuzz **≥ 1h per target** in CI nightly.
* Coverage **≥ 85%** lines, **≥ 70%** branches.
* Chaos scripts present; 1h chaos mini-drill passes (no leaks).

### 2.3 Gold (Ops-Ready)

* Fuzz **≥ 4h per target** nightly; zero crashes over trailing 14 days.
* Soak **24h** at 80% cap; stable fds/memory; tail lat OK.
* Coverage **≥ 90%** lines, **≥ 80%** branches.
* Perf regressions tracked; baselines stored and enforced.

---

## 3) Invocation Examples

### 3.1 All tests (fast path)

```bash
cargo test -p svc-gateway --all-targets -- --nocapture
```

### 3.2 Integration only

```bash
cargo test -p svc-gateway --test '*'
```

### 3.3 Property tests

```bash
cargo test -p svc-gateway --features proptest -- proptest-case=10000
```

### 3.4 Fuzz target (1-hour)

```bash
cargo fuzz run headers_fuzz -- -max_total_time=3600
```

### 3.5 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" RUST_TEST_THREADS=1 cargo test -p svc-gateway --test loom_*
```

### 3.6 Benchmarks

```bash
cargo bench -p svc-gateway
```

---

## 4) Observability Hooks

* **Structured logs**: JSON on failure, correlation IDs (`X-Corr-ID`) threaded through requests.
* **Golden metrics** exposed in tests; assertions consume `/metrics` to verify:

  * `*_latency_seconds` histograms,
  * `rejected_total{reason=...}`,
  * `gateway_degraded`,
  * inflight gauges.
* **Artifacts**: On CI failure, upload:

  * test logs (JSONL),
  * flamegraphs (worst tail run),
  * `/metrics` snapshots,
  * failed response bodies for taxonomy diffs.

---

## 5) CI Enforcement

**PR (quick)**

* `cargo fmt -- --check`
* `cargo clippy -- -D warnings`
* `cargo test --workspace --all-targets`
* `cargo llvm-cov --summary-only --workspace --lcov --output-path coverage.lcov`
* Perf smoke: short `bombardier` run (30–60s) comparing to last-good short baseline
* `cargo deny check advisories bans licenses`

**Nightly (deep)**

* Fuzz: each target `-max_total_time=14400` (4h)
* Soak: 24h chaos at 80% cap (metrics polled)
* Perf: full workload matrix (GET sizes, spikes, decode tests)
* Upload baselines to `testing/performance/baselines/` and trend dashboards

**Fail conditions**

* p95 ↑ >10% or throughput ↓ >10% vs baseline (same host/profile)
* Coverage below gate thresholds
* New taxonomy drift (status or JSON fields mismatch)
* Any fuzz crash/ASan issue

---

## 6) Suite Layout (Recommended)

```
svc-gateway/
├─ src/
│  └─ ... #[cfg(test)] unit tests
├─ tests/
│  ├─ health_ready.rs
│  ├─ routing_happy.rs
│  ├─ error_taxonomy.rs
│  ├─ safety_caps.rs           # 1 MiB body, decode ratio/abs-cap → 413
│  ├─ quotas_backpressure.rs   # spike 600 rps → early 429
│  ├─ degrade_first.rs         # writes shed before reads
│  ├─ config_reload.rs
│  └─ downstream_slow.rs
├─ tests_loom/
│  ├─ loom_backpressure.rs     # bounded queues; no deadlock; cancellation safe
│  └─ loom_shutdown.rs         # graceful drain; no cross-'.await' locks
├─ fuzz/
│  └─ fuzz_targets/
│     ├─ headers_fuzz.rs
│     ├─ range_header_fuzz.rs
│     ├─ path_fuzz.rs
│     ├─ gzip_bomb_fuzz.rs
│     └─ cap_token_fuzz.rs
├─ testing/
│  ├─ performance/
│  │  ├─ workloads.yaml
│  │  ├─ baselines/
│  │  │  └─ svc-gateway_bronze_64k.json
│  │  └─ perf_harness.rs
│  └─ chaos/
│     ├─ toxiproxy_scenarios.yaml
│     └─ soak_runner.sh
└─ scripts/
   ├─ test_all.sh
   ├─ fuzz_all.sh
   └─ perf_smoke.sh
```

---

## 7) Canonical Scenarios (Must Not Regress)

1. **Body overflow**: 1,048,577-byte POST → 413 `body-cap`; memory bounded.
2. **Decode overflow**: 100 KiB gz → ~12 MiB decoded → 413 `decoded-cap`; process steady.
3. **Burst ingress**: 600 rps for 60s → early 429; p95 recovers < 2× baseline within 30s after spike.
4. **Downstream stall**: overlay/index 500 ms P99 → `/readyz` degrades; writes shed; reads continue ≤ target tails.
5. **Ambiguous length**: both `Content-Length` and `Transfer-Encoding: chunked` → 400.
6. **Range edge**: `Range: bytes=-1` on 0-length body → 416; no panic.
7. **Config reload**: halve rate limit; reject curve tracks new ceiling within 1s buckets.
8. **Loom drain**: cancel ~25% of requests mid-flight → no deadlocks, resources freed.

---

## 8) Open Questions (crate-specific answers)

* **Which invariants are loom-checked?**
  Bounded inflight queue; graceful shutdown (no task holds a lock across `.await`); cancellation safety; no double-close on channels; time-bounded readiness flip.

* **Which fuzz targets are mandatory?**
  `headers_fuzz`, `range_header_fuzz`, `gzip_bomb_fuzz` are mandatory; `path_fuzz` and `cap_token_fuzz` are strongly recommended.

* **What SLOs are measured in perf tests?**
  Intra-AZ GET p95 < 80 ms; inter-AZ p95 < 200 ms; 64 KiB write p95 ≤ 150 ms @ 400 rps; sustained rejects < 1% over 5m windows under nominal load.

---

## 9) Developer Quickstart (copy/paste)

```bash
# Fast feedback (unit + integ)
cargo test -p svc-gateway --all-targets -- --nocapture

# Property tests
cargo test -p svc-gateway --features proptest -- proptest-case=10000

# Fuzz (install cargo-fuzz first)
cargo fuzz run headers_fuzz -- -max_total_time=600

# Loom model checks
RUSTFLAGS="--cfg loom" RUST_TEST_THREADS=1 cargo test -p svc-gateway --test loom_*

# Perf smoke (adjust $BIND/$ADDR)
bombardier -c 128 -d 60s -l -m GET http://$BIND/o/$ADDR
```

---

✅ With this TESTS.md, `svc-gateway` has a **clear, enforceable** testing contract that guards safety invariants, tail latency, and taxonomy stability—and scales from fast PR feedback to nightly Gold-tier assurance.

```
```
