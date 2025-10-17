

---

# 🧪 TESTS.md — svc-wallet

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `svc-wallet`:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals and Bronze→Silver→Gold acceptance gates.
* Invocation commands for local devs & CI.
* Economic safety invariants: **no doublespends**, **idempotency**, **sequence monotonicity**, **caps-only auth**, **degrade-first readiness**, **OAP/1 bounds** (≤1 MiB body, ≤10× decompression).

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure functions and modules (fast <100 ms).
**Focus areas:**

* Amount math (no overflow/underflow; fixed-point rounding).
* Sequence window logic (monotonic, gap handling).
* Idempotency key cache behavior (TTL, eviction).
* DTO validation (schema, bounds: body ≤ 1 MiB; decompress ≤ 10×).
* Policy checks (cap scope → operation map).

**Location:** `src/**` with `#[cfg(test)]`.
**Run:**

```bash
cargo test -p svc-wallet --lib -- --nocapture
```

**Representative cases:**

* `amount::safe_add` rejects overflow.
* `sequence::next_ok(prev, candidate)` enforces strictly increasing.
* `idem::cache` returns “seen” on replay within TTL; purges post-TTL.
* `dto::{Spend, Balance}` serde round-trip stable vs. golden JSON.

---

### 1.2 Integration Tests

**Scope:** End-to-end crate surface (`tests/*.rs`) against in-process server or ephemeral binary.
**Must include:**

* **API round-trip:** `POST /wallet/spend` → 200 OK with updated balance; `GET /wallet/balance`.
* **Idempotency & sequencing:** same request (same idempotency key) → single commit; out-of-order sequence → 409.
* **Security/caps:** missing/expired capability → 401/403; wrong scope → 403.
* **Config reload:** SIGHUP hot-reload updates policy; rejects/accepts reflect new rules (see CONFIG.md).
* **Concurrency invariants:** bounded inflight; shutdown drains; readiness fails writes under pressure (see CONCURRENCY.md).
* **Interop:** OAP/1 bounds enforced; DTOs lint clean (see INTEROP.md).
* **Observability:** metrics histograms present; structured errors; corr_id propagation (OBSERVABILITY.md).
* **Performance smoke:** p95 < 150 ms intra-AZ under light load (PERFORMANCE.md SLO reference).

**Run:**

```bash
cargo test -p svc-wallet --test '*'
```

**Fixtures & harness:**

* Start ephemeral `ron-ledger` and `ron-accounting` doubles (or mocks) with deterministic results.
* Seed accounts with balances; write golden vectors in `tests/vectors/`.
* Provide `tests/harness.rs` to spin server with `configs/svc-wallet.example.toml`.

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Targets & invariants:**

* **Amounts:** for random `(a,b)`, `debit(a,b)` never yields negative; rounding is consistent.
* **Sequences:** for random monotone series, acceptance iff strictly increasing; for permutations, detect first violation.
* **DTO round-trip:** `dto == from_json(to_json(dto))`.
* **State transitions:** applying a valid spend is **idempotent** when re-applied with same idempotency key; ledger state unchanged on replay.
* **Caps policy:** randomly generated cap sets → only allowed opcodes pass; others reject.

**Run:**

```bash
cargo test -p svc-wallet --features proptest
```

---

### 1.4 Fuzz Tests

**Scope:** wire-facing parsers, DTOs, and idempotency/sequence logic.
**Tooling:** `cargo fuzz`, libFuzzer.
**Fuzz targets (examples in `fuzz/fuzz_targets/`):**

* `dto_spend_parser.rs` — arbitrary JSON → parse → validate → no panics; rejects over-large payloads.
* `oap_frame.rs` — arbitrary framed bodies (with compression) → enforce ≤1 MiB and ≤10× ratio; no allocator blowups.
* `idempotency_key.rs` — random key streams → must not cause cache poisoning or unbounded growth.
* `sequence_machine.rs` — randomized in-order/out-of-order mixes → detects violations without panics/deadlocks.

**Acceptance:** 4h fuzz run, zero crashes/oom/timeouts.
**Run (local quick):**

