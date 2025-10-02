

---

````markdown
# 🧪 TESTS — ron-transport

*Audience: developers, auditors, CI maintainers*  
*MSRV: 1.80.0 (Tokio/loom compatible)*

This document is the testing **contract** for `ron-transport`. It maps crate **invariants** to concrete tests, defines **gates** (Bronze→Silver→Gold), and standardizes local & CI **invocation**.

---

## 0) Scope & Goals

We test the transport **library** (TCP/TLS and optional Tor via Arti) for:

- Correctness of **invariants** (I-1…I-18 in IDB): single‐writer, no locks across `.await`, strict I/O ceilings (≤1 MiB / ~64 KiB), **8× decompression guard**, quotas before work, readiness tri-state, amnesia, and error taxonomy stability.
- **Resilience** under contention, cancellation, and faults (timeouts, bootstrap stalls).
- **Performance** SLO guardrails (handshake p95, rejects under stress).
- **Drift control**: feature matrix, multi-runtime (Tokio current_thread & multi_thread), and observability surface (buckets, reason strings).

> Note: `ron-transport` does **not parse OAP**—tests exercise I/O limits and framing caps, not protocol decoding.

---

## 1) Taxonomy

### 1.1 Unit tests (fast, <100 ms each)

Location: `src/**/*` with `#[cfg(test)]`.

**Required units**
- **Rate limiter**: token math monotonicity and burst behavior.
- **Length-guards**: `max_frame_bytes` enforcement (reject >1 MiB).
- **Chunking**: ~64 KiB chunk emission under varying input sizes.
- **Error taxonomy**: every public error maps to a stable `kind()` string.
- **Zeroize loader**: PEM key loader zeroizes buffers on parse error/success.
- **Config validation**: boundary values (min/max sizes, timeouts).
- **UDS guards (unix)**: perms checks (0700 dir / 0600 socket).

