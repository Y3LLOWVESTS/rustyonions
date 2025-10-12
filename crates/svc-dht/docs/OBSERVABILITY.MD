````markdown
# 📈 OBSERVABILITY — svc-dht

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- Metrics (Prometheus/OTEL)
- Health & readiness semantics
- Logs (JSON schema, fields)
- Tracing spans & correlation
- Alerts & SLOs
- Dashboards & CI guardrails

This doc mirrors the IDB, Concurrency, Security, and Config blueprints. Names, units, and buckets are **canonical**.

---

## 1) Metrics (Prometheus/OpenMetrics)

### 1.1 Golden Metrics (every service)

| Metric | Type | Labels | Notes |
|---|---|---|---|
| `http_requests_total` | Counter | `route,method,status` | Increments on response write |
| `request_latency_seconds` | Histogram | `route,method` | Buckets: `le={0.01,0.025,0.05,0.1,0.25,0.5,1,2,5}` |
| `inflight_requests` | Gauge | `route` | Active reqs; mirrors concurrency cap |
| `bus_lagged_total` | Counter | `channel` | Broadcast drops due to lag |
| `service_restarts_total` | Counter | `task` | Supervisor restarts (jittered) |
| `rejected_total` | Counter | `reason,endpoint` | `reason∈{quota,ratio_cap,frame_cap,bad_sig,not_ready,busy,unauth}` |

### 1.2 DHT-Specific Metrics (authoritative)

| Metric | Type | Labels | Notes |
|---|---|---|---|
| `dht_lookup_hops_histogram` | Histogram | `op∈{FIND_NODE,FIND_VALUE}` | Buckets: `le={1,2,3,4,5,6,8,10}` (integer hops) |
| `dht_lookup_latency_seconds` | Histogram | `op` | End-to-end lookup wall time |
| `dht_hedge_spawned_total` | Counter | `op` | Hedged probe count |
| `dht_hedge_canceled_total` | Counter | `op` | Loser cancellations |
| `dht_rpcs_total` | Counter | `op,peer_asn,code` | Code: `ok,timeout,busy,not_ready,bad_sig` |
| `dht_rpcs_inflight` | Gauge | `op` | Parallel RPC probes (α+β bounded) |
| `dht_bucket_asn_entropy_bits` | Gauge | `bucket` | Shannon entropy per bucket snapshot |
| `dht_bucket_asn_max_pct` | Gauge | `bucket` | Max ASN share in bucket (0–100) |
| `dht_route_updates_total` | Counter | `action∈{insert,evict,refresh}` | Produced by BucketWriter |
| `dht_bootstrap_seeds_up` | Gauge | `seed` | 1 if reachable |
| `dht_ready_bucket_fill_pct` | Gauge | — | % non-empty buckets |
| `provider_records_total` | Gauge | `state∈{live,stale,rejected}` | Current provider record counts |
| `provider_republish_total` | Counter | `result∈{ok,timeout,reject}` | Republisher outcomes |
| `pq_dual_sign_ratio` | Gauge | — | Fraction dual-signed in last window |
| `pq_verify_fail_total` | Counter | `alg∈{ed25519,mldsa87}` | Signature verify failures |
| `tls_handshake_failures_total` | Counter | `reason` | From transport layer |
| `queue_depth` | Gauge | `queue∈{work,route,events}` | Sampled queue depth |
| `queue_dropped_total` | Counter | `queue` | Drops due to backpressure |

**Registration Discipline**  
All metrics are registered exactly once in `Metrics::new()`; handles (`Arc<Metrics>`) are cloned. CI greps for duplicate `register_*` calls (see §6).

### 1.3 Exemplary Rust registration (paste-ready)

