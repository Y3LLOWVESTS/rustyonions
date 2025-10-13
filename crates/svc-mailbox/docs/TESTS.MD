

````markdown
# 🧪 TESTS.md — svc-mailbox

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `svc-mailbox`:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Canonical invocation commands for devs & CI.
* Micronode (amnesia) vs Macronode (persistent) test matrix.

**Service invariants under test (authoritative):**
1) **At-least-once delivery** with **idempotency keys** enforced at API boundary.  
2) **Visibility timeouts**: un-ACKed messages re-appear after `visibility_ms`.  
3) **DLQ**: poison messages are quarantined after bounded retries.  
4) **Backpressure & shed-writes**: `/readyz` flips to degraded before collapse.  
5) **Capability auth**: requests without valid capability are rejected and never enqueued.  
6) **Strict DTOs** (`deny_unknown_fields`) and OAP/1 frame limits (max_frame = 1 MiB).

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100ms each)

**Scope:** Pure functions & small modules in `src/**`.

**Required units (examples):**
- `envelope::validate(dto)` — rejects unknown fields; checks size bounds & idem key presence.
- `idempotency::key_from(headers, body)` — stable key, collision tests.
- `retry::policy(next_retry)` — bounded retries → DLQ decision; jitter correctness.
- `visibility::extend(now, ttl)` — monotonicity & overflow guards.
- `quota::drr_bucket(rate, burst)` — token math; no negative under concurrency.
- `metrics::labels(route...)` — label cardinality bounded; sanitization.

**Location:** inline `#[cfg(test)]` blocks near the code.

