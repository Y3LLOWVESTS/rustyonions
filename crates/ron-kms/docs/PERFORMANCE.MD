

---

# ⚡ PERFORMANCE.md — ron-kms

---

title: Performance & Scaling — ron-kms
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-09
audience: contributors, ops, perf testers
-----------------------------------------

## 0) Purpose

Define the **performance profile** for `ron-kms` (custody/sign/verify, wrap/unwrap, key lifecycle):

* SLOs for latency/availability/error budgets and attestation posture.
* Benchmarks & workloads (classical vs PQ/hybrid) with **explicit baselines**.
* Perf harness & profiling tools wired to CI (drift gates).
* Scaling knobs (semaphores, pools, verify cache, per-tenant fairness).
* Chaos & triage steps for realistic outages (HSM/TPM flap, PQ spikes, memory pressure).

Binds to canon: Scaling Blueprint, Six Concerns (PERF), Hardening defaults, Observability metrics, QUANTUM (PQ phases), CONFIG (env knobs).

---

## 1) SLOs / Targets

### 1.1 Latency (p95 steady state, non-saturated)

* **sign(Ed25519)**: ≤ **1.0 ms**
* **sign(ML-DSA/Dilithium3)**: ≤ **10 ms** (HSM/file backend profile noted in baseline table)
* **verify (cache-hit)**: ≤ **1.0 ms**
* **verify (cache-miss, classical)**: ≤ **5.0 ms**
* **wrap/unwrap (ML-KEM/Kyber)**: ≤ **6.0 ms** p95

> All targets measured intra-region on the documented baseline host; inter-region adds network RTT and is tracked separately in dashboards.

### 1.2 Availability & Error Budget

* **Availability**: `/readyz=true` ≥ 99.99% monthly (planned maintenance excluded).
* **Error budget**: 5xx < **0.1%**; backpressure 429/503 < **1%** over 5-min windows.
* **Audit integrity**: `kms_audit_integrity_failed_total == 0`.
* **Attestation posture** (when policy requires): `attestation_ready=true` ≥ 99.99%.

### 1.3 Resource Ceilings @ Target Load

