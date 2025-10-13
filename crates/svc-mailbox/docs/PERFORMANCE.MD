````markdown
---
title: Performance & Scaling — svc-mailbox
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-12
audience: contributors, ops, perf testers
---

# ⚡ PERFORMANCE.md — svc-mailbox

## 0. Purpose

This document defines the **performance profile** of `svc-mailbox`:

- SLOs (latency/throughput/error budget) and capacity formulas.
- Benchmarks & repeatable workloads it **must** sustain.
- Perf harness & profiling tools (how to run, what to look for).
- Scaling knobs, expected bottlenecks, and triage steps.
- CI regression gates to prevent silent perf drift.

Tied to:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones (Bronze→Gold).
- **Perfection Gates**: F (no perf regressions), L (scaling chaos-tested).

---

## 1. SLOs / Targets

> Scope: single node, intra-region; 8 vCPU / 16 GiB RAM baseline unless specified. All SLOs exclude downstream fanout (that work is in `svc-mod`).

### 1.1 Latency (intra-region)

- **Enqueue (POST /v1/send):** p95 < **35 ms**, p99 < **75 ms**  
- **Dequeue (POST /v1/recv):** p95 < **35 ms**, p99 < **75 ms**  
- **Ack/Nack:** p95 < **20 ms**  
- **Health/Ready:** p95 < **10 ms**

### 1.2 Throughput (per node, sustained ≥15 min)

- **SEND**: ≥ **8,000 req/s**  
- **RECV (batch=32, visibility=5s)**: ≥ **12,000 req/s** (responses may be empty)  
- **ACK/NACK**: ≥ **15,000 req/s**  
- **Combined mixed workload** (40% send, 20% recv, 40% ack): ≥ **20,000 req/s**

> **Note:** RECV throughput is demand-driven and constrained by available ready messages and client leases. Numbers above assume hot queues.

### 1.3 Error Budget (steady state)

- 5xx **< 0.1%**  
- 429/503 backpressure **< 1%** (sustained)  
- Visibility timeouts (`mailbox_visibility_timeout_total / mailbox_enqueued_total`) **< 1%**  
- Integrity failures **= 0** (critical if > 0)

### 1.4 Resource Ceilings (at target load)

- **CPU**: < **70%** total across cores  
- **Memory**: < **1.5 GiB** RSS steady state (macronode profile)  
- **FDs**: < **40%** of system limit  
- **Allocations/op** (SEND): median ≤ **3** small allocs (payload buffers excluded; we use `bytes::Bytes`)

### 1.5 Milestones (gating)

- **Bronze**: Meets latency SLOs at 5k req/s mixed; no 5xx spikes during chaos restart.  
- **Silver**: Meets targets above; saturated behavior = graceful 429/Retry-After; scanner jitter < 100 ms.  
- **Gold**: 1.3× targets (26k mixed req/s) with p95s within +20% of SLO; 24h soak, zero integrity failures.

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

- **Hot paths**:
  - Idempotency table operations (insert/lookup TTL).
  - Inflight map update (monotonic deadline math).
  - Envelope (de)serialization (`serde_json` and protobuf).
  - BLAKE3 hashing of typical payload sizes (2 KiB, 8 KiB, 64 KiB).