Run:
```bash
cargo test -p ron-transport --lib
````

### 1.2 Integration tests

Location: `tests/*.rs`. Use only loopback/UDS and in-proc Arti (no external daemons).

**Must include**

* **TLS listener/dialer** happy path with self-signed cert → handshake OK; **handshake_latency_seconds** buckets populated.
* **Feature matrix**: `--no-default-features`, `--features tls`, `--features arti`, `--features tls,arti`.
* **Readiness**: Arti backend starts `Degraded("arti_bootstrap")` → `Ready` (or `NotReady` after configured timeout).
* **Backpressure**: semaphore caps produce **early reject** (`rejected_total{reason="peer_limit"}`) not buffering.
* **Per-conn / per-peer / global rate limits**: enforced and observable (rejects increment, pacing within ±20%).
* **UDS** (unix): positive and negative PEERCRED paths with `allow_uids`.
* **Config reload**: non-disruptive knobs hot-swap; disruptive knobs require rebind (assert service behavior via test harness).
* **Error reasons**: all rejects surface canonical `reason` labels.

Run:

```bash
cargo test -p ron-transport --test '*'
```

### 1.3 Property-based tests

Tooling: `proptest` (preferred).

**Properties**

* **Token bucket**: never overspends; time scaling linearity; ceiling respected.
* **Chunker**: concatenation of emitted chunks == input; each chunk ≤ configured size; last chunk ≤ size.
* **Durations / sizes**: parse↔format round-trip within accepted units.
* **Cancellation**: arbitrary cancel points ⇒ all tasks quiesce ≤100 ms (uses mock duplex with injected cancellations).
* **Inflation guard**: arbitrary “compressed” sequence cannot produce >8× expansion via the guard adapter (pure guard test; not tied to a specific codec).

Run:

```bash
cargo test -p ron-transport --features proptest -- --nocapture
```

### 1.4 Fuzz tests

Tooling: `cargo fuzz` (libFuzzer).

**Targets**

* `fuzz_config_roundtrip`: random TOML/JSON fragments → parse→validate→(optional)serialize never panics.
* `fuzz_pem_loader`: random bytes as PEM; loader never panics; always zeroizes temporary buffers.
* `fuzz_length_guard`: adversarial sequences against the frame/reader guard; never read >1 MiB; rejects cleanly with `TooLarge`.
* `fuzz_reason_wire`: random error paths produce valid/stable `kind()`/`reason` strings.

**Acceptance**

* **CI Bronze:** build targets.
* **Silver:** ≥1 h run, zero crashes.
* **Gold (nightly):** ≥4 h run, zero crashes.

Run (examples):

```bash
cargo fuzz run fuzz_config_roundtrip -- -max_total_time=3600
cargo fuzz run fuzz_length_guard     -- -max_total_time=14400
```

### 1.5 Loom (concurrency model)

Model small interleavings for:

* **Single writer**: no double-writer; reader/writer split safety.
* **Await-holding-lock**: compile-time deny + loom asserts no lock held across await on hot paths.
* **Cancellation**: writer/reader + controller token—eventual cancellation within bound.
* **Backpressure**: semaphore + writer can’t deadlock with reader on shutdown.

Run:

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-transport --test loom_*
```

### 1.6 Soak / Chaos (long-running)

* **Soak (24 h)**: loopback TLS with steady QPS; verify *no* FD/memory leaks; latency histograms steady; no error drift.
* **Chaos**:

  * Add latency/jitter/loss (tc/netem or in-harness shims).
  * Arti: deny directory access to force bootstrap failure → `Degraded` then `NotReady` at `bootstrap_timeout`.
  * Hot peer: 1 peer exceeds per-peer caps → rejects, no collapse.

Run (reference rigs only; nightly/weekly CI job):

```bash
cargo test -p ron-transport --test soak_tls_loopback -- --ignored --nocapture
```

### 1.7 Performance / Benchmarks

Tooling: `criterion`.

**Benches**

* `bench_handshake_tls_loopback`: p50/p95 handshake latency.
* `bench_rate_limiter`: `try_consume` throughput and variance.
* `bench_chunker`: throughput vs chunk size; target stable 64 KiB.

Targets / SLOs (reference rig):

* Intra-AZ handshake **p95 < 80 ms**; cross-region **p95 < 200 ms** (simulated).
* Reject rates (`too_large`/`rate_limited`/`peer_limit`) **< 1%** at target load.

Run:

```bash
cargo bench -p ron-transport
```

---

## 2) Mapping invariants → tests

| IDB Invariant                    | Test ID(s) / Area                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------ |
| **I-3 TLS type discipline**      | IT-TLS-01/02: listener/dialer use `tokio_rustls::rustls::*`; compile-time API check  |
| **I-4 Single writer**            | LOOM-SW-01; INT-CANCEL-01                                                            |
| **I-5 No locks across `.await`** | LOOM-LOCK-01; Clippy deny; code scan in CI                                           |
| **I-6 Timeouts & ceilings**      | IT-TIME-01/02; FUZZ-LG-01 (length guard)                                             |
| **I-7 Owned bytes**              | UNIT-CHUNK-01; no borrowed slices escape (compile check)                             |
| **I-8 OAP limits**               | UNIT-LEN-01; IT-CEIL-01; PERF-CHUNK-01                                               |
| **I-8b 8× decompression guard**  | PROP-INFL-01; (optional) FUZZ-INFL-01                                                |
| **I-9 Observability**            | IT-OBS-01: metrics present; reason labels stable                                     |
| **I-10 PQ-ready**                | IT-PQ-01: pq=off no code path change; (feature `pq`) IT-PQ-02 hybrid handshake smoke |
| **I-11 Amnesia**                 | IT-AMN-01: no disk artifacts with amnesia=ON (incl. Arti); fail on writes            |
| **I-12 Rate limits**             | IT-RATE-01..03: per-conn/per-peer/global; pacing ±20%, rejects increment             |
| **I-13 Error taxonomy**          | UNIT-ERR-01: `kind()` stable mapping coverage                                        |
| **I-14 Readiness**               | IT-READY-01: transitions; Arti bootstrap semantics                                   |
| **I-15 Buckets fixed**           | IT-OBS-02: handshake histogram buckets names/edges as per IDB                        |
| **I-16 SEC/RES/PERF gates**      | CI matrix + gates (Bronze/Silver/Gold)                                               |
| **I-17 In-proc Arti only**       | IT-ARTI-01: no external process; library-only                                        |
| **I-18 No unsafe hot paths**     | `cargo geiger`/audit + Miri (sanity)                                                 |

---

## 3) Gates (Bronze → Silver → Gold)

### Bronze (MVP / PR-gate)

* ✅ Unit + integration pass across **feature matrix**:

  * `--no-default-features`, `--features tls`, `--features arti`, `--features tls,arti`
* ✅ Multi-runtime: tests pass on **Tokio current_thread** and **multi_thread**
* ✅ Clippy (deny await-holding-lock on hot paths), rustfmt check
* ✅ Coverage ≥ **70%** (excludes fuzz/bench)
* ✅ Fuzz targets **build** (no run time requirement)

### Silver (Usefully hard)

* ✅ Property tests run and stable (no flaky seeds)
* ✅ Fuzz run ≥ **1 h** in CI, 0 crashes
* ✅ **Loom** suite passes (SW, LOCK, CANCEL)
* ✅ Coverage ≥ **85%**
* ✅ Basic perf: loopback TLS handshake **p95 < 100 ms** on reference rig
* ✅ Observability contract: all required metrics/labels/buckets present

### Gold (Ops-ready)

* ✅ Nightly fuzz ≥ **4 h**, 0 crashes
* ✅ **Soak 24 h**: no FD/memory leaks; no error drift
* ✅ Perf regression tracked; intra-AZ TLS handshake **p95 < 80 ms**
* ✅ PQ hybrid **smoke** behind `pq` feature (when upstream stack supports)
* ✅ Chaos drills: latency/loss, Arti bootstrap failure, hot peer flood
* ✅ Coverage ≥ **90%**

---

## 4) Invocation cheat-sheet

### Full test sweep (developer)

```bash
# fast path
cargo test -p ron-transport --all-targets -- --nocapture

# feature matrix
cargo test -p ron-transport --no-default-features
cargo test -p ron-transport --features tls
cargo test -p ron-transport --features arti
cargo test -p ron-transport --features tls,arti
```

### Loom

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-transport --test loom_*
```

### Fuzz (examples)

```bash
cargo fuzz run fuzz_config_roundtrip -- -max_total_time=3600
cargo fuzz run fuzz_length_guard     -- -max_total_time=14400
```

### Benchmarks

```bash
cargo bench -p ron-transport
```

### ThreadSanitizer (nightly job)

```bash
RUSTFLAGS="-Zsanitizer=thread" RUSTDOCFLAGS="-Zsanitizer=thread" \
cargo +nightly test -p ron-transport -Zbuild-std --target x86_64-unknown-linux-gnu
```

---

## 5) CI wiring (reference)

**Jobs**

1. **Feature matrix:** build+test four combos (see above).
2. **Multi-runtime:** run suite twice (current_thread vs multi_thread).
3. **Lint:** fmt/clippy (deny on await-holding-lock for hot paths).
4. **Security/audit:** `cargo deny check advisories` + `cargo geiger` (no unsafe on hot paths).
5. **Coverage:** `grcov` (or `cargo tarpaulin` on Linux).
6. **Loom:** dedicated job (`RUSTFLAGS=--cfg loom`).
7. **TSan:** nightly sanitizer job.
8. **Fuzz:** nightly (≥4 h) + on-PR quick 5-min sanity to ensure corpora still compile.
9. **Perf snapshot:** criterion artifacts uploaded; simple regression gate on p95 deltas.
10. **Artifacts:** store failing seeds and PCAPs (if any) as CI artifacts (never store secrets).

**Suggested snippets**

```yaml
strategy:
  matrix:
    features: ["", "tls", "arti", "tls,arti"]
    runtime:  ["current", "multi"]

steps:
- run: cargo test -p ron-transport --features ${{ matrix.features }} -- --nocapture
  env:
    RON_TEST_RUNTIME: ${{ matrix.runtime }}  # harness reads this to select scheduler
```

---

## 6) Observability hooks in tests

* Use a **test recorder** for metrics; assert:

  * `handshake_latency_seconds{backend=...}` histogram exists with canonical buckets.
  * `rejected_total{reason="too_large|rate_limited|peer_limit|timeout|tls|arti_bootstrap"}` increments as expected.
* Emit **JSON** logs with fields `{conn_id, corr_id, backend, reason}` and assert presence on failure paths.
* Ensure reason strings are **stable** (unit snapshot test over the error enum mapping).

---

## 7) Test data & harness rules

* **No external Tor daemons.** Arti runs in-proc; when amnesia=ON, assert no files created.
* **Loopback/UDS only.** Never hit public endpoints in CI.
* **Secrets:** never check in PEM private keys; generate ephemeral self-signed certs in tests.
* **Flaky guards:** mark long soaks as `#[ignore]`; scheduled in nightly pipeline.

---

## 8) Answers to template “Open Questions”

* **Which invariants are loom-checked?**
  Single-writer, no-lock-across-await, deterministic cancellation, backpressure shutdown (I-4, I-5).

* **Which fuzz targets are mandatory?**
  `fuzz_config_roundtrip`, `fuzz_pem_loader`, `fuzz_length_guard` (mandatory); `fuzz_reason_wire` (nice-to-have).

* **What SLOs are measured in perf tests?**
  TLS handshake p95 (<80 ms intra-AZ on reference rig), p99 tail trend; rate-limit pacing within ±20%; reject rates <1% under target load.

---

## 9) Named test IDs (for PR references)

* UNIT-RL-01 (rate limiter monotonic), UNIT-LEN-01 (frame ceiling), UNIT-CHUNK-01 (chunker), UNIT-ERR-01 (error kind map), UNIT-PEM-01 (zeroize).
* IT-TLS-01/02 (listener/dialer), IT-READY-01 (tri-state), IT-RATE-01..03 (per-conn/per-peer/global), IT-OBS-01/02 (metrics/buckets), IT-UDS-01 (PEERCRED), IT-AMN-01 (no disk), IT-CEIL-01 (reject >1 MiB).
* PROP-TB-01 (token bucket), PROP-CH-01 (chunker), PROP-DUR-01 (durations/sizes), PROP-INFL-01 (inflation guard).
* FUZZ-CFG-01, FUZZ-PEM-01, FUZZ-LG-01, FUZZ-RSN-01.
* LOOM-SW-01, LOOM-LOCK-01, LOOM-CANCEL-01, LOOM-BP-01.
* SOAK-TLS-24H-01; CHAOS-ARTI-01; CHAOS-HOT-PEER-01.

---

## 10) Exit criteria

A PR is **mergeable** when it satisfies **Bronze** gates and does not regress any **Gold** nightly signals. A release is **ship-ready** when **Gold** is green for 7 consecutive nightly runs.

```

---

