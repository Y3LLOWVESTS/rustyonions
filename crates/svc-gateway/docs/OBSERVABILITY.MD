
````markdown
# 📈 OBSERVABILITY.md — svc-gateway

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- Metrics (Prometheus / optional OTEL export)
- Health & readiness semantics
- Logs (JSON schema, fields, redaction)
- Tracing spans & correlation (ingress → downstream)
- Alerts & SLOs with runbook pointers

`svc-gateway` is the hardened **HTTP ↔ OAP/1 ingress**. Observability must prove edge caps (timeouts, quotas, DRR, decode guards), capability checks, and predictable degrade behavior (shed writes first).

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (required for every service)

- `http_requests_total{route,method,status}` (Counter)
- `request_latency_seconds{route,method}` (Histogram; use native buckets + exemplars)
- `inflight_requests{route}` (Gauge) — can be implied by concurrency limit layer
- `service_restarts_total` (Counter) — supervised restarts
- `rejected_total{reason}` (Counter) — canonical reasons: `unauth|quota|body_cap|decoded-cap|decoded-ratio|method|host|degraded`
- `bus_lagged_total` (Counter) — broadcast backlog dropped (if subscribed)

**Histogram buckets (suggested):** `5, 10, 20, 50, 80, 120, 200, 300, 500, 1000 ms` (seconds representation)

### 1.2 Gateway-Specific Metrics

- **Quotas / DRR**
  - `gateway_quota_exhaustions_total{tenant}` (Counter)
  - `gateway_drr_queue_depth{tenant}` (Gauge)
  - `busy_rejections_total{endpoint}` (Counter)
- **Bytes**
  - `bytes_in_total{route}` (Counter)
  - `bytes_out_total{route}` (Counter)
- **Decode Guards**
  - `decoded_reject_total{reason="decoded-cap|decoded-ratio"}` (Counter)
- **TLS / Handshake**
  - `tls_handshake_failures_total{reason}` (Counter)
- **Degrade & Readiness**
  - `gateway_degraded{bool}` (Gauge: 1 when shedding writes)
  - `readyz_fail_total{key}` (Counter; missing readiness key cause)

### 1.3 Registration Discipline

- All metrics are **registered once** during `Metrics::new()` (or module init).
- Metric handles are **cloned** into tasks/handlers; no re-registration at call sites.
- CI asserts **no duplicate registration** and that **all reason labels** used by code are present in tests (golden list).

### 1.4 Exemplars (optional but recommended)

- If using OTEL/Tracing linkage, attach exemplars (trace IDs) to latency histograms for top-N slow routes investigations.

---

## 2) Health & Readiness

### 2.1 Endpoints

- `/healthz` — **liveness** only (200 if process is alive).
- `/readyz` — **readiness** gate (200 only when all keys satisfied).
- `/metrics` — Prometheus text exposition, always best-effort.
- `/version` — build/scm metadata (no secrets).

### 2.2 Readiness Keys (svc-gateway)

- `config_loaded` — validated config snapshot is active
- `listener_bound` — HTTP/TLS sockets bound and accepting
- `metrics_bound` — metrics endpoint bound (localhost allowed)
- `policy_loaded` — capability verification key(s) present (if required)
- `downstream_accessible` — optional; ping essential dependencies if strict mode enabled

### 2.3 Failure Semantics

- **Degrade mode**: prefer read-only. `/readyz` → `503` with:
  ```json
  { "degraded": true, "missing": ["<keys>"], "retry_after": 1 }
````

* **Writes shed first**: increment `rejected_total{reason="degraded"}` and set `gateway_degraded{bool}=1`.
* `/healthz` continues to return `200` during degrade unless a fatal condition is reached (process unhealthy).

---

## 3) Logs

### 3.1 Format

* **JSON Lines** (`application/jsonl`), one event per line.
* Timestamp: RFC3339 with millis (UTC).

### 3.2 Required Fields

| Field        | Type   | Notes                                                                                  |      |       |       |        |
| ------------ | ------ | -------------------------------------------------------------------------------------- | ---- | ----- | ----- | ------ |
| `ts`         | string | ISO8601/RFC3339 UTC                                                                    |      |       |       |        |
| `level`      | string | `INFO                                                                                  | WARN | ERROR | DEBUG | TRACE` |
| `service`    | string | `svc-gateway`                                                                          |      |       |       |        |
| `event`      | string | `http_req`, `quota_exhausted`, `decoded_reject`, `cap_denied`, `readyz`, `panic`, etc. |      |       |       |        |
| `route`      | string | normalized route id (e.g., `/api/put`)                                                 |      |       |       |        |
| `method`     | string | `GET                                                                                   | POST | ...`  |       |        |
| `status`     | number | HTTP status code                                                                       |      |       |       |        |
| `reason`     | string | must align with `rejected_total{reason=...}`                                           |      |       |       |        |
| `corr_id`    | string | ULID/UUID; ingress-created if absent                                                   |      |       |       |        |
| `span_id`    | string | tracing span id (if enabled)                                                           |      |       |       |        |
| `tenant`     | string | optional; if multi-tenant label present                                                |      |       |       |        |
| `peer_addr`  | string | remote IP:port (redacted rules apply)                                                  |      |       |       |        |
| `user_agent` | string | truncated (≤256 chars)                                                                 |      |       |       |        |
| `req_bytes`  | number | request body size                                                                      |      |       |       |        |
| `resp_bytes` | number | response body size                                                                     |      |       |       |        |
| `latency_ms` | number | end-to-end duration                                                                    |      |       |       |        |

### 3.3 Redaction & Secrets

* **Never** log tokens/keys. If correlation with tokens is required, log a **hash prefix** of a non-reversible token ID.
* Truncate and redact PII-bearing headers (e.g., `Authorization`, custom secrets).
* Config-change diffs must **redact secrets** (`***`).

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` (JSON formatter).
* **Span naming**: `svc.gateway.<operation>`
  e.g., `svc.gateway.ingress`, `svc.gateway.decode`, `svc.gateway.cap_check`, `svc.gateway.forward`.
