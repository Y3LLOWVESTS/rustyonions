
---

# 🔗 INTEROP.md — micronode

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*
*status: draft*
*last-updated: 2025-09-23*

---

## 0) Purpose

Define the **interop surface** of `micronode`:

* Wire protocols & message formats (OAP/1, HTTP/TLS; Tor via `ron-transport` feature).
* DTOs & schemas (strict, versioned).
* Bus topics and events (kernel + service facets).
* Canonical test vectors and error taxonomy.

This file is drift-proofed against the canon (immutable **33-crate** set) and Interop GMI-1.6/Hardening v2.0 limits: **OAP/1 `max_frame=1 MiB`, streaming chunk ≈ 64 KiB, decompression ≤ 10×**. &#x20;

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

* **HTTP/1.1 + TLS 1.3** (`tokio_rustls::rustls::ServerConfig`), uniform across profiles. Tor/mTLS available behind `ron-transport` features. &#x20;
* **OAP/1 framed control/data**: **`max_frame=1 MiB`**; storage/data streaming uses **\~64 KiB chunks** (distinct from frame size).&#x20;

### 1.2 Exposed Endpoints (Micronode profile)

Micronode bundles a minimal public surface (DX-lean, single-tenant):

| Path                  | Method | Purpose                          | Auth (cap) | Limits / Notes                                                                   |
| --------------------- | ------ | -------------------------------- | ---------- | -------------------------------------------------------------------------------- |
| `/o/{addr}`           | GET    | Read object by content address   | required   | BLAKE3 verify on read; range OK.                                                 |
| `/put`                | POST   | Store object                     | required   | Body ≤ 1 MiB (if non-stream); streamed in \~64 KiB chunks; decompression ≤ 10×.  |
| `/mbox/send` *(feat)* | POST   | Mailbox send (store-and-forward) | required   | At-least-once, idempotency key, visibility timeout.                              |
| `/mbox/recv` *(feat)* | GET    | Mailbox receive                  | required   | ACK/visibility; DLQ on poison messages.                                          |
| `/metrics`            | GET    | Prometheus metrics               | none       | Golden metrics present (latency hist, rejects).                                  |
| `/healthz`            | GET    | Liveness                         | none       | Always 200 if process alive.                                                     |
| `/readyz`             | GET    | Readiness                        | none       | Gates config/deps; degrades before collapse.                                     |
| `/version`            | GET    | Build/version + PQ posture       | none       | Surfaces PQ flags for audit.                                                     |

**Transport invariants:** Timeouts 5s; 512 inflight; 500 rps; 1 MiB request cap; decompression ≤ 10× + absolute cap. Enforced by the shared hardening layer.&#x20;

**Overlay/DHT boundary:** overlay sessions/gossip live in `svc-overlay`; **discovery (Kademlia/Discv5) is `svc-dht`**. Micronode doesn’t embed DHT logic in overlay.&#x20;

---

## 2) DTOs / Schemas

All wire DTOs are **pure types** (`ron-proto`) with `#[serde(deny_unknown_fields)]`; unknown fields are ignored on read but **never silently dropped** on re-emit; schema compat tested in CI. Encoding for manifests is **DAG-CBOR** with canonicalization (no floats). &#x20;

### 2.1 ObjectManifestV2 (content addressing)

```rust
/// DAG-CBOR, canonical. Content-addressed by BLAKE3-256 (b3:<hex>).
struct ObjectManifestV2 {
  id: String,          // b3:<hex>
  size: u64,           // bytes
  chunks: Vec<Chunk>,  // ~64 KiB streaming chunks
}
```

* **Addressing:** `b3:<hex>` everywhere; **verify full digest before serving**.&#x20;
* **Validation:** multi-chunk rehydrate MUST yield identical `id`. (Vector below §4.2.)&#x20;

### 2.2 OAP/1 Canonical Envelope (control/data)

| Field       | Type  | Description                              |
| ----------- | ----- | ---------------------------------------- |
| `len`       | u32   | Payload length (remaining)               |
| `ver`       | u8    | Protocol version (1)                     |
| `flags`     | u16   | `REQ`, `RESP`, `EVENT`, …                |
| `tenant_id` | u128  | ULID/UUID; 0 for single-tenant micronode |
| `corr_id`   | u64   | Correlation ID (tracing)                 |
| `payload`   | bytes | Application-opaque (may be compressed)   |

**Envelope semantics & vectors** are normative in Interop GMI-1.6 / OAP-1 spec; vectors live in `/tests/vectors/`.&#x20;

### 2.3 Capability (macaroon) sketch

```json
{
  "typ":"macaroon",
  "alg":"Hybrid-Ed25519+Dilithium3",
  "caveats":["ttl=60s","method=GET","path=/o/","max-bytes=1048576"],
  "kid":"cap-2025-09",
  "sig":"<base64>"
}
```

Hybrid signatures are accepted where configured; SDKs default to hybrid and log any downgrade to classical.&#x20;

---

## 3) Bus Topics

Micronode embeds kernel supervision/bus; topics follow the kernel’s minimal contract.

### 3.1 Events Published

