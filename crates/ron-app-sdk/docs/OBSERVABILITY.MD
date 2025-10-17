
````markdown
# 📈 OBSERVABILITY.md — ron-app-sdk

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- Metrics (Prometheus/OTEL) — **SDK-scoped**, exported via optional feature flags and host integrations
- Health & readiness semantics — **programmatic** readiness for host `/readyz`
- Logs (JSON schema, fields) — stable, redacted
- Tracing spans & correlation — B3 / W3C propagation, deterministic names/attrs
- Alerts & SLOs — SDK-added overhead and error budgets

> Note: `ron-app-sdk` is a **library** (no HTTP listener). Observability is:
> 1) emitted from the caller process (host service), and  
> 2) exposed via **metrics handles**, **tracing spans**, and **structured logs**.

---

## 1) Metrics (Prometheus-style)

The SDK emits metrics **only if** the host enables the `metrics` feature and registers collectors. Names are stable and **registered once** in `Metrics::new()`; clones are cheap handles.

### 1.1 Golden Metrics (SDK)

- `sdk_requests_total{endpoint, outcome="ok|error", code}` (Counter)  
  Total calls by endpoint. `code` mirrors typed error or transport status (e.g., `server_5xx`, `cap_denied`, `deadline`).

- `sdk_request_latency_seconds{endpoint, quantile}` (Histogram, OTEL-compatible buckets)  
  End-to-end latency (includes retries). Track p50/p95/p99.

- `sdk_inflight_requests{endpoint}` (Gauge)  
  In-flight call count (increment on call start, decrement on finish).

- `sdk_backoff_retries_total{endpoint, reason="timeout|server_5xx|rate_limited|overload"}` (Counter)  
  Count of retries executed and why.

- `sdk_io_timeouts_total{op="connect|read|write|overall"}` (Counter)  
  Timeouts at each layer; `overall` maps to deadline exceeded.

- `sdk_oap_violations_total{kind="frame|schema"}` (Counter)  
  Frame > 1 MiB, or DTO with unknown fields.

- `sdk_auth_failures_total{reason="denied|expired"}` (Counter)  
  Capability failures; **non-retriable** by policy.

- `sdk_cache_events_total{event="hit|miss|insert|evict"}` (Counter, if cache enabled)  
  Ephemeral LRU cache behavior.

- `sdk_tor_events_total{event="bootstrap|unavailable"}` (Counter, when `transport=tor`)  
  Tor/arti readiness signals from the transport binding.

### 1.2 Plane / Operation-Specific

- **Mailbox**:  
  - `sdk_mailbox_acks_total{outcome}` (Counter)  
  - `sdk_mailbox_delivery_latency_seconds` (Histogram)

- **Storage**:  
  - `sdk_storage_put_bytes_total` (Counter)  
  - `sdk_storage_get_bytes_total` (Counter)  
  - `sdk_integrity_verify_total{outcome="ok|mismatch"}` (Counter; terminal BLAKE3 verify)

- **Edge (byte-range)**:  
  - `sdk_edge_range_requests_total{partial="true|false"}` (Counter)

- **Index**:  
  - `sdk_index_resolve_total{outcome}` (Counter)

> **Label discipline**: prefer low-cardinality labels (`endpoint`, `outcome`, `reason`). Never include user IDs, tokens, or paths with high entropy.

### 1.3 Registration Discipline

