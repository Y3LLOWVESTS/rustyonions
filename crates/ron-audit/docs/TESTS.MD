

---

# 🧪 TESTS.md — ron-audit

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **testing contract** for `ron-audit`:

* Unit / integration / property / fuzz / loom / chaos / performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* One-command invocations for devs & CI, with reproducible datasets & baselines.

`ron-audit` is a **library** embedded by hosts. Tests target the lib surface and its required host behaviors (metrics, shedding, WAL/export) via harnesses and fixtures.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, pure)

**Scope**

* Canonicalization and normalization helpers.
* `dedupe_key` stability and cost.
* Record building & invariant guards (no floats, NFC text, size bounds).
* Hashing (`self_hash`) and `prev→self` link computation.
* Signature attach/verify plumbing (using deterministic test keys).
* Export manifest struct encode/decode (no I/O).

**Layout**

* `src/**` with `#[cfg(test)] mod tests` blocks.
* Fixture helpers in `tests/common/mod.rs`.

**Run**

```bash
cargo test -p ron-audit --lib -- --nocapture
```

**Key assertions**

* Determinism: same input → same canonical bytes/hash/signature.
* Bounds respected: attrs ≤ configured max; error on overflow.
* Zero panics on valid inputs; precise error types on invalid.

---

### 1.2 Integration Tests (end-to-end lib surface)

**Scope**

* Append pipeline: canonicalize → BLAKE3 → chain → sign → (optional) WAL/export (mocked fs).
* Verify-on-read path over synthetic batches.
* Config toggles: Micronode (no WAL/export) vs Macronode (with WAL/export).
* Concurrency invariants, backpressure & shedding (bounded queue).

**Layout**

* `tests/*.rs`:

  * `append_roundtrip.rs`
  * `verify_chain.rs`
  * `macronode_export.rs`
  * `micronode_mode.rs`
  * `shedding_semantics.rs`

**Must include**

* **API round-trip**: Build `AuditRecord` → append → read-verify → export checkpoint hash equals in-memory root.
* **Config reload semantics**: apply new audit knobs (queue depth/batch size) through the exposed API (or host shim) and confirm no record loss or invariant break.
* **Concurrency adherence**: simulate burst; assert **shed-not-block** with accurate counters.

**Run**

```bash
cargo test -p ron-audit --tests -- --nocapture
```

---

### 1.3 Property-Based Tests (determinism & correctness)

**Tooling**: `proptest`

**Targets & invariants**

* **Canonicalization** (`prop_canon_deterministic`):

  * For arbitrary Unicode text/attrs/maps (bounded size), canonical form is **deterministic** and stable across runs.
  * Canonicalize → parse back (if applicable) → canonicalize again ⇒ identical bytes.
* **Hash chain** (`prop_chain_continuity`):

  * For a random sequence of valid records, `prev` linkage is correct; removing a record or altering any byte is detected by re-verification.
* **Signature** (`prop_signature_valid`):

  * Deterministically signed records verify; any bit-flip fails.
* **Dedupe key** (`prop_dedupe_stable`):

  * Equal logical records (modulo stable ordering) yield same dedupe key; small perturbations change it.

**Run**

```bash
cargo test -p ron-audit --features proptest -- --nocapture
```

> Keep generators **bounded** (e.g., body ≤ 4 KiB, attrs map ≤ 32 entries) to avoid pathological memory usage and to mirror PERF envelopes.

---

### 1.4 Fuzz Tests (robustness at the edges)

**Tooling**: `cargo fuzz` (LLVM libFuzzer)

**Targets**

* `fuzz_targets/canonicalize_fuzz.rs`

  * Feed arbitrary byte sequences as “records” → ensure canonicalizer rejects/normalizes without panics or UB.
* `fuzz_targets/record_parse_fuzz.rs`

  * Decode candidate `AuditRecord` bytes → validate invariants (size, NFC) → re-encode → compare (where defined).
* `fuzz_targets/export_manifest_fuzz.rs`

  * Decode export manifest; re-encode/stress unusual field orders; ensure no crash & round-trip stability.

**Corpora**

* Seed from `tests/data/*.bin` (hand-crafted edge cases: empty attrs, max attrs, high Unicode, near-limit sizes).
* Persist crashers under `fuzz/corpus/*` and minimize.

**Acceptance**

* CI (nightly): **≥ 1h** per target, zero crashes/oom.
* Weekly soak: **≥ 4h** per target in scheduled job.

**Run (examples)**

```bash
cargo fuzz run canonicalize_fuzz -- -max_total_time=3600
cargo fuzz run record_parse_fuzz -- -max_total_time=3600
```

---

### 1.5 Loom Tests (concurrency model checking)

**Tooling**: `loom` (dev-only)

**Models**

* **Bounded queue, shed-not-block**:

  * Two producers / one consumer contending on a bounded channel.
  * Invariant: when full, **try_send** path returns Busy and increments `drop{reason="audit_backpressure"}`; no spin/park deadlock.
