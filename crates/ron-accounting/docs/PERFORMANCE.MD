

````markdown
---
title: Performance & Scaling — ron-accounting
status: draft
msrv: 1.80.0
crate_type: lib (adapter optional via features)
last-updated: 2025-10-15
audience: contributors, ops, perf testers
---

# ⚡ PERFORMANCE.md — ron-accounting

## 0. Purpose

Define the **performance profile** and guardrails for `ron-accounting`, a transient metering **library** with an optional HTTP/UDS adapter:

- SLOs (latency/throughput/memory) by **profile** (Micronode/Edge vs Macronode/Server).
- Benchmarks & production-like workloads it must sustain.
- Harness & profilers (Criterion, flamegraph, tokio-console, perf/coz).
- Scaling knobs, known bottlenecks, and triage steps.
- **Regression gates** to block silent perf drift (Gate F) and validate scaling/chaos (Gate L).

It ties directly into:
- **Scaling Blueprint v1.3.1**
- **Omnigate Build Plan** (Bronze → Gold)
- **IDB** (bounded queues, ordered/idempotent exports)
- **CONCURRENCY** (no await-locks, backpressure)
- **OBSERVABILITY** (export latency, queue depth, degraded)

---

## 1. SLOs / Targets

> SLOs are expressed per **profile**. “Amortized” means averaged over 5 minutes steady state.

### 1.1 Library Core (hot path + sealing + export)

| Metric | Micronode (amnesia=true) | Macronode (WAL on) | Notes |
|---|---:|---:|---|
| **record_row()** latency (amortized) | ≤ **90 ns/op** | ≤ **110 ns/op** | single-threaded, cache-hot, 64-bit inc |
| **seal_slice()** (1–4k rows) | ≤ **350 µs** | ≤ **450 µs** | includes digest + canonical encode |
| **export p99** (enqueue→ACK/dup) | ≤ **600 ms** | ≤ **1.0 s** | ledger intra-region; 10s hard deadline |
| **ordering_wait p99** | ≤ **50 ms** | ≤ **80 ms** | head-of-line hold for N−1 |
| **dup rate** | ≤ **0.5%** | ≤ **0.5%** | idempotence observed, not failure |
| **CPU @ 50k rows/s** | ≤ **35%** of 1 core | ≤ **45%** | x86_64, AVX2 available |
| **RSS steady** | ≤ **96 MiB** | ≤ **160 MiB** | includes shard maps + small queues |

### 1.2 Adapter (feature `http-adapter`) — if enabled

| Metric | Target |
|---|---:|
| **/export** p95 (1 MiB cap) | ≤ **12 ms** (local GW); ≤ **25 ms** (intra-AZ) |
| **Sustained ingress** | ≥ **2k req/s** per node (small payloads) |
| **429/503** | < **1%** at target load (backpressure visible) |
| **/metrics and /readyz** | < **2 ms** handler time |

> **Error budget** (core + adapter): 5xx/“fail” statuses ≤ **0.1%**; rejects (capacity/order_overflow) ≤ **1%** and **observable**.

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

- `bench_record_row`: tight loop of `recorder.record(key, inc)`, randomized keys (80/20 hot/cold).
- `bench_seal_slice`: 1k / 2k / 4k rows, digest + canonical encode (DAG-CBOR).
- `bench_idempotent_put`: mock exporter returning `Ack::Ok`/`Ack::Duplicate` with configurable latency.

Run:

