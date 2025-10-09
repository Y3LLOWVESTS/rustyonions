

# 📈 OBSERVABILITY.md — svc-registry

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus & OpenTelemetry)
* Health/readiness semantics
* Logs (JSON schema & redaction)
* Tracing spans & correlation
* Alerts, SLOs, and runbooks

This document mirrors the IDB and CONFIG contracts: **read-fast, write-audited**, **non-blocking bus**, **single-writer commit**, **parametric hardening**, and **tamper-evident audit chain**.

---

## 1) Metrics (Prometheus-style)

All metrics are registered **once** in `metrics::new()` and exposed at `/metrics`. Labels use stable, low-cardinality values (no raw user IDs, tokens, or long strings).

### 1.1 Golden Metrics (every service)

| Metric                    | Type      | Labels                  | Notes                                                      |          |           |        |             |             |          |
| ------------------------- | --------- | ----------------------- | ---------------------------------------------------------- | -------- | --------- | ------ | ----------- | ----------- | -------- |
| `http_requests_total`     | Counter   | `{route,method,status}` | Route values are normalized (e.g., `/registry/{version}`)  |          |           |        |             |             |          |
| `request_latency_seconds` | Histogram | `{route,method}`        | Buckets: `[5ms,10ms,25ms,50ms,100ms,250ms,500ms,1s,2s,5s]` |          |           |        |             |             |          |
| `inflight_requests`       | Gauge     | `{route}`               | Matches concurrency cap; sampled each scrape               |          |           |        |             |             |          |
| `service_restarts_total`  | Counter   | `{component}`           | Supervisor restarts (by task)                              |          |           |        |             |             |          |
| `rejected_total`          | Counter   | `{reason}`              | Reasons: `busy                                             | body_cap | ratio_cap | unauth | invalid_sig | revoked_key | timeout` |
| `bus_lagged_total`        | Counter   | `{consumer}`            | Broadcast backlog dropped for slow consumer                |          |           |        |             |             |          |

### 1.2 Registry-Specific Metrics

| Metric                            | Type      | Labels             | What it means                             |                                  |                 |
| --------------------------------- | --------- | ------------------ | ----------------------------------------- | -------------------------------- | --------------- |
| `registry_updates_total`          | Counter   | `{result="accepted | rejected                                  | superseded"}`                    | Commit outcomes |
| `registry_pending_proposals`      | Gauge     | none               | Live count of proposals not yet committed |                                  |                 |
| `registry_signers`                | Gauge     | `{org}`            | Current signer set size by org/realm      |                                  |                 |
| `commit_cas_fail_total`           | Counter   | none               | HEAD CAS races detected & handled         |                                  |                 |
| `audit_chain_verify_seconds`      | Histogram | none               | Background deep-verify time per batch     |                                  |                 |
| `registry_recoveries_total`       | Counter   | `{kind="startup    | crash"}`                                  | Successful recovery to last HEAD |                 |
| `bus_overflow_dropped_total`      | Counter   | none               | Non-blocking bus drops counted            |                                  |                 |
| `registry_storage_bytes`          | Gauge     | none               | On-disk size (Macronode)                  |                                  |                 |
| `registry_log_segments`           | Gauge     | none               | Segments after pruning                    |                                  |                 |
| `registry_checkpoint_age_seconds` | Gauge     | none               | Age of last signed checkpoint             |                                  |                 |
| `registry_reload_success_total`   | Counter   | `{source="sighup   | bus"}`                                    | Hot reload success               |                 |
| `registry_reload_fail_total`      | Counter   | `{reason}`         | Hot reload rollback events                |                                  |                 |

> **Metric discipline:** No high-cardinality labels. `proposal_id`, `payload_b3`, and `corr_id` belong in **traces/logs**, not metric labels.

### 1.3 OTEL Metrics (optional)

When OTEL exporter is enabled, the same counters/histograms are emitted as OTEL instruments using identical names and semantic labels.

### 1.4 Registration Discipline

* All metrics are created in a single `Metrics` struct with cheap clone handles.
* CI has a golden test to prevent duplicate registration and to assert **presence** of mandatory metrics.

---

## 2) Health & Readiness

### 2.1 Endpoints

* **`/healthz`** — *liveness*. Returns `200 OK` if process is running and supervisor alive.
* **`/readyz`** — *readiness*. Returns `200 OK` only when:

  * config loaded & validated,
  * storage opened and last `HEAD` validated,
  * commit pipeline ready (write path),
  * bus publisher attached (non-blocking),
  * checkpointor initialized (Macronode).

### 2.2 Response Schema

`/readyz` returns JSON with detail:

```json
{
  "ready": true,
  "write_ready": true,
  "missing": [],
  "version": 42,
  "head_b3": "b3:..."
}
```

On degraded state:

```json
{
  "ready": false,
  "write_ready": false,
  "degraded": true,
  "missing": ["storage", "bus"],
  "retry_after": 5
}
```

### 2.3 Failure Semantics

* **Fail-closed writes, fail-open reads.**
  If storage/bus degrade, `/readyz` flips **write-unready first**; read endpoints may continue.
* **Rate limiting in degraded mode** ensures fairness: 503 with `Retry-After`.

---

## 3) Logs

### 3.1 Format & Transport

* **JSON Lines** (`application/jsonl`), one event per line, UTC timestamps.
* Produced via `tracing-subscriber` JSON formatter (`fmt::layer().json()`).
* Log levels: `TRACE` for dev only; `DEBUG` limited; use `INFO`, `WARN`, `ERROR` in prod.

### 3.2 Required Fields

| Field         | Example                    | Notes                                                     |
| ------------- | -------------------------- | --------------------------------------------------------- |
| `ts`          | `2025-10-08T18:22:11.123Z` | RFC3339Nano                                               |
| `level`       | `INFO`                     |                                                           |
| `service`     | `svc-registry`             | Static string                                             |
| `event`       | `commit.accepted`          | Dot-namespaced                                            |
| `corr_id`     | `01J...ULID`               | From `X-Corr-ID`, generated if absent                     |
| `route`       | `/registry/commit/{id}`    | Normalized                                                |
| `method`      | `POST`                     |                                                           |
| `latency_ms`  | `42`                       | For request logs                                          |
| `client_ip`   | `10.0.0.12`                | If available; avoid for public privacy if policy requires |
| `status`      | `200`                      | HTTP status                                               |
| `err.code`    | `InvalidSig`               | For failures                                              |
| `proposal_id` | `p_7f...`                  | **Do not include payload**                                |
| `version`     | `42`                       | Committed version                                         |
| `payload_b3`  | `b3:...`                   | Content hash (safe)                                       |

### 3.3 Redaction & Secrets

* **Never** log macaroon/token values, private keys, or raw payloads.
* Config diffs are **redacted** on reload events (show key names, not values).
* For PII policy, treat `client_ip` as **sensitive** in public deployments (mask or omit per env).

### 3.4 Log Examples

**Commit success**

```json
{"ts":"2025-10-08T18:22:11.123Z","level":"INFO","service":"svc-registry","event":"commit.accepted","corr_id":"01J...","proposal_id":"p_abcd","version":42,"payload_b3":"b3:7a...","latency_ms":73}
```

**Rejected approval**

```json
{"ts":"2025-10-08T18:23:01.555Z","level":"WARN","service":"svc-registry","event":"approve.rejected","corr_id":"01J...","proposal_id":"p_abcd","err.code":"RevokedKey","reason":"signer revoked"}
```

---

## 4) Tracing & Correlation

### 4.1 Span Naming & Structure

* **Prefix:** `svc.registry.<operation>`
* **Core spans:**

  * `svc.registry.propose`
  * `svc.registry.approve`
  * `svc.registry.commit`
  * `svc.registry.emit_bus`
  * `svc.registry.checkpoint`
  * `svc.registry.retention`
  * `http.request` (from tower/axum)
* **Attributes:**

  * `proposal_id`, `target_version`, `quorum_m`, `quorum_n`
  * `payload_b3`, `result`, `err.code`
  * `client_ip` (if policy allows)
* **Links:** `http.request` ↔ registry operation spans.

### 4.2 Correlation IDs

* Ingress middleware:

  * Accept `X-Corr-ID` (ULID/UUID). If absent, generate.
  * Propagate in:

    * response headers (`X-Corr-ID`),
    * bus events (as field `corr_id`),
    * logs and spans (attribute `corr_id`).

### 4.3 OpenTelemetry

* Feature-flagged exporter (`otlp`).
* **Resource attributes:** `service.name=svc-registry`, `service.version`, `service.instance.id`.
* **Sampling:** default parent-based; 1–10% head sampling acceptable for reads; **force sample** write/commit paths on error.

---

## 5) Alerts & SLOs

### 5.1 SLOs (suggested)

| Surface                         | Objective                                         |
| ------------------------------- | ------------------------------------------------- |
| GET `/registry/head`            | p95 < **50ms** intra-region (<200ms inter-region) |
| Commit (`propose→commit`)       | p95 < **750ms** under normal load                 |
| 5xx rate                        | < **0.1%** of requests                            |
| 429/503 rate                    | < **1%** sustained (burst tolerant)               |
| Audit chain verification errors | **0**                                             |