* **No cross-task aliasing bugs**:

  * Internal state (e.g., last hash, prev pointer) not observed in a torn state; no reading partially written record.
* **Shutdown quiescence**:

  * A `shutdown` signal causes consumers to drain current batch and stop; no stuck tasks; no double-drop.

**Run**

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-audit --test loom_* -- --nocapture
```

> Keep state small; use Loom’s model bounds (max threads/steps) to constrain exploration while catching races.

---

### 1.6 Chaos / Soak (host-simulated, but required for DoD)

**Scope**

* Disk latency/pressure injection (WAL), message storms (burst), and bitrot simulation against **export** (staging).
* Verify **no leaks** (FD/memory), consistent shedding within budget, and **no crashes**.

**Harness**

* Scripts under `testing/runbook/` and `testing/performance/audit/`:

  * `chaos_inject.sh wal-latency 300`
  * `chaos_inject.sh storm 600`
  * Bitrot: corrupt a single exported chunk; ensure `audit_verify_fail_total > 0` and run §6.2 recovery from RUNBOOK.

**Acceptance**

* **24h soak** in CI/staging: zero panics; stable memory/FD counts; all alerts within budgets (see OBSERVABILITY).

---

### 1.7 Performance / Load (tie to PERFORMANCE.md)

**Tooling**

* `criterion` micro-benches (`benches/append_hot.rs`, `verify_hot.rs`, `dedupe_key.rs`).
* Workspace rigs: `testing/performance/audit/{wal_steady.sh,export_batch.sh,shed_storm.sh}`.

**Measured SLOs (must hold on lab baseline hardware)**

* **Micronode (RAM-only)**:

  * Append throughput (single core): p50 ≥ **120k ops/s**, p95 ≥ **90k ops/s**.
  * Append latency: p95 ≤ **0.8 ms**, p99 ≤ **1.5 ms**.
* **All**:

  * Verify `self_hash`: ≥ **400k ops/s** (single core).
  * Chain link check: ≥ **250k links/s**.
* **Macronode (WAL/export)**:

  * `wal_fsync_seconds` p95 ≤ **8 ms**, p99 ≤ **15 ms** (batched).
  * `export_batch_seconds` p95 ≤ **250 ms** per 10k records.

**Regression gates**

* Drop >10% throughput or >2 allocs/op vs last green baseline = CI fail.

**Run**

```bash
cargo bench -p ron-audit -- --save-baseline=local
bash testing/performance/audit/wal_steady.sh
bash testing/performance/audit/export_batch.sh
bash testing/performance/audit/shed_storm.sh
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* ✔ Unit + integration tests pass on x86_64.
* ✔ Code coverage **≥ 70%** (lines).
* ✔ Fuzz harnesses compile.
* ✔ Loom smoke model runs (bounded queue basic).

### 2.2 Silver (Useful Substrate)

* ✔ Property tests included and passing.
* ✔ Fuzz **≥ 1h**/target nightly, zero crashes.
* ✔ Coverage **≥ 85%** (lines), **≥ 75%** (branches) for core modules.
* ✔ Chaos scripts present; 1h soak with **no leaks** and shedding ≤ budget.

### 2.3 Gold (Ops-Ready)

* ✔ Fuzz **≥ 4h** nightly per target, zero crashes in last 14 days.
* ✔ Chaos/soak **24h** quarterly; attach Grafana snapshots; no panics, budget respected.
* ✔ Coverage **≥ 90%** lines, **≥ 80%** branches for `append`, `verify`, canonicalization, export manifest.
* ✔ Perf baselines green; no >10% regressions; gates enforced in CI.

---

## 3) Invocation Examples

### 3.1 All tests (fast & integ)

```bash
cargo test -p ron-audit --all-targets -- --nocapture
```

### 3.2 Property tests only

```bash
cargo test -p ron-audit prop_ -- --nocapture
```

### 3.3 Fuzz targets

```bash
cargo fuzz run canonicalize_fuzz -- -max_total_time=60
cargo fuzz run record_parse_fuzz -- -max_total_time=60
cargo fuzz run export_manifest_fuzz -- -max_total_time=60
```

