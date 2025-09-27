
---

````markdown
# 🧪 TESTS.md — ron-kernel

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/Loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ron-kernel`:

* Unit, integration, property, fuzz (where applicable), chaos/soak, performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* **Invariant mapping** to tests (IDB), plus CI wiring to prevent drift.

This file complements: `docs/IDB.md`, `docs/CONCURRENCY.md`, `docs/CONFIG.md`, `docs/PERFORMANCE.md`, `docs/SECURITY.md`.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100ms)

**Scope:** pure logic & small async units.

**Must-cover (examples):**
- `supervisor::backoff(attempt)` — jittered exponential backoff caps at 30s; monotonic non-decreasing.
- `supervisor::intensity_cap` — ≤ 5 restarts / 60s quarantines the child.
- `config::validate()` — fail-closed for illegal values; redaction on diff logs.
- `bus::bounded_broadcast` helpers — capacity set; overflow increments counters.
- `health::merge_ready()` — state machine correctness under single-threaded updates.
- `metrics` registration — golden metrics present (`service_restarts_total`, `bus_lagged_total`, `request_latency_seconds`).

**Run:**
```bash
cargo test -p ron-kernel --lib
````

---

### 1.2 Integration Tests (end-to-end surface)

**Scope:** crate wiring in `tests/*.rs`.

**Required scenarios:**

* **Config reload semantics** (file + SIGHUP + bus event): success and rollback on invalid update.
* **Bus fanout under pressure**: 1→N publish with slow consumer — overflow counted, kernel never blocks.
* **Supervisor crash→restart→quarantine**: emitted `KernelEvent::ServiceCrashed{reason}`, caps enforced.
* **Readiness gating** (demo bin): `/healthz` liveness OK; `/readyz` degrades under backpressure.
* **Amnesia label propagation**: metrics carry `amnesia="on|off"` after toggle.

**Run:**

```bash
cargo test -p ron-kernel --test '*'
```

---

### 1.3 Property-Based Tests (proptest)

**Targets & invariants:**

* **Config precedence**: `CLI > ENV > FILE > DEFAULTS` (associativity & idempotence of merge).
* **Backpressure law**: For any arrival sequence, queue depth ≤ capacity; on overflow, returns Busy or drops (per policy) and increments counter.
* **Restart windows**: Arbitrary crash sequences respect `(cap, window)`; quarantine iff exceeded.
* **Readiness monotonicity**: Once `ready=true`, it cannot flip to `true` again without a preceding `false` caused by a documented condition.

**Run:**

```bash
cargo test -p ron-kernel --features proptest
```

---

### 1.4 Fuzz Tests (targeted)

**Kernel has no external protocol parser; fuzz only minimal surfaces:**

* **Config loader** (TOML/JSON): structure-aware fuzz to ensure no panics, graceful errors, redaction works.
* **Optional demo HTTP**: fuzz request line/headers (if demo routes are enabled).

**Tooling & runs:**

```bash
cargo fuzz list
cargo fuzz run cfg_parser -- -max_total_time=60
```

**Acceptance:** CI nightly ≥ 1h total across fuzz targets; zero crashes/ooms.

---

### 1.5 Chaos / Soak Tests

**Scope:** concurrency & resilience.

**Scenarios:**

* **Restart storm**: induce panics at {250ms, 500ms, 1s}; verify backoff+jitter; quarantine after cap; `/readyz` degrades then recovers.
* **Lagging consumer**: stall a receiver; assert `bus_lagged_total` increases, no deadlocks, throughput for other receivers stable.
* **Shutdown drain**: send `Shutdown`; all tasks exit ≤ deadline (1–5s); stragglers aborted; `tasks_aborted_total` increments.

**Acceptance:** 24h soak without FD/memory leaks (RSS flat within tolerance), no deadlocks.

---

### 1.6 Performance / Load Tests

See `docs/PERFORMANCE.md` for SLOs & rigs. Kernel-specific:

* **Publish→deliver latency** (p50/p95/p99).
* **Sustained fanout throughput** (1→8 receivers).
* **Config hot-swap latency**.
* **Crash→respawn latency**.

**Run:**

```bash
cargo bench -p ron-kernel
cargo run -p ron-kernel --bin perf_bus_fanout
cargo run -p ron-kernel --bin perf_restart_storm
cargo run -p ron-kernel --bin perf_config_reload
```

---

## 2) Invariant → Test Matrix (audit-critical)

| Invariant (IDB)                                  | Unit                             | Integration / Property / Chaos                                   | CI Gate         |
| ------------------------------------------------ | -------------------------------- | ---------------------------------------------------------------- | --------------- |
| No lock across `.await`                          | lint test (`await_holding_lock`) | Loom model (see §3.3), clippy deny-wall                          | ✅ block on fail |
| Bounded channels; overflow counted, not blocking | `bus::capacity_overflow()`       | `bus_fanout_load.rs`; proptest backpressure law                  | ✅               |
| Crash-only restarts; caps + jitter               | `backoff_caps()`                 | `restart_storm.rs` (chaos), property restart window              | ✅               |
| Config hot-swap; rollback on invalid             | `config::validate()`             | `config_reload_loop.rs` (file, SIGHUP, bus)                      | ✅               |
| Readiness degrade under pressure                 | `health::merge_ready()`          | demo `/readyz` under load; verify 503 + Retry-After              | ✅               |
| Amnesia flag surfaced & labeled                  | `metrics_labels()`               | toggle & scrape; labels present                                  | ✅               |
| Golden metrics exported                          | `metrics_registry_contains()`    | scrape verifies `{bus_lagged_total, service_restarts_total,...}` | ✅               |

