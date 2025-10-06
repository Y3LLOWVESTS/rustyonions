

---

# 🔗 INTEROP.md — ron-metrics

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-metrics`:

* Wire protocols & message formats:

  * **Prometheus text exposition (v0)** over HTTP.
  * **Health/readiness JSON** over HTTP.
  * Optional **OTLP/OTel export (egress)** when the `otel` feature is enabled.
* DTOs & schemas (readiness/health JSON; stable label taxonomy for metrics).
* Canonical test vectors (metrics lines, readiness payloads).
* Transport & TLS invariants (loopback/UDS by default, rustls if TLS).

This keeps every embedding service consistent with **GMI-1.6 Omni-Gate** and the **12 Pillars (P5: Observability)** while remaining a minimal, auditable surface.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols (Exposer)

* **HTTP/1.1** (required); **HTTP/2** permitted if the host HTTP stack enables it.
* **Bind targets**:

  * **Loopback TCP** (default) e.g., `127.0.0.1:9100`.
  * **UDS** (Unix Domain Socket) for local scrapes (preferred in hardened mode).
  * **TCP + TLS** via `tokio_rustls::rustls::ServerConfig` (when fronted by a gateway or for cross-namespace scrape).
* **Methods**: **GET-only**. All non-GET → `405 Method Not Allowed`. Request bodies are ignored/rejected.

### 1.2 Exposed Endpoints (Library-hosted by `Metrics::serve`)

* `GET /metrics`

  * **Content-Type**: `text/plain; version=0.0.4` (Prometheus exposition v0).
  * **Stability**: Metric **names** and **label keys** are SemVer-sensitive contracts (see §6).
  * **Required base labels** on first-party families: `service`, `instance`, `build_version`, `amnesia`.

* `GET /healthz`

  * **Liveness** probe. Returns `200` with a small body (implementation-defined; may be `"ok"`).

* `GET /readyz`

  * **Readiness** probe. Returns `200` only when minimal dependencies are ready; else `503 Service Unavailable`.
  * **Content-Type**: `application/json`.
  * **Response schema**: see §2.1.
  * Includes `Retry-After: <seconds>` when not ready.

### 1.3 Transport Invariants

* **TLS**: if enabled, must use `tokio_rustls::rustls::ServerConfig`. No other TLS types are exposed in the API.
* **Keep-alive**: enabled; idle/read/write timeouts are applied by the host service (see CONFIG).
* **Response sizes**: `/metrics` aims to remain small (≪ 4 MiB). Histograms use bounded bucket sets.
* **Hygiene**: No request bodies; no compression negotiation by default (gateway may compress). No cookies.

### 1.4 Egress Protocols (Optional OTLP)

If `feature = "otel"` is enabled, this crate can **export** metrics to an **OTLP/OTel collector**. This is **egress**, not an ingress surface, and the public Rust API does not change.

* **Preferred**: **OTLP over gRPC** to the collector endpoint configured by the host.
* **Fallback**: **OTLP over HTTP/JSON** if your collector supports it.
* **Mapping**: 1:1 from Prometheus families/labels to OTLP **metric + attributes** (see §2.4).

---

## 2) DTOs / Schemas

### 2.1 ReadinessResponse (JSON over `/readyz`)

```rust
/// JSON returned by GET /readyz when not ready (503) or ready (200).
#[derive(serde::Serialize)]
struct ReadinessResponse<'a> {
  degraded: bool,        // true if any dependency is missing/degraded
  missing: Vec<&'a str>, // e.g., "config_loaded", "port_bound", "bus_attached", "uds_ready"
  retry_after: u32,      // seconds; present when degraded (non-zero)
  since: Option<String>, // RFC3339 timestamp when the current state began
}
```

**Example (not ready):**

```json
{
  "degraded": true,
  "missing": ["config_loaded","port_bound"],
  "retry_after": 5,
  "since": "2025-10-05T18:09:00Z"
}
```

**Example (ready):**

```json
{
  "degraded": false,
  "missing": [],
  "retry_after": 0,
  "since": "2025-10-05T18:11:12Z"
}
```

### 2.2 HealthResponse (JSON or plain text over `/healthz`)

Minimal by design; either plain text `"ok"` or:

```json
{ "ok": true }
```

### 2.3 Metrics Exposition (Prometheus text, v0)

* Lines follow Prometheus text exposition **v0.0.4**.
* **Required base labels** (present on all first-party families):
  `service`, `instance`, `build_version`, `amnesia`.
* **Suffix discipline** (names end with): `*_seconds`, `*_bytes`, `*_total`.
* **Core families (examples)**:

  * `request_latency_seconds` (Histogram)
  * `bus_lagged_total` (Counter)
  * `service_restarts_total` (Counter)
  * `zk_proof_latency_seconds` (Histogram)
  * `pq_kex_failures_total`, `pq_verify_failures_total`, `zk_verify_failures_total` (Counters; may be zero)

**Example lines:**

```
# TYPE request_latency_seconds histogram
request_latency_seconds_bucket{service="svc-index",instance="idx-1",build_version="v0.1.0",amnesia="off",le="0.010"} 42
request_latency_seconds_bucket{service="svc-index",instance="idx-1",build_version="v0.1.0",amnesia="off",le="+Inf"} 58
request_latency_seconds_sum{service="svc-index",instance="idx-1",build_version="v0.1.0",amnesia="off"} 1.234
request_latency_seconds_count{service="svc-index",instance="idx-1",build_version="v0.1.0",amnesia="off"} 58

