

---

# 🧪 TESTS.md — omnigate

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **testing contract** for **omnigate**:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals and Bronze→Silver→Gold acceptance gates.
* Reproducible invocation for local dev and CI.
* Drift guards that align with `PERFORMANCE.md`, `CONCURRENCY.md`, `OBSERVABILITY.md`, `RUNBOOK.md`, and `SECURITY.md`.

> Omnigate is stateless (beyond counters/caches), streams responses, and orchestrates `svc-index`, `svc-storage`, `svc-mailbox`. Tests must prove: **shed-before-collapse**, **no locks across `.await` on hot paths**, **stream over buffer**, **OAP/1 invariants** (1 MiB frame cap; 64 KiB typical stream chunks), and **amnesia mode** correctness (Micronode).

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure logic and small components (<100 ms).
**Targets (typical):**

* `planner::join_plan` (deterministic plan selection).
* `policy::cap_check` (macaroon/capability gating).
* `dto::{encode,decode}` round-trips (no alloc spikes).
* `stream::chunker` (64 KiB default; boundaries honored).
* `limits::ingress` (1 MiB body cap; ≤10× decompression guard math).

**Location:** `src/**` with `#[cfg(test)]`.

**Run:**

```bash
cargo test -p omnigate --lib -- --nocapture
```

### 1.2 Integration Tests

**Scope:** End-to-end crate surface in a controlled sandbox with **mock downstreams**.
**Location:** `crates/omnigate/tests/*.rs` + helpers in `crates/omnigate/testing/`.

**Must include:**

* **API round-trip**: `/v1/view/*`, `/v1/media/* (range-start)` via the service interface (behind a mock gateway client if present).
* **Config reload** semantics: change limits (inflight caps, chunk size) and verify live effect without restarts (see `CONFIG.md`).
* **Concurrency invariants**: backpressure kicks (429/503) before collapse; readiness flips writes first; graceful drain on SIGTERM (see `CONCURRENCY.md` and `RUNBOOK.md`).
* **Amnesia mode (Micronode)**: RAM-only; assert **zero disk artifacts** during test (temp dir watchers).
* **ETag/addr discipline**: `b3:<hex>` echoed and respected on range requests.
* **Error-path**: downstream timeouts/5xx → mapped to caller SLO without deadlocks.

**Run:**

```bash
cargo test -p omnigate --test '*'
```

> **Mock strategy:** Use lightweight `axum` test servers for `svc-index`/`svc-storage`/`svc-mailbox` with per-endpoint latency knobs and failure injection (HTTP 5xx, timeouts, slow-loris). Provide helpers under `testing/mocks/`.

### 1.3 Property-Based Tests

**Scope:** Parsers/codecs, policy evaluation, planner decisions, and protocol envelopes.

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Invariants:**

* **No panics** for any valid/invalid envelope (defensive decoding).
* **Round-trip**: `encode(decode(x)) == normalize(x)` for DTOs and envelopes.
* **Idempotent transitions**: planner decisions under commutative sets of inputs (ordering independence where specified).
* **Monotone backpressure**: increasing load never **reduces** rejection rate when at cap (non-decreasing shed function).
* **No unbounded growth**: for any stream length ≤ 1 MiB/frame, memory growth is O(1) w.r.t. frames (chunked streaming).

**Run:**

```bash
cargo test -p omnigate --features proptest
```

### 1.4 Fuzz Tests

**Scope:** Wire-facing surfaces and parsers: envelopes, DTOs, header parsing, range request handling.
**Tooling:** `cargo fuzz` with `libFuzzer`.

**Targets (examples):**

* `fuzz_targets/envelope_decode.rs` — random bytes → decode → must not panic; reject invalid frames within time/size caps.
* `fuzz_targets/policy_caps.rs` — random capability tokens → check; ensure no OOM/UB.
* `fuzz_targets/range_headers.rs` — malformed `Range:` headers must not hang or over-allocate; correct 206/416 mapping.

**Corpus:** Seed with unit/integration fixtures + minimized crashes.
**Acceptance:**

* CI (nightly): ≥ **1 hour** per target, **zero crashes**.
* Gold: ≥ **4 hours** nightly per target.

**Run (example):**

```bash
cargo fuzz run envelope_decode -- -max_total_time=3600
```

### 1.5 Chaos / Soak

**Scope:** Service behavior under faults and time.
**Inject:**

* **Process crashes** (kill -9) during active streams → must resume cleanly post-restart; no corrupt output; no orphan FDs.
* **Bus lag/drops** → no deadlocks; health endpoints remain sane.
* **Downstream latency/jitter** → verify **shed-before-collapse** and SLOs within +10% baseline.
* **Decompression bombs/slow-loris** at ingress → 413/429/503 as appropriate; no memory creep.

**Acceptance:** **24 h** soak → zero FD leak, stable RSS, SLOs honored, alerts fire and auto-resolve (see `RUNBOOK.md` Gate J).

