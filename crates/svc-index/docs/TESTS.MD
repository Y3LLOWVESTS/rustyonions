

---

# 🧪 TESTS.md — `svc-index`

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/Loom compatible)*

---

## 0) Purpose

Define the **test contract** for `svc-index`:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals and **Bronze → Silver → Gold** acceptance gates.
* Canonical invariants and reproducible invocation for devs & CI.

This crate resolves **name / `b3:<hex>` → manifest → provider set**, with cache + sled DB, and **hedged DHT lookups** under hop caps. Tests must assert **readiness truth**, **error taxonomy**, **limits/quotas**, **metrics presence with bounded labels**, and **copy-before-await** concurrency safety.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure modules and fast paths (<100 ms): caching helpers, DTO shaping, capability checks (if any), error mappers, metrics emitters.

**Key invariants (unit):**

* DTO encode/decode stable; schema changes are additive (no breaking rename).
* Error taxonomy maps to stable HTTP codes (e.g., `NotFound`, `Invalid`, `TooLarge`, `Throttled`, `Deadline`).
* Cache TTL & size math (LRU/LFU policy functions) correct and monotonic.
* **Label bounds** respected by metric emitters: `{route} ≤ 10`, `{type} ∈ {name,cid,facet}`.

**Location / Run:**

* `src/**/*_test.rs` or `#[cfg(test)]` modules near code.

```bash
cargo test -p svc-index --lib
```

---

### 1.2 Integration Tests

**Scope:** End-to-end crate surface under real Tokio runtime: HTTP resolve, readiness gates, config reload, DHT hedging, sled DB, metrics.

**Must include (contract):**

* **Resolve round-trip:** `GET /resolve?name=<n>` returns manifest & provider set; 404 on unknown; stable JSON/CBOR shape.
* **/readyz truth:** 503 with reasons until {config, DB, bus, DHT} healthy; 200 afterward; degrade to 503 if any dep fails.
* **Config reload:** SIGHUP or API reload flips limits (RPS, inflight, deadlines) **without dropping inflight**; emits `ConfigUpdated`.
* **Concurrency invariants:** bounded inflight, backpressure returns `429/503` (structured); graceful shutdown emits `Shutdown`.
* **DHT hedging:** second wave only when first wave tail detected; **no double-counted results**; cancellations don’t leak tasks.
* **Cache behavior:** warm → cold sweep → re-warm; negative caching optional and bounded.
* **Metrics & labels:** required metrics present; label sets bounded; no high-cardinality expansions.
* **Amnesia mode:** no on-disk writes when `RON_AMNESIA=1`; zeroization hook exercised on shutdown.

**Recommended file map:**

```
tests/
  resolve_happy.rs
  readyz_degraded.rs
  config_reload.rs
  dht_hedge.rs
  cache_warm_cold.rs
  metrics_labels.rs
  amnesia_mode.rs
  limits_quota_deadlines.rs
```

**Run:**

```bash
cargo test -p svc-index --test '*'
```

---

### 1.3 Property-Based Tests

**Scope:** Codec round-trips (JSON/CBOR), request parsers, merge of provider sets, cache keying, and small state machines (hedge controller).

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants (examples):**

* **Round-trip:** `decode(encode(x)) == x` for DTOs (with canonicalization if needed).
* **Idempotent merges:** merging provider sets is associative/commutative/idempotent; no duplicates after normalize.
* **Parser safety:** arbitrary inputs never panic; invalid inputs yield structured errors.
* **Budgeting:** per-leg deadlines sum ≤ end-to-end budget; hedge never violates budget.

**Location / Run:**

* `tests/prop_*.rs` or `proptests/*.rs`

```bash
cargo test -p svc-index --test prop_*
```

---

### 1.4 Fuzz Tests

**Scope:** Wire-facing surfaces & parsers: HTTP resolve handler, JSON/CBOR DTOs, DHT response decoder, and any OAP envelope used here.

**Tooling:** `cargo fuzz` (nightly toolchain; allowed as CI nightly job).

**Targets (recommended):**

```
fuzz/fuzz_targets/
  dto_roundtrip.rs        # JSON/CBOR → DTO → JSON/CBOR
  http_resolve.rs         # random paths/queries/headers → handler
  dht_response.rs         # arbitrary peer replies → decoder/merger
```

**Acceptance:** No crashes/UB after:

* **CI nightly:** ≥ 1h per target (Silver).
* **Nightly long-run:** ≥ 4h per target (Gold).

**Run (example):**

