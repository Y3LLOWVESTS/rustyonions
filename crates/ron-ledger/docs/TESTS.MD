

---

````markdown
# 🧪 TESTS.md — ron-ledger

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/Loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ron-ledger`:

- Unit, integration, property, fuzz, Loom, chaos/soak, and performance.
- Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
- Deterministic, reproducible invocation for devs & CI.

**Primary invariants under test (from IDB):**  
I-1 append-only, I-2 conservation, I-4 capability-gated writes, I-5 deterministic state hash, I-6 idempotent ingestion, I-7 total ordering (single writer), I-8 observability, I-10 amnesia (no disk artifacts), I-11 key/KID hygiene, I-13 crash-recovery monotonicity, I-14 reversible bridges, I-15 evolvable entry kinds.

Cross-links: API (wire contracts & taxonomy), CONCURRENCY (queues/backpressure/shutdown), SECURITY (breakers, mTLS), OBSERVABILITY (metrics/alerts), INTEROP (vectors), PERFORMANCE (SLOs & baselines).

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** pure functions/modules; fast (<100ms).  
**Location:** `src/**/*_tests.rs` or `#[cfg(test)]` in module files.

**Must cover (non-exhaustive):**
- Accumulator hashing round-trips (I-5).
- RejectReason mapping and serde enums (API taxonomy).
- Config defaults & validation (CONFIG).
- Redaction helpers (SECURITY §redaction).
- DTO serde with `deny_unknown_fields` (API §wire).

