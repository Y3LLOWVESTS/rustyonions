---

title: 📈 OBSERVABILITY — svc-rewarder
crate: svc-rewarder
owner: Stevan White
last-reviewed: 2025-10-14
status: draft
msrv: 1.80.0 (Tokio/loom compatible)
------------------------------------

# 📈 OBSERVABILITY.md — `svc-rewarder`

This document defines **what is observable**, **how it’s exposed**, and **how we use it** to keep `svc-rewarder` healthy and auditable:

* Metrics (Prometheus / OTEL)
* Health & readiness semantics
* Logs (JSON schema + redaction)
* Tracing spans & correlation (incl. exemplars)
* Alerts, SLOs, dashboards, and CI enforcement

`svc-rewarder` computes deterministic rewards from sealed inputs and emits **idempotent** settlement intents to the ledger. Observability must prove: **conservation**, **idempotency**, **determinism**, and **bounded cost**.

---

## 0) Purpose

1. Detect and explain failures quickly (mean time to detect ≤ 5m, mitigate ≤ 15m).
2. Prove economic invariants at runtime (Σ payouts ≤ pool_total; no double-emit).
3. Provide forensics (who/what/when/why) without logging secrets or PII.
4. Enforce performance budgets and guardrails (rate/size/cost caps).

---

## 1) Metrics (Prometheus-style)

All metrics are **registered once** in `Metrics::new()` and used via clones. Histograms use **native seconds**; counters are monotonic; gauges for instantaneous state only.

### 1.1 Golden Metrics (common across services)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds_bucket{route,method,le}` + `_sum/_count` (Histogram)
* `inflight_requests{route}` (Gauge) — mirrors concurrency cap
* `bus_lagged_total` (Counter) — broadcast backlog dropped
* `service_restarts_total` (Counter) — supervised restarts
* `rejected_total{reason}` (Counter) — `unauth|oversize|decompress_cap|schema|stale|invariant|cost_cap`

**Recommended buckets** for `request_latency_seconds`:
`[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2, 5]`

### 1.2 `svc-rewarder` Service-Specific Metrics

**Compute pipeline**

* `reward_runs_total{status}` (Counter) — `ok|quarantined|fail`
* `reward_compute_latency_seconds{tenant,policy_id}` (Histogram) — end-to-end compute
* `reward_prepare_seconds` (Histogram) — CID fetch/validation
* `reward_invariant_fail_total{kind}` (Counter) — `conservation|overflow|negative|idempotency|dust_policy`
* `reward_cost_estimate_ms{tenant}` (Histogram) — estimator output; used for caps
* `reward_dust_residual_minor_units_total` (Counter) — accumulated dust bucket

**Idempotency / commitment**

* `reward_runs_idempotent_total` (Counter) — duplicate run_keys accepted (no new effects)
* `reward_runs_conflict_total` (Counter) — same epoch_id with different commitment (should be near-zero)

**Ledger egress**

* `ledger_intents_total{result}` (Counter) — `accepted|dup|error`
* `ledger_emit_latency_seconds` (Histogram)
* `ledger_retry_total{reason}` (Counter) — network|timeout|5xx

**Readiness / backpressure**

* `readyz_degraded{cause}` (Gauge as 0/1) — `backpressure|dep_outage|quota|cost_cap`
* `work_queue_depth` (Gauge)
* `per_tenant_rps{tenant}` (Gauge) — smoothed EWMA window

**Security / auth**

* `auth_failures_total{scope}` (Counter)
* `tls_handshake_failures_total` (Counter)

**Cardinality budget**

* High-cardinality **labels limited to**: `tenant` (bounded), `policy_id` (versioned, bounded), `status/result/kind/cause/scope`.
* **Never** label with `run_key`, `inputs_cid`, or `peer_addr`.

### 1.3 Registration Discipline

* All metrics created in `Metrics::new()`; panic on double-register.
* CI “grep” check: metrics names appear **exactly once** in code; unit tests ensure exposure at `/metrics`.
* Exemplar support (if enabled): histograms attach `trace_id` exemplars for critical paths.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness** only (200 if process alive).
* `GET /readyz` — **readiness** only when all keys (below) set + backpressure within limits; returns 200 or 503 with JSON.
* `GET /version` — git SHA, build info, feature flags.

### 2.2 Readiness Keys (svc-rewarder)

* `config_loaded` — dynamic config parsed & applied
* `policy_registry_ok` — can fetch signed policy docs
* `ledger_ok` — egress intent endpoint reachable and token valid
* `accounting_ok` — can read sealed snapshots
* `queue_ok` — bounded work queue under `MAX_INFLIGHT`
* `metrics_ok` — registry exported
* `amnesia_ok` — tmpfs/path compliance when enabled

### 2.3 Failure Semantics

* **Fail-open reads / fail-closed writes**

  * `GET` endpoints may remain `ready` during partial deps loss if safe.
  * Any failure impacting **compute or emit** results in `/readyz` = 503.
* **Degraded mode body (503)**:

