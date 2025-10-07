---

# 📈 OBSERVABILITY — `oap`

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*
*crate type: protocol/lib (no sockets, no HTTP endpoints)*

> `oap` is a **pure protocol library**: it has **no `/metrics`, `/healthz`, `/readyz` endpoints** of its own and emits **no logs** directly.
> Observability is exposed via **library hooks** (metrics traits/handles) that hosts (e.g., `svc-gateway`, `svc-overlay`) wire into their exporters.

---

## 0) Purpose

Define **what is observable** in `oap`, **how to surface it from hosts**, and **how to use it** for:

* Metrics (Prometheus/OTel via host exporters)
* Health & readiness semantics (host-owned; `oap` provides readiness inputs)
* Logs (host JSON schema aligned with `oap` error taxonomy)
* Tracing spans & correlation (via `tracing` in host with `oap` span fields)
* Alerts & SLOs tailored to OAP/1 framing paths

---

## 1) Metrics (Prometheus-style)

### 1.0 Registration Discipline (lib/host contract)

* `oap` defines a **Metrics facade** (cloneable handle) with **no global registry usage**.
* Hosts **create and register** Prometheus/OTel instruments **once** and pass a handle into `oap`.
* CI guard in hosts: grep to prevent **duplicate registration**; unit test that handle is shared.

### 1.1 Golden Metrics (host-level, always present)

> These are emitted by services embedding `oap` (gateway/overlay/etc.). They’re standard across RustyOnions.

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram)
* `inflight_requests{route}` (Gauge)
* `bus_lagged_total` (Counter)
* `service_restarts_total` (Counter)
* `rejected_total{reason}` (Counter)

### 1.2 OAP-Specific (library-signature; host-exported)

> Names are **canonical** for all `oap` embeddings.

* `oap_frames_total{kind="HELLO|START|DATA|ACK|END|ERROR"}` (Counter)
  Count of frames successfully parsed or emitted.
* `oap_rejects_total{reason="frame_oversize|unknown_envelope|header_malformed|credit_violation|missing_obj|truncated"}` (Counter)
  Deterministic parser/validator rejects.
* `oap_frame_bytes` (Counter)
  Total payload bytes processed by the framer.
* `oap_inflight_frames` (Gauge)
  Algebraic inflight (seq window) managed by the host using `ACK{up_to}` results.
* `oap_ack_window` (Gauge)
  Current advertised ACK credit window (frames).
* `oap_read_frame_seconds` / `oap_write_frame_seconds` (Histogram)
  Latency measured **by host** around `oap::decode/encode`.
* (Optional ECON/GOV tag) `economic="true|false"` label **only if** the host processes frames within ledger/rewarder context. This tag is **non-normative** and must not alter behavior.

### 1.3 Recommended Buckets / Cardinality Rules

* Histograms: `request_latency_seconds` and `oap_read_frame_seconds` should share buckets (e.g., `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1]`).
* Avoid high-cardinality labels on `oap_rejects_total`; **only** use the enumerated `reason` values above.

### 1.4 Minimal Rust Wiring (host side)

```rust
/// Host-owned metrics handle passed into oap.
#[derive(Clone)]
pub struct OapMetrics {
    pub frames_total: prometheus::CounterVec,
    pub rejects_total: prometheus::CounterVec,
    pub frame_bytes: prometheus::Counter,
    pub inflight_frames: prometheus::Gauge,
    pub ack_window: prometheus::Gauge,
    pub read_frame_sec: prometheus::Histogram,
    pub write_frame_sec: prometheus::Histogram,
}

// In host bootstrap:
let frames_total = prometheus::register_counter_vec!(
    "oap_frames_total", "Count of OAP frames", &["kind"]
)?;
let rejects_total = prometheus::register_counter_vec!(
    "oap_rejects_total", "Count of OAP rejects", &["reason"]
)?;
// ... (others similar) ...
let oap_metrics = OapMetrics { frames_total, rejects_total, /* ... */ };

// Pass to oap:
let parser = oap::Parser::with_metrics(oap_metrics.clone());
```

---

## 2) Health & Readiness

> `oap` does **not** own endpoints; hosts expose `/healthz` and `/readyz`. `oap` supplies **readiness inputs** and **never blocks** liveness.

### 2.1 Endpoints (host)

* `/healthz` — **liveness**: process up. `oap` has no bearing; always `200` if process alive.
* `/readyz` — **readiness**: host flips **only** when all keys satisfied (see below).

### 2.2 OAP-Related Readiness Keys (host)

* `oap_parser_ready=true` — parser constructed with valid config.
* `oap_reject_rate_ok=true` — rolling 1–5 min reject **rate < 1%** of total frames.
* `oap_ack_window_ok=true` — advertised window within configured bounds (not zero for >N seconds under load).
* `oap_backpressure_ok=true` — inflight frame gauge below threshold.

> If any are false → `/readyz` returns `503` with JSON:
> `{ "degraded": true, "missing": ["oap_reject_rate_ok"], "retry_after": 5 }`

### 2.3 Failure Semantics (host)

* **Fail-open reads / fail-closed writes** elsewhere still apply; for `oap`, **reject early** on parse errors and surface metrics.
* When sustained `oap_rejects_total` increases rapidly, **flip readiness** before queues collapse.

---

## 3) Logs

> `oap` itself doesn’t log. Hosts must map `oap` outcomes to structured JSON logs.

### 3.1 JSON Schema (host)

Required fields for events around `oap` parse/encode:

