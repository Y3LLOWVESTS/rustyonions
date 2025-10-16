
````markdown
# 📈 OBSERVABILITY.md — ron-accounting
*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- Metrics (Prometheus/OTEL)
- Health & readiness semantics
- Logs (JSON schema + fields)
- Tracing spans & correlation
- Alerts & SLOs (with PromQL examples + runbook hooks)

> **North Star:** `ron-accounting` is a **transient metering library** that seals **time-sliced** usage and exports slices **ordered + idempotently** to `ron-ledger`, with **bounded** resources and **backpressure-over-buffering**. Observability must continuously **prove** these properties.

> **Scope:** The **library** registers metrics and emits logs/traces; **adapters** (HTTP/UDS) expose `/metrics`, `/healthz`, `/readyz`.

---

## 1) Metrics (Prometheus-style)

All metric names are **snake_case**, dimensions are **limited & sampled** to bound cardinality.  
**Registration discipline:** create once in `Metrics::new()`, pass handles (Arc) to call sites.

### 1.1 Golden Metrics (standardized across services/adapters)

- `http_requests_total{route,method,status}` (Counter) — adapter only
- `request_latency_seconds{route,method}` (Histogram) — adapter only
- `inflight_requests{route}` (Gauge) — implied cap via server pool
- `bus_lagged_total` (Counter) — broadcast lag/drop events
- `service_restarts_total{task}` (Counter) — supervised task restarts
- `rejected_total{reason}` (Counter) — unified shed/reject counter (see §1.3)
- `config_reload_fail_total` (Counter) — failed hot-reloads (CONFIG §5)
- `auth_failures_total{service="ron-accounting"}` (Counter) — SECURITY tie-in (adapter)

### 1.2 ron-accounting — Core Metrics (library)

**Export lifecycle**
- `accounting_exports_total{status}` (Counter)  
  `status ∈ {ok, dup, retry_network, retry_remote_5xx, fail}`

**Queues & backpressure (sampled labels)**
- `queue_depth{queue}` (Gauge) — `queue ∈ {pending_slices, lane}`  
  > For lanes, **sample** `(tenant,dimension)`; surface top-N via logs, not labels.
- `queue_dropped_total{queue,reason}` (Counter) — `reason ∈ {capacity, order_overflow, wal_full}`

**Readiness & health**
- `accounting_degraded` (Gauge: 0/1)
- `readyz_missing_total{key}` (Counter)

**Ordering & idempotence**
- `ordering_wait_seconds` (Histogram) — time slice waited for prior seq (no high-card keys)
- `dup_detected_total` (Counter) — downstream replied duplicate

**WAL (Macronode / amnesia=false)**
- `accounting_wal_size_bytes` (Gauge)
- `accounting_wal_entries` (Gauge)
- `accounting_wal_corrupt_total` (Counter)
- `accounting_wal_fsync_seconds` (Histogram)
- `accounting_wal_replay_seconds` (Histogram)

**Rollover & windows**
- `rollover_events_total{result}` (Counter) — `{ok, skipped_dup, error}`
- `rollover_skew_ms` (Histogram) — |T(now) − UTC boundary| when sealing

**Exporter retry**
- `backoff_retries_total{op}` (Counter) — `op ∈ {put, handshake}`
- `export_latency_seconds` (Histogram) — enqueue→ACK/dup/fail, respects global 10s budget

### 1.3 Canonical reasons for `rejected_total{reason}` / `queue_dropped_total{queue,reason}`

- `capacity` — generic queue full
- `order_overflow` — per-stream lane cap hit while holding order
- `wal_full` — WAL size/entries/age cap exceeded
- `unauth` — macaroon/TLS check failed (adapter)
- `decompress_cap` / `ratio_cap` — decompression limits (adapter)
- `oversize` — DTO exceeds 1 MiB (adapter)
- `busy` — hot-path `try_send` rejection

> **Cardinality:** never include raw `tenant`/`dimension` as labels in hot metrics. Use sampled gauges & periodic log summaries instead.

### 1.4 Copy-paste: Rust registration skeleton

