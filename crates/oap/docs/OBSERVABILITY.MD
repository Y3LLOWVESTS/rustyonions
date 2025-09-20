
---

# 📈 OBSERVABILITY.md (Template)

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

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (every service)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram)
* `inflight_requests{route}` (Gauge or implied by concurrency limit)
* `bus_lagged_total` (Counter) — broadcast backlog dropped
* `service_restarts_total` (Counter) — supervised restarts
* `request_latency_seconds` (Histogram) — p95 latency tracked
* `rejected_total{reason}` (Counter) — quota/timeouts/errors

### 1.2 Service-Specific (examples)

* **Storage:** `storage_get_latency_seconds`, `chunks_written_total`, `integrity_fail_total`
* **DHT:** `ron_dht_lookup_ms`, `ron_dht_missing_records`, `ron_dht_repair_bytes_total`
* **Mailbox:** `mailbox_visibility_timeout_total`, `mailbox_dlq_total`
* **Gateway:** `quota_exhaustions_total`, `bytes_in_total`, `bytes_out_total`

### 1.3 Registration Discipline

* Metrics **registered once** in `Metrics::new()`; clone handles everywhere.
* CI grep prevents duplicate registration.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `/healthz` — liveness (always `200 OK` if process alive).
* `/readyz` — readiness (only `200 OK` once: config loaded, DB open, listeners bound, bus attached).

### 2.2 Readiness Keys

* `db_open`
* `bus_attached`
* `tor_bootstrap` (when applicable)
* `hs_published` (for onion services)

### 2.3 Failure Semantics

* Fail-open reads / fail-closed writes (per Final Blueprint).
* Degraded mode: `503` with JSON `{ "degraded": true, "missing": ["<keys>"], "retry_after": <s> }`.

---

## 3) Logs

### 3.1 Format

* JSON lines (`application/jsonl`), one event per line.
* Required fields:

  * `ts` (ISO8601)
  * `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `service`
  * `event` (e.g., `oap_frame`, `quota_exhausted`, `service_crashed`)
  * `reason` (string; must align with `rejected_total{reason}`)
  * `corr_id` (UUID/ULID, propagated across services)
  * `latency_ms` (if applicable)
  * `content_id` (for storage/overlay ops)

### 3.2 Redaction & Secrets

* No PII in logs.
* Keys, caps, and payloads never logged.
* Config snapshot logs redact secrets (per Hardening Blueprint).

---

## 4) Tracing & Correlation

* Use `tracing` crate with `tracing-subscriber` JSON formatter.
* Span naming convention:

  * `svc.<crate>.<operation>` (e.g., `svc.gateway.get_object`)
* Correlation IDs:

  * Injected on ingress (HTTP `X-Corr-ID`)
  * Propagated via bus events and SDK.
* OpenTelemetry exporters optional (feature-flagged).

---

## 5) Alerts & SLOs

### 5.1 Standard SLOs

* Public GET p95 latency < 80 ms intra-region, < 200 ms inter-region.
* 5xx error rate < 0.1%.
* 429/503 < 1%.
* RF(observed) ≥ RF(target).

### 5.2 Alerts (examples)

* `ron_dht_missing_records > 0 for 10m` (warning), 30m (critical).
* `bus_lagged_total > 0` sustained.
* `quota_exhaustions_total > 100/min` (burst detection).
* `integrity_fail_total > 0` (critical).

### 5.3 Runbooks

Each alert links to `RUNBOOK.md` for triage & recovery steps.

---

## 6) CI / Enforcement

* CI greps ensure:

  * All crates expose `/metrics`, `/healthz`, `/readyz`.
  * `rejected_total{reason}` present when `ServiceCrashed{reason}` emitted.
* Lints for `await_holding_lock` & readiness gating (no sleeps).
* Docs updated every 90 days.

---

✅ With this template, every crate will emit **consistent metrics/logs/traces**, making cross-cutting dashboards trivial and incident response fast.
