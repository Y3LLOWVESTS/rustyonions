
---

# 🧪 TESTS.md — svc-registry

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

This document defines the **testing contract** for `svc-registry`:

* **Taxonomy:** unit, integration, property, fuzz, loom/concurrency, chaos/soak, performance.
* **Coverage gates:** Bronze → Silver → Gold (ops-ready).
* **Repro commands:** dev & CI, plus harness layout and required artifacts.

`svc-registry` is an **append-only, signed registry** with a **fast read plane** and a **quorum-gated write plane** (`propose → approve (M-of-N) → commit`). Correctness hinges on: canonical JSON (`payload_b3`), signature verification, **serialized HEAD advance** (single-writer critical section), **non-blocking bus** (lossy), and **checkpoint restore** behavior.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, deterministic)

**Scope:** pure logic and small components; target <100 ms each.

Focus areas:

* Canonicalization: `to_canonical_json`, `payload_b3` derivation.
* Signatures: `verify_approval(ed25519, payload_b3)`, threshold M-of-N count logic.
* CAS/HEAD logic: “monotonic version”, “no fork”, “idempotent commit”.
* DTO hygiene: `serde` with `deny_unknown_fields` (rejects unknowns; stable defaulting).
* Config parsing/validation (toggle read-only, amnesia).
* Metrics label cardinality helpers (route normalization).
* Error taxonomy mapping (Busy, Deps, BadRequest, Unauthorized) to HTTP codes.

**Location:** co-located in `src/**` under `#[cfg(test)]`.
**Run:**

```bash
cargo test -p svc-registry --lib -- --nocapture
```

---

### 1.2 Integration Tests (black-box)

**Scope:** end-to-end via public surface in `tests/*.rs`.

**Must include:**

* **Read plane:**

  * `GET /registry/head` returns current immutable version; route metrics emitted.
  * `GET /registry/{version}` returns exact snapshot; cache headers correct.
  * `SSE /registry/stream` emits on commit; clients can reconcile via `/head` after simulated drops.
* **Write plane:**

  * `propose → approve (M-of-N) → commit` happy path; HEAD monotonic; audit event emitted.
  * Rejects: insufficient approvals, bad signatures, stale proposal, read-only or deps-down.
* **Config & lifecycle:**

  * Config reload/no-downtime (if supported) or restart semantics; amnesia vs persistent behavior.
  * Readiness gates: `/healthz` vs `/readyz` for read/writes.
* **Concurrency invariants:**

  * Serialized HEAD advance under concurrent commits (no ABA, no fork).
  * Backpressure and 429 behavior when inflight exceeds cap.
* **Checkpoint:**

  * Persist, restart, restore → read-ready < SLO; write-ready < SLO; HEAD preserved.

**Run:**

```bash
cargo test -p svc-registry --test '*' -- --nocapture
```

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Targets & invariants:**

* **Canonicalization ↔ hashing:** for arbitrary JSON maps (bounded depth/size):

  * `hash(canonicalize(x)) == hash(canonicalize(canonicalize(x)))` (idempotency).
  * Ordering invariance for unordered fields.
* **Approval set semantics:**

  * For random signer sets (N up to 32, M ≤ N):
    `M_of_N(approvals).is_satisfied() ⇒ verify_all(payload_b3)`;
    superfluous approvals do not change commit result.
* **CAS/HEAD:**

  * For random commit sequences, HEAD is strictly monotonic and linearizable with the lock semantics.
* **DTO hygiene:**

  * Randomly injected unknown fields are **rejected** (schema stable).
* **SSE reconciliation:**

  * Arbitrary event loss patterns still converge after `GET /head`.

**Run:**

```bash
cargo test -p svc-registry --lib --features proptest -- --nocapture
```

---

### 1.4 Fuzz Tests

**Tooling:** `cargo fuzz` (libFuzzer).
**Corpus:** seed with **canonical vectors** (small/med/large descriptors), API samples, and failing cases from CI and prod (sanitized).

**Required fuzz targets:**

* `fuzz_canonical_json`: malformed UTF-8, nesting bombs, float edge cases → no panics; bounded allocations.
* `fuzz_api_payloads`: `propose/approve/commit` payloads with random field omission/duplication/unknowns; ensure proper typed errors.
* `fuzz_sse_frames`: random frame chops/replays; parser never panics; client convergence property holds in harness.
* `fuzz_config`: env + file overlay permutations; no UB; stable precedence.

**Acceptance (per tier):**

* **Silver:** 1h wall-time, zero crashes/oom.
* **Gold (nightly):** ≥4h per target, RSS bounded; no new findings.

**Run examples:**

