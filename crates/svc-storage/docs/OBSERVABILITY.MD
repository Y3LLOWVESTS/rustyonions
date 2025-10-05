

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus/OTEL)
* Health & readiness semantics
* Logs (JSON schema & redaction)
* Tracing spans & correlation
* Alerts & SLOs (with example PromQL)

This file complements `docs/IDB.md`, `docs/SECURITY.md`, and `docs/CONFIG.md`.

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (every service)

* `http_requests_total{route,method,status}` — **Counter**
* `request_latency_seconds{route,method}` — **Histogram** (end-to-end)
* `inflight_requests{route}` — **Gauge** (or derived from concurrency cap)
* `bus_lagged_total{topic}` — **Counter** (broadcast backlog drop)
* `service_restarts_total{kind}` — **Counter** (supervised restarts)
* `rejected_total{reason,route}` — **Counter** (`busy|unauth|payload_too_large|decompress_cap|ratio_cap|timeout|shutdown`)

> **Histogram buckets (default):** `0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5, 1, 2, 5` seconds.
> Keep buckets identical across services for cross-dashboards.

### 1.2 Service-Specific (svc-storage)

**Latency/throughput**

* `storage_get_latency_seconds{route,range}` — **Histogram**
* `storage_put_latency_seconds{route}` — **Histogram**
* `bytes_in_total{route}` / `bytes_out_total{route}` — **Counter**
* `chunks_read_total{tier}` / `chunks_written_total{tier}` — **Counter**

**Integrity & safety**

* `integrity_fail_total{reason}` — **Counter** (`checksum|decompress_cap|header_mismatch`)
* `decompress_reject_total{reason}` — **Counter** (`cap|format|truncated`)
* `quota_exhaustions_total{kind}` — **Counter** (`rps|inflight|body_bytes`)

**Durability & repair**

* `rf_target` / `rf_observed` — **Gauge**
* `repair_bytes_total{phase}` — **Counter** (`planned|executed`)
* `repair_seconds_total{phase}` — **Counter**
* `repair_wait_seconds` — **Histogram** (pacing token waits)

**Queues**

* `queue_depth{queue}` — **Gauge** (`work|repair`)
* `queue_dropped_total{queue}` — **Counter** (`work|events`)

**Auth/TLS (if local termination)**

* `auth_failures_total{kind}` — **Counter** (`macaroon|scope|expired|nbf|aud`)
* `tls_handshake_failures_total{kind}` — **Counter**

### 1.3 Registration Discipline

* All metrics **registered once** in `Metrics::new()` (or equivalent) and injected via handles.
* **No lazy registration in hot-paths.**
* CI enforces: a single registration site, and presence of the **golden set** above.

### 1.4 Example PromQL Snippets

* **p95 GET latency (intra-region)**

  ```
  histogram_quantile(0.95,
    sum by (le) (rate(storage_get_latency_seconds_bucket[5m]))
  )
  ```
* **Error budget: 5xx < 0.1%**

  ```
  sum(rate(http_requests_total{status=~"5.."}[5m]))
  /
  sum(rate(http_requests_total[5m]))
  ```
* **Busy rejections**

  ```
  sum(rate(rejected_total{reason="busy"}[5m]))
  ```
* **RF drift**

  ```
  max(rf_target) - min(rf_observed)
  ```

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **Liveness**: returns `200 OK` if the process is alive (no dependencies).
* `GET /readyz` — **Readiness**: returns `200 OK` **only when** all readiness keys are satisfied.
* `GET /metrics` — Prometheus exposition.
* `GET /version` — build metadata (git SHA, semver, features, msrv).

### 2.2 Readiness Keys (svc-storage)

Readiness **fails** until all of the following are true:

* `config_loaded=true` — config parsed & validated.
* `listener_bound=true` — TCP/UDS bound and accepting.
* `metrics_bound=true` — metrics server bound.
* `placement_cache_warm=true` — initial placement/index hints loaded (or timed out with safe defaults).
* `durability_pacer_ready=true` — repair pacer and workers initialized.
* `amnesia_mode_ok=true` — if `amnesia=true`, no persistent spill will occur; if would-spill → **unready**.
* `auth_verifier_ready=true` — macaroon verifier loaded (for PUT/admin endpoints).

### 2.3 Failure Semantics

* **Fail-open reads / fail-closed writes** (as per canon):

  * Reads may continue during partial dependency loss **if** integrity checks and limits can be enforced.
  * Writes become **unready** (503) whenever required invariants can’t be guaranteed.