### 3.4 Loom models

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-audit --test loom_* -- --nocapture
```

### 3.5 Benches

```bash
cargo bench -p ron-audit
```

### 3.6 Soak / chaos (staging)

```bash
bash testing/runbook/chaos_inject.sh wal-latency 300
bash testing/runbook/chaos_inject.sh storm 600
```

---

## 4) Observability Hooks

* Tests **must** use structured logs (`tracing`) and, on failure, print:

  * input seed / case index,
  * corr_id (uuid v4 per test),
  * minimal repro snippet.
* Integration tests expose Prometheus test registries to assert metric **names & increments**:

  * e.g., `audit_drop_total{reason="audit_backpressure"}` increments when queue is full,
  * `audit_verify_fail_total==0` for clean paths.
* Benches emit Criterion reports; CI archives JSON baselines.
* Chaos/soak scripts capture:

  * `/metrics` snapshots pre/post,
  * selected Grafana panel PNGs (if available),
  * system stats (CPU, IO, FD counts).

---

## 5) CI Enforcement

**Jobs (GitHub Actions)**

* **Lint & hygiene**: `cargo fmt -- --check`, `cargo clippy -D warnings`, `cargo deny check advisories bans licenses sources`.
* **Test matrix**:

  * `cargo test -p ron-audit --all-targets` on:

    * ubuntu-latest x86_64,
    * ubuntu-arm64 (self-hosted/emu acceptable for smoke),
  * **loom** job: `RUSTFLAGS="--cfg loom" cargo test -p ron-audit --test loom_*`.
* **Coverage**:

  * `grcov` or `cargo-tarpaulin` producing LCOV; thresholds enforced (Bronze/Silver/Gold).
* **Fuzz (nightly)**:

  * `cargo fuzz run <target> -- -max_total_time=3600` (artifact crashers uploaded).
* **Perf gates**:

  * `cargo bench` with Criterion JSON diff vs `testing/performance/baselines/ron-audit/`.
  * Fail if p50/p95 throughput ↓ >10% or allocs/op ↑ >2.
* **Chaos (scheduled / staging)**:

  * Triggered workflow running `testing/runbook/chaos_inject.sh` modes and archiving metrics/log bundles.

**Artifacts**

* Criterion baselines, LCOV, fuzz corpora/crashers (if any), chaos bundles.

---

## 6) Canon-Specific Answers (crate-filled)

**Which invariants are loom-checked?**

1. **Bounded queue never grows unbounded**; full queue returns Busy → increments `audit_drop_total{reason="audit_backpressure"}`.
2. **No deadlocks** between producers/consumer on shutdown; all tasks quiesce.
3. **No torn reads**: readers never observe a partially formed record.

**Which fuzz targets are mandatory?**

* `canonicalize_fuzz` (MUST) — catches Unicode/canonical edge cases.
* `record_parse_fuzz` (MUST) — wire safety for record bytes.
* `export_manifest_fuzz` (SHOULD) — durability metadata stability.

**What SLOs are measured in perf tests?**

* Micronode append p50/p95 throughput & p95/p99 latency (per §1.7).
* Verify throughput (self_hash, chain).
* Macronode `wal_fsync_seconds` p95/p99 and `export_batch_seconds` p95 (with fixed batch/cadence).

**Minimum coverage by module (Gold)**

* `append` + `verify`: **≥ 95% lines**, **≥ 85% branches**.
* Canonicalization: **≥ 95% lines**.
* Export manifest: **≥ 90% lines**.

**Reproducibility**

* Property tests print failing seeds; rerun with `PROPTEST_CASES=1 PROPTEST_SEED=<seed>`.
* Fuzzers keep minimized artifacts under `fuzz/artifacts/*`; add as regression tests.

**Non-goals**

* No network I/O in unit/property tests.
* No flaky wall-clock dependencies (use deterministic clocks in tests).

---

## 7) Local Developer Quickstart

```bash
# 1) Run fast unit/integration
cargo test -p ron-audit --all-targets

# 2) Property tests (bounded)
PROPTEST_CASES=512 cargo test -p ron-audit prop_

# 3) Fuzz for a minute each (smoke)
cargo fuzz run canonicalize_fuzz -- -max_total_time=60
cargo fuzz run record_parse_fuzz -- -max_total_time=60

# 4) Loom models
RUSTFLAGS="--cfg loom" cargo test -p ron-audit --test loom_*

# 5) Benches and compare locally
cargo bench -p ron-audit -- --save-baseline=dev
```

---

## 8) Appendix — File Map & Stubs

```
crates/ron-audit/
├─ benches/
│  ├─ append_hot.rs
│  ├─ verify_hot.rs
│  └─ dedupe_key.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ canonicalize_fuzz.rs
│  │  ├─ record_parse_fuzz.rs
│  │  └─ export_manifest_fuzz.rs
│  └─ cargo-fuzz.toml
├─ tests/
│  ├─ append_roundtrip.rs
│  ├─ verify_chain.rs
│  ├─ macronode_export.rs
│  ├─ micronode_mode.rs
│  ├─ shedding_semantics.rs
│  └─ common/mod.rs
└─ testing/
   ├─ performance/audit/{wal_steady.sh,export_batch.sh,shed_storm.sh}
   └─ runbook/{chaos_inject.sh,audit_verify.sh,diagnostics_bundle.sh,Makefile.perf}
```

---

**With this contract in place**: new work can’t land without adequate tests; CI enforces coverage, perf, and fuzz/loom discipline; and ops can trust the audit plane under load, bursts, or black-swan events.