```rust
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use once_cell::sync::Lazy;

pub static PROM_HANDLE: Lazy<PrometheusHandle> = Lazy::new(|| {
    PrometheusBuilder::new()
        .set_buckets_for_metric("request_latency_seconds", &[0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.0,5.0]).unwrap()
        .set_buckets_for_metric("dht_lookup_latency_seconds", &[0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.0,5.0]).unwrap()
        .set_buckets_for_metric("dht_lookup_hops_histogram", &[1.0,2.0,3.0,4.0,5.0,6.0,8.0,10.0]).unwrap()
        .install_recorder().unwrap()
});
````

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness**. Returns `200 OK` if process is serving the handler.
* `GET /readyz` — **readiness**. Returns:

  * `200 OK` when **all** readiness keys are satisfied.
  * `503 Service Unavailable` with JSON body on degradation.

### 2.2 Readiness Keys (svc-dht)

* `config_loaded` — config parsed & validated.
* `listeners_bound` — RPC + HTTP bound successfully.
* `bootstrap_min_seeds` — ≥ `bootstrap.required` seeds reachable.
* `bucket_fill_pct` — ≥ `bootstrap.ready_bucket_fill_pct` non-empty buckets.
* `metrics_exporter_ready` — `/metrics` bound.
* `pq_policy_loaded` — PQ config valid; warnings allowed pre-`require_pq_on`.

### 2.3 Failure Semantics

* **Fail-open reads / fail-closed writes** during degradation:

  * Lookup reads may serve best-effort if buckets exist.
  * Mutating operations (e.g., `PROVIDE`) return `503 NotReady`.
* **Degraded response (503) schema:**

```json
{
  "service": "svc-dht",
  "degraded": true,
  "missing": ["bootstrap_min_seeds","bucket_fill_pct"],
  "retry_after": 10
}
```

---

## 3) Logs (JSON Lines)

### 3.1 Schema (canonical fields)

* `ts` (RFC3339/ISO8601, UTC)
* `level` (`TRACE|DEBUG|INFO|WARN|ERROR`)
* `service` (`"svc-dht"`)
* `event` (taxonomy: `rpc_accept`, `rpc_result`, `lookup_start`, `lookup_done`, `bucket_update`, `quota_exhausted`, `not_ready`, `verify_fail`, `tls_handshake_fail`, `service_crashed`)
* `corr_id` (ULID)
* `op` (`FIND_NODE|FIND_VALUE|PROVIDE|STORE`)
* `peer_id` (string), `peer_addr` (redactable), `asn` (number)
* `reason` (aligns with `rejected_total{reason}`)
* `latency_ms` (number)
* `hops` (int)
* `bytes_in`, `bytes_out` (ints)
* `bucket` (int) when applicable

**Example lines:**

```json
{"ts":"2025-10-10T15:42:33.512Z","level":"INFO","service":"svc-dht","event":"lookup_done","corr_id":"01JAB5AH5TX5WAF3JF3C7ZV1MY","op":"FIND_VALUE","hops":3,"latency_ms":87,"peer_id":"12D3KooW...","asn":13335}
{"ts":"2025-10-10T15:42:34.101Z","level":"WARN","service":"svc-dht","event":"quota_exhausted","reason":"quota","endpoint":"rpc","inflight":512}
{"ts":"2025-10-10T15:42:34.555Z","level":"WARN","service":"svc-dht","event":"verify_fail","op":"PROVIDE","reason":"bad_sig","alg":"mldsa87"}
```

### 3.2 Redaction & Secrets

* **Never** log provider **payloads**, macaroons, keys, or raw cert material.
* Addresses may be truncated (e.g., `/24`) if policy requires; always log ASN instead.

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` JSON formatter (or OTEL feature).
* **Span naming**: `svc.dht.<component>.<operation>`, e.g., `svc.dht.lookup.find_value`.
* **Span attributes**: `corr_id`, `op`, `target_key`, `hop`, `peer_id`, `asn`.
* **Correlation IDs**:

  * Inject on ingress:

    * HTTP: `X-Corr-Id` header (generate if missing)
    * DHT: include in envelope metadata if available; otherwise generated per request
  * Propagate via events/metrics labels when feasible.
* **Sampling**:

  * Default trace sampling 1% (configurable); force sampling for WARN/ERROR.

**Span skeleton:**

```rust
let span = tracing::info_span!(
    "svc.dht.lookup.find_value",
    corr_id = %corr_id,
    target = %hex::encode(key),
    hop_budget = cfg.hop_budget
);
let _g = span.enter();
```

---

## 5) Alerts & SLOs

### 5.1 SLOs (svc-dht)

* **Lookup** p95 latency: **<200ms** intra-region; p99 hops **≤5**.
* 5xx rate (HTTP) **<0.1%**; `429|503` combined **<1%** steady state.
* Bootstrap time to ready (cold start): median **≤120s**, p95 **≤300s**.
* PQ compliance ratio (`pq_dual_sign_ratio`) **≥0.95** during migration window.
* Diversity floor: `avg_over_time(dht_bucket_asn_entropy_bits[15m]) ≥ 1.5`.

### 5.2 Alert Rules (PromQL)

```promql
# Hop SLO p99 violated (5m window)
(1 - histogram_quantile(0.99, sum by (le) (rate(dht_lookup_hops_histogram_bucket[5m])))) < 0.01
```

* **Severity:** critical, **For:** 5m, **Runbook:** `runbook://svc-dht/hops-slo`

```promql
# PQ migration compliance (warn)
avg_over_time(pq_dual_sign_ratio[30m]) < 0.95
```

* **Severity:** warning, **For:** 30m, **Runbook:** `runbook://svc-dht/pq-migration`

```promql
# Diversity erosion (warn)
avg_over_time(dht_bucket_asn_entropy_bits[15m]) < 1.5
```

* **Severity:** warning, **For:** 30m, **Runbook:** `runbook://svc-dht/eclipse-mitigation`

```promql
# Quota storm (warn)
sum(rate(rejected_total{reason="quota"}[5m])) > 5
```

* **Severity:** warning, **For:** 10m, **Runbook:** `runbook://svc-dht/limits-tuning`

```promql
# Bootstrap stuck (critical): not ready past boot budget
(dht_ready_bucket_fill_pct < on() group_left() 60) and on() (time() - process_start_time_seconds > 300)
```