```rust
pub struct Metrics {
    pub exports_total: prometheus::IntCounterVec,
    pub queue_depth: prometheus::IntGaugeVec,
    pub queue_dropped_total: prometheus::IntCounterVec,
    pub degraded: prometheus::IntGauge,
    pub export_latency: prometheus::Histogram,
    pub ordering_wait: prometheus::Histogram,
    pub wal_size_bytes: prometheus::IntGauge,
    pub wal_entries: prometheus::IntGauge,
    pub wal_corrupt_total: prometheus::IntCounter,
    pub backoff_retries_total: prometheus::IntCounterVec,
    pub config_reload_fail_total: prometheus::IntCounter,
}

impl Metrics {
    pub fn new(reg: &prometheus::Registry) -> Self {
        let export_latency = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new("export_latency_seconds", "export end-to-end")
                .buckets(vec![0.01,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0])
        ).unwrap();
        reg.register(Box::new(export_latency.clone())).unwrap();
        // ...register others once; store handles in Self
        Self { /* assign fields */ exports_total: prometheus::IntCounterVec::new(
                prometheus::Opts::new("accounting_exports_total","exports by status"),
                &["status"]).unwrap(),
            // ...
            degraded: prometheus::IntGauge::new("accounting_degraded","0/1 readiness").unwrap(),
            config_reload_fail_total: prometheus::IntCounter::new("config_reload_fail_total","reload failures").unwrap(),
        }
    }
}
````

---

## 2) Health & Readiness

### 2.1 Endpoints (adapter builds)

* `/healthz` — **liveness** (process up). Always `200 OK` if process alive.
* `/readyz` — **readiness**: returns `200 OK` when the keys in §2.2 are **all true**; else `503` with JSON.

### 2.2 Readiness Keys (authoritative)

* `config_loaded` — config validated; unsafe flips rejected
* `queues_bounded_ok` — pending depth below threshold; shed within budget
* `exporter_ok` — ledger reachable (recent success) OR backlog under threshold
* `wal_ok` — if WAL enabled: dir usable, fsync OK, quotas not exceeded
* `boundary_ticker_ok` — UTC boundary not missed in last 2 windows

**/readyz failure body example**

```json
{ "degraded": true, "missing": ["exporter_ok","wal_ok"], "retry_after": 15 }
```

**Semantics**

* **Fail-open reads / Fail-closed writes:** library read-only APIs may continue; exporting new slices stops while not ready.
* `accounting_degraded` mirrors readiness (1 while not ready).

---

## 3) Logs

### 3.1 Format

* JSON Lines (`application/jsonl`), one event per line.
* Required fields:

  * `ts` (ISO8601), `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `service` (`ron-accounting`), `event` (machine string)
  * `corr_id` (ULID/UUID), `peer_addr` (adapter)
  * `tenant`, `dimension` (opaque IDs; prefer **short-id** at INFO)
  * `reason` (aligned to `rejected_total{reason}`)
  * `seq`, `b3` (short digest), `latency_ms`/`elapsed_ms` where relevant

**Examples**

```json
{"ts":"2025-10-14T21:03:44Z","level":"INFO","service":"ron-accounting",
 "event":"rollover.sealed","tenant":"a1f5","dimension":"bytes","seq":128,"b3":"d8e2..4a","skew_ms":142,"corr_id":"01J9..."}
```

```json
{"ts":"2025-10-14T21:03:45Z","level":"WARN","service":"ron-accounting",
 "event":"lane.shed","tenant":"a1f5","dimension":"bytes","reason":"order_overflow","queue":"lane","corr_id":"01J9..."}
```

### 3.2 Redaction & Secrets

* No PII; never log tokens/caps/DTO bodies.
* INFO logs use **short-ids**; full IDs allowed only at DEBUG behind a `redact=false` guard.

---

## 4) Tracing & Correlation

* Use `tracing` + JSON formatter; optional OTEL exporter (feature `otel`).

**Span names**

* `svc.ron_accounting.rollover`
* `svc.ron_accounting.export.put`
* `svc.ron_accounting.wal.append`
* `svc.ron_accounting.wal.replay`

**Span fields**: `tenant`, `dimension`, `seq`, `status`, `retries`, `deadline_ms` (avoid high-card label explosions).

**Correlation**

* Accept `X-Corr-ID` on adapter ingress; generate ULID if absent.
* Propagate `corr_id` via logs; use OTEL `traceparent` when enabled.

**OTEL sampling defaults**

* When `otel` feature is enabled, default to **head-based sampling at 1%**, configurable via env/config.
* Do **not** include `tenant`/`dimension` as span attributes in high-fanout paths; keep detailed IDs in logs only.

---

## 5) Alerts & SLOs

### 5.1 SLOs (tune per profile)

