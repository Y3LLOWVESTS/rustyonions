````markdown
# 📈 OBSERVABILITY.md — svc-overlay

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- **Metrics** (Prometheus via `metrics` facade or `prometheus` registry)
- **Health & readiness** truth semantics
- **Logs** (structured JSON fields & redaction)
- **Tracing** (spans, correlation)
- **Alerts & SLOs** (actionable, runbook-linked)

> Overlay role reminder: `svc-overlay` is the **session & gossip plane** (OAP/1 framing, backpressure, fairness). It is **not** DHT/discovery.

---

## 1) Metrics (Prometheus-style)

> **Library**: prefer the `metrics` facade (`metrics`, `metrics-exporter-prometheus`) so we can swap backends; alt path: `prometheus` crate with a `Registry`.  
> **Registration**: define names **once** in a module (e.g., `observe.rs`) and re-use via functions/macros; never register per-request.

### 1.1 Golden Metrics (every service)

| Name & Labels | Type | Notes |
|---|---|---|
| `http_requests_total{route,method,status}` | Counter | Admin plane (metrics/health/ready/version) |
| `request_latency_seconds{route,method}` | Histogram | Use explicit buckets (e.g., 5ms…5s, +∞) |
| `inflight_requests{route}` | Gauge | Or infer from concurrency limiter |
| `service_restarts_total` | Counter | Supervision restarts (reason in logs) |
| `bus_lagged_total` | Counter | Broadcast drops due to lag |
| `rejected_total{reason}` | Counter | Canonical reject reasons (see §5) |

> **Buckets**: aim for latency buckets: `0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, +Inf`.

### 1.2 Overlay-Specific Metrics

**Sessions & handshakes**
- `overlay_sessions_active` (Gauge)
- `overlay_sessions_open_total` (Counter)
- `overlay_handshake_attempts_total` (Counter)
- `overlay_handshake_failures_total{reason}` (Counter)  
  *reasons*: `timeout|tls|mtls|pq_neg_fail|ratelimit|proto`

**Frames, bytes & flow**
- `overlay_frames_total{dir,kind}` (Counter) — `dir=rx|tx`, `kind=gossip|ctrl|other`
- `overlay_bytes_total{dir}` (Counter)
- `overlay_frame_size_bytes{dir}` (Histogram) — ensure cap at **1 MiB**; bucket up to 1 MiB
- `overlay_chunk_bytes{dir}` (Histogram) — application chunk (~64 KiB typical)
- `overlay_backpressure_events_total{peer_bucket}` (Counter) — bucket peer by coarse class: `tiny|small|med|large` (avoid per-peer cardinality)

**Queues & fairness**
- `overlay_queue_depth{queue}` (Gauge) — `queue=ingress|gossip_ingress|egress`
- `overlay_queue_dropped_total{queue,reason}` (Counter) — `reason=full|timeout|shed`
- `overlay_egress_permits{state}` (Gauge) — `state=available|in_use`
- `overlay_fanout_current` (Gauge) — instantaneous gossip fanout

**Security & PQ**
- `auth_failures_total{reason}` (Counter) — `unauth|expired|invalid_caveat`
- `tls_handshake_failures_total{reason}` (Counter) — `cert|version|cipher|alpn`
- `overlay_pq_negotiations_total{mode,outcome}` (Counter) — `mode=off|hybrid`, `outcome=ok|fail|incompatible`

**Paths (optional features)**
- `overlay_tor_bootstrap_seconds` (Histogram) — if `arti` enabled
- `overlay_quic_path_events_total{event}` (Counter) — `amplification|mtu_change|path_migrate`

### 1.3 Cardinality & Discipline

