# 📈 OBSERVABILITY.md — svc-passport

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus/OTel)
* Health & readiness semantics
* Logs (JSON schema, fields & redaction)
* Tracing spans & correlation
* Alerts, SLOs, and runbooks
* CI enforcement to prevent drift

`svc-passport` mints **capability-scoped tokens** with **short TTLs**, performs local preflight verification (authoritative verification is in `ron-auth`), and coordinates revocation/rotation with `ron-kms` and `svc-registry`. Observability emphasizes **security outcomes**, **latency SLOs**, and **revocation propagation**.

---

## 1) Metrics (Prometheus-style)

All metrics live under the `svc_passport_` namespace unless noted. Exposed on `/metrics` (text format).

### 1.1 Golden Metrics (every service)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram)
* `inflight_requests{route}` (Gauge)
* `bus_lagged_total` (Counter) — broadcast backlog dropped
* `service_restarts_total` (Counter) — supervised restarts
* `rejected_total{reason}` (Counter) — quota/timeouts/errors

### 1.2 Service-Specific (passport)

**Issuance / Verify / Revoke**

* `svc_passport_issue_latency_seconds{result}` (Histogram)
  Buckets: `le={0.005,0.01,0.02,0.04,0.06,0.1,0.2,0.5,1.0}`
* `svc_passport_verify_latency_seconds{result}` (Histogram)
  Buckets: `le={0.001,0.003,0.005,0.01,0.02,0.05,0.1}`
* `svc_passport_revoke_latency_seconds{result}` (Histogram)
* `svc_passport_tokens_issued_total{aud,svc,alg}` (Counter)
* `svc_passport_revocations_total{reason}` (Counter)
* `svc_passport_rejects_total{reason}` (Counter)
  Reasons include: `policy_violation`, `ttl_too_long`, `unknown_caveat`, `over_limit`, `ratio_cap`, `pq_negotiation`, `busy`, `kms_timeout`, `registry_unavailable`

**Queues / Backpressure**

* `svc_passport_queue_depth{queue}` (Gauge — sampled)
  Queues: `issue`, `revoke`
* `svc_passport_queue_dropped_total{queue}` (Counter)
* `busy_rejections_total{endpoint}` (Counter) — HTTP 429 due to bounded mpsc full

**KMS / Registry / Policy**

* `svc_passport_kms_call_latency_seconds{op}` (Histogram; `op="sign"|"rotate"` )
* `svc_passport_kms_failures_total{op,reason}` (Counter)
* `svc_passport_registry_fetch_latency_seconds` (Histogram)
* `svc_passport_registry_failures_total{reason}` (Counter)
* `svc_passport_policy_update_total{source="bus|poll"}` (Counter)

**Rotation / Propagation**

* `svc_passport_epoch_current` (Gauge — epoch number)
* `svc_passport_revocation_propagation_seconds` (Histogram)
  (event→downstream-refusal observed via synthetic probe)

**Config / Health**

* `config_reloads_total` (Counter)
* `config_reload_errors_total` (Counter)
* `config_reload_duration_seconds` (Histogram)
* `readiness_state{component}` (Gauge: `1=ready,0=not-ready`)
  Components: `issue_path`, `verify_preflight`, `kms`, `registry`, `queues`

**TLS (if enabled)**

* `tls_handshake_failures_total` (Counter)

> **Registration discipline:** All metrics are registered **once** in `Metrics::new()`; cloned handles are passed to tasks. CI guards prevent duplicate `register_*` calls.

### 1.3 Exemplars & Tracing Links (optional)

If OTel is enabled, attach trace IDs as exemplars to latency histograms when available.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness**: returns `200 OK` if the process event loop is responsive.
* `GET /readyz` — **readiness**: returns `200 OK` only if:

  * Config snapshot is valid and current
  * Listeners bound
  * KMS/Registry probed healthy (within last `probe_ttl`)
  * Queues under thresholds (e.g., `< 80% capacity`)
  * For **degraded** mode: verification preflight may remain ready while issuance sheds

### 2.2 Keys contributing to readiness

* `kms_ok`, `registry_ok`, `issue_queue_ok`, `revoke_queue_ok`, `policy_sync_ok`

`/readyz` body (JSON):

```json
{
  "ready": false,
  "degraded": true,
  "missing": ["kms_ok"],
  "retry_after": 5,
  "components": {
    "issue_path": false,
    "verify_preflight": true
  }
}
```

### 2.3 Failure Semantics

* **Fail-closed writes**: issuance/revoke go Not Ready on KMS/registry failure or queue saturation.
* **Fail-open reads**: verification **preflight** may stay Ready to help the system drain and diagnose.
* **Backoff headers**: `Retry-After` set on `429/503`.

---

## 3) Logs

### 3.1 Format (JSONL)

* One event per line; UTF-8; `application/jsonl`.

**Required fields**

