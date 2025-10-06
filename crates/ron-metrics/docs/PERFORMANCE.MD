

---

# ⚡ PERFORMANCE.md — ron-metrics

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-05
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of `ron-metrics`:

* SLOs for the tiny HTTP exposer (`/metrics`, `/healthz`, `/readyz`) and library-level overhead.
* Benchmarks & workloads it must sustain.
* Perf harness & profiling tools.
* Scaling knobs, bottlenecks, and triage steps.
* Regression gates to prevent silent perf drift.

It ties directly into:

* **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
* **Omnigate Build Plan** milestones Bronze→Gold.
* **Perfection Gates**: **F** (perf regressions barred), **L** (scaling chaos-tested).

---

## 1. SLOs / Targets

> Scope: single process, steady-state scrapes, Prometheus text v0, default histogram buckets. Targets are measured on loopback unless stated. TLS handshake excluded from `/metrics` SLI (measure it separately under “TLS handshake”).

### 1.1 Exposer (HTTP)

* **/healthz latency (local)**

  * p50 ≤ **0.5 ms**, p95 ≤ **2 ms**, p99 ≤ **5 ms** @ 32 concurrent, 500 RPS sustained.

* **/readyz latency (local, ready)**

  * p50 ≤ **0.8 ms**, p95 ≤ **3 ms**, p99 ≤ **7 ms** @ 32 concurrent, 300 RPS sustained.
  * Not-ready path returns `503` with `Retry-After`; p95 ≤ **4 ms**.

* **/metrics exposition latency (local)**

  * With **first-party families only** (~dozens of lines): p50 ≤ **1 ms**, p95 ≤ **5 ms**, p99 ≤ **10 ms** @ 16 concurrent, 200 RPS sustained.
  * With **large registry** (~10k time series): p50 ≤ **15 ms**, p95 ≤ **60 ms**, p99 ≤ **120 ms** @ 4 concurrent, 10 RPS sustained.
  * **Budget**: text size **≪ 4 MiB** (alert at 2 MiB).

* **TLS keep-alive (local)**

  * p95 ≤ **12 ms** for `/metrics` (steady-state).
  * **Handshake SLI** (cold): p95 ≤ **120 ms** (local); not counted against `/metrics` SLO.

* **Throughput ceilings** (guidance, not a hard SLO):

  * `/healthz`: **≥ 2,000 RPS** on loopback.
  * `/readyz` (ready): **≥ 1,000 RPS**.
  * `/metrics` (first-party only): **≥ 300 RPS**; (10k series) **≥ 10 RPS**.

* **Error budget (exposer)**

  * 5xx < **0.1%** (excluding intentional `503` for not-ready).
  * Timeout/`-1` socket errors < **0.05%**.
  * Method violations (non-GET) → `405` are not counted as errors.

* **Resource ceilings @ target load**

  * CPU: < **5%** of one core for `/metrics` with first-party families; < **30%** for 10k series @ 10 RPS.
  * RSS steady-state: < **64 MiB** (exposer only; registry memory is shared with host).
  * FD usage: < **1%** of system limit.

### 1.2 Library overhead (hot-path use)

* **Counter/Histogram ops** (increment/observe)

  * Mean cost ≤ **200 ns** per op (amortized; lock-free fast path).
  * Allocations/op: **0** on hot path.

* **Cold start**

  * `Metrics::new()` register families ≤ **5 ms** (no network I/O).
  * `serve()` bind (loopback): ≤ **10 ms** to bound port, start task.

* **Amnesia mode**

  * Label `amnesia="on|off"` added to all first-party families; overhead: **negligible** (label set added once, reused).

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

* **Targets**:

  * Encoding `/metrics` text for small/large registries.
  * Counter/Histogram hot-path increments.
* **Location**: `benches/exposer_bench.rs`, `benches/hotpath_bench.rs`.
* **Run**:

  ```
  cargo bench -p ron-metrics
  ```

### 2.2 Integration load (wrk | bombardier)