```bash
cargo fuzz run dto_spend_parser -- -max_total_time=60
```

---

### 1.5 Chaos / Soak Tests

**Scope:** service behavior under faults & duration.
**Fault injections (via `ronctl chaos inject` or test harness):**

* **Process crash:** wallet crashes during commit → supervisor restarts cleanly; no doublespends post-replay.
* **Downstream loss:** ledger disconnects; wallet fails **writes** (429/503), **reads** may continue; auto-recovery < 5 min.
* **Bus lag/drops:** ensure no deadlocks; backpressure metrics reflect condition.
* **Disk full / slow I/O:** wallet remains responsive; emits rejects with structured errors.
* **Quota storms:** rejects are controlled; no starvation of healthy tenants (policy DRR/gateway posture).

**Acceptance (24h soak):**

* Zero FD leaks; RSS stable (no unbounded growth).
* Zero economic safety violations (no doublespends, no negative balances).
* Readiness correctly degrades/restores; error budget burn < 5% post-recovery window.

**Run (examples):**

```bash
ronctl chaos inject --target svc-wallet --fault=disconnect --dep=ron-ledger
ronctl chaos inject --target svc-wallet --fault=latency --dep=ron-accounting --p95=250ms
```

---

### 1.6 Performance / Load Tests

**Scope:** throughput, latency, quotas under realistic mixes.
**Targets:**

* **SLO reference:** p95 ≤ 150 ms for intra-AZ writes at baseline RPS; p99 ≤ 250 ms.
* **Scaling knob validation:** inflight ceilings, concurrency, quotas; ensure histograms shift predictably.
* **Regression guard:** compare to prior release via saved baselines.

**Tools:** `criterion` (microbench), custom HTTP harness in `testing/load/` (or k6/bombardier), `cargo flamegraph` on regressions.

**Run (examples):**

```bash
cargo bench -p svc-wallet
testing/load/run_load.sh http://127.0.0.1:8080 --rps 300 --duration 300s
cargo flamegraph -p svc-wallet
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Coverage ≥ **70%** (statements/branches).
* Fuzz harness **builds**; one target runs for ≥ 60 s in CI smoke.

### 2.2 Silver (Useful Substrate)

* Property tests present for amounts, sequences, DTOs.
* Fuzz run ≥ **1h** in CI (nightly).
* Coverage ≥ **85%**.
* Chaos scripts exist; 1h soak passes locally (no leaks).

### 2.3 Gold (Ops-Ready)

* Fuzz run ≥ **4h nightly**, zero crashes.
* Chaos/soak **24h** in CI (or controlled staging), no econ violations.
* Coverage ≥ **90%** across critical paths (amounts, sequence, idempotency, caps).
* Perf regression tracked release-to-release (baseline JSON + trend chart).
* Error budget SLOs wired to alerts (p95, error rate, conflicts, quotas, ledger connect errors).

---

## 3) Invocation Examples

### 3.1 All Tests

```bash
cargo test -p svc-wallet --all-targets -- --nocapture
```

### 3.2 Fuzz Target (quick)

```bash
cargo fuzz run dto_spend_parser -- -max_total_time=60
```

### 3.3 Loom (concurrency model checks)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-wallet --test loom_*
```

### 3.4 Benchmarks

```bash
cargo bench -p svc-wallet
```

### 3.5 Load (example)

```bash
testing/load/run_load.sh http://127.0.0.1:8080 --rps 300 --duration 300s --concurrency 64
```

---

## 4) Observability Hooks

* Tests emit **structured JSON logs** on failure (include `corr_id`, `account_id`, `sequence`, `reason`).
* Metrics assertions:

  * `request_latency_seconds` buckets present (with expected labels).
  * `wallet_conflicts_total` increases on replay/sequence tests.
  * `quota_exhaustions_total` increases during quota storm tests.
  * `ledger_connect_errors_total` increases when ledger is yanked.
* CorrIDs propagate across client→gateway→wallet in integration harness; on failure, print the end-to-end trace.

---

## 5) CI Enforcement

**GitHub Actions / CI jobs:**

