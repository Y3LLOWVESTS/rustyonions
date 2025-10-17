
---

title: Performance & Scaling — svc-edge
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-17
audience: contributors, ops, perf testers

# PERFORMANCE.md — svc-edge

## 0. Purpose

Define the **performance profile** of `svc-edge`:

* SLOs for latency, throughput, and shedding.
* Benchmarks & workloads it must sustain.
* Perf harness & profiling workflow.
* Scaling knobs, known bottlenecks, and a triage/runbook.
* Regression gates and CI wiring to prevent silent drift.

This document ties directly into:

* **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks)
* **Omnigate Build Plan** (Bronze → Silver → Gold)
* **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested)

---

## 1. SLOs / Targets (Service)

> SLOs assume **intra-zone** clients and packs/CAS **local** (micronode) or on fast local storage (macronode). Live fills add upstream variance but stay bounded by retry budgets.

### Latency (GET `/edge/assets/*`)

* **Warm hit (pack/CAS local):** p95 < **40 ms**, p99 < **80 ms**
* **Live fill (miss → fetch):** p95 < **250 ms** (excludes upstream 4xx)
* **Inter-region access via gateway:** p95 < **200 ms**

### Throughput (per instance)

* Sustain **≥ 400 req/s** at target SLOs (measured on 4 vCPU / 8 GiB node)
* Graceful backpressure when load > **500 req/s** (RPS cap) with **< 1%** 429s over 15m

### Error Budget (per 30-day window)

* 5xx < **0.1%**
* 429/503 (shed/timeout) < **1.0%** sustained
* Broadcast bus overflow (`bus_lagged_total`) < **0.01%** of events

### Resource Ceilings (steady state at 400 rps)

* CPU < **60%** total on 4 vCPU
* Memory < **512 MiB** RSS
* FD usage < **40%** of limit
* GC/allocs: < **1 small alloc** per request on hot path (goal)

### Cold Start

* Process start to ready (`/readyz=200`) < **300 ms** with no pack verification
* With pack verify (10k entries): < **2 s**

---

## 2. Benchmarks & Harness

### 2.1 Synthetic Load (HTTP)

Use `bombardier` (or `wrk`) to measure latency distributions and shedding.

```
bombardier -c 128 -d 2m -l -H "X-Corr-ID: test" http://127.0.0.1:8080/edge/assets/tile.pmtiles
bombardier -c 128 -d 2m -l -H "Range: bytes=0-65535" http://127.0.0.1:8080/edge/assets/tile.pmtiles
```

Live fill (allow-listed host; ensure upstream is reachable and cached after first run):

```
bombardier -c 64 -d 2m -l http://127.0.0.1:8080/edge/assets/fonts/remote.woff2
```

### 2.2 Micro-bench (Criterion)

* **Targets:**

  * Range parser & header validators
  * Pack (PMTiles/MBTiles) random access reads
  * BLAKE3 streaming hasher (full-object and segmented)
* Bench command:

```
cargo bench -p svc-edge
```

### 2.3 Profiling

* **CPU flamegraph:**

```
cargo flamegraph -p svc-edge --bin svc-edge -- --bind 127.0.0.1:8080
```

* **Async stalls (tokio-console):**

```
RUSTFLAGS="--cfg tokio_unstable" TOKIO_CONSOLE_BIND=127.0.0.1:6669 cargo run -p svc-edge
```

* **Sys profiling (Linux perf):**

```
perf record -F 99 -g -- ./target/release/svc-edge --bind 127.0.0.1:8080
perf report
```

* **Causal profiling (coz):**

```
coz run -- ./target/release/svc-edge --bind 127.0.0.1:8080
```

### 2.4 Chaos/Perf Mix

* Slow-loris simulation (many connections with trickle reads)
* Compression bomb attempts (guarded by ratio & absolute caps)
* Worker crashes under load (ensure `/readyz` flips & recovers)

Artifacts (must-save to repo under `testing/performance/`):

* Bombardier/wrk JSON outputs
* Flamegraphs (SVG)
* tokio-console traces (if collected)
* Config used (TOML)

---

## 3. Scaling Knobs

|                                     Knob | Where   | Effect                            | Guidance                                                     |
| ---------------------------------------: | ------- | --------------------------------- | ------------------------------------------------------------ |
|                      `ingress.rps_limit` | CONFIG  | Admission control                 | Keep ≤ 500; raise only with headroom & tests                 |
|                   `ingress.max_inflight` | CONFIG  | Concurrency                       | Default 512; reduce if RAM pressure                          |
|        Work queue capacity (`mpsc(512)`) | Code    | Backpressure vs. head-of-line     | Prefer 512; measure busy drops before raising                |
|                     Tokio worker threads | Runtime | Parallelism                       | Default = cores; pin if oversub/NUMA issues                  |
| Hashing path (`spawn_blocking` or async) | Impl    | CPU contention                    | Move to `spawn_blocking` only if flamegraph shows contention |
|                 Chunk size (range reads) | Impl    | Syscalls vs. memory               | Start 64 KiB; tune up to 256 KiB for high-latency disks      |
|                    Multi-range responses | CONFIG  | Response complexity               | Disabled by default; enabling can hurt perf                  |
|   Retry budget (`base,max,count,jitter`) | CONFIG  | Upstream latency vs. success rate | Keep bounded; jitter on to prevent herds                     |

