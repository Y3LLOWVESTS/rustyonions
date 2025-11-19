

````markdown
# 📈 OBSERVABILITY.md — svc-interop

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Make `svc-interop` observable in production with **consistent**:
- Metrics (Prometheus/OpenTelemetry)
- Health & readiness semantics
- Logs (structured JSON, redaction)
- Tracing spans & cross-service correlation
- Alerts & SLOs tied to hardening limits and error taxonomy

This doc is the single source of truth for dashboards and alerting. It complements `IDB.md`, `SECURITY.md`, `CONFIG.md`, and the Concurrency Model.

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (every service)

> All metrics are **registered once** in `metrics::Registry::new()`; no lazy, per-request registration.

- `http_requests_total{route,method,status}` (Counter)  
  Count of completed HTTP requests, labeled by normalized `route` pattern (e.g., `/webhooks/:provider`), verb, and status.

- `request_latency_seconds{route,method}` (Histogram)  
  **Buckets (in seconds):** `0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2, 5`.  
  SLOs read p50/p95/p99 from this series.

- `inflight_requests{route}` (Gauge)  
  Use tower’s `ConcurrencyLimitLayer` gauge or an explicit guard to track current in-flight per route.

- `rejected_total{reason}` (Counter)  
  Reason codes **match IDB taxonomy**:  
  `UNAUTH, BAD_ORIGIN, EXPIRED_TOKEN, SCOPE_MISMATCH, BODY_LIMIT, DECOMP_LIMIT, RATE_LIMIT, BACKPRESSURE, DOWNSTREAM_UNAVAILABLE, POLICY_BLOCKED`.

- `bus_lagged_total` (Counter)  
  Increment when broadcast lag forces drop of oldest event.

- `service_restarts_total{task}` (Counter)  
  Supervisor restarts by task (`listener`, `worker`, `brownout_sentinel`).

- `brownout_active{component="bridge"}` (Gauge: 0/1)  
  On when error-rate or upstream p95 breaches thresholds (see §5).

### 1.2 Service-Specific (svc-interop)

- `webhook_verify_total{provider, result}` (Counter)  
  Edge signature verification attempts. `result = "ok" | "fail"`.

- `capabilities_issued_total{scope}` (Counter)  
  Macaroon capabilities minted via `svc-passport`. Label `scope` is low-cardinality (e.g., `write:put`, `read:get`).

- `audit_emitted_total{sink}` (Counter) & `audit_drop_total` (Counter)  
  Emitted vs dropped audit events (when the audit queue sheds).

- `stream_chunks_total{route}` (Counter)  
  Number of 64 KiB chunks processed for requests > 1 MiB.

- `hash_compute_seconds{alg}` (Histogram)  
  Time to compute content hashes. `alg = "blake3-256"` in production; SHA-256 **may** appear transiently for tests/edge verify benchmarking.

- `queue_depth{queue}` (Gauge) & `queue_dropped_total{queue}` (Counter)  
  Depth/pressure and drops for `work`, `events`, and `audit` queues.

- `upstream_rpc_seconds{service, op, outcome}` (Histogram)  
  Outbound calls to internal services, labeled by service (`passport|mailbox|storage|index`) and operation.

### 1.3 Registration Discipline

- All metrics live in `svc_interop_metrics` module; `Registry` (or OTEL meter) is injected via DI at startup.  
- CI contains a grep check for `register_*` calls to prevent duplicates.  
- Labels are **bounded**; no unbounded/user data in labels (no request IDs, emails, etc.).

### 1.4 Exemplars (optional but recommended)

If OTEL + Prometheus exemplars are enabled, attach `trace_id` to `request_latency_seconds` and `upstream_rpc_seconds` observations for rapid jump-to-trace.

---

## 2) Health & Readiness

### 2.1 Endpoints

- `/healthz` — **liveness**: returns `200 OK` if the process is up and the supervisor is not in crash-loop.
- `/readyz` — **readiness**: returns `200 OK` only when required deps are green **and** brownout not engaged for the requested path.

### 2.2 Readiness Keys (svc-interop)

- `listener_bound` — HTTP listener bound to `bind_addr`.
- `metrics_bound` — metrics listener bound to `metrics_addr`.
- `policy_loaded` — provider policy/allow-list loaded and validated.
- `passport_reachable` — `svc-passport` health within timeout.
- `queues_within_limits` — all bounded queues below critical watermarks.
- `brownout_bridge=false` — brownout sentinel not inhibiting **bridging** calls.

