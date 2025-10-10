````markdown
# ⚡ PERFORMANCE.md — svc-passport

---
title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-09
audience: contributors, ops, perf testers
---

## 0) Purpose

Define the **performance profile** for `svc-passport`:
- SLOs (issuance/verify/revoke) and error budgets.
- Benchmarks & perf harness.
- Profiling toolkit and triage playbook.
- Scaling knobs and known bottlenecks.
- CI regression gates to prevent silent drift.

Anchors:
- Scaling Blueprint v1.4 SLO patterns and runbooks.
- Pillar 3 (Identity & Keys) role/SLO expectations.
- Observability metrics of record and alert names defined for this crate.

---

## 1) SLOs / Targets

### Service (per instance, steady-state)

**Latency**
- **Issue**: p95 ≤ **40 ms**, p99 ≤ **100 ms**.  
- **Verify (preflight)**: p95 ≤ **10 ms**, p99 ≤ **25 ms**.  
- **Revocation propagation** (event → downstream refusal): p99 ≤ **5 s**.

**Throughput**
- Target **≥ 500 rps** on a 4c/8GB node for `POST /v1/passport/issue` (reference bench); verify capacity before raising limits.

**Error Budget**
- 5xx < **0.1%**; 429/503 < **1%** sustained.

**Resource Ceilings (initial)**
- CPU: < **70%/core** at target load; Memory: < **512 MiB** steady; FD usage < **50%** of limit (tune by environment).

**Readiness & Degradation**
- `/readyz` fails **issuance** under pressure or missing deps (KMS/Registry); the verify preflight may remain available if fail-closed semantics are preserved for issuance.

**Metrics of Record**
- `svc_passport_issue_latency_seconds{le,...}`  
- `svc_passport_verify_latency_seconds{le,...}`  
- `svc_passport_rejects_total{reason}`  
- `busy_rejections_total{endpoint="/v1/passport/issue"}`

**PromQL examples**
- p95 issuance:  
  `histogram_quantile(0.95, sum(rate(svc_passport_issue_latency_seconds_bucket[5m])) by (le))`
- 429 backpressure:  
  `sum(rate(busy_rejections_total{endpoint="/v1/passport/issue"}[5m]))`

**SLO Review & Derivation**
- Issuance p95 ≤ 40 ms is set to ~50% of the intra-region north–south budget to preserve headroom for gateway + network.
- Verify p95 ≤ 10 ms reflects a cache-friendly read path with no KMS hop.
- **Cadence:** Quarterly SLO review during release planning; changes must update alerts, baselines, and this doc.

**Cold-Start**
- First request after process start: **p95 ≤ 150 ms** (DNS/TLS warmups, KMS client init). Measure during rolling deploys.

**Inter-Region Verify**
- When caller and `svc-passport` are in distinct regions: **p95 ≤ 120 ms**, **p99 ≤ 200 ms**.

**Burn-Rate Alerts (multi-window)**
- **Fast**: 2h window, alert if projected monthly error-budget burn ≥ **14×**.
- **Slow**: 24h window, alert if projected monthly burn ≥ **6×**.
- Apply to latency SLOs and failure budgets (5xx + 429/503).

---

## 2) Benchmarks & Harness

**Micro-bench (Criterion)**
- Hot paths: request parsing/validation, caveat evaluation, macaroon/passport encoding, KMS client sign call, JSON (serde) encode.
- Location: `benches/` with Criterion; commit plots as PR artifacts.

**Integration Load**
- `testing/performance/` rigs (wrk or bombardier) driving:
  - `/v1/passport/issue` and `/v1/passport/verify`
  - Mixed profile (e.g., 80/20 issue/verify)
  - Spike tests to confirm controlled 429s instead of tail-latency collapse.

**Profiling**
- `cargo flamegraph` (CPU hotspots)
- `tokio-console` (async stalls, contention)
- `perf` or `coz` (causal profiling)
- `hyperfine` for CLI helpers (if any) and cold-start timing

**Chaos/Perf Blend**
- Latency injection on KMS/Registry client.
- Slow-loris at ingress—expect early 429 under configured caps.
- Compression/bomb inputs should be rejected upstream (body cap).

**CI Integration**
- Nightly perf runs comparing to JSON baselines stored in `testing/performance/baselines/`.  
- Publish Grafana snapshots and flamegraphs as build artifacts.

---

## 3) Scaling Knobs

**Concurrency**
- Tower concurrency limit for `/issue`.
- Semaphore around KMS sign (prevent stampede).
- Consider separate runtimes or pools for `verify` vs `issue` to avoid head-of-line blocking.

**Memory**
- Prefer streaming + `bytes::Bytes`.
- Avoid large intermediates; hard-cap request body at **1 MiB**.

**I/O**
- Use UDS for local `ron-kms`/`ron-auth` where applicable.
- Respect read/write/idle timeouts; align with `ron-transport` defaults.

**Horizontal**
- Stateless service; scale replicas behind gateway when `inflight_requests` approaches capacity or headroom policy is violated.

**Vertical**
- More cores benefit JSON + signature encode/verify; pin crypto pools if introduced.

**Policy Toggles**
- `limits.rps`, `limits.body_max_bytes`, `amnesia`, `pq.mode` ∈ { `mirror`, `require`, `enforce` }.

**Verify Path Cache (optional)**
- TTL: **1–3 s**; target hit-rate **≥ 70%** steady-state.
- Invalidate on revocation event or epoch bump.
- Cache key includes passport hash + policy epoch.

**KMS Client Backoff**
- Jittered exponential: base **25 ms**, max **250 ms**, ≤ **3** attempts per call.
- Per-endpoint circuit opens at ≥ **50%** errors over **30 s** window.

