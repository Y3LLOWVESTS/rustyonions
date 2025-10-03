
````markdown
# 🔗 INTEROP.md — svc-gateway

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-gateway`:

* Wire protocols & message formats (OAP/1 over HTTP/TLS, QUIC optional).
* DTOs & schemas shared across crates (ron-proto).
* Bus topics and events consumed/published.
* Canonical test vectors to freeze interop behavior.

This ensures all integrations (internal crates, SDKs, external clients) remain consistent with **GMI-1.6 Omni-Gate** and the RustyOnions canon.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

- **HTTP/1.1 + HTTP/2** over TLS 1.3 (`tokio_rustls::rustls::ServerConfig` only).
- **OAP/1 framed** (content-addressed object plane).
- **QUIC** optional (feature-flag, future interop).

### 1.2 Exposed Endpoints

- `GET /o/{addr}`  
  → Returns object by **content address** (`b3:<hex>`).  
  Requires no capability for public objects; private requires macaroon.

- `POST /put`  
  → Stores object; requires macaroon capability.  
  Body ≤ 1 MiB; decompression ratio ≤ 10×.

- Control plane:
  - `GET /healthz` — liveness probe.
  - `GET /readyz` — readiness gate.
  - `GET /metrics` — Prometheus metrics.
  - `GET /version` — build/version info.

### 1.3 Transport Invariants

- `max_frame = 1 MiB` (OAP/1 bound).
- Streaming chunk size = 64 KiB (for storage/overlay).
- TLS strictly `tokio_rustls::rustls::ServerConfig`.
- No custom framing; only canonical OAP/1 envelope.

---

## 2) DTOs / Schemas

### 2.1 ObjectManifestV2

```rust
struct ObjectManifestV2 {
  id: String,          // b3:<hex>
  size: u64,           // bytes
  chunks: Vec<Chunk>,  // 64 KiB
}
````

* **Encoding:** DAG-CBOR, strict schema, versioned.
* **Validation:** Digest recomputation required before serving.
* **Interop guarantee:** Forward-compatible (ignore unknown fields).

### 2.2 Canonical Envelope (OAP/1)

| Field       | Type | Description                       |
| ----------- | ---- | --------------------------------- |
| `len`       | u32  | Remaining length                  |
| `ver`       | u8   | Protocol version (1)              |
| `flags`     | u16  | `REQ`, `RESP`, `EVENT`, etc.      |
| `tenant_id` | u128 | ULID/UUID; 0 if unused            |
| `corr_id`   | u64  | Correlation ID for tracing        |
| `payload`   | []   | Application payload (may be COMP) |

---

## 3) Bus Topics

### 3.1 Events Published

* `svc-gateway.health` → `KernelEvent::Health { service, ok }`
* `svc-gateway.crash` → `KernelEvent::ServiceCrashed { service, reason }`
* `svc-gateway.degraded` → signals write-shed mode
* Service-specific forwardings:

  * `overlay.obj_put`
  * `overlay.obj_get`

### 3.2 Events Subscribed

* `config.updated` → update runtime config snapshot.
* `bus.shutdown` → trigger graceful shutdown.

---

## 4) Canonical Test Vectors

### 4.1 Frame Round-Trip

Input (hex):

```
01 00 00 10 …  
```

Output (JSON):

```json
{ "ver":1, "flags":"REQ", "tenant_id":"000…0", "corr_id":42, "payload":"…" }
```

### 4.2 Manifest Digest

Payload:

```
"hello world"
```

Digest:

```
b3:9f64a747e1… (BLAKE3)
```

### 4.3 Capability Example

```json
{
  "typ": "macaroon",
  "caveats": ["ttl=60s","method=GET","path=/o/"],
  "sig": "base64…"
}
```

---

## 5) Error Taxonomy

* `400 BadVersion` → unsupported OAP version.
* `401 Unauthorized` → missing/invalid macaroon.
* `413 FrameTooLarge` → body exceeds 1 MiB.
* `429 QuotaExceeded` → tenant over quota or DRR reject.
* `503 NotReady` → readiness gate failed (degraded state).

Error responses always JSON:

```json
{ "code":429, "reason":"quota", "retry_after":30, "corr_id":"ulid…" }
```

---

## 6) Interop Guarantees

* **No Kernel Drift:** Kernel surface frozen; gateway only bridges ingress.
* **SemVer Discipline:** Any breaking schema/protocol change requires a major version.
* **Backward Compatibility:** Unknown fields ignored, never dropped.
* **Append-only error taxonomy:** Reasons may be added but not repurposed.
* **Auditability:** Canonical vectors stored in `/tests/vectors/` and used by SDK CI.
* **Amnesia mode:** If enabled, no persistence of caps or keys across restarts.

---

## 7) References

* [Interop Blueprint GMI-1.6](../../docs/Interop_Blueprint.md)
* [OAP/1 Spec](../../docs/specs/OAP-1.md)
* [OBSERVABILITY.md](./OBSERVABILITY.md) — corr_id tracing IDs
* [CONCURRENCY.md](./CONCURRENCY.md) — shutdown & readiness

---

✅ With this doc, `svc-gateway` declares its **wire-level constitution**: protocols, DTOs, events, vectors, and guarantees. SDKs and auditors can rely on it as the single source of truth for ingress interop.

```

