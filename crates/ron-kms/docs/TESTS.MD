
---

# 🧪 TESTS.md — ron-kms

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ron-kms`:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Commands for devs & CI, plus artifact expectations (flamegraphs, perf JSON, fuzz corpora).

This test plan enforces:

* **Security**: capability checks precede crypto work; attestation policy honored.
* **Interop**: HTTP+gRPC DTOs stable; canonical encodings; KATs for classical & PQ.
* **Concurrency**: pools/semaphores fair; no deadlocks under burst/backpressure.
* **Perf**: SLOs & drift gates per `PERFORMANCE.md`.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope (fast, <100ms each):**

* Crypto adapters: Ed25519, ML-DSA (Dilithium), ML-KEM (Kyber), AEAD wrappers (seal/open).
* DTO/codec: request/response structs, error taxonomy, capability parsing.
* Policy evaluators: `verify_mode` transitions (or/and/pq_only), tenant overrides.
* Verify cache: LRU eviction, TTL semantics, staleness counters.
* Config guards: env/CLI precedence, deny_unknown_fields.

**Locations & patterns**

* `src/**/mod.rs` with `#[cfg(test)]`
* `tests/common/*.rs` shared helpers (mock backends, fixed RNG, KAT loader)

**Run**

```bash
cargo test -p ron-kms --lib
```

**Must assert**

* No panic on bad inputs; errors are typed (`Error::Policy`, `Error::Capability`, `Error::Backend`).
* Deterministic results with fixed RNG (seeded `StdRng`).
* Backends behind trait object are **pure** under unit conditions (use mock).

---

### 1.2 Integration Tests

**Scope (E2E crate surface in `tests/*.rs`):**

* **API round-trip** (HTTP & gRPC if enabled):

  * `POST /v1/kms/sign|verify|wrap|unwrap|keys|rotate`
  * Happy paths & negative cases (capability missing, malformed DTOs, policy reject).
* **Config reload semantics**:

  * SIGHUP or `/admin/reload` toggles values without dropping inflight; verify `readyz` stays true; metrics reflect new limits.
* **Concurrency invariants**:

  * Backpressure: saturated inflight caps return 429/503 **before** heavy work.
  * Graceful shutdown: inflight drained; no leaked tasks/FDs; bus `Shutdown` observed.
* **Backend matrices**:

  * **sealed-file** backend (CI default).
  * **PKCS#11** (smoke; gated by `KMS_E2E_PKCS11=1` to avoid flakiness in CI).
  * **TPM** (optional; `KMS_E2E_TPM=1`).
* **PQ modes**:

  * `verify=or` (Hybrid-OR), `verify=and` (Hybrid-AND), `pq_only`.

**Run**

```bash
cargo test -p ron-kms --test '*'
# Enable optional backends as needed:
KMS_E2E_PKCS11=1 cargo test -p ron-kms --test e2e_pkcs11
```

**Must assert**

* Semver-stable fields; unknown fields rejected when configured.
* Canonical errors: 401/403 cheap; 4xx taxonomy consistent across HTTP/gRPC.

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred)
**Targets & invariants:**

* **DTO codecs**: JSON ↔ struct round-trip (lossless), unknown-field behavior.
* **Signature bundles** (classical/PQ/hybrid):

  * Round-trip encode/decode stable across versions.
  * Idempotent `verify(verify(sign(m))) == Ok` for allowed alg sets.
* **Policy state machine**:

  * Transitions over `(tenant_phase, verify_mode, rollout_percent)` never admit invalid combos (e.g., AND with missing PQ half).
* **Verify cache**:

  * TTL monotonicity; staleness never decreases; eviction obeys LRU.

**Run**

```bash
cargo test -p ron-kms --features proptest
```

---

### 1.4 Fuzz Tests

**Tooling:** `cargo fuzz`, libFuzzer
**Targets (examples)**

* `fuzz_targets/json_api.rs`: fuzz HTTP JSON bodies for sign/verify/keys/rotate.
* `fuzz_targets/grpc_api.rs`: fuzz gRPC-encoded messages (if enabled).
* `fuzz_targets/codec_bundle.rs`: fuzz signature bundle parser (hybrid/classical).
* `fuzz_targets/pkcs11_adapter.rs`: fuzz backend responses (behind a mock shim).

**Corpus**

* Seed with:

  * Minimised CI failures.
  * Captured real-world invalid samples (redacted).
  * KAT files (shrunk variants) to teach structure.

**Acceptance**

* **Bronze**: targets build.
* **Silver**: ≥1h CI nightly, zero crashes.
* **Gold**: ≥4h nightly, zero crashes; coverage >90% regions of codec & policy logic.

**Run**