```bash
cargo bench -p ron-accounting
````

### 2.2 Integration / Soak

* `testing/performance/ingress_export.rs`: spins N producer tasks → queue → exporter (mock or real ledger sandbox).
* Workload profiles:

  * **W1: Hot Key** — 1 tenant, 1 dimension, 1k keys (Zipf α=1.2), 50k rows/s.
  * **W2: Wide Tenancy** — 1k tenants × 3 dims, 100 keys each, 100k rows/s total.
  * **W3: Edge Bursts** — bursts 200k rows over 1s every 10s; window len 60s.
  * **W4: WAL Stress** — WAL enabled, cap 512 MiB, fsync jitter 0–5 ms.

Run (examples):

```bash
cargo run -p ron-accounting-perf -- w1 --rate 50000 --mins 10
cargo run -p ron-accounting-perf -- w4 --wal --cap-mb 512 --mins 30
```

### 2.3 Profiling Stack

* **CPU flamegraph**: `cargo flamegraph -p ron-accounting --bench bench_seal_slice`
* **Async stalls**: `RUSTFLAGS="--cfg tokio_unstable" RUST_LOG=debug tokio-console`
* **Linux perf**: `perf stat -d -d -d <cmd>` and `perf record -g <cmd>`
* **Causal profiling** (optional): `coz run --- <bench>` to locate “what if faster?” regions

### 2.4 Chaos/Perf Blend

* Inject exporter 5xx at 10–30%, 200–800 ms tail.
* Slow fsync (WAL) with random 2–8 ms sleeps (spawn_blocking).
* Enforce DTO oversize/ratio to measure fast-reject path.

---

## 3. Scaling Knobs

* **Sharding**: `shards` (power of two) for hot maps; scale ~linearly to #cores up to L3 limits.
* **Lane capacity**: per-stream ordered buffer (e.g., 64–1024); higher reduces `ordering_wait` at memory cost.
* **Pending queue cap**: total sealed pending; tune to match exporter throughput (avoid frequent degraded flip).
* **Backoff**: base=50–100 ms; cap=2 s; global op deadline=10 s (export). Lower backoff boosts throughput at risk of thundering herd.
* **WAL caps** (Macronode): `max_bytes`, `max_entries`, fsync policy (on seal vs interval).
* **Encoding**: DAG-CBOR vs MsgPack; CBOR is default & fastest in our stack (avoid unnecessary allocations).
* **Concurrency**: #workers for export == min(active streams, cores); never exceed N=cores×2.
* **IO**: zero-copy `bytes::Bytes` in hot paths; pre-allocated row buffers during seal.

---

## 4. Bottlenecks & Known Limits

* **Canonical encoding**: map ordering + varint work dominates short slices (<512 rows). Must keep slice size ≥1k rows for best amortization.
* **BLAKE3 hashing**: mostly memory-bound; aligns well with AVX2/NEON; watch alignment on ARM.
* **WAL fsync**: latency spikes can hold the sealing path if configured sync-on-seal; prefer batched fsync or fsync-dir only with durable rename.
* **Ordered lanes**: high spread (many concurrent streams) + small caps → `order_overflow` rejects. This is **by design**; increase lane cap only with memory budget.
* **Adapter TLS handshake**: if enabled, initial spikes can add ~1–2 ms; reuse connections from gateway.

> **Bronze** target: pass W1/W2 @ stated rates.
> **Silver**: pass W3 bursts without degraded >30s.
> **Gold**: pass W4 (WAL stress) with p99 exporter ≤1s and no data loss (dup ok).

---

## 5. Regression Gates (CI)

* **Fail** PR if any of the following vs baseline (last green on main):

  * `bench_record_row` time/op ↑ **>10%**
  * `bench_seal_slice` time/op ↑ **>10%**
  * `export_latency_seconds` p95 (integration rig) ↑ **>10%**
  * CPU per op ↑ **>15%** or RSS steady ↑ **>20%**
* Baselines stored under `testing/performance/baselines/{host}-{cpu}/{commit}.json`.
* Waiver process: PR must include reason (e.g., upstream crate bump) + updated baselines + risk note.

Example CI steps (sketch):

```yaml
- name: Bench
  run: cargo bench -p ron-accounting -- --save-baseline pr

- name: Compare Criterion
  run: cargo bench -p ron-accounting -- --bench compare --baseline main || echo "::error::Perf regression vs main"