* Degraded `/readyz` response body (JSON):

  ```json
  {
    "ok": false,
    "degraded": true,
    "missing": ["placement_cache_warm","durability_pacer_ready"],
    "retry_after": 15
  }
  ```

---

## 3) Logs

### 3.1 Format

* **JSON lines** (`application/jsonl`), one event per line.
* **Required fields**

  * `ts` (ISO-8601 with timezone)
  * `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `service` (`"svc-storage"`)
  * `event` (e.g., `get_object`, `put_object`, `repair_plan`, `repair_execute`, `oap_reject`)
  * `route`, `method`, `status`
  * `corr_id` (ULID/UUID; injected from `X-Corr-ID` or generated)
  * `latency_ms` (if request-scoped)
  * `b3_prefix` (first 8 hex of content address, when applicable)
  * `bytes_in`, `bytes_out`
  * `reason` (align with `rejected_total{reason=…}` if an error/reject)
  * `amnesia` (`true|false`)

**Optional fields**

* `cap_id_prefix`, `actor`, `peer` (sanitized)
* `rf_target`, `rf_observed` (on durability logs)
* `plan_id`/`repair_job_id`

### 3.2 Redaction & Secrets

* **Never** log secret material (keys, macaroons, full tokens).
* Truncate content addresses to `b3_prefix` (≤8 hex).
* Config diffs in logs must **redact** secret paths and values.
* Capabilities are represented by **capability IDs** or prefixes only.

### 3.3 Example Log Lines

**Accepted GET Range**

```json
{"ts":"2025-10-04T19:12:03.521Z","level":"INFO","service":"svc-storage","event":"get_object",
 "route":"/o/{b3}","method":"GET","status":206,"corr_id":"01JABCDXYZ...",
 "b3_prefix":"b3:9fa1d2c3","latency_ms":42,"bytes_out":65536,"amnesia":false}
```

**Rejected PUT (busy)**

```json
{"ts":"2025-10-04T19:12:12.006Z","level":"WARN","service":"svc-storage","event":"put_object",
 "route":"/o","method":"PUT","status":429,"reason":"busy","corr_id":"01JABCDEFG...",
 "bytes_in":1048576,"amnesia":false}
```

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` with **JSON** formatter by default.
* **Span naming**: `svc.storage.<operation>`

  * `svc.storage.get`, `svc.storage.put`, `svc.storage.repair.plan`, `svc.storage.repair.exec`
* **Fields on spans** (at minimum):

  * `corr_id`, `b3_prefix`, `range`, `bytes_in`, `bytes_out`, `rf_target`, `rf_observed`
* **Events** inside spans for significant steps:

  * `read_start`, `chunk_verified`, `decompress_reject`, `write_committed`, `repair_token_wait`, `repair_done`
* **Correlation IDs**

  * Ingress: accept `X-Corr-ID`; if absent, generate ULID and set response header.
  * Propagate through internal bus events and to downstream RPCs as metadata.
* **OpenTelemetry** (optional, feature `otel`)

  * Exporter: OTLP/gRPC
  * Resource attrs: `service.name=svc-storage`, `service.version`, `deployment.environment`

---

## 5) Alerts & SLOs

### 5.1 SLOs (svc-storage)

* **Public GET p95 start latency**:

  * **< 80 ms** intra-region; **< 200 ms** inter-region.
* **Availability**:

  * `5xx` rate **< 0.1%** (rolling 30 days).
  * `429|503` rate **< 1%** (rolling 30 days).
* **Durability posture**:

  * `rf_observed ≥ rf_target` (steady-state).

### 5.2 Alert Rules (examples)

> Tune thresholds to your environment; link each alert to `RUNBOOK.md`.

* **Critical — Integrity failure**

  ```promql
  sum(rate(integrity_fail_total[5m])) > 0
  ```
* **Critical — Readiness flap**

  ```promql
  max_over_time(up{job="svc-storage"}[5m]) < 1
  OR
  probe_success{target="readyz"} == 0
  ```
* **Warning — Busy spikes**

  ```promql
  sum(rate(rejected_total{reason="busy"}[5m])) > 20
  ```
* **Warning — Repair stuck**

  ```promql
  rate(repair_bytes_total{phase="executed"}[15m]) == 0
  AND
  (max(rf_target) - min(rf_observed)) > 0
  ```
* **Warning — Decompression rejects**

  ```promql
  sum(rate(decompress_reject_total[5m])) > 5
  ```

### 5.3 Dashboards (suggested panels)

* **Latency**: p50/p90/p95 for GET & PUT; range vs non-range.
* **Throughput**: bytes in/out; chunks read/written by tier.
* **Errors**: 4xx/5xx stacked; busy & timeout counters.
* **Durability**: rf_target vs rf_observed; repair bytes/sec; repair wait histogram.
* **Queues**: work/repair depth and drops.
* **Readiness & Health**: sparkline of readyz, restart counts.