# TYPE bus_lagged_total counter
bus_lagged_total{service="svc-overlay",instance="ovl-2",build_version="v0.1.0",amnesia="on"} 0

# TYPE pq_verify_failures_total counter
pq_verify_failures_total{service="svc-gateway",instance="gw-1",build_version="v0.1.0",amnesia="off",algo="kyber1024",role="client"} 0
```

### 2.4 OTLP Mapping (when `otel` feature is enabled — egress)

* **Resource attributes** (typical):

  * `service.name = <service>` (maps from `service` label)
  * `service.instance.id = <instance>`
  * `service.version = <build_version>`
  * `ron.amnesia = "on"|"off"`
* **Metric attributes**: carry Prometheus label sets (e.g., `algo`, `role`, `le` for buckets).
* **Histogram mapping**: Prometheus `_bucket/_sum/_count` → OTLP Histogram with bounds; counters map to Sum with `monotonic=true`.

---

## 3) Bus Topics

`ron-metrics` is a **pure library**; it does **not** publish or subscribe to the kernel bus.

> For integrators: If your service emits bus events to reflect health/readiness, use your service’s namespace (e.g., `svc-index.health`, `svc-index.ready`) and keep them decoupled from `ron-metrics`. This crate only **observes/exports** via HTTP/OTLP.

---

## 4) Canonical Test Vectors

### 4.1 `/readyz` JSON (503 example)

**Request**

```
GET /readyz HTTP/1.1
Host: 127.0.0.1:9100
```

**Response**

```
HTTP/1.1 503 Service Unavailable
Content-Type: application/json
Retry-After: 5

{"degraded":true,"missing":["config_loaded","port_bound"],"retry_after":5,"since":"2025-10-05T18:09:00Z"}
```

### 4.2 `/metrics` Prometheus lines

```
# HELP request_latency_seconds Request latency (seconds)
# TYPE request_latency_seconds histogram
request_latency_seconds_bucket{service="svc-overlay",instance="ovl-1",build_version="v0.1.0",amnesia="off",le="0.1"} 10
request_latency_seconds_bucket{service="svc-overlay",instance="ovl-1",build_version="v0.1.0",amnesia="off",le="+Inf"} 12
request_latency_seconds_sum{service="svc-overlay",instance="ovl-1",build_version="v0.1.0",amnesia="off"} 0.845
request_latency_seconds_count{service="svc-overlay",instance="ovl-1",build_version="v0.1.0",amnesia="off"} 12
```

### 4.3 OTLP resource (illustrative)

```
resource:
  service.name: "svc-gateway"
  service.instance.id: "gw-1"
  service.version: "v0.1.0"
  ron.amnesia: "on"