- name: Integration Perf
  run: cargo run -p ron-accounting-perf -- w1 --rate 50000 --mins 5 --json > perf.json

- name: Gate
  run: python scripts/perf_gate.py perf.json testing/performance/baselines/main.json
```

---

## 6. Perf Runbook (Triage)

1. **Is readiness flipping?** Check `accounting_degraded`, `/readyz.missing`. If yes, increase lane/pending caps **temporarily** or reduce ingress.
2. **Which counter is hot?**

   * `export_latency_seconds` p99 high → check ledger 5xx vs network; examine `backoff_retries_total{op}`.
   * `ordering_wait_seconds` p99 high → increase lane cap modestly (×2), consider WFQ weights for noisy tenants.
3. **CPU hot spot?**

   * Run `cargo flamegraph`: if encode dominates, try larger slice (2–4k rows). If hashing dominates, verify BLAKE3 is properly compiled w/ SIMD.
4. **Async stalls?**

   * `tokio-console`: look for long polls; ensure no lock across `.await`. Replace blocking I/O with `spawn_blocking`.
5. **WAL contention?**

   * Measure `accounting_wal_fsync_seconds`; if tail heavy, move to batched fsync or lower fsync frequency (within durability budget).
6. **Memory pressure?**

   * Inspect `queue_depth` & RSS. Large tenant spread? Reduce lane caps; enforce sampling. Consider compacting maps during low load.
7. **Adapter path only:**

   * TLS handshake spikes? Enable keep-alive from ingress GW; raise server accept backlog modestly.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs validated for both profiles (Micronode/Macronode).
* [ ] Criterion micro-benchmarks land + run green locally and CI.
* [ ] Integration perf rig (W1–W4) runs and exports Prom metrics.
* [ ] Flamegraph + tokio-console traces captured and archived once per release.
* [ ] Scaling knobs reviewed with ops defaults in `docs/CONFIG.md`.
* [ ] Regression gates active; baselines updated on release.
* [ ] Gold milestone achieved (see §4 definition) or tracked in Build Plan.

---

## 8. Appendix

### 8.1 Commands (no comments, copy/paste)

```bash
cargo bench -p ron-accounting
cargo flamegraph -p ron-accounting --bench bench_seal_slice
RUSTFLAGS="--cfg tokio_unstable" RUST_LOG=debug cargo run -p ron-accounting-perf -- w2 --rate 100000 --mins 10
perf stat -d -d -d cargo run -p ron-accounting-perf -- w3 --mins 15
```

### 8.2 Criterion bench skeletons (copy-paste)

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};
use ron_accounting::accounting::{Recorder, Row, AccountKey, Dimension};

fn bench_record_row(c: &mut Criterion) {
    let rec = Recorder::default();
    let key = AccountKey { ns: 1, id: 0xAA };
    c.bench_function("record_row_hot", |b| {
        b.iter(|| rec.record(black_box(key), black_box(1u64)));
    });
}

fn bench_seal_slice(c: &mut Criterion) {
    let rec = Recorder::default();
    for _ in 0..2048 { rec.record(AccountKey{ns:1,id:42}, 1); }
    c.bench_function("seal_slice_2k", |b| b.iter(|| rec.seal_now()));
}

criterion_group!(benches, bench_record_row, bench_seal_slice);
criterion_main!(benches);
```

### 8.3 Reference Workloads (json config idea)

```json
{ "workload": "W2", "rate": 100000, "tenants": 1000, "dims": ["bytes","requests","cpu"], "mins": 10 }
```

### 8.4 Perfection Gates tie-in

* **Gate F:** CI perf gates enforce ≤10% latency/throughput drift; block merge on regressions.
* **Gate L:** W3/W4 chaos+scaling validated each release; attach flamegraphs and perf artifacts.

### 8.5 History

* 2025-10-15: Initial SLOs established; W1/W2 rigs land; regression gates wired.

```
```
