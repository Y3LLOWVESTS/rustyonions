

---

# 📈 OBSERVABILITY.md — svc-edge

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus/OTEL)
* Health & readiness semantics
* Logs (JSON schema, fields)
* Tracing spans & correlation
* Alerts & SLOs

Design center: **determinism over cleverness**, **fail-fast**, **degrade-loudly**. Every invariant in `docs/IDB.md` has observable proof here.

---

## 1) Metrics (Prometheus-style)

> All metrics are **registered once** in `Metrics::new()` and cloned; unit labels are explicit; histograms use base-2 buckets where relevant.

### 1.1 Golden Metrics (every service)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds_bucket{route,method,le}` + `_sum`/`_count` (Histogram)
* `inflight_requests{route}` (Gauge)
* `service_restarts_total{component}` (Counter)
* `rejected_total{reason}` (Counter) — canonical reject reasons: `amnesia|not_allowed|body_cap|decompress_cap|ratio_cap|timeout|rate_limit|invalid_range|etag_invalid|draining`
* `bus_lagged_total{bus}` (Counter) — broadcast backlog dropped

### 1.2 svc-edge–Specific Metrics

#### Ingress & Shedding

* `edge_inflight{endpoint}` (Gauge) — active requests (mirrors concurrency cap)
* `edge_queue_depth{queue="work"}` (Gauge)
* `edge_queue_dropped_total{queue="work"}` (Counter)
* `busy_rejections_total{endpoint}` (Counter) — 429s emitted due to full work queue

#### Cache & HTTP Semantics

* `edge_cache_hits_total{kind="pack|cas",range="hit|miss"}`
* `edge_cache_misses_total{cause="cold|stale|bypass"}`
* `edge_range_requests_total{result="206|416"}`

#### Live Fills

* `edge_live_fill_attempts_total{host}` (Counter)
* `edge_live_fill_success_total{host}` (Counter)
* `backoff_retries_total{op="live_fill"}` (Counter)
* `io_timeouts_total{op="fill|read|write|connect"}` (Counter)
* `edge_cas_writes_total{status="ok|dedup|error"}` (Counter)
* `edge_bytes_out_total{route}` / `edge_bytes_in_total{route}` (Counter)

#### Security Signals

* `tls_handshake_failures_total` (Counter) — if TLS local
* `audit_records_total{kind="fill|reject"}` (Counter, when audit enabled)

### 1.3 Histogram Buckets (normative)

* `request_latency_seconds`: `[0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28, 2.56, 5.12]`
* `fill_latency_seconds`: same buckets (export as separate histogram if measured)

### 1.4 Registration Discipline

* Define all metrics in a single `Metrics` struct; export handles via `Arc<Metrics>`.
* CI grep prevents duplicate `register_*` calls.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `/healthz` — liveness. Always `200 OK` if the process is alive and main loop not panicking.
* `/readyz` — readiness. `200 OK` only if:

  * Config loaded & valid
  * Listener bound and caps installed (timeouts, inflight, rps, body, decompress)
  * **Mode=offline:** all packs verified/readable
  * **Mode=live:** CAS reachable + allow-list non-empty
  * Amnesia constraint satisfied (no writeable paths)
  * Shedder not currently tripping hard limits (rate/inflight)

### 2.2 Response Shape

On not-ready (or degraded):

```json
{
  "ready": false,
  "degraded": true,
  "missing": ["packs_verified","cas_reachable","shedder_stable"],
  "retry_after": 1
}
```

* Always **fail-closed**: if any invariant check fails, return `503`.

### 2.3 Failure Semantics

* **Intake stops first** (listener refuses new work with 429).
* **In-flight drains** within deadline; `/readyz` remains degraded until stable.
* On amnesia violation risk → reject writes & degraded state instead of persisting.

---

## 3) Logs

### 3.1 Format (JSONL)

* One JSON object per line.
* Required fields (normative):