**Run (hints):**

* Use `testing/chaos/latency_injector.rs` + docker compose for mock downstreams.
* Drive traffic via `wrk`/`bombardier` profiles in `testing/performance/`.

### 1.6 Performance / Load

**Scope:** Throughput, latency, quotas; guardrails per `PERFORMANCE.md`.
**SLO assertions (intra-AZ, 16-core AMD64 baseline):**

* Hydration **p95 ≤ 150 ms**; range-start **p95 < 100 ms**; public GET p95 **< 80 ms** intra-region.
* Shed **< 1%** steady-state; 5xx (excl. 429/503) **< 0.1%**.

**Tools:**

* `criterion` for micro-benches: planner, serde, policy.
* `wrk` / `bombardier` for HTTP profiles.
* Optional: `coz`/`perf` for causal profiling (artifact upload on regression).

**Run (examples):**

```bash
cargo bench -p omnigate
bombardier -c 256 -n 200000 http://127.0.0.1:9444/v1/view/home
wrk -t8 -c256 -d2m http://127.0.0.1:9444/v1/media/obj123 -H 'Range: bytes=0-65535'
```

---

## 2) Coverage & Gates

> Coverage measured with **grcov** (LLVM source-based) or **tarpaulin** (Linux).
> Perf baselines stored under `testing/performance/baselines/` and updated via ADR + reviewer sign-off.

### 2.1 Bronze (MVP)

* Unit + integration tests **pass**.
* Coverage **≥ 70%** lines / **≥ 60%** branches (excluding mocks).
* Fuzz harness **builds**; seed corpus committed.
* Minimal perf profile runs (smoke: ≤5 min) without regression.

### 2.2 Silver (Useful Substrate)

* Property tests **present** and run in CI.
* Fuzz per-target **≥ 1 h** nightly; **zero crashes**.
* Coverage **≥ 85%** lines / **≥ 75%** branches.
* Chaos scripts exist; **1 h** soak on PRs labeled `perf:rapid`.
* Perf regression gate: p95/throughput must be within **±10%** of last green.

### 2.3 Gold (Ops-Ready)

* Fuzz **≥ 4 h** nightly per target; **0** new findings **7 days**.
* Chaos/soak **24 h** weekly on `main`; no FD/memory leaks; alerts fire and auto-resolve.
* Coverage **≥ 90%** lines / **≥ 80%** branches; doc-tests included.
* Perf regressions **blocked** (latency ↑ >10% or throughput ↓ >10% fails CI).
* Amnesia soak test passes (no disk artifacts).
* Facet-specific SLOs enforced if PR labeled `facet:media|feed`.

---

## 3) Invocation Examples

### 3.1 All tests (workspace hygiene)

```bash
cargo test -p omnigate --all-targets -- --nocapture
```

### 3.2 Integration only

```bash
cargo test -p omnigate --test '*'
```

### 3.3 Property tests

```bash
cargo test -p omnigate --features proptest -- --nocapture
```

### 3.4 Fuzz target (1 minute smoke)

```bash
cargo fuzz run envelope_decode -- -max_total_time=60
```

### 3.5 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p omnigate --test loom_*
```

> Loom tests live in `tests/loom_*.rs` and model: **one-writer-per-socket**, **bounded queues**, **no locks across `.await`** on hot paths.

### 3.6 Micro-benchmarks

```bash
cargo bench -p omnigate
```

### 3.7 Coverage (grcov)

```bash
# Linux/macOS with llvm-tools-preview installed
cargo clean
RUSTFLAGS="-Zinstrument-coverage" LLVM_PROFILE_FILE="omnigate-%p-%m.profraw" \
  cargo test -p omnigate --all-targets