```bash
cargo fuzz run dto_roundtrip -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak Tests

**Scope:** Service behavior under faults & duration: process crashes, bus lag/drops, peer churn, disk pressure, slow I/O, cache-cold storms.

**Faults injected (must cover):**

* **Latency on DHT path** (+200 ms p95): hedging maintains SLOs; hop cap respected.
* **Cache-cold sweep:** evict ≥ 80% hot set; p95 recovers < 3 min.
* **Slow-loris & decompression bomb:** structured rejects; process stable.
* **Disk near-full on sled root:** readiness degrades truthfully.
* **24h soak:** zero FD/memory leak creep; stable `/readyz`.

**Run (examples):**

```bash
ronctl chaos inject --target svc-index --fault=latency --arg=dht:+200ms --duration=10m
ronctl chaos inject --target svc-index --fault=cache_cold --arg=0.8 --duration=5m
ronctl chaos soak   --target svc-index --duration=24h
```

**Acceptance:** All pass criteria in RUNBOOK.md §9; logs/metrics artifacts attached to CI.

---

### 1.6 Performance / Load Tests

**Scope:** Throughput, latency, quotas, and regression gates—aligned with PERFORMANCE.md.

**Workloads (minimum):**

* **Hot/cold mix:** 80/20 key distribution; fixed RPS steps.
* **Hedged lookups:** α/β fan-out under tail injection.
* **Limits:** body cap (1 MiB), inflight (512), deadlines (5 s) enforced.

**SLOs measured:**

* p95 intra-region `< 80 ms`, inter-region `< 200 ms`.
* Error budget: 5xx `< 0.1%`, throttle 429/503 `< 1%`, cache hit ratio `> 0.8`.

**Tooling & harness:**

* `criterion` for micro-bench (allocs/op).
* `testing/performance/` scripts for `wrk`, `bombardier`, `gwsmoke`.

```bash
cargo bench -p svc-index
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Coverage ≥ 70% (line or region).
* Fuzz harness builds; chaos scripts exist (not necessarily run in CI).

### 2.2 Silver (Useful Substrate)

* Property tests included for DTOs, hedging controller, mergers.
* Fuzz run ≥ 1h/nightly per target, zero crashes.
* Coverage ≥ 85%.
* Chaos tests scripted & invoked for at least one latency and one cache-cold scenario.
* Metrics presence & **label bounds** asserted in tests.

### 2.3 Gold (Ops-Ready)

* Fuzz run ≥ 4h/nightly per target, zero crashes.
* 24h soak in CI staging: zero FD/memory leak creep; truthful `/readyz`.
* Coverage ≥ 90% (excluding auto-derived code).
* Performance regression tracked **release-to-release** with baselines; fail if p95 ↑ >10% or throughput ↓ >10% at fixed error budget.

---

## 3) Invocation Examples

### 3.1 All tests (fast)

```bash
cargo test -p svc-index --all-targets -- --nocapture
```

### 3.2 Property tests only

```bash
cargo test -p svc-index --test prop_*
```

### 3.3 Fuzz target (1 minute smoke)

```bash
cargo fuzz run http_resolve -- -max_total_time=60
```

### 3.4 Loom (concurrency model checks)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-index --test loom_*
```

### 3.5 Benchmarks

```bash
cargo bench -p svc-index
```

---

## 4) Canonical Invariants (must be tested)

1. **Readiness truth:** `/readyz` returns 200 only if {config, DB, bus, DHT} healthy; degrade with `Retry-After` and reasons on failure.
2. **Error taxonomy:** Stable mapping: `NotFound`→404, `Invalid`→400, `TooLarge`→413, `Throttled`→429, `Deadline`→504, `Internal`→500.
3. **Limits & quotas:** request deadline 5 s; inflight ≤ 512; body cap 1 MiB; decompress ≤ 10×; over-limit requests are rejected early, logged, and metered (`rejected_total{reason}`).
4. **Hedged DHT lookups:** α first wave then β; p99 hop cap ≤ 5; no duplicate providers; cancellations do not leak tasks.
5. **Cache correctness:** TTL honored; no stale after TTL; negative cache (if enabled) expires; miss storms do not starve other routes.
6. **DB discipline:** single sled root; **copy-before-await** on iterators; integrity failures are surfaced and gate readiness.
7. **Metrics canon:** presence and **bounded labels** for `request_latency_seconds`, `rejected_total{reason}`, `index_resolve_latency_seconds{type}`, `index_dht_lookup_ms`, `index_cache_{hits,misses}_total`, `inflight_requests{route}`, `bus_overflow_dropped_total==0`.
8. **Amnesia mode:** `RON_AMNESIA=1` yields no durable writes; cache lives in RAM; shutdown triggers zeroization hook.
9. **Compatibility:** DTOs remain backward-compatible within minor versions; property tests enforce decode of older golden vectors.
10. **Security posture (light tests):** TLS optional paths start and serve; secrets never logged; capability checks (if enabled) reject unauth.

---

## 5) Observability Hooks

* Tests must emit **structured logs** on failure with `corr_id`, route, and reason; see `OBSERVABILITY.md`.
* Integration tests assert Prometheus text has required metrics and forbidden label bloat.
* Chaos & soak attach:

  * flamegraph (hotspots),
  * tokio-console snapshots (blocked tasks, semaphore pressure),
  * FD/memory usage series.

---

## 6) CI Enforcement

**Recommended jobs (GitHub Actions):**

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --all-targets -- --nocapture
      - run: cargo fmt -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo deny check

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --workspace --lcov --output-path lcov.info --fail-under-lines 85

  fuzz-nightly:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run dto_roundtrip -- -max_total_time=3600

  soak-staging:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: ronctl chaos soak --target svc-index --duration=24h
```