| Field                  | Type           | Notes                                                                 |       |      |      |        |
| ---------------------- | -------------- | --------------------------------------------------------------------- | ----- | ---- | ---- | ------ |
| `ts`                   | RFC3339 string | UTC                                                                   |       |      |      |        |
| `level`                | string         | `TRACE                                                                | DEBUG | INFO | WARN | ERROR` |
| `service`              | string         | `"svc-edge"`                                                          |       |      |      |        |
| `event`                | string         | e.g., `request`, `live_fill`, `reject`, `pack_mount`, `ready_changed` |       |      |      |        |
| `route`                | string         | e.g., `/edge/assets`                                                  |       |      |      |        |
| `status`               | int            | HTTP status                                                           |       |      |      |        |
| `method`               | string         | HTTP verb                                                             |       |      |      |        |
| `reason`               | string         | MUST match `rejected_total{reason}` values when a reject happens      |       |      |      |        |
| `corr_id`              | string         | UUID/ULID; generated if absent                                        |       |      |      |        |
| `peer_addr`            | string         | remote:port (redact if policy demands)                                |       |      |      |        |
| `latency_ms`           | number         | for request/operation events                                          |       |      |      |        |
| `bytes_in`/`bytes_out` | number         | where applicable                                                      |       |      |      |        |
| `mode`                 | string         | `offline                                                              | live` |      |      |        |
| `host`                 | string         | upstream host when live fill                                          |       |      |      |        |
| `etag`                 | string         | strong etag (hash) when known                                         |       |      |      |        |
| `hash`                 | string         | BLAKE3(CAS key) when known                                            |       |      |      |        |

**Never log**: token contents, raw headers, secret paths, pack bytes.

### 3.2 Examples

**Warm hit**

```json
{"ts":"2025-10-17T14:21:09Z","level":"INFO","service":"svc-edge","event":"request","route":"/edge/assets","method":"GET","status":200,"reason":"","corr_id":"01HE0...","peer_addr":"203.0.113.42:59123","latency_ms":18,"bytes_out":32768,"mode":"offline"}
```

**Queue shed (busy)**

```json
{"ts":"2025-10-17T14:22:41Z","level":"WARN","service":"svc-edge","event":"reject","route":"/edge/assets","status":429,"reason":"rate_limit","corr_id":"01HE1...","peer_addr":"203.0.113.44:51433","mode":"live"}
```

**Live fill success**

```json
{"ts":"2025-10-17T14:23:02Z","level":"INFO","service":"svc-edge","event":"live_fill","route":"/edge/assets","status":200,"host":"fonts.gstatic.com","corr_id":"01HE2...","latency_ms":173,"bytes_out":14560,"etag":"\"blake3:76ab...\"","hash":"76ab...","mode":"live"}
```

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` JSON formatter.
* **Span naming**: `svc.edge.<op>`

  * `svc.edge.http` — listener middleware span per request
  * `svc.edge.cache` — cache lookup/serve
  * `svc.edge.fill` — live fill (allow-check → fetch → validate → cas-write)
  * `svc.edge.range` — range parse/serve
* **Fields** on spans: `corr_id`, `route`, `range`, `host`, `attempt`, `reason`.
* **Propagation**:

  * Accept `X-Corr-ID` header; if missing, generate ULID. Echo as response header.
  * OTEL (optional, feature-flag `otel`) exports spans to collector (OTLP/gRPC).
* **Sampling**:

  * Default 1% for success paths; 100% for `status>=500` or `reason` present (tail sampling recommended at collector).

---

## 5) Alerts & SLOs

### 5.1 SLOs (svc-edge)

* **Latency (warm hit, intra-node):** p95 < **40 ms**, p99 < **80 ms**
* **Error rate (5xx):** < **0.1%**
* **Shedding (429/503):** < **1%** sustained over 15m
* **Live fill success:** ≥ **99%** (excluding upstream 4xx)
* **Ready uptime:** ≥ **99.9%** during business hours

### 5.2 Prometheus Rules (examples)

```yaml
groups:
- name: svc-edge-slo
  rules:
  - alert: EdgeHighErrorRate
    expr: sum(rate(http_requests_total{status=~"5..",service="svc-edge"}[5m])) 
          / sum(rate(http_requests_total{service="svc-edge"}[5m])) > 0.001
    for: 10m
    labels: {severity: warning}
    annotations: {summary: "svc-edge 5xx > 0.1% for 10m"}

  - alert: EdgeExcessiveShedding
    expr: sum(rate(rejected_total{reason=~"rate_limit|timeout|draining",service="svc-edge"}[5m]))
          / sum(rate(http_requests_total{service="svc-edge"}[5m])) > 0.01
    for: 15m
    labels: {severity: critical}
    annotations: {summary: "svc-edge shedding > 1% sustained"}

  - alert: EdgeNotReady
    expr: probe_success{job="svc-edge-readyz"} == 0
    for: 5m
    labels: {severity: critical}
    annotations: {summary: "svc-edge readiness failing >5m"}