```

> Store these exact vectors under `/tests/vectors/interop/ron-metrics/` to stabilize SDK tests.

---

## 5) Error Taxonomy (HTTP)

* `405 Method Not Allowed` — anything not GET on `/metrics`, `/healthz`, `/readyz`.
* `415 Unsupported Media Type` — if a client attempts to POST or sends a body.
* `503 Service Unavailable` — not ready (`/readyz`).
* TLS handshake failures are transport-level errors (not HTTP-mapped).
* **Non-goals**: rate-limiting, auth, or quotas are out of scope; front with the gateway if required.

---

## 6) Interop Guarantees

* **Prometheus contract SemVer**:

  * **Stable**: metric **names**, **units/suffixes**, and **existing label keys**.
  * **Additive**: new families; new **low-cardinality** labels (documented).
  * **Breaking**: renaming/removing families; removing labels; changing units.
* **Suffix discipline**: `*_seconds`, `*_bytes`, `*_total` only.
* **Base labels**: `service`, `instance`, `build_version`, `amnesia` always present on first-party families.
* **PQ/ZK families**: exist even when features are disabled (values may be zero).
* **Exposer policy**: GET-only; loopback/UDS by default; TLS via rustls type when enabled.
* **Readiness**: degrades before collapse (fail-open reads / fail-closed writes) with `Retry-After`.

---

## 7) References

* **Interop Blueprint (GMI-1.6)** — alignment for node surfaces and gateway expectations.
* **OBSERVABILITY.md** — golden metrics, buckets, SLOs, readiness semantics.
* **CONFIG.md** — bind targets (TCP/UDS), timeouts, optional TLS config, OTLP endpoint (if `otel`).
* **SECURITY.md** — transport hardening, loopback first, TLS invariants, amnesia mode.
* **CONCURRENCY.md** — no locks across `.await`, single registration, bounded tasks.

---

## 8) Notes for External SDK Authors

* The **only** ingress interop surface is **HTTP GET** for `/metrics`, `/healthz`, `/readyz`.
* Prefer scraping via loopback/UDS; if remote scrape is required, terminate TLS at the gateway and enforce auth there.
* If consuming via **OTLP**, connect to your collector (this crate does not implement a collector).
* **Polyglot guidance**: Non-Rust SDKs (TS/Python/Swift) should implement:

  * a Prom v0 parser and/or JSON readiness schema (§2.1),
  * tests against the canonical vectors in `/tests/vectors/interop/ron-metrics/`.

---

## 9) Interop DoD (Definition of Done)

* [ ] `/metrics` exposes families with required base labels and correct suffixes.
* [ ] `/readyz` returns 200/503 with schema in §2.1 and `Retry-After` on 503.
* [ ] `/healthz` returns 200 for liveness.
* [ ] PQ/ZK families present (zero allowed).
* [ ] If `otel` is enabled, OTLP attributes map as in §2.4 (resource + metric attributes).
* [ ] CI taxonomy checks pass (suffix discipline + base labels present).
* [ ] Interop vectors stored under `/tests/vectors/interop/ron-metrics/` and used by SDK tests.

---

### Appendix — Mermaid (ingress flow)

```mermaid
flowchart LR
  S[Scraper (Prometheus)] -->|HTTP GET| M[/metrics endpoint/]
  M -->|text v0| P[(Prometheus TSDB)]
  H[GET /healthz] --> L[Liveness 200]
  R[GET /readyz] -->|200/503 + JSON| Ops[Readiness Gate]
```

---
