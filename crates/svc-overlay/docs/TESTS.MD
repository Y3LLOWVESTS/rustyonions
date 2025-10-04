
---

# 🧪 TESTS.md — `svc-overlay`

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **testing contract** for `svc-overlay`:

* Unit, integration, property, fuzz, chaos, loom, performance.
* Explicit coverage goals & **Bronze → Silver → Gold** acceptance gates.
* Copy-paste invocations for devs & CI.

**Scope reminders (normative):**

* **OAP/1** framing: `max_frame = 1 MiB`; stream in **~64 KiB** chunks; no full-buffering.
* **Truthful readiness**: `/readyz` must flip under overload/dependency failure.
* **QUIC 0-RTT disabled** by default; PQ-hybrid handshake may add ≤30% overhead.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100ms)

**Scope:** pure logic, guards, small I/O shims.
**Location:** `src/**`, `#[cfg(test)]`.

**Must-cover units:**

* Frame sizing & caps: `max_frame` (1 MiB), `ratio_cap`, `content_type` guards.
* Window/backpressure calculators (`peer.send_window_frames`, `inflight_frames`).
* Timeout selection (read/write/idle, handshake).
* Admin handlers return cheap/constant-time responses for `/healthz`, `/readyz`, `/version`.
* Config parse/merge/defaults; env override precedence.
* Feature flags toggles (tor/quic enabled/disabled wiring).

Run:

```bash
cargo test -p svc-overlay --lib
```

### 1.2 Integration Tests (E2E, `tests/*.rs`)

**Scope:** crate surface via public API/admin; one-node & multi-node paths.

**Mandatory suites:**

* **Admin Roundtrip:** `/healthz`, `/readyz`, `/version`, `/metrics` (status codes, latency < 30ms locally).
* **OAP Session Handshake:** HELLO→READY success; failure cases (oversize, bad magic, timeout).
* **OAP Data Path:** 1 KiB control, 64 KiB, 1 MiB frames—verify streaming (no >64 KiB buffer growth), checksum, and backpressure behavior.
* **Config Reload:** `SIGHUP` or watcher stub → config version increments; invariants preserved (timeouts/windows capped).
* **Concurrency Semantics:** graceful shutdown drains in-flight; no new sessions after drain signal.
* **Readiness Truthfulness:** drive overload → `/readyz` flips to 503; recover → 200 OK.
* **Gateway→Overlay Path:** stub gateway issues GET; overlay returns framed object; logs/metrics include `corr_id`.
* **PQ Mode Toggle:** `OVERLAY_PQ_MODE=hybrid` vs `classical`; assert p95 handshake delta ≤ 30% on local runner.
* **QUIC Listener (feature-gated):** start QUIC listener; 0-RTT disabled; handshake succeeds; anti-amplification respected.

Run:

```bash
cargo test -p svc-overlay --test '*'
```

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Targets & invariants:**

* **OAP/1 Codec RT:** `decode(encode(frame)) == frame` for legal frames across sizes (1 B .. 1 MiB).
* **Rejection Monotonicity:** any byte flip that violates caps must yield a rejection (never a panic).
* **Idempotent State Transitions:** applying READY again is a no-op; window updates don’t exceed caps.
* **Timeout Algebra:** derived timeouts never < 0; `idle >= read/write`.

Hints:

* Strategy for frames: random header fields within legal ranges; random payloads with size classes S/M/L.
* Shrinking must preserve invalidity for negative tests.

Run (example module):

```bash
cargo test -p svc-overlay --test props_oap
```

### 1.4 Fuzz Tests

**Tooling:** `cargo fuzz` with `libFuzzer` targets.

**Mandatory fuzz targets:**

* `oap_frame_parse_fuzz`: random bytes → frame parser (expect decode or structured rejection; never UB/panic).
* `oap_header_decode_fuzz`: header-only decoder (bit flips on flags/sizes).
* `admin_path_fuzz`: HTTP admin path parser (reject directory traversal, bad headers).
* (if QUIC enabled) `quic_stream_fuzz`: stream open/close/order anomalies (feature-gated).

