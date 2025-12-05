```md
---
title: Performance & Scaling — svc-admin
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-12-04
audience: contributors, ops, perf testers
---

# PERFORMANCE.md — svc-admin

## 0. Purpose

This document defines the **performance profile** of `svc-admin`:

- Service-level objectives (SLOs) for the operator UI and admin API.
- Expected workloads and scaling envelope (nodes × operators).
- Perf harness and profiling tools.
- Scaling knobs and known bottlenecks (metrics fanout, auth, JSON mapping).
- Regression gates to prevent silent perf drift.

It ties into:

- **SCALING blueprint** (planes, readiness, golden metrics).  
- **Hardening blueprint v2.0** (HTTP timeouts, caps, concurrency).:contentReference[oaicite:1]{index=1}  
- **svc-admin CONCURRENCY.md** (bounded channels, timeouts, cooperative shutdown).:contentReference[oaicite:2]{index=2}  

`svc-admin` is an **operator-facing admin GUI & proxy**, not a data plane. It is expected to be **lightweight but responsive** under:

- 1–20 active operators per instance.
- 1–100 nodes per instance (configurable), with short-horizon metrics buffers only.:contentReference[oaicite:3]{index=3}  

---

## 1. SLOs / Targets

### 1.1 Service Role

`svc-admin`:

- serves a React/TypeScript SPA (static assets),:contentReference[oaicite:4]{index=4}  
- exposes a JSON API for node status & metrics,
- fans out to nodes’ `/healthz`, `/readyz`, `/api/v1/status`, `/metrics`,:contentReference[oaicite:5]{index=5}  
- optionally validates operator identity via ingress/passport.  

It is **read-mostly** and **control-plane only**. SLOs are tuned for *human operators*, not bulk programmatic traffic.

### 1.2 Latency Targets (steady state)

All numbers are **per svc-admin instance**, in a healthy cluster, intra-region.

**Local-only endpoints** (no node fanout):

- `GET /`, `GET /assets/*`, `GET /api/ui-config`, `GET /api/me`, `GET /api/nodes`:
  - p50: ≤ 10 ms
  - p95: ≤ 30 ms
  - p99: ≤ 60 ms

These are dominated by:

- config lookups,
- small JSON serialization,
- (optionally) local token parsing for `/api/me` in passport mode.

**Node-backed endpoints** (single-node fanout):

- `GET /api/nodes/{id}/status`
- `GET /api/nodes/{id}/metrics/summary`
- `GET /api/nodes/{id}/metrics/facets`

Targets (assuming node admin APIs are healthy and within Hardening HTTP limits):  

- p50: ≤ 80 ms
- p95: ≤ 400 ms
- p99: ≤ 1000 ms

If nodes are slow or unreachable, `svc-admin` **fails fast with typed errors** and surfaces partial data, rather than burning the entire error budget.:contentReference[oaicite:8]{index=8}  

### 1.3 Throughput Targets

Admin traffic is expected to be low compared to data plane, but SLOs should support:

- **Target throughput**:  
  - ≥ 200 requests/s sustained (mixed UI + JSON) per instance.
  - Peak bursts up to ~500 requests/s (aligned with global Hardening RPS cap).:contentReference[oaicite:9]{index=9}  

- **Fanout**:
  - For a single operator dashboard view on N nodes, svc-admin should tolerate:
    - N = 50 nodes,
    - metrics sampling interval = 15–30s,
    - without saturating CPU or blowing timeouts.

### 1.4 Error Budget & Resource Ceilings

**Error budget:**

- 5xx (server) error rate: < 0.1% of requests.
- 429/503 (backpressure / not ready) from svc-admin: < 1% over 5-minute windows.:contentReference[oaicite:10]{index=10}  
- Upstream errors (node unavailable, node 5xx) are **counted separately** and surfaced in UI, but they do not count against svc-admin’s *own* error budget.

**Resource ceilings** (guidance per instance, at target load):

- CPU:
  - svc-admin itself: ≤ 1 core equivalent at 200 req/s.
- Memory:
  - steady state: ≤ 512 MiB (including TLS, metrics buffers, and SPA assets).
- File descriptors:
  - target: < 50% of system limit.
  - under Hardening defaults: ≤ 512 concurrent connections.:contentReference[oaicite:11]{index=11}  

No explicit **edge/mobile SLOs** apply to the Rust service; those apply to the SPA bundle separately (handled in front-end perf budgets).

---

## 2. Benchmarks & Harness

### 2.1 Micro-benchmarks

Micro-benchmarks should focus on **CPU-heavy and allocation-heavy** paths:

- JSON mapping:
  - Node `/api/v1/status` → `AdminStatusView` DTO.:contentReference[oaicite:12]{index=12}  
- Metrics parsing:
  - `/metrics` text → summarized metrics and facets structures.
- Auth decoding (optional):
  - passport JWT verification (if done locally),
  - ingress header parsing (string → roles).

**Tooling:**

- `criterion`-based benches under `benches/`:
  - `status_mapping_bench.rs`
  - `metrics_parse_bench.rs`
  - `auth_decode_bench.rs` (if auth work is local)

Benchmarks must run under `cargo bench -p svc-admin --features benches` to avoid leaking bench-only symbols into prod builds (see global notes on gating bench features).:contentReference[oaicite:13]{index=13}  

### 2.2 Integration Load Tests

Create `testing/performance/svc-admin/` with scripts and configs:

- **HTTP load:**

  - `svc_admin_smoke.sh`:

    - Launch svc-admin with a small `svc-admin.toml` (3–5 nodes).
    - Use `wrk` or `bombardier` to drive:

      - `GET /api/ui-config`
      - `GET /api/me`
      - `GET /api/nodes`
      - `GET /api/nodes/{id}/status`
      - `GET /api/nodes/{id}/metrics/summary`

    - Measure p50/p95/p99 latency and RPS.

- **Fanout / node scaling test:**

  - `svc_admin_fanout.sh`:

    - Run svc-admin against a **fake-node** implementation that:
      - serves synthetic `/readyz`, `/api/v1/status`, `/metrics` at predictable latency.
    - Scale node count (`N = 10, 25, 50, 100`) and sampling interval (`T = 10, 15, 30s`).
    - Validate that:
      - CPU remains below threshold,
      - metrics polling tasks remain within timeouts,
      - svc-admin remains ready (`/readyz` = 200).:contentReference[oaicite:14]{index=14}  

### 2.3 Profiling

Recommended tools:

- `cargo flamegraph`:
  - Identify hotspots in:
    - metrics parsing,
    - JSON mapping,
    - TLS handshake overhead (if not offloaded).:contentReference[oaicite:15]{index=15}  
- `tokio-console`:
  - Observe async stalls and long tasks:
    - metrics sampler tasks blocking,
    - auth/JWK refreshers,
    - config reload handling.:contentReference[oaicite:16]{index=16}  
- `hyperfine`:
  - CLI-level perf for one-off admin actions (e.g. CLI wrapper to hit `/api/nodes`).

### 2.4 Chaos/Perf Blend

Test **resilience under degraded conditions**:

- Node `/metrics` responses delayed near the timeout.
- Nodes returning large but bounded `/metrics` bodies (decompression guard + size caps).  
- Partial node outages (1/10 nodes slow or unreachable).

Goals:

- svc-admin **stays responsive** to other nodes.
- `/readyz` remains 200 unless svc-admin’s own invariants break (config, metrics sampler crash, auth misconfig).:contentReference[oaicite:18]{index=18}  

Nightly CI (or scheduled runs) should include at least one perf/chaos scenario.

---

## 3. Scaling Knobs

The main **scaling levers** for svc-admin are:

### 3.1 Concurrency & Server Stack

- **Axum/Hyper server settings**:

  - `max_conns` (global concurrency cap) — aligned with Hardening defaults (≤ 512).:contentReference[oaicite:19]{index=19}  
  - HTTP keep-alive & idle timeouts.

- **Tower layers:**

  - `TimeoutLayer` for per-request timeouts (inbound + outbound).
  - `ConcurrencyLimitLayer` or `RateLimitLayer` for backpressure (esp. node-fanout endpoints).  

### 3.2 Background Tasks

From `CONCURRENCY.md`:​:contentReference[oaicite:21]{index=21}  

- Metrics sampler:
  - one task per node group or per node (depending on design).
  - `poll_interval` and `timeout` are key knobs.
- Config reload:
  - single writer for config snapshot.
- Auth key/JWK refresh:
  - refresh interval and timeout; must not saturate CPU or block the runtime.

These tasks must use **bounded channels** and **cooperative cancellation** to avoid unbounded memory or stuck tasks.:contentReference[oaicite:22]{index=22}  

### 3.3 Memory & Buffering

Given anti-scope forbids TSDB, metrics retention is **short-horizon** and in-memory only.  

Knobs:

- Per-node metrics buffer length (e.g., last N points or last M minutes).
- Max metrics series tracked per node (e.g., limited set of golden metrics+facets).
- JSON response sizes for metrics summaries (avoid dumping raw `/metrics`).

### 3.4 Horizontal & Vertical Scaling

- **Horizontal scaling**:

  - svc-admin is stateless (beyond short in-memory buffers).
  - Multiple instances can be run behind an ingress/load balancer.
  - Operators can pin sessions to one instance for stable dashboards, but this is optional.

- **Vertical scaling**:

  - If metrics fanout is heavy (many nodes), scale up CPU and memory.
  - If parsing `/metrics` becomes expensive, consider:
    - reducing metrics sampling frequency,
    - limiting which metrics are parsed.

---

## 4. Bottlenecks & Known Limits

### 4.1 Hot Spots

Expected hot spots:

1. **Metrics parsing**:

   - Parsing Prometheus text for `/metrics` is CPU and allocation heavy.
   - This is acceptable at low frequency but can dominate CPU at high N×poll_rate.

2. **TLS handshakes**:

   - Cold connections to nodes or IdP can incur handshake cost; keep-alives help amortize.:contentReference[oaicite:24]{index=24}  

3. **Auth / JWT verification** (passport mode, if done locally):

   - Verifying JWTs per request can become significant if traffic rises; prefer:
     - header-based identity (`auth.mode="ingress"`) where appropriate, or
     - caching results (e.g., short-lived session tokens at ingress).

4. **JSON mapping & serialization**:

   - Node status JSON → strongly typed DTOs (status) → JSON for SPA.:contentReference[oaicite:25]{index=25}  

### 4.2 Acceptable vs Must-Fix

- **Acceptable**:

  - Node count up to ~50 with 15–30s metrics polling.
  - 1–5 operators actively using dashboards.
  - CPU spikes during config reloads or JWK refresh are short and rare.

- **Must-fix**:

  - svc-admin `/readyz` flipping frequently due to metrics sampler overload.
  - p95 latency for local-only endpoints (`/api/ui-config`, `/api/me`) creeping above ~50–80ms at normal operator load.
  - `/metrics` parsing causing sustained high CPU (>70–80%) at moderate node counts.
  - Out-of-memory events from unbounded channels or metrics buffers (violates bounded-channels invariant).  

---

## 5. Regression Gates

### 5.1 Baselines

Perf baselines should be stored under:

- `testing/performance/baselines/svc-admin/`:

  - `baseline-local-only.json` (UI-only endpoints).
  - `baseline-node-fanout-10nodes.json`.
  - `baseline-node-fanout-50nodes.json`.

Each baseline captures:

- target RPS,
- p50/p95/p99 latency,
- CPU/mem snapshots.

### 5.2 CI Rules

CI or nightly jobs must **fail** if:

- p95 latency for local-only endpoints (`/api/ui-config`, `/api/me`) **increases >10%** over baseline at equal load.
- p95 latency for node-backed endpoints (`/api/nodes/{id}/status`) **increases >20%** over baseline, after discounting upstream node latency changes.
- Throughput at fixed load **drops >10%**.
- CPU or memory at baseline load **increases >15%**.

Escape hatch:

- A regression may be temporarily accepted if:
  - traced to an upstream dependency or security hardening change,
  - documented in CHANGELOG + PERF notes,
  - accompanied by a work item to recover performance.

---

## 6. Perf Runbook (Triage)

When SLOs are breached:

1. **Confirm the scope**:

   - Is the problem local-only endpoints (UI, `/api/me`) or node-backed endpoints?
   - Are nodes or IdP themselves healthy (check node `/healthz`, `/readyz`, `/metrics`)?:contentReference[oaicite:27]{index=27}  

2. **Check metrics** (svc-admin `/metrics`):

   - Look at:
     - `http_request_duration_seconds` histograms for svc-admin.:contentReference[oaicite:28]{index=28}  
     - `svc_admin_upstream_errors_total{kind}`.
     - `svc_admin_rejected_total{reason}` (busy, oversized, unauth, etc.).
   - Confirm if errors are upstream (nodes) vs internal.

3. **Profile**:

   - Run `cargo flamegraph -p svc-admin --bin svc-admin` under representative load.
   - Use `tokio-console` to inspect long-running tasks or blocked futures.  

4. **Inspect scaling knobs**:

   - Reduce metrics polling frequency or sampled nodes.
   - Increase concurrency caps slightly if they are too tight (within Hardening limits).
   - Consider enabling keep-alive or tuning timeouts.

5. **Chaos toggle**:

   - Temporarily disable expensive features (e.g. facet metrics parsing) to see if latency improves.
   - Narrow the metrics parsing to a subset of golden metrics only.

6. **Node vs svc-admin separation**:

   - If nodes are slow/unhealthy, svc-admin should:
     - keep `/readyz` reporting **ready** (svc-admin itself healthy),
     - surface per-node failures in UI, but not treat them as its own perf failure.  

7. **Record and update**:

   - Capture flamegraphs, tokio-console snapshots, and metric snapshots.
   - Update `testing/performance/baselines/` and this doc’s history if a new equilibrium is reached.

---

## 7. Acceptance Checklist (DoD)

For **svc-admin Beta+** on the PERF concern:

- [ ] SLOs for latency, throughput, and error budget are defined (this doc).
- [ ] Micro-benchmarks exist for status mapping and metrics parsing.
- [ ] Integration perf harness (`svc_admin_smoke.sh`, `svc_admin_fanout.sh`) runs locally.
- [ ] At least one **flamegraph** and **tokio-console trace** has been captured and archived.
- [ ] Scaling knobs (poll intervals, concurrency caps, metrics limits) are documented in `CONFIG.md` and this doc.
- [ ] Regression gates are wired into CI or scheduled nightly runs.
- [ ] Perf runbook section is reviewed and kept in sync with observed behavior.

---

## 8. Appendix

### 8.1 Reference SLOs (Scaling Blueprint)

svc-admin inherits the workspace’s **PERF** concern expectations:​:contentReference[oaicite:31]{index=31}  

- p95 GET < 80 ms intra-region (for local-only endpoints).
- Failures < 0.1%.
- RF observed ≥ RF target (for data-plane nodes; svc-admin’s job is to visualize, not enforce).

### 8.2 Reference Workloads

- Operators checking node overview/dashboard every few seconds.
- Occasional drilling into:
  - plane-level status,
  - facet metrics,
  - auth mode & environment differences.  

### 8.3 Perfection Gates Tie-in

- **Gate F (perf regressions barred):**
  - Any measurable regression beyond defined thresholds **must** be caught by CI and documented.

- **Gate L (scaling validated under chaos):**
  - svc-admin must be tested under:
    - partial node outages,
    - slow metrics,
    - auth misconfig scenarios (passport issues)  
    while remaining usable and truthful.

### 8.4 History

- *2025-12-04*: Initial PERF profile for svc-admin drafted (pre-Beta) based on IDB, Hardening, and Concurrency docs.

```
