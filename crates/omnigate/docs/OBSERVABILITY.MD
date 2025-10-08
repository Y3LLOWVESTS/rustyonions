---

title: OBSERVABILITY.md — omnigate
crate: omnigate (impl: svc-gateway)
owner: Stevan White
last-reviewed: 2025-10-07
status: draft
msrv: 1.80.0 (Tokio/loom compatible)
------------------------------------

# 📈 OBSERVABILITY — omnigate

*Audience: developers, operators, auditors*

This file defines **what is observable**, **how we expose it**, and **how it’s used** for omnigate:

* Metrics (Prometheus/OTEL)
* Health & readiness semantics
* Logs (JSON schema, fields)
* Tracing spans & correlation
* Alerts & SLOs
* CI enforcement to avoid drift

> Edge canon: quotas/DRR admission happens **before** heavy work; failures are **deterministic** (`429/400/413/503`) and **explainable** via metrics + logs.

---

## 0) Purpose

Create a **single source of truth** for operations. Everything here must be:

* **Deterministic** (same event → same counters/fields).
* **Low-cardinality** (labels bounded).
* **Minimal overhead** (histogram buckets curated).

---

## 1) Metrics (Prometheus-style)

> Registration: all metrics are registered **once** in `Metrics::new()`; handles are cloned. CI greps for duplicate registrations.

### 1.1 Golden Metrics (every service)

* `http_requests_total{route,method,status,class}` (Counter)
* `request_latency_seconds{route,method,class}` (Histogram; see buckets below)
* `inflight_requests{route}` (Gauge) — implied by concurrency limit
* `bus_lagged_total` (Counter) — broadcast backlog dropped
* `service_restarts_total{kind}` (Counter) — supervised restarts
* `rejected_total{reason,route,class}` (Counter) — quota/timeouts/errors (canonical reasons below)

**Canonical `reason` values (bounded):**
`quota | busy | body_cap | decompress_cap | ratio_cap | unauth | degraded | reload_failed | tls_handshake | bad_request | downstream_timeout | downstream_5xx`

### 1.2 Gateway-Specific

* **Admission / DRR**

  * `fq_inflight{class}` (Gauge) — current in-flight per class (`anon|authed|internal`)
  * `fq_tokens{class}` (Gauge) — DRR tokens snapshot (optional)
  * `quota_exhaustions_total{scope}` (Counter) — `global|per_ip|per_cap`
* **Bytes & Encoding**

  * `bytes_in_total{route}` (Counter)
  * `bytes_out_total{route}` (Counter)
  * `decompress_attempts_total{codec}` (Counter)
  * `decompress_reject_total{reason}` (Counter) — `ratio_cap|absolute_cap|unknown_codec`
* **Auth / TLS / PQ**

  * `auth_failures_total{issuer,reason}` (Counter) — `expired|not_yet_valid|aud_mismatch|sig_invalid|missing`
  * `tls_handshake_failures_total{reason}` (Counter) — `no_sni|invalid_cert|protocol`
  * `pq_fallback_total{peer}` (Counter) — hybrid mode accepted classical-only peer (policy `allow`)
* **Downstream Hydration**

  * `downstream_requests_total{service,method,status}` (Counter)
  * `downstream_latency_seconds{service,method}` (Histogram)
  * `downstream_errors_total{service,kind}` (Counter) — `timeout|5xx|bad_gateway|unreachable`
* **Config / Readiness**

  * `reload_failed_total` (Counter)
  * `readyz_degraded` (Gauge: 0/1)
  * `queue_depth{queue}` (Gauge) — `work|events`
  * `queue_dropped_total{queue}` (Counter)

### 1.3 Histogram Buckets (curated)

* `request_latency_seconds`: `0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5, 1, 2`
* `downstream_latency_seconds`: `0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5, 1, 2`
* Rationale: inter-zone budget p95 ≤ 200–300 ms; short buckets preserve p50/p90 resolution with low cardinality.

### 1.4 Registration Discipline

* **One place** (`Metrics::new`) creates all instruments.
* Labels are **bounded enums** (no raw user IDs, IPs, or content hashes).
* Unit tests assert help text and label sets don’t change unintentionally.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — liveness. Always `200 OK` if process alive.
* `GET /readyz` — readiness. `200 OK` only when **all required keys** are true.

### 2.2 Readiness Keys (gateway)

