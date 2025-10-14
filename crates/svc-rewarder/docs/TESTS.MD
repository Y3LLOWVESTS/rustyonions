

---

# 🧪 TESTS.md — `svc-rewarder`

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `svc-rewarder`:

* Unit, integration, property, fuzz, chaos, soak, and performance tests.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Deterministic **golden vectors** and **idempotency** checks.
* Invocation commands for devs & CI, including **PQ/hybrid on/off** and **Micronode/Macronode** profiles.

Key crate invariants under test:

* **Determinism**: same sealed inputs ⇒ identical outputs.
* **Conservation**: reward totals do not exceed policy budgets; no overflow/underflow.
* **Idempotency**: duplicate intent emits produce `dup` (not double-credit).
* **Bounded work**: no unbounded CPU/memory; backpressure works (429/503).
* **Reject-paths**: body size cap, decompression ratio cap, schema validation, capability checks.
* **Quarantine**: when ledger is degraded, results are quarantined and not emitted.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, pure)

**Scope**

* Reward calculus primitives (weighting, aggregation, rounding).
* Policy parsing & validation (policy_id/policy_hash).
* Idempotency key formation (`run_key`) and hashing.
* Serialization/DTOs (no alloc explosions; serde round-trip).

**Location**

* `src/*` with `#[cfg(test)] mod tests { … }`

**Examples**

* `calc::weights_rounding_is_stable()`
* `policy::hash_matches_manifest()`
* `keys::run_key_stable_across_retries()`
* `dto::sealed_inputs_roundtrip()`

**Run**

```bash
cargo test -p svc-rewarder --lib -- --nocapture
```

---

### 1.2 Integration Tests (end-to-end)

**Scope**

* HTTP surface: `POST /rewarder/epochs/{id}/compute` happy-path + errors.
* **Config reload** via `SIGHUP`/`SIGUSR1`: clamps and queue bounds update without restart.
* **Backpressure**: MAX_INFLIGHT / WORK_QUEUE_MAX enforcement (429/503 with reason).
* **Quarantine path**: ledger brownout ⇒ 202 Accepted with `quarantine=true`, no emit.
* **Idempotency**: repeated POST with same `run_key` ⇒ `dup` classification at ledger.

**Location**

* `tests/*.rs` (e.g., `tests/http_compute.rs`, `tests/backpressure.rs`, `tests/quarantine.rs`)

**Run**

```bash
cargo test -p svc-rewarder --test '*'
```

---

### 1.3 Property-Based Tests (proptest/quickcheck)

**Scope & Invariants**

* **Round-trip**: sealed inputs encode/decode without loss.
* **Determinism**: f(inputs) == f(inputs) for randomized valid inputs.
* **Conservation**: sum(rewards) ≤ policy_budget and ≥ 0 across arbitrary partitions.
* **Monotonicity** (where defined): increasing eligible work never decreases total payout.
* **Idempotency**: emit(intent(inputs)) twice ⇒ `dup` on second, never double-credit.

**Hints**

* Use strategies that generate **valid** policies and bounded cardinalities (S/M/L scales).
* Reject invalid cases early with clear shrinking messages.

**Run**

```bash
cargo test -p svc-rewarder --features proptest
```

---

### 1.4 Fuzz Tests (cargo-fuzz)

**Targets**

* `fuzz_targets/parse_sealed_inputs.rs` (JSON/CBOR/MsgPack if supported).
* `fuzz_targets/policy_decoder.rs`
* `fuzz_targets/http_compute.rs` (harness stubs auth/ledger; focuses on reject-paths and panics).

**Corpus**

* Seed with:

  * real-world (redacted) exemplars,
  * minimized cases from CI failures,
  * adversarial size/compression variants.

**Acceptance**

* **No panics / OOM / UB** in **4h** run.
* Reject-paths hit **without** sustained RSS growth.

**Run (local quick)**

```bash
cargo fuzz run parse_sealed_inputs -- -max_total_time=60
```

**Run (CI/nightly)**

```bash
cargo fuzz run parse_sealed_inputs -- -max_total_time=14400
```

---

### 1.5 Chaos & Soak Tests

**Faults**

* **Process crash**: service restarts cleanly; inflight drains on shutdown signal.
* **Bus lag/drop**: no deadlocks; reconnection restores readiness.
* **Ledger brownout**: egress p99 > 250ms ⇒ quarantine-only; recovery exits quarantine.
* **Disk full / slow I/O** (tmpfs/disk): reject non-critical artifacts; remain serving.
* **Slow-loris** & **compression/size bombs**: capped and rejected with metrics.

**Soak**