```bash
cargo fuzz run fuzz_canonical_json -- -max_total_time=3600
cargo fuzz run fuzz_api_payloads  -- -max_total_time=14400
```

---

### 1.5 Loom / Concurrency Tests

**Goal:** model the **single-writer HEAD critical section**, bus non-blocking publish, and readiness flips.

**Scenarios:**

* **Serialized HEAD advance:** multiple writers contend; exactly one CAS succeeds; others retry or fail deterministic.
* **Backpressure without deadlock:** request cap + semaphore ensures no task holds lock across `.await`.
* **Shutdown order:** tasks drain; no publish after shutdown; no stalls on oneshot channels.
* **Readiness flips:** write-plane disabled while read-plane stays green under dependency loss.

**Run:**

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-registry --test loom_* -- --nocapture
```

---

### 1.6 Chaos / Soak Tests

**Targets:** service binary under fault injection in `testing/chaos/`.

**Faults to inject:**

* **Process crashes:** random SIGKILL while commits are in-flight → upon restart, checkpoint restore preserves HEAD; read-ready within SLO.
* **Bus lag/drops:** throttle consumers → rising `bus_lagged_total` while clients still converge via `/head`.
* **Disk issues (Macronode):** fsync latency spikes, ENOSPC; service sheds writes, read SLOs hold.
* **Network:** intermittent TCP resets; SSE resilience.

**Acceptance:**

* **Silver soak:** 6h, no FD/memory leaks; SLOs mostly green.
* **Gold soak:** 24h, zero deadlocks; error budget intact; post-soak heap stable.

**Harness:**

```bash
testing/chaos/run_chaos.sh --fault=storage-latency --ms=50 --duration=6h
```

---

### 1.7 Performance / Load Tests

**Tools:** `criterion`, `bombardier`/`wrk`, custom Rust harness in `testing/performance/`.

**Read plane targets (baseline 4c/8GiB):**

* `/registry/head`: **≥ 2,500 RPS/node** at **p95 < 50 ms** intra-region.
* `/registry/{version}`: **≥ 3,000 RPS/node**, **p95 < 100 ms**.

**Write plane:**

* Commit cadence **1–5 commits/s** with M-of-N approvals: **commit p95 < 750 ms**, p99 < 1,200 ms.

**Checkpoint restore (Macronode):**

* 50k descriptors: **read-ready < 10 s**, **write-ready < 20 s** after restart.

**Run:**

```bash
cargo bench -p svc-registry
testing/performance/run_reads.sh http://127.0.0.1:9444
testing/performance/run_commits.sh http://127.0.0.1:9444
```

---

## 2) Coverage & Gates

> Coverage via `grcov` (preferred) or `tarpaulin` on Linux; exclude generated code and benches.

### 2.1 Bronze (MVP)

* Unit + integration pass.
* Coverage **≥ 70%** lines; **≥ 60%** branches.
* Fuzz targets compile; loom suite compiles.

### 2.2 Silver (Useful Substrate)

* Property tests running; loom suite enabled on PRs (smoke subset).
* Fuzz nightly **≥ 1h** per target, zero crashes; corpus artifacted.
* Chaos soak **≥ 6h** quarterly.
* Coverage **≥ 85%** lines; **≥ 70%** branches.

### 2.3 Gold (Ops-Ready)

* Fuzz nightly **≥ 4h** per target, zero new crashes; RSS bounded.
* Chaos soak **24h** monthly; artifacts (metrics, logs, flamegraphs) attached.
* Perf regression tracked release-to-release with JSON baselines; hard gate for regression (see PERFORMANCE.md §5).
* Coverage **≥ 90%** lines; **≥ 80%** branches.

---

## 3) Invocation Examples

### 3.1 Run everything (dev)

```bash
cargo test -p svc-registry --all-targets -- --nocapture
```

### 3.2 Fuzz target (1 minute smoke)

```bash
cargo fuzz run fuzz_canonical_json -- -max_total_time=60
```

### 3.3 Loom suite

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-registry --test loom_* -- --nocapture
```

### 3.4 Benches

```bash
cargo bench -p svc-registry
```

### 3.5 Chaos smoke (10 minutes)

```bash
testing/chaos/run_chaos.sh --fault=bus-slow --duration=10m
```

---

## 4) Observability Hooks

* All tests **emit structured JSON** logs; failures dump last 200 lines from the **fixture service** and include correlation IDs.
* Prometheus scrape during integration/perf tests; harness captures:

  * `request_latency_seconds{route=...}`
  * `commit_latency_seconds`
  * `rejected_total{reason}`
  * `bus_lagged_total`
  * `registry_checkpoint_age_seconds`
* On failure, artifacts uploaded: logs, metrics snapshot (`/metrics` text), flamegraphs (if collected), fuzz crash inputs (minimized).