* `listener_bound` — HTTP listener bound and serving.
* `tls_ok` — TLS configured/valid when `tls.enabled=true`.
* `config_loaded` — effective config computed and validated.
* `passport_ok` — capability verifier reachable (or last-good cache valid).
* `downstream_health_ok` — all configured downstream `/healthz` pass **or** strict boot off & degraded set.
* `queue_headroom_ok` — `fq_inflight{class}` below `max_inflight` − safety margin.
* `reload_ok` — last reload succeeded (else set `reload_failed_total` and readiness fails).

**Truth table (summary):**

| Key                  | Required? | Ready action                                                   |
| -------------------- | --------- | -------------------------------------------------------------- |
| listener_bound       | Yes       | If false → `503`                                               |
| tls_ok (if enabled)  | Yes       | If false → `503`                                               |
| passport_ok          | Yes       | If false → `503`                                               |
| downstream_health_ok | Yes*      | If strict-boot=off and failing → `503` with `"degraded": true` |
| queue_headroom_ok    | Yes       | If false → `503` (shed)                                        |
| reload_ok            | Yes       | If false → `503`                                               |

### 2.3 Failure Semantics

* **Fail-open reads / fail-closed writes**:

  * GETs may continue in degraded mode; mutating routes are **shed** first.
* `/readyz` body (JSON) on failure:

  ```json
  {
    "ready": false,
    "degraded": true,
    "missing": ["downstream_health_ok"],
    "retry_after": 5
  }
  ```

---

## 3) Logs

### 3.1 Format

* **JSON lines**, `application/jsonl`, one event per line.
* **Required fields** (bounded names; snake_case):

  * `ts` (RFC3339)
  * `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `service` (e.g., `"omnigate"`)
  * `event` (machine key; e.g., `admit`, `quota_exhausted`, `downstream_call`, `auth_failed`)
  * `route`, `method`, `status` (if HTTP)
  * `class` (`anon|authed|internal`)
  * `corr_id` (ULID/UUID v4)
  * `req_id` (per-request id; new per hop)
  * `latency_ms` (if applicable)
  * `reason` (must match `rejected_total{reason}`)
  * `peer_addr` (anonymized / truncated)
  * `version` (semver of service)
* **Optional fields**:

  * `service_target` (`index|storage|mailbox|overlay|dht|passport`)
  * `bytes_in`, `bytes_out`
  * `pq_fallback` (bool)
  * `retry_count`

### 3.2 Redaction & Secrets

* **Never** log: tokens, PEMs, full Authorization headers, raw request bodies.
* Redact structured snapshots (`<REDACTED>`); log only key presence/length.
* PII: do not log; where unavoidable for debugging, hash with stable salt and gate behind `debug_pi` feature (dev-only).

### 3.3 Examples

* **Admission reject (quota)**:

  ```json
  {"ts":"2025-10-07T16:53:47.134Z","level":"INFO","service":"omnigate",
   "event":"quota_exhausted","route":"/o/{addr}","method":"GET","class":"anon",
   "status":429,"reason":"quota","corr_id":"01J9Z7...","req_id":"01J9Z7..."}
  ```

* **Downstream success**:

  ```json
  {"ts":"2025-10-07T16:53:48.021Z","level":"INFO","service":"omnigate",
   "event":"downstream_call","service_target":"storage","method":"GET","status":200,
   "latency_ms":72,"corr_id":"01J9Z7...","req_id":"01J9Z7..."}
  ```

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` JSON formatter.
* **Span naming**: `svc.gateway.<operation>` (e.g., `svc.gateway.admit`, `svc.gateway.hydrate.storage.get`)
* **Key span attributes**: `route`, `class`, `corr_id`, `peer_addr`, `pq_mode`, `quota_scope`, `downstream=storage|index|...`
* **Correlation IDs**:

  * Ingress: read `X-Corr-ID`; if missing, generate ULID; echo back as `X-Corr-ID`.
  * Per hop (`req_id`): always new; include `X-Request-ID`.
* **Propagation**:

  * Inject `X-Corr-ID`,`X-Request-ID` into downstream requests.
  * Optional OTEL exporter (feature `otel`): map spans/attrs to OTLP; respect sampling rate (default head 1–5%).

---

## 5) Alerts & SLOs

### 5.1 SLOs (starter budgets)

* **Public GET p95**: ≤ **80 ms** intra-region; ≤ **200 ms** inter-region.
* **End-to-end p95** (typical mix): ≤ **300 ms** under nominal load.
* **5xx rate**: < **0.1%**.
* **429/503 rate**: < **1%** sustained.
* **Ready** time: ≥ **99.9%** monthly.

