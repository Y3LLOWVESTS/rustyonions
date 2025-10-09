

---

# ⚡ PERFORMANCE.md — svc-registry

---

title: Performance & Scaling — svc-registry
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-08
audience: contributors, ops, perf testers
-----------------------------------------

## 0. Purpose

Define the **performance profile** for `svc-registry`: append-only, signed, quorum-gated write plane with a **fast, cacheable read plane**. This doc sets SLOs, workloads, harnesses, profiling, scaling knobs, and CI regression gates. It aligns with Scaling Blueprint SLOs and Perfection Gates (F: perf regressions barred; L: scaling chaos-tested).

---

## 1. SLOs / Targets

### 1.1 Read Plane

* **GET `/registry/head`**

  * Intra-region: **p95 < 50 ms**, p99 < 90 ms
  * Inter-region: **p95 < 200 ms**, p99 < 350 ms
  * Sustained throughput: **≥ 2,500 RPS/node** at p95 budget above
  * Degradation policy: graceful **429** once inflight > concurrency cap for ≥10s

* **GET `/registry/{version}` (immutable)**

  * Intra-region p95 < 100 ms (cacheable), ≥ 3,000 RPS/node sustained

* **SSE `/registry/stream`**

  * Event propagation p95 **< 200 ms** from commit to first byte
  * Lossy allowed under lag; clients reconcile via `/registry/head`

### 1.2 Write/Commit Plane (propose → approve → commit)

* **Commit p95 < 750 ms**, p99 < 1200 ms at **1–5 commits/s** regional cadence
* **Verify budget** (M-of-N approvals, Ed25519 over `payload_b3`): p95 **< 7 ms/approval**
* **Error budget**: 5xx **< 0.1%**, 429/503 **< 1%** sustained, audit-verify failures **= 0** (hard fail)

### 1.3 Resource Ceilings (at target load)

* CPU: median **< 60% of 1 core**, p95 **< 85%** during bursts
* Memory: steady **< 256 MiB** (sled/sqlite), burst **< 512 MiB** during checkpoint/compaction
* FD usage **< 30%** of limit; inflight HTTP bounded by `max_conns`

### 1.4 Cold-start / Readiness

* Process start → **/readyz true** (read plane only): **< 800 ms**
* Full write readiness (storage + signer set hydrated): **< 2.5 s**
* After crash/upgrade with checkpoint restore: **< 3.5 s** to ready (reads), **< 6 s** (writes)

### 1.5 Profile Variants

* **Micronode (amnesia=ON, tmpfs)**: lower FS latency; expect **~10–20% better** read p95; **commit p95 may worsen** if signer verification dominates CPU (small machines).
* **Macronode (persistent)**: higher fsync latency; mitigate via checkpoint cadence and storage tuning (§3).

---

## 2. Benchmarks & Harness

### 2.1 Micro (Criterion)

* Verify approvals (Ed25519) over canonical JSON (`payload_b3`)
* BLAKE3 hashing of descriptor sets (varying sizes)
* CAS/HEAD advance (append-only log)
* Track: time/op, allocs/op, P50/P95/P99, slope vs payload size

### 2.2 Integration Load

* **Reads**: `wrk`/`bombardier` against `/registry/head` and `/registry/{version}` with realistic cache headers; RPS sweeps (500 → 3,000).
* **Commits**: simulate **M-of-N** approvals (distinct signer IDs), bursts to **5 commits/s**; assert commit p95/p99 SLOs, **0** audit failures; verify **non-blocking bus** publish with drop counters.

### 2.3 Profiling

* `cargo flamegraph` (read/commit scenarios)
* `tokio-console` for task stalls in approve/commit pipeline
* `perf` or `coz` for causal profiling

### 2.4 Chaos + Perf

* Inject storage latency/fsync jitter; slow bus consumers; verify **/readyz** sheds writes first (429/503 + `Retry-After`) and read plane remains green.

### 2.5 CI Integration

* Nightly perf job runs the above and compares to JSON baselines under `testing/performance/baselines/` (kept per-profile: micronode/macronode).
* On regression, CI uploads flamegraphs + console traces as artifacts.

---

## 3. Scaling Knobs

