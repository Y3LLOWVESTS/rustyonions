

---

# ⚡ PERFORMANCE.md — ron-audit

---

title: Performance & Scaling — ron-audit
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-08
audience: contributors, ops, perf testers
-----------------------------------------

## 0) Purpose

Define the **performance profile** of `ron-audit`, a **library** for tamper-evident, hash-chained audit records with optional durability/export handled by the **host**. We set measurable targets, benches, knobs, bottlenecks, and CI gates to prevent drift—while honoring SECURITY (no `unsafe`), IDB bounds (record/attrs size limits), and CONFIG profiles (Micronode amnesia vs. Macronode durable).

---

## 1) SLOs / Targets (Library Mode)

Assume 1 KiB canonical body, ≤1 KiB attrs, release build, AVX2 x86_64, single core unless noted.

### 1.1 Quick-scan SLO table

| Profile                                                                                | Metric                              | Target                                                              |
| -------------------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------- |
| Micronode (amnesia; `feature = "wal"` **disabled**, `feature = "export"` **disabled**) | **Append throughput (single core)** | ≥ **120k ops/s** p50, ≥ **90k ops/s** p95                           |
|                                                                                        | **Append latency**                  | p95 ≤ **0.8 ms**, p99 ≤ **1.5 ms**                                  |
|                                                                                        | **Allocations/op**                  | ≤ **8**                                                             |
|                                                                                        | **Bytes/op (heap)**                 | ≤ **3 KiB**                                                         |
|                                                                                        | **Cold start**                      | ≤ **2 ms**                                                          |
| Macronode (durable; `feature = "wal"` **enabled**, `feature = "export"` optional)      | **WAL fsync**                       | `wal_fsync_seconds` p95 ≤ **8 ms**, p99 ≤ **15 ms** (NVMe, batched) |
|                                                                                        | **Export/checkpoint**               | `export_batch_seconds` p95 ≤ **250 ms** per 10k records             |
| All                                                                                    | **Verify self-hash**                | ≥ **400k ops/s** (single core, 1 KiB)                               |
| All                                                                                    | **Chain link check (prev→self)**    | ≥ **250k links/s** (in-mem batch)                                   |
| All                                                                                    | **Busy / shed**                     | `busy_rejections_total{endpoint="emit"}` ≤ **0.5%** at design load  |
| All                                                                                    | **Verify failures**                 | `audit_verify_fail_total` **= 0** steady-state                      |

**Error budget** (observed by host): failures <0.1%, quota 429/503 <1%, bus overflow <0.01%.

> Note: `ron-audit` is lib-only; hosts own registration of metrics, readiness keys, quotas, and shedding policy. This doc defines the numeric goals the host should see.

---

## 2) Benchmarks & Harness

### 2.1 Criterion micro-benches (crate-local)

* `benches/append_hot.rs` — canonicalize → BLAKE3 → chain → Ed25519 sign (RAM).
* `benches/verify_hot.rs` — `self_hash`, `prev→self` chain continuity.
* `benches/dedupe_key.rs` — cost & stability of `dedupe_key`.

Run & store baselines:

```
cargo bench -p ron-audit -- --save-baseline=main
```

### 2.2 Workspace perf rigs (host-facing but required for DoD)

* `testing/performance/audit/wal_steady.sh` — 1–5 kHz append with batched fsync; capture `wal_fsync_seconds`, `audit_queue_depth`.
* `testing/performance/audit/export_batch.sh` — synthesize 10k records; time export/checkpoint; record `export_batch_seconds`.
* `testing/performance/audit/shed_storm.sh` — drive queue to 110% capacity; validate `audit_drop_total{reason}`, `busy_rejections_total` rates; verify alerts.

### 2.3 Profiling

* `cargo flamegraph` (CPU hotspots), `perf stat` (LLC/branch), `coz` (optional causal profiling).
* For async hosts, use `tokio-console` to spot stalls around audit sinks.

---

## 3) Scaling Knobs (Library Levers)

* **CPU parallelism:** run N workers ≈ `min(physical_cores, queue_bound)`. Append path is CPU-bound (BLAKE3 + Ed25519).
* **Queue bounds:** bounded `mpsc`; burst absorption via depth; **shed** on full (never unbounded growth). Track `audit_queue_depth` and drops.
* **Batching:** WAL fsync batch = 8–64 records. Larger batches → lower p95 fsync but longer worst-case flush. Tune per media.
* **Export cadence:** checkpoint per **10k records** or **15 min**, whichever first; balance Merkle/build cost vs. recovery time.
* **Amnesia mode:** disable `wal`/`export` features in Micronode; rely on verification on read + replication at higher layers.

---

## 4) Bottlenecks & Known Limits