* `micronode.health` → `KernelEvent::Health { service, ok }`
* `micronode.crash`  → `KernelEvent::ServiceCrashed { service, reason }`
* Service facets (when enabled): `overlay.obj_put`, `overlay.obj_get` (for index hydration). &#x20;

### 3.2 Events Subscribed

* `config.updated { version }` → refresh effective config.
* `bus.shutdown` → graceful drain & readiness shed ordering. (Degrades **before** collapse.)&#x20;

---

## 4) Canonical Test Vectors

Vectors are **authoritative** for interop and stored under `/tests/vectors/`; CI asserts round-trip determinism.&#x20;

### 4.1 OAP/1 Frame Round-Trip

* **Input (hex)** → decode → JSON `{"ver":1,"flags":["REQ"],"tenant_id":"0","corr_id":42,"verb":"OBJ_GET","payload_len":<n>}`
* **Re-encode** → byte-identical. (Files: `oap1_obj_get_roundtrip.json`, `.hex`)&#x20;

### 4.2 Manifest Digest — “hello world”

* Payload: ASCII `hello world`
* Expect: `size=11`; single-chunk acceptable; multi-chunk rehydrate equals single-chunk `id`; `root.id = b3:<hex-of(hello world)>`. (Files: `hello_world.*`)&#x20;

### 4.3 Capability Acceptance / Rejection

* **Accept:** hybrid macaroon with valid `nbf/exp`, `method=POST`, `path=/put`, `max-bytes=1 MiB`. → **201 Created**
* **Reject:** tampered caveat / bad `kid` / missing PQ leg → **403 InvalidCapability**. (Files: `cap_ok.json`, `cap_tampered.json`, `cap_missing_pq_leg.json`)&#x20;

### 4.4 Size & Quota

* `POST /put` with 1 MiB + 1 byte → **413 FrameTooLarge**.
* OAP/1 non-stream frame with `len > 1 MiB` → **FrameTooLarge**.&#x20;

---

## 5) Error Taxonomy (HTTP ↔ OAP/1)

| Code / Variant  | When                              | HTTP | OAP/1           |   |
| --------------- | --------------------------------- | ---- | --------------- | - |
| `BadVersion`    | `ver != 1`                        | 400  | `BadVersion`    |   |
| `FrameTooLarge` | frame (non-stream) exceeds 1 MiB  | 413  | `FrameTooLarge` |   |
| `ChunkTooLarge` | a streaming chunk `len > ~64 KiB` | 413  | `ChunkTooLarge` |   |
| `QuotaExceeded` | tenant quota exhausted            | 429  | `QuotaExceeded` |   |
| `NotReady`      | readiness gate failed             | 503  | `NotReady`      |   |
| `NotFound`      | unknown object address            | 404  | —               |   |

---

## 6) Interop Guarantees

* **No Kernel Drift:** Kernel’s public surface (Bus, KernelEvent, Metrics, HealthState, Config, `wait_for_ctrl_c()`) is semver-frozen; micronode adheres.&#x20;
* **SemVer Discipline:** Any wire-visible breaking change (paths, DTOs, flags, errors) bumps **major**.&#x20;
* **Backward Compatibility:** Unknown fields ignored on read, never silently dropped on re-emit; unknown verbs → `501`; TLS type uniform.&#x20;
* **Resource Bounds:** `max_frame`, `chunk_size`, connection/time budgets enforced centrally by transport config + hardening layer.&#x20;
* **Amnesia Mode:** Wire contracts unchanged; only durability semantics differ (RAM-only, zero disk) — **micronode default**.&#x20;

---

## 7) References

* **Interop Blueprint GMI-1.6** (Omni-Gate alignment; vectors, flags, LZ4 profile).
* **OAP/1 Spec** (framing, verbs, errors; `max_frame=1 MiB`, streaming ≈ 64 KiB).&#x20;
* **HARDENING v2.0** (timeouts, caps, decompression ≤ 10×, UDS perms).&#x20;
* **SCALING/OBSERVABILITY** (readiness degrades before collapse; golden metrics).&#x20;
* **Complete Crate List (33, immutable)** — guarantees profile parity with macronode & services.&#x20;

---

## 8) CI & Enforcement

* **Vectors:** `/tests/vectors/` must pass (frame round-trip, manifest digest, capability accept/reject, size/quota).&#x20;
* **Schema Guard:** DTOs use `deny_unknown_fields`; compat tests run in CI.&#x20;
* **Hardening Gate:** 5s/512/500rps/1 MiB/≤10× enforced; `/metrics`, `/healthz`, `/readyz`, `/version` present.&#x20;
* **Feature Matrix:** Build/run with amnesia ON/OFF; transport with/without `arti`.&#x20;

---

## 9) Notes on Profile Parity

Micronode (single-binary, **amnesia ON by default**) and Macronode (multi-service) share identical **wire contracts** and SDKs; only **deployment/scale** differs.&#x20;

---

✅ **Outcome:** This interop spec declares micronode’s **wire-level contract**—protocol limits, DTO strictness, bus events, and canonical vectors—so SDKs and services interoperate **without drift** across the 33-crate canon.
