

# 📈 OBSERVABILITY.md — macronode

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

macronode must surface **uniform observability signals** across its composed services and runtime profile.
This doc defines:

* Metrics (Prometheus/OTEL export)
* Health & readiness semantics
* Logs (JSON schema, redaction)
* Tracing spans & correlation rules
* Alerts & SLOs (operator-ready)

This ensures macronode deployments remain auditable, drift-proof, and SRE-ready under the **Six Concerns spine** (RES, PERF, GOV).

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (always present)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram, p95 tracked)
* `inflight_requests{route}` (Gauge; concurrency cap visible)
* `bus_lagged_total` (Counter; dropped broadcast backlog)
* `service_restarts_total` (Counter; crash-only supervision)
* `rejected_total{reason}` (Counter; quota/timeouts/errors, matches log.reason)

### 1.2 macronode Profile Metrics

* Aggregates from child services:

  * `ron_dht_lookup_ms`, `ron_dht_missing_records`, `ron_dht_repair_bytes_total`
  * `storage_get_latency_seconds`, `chunks_written_total`, `integrity_fail_total`
  * `mailbox_visibility_timeout_total`, `mailbox_dlq_total`
  * `quota_exhaustions_total`, `bytes_in_total`, `bytes_out_total`
* Node-profile SLO gauges:

  * `rf_observed` vs `rf_target` (replication factor)
  * `facet_latency_seconds{facet=graph|search|feed|media}`

### 1.3 Registration Discipline

* Metrics are **registered once** (via `ron-metrics::Metrics::new()`); clones used elsewhere.
* CI greps deny duplicate registration (see Concurrency v1.3 rules).

---

## 2) Health & Readiness

### 2.1 Endpoints

* `/healthz` → liveness (200 if process alive).
* `/readyz` → readiness: 200 only when config, DB, listeners, bus, and profile services are up.

### 2.2 Readiness Keys

* `db_open`
* `bus_attached`
* `dht_bootstrap`
* `rf_target_met`
* `tor_bootstrap` and `hs_published` (when onion profile enabled)

### 2.3 Failure Semantics

* Fail-open on reads, fail-closed on writes (per Final + Hardening v2.0).
* Degraded mode: `503` + JSON:

```json
{ "degraded": true, "missing": ["dht_bootstrap"], "retry_after": 10 }
```

---

## 3) Logs

### 3.1 Format

Structured JSONL, one event per line:

* `ts` — ISO8601 UTC
* `level` — INFO|WARN|ERROR|DEBUG|TRACE
* `service` — crate/service name (`svc-gateway`, `svc-dht`, etc.)
* `event` — semantic label (`oap_frame`, `quota_exhausted`, `service_crashed`)
* `reason` — string (aligns with `rejected_total{reason}`)
* `corr_id` — UUID/ULID (propagated)
* `latency_ms` — optional numeric
* `content_id` — for storage/overlay ops

### 3.2 Redaction

* No PII, keys, or payloads logged.
* Config logs redact secrets (KMS, macaroon sigs, Tor keys).
* Amnesia mode: logs ephemeral, zeroized on shutdown (Micronode default; Macronode optional).

---

## 4) Tracing & Correlation

* Use `tracing` crate with `tracing-subscriber` JSON formatter.
* Span naming: `svc.<crate>.<operation>` (e.g., `svc.gateway.get_object`).
* Correlation IDs:

  * Injected on ingress (`X-Corr-ID` header).
  * Propagated across bus and SDK calls.
* Optional OTEL exporters (behind `otel` feature).

---

## 5) Alerts & SLOs

### 5.1 Standard SLOs

* Public GET p95 latency < 80 ms intra-region, < 200 ms inter-region.
* 5xx error rate < 0.1%.
* Quota errors (429/503) < 1%.
* `rf_observed >= rf_target` sustained.

### 5.2 Alerts

* `ron_dht_missing_records > 0 for 10m` (warn), 30m (crit).
* `bus_lagged_total > 0` sustained.
* `quota_exhaustions_total > 100/min`.
* `integrity_fail_total > 0` (critical).
* `service_restarts_total > 5 in 5m` (crash storm).

### 5.3 Runbooks

Each alert links to `RUNBOOK.md` with triage & recovery steps (see Scaling runbooks for DHT failover & RF repair).

---

## 6) CI / Enforcement

* CI greps verify:

  * `/metrics`, `/healthz`, `/readyz` exist.
  * `rejected_total{reason}` aligns with `ServiceCrashed{reason}`.
* Clippy lint wall denies: `await_holding_lock`, `unwrap_used`, `expect_used`.
* ThreadSanitizer mandatory on critical crates (gateway, overlay, dht, storage).
* Loom interleaving tests for readiness DAG.
* Docs & dashboards refreshed every 90 days.

---

✅ With this template, macronode emits **uniform observability** across its services, ensuring ops teams can build dashboards, respond to incidents, and audit deployments without drift.