* 24h steady **S** workload:

  * p95 latency flat (±10%),
  * **no RSS drift**,
  * no file descriptor growth/leaks.

**Run (examples)**

```bash
# Inject egress jitter 50–500ms, run 30m
scripts/chaos_ledger_jitter.sh 50 500

# Slow-loris against HTTP for 10m
scripts/chaos_slow_loris.sh http://localhost:8080

# 24h soak at S-scale
scripts/soak_run.sh S 24h
```

---

### 1.6 Performance / Load Tests

**SLOs under test (recap)**

* p95 `< 500ms`, p99 `< 2s` (intra-region).
* Error budget: 5xx `< 0.1%`, 429/503 `< 1%` sustained.
* PQ/hybrid overhead: p95 **≤ +5%** vs baseline.

**Workloads**

* **S**: 1k actors / 10k events
* **M**: 10k actors / 100k events
* **L**: 50k actors / 500k events

**Tools**

* Micro: `criterion`
* HTTP: `bombardier` / `wrk`
* Profiling: `cargo flamegraph`, `tokio-console`

**Run (examples)**

```bash
# Micro-bench (save baseline)
cargo bench -p svc-rewarder -- --save-baseline main

# Steady M profile for 60s
bombardier -c 128 -d 60s -m POST -f testing/performance/payloads/M.json \
  http://localhost:8080/rewarder/epochs/demo/compute

# Burst (2× steady) for 60s
bombardier -c 256 -d 60s -m POST -f testing/performance/payloads/M.json \
  http://localhost:8080/rewarder/epochs/demo/compute
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* ✅ Unit + integration tests pass (Micronode profile, PQ off).
* ✅ Code coverage **≥ 70%** (grcov/tarpaulin).
* ✅ Fuzz harness builds.
* ✅ Golden vectors (S/M/L) checked into `testing/performance/vectors/`.

### 2.2 Silver (Useful Substrate)

* ✅ Property tests in place (determinism, conservation, idempotency).
* ✅ Fuzz run **≥ 1h** in CI (no crashes).
* ✅ Coverage **≥ 85%**.
* ✅ Chaos scripts present (brownout, slow-loris, size/compression bombs).
* ✅ Basic soak (≥ 2h) shows **no** RSS/FD drift.

### 2.3 Gold (Ops-Ready)

* ✅ Nightly fuzz **≥ 4h** (all targets).
* ✅ 24h soak stable (no drift; p95 within ±10%).
* ✅ Coverage **≥ 90%** (critical modules ~95%).
* ✅ Perf regression tracked release-to-release (baselines updated per policy).
* ✅ PQ/hybrid ON suite passes with p95 delta **≤ +5%**.

---

## 3) Invocation Examples

### 3.1 All tests (workspace hygiene)

```bash
cargo test -p svc-rewarder --all-targets -- --nocapture
```

### 3.2 Integration focus (with logs)

```bash
RUST_LOG=info,svc_rewarder=debug cargo test -p svc-rewarder --test '*'
```

### 3.3 Property tests (proptest)

```bash
cargo test -p svc-rewarder --features proptest -- --nocapture
```

### 3.4 Fuzz (quick vs long)

```bash
cargo fuzz run parse_sealed_inputs -- -max_total_time=60
cargo fuzz run parse_sealed_inputs -- -max_total_time=14400
```

### 3.5 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-rewarder --test loom_*
```

### 3.6 Benchmarks (Criterion)

```bash
cargo bench -p svc-rewarder
```

### 3.7 Perf smoke (HTTP)

```bash
bombardier -c 64 -n 5000 -m POST -f testing/performance/payloads/S.json \
  http://localhost:8080/rewarder/epochs/demo/compute
```

---

## 4) Observability Hooks

* Tests log **structured JSON** on failure (include `corr_id`, `tenant`, `epoch`).
* Integration/chaos tests **scrape metrics** before/after to assert:

  * `reward_compute_latency_seconds` (p95, p99)
  * `ledger_intents_total{result}`
  * `rejected_total{reason}`
  * `queue_depth`, `readyz_degraded{cause}`
  * 5xx/429/503 rates
* Each test emits a final **verdict block** with sampled metrics for audit trails.

---

## 5) CI Enforcement (matrix & gates)

**Matrix**

* OS: ubuntu-latest
* Toolchains: stable (MSRV pinned 1.80.0 for build), latest stable for tests
* Features/profiles:

  * `profile=micronode, pq=off`
  * `profile=macronode, pq=off`
  * `profile=macronode, pq=on`

**Jobs**