### 5.2 PromQL Alert Examples

* **Quota bursts (warning):**

  ```promql
  sum(rate(quota_exhaustions_total[5m])) > 100
  ```
* **Downstream instability (critical):**

  ```promql
  sum(rate(downstream_errors_total{kind=~"timeout|unreachable|5xx"}[5m])) > 50
  ```
* **Degraded readiness (page if 5m+):**

  ```promql
  max_over_time(readyz_degraded[5m]) == 1
  ```
* **Latency SLO breach (warning):**

  ```promql
  histogram_quantile(0.95, sum by (le) (rate(request_latency_seconds_bucket[5m]))) > 0.3
  ```
* **TLS issues:**

  ```promql
  rate(tls_handshake_failures_total[5m]) > 0
  ```

> Each alert must link to `RUNBOOK.md` with *what changed*, *first checks*, *rollback/mitigation*.

---

## 6) Health & Readiness — Operational Details

* `/metrics` should bind **localhost/private** only.
* `/readyz` flips **degraded** on:

  * failed config reload (`reload_failed_total` incremented),
  * downstream health failing (strict boot off),
  * DRR saturation (`fq_inflight{class}` at max for > N seconds).
* Shed writes first in degraded mode; GETs may pass if safe.

---

## 7) CI / Enforcement

* **Unit tests** assert:

  * All canonical `reason` variants are covered and mapped from code paths.
  * Metric labels are from **finite enums** (no unbounded cardinality).
* **Golden tests** on:

  * `/readyz` JSON schema.
  * Log field presence for representative events.
* **Lint gates**:

  * `-D clippy::await_holding_lock`
  * deny duplicate metric registration (simple grep + run-time panic test).
* **Drift guards**:

  * Script verifies that `CONFIG.md` capacities (e.g., `work=512`) match `queue_depth` gauges and channel construction.

---

## 8) OTEL (Optional)

* Feature `otel` enables OTLP exporter:

  * Resource attributes include: `service.name=omnigate`, version, region, node role.
  * Span events for `admit`, `reject`, `downstream_call`, `reload`.
  * Sampling defaults: head 5% (configurable), tail-based sampling recommended for errors.

---

## 9) Example `/readyz` & `/healthz` Bodies

* `/healthz`:

  ```json
  {"ok": true, "uptime_s": 12345}
  ```
* `/readyz` (OK):

  ```json
  {"ready": true, "degraded": false}
  ```
* `/readyz` (degraded):

  ```json
  {"ready": false, "degraded": true, "missing": ["downstream_health_ok"], "retry_after": 5}
  ```

---

## 10) Mermaid — Observability Flow

```mermaid
flowchart LR
  IN[Ingress] --> ADM{Admit?}
  ADM -- No --> REJ[Reject 4xx/5xx]
  ADM -- Yes --> HYD[Hydrate (downstream)]
  HYD --> OUT[Response]
  subgraph Signals
    MET[Prometheus metrics]
    LOG[JSON logs]
    TRC[Tracing/OTEL]
  end
  IN -. emit .-> MET
  ADM -. emit .-> MET
  REJ -. emit .-> MET
  HYD -. emit .-> MET
  OUT -. emit .-> MET
  IN -. log .-> LOG
  REJ -. log .-> LOG
  HYD -. span .-> TRC
  ADM -. span .-> TRC
```

**Text:** Every phase emits metrics; rejects log structured reasons; hydration and admission emit spans; readiness reflects the system’s ability to accept work.

---

## 11) Implementation Notes

* Prefer **`prometheus` crate** counters/gauges/histograms; register at start.
* Histograms: avoid per-route cardinality explosion—use a **small route set** (static names or templated groups).
* Wrap HTTP layers with:

  * request timer,
  * in-flight gauge,
  * admission → rejection metric mapping,
  * decompression guards that increment `decompress_reject_total`.
* Metrics **must not** depend on user input for label values.

---

## 12) Maintenance

* Review this file **every 90 days** or on any observable change.
* When adding a new rejection path, **update**:

  1. `rejected_total{reason}` enum list,
  2. log `reason` mapping,
  3. RUNBOOK alert sections,
  4. CI golden tests.

---

**Outcome:** With these signals, omnigate’s behavior is **predictable** under stress, **explainable** during incidents, and **provable** (via CI) against drift. Operators get crisp alerts; auditors get deterministic logs; developers get actionable spans and histograms.
