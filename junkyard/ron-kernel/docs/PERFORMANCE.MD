

---

````markdown
---
title: Performance & Scaling — ron-kernel
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-09-26
audience: contributors, ops, perf testers
---

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of `ron-kernel`:
- Library-level SLOs (event publish/deliver latency, restart detection, config hot-swap).
- Benchmarks & workloads it must sustain.
- Perf harness & profiling tools.
- Scaling knobs, bottlenecks, and triage steps.
- Regression gates to prevent silent perf drift.

It ties directly into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze→Gold.
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).

---

## 1. SLOs / Targets

> Scope reminder: `ron-kernel` is orchestration (supervision, bus, config, health+metrics surfacing). No protocol/data-plane loops live here. Targets below cover the **library** and, where noted, the **demo observability bin**.

### Library SLOs

**Event Bus (broadcast 1→N)**
- p99 **publish→deliver** (1 producer → 8 receivers, `Bytes` payload ~128B):
  - p50 ≤ **20µs**, p95 ≤ **60µs**, p99 ≤ **200µs**
- Sustained **publish rate** with 8 receivers:
  - ≥ **250k events/s** (steady, no lag drops) on 4c/8GB Linux
- **Lag budget** (drop on slow consumer):
  - `bus_lagged_total / events_published_total` ≤ **0.01%**

**Supervisor**
- **Crash detection → restart spawn**:
  - p95 ≤ **10ms**, p99 ≤ **25ms**
- **Restart storm control**:
  - Cap respected (≤ 5/60s per child) with jittered backoff

**Config Hot-Swap**
- **Validate + swap snapshot**:
  - p95 ≤ **10ms**, p99 ≤ **30ms** (median config size ≤ 4KB)

**Readiness Change Propagation**
- Event to observable flip (`/readyz` in demo):
  - p95 ≤ **15ms**

### Demo Observability Bin (localhost only)

**HTTP (health/ready/metrics)**
- p95 `/readyz` ≤ **3ms**, p99 ≤ **10ms**
- `/metrics` scrape renders ≤ **10ms** at 2k series

**Resource ceilings** at library targets (4c/8GB, Linux, Rust 1.80):
- Kernel steady **CPU** < **5% of one core** at 100k events/s fanout
- **Memory** < **100 MB** steady for kernel structures (excl. app/services)
- **FDs** < **256** steady (demo bin)

> Calibrate on your baseline hardware and record in **Appendix → History** after first run.

---

## 2. Benchmarks & Harness

**Micro-benchmarks (Criterion)**
- `broadcast_publish_latency`: 1→{1,2,4,8,16} receivers; payload `{32,128,512}B`; measures publish→first_deliver.
- `broadcast_fanout_throughput`: steady publish at target rates; assert **no lag**.
- `supervisor_restart_path`: measure panic→respawn overhead (mock child).
- `config_hot_swap`: parse+validate+swap of 1–8KB configs.

