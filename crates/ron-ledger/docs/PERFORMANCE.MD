

---

# ⚡ PERFORMANCE.md — Template

````markdown
---
title: Performance & Scaling Template — ron-ledger
status: reviewed
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-13
audience: contributors, ops, perf testers
owners: [Stevan White]
links:
  - IDB: ./IDB.md
  - CONCURRENCY: ./CONCURRENCY.md
  - SECURITY: ./SECURITY.md
  - OBSERVABILITY: ./OBSERVABILITY.md
  - INTEROP: ./INTEROP.md
  - CONFIG: ./CONFIG.md
---

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of `ron-ledger`:
- Service-level objectives (SLOs) with concrete load profiles.
- Benchmarks & workloads it must sustain (Micronode vs. Macronode).
- Perf harness & profiling tools and how to run them.
- Scaling knobs, expected bottlenecks, and triage steps.
- Regression gates to prevent silent perf drift in CI.

It ties directly into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze → Gold.
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).

---

## 1. SLOs / Targets

`ron-ledger` is primarily a **service** (single committer writer, deterministic batching). Targets below are **per node** and validated in CI perf runs.

### 1.1 Latency (commit path)
- **p95 commit latency** (`ledger_commit_latency_seconds`): ≤ **80 ms**
- **p99 commit latency**: ≤ **150 ms**
- **Root publication lag** (commit → `/roots` available): ≤ **1 s**

### 1.2 Read endpoints
- **/roots intra-region** p95: ≤ **20 ms**
- **/roots inter-region** p95: ≤ **120 ms** (typical WAN RTT bound)

### 1.3 Throughput (ingest)
Two reference profiles (from IDB §10):

- **Micronode (Amnesia)**: **≥ 300 RPS** sustained, median batch = 8, p90 batch = 32  
- **Macronode (Persistent/RocksDB)**: **≥ 1200 RPS** sustained, median batch = 16, p90 batch = 64  
- **Burst tolerance**: **10× surge** drains within **≤ 5 s** without loss/dup

### 1.4 Error Budget & Saturation
- **Application failures** (non-429/503): < **0.1%** of requests over 28d
- **Backpressure responses** (429/503): < **1.0%** at target load
- **Seq gap incidents**: **0** (any delta consumes full budget until resolved)
- **Queue headroom**: `queue_depth{q="commit"} < 0.8 * capacity` for **≥ 99%** of minutes

### 1.5 Resource ceilings (at target load)
- **CPU**: < **70%** of a single core sustained on the Committer thread; < **75%** node-wide
- **Memory**: < **2.0 GiB** RSS (Macronode), < **600 MiB** RSS (Micronode)
- **FD usage**: < **30%** of system limit
- **Disk** (Macronode): WAL fsync p99 ≤ **3 ms**; compaction stalls < **0.5%** wall time

*(Edge/mobile applies only to Micronode builds; cold-start target ≤ **200 ms** process start → ready.)*

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)
Hot paths:
- Accumulator `update(prev_root, entry)` (Merkle/BLAKE3 by default)
- Binary encoding/decoding (`postcard`/`bincode`) of `LedgerEntry`
- Reject taxonomy mapping (parsing/serialization)
- Batch commit path (in-mem sink)

Run:
```bash
cargo bench -p ron-ledger --bench micro
````

Outputs live under `target/criterion/…` and are archived by CI.

### 2.2 Integration load tests

Targets the **real server** over HTTP/UDS with production knobs.

Tools:

* `bombardier` for steady RPS and latency histograms
* `wrk` for scripted pipelines (Lua)
* `k6` for scenario orchestration and soak
* `vegeta` for precise replay of **canonical vectors** (INTEROP §4)

Examples:

```bash
# Macronode steady ingest (1200 RPS, batch size ~16)
bombardier -c 128 -d 5m -l -m POST -H "Authorization: Bearer $MACAROON" \
  -f testing/performance/payloads/ingest_batch16.json https://localhost:5082/ingest

# Read roots (intra-region)
bombardier -c 64 -d 3m -l https://localhost:5082/roots?since=0

