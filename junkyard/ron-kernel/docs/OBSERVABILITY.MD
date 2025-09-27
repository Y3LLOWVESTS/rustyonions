

---

````markdown
# 📈 OBSERVABILITY.md — ron-kernel

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

The kernel is orchestration only (supervision, bus, config, health, metrics).  
Protocol/data-plane metrics are **delegated to services**; the kernel exposes **golden signals**.

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics

The kernel must expose the following counters/gauges/histograms:

* `bus_lagged_total` (Counter) — dropped events due to slow consumers
* `service_restarts_total{service}` (Counter) — supervised child restarts
* `request_latency_seconds{op}` (Histogram) — latency for kernel ops (`config_reload`, `bus_publish`, `/readyz`)
* `busy_rejections_total{queue}` (Counter) — send failures due to backpressure
* `queue_dropped_total{queue}` (Counter) — items dropped due to overflow
* `tasks_spawned_total{kind}`, `tasks_aborted_total{kind}` (Counters) — task lifecycle events

> **Note:** HTTP-style metrics (`http_requests_total`, `inflight_requests`) apply only to the **demo observability bin**, not the core library.

### 1.2 Registration Discipline

* All metrics registered once in `Metrics::new()`.  
* Clone handles everywhere (`Arc<Metrics>`).  
* CI ensures no duplicate registration (`cargo deny`/lint).

---

## 2) Health & Readiness

### 2.1 Endpoints (demo bin)

* `/healthz` — process alive. Always `200 OK` if process is running.
* `/readyz` — readiness = `200 OK` only if:
  - config loaded
  - bus initialized
  - supervisor children within restart caps

### 2.2 Failure Semantics

* Fail-open reads; fail-closed writes (per Final Blueprint).  
* Degraded mode: return `503` with JSON:
  ```json
  { "degraded": true, "missing": ["bus_attached"], "retry_after": 5 }
````

---

## 3) Logs

### 3.1 Format

* **JSON lines** (`application/jsonl`), one event per line.
* Required fields:

  * `ts` (ISO8601 UTC)
  * `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `service="ron-kernel"`
  * `event` (e.g., `service_crashed`, `config_updated`, `shutdown`)
  * `reason` (string; aligns with `rejected_total{reason}`)
  * `corr_id` (UUID/ULID, propagated if present)
  * `latency_ms` (if measured)
  * `amnesia` (`on|off`)

### 3.2 Redaction

* No secrets/keys logged.
* Config diffs redact sensitive values.
* Errors log only **key identifiers**, never raw contents.

---

## 4) Tracing & Correlation

* Use `tracing` crate with JSON `tracing-subscriber`.
* Span naming: `kernel.<component>.<operation>`

  * e.g., `kernel.bus.publish`, `kernel.supervisor.restart`.
* Correlation IDs:

  * Ingested if present (e.g., via service bus or CLI test rig).
  * Propagated into all bus events and spans.
* OTEL exporters behind a `telemetry` feature flag.

---

## 5) Alerts & SLOs

### 5.1 Kernel SLOs

* Bus publish→deliver p99 < **200µs** @ 8 receivers.
* Config reload p95 < **10ms**.
* Supervisor restart detection < **25ms**.
* Bus overflow ratio < **0.01%**.

### 5.2 Alerts (examples)

* `bus_lagged_total > 0 sustained 5m` — slow consumers.
* `service_restarts_total{service} > 5 in 5m` — crash loop.
* `/readyz` returns 503 > 1m — degraded kernel.
* `busy_rejections_total{queue}` > threshold — backpressure overload.

### 5.3 Runbooks

Every alert must link to [RUNBOOK.md](./RUNBOOK.md) for triage/recovery steps.

---

## 6) CI / Enforcement

* CI must confirm:

  * Golden metrics exported (`bus_lagged_total`, `service_restarts_total`, etc.).
  * `/metrics`, `/healthz`, `/readyz` present in demo bin.
  * Structured JSON logs (no bare printlns).
* Lints:

  * `clippy::await_holding_lock` enforced (protects metrics paths).
  * Deny secret logging via regex grep in CI (`key=`, `token=`, etc.).
* Observability docs refreshed every 90 days; drift check in PR template.

---

✅ With this, `ron-kernel` exposes consistent **golden metrics, structured logs, and spans**, enabling **cross-crate dashboards** and fast incident triage.

```

---

