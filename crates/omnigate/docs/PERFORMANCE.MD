

---

# ⚡ PERFORMANCE.md — omnigate

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-07
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0) Purpose

This document defines Omnigate’s **performance profile**:

* SLOs for latency/throughput, error budgets, and resource ceilings.
* Benchmarks & workloads Omnigate must sustain (local → regional → global).
* Perf harness & profiling tools, plus chaos blends.
* Scaling knobs (concurrency, buffers, streaming) and known bottlenecks.
* Regression gates to block silent performance drift in CI.

Ties into:

* Scaling Blueprint (ingress SLOs, runbooks, profiles).
* Omnigate Blueprint (hydration SLOs, streaming bounds, amnesia).
* Six Concerns (PERF mapping, CI labels).
* Concurrency & Aliasing (zero-copy, bounded backpressure).
* Hardening (ingress limits, decompression guards).

---

## 1. SLOs / Targets

> Omnigate sits behind the gateway and hydrates views by orchestrating index, storage, and mailbox. It is **stateless** beyond counters/caches and honors OAP/1 bounds: `max_frame = 1 MiB`, streaming chunks ≈ **64 KiB**.

### 1.1 Latency

* **Hydration (composed view, no heavy mods/ranking)**

  * **p95 ≤ 150 ms** intra-AZ (micronode/macronode).
* **Media byte-range “range start”**

  * **p95 < 100 ms** intra-AZ.
* **Public GET via gateway**

  * **p95 < 80 ms** intra-region; **p95 < 200 ms** inter-region.
* **Mailbox fan-out read (moderate)**

  * **p95 < 200 ms**.

### 1.2 Throughput (per node, qualified)

* Sustain **≥ 2,000 req/s** for a **read-heavy hydration mix** on a **16-core AMD64 baseline (3.0+ GHz), intra-AZ topology**, while meeting the above p95 targets and error budgets.
* Graceful backpressure (shed before collapse) when exceeding configured quotas.

### 1.3 Error Budget

* **5xx (excluding 429/503 shed)**: < **0.1%**.
* **429/503 (quota/readiness shed)**: < **1%** steady-state; burst-safe under DRR.
* **Bus overflow**: < **0.01%** (watch `bus_overflow_dropped_total`).

### 1.4 Resource Ceilings (at target load)

* **CPU**: < **70%** per core average (**p95 < 90%**) to preserve headroom.
* **Memory**: < **1.5 GiB** RSS steady-state (service process, excl. OS cache).
* **FDs**: < **50%** of system limit; zero FD leaks over 24h soak.
* **Amnesia (Micronode)**: RAM-only caches; zero on-disk artifacts.

### 1.5 Ingress Limits (hardening defaults)

* **Timeout:** 5s
* **Inflight cap:** 512
* **RPS cap:** 500 per instance (tunable)
* **Body cap:** 1 MiB
* **Decompression guard:** ≤10× with absolute cap
* Always **stream over buffer** for large payloads.

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (in-repo, `cargo bench`)

* **Criterion** benches for hot hydration paths:

  * DTO parse/serialize (protocol types).
  * Envelope routing + capability/policy check (no network).
  * Join-planner & response shaping (own `bytes::Bytes`, avoid copies).
* Track **allocations/op** (`heaptrack`/Massif) and copies avoided.

### 2.2 Integration load tests (`testing/performance/*`)

* **wrk/bombardier** via the **gateway** against:

  * `/v1/view/*` (hydration mix: ~80% GETs, 20% with mailbox peek).
  * `/v1/media/*` (range start & sustained streaming, 64 KiB chunks).
* **gwsmoke-style** suites for GET/HEAD/RANGE.
* **Soak**: 24h continuous with zero FD leaks and stable RSS.

### 2.3 Profiling

* `cargo flamegraph` (user+kernel) to locate hotspots (policy eval, serde, join shaping, TLS if present).
* `tokio-console` for stalls/backpressure and semaphore contention.
* `perf` / `coz` for causal profiling on long tails.
* **PQ-hybrid profiling (pilot)**: toggle **Kyber+X25519** handshakes in transport and measure inter-region handshake deltas under load (baseline retained; pilot off by default).
* **ZK/Econ profiling (pilot)**: measure ledger commitment/verify overhead for econ-hydrated views (phased; off by default).

### 2.4 Chaos/perf blend

* Latency injection on downstream calls.
* Slow-loris and decompression-bomb exercises (expect 413/429/503).
* Packet loss/jitter simulation for inter-region paths.

### 2.5 CI Integration

* Nightly perf runs vs baselines; **flamegraph artifacts** uploaded on threshold breach.
* Soak subset (≥1h) per PR label `perf:rapid`; full 24h soak on `main` weekly.

---

## 3. Scaling Knobs

All knobs are config-tunable (env or config file). Suggested defaults in parentheses.

* **Concurrency & Backpressure**

  * Global inflight cap (env `OMNIGATE_INFLIGHT_MAX`, default **512**).
  * Per-downstream semaphores: `index`, `storage`, `mailbox` (e.g., **128/128/64**).
  * Optional **hedged reads** (read paths only) with budget ceiling (off by default).
* **Streaming & Memory**

  * Streaming chunk size (**64 KiB** typical; allow **32–128 KiB** experimentation).
  * Prefer **`bytes::Bytes`**; avoid full-buffer hydration; zero-copy where possible.
  * Allocator pin (mimalloc/jemalloc) only with evidence.