```

### 5.3 Runbooks (link from alerts)

* `RUNBOOK.md#edgehigherrorrate`
* `RUNBOOK.md#edgeexcessiveshedding`
* `RUNBOOK.md#edgenotready`

Each runbook must include: **What changed?** (deploy, config, upstream), **Immediate actions**, **Triage checks** (dashboards/metrics), **Rollback/Remedies**.

---

## 6) Dashboards (minimum viable)

### 6.1 Overview

* Requests: stacked `http_requests_total` by status class
* Latency: p50/p95/p99 for `/edge/assets`
* Shed: `rejected_total` by `reason`
* Cache: hits vs misses; range 206/416
* Live fill: attempts/success; `io_timeouts_total`, `backoff_retries_total`
* Concurrency: `edge_inflight`, queue depth & dropped
* Readiness timeline: `probe_success` or `/readyz` status

### 6.2 Drilldowns

* **Per-host live fill** panel: success rate & latency per `host`
* **Per-endpoint** panel: `/edge/assets` vs ops endpoints
* **Security**: `tls_handshake_failures_total`, `audit_records_total` (if enabled)

---

## 7) CI / Enforcement

* Unit tests assert **metric contract** exists (crashes if missing labels/series).
* CI grep:

  * `/metrics`, `/healthz`, `/readyz` routes present
  * `rejected_total{reason=...}` includes all canonical reasons (`amnesia|not_allowed|body_cap|decompress_cap|ratio_cap|timeout|rate_limit|invalid_range|etag_invalid|draining`)
* Lints:

  * `await_holding_lock` check in reviews
  * No sleeps in readiness handlers
* Docs: update this file every **90 days** or on metrics/endpoint changes.

---

## 8) Export & Integrations

* **Prometheus**: pull `/metrics` (bind to loopback; scrape via sidecar/gateway).
* **OTel (optional)**: enable `otel` feature to export traces; set endpoint via env `OTEL_EXPORTER_OTLP_ENDPOINT`.
* **Log shipper**: JSON → vector/Fluent Bit → SIEM/Splunk; ensure redaction policies loaded.

---

## 9) Example Scrape Configs

### 9.1 Prometheus scrape (sidecar/gateway)

```yaml
scrape_configs:
- job_name: svc-edge
  static_configs:
  - targets: ['127.0.0.1:9090']   # metrics_addr in CONFIG
  relabel_configs:
  - source_labels: [__address__]
    target_label: service
    replacement: svc-edge
```

### 9.2 Blackbox readiness probe

```yaml
- job_name: svc-edge-readyz
  metrics_path: /probe
  params:
    module: [http_2xx]
  static_configs:
  - targets:
    - http://127.0.0.1:8080/readyz
  relabel_configs:
  - source_labels: [__address__]
    target_label: service
    replacement: svc-edge
```

---

## 10) Field Guide (How to Use Signals)

* **Spike in 429/`busy_rejections_total`** → queue full; tune `rps_limit`, add replicas, or widen queue slightly. Confirm with `edge_queue_depth`.
* **Rise in `io_timeouts_total{op="fill"}`** → upstream flaky; confirm `edge_live_fill_success_total` dip; widen retry window within budget; check allow-listed host status pages.
* **`edge_range_requests_total{result="416"}` uptick** → clients sending invalid ranges; audit access patterns; consider blocking abusive peers.
* **`edge_rejects_total{reason="amnesia"}`** → disk write attempted in amnesia mode; verify config and mount options.
* **Latency p95 regression with flat CPU** → check pack I/O and CAS locality; pin hot assets; consider `spawn_blocking` for hashing if CPU-bound.