* **Local loopback (no TLS)**:

  ```
  # /metrics, first-party families
  bombardier -c 16 -d 60s -l http://127.0.0.1:9100/metrics

  # /metrics, large registry (set RON_METRICS_LOAD=large in host to synthesize 10k series)
  bombardier -c 4 -d 60s -l http://127.0.0.1:9100/metrics

  # /healthz and /readyz
  bombardier -c 32 -d 60s -l http://127.0.0.1:9100/healthz
  bombardier -c 32 -d 60s -l http://127.0.0.1:9100/readyz
  ```

* **TLS (keep-alive)**:

  ```
  bombardier -c 16 -d 60s -l https://127.0.0.1:9443/metrics
  ```

### 2.3 Profiling

* **Flamegraph** (Linux/macOS):

  ```
  cargo flamegraph -p ron-metrics --bin exposer_demo
  ```

* **tokio-console** (async stalls):

  ```
  RUSTFLAGS="--cfg tokio_unstable" \
  RUST_LOG=trace \
  TOKIO_CONSOLE_BIND=127.0.0.1:6669 \
  cargo run -p ron-metrics --bin exposer_demo
  ```

* **Causal profiling (coz)**:

  ```
  coz run -- cargo run -p ron-metrics --bin exposer_demo
  ```

* **CLI latency sanity (if demo CLI present):**

  ```
  hyperfine --warmup 5 'curl -sS http://127.0.0.1:9100/metrics > /dev/null'
  ```

### 2.4 Chaos/perf blend

* **Latency injection**: simulate slow registry collectors.
* **Slow-loris**: ensure request timeouts close sockets.
* **Compression bombs**: verify we don’t negotiate compression (gateway responsibility).

### 2.5 CI integration

* Nightly perf job executes:

  * Criterion suite; compares medians vs baselines.
  * `/metrics` bombardier run (30–60s) on loopback; extracts p50/p95/p99, RPS, non-2xx.
  * Stores artifacts and JSON results.

---

## 3. Scaling Knobs

* **Concurrency / backpressure**

  * Max concurrent requests capped via Tower layer (e.g., 64).
  * Inflight gauge is optional; 429/503 preferred over queue growth.

* **Buffers / memory**

  * Use `bytes::Bytes` for response buffers.
  * Tune `/metrics` text buffer: pre-size based on previous write length (sticky heuristic).
  * Histogram buckets: keep default bounded sets; do not add high-cardinality labels.

* **I/O**

  * Prefer loopback or UDS for scrapes.
  * Keep-alive enabled; read/write/idle timeouts per CONFIG.

* **TLS**

  * `tokio_rustls::rustls::ServerConfig`; enable session resumption for steady-state.
  * Record handshake latency separately to guard noise in `/metrics` SLI.

* **Feature flags**

  * `otel` exporter can be toggled off in constrained environments.
  * PQ/ZK families are always present (values may be 0); no perf impact when unused.

* **Horizontal**

  * Exposer is stateless → scale with replicas if scrape fan-out demands.

---

## 4. Bottlenecks & Known Limits

* **Large registries (10k+ series)**

  * Text encoder dominates CPU; bucket heavy histograms increase output size.
  * Acceptable at low RPS (≥10 RPS target); beyond that, consider scrape interval ↑ or per-service selective registry.

* **TLS handshake bursts**

  * Cold connections inflate tail latencies; prefer keep-alive and gateway-side TLS termination.

* **High-cardinality labels (anti-goal)**

  * Not supported for first-party families. Host-added families must keep cardinality low; CI checks enforce.

* **Compression**

  * Not negotiated by the exposer; let gateway handle transport compression if required.

* **Non-GET methods**

  * Rejected with 405; no cycles spent parsing bodies.

Bronze milestone: hits loopback SLOs for first-party registries.
Gold milestone: sustains 10k-series registry at stated budgets, with chaos toggles exercised and no regression gates tripped.

---

## 5. Regression Gates

> Gate **F**: perf regressions barred; Gate **L**: scaling validated under chaos.