---

## 6) CI / Enforcement

* **Golden endpoints** must exist: `/metrics`, `/healthz`, `/readyz`, `/version`.
* CI checks:

  * **Metrics presence**: grep for `http_requests_total`, `rejected_total`, `storage_get_latency_seconds`, `integrity_fail_total`, `rf_observed`.
  * **Duplicate registration**: ensure single `Metrics::new()` registration site.
  * **Log schema**: compile-time `tracing` fields lint via wrapper macros (optional).
  * **Readiness contracts**: integration test boots service and asserts `/readyz` transitions on dependency toggles.
* **90-day doc review**: this file’s `last-reviewed` date updated on substantive changes.

---

## 7) Implementation Notes (practical wiring)

* **Metrics handles**: construct once, pass via `Arc<Metrics>`.
* **Request labels**: keep `route` cardinality **low** (templated paths like `/o/{b3}`).
* **Histograms**: prefer **base-2** bucket growth where practical; keep bucket count ≤ 12.
* **Cardinality controls**:

  * Truncate `b3` to `b3_prefix` in logs/spans (not in labels).
  * Avoid labeling metrics with unbounded values (no `cap_id`, no raw `b3`).
* **Sampling**:

  * Use `log.sample` config for high-throughput deployments (e.g., `0.1`).

---

## 8) Example Rust (minimal pattern)

```rust
// Pseudocode skeleton — align with workspace prometheus facade
pub struct Metrics {
    pub http_requests_total: prometheus::IntCounterVec,
    pub request_latency_seconds: prometheus::HistogramVec,
    pub storage_get_latency_seconds: prometheus::HistogramVec,
    pub rejected_total: prometheus::IntCounterVec,
    pub integrity_fail_total: prometheus::IntCounterVec,
    pub rf_target: prometheus::IntGauge,
    pub rf_observed: prometheus::IntGauge,
    // ...
}

impl Metrics {
    pub fn new() -> Self {
        use prometheus::{opts, histogram_opts, register_int_counter_vec, register_histogram_vec, register_int_gauge};
        let lat_buckets = vec![0.005,0.01,0.02,0.05,0.1,0.25,0.5,1.0,2.0,5.0];
        Self {
            http_requests_total: register_int_counter_vec!(
                opts!("http_requests_total","HTTP requests"),
                &["route","method","status"]
            ).unwrap(),
            request_latency_seconds: register_histogram_vec!(
                histogram_opts!("request_latency_seconds","e2e latency").buckets(lat_buckets.clone()),
                &["route","method"]
            ).unwrap(),
            storage_get_latency_seconds: register_histogram_vec!(
                histogram_opts!("storage_get_latency_seconds","GET storage latency").buckets(lat_buckets),
                &["route","range"]
            ).unwrap(),
            rejected_total: register_int_counter_vec!(
                opts!("rejected_total","Rejected requests"),
                &["reason","route"]
            ).unwrap(),
            integrity_fail_total: register_int_counter_vec!(
                opts!("integrity_fail_total","Integrity check failures"),
                &["reason"]
            ).unwrap(),
            rf_target: register_int_gauge!("rf_target","Target replication factor").unwrap(),
            rf_observed: register_int_gauge!("rf_observed","Observed replication factor").unwrap(),
        }
    }
}
```

---

## 9) Readiness Drill (operator checklist)

* **Cold start**: `/healthz` 200, `/readyz` 503 until `placement_cache_warm` and `durability_pacer_ready` true; turns 200 within your SLO.
* **Amnesia mode**: verify `/readyz` **fails** when workload would spill to disk.
* **Chaos**: kill a worker; watch `service_restarts_total` and `/readyz` remain green; p95 unaffected within budget.
* **Repair event**: drop a replica; `rf_observed` dips then recovers; `repair_bytes_total` advances; alert stays green.

---

## 10) Change Management

* Any metric name/label change is **breaking** for dashboards and alerts; document in `CHANGELOG.md` with migration notes.
* Additive metrics are safe; keep names **stable** and consistent with other crates.
* Review observability docs on every change to routes, pacing, queues, or durability logic.

---

**Observability Definition of Done:**
Golden endpoints exist; **golden metrics** + **storage-specific metrics** registered; logs are structured + redacted; tracing spans propagate `corr_id`; `/readyz` truthfully reflects dependencies and amnesia; SLOs + alerts are wired with PromQL; CI enforces presence and registration discipline.