Run:
```bash
cargo bench -p ron-kernel
````

**Integration load rigs (`testing/performance/*`)**

* `bus_fanout_load.rs`: N producers, M subscribers, adjustable capacities, random consumer stalls (to trigger lag).
* `restart_storm.rs`: inject panics, verify caps/jitter and event metrics.
* `config_reload_loop.rs`: SIGHUP or bus-driven reload at 10–50Hz bursts.

Run:

```bash
cargo run -p ron-kernel --bin perf_bus_fanout
cargo run -p ron-kernel --bin perf_restart_storm
cargo run -p ron-kernel --bin perf_config_reload
```

**Profiling**

* Hotspots: `cargo flamegraph -p ron-kernel`
* Async stalls: `RUSTFLAGS="--cfg tokio_unstable" tokio-console` (when enabled)
* CLI micro-latency: `hyperfine -w 3 'ron-kernel-demo --help'`
* Causal profiling (opt): `coz run -- cargo run ...`

**Chaos/perf blend**

* Latency injection in subscribers, bursty publishers, simulated slow consumer to exercise `bus_lagged_total`.

**CI Integration**

* Nightly perf job compares Criterion baselines (`target/criterion`) and JSON exports in `testing/performance/baselines/`.

---

## 3. Scaling Knobs

* **Bus capacity**: `bus.capacity` (default 4096). Larger capacity reduces drops at cost of memory and tail latency under burst.
* **Receiver fanout**: number of broadcast receivers per task; prefer **one receiver per task** (do not share).
* **Payload type**: prefer `Arc<[u8]>`/`Bytes` over owned `Vec<u8>` to minimize clone costs.
* **Runtime threads**: Tokio worker count (default = cores). For pure kernel workloads, default is typically optimal.
* **Logging verbosity**: `info` vs `debug` has material impact; JSON formatting can dominate under very high rates.
* **Metrics cardinality**: limit label explosion; avoid per-request high-cardinality labels.
* **Backoff jitter**: broader jitter lowers thundering herds at tiny cost to mean time-to-recover.

---

## 4. Bottlenecks & Known Limits

* **Broadcast O(N) fanout**: each publish touches all active receivers; payload cloning dominates if not using `Bytes`/`Arc`.
* **Logging hot path**: synchronous JSON formatting is measurable at >100k events/s; restrict to WARN on steady state.
* **Histogram buckets**: too many buckets for `request_latency_seconds` increases CPU and scrape size; use a modest bucket set.
* **Config reload frequency**: extreme reload rates (≫ 10Hz) can cause unnecessary churn; batched updates recommended.
* **OS scheduling**: saturated cores can delay timers/backoff; keep a CPU headroom buffer.

Classify:

* **Must-fix for Gold**: broadcast cloning patterns, log verbosity under load.
* **Acceptable for Bronze/Silver**: occasional lag drop ≤ 0.01%, demo HTTP p99 ≤ 10ms.

---

## 5. Regression Gates

CI fails if any of the following relative to the stored baseline:

* **Bus latency**: p95 ↑ > **10%** or p99 ↑ > **15%**
* **Throughput**: sustained publish rate ↓ > **10%**
* **Supervisor**: panic→respawn p95 ↑ > **20%**
* **Config swap**: p95 ↑ > **20%**
* **Resource**: CPU or memory at target load ↑ > **15%**

Baselines:

* Stored in `testing/performance/baselines/` (Criterion JSON + derived CSV).
* Update allowed only with **explicit waiver** explaining cause (e.g., dependency upgrade) and attaching new calibration runs.

---

## 6. Perf Runbook (Triage)

1. **Confirm symptom**

   * Check `request_latency_seconds` and `bus_lagged_total`
   * Validate queue depths and drop counters per queue

2. **Flamegraph first**

   * Look for logging/serialization/metrics overhead (“fmt::write”, histogram updates)

3. **tokio-console**

   * Identify stuck tasks, long polls, or starving executors

4. **Reduce noise**

   * Set `log.level=warn`, drop verbose spans, reduce metrics cardinality

5. **Tighten data path**

   * Switch payload to `Bytes`/`Arc<[u8]>`, eliminate copies, pre-allocate where safe

6. **Backpressure tuning**

   * Lower `bus.capacity` to fail fast under destructive bursts or raise it to absorb brief spikes (watch tail latency)

7. **Chaos toggle**

   * Remove artificial delays, disable intermittent chaos; confirm headroom

8. **Re-measure**

   * Re-run `cargo bench` and perf rigs; compare deltas to baselines

---

## 7. Acceptance Checklist (DoD)

* [ ] Library SLOs defined and agreed
* [ ] Criterion benches implemented and reproducible locally
* [ ] Perf rigs produce machine-readable outputs and graphs
* [ ] CI regression gates wired to baseline
* [ ] First calibration run captured in **Appendix → History**
* [ ] Perf triage playbook validated on at least one induced regression

---

## 8. Appendix

### Reference SLO Hints (Scaling Blueprint alignment)

* Core orchestrator latency budgets should remain **sub-millisecond p95** within a node.
* Error budgets: event loss (lag) ≤ **0.01%** by design; restart storms bounded by caps.

### Reference workloads

* `perf_bus_fanout`: 1 publisher @ {50k,100k,250k} evt/s to 8 receivers; 30s soak each.
* `perf_restart_storm`: panic child every {250ms, 500ms, 1s}; ensure caps/jitter respected.
* `perf_config_reload`: 10Hz reload for 60s; ensure no leaks, flat latency.

### Tooling commands (copy-paste)

```bash
cargo bench -p ron-kernel
cargo run -p ron-kernel --bin perf_bus_fanout
cargo run -p ron-kernel --bin perf_restart_storm
cargo run -p ron-kernel --bin perf_config_reload
cargo flamegraph -p ron-kernel
```

### History (fill as you measure)

* 2025-MM-DD — Baseline, 4c/8GB, Linux 6.x, Rust 1.80:

  * publish→deliver p99 = ___ µs @ 8 receivers, 128B payload
  * sustained publish = ___ events/s, `bus_lagged_total` ratio = ___%
  * supervisor panic→respawn p95 = ___ ms
  * config swap p95 = ___ ms

```
---
```