| Field           | Type             | Notes                                                                                                    |       |       |           |        |
| --------------- | ---------------- | -------------------------------------------------------------------------------------------------------- | ----- | ----- | --------- | ------ |
| `ts`            | string (ISO8601) | UTC                                                                                                      |       |       |           |        |
| `level`         | string           | `INFO                                                                                                    | WARN  | ERROR | DEBUG     | TRACE` |
| `service`       | string           | `svc-passport`                                                                                           |       |       |           |        |
| `event`         | string           | e.g., `issue.request`, `issue.success`, `issue.reject`, `revoke.success`, `policy.updated`, `ready.flip` |       |       |           |        |
| `route`         | string           | `/v1/passport/issue` etc.                                                                                |       |       |           |        |
| `corr_id`       | string           | ULID/UUID; from `X-Corr-ID` or generated                                                                 |       |       |           |        |
| `peer_addr`     | string           | redacted for privacy where needed                                                                        |       |       |           |        |
| `latency_ms`    | number           | if applicable                                                                                            |       |       |           |        |
| `result`        | string           | `ok                                                                                                      | error | busy  | rejected` |        |
| `reason`        | string           | aligns with `svc_passport_rejects_total{reason}`                                                         |       |       |           |        |
| `kid`           | string           | key id (non-secret)                                                                                      |       |       |           |        |
| `epoch`         | number           | current epoch                                                                                            |       |       |           |        |
| `alg`           | string           | e.g., `ed25519`, `ed25519+ml-dsa`                                                                        |       |       |           |        |
| `aud`           | string           | audience (non-PII)                                                                                       |       |       |           |        |
| `svc`           | string           | target service (from caveat)                                                                             |       |       |           |        |
| `caveats_count` | number           | count only (no values)                                                                                   |       |       |           |        |
| `pq_fallback`   | bool             | set if hybrid unavailable and classical used                                                             |       |       |           |        |
| `cfg_version`   | number           | config snapshot id                                                                                       |       |       |           |        |

**Examples**

```json
{"ts":"2025-10-09T14:22:18Z","level":"INFO","service":"svc-passport","event":"issue.success","route":"/v1/passport/issue","corr_id":"01J8...","latency_ms":23,"result":"ok","kid":"issuer-v1","epoch":42,"alg":"ed25519+ml-dsa","aud":"svc-mailbox","svc":"svc-mailbox","caveats_count":4,"pq_fallback":false}
```

```json
{"ts":"2025-10-09T14:22:20Z","level":"WARN","service":"svc-passport","event":"issue.reject","route":"/v1/passport/issue","corr_id":"01J8...","latency_ms":1,"result":"rejected","reason":"ttl_too_long","aud":"svc-mailbox","cfg_version":177}
```

### 3.2 Redaction & Secrets

* **Never** log tokens, private keys, macaroon contents, or user identifiers/PII.
* Redact config diffs; secret-bearing fields are replaced with `"***"`.
* Limit IP/peer logging to **coarse** addresses or hash when required by policy.

---

## 4) Tracing & Correlation

* Use `tracing`/`tracing-subscriber` JSON formatter.
* **Span names (convention):**

  * `svc.passport.issue` (server span)
  * `svc.passport.verify` (server span)
  * `svc.passport.revoke` (server span)
  * `svc.passport.kms.sign` (client span)
  * `svc.passport.registry.fetch` (client span)
  * `svc.passport.bus.publish` (client span)
* **Fields on spans:**

  * `corr_id`, `kid`, `epoch`, `alg`, `aud`, `svc`, `result`, `reason`
* **Correlation IDs:**

  * Ingest from `X-Corr-ID`; generate ULID if missing; propagate on bus events.
* **OTel (optional via feature flag):**

  * Map spans to semantic `rpc.server` and `rpc.client` where applicable.
  * Export via OTLP/HTTP if configured; attach exemplars to latency histograms.

---

## 5) Alerts & SLOs

### 5.1 SLOs (per instance, steady-state)

* **Issuance latency:** p95 ≤ **40ms**, p99 ≤ **100ms**
* **Verify latency:** p95 ≤ **10ms**, p99 ≤ **25ms**
* **Revocation propagation:** ≤ **5s** p99 from event to downstream refusal
* **5xx error rate:** < **0.1%**
* **429/503 rate:** < **1%** sustained
* **Config reload:** p95 < **100ms**, p99 < **250ms**

### 5.2 Prometheus Alert Rules (examples)

```yaml
groups:
- name: svc-passport-slos
  rules:
  - alert: PassportIssueLatencyHigh
    expr: histogram_quantile(0.99, sum(rate(svc_passport_issue_latency_seconds_bucket[5m])) by (le)) > 0.1
    for: 5m
    labels: {severity: page}
    annotations:
      summary: "svc-passport issuance p99 > 100ms"
      runbook: "RUNBOOK.md#passport-issue-latency"

  - alert: RevocationPropagationSlow
    expr: histogram_quantile(0.99, sum(rate(svc_passport_revocation_propagation_seconds_bucket[5m])) by (le)) > 5
    for: 5m
    labels: {severity: page}
    annotations:
      summary: "Revocation propagation p99 > 5s"
      runbook: "RUNBOOK.md#revocation-propagation"

  - alert: BackpressureSustained
    expr: sum(rate(busy_rejections_total{endpoint="/v1/passport/issue"}[5m])) > 1
    for: 10m
    labels: {severity: warn}
    annotations:
      summary: "Sustained 429s on issuance"
      runbook: "RUNBOOK.md#backpressure"

  - alert: KMSFailuresBurst
    expr: sum(rate(svc_passport_kms_failures_total[5m])) > 5
    for: 10m
    labels: {severity: page}
    annotations:
      summary: "KMS failures burst"
      runbook: "RUNBOOK.md#kms-failures"

  - alert: ConfigReloadSlowOrFailing
    expr: histogram_quantile(0.99, sum(rate(config_reload_duration_seconds_bucket[5m])) by (le)) > 0.25
      or increase(config_reload_errors_total[10m]) > 0
    for: 10m
    labels: {severity: warn}
    annotations:
      summary: "Config reloads slow or failing"
      runbook: "RUNBOOK.md#config-reload"
```

