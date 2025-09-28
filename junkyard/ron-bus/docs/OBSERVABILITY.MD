

**Path:** `crates/ron-bus/docs/OBSERVABILITY.md`

```markdown
# 📈 OBSERVABILITY.md — ron-bus

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for `ron-bus`:

* Metrics (Prometheus/OTEL via host registry)
* Health & readiness semantics (N/A here; host only)
* Logs (host only; this crate emits none)
* Tracing spans & correlation (optional host feature)
* Alerts & SLOs (built around bus metrics)

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (subset relevant to ron-bus)

* `bus_overflow_dropped_total` (Counter) — incremented when `RecvError::Lagged(n)` occurs.  
* `bus_queue_depth` (Gauge) — updated each subscriber loop tick to approximate backlog.  
* `bus_subscribers_total` (Gauge) — host-maintained, count of active receivers.  
* `service_restarts_total` — **N/A** (no background tasks in this crate).  
* `rejected_total{reason}` — **N/A** here; host may map `Error::Busy` or config rejects.  
* `request_latency_seconds` — **N/A** (no HTTP).  

### 1.2 Registration Discipline

* `ron-bus` never registers metrics globally; instead, it updates **host-provided handles**.  
* Hosts must register once in `Metrics::new()`, then pass clones into bus subscribers.  
* CI grep prevents duplicate registration at the host level.

---

## 2) Health & Readiness

### 2.1 Endpoints

- **N/A.** This crate does not bind `/healthz` or `/readyz`.  

### 2.2 Readiness Keys

- Hosts consuming `ron-bus` may include:  
  - `bus_attached=true` once a `Bus` is constructed.  
  - `bus_capacity` reported as metadata.  

### 2.3 Failure Semantics

- No `/readyz` here. Hosts may fail readiness if:
  - `bus_overflow_dropped_total` climbs continuously, or
  - `bus_queue_depth` remains > threshold.  

---

## 3) Logs

### 3.1 Format

- **N/A:** `ron-bus` does not emit logs.  
- Hosts log **event variant names** or context around publish/recv.  
- Required fields for host logs (per blueprint): `ts`, `level`, `service`, `event`, `reason`, `corr_id`.  

### 3.2 Redaction & Secrets

- Payload bodies **must not** be logged.  
- Hosts must redact secrets (caps, PII) at ingress before publish.

---

## 4) Tracing & Correlation

- Optional: if the `tracing` feature is enabled, `ron-bus` may emit spans:  
  - `svc.ron_bus.publish`  
  - `svc.ron_bus.recv`  
- Correlation IDs:  
  - Injected by hosts (HTTP `X-Corr-ID` or SDK).  
  - Propagated by attaching IDs to event structs.  

OpenTelemetry exporters are host-side, not in this crate.

---

## 5) Alerts & SLOs

### 5.1 Standard SLOs (bus-focused)

* `bus_overflow_dropped_total` should remain 0 in steady state; spikes tolerated during bursts but **not sustained**.  
* `bus_queue_depth` should return to baseline within N seconds after burst.  
* Subscriber loss (drop in `bus_subscribers_total`) should be matched by host recovery within 30s.  

### 5.2 Alerts (examples)

* `bus_overflow_dropped_total > 0 sustained for 10m` → **Warning**.  
* `bus_queue_depth > (capacity/2) sustained for 5m` → **Critical**.  
* `bus_subscribers_total < expected_min` → **Critical** (host-specific).  

### 5.3 Runbooks

Each alert links to `RUNBOOK.md`. Example triage:

- Overflow: Check subscriber health; increase capacity if all subscribers are healthy but lag persists.  
- Depth sustained: Investigate slow consumers.  

---

## 6) CI / Enforcement

* CI lints to ensure:  
  - `ron-bus` does **not** register metrics directly; only updates via handles.  
  - Code examples increment `bus_overflow_dropped_total` and set `bus_queue_depth` in Lagged handler.  
* Clippy wall: deny `await_holding_lock`.  
* Doc tests: examples must not log payloads.  
* Observability docs reviewed every 90 days.

---

✅ With this template, `ron-bus` provides **bounded, lossy, observable** semantics: overflow is counted, depth is visible, and subscribers are tracked. Operators get clear dashboards, and developers get safe copy-paste patterns.
```