* **Concurrency**: Tower concurrency cap; per-route semaphore; **serialized HEAD advance** to control tail latency.
* **Storage**: choose backend (`sled`/`sqlite`); tune checkpoint interval (target **checkpoint_age_seconds < 60s**); set compaction window off-peak.
* **I/O**: stream request bodies; hard cap `max_frame=1 MiB`; leverage `bytes::Bytes` zero-copy.
* **Bus**: bounded broadcast buffer; drops counted and reconciled via `/registry/head`.
* **Horizontal**: scale read replicas freely (stateless); maintain **single logical writer** per instance for commit section.
* **Vertical**: if verify-bound → add CPU; if fsync-bound → faster disk/tmpfs or batch fsyncs (with integrity guardrails).
* **Sharding Thresholds (guidance)**: when registry size > **50k descriptors** OR commit RPS > **10/s** sustained, evaluate (a) **partitioned logs** by region/facet and (b) signer-pool offloading.

---

## 4. Bottlenecks & Known Limits

* **Signature verify** dominates at **M ≥ 5** approvals—watch verify budget per approval.
* **Canonicalization + BLAKE3** on large payloads; ensure DTO hygiene (`deny_unknown_fields`) to avoid decode churn.
* **Storage fsync / checkpoint** pauses—monitor `registry_checkpoint_age_seconds` and write-amp gauges.
* **Bus slow consumers** → intentional **lossy** behavior; correctness via `/registry/head` reconciliation.
* **Acceptable**: short **busy/Retry-After** during governance storms.
* **Must-fix**: any audit verify failure; sustained checkpoint staleness; commit p95 > 750 ms for >5 min.

---

## 5. Regression Gates (CI must fail if…)

* **Read p95** (`/registry/head`) ↑ **> 10%** vs baseline
* **Commit p95** ↑ **> 10%** vs baseline
* **Throughput** at fixed RPS ↓ **> 10%**
* CPU or Memory at target load ↑ **> 15%**
* Golden metrics missing: `http_requests_total`, `request_latency_seconds{*}`, `rejected_total{reason}`, `bus_lagged_total`, `registry_checkpoint_age_seconds`

Waivers allowed only with an attached root cause (e.g., upstream dep) and new baselines approved by owners.

---

## 6. Perf Runbook (Triage)

1. **Dashboards**

   * Service Overview: p95/p99, inflight, rejects
   * Commit Path: verify time/approval, approvals per commit, commit latency
   * Audit/Retention: checkpoint age, fsync time, storage bytes
2. **PromQL quick checks**

   * p95 read latency (5m):

     ```
     histogram_quantile(0.95,
       sum by (le, route) (rate(request_latency_seconds_bucket{route="/registry/head"}[5m]))
     )
     ```
   * Commit p95 (5m):

     ```
     histogram_quantile(0.95,
       sum by (le) (rate(commit_latency_seconds_bucket[5m]))
     )
     ```
   * Error rate (5xx):

     ```
     sum(rate(http_requests_total{code=~"5.."}[5m])) 
     / ignoring(code) sum(rate(http_requests_total[5m]))
     ```
   * Bus lag/drops:

     ```
     rate(bus_lagged_total[5m])
     ```
   * Checkpoint staleness:

     ```
     max_over_time(registry_checkpoint_age_seconds[5m])
     ```
3. **Flamegraph** read/commit; look for verify/hash/storage hotspots
4. **tokio-console**; find stalled tasks in commit pipeline
5. **Stress knobs**: lower commit concurrency; increase CPU; tighten bus buffers; adjust checkpoint cadence
6. **Chaos toggle**: disable heavy exporters/compression; re-measure
7. **Profiles**: if Micronode, confirm tmpfs and amnesia; if Macronode, inspect disk IO (fsync)

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs (reads, commits, cold-start) defined and alert rules staged
* [ ] Bench harness runs locally + CI with artifacts (flamegraphs, console traces)
* [ ] Scaling knobs documented in `CONFIG.md` and observable via `/metrics`
* [ ] Regression gates wired to baselines (micronode & macronode)
* [ ] Runbook PromQL validated against staging data
* [ ] Perf evolution entry added (§8)

---

## 8. Appendix

### 8.1 Perf Evolution (change log)

* 2025-10-08: Initial SLOs and baselines (Micronode/Macronode split).
* (add future perf-affecting changes here with links to PRs and new baselines)

### 8.2 Reference Workloads

* Read sweeps: 500→3,000 RPS `/registry/head`, 1k→4k RPS `/registry/{version}`
* Commit cadence: 1–5 commits/s with M-of-N approvals (N ≥ 7, M ≥ 3)

### 8.3 PQ Note (forward-looking)

* If migrating to PQ/hybrid signatures (e.g., Dilithium-class), **verify cost will rise** (order(s) vs Ed25519). Keep verify budget separately tracked and plan for CPU scale-up or signer offload.

---