* **Ingress correlation**

  * Accept `X-Corr-ID` or `X-Request-Id`; if missing, generate ULID.
  * Propagate `X-Corr-ID` downstream.
* **W3C TraceContext (optional)**

  * If `otel` feature enabled, support `traceparent`/`tracestate` propagation.
* **Exemplars**

  * Attach trace IDs to latency histograms where the metrics backend supports exemplars.

---

## 5) Alerts & SLOs

### 5.1 SLOs (svc-gateway default; tune per environment)

* **Latency**: public GET p95 < **80 ms** intra-region; < **200 ms** inter-region.
* **Error budget**: 5xx rate < **0.1%**.
* **Backpressure**: 429/503 combined < **1%** sustained (5m).
* **Decode safety**: `decoded_reject_total` spike alerts when > **10/min**.
* **Degradation**: `gateway_degraded{bool} == 1` for > **60s** is critical.

### 5.2 PromQL Alert Examples

**High backpressure (warning)**

```promql
sum(rate(busy_rejections_total[5m])) / sum(rate(http_requests_total[5m])) > 0.01
```

**Decode bomb detection (critical)**

```promql
sum(rate(decoded_reject_total{reason=~"decoded-(cap|ratio)"}[1m])) > 10
```

**Readiness degraded (critical)**

```promql
max_over_time(gateway_degraded{bool="1"}[1m]) == 1
```

**Latency regression p95 (warning)**

```promql
histogram_quantile(0.95, sum by (le) (rate(request_latency_seconds_bucket[5m]))) > 0.15
```

**TLS handshake failures (warning)**

```promql
sum(rate(tls_handshake_failures_total[5m])) > 5
```

### 5.3 Runbooks

* Every alert must link to `RUNBOOK.md` sections:

  * **Backpressure** → check quotas/DRR config, traffic anomaly, downstream health.
  * **Decode rejections** → confirm content-encoding, malformed clients, abuse pattern.
  * **Degraded readiness** → identify missing readiness keys; evaluate downstream outages.
  * **Latency regression** → inspect hotspots with exemplars; compare to baseline JSON.

---

## 6) CI / Enforcement

* **Endpoint conformance tests** ensure:

  * `/metrics` exposes all golden metrics with expected labels.
  * `/readyz` returns 200 only when readiness keys satisfied; 503 otherwise with explanatory JSON.
* **Reason taxonomy tests**:

  * Golden list for `rejected_total{reason}`; fail CI if code emits unknown reasons.
* **Lint gates**:

  * `await_holding_lock` forbidden in handlers/supervisor.
  * Readiness gates disallow “sleep-based” probes.
* **Doc freshness**: this file is reviewed every **90 days** or on any observable change.

---

## 7) Dashboards (Operator Starter Pack)

Create a minimal Grafana dashboard with these panels:

1. **Requests & Errors**

   * `sum(rate(http_requests_total[1m])) by (status)`
2. **Latency p50/p95/p99**

   * `histogram_quantile(0.95, sum by (le)(rate(request_latency_seconds_bucket[5m])))`
3. **Backpressure**

   * `sum(rate(busy_rejections_total[5m])) by (endpoint)`
   * `sum(rate(rejected_total{reason="quota"}[5m]))`
4. **Decode Safety**

   * `sum(rate(decoded_reject_total[5m])) by (reason)`
5. **DRR Fairness**

   * `avg(gateway_drr_queue_depth) by (tenant)`
   * `sum(rate(gateway_quota_exhaustions_total[5m])) by (tenant)`
6. **Readiness & Health**

   * `max(gateway_degraded)` and `sum(rate(readyz_fail_total[5m])) by (key)`
7. **TLS Health**

   * `sum(rate(tls_handshake_failures_total[5m])) by (reason)`

---

## 8) Operator Playbook Snippets

**Confirm degrade path is active**

```bash
curl -sS http://$HOST:$PORT/readyz | jq .
```

**Scrape metrics locally**

```bash
curl -sS http://127.0.0.1:$METRICS_PORT/metrics | head -n 50
```

**Trace a single slow request (requires tracing enabled)**

```bash
# Look up recent slow spans via your tracing backend and correlate with corr_id from logs
```

---

## 9) Implementation Pointers

* **Metrics wiring**: initialize in `main()`/`Metrics::new()`; pass handles to router layers and handlers.
* **HTTP layers**: add latency histogram & inflight gauges at the router (tower-http) + custom reject metrics in error handler.
* **Readyz provider**: central readiness registry with atomic keys; route updates through config watcher.
* **Logging**: set subscriber early; ensure JSON, UTC time, and structured fields; redact secrets globally.

---

## 10) Observability Contract (What reviewers check)

* Golden metrics present with correct labels.
* Error taxonomy mirrored between logs and metrics (`reason` field).
* Readiness: shedding writes first is **observable** (metrics + endpoint behavior).
* Decode guard outcomes (ratio/abs cap) produce **deterministic** 413 + labeled metrics.
* Correlation IDs: always present end-to-end.

---

## 11) Change Log (this file)

* 2025-10-02: Initial version aligned to IDB/CONFIG/SECURITY for svc-gateway.

```
```