- **Run**
  ```bash
  cargo bench -p svc-mailbox
````

### 2.2 Integration Load (wrk/bombardier)

* **Send flood** (idempotent mix):

  ```bash
  bombardier -c 400 -d 5m -l -m POST \
    -H "Authorization: Bearer $MAC" \
    -H "Content-Type: application/json" \
    -H "X-Corr-Id: test" \
    -f testing/performance/payloads/send.json \
    http://$HOST:8080/v1/send
  ```
* **Recv loop (NDJSON)**:

  ```bash
  curl -N -sS -X POST http://$HOST:8080/v1/recv/stream \
    -H "Authorization: Bearer $MAC" \
    -H "Content-Type: application/json" \
    -d '{"topic":"user:42:inbox","visibility_ms":5000,"max_messages":32}'
  ```
* **Mixed rig**: `testing/performance/mixed.lua` (wrk script) with 40/20/40 split.

### 2.3 Profiling

* **CPU flamegraph**

  ```bash
  cargo flamegraph -p svc-mailbox --bin svc-mailbox -- --bind 0.0.0.0:8080
  ```
* **Async stalls** (`tokio-console`)

  ```bash
  RUSTFLAGS="--cfg tokio_unstable" RUST_LOG=info \
  TOKIO_CONSOLE_BIND=127.0.0.1:6669 \
  cargo run -p svc-mailbox
  # open console: http://127.0.0.1:6669
  ```
* **Heap/allocs**

  ```bash
  sudo perf record -g -- cargo run -p svc-mailbox
  cargo install dhat; # or use heaptrack / valgrind on debug builds
  ```

### 2.4 Chaos/Perf Blend

* Introduce restart during load (`SIGTERM`; expect p95 blip < 500 ms, no 5xx burst > 1%).
* Inject 2× decompression bombs to confirm 413 and ratio caps.
* Simulate capability revocation surge; verify ≤ `cap_cache_ttl` (30s) to steady state.

### 2.5 CI Integration (nightly)

* Nightly GitHub Action runs `testing/performance/ci_mix.sh` for 3 minutes per node size; diffs against baselines; adds flamegraph artifact on regression.

---

## 3. Scaling Knobs

> Most knobs are defined in `CONFIG.md` under `[mailbox]`; they map directly to performance characteristics.

* **Shards (`ready_shards`)**
  Parallelism unit. Increase to scale dequeue concurrency; avoid HOL blocking across topics.
* **Shard capacity (`shard_capacity`)**
  Backpressure threshold. Too low → 429s; too high → memory bloat. Start: 4,096; increase linearly with traffic.
* **Global inflight cap (`global_inflight`)**
  Safety ceiling for leased messages. Must be ≥ one shard capacity; typical = 2× total shard cap / avg batch size.
* **Visibility (`default_visibility`, `visibility_ms_min`)**
  Lease duration; shorter increases redelivery churn; longer delays retries. Default 5s; do not go below 250 ms.
* **Batching (`max_messages`, `max_bytes`)**
  Throughput lever for RECV; sweet spot: 16–64 messages, bytes ≤ 512 KiB.
* **Replay window (`t_replay`)**
  Dedup memory/time trade-off (default 300 s). Larger windows reduce dup traffic, increase RAM.
* **Backoff (`backoff_base`, `backoff_max`)**
  NACK/consumer retry pressure valve; base=200 ms, max=60 s.
* **Profile (`micronode|macronode`)**
  Controls DLQ durability and memory profile.

**Transport/IO**

* JSON vs gRPC: gRPC yields ~20–30% lower CPU at high QPS due to binary encoding and streaming.
* **Zero-copy**: ensure payload handling uses `bytes::Bytes`; avoid string copies.

---

## 4. Bottlenecks & Known Limits

* **Content hashing (BLAKE3)**: CPU-bound at large payloads. Mitigation: parallel hash enabled by default; keep payload ≤ 1 MiB (hard cap).
* **JSON encode/decode**: Use gRPC for heavy consumers; for HTTP, keep `attrs` small and stable.
* **Idempotency map**: High churn with tiny `t_replay` and high QPS can cause cache thrash; ensure map size/TTL tuned (default OK up to ~50k rps cluster-wide).
* **Visibility scanner**: Excessively small visibility windows (< 500 ms) cause requeue storms; keep ≥ 1× RTT + processing time; default 5 s.
* **TLS handshakes**: Prefer keep-alive; gateway side terminates where possible; ensure session resumption.
* **Disk (macronode DLQ)**: If backed by disk/object store, reprocess burst can spike IO; throttle reprocess (`limit`) and stagger.

**Acceptable vs Must-fix**

* Minor reorder within shard under failover (bounded ≤ 32) — acceptable (per IDB).
* Sustained integrity failures — **must-fix immediately**.
* 429 > 1% in steady state — **tune or scale**.

---

## 5. Regression Gates

> CI fails the PR if any of the following exceed thresholds versus the **previous baseline** (stored under `testing/performance/baselines/`):

* p95 **request_latency_seconds** ↑ > **10%** (per route)
* Throughput (mixed workload) ↓ > **10%**
* CPU or memory at target load ↑ > **15%**
* `mailbox_visibility_timeout_total / mailbox_enqueued_total` ↑ > **0.5 pp**
* Any `integrity_fail_total` > 0

**How enforced**

```bash
# Generate new run artifacts
testing/performance/ci_mix.sh --host 127.0.0.1:8080 --out artifacts/
# Compare with baseline JSON
testing/performance/compare.py artifacts/ baselines/latest.json --fail-threshold 0.10
```

> Waivers require linking a root cause (e.g., dependency upgrade) and a planned fix window.

---

## 6. Capacity Model (Rules of Thumb)

Let:

* `S` = number of shards (`ready_shards`)
* `C` = per-shard capacity (`shard_capacity`)
* `V` = `default_visibility` (seconds)
* `μ_send` = mean processing time for SEND (ms)
* `μ_recv` = mean processing time for RECV (ms)
* `B` = `max_messages` per RECV
* `λ_send_max` ≈ `S * min( 1000/μ_send , C/V )`
* `λ_recv_max` ≈ `S * min( (B * 1000)/μ_recv , C/V )`

Interpretation:

* Each shard can **emit at most** `C/V` messages/sec without starving capacity (leases consume inflight budget for `V` seconds).
* If CPU/path cost dominates (μ), CPU caps the rate; otherwise leasing math does.

**Example** (defaults: `S=8`, `C=4096`, `V=5s`, `μ_send=0.20ms`, `μ_recv=0.35ms`, `B=32`):

* `C/V = 819.2 msg/s/shard` → `~6.5k msg/s` cluster limit by inflight if fully leased.
* CPU path allows `5k req/s/shard` for send (but inflight caps lower).
* Increase `S` to 16 or reduce `V` to 3s (if consumer is fast) to raise ceiling — monitor visibility timeouts.

---

## 7. Perf Runbook (Triage)

1. **Confirm SLO breach**: Grafana panels (latency p95, saturation, visibility timeouts).
2. **Identify limiter**:

   * If `saturation{work}` > 0.8 and CPU < 70% → inflight/lease limited → raise `S` or adjust `V/B`.
   * If CPU > 85% with low saturation → optimize CPU path or switch heavy consumers to gRPC.
   * If visibility timeouts rising → increase `V` or reduce client batch/processing time.
3. **Profile**:

   * Flamegraph for SEND/RECV endpoints during breach. Look for `blake3`, `serde_json`, map contention.
   * `tokio-console`: long polls, blocked tasks.
4. **Tune knobs** (in order):

   * Increase `ready_shards` by 2×; keep `global_inflight` ≥ `S*C/2`.
   * Raise `max_messages` from 32 → 64 (keep `max_bytes ≤ 512 KiB`).
   * Adjust `default_visibility` toward 3–7 s window guided by consumer RTT.
   * For HTTP heavy traffic, move hot paths to gRPC.
5. **Re-test** at 1.2× target; verify 429 rate and p95s.
6. **If still failing**:

   * Pinpoint payload sizes (are clients hitting 1 MiB often?).
   * Check TLS termination (offload to gateway).
   * Verify kernel params: file descriptor limits, TCP backlog, reuseport.

---

## 8. Acceptance Checklist (DoD)

* [ ] SLOs concretely defined & recorded in dashboards.
* [ ] Micro + integration harness implemented and runnable locally & in CI.
* [ ] Flamegraph and `tokio-console` traces captured at least once per milestone.
* [ ] Scaling knobs documented & wired to config (see `CONFIG.md`).
* [ ] Regression gates active in CI with baselines.
* [ ] Perf runbook validated via a dry run.
* [ ] Bronze→Gold milestone evidence stored under `testing/performance/reports/`.

---

## 9. Appendix

### 9.1 Reference Workloads

* **Hot Inbox**: 5 producer clients push 8 KiB payloads @ 6k rps total; 20 consumer clients recv(32)@5s, ack immediately.
* **Spiky Producer**: 30s burst to 2× sustained rps; expect graceful 429/Retry-After (no 5xx burst).
* **Poison Mix**: 0.1% malformed payloads; expect DLQ growth with `reason="parse_error"`, zero 5xx.
* **Replay Storm**: 5% duplicate SENDs (same idem/payload); server must return `{duplicate:true}` quickly.

### 9.2 Tooling Quick Ref

```bash
# Local node with tuned knobs (example)
SVCMBX_BIND_ADDR=0.0.0.0:8080 \
SVCMBX_SHARDS=16 \
SVCMBX_SHARD_CAP=8192 \
SVCMBX_GLOBAL_INFLIGHT=16384 \
SVCMBX_VISIBILITY_DEFAULT=5s \
SVCMBX_MAX_MESSAGES=32 \
SVCMBX_MAX_BYTES=512KiB \
SVCMBX_T_REPLAY=300s \
RUST_LOG=info \
cargo run -p svc-mailbox
```

### 9.3 Perfection Gates

* **Gate F (no regressions)**: CI blocks >10% degradation in p95/throughput, >15% CPU/mem.
* **Gate L (scaling)**: Demonstrate 1.3× target under chaos; attach traces and metrics.

### 9.4 History (fill as we iterate)

* `2025-10-12`: Initial SLOs + capacity model; Bronze targets achieved on 8 vCPU node.

---

```
```