* **CPU**: < **60%/core** median (headroom for PQ spikes).
* **Memory**: < **512 MiB** steady (service process; HSM external).
* **FDs**: < **30%** of system limit (bounded PKCS#11 sessions).
* **GC/allocs**: no per-op alloc regressions > **15%** vs baseline.

### 1.4 PQ / Hybrid Rollout SLO Hooks

* During rollout: `kms_reject_total{reason="pq_policy"} == 0` for tenants at Phase 0–2; violations must auto-rollback policy to `verify=or`.
* Verify cache **staleness alert**: `cache_age_ms > 2×TTL` (numerically enforced; see §6.4).

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

Hot paths & counters:

* `sign(ed25519)`, `sign(ml-dsa)`, `verify(hit/miss)`, `wrap`, `unwrap`, AEAD seal/open.
* Metrics: ops/sec, ns/op, allocations/op; heaptrack snapshots optional per PR.
* Distributions captured for p50/p95/p99 with run metadata (commit, toolchain, host).

### 2.2 Integration Load (HTTP + gRPC)

* Endpoints: `POST /v1/kms/{sign|verify|wrap|unwrap|keys|rotate}` and gRPC mirrors.
* Rigs: `bombardier`, `wrk`, `gwsmoke` suites; gRPC load via `ghz`.
* Mixes:

  * **Steady**: 70% sign, 20% verify, 10% wrap/unwrap.
  * **Burst**: 10× RPS for 60s every 5 min; ensure shed before work.
  * **Rotate flood**: simulate PQ migration spike (rotate QPS 10× for 5 min).
  * **Tenant isolation**: dual-tenant test (80/20 QPS) verifies fair-share caps.

### 2.3 Profiling

* `cargo flamegraph` (CPU hotspots: PKCS#11, PQ sig/KEM, AEAD, serde).
* `tokio-console` (stalls; pool contention).
* `perf` / `coz` for causal gains; `hyperfine` for one-shot API latency.

### 2.4 Chaos/Perf Blend

* HSM/TPM adapter latency injection & session flap.
* Slow-loris ingress; oversized/compress-bomb bodies (must reject pre-work).
* Network jitter & packet loss (tc netem) to model inter-region tails.

### 2.5 CI Integration

* Nightly perf pipeline runs all suites; baselines loaded from `testing/performance/baselines/ron-kms/*.json`.
* CI fails on drift gates (§5). Waivers require linked upstream issue + temporary baseline bump PR.

---

## 3. Scaling Knobs

* **Concurrency & Backpressure**

  * Global **semaphore caps** per op type with **per-tenant fair-share**:
    `KMS_MAX_INFLIGHT_SIGN`, `KMS_MAX_INFLIGHT_VERIFY`, `KMS_TENANT_SHARE_MIN` (e.g., 5% floor).
  * Early shed at ingress (429/503) before heavy work paths.

* **Backend Pools**

  * Bounded **PKCS#11/TPM session pools**; JIT acquisition; circuit-breakers on flap; exponential backoff with jitter.

* **Verify Cache**

  * Size & TTL: `KMS_VERIFY_CACHE_MAX`, `KMS_VERIFY_CACHE_TTL_MS`; LRU keyed by `kid#vN`.
  * Miss path p95 target ≤ 5 ms; **stale alert** at `2×TTL`.

* **PQ Policy**

  * `KMS_PQ_PRODUCE_HYBRID=true|false`
  * `KMS_PQ_VERIFY_MODE=or|and|pq_only`
  * `KMS_PQ_ROLLOUT_PERCENT=0..100` (per-tenant)

* **Ingress Hardening**

  * Timeouts, RPS caps, body cap **1 MiB**, decompression ratio ≤ **10×**.

* **Amnesia Mode**

  * RAM-only keys; ephemeral logs; verify cache allowed, memory-bounded.

---

## 4. Bottlenecks & Known Limits

* **PKCS#11/TPM latency & session churn**: tail-latency driver; mitigate with pooling, breaker, and fair scheduler.
* **PQ cryptography cost**: ML-DSA/KEM larger & slower; rollout phases must budget +CPU and +wire bytes.
* **Verify cache cold starts**: spike on rollout or mass rotate; watch `cache_miss_ratio`.
* **Noisy neighbors**: without per-tenant shares, a single tenant can starve others under burst—use fair-share caps.

Acceptable vs Must-fix:

* Acceptable (Bronze): brief p99 spikes during backend failover (<2 min).
* Must-fix: sustained `kms_reject_total{reason="pq_policy"}` > 0, verify cache staleness breaches, pool starvation, or 5xx > 0.1%.

---

## 5. Regression Gates (CI must fail if…)

* **Latency**: p95 `sign(ed25519)` ↑ > **10%**, `sign(ml-dsa)` ↑ > **15%**, `verify(hit)` ↑ > **10%** vs baseline.
* **Throughput**: `sign/sec` ↓ > **10%** at fixed error budget.
* **Resources**: CPU or memory ↑ > **15%** at target load.
* **Taxonomy**: sustained 5xx > **0.1%** or 429/503 > **1%** (without a matching burst test).
* **PQ**: increase in `kms_reject_total{reason="pq_policy"}` above baseline > **0** for Phase 0–2 tenants.
* **Fairness**: tenant P95 latency skew > **2×** between top-QPS and low-QPS tenants in fair-share suite.

Baselines in `testing/performance/baselines/ron-kms/`. Waivers require explicit PR with justification and time-boxed expiry.

---

## 6. Perf Runbook (Triage)

1. **Confirm the hotspot**

   * Flamegraph → PKCS#11 vs PQ crypto vs serde.
   * tokio-console → pool waits, blocked tasks.

2. **Read metrics**

   * `kms_latency_seconds{op,alg}`, `kms_ops_total{op,alg}`, `rejected_total{reason}`, `backend_reconnects_total`, `verify_cache_{hit,miss,stale}_total`.

3. **Isolate & toggle**

   * Switch `KMS_PQ_VERIFY_MODE=or` and/or lower `KMS_PQ_ROLLOUT_PERCENT` to separate PQ cost from infra issues.
   * Temporarily disable request decompression.

4. **Scale knobs**

   * Raise per-op semaphore by **10–20%**; enlarge verify cache **by 25%** (watch memory).
   * Adjust pool sizes cautiously; verify breaker isn’t open-flapping.

5. **Memory pressure drill**

   * Capture heap: run with `MALLOC_CONF`/jemalloc profiling or `heaptrack` for 2 minutes at load.
   * If cache growth drives RSS: lower `KMS_VERIFY_CACHE_MAX`, shorten TTL by **25%**, re-run.

6. **Staleness thresholds**

   * If `verify_cache_stale_max_ms > 2×TTL` (e.g., TTL=300_000 ms → alert at >600_000 ms), force cache sweep and drop items > TTL; re-warm via background verifies if enabled.

7. **Backend failover**

   * If HSM flap: reduce pool concurrency by **30%**, increase backoff jitter, and (policy permitting) fail over to sealed-file backend for **verify-only**; block sign if attestation required.

8. **Record outcome**

   * Append regression note to Appendix E (symptom, cause, fix, commit, baseline diffs).

---

## 7. Acceptance Checklist (DoD)

* [ ] Histograms wired: `kms_latency_seconds{op,alg}` (p50/p95/p99), cache hit/miss/stale counters.
* [ ] Baselines captured on documented host (see §8.A) for **classical** and **PQ**.
* [ ] Nightly perf CI compares against baselines; drift gates enforced.
* [ ] Tenant fair-share suite passing; skew ≤ 2× at p95.
* [ ] Chaos drills run: HSM flap, burst 10×, rotate flood, memory pressure.
* [ ] Runbook exercised once; artifacts (SVG flamegraph, tokio-console trace) attached to CI.

---

## 8. Appendix

### A) Documented Baseline Host & Toolchain

| Profile         | Host          |       vCPU/RAM | Toolchain              | HSM/Backend             |
| --------------- | ------------- | -------------: | ---------------------- | ----------------------- |
| **A (default)** | AWS m6i.large | 2 vCPU / 8 GiB | rustc 1.80.x, jemalloc | File-sealed (no HSM)    |
| **B (HSM)**     | AWS m6i.large |          2 / 8 | rustc 1.80.x           | PKCS#11 (YubiHSM-class) |

> All numbers below were last refreshed on the date in header.

### B) Reference Baselines (A — default)

| Operation               |     p50 |     p95 |     p99 | Throughput (ops/sec) |
| ----------------------- | ------: | ------: | ------: | -------------------: |
| sign(Ed25519)           | 0.35 ms | 0.85 ms |  1.6 ms |               11,500 |
| verify(hit)             | 0.20 ms | 0.70 ms |  1.2 ms |               18,000 |
| verify(miss, classical) | 0.90 ms |  3.6 ms |  6.8 ms |                7,800 |
| sign(ML-DSA)            |  4.4 ms |  9.2 ms | 14.8 ms |                1,800 |
| wrap(ML-KEM)            |  2.8 ms |  5.7 ms |  9.9 ms |                3,200 |

*(Illustrative placeholders until your first CI run writes real JSON baselines.)*

### C) PQ Hybrid Cost Deltas (A — default)

| Scenario                 | Δ p95 latency vs classical | Δ payload size |
| ------------------------ | -------------------------: | -------------: |
| **Hybrid-OR verify**     |                    +1.2 ms |       +1.1 KiB |
| **Hybrid-AND verify**    |                    +2.3 ms |       +1.1 KiB |
| **sign (hybrid bundle)** |                    +7.9 ms |       +1.1 KiB |

> Use these as sanity bands; CI will learn exact values per host.

### D) Workloads (explicit mixes)

* **Steady**: 70/20/10 sign/verify/wrap, 5m duration, RPS = target × 0.8.
* **Burst**: RPS = target × 8 for 60s every 5m, 30m duration.
* **Rotate flood**: rotate QPS = steady × 10 for 5m, then steady.
* **Tenant isolation**: Tenants A/B at 80/20 QPS; ensure p95 skew ≤ 2×.

### E) CI Wiring (commands)

```
cargo bench --profile perf
scripts/perf/run_http.sh --mix steady --duration 300
scripts/perf/run_grpc.sh --mix burst --burst-mult 10 --interval 300
scripts/perf/check_drift.py testing/performance/baselines/ron-kms/*.json target/perf/*.json
```

### F) History (regressions & fixes)

* *2025-10-09*: **Template init** — added baseline scaffolds; tenant fairness suite; PQ delta bands.
* *YYYY-MM-DD*: (add entries as they occur)

---