> CI Gate column means PR must fail if any linked test fails.

---

## 3) Invocation Examples

### 3.1 All tests (workspace style)

```bash
cargo test -p ron-kernel --all-targets -- --nocapture
```

### 3.2 Fuzz target (config)

```bash
cargo fuzz run cfg_parser -- -max_total_time=60
```

### 3.3 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-kernel --test loom_* -- --ignored
```

### 3.4 Benches (Criterion)

```bash
cargo bench -p ron-kernel
```

---

## 4) Observability Hooks

* **Structured JSON logs**: tests install a JSON subscriber; on failure, logs include `corr_id`, `event`, `queue`, `amnesia`.
* **Metrics during tests**: ephemeral Prometheus registry per test; assert presence/values for golden metrics.
* **Bus tail helper**: a test-only utility to subscribe and assert on `KernelEvent::*`.

---

## 5) CI Enforcement

**Jobs (suggested GitHub Actions):**

* **Unit+Integration:** `cargo test --workspace --all-targets`
* **Lints:** `cargo clippy -p ron-kernel -- -D warnings -D clippy::await_holding_lock`
* **Fmt:** `cargo fmt -- --check`
* **Deny:** `cargo deny check advisories bans licenses sources`
* **Coverage:** `grcov` or `tarpaulin` (linux)
* **Loom (PRs):** run `--cfg loom` for `loom_*` tests (ignored by default otherwise)
* **Fuzz (nightly):** `cargo fuzz run cfg_parser -- -max_total_time=3600`
* **Perf (nightly):** run perf rigs; compare to baselines; fail on regression (see PERFORMANCE.md)

**Coverage targets (Linux, stable toolchain):**

* Bronze ≥ **70%**, Silver ≥ **85%**, Gold ≥ **90%** (line + branch where supported).

---

## 6) Reference Test Skeletons (copy-paste)

**Unit — backoff caps**

```rust
#[test]
fn backoff_caps_at_30s() {
    for a in 0..12 {
        let d = supervisor::backoff(a);
        assert!(d <= std::time::Duration::from_secs(30));
    }
}
```

**Integration — bus overflow never blocks**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn bus_overflow_counts_not_blocks() {
    let (tx, _) = tokio::sync::broadcast::channel(1024);
    let slow = tx.subscribe();
    // drop slow consumer messages by not receiving
    for _ in 0..10_000 {
        let _ = tx.send(bytes::Bytes::from_static(b"x"));
    }
    // assert: kernel counter incremented; no deadlock occurred
    assert!(metrics::bus_lagged_total().get() > 0);
}
```

**Property — config precedence**

```rust
use proptest::prelude::*;
proptest!{
  #[test]
  fn merge_precedence(cli in any::<Config>(), env in any::<Config>(), file in any::<Config>()) {
    let merged = precedence::merge(&[Defaults, &file, &env, &cli]);
    prop_assert_eq!(merged, precedence::apply(cli, env, file));
  }
}
```

**Loom — shutdown race**

```rust
#[test]
#[ignore] // run with --cfg loom
fn loom_shutdown_no_deadlock() {
    loom::model(|| {
        let (tx, rx) = loom::sync::mpsc::channel::<u8>();
        let stop = loom::sync::Arc::new(loom::sync::atomic::AtomicBool::new(false));
        // model a worker loop using try_recv with stop flag...
    });
}
```

---

## 7) Coverage & Gates

### 7.1 Bronze (MVP)

* [ ] Unit + integration tests pass
* [ ] Code coverage ≥ **70%**
* [ ] Fuzz harness builds

### 7.2 Silver (Useful Substrate)

* [ ] Property tests included
* [ ] Fuzz run ≥ **1h** in CI
* [ ] Coverage ≥ **85%**
* [ ] Chaos scripts (restart/lag/shutdown) present

### 7.3 Gold (Ops-Ready)

* [ ] Nightly fuzz ≥ **4h**; zero crashes
* [ ] 24h soak; FD/memory stable
* [ ] Coverage ≥ **90%**
* [ ] Perf regressions tracked & gated release-to-release

---

## 8) Open Questions (crate-specific; answer during implementation)

* Which queues adopt **drop-oldest** vs **reject-new**? (Default: broadcast=drop-oldest, mpsc=reject-new.)
* Which **Loom** tests are mandatory vs optional for the current release?
* Exact **metrics cardinality** budget for tests (limit test label explosion).
* Are demo HTTP routes enabled in CI (localhost)? If yes, which fuzz targets?

---

## 9) Appendix — Commands (quick copy)

```bash
# Units & integrations
cargo test -p ron-kernel --all-targets -- --nocapture

# Property
cargo test -p ron-kernel --features proptest

# Loom (PRs)
RUSTFLAGS="--cfg loom" cargo test -p ron-kernel --test loom_* -- --ignored

# Fuzz
cargo fuzz run cfg_parser -- -max_total_time=3600

# Coverage (example with tarpaulin)
cargo tarpaulin -p ron-kernel --out Xml

# Performance (see PERFORMANCE.md for acceptance thresholds)
cargo bench -p ron-kernel
```

---

```
```