* Build & tests:
  `cargo test --workspace --all-targets --locked`
* Lints & hygiene:
  `cargo fmt -- --check`
  `cargo clippy --workspace --all-targets -- -D warnings`
  `cargo deny check advisories bans licenses sources`
* Coverage (choose one; prefer llvm-cov if allowed):
  `cargo llvm-cov --workspace --lcov --output-path coverage.lcov`
  or `cargo tarpaulin --workspace --out Lcov`
* Fuzz (nightly):
  `cargo fuzz run dto_spend_parser -- -max_total_time=3600`
* Chaos smoke (staging):
  invoke `ronctl chaos inject` matrix against wallet + deps.
* Perf guardrail:
  run `testing/load` harness; compare to `perf/baselines/svc-wallet.json`; fail on >10% p95 regression.

**Artifacts:** store `coverage.lcov`, fuzz crashers (if any), perf baselines, and failed test logs (JSON) for 14 days.

---

## 6) Invariants to Validate (Crate-Specific)

**Economic safety**

* At-most-once debit for `(account_id, sequence, idemp_key)`.
* Monotone sequences per account; reject gaps/out-of-order with 409.
* Ledger is **source of truth**; counters reconcile deterministically.

**Security**

* **Caps-only auth**: all mutating endpoints require correct macaroon scopes.
* No secrets in logs; PII redaction patterns verified.

**Resilience**

* `/readyz` fails **writes** before collapse; recovers automatically when deps healthy.
* Bounded inflight; no unbounded queues; graceful shutdown drains.

**Interop**

* OAP/1 body cap (≤ 1 MiB) & decompress ≤ 10× strictly enforced.
* DTOs stable; schema lint passes; round-trip fidelity.

**Observability**

* Required metrics and labels exported; error taxonomy stable.

---

## 7) Example Test Matrix (Names & Paths)

* `tests/api_spend_ok.rs` — happy path spend + balance readback.
* `tests/api_idempotency.rs` — replay with same idemp key → single commit.
* `tests/api_sequence_conflict.rs` — out-of-order sequence → 409 + metric increment.
* `tests/security_caps.rs` — missing/wrong scope → 401/403.
* `tests/config_reload.rs` — SIGHUP policy change → behavior flips accordingly.
* `tests/concurrency_readiness.rs` — pressure test: writes fail readiness first.
* `tests/interop_bounds.rs` — 1 MiB body / 10× decompress cap enforced.
* `tests/observability_metrics.rs` — histogram & counters exposed.
* `tests/ledger_disconnect.rs` — ledger down: writes 503/429; auto-recover < 5 min.
* `tests/accounting_reconcile.rs` — counters match ledger snapshot after flush.
* `tests/loom_shutdown.rs` — (loom) shutdown drains, no deadlocks.
* `tests/loom_sequence.rs` — (loom) concurrent spends respect atomic sequence gate.

---

## 8) Open Questions (crate answers)

* **Which invariants are loom-checked?**
  (1) shutdown/drain ordering, (2) sequence gate atomicity, (3) idempotency cache races.

* **Which fuzz targets are mandatory?**
  `dto_spend_parser`, `oap_frame`, `sequence_machine`, `idempotency_key`.

* **What SLOs are measured in perf tests?**
  p95 write confirmation ≤ 150 ms (intra-AZ), p99 ≤ 250 ms, error rate ≤ 1%, conflicts ≤ 10/min sustained (investigate).

---

## 9) Quickstart Commands

**Local everything (fast):**

```bash
cargo test -p svc-wallet --all-targets -- --nocapture
```

**Property tests (proptest-heavy):**

```bash
cargo test -p svc-wallet --features proptest -- --ignored proptest
```

**Fuzz (1 min smoke):**

```bash
cargo fuzz run dto_spend_parser -- -max_total_time=60
```

**Loom model checks:**

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-wallet --test loom_*
```

**Perf smoke + flamegraph:**

```bash
testing/load/run_load.sh http://127.0.0.1:8080 --rps 200 --duration 120s
cargo flamegraph -p svc-wallet
```

---
