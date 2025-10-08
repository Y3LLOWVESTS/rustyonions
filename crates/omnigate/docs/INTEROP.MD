---

title: 🔗 INTEROP.md — omnigate
crate: omnigate (impl: svc-gateway)
owner: Stevan White
last-reviewed: 2025-10-07
status: draft
msrv: 1.80.0
------------

# INTEROP — omnigate

*Audience: developers, auditors, external SDK authors*

This file declares omnigate’s **wire-level contract** so SDKs and neighboring services remain consistent with the **GMI-1.6 Omni-Gate** canon.

> Role recap: omnigate is the **public edge**. It speaks **HTTP/1.1(+TLS)** to clients, applies **admission (quotas/DRR)** and **capability auth (macaroons via svc-passport)**, and **brokers** to internal services (index, storage, mailbox, overlay, dht). It is **stateless** aside from counters and short-lived verifier caches.

---

## 0) Purpose

Define the **interop surface** of `omnigate`:

* Wire protocols & message formats (HTTP/1.1+TLS at ingress; OAP/1 & internal REST behind the gate).
* DTOs & schemas (JSON, DAG-CBOR references).
* Bus topics and `KernelEvent`s.
* Canonical **test vectors** for SDK conformance.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

* **HTTP/1.1 + TLS 1.3** (recommended; dev may use plain HTTP)
* TLS implementation: **`tokio_rustls::rustls::ServerConfig`** only
* **Compression**: request bodies may be compressed; decompression is allowed with strict guards (see §5)

> QUIC/HTTP/3 not exposed by omnigate; if added, it will be versioned and feature-gated.

### 1.2 Public Endpoints (v1)

The public surface is versioned under `/v1`. Routes below are **stable** contract points; omnigate **proxies** to downstreams and enforces security/limits.

#### Objects (Storage proxy, read-only at edge)

* `GET /v1/o/{content_id}`

  * `content_id`: `b3:<hex>` BLAKE3 digest (content-addressing)
  * Request headers: `Range?`, `If-None-Match?`, `Accept?`, `Accept-Encoding?`
  * Response headers: `ETag`, `Content-Length`, `Content-Range?`, `Accept-Ranges: bytes`, `Cache-Control` (per policy)
  * Status: `200`, `206`, `304`, `404`, `416`

#### Index (Naming/metadata)

* `GET /v1/index/{name}?version=latest|<semver>`

  * Returns metadata resolving to `content_id` and auxiliary fields
  * Status: `200`, `404`

#### Mailbox (Signals)

* `GET /v1/mailbox/{channel}?wait_s=<n>` — long poll (bounded by downstream timeout)
* `POST /v1/mailbox/{channel}` — send; supports `Idempotency-Key` header

  * Status: `200`, `202`, `400`, `401/403`, `404`, `409`

#### Overlay & DHT (Discovery, read-only at edge)

* `GET /v1/dht/lookup?key=<hex>` → service records (subset of internal schema)

  * Status: `200`, `404`, `503 (degraded)`

#### Ops

* `GET /healthz` → `200` if process is alive
* `GET /readyz` → `200` if fully ready; otherwise `503` with JSON body
* `GET /metrics` → Prometheus text (binds private/localhost)

### 1.3 Transport Invariants

* **Ingress body cap**: `max_body_bytes = 1 MiB`
* **Decompression cap**: **ratio ≤ 10×** and **post-decompress ≤ 1 MiB**
* **Streaming chunk size** (downstream/storage detail): ~**64 KiB**
* **Auth header**: default `Authorization: Bearer <macaroon>`; header name configurable (`OMNIGATE_MACAROON_HEADER`)
* **Correlation**: `X-Corr-ID` accepted/echoed; `X-Request-ID` set per hop

---

## 2) DTOs / Schemas

Omnigate forwards/returns JSON DTOs. It does not author business DTOs; it **stabilizes** boundary shapes and passes through downstream fields that are explicitly documented here.

> **Encoding rules**
>
> * JSON objects use **snake_case** field names.
> * Unknown fields **may** appear; consumers must ignore unknowns (for forward compatibility).
> * Binary content is returned as bytes (not base64) via `/v1/o/...`.

### 2.1 Index Resolve (response)

```json
{
  "name": "app-config",
  "version": "1.2.3",
  "content_id": "b3:1b3f3e...c9",
  "size": 2048,
  "etag": "\"b3:1b3f3e...c9\"",
  "metadata": {
    "created_at": "2025-09-05T12:34:56Z"
  }
}
```