- **Never** label with raw `peer_id` or `addr`. Use **bucketed** or **hashed-prefix** labels only where needed.
- Define metrics in a single **inventory** (e.g., `observe::inventory()`), and expose helper fns:
  ```rust
  // facade examples
  metrics::increment_counter!("overlay_handshake_attempts_total");
  metrics::histogram!("overlay_frame_size_bytes", len as f64, "dir" => "rx");
  metrics::gauge!("overlay_queue_depth", depth as f64, "queue" => "egress");
````

* **CI**: a grep test ensures no dynamic metric names; labels restricted to whitelisted sets.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness**: returns `200 OK` if process event loop ticks.
* `GET /readyz` — **readiness truth**: `200 OK` only when safe to accept **writes**.

### 2.2 Readiness Check Matrix

| Key               | Meaning                                     | Source                  |
| ----------------- | ------------------------------------------- | ----------------------- |
| `listeners_bound` | sockets bound & accepting                   | listener task heartbeat |
| `metrics_bound`   | metrics server bound                        | admin server            |
| `cfg_loaded`      | config validated, snapshot swapped          | config watcher          |
| `pq_ready`        | if `pq_mode=hybrid`: KMS path available     | KMS client              |
| `tor_bootstrap`   | if `arti`: circuit ready                    | arti client             |
| `queues_ok`       | sustained depth < 90% over 30s              | queue sampler           |
| `shed_rate_ok`    | `rejected_total{reason="busy"}` < threshold | rate window             |
| `fd_headroom`     | file descriptors > reserve                  | proc sampler            |

### 2.3 Failure Semantics

* **Writes fail first** under pressure (fair shedding); `/readyz` flips **degraded** when:

  * `queues_ok=false` OR `shed_rate_ok=false` (sustained 30s), OR any **required** capability (e.g., `pq_ready` when configured) is missing.
* **Responses**:

  * `200 OK` → `{ "ready": true }`
  * `503 Service Unavailable` →

    ```json
    {
      "ready": false,
      "degraded": true,
      "missing": ["pq_ready","queues_ok"],
      "retry_after": 5
    }
    ```
* **Metric**: `readyz_state` gauge (`0=down`, `1=ready`, `2=degraded`).

---

## 3) Logs

### 3.1 Format (JSON lines)

Required fields per event:

| Field        | Example                    | Notes                                                |                  |                      |                                                 |        |
| ------------ | -------------------------- | ---------------------------------------------------- | ---------------- | -------------------- | ----------------------------------------------- | ------ |
| `ts`         | `2025-10-03T20:13:58.123Z` | RFC3339/ISO8601                                      |                  |                      |                                                 |        |
| `level`      | `INFO`                     | `INFO                                                | WARN             | ERROR                | DEBUG                                           | TRACE` |
| `service`    | `svc-overlay`              | constant                                             |                  |                      |                                                 |        |
| `event`      | `handshake_fail`           | enum-like (`accept`, `handshake_start                | ok               | fail`, `oap_frame_rx | tx`, `reject`, `readyz_change`, `shutdown_begin | end`)  |
| `reason`     | `timeout`                  | aligns with metrics label values                     |                  |                      |                                                 |        |
| `corr_id`    | ULID/UUID                  | injected/propagated                                  |                  |                      |                                                 |        |
| `peer_addr`  | `1.2.3.4:443`              | omit for UDS; do not log for Tor if policy disallows |                  |                      |                                                 |        |
| `peer_id`    | `ed25519:…`                | **optional**, only if policy allows                  |                  |                      |                                                 |        |
| `cap_id`     | `cap:feed:write`           | capability class, not token                          |                  |                      |                                                 |        |
| `latency_ms` | `12`                       | for operations                                       |                  |                      |                                                 |        |
| `bytes`      | `65536`                    | for frame events                                     |                  |                      |                                                 |        |
| `dir`        | `rx                        | tx`                                                  | for frame events |                      |                                                 |        |

### 3.2 Redaction & Sampling

* Never log **tokens**, **keys**, or raw payloads.
* Redact/omit `peer_addr` if Tor policy dictates.
* Sample repetitive errors (e.g., handshake floods) to **1:N** with counters still incrementing.
* Config diffs logged with **secrets redacted**.

---

## 4) Tracing & Correlation

* **Crates**: `tracing`, `tracing-subscriber` (JSON), optional `tracing-opentelemetry` under `otel` feature.
* **Span naming** (hierarchical):

  * `svc.overlay.accept`
  * `svc.overlay.handshake`
  * `svc.overlay.session`
  * `svc.overlay.gossip.ingress`
  * `svc.overlay.gossip.egress`
* **Fields**: `corr_id`, `peer_id?`, `dir`, `frame_bytes`, `fanout`, `pq_mode`.
* **Correlation IDs**:

  * Inject on ingress (HTTP admin: `X-Corr-ID`; overlay path: generate if absent).
  * Propagate via bus events and span fields.
* **Example**:

  ```rust
  #[tracing::instrument(name = "svc.overlay.handshake", skip(stream), fields(corr_id=%cid, pq_mode=%pq_mode))]
  async fn do_handshake(stream: &mut TlsStream) -> Result<()> { /* … */ }
  ```

---

## 5) Alerts & SLOs

### 5.1 SLOs (overlay-specific, tune per env)

* **Handshake p95** `< 50ms` intra-cluster; `< 150ms` inter-region.
* **Frame reject rate** (`rejected_total{reason=~"oversize|ratio_cap"}` / `overlay_frames_total{dir="rx"}`) `< 0.5%`.
* **Busy/Shedding rate** (`rejected_total{reason="busy"}` / requests) `< 1%` sustained.
* **Readiness availability** (`readyz_state==1`) `≥ 99.9%` monthly.

### 5.2 Alert Rules (PromQL sketches)

* **Handshake flood (warning)**

  ```
  rate(overlay_handshake_attempts_total[1m]) > 500
    and on() rate(overlay_handshake_failures_total{reason="ratelimit"}[1m]) / rate(overlay_handshake_attempts_total[1m]) > 0.2
  ```
* **Sustained shedding (critical)**

  ```
  rate(rejected_total{reason="busy"}[5m]) > 50
    and (avg_over_time(overlay_queue_depth{queue="egress"}[5m]) / QUEUE_CAP) > 0.9
  ```
* **Oversize/ratio cap hits (warning)**

  ```
  rate(rejected_total{reason=~"oversize|ratio_cap"}[10m]) > 0
  ```
* **Ready degraded (critical)**

  ```
  max_over_time(readyz_state[2m]) == 2
  ```

> **Runbooks**: each alert links to `RUNBOOK.md#<anchor>` with triage (which dashboards to check, how to adjust caps safely, how to drain).