**Run:**
```bash
cargo test -p ron-ledger --lib
````

---

### 1.2 Integration Tests

**Scope:** end-to-end crate surface (`tests/*.rs`) via real HTTP/UDS and feature-gated profiles.

**Required suites:**

* `tests/api_roundtrip.rs` — POST `/ingest` happy/partial/unauth + GET `/roots` determinism (I-5, I-6).
* `tests/backpressure.rs` — saturate ingress → expect 429 `busy` (CONCURRENCY §queues).
* `tests/readyz_gates.rs` — breakers and readiness flip semantics (SECURITY/OBSERVABILITY).
* `tests/config_reload.rs` — reload/boot with bad config → fail-closed (CONFIG).
* `tests/uds_peers.rs` — UDS + PEERCRED identity (INTEROP/SECURITY).
* `tests/reversible.rs` — governance reversal nets to zero; links preserved (I-2, I-14).
* `tests/amnesia_profile.rs` — Micronode leaves **no** disk artifacts (I-10).
* `tests/seq_monotonic.rs` — restart mid-ingest; no gaps; same root (I-13).

**Run:**

```bash
cargo test -p ron-ledger --test '*'
```

---

### 1.3 Property-Based Tests

**Scope:** parsers/codecs/state machine invariants.
**Tooling:** `proptest`.

**Invariants:**

* **DTO round-trip:** JSON → DTO → JSON preserves canonical form; unknown fields rejected (API).
* **Accumulator determinism:** same batch ordering always yields same `new_root` (I-5).
* **Idempotency:** replaying identical batch does not change state (I-6).
* **Conservation:** sum(in) − sum(out) == delta(mint−burn) across arbitrary batches (I-2).

**Run:**

```bash
cargo test -p ron-ledger --features proptest
```

---

### 1.4 Fuzz Tests

**Scope:** wire-facing endpoints & parsers.

**Tooling:** `cargo fuzz` (libFuzzer).
**Targets (in `fuzz/fuzz_targets/`)**:

* `ingest_json_fuzz.rs` — randomize fields/ordering; enforce `deny_unknown_fields` and taxonomy.
* `entry_nonce_fuzz.rs` — malformed base64/non-16B nonce → graceful reject (no panic).
* `rocks_replay_fuzz.rs` (macronode-feature) — fuzz WAL fragments → monotonic replay, no UB.

**Corpus:** seeded from INTEROP canonical vectors + production anonymized shapes.
**Acceptance:** **4h** nightly run, **0** crashes, **0** OOMs.

**Run (quick dev):**

```bash
cargo fuzz run ingest_json_fuzz -- -max_total_time=60
```

---

### 1.5 Loom (Concurrency Model)

**Scope:** shutdown/backpressure/single-writer scheduling under adversarial interleavings.
**Tooling:** `loom`.

**Models (in `tests/loom_*.rs`):**

* `loom_shutdown.rs` — no locks held across `.await` during shutdown; all tasks join (CONCURRENCY golden rule).
* `loom_backpressure.rs` — bounded channels never lose/duplicate entries; `try_send` → Busy path metrics (I-7).
* `loom_single_writer.rs` — exclusive commit path; no concurrent writes to WAL/accumulator (I-7).

**Run:**

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-ledger --test loom_*
```

---

### 1.6 Chaos / Soak

**Scope:** destructive ops + long-haul stability (staging/perf env).

**Faults:**

* **Kill Committer** mid-fsync (SIGKILL) → safe-mode + monotonic recovery (I-13).
* **Disk pressure** to 95% → rejects writes, remains responsive for reads (/roots).
* **Slow-loris** on `/ingest` → no task leaks; backpressure counters rise.
* **mTLS break** for privileged plane → readiness flips; unauth rejects.

**Acceptance:** **24h** soak with **0** FD/memory leaks; no seq gaps; SLOs return to green after faults.

**Run (examples):**

```bash
ron-ledgerctl chaos kill --target committer --signal SIGKILL
ron-ledgerctl chaos disk-fill --percent 95 --duration 10m
ron-ledgerctl chaos slowloris --conns 500 --duration 10m
```

---

### 1.7 Performance / Load

See `PERFORMANCE.md` for SLOs & baselines.
**Harness:** `testing/performance/*` (bombardier/k6/vegeta scripts) + Criterion micro-benches.

**Acceptance (per node):**

* p95 commit ≤ 80ms; p99 ≤ 150ms (steady state).
* Micronode ≥ 300 RPS; Macronode ≥ 1200 RPS sustained.
* 429/503 < 1% at target load.

**Run:**

```bash
cargo bench -p ron-ledger
bash testing/performance/run_macronode_steady.sh
bash testing/performance/run_micronode_steady.sh
```

---

## 2) Coverage & Gates

We enforce three tiers. Coverage via **grcov** (default) or **tarpaulin** (fallback CI runner).

### 2.1 Bronze (MVP)

* Unit + integration pass.
* Coverage ≥ **70%** lines; ≥ **60%** branches.
* Fuzz harness builds across targets.

### 2.2 Silver (Useful Substrate)

* Property tests enabled and passing.
* Fuzz **≥ 1h** in CI (sanitizers on).
* Loom models executed.
* Coverage ≥ **85%** lines; ≥ **75%** branches.
* Chaos **smoke** (short) jobs scripted.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h**, zero crashes.
* Chaos/soak **24h** in CI perf env; no leaks; seq gaps **0**.
* Coverage ≥ **90%** lines; ≥ **80%** branches.
* Performance regression tracked release-to-release (PERFORMANCE baselines).
* API/Obs **taxonomy lock** test: RejectReason set and metric labels unchanged unless documented.

---

## 3) Invocation Examples

### 3.1 All tests verbose

```bash
cargo test -p ron-ledger --all-targets -- --nocapture
```

### 3.2 Specific integration suite

```bash
cargo test -p ron-ledger --test api_roundtrip
```

### 3.3 Fuzz (quick smoke)

```bash
cargo fuzz run ingest_json_fuzz -- -max_total_time=60
```

### 3.4 Loom models

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-ledger --test loom_shutdown
```

### 3.5 Benchmarks

```bash
cargo bench -p ron-ledger
```

---

## 4) Observability Hooks

* Tests log **structured JSON** on failure (compatible with `tracing`/`tracing-test`).
* Each integration test **propagates `corr_id`** into headers and asserts it appears in logs and OTEL spans.
* Metrics assertions:

  * `rejected_total{reason="busy"}` increments on backpressure tests.
  * `ledger_commit_latency_seconds_*` histogram buckets update after load tests.
  * `ledger_seq_gap_detected_total` remains **0** outside chaos tests.

---

## 5) CI Enforcement

**Required jobs (GitHub Actions):**

* Build & Unit/Integration:

  * `cargo test --workspace --all-targets`
* Lints & Supply Chain:

  * `cargo fmt -- --check`
  * `cargo clippy -- -D warnings`
  * `cargo deny check advisories bans sources`
* Public API / SemVer:

  * `cargo public-api --deny-changes`
  * `cargo semver-checks --workspace`
* Coverage:

  * `grcov` (upload artifact + gate thresholds)
* Fuzz:

  * Nightly `cargo fuzz run …` with `-max_total_time=14400` (Gold) and ASan/UBSan enabled
* Loom:

  * `RUSTFLAGS="--cfg loom" cargo test -p ron-ledger --test 'loom_*'`
* Perf (nightly/perf env):

  * Run `testing/performance/*` and compare JSON baselines; fail on >10% drift (PERFORMANCE §5)
* OpenAPI & Taxonomy lock:

  * `oasdiff` denies breaking HTTP changes (unless major)
  * Unit asserts that `RejectReason` set matches allowed values in OBSERVABILITY/API

Artifacts: junit XML, coverage HTML, flamegraphs, k6 summaries, fuzz crashes (if any), metrics snapshots.

---

## 6) Test Matrices (Profiles & Features)

| Dimension   | Values                                     | Required                     |
| ----------- | ------------------------------------------ | ---------------------------- |
| Profile     | `micronode`, `macronode`                   | Yes (all integration suites) |
| TLS/mTLS    | off, TLS1.3, mTLS (privileged)             | Yes (readyz, unauth paths)   |
| Transport   | HTTP, UDS (PEERCRED)                       | Yes                          |
| Accumulator | default (Merkle/SHA-256), `blake3` feature | At least one per release     |
| Arch        | x86_64, aarch64                            | Nightly perf runs            |

---

## 7) Directory Layout (conventions)

```
crates/ron-ledger/
  src/
    ... modules and #[cfg(test)] unit tests ...
  tests/
    api_roundtrip.rs
    backpressure.rs
    readyz_gates.rs
    config_reload.rs
    uds_peers.rs
    reversible.rs
    amnesia_profile.rs
    seq_monotonic.rs
    loom_shutdown.rs
    loom_backpressure.rs
    loom_single_writer.rs
  fuzz/
    fuzz_targets/
      ingest_json_fuzz.rs
      entry_nonce_fuzz.rs
      rocks_replay_fuzz.rs
    corpus/
      ingest_json_fuzz/
      entry_nonce_fuzz/
  testing/
    performance/
      run_macronode_steady.sh
      run_micronode_steady.sh
      baselines/{micronode,macronode}/baseline.json
    vectors/   # canonical interop vectors (goldens)
      happy.json
      busy.json
      unknown_field.json
      reversible.json
      malformed_nonce.json
      mtls_fail.json
```

---

## 8) Golden Assertions (examples)

* **Deterministic roots:**
  Ingest fixed `vectors/happy.json` twice → `/roots` `new_root` unchanged (I-5/I-6).

* **Unknown field rejection:**
  `vectors/unknown_field.json` → HTTP 400/422; `reasons[*].reason == "unknown_kind"`; no commit (API contract).

* **Reversal conservation:**
  `vectors/reversible.json` → sum(delta) == 0; link to original preserved (I-2/I-14).

* **Crash replay:**
  Kill committer during commit; restart → `ledger_seq_gap_detected_total` unchanged; `/roots` monotonic (I-13).

---

## 9) Open Questions (kept current)

* **Loom coverage target:** minimum 3 models above; extend to cover commit batching fairness?
* **Mandatory fuzz targets:** the three listed are **required**; add `readyz_json_fuzz`?
* **Perf SLO cuts:** do we promote p95=70ms target on BLake3 feature in production?

---

## 10) Developer Quickstart

Run everything locally (fast path):

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test -p ron-ledger --all-targets -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p ron-ledger --test 'loom_*'
cargo bench -p ron-ledger
```

Nightly (perf/staging):

```bash
bash testing/performance/run_macronode_steady.sh
cargo fuzz run ingest_json_fuzz -- -max_total_time=3600
```

---

✅ With this testing contract, `ron-ledger` maintains **determinism, safety, and performance** across code changes, preventing silent drift and ensuring auditors can reproduce claims end-to-end.

```
```