grcov . -s . -t lcov --binary-path ./target/debug/ -o ./target/coverage/omnigate.lcov
```

---

## 4) Observability Hooks

* **Structured JSON** logs on failure with `corr_id`, `route`, `downstream`, `latency_ms`, and `reason`.
* Test harness must **propagate `corr_id`** across calls and include it in assertions on logs/metrics.
* Prometheus scraping enabled for integration/perf tests; store sample snapshots under `testing/performance/snapshots/` on CI failures.
* Each chaos/perf run uploads **flamegraph** + **console traces** as artifacts (see CI).

---

## 5) CI Enforcement

**GitHub Actions (suggested jobs):**

* **lint:** `cargo fmt -- --check` ; `cargo clippy --workspace -- -D warnings`
* **unit+integration:** `cargo test --workspace --all-targets`
* **deny:** `cargo deny check advisories bans sources licenses`
* **coverage:** grcov (upload `omnigate.lcov` to artifact)
* **fuzz (nightly):** `cargo fuzz run <each_target> -- -max_total_time=14400`
* **perf (nightly):** run `testing/performance/run_profile.sh` against the service; **compare to baselines**; fail if thresholds breached
* **chaos/soak (weekly on main):** 24h job; enforce zero-leak and alert-fire criteria
* **loom (PR-labeled `concurrency`):** `RUSTFLAGS="--cfg loom" cargo test -p omnigate --test loom_*`

**Failure policy:**

* Any **panic**, **OOM**, or **ASAN/LSAN** finding (optional nightly) fails.
* Perf regression: p95 ↑ >10% or throughput ↓ >10% at fixed mix **fails**.
* Label-aware gates: `facet:media|feed` requires facet-specific tests to pass.

---

## 6) Suite Layout

```
crates/omnigate/
├─ src/
│  └─ ...                               # normal code; small unit tests inline
├─ tests/
│  ├─ integration_views.rs              # /v1/view/* hydration happy/sad paths
│  ├─ integration_media_range.rs        # range start/continuation; ETag; 206/416
│  ├─ integration_config_reload.rs      # reload limits; verify live effect
│  ├─ integration_amnesia.rs            # Micronode RAM-only; disk watchers assert
│  ├─ loom_backpressure.rs              # loom models for inflight/semaphores
│  └─ security_hardening.rs             # body cap; decompression guard behavior
├─ testing/
│  ├─ mocks/
│  │  ├─ index_mock.rs                  # latency/fault knobs
│  │  ├─ storage_mock.rs                # range/partial, 5xx, slow-loris
│  │  └─ mailbox_mock.rs                # fan-out variants
│  ├─ chaos/
│  │  └─ latency_injector.rs
│  ├─ performance/
│  │  ├─ run_profile.sh                 # wrk/bombardier drivers
│  │  └─ baselines/
│  │     ├─ hydration_intra_16c.json
│  │     └─ range_start_intra_16c.json
│  └─ corpora/
│     ├─ envelopes/
│     ├─ caps/
│     └─ range_headers/
└─ fuzz/
   ├─ Cargo.toml
   └─ fuzz_targets/
      ├─ envelope_decode.rs
      ├─ policy_caps.rs
      └─ range_headers.rs
```

---

## 7) Required Assertions by Area

* **Ingress Hardening:**

  * Requests >1 MiB body → 413; decompression >10× → 400/413; slow-loris → 408/429 (depending on layer).
* **Backpressure:**

  * When `inflight >= OMNIGATE_INFLIGHT_MAX`, new requests **shed** (429/503) and **do not** starve in-flight streams.
* **Streaming Discipline:**

  * For large objects, RSS remains stable (chunked streaming); no full-buffer spikes; zero-copy paths exercised (`bytes::Bytes`).
* **Concurrency:**

  * **No locks across `.await`** on hot paths; one writer per socket; bounded channels. (Enforced via loom + code review hints.)
* **Policy/Capability:**

  * Deny decisions are deterministic; cache TTL honored; tampered caps rejected without panic.
* **Micronode Amnesia:**

  * With `--amnesia`, no filesystem writes occur; temp dirs cleaned on shutdown.
* **Perf SLOs:**

  * Hydration p95 ≤ 150 ms; range-start p95 < 100 ms; sheds < 1% steady; 5xx < 0.1%.

---

## 8) Open Questions (crate-specific)

* Which planner invariants should be formalized in **TLA+** (optional) for scaling DAG correctness?
* Minimal **hedged reads** budget that improves p95 without inflating downstream load?
* PQ/ZK pilots: which perf counters become **gates** (e.g., handshake p95 delta ≤ +5 ms intra-region; econ p95 ≤ 250 ms)?
* Do we require **ASAN/LSAN** on nightly in CI for this crate, or rely solely on fuzz + heaptrack?

---

## 9) Appendix — Useful Commands

**Run mocks + service (two shells):**

```bash
# 1) start omnigate
OMNIGATE_INFLIGHT_MAX=512 RUST_LOG=info,omnigate=debug \
  cargo run -p omnigate -- --config ./configs/omnigate.toml

# 2) start mocks (index/storage/mailbox)
cargo run -p omnigate --bin mocks -- --latency 15ms --p50 80ms --p95 150ms
```

**Quick perf smoke (local):**

```bash
bombardier -c 128 -n 50000 http://127.0.0.1:9444/v1/view/home
wrk -t4 -c128 -d60s http://127.0.0.1:9444/v1/media/obj123 -H 'Range: bytes=0-65535'
```

**Generate flamegraph on failed perf run:**

```bash
cargo flamegraph -p omnigate --bin omnigate
```

---

### ✅ Summary

This testing contract ensures omnigate’s **stateless streaming**, **backpressure safety**, and **SLO compliance** do not silently drift. The suite spans logic purity (unit), interop (integration), rigor (property/fuzz), resilience (chaos/soak), and efficiency (perf), with CI gates and baselines to enforce **shed-before-collapse** and amnesia rules across environments.

---