* **lint:** `cargo fmt -- --check`, `cargo clippy -D warnings`
* **build:** `cargo build --workspace --locked`
* **test:** `cargo test --workspace --all-targets -- --nocapture`
* **coverage:** grcov or tarpaulin; fail if below gate
* **fuzz-nightly:** each `cargo fuzz run <target> -- -max_total_time=14400`
* **perf-nightly:** run steady/burst on M; compare to baselines; fail on PERF gates
* **deny:** `cargo deny check advisories bans licenses sources`

**Perf gates (CI fail if)**

* p95 ↑ **>10%** or throughput ↓ **>10%** vs baseline (same profile/scale).
* CPU/RSS ↑ **>15%** steady.
* PQ ON delta p95 **> +5%** vs PQ OFF baseline.

---

## 6) Suite Details (what exactly we test)

### 6.1 Unit (must-pass list)

* Arithmetic safety: no overflow/underflow; saturating or checked math where declared.
* Rounding: banker’s or specified rule—property test for **associativity stability**.
* Policy hash consistency (content-address).
* Run key determinism across retries and replays.

### 6.2 Integration (must-pass list)

* **Happy path**: S/M/L vectors produce expected digests; HTTP 200; intent `accepted`.
* **Idempotency**: same request twice ⇒ first `accepted`, second `dup`.
* **Backpressure**: exceed inflight/queue ⇒ 429/503 with `reason` label; metrics increase.
* **Reject-paths**: body > 1 MiB ⇒ 413/400; decompress ratio > 10 ⇒ 400; malformed JSON ⇒ 400.
* **Quarantine**: ledger p99 degraded ⇒ 202 with `quarantine=true`; **no emit** recorded.

### 6.3 Property (must-pass invariants)

* Determinism, conservation, monotonicity (as defined), idempotency.
* Round-trip DTOs; no panic with arbitrarily generated valid inputs.

### 6.4 Fuzz

* Parsers/codecs: no panic/UB; coverage grows.
* HTTP compute: malformed/malicious inputs are rejected within caps; **no RSS climb**.

### 6.5 Chaos & Soak

* Brownout drill enters/exits quarantine correctly; burn-rate policy intact.
* Slow-loris does not starve worker pool; backpressure engages.
* 24h soak: flat RSS/FD; p95 stable within ±10%.

### 6.6 Performance

* S/M/L: p95/p99 within targets; dup ratio ≤ 1%.
* PQ ON: p95 delta ≤ +5%; worker reduction (-1) if needed to keep CPU < 70%.

---

## 7) File/Folder Layout (tests & harness)

```
svc-rewarder/
├─ src/
│  └─ ... (unit tests inline with modules)
├─ tests/
│  ├─ http_compute.rs
│  ├─ backpressure.rs
│  ├─ quarantine.rs
│  ├─ idempotency.rs
│  └─ loom_shutdown.rs            # model shutdown/backpressure
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ parse_sealed_inputs.rs
│  │  ├─ policy_decoder.rs
│  │  └─ http_compute.rs
│  └─ corpora/...
├─ testing/
│  ├─ performance/
│  │  ├─ payloads/{S,M,L}.json
│  │  ├─ vectors/{S,M,L}.json      # sealed, canonical
│  │  ├─ scenarios/{steady,burst,brownout}.md
│  │  └─ baselines/...
│  ├─ chaos/
│  │  ├─ chaos_ledger_jitter.sh
│  │  └─ chaos_slow_loris.sh
│  └─ helpers/
│     └─ metrics_assert.rs         # scrape & assert helper
```

---

## 8) Open Questions (crate-specific knobs to confirm)

* **Monotonicity domain**: enumerate exact conditions where monotonicity must hold (per policy semantics).
* **Golden vectors**: who owns updates & sign-off workflow (tie to GOVERNANCE.md)?
* **Ledger stub**: how we standardize latency/jitter injection for CI portability.
* **Mobile harness**: emulator/container profile we standardize on (2 vCPU/2 GiB RAM).

---

## 9) Quick Start — Developer Shortcuts

```bash
# Run unit + integration with logs
RUST_LOG=info,svc_rewarder=debug cargo test -p svc-rewarder --all-targets -- --nocapture

# Property tests only (fast seed)
cargo test -p svc-rewarder --features proptest -- --nocapture proptest-regressions

# Fuzz (quick)
cargo fuzz run parse_sealed_inputs -- -max_total_time=60

# Perf smoke (S scale)
bombardier -c 64 -n 5000 -m POST -f testing/performance/payloads/S.json \
  http://localhost:8080/rewarder/epochs/demo/compute
```

---

✅ With this TESTS.md, `svc-rewarder` has a **clear, auditable testing contract** that enforces our invariants, validates SLOs, and prevents silent drift—ready for Bronze today and paving a straight path to Gold.