```bash
cargo fuzz run json_api -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak Tests (service profile)

**Harness:** `testing/chaos/*` scripts (tc netem, fault injection)
**Faults injected**

* **Backend flap** (PKCS#11 reconnect storms, session exhaustion).
* **Process crash** (SIGSEGV via test flag) → supervisor restart; clean readiness in <10s.
* **Bus lag/drops** → no deadlocks; events eventually consistent.
* **Disk full/slow I/O** (file backend) → sign must fail fast; verify allowed if policy permits cached verification.
* **Network jitter** (+100ms RTT, 1% loss) → inter-AZ tail tracked.

**Acceptance**

* **24h soak**: zero FD/memory leaks (RSS stable), no task leaks, p95 within +15% baseline for steady mix.
* No `kms_audit_integrity_failed_total`.

**Run (examples)**

```bash
# HSM flap, 15m
testing/chaos/hsm_flap.sh --duration 900
# Soak 24h, steady mix, capture artifacts
testing/chaos/soak.sh --duration 86400 --mix steady
```

---

### 1.6 Performance / Load Tests

**Harness:** `criterion` micro-bench + `testing/performance/*`
**Metrics & SLOs** (see `PERFORMANCE.md`):

* p95 sign(Ed25519) ≤ 1.0 ms; sign(ML-DSA) ≤ 10 ms.
* verify(hit) ≤ 1.0 ms; verify(miss,classical) ≤ 5.0 ms.
* Throughput at baseline host (ref table); drift gates ±10–15%.

**Workloads**

* Steady 70/20/10 (sign/verify/wrap).
* Burst 10× for 60s every 5m.
* Rotate flood (PQ migration spike).
* Tenant isolation (80/20 QPS; p95 skew ≤ 2×).

**Run**

```bash
cargo bench -p ron-kms
testing/performance/run_http.sh --mix steady --duration 300
testing/performance/run_grpc.sh --mix burst --burst-mult 10 --interval 300
```

**Artifacts**

* Flamegraph SVGs, tokio-console traces, perf JSON in `target/perf/`.
* Baselines under `testing/performance/baselines/ron-kms/`.

---

## 2) Coverage & Gates

### 2.1 Coverage measurement

* Tool: `grcov` (Linux) or `cargo-tarpaulin` (fallback).
* **Exclude** benches, examples, generated code; **include** adapters, policy, codecs, cache, API handlers.

### 2.2 Bronze (MVP)

* Unit + integration tests pass on Linux/x86_64.
* Coverage ≥ **70%** overall; ≥ **80%** for codec & policy modules.
* Fuzz harness builds for all listed targets.
* CI smoke for sealed-file backend; PKCS#11 gated.

### 2.3 Silver (Useful Substrate)

* Property tests included for codecs & policy.
* Fuzz runs ≥ **1h** nightly; zero crashes.
* Coverage ≥ **85%** overall; ≥ **90%** codec & policy.
* Chaos scripts present; 1h chaos job passes (no leaks).
* Loom tests for pool fairness and shutdown ordering pass.

### 2.4 Gold (Ops-Ready)

* Fuzz runs ≥ **4h** nightly; zero crashes; new repros filed automatically.
* **24h soak** weekly; no FD/memory leaks; tails within ±15% of baseline.
* Coverage ≥ **90%** overall; **no** `#[allow(dead_code)]` in src/.
* Performance drift gates enforced (CI fails if p95 ↑ >10–15% or throughput ↓ >10%).
* KATs for classical/PQ/hybrid pinned and verified release-to-release.

---

## 3) Invocation Examples

### 3.1 All tests (fast path)

```bash
cargo test -p ron-kms --all-targets -- --nocapture
```

### 3.2 Integration matrix

```bash
# default (sealed-file)
cargo test -p ron-kms --test e2e_http
# with gRPC
KMS_ENABLE_GRPC=1 cargo test -p ron-kms --test e2e_grpc
# with PKCS#11 (opt-in)
KMS_E2E_PKCS11=1 cargo test -p ron-kms --test e2e_pkcs11 -- --test-threads=1
```

### 3.3 Property tests

```bash
cargo test -p ron-kms --features proptest --test prop_*
```

### 3.4 Fuzz

```bash
cargo fuzz run json_api -- -max_total_time=3600
```

### 3.5 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-kms --test loom_* -- --include-ignored
```

### 3.6 Benchmarks & perf suite

```bash
cargo bench -p ron-kms
testing/performance/run_http.sh --mix steady --duration 300
```

---

## 4) Observability Hooks

* Tests emit **structured JSON** logs on failure with `corr_id`, `tenant`, `op`, `alg`, `backend`, `verify_mode`.
* Histograms/counters are scraped in perf/chaos jobs and written to `target/perf/*.json`.
* Chaos jobs must upload artifacts (flamegraph SVG, tokio-console capture, perf JSON, ghz/bombardier outputs) in CI under:

  * `artifacts/kms-<job>-YYYYMMDD-HHMM/…`

---

## 5) CI Enforcement

**Jobs (GitHub Actions):**

* **Lint/Sec**: `cargo fmt -- --check`, `cargo clippy -D warnings`, `cargo deny check advisories licenses bans sources`
* **Unit & Integration**: `cargo test --workspace --all-targets`
* **Property**: `cargo test -p ron-kms --features proptest --test prop_*`
* **Coverage**: `grcov` (Linux) → badge + threshold gates
* **Fuzz (nightly)**: run each target ≥1h; upload crashers/corpus; fail on new crash
* **Perf (nightly)**: run perf suite; compare to baselines; **fail** on drift
* **Chaos (weekly)**: 24h soak; leak check; artifacts uploaded

**Fail conditions**

* Coverage < target for tier.
* New 5xx/429 taxonomy mismatch in E2E.
* Perf gates breached (see `PERFORMANCE.md`).
* Fuzz crash not triaged within SLA.

---

## 6) Open Questions (crate-specific resolutions)

* **Which invariants are loom-checked?**

  * Pool fairness: no tenant starvation under burst (fair-share semaphore).
  * Shutdown ordering: no task after backend drop; no double-close on PKCS#11 sessions.
  * Cache + reload: TTL updates while concurrent lookups do not deadlock.

* **Which fuzz targets are mandatory?**

  * `json_api`, `codec_bundle` (mandatory Bronze+).
  * `grpc_api` (if gRPC enabled), `pkcs11_adapter` (Silver+ when backend present).

* **What SLOs are measured in perf tests?**

  * p95 latencies & throughput per op/alg (Ed25519, ML-DSA, ML-KEM).
  * Tenant p95 skew (≤ 2×).
  * Backpressure ratio (<1%).
  * Cache hit/miss/stale ratios; staleness ≤ 2× TTL.

---

## 7) Test Data & KATs

* **KATs** live in `testing/vectors/ron-kms/`:

  * `ed25519/*.kat`, `ml-dsa/*.kat`, `ml-kem/*.kat`, `hybrid/*.kat`
* Each KAT includes: message, public params, signature/ciphertext, expected verify result.
* **Rule**: KATs are immutable; add new files for new alg versions.
* CI verifies KATs across all backends that implement the alg.

---

## 8) Flake & Secret Hygiene

* No network in unit/property tests; integration uses loopback only.
* Skip HSM/TPM tests in CI unless explicitly enabled; mark as `#[ignore]` locally if hardware absent.
* Never log secrets or private material; tests redact with `<redacted>` helpers.
* Deterministic seeds for RNG in tests (`StdRng::seed_from_u64(42)`), except for fuzz.

---

## 9) Local Dev Shortcuts

```bash
# Fast inner loop
cargo nextest run -p ron-kms

# Only codec+policy
cargo test -p ron-kms codec policy -- --nocapture

# Re-run last 10 failing tests
cargo test -p ron-kms -- --include-ignored --failed --run-ignored
```

---

## 10) Artifacts & Reporting

* **On failure**, CI uploads:

  * Test logs (JSON), flamegraphs, tokio-console traces.
  * Fuzz crashers & minimized repros.
  * Perf run JSON + diff vs baseline.
* **Dashboards** link back to build via build number + git SHA.
* **Auto-file** GitHub issues for new fuzzers (nightly) with minimized repro and stack.

---

## 11) Bronze → Silver → Gold Checklist

* [ ] **Bronze**: unit + integration green; coverage ≥70%; fuzz builds; sealed-file backend E2E.
* [ ] **Silver**: property tests; fuzz ≥1h nightly; coverage ≥85%; chaos scripts pass 1h; loom suites pass.
* [ ] **Gold**: fuzz ≥4h nightly; 24h soak weekly; coverage ≥90%; perf drift gates enforced; KATs pinned across algs/backends.

---

## 12) Commands (copy-paste)

```bash
# Unit + integration + property
cargo test -p ron-kms --all-targets --features proptest -- --nocapture

# Fuzz (1h)
cargo fuzz run json_api -- -max_total_time=3600

# Loom suite
RUSTFLAGS="--cfg loom" cargo test -p ron-kms --test loom_* -- --include-ignored

# Perf (steady + burst)
cargo bench -p ron-kms
testing/performance/run_http.sh --mix steady --duration 300
testing/performance/run_grpc.sh --mix burst --burst-mult 10 --interval 300

# Chaos (HSM flap 15m; soak 24h)
testing/chaos/hsm_flap.sh --duration 900
testing/chaos/soak.sh --duration 86400 --mix steady
```

---

✅ With this test contract, `ron-kms` is verifiable end-to-end: correctness (KATs), safety (policy/attestation), robustness (loom/chaos), and speed (perf gates). It’s CI-enforceable and auditor-friendly, and it scales with PQ migration without drift.