**Run:**
```bash
cargo test -p svc-mailbox --lib -- --nocapture
````

---

### 1.2 Integration Tests (crate surface, end-to-end)

**Scope:** `tests/*.rs`, black-box via HTTP/OAP harness + in-proc kernel.

**Mandatory scenarios:**

* **Happy path:** `SEND -> RECV -> ACK` preserves payload & metadata.
* **Idempotency:** duplicate `SEND` with same `idem_key` is not double-enqueued.
* **Visibility timeout:** `RECV` without `ACK` → item reappears after `visibility_ms`.
* **NACK behavior:** explicit NACK requeues immediately; counter increments.
* **DLQ path:** poison handler → message hits `DLQ` after `N` bounded retries.
* **Degraded readiness:** induced CPU/IO pressure flips `/readyz`; writes are shed (503) while liveness remains 200.
* **Config reload:** SIGHUP or control API changes `visibility_ms`/limits at runtime without losing in-flight messages.
* **Capability auth:** valid capability → 2xx; expired/invalid → 401/403 and not enqueued.
* **Quota enforcement:** sustained over-rate → `rejected_total{reason="quota"}` rises; no internal queue growth.

**Run:**

```bash
cargo test -p svc-mailbox --test '*'
```

---

### 1.3 Property-Based Tests (model invariants)

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants & models:**

* **Round-trip:** `encode(envelope)` then `decode(...) == envelope` (modulo canonicalization).
* **State machine (delivery):** sequences over {SEND, RECV, ACK, NACK, TICK(dt)} preserve:

  * No negative queue sizes.
  * At-least-once: any `RECV`ed message will be either `ACK`ed once or returned to ready after `TICK(>=visibility_ms)`.
  * Idempotency: for any `(idem_key, payload)`, `total_enqueued(idem_key) ∈ {0,1}`.
* **Backpressure:** generated arrival processes (Poisson bursts) never violate shed-writes rule: if `inflight > capacity`, `accept == false`.

**Run:**

```bash
cargo test -p svc-mailbox --features proptest -- --ignored proptest-regressions
```

---

### 1.4 Fuzz Tests (wire-facing & parsers)

**Tooling:** `cargo fuzz` (libFuzzer).

**Targets (must-have):**

* `fuzz_targets/oap_frame_parser.rs` — OAP/1 framing, length prefixes, chunk caps.
* `fuzz_targets/envelope_json.rs` — strict DTOs; unknown-field rejection.
* `fuzz_targets/cap_decode.rs` — capability header parsing/validation (structure only, no secrets).
* `fuzz_targets/policy_doc.rs` — policy reload document parsing.

**Corpus:**

* Seed with minimized samples from integration tests (valid & invalid).
* Keep crashers & “interesting” inputs under `fuzz/corpus/<target>`; CI syncs artifacts.

**Acceptance:**

* CI quick run **≥ 60s** per target; Nightly soak **≥ 4h** aggregate; **zero** crashes/asserts/UB.

**Run:**

```bash
cargo fuzz run oap_frame_parser -- -max_total_time=60
cargo fuzz run envelope_json -- -max_total_time=60
```

---

### 1.5 Chaos / Soak Tests (service behavior over time)

**Scope:** `testing/chaos/*` harness driving a real binary (dev profile) with fault injection.

**Faults injected:**

* Process crashes of consumers; supervised restart loops must stabilize (no message loss).
* Bus lag & drops; no deadlocks; requeues bounded; forward progress resumes.
* Disk full & slow I/O (Macronode profile only); shed-writes engages before collapse.
* CPU 60% saturation for 5–10 min; `/readyz` degrades early; p95 stays bounded.

**Acceptance:**

* **24h** soak: zero FD leaks, zero unbounded RSS growth (RSS slope ≤ 5%/24h under steady load).
* Golden metrics steady: no monotonic `dlq_depth` increase under healthy traffic.

**Run (examples):**

```bash
testing/chaos/inject.sh --target svc-mailbox --fault cpu --percent 60 --duration 600
testing/soak/run_24h.sh --profile micronode
```

---

### 1.6 Performance / Load Tests

**SLOs under test (from RUNBOOK/PERF):**

* Enqueue→dequeue **p95 < 50 ms (intra-AZ)**; **p95 < 120 ms (cross-AZ)**.
* Fanout path (mailbox + mods + gateway) **p95 < 2 s** for 10→10k followers (measured in system tests; mailbox exposes its share).

**Workloads:**

* Payload sizes: 1 KiB (small), 64 KiB (typical), 512 KiB (near max frame headroom).
* Concurrency: 64, 128, 256, 512 clients.
* Arrival: Poisson bursts + periodic spikes to exercise shed-writes.

**Tools & harness:**

* **Criterion** microbench for hot functions (idempotency keying, DTO validation).
* **Custom load harness** in `testing/load/` (Rust + reqwest) for realistic latency histograms (Prom-exposed).
* Optional CLI spot-checks: `hyperfine` for sanity.

**Run:**

```bash
cargo bench -p svc-mailbox
testing/load/run_load.sh --concurrency 256 --size 64KiB --duration 600
```

---

## 2) Coverage & Acceptance Gates

We track **line**, **branch**, and **path** coverage (excluding generated code).

### Bronze (MVP)

* Unit + integration tests pass.
* Coverage (lines) **≥ 70%**.
* Fuzz harness builds (no run required on PR).
* One property test module present.

### Silver (Useful Substrate)

* Property tests active; at least one state-machine model.
* Fuzz run in CI **≥ 1h** aggregate (parallel across targets).
* Coverage **≥ 85%** lines, **≥ 70%** branches.
* Chaos scripts exist and run locally (10–15 min).

### Gold (Ops-Ready)

* Nightly fuzz **≥ 4h** aggregate; zero new crashers for 14 consecutive nights.
* Chaos/soak **24h** weekly job green; no FD/mem leaks.
* Coverage **≥ 90%** lines, **≥ 80%** branches; critical modules (enqueue path) **≥ 95%** lines.
* Performance regression dashboard: no p95 degradation > 10% vs last release.

**Release block rules:** Any regression of SLOs, chaos failures, or new fuzz crashers **blocks** release until triaged & waived by owners.

---

## 3) Invocation Examples

### All Tests (fast path)

```bash
cargo test -p svc-mailbox --all-targets -- --nocapture
```

### Focus integration

```bash
cargo test -p svc-mailbox --test send_recv_ack -- --nocapture
```

### Fuzz target (local quick)

```bash
cargo fuzz run oap_frame_parser -- -max_total_time=60
```

### Loom (concurrency model checks)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-mailbox --test loom_*
```

### Benchmarks

```bash
cargo bench -p svc-mailbox
```

---

## 4) Observability Hooks (for tests)

* Test harness sets `RUST_LOG=json,svc_mailbox=debug` and injects a **corr_id** per test case.
* Failures must dump:

  * Last 100 structured logs (filtered by corr_id).
  * Snapshot of key metrics (selected Prom exposition lines).
  * For integration/chaos: topology info (readyz/healthz states).
* Property/fuzz tests write **regression artifacts** to `target/test-artifacts/` (captured by CI).

---

## 5) CI Enforcement

**Jobs (GitHub Actions):**

1. **Test (PR):**

   * `cargo test --workspace --all-targets --locked`
   * `cargo fmt --all -- --check`
   * `cargo clippy --workspace -- -D warnings`
2. **Security & supply chain:**

   * `cargo deny check advisories bans sources licenses`
3. **Coverage (push to main):**

   * `grcov` or `tarpaulin` → upload to artifact + badge
4. **Fuzz (nightly):**

   * Run each target `-max_total_time=1200` (20 min) → aggregate ≥ 4h
   * Upload new corpus & minimized crashers
5. **Chaos/Soak (weekly):**

   * `testing/soak/run_24h.sh --profile micronode`
   * Publish metrics panel snapshot & RSS/FD trends
6. **Perf (release-candidate):**

   * Run `testing/load/run_load.sh` at 64/256/512 concurrency
   * Compare to baseline; fail if p95 regression > 10%

**Flake management:**

* Quarantine list for known flaky tests lives in `testing/flaky.yml`; entries expire in ≤14 days with owner & issue link.

---

## 6) Test Matrix (Micronode vs Macronode)

| Profile   | Storage | Must-pass suites                               | Extra checks                                  |
| --------- | ------- | ---------------------------------------------- | --------------------------------------------- |
| Micronode | RAM     | Unit, Integration, Property, Fuzz quick, Chaos | No on-disk artifacts; amnesia metrics present |
| Macronode | Disk    | All above + Disk fault tests                   | Disk-full/slow-IO; backup/restore round-trip  |

---

## 7) Loom Scenarios (Concurrency Model)

**Files:** `tests/loom_queue.rs`, `tests/loom_visibility.rs`

**Scenarios:**

* **Queue push/pop under contention** (N producers, M consumers): no lost or duplicated indices; no deadlocks.
* **Visibility window race:** `RECV` → `TICK(≈visibility_ms)` → concurrent `ACK` vs requeue: commits exactly once.
* **Shutdown path:** no locks held across `.await`; shutdown cancels tasks & drains channels deterministically.

**Pass criteria:** All loom tests pass with bounded state space (set via `LOOM_MAX_PREEMPTIONS`); no deadlocks or panics.

---

## 8) Mandatory Fuzz Targets & Acceptance

* `oap_frame_parser` (framing/lengths)
* `envelope_json` (strict DTOs)
* `cap_decode` (capability header structure)
* `policy_doc` (policy reload document)

**Acceptance (per release):**

* Each target runs **≥ 30 min** in CI on RC branch.
* **Zero** new crashers; any historical reproducer must be fixed or explicitly waived.

---

## 9) Performance SLO Measurements

* **Intra-AZ:** enqueue→dequeue p95 < 50 ms at 256 concurrency, 64 KiB payload, 4c/8GB node.
* **Cross-AZ:** p95 < 120 ms at 128 concurrency (simulate 3–5 ms RTT delta).
* **Throughput target:** ~500 rps stable at 4c/8GB with p95 SLOs respected.
* **Degraded mode:** under CPU 60% saturation, shed-writes activates within 5s and p95 remains < (+25% of baseline).

Artifacts: latency histograms (Prom), percentile tables (CSV), compare vs last release.

---

## 10) Artifacts & Layout

```
testing/
  chaos/
    inject.sh
    faults/*.json
    scenarios/*.md
  load/
    run_load.sh
    scenarios/*.toml
  soak/
    run_24h.sh
fuzz/
  fuzz_targets/*.rs
  corpus/<target>/*
  artifacts/<date>/*
tests/
  *.rs                    # integration
  loom_*.rs               # loom
benches/
  *.rs                    # criterion
target/test-artifacts/    # logs, metrics snapshots, reproducers
```

---

## 11) Open Questions (Resolved for svc-mailbox)

**Which invariants are loom-checked?**

* Queue index consistency; no lost/dup under N:M; shutdown drain; visibility vs ACK race.

**Which fuzz targets are mandatory?**

* `oap_frame_parser`, `envelope_json`, `cap_decode`, `policy_doc` (see §8).

**What SLOs are measured in perf tests?**

* Enqueue→dequeue p95 intra-AZ (<50 ms), cross-AZ (<120 ms); throughput (~500 rps on 4c/8GB); degraded-mode stabilization.

---

## 12) Developer Quickstart

Common one-liners for local iteration:

```bash
# Fast inner loop
cargo test -p svc-mailbox --lib -- --nocapture

# Focus a single integration test
cargo test -p svc-mailbox --test send_recv_ack -- --ignored --nocapture

# Property tests with more cases
PROPTEST_CASES=10000 cargo test -p svc-mailbox --features proptest -- --ignored

# Quick fuzz sanity
cargo fuzz run envelope_json -- -max_total_time=30

# Run criterion benches (compare before/after)
cargo bench -p svc-mailbox
```

---

✅ With this contract, `svc-mailbox` maintains reproducible quality: no silent drift in delivery guarantees, strict DTOs, predictable backpressure, and ops-ready performance.

```
```
