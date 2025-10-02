---

# 📈 OBSERVABILITY — `ron-transport`

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible); Tokio 1.x; Loom 0.7+ (dev-only)*

`ron-transport` is a **library** crate. It exposes metrics handles, logs, spans, and a tri-state **readiness** API that callers can wire to their own `/metrics`, `/healthz`, `/readyz`. It does **not** host HTTP endpoints itself.

---

## 0) Purpose

Define **what is observable**, **how it’s exposed**, and **how it’s consumed** by embedding services:

* Metrics (Prometheus/OpenTelemetry)
* Readiness semantics (tri-state API)
* Logs (structured JSON schema)
* Tracing spans & correlation
* Alerts & SLOs (recommended, consumed by services)

---

## 1) Metrics (Prometheus-style)

### 1.1 Transport “golden” metrics (crate-specific)

> All names are stable; labels are **strictly bounded** to avoid high-cardinality explosions.

| Name                           | Type      | Labels                | Description                                     | Units / Buckets                               |                            |                                      |                                                         |     |
| ------------------------------ | --------- | --------------------- | ----------------------------------------------- | --------------------------------------------- | -------------------------- | ------------------------------------ | ------------------------------------------------------- | --- |
| `transport_dials_total`        | Counter   | `backend`=`tcp        | tls                                             | arti`, `result`=`ok                           | err`                       | Outbound dial attempts and outcomes. | n/a                                                     |     |
| `transport_accepts_total`      | Counter   | `backend`             | Accepted inbound connections.                   | n/a                                           |                            |                                      |                                                         |     |
| `handshake_latency_seconds`    | Histogram | `backend`             | Time from SYN/accept to ready-to-R/W.           | Buckets: `[0.01,0.025,0.05,0.1,0.25,0.5,1,2]` |                            |                                      |                                                         |     |
| `conn_inflight`                | Gauge     | `backend`             | Currently open connections.                     | count                                         |                            |                                      |                                                         |     |
| `bytes_in_total`               | Counter   | `backend`             | Total bytes read from peers.                    | bytes                                         |                            |                                      |                                                         |     |
| `bytes_out_total`              | Counter   | `backend`             | Total bytes written to peers.                   | bytes                                         |                            |                                      |                                                         |     |
| `io_read_size_bytes`           | Histogram | `backend`             | Size of read chunks.                            | P2 buckets up to `1MiB`                       |                            |                                      |                                                         |     |
| `io_write_size_bytes`          | Histogram | `backend`             | Size of write chunks.                           | P2 buckets up to `1MiB`                       |                            |                                      |                                                         |     |
| `rejected_total`               | Counter   | `reason`=`timeout     | too_large                                       | rate_limit                                    | peer_limit                 | arti_bootstrap`                      | All **early rejections** (backpressure over buffering). | n/a |
| `tls_handshake_failures_total` | Counter   | `backend`             | TLS handshake failures surfaced from rustls.    | n/a                                           |                            |                                      |                                                         |     |
| `queue_depth`                  | Gauge     | `queue`=`writer_inbox | work                                            | events`                                       | Depth of bounded channels. | items                                |                                                         |     |
| `queue_dropped_total`          | Counter   | `queue`               | Items dropped due to lag/full.                  | n/a                                           |                            |                                      |                                                         |     |
| `tasks_spawned_total`          | Counter   | `kind`=`reader        | writer`                                         | Per-connection task spawns.                   | n/a                        |                                      |                                                         |     |
| `tasks_aborted_total`          | Counter   | `kind`                | Per-connection task aborts (deadline exceeded). | n/a                                           |                            |                                      |                                                         |     |

**Cardinality budget**

* `backend` ∈ `{tcp,tls,arti}` (feature-gated).
* `result` ∈ `{ok,err}` only.
* `reason` enumerated as above; **no free-form strings**.
* **Never** label with `peer_addr`, cert subjects, or IDs.

**Registration discipline**

* All metrics **registered once** (e.g., in `Metrics::new()` using `OnceLock`).
* Only **handles** are cloned into tasks.
* CI denies duplicate registration by grepping for `register_*` outside the metrics module.

**Example (pseudo-Rust)**

