

---

# 📄 Paste-ready — `crates/ron-ledger/docs/OBSERVABILITY.md`

````markdown
---
title: ron-ledger — Observability (Metrics, Logs, Tracing, SLOs)
status: reviewed
msrv: 1.80.0
last-updated: 2025-10-13
audience: developers, ops, auditors
pillar: P12 — Economics & Wallets
concerns: [RES, SEC, PERF, GOV]
owners: [Stevan White]
links:
  - IDB: ./IDB.md
  - CONCURRENCY: ./CONCURRENCY.md
  - SECURITY: ./SECURITY.md
  - CONFIG: ./CONFIG.md
  - RUNBOOK: ./RUNBOOK.md
---

# 0) Purpose (normative)
This document defines **the** source of truth for `ron-ledger` observability: metric names + labels, health/readiness semantics, log schema, tracing model, SLOs, alerts, dashboards, and CI enforcement. If it’s not here, it **doesn’t exist**. It satisfies IDB I-8 (observability) and supports I-6/I-7/I-13 (idempotency/ordering/recovery).

---

# 1) Metrics

## 1.1 Golden (4 Signals) + HTTP
- `http_requests_total{route,method,status}` counter
- `http_request_duration_seconds{route,method}` histogram
- `process_cpu_seconds_total`, `process_resident_memory_bytes`, `process_open_fds` (saturation)

Cardinality rules:
- `route` must be templated (e.g., `/ingest`, `/roots`, not raw IDs).
- No unbounded labels (peer IPs, user IDs) on high-churn series.

## 1.2 Ledger-specific (authoritative taxonomy)
- `ledger_commits_total` counter
- `ledger_commit_latency_seconds` histogram **(primary latency SLO)**
- `ledger_roots_published_total` counter
- `ledger_seq_gap_detected_total` counter
- `ledger_safe_mode` gauge {0|1}
- `queue_depth{q}` gauge (`q in {"ingress","preval","commit"}`)
- `busy_rejections_total{q}` counter
- `drops_total{q="preval"}` counter
- `prevalidator_duration_seconds` histogram
- `sequencer_batch_size` histogram
- `committer_batch_latency_seconds` histogram

## 1.3 Security/Interop signals
- `rejected_total{reason}` counter (taxonomy: `unauth`,`cap_invalid`,`unknown_kind`,`policy_denied`,`busy`,`too_large`,`timeout`,`internal`)
- `auth_failures_total{peer}` counter (cap parse/mTLS failures; `peer` is **low-cardinality class**, not raw IP)
- `tls_handshake_failures_total` counter
- `breaker_open_total{target}`, `breaker_open_seconds{target}` counters/gauges (`target in {"kms","policy","wallet"}`)
- `wal_fsync_fail_total` counter

## 1.4 Exemplars & OTEL bridging (required)
- Histograms `*_duration_seconds` and `ledger_commit_latency_seconds` **MUST** attach exemplars with `trace_id` when a span is present.
- The deployment MUST run a Collector that **scrapes Prometheus** and **exports OTLP metrics & traces**, preserving exemplars:
  - Prom → OTEL Collector (prometheusreceiver with exemplars) → OTLP exporter to APM.
- Resource attrs REQUIRED: `service.name="ron-ledger"`, `service.instance.id`, `deployment.environment`.

## 1.5 Buckets (defaults; configurable)
- `ledger_commit_latency_seconds`: `[0.005, 0.01, 0.02, 0.04, 0.08, 0.15, 0.3, 0.6]`
- `committer_batch_latency_seconds`: same as above
- `http_request_duration_seconds`: Prom HTTP defaults

---

# 2) Health / Readiness

## 2.1 `/healthz` (liveness)
- Returns `200 {"ok":true,"version": "..."}`
- No external calls; always shallow.

## 2.2 `/readyz` (readiness; degradations included)
- `200 {"ready": true, "degraded": false, "kms_ready": true, "policy_ready": true, "wallet_ready": true, "backpressure_ok": true, "committer_lag_ms": 3}`
- **False** if:
  - any `breaker_open` hard-open,
  - `queue_depth{q="commit"} >= 0.9*capacity`,
  - `ledger_safe_mode==1`,
  - Committer lag exceeds threshold (cfg).
- **Writes fail-closed**, **reads may fail-open** (documented in RUNBOOK).

---

# 3) Logs (structured, PII-free)

## 3.1 JSON schema (minimum fields)
```json
{
  "ts": "RFC3339Nano",
  "lvl": "INFO|WARN|ERROR",
  "msg": "string",
  "corr_id": "uuid",
  "span_id": "hex",
  "seq": "u64 (optional)",
  "entry_id": "uuid (optional)",
  "kid": "string (KID only)",
  "capability_ref": "string",
  "route": "/ingest|/roots|...",
  "reason": "RejectReason (optional)",
  "q": "ingress|preval|commit (optional)"
}
````

## 3.2 Redaction & retention

* **Never** log secrets, tokens, macaroon bodies, signatures, or raw PII. KID-only.
* Retention: 7d hot (encrypted), 30d cold (auditor access).

---

# 4) Tracing (OpenTelemetry)

* **Mandatory in production.** Feature flag controls code wiring, but runtime **must** set `OTEL_EXPORTER_OTLP_*` for prod.
* Head-based sampling: 1% baseline; **100%** when `ledger_safe_mode=1` or any breaker is open.
* Span set (minimum):

  * `svc.ron-ledger.ingress.handle`
  * `svc.ron-ledger.prevalidator.check`
  * `svc.ron-ledger.sequencer.assign`
  * `svc.ron-ledger.commit.append_wal`
  * `svc.ron-ledger.commit.checkpoint`
  * `svc.ron-ledger.publish.root`
* Attributes on spans: `corr_id`, `seq`, `entry_id`, `kid`, `capability_ref`, `queue`, `batch_size`.
* **Baggage**: propagate `corr_id`, `peer_class`, `tenant` (if present).
* **Exemplars**: attach `trace_id` to latency histograms (see §1.4).

```mermaid
flowchart LR
  A[HTTP/UDS Request] -->|corr_id| B[Ingress span]
  B --> C[Prevalidator span]
  C --> D[Sequencer span]
  D --> E[Commit append_wal span]
  E --> F[Commit checkpoint span]
  E --> G{{Prom histogram exemplar (trace_id)}}