---

## 6) CI / Enforcement

* **Interface checks**:

  * `/metrics`, `/healthz`, `/readyz` endpoints exist and respond.
  * `/readyz` flips to **503** under induced queue saturation in an integration test.
* **Metrics schema golden test**:

  * Assert emission of required counters/histograms on synthetic traffic.
  * Grep for dynamic metric names; enforce **label allowlist**.
* **Logging**:

  * Redaction unit tests (no secret tokens/keys appear).
  * Size caps verified (truncate large error chains).
* **Lint gates**:

  * `clippy::await_holding_lock` denied.
  * Backpressure paths tested (drop vs reject) with metrics assertions.
* **Docs**:

  * This file reviewed every **90 days**; PRs altering observability must update it.

---

## 7) Implementation Notes (copy-paste)

**Prometheus exporter (metrics facade)**

```rust
// Cargo.toml
// metrics = "0.24"
// metrics-exporter-prometheus = "0.15"
// tracing = "0.1", tracing-subscriber = { version="0.3", features=["json"] }

use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

pub fn start_metrics(addr: SocketAddr) -> anyhow::Result<()> {
    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        .set_buckets_for_metric(vec![
            ("request_latency_seconds", vec![0.005,0.01,0.02,0.05,0.1,0.2,0.5,1.0,2.0,5.0]),
            ("overlay_frame_size_bytes", vec![512.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 524288.0, 1048576.0]),
        ])?;
    builder.install()?;
    Ok(())
}
```

**Emit examples**

```rust
// Handshake:
metrics::increment_counter!("overlay_handshake_attempts_total");
let _t = std::time::Instant::now();
// ...
metrics::histogram!("request_latency_seconds", _t.elapsed().as_secs_f64(), "route" => "handshake", "method" => "tcp");

// Frame RX:
metrics::increment_counter!("overlay_frames_total", "dir" => "rx", "kind" => "gossip");
metrics::histogram!("overlay_frame_size_bytes", frame_len as f64, "dir" => "rx");

// Queue depth (sampled):
metrics::gauge!("overlay_queue_depth", depth as f64, "queue" => "egress");
```

**Readiness aggregator sketch**

```rust
#[derive(Default)]
struct Ready {
    listeners_bound: bool,
    metrics_bound: bool,
    cfg_loaded: bool,
    pq_ready: Option<bool>,     // None if pq off
    tor_bootstrap: Option<bool>,// None if arti off
    queues_ok: bool,
    shed_rate_ok: bool,
    fd_headroom: bool,
}

impl Ready {
    fn state(&self) -> (u16, serde_json::Value) {
        let mut missing = vec![];
        let mut degraded = false;

        macro_rules! need { ($k:ident) => { if !self.$k { missing.push(stringify!($k)); degraded = true; } } }
        need!(listeners_bound);
        need!(metrics_bound);
        need!(cfg_loaded);
        need!(queues_ok);
        need!(shed_rate_ok);
        need!(fd_headroom);
        if let Some(false) = self.pq_ready { missing.push("pq_ready"); degraded = true; }
        if let Some(false) = self.tor_bootstrap { missing.push("tor_bootstrap"); degraded = true; }

        if degraded {
            (503, serde_json::json!({"ready": false, "degraded": true, "missing": missing, "retry_after": 5}))
        } else { (200, serde_json::json!({"ready": true})) }
    }
}
```

**Tracing subscriber (JSON)**

```rust
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::from_default_env()
        .add_directive("svc_overlay=info".parse().unwrap());
    fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .flatten_event(true)
        .init();
}
```

---

## 8) Dashboards (starter pack)

* **Overview**: `readyz_state`, `overlay_sessions_active`, `overlay_frames_total{dir}`, `overlay_bytes_total{dir}`, `overlay_queue_depth{queue}`.
* **Saturation**: `rejected_total{reason}`, `overlay_backpressure_events_total`, `overlay_egress_permits{state}`.
* **Security**: `auth_failures_total{reason}`, `tls_handshake_failures_total{reason}`, `overlay_pq_negotiations_total{outcome}`.
* **Latency**: `request_latency_seconds{route}`, `overlay_frame_size_bytes{dir}` (CDF), handshake p95.

---

## 9) Policy & Hygiene

* **No high-cardinality labels**; any proposal adding `peer_id`/`addr` to labels must be reviewed by owner + ops.
* **Metric names are API**: any rename → alias for ≥1 minor + CHANGELOG note.
* **Logs are evidence**: if it didn’t happen in logs/metrics, assume it didn’t happen.
* **Truth over comfort**: `/readyz` must reflect reality; **never** keep it green to “avoid flapping.”

---

✅ With this plan, `svc-overlay` emits **consistent, low-cardinality, decision-ready** signals. You can watch frame flow, fairness, saturation, and security posture in real time—and you can prove behavior under chaos without guessing.

```
```