**Corpus:**

* Seed from **proptest failure artifacts**, CI regressions, and red-team captures.
* Keep under `fuzz/corpus/<target>/*`; minimize on PR.

**Acceptance:** **4h** fuzz (nightly) → **zero crashes**, **zero OOM**.

Dev runs:

```bash
cargo fuzz run oap_frame_parse_fuzz -- -max_total_time=60
cargo fuzz run oap_header_decode_fuzz -- -max_total_time=60
```

### 1.5 Chaos / Soak Tests (service level)

**Harness:** `testing/overlay-smoke` (custom), optional `ronctl chaos`.

Inject:

* **Process crash** at random (supervisor/pod restarts) → ready within 5s; no partial readiness.
* **Bus lag/drops** → no deadlock; events recovered; `bus_overflow_dropped_total == 0` (or below budget).
* **Slowloris peers** on OAP → backpressure engages; no memory growth.
* **Disk slow/full** (if any temp spill configured) → reject gracefully; no hangs.

**Acceptance:** **24h** soak at **Bronze** rate (see PERFORMANCE.md) → **zero FD leaks**, **stable RSS**, **no latency drift**.

Run (examples):

```bash
ronctl chaos inject --target svc-overlay --fault latency --ms 150 --duration 5m
ronctl chaos inject --target svc-overlay --fault slowloris --conns 50 --duration 10m
testing/overlay-smoke --rate bronze --duration 24h
```

### 1.6 Performance / Load Tests

**Tools:** `criterion` (micro), `testing/overlay-smoke` (integration), `hyperfine` for admin.

**Measures:**

* **Handshake p95:** intra-node < 50 ms; PQ-hybrid < 75 ms.
* **Data p95:** ≤256 KiB intra < 80 ms; 1 MiB intra < 120 ms.
* **Throughput:** Bronze ≥ 500 rps @ 4 vCPU / 8 GiB; Silver ≥ 1k rps @ 8 vCPU / 16 GiB.
* **SDK parity:** Rust vs TS client p95 within **10%** (intra-region).

Run:

```bash
cargo bench -p svc-overlay
testing/overlay-smoke --mix S=50,M=40,L=10 --conn-reuse --duration 15m
hyperfine 'curl -s localhost:9600/readyz'
```

### 1.7 Concurrency Model (Loom)

**Tooling:** `loom` feature.
**Targets:**

* **Graceful Shutdown:** no lost wakeups; channels drain; no deadlock.
* **Backpressure Path:** window & inflight counters never underflow/overflow across interleavings.
* **Config Watcher:** single atomic swap; readers never see torn config.

Run:

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-overlay --test loom_*
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests passing locally and in CI.
* **Coverage ≥ 70%** (lines) via grcov or tarpaulin.
* Fuzz harness builds and runs for ≥60s (smoke).

### 2.2 Silver (Useful Substrate)

* Property tests present for codecs/state.
* CI fuzz run **≥ 1h** with zero crashes.
* Chaos scripts land in repo and run for **≥ 30m**.
* **Coverage ≥ 85%**.
* Loom suite covers shutdown, backpressure, config swap.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h**, zero crashes.
* **24h soak** at Bronze rate; no leaks/drift; `/readyz` truth verified.
* **Coverage ≥ 90%** and enforced as a PR status check.
* Perf regressions gated (±10% thresholds vs baselines).
* SDK parity check (Rust vs TS) enforced (≤10% delta).

---

## 3) Invocation Examples

### 3.1 All tests

```bash
cargo test -p svc-overlay --all-targets -- --nocapture
```

### 3.2 Specific integration suite

```bash
cargo test -p svc-overlay --test overlay_admin_roundtrip
cargo test -p svc-overlay --test oap_session_handshake
cargo test -p svc-overlay --test oap_streaming_datapath
```

### 3.3 Fuzz targets (short smoke)