**Headroom Policy**
- Add a replica when sustained CPU > **70%** for **15 m** or when p95 issuance latency breaches SLO for **3** consecutive windows.

---

## 4) Bottlenecks & Known Limits

| Scenario                                | Bottleneck                      | Mitigation / Knob                                           |
|-----------------------------------------|---------------------------------|--------------------------------------------------------------|
| Many caveats / large payload (≥ 64)     | serde encode/decode CPU         | Cap caveats; stream parse; benchmark size ladder             |
| KMS latency spike                       | External RTT + retries          | Tighten semaphore; reduce attempts; tune circuit thresholds  |
| Inter-region verify                     | WAN RTT                         | Prefer regional endpoint; enable short-TTL verify cache      |
| Spiky ingress                           | Concurrency/queue caps          | Pre-emptive 429; autoscale via headroom policy               |
| Long-run memory creep                   | Allocations on hot path         | Pool `Bytes`; borrow JSON; 24h soak slope < **2%**           |
| PQ hybrid enabled                       | Extra CPU during issuance       | `pq.mode="mirror"` to measure; scale cores before `enforce`  |

---

## 5) Regression Gates (CI must fail if…)

- **Issuance**: p95 latency ↑ > **10%** vs baseline **or** p99 > **100 ms** sustained 5 m.  
- **Verify**: p95 latency ↑ > **10%** **or** p99 > **25 ms** sustained 5 m.  
- **Throughput** (4c/8GB reference): ↓ > **10%** from baseline at fixed test profile.  
- **CPU/Memory**: regress > **15%** at target RPS.  
- **Soak**: 24h RSS slope must remain **< 2%/24h** at baseline load.  
- **Cold-start**: p95 ↑ > **20%** vs baseline fails.  
- **Verify-cache (if enabled)**: hit-rate ↓ > **10pp** vs baseline fails.

Baselines live in `testing/performance/baselines/`.  
Waivers are allowed only if traced to an upstream dependency **and** SLOs remain met; document rationale in the PR.

---

## 6) Perf Runbook (Triage)

1. **Confirm symptom on dashboards**: p95/p99 issuance & verify, 429/503, burn-rate (2h/24h).  
2. **Flamegraph** the hot path: look for KMS sign, serde hotspots, or policy evaluation.  
3. **tokio-console**: identify task stalls, oversized critical sections, or semaphore contention.  
4. **Knob sweep**:  
   - Lower issuance concurrency by **25%** and re-test.  
   - Tune KMS retries/backoff; verify circuit state.  
   - Validate `limits.rps` vs node size.  
5. **Chaos isolate**: inject latency on KMS/Registry; verify alert firing and `/readyz` behavior (issuance should degrade/fail-closed).  
6. **Scale**: if inflight > **80%** or sustained 429s, add replicas; confirm recovery.  
7. **Verify path issues**: if verify p95 breaches with normal CPU, check verify-cache hit-rate. If < **70%**, inspect revocation churn/epoch bumps causing cache flushes.  
8. **Revocation lag**: if p99 > **5 s**, check bus delivery, registry responsiveness, and apply epoch bump replay if required.  
9. **Leak suspicion**: run 24h soak with allocation tracking; inspect RSS slope and object lifetime via `heaptrack`/`pprof-rs` if available.

---

## 7) Acceptance Checklist (DoD)

- [ ] SLOs wired into alerts (`PassportIssueLatencyHigh`, `RevocationPropagationSlow`, `BackpressureSustained`, burn-rate fast/slow).  
- [ ] Perf harness (`criterion` + load rigs) runs locally & in nightly CI; baselines stored.  
- [ ] One set of **flamegraph** + **tokio-console** traces attached to PR artifacts.  
- [ ] Scaling knobs documented in `CONFIG.md` and validated under load.  
- [ ] Regression gates enforced in CI; waiver policy documented in CONTRIBUTING.  
- [ ] Runbook anchors referenced from alert annotations.

---

## 8) Appendix

**Baseline JSON Schema (stored in `testing/performance/baselines/issuance-4c8g.json`)**
```json
{
  "node_size": "4c/8g",
  "issuance": { "p50_ms": 18, "p95_ms": 40, "p99_ms": 100, "throughput_rps": 500, "cpu_pct": 60, "rss_mib": 400 },
  "verify":   { "p50_ms": 4,  "p95_ms": 10, "p99_ms": 25,  "throughput_rps": 2000 },
  "cold_start": { "p95_ms": 150 },
  "burn_rate": { "fast_2h_x": 14, "slow_24h_x": 6 }
}
````

**Perf Matrix (minimum)**

* Node sizes: **2c/4g**, **4c/8g (reference)**, **8c/16g**
* Modes: `pq.mode` ∈ { **mirror**, **require** }, verify-cache ∈ { **on**, **off** }
* Regions: **intra-region**, **inter-region**

**Example CI perf gate (pseudo-YAML)**

```yaml
perf_gates:
  - name: issuance_p95
    compare: "histogram_quantile(0.95, sum(rate(svc_passport_issue_latency_seconds_bucket[5m])) by (le))"
    op: "<="
    threshold_delta_pct: 10
  - name: verify_p95
    compare: "histogram_quantile(0.95, sum(rate(svc_passport_verify_latency_seconds_bucket[5m])) by (le))"
    op: "<="
    threshold_delta_pct: 10
  - name: soak_rss_slope
    compare: "deriv(process_resident_memory_bytes[24h])"
    op: "<="
    threshold_abs: 0.02
  - name: cold_start_p95
    compare: "svc_passport_cold_start_ms_p95"
    op: "<="
    threshold_delta_pct: 20
```

**History**

* Use this section to record regressions/fixes and link PRs to build institutional memory.

---

```
```