* `ts` (ISO8601), `level` (`INFO|WARN|ERROR`)
* `service` (e.g., `svc-gateway`)
* `event` (`oap_frame`, `oap_reject`)
* `reason` (must be one of the canonical `oap_rejects_total{reason}` values)
* `frame_kind` (`HELLO|START|DATA|ACK|END|ERROR`)
* `corr_id` (UUID/ULID), `peer_addr`, `tenant` (if multi-tenant)
* `latency_ms` (for read/write around decode/encode)
* **Never include raw DATA** or secrets; **do not log PQ flags** bitmasks.

Example (reject):

```json
{"ts":"2025-10-06T15:01:02Z","level":"WARN","service":"svc-gateway",
 "event":"oap_reject","reason":"frame_oversize","corr_id":"01JB2...","peer_addr":"1.2.3.4:5555"}
```

---

## 4) Tracing & Correlation

* Use `tracing` with JSON formatting at host. `oap` exposes span fields to record `frame_kind`, `seq`, `ack_up_to`.
* **Span names:**

  * `svc.gateway.oap.decode_frame`
  * `svc.gateway.oap.encode_frame`
* **Fields:** `frame_kind`, `seq`, `len`, `obj_present` (bool), `ack_up_to`.
* **Correlation IDs:** injected on ingress (e.g., HTTP `X-Corr-ID`), propagated in spans and logs.
* **OpenTelemetry:** optional exporter behind a host feature; spans should be short-lived (per frame), children of request-level span.

---

## 5) Alerts & SLOs (for services embedding `oap`)

### 5.1 SLOs (per region unless noted)

* **OAP decode p95** < **20 ms** (in-process framing).
* **OAP reject rate** < **0.5%** of frames, excluding benign `Truncated` that are followed by successful completion within 100ms.
* **ACK window starvation** time at zero < **5 s** over any 5 min window.
* **No frame oversize** events (they indicate hostile or broken peers).

### 5.2 Example Prometheus Alerts (host)

```promql
# High reject rate
(sum(rate(oap_rejects_total[5m])) / clamp_min(sum(rate(oap_frames_total[5m])), 1)) > 0.005
```

* **Warn:** for 5m
* **Crit:** for 15m

```promql
# Decode latency regression
histogram_quantile(0.95, sum(rate(oap_read_frame_seconds_bucket[5m])) by (le)) > 0.02
```

```promql
# ACK starvation
avg_over_time(oap_ack_window[5m]) == 0 and sum(rate(oap_frames_total[5m])) > 0
```

```promql
# Oversize frames present (should be ~0)
increase(oap_rejects_total{reason="frame_oversize"}[15m]) > 0
```

### 5.3 Runbooks (link from alerts)

* **Reject Rate High:** inspect `reason` breakouts; if `header_malformed` spikes, suspect client upgrade mismatch; if `credit_violation`, check host ACK logic. Throttle or quarantine offending peers.
* **Decode p95 High:** CPU pressure or pathological split patterns; look for GC/allocator churn; enable pooling or adjust chunk size (host I/O).
* **ACK Starvation:** verify host credit accounting; ensure worker pool isn’t starved; inspect backpressure thresholds and `/readyz` state.

---

## 6) CI / Enforcement

* **Unit tests** (in `oap`): ensure error taxonomy → metric label mapping is stable and exhaustive.
* **Golden tests** (host): sample metrics endpoint must include:

  * `oap_frames_total`, `oap_rejects_total`, `oap_frame_bytes`, `oap_inflight_frames`, `oap_ack_window`.
* **Lints:** forbid `await_holding_lock`; ensure readiness gating doesn’t use sleeps.
* **Docs freshness:** review this file **every 90 days** or when OAP/1 grammar or error reasons change.

---

## Appendix A — Minimal Metrics Facade (library API sketch)

```rust
/// Library-visible trait; host supplies a concrete Prom/OTel-backed handle.
pub trait OapMetricsExt: Clone + Send + Sync + 'static {
    fn inc_frame(&self, kind: &'static str);
    fn inc_reject(&self, reason: &'static str);
    fn add_frame_bytes(&self, n: u64);
    fn set_inflight(&self, v: i64);
    fn set_ack_window(&self, v: i64);
    fn time_decode<F, R>(&self, f: F) -> R where F: FnOnce() -> R;
    fn time_encode<F, R>(&self, f: F) -> R where F: FnOnce() -> R;
}

// Parser calls (illustrative):
metrics.time_decode(|| {
    // parse
});
metrics.inc_frame("DATA");
metrics.inc_reject("header_malformed");
```

---

## Appendix B — Readiness Adapter (host sketch)

```rust
pub struct OapReadiness {
    reject_ctr: prometheus::Counter,
    frame_ctr: prometheus::Counter,
    ack_window: prometheus::Gauge,
}
impl OapReadiness {
    pub fn ready(&self) -> bool {
        let frames = self.frame_ctr.get();
        let rejects = self.reject_ctr.get();
        let rate_ok = frames == 0.0 || (rejects / frames) < 0.005;
        let ack_ok = self.ack_window.get() > 0.0;
        rate_ok && ack_ok
    }
}
```

---

### TL;DR

* `oap` stays **silent** by itself and exposes **hooks**.
* Hosts surface **canonical OAP metrics**, wire **readiness** using those signals, and align **logs/traces** to the error/grammar canon.
* With this contract, dashboards and alerts remain consistent across every service that speaks OAP/1.