```json
{ "degraded": true, "missing": ["ledger_ok"], "cause": "dep_outage", "retry_after": 5 }
```

* Backpressure enables **deterministic shedding** before saturation; update `readyz_degraded{cause="backpressure"}`.

---

## 3) Logs

### 3.1 Format & Required Fields

* **Format:** JSON Lines (`application/jsonl`), one event per line.
* **Required fields:**

  * `ts` (ISO8601 UTC)
  * `level` (`TRACE|DEBUG|INFO|WARN|ERROR`)
  * `service` (`svc-rewarder`)
  * `event` (e.g., `compute.start`, `compute.done`, `invariant.fail`, `ledger.emit`, `readyz.degraded`)
  * `reason` (aligned with `rejected_total{reason}` / taxonomy)
  * `corr_id` (UUID/ULID)
  * `tenant`
  * `latency_ms` (when applicable)
  * `run_key` (**hash-truncated**, not raw token; e.g., first 8 bytes hex)
  * `policy_hash` (full or truncated)
  * `commitment` (hash of epoch_id||policy_hash||inputs_cid)
  * `status` (`ok|quarantined|fail`) where applicable

**Optional fields:** `epoch_id`, `policy_id`, `queue_depth`, `retry`, `backoff_ms`.

### 3.2 Redaction & Secrets

* **Never** log: macaroon contents, TLS secrets, authorization headers, raw `run_key`, `inputs_cid` payloads.
* Redact config dumps; log only secret **presence & length**, not values.
* PII minimization: external user identifiers should be hashed/tokenized upstream and **never** logged here.

### 3.3 Event Examples (short)

```json
{"ts":"2025-10-14T18:21:05Z","level":"INFO","service":"svc-rewarder","event":"compute.start","tenant":"acme","corr_id":"01JC...","epoch_id":"2025-10-01","policy_id":"rev42","policy_hash":"b3b7...","run_key":"3af8c2e1","queue_depth":12}
{"ts":"2025-10-14T18:21:05Z","level":"WARN","service":"svc-rewarder","event":"invariant.fail","reason":"conservation","tenant":"acme","corr_id":"01JC...","policy_hash":"b3b7...","run_key":"3af8c2e1","status":"quarantined"}
{"ts":"2025-10-14T18:21:06Z","level":"INFO","service":"svc-rewarder","event":"compute.done","tenant":"acme","corr_id":"01JC...","run_key":"3af8c2e1","latency_ms":842,"status":"ok"}
{"ts":"2025-10-14T18:21:06Z","level":"INFO","service":"svc-rewarder","event":"ledger.emit","tenant":"acme","corr_id":"01JC...","run_key":"3af8c2e1","result":"accepted","latency_ms":52}
```

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` JSON layers.
* **Span naming:** `svc.rewarder.compute`, `svc.rewarder.prepare`, `svc.rewarder.invariants`, `svc.rewarder.ledger.emit`, `svc.rewarder.readyz`.
* **Span fields:** `tenant`, `epoch_id`, `policy_id`, `policy_hash`, `run_key_trunc`, `cost_estimate_ms`.
* **Correlation IDs:**

  * Inject from HTTP header `X-Corr-ID` if present; else generate.
  * Propagate through bus/events/outbound requests as `X-Corr-ID`.
* **OpenTelemetry (optional, feature-flagged):** export traces/metrics; attach **exemplars** (trace_id) to `reward_compute_latency_seconds` and `request_latency_seconds`.
* **Sampling:**

  * **Default**: parent-based 10% for compute path, 100% for `ERROR/WARN`.
  * **Burst mode**: raise to 50% during incidents (feature flag).

---

## 5) Alerts & SLOs

### 5.1 SLOs (svc-rewarder)

* **Latency (compute)**: p95 `reward_compute_latency_seconds` < **500 ms** (baseline), p99 < **2 s** intra-region.
* **Availability**: `/readyz` **≥ 99.9%** monthly (excludes maintenance windows).
* **Correctness**: `reward_invariant_fail_total` daily = **0** (non-zero triggers review).
* **Idempotency**: `ledger_intents_total{result="dup"}` / total ≤ **1%** (retries OK, double-emits not).
* **Error budget**: 5xx rate < **0.1%**; 429/503 < **1%** sustained.

### 5.2 Alert Rules (PromQL examples)

**Auth failures spike**

```promql
ALERT AuthFailuresSpike
  IF sum(rate(auth_failures_total[5m])) by (tenant) > 10
  FOR 10m
  LABELS { severity="high", service="svc-rewarder" }
  ANNOTATIONS { summary="Auth failures spike for {{ $labels.tenant }}" }
```

**Ready degraded**

```promql
ALERT ReadyDegraded
  IF max_over_time(readyz_degraded[10m]) > 0
  FOR 10m
  LABELS { severity="high", service="svc-rewarder" }
  ANNOTATIONS { summary="svc-rewarder readiness degraded" }
```

**Invariant breach**

```promql
ALERT InvariantFailures
  IF increase(reward_invariant_fail_total[10m]) > 0
  LABELS { severity="critical", service="svc-rewarder" }
  ANNOTATIONS { summary="Economic invariant failure detected" }