Horizontal scaling is **first choice** (stateless edge). Add replicas behind the gateway/LB once CPU > 70% or shed > 1%.

---

## 4. Bottlenecks & Known Limits

**Must-watch hotspots**

* **BLAKE3 hashing** on cold fills of large assets (CPU-bound). Mitigation: segment hashing, potential `spawn_blocking`.
* **PMTiles/MBTiles random access** with small page size on remote/block storage. Mitigation: OS page cache hints, larger read chunks.
* **TLS handshake** (when terminating locally). Mitigation: keep-alive, gateway termination preferred.
* **HTTP header parsing** for `Range` and ETag validation on extremely chatty clients. Mitigation: fast-path prechecks, minimal allocs.
* **Queue pressure**: full `mpsc(512)` leads to 429 spikes. Mitigation: tune RPS limit first; scale out.

**Acceptable trade-offs (Bronze)**

* First live fill request can pay hashing cost; subsequent hits are fast.

**Gold targets**

* Zero extra alloc on hot path (serve from pack/CAS) beyond response buffer; steady CPU < 50% at 400 rps.

---

## 5. Regression Gates (CI)

Fail CI when any of the following hold vs. stored baseline (± tolerance):

* p95 latency ↑ **> 10%**
* Throughput (req/s) ↓ **> 10%**
* CPU or memory ↑ **> 15%**
* 429/503 shed rate ↑ **> 0.5 pp** absolute

**Baselines** saved under `testing/performance/baselines/{profile}.json` (include node shape, commit SHA, config).

Optional waiver mechanism:

* A PR label `perf-waiver` requires explicit reviewer ack and a **runbook note** explaining why (e.g., upstream crate change with planned fix).

---

## 6. Perf Runbook (Triage)

1. **Confirm the symptom**

   * Dashboards: p95/p99, `rejected_total{reason}`, CPU/Mem, queue depth
   * Compare to recent baseline (same node type/config)

2. **Identify bottleneck**

   * Run `cargo flamegraph` under similar load; look for hashing, pack I/O, decompression, TLS
   * `tokio-console`: check for long tasks / blocked resources

3. **Apply fast knobs**

   * Drop `ingress.rps_limit` or add replicas (scale out)
   * Increase chunk size to reduce syscalls (64 KiB → 128/256 KiB)
   * Enable `spawn_blocking` for hashing if CPU-bound on runtime threads

4. **Validate backpressure**

   * Inspect `busy_rejections_total` and `edge_queue_depth`
   * Ensure 429 rates fall back < 1% within 5–10 minutes post-change

5. **Upstream issues (live fill)**

   * Rising `io_timeouts_total{op="fill"}` + retries? Reduce max retries or cap concurrency for fills; verify allow-listed host status

6. **Record & codify**

   * Update `testing/performance/baselines/` with new numbers and machine profile
   * Add notes to `RUNBOOK.md` and `CHANGELOG.md` if behavior changed

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs documented (latency/throughput/error budget) and reviewed.
* [ ] Load harness runs locally and in CI (nightly).
* [ ] At least one flamegraph and one tokio-console capture checked into artifacts.
* [ ] Scaling knobs verified (RPS, inflight, queue capacity) and documented.
* [ ] Regression gates wired and baselines stored.
* [ ] Runbook updated with any recent triage outcomes.

---

## 8. Appendix

### 8.1 Reference SLOs (Scaling Blueprint)

* p95 GET < **80 ms** intra-region; < **200 ms** inter-region
* Failures < **0.1%**; RF observed ≥ RF target

### 8.2 Reference Workloads

* `gwsmoke` GET/HEAD/RANGE mixes
* 24h soak on common asset sizes (4 KiB, 64 KiB, 1 MiB, 8 MiB)
* Burst tests: 10× RPS for 60 s (observe shed behavior and recovery)

### 8.3 CI Hook (sketch)

```yaml
name: perf-regression-guard
on:
  schedule: [{cron: "0 6 * * *"}]
  workflow_dispatch:
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build -p svc-edge --release
      - name: Start svc-edge
        run: ./target/release/svc-edge --bind 127.0.0.1:8080 --metrics 127.0.0.1:9090 --mode offline --pack ./testing/data/tile.pmtiles
      - name: Warmup
        run: bombardier -c 64 -d 30s http://127.0.0.1:8080/edge/assets/tile.pmtiles
      - name: Measure
        run: bombardier -c 128 -d 90s -o json -l http://127.0.0.1:8080/edge/assets/tile.pmtiles > perf.json
      - name: Compare to baseline
        run: cargo run -p perf-compare -- perf.json testing/performance/baselines/edge_offline.json --fail-on 0.10
```

### 8.4 History

* 2025-10-17: Initial SLOs set (warm hit p95<40 ms; 400 rps per node target; shed<1%).
* Subsequent entries: record regressions, fixes, and configuration adjustments.

---

✅ With this **Performance & Scaling** spec, `svc-edge` has clear, testable targets; a repeatable harness; levers for tuning; and CI guardrails that keep us **fast, predictable, and drama-free**.
