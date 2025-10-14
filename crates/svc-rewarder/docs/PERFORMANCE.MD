

```markdown
---
title: Performance & Scaling — svc-rewarder
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-14
audience: contributors, ops, perf testers
review-cadence: quarterly or on SemVer major/minor
governance: perf audit sign-off tracked in GOVERNANCE.md (Perf Board)
---

# PERFORMANCE.md — `svc-rewarder`

## 0) Purpose

This defines the **performance contract** for `svc-rewarder`:
- SLOs (latency/availability/correctness), throughput targets, error budgets, and dependency budgets.
- Benchmarks & workloads (micro + integration), profiling, chaos blends, and **mobile cold-start** checks.
- Scaling knobs **with defaults & env mapping** (incl. PQ/hybrid budget).
- Known bottlenecks and CI **regression gates**.
- Ops **runbook + alert routing**.

**Review & baselines.** Reviewed **quarterly** or on **SemVer major/minor** bumps. Baselines refresh when **material changes** occur:
- Input cardinality shift **>10%** (actors/events),
- policy weight/algorithm change,
- enabling/disabling **PQ/hybrid** or parameter updates,
- dependency SLO contract update (e.g., ledger p99 target).

All refreshes are logged in `CHANGELOG.md` and signed off in `GOVERNANCE.md`.

---

## 1) SLOs / Targets

### 1.1 Service SLOs

| Domain | Metric | Target | Scope / Exclusions | Rationale |
|---|---|---:|---|---|
| Compute latency | `reward_compute_latency_seconds` p95 | < **500 ms** | intra-region; excludes **declared** ledger brownouts | CPU-bound deterministic math |
| Compute latency | `…` p99 | < **2 s** | same | Headroom for L-scale epochs |
| Availability | `/readyz` SLI | ≥ **99.9%** / month | excludes planned maint; excludes **declared** brownouts | Avoid LB thrash; protect budget |
| Correctness | `reward_invariant_fail_total` | **0** / day | — | Economic safety invariant |
| Idempotency | `ledger_intents_total{result="dup"}/total` | ≤ **1%** | excludes replay tests | Retries OK; double-emits not |
| Error budget | 5xx rate | < **0.1%** | compute + egress | Real defects, not overload |
| Quota | 429/503 | < **1%** sustained | overload must be graceful | Backpressure functions |

**PQ/hybrid budget.** When `REQUIRE_PQ_ON=true` (e.g., Dilithium/ML-DSA signing of results/intents), the **additional p95** latency must be **≤ +5%** of baseline; CPU headroom must remain under ceilings (§1.3). This delta is measured per release (§7).

### 1.2 Dependency SLOs (assumptions → behavior)

| Dependency | Budget (p95/p99) | On Violation (fallback behavior) |
|---|---|---|
| Ledger egress | **<100 ms / <250 ms** | Enter **quarantine-only** (enqueue but **do not emit**); return 202 Accepted (quarantine=true); raise `LedgerEgressDegraded` alert; backoff retries; exit when 10-min p95 < budget |
| Auth/cap cache | **<5 ms / <15 ms** | Use local capability cache; if miss + upstream > budget, 503 with `cap_timeout=true` and circuit break per-tenant |
| Tmpfs writes (amnesia) | **<3 ms / <10 ms** | Spill minimal artifacts to memory buffers; skip non-critical traces; log `amnesia_spill_total` |

### 1.3 Throughput & Ceilings (per node)

- **Micronode:** steady **50–100 runs/s**; burst **200 runs/s** ≤ 60s with graceful shedding.  
- **Macronode:** steady **300–500 runs/s** at target ceilings; horizontal scale via scheduler.

**Ceilings at target load**
- **CPU:** mean < **70%**/core (spikes ≤85% <60s).
- **Memory (RSS):** steady < **1.5 GiB**.
- **FDs:** < **30%** of limit.
- **Queues:** alert before saturation (thresholds §6).

### 1.4 Edge / Mobile (Micronode)

- **Cold start:** `/readyz=200` < **2 s** on laptop-class or Android-like sim (2 vCPU / 2 GiB RAM).
- **Amnesia:** no persist after run (tmpfs). Verified in soak.

### 1.5 Error-budget burn-rate policy

- **Page:** 15-min burn-rate **>2×**, or 1-h **>1×** monthly budget.  
- **Ticket:** 24-h **>0.5×**.

---

## 2) Benchmarks & Harness

### 2.1 Micro-bench (pure calculus)

- **Criterion** benches on hot loops (weighting/aggregation). Track **ops/sec**, **allocs/op**, **peak RSS** (dhat/heaptrack).
- Save baselines:
```

cargo bench --bench reward_calc -- --save-baseline main