```rust
pub struct Metrics {
    pub dials: prometheus::IntCounterVec,
    pub accepts: prometheus::IntCounterVec,
    pub handshake_latency: prometheus::HistogramVec,
    pub conn_inflight: prometheus::IntGaugeVec,
    pub bytes_in: prometheus::IntCounterVec,
    pub bytes_out: prometheus::IntCounterVec,
    pub rejected: prometheus::IntCounterVec,
    // ...
}

static METRICS: once_cell::sync::OnceCell<Metrics> = once_cell::sync::OnceCell::new();

pub fn metrics() -> &'static Metrics { METRICS.get().expect("init metrics first") }
```

**OTEL mapping (if exporter enabled)**

* Histograms -> OTEL `Histogram` with unit `"s"` (latency) or `"By"` (bytes).
* Counters -> `Sum` (monotonic true).
* Gauges -> `Gauge` (asynchronous preferred for `conn_inflight`/`queue_depth`).

---

## 2) Health & Readiness

`ron-transport` exposes **readiness as an API**, not an endpoint:

```rust
#[derive(Clone, Debug)]
pub enum ReadyState {
  Ready,
  Degraded { reason: &'static str },  // e.g., "arti_bootstrap"
  NotReady { reason: &'static str },  // e.g., "tls_config_missing"
}

pub trait TransportHealth {
  fn readiness(&self) -> ReadyState;
}
```

**Readiness keys (informational for consumers)**

* `listener_bound` (socket bound successfully)
* `tls_config_loaded` (when `backend=tls`)
* `arti_bootstrap` (when `backend=arti`; `Degraded` until circuits usable)

**Consumer mapping (recommended)**

* `/healthz` (liveness): always `200` if process alive.
* `/readyz` (readiness):

  * `Ready` → `200 OK`.
  * `Degraded(reason)` → `503`, body `{"degraded":true,"reason":"<reason>"}` (or `200` if policy allows degraded).
  * `NotReady(reason)` → `503`, body `{"ready":false,"reason":"<reason>"}`.

**Failure semantics**

* Prefer **fail-closed on writes** when `Degraded/NotReady`.
* Reads can be **fail-open** at the consumer’s discretion, but should be rate-limited.

---

## 3) Logs

### 3.1 Format & schema

