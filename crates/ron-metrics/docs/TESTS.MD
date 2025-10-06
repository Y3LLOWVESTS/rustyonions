
---

# 🧪 TESTS.md — ron-metrics

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ron-metrics`:

* Unit, integration, property, fuzz (targeted), chaos/soak (lightweight for exposer), and performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Reproducible commands for devs & CI.
* **Contract checks** for the metrics taxonomy (suffixes) and **base labels** (`service, instance, build_version, amnesia`).
* Readiness/health **schema checks** and GET-only **method policy**.

Key invariants under test:

* Metrics families are **registered once**, handles are **clone-cheap**, hot path is **alloc-free**.
* **GET-only** endpoints: `/metrics`, `/healthz`, `/readyz` (non-GET → 405, bodies rejected).
* `/readyz` **degrades before collapse** and returns **JSON** with `Retry-After`.
* **Suffix discipline**: only `*_seconds`, `*_bytes`, `*_total`.
* **PQ/ZK** families exist (values may be zero).
* Optional **OTLP egress** (feature `otel`) doesn’t change the Rust API and maps 1:1 attributes.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** small, fast (<100 ms) logic and helpers.

**Recommended files:**

* `src/labels.rs` → base label injection helpers.
* `src/taxonomy.rs` → suffix/low-cardinality guards.
* `src/ready.rs` → readiness JSON encoding.

**Run:**

```bash
cargo test -p ron-metrics --lib
```

**Examples to include:**

* `labels::base_labels_present_on_first_party()` (pure function).
* `taxonomy::suffixes_ok()` → param cases for `_seconds/_bytes/_total`.
* `ready::json_shape()` → serde JSON round-trip; optional fields.

---

### 1.2 Integration Tests

**Scope:** end-to-end surface in `tests/*.rs`, async with Tokio.

**Must include:**

* **Exposer smoke:** bind port `0`, start `Metrics::serve`, GET each endpoint:

  * `/healthz` → 200
  * `/readyz` (ready) → 200 with `{"degraded":false,...}`
  * `/readyz` (degraded) → `503` + `Retry-After` and JSON with `missing=[..]`
* **Taxonomy & base labels:** GET `/metrics`, assert:

  * all first-party families end in allowed suffixes,
  * base labels present on **every** first-party line.
* **PQ/ZK presence:** lines exist (values may be `0`).
* **Method policy:** POST/PUT/PATCH/DELETE → `405`.
* **Large registry mode:** when `RON_METRICS_LOAD=large`, ensure:

  * output is parsable,
  * p95 exposition time within PERF budgets (see PERFORMANCE.md),
  * size << 4 MiB (warn at 2 MiB).
* **Amnesia truthfulness:** set `RON_AMNESIA=on|off` and assert label values switch accordingly.
* **OTLP egress (feature `otel`, optional):** spin a dummy collector or HTTP test server; assert requests carry mapped attributes (resource: `service.name`, `service.instance.id`, `service.version`, `ron.amnesia`).

**Suggested files:**

* `tests/exposer_smoke.rs`
* `tests/readyz_json.rs`
* `tests/taxonomy_labels.rs`
* `tests/method_policy.rs`
* `tests/large_registry.rs`
* `tests/otel_export.rs` (cfg(feature = "otel"))

**Run:**

```bash
cargo test -p ron-metrics --test '*'
```

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants:**

* **Readiness JSON**:

  * boolean and `missing` set arbitrary (deduped) → always encodes/decodes; no panics.
  * `retry_after = 0` iff `degraded = false` (property).
* **Metrics line format** (Prometheus text v0):

  * arbitrary safe label values → lines remain ASCII-safe and escapings correct,
  * base labels always injected,
  * no forbidden suffixes appear.
* **Monotonic counters**: multiple increments never decrease.

**Suggested file:**

* `tests/prop_contracts.rs`

**Run:**

```bash
cargo test -p ron-metrics --features proptest -- --nocapture
```

---

### 1.4 Fuzz Tests

**Scope:** HTTP **request path** hardening + readiness encoder.

**Tooling:** `cargo fuzz`.

**Targets:**

* `fuzz_targets/http_method_fuzz.rs`

  * Mutated methods/headers/bodies against the tiny router → never panic; only GET allowed; no body parsing path reaches UB.
* `fuzz_targets/readyz_json_fuzz.rs`

  * Random states → encode→parse (serde) round-trip; no panics, bounded size.

**Acceptance:** **4h** dedicated run locally/nightly with **zero** crashes/ooms.

**Run (examples):**

```bash
cargo fuzz run http_method_fuzz -- -max_total_time=120
cargo fuzz run readyz_json_fuzz -- -max_total_time=120
```

---

### 1.5 Chaos / Soak Tests

**Scope:** light for a library/exposer; ensure resilience to abusive clients.

**Scenarios:**

* **Slow-loris** (headers dribble) → timeouts close sockets; no task leak.
* **Burst scrapes** → keep-alive preserves tails; readiness stays 200.
* **Large registry soak (10k series)** → 24h at low RPS, **zero FD/memory leaks**.

**Harness:** `testing/chaos/slow_loris.py` or equivalent k6 scenario.
**Acceptance:** 24h soak green; inflight tasks return to baseline.

---

### 1.6 Performance / Load Tests

**See `PERFORMANCE.md` for SLOs**, and keep these hooks here to run from CI/dev:

* **Criterion** micro-bench:

  * `benches/hotpath_bench.rs` → counter/histogram ops (alloc-free).
  * `benches/exposer_bench.rs` → encode `/metrics` small vs large registries.
* **Loopback load (bombardier):**

  * `/metrics` small: p95 ≤ 5–10 ms at given conc.
  * `/metrics` large(10k): p95 ≤ 60–120 ms at given conc.
  * `/readyz`: p95 ≤ 3–4 ms; not-ready path ≤ 4 ms.

**Run:**

```bash
cargo bench -p ron-metrics
bombardier -c 16 -d 30s -l http://127.0.0.1:9100/metrics
RON_METRICS_LOAD=large bombardier -c 4 -d 30s -l http://127.0.0.1:9100/metrics
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass (Linux x86_64 at minimum).
* Line coverage **≥ 70%** (tarpaulin/grcov).
* Fuzz harness builds and runs for ≥ 5 minutes smoke.

### 2.2 Silver (Useful Substrate)

* Property tests present and green.
* Fuzz runs **≥ 1h** in nightly CI (both targets).
* Coverage **≥ 85%**.
* Chaos tests scripted and runnable (slow-loris + burst).

### 2.3 Gold (Ops-Ready)

* Fuzz runs **≥ 4h** nightly with **zero** crashes.
* Soak **24h**: no FD/memory leaks; tails within budgets.
* Coverage **≥ 90%** line / **≥ 80%** branch on core modules.
* Perf regression tracked **release-to-release** (p95/throughput/CPU/RSS thresholds from `PERFORMANCE.md`); CI fails on drift.

---

## 3) Invocation Examples

### 3.1 All Tests (library + integration)

```bash
cargo test -p ron-metrics --all-targets -- --nocapture
```

### 3.2 Feature-gated (OTLP egress)

```bash
cargo test -p ron-metrics --features otel --test otel_export
```

### 3.3 Loom (concurrency model)

> Validates “register once, clone everywhere”, no lock across `.await`, graceful shutdown.

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-metrics --test loom_*
```

### 3.4 Fuzz targets

```bash
cargo fuzz run http_method_fuzz -- -max_total_time=60
cargo fuzz run readyz_json_fuzz -- -max_total_time=60
```

### 3.5 Benchmarks

```bash
cargo bench -p ron-metrics
```

---

## 4) Observability Hooks

* Tests emit **structured JSON logs** on failure (enable with `RUST_LOG` + `test-log` or `tracing-test`):

  * `RUST_LOG=info,ron_metrics=trace cargo test -p ron-metrics -- --nocapture`
* Correlation IDs:

  * Use a per-test `corr_id` span; include in log events for cross-crate debugging.
* Artifacts:

  * Save `/metrics` and `/readyz` bodies to `target/test-artifacts/<test-name>/` when assertions fail to aid reproduction.
  * Optional: run `promtool check metrics` if `PROMTOOL` env points to a binary (test skips otherwise).

---

## 5) CI Enforcement

**Core jobs (Linux, MSRV matrix):**

* `cargo fmt -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --workspace --all-targets`
* **Coverage** (Linux): `grcov` or `tarpaulin --engine llvm --timeout 1200`
* **Advisories**: `cargo deny check advisories bans licenses`
* **Public API diff** (informational, gate on approval): `cargo public-api --simplified --deny-changes -p ron-metrics`
* **Perf smoke** (loopback 30s, small + large registry) — parse bombardier JSON; fail on thresholds breaches.
* **Fuzz (nightly)**: run each target ≥ 1h; upload crashes/corpus.
* **Loom**: `RUSTFLAGS="--cfg loom"` for `tests/loom_*`.

**Caching & artifacts:**

* Cache cargo + criterion target dirs.
* Upload failure artifacts: `/metrics` snapshot, `/readyz` JSON, flamegraphs, bombardier outputs.

---

## 6) ron-metrics Specifics (answers to “Open Questions”)

**Which invariants are loom-checked?**

* **Single registration**: creating `Metrics` twice in a loom model either dedupes or errors deterministically; no double-register races.
* **No lock across `.await`**: loom model around the minimal handler ensures no `Mutex` held across awaits.
* **Graceful shutdown**: `serve()` returns a `JoinHandle` that completes without leaked tasks under shutdown ordering permutations.

**Which fuzz targets are mandatory?**

* `http_method_fuzz` (required): malformed/hostile methods/headers/bodies do not panic; non-GET → 405.
* `readyz_json_fuzz` (required): arbitrary state serializes to bounded JSON and round-trips.

**What SLOs are measured in perf tests?**

* `/metrics` p95 (small/large registry), `/readyz` p95 (ready & not-ready), CPU/RSS at target RPS, and text size budget (warn at 2 MiB, cap << 4 MiB). Thresholds mirror `PERFORMANCE.md`.

---

## 7) File Layout (suggested)

```
crates/ron-metrics/
├─ src/
│  ├─ lib.rs
│  ├─ labels.rs
│  ├─ ready.rs
│  └─ taxonomy.rs
├─ tests/
│  ├─ exposer_smoke.rs
│  ├─ readyz_json.rs
│  ├─ taxonomy_labels.rs
│  ├─ method_policy.rs
│  ├─ large_registry.rs
│  ├─ otel_export.rs            # cfg(feature="otel")
│  └─ loom_concurrency.rs       # cfg(loom)
├─ benches/
│  ├─ hotpath_bench.rs
│  └─ exposer_bench.rs
├─ fuzz/
│  └─ fuzz_targets/
│     ├─ http_method_fuzz.rs
│     └─ readyz_json_fuzz.rs
└─ testing/
   ├─ chaos/slow_loris.py
   └─ performance/ (harness + baselines)
```

---

## 8) Bronze → Silver → Gold DoD (Quick Checklist)

* **Bronze**

  * [ ] Unit + integration tests pass on MSRV.
  * [ ] Coverage ≥ 70%; fuzz smoke OK.

* **Silver**

  * [ ] Property tests green.
  * [ ] Nightly fuzz 1h each; chaos scripts runnable.
  * [ ] Coverage ≥ 85%.

* **Gold**

  * [ ] Nightly fuzz 4h; 24h soak w/o leaks.
  * [ ] Coverage ≥ 90% line / ≥ 80% branch core modules.
  * [ ] Perf drift gates enforced; artifacts archived.

---

### Handy Commands

```bash
# Everything (fast path)
cargo fmt -- --check && cargo clippy --all-targets -- -D warnings && cargo test -p ron-metrics --all-targets

# Taxonomy/base-label contract check
curl -fsS http://127.0.0.1:9100/metrics \
 | grep -E '^[a-zA-Z_:][a-zA-Z0-9_:]*' \
 | awk '{print $1}' \
 | grep -Ev '(_seconds|_bytes|_total)$' && echo "suffix violation" && exit 1

# Optional: promtool validation if available
PROMTOOL=/usr/local/bin/promtool \
curl -fsS http://127.0.0.1:9100/metrics | "$PROMTOOL" check metrics
```

---

✅ With this contract, `ron-metrics` keeps tests **reproducible, enforceable, and ops-aligned**. It guards the wire/label contract, validates readiness semantics, and blocks perf drifts—so dashboards and alerts never silently break.