```

**Reference scales (sealed golden vectors)**
- **S:** 1k actors / 10k events
- **M:** 10k actors / 100k events
- **L:** 50k actors / 500k events

### 2.2 Integration load (HTTP)

**Steady**
```

bombardier -c 128 -d 60s -m POST -f testing/performance/payloads/M.json 
[http://localhost:8080/rewarder/epochs/demo/compute](http://localhost:8080/rewarder/epochs/demo/compute)

```

**Burst (2× steady)**
```

bombardier -c 256 -d 60s -m POST -f testing/performance/payloads/M.json 
[http://localhost:8080/rewarder/epochs/demo/compute](http://localhost:8080/rewarder/epochs/demo/compute)

````

**Dependency slow (ledger 50–500 ms jitter)**  
Run with fault injection (see RUNBOOK) or egress stub; verify `ledger_intents_total{result}` mix and **idempotency ≤1%**.

### 2.3 Profiling

- `cargo flamegraph` (attach SVG for regressions).  
- `tokio-console` (stalls/blocked I/O).  
- `perf` / `coz` (optional causal profiling).

### 2.4 Chaos / soak / fuzz→perf

- **Chaos:** latency/jitter on egress; **slow-loris** on HTTP; reject **size/compression bombs** (≤1 MiB, ≤10× ratio).
- **Soak:** 24h steady **S** profile; assert no p95 drift and no RSS growth.
- **Fuzz→perf bridge:** feed corpora from `testing/fuzz/corpora/reward_inputs/` into integration runner to profile adversarial yet valid inputs; gate by ensuring p95 ≤ 1.2× of nominal for same scale.

### 2.5 Mobile cold-start harness

- Run container or emulator with **2 vCPU / 2 GiB RAM**; assert `/readyz` < **2 s** and first compute p95 < **750 ms** for **S** vector.

### 2.6 CI wiring

- Nightly perf compares to baselines in `testing/performance/baselines/` and **fails** on §5 gates.
- `promtool check rules` validates required metrics/alerts presence.

---

## 3) Scaling Knobs (defaults & env)

| Knob | Env | Default | Notes |
|---|---|---:|---|
| Compute workers | `CONCURRENCY_COMPUTE_WORKERS` | `num_cpus::get().saturating_sub(1)` | Heavy phases use `spawn_blocking` |
| Max inflight compute | `MAX_INFLIGHT` | `512` | Deterministic 429/503 beyond cap |
| Per-tenant RPS clamp | `TENANT_RPS_MAX` | `100` | Token bucket per tenant |
| Work queue bound | `WORK_QUEUE_MAX` | `4096` | Reject > cap; emit `rejected_total{reason}` |
| Ledger retries | `LEDGER_MAX_RETRIES` | `2` | Exponential backoff; jitter |
| Ledger deadline | `LEDGER_DEADLINE_MS` | `1000` | Aligns with dependency budget |
| Body size cap | `REQ_BODY_MAX_BYTES` | `1_048_576` | 1 MiB |
| Decompress ratio cap | `DECOMP_RATIO_MAX` | `10` | Drop + metric on exceed |
| PQ requirement | `REQUIRE_PQ_ON` | `false` | If **true**, see PQ budget & **reduce workers by 1** |
| Hot-reload (RPS clamp) | `SIGHUP`/`SIGUSR1` | — | Re-reads `TENANT_RPS_MAX` & queue caps without restart |

**Profiles**
- **Micronode:** smaller queues, tmpfs artifacts, faster start.
- **Macronode:** horizontal scale; LB eject on `/readyz` degraded > **15s**.

---

## 4) Bottlenecks & Known Limits

**Heatmap (typical)**

| Area | S | M | L | Approx. CPU share (M) | Mitigation |
|---|:--:|:--:|:--:|:--:|---|
| Calc hot loop | ● | ●● | ●●● | ~60% | Precompute constants; tighten inner loop; avoid heap churn |
| Serialization | ○ | ● | ●● | ~20% | `bytes::Bytes`; buffer reuse; streaming |
| Ledger egress | ○ | ● | ●● | ~10% | Deadlines + retries; classify `accepted|dup|error`; quarantine on repeat |
| Queue pressure | ○ | ● | ●● | n/a | Per-tenant clamps; early 429; observe `queue_depth` |

Legend: ○ low, ● medium, ●● high, ●●● very high

**Bronze → Gold**
- **Bronze:** SLOs green single-AZ.
- **Silver:** Burst + chaos green; dependency jitter tolerated with correct fallbacks.
- **Gold:** Horizontal scale under ledger brownouts; alerts + runbook validated by drill.

---

## 5) Regression Gates (CI must fail if…)

- p95 compute latency **↑ >10%** vs baseline (same scale/profile).
- Throughput at target load **↓ >10%**.
- Mean CPU or steady RSS **↑ >15%**.
- Required metrics missing / duplicated registration.
- **SemVer note:** major versions may accept temporary **+20%** CPU/RSS if feature-driven **and** a tracking issue + rollback plan exist (attach flamegraph + console traces).  
- **API coupling:** perf-impacting API changes (per `cargo public-api`) **must** refresh baselines and include a waiver note.

---

## 6) Perf Runbook (alerts, routing, triage)

**Alert routing**
- **Pages:** PagerDuty service `svc-rewarder`; **secondary**: Observability on-call.
- **Tickets:** Jira project `RON`, type `Incident` or `Perf Regression`.

**Page thresholds**
- Burn-rate (§1.5) violation.
- `reward_compute_latency_seconds` p95 > **1 s** for **>5 min**.
- `ledger_egress_errors_total` > **5/min** for **5 min** (or p99 > **250 ms** for **10 min**).

**Ticket thresholds**
- `429_total` sustained > **1%** for **15 min**.
- `queue_depth` > **80%** of `WORK_QUEUE_MAX` for **10 min**.

**Triage steps**
1. **Flamegraph**: identify hot frames (calc vs serialization vs egress).
2. **tokio-console**: stalls/blocked tasks/oversized critical sections.
3. **Dashboards**: p95/p99, `/readyz`, `rejected_total{reason}`, `ledger_intents_total{result}`, `queue_depth`.
4. **Knobs**: reduce per-tenant RPS; adjust `COMPUTE_WORKERS`; tune queue caps; observe CPU.
5. **Chaos toggles**: quarantine-only; raise tracing sample.
6. **Edge path**: verify Micronode amnesia; tmpfs behavior under load.
7. **Escalate**: any invariant failure → quarantine, diff against golden vectors, follow incident RUNBOOK.

---

## 7) Acceptance Checklist (DoD)

- [ ] SLOs wired to metrics/alerts (p95/p99/availability/error budget).
- [ ] Baselines created for **S/M/L** and stored in `testing/performance/baselines/`.
- [ ] Flamegraph & tokio-console traces captured ≥ once per release.
- [ ] Scaling knobs documented **with defaults** and envs; hot-reload validated.
- [ ] Regression gates enforced in CI; `promtool` rules pass.
- [ ] Chaos + 24h soak completed without drift/leaks.
- [ ] **PQ/hybrid delta** measured: p95 increase **≤ 5%**; worker reduction adjusted if needed.
- [ ] **Mobile cold-start** validated: `/readyz` < **2 s**; first compute p95 < **750 ms** (S).

---

## 8) Appendix

### 8.1 Mermaid — perf flow

```mermaid
sequenceDiagram
  participant C as Client
  participant R as svc-rewarder
  participant L as Ledger
  C->>R: POST /epochs/{id}/compute (sealed inputs)
  activate R
  R->>R: Validate + deserialize (bounded)
  R->>R: Reward calculus (CPU-bound, worker pool)
  R->>L: Emit intent (idempotency key)
  L-->>R: accepted | dup | error
  R-->>C: 200 OK (result) | 202 (quarantine) | 429/503 (graceful)
  deactivate R
````

### 8.2 Baseline artifact format (example)

```json
{
  "profile": "macronode",
  "scale": "M",
  "timestamp": "2025-10-14T18:30:00Z",
  "targets": { "p95_ms": 500, "p99_ms": 2000, "rps": 350 },
  "observed": { "p95_ms": 472, "p99_ms": 1630, "rps": 368, "cpu_mean_pct": 63, "rss_mb": 980, "pq_on": false }
}
```

### 8.3 Minimal request payload (sealed inputs)

```json
{
  "inputs_cid": "b3:7d9c...c1",
  "policy_id": "rev42",
  "policy_hash": "b3:3aa1...f83",
  "dry_run": false
}
```

### 8.4 `wrk` Lua (latency + success capture)

```lua
-- testing/performance/scripts/compute.lua
wrk.method = "POST"
wrk.path = "/rewarder/epochs/demo/compute"
wrk.body = io.open("testing/performance/payloads/M.json", "r"):read("*a")
wrk.headers["Content-Type"] = "application/json"
response = function(status, headers, body)
  -- Count 200/202 as success, 429/503 as graceful overload
end
```

### 8.5 Reference workloads & corpora

* Sealed vectors: `testing/performance/vectors/{S,M,L}.json`
* Fuzz corpora: `testing/fuzz/corpora/reward_inputs/`
* Scenarios: `testing/performance/scenarios/{steady,burst,brownout}.md`

### 8.6 Perfection Gates

* **Gate F:** perf regressions barred via §5.
* **Gate L:** scaling validated under chaos & brownouts.

```

---