- Metrics are **constructed once**:
  ```rust
  pub struct Metrics { /* counters/histograms/gauges */ }
  impl Metrics {
      pub fn new(registry: &prometheus::Registry) -> Self { /* register */ }
  }
````

* Expose `&Metrics` on `RonAppSdk` (or store in `Arc<Metrics>`).
* CI “grep guard” forbids duplicate registration strings.

---

## 2) Health & Readiness

The SDK doesn’t own endpoints. It provides **programmatic readiness** for host `/readyz`.

### 2.1 Programmatic API

* `RonAppSdk::ready() -> ReadyReport`

  ```json
  { "ready": true, "checks": { "config": "ok", "transport": "ok", "tor": "n/a" } }
  ```
* `Ready keys`:

  * `config` — validated SDK config (gateway_addr, deadlines, retry caps).
  * `transport` — TLS client instantiated and handshake attempted to gateway (optional proactive probe).
  * `tor` — SOCKS reachable and bootstrap observed (when `transport=tor`).

### 2.2 Failure Semantics (for host mapping)

* If any key is not “ok”, host should expose `/readyz` = **503** with JSON:

  ```json
  { "sdk_ready": false, "missing": ["transport"], "retry_after": 5 }
  ```
* SDK calls **fail closed** (typed errors) when not ready (e.g., `TorUnavailable`, `Transport`).

---

## 3) Logs

### 3.1 Format

* JSON lines (`application/jsonl`), one event per call attempt and one summary per call.
* **Required fields** (all lower_snake_case):

  * `ts` (ISO8601), `level` (`INFO|WARN|ERROR|DEBUG`)
  * `service` (host sets), `crate` = `"ron-app-sdk"`
  * `event` (`"sdk.call"|"sdk.retry"|"sdk.decode"|"sdk.error"`)
  * `endpoint` (`"mailbox_send"|"storage_put"|...`)
  * `attempt` (u32), `max_attempts` (u32)
  * `deadline_ms` (i64), `latency_ms` (i64, summary only)
  * `outcome` (`"ok"|"error"`), `error_kind` (stable enum string)
  * `corr_id` (UUID/ULID if present), `node_profile` (`"micronode"|"macronode"`), `amnesia` (bool)
  * `frame_count` (u32), `payload_len` (u64)
  * `transport` (`"tls"|"tor"`)

### 3.2 Redaction & Secrets

* **Never** log capability bodies or idempotency keys; include `idem_key_redacted=true` flag only.
* Truncate paths/URLs to origin; log content lengths, not contents.
* Config diff logs redact secrets.

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` JSON formatter. OTEL export via feature flag is allowed (host decides).
* **Span names** (consistent, low-cardinality):

  * `sdk.ron-app-sdk.call`
  * `sdk.ron-app-sdk.retry`
  * `sdk.ron-app-sdk.decode`
* **Span attributes** (at minimum):

  * `endpoint`, `attempt`, `deadline_ms`, `payload_len`, `frame_count`, `node_profile`, `amnesia`, `transport`, `outcome`
* **Correlation IDs**:

  * If host injects W3C `traceparent` / B3 headers into capability metadata, the SDK **propagates** them end-to-end.
  * Provide helper: `SdkTracing::propagate_from_current_span(headers_mut)`.

**Example setup (host side):**

```rust
use tracing_subscriber::{fmt, EnvFilter};
fn init_tracing() {
    let filter = EnvFilter::from_default_env()
        .add_directive("ron_app_sdk=info".parse().unwrap());
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_current_span(false).flatten_event(true))
        .init();
}
```

---

## 5) Alerts & SLOs

### 5.1 Standard SLOs (SDK-added)

* **Overhead**: SDK-added median latency ≤ **2 ms**; p95 ≤ **5 ms** for payloads ≤ 64 KiB (loopback, warm TLS).
* **Error rate**: non-retriable error rate (auth/OAP/schema) < **0.1%** of total calls.
* **Retry pressure**: retries/call p95 ≤ **2** under transient faults (20% 5xx + 2% timeouts).
* **OAP violations**: **0** per release (`sdk_oap_violations_total == 0` steady state).

### 5.2 Example Alerts (PromQL, host scrapes SDK metrics)

* **Transport instability**

  ```promql
  increase(sdk_io_timeouts_total{op="connect"}[5m]) > 50
  ```
* **Auth failures spike**

  ```promql
  rate(sdk_auth_failures_total[5m]) > 1
  ```
* **Retry storm**

  ```promql
  rate(sdk_backoff_retries_total[5m]) > 100
  ```
* **Integrity mismatch**

  ```promql
  increase(sdk_integrity_verify_total{outcome="mismatch"}[1h]) > 0
  ```

> Each alert **must** link to a `RUNBOOK.md` entry (triage: check gateway, tokens, DNS/Tor).

---

## 6) CI / Enforcement

* **Unit tests** assert metrics registration once and label sets are low-cardinality.
* **Doctests** for logging ensure required fields exist.
* **Microbench** job records SDK overhead p50/p95; compare against SLO.
* **Lint**: forbid logging secrets (`deny(secrets_in_logs)` custom lint or grep).
* **Golden file** tests: JSON event schema (serde snapshots) for `sdk.call` and `sdk.error`.

