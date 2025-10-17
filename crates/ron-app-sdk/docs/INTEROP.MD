

````markdown
# 🔗 INTEROP.md — ron-app-sdk

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-app-sdk`:

- Wire protocols & message formats (OAP/1 over TLS or Tor).
- DTOs & schemas (from `ron-proto`), plus strictness rules.
- Canonical gateways/endpoints and plane semantics (Mailbox, Edge, Storage, Index).
- Canonical test vectors (frames, capability examples, byte-range semantics).
- Error taxonomy alignment and guarantees, consistent with **GMI-1.6 Omni-Gate**.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols (client → gateway/node)

- **OAP/1 framed** over:
  - **TLS 1.3** (`transport=tls`) via `tokio-rustls` (client mode).
  - **Tor** (`transport=tor`) via local SOCKS (arti/tor); gateway address is a `.onion` host.
- **HTTP/1.1** semantics are exposed by the gateway for “Edge” style paths (byte-range GET). The SDK still wraps these as typed calls.

> The SDK is **client-only**: it opens no server sockets and does not listen on any port.

### 1.2 Exposed Endpoints (Gateway facets)

| Plane    | Endpoint (canonical)           | Verb | Summary                                                                 |
|----------|--------------------------------|------|-------------------------------------------------------------------------|
| Storage  | `GET /o/{addr_b3}`             | GET  | Fetch object by BLAKE3 address (`b3:<hex>`). Supports range via Edge.   |
| Storage  | `POST /put`                    | POST | Store object (body); returns `AddrB3` receipt; **requires capability**. |
| Edge     | `GET /edge/{path}`             | GET  | Gatewayed fetch; supports `Range: bytes=start-end`.                     |
| Mailbox  | `POST /mailbox/send`           | POST | At-least-once send; **idempotency key** recommended.                    |
| Mailbox  | `GET /mailbox/recv`            | GET  | Receive pending messages (batch).                                       |
| Mailbox  | `POST /mailbox/ack`            | POST | Acknowledge processed messages.                                         |
| Index    | `POST /index/resolve`          | POST | Resolve logical key → `AddrB3`.                                         |

> **Transport Invariants:**  
> • `max_frame = 1 MiB` (OAP/1 hard ceiling).  
> • Streaming chunk size ≈ **64 KiB** on Storage paths.  
> • All mutating endpoints require a **capability** (macaroon) and should use an **idempotency key**.

---

## 2) DTOs / Schemas

All DTOs are defined in `ron-proto`. The SDK enforces:

- **Strict deserialization** (`serde(deny_unknown_fields)`) for inbound DTOs (fail-closed on unexpected fields).
- **Canonical serialization** (no extra/unknown fields) for outbound payloads.
- **Content addressing** format: `AddrB3` strings are **`"b3:<hex>"`** (lowercase hex, no separators).

### 2.1 Object Manifest (example)

```rust
// ron-proto (sketch)
pub struct ObjectManifestV2 {
  pub id: String,          // b3:<hex>
  pub size: u64,           // bytes
  pub chunks: Vec<Chunk>,  // ~64 KiB each (last may be shorter)
}
````

* **Encoding**: DAG-CBOR (strict schema, versioned).
* **Validation**: If the SDK is the terminal consumer (e.g., after `storage_put`), it may optionally perform full **BLAKE3** digest verification.

### 2.2 Canonical Envelope (OAP/1)

| Field       | Type   | Description                               |
| ----------- | ------ | ----------------------------------------- |
| `len`       | `u32`  | Remaining length (excluding this prefix). |
| `ver`       | `u8`   | Protocol version (**1**).                 |
| `flags`     | `u16`  | Bit flags: `REQ`, `RESP`, `EVENT`, …      |
| `tenant_id` | `u128` | ULID/UUID; `0` if unused.                 |
| `corr_id`   | `u64`  | Correlation ID (for tracing).             |
| `payload`   | `[]`   | App-opaque bytes (may be compressed).     |

SDK obligations:

* **Refuse** to build/forward frames where `len > 1_048_576` (1 MiB).
* Map `corr_id` to tracing spans; propagate correlation across retries.

---

## 3) Bus Topics

**N/A for SDK.** `ron-app-sdk` does **not** publish or subscribe to kernel bus topics.
(Hosts may forward SDK health/metrics to their buses; that is out-of-scope for this crate.)

---

## 4) Canonical Test Vectors

> Test vectors live under `tests/vectors/` and are consumed by interop & property tests.

### 4.1 OAP/1 Frame — Minimal Request

* **Purpose**: validate frame size checks, version byte, and correlation propagation.
* **Given**:

  * `ver = 0x01`
  * `flags = REQ`
  * `tenant_id = 0`
  * `corr_id = 0x1122_3344_5566_7788`
  * `payload = 32 bytes` (opaque)
* **Expect**:

  * `len == 1 (ver) + 2 (flags) + 16 (tenant) + 8 (corr) + 32 (payload) = 59`
  * Parser accepts; encoder refuses if `payload.len()` pushes `len > 1 MiB`.