---

## 11) Glossary (labels & reasons)

* `reason` values (complete set):
  `amnesia`, `not_allowed`, `body_cap`, `decompress_cap`, `ratio_cap`, `timeout`, `rate_limit`, `invalid_range`, `etag_invalid`, `draining`
* `route` examples: `/edge/assets`, `/metrics`, `/readyz`, `/healthz`
* `host`: upstream domain for live fill (allow-listed only)
* `mode`: `offline|live`

---

## 12) Appendix — Minimal Metrics Skeleton (Rust)

```rust
pub struct Metrics {
    pub http_requests_total: prometheus::IntCounterVec,
    pub request_latency: prometheus::HistogramVec,
    pub inflight: prometheus::IntGaugeVec,
    pub rejected_total: prometheus::IntCounterVec,
    pub edge_cache_hits_total: prometheus::IntCounterVec,
    pub edge_cache_misses_total: prometheus::IntCounterVec,
    pub edge_range_requests_total: prometheus::IntCounterVec,
    pub edge_inflight: prometheus::IntGaugeVec,
    pub edge_queue_depth: prometheus::IntGauge,
    pub edge_queue_dropped_total: prometheus::IntCounter,
    pub busy_rejections_total: prometheus::IntCounterVec,
    pub edge_live_fill_attempts_total: prometheus::IntCounterVec,
    pub edge_live_fill_success_total: prometheus::IntCounterVec,
    pub io_timeouts_total: prometheus::IntCounterVec,
    pub backoff_retries_total: prometheus::IntCounterVec,
    pub edge_cas_writes_total: prometheus::IntCounterVec,
}

impl Metrics {
    pub fn new() -> Self {
        use prometheus::{opts, register_int_counter_vec, register_histogram_vec, register_int_gauge_vec, register_int_counter, register_int_gauge};
        let request_latency = register_histogram_vec!(
            "request_latency_seconds",
            "Request latency in seconds",
            &["route","method"],
            vec![0.005,0.01,0.02,0.04,0.08,0.16,0.32,0.64,1.28,2.56,5.12]
        ).unwrap();
        Self {
            http_requests_total: register_int_counter_vec!("http_requests_total","Total HTTP requests",&["route","method","status"]).unwrap(),
            request_latency,
            inflight: register_int_gauge_vec!("inflight_requests","Inflight by route",&["route"]).unwrap(),
            rejected_total: register_int_counter_vec!("rejected_total","Rejected by reason",&["reason"]).unwrap(),
            edge_cache_hits_total: register_int_counter_vec!("edge_cache_hits_total","Cache hits",&["kind","range"]).unwrap(),
            edge_cache_misses_total: register_int_counter_vec!("edge_cache_misses_total","Cache misses",&["cause"]).unwrap(),
            edge_range_requests_total: register_int_counter_vec!("edge_range_requests_total","Range results",&["result"]).unwrap(),
            edge_inflight: register_int_gauge_vec!("edge_inflight","Inflight by endpoint",&["endpoint"]).unwrap(),
            edge_queue_depth: register_int_gauge!("edge_queue_depth","Work queue depth").unwrap(),
            edge_queue_dropped_total: register_int_counter!("edge_queue_dropped_total","Dropped from work queue").unwrap(),
            busy_rejections_total: register_int_counter_vec!("busy_rejections_total","429 by endpoint",&["endpoint"]).unwrap(),
            edge_live_fill_attempts_total: register_int_counter_vec!("edge_live_fill_attempts_total","Live fill attempts",&["host"]).unwrap(),
            edge_live_fill_success_total: register_int_counter_vec!("edge_live_fill_success_total","Live fill success",&["host"]).unwrap(),
            io_timeouts_total: register_int_counter_vec!("io_timeouts_total","I/O timeouts",&["op"]).unwrap(),
            backoff_retries_total: register_int_counter_vec!("backoff_retries_total","Retries",&["op"]).unwrap(),
            edge_cas_writes_total: register_int_counter_vec!("edge_cas_writes_total","CAS write results",&["status"]).unwrap(),
        }
    }
}
```

---