**Perf regressions:** separate workflow compares p95/throughput to baselines in `testing/performance/baselines/`; fail on >10% drift.

---

## 7) Loom (Concurrency) Plan

**Targets:**

* **Hedge controller:** races between α completion and β spawn; ensure cancellation paths never deadlock or leak.
* **Copy-before-await:** prove no iterator is held across awaits during DB scans (model with mock DB).
* **Shutdown gate:** ensure `Shutdown` drains in-flight tasks without orphaning background work.

**Run:**

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-index --test loom_*
```

---

## 8) Golden Vectors & Fixtures

* `testing/vectors/dto/*.json|*.cbor` — manifests, provider sets, and edge cases (empty, huge, unicode).
* `testing/fixtures/peers/*.json` — DHT peer reply samples including timeouts/partial data.
* **Rule:** never change existing vectors in place; **append** and bump vector version notes in `docs/api-history/svc-index/`.

---

## 9) Coverage Map (what counts toward thresholds)

* **Included:** resolve handler, DTO codecs, cache policy, hedging controller, readiness, metrics emitters.
* **Excluded:** auto-derived serde impls, trivial boilerplate.
* **Stretch:** platform-specific (aarch64/x86_64) branches for Micronode amnesia.

---

## 10) Bronze → Silver → Gold Gate Checklist

**Bronze**

* [ ] Unit + integration green; `/readyz` truth tests present.
* [ ] Coverage ≥ 70%.
* [ ] Fuzz targets compile.
* [ ] Metrics presence test exists.

**Silver**

* [ ] Property tests for DTO round-trip and provider-merge.
* [ ] Fuzz ≥ 1h/nightly per target with zero crashes.
* [ ] Coverage ≥ 85%.
* [ ] Chaos latency and cache-cold tests scripted and run weekly.
* [ ] Label-bounds test enforces `{route} ≤ 10`, `{type}∈{name,cid,facet}`.

**Gold**

* [ ] Fuzz ≥ 4h/nightly zero crashes (all targets).
* [ ] 24h soak: zero FD/mem leak; truthful `/readyz` throughout.
* [ ] Coverage ≥ 90%.
* [ ] Perf baselines locked; CI fails on >10% p95 or throughput regressions.
* [ ] Golden vectors versioned; decode old versions passes.

---

## 11) Open Questions (crate-specific decisions to finalize)

* **Loom scope:** exact set of DB iterator sites to model (list functions once code lands).
* **Mandatory fuzz targets:** confirm `http_resolve` and `dto_roundtrip` are **required**, `dht_response` optional if client fully mocked.
* **Perf SLO focus:** confirm inter-region p95 workload and peer churn parameters to reflect production.
* **Negative cache policy:** whether to keep and how to bound TTL to avoid poisoning.

---

### Commands summary (for humans)

* Fast loop: `cargo test -p svc-index --all-targets`
* Property: `cargo test -p svc-index --test prop_*`
* Loom: `RUSTFLAGS="--cfg loom" cargo test -p svc-index --test loom_*`
* Fuzz (1 min): `cargo fuzz run http_resolve -- -max_total_time=60`
* Benches: `cargo bench -p svc-index`
* Chaos: `ronctl chaos inject --target svc-index --fault=latency --arg=dht:+200ms --duration=10m`

---

With this contract, `svc-index` testing is **auditable, reproducible, and CI-enforceable**. It locks the core invariants (readiness, hedging, cache, limits, metrics) and gives operators confidence through chaos and soak evidence while keeping developer loops fast.
