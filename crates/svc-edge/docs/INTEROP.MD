

---

# 🔗 INTEROP.md — svc-edge

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-edge`:

* Wire protocols & message formats (**HTTP/1.1 over TLS** for ingress; no custom framing on the public plane).
* DTOs & schemas (HTTP headers, readiness JSON).
* Bus topics and events (kernel/control).
* Canonical test vectors (requests/responses, headers, error shapes, range semantics).

This ensures all inter-crate and external integrations remain consistent with **GMI-1.6 Omni-Gate** principles and RustyOnions interop guidelines.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols (public)

* **HTTP/1.1** (required)
* **TLS 1.3** (recommended at gateway/ingress; optionally local via `tokio-rustls`)
* CORS: **deny-by-default** (configurable allow-list)

### 1.2 Exposed Endpoints (service API)

| Method | Path                   | Purpose                                                  | Notes                                                     |
| -----: | ---------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
|    GET | `/edge/assets/{*path}` | Serve immutable bytes from **pack/CAS** or **live fill** | Range/ETag/304 supported; multi-range disabled by default |
|    GET | `/healthz`             | Liveness                                                 | Always `200` if process alive                             |
|    GET | `/readyz`              | Readiness                                                | `200` only when all gates pass; JSON on degrade           |
|    GET | `/metrics`             | Prometheus exposition                                    | Bind to loopback or protect at gateway                    |

### 1.3 Transport Invariants

* **Max request body**: `1 MiB` (configurable), applies to PUT/POST (if ever added) and to compressed bodies before/after decompress caps.
* **Decompression caps**: ratio ≤ `10×`, absolute ≤ `10 MiB` (configurable).
* **Timeouts**: per-request ≤ `5s` (configurable).
* **Range**: strict single-range; `416` on unsatisfiable.
* **ETag**: **strong**, content-addressed (BLAKE3 of bytes). Format: `"\"b3:<hex>\""`.

> `svc-edge` intentionally **does not** expose OAP/1, QUIC, or gRPC on the public edge. Those live behind Omni-Gate or other internal planes.

---

## 2) DTOs / Schemas

### 2.1 HTTP Semantics (canonical)

#### Request headers (ingress)

* `Range: bytes=<start>-<end>` (optional)
* `If-None-Match: "<etag>"` (optional)
* `Accept-Encoding: gzip, br` (clients may send; server obeys caps)
* `X-Corr-ID: <uuid|ulid>` (optional; generated if absent)

#### Response headers

* `ETag: "<strong-etag>"` (always on 200/206)
* `Accept-Ranges: bytes` (on 200/206)
* `Content-Range: bytes <start>-<end>/<total>` (on 206)
* `Cache-Control: public, immutable` (recommended for immutable assets)
* `X-Corr-ID: <uuid|ulid>` (always echoed)

### 2.2 Readiness JSON (degraded example)

```json
{
  "ready": false,
  "degraded": true,
  "missing": ["packs_verified", "cas_reachable", "shedder_stable"],
  "retry_after": 1
}
```

Fields are stable; additional keys may be added additively.

### 2.3 Content Address (CAS key / ETag)

* **Algorithm**: BLAKE3 over exact response bytes (after range slicing for 206, the ETag remains the full-object hash; this is deliberate to keep validators strong).
* **Encoding**: lowercase hex, prefixed by `b3:`; embedded as a strong ETag string: `"\"b3:<hex>\""`.

> Client **MUST NOT** assume ETag equals the range slice hash; it identifies the full object.

---

## 3) Bus Topics (internal control plane)

`svc-edge` is primarily edge-HTTP; it **observes** and **emits** a few kernel events on the internal bus.

### 3.1 Events Published

* `kernel.health` → `KernelEvent::Health { service: "svc-edge", ok: bool }`
* `kernel.ready_changed` → `KernelEvent::ReadyChanged { service, ready: bool, reasons: Vec<String> }`
* `svc_edge.audit` (optional) → `{ kind: "fill"|"reject", host?, path, etag?, hash?, reason?, bytes?, corr_id }`
* `kernel.crash` → `KernelEvent::ServiceCrashed { service: "svc-edge", reason }` (on fatal errors only)

### 3.2 Events Subscribed

* `kernel.config_updated` → `{ version: u64 }` → triggers snapshot recompute/reload
* `kernel.shutdown` → graceful drain & stop intake

> Event payloads follow our **Kernel DTO** convention (serde JSON; `#[serde(deny_unknown_fields)]` for readers; writers may include optional fields).

---

## 4) Canonical Test Vectors

> These vectors lock the HTTP contract and range/etag behavior. Store them under `tests/vectors/interop/` and use in integration tests.

### 4.1 Warm Hit (200 OK)

**Request**

```
GET /edge/assets/fonts/roboto.woff2 HTTP/1.1
Host: localhost
X-Corr-ID: 01HXYZ123ABCDEF
```

**Response (excerpt)**

```
HTTP/1.1 200 OK
ETag: "b3:<hex>"
Accept-Ranges: bytes
Content-Length: 18724
X-Corr-ID: 01HXYZ123ABCDEF
```

**Assertions**

* Status `200`
* `ETag` present, starts with `"b3:"`
* `rejected_total` **not** incremented

---

### 4.2 Conditional (304 Not Modified)

**Request**

```
GET /edge/assets/tile.pmtiles HTTP/1.1
Host: localhost
If-None-Match: "b3:<same-hex>"
```

**Response**