### 5.2 Example Prometheus Alert Rules

```yaml
groups:
- name: svc-registry
  rules:
  - alert: RegistryHighErrorRate
    expr: sum(rate(http_requests_total{service="svc-registry",status=~"5.."}[5m]))
          / sum(rate(http_requests_total{service="svc-registry"}[5m])) > 0.001
    for: 10m
    labels: {severity: warning}
    annotations:
      summary: High 5xx error rate
      runbook: https://…/RUNBOOK.md#errors

  - alert: RegistryBusyBursts
    expr: rate(rejected_total{service="svc-registry",reason="busy"}[5m]) > 5
    for: 10m
    labels: {severity: warning}
    annotations:
      summary: Busy rejections sustained
      runbook: https://…/RUNBOOK.md#busy

  - alert: RegistryBusLagging
    expr: increase(bus_lagged_total{service="svc-registry"}[10m]) > 0
    for: 15m
    labels: {severity: critical}
    annotations:
      summary: Bus lag/drops detected; consumers may be behind
      runbook: https://…/RUNBOOK.md#bus

  - alert: RegistryCheckpointStale
    expr: (time() - registry_checkpoint_age_seconds) > 1800
    for: 15m
    labels: {severity: warning}
    annotations:
      summary: Checkpoint age exceeded 30m; pruning may stall
      runbook: https://…/RUNBOOK.md#checkpoint
```

### 5.3 Runbooks (anchors)

* **Errors:** verify storage health, check recent deploys, inspect `err.code` distribution.
* **Busy:** confirm RPS/inflight caps, examine queue depth gauges, consider upstream backoff.
* **Bus:** check subscriber health; reconcile via `GET /registry/head`.
* **Checkpoint:** ensure checkpointor running; verify storage fsync and disk space.

---

## 6) CI / Enforcement

* **Golden tests**:

  * `/metrics` exposes all golden metrics with expected types.
  * `/healthz` and `/readyz` endpoints exist; `/readyz` flips in chaos tests.
  * Trace presence test: successful commit emits `{propose,approve,commit,emit_bus}` spans with consistent `proposal_id`.
* **Linters**:

  * `-W clippy::await_holding_lock`
  * forbid logging secrets via custom lint (grep for `macaroon`, `private_key`).
* **Docs freshness**: This file reviewed **every 90 days** or when adding/removing metrics/spans.

---

## 7) Dashboards (starter pack)

* **Service Overview**

  * Requests by status; p95/p99 latency; inflight gauge.
  * Busy rejections & queue depths.
* **Commit Path**

  * `registry_updates_total` split; `commit_cas_fail_total`.
  * Quorum M/N distribution (from logs/spans via OTEL).
* **Audit & Retention**

  * `registry_checkpoint_age_seconds`, `registry_log_segments`, `registry_storage_bytes`.
  * `audit_chain_verify_seconds` histogram heatmap.
* **Reliability**

  * `service_restarts_total` by component.
  * `registry_reload_*` counters.

---

## 8) Field & Label Reference (cheat sheet)

### 8.1 `rejected_total{reason}` reasons

`busy | body_cap | ratio_cap | unauth | invalid_sig | revoked_key | timeout | malformed | chain_mismatch`

### 8.2 Span attribute keys

`corr_id, proposal_id, target_version, quorum_m, quorum_n, payload_b3, result, err.code`

### 8.3 Readiness keys

`storage_ready, bus_attached, commit_ready, checkpointor_ready, config_loaded`

---

## 9) Privacy & Compliance

* `/metrics` on **localhost** by default; expose via scrape proxy.
* Logs avoid PII; IP logging governed by deployment policy; redact if public.
* Spans/metrics contain **hashes**, not raw content.

---

## 10) Implementation Notes (code pointers)

* **Metrics**: `metrics.rs` (constructor + handles), `routes/metrics.rs` (exporter)
* **Health/Ready**: `routes/health.rs`
* **Tracing**: `observability/tracing.rs` (layer setup, correlation extractor)
* **Logging**: `observability/logging.rs` (JSON formatter, redaction)
* **CI**: `tests/observability_golden.rs` (exposes endpoints & asserts presence)

---

## 11) Future Enhancements

* Exemplars on histograms (linking latency to trace IDs)
* RED vs USE dashboards (per route and per internal component)
* Error budget burn alerts for SLOs
* OTEL logs pipeline (unify logs + traces downstream)

---

✅ With this guide, `svc-registry` emits **consistent, low-cardinality, auditor-friendly** telemetry. Operators can **see** readiness and load, auditors can **prove** invariants, and developers can **trace** any write from request to commit to bus.