* **Exporter p99 latency ≤ 1.0s** (intra-region); p99.9 ≤ 2.5s.
* **Backlog bounded:** `queue_depth{queue="pending_slices"}` ≤ 0.5 × cap for ≥ 99% of minutes/day.
* **Idempotence/ordering:** `dup_detected_total / accounting_exports_total{status="ok"} ≤ 0.5%`.
* **Error budget:** `(status="fail" OR "retry_remote_5xx") / all ≤ 0.1%`.
* **Readiness:** `/readyz` green ≥ 99.9%.

### 5.2 PromQL — panels & alerts

**Panels**

```promql
// Exporter latency p99
histogram_quantile(0.99, sum by (le) (rate(export_latency_seconds_bucket[5m])))

// Rejects by reason (rate)
sum by (reason)(rate(rejected_total[5m]))

// Degraded flag
max_over_time(accounting_degraded[5m]) == 1

// WAL saturation (replace denominator with cap recording rule)
accounting_wal_size_bytes / 5.36870912e8
```

**Alert rules (sketch)**

```yaml
groups:
- name: ron-accounting
  rules:
  - alert: AccountingDegraded
    expr: max_over_time(accounting_degraded[5m]) == 1
    for: 5m
    labels: {severity: page}
    annotations:
      summary: "ron-accounting degraded"
      runbook: "RUNBOOK.md#degraded"

  - alert: ExporterLatencyP99High
    expr: histogram_quantile(0.99, sum by (le) (rate(export_latency_seconds_bucket[5m]))) > 1.0
    for: 10m
    labels: {severity: warn}
    annotations:
      summary: "Exporter p99 latency high"
      runbook: "RUNBOOK.md#exporter-latency"

  - alert: WALCapacity80Percent
    expr: (accounting_wal_size_bytes / 5.36870912e8) > 0.8
    for: 10m
    labels: {severity: warn}
    annotations:
      summary: "WAL usage >80%"
      runbook: "RUNBOOK.md#wal-usage"

  - alert: RejectsSpike
    expr: sum(rate(rejected_total[1m])) by (reason) > 100
    for: 5m
    labels: {severity: warn}
    annotations:
      summary: "Sustained rejects by reason"
      runbook: "RUNBOOK.md#rejects"

  - alert: ConfigReloadFailures
    expr: increase(config_reload_fail_total[1h]) > 0
    for: 0m
    labels: {severity: error}
    annotations:
      summary: "Config reload failures detected"
      runbook: "RUNBOOK.md#config-reload"

  - alert: AuthFailuresSpike
    expr: increase(auth_failures_total{service="ron-accounting"}[5m]) > 50
    for: 5m
    labels: {severity: warn}
    annotations:
      summary: "Auth failures spiking"
      runbook: "RUNBOOK.md#auth-failures"

  - alert: OrderingWaitP99High
    expr: histogram_quantile(0.99, sum by (le) (rate(ordering_wait_seconds_bucket[5m]))) > 0.5
    for: 10m
    labels: {severity: warn}
    annotations:
      summary: "Ordering holds > 500ms at p99"
      runbook: "RUNBOOK.md#ordering-holds"
```

### 5.3 Runbooks (anchor these in your repo)

* **Degraded:** verify exporter reachability, WAL health, queue depths, recent sheds; consider safe cap adjustments.
* **Exporter latency:** inspect ledger 5xx vs network; retry statuses; check backoff caps & global 10s budget.
* **WAL usage:** free space or adjust cap; validate fsync timing; ensure correct amnesia/WAL profile.
* **Rejects spike:** use logs to identify top tenants/dimensions; consider WFQ weights or upstream shaping.
* **Auth failures:** confirm token rotation timing, clock skew at edge, macaroon caveats.

---

## 6) CI / Enforcement

* **Build-time**

  * `cargo clippy` denies `await_holding_lock` (performance & safety).
  * `cargo-deny` for advisories/licensing/bans.
* **Test-time**

  * Unit tests assert metric registration (no duplicate names).
  * Property tests verify ordering invariants & shed counters.
  * Loom models (PR-only) check no deadlocks in queue/worker/shutdown.
* **Alerts lint**

  * `promtool check rules ops/alerts/ron-accounting.yaml` to validate alert syntax/exprs.
* **Doc freshness**

  * A small registry test emits metric inventory; CI diffs against this doc.
* **Endpoint presence (adapter)**

  * CI greps for `/metrics`, `/healthz`, `/readyz` routes when `http-adapter` feature is on.

---

## 7) OTEL (optional)