* **Compatibility**: `metadata` is an open map; clients must ignore unknown keys.

### 2.2 Mailbox Message (request/response)

```json
{
  "channel": "alerts",
  "msg": { "kind": "deploy_started", "env": "prod" },
  "idempotency_key": "01J9Z7K9WZJ6Y8...",
  "ttl_s": 60
}
```

* Response (send): `{"status":"accepted","message_id":"01J9Z7..."}`
* Response (receive): `{"messages":[{"message_id":"01J9Z7...","msg":{...},"received_at":"..."}]}`

### 2.3 DHT Record (response, trimmed)

```json
{
  "key": "9a0c...f2",
  "records": [
    {
      "service": "storage",
      "addr": "http://10.0.0.12:9320",
      "ttl_s": 300
    }
  ]
}
```

### 2.4 ObjectManifestV2 (reference for storage; not emitted by gateway)

```rust
// Reference only — emitted by storage; gateway may pass through fields unchanged.
struct ObjectManifestV2 {
  id: String,          // b3:<hex>
  size: u64,           // bytes
  chunks: Vec<Chunk>,  // ~64 KiB each
}
```

* Encoding: **DAG-CBOR** (strict), if requested via explicit Accept header and feature is enabled end-to-end.

---

## 3) Bus Topics

> Omnigate does not expose a public bus. Internally, it emits/consumes kernel events via the runtime bus.

### 3.1 Published (examples)

* `omnigate.health` → `KernelEvent::Health { service: "omnigate", ok: bool }`
* `omnigate.config.updated` → `KernelEvent::ConfigUpdated { version: u64, ok: bool }`
* `omnigate.service.crashed` → `KernelEvent::ServiceCrashed { service, reason }`

### 3.2 Subscribed

* `bus.shutdown` → triggers graceful drain
* `config.updated` → swaps `ArcSwap<Config>` snapshot

> Bus payloads are JSON with snake_case keys; correlation fields (`corr_id`) included when relevant.

---

## 4) Canonical Test Vectors

### 4.1 BLAKE3 Content ID

* **Payload**: ASCII `"hello world\n"`
* **Digest (hex)**: `b94d27b9934d3e08a52e52d7da7dabfade4f`… (example; compute precise for tests)
* **Content ID**: `b3:<hex>`
* **HTTP**

  ```bash
  curl -H "Authorization: Bearer <cap>" \
       -H "X-Corr-ID: 01J9Z7..." \
       https://gw.example.com/v1/o/b3:<hex>
  ```

### 4.2 Range Read

* **Request**: `Range: bytes=0-65535`
* **Expected**: `206 Partial Content`, `Content-Range: bytes 0-65535/<total>`, `Accept-Ranges: bytes`

### 4.3 Compressed Body Guard

* **Request**: `Content-Encoding: gzip`, body expands to > **10×** original or > **1 MiB** absolute
* **Expected**: `400` with `{"error":"decompress_cap","corr_id":"..."}` and metric `decompress_reject_total{reason="ratio_cap"}`

### 4.4 Capability (macaroon) Example

```json
{
  "typ": "macaroon",
  "issuer": "svc-passport",
  "caveats": ["ttl=60s", "method=GET", "path=/v1/o/", "aud=omnigate"],
  "sig": "base64url..."
}
```

* Missing/invalid → `401 unauth`
* Valid but insufficient caveats → `403 forbidden`

### 4.5 Readiness Degraded

* Downstream health fails, strict boot **off**:

  * `/readyz` → `503`, body: `{"ready":false,"degraded":true,"missing":["downstream_health_ok"],"retry_after":5}`

> Store concrete vectors under `tests/vectors/omnigate/**` (JSON bodies, headers, status codes).

---

## 5) Error Taxonomy (Wire Mapping)