* **JSON lines** (`application/jsonl`), one event per line.
* **Required fields** (keys are stable and lower_snake_case):

  * `ts` (ISO8601 with timezone)
  * `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  * `crate` = `"ron-transport"`
  * `backend` (`"tcp"|"tls"|"arti"`)
  * `event` (`"dial"|"accept"|"handshake"|"read"|"write"|"reject"|"shutdown"`)
  * `reason` (if `event="reject"`, one of the canonical reasons)
  * `conn_id` (short hex or ULID; per-connection)
  * `peer_addr` (when safe; **do not** log for Tor/Arti unless policy allows)
  * `duration_ms` (for timed ops)
  * `bytes` (for read/write events)
  * `corr_id` (if provided by caller; propagate verbatim)

**Redaction & secrets**

* **Never** log PEMs, keys, tokens, or plaintext payloads.
* Redact `peer_addr` under Arti unless explicitly enabled for forensics (gated by consumer policy).

**Examples**

```json
{"ts":"2025-09-28T15:31:22.481Z","level":"INFO","crate":"ron-transport","backend":"tls","event":"handshake","conn_id":"01JD...","duration_ms":38}
{"ts":"2025-09-28T15:31:23.002Z","level":"WARN","crate":"ron-transport","backend":"tcp","event":"reject","reason":"rate_limit","conn_id":"01JD..."}
{"ts":"2025-09-28T15:31:25.114Z","level":"ERROR","crate":"ron-transport","backend":"arti","event":"reject","reason":"arti_bootstrap"}
```

---

## 4) Tracing & Correlation

* Use `tracing` crate; JSON subscriber recommended for prod.
* **Span hierarchy** (suggested):

  * `transport.conn` — per-connection span; attrs: `backend`, `conn_id`, `local_addr`, `peer_addr?`

    * `transport.handshake` — measured by `handshake_latency_seconds`
    * `transport.read` — repeated; annotate `bytes`
    * `transport.write` — repeated; annotate `bytes`
* **Correlation ID** (`corr_id`):

  * Caller injects (HTTP `X-Corr-ID` or bus metadata).
  * Propagate as a span field and log field; do not generate here unless absent and service policy allows.

**Example**

```rust
#[tracing::instrument(name="transport.conn", skip_all, fields(backend=?backend, conn_id=%id))]
async fn run_conn(...) -> Result<()> {
    let _hs = tracing::info_span!("transport.handshake").entered();
    // ...
    Ok(())
}
```

**OTEL exporters**

* Optional feature flag in the *consumer*. Spans map 1:1; resource attrs should include `service.name` (consumer) and `library.name="ron-transport"`.

---

## 5) Alerts & SLOs (recommended; implemented by consumers)

**SLOs (intra-DC targets; adjust per environment)**

* `handshake_latency_seconds` p95 ≤ **100 ms**; p99 ≤ **250 ms**.
* `rejected_total{reason!="arti_bootstrap"}` ratio ≤ **1%** of total dials+accepts.
* `tls_handshake_failures_total` baseline: **0** sustained; bursts trigger investigation.
* `conn_inflight{backend="arti"}` should stabilize after bootstrap; prolonged zero with traffic → degraded.

**Example Prometheus alerts**

```promql
# High rejection rate (excluding arti bootstrap during warmup)
sum(rate(rejected_total{reason!="arti_bootstrap"}[5m]))
/
sum(rate(transport_dials_total[5m]) + rate(transport_accepts_total[5m])) > 0.05
```

```promql
# TLS handshake failures sustained
rate(tls_handshake_failures_total[10m]) > 0
```

```promql
# Handshake latency regression
histogram_quantile(0.95, sum(rate(handshake_latency_seconds_bucket[5m])) by (le)) > 0.1
```

**Runbook pointers (suggested)**

* High `rate_limit` → increase `egress/ingress_bytes_per_sec` or reduce producers; verify backpressure policy.
* High `too_large` → upstream frame sizing bug; check OAP bounds.
* `arti_bootstrap` sustained → check Tor network, bootstrap timeout, egress firewall.

---

## 6) CI / Enforcement

* Metric **smoke tests**: opening/closing a loopback connection must increment `transport_accepts_total`, update `conn_inflight`, and fill `handshake_latency_seconds`.
* **Name stability test**: unit test enumerates expected metric names/labels to prevent accidental renames.
* Logs **contract test**: sample events contain required fields (`event`, `backend`, `conn_id`, `reason?`).
* Readiness **contract test**: arti backend reports `Degraded("arti_bootstrap")` before `Ready`.
* Lints: ensure no `await` under lock in hot paths; enforce histogram bucket sets via a constants module.

---

## 7) Dashboards (starter kit)

**Transport Overview**

* Row 1: `conn_inflight` by backend (stacked area), `transport_dials_total` & `transport_accepts_total` rates (per backend).
* Row 2: `handshake_latency_seconds` p50/p95/p99 (quantiles), `rejected_total` by reason (bar).
* Row 3: `bytes_in_total` / `bytes_out_total` rates, `tls_handshake_failures_total` (single stat).
* Row 4: `queue_depth{queue="writer_inbox"}` and `queue_dropped_total` (to spot slow consumers).

**Arti Focus (when enabled)**

* `rejected_total{reason="arti_bootstrap"}` (rate), `conn_inflight{backend="arti"}`, time-to-ready (derived from logs/spans), overlay service `/readyz` status.

---

## 8) Metric Reference (copy/paste)

```text
transport_dials_total{backend,result}
transport_accepts_total{backend}
handshake_latency_seconds{backend}
conn_inflight{backend}
bytes_in_total{backend}
bytes_out_total{backend}
io_read_size_bytes{backend}
io_write_size_bytes{backend}
rejected_total{reason}
tls_handshake_failures_total{backend}
queue_depth{queue}
queue_dropped_total{queue}
tasks_spawned_total{kind}
tasks_aborted_total{kind}
```

---

## 9) Integration Notes (for consumers)

* Expose `/metrics`, `/healthz`, `/readyz` in your service; **wire** `ron-transport`’s metrics registry and readiness API to those endpoints.
* Use **the same bucket sets** to keep dashboards consistent across crates.
* Respect **tri-state readiness** in autoscalers and deployment probes (treat `Degraded` as either soft-fail or hard-fail per policy).
* Never add high-cardinality labels (peer IPs, cert CNs) to transport metrics.

---

With this, `ron-transport` remains **observable, predictable, and low-cardinality** by default—easy to debug, safe to run, and trivial to dashboard across environments.