* **Severity:** critical, **Runbook:** `runbook://svc-dht/bootstrap-stuck`

### 5.3 Runbooks

Each alert has an entry in `RUNBOOK.md` with triage steps, graphs to consult, and mitigations (e.g., raise seeds, relax ASN floor temporarily under change control).

---

## 6) CI / Enforcement

* **Static checks** (script or `cargo nextest`):

  * Assert `/metrics`, `/healthz`, `/readyz` routes exist in the binary target.
  * Grep duplicate `register_*` metrics calls (deny duplicates).
  * Enforce log schema fields by snapshot tests (`serde_json::Value` schema check).
* **Lints**:

  * `-D clippy::await_holding_lock` (prevents metrics/logs under held locks).
  * `-D warnings` globally.
* **Golden tests**:

  * Produce a tiny registry dump; compare metric **names**/units to a golden file.
  * Integration test for `/readyz` gating transitions: not-ready → ready after seeded.

---

## 7) Dashboards (Grafana outline)

**Dashboard: svc-dht — Overview**

1. **Golden RPS/Latency**

   * Panels: `http_requests_total` (rate by route/method), `request_latency_seconds` (p50/p95)
2. **Lookup Health**

   * `dht_lookup_latency_seconds` p50/p95/p99
   * `dht_lookup_hops_histogram` p50/p95/p99 (via quantile)
   * `dht_rpcs_total` by `code`
3. **Backpressure**

   * `queue_depth{queue}` (work/route), `queue_dropped_total{queue}`, `busy_rejections_total`
4. **Bootstrap & Readiness**

   * `dht_bootstrap_seeds_up` (by seed), `dht_ready_bucket_fill_pct`, `/readyz` flip annotations
5. **Diversity & Eclipse**

   * `dht_bucket_asn_entropy_bits` heatmap (bucket idx), `dht_bucket_asn_max_pct`
6. **PQ Migration**

   * `pq_dual_sign_ratio`, `pq_verify_fail_total{alg}`
7. **Errors**

   * `rejected_total{reason}`, `tls_handshake_failures_total`, `io_timeouts_total{op}`

---

## 8) OTEL (optional, feature `otel`)

* Export **traces** (and optionally **metrics/logs**) via OTLP/HTTP to your collector.
* Resource attributes: `service.name=svc-dht`, `service.version`, `deployment.environment`.
* Sampling: parent-based, 1% default; ERROR/WARN always sampled.

Example (pseudocode):

```rust
#[cfg(feature = "otel")]
fn init_tracing_otel() -> anyhow::Result<()> {
    use opentelemetry::sdk::{trace as sdktrace, Resource};
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().http())
        .with_trace_config(sdktrace::Config::default().with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "svc-dht"),
        ])))
        .install_batch(opentelemetry::runtime::Tokio)?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
    Ok(())
}
```

---

## 9) Endpoint Contracts

* `GET /metrics` — OpenMetrics; **bind localhost** in production (see CONFIG.md).
* `GET /healthz` — `200 OK` if handler executes.
* `GET /readyz` — `200/503` with JSON body; never block on long operations; use cached readiness snapshot updated by Supervisor.

---

## 10) Metric Naming Rules

* **snake_case**, nouns first: `dht_lookup_latency_seconds`.
* Units in name (`_seconds`, `_bytes`, `_total`, `_bits`) per Prometheus conventions.
* Label cardinality is bounded; **never** use unbounded values (full IPs, raw error messages). Prefer ASN, short reason enums.

---

## 11) Test Matrix (Observability)

| Scenario                    | Expectation                                                                                     |
| --------------------------- | ----------------------------------------------------------------------------------------------- |
| Cold start before bootstrap | `/readyz` = 503 with `missing` keys; `dht_bootstrap_seeds_up` reflects reachability             |
| Eclipse simulation          | `dht_bucket_asn_entropy_bits` < 1.5 → WARNING alert                                             |
| Quota storm                 | `rejected_total{reason="quota"}` increases; WARN alert fires                                    |
| Bad signature flood         | `pq_verify_fail_total{alg}` increases; `rejected_total{reason="bad_sig"}` increments; no panics |
| Broadcast lag               | `bus_lagged_total` increments; WARN log shows `lag`                                             |
| Hedging active              | `dht_hedge_spawned_total` > 0; `dht_hedge_canceled_total` tracks losers                         |
| Ready → drain               | `/readyz` flips to 503 during shutdown; later `/healthz` stays 200 until process exit           |

---

## 12) FAQ / Notes

* **Where do corr_ids come from?** HTTP header `X-Corr-Id` or generated ULID. Carried on lookup spans and log lines.
* **Why hops histogram?** It directly enforces the IDB hop SLO; alerts trigger early under routing pathologies.
* **Why entropy metric?** Operational defense against eclipse; gives auditors a quantitative proof.

---

✅ With this plan, `svc-dht` emits **uniform, high-signal** telemetry: operators get actionable alerts; auditors get measurable proofs; developers get spans and metrics that map 1:1 to the IDB and Concurrency docs.

```
```