### 2.3 Degraded & Failure Semantics

- **Read paths (GET `/o/:addr`)** can remain ready when **bridging** is browned out (write/webhook paths return `503` with explicit reason).  
- Degraded response shape for browned-out endpoints:
  ```json
  { "degraded": true, "missing": ["passport_reachable"], "retry_after": 5 }
````

* Brownout triggers (see §5) are reflected in `brownout_active{component="bridge"}` and in `/readyz` JSON.

---

## 3) Logs

### 3.1 Format & Sink

* JSON lines (`application/jsonl`), one event per line; UTC timestamps.
* Log levels: `ERROR, WARN, INFO, DEBUG, TRACE`. Production defaults to `INFO`.
* Emit to stdout; optional file sink is **disallowed** in amnesia mode.

### 3.2 Required Fields (schema)

| Field        | Type   | Notes                                                                                              |        |               |                           |        |
| ------------ | ------ | -------------------------------------------------------------------------------------------------- | ------ | ------------- | ------------------------- | ------ |
| `ts`         | string | RFC3339/ISO8601 with ms                                                                            |        |               |                           |        |
| `level`      | string | `ERROR                                                                                             | WARN   | INFO          | DEBUG                     | TRACE` |
| `service`    | string | `"svc-interop"`                                                                                    |        |               |                           |        |
| `event`      | string | Machine-readable key: `http.access`, `webhook.verify`, `audit.emit`, `brownout.toggle`, `rpc.call` |        |               |                           |        |
| `reason`     | string | One of the **reason** taxonomy when applicable (see metrics)                                       |        |               |                           |        |
| `corr_id`    | string | ULID/UUID; injected/propagated                                                                     |        |               |                           |        |
| `trace_id`   | string | OTEL hex id when tracing is enabled                                                                |        |               |                           |        |
| `route`      | string | Normalized (e.g., `/webhooks/:provider`)                                                           |        |               |                           |        |
| `method`     | string | HTTP verb                                                                                          |        |               |                           |        |
| `status`     | number | HTTP status                                                                                        |        |               |                           |        |
| `latency_ms` | number | End-to-end duration for the request                                                                |        |               |                           |        |
| `provider`   | string | `github                                                                                            | stripe | slack_webhook | slack_rest` when relevant |        |
| `req_id`     | string | Provider request id (safe metadata only)                                                           |        |               |                           |        |
| `b3_hash`    | string | `"b3:<hex>"` internal **BLAKE3** hash for payloads (audit context)                                 |        |               |                           |        |
| `size_bytes` | number | Request/response size (when meaningful)                                                            |        |               |                           |        |
| `client_ip`  | string | Redacted/anonymized per policy if required                                                         |        |               |                           |        |

### 3.3 Redaction & Secrets

* Never log tokens, macaroons, webhook signing secrets, or raw `Authorization` headers.
* For provider signature headers, log **only** presence and algorithm label (e.g., `"sig_alg":"hmac-sha256"`), not the signature value.
* Config dumps must redact strings named like `*_SECRET*`, `*_TOKEN*`, `*_KEY*`.

### 3.4 Example Log (webhook success)

```json
{
  "ts":"2025-10-12T18:09:41.123Z",
  "level":"INFO",
  "service":"svc-interop",
  "event":"webhook.verify",
  "provider":"github",
  "route":"/webhooks/:provider",
  "method":"POST",
  "status":202,
  "latency_ms":42,
  "sig_alg":"hmac-sha256",
  "verified":true,
  "b3_hash":"b3:f1c2…",
  "corr_id":"01JABCDXYZ6M3K3H3K8Q3Z8P6P",
  "trace_id":"b1f3…",
  "reason":""
}
```

---

## 4) Tracing & Correlation

* **Tracing library:** `tracing` + `tracing-subscriber` JSON formatter; OTEL exporter feature-flagged.
* **Span naming:** `svc.interop.<area>.<op>`
  Examples: `svc.interop.http.put`, `svc.interop.http.get_object`, `svc.interop.webhook.verify`, `svc.interop.rpc.passport.mint`, `svc.interop.audit.emit`.
* **Correlation IDs:**

  * Ingress: honor `X-Corr-ID` if present or generate ULID; echo as response header.
  * Propagation: include `X-Corr-ID` on outbound calls; also rely on OTEL context if enabled.
* **Span attributes (stable set):**
  `http.route`, `http.method`, `http.status_code`, `peer.ip`, `provider`, `reason`, `queue.name`, `queue.depth`, `rpc.service`, `rpc.op`, `rpc.outcome`.

**Sampling:** default head-based `parent_or_else(ALWAYS_ON for WARN/ERROR spans, 1% for INFO)`. Allow dynamic reconfig via `/debug/config` if available.

---

## 5) Alerts & SLOs

### 5.1 SLOs (svc-interop)

* **Availability:** bridge endpoints ≥ 99.9% monthly; read paths ≥ 99.95%.
* **Latency (intra-region):** p95 ≤ 120 ms for `/put` and `/webhooks/:provider`; p95 ≤ 80 ms for `GET /o/:addr`.
* **Error budget guardrails:** 5xx < 0.1%; `rejected_total{reason=~"BACKPRESSURE|RATE_LIMIT"}` < 1% of requests over 5m.

### 5.2 Brownout Policy (readiness integration)

* Engage brownout for **bridging** when either condition holds over 5 minutes:

  * `rate( http_requests_total{status=~"5.."}[5m] ) / rate( http_requests_total[5m] ) > 0.01`, or
  * `histogram_quantile(0.95, sum by (le) (rate(request_latency_seconds_bucket{route=~"/webhooks/.+|/put"}[5m]))) > 0.5` (seconds), or
  * `queue_depth{queue="work"} > 0.8 * CAPACITY` sustained for 1 minute.
* Clear brownout automatically when all three recover below thresholds.

### 5.3 Prometheus Alert Examples

> Names assume a `job="svc-interop"` target.

* **High 5xx Rate (critical):**

  ```promql
  (sum by(job) (rate(http_requests_total{job="svc-interop",status=~"5.."}[5m]))
   / clamp_min(sum by(job) (rate(http_requests_total{job="svc-interop"}[5m])), 1e-9)) > 0.01
  ```

  **for:** 10m → page.

* **Readiness Flapping (warning):**

  ```promql
  changes(probe_success{job="svc-interop-readyz"}[15m]) > 4
  ```

* **Brownout Active (info→warn):**

  ```promql
  brownout_active{component="bridge",job="svc-interop"} == 1
  ```

* **Queue Pressure (work) (warning):**

  ```promql
  avg_over_time(queue_depth{queue="work",job="svc-interop"}[5m]) > 0.8 * 512
  ```

* **Webhook Verify Fail Burst (warning):**

  ```promql
  rate(webhook_verify_total{result="fail",job="svc-interop"}[5m]) > 5
  ```

### 5.4 Dashboards (must-have panels)

1. **Traffic & Errors:** `http_requests_total` stacked by status; 5xx overlay.
2. **Latency:** p50/p95/p99 from `request_latency_seconds` by route.
3. **Backpressure:** `queue_depth{work}`, `rejected_total{reason}` stacked.
4. **Brownout State:** single-stat `brownout_active{component="bridge"}` with sparkline of triggers.
5. **Webhook Verifications:** ok vs fail by provider.
6. **Upstream RPCs:** `upstream_rpc_seconds` p95 by downstream service.

---

## 6) CI / Enforcement

* **Static checks:**

  * Grep for all golden metrics names in the codebase.
  * Lint that every handler emits a `reason` on non-2xx.
  * Ensure `/metrics`, `/healthz`, `/readyz` routes are registered.
* **Runtime tests:**

  * Integration test ensures **at least one sample** is produced for each golden metric on a smoke run.
  * Readiness test simulates brownout, asserts `/readyz` flips writes to `503` while reads remain `200`.
* **No checkbox usage in docs:** progress tracked in issues, not by checkboxes.

---

## 7) Implementation Cheatsheet (Rust)

> Sketch — adapt to your metrics wrapper. Keep labels **low-cardinality**.

```rust
// Registration (once)
pub struct Metrics {
    pub http_requests_total: prometheus::IntCounterVec,
    pub request_latency_seconds: prometheus::HistogramVec,
    pub inflight_requests: prometheus::IntGaugeVec,
    pub rejected_total: prometheus::IntCounterVec,
    pub brownout_active: prometheus::IntGauge,
    pub queue_depth: prometheus::IntGaugeVec,
    pub queue_dropped_total: prometheus::IntCounterVec,
    pub webhook_verify_total: prometheus::IntCounterVec,
    pub upstream_rpc_seconds: prometheus::HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        use prometheus::*;
        let http_requests_total = IntCounterVec::new(
            opts!("http_requests_total", "HTTP requests"),
            &["route","method","status"]
        ).unwrap();
        let request_latency_seconds = HistogramVec::new(
            HistogramOpts::new("request_latency_seconds","Latency")
                .buckets(vec![0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.0,5.0]),
            &["route","method"]
        ).unwrap();
        // ... register with default registry
        Self { http_requests_total, request_latency_seconds, /* ... */ 
               inflight_requests: IntGaugeVec::new(opts!("inflight_requests",""), &["route"]).unwrap(),
               rejected_total: IntCounterVec::new(opts!("rejected_total",""), &["reason"]).unwrap(),
               brownout_active: IntGauge::new("brownout_active","1 when brownout is on").unwrap(),
               queue_depth: IntGaugeVec::new(opts!("queue_depth",""), &["queue"]).unwrap(),
               queue_dropped_total: IntCounterVec::new(opts!("queue_dropped_total",""), &["queue"]).unwrap(),
               webhook_verify_total: IntCounterVec::new(opts!("webhook_verify_total",""), &["provider","result"]).unwrap(),
               upstream_rpc_seconds: HistogramVec::new(
                   HistogramOpts::new("upstream_rpc_seconds","RPC to internal services")
                    .buckets(vec![0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.0,5.0]),
                   &["service","op","outcome"]).unwrap()
        }
    }
}
```

**Tower timing wrapper pattern**

```rust
async fn timed<S, B>(metrics: Metrics, route: &'static str, method: &'static str, fut: S) -> S::Output
where
  S: std::future::Future
{
    let timer = metrics.request_latency_seconds
        .with_label_values(&[route, method])
        .start_timer();
    let out = fut.await;
    timer.observe_duration();
    out
}
```

---

## 8) OTEL Mapping (optional)

If OTEL export is enabled:

* **Metrics:** map Prometheus series via OTEL **View** names; preserve units (`s`, `1`, `bytes`).
* **Traces:** exporter `otlp` to endpoint from `CONFIG.md` (`tracing.otel_endpoint`); resource attrs:
  `service.name="svc-interop"`, `service.version`, `service.instance.id`, `deployment.environment`.

---

## 9) Audit Linkage (Security-relevant Observability)

* Every allowed/denied action emits an **audit event**; the **BLAKE3 `b3_hash`** and `reason` must match the logs and metrics for the same `corr_id`.
* Ensure `audit_emitted_total` increments and `audit_drop_total` remains zero under nominal load.
* In amnesia mode, verify the sink is memory/stdout only (no files).

---

## 10) Cardinality Guardrails

* **Allowed label sets:**

  * `route` — normalized, finite set only.
  * `provider` — enum: `github|stripe|slack_webhook|slack_rest`.
  * `reason` — closed taxonomy (see §1.1).
  * **Prohibited:** request IDs, user IDs, file hashes as labels (appear as fields in logs, not labels).

---

## 11) Runbooks (pointer index)

* **Webhook verify fails:** Check `webhook_verify_total{result="fail"}` and logs for `BAD_ORIGIN` or signature mismatch; confirm provider secret rotation.
* **Brownout engaged:** Inspect `upstream_rpc_seconds` p95 and `queue_depth{work}`; if upstream `passport` is slow, consider partial traffic shed or raise *passport* SLO.
* **Queue shedding:** Look at `queue_dropped_total{queue="audit"}`; if rising, increase sink capacity or move to stdout in amnesia mode.
* **High 5xx:** Correlate `rejected_total{reason}` with `http_requests_total{status="5xx"}`; check error taxonomy mapping.

---

## 12) Maintenance

* Update this file whenever a new route, reason code, or queue is added.
* Dashboards & alerts should be version-controlled alongside this doc.
* Review cadence: every 90 days or after any incident affecting `svc-interop`.

```