| HTTP | `error` code             | Trigger                                               | Notes / Metrics                                          |                |
| ---- | ------------------------ | ----------------------------------------------------- | -------------------------------------------------------- | -------------- |
| 400  | `bad_request`            | Malformed headers/params; unsupported encoding        | `rejected_total{reason="bad_request"}`                   |                |
| 401  | `unauth`                 | Missing/invalid macaroon                              | `rejected_total{reason="unauth"}`                        |                |
| 403  | `forbidden`              | Capability present but lacks caveats/scope            | `rejected_total{reason="forbidden"}`                     |                |
| 404  | `not_found`              | Object/index/mailbox missing                          |                                                          |                |
| 413  | `body_cap`               | Body > **1 MiB**                                      | `rejected_total{reason="body_cap"}`                      |                |
| 415  | `unsupported_media_type` | Codec not allowed                                     |                                                          |                |
| 416  | `range_not_satisfiable`  | Invalid `Range`                                       |                                                          |                |
| 429  | `quota` or `busy`        | RPS/exhaustion or DRR queue full                      | `rejected_total{reason="quota                            | busy"}`        |
| 500  | `internal`               | Unexpected error                                      |                                                          |                |
| 502  | `bad_gateway`            | Downstream non-success or protocol mismatch           | `downstream_errors_total{kind="5xx                       | bad_gateway"}` |
| 503  | `degraded`               | Readiness failing; reload or downstream health issues | `readyz_degraded=1`, `rejected_total{reason="degraded"}` |                |
| 504  | `downstream_timeout`     | Downstream exceeded deadline                          | `downstream_errors_total{kind="timeout"}`                |                |

**Determinism:** identical inputs under the same load yield identical status + `error` code and increment the same metric labels.

---

## 6) Interop Guarantees

* **Path versioning**: `/v1` is stable. Breaking changes land under `/v2`.
* **Forward compatibility**: clients **must ignore unknown JSON fields**; the gateway never silently drops known required fields.
* **Security limits are part of contract**:

  * Body cap **1 MiB**, decompression ratio cap **10×** (and absolute cap), enforced **before** heavy work.
  * Admission occurs prior to downstream calls (quotas/DRR).
* **Auth**: capability macaroons with explicit caveats (method/path/TTL/audience). Header name can be configured, but a deprecation alias window is provided for renames.
* **TLS**: rustls only; ciphers follow workspace policy; no plaintext auth secrets.
* **No kernel drift**: event names and readiness semantics align with runtime blueprints.

---

## 7) Conformance & CI

* **SDK Conformance Suite** (`tests/interop_*`):

  * Range reads, decompression caps, quota/DRR rejections, capability caveat enforcement.
  * Deterministic error mapping & metrics emission.
* **Golden Vectors**: `tests/vectors/omnigate/*.json` must remain stable across minor releases.
* **CI Gates**:

  * OpenAPI (if present) diff must be acknowledged.
  * `cargo test -p omnigate --features interop` runs vectors.
  * `cargo deny` / SBOM checks ensure supply chain stability.

---

## 8) References

* `OBSERVABILITY.md` — metrics names & correlation fields
* `CONFIG.md` — limits, timeouts, TLS, auth header name
* `SECURITY.md` — threat model, redaction rules
* Interop Blueprint GMI-1.6 — canonical invariants
* OAP/1 Spec — for downstream protocol reference (gateway does not expose OAP externally)

---

## 9) Appendix — Example Sessions

### 9.1 Object GET (range)

```bash
curl -sS https://gw.example.com/v1/o/b3:1b3f...c9 \
  -H "Authorization: Bearer <cap>" \
  -H "X-Corr-ID: 01J9Z7K9..." \
  -H "Range: bytes=0-65535" \
  -D- -o /dev/null
```

**Expected**: `206`, `Content-Range`, `X-Corr-ID`, `X-Request-ID`, metrics incremented:
`http_requests_total{route="/v1/o/:id",method="GET",status="206"}` and histogram bucket for latency.

### 9.2 Mailbox Send (idempotent)

```bash
curl -sS -X POST https://gw.example.com/v1/mailbox/alerts \
  -H "Authorization: Bearer <cap>" \
  -H "Idempotency-Key: 01J9Z7..." \
  -H "Content-Type: application/json" \
  --data '{"msg":{"text":"deploy-started"}, "ttl_s": 60}'
```

**Expected**: `202` with message id; repeated with same idempotency key → same outcome, no duplicate.

### 9.3 Capability Failure

```bash
curl -sS https://gw.example.com/v1/o/b3:deadbeef \
  -H "Authorization: Bearer <invalid>" -i
```

**Expected**: `401`, JSON `{"error":"unauth",...}`, metric `rejected_total{reason="unauth"}` increments, structured log with `event="auth_failed"`.

---

**Outcome:** With this INTEROP spec, omnigate’s behavior at the wire is **predictable**, **secure**, and **testable**. SDKs and services can evolve independently without breaking the edge contract.
