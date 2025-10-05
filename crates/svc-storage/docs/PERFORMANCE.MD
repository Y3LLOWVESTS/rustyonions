

---

````markdown
---
title: Performance & Scaling — svc-storage
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-04
audience: contributors, ops, perf testers
---

# PERFORMANCE.md — svc-storage

## 0) Purpose

Defines the **performance profile** of `svc-storage`:
- Service SLOs (latency/throughput/error budgets) for reads/writes.
- Benchmarks & load rigs to sustain those targets.
- Profiling stack and chaos/perf drills.
- Scaling knobs, bottlenecks, and triage steps.
- CI regression gates to prevent silent drift.

**Canon ties:** Scaling Blueprint, Hardening & Concurrency blueprints, INTEROP (OAP/1: 1 MiB frame), IDB (BLAKE3 addressing, ≈64 KiB chunking), OBSERVABILITY metrics, SECURITY (decompression caps).

> **Facet:** Media — Range start p95 explicitly tracked.

---

## 1) SLOs / Targets

### 1.1 Read Path (GET/HEAD/Range)
- **Latency (intra-region):** p95 **< 80 ms** (range start p95 **< 100 ms**).
- **Latency (inter-region):** p95 **< 200 ms** (hedged reads allowed per policy).
- **Correctness:** Strong `ETag: "b3:<hex>"`, correct 200/206/304/416 semantics.

### 1.2 Write Path (PUT/POST)
- **Latency:** p95 **< 250 ms** for 1 MiB objects (digest-on-write).
- **Caps:** Body cap **1 MiB**; decompression **≤ 10×** **and** absolute output cap; 413 on exceed.
- **Idempotency:** PUT by address; verify BLAKE3 digest before success.

### 1.3 Throughput & Backpressure
- **Sustain:** ≥ **500 req/s per node** on 4c / 8 GiB reference.
- **Shedding:** Graceful via `max_inflight` + `max_rps` (Tower caps), structured 429/503.

### 1.4 Error Budget & Saturation
- **5xx < 0.1%**, **429/503 < 1%**, **bus_overflow < 0.01%** at target load.
- **Readiness:** `/readyz` fails **writes first** under pressure; turns green only after placement/cache/pacer ready.

### 1.5 Durability/Repair (Perf-adjacent)
- **RF gauges:** `rf_target`, `rf_observed` published.
- **Repair pacing:** ≤ **50 MiB/s** per repair worker to avoid cache thrash.

---

## 2) Benchmarks & Harness

### 2.1 Micro-bench (Criterion)
Target hot paths:
- BLAKE3 digest (1 MiB object).
- Chunked range writer (≈ 64 KiB buckets).
- Safe decompression (ratio guard ON).