* Feature `otel` enables OTLP/gRPC export for traces (and optionally metrics).
* **Sampling:** default head-based **1%**; configurable via env/config. Keep labels low-card; avoid `tenant`/`dimension` attributes in hot spans.
* **Interop:** If Prometheus & OTEL metrics are both enabled, prefer **Prometheus for alerting**. Keep OTEL metrics export off unless a central collector exists. Avoid dual-sourcing dashboards.

---

## 8) Dashboards (starter layout + JSON)

**Overview panels**

* Export throughput — `rate(accounting_exports_total[5m])` by `status`
* Export latency p50/p95/p99 — `export_latency_seconds_bucket`
* Degraded flag — `accounting_degraded`
* Rejects by reason — `rate(rejected_total[5m])`
* Pending/backlog depth vs cap — `queue_depth{queue="pending_slices"}`
* WAL size vs cap; WAL corrupt counter

**Drill-downs**

* Ordering wait histogram
* Backoff retries by op
* Rollover skew ms over time
* Top N shed offenders (from logs, not labels)

**Grafana import JSON (starter)**

```json
{
  "title": "ron-accounting — Overview",
  "schemaVersion": 36,
  "panels": [
    {"type":"timeseries","title":"Export latency p99",
     "targets":[{"expr":"histogram_quantile(0.99, sum by (le) (rate(export_latency_seconds_bucket[5m])))"}]},
    {"type":"timeseries","title":"Exports by status (rate)",
     "targets":[{"expr":"sum by (status)(rate(accounting_exports_total[5m]))"}]},
    {"type":"timeseries","title":"Pending queue depth",
     "targets":[{"expr":"queue_depth{queue=\"pending_slices\"}"}]},
    {"type":"stat","title":"Degraded","targets":[{"expr":"max(accounting_degraded)"}]},
    {"type":"bargauge","title":"Rejects by reason",
     "targets":[{"expr":"sum by (reason)(rate(rejected_total[5m]))"}]},
    {"type":"timeseries","title":"WAL size",
     "targets":[{"expr":"accounting_wal_size_bytes"}]}
  ]
}
```

---

## 9) Sampling & Cardinality Policy

* **Never** add `tenant` or `dimension` as unconditional Prom labels.
* Use **sampled gauges** (e.g., 10 random keys/min) plus aggregates.
* For top-N diagnosis, rely on **structured logs** with short-ids and periodic rollups (every 60s).

---

## 10) Copy-paste snippets (adapter)

**/metrics handler (axum)**

```rust
async fn metrics(State(_): State<AppState>) -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    prometheus::TextEncoder::new().encode(&metric_families, &mut buf).unwrap();
    (StatusCode::OK, buf)
}
```

**/readyz gate**

```rust
#[derive(serde::Serialize)]
struct Readyz { degraded: bool, missing: smallvec::SmallVec<[&'static str; 4]>, retry_after: u64 }

async fn readyz(State(s): State<AppState>) -> impl IntoResponse {
    let missing = s.check_keys(); // Vec<&'static str>
    if missing.is_empty() {
        s.metrics.degraded.set(0);
        return (StatusCode::OK, Json(Readyz{ degraded: false, missing: smallvec::smallvec![], retry_after: 0 }));
    }
    s.metrics.degraded.set(1);
    (StatusCode::SERVICE_UNAVAILABLE, Json(Readyz{ degraded: true, missing: missing.into(), retry_after: 15 }))
}
```

---

## 11) Field Guide (incident first-checks)

1. `accounting_degraded` → if 1, open `/readyz` and inspect `missing`.
2. `accounting_exports_total{status}` → spikes in `retry_*` or `fail`?
3. `queue_depth{queue="pending_slices"}` → trending to cap?
4. `queue_dropped_total{reason}` → which reason dominates (capacity vs order_overflow vs wal_full)?
5. `accounting_wal_size_bytes` & `accounting_wal_corrupt_total` → storage trouble?
6. Logs: `event=lane.shed` or `event=export.fail` with `reason`.
7. **Ordering holds:** check `ordering_wait_seconds` p95/p99 → if high, inspect ledger ACK latency or adjust WFQ weights.
8. **Auth failures:** `auth_failures_total` spike? Verify token rotation, clock skew at edge, macaroon caveats.

---

## 12) Governance

* Update this doc when:

  * A **new metric** is added or any label changes.
  * Readiness semantics change.
  * New adapter endpoints appear.
* Review cadence: **90 days** or on any concurrency/config/security change.
* Owners should be pinged on stale docs (>90 days since `last-reviewed`).

---

```

