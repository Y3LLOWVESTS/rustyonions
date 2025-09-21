

# 🔗 INTEROP.md (Template)

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `<crate>`:

* Wire protocols & message formats (OAP/1, gRPC, HTTP).
* DTOs & schemas.
* Bus topics and events.
* Canonical test vectors.
  This ensures all inter-crate and external integrations remain consistent with **GMI-1.6 Omni-Gate**.

---

## 1) Protocols & Endpoints

* **Ingress Protocols:** (e.g., HTTP/1.1 + TLS, OAP/1 framed, QUIC).
* **Exposed Endpoints:**

  * `GET /o/{addr}` → returns object (content-addressed).
  * `POST /put` → store object; requires capability.
* **Transport Invariants:**

  * `max_frame = 1 MiB` (OAP/1).
  * Streaming chunk size = 64 KiB (storage detail).
  * TLS = `tokio_rustls::rustls::ServerConfig` only.

---

## 2) DTOs / Schemas

### 2.1 Example: ObjectManifestV2

```rust
struct ObjectManifestV2 {
  id: String,          // b3:<hex>
  size: u64,           // bytes
  chunks: Vec<Chunk>,  // 64 KiB
}
```

* **Encoding:** DAG-CBOR (strict schema, versioned).
* **Validation:** Full digest check required before serving.

### 2.2 Canonical Envelope (OAP/1)

| Field       | Type | Description                      |
| ----------- | ---- | -------------------------------- |
| `len`       | u32  | Remaining length                 |
| `ver`       | u8   | Protocol version (1)             |
| `flags`     | u16  | `REQ, RESP, EVENT, …`            |
| `tenant_id` | u128 | ULID/UUID; 0 if unused           |
| `corr_id`   | u64  | Correlation ID (for tracing)     |
| `payload`   | \[]  | Application-opaque (may be COMP) |

---

## 3) Bus Topics

### 3.1 Events Published

* `<crate>.health` → `KernelEvent::Health { service, ok }`.
* `<crate>.crash` → `KernelEvent::ServiceCrashed { service, reason }`.
* Service-specific events (example):

  * `overlay.obj_put`
  * `overlay.obj_get`

### 3.2 Events Subscribed

* `config.updated` → update config snapshot.
* `bus.shutdown` → trigger graceful shutdown.

---

## 4) Canonical Test Vectors

* **Frame Round-Trip:**

  * Input: hex `"0100..."` → Output: JSON `{ "ver":1, "flags":REQ, … }`.
* **Manifest Digest:**

  * Payload `b"hello world"` → Digest `b3:9f64a...`.
* **Capability Example:**

  ```json
  {
    "typ":"macaroon",
    "caveats":["ttl=60s","method=GET","path=/o/"],
    "sig":"base64…"
  }
  ```

---

## 5) Error Taxonomy

* `400 BadVersion` → unsupported OAP version.
* `413 FrameTooLarge` → exceeds 1 MiB.
* `429 QuotaExceeded` → tenant over quota.
* `503 NotReady` → readiness gate failed.

---

## 6) Interop Guarantees

* **No Kernel Drift:** Kernel API surface frozen.
* **SemVer Discipline:** Breaking schema change → major version.
* **Backward Compatibility:** Unknown fields ignored on read; never silently dropped.
* **Auditability:** Canonical vectors stored in `/tests/vectors/`.

---

## 7) References

* [Interop Blueprint GMI-1.6](../../docs/Interop_Blueprint.md)
* [OAP/1 Spec](../../docs/specs/OAP-1.md)
* [OBSERVABILITY.md](./OBSERVABILITY.md) (for tracing IDs)
* [CONCURRENCY.md](./CONCURRENCY.md) (for readiness semantics)

---

✅ With this template, every crate declares its **wire-level contract**—ensuring bus events, protocol frames, and SDKs never drift.

---