```
HTTP/1.1 304 Not Modified
ETag: "b3:<same-hex>"
```

**Assertions**

* Status `304`
* Body empty
* Latency bucketed under `/edge/assets`

---

### 4.3 Range (206 Partial Content)

**Request**

```
GET /edge/assets/tile.pmtiles HTTP/1.1
Range: bytes=0-65535
```

**Response (excerpt)**

```
HTTP/1.1 206 Partial Content
Content-Range: bytes 0-65535/1234567
ETag: "b3:<full-object-hex>"
```

**Assertions**

* Status `206`
* `Content-Range` inclusive bounds
* `ETag` corresponds to **full** object, not slice

---

### 4.4 Unsatisfiable Range (416)

**Request**

```
GET /edge/assets/tile.pmtiles
Range: bytes=999999999-1000000000
```

**Response**

```
HTTP/1.1 416 Range Not Satisfiable
```

**Assertions**

* Status `416`
* `rejected_total{reason="invalid_range"}` incremented

---

### 4.5 Live Fill Allowed (200 via cache-miss + fill)

**Request**

```
GET /edge/assets/fonts/remote.woff2
```

**Preconditions**

* `mode=live`
* `allow=["fonts.gstatic.com"]`
* upstream returns `200` with body + `ETag` or content

**Assertions**

* First request: `202 Accepted` **or** `200` depending on implementation timing; subsequent request: `200`
* `edge_live_fill_success_total{host="fonts.gstatic.com"}` increments
* CAS write recorded (`edge_cas_writes_total{status="ok"}`)

---

### 4.6 Live Fill Blocked (403)

**Request**

```
GET /edge/assets/fonts/bad.woff2
```

**Preconditions**

* `mode=live`, `allow=["fonts.gstatic.com"]` only
* Path requires egress to `evil.example.com`

**Assertions**

* Status `403`
* `rejected_total{reason="not_allowed"}` incremented

---

### 4.7 Shed/Busy (429)

**Scenario**

* Fill queue `mpsc(512)` is full or RPS limiter tripped

**Assertions**

* Status `429`
* `busy_rejections_total{endpoint="/edge/assets"}` increments
* `X-Reason: rate_limit` header included

---

### 4.8 Decompress Guard (400/413)

**Scenario**

* Client sends compressed body that exceeds ratio or absolute cap

**Assertions**

* `400` with `reason="decompress_cap"` **or** `413` with `reason="body_cap"`
* Corresponding `rejected_total` label increments

---

## 5) Error Taxonomy (wire-visible)

| HTTP | Reason (header/metric) | When                                  | Notes                                |                                                     |                        |
| ---: | ---------------------- | ------------------------------------- | ------------------------------------ | --------------------------------------------------- | ---------------------- |
|  200 | —                      | Cache hit/full response               | `ETag` present                       |                                                     |                        |
|  206 | —                      | Valid single range                    | `Content-Range` + full-object `ETag` |                                                     |                        |
|  304 | —                      | `If-None-Match` match                 | Body empty                           |                                                     |                        |
|  403 | `not_allowed`          | Upstream host not on allow-list       | Live mode only                       |                                                     |                        |
|  416 | `invalid_range`        | Unsatisfiable byte range              |                                      |                                                     |                        |
|  429 | `rate_limit`           | RPS or queue shed                     | Deterministic backpressure           |                                                     |                        |
|  503 | `timeout               | draining                              | amnesia`                             | Upstream timeout / draining / amnesia write blocked | `Retry-After` optional |
|  400 | `decompress_cap`       | Decompression ratio/absolute exceeded |                                      |                                                     |                        |
|  413 | `body_cap`             | Body too large                        |                                      |                                                     |                        |

> Every reject/error emits `rejected_total{reason=...}` and a structured log with the same reason.

---

## 6) Interop Guarantees

* **HTTP contract is stable** and versioned by crate SemVer. Breaking changes require a **major** bump and CHANGELOG entry.
* **Strong ETag** and CAS hashing are normative; algorithm changes require compatibility period or major bump.
* **Unknown JSON fields** in readiness responses may be added; clients **must ignore** unknown keys.
* **No Kernel Drift**: Bus event names and shapes are consistent with kernel expectations.
* **Backwards-compatible additions** (new headers, metrics, JSON fields) are allowed; removals/renames are breaking.

---

## 7) References

* `docs/IDB.md` — Invariants for caching, caps, and failure posture
* `docs/CONFIG.md` — All knobs (caps, allow-list, retry/jitter)
* `docs/CONCURRENCY.md` — Bounded queues & shutdown semantics
* `docs/SECURITY.md` — Threat model & hardening gates
* `docs/OBSERVABILITY.md` — Metrics/logs/tracing contracts
* Interop Blueprint GMI-1.6 (Omni-Gate) — repo root
* (Optional) `/docs/openapi/svc-edge.yaml` — concise OpenAPI for HTTP surface

---

### Notes for SDK Authors

* Treat ETag as **strong**. Use `If-None-Match` for efficient revalidation.
* Don’t assume multi-range; issue multiple single ranges if needed.
* Propagate `X-Corr-ID` for traceability.
* Respect `429` and `Retry-After` (if present); apply client-side jitter before retrying.

---

✅ With this interop spec, **svc-edge** declares its **wire-level contract** end-to-end: HTTP semantics, headers, readiness JSON, bus events, and canonical vectors. This prevents drift across crates and makes external SDKs straightforward and predictable.