* **Ed25519 sign/verify** dominates CPU after BLAKE3; scales linearly with threads until memory bandwidth limits.
* **WAL fsync p99 spikes** under contention; must be smoothed with batching and fast storage; watch `wal_fsync_seconds`.
* **Queue saturation** on bursts is **by design shed-not-block**; acceptable if within shed budget; otherwise raise bounds/workers or add replicas.
* **Schema growth impact:** any interop change that increases record size (new fields) **must** be re-baselined; expect proportional hash/sign cost increase.
* **PQ readiness (future):** migrating signatures to ML-DSA/Dilithium will reduce throughput materially; track as a controlled perf step (see §8.4).

---

## 5) Regression Gates (CI-enforced)

CI fails if any of the following exceed the last green **baseline**:

* **Append throughput (RAM)**: drop > **10%** (p50 or p95).
* **Verify throughput**: drop > **10%**.
* **Allocations/op**: +> **2** allocs vs. baseline.
* **WAL fsync p95** (Macronode rig): +> **20%** at fixed batch size.
* **Busy/shed rate** at design load: +> **0.5 percentage points** absolute.

Baselines are committed under:

```
testing/performance/baselines/ron-audit/{criterion_json, metrics_snapshots}/
```

Waivers require: root cause (e.g., upstream `blake3`/`ring` change), CHANGELOG note, and perf sign-off.

---

## 6) Perf Runbook (Triage)

1. **Dashboards first:** Audit Throughput, Queue Health, Shedding, Verification, Durability.
2. **Counters:** rising `audit_drop_total{reason}` or `busy_rejections_total` ⇒ increase queue bounds/workers or scale out; confirm shedding is measured (not silent).
3. **Verification:** any `audit_verify_fail_total` > 0 ⇒ isolate range; recompute chain/Merkle; compare with latest checkpoint; **escalate to SECURITY owners**.
4. **CPU profile:** `cargo flamegraph` → check BLAKE3 vs. Ed25519 balance; consider batching or enabling target-CPU flags in host build.
5. **I/O smoothing:** tune fsync batch; ensure WAL/export on low-latency disk; watch `wal_fsync_seconds` p95.
6. **Chaos reproduction:** re-run `shed_storm.sh` and `wal_steady.sh` with disk-pressure or latency injection (see RUNBOOK) to confirm remediation.

Readiness: hosts should fail-closed for write paths if the audit plane is degraded (document the readiness key and consequences in the host’s README/RUNBOOK).

---

## 7) Acceptance Checklist (DoD)

* [ ] SLOs codified with quick-scan table.
* [ ] Criterion benches present, baselines captured and stored.
* [ ] Workspace rigs for WAL/export/shed run and produce metrics snapshots.
* [ ] Required metrics wired by host (`audit_appended_total`, `audit_drop_total{reason}`, `audit_queue_depth`, `audit_bytes_total`, `audit_verify_fail_total`, `wal_fsync_seconds`, `export_batch_seconds`, `busy_rejections_total`).
* [ ] CI regression gates enforce thresholds; waiver path documented.
* [ ] Runbook section updated with escalation and chaos steps.
* [ ] PRs that can affect perf include baseline diffs and reviewer sign-off.

---

## 8) Appendix

### 8.1 Grafana panels (formulas)

| Panel            | Query (PromQL)                                                                                                              |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Audit Throughput | `sum by(service,kind)(rate(audit_appended_total[5m]))`                                                                      |
| Queue Health     | `audit_queue_depth` with threshold; overlay `rate(busy_rejections_total[5m])`                                               |
| Shedding         | `sum by(reason)(rate(audit_drop_total[5m]))`                                                                                |
| Verification     | `rate(audit_verify_fail_total[5m])`                                                                                         |
| Durability       | `histogram_quantile(0.95, sum(rate(wal_fsync_seconds_bucket[5m])) by (le))` and `rate(audit_export_checkpoints_total[15m])` |

### 8.2 Perf flow (Mermaid)

```mermaid
flowchart LR
  A[emit(record)] --> B{bounded queue}
  B -- try_send ok --> C[append: canonicalize → BLAKE3 → chain → sign]
  B -- full --> D[drop + count busy/ shed]
  C -->|Micronode| E[verify on read]
  C -->|Macronode| F[WAL batched fsync]
  F --> G[export + checkpoint]
```

### 8.3 Baseline hardware (example — commit with your lab specs)

| CPU           | Storage   | OS           | Rust   | Notes                         |
| ------------- | --------- | ------------ | ------ | ----------------------------- |
| Ryzen 9 7950X | NVMe Gen4 | Ubuntu 24.04 | 1.80.0 | AVX2 on; governor performance |

### 8.4 PQ & Interop evolution notes

* **PQ**: When enabling `feature = "pq"` (ML-DSA/Dilithium), expect **≥2×** sign/verify cost; capture new baselines and relax throughput gates accordingly in a dedicated PR.
* **Interop**: Adding fields to `AuditRecord` (schema growth) increases canonical bytes; **re-run benches** and adjust baselines.

---

### Governance tie-in

Scaling choices (e.g., replicas, queue policy changes) should follow the project’s governance workflow: include perf evidence, dashboard screenshots, and CI baseline diffs in the proposal/PR.

---