> Vector files:
> • `oap1_min_req.bin` (raw)
> • `oap1_min_req.json` (decoded fields)

### 4.2 Capability (Macaroon) Example

```json
{
  "typ": "macaroon",
  "ver": 1,
  "caveats": ["ttl=60s", "method=POST", "path=/put", "tenant=t01"],
  "sig": "base64-opaque"
}
```

* **Usage**: attached to every mutating request; transported as an auth header or within the OAP envelope headers (transport-dependent).
* **SDK rule**: on `CapabilityExpired/Denied`, **do not retry**; surface typed error.

### 4.3 Byte-Range GET (Edge)

* **Input**: `range = bytes=0-65535` (first 64 KiB)
* **Expect**:

  * Gateway returns `206 Partial Content` with `Content-Range: bytes 0-65535/*`.
  * SDK maps to `edge_get(cap, path, Some(ByteRange::from(0..=65535)))` and exposes the raw bytes to the caller.

### 4.4 Content Addressing Receipt (Storage PUT)

* **Input**: arbitrary payload ≤ 1 MiB; idempotency key set.
* **Expect**:

  * Response DTO contains `AddrB3` as `b3:<hex>`.
  * If SDK “terminal verify” is enabled, it computes BLAKE3 of the payload and asserts equality with `<hex>` before returning.

> Note: Digest equality is asserted **case-insensitively** for hex, but SDK **emits lowercase**.

---

## 5) Error Taxonomy (Wire ↔ SDK)

The SDK maps wire-level failures to stable `SdkError` variants (non-exhaustive). Key cases:

| Wire / Gateway Condition           | HTTP/OAP Code             | SDK Error Variant                                   | Retry?           |
| ---------------------------------- | ------------------------- | --------------------------------------------------- | ---------------- |
| Unsupported protocol/version       | `400 BadVersion` / OAP v! | `SdkError::OapViolation { reason: "version" }`      | **No**           |
| Frame exceeds 1 MiB                | `413 FrameTooLarge`       | `SdkError::OapViolation { reason: "frame" }`        | **No**           |
| Capability missing/invalid/expired | `401/403`                 | `SdkError::CapabilityDenied/CapabilityExpired`      | **No**           |
| Quota/rate limit                   | `429` (`Retry-After?`)    | `SdkError::RateLimited { retry_after }`             | **Yes** (hinted) |
| Gateway not ready                  | `503 NotReady`            | `SdkError::Server(503)`                             | **Yes**          |
| Transient server                   | `5xx`                     | `SdkError::Server(code)`                            | **Yes**          |
| Not found                          | `404`                     | `SdkError::NotFound`                                | No               |
| Conflict on idempotent PUT         | `409`                     | `SdkError::Conflict`                                | No               |
| Schema invalid / unknown fields    | decode failure            | `SdkError::SchemaViolation { path, detail }`        | No               |
| Connect/read/write timeout         | I/O timeout               | `SdkError::Transport(std::io::ErrorKind::TimedOut)` | **Yes**          |
| Overall deadline exceeded          | client timeout            | `SdkError::DeadlineExceeded`                        | No               |
| Tor SOCKS unavailable              | local transport           | `SdkError::TorUnavailable`                          | Maybe (config)   |

**Retry policy** follows the SDK’s invariants: **full-jitter exponential backoff**; idempotent or idem-keyed ops only.

---

## 6) Interop Guarantees

* **No Kernel Drift**: SDK surface tracks the canon gateways/planes; it never embeds kernel/bus semantics.
* **SemVer Discipline**:

  * **Schema/DTO** changes are **additive-first** (new optional fields).
  * Removing/renaming fields or changing types is **major** only and requires migration notes.
* **Strict Read / Canonical Write**:

  * SDK **rejects unknown inbound fields** (fail-closed), ensuring callers update promptly when schemas evolve.
  * SDK **emits only known fields** and canonical encodings.
* **Frame Limits**: OAP/1 1 MiB cap and ~64 KiB storage streaming are **hard** interop constraints.
* **Auditability**: Canonical vectors live in `/tests/vectors/` and are exercised in CI (round-trip + property tests).
* **Profile Parity**: Micronode/Macronode behave identically at the SDK surface (capability & envelope semantics unchanged).

---

## 7) References

* **Interop Blueprint GMI-1.6** — Omni-Gate contracts and planes.
* **OAP/1 Spec** — Version, flags, frame limits, correlation.
* **OBSERVABILITY.md** — Correlation IDs & span fields.
* **CONCURRENCY.md** — Deadline & retry contracts.
* **SECURITY.md** — Capability handling, redaction, and fail-closed posture.
* **CONFIG.md** — `transport`, deadlines, retry caps, Tor SOCKS, idempotency.

---

✅ With this, external integrators and alternate SDK authors can implement compatible clients: OAP/1 framing, gateway endpoints, strict DTOs, capability handling, and error → retry semantics are all explicit and testable.

```