* **Fail PR** if any of the below vs baseline (median of 3 runs):

  * `/metrics` p95 ↑ **>10%** (both small and large registry modes).
  * `/metrics` throughput ↓ **>10%** at fixed concurrency.
  * `/healthz` or `/readyz` p95 ↑ **>20%**.
  * CPU or RSS ↑ **>15%** for the same workload.
  * Text size for first-party families ↑ **>10%** without CHANGELOG note.

* **Baselines**

  * Stored under `testing/performance/baselines/ron-metrics/`:

    * `small_registry.json`, `large_registry.json`
    * `tls_keepalive.json`

* **Waivers**

  * Allowed only with explicit justification (e.g., upstream prometheus crate change) and a temporary suppression that expires within one release.

---

## 6. Perf Runbook (Triage)

1. **Confirm readiness**

   * `/readyz` should be 200; if 503, resolve missing deps first.

2. **Check metrics**

   * Inspect `exposition_latency_seconds{endpoint="/metrics"}` and request counters.
   * Verify `amnesia` label presence (truthful to CONFIG), base labels on all families.

3. **Flamegraph**

   * Run `cargo flamegraph` during a bombardier run; hotspots typically in text encoding or histogram aggregation.

4. **tokio-console**

   * Look for long `.await` or blocked tasks; verify no lock spans across `.await` (IDB invariant).

5. **Stress knobs**

   * Lower concurrency cap; increase keep-alive; pre-size buffers using previous content length.
   * For large registry: reduce series count or scrape interval; gate debug families.

6. **TLS isolation**

   * If tails inflate only under TLS, check handshake vs steady-state; enable session resumption or terminate at gateway.

7. **Chaos toggles**

   * Disable any simulated latency/slow-loris; rerun to isolate.

8. **Cardinality audit**

   * Ensure no high-cardinality labels were introduced; enforce base/suffix invariants.

9. **Document**

   * If budget exceeded, add a short postmortem and a baseline update (only after cause understood).

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs validated locally for `/healthz`, `/readyz`, and `/metrics` (small & large registries).
* [ ] Criterion benches exist and run green.
* [ ] Load harness (bombardier/wrk) scripted and produces JSON artifacts.
* [ ] Flamegraph and tokio-console traces captured at least once and archived.
* [ ] Scaling knobs documented in CONFIG and here.
* [ ] Regression gates wired into CI with baselines.
* [ ] Chaos/perf blend executed (slow-loris, latency inject) with no gate failures.
* [ ] CHANGELOG updated for any meaningful perf-affecting change (encoder, buckets, label sets).

---

## 8. Appendix

**Reference SLOs (Scaling Blueprint)**

* Intra-region p95 GET < **80 ms**; inter-region < **200 ms** (general service targets; `ron-metrics` meets far below).
* Failures < **0.1%**.

**Reference workloads**

* `gwsmoke` GET/HEAD/RANGE (service baseline).
* Soak test 24h on gateway+mailbox (observability probes enabled).

**Perfection Gates tie-in**

* **Gate F**: PRs fail on perf regression (thresholds in §5).
* **Gate L**: scaling validated under chaos (latency inject, slow-loris).

**History**

* Keep dated notes of any regression, cause, and fix; update baselines deliberately.

---

### Handy Commands

```
# Small registry loopback perf
bombardier -c 16 -d 60s -l http://127.0.0.1:9100/metrics -o json > small_registry.json

# Large registry loopback perf (host enables synthetic 10k series)
RON_METRICS_LOAD=large bombardier -c 4 -d 60s -l http://127.0.0.1:9100/metrics -o json > large_registry.json

# Quick latency spot-check
hyperfine --warmup 5 'curl -sS http://127.0.0.1:9100/healthz > /dev/null'
```

---

**Notes**

* Numbers above are **SLO targets and CI thresholds**; absolute values depend on hardware. Capture baselines on your CI runners and tune only with evidence.
* Ensure **suffix discipline** (`*_seconds|*_bytes|*_total`) and **base labels** (`service,instance,build_version,amnesia`) stay intact; they are part of the perf contract (dashboards/alerts).