Run:
```bash
cargo bench -p svc-storage
````

### 2.2 Integration Load (wrk / bombardier / gwsmoke)

Rigs live in `testing/performance/`. Core scenarios:

* **Read:** cold→warm GET `/o/{b3}`, HEAD, Range (0–64 KiB & mid-file).
* **Write:** PUT/POST 1 MiB with digest-on-write.
* **Abuse:** over-cap body, compression bombs (expect 413), slow-loris.

**Quick repro (example, adjust port/path):**

```bash
bombardier -c 400 -d 30s -l http://127.0.0.1:8080/o/b3:...             # GET mix
wrk -t12 -c400 -d30s --latency http://127.0.0.1:8080/o/b3:...           # latency dist
curl -sS -X PUT --data-binary @1MiB.bin http://127.0.0.1:8080/put       # write path
```

### 2.3 Profiling Stack

* **CPU:** `cargo flamegraph` for hash/copy/compression hotspots.
* **Async:** `tokio-console` for stalls and queue starvation.
* **Causal:** `coz` / `perf` for “what-if” analysis.
* **Heap:** `heaptrack` / `dhat-rs` on selected endpoints.

### 2.4 Chaos + Perf Blend

* **Latency injection**, **slow-loris**, **zip-bomb** — service must degrade predictably with fast rejects (no collapse).

---

## 3) Scaling Knobs

* **Concurrency:** Tokio task budgets; `max_inflight`, `max_rps` (Tower caps) configurable.
* **Chunking & Buffers:** storage **streaming chunk ≈ 64 KiB**; tuned read/write buffers (default 256 KiB).
* **I/O semantics:** streaming + zero-copy (`bytes::Bytes`) on hot paths.
* **Horizontal:** add replicas; enable **hedged reads** (e.g., 30 ms) for tail latency.
* **Vertical:** allocate CPU for hashing/decompression; watch FD ceilings (`max_conns`/ulimits).
* **Amnesia mode (Micronode):** RAM-only; refuse spill and degrade readiness rather than disk spill.

---

## 4) Bottlenecks & Known Limits

* **CPU:** hashing and (de)compression dominate for 1 MiB bodies.
* **Disk I/O:** cold Range seeks; mitigate with cache warmup + hedged reads.
* **Safety caps:** ratio/output caps intentional; keep 413 path low-overhead.
* **Repairs:** repair storms contend with reads; rely on pacing + observe `rf_*` gauges.

---

## 5) Golden Metrics & Dashboards

Export at minimum:

* `requests_total{route,code}`, `bytes_{in,out}_total`, `latency_seconds{route}`,
* `inflight`, `rejected_total{reason}`, `integrity_fail_total{reason}`,
* `rf_target`, `rf_observed`, readiness and repair pacing panels.

Dashboards must show: p50/p95/p99 latency split by route; error budget burn; shed rate; cache hit ratio; repair bandwidth.

---

## 6) Regression Gates (CI must fail if…)

Compare to baselines in `testing/performance/baselines/`:

* **p95 latency** ↑ **> 10%** (read or write) at same payload mix.
* **Throughput** ↓ **> 10%** (1 MiB writes, mixed range).
* **CPU or memory** ↑ **> 15%** at target RPS (exclude cold start).
* **Error budget breach:** > 0.1% 5xx or > 1% 429/503 at target load.

**MSRV & runner matrix (nightly perf workflow):**

* Rust **1.80.0** and **stable** latest.
* Runtimes: `tokio` multithread + current-thread.
* Arch: x86_64 (required), aarch64 (best-effort).

**Loom/TLA+:**

* Loom tests cover pacer/semaphore interleavings (no deadlocks, no await-holding-locks).
* (Optional) TLA+ model for readiness DAG (pacer → cache → writes) validated in docs/.

---

## 7) Perf Runbook (Triage)

1. **Flamegraph:** confirm hotspots; check buffer sizes and ≈ 64 KiB chunking.
2. **tokio-console:** find task stalls; validate `max_inflight`/`max_rps` limits are active.
3. **Metrics sweep:** `rejected_total{reason}`, `integrity_fail_total`, RF gauges, shed rate.
4. **Knob sweep:** raise/lower semaphores; adjust buffers; enable hedged reads (e.g., 30 ms).
5. **Chaos toggle:** run zip-bomb + slow-loris; confirm fast 413/timeouts without collapse.
6. **Amnesia checks:** verify “refuse write / readiness degrade” behavior (Micronode).
7. **Exit:** p95s within budget ≥ 30 min; error budgets green; diff graphs attached to CI artifact.

---

## 8) Acceptance Checklist (Definition of Done)

* [ ] SLOs pinned (intra/inter, read/write) **and** on dashboards.
* [ ] Criterion + load rigs runnable locally and in CI (nightly).
* [ ] Flamegraph/console traces captured at least once per release train.
* [ ] Scaling knobs documented (`max_rps`, `max_inflight`, buffers, chunk size).
* [ ] Regression gates wired with baselines.
* [ ] Readiness drill gates on placement cache + durability pacer.
* [ ] Repair pacing + RF gauges validated under induced replica loss.
* [ ] **Loom** suite passes pacer/semaphore interleavings.

---

## 9) Reference Workloads (DX quickstart)

**Read mix A (default):** 70% GET (50% range 0–64 KiB, 20% mid-file), 20% HEAD, 10% cold GET
**Write mix B:** PUT 1 MiB at 10% total RPS; include compressed/uncompressed parity and ratio-cap tests
**Soak:** 24 h read-heavy with 1–2% induced repair; verify pacing and tail latency

**Commands (examples):**

```bash
# Range-heavy GETs
wrk -t8 -c256 -d2m --latency http://127.0.0.1:8080/o/b3:ABC... \
  --script=testing/performance/scripts/range.lua --latency

# Mixed GET/HEAD with warmup
bombardier -c 300 -d 120s -l http://127.0.0.1:8080/o/b3:ABC...

# Writes @ 1 MiB
dd if=/dev/urandom of=1MiB.bin bs=1M count=1
curl -sS -X PUT --data-binary @1MiB.bin http://127.0.0.1:8080/put

# Abuse: zip-bomb (expect 413); path and payload provided in repo fixtures
curl -sS -X POST --data-binary @testing/performance/fixtures/zip_bomb.bin \
  http://127.0.0.1:8080/put
```

---

## 10) Appendix (Interop, SemVer, PQ hooks)

**Interop / SDK parity**

* SDKs (Rust/TS) must honor protocol caps: **1 MiB** max body, decompression **≤ 10×**, and prefer streaming I/O with ≈ 64 KiB reads.
* Round-trip tests live in `testing/performance/sdk_roundtrips/`.

**SemVer / CHANGELOG rules**

* Metrics names/labels are part of the public surface: **renames are breaking** and require a documented migration in CHANGELOG plus a compatibility window (dual-emit when feasible).

**Post-Quantum note**

* Storage addressing is BLAKE3 (hash-based; not impacted by Shor). Credential/cap verification remains classical; QUANTUM.md tracks optional PQ-hybrid KMS for capabilities. No perf impact today; add PQ perf track when enabled.

**CI wiring snippet (illustrative)**

```yaml
# .github/workflows/perf.yml (excerpt)
jobs:
  perf:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        toolchain: [1.80.0, stable]
        runtime: [multithread, current]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with: { toolchain: ${{ matrix.toolchain }} }
      - run: cargo build -p svc-storage --release
      - run: testing/performance/run_load.sh --runtime ${{ matrix.runtime }}
      - run: testing/performance/compare_baselines.sh
```

```

---