```

---

# 5) SLOs, Error Budgets, Alerts

## 5.1 SLOs (targets)

* **Latency (commit path):** p95 `ledger_commit_latency_seconds` ≤ **80ms**, p99 ≤ **150ms**
* **Availability (commit path):** 99.9% of commits complete within SLO over a **28-day** window (error budget: 0.1%)
* **Recovery:** `ledger_seq_gap_detected_total` delta = 0 over 28d; any delta consumes 100% of budget until resolved
* **Backpressure:** `queue_depth{q="commit"} < 0.8*capacity` 99% of minutes

## 5.2 Alerting (PromQL; warn→page escalation)

* **Commit latency WARN**

  ```
  histogram_quantile(0.95, sum(rate(ledger_commit_latency_seconds_bucket[5m])) by (le))
    > 0.08
  ```

  Escalate to **PAGE** if persists > **5m** or p99 > **0.15** for 5m.

* **Commit queue PAGE**

  ```
  max_over_time(queue_depth{q="commit"}[2m]) / CAPACITY_COMMIT > 0.8
  ```

* **Ingress busy WARN**

  ```
  rate(busy_rejections_total{q="ingress"}[1m]) > 50
  ```

* **Safe mode PAGE**

  ```
  ledger_safe_mode == 1
  ```

* **Seq gap PAGE**

  ```
  increase(ledger_seq_gap_detected_total[5m]) > 0
  ```

* **KMS breaker PAGE**

  ```
  sum(increase(breaker_open_seconds{target="kms"}[5m])) > 30
  ```

* **Auth failures WARN**

  ```
  rate(auth_failures_total[5m]) > 10
  ```

Each alert links to RUNBOOK entries and the relevant dashboard panel.

---

# 6) Dashboards (Grafana; required panels)

Store in repo at `ops/grafana/ron-ledger.json` and version it.

**Required rows**

1. **Golden Overview**

   * Commit latency: p50/p95/p99 (with exemplars link to traces)
   * Commit queue depth & capacity line
   * Error rate: `rejected_total{reason}` stacked
   * Traffic: ingest RPS and batch size

2. **Commit Path**

   * `committer_batch_latency_seconds` heatmap
   * Sequencer batch size histogram
   * Safe-mode and seq-gap timeline (annotation: deployments)

3. **Security**

   * Auth failures & TLS handshake failures
   * Breaker state by target

4. **Backpressure**

   * Queue depths (three gauges)
   * Busy rejections and drops rate

---

# 7) CI / Enforcement

* **Metrics presence gate:** integration test boots the service and scrapes `/metrics`; asserts presence of all §1 series.
* **Taxonomy lock:** `rejected_total{reason}` and queue label sets are unit-tested; changes require doc update + major/minor as appropriate.
* **Alert rules lint:** `promtool check rules ops/prometheus/alerts.yml` must pass; golden canary tests simulate signals to assert alert fire.
* **Dashboard drift:** PRs modifying panels bump `ops/grafana/ron-ledger.json` `version` and pass JSON lint.

---

# 8) Field Guide (examples)

## 8.1 `/readyz` degraded sample

```json
{
  "ready": false,
  "degraded": true,
  "kms_ready": false,
  "policy_ready": true,
  "wallet_ready": true,
  "backpressure_ok": false,
  "committer_lag_ms": 187,
  "reason": "breaker_open:kms; commit_queue_high"
}
```

## 8.2 Error log (redacted)

```json
{
  "ts":"2025-10-13T17:33:51.431Z",
  "lvl":"ERROR",
  "msg":"capability verify failed",
  "corr_id":"11b9d6e1-0c39-4f1d-9f7b-9b9a5c63d5ab",
  "span_id":"8f2c1a",
  "kid":"kid-2025-09",
  "route":"/ingest",
  "reason":"cap_invalid"
}
```

---

# 9) OTEL Feature & Collector (mandated in prod)

* Code exposes OTEL via `tracing-opentelemetry`. Production **must** set:

  * `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_RESOURCE_ATTRIBUTES=service.name=ron-ledger,...`
* Collector config MUST:

  * scrape Prom with exemplars enabled,
  * export **metrics + traces** via OTLP,
  * include exemplar to trace linking (supports “View Trace” from Grafana panel).

---

# 10) Maintenance

* **Review cadence:** every **90 days** or upon any change to DTOs/endpoints/concurrency or security posture.
* **Who:** owners + SRE + security engineer; record decisions in this file + CHANGELOG.

```
```

---

## Why this addresses Grok’s nits

* **OTEL is mandatory in prod**, with exemplar bridging spelled out (Prom → Collector → OTLP), so traces link from latency spikes.
* **Explicit warn→page escalations** and a **28-day error-budget policy** make SLOs operationally meaningful.
* **PromQL** rules are included and gated by CI (promtool + canary tests).
* **Readiness keys** and SAFE-MODE semantics line up exactly with `IDB.md`/`CONCURRENCY.md`/`SECURITY.md`.
* **Cardinality controls** ensure we don’t melt Prometheus.