```bash
cargo fuzz run oap_frame_parse_fuzz -- -max_total_time=60
cargo fuzz run oap_header_decode_fuzz -- -max_total_time=60
```

### 3.4 Loom (model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-overlay --test loom_shutdown
RUSTFLAGS="--cfg loom" cargo test -p svc-overlay --test loom_backpressure
RUSTFLAGS="--cfg loom" cargo test -p svc-overlay --test loom_config_swap
```

### 3.5 Benchmarks

```bash
cargo bench -p svc-overlay
testing/overlay-smoke --rate bronze --duration 15m
```

---

## 4) Observability Hooks

* All tests emit **structured JSON logs** on failure (include `corr_id`, `peer_id`, `frame_class`).
* Prometheus scrape enabled on test admin port; store `.prom` snapshots in `testing/artifacts/metrics/`.
* Integration tests **propagate corr_ids** across gateway→overlay so traces line up.
* Failures must attach:

  * last 200 lines of logs (`testing/artifacts/logs/*.log`)
  * flamegraph SVG for perf failures (`testing/artifacts/flamegraphs/*.svg`)
  * fuzz reproducers (`fuzz/artifacts/<target>/*`)

---

## 5) CI Enforcement

**GitHub Actions jobs (minimum):**

* `test`:

  * `cargo test --workspace --all-targets --locked`
  * `cargo fmt -- --check`
  * `cargo clippy --workspace -- -D warnings`
* `security`:

  * `cargo deny check advisories bans sources licenses`
* `coverage`:

  * `cargo build --tests`
  * grcov/tarpaulin job → artifact + PR comment; **fail if < gate**
* `fuzz-nightly`:

  * each target ≥ 1h (Bronze) / 4h (Gold) with corpus persist/minimize
* `perf-baseline-nightly`:

  * run `testing/overlay-smoke` and `cargo bench`
  * compare to `testing/performance/baselines/`; **fail on ±10% breach**
* `loom`:

  * run `RUSTFLAGS="--cfg loom" cargo test -p svc-overlay --test 'loom_*'`

Artifacts uploaded to:

* `testing/artifacts/logs/`, `testing/artifacts/flamegraphs/`, `testing/performance/reports/`, `fuzz/artifacts/`

---

## 6) Open Questions (crate-specific to finalize)

* **Loom scope:** which additional critical paths (e.g., bus publisher/consumer) warrant loom modeling?
* **Fuzz targets:** add `oap_length_prefixed_fuzz` (length-prefix corner cases)? QUIC stream reorderer under feature?
* **Perf SLOs:** any AZ/region-specific SLO deviations for edge/micronode profiles?
* **Compression:** zstd on/off matrix per size class—keep or drop from nightly due to cost?
* **Backpressure policy:** formal property (no starvation) as a prop-test?

---

## 7) Traceability Matrix (what proves what)

| Requirement / Invariant                     | Proof Artifact                                 |
| ------------------------------------------- | ---------------------------------------------- |
| OAP `max_frame=1MiB`, 64 KiB streaming      | `oap_streaming_datapath` integration + props   |
| Truthful `/readyz` under overload/deps      | `overlay_admin_roundtrip` + chaos latency test |
| Graceful shutdown drains in-flight          | `loom_shutdown` + integration `shutdown_drain` |
| Backpressure bounded; no overflow/underflow | `loom_backpressure` + unit window calc tests   |
| PQ-hybrid overhead ≤ 30%                    | perf report + bench `handshake_pq_hybrid`      |
| SDK parity ≤ 10% (Rust vs TS)               | `overlay-smoke` report (polyglot mode)         |
| Zero panics on arbitrary wire input         | fuzz targets (`oap_*_fuzz`) nightly            |
| No leaks / stable RSS over 24h              | soak report + /metrics snapshots               |

---

With this test plan in place, `svc-overlay` has a **clear, auditable, CI-enforced testing contract** that anchors your invariants, validates scaling knobs, and keeps ops-critical guarantees from drifting.
