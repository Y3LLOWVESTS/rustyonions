

---

````markdown
# 🧪 TESTS.md — ron-accounting

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ron-accounting`:

- **What** we test: unit, integration, property, fuzz, chaos/soak, performance.
- **Why** we test: enforce invariants (amnesia-safe, idempotent exports, bounded queues, degraded readiness semantics).
- **How** we test: deterministic harnesses, reproducible fixtures, CI gates (Bronze→Silver→Gold).

This crate is **transient metering** with **idempotent batch export** to `ron-ledger` over UDS. It must never become a ledger or accumulate durable economic truth.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope**
- Pure functions and narrow modules:
  - time-slice bucketing (`bucket_for(ts, window)`),
  - aggregation math (sum/min/max/count, overflows guarded),
  - idempotency key generation (stable across retries),
  - bounded queue math (capacity, backpressure thresholds),
  - config parsing + defaults (no panics on unknown keys).

**Location**
- Inline under `src/**` guarded by `#[cfg(test)]`.

**Examples**
```bash
cargo test -p ron-accounting --lib
````

**Must-cover assertions**

* No panics on malformed but non-fatal inputs (graceful reject paths).
* Arithmetic is saturating/checked where specified.
* `IdempotencyKey::new(batch)` is stable for identical logical content.

---

### 1.2 Integration Tests

**Scope**

* End-to-end crate surface in `tests/*.rs` using a **mock ledger** over UDS and a **test bus**.
* Scenarios:

  1. **API round-trip**: ingest metering → aggregate → export → mock-ledger append; verify counts.
  2. **Config reload**: change policy/limits → `ConfigUpdated` emitted → new behavior takes effect without process restart.
  3. **Readiness semantics**: ledger down → `/readyz=503` (writes degraded, reads OK); ledger back → `/readyz=200`.
  4. **Backpressure**: hit queue caps → `rejected_total{reason="overload"}` increments and requests are shed (no deadlock).
  5. **Shutdown flush**: SIGINT/SIGTERM → in-flight export flushed within `FLUSH_TIMEOUT` and exits.

**Examples**

```bash
cargo test -p ron-accounting --test '*'
```

**Fixtures**

* `tests/fixtures/uds/` (socket path setup/teardown).
* `tests/fixtures/policy.toml` (quota variations).

---

### 1.3 Property-Based Tests

**Scope**

* Parsers/encoders and state transitions with adversarial inputs using `proptest`.

**Targets & Invariants**

* **Batch encode/decode**: round-trip property (`decode(encode(x)) == x`) for batches of counters with random tenant IDs and timestamps.
* **Idempotency**: exporting the same logical batch any number of times commits **exactly once** (mock ledger enforces `idempotency_key` uniqueness).
* **Time-slice bucketing**: any timestamp jitter within ±(window/2) remains in deterministic bucket given the configured window policy.
* **Aggregation commutativity/associativity**: split/merge chunking strategies yield identical results.

**Examples**

```bash
cargo test -p ron-accounting --test prop_* -- --nocapture
```

---

### 1.4 Fuzz Tests

**Scope**

* Wire-adjacent boundaries and formats (not general HTTP fuzzing here; focus on internal batch codecs and capability token parsing if present).

**Tooling**

* `cargo fuzz` with targets:

  * `fuzz_targets/batch_codec.rs`
  * `fuzz_targets/policy_parser.rs` (if custom parsing layer exists)
  * `fuzz_targets/cap_token.rs` (optional; verify no panics, robust erroring)

**Acceptance**

* Bronze: builds + smoke (60s).
* Silver: ≥ 1h in CI, zero crashes/ooms.
* Gold: ≥ 4h nightly, corpus minimized and checked in.

**Examples**

```bash
cargo fuzz run batch_codec -- -max_total_time=60
```

---

### 1.5 Chaos / Soak Tests

**Scope**

* Service behavior under faults; validates supervision, bounded queues, and degraded modes.

**Fault injections**

1. **Ledger outage**: remove/close UDS; expect `/readyz=503`, bounded `export_backlog_gauge`, no OOM; auto-recovery on return.
2. **Export latency**: `tc netem` on loopback; p95 export latency remains within SLO; backlog slope bounded.
3. **Kill under load**: `SIGKILL` process; supervisor restart with jitter; no duplicate ledger records (idempotency).
4. **Time skew**: advance clock by +2m, then resync NTP; slice boundaries remain consistent; reconciliation passes.
5. **Disk pressure** (if WAL/cache feature is enabled): fill disk to threshold; verify WAL corruption counters and safe fallback.

**Acceptance**

* **24h soak**: zero FD leaks (check `lsof` delta), resident set stable within budget, no restart storms, chaos alerts fired & recovered.

**Examples**

```bash
# simulate ledger drop (in test env)
rm -f "${RON_LEDGER_SOCK:-$XDG_RUNTIME_DIR/ron/ledger.sock}" || true
# latency
tc qdisc add dev lo root netem delay 200ms 50ms ; sleep 60 ; tc qdisc del dev lo root
# kill under load
pkill -9 ron-accounting ; systemctl start ron-accounting
```

---

### 1.6 Performance / Load Tests

**Scope**

* Throughput, latency, and queueing under realistic distributions (multi-tenant, bursty).

**SLOs (test-time)**

* Export latency **p95 < 150ms** steady, **< 500ms** under 5-minute peaks.
* `export_backlog_gauge < 1000` for 99.9% of minutes in a 1-hour run.
* Zero data loss: nightly idempotent reconciliation = 100%.

**Tooling**

* `criterion` micro-bench for hot paths (bucket, aggregate, encode).
* External driver (e.g., k6 or a Rust loadgen in `testing/loadgen/`) that:

  * Spawns N tenants with Poisson bursts,
  * Randomizes request sizes,
  * Injects occasional ledger pauses.

**Examples**

```bash
cargo bench -p ron-accounting
cargo run -p testing-loadgen -- --rps 500 --tenants 64 --duration 900s
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Coverage (line) **≥ 70%** (grcov or tarpaulin).
* Fuzz harness builds; one target runs for ≥ 60s in CI.

### 2.2 Silver (Useful Substrate)

* Property tests included (batch codec + idempotency).
* Fuzz runs **≥ 1h** (key targets).
* Coverage **≥ 85%** (line), function coverage report attached to CI artifact.
* Chaos scripts present and green in non-prod env (ledger drop + latency).

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h**; zero crashes across week; corpus minimized.
* **24h soak**: no FD/memory leaks; restart storm alert never fires.
* Coverage **≥ 90%** (line), **≥ 75%** (branch) on critical modules.
* Performance regression tracked **release-to-release**; p95 budgets enforced by CI.

---

## 3) Invocation Examples

### 3.1 All Tests (crate)

```bash
cargo test -p ron-accounting --all-targets -- --nocapture
```

### 3.2 Integration Only

```bash
cargo test -p ron-accounting --test '*'
```

### 3.3 Property Tests

```bash
cargo test -p ron-accounting --test prop_* -- --nocapture
```

### 3.4 Fuzz Target (smoke)

```bash
cargo fuzz run batch_codec -- -max_total_time=60
```

### 3.5 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-accounting --test loom_* -- --nocapture
```

### 3.6 Benchmarks

```bash
cargo bench -p ron-accounting
```

---

## 4) Observability Hooks

* Tests use **structured JSON logs** with `corr_id` propagation; failures print the last N spans for the relevant `corr_id`.
* On integration/soak tests:

  * Scrape `:9600/metrics` and attach to test artifacts (Prom text and summarized CSV).
  * Save `/readyz` samples at 1s intervals to detect flapping.
* PromQL assertions (example; run via test helper or CI step):

  * `histogram_quantile(0.95, rate(ron_accounting_export_latency_seconds_bucket[5m])) < 0.15`
  * `increase(ron_accounting_rejected_total{reason="overload"}[5m]) == 0` (steady test)
  * `max_over_time(ron_accounting_export_backlog_gauge[10m]) < 1000`

---

## 5) CI Enforcement

* **Build & Format**

  * `cargo test --workspace --all-targets`
  * `cargo fmt -- --check`
  * `cargo clippy --workspace --all-targets -- -D warnings`

* **Security / Supply Chain**

  * `cargo deny check advisories bans sources licenses`

* **Coverage**

  * `grcov` (Linux): generate lcov and upload artifact; gate on thresholds per Bronze/Silver/Gold.

* **Fuzz (nightly)**

  * Matrix job runs `cargo fuzz` for key targets with `-max_total_time=14400` at Gold.

* **Chaos (non-prod env)**

  * Ephemeral VM job runs ledger-drop + latency drill; collects metrics and verifies PromQL SLOs.

* **Performance**

  * Criterion results compared against previous release; CI fails on >10% regression p95 unless waived.

---

## 6) Loom (Concurrency Model) — What We Check

* **Shutdown-while-exporting**: no await-holding-lock; flush completes or times out cleanly.
* **Bounded queues**: producers block or shed as configured; no lost-wakeups.
* **Readiness flip**: transitions between `ready`↔`degraded` cannot deadlock exporters or readers.
* **Backpressure propagation**: when ledger stalls, exporters slow; ingestion sheds before unbounded growth.

**Examples**

* `tests/loom_shutdown.rs`
* `tests/loom_backpressure.rs`

---

## 7) Fuzz Targets — What’s Mandatory

* `batch_codec` (mandatory): batch framing + varint/length limits; reject over-long frames; ≤ 1 MiB gate if applied.
* `policy_parser` (mandatory if custom parser exists): unknown/duplicate fields → deterministic errors; no panics.
* `cap_token` (optional): capability token parsing/validation (length, charset, prefix).

Artifacts include minimized corpus and dictionary files under `fuzz/corpus/*`.

---

## 8) Performance SLOs — What We Measure

* **Export latency p95**: target **< 150ms** steady, **< 500ms** peak (5-minute).
* **Backlog bound**: `export_backlog_gauge < 1000` for 99.9% minutes (1-hour test).
* **Degraded write rate**: `increase(rejected_total{reason="degraded"}[30m]) < 1000` in ledger-outage drill.
* **Zero data loss**: post-test reconciliation equals expected aggregates.

Harness sources live under `testing/`:

* `testing/loadgen/` (Rust load driver)
* `testing/analyze/` (PromQL assertions + report)

---

## 9) Open Questions (fill before Gold)

* Which policy knobs are part of property tests (window_len_s, max_labels)?
* Do we enforce a **hard byte/frame** cap per batch in codec? (If yes, add fuzz dict for boundary values.)
* WAL/cache feature: which corruption scenarios are in chaos tests (fsync drop, partial write)?
* Cross-crate story: should we add **joint soak** with `ron-ledger` to validate end-to-end idempotency at scale?

---

## 10) Quickstart Cheatsheet

```bash
# Unit + integration
cargo test -p ron-accounting --all-targets -- --nocapture

# Property tests
cargo test -p ron-accounting --test prop_* -- --nocapture

# Fuzz (quick smoke)
cargo fuzz run batch_codec -- -max_total_time=60

# Loom model checks
RUSTFLAGS="--cfg loom" cargo test -p ron-accounting --test loom_* -- --nocapture

# Benchmarks
cargo bench -p ron-accounting
```

---

## 11) Bronze → Silver → Gold Checklist

* [ ] **Bronze**: ≥70% line cov; unit+integ pass; fuzz builds & 60s smoke green.
* [ ] **Silver**: ≥85% line cov; property tests land; fuzz ≥1h; chaos scripts pass (ledger drop + latency).
* [ ] **Gold**: ≥90% line, ≥75% branch on critical paths; fuzz ≥4h nightly; 24h soak clean; perf SLOs green; regression dashboard attached.

---

```