---

## 7) Copy-Paste: Metrics Handle

```rust
// src/metrics.rs (sketch)
use prometheus::{Registry, IntCounterVec, HistogramVec, IntGaugeVec};

#[derive(Clone)]
pub struct Metrics {
    pub reqs: IntCounterVec,
    pub lat: HistogramVec,
    pub inflight: IntGaugeVec,
    pub retries: IntCounterVec,
    pub timeouts: IntCounterVec,
    pub oap_violations: IntCounterVec,
    pub auth_failures: IntCounterVec,
}

impl Metrics {
    pub fn new(reg: &Registry) -> Self {
        let reqs = IntCounterVec::new(
            prometheus::Opts::new("sdk_requests_total", "SDK calls"),
            &["endpoint","outcome","code"]
        ).unwrap();
        let lat = HistogramVec::new(
            prometheus::HistogramOpts::new("sdk_request_latency_seconds", "End-to-end latency"),
            &["endpoint"]
        ).unwrap();
        let inflight = IntGaugeVec::new(
            prometheus::Opts::new("sdk_inflight_requests", "In-flight calls"),
            &["endpoint"]
        ).unwrap();
        let retries = IntCounterVec::new(
            prometheus::Opts::new("sdk_backoff_retries_total", "Retries performed"),
            &["endpoint","reason"]
        ).unwrap();
        let timeouts = IntCounterVec::new(
            prometheus::Opts::new("sdk_io_timeouts_total", "Timeouts by op"),
            &["op"]
        ).unwrap();
        let oap_violations = IntCounterVec::new(
            prometheus::Opts::new("sdk_oap_violations_total", "OAP/DTO violations"),
            &["kind"]
        ).unwrap();
        let auth_failures = IntCounterVec::new(
            prometheus::Opts::new("sdk_auth_failures_total", "Auth failures"),
            &["reason"]
        ).unwrap();

        reg.register(Box::new(reqs.clone())).unwrap();
        reg.register(Box::new(lat.clone())).unwrap();
        reg.register(Box::new(inflight.clone())).unwrap();
        reg.register(Box::new(retries.clone())).unwrap();
        reg.register(Box::new(timeouts.clone())).unwrap();
        reg.register(Box::new(oap_violations.clone())).unwrap();
        reg.register(Box::new(auth_failures.clone())).unwrap();

        Self { reqs, lat, inflight, retries, timeouts, oap_violations, auth_failures }
    }
}
```

---

## 8) Copy-Paste: Span Emission

```rust
pub async fn storage_put_with_spans(
    sdk: &RonAppSdk, cap: Capability, blob: bytes::Bytes, deadline: std::time::Duration
) -> Result<AddrB3, SdkError> {
    let span = tracing::info_span!(
        "sdk.ron-app-sdk.call",
        endpoint = "storage_put",
        payload_len = blob.len() as i64,
        deadline_ms = deadline.as_millis() as i64,
    );
    let _g = span.enter();
    let t0 = std::time::Instant::now();
    match sdk.storage_put(cap, blob, deadline, None).await {
        Ok(addr) => {
            tracing::info!(outcome="ok", latency_ms=?t0.elapsed().as_millis(), content_id=%addr.to_string());
            Ok(addr)
        }
        Err(e) => {
            tracing::warn!(outcome="error", error_kind=%e.kind(), latency_ms=?t0.elapsed().as_millis());
            Err(e)
        }
    }
}
```

---

## 9) Red/Green Dashboards (suggested panels)

* **Requests**: `sdk_requests_total` stacked by `endpoint`
* **Latency**: `sdk_request_latency_seconds` p50/p95/p99 by `endpoint`
* **Timeouts & Retries**: `sdk_io_timeouts_total` and `sdk_backoff_retries_total`
* **Auth/OAP**: `sdk_auth_failures_total`, `sdk_oap_violations_total`
* **Tor**: `sdk_tor_events_total` (if used)

---

## 10) Review & Maintenance

* Refresh this document **every 90 days** or on observability schema changes.
* Any new public API **must** add metrics/tracing/log coverage and tests.
* Keep metric names **stable**; if renaming, provide an alias period and dashboard migration notes.

---

```