```

**DoS / decompression cap**

```promql
ALERT DoSCapTriggered
  IF sum(rate(rejected_total{reason=~"oversize|decompress_cap|cost_cap"}[5m])) > 50
  FOR 5m
  LABELS { severity="medium", service="svc-rewarder" }
```

**Ledger egress errors**

```promql
ALERT LedgerEgressErrors
  IF rate(ledger_intents_total{result="error"}[5m]) > 1
  FOR 10m
  LABELS { severity="high", service="svc-rewarder" }
```

**Compute latency regression**

```promql
ALERT ComputeLatencyP95High
  IF histogram_quantile(0.95, sum by (le) (rate(reward_compute_latency_seconds_bucket[5m]))) > 0.5
  FOR 15m
  LABELS { severity="medium", service="svc-rewarder" }
```

### 5.3 Runbooks

Each alert **must** link to `RUNBOOK.md` sections:

* Authentication spikes → rotate keys, check gateway logs, lower rate caps, enable quarantine-only mode.
* Readiness degraded → identify `missing` keys in `/readyz` JSON, follow dep-specific steps.
* Invariant failures → quarantine; pull manifest; compare to golden vectors; root cause and hotfix.
* Ledger errors → verify idempotency (`dup` vs `error`), backoff policy, and downstream status.

---

## 6) Dashboards (starter layouts)

**Dashboard: Rewarder Overview**

* Row A — SLOs: p95/99 compute latency; `/readyz` SLI; 5xx/429/503 rates.
* Row B — Throughput: `http_requests_total` by route/method; `reward_runs_total{status}` stacked.
* Row C — Egress: `ledger_intents_total{result}`; latency hist; retries.
* Row D — Quality: `invariant_fail_total{kind}`; `runs_conflict_total`; `idempotent_total`.
* Row E — Capacity: `work_queue_depth`; `inflight_requests`; `readyz_degraded{cause}`; per-tenant RPS.
* Row F — Security: `auth_failures_total{scope}`; `rejected_total{reason}`.

---

## 7) CI / Enforcement

* **Exposure checks:** integration test scrapes `/metrics`, `/healthz`, `/readyz`, `/version`.
* **Metric presence:** tests assert existence of golden metrics + service-specific (`reward_*`, `ledger_*`).
* **Duplicate registration guard:** panic on second registration; CI grep ensures single definition.
* **Lint gates:** deny `await_holding_lock`; no sleeps for readiness gating; warn on unlabelled counters.
* **Docs freshness:** observability docs touched **≤ 90 days** ago (CI date check).
* **Schema contract:** DTO schemas (schemars) validated in CI; OpenAPI/JSON Schema published.
* **Alert rules lint:** `promtool check rules` in CI; rules versioned next to service.

---

## 8) OTEL / Exporters (optional)

* Feature-flag `otel` to export traces/metrics to an OTEL collector.
* Resource attributes: `service.name=svc-rewarder`, `service.version`, `deployment.environment`.
* Map Prometheus histograms to OTEL **explicit bucket histograms**; attach exemplars (trace ids) where supported.

---

## 9) Economic Observability (Proof Hooks)

To **prove** correctness in dashboards and during audits:

* `reward_dust_residual_minor_units_total` trend must be **non-decreasing** and small.
* **Conservation panel**: compute Σ(payouts) vs pool_total from manifest samples (no raw PII) — must satisfy `≤`.
* **Idempotency panel**: show `ledger_intents_total{result="dup"}` rising when retries occur; **no** spike in `accepted` for same `run_key`.
* **Determinism check**: repeated `dry_run=true` calls yield identical `reward_compute_latency_seconds` distributions and identical `commitment`.

---

## 10) Red/Black Switches (incident posture)

* **Quarantine-only mode** (feature flag): compute manifests but suppress ledger emission; surfaces as `readyz_degraded{cause="dep_outage"}` if ledger is the cause.
* **Burst sampling**: raise trace sampling to 50% while incident flag is set; auto-restore after 60m stable.
* **RPS clamp**: dynamically lower per-tenant and global RPS caps; expose current caps via `/metrics` (`per_tenant_rps` gauge is authoritative).

---

## 11) Appendix — Field & Label Reference

**`rejected_total{reason}` reasons (canonical):**
`unauth, oversize, decompress_cap, schema, stale, invariant, cost_cap`

**`reward_invariant_fail_total{kind}` kinds:**
`conservation, overflow, negative, idempotency, dust_policy`

**`ledger_intents_total{result}` results:**
`accepted, dup, error`

**Span names:**
`svc.rewarder.prepare`, `svc.rewarder.compute`, `svc.rewarder.invariants`, `svc.rewarder.ledger.emit`, `svc.rewarder.readyz`

---

✅ With this in place, `svc-rewarder` emits **consistent, low-cardinality, decision-ready signals** that prove economic safety and enable fast incident response—while keeping secrets and PII out of the telemetry stream.