* **I/O**

  * Prefer **range requests**; respect/emit ETags (`b3:<hex>`).
  * Keep-alive tuning via gateway; Omnigate avoids socket fan-out per request.
* **Horizontal**

  * Stateless: scale replicas behind gateway; DRR quotas at ingress.
* **Vertical**

  * CPU pools only if profiled hotspots justify (hash/serde).
* **Micronode/Edge**

  * **Amnesia ON** (RAM-only), reduced chunk/queue sizes, feature-gate heavy facets.

---

## 4. Bottlenecks & Known Limits

* **Policy & Capability checks**: macaroons/policy eval on every request; use short-TTL cache per request context.
* **Downstream latency**: index/storage dominate p95; keep semaphores tight; prefer range+cache.
* **TLS handshake**: terminated at gateway; handshake storms appear as queueing → verify DRR shed.
* **Decompression bombs / oversized bodies**: rejected by ingress caps; Omnigate must stream joins and never over-buffer.
* **ZK/Econ hydration (pilot)**: phased commitments can add ~**50 ms p95**; keep off by default and benchmark when enabled.
* **DHT/discovery**: explicitly **not** on the BFF hot path.

**Bronze → Gold progression**

* Bronze: hydration basics with quotas & metrics.
* Silver: tuned range path + mailbox fan-out.
* Gold: multi-tenant load + sandboxed mods (macronode), chaos-validated.

---

## 5. Regression Gates

CI fails if any breach vs last green baseline (same request mix, quotas, topology):

* **Latency**: p95 hydration or range-start **↑ > 10%**; public GET intra/inter-region miss targets.
* **Throughput**: **↓ > 10%** at fixed error budget.
* **CPU/Memory**: **↑ > 15%** at same RPS.
* **Shed behavior**: 429/503 increase **> 0.5 pp** without a quota config change.
* **Facet-specific gates**: if PR labeled `facet:media` or `facet:feed`, any breach on that facet’s SLO **fails CI**.

Baselines live in `testing/performance/baselines/` (JSON + charts). Updates require:

1. reviewer sign-off (PERF owner + OPO), 2) attached flamegraphs, 3) short ADR noting reason.
   Escape hatch: allowed if traced to upstream dep **and** SLOs still met; must include waiver ADR.

---

## 6. Perf Runbook (Triage)

1. **Readiness & shed**: check `/readyz` (writes flip first), gateway DRR, inflight gauges.
2. **Flamegraph now**: identify hot spots (policy, serde, join shaping).
3. **`tokio-console`**: look for stalls, long polls, saturated semaphores per downstream.
4. **Metrics scan**: `*_latency_seconds`, `rejected_total{reason}`, `bus_overflow_dropped_total`.
5. **Knob sweep**: adjust per-downstream semaphores, streaming chunk (±64 KiB), hedging budget; re-run load.
6. **Chaos isolate**: disable compression/jitter, then re-enable gradually.
7. **PQ sweep (pilot)**: toggle Kyber+X25519; capture new flamegraph & handshake metrics.
8. **Amnesia proof**: for micronode, verify zero disk writes (audit `/proc`/fs watchers) during soak.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs encoded for hydration, range start, and public GET (intra/inter-region).
* [ ] Bench harnesses (Criterion + wrk/bombardier + soak) run locally & in CI.
* [ ] Flamegraph and tokio-console traces captured at least once per milestone.
* [ ] Scaling knobs documented and wired (semaphores, streaming chunks, hedging).
* [ ] Regression gates + baselines + artifact uploads active in CI (facet labels honored).
* [ ] Amnesia soak test enforces zero disk artifacts (micronode).
* [ ] PQ/ZK pilot benches exist (gated, off by default) with clear toggle/metrics.
* [ ] Perf runbook updated after incidents or waivers (ADR linked).

---

## 8. Appendix

### 8.1 Reference SLOs

* Public GET **p95 < 80 ms** intra-region; **< 200 ms** inter-region; 5xx < **0.1%**; 429/503 < **1%**.
* Range start **p95 < 100 ms**; **64 KiB** stream chunks; OAP frame cap **1 MiB**.
* Hydration **p95 ≤ 150 ms**; mailbox fan-out **p95 < 200 ms**.

### 8.2 Reference workloads

* `gwsmoke` GET/HEAD/RANGE suites.
* 24h soak on hydration + mailbox mixes (no FD leaks; stable RSS).

### 8.3 Mermaid — Hydration Path with Backpressure

```mermaid
flowchart LR
  C[Client] --> G[Gateway<br/>(TLS, quotas, DRR)]
  G --> O[Omnigate<br/>(stateless, streaming)]
  O --> I[Index]
  O --> S[Storage]
  O --> M[Mailbox]
  O -->|Backpressure| Shed[429/503 Shed]
  classDef core fill:#0b7285,color:#fff,stroke:#084c61,stroke-width:1px
  class O core
```

### 8.4 Dependency pins (perf-relevant)

* tokio 1.x (workspace-pinned), axum 0.7.x, tower-http 0.6.x, prometheus 0.14.x, tokio-rustls 0.26.x.
* Prefer rustls; native-tls only by exception with ADR.

### 8.5 History

* Record regressions, root causes, knobs changed (before/after flames, metric links, ADR IDs).

---

### Notes

* OAP constants and content-addressing rules (BLAKE3, `b3:<hex>`) are **normative** and must not drift.
* Never hold locks across `.await`; prefer owned `Bytes` on hot paths; ensure one writer/socket and bounded queues.

---