---

## 5) CI Enforcement

**Matrix:**

* `ubuntu-latest` (stable), `macos-latest` (smoke), `windows-latest` (unit only).
* Toolchain MSRV 1.80.0 + stable latest (dual build).

**Jobs (required to pass):**

1. **Lint & deny**

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo deny check advisories bans sources licenses
   ```
2. **Unit + integration + loom-smoke**

   ```bash
   cargo test --workspace --all-targets -- --nocapture
   RUSTFLAGS="--cfg loom" cargo test -p svc-registry --test loom_smoke -- --nocapture
   ```
3. **Coverage (Linux)**

   * `grcov` or `tarpaulin` → badge + PR comment; enforce gates per tier.
4. **Fuzz (nightly)**

   ```bash
   cargo fuzz run fuzz_canonical_json -- -max_total_time=3600
   cargo fuzz run fuzz_api_payloads  -- -max_total_time=3600
   ```

   Artifacts: new corpus seeds, crashes (if any), sanitizer logs.
5. **Perf (nightly)**

   * Run `testing/performance/*` against local binary; compare to JSON baselines; fail on regressions (see PERFORMANCE.md §5).
6. **Chaos/soak (scheduled)**

   * Quarterly 6h (Silver), monthly 24h (Gold); artifacts retained 90d.

---

## 6) Open Questions (crate-specific resolution)

* **Loom scope:** Do we model bus publish as bounded mpsc or broadcast with drop semantics? *(Default: broadcast with bounded ring; verify no deadlock under full ring).*
* **Mandatory fuzz targets:** Is `fuzz_sse_frames` mandatory at Bronze? *(Default: optional at Bronze; mandatory at Silver+).*
* **Perf SLO verification cadence:** Are commit p95 gates enforced per-PR (smoke) or nightly only? *(Default: nightly hard gate; PR smoke gate with relaxed thresholds).*
* **Checkpoint scaling tests:** Lock sizes for “50k descriptors” scenario—fixed or proportional to current prod snapshot? *(Default: fixed 50k for repeatability + a proportional suite for staging).*

---

## 7) Repository Layout (tests & harness)

```
svc-registry/
├─ src/                            # unit tests co-located under #[cfg(test)]
├─ tests/
│  ├─ read_plane.rs                # /head, /{version}, SSE reconcile
│  ├─ write_plane.rs               # propose/approve/commit happy & rejects
│  ├─ readiness.rs                 # /healthz, /readyz gating
│  ├─ checkpoint_restore.rs        # restart/restore SLOs
│  └─ dto_hygiene.rs               # deny_unknown_fields, schema stability
├─ tests_loom/
│  ├─ loom_head_serialize.rs       # single-writer CAS
│  └─ loom_shutdown.rs             # orderly shutdown, no deadlocks
├─ fuzz/
│  ├─ fuzz_canonical_json/
│  ├─ fuzz_api_payloads/
│  ├─ fuzz_sse_frames/
│  └─ fuzz_config/
├─ testing/
│  ├─ performance/
│  │  ├─ run_reads.sh
│  │  ├─ run_commits.sh
│  │  └─ baselines/*.json
│  └─ chaos/
│     └─ run_chaos.sh
```

---

## 8) Canonical Vectors (must ship with repo)

* `vectors/approvals/` — M-of-N approval sets (N = 3,5,7; M = 2,3,4), keys + signatures.
* `vectors/descriptors/` — small (1 KiB), medium (64 KiB), large (1 MiB) canonical JSON samples.
* `vectors/sse/` — event sequences with synthetic gaps (drop patterns).
* `vectors/config/` — valid/invalid configs, env overlays.

These vectors seed property tests and fuzz corpora and are referenced by integration tests.

---

## 9) Auditor Checklist

* [ ] Unit tests cover canonicalization, verify, CAS, DTO hygiene.
* [ ] Integration covers read plane, write plane (happy/reject), readiness, checkpoint restore.
* [ ] Properties prove idempotency and M-of-N semantics; unknown fields rejected.
* [ ] Fuzz runs exist for canonical JSON, API payloads; nightly ≥1h (Silver), ≥4h (Gold).
* [ ] Loom verifies single-writer CAS and no lock-across-await on hot paths.
* [ ] Chaos/soak completed per tier with artifacts attached.
* [ ] Perf baselines exist; regressions gated; restore SLOs measured.
* [ ] Coverage meets tier gate; report artifacted in CI.

---

✅ With this contract, `svc-registry` preserves **correctness** (no forks, verified commits), **resilience** (lossy bus + reconcile), and **performance** (fast reads, bounded commit latency) under continuous testing—dev, CI, and chaos alike.