### 5.3 Runbooks (outline)

* **Issue latency high** → check KMS latency histogram, breaker state, RPS/inflight caps, queue depth.
* **Propagation slow** → inspect bus health, subscriber lag, downstream verify caches; consider forced epoch bump.
* **Backpressure sustained** → scale out replicas or lower RPS; validate client retry strategy.
* **KMS failures** → switch to fallback KMS region if policy allows; check network ACLs.
* **Config reload anomalies** → validate file/env precedence; inspect redacted diff; roll back.

---

## 6) Dashboards (suggested panels)

1. **Overview**

   * Request rate, error rate, p95/p99 issuance/verify
   * 429/503 share
2. **Backpressure**

   * Queue depth (issue/revoke), drops, busy rejections
3. **Security**

   * Rejects by reason; revocations total; epoch gauge
4. **KMS/Registry**

   * KMS call latencies/failures; registry fetch latencies/failures
5. **Propagation**

   * Revocation propagation histogram + SLO line
6. **Config/Health**

   * Reload duration histogram; reload errors; readiness state gauges

---

## 7) Implementation Notes

* **Histogram buckets** chosen to match SLOs; do **not** change without dashboard/alert updates.
* **Metric cardinality**: keep label sets small; for `aud`/`svc` only allow known values from policy registry; hash/limit otherwise.
* **Once-only registration** in `Metrics::new()`; expose a lightweight handle to tasks.
* **Exemplars** require tracing context; guard with feature flags.

---

## 8) CI / Enforcement

* **Metrics presence**: unit test that `/metrics` contains:

  * `svc_passport_issue_latency_seconds`
  * `svc_passport_verify_latency_seconds`
  * `svc_passport_rejects_total`
  * `config_reloads_total`
* **Readiness contract**: integration test flips `/readyz` Not Ready on simulated KMS outage and reverts when recovered.
* **Logging schema check**: JSON log snapshot test enforces required fields; rejects token leakage via denylist matcher.
* **Clippy**: `-W clippy::await_holding_lock` and log `reason` ↔ metric mapping.
* **Docs freshness**: this file reviewed every **90 days** or on metric/endpoint change.

---

## 9) Field Mapping (Logs ↔ Metrics ↔ Spans)

| Concept     | Logs Field            | Metric Label             | Span Field            |
| ----------- | --------------------- | ------------------------ | --------------------- |
| Correlation | `corr_id`             | exemplar (trace_id)      | `corr_id`, trace ids  |
| Outcome     | `result`              | `{result}` on histograms | `result`              |
| Reason      | `reason`              | `{reason}` on rejects    | `reason`              |
| Key         | `kid`, `epoch`, `alg` | `alg` (on tokens_issued) | `kid`, `epoch`, `alg` |
| Target      | `aud`, `svc`          | `{aud,svc}`              | `aud`, `svc`          |
| Latency     | `latency_ms`          | histograms               | span duration         |

---

## 10) Examples (Dev Harness)

**curl smoke**

```bash
curl -sS localhost:8080/readyz | jq
curl -sS localhost:8080/metrics | grep svc_passport_issue_latency_seconds | head
```

**PromQL**

* p95 issuance:

  ```
  histogram_quantile(0.95, sum(rate(svc_passport_issue_latency_seconds_bucket[5m])) by (le))
  ```
* rejects by reason (top-5):

  ```
  topk(5, sum(rate(svc_passport_rejects_total[5m])) by (reason))
  ```
* backpressure:

  ```
  sum(rate(busy_rejections_total{endpoint="/v1/passport/issue"}[5m]))
  ```

---

## 11) Red/Green Checklist (ship gate)

* [ ] `/metrics` exposes all golden + service metrics.
* [ ] `/readyz` semantics match fail-closed issuance & pass-through verify preflight.
* [ ] Logs contain required fields; **no secrets or tokens** appear in any code path.
* [ ] Tracing spans present for issue/verify/revoke + KMS/Registry client calls.
* [ ] Alerts loaded; dashboards render SLOs; runbooks referenced.
* [ ] CI tests green for metrics presence, readiness transitions, and logging schema.

---

✅ With this, `svc-passport` emits **consistent, audit-ready** telemetry that proves our invariants in real time, supports fast incident triage, and keeps PQ/rotation governance observable end-to-end.