# Soak (24h) with k6
k6 run testing/performance/k6/soak_ingest_roots.js
```

### 2.3 Profiling & diagnostics

* **Flamegraph** (CPU): `cargo flamegraph -p ron-ledger --bin ron-ledgerd`
* **tokio-console**: enable `RUSTFLAGS="--cfg tokio_unstable"` in dev; connect console during load
* **perf** (Linux): `perf record -F 199 -g -- ./target/release/ron-ledgerd …`
* **coz** causal profiling (optional)
* **jemalloc/mi-malloc** compare runs (see 3.4)

### 2.4 Chaos/perf blend

* **Slow-loris** on ingress
* **Disk-pressure** (fill FS until threshold) to validate ENOSPC fail-closed
* **Kill Committer** mid-fsync (safe-mode + monotonic recovery)
* **TLS handshake storm** (session resumption on/off)

### 2.5 CI integration

Nightly perf workflow:

1. Boot service (Micronode + Macronode matrices)
2. Replay canonical vectors (INTEROP §4)
3. Run `bombardier` suites (5m each) and capture `/metrics`
4. Compare against **baselines** (see §5)

---

## 3. Scaling Knobs

### 3.1 Concurrency & Batching

* **Single Committer** (invariant): scale by upstream **batch size** and **prevalidator N**
* `concurrency.prevalidators = max(2, num_cpus/2)` (CONFIG)
* `queues.*` capacities (ingress=2000, prevalidator=2000, commit=1000 default)
* `commit.batch_max_entries` (CONFIG; tune 16–128 target)

### 3.2 Memory & Buffers

* Use `bytes::Bytes` for zero-copy IO
* In-mem segment buffer sizes (Micronode): 256–1024 KiB
* JSON decode buffer cap (to avoid growth past `limits.max_body_bytes`)

### 3.3 I/O

* **Streaming** request decoding for large batches; validate sizes early
* TLS session resumption and TCP keepalive enabled
* UDS path for in-host low-latency ingest (PEERCRED + Bearer)

### 3.4 Allocators & Codegen

* Compare `jemalloc` vs system malloc:

  * `MALLOC_CONF=background_thread:true,dirty_decay_ms:250,muzzy_decay_ms:250`
* Codegen:

  * `RUSTFLAGS="-C target-cpu=native -C lto=thin -C codegen-units=1"`
  * `CARGO_PROFILE_RELEASE_LTO=true`

### 3.5 Storage (Macronode)

RocksDB tuning (starting points):

* `write_buffer_size = 64MiB`
* `max_write_buffer_number = 4`
* `target_file_size_base = 64MiB`
* `max_background_jobs = min(4, num_cpus)`
* Rate limiter: 200MB/s (burst 400MB/s) to avoid noisy neighbor
* Disable WAL sync batching only if latency SLO holds (default: fsync per batch)

---

## 4. Bottlenecks & Known Limits

Current hotspots (watch in flamegraphs / console):

* **Accumulator hashing** (Merkle/SHA-256). Switch to **BLAKE3** feature if CPU-bound and compliance allows.
* **Serialization**: JSON parsing on ingress for very large batches; prefer UDS + binary gateway if needed (future).
* **RocksDB compactions**: Mis-tuned jobs stall commit; monitor `rocksdb_compaction_seconds_*`.
* **fsync**: p99 spikes under burst; keep batch sizes modest if FS jitter high.
* **TLS handshakes**: Ensure resumption; offload via front proxy if needed.

Classification:

* **Must-fix for Gold**: commit p95 > 80ms, compaction stalls, seq gap any delta
* **Acceptable for Bronze**: occasional TLS p99 spikes during rolling restart

---

## 5. Regression Gates

CI fails the perf job if any threshold breached vs. **baselines**:

* **Latency**: p95 commit ↑ > **10%** OR p99 ↑ > **10%**
* **Throughput**: sustained RPS ↓ > **10%**
* **CPU/Mem**: RSS ↑ > **15%** or CPU ↑ > **15%**
* **Backpressure**: `429/503` fraction > **1%** at target load
* **Seq gaps**: any non-zero delta

Baselines stored as JSON:
`testing/performance/baselines/{micronode,macronode}/baseline.json`

Format:

```json
{
  "commit_p95_ms": 62.0,
  "commit_p99_ms": 118.0,
  "ingest_rps_sustained": 1250,
  "rss_mib": 980,
  "cpu_pct": 64,
  "backpressure_frac": 0.004
}
```

Allow **waivers** only if:

* Change is intentional & justified (e.g., stronger crypto),
* Owner + perf reviewer approve,
* Baseline is updated with rationale in `History`.

---

## 6. Perf Runbook (Triage)

1. **Confirm symptoms**
   Check Grafana: Commit latency p95/p99, queue depth (commit), 429 rate, safe-mode flag.

2. **Hotspots**

   * Run `cargo flamegraph` during load; look for hash, serde, WAL fsync, RocksDB compaction.
   * Open **tokio-console**; identify long-lived tasks or blocked resources.

3. **Knobs to try (one at a time)**

   * Reduce `commit.batch_max_entries` (e.g., 64 → 32) if fsync spikes.
   * Increase `concurrency.prevalidators` by +2 if ingress bottleneck.
   * Bump `queues.ingress` by +25% to smooth micro-bursts (watch memory).
   * Enable **BLAKE3** accumulator feature if CPU-bound and allowed.
   * Tune RocksDB: raise `max_background_jobs`, lower `write_buffer_size` to reduce stalls.

4. **I/O sanity**

   * Ensure TLS session resumption; test UDS path.
   * Validate `limits.max_body_bytes` isn’t clipping legit requests.

5. **Chaos toggles**

   * Reproduce with slow-loris off; disable compression (if any frontend) to isolate CPU.
   * Retry with clean FS (no compaction backlog) to attribute disk vs CPU.

6. **When to page**

   * `ledger_safe_mode == 1`
   * `increase(ledger_seq_gap_detected_total[5m]) > 0`
   * p99 commit > 150ms for >5m

7. **Document**
   Append findings to **History** with flamegraphs and console screenshots; update baselines if improved.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined (commit p95/p99, throughput, error budget, resource ceilings).
* [ ] Bench harness runs locally **and** in CI (Micronode + Macronode).
* [ ] Flamegraph and tokio-console traces collected at least once per release.
* [ ] Scaling knobs & defaults documented (queues, prevalidators, batch sizes, RocksDB).
* [ ] Regression gates wired; baselines committed; promtool/grafana jobs green.
* [ ] Perf runbook validated; on-call can follow it end-to-end.

---

## 8. Appendix

### 8.1 Reference SLOs (Scaling Blueprint)

* p95 commit ≤ **80 ms** intra-region; inter-region reads ≤ **120 ms**.
* Failures < **0.1%**; backpressure < **1%**.

### 8.2 Reference workloads

* **Canonical vectors** (INTEROP §4): happy path, idempotent replay, unknown-field reject, busy, reversible, malformed nonce, mTLS fail.
* **Soak** (24h): Mixed ingest/roots at 60% of target RPS.

### 8.3 Tooling quick commands

```bash
# Build release with native optimizations
RUSTFLAGS="-C target-cpu=native -C lto=thin -C codegen-units=1" cargo build -p ron-ledger --release

# Start Micronode profile (Amnesia)
RON_LEDGER_PROFILE=micronode ./target/release/ron-ledgerd --config configs/micronode.toml

# Start Macronode profile (RocksDB)
RON_LEDGER_PROFILE=macronode ./target/release/ron-ledgerd --config configs/macronode.toml

# Run bombardier ingest test (edit MACAROON + URL)
bombardier -c 128 -d 5m -l -m POST -H "Authorization: Bearer $MACAROON" \
  -f testing/performance/payloads/ingest_batch16.json https://localhost:5082/ingest

# Flamegraph around commit path
cargo flamegraph -p ron-ledger --bin ron-ledgerd
```

### 8.4 Perfection Gates

* **Gate F**: CI perf job blocks merges on regression thresholds (§5).
* **Gate L**: Chaos + perf blend passes (kill mid-fsync, disk-pressure, slow-loris) with SLOs intact.

### 8.5 History (summarize lessons/regressions)

* 2025-10-13: Initial performance profile established; Macronode ≥1200 RPS; p95 commit 62ms; RocksDB background jobs = 4.

```
```
