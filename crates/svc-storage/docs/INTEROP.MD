# 🔗 INTEROP — svc-storage

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-storage`:

* Wire protocols & message formats (HTTP/1.1 + TLS, optional UDS; internal bus events).
* DTOs & schemas (error envelope, PUT/POST response, manifests).
* Bus topics and events (published/subscribed).
* Canonical interop drills & vectors.

This keeps inter-crate and external integrations consistent with the RustyOnions **Omni-Gate (GMI-1.6)** posture and the crate’s IDB.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

* **HTTP/1.1** (primary), optional **TLS 1.3** via `tokio_rustls::rustls::ServerConfig`.
* **UDS** (Unix Domain Socket) for intra-node callers; hardened with `SO_PEERCRED`.
* **No gRPC/QUIC** on this service (gateway may add frontends; storage remains HTTP).

### 1.2 Exposed HTTP Endpoints (stable)

| Method | Path       | Auth                    | Body/Media                 | Success Codes       | Notes                                                       |
| -----: | ---------- | ----------------------- | -------------------------- | ------------------- | ----------------------------------------------------------- |
|    GET | `/o/{b3}`  | none (policy-driven)    | —                          | `200`, `206`, `304` | Content-addressed read; `Range` supported                   |
|   HEAD | `/o/{b3}`  | none                    | —                          | `200`, `206`        | Headers only; range validation                              |
|    PUT | `/o/{b3}`  | **macaroon** (required) | bytes (or xfer-compressed) | `201`, `200`        | Idempotent by address; verifies `{b3}` over canonical bytes |
|   POST | `/o`       | **macaroon** (required) | bytes (or xfer-compressed) | `201`, `200`        | Digest-on-write helper → returns `{"address":"b3:<hex>"}`   |
|    GET | `/metrics` | none                    | text/plain; OpenMetrics    | `200`               | Prometheus exposition                                       |
|    GET | `/healthz` | none                    | —                          | `200`               | Liveness                                                    |
|    GET | `/readyz`  | none                    | application/json           | `200`, `503`        | Readiness (fail-closed on writes)                           |
|    GET | `/version` | none                    | application/json           | `200`               | Build info                                                  |

**Transport Invariants**

* **HTTP request cap**: body ≤ **1 MiB** (reject `413`).
* **Safe decompression**: ratio ≤ **10×** + absolute output cap; `413` with structured error on exceed.
* **Streaming chunk size** (implementation detail): ~**64 KiB**.
* **Strong validators**: `ETag: "b3:<hex>"`, `Vary: Accept-Encoding`, `Accept-Ranges: bytes`.

**Headers & Conventions**

* **Auth (privileged ops)**: `Authorization: Macaroon <token>`.
* **Range reads**: `Range: bytes=<start>-<end>`; `416` if invalid.
* **Caching**: `If-None-Match`, `If-Match` supported (operate on strong ETag).
* **Correlation**: optional `X-Corr-ID` (ULID/UUID); echoed and logged.

---

## 2) DTOs / Schemas

### 2.1 Error Envelope (HTTP)

```json
{
  "error": "busy|unauth|payload_too_large|decompress_cap|ratio_cap|timeout|not_found|conflict",
  "message": "human-readable detail",
  "corr_id": "01J…ULID"
}
```

* Always present for non-2xx (except `304`).
* `corr_id` mirrors `X-Corr-ID` or is server-generated.

### 2.2 PUT/POST Response

```json
{
  "address": "b3:<hex>"
}
```

* `PUT /o/{b3}`: body optional; if present, must match route.
* `POST /o`: always returns computed address; `Location: /o/{b3}`.

### 2.3 Manifest (local schema; not externally served by default)

When manifests are emitted/shared (e.g., via admin/debug), they are **DAG-CBOR v2**:

```rust
// Pseudocode schema (DAG-CBOR, deterministic ordering)
struct ObjectManifestV2 {
  id: String,            // "b3:<hex>" canonical content ID
  size: u64,             // object length in bytes
  chunk_size: u32,       // nominal streaming chunk size (bytes)
  chunks: Vec<ChunkRef>, // ordered, contiguous coverage
}

struct ChunkRef {
  offset: u64,           // absolute object offset
  len: u32,              // bytes in this chunk
  id: String             // "b3:<hex>" of the chunk
}
```

**Validation**: on ingestion or repair, the service verifies the object `id` by hashing the **canonical** (decompressed) bytes; chunk map must cover `[0,size)` without gaps/overlaps.

---

## 3) Bus Topics (Kernel/Event Bus)

### 3.1 Published

* `kernel.health` → `KernelEvent::Health { service: "svc-storage", ok: bool }`
* `storage.repair.planned` → `{ plan_id, addresses, rf_target }`
* `storage.repair.executed` → `{ plan_id, bytes, duration_ms }`
* `storage.ingest` → `{ address: "b3:<hex>", new: bool, bytes: u64 }`
* `storage.get` → `{ address: "b3:<hex>", range: [start,end]? }` *(sampling recommended)*

> **Note:** High-cardinality fields (full addresses) should be truncated to prefix for events destined for shared topics.

### 3.2 Subscribed

* `config.updated` → swap config snapshot, rebind where necessary.
* `bus.shutdown` → initiate graceful drain.
* *(Optional)* `placement.updated` → refresh provider/region hints.

---

## 4) Canonical Interop Vectors & Drills

> Vectors are expressed as **procedures** so SDKs and integrators can reproduce them verbatim. Where a literal digest is not embedded, the client must compare the system’s returned digest to its own computation.

### 4.1 Idempotent PUT

1. Compute `b3` over canonical bytes (client).
2. `PUT /o/{b3}` with body (optionally transfer-compressed).
3. Expect `201 Created` on first write; **repeat** the same request → `200 OK`.
4. Mismatch (intentionally corrupt body) → `409 Conflict`.

**Assertions**

* `ETag: "b3:<hex>"` on subsequent `GET`/`HEAD`.
* `storage_put_latency_seconds` observed; no duplicate durable state.

### 4.2 Digest-on-Write POST

1. `POST /o` with canonical bytes.
2. Expect `{ "address":"b3:<hex>" }` and `Location`.
3. `GET /o/{address}` must return identical bytes; `ETag` must match.

### 4.3 Range Semantics

1. `GET /o/{b3}` with `Range: bytes=0-65535`.
2. Expect `206 Partial Content`, `Content-Range: bytes 0-65535/<size>`, `Content-Length: 65536`.

### 4.4 Safe Decompression

1. Send a body that would decompress **>10×** its compressed size with `Content-Encoding: gzip`.
2. Expect `413` with error `"decompress_cap"` and metric `decompress_reject_total{reason="cap"}` incremented.

### 4.5 Readiness & Backpressure

* Flip readiness dependency (e.g., placement cache unavailable) → `/readyz` returns `503` with JSON describing missing keys.
* Saturate `work` queue → `429` with `Retry-After`; `rejected_total{reason="busy"}` increments.

### 4.6 Integrity Enforcement

* Corrupt on-disk chunk (test harness) → GET fails with 5xx and `integrity_fail_total{reason="checksum"}`.

---

## 5) Error Taxonomy (HTTP)

| Code | Name/Reason                  | When                                      |
| ---: | ---------------------------- | ----------------------------------------- |
|  400 | `bad_request`                | Malformed address/headers                 |
|  401 | `unauth`                     | Missing/invalid macaroon                  |
|  403 | `forbidden`                  | Capability present but insufficient scope |
|  404 | `not_found`                  | Unknown `b3`                              |
|  409 | `conflict`                   | PUT body digest mismatch with `{b3}`      |
|  413 | `payload_too_large`          | Body exceeds cap                          |
|  413 | `decompress_cap`/`ratio_cap` | Decompression limits exceeded             |
|  416 | `range_not_satisfiable`      | Invalid range                             |
|  429 | `busy`                       | Backpressure: queue full / rate limit     |
|  503 | `not_ready`                  | Readiness gate failed / degraded          |

> All non-2xx include the **Error Envelope** (§2.1). `Retry-After` accompanies `429/503` when appropriate.

---

## 6) Interop Guarantees

* **Content addressing is canonical**: object identity is `b3:<hex>` of **canonical** bytes; ETag is strong and equals that identity.
* **Transport caps are stable**: 1 MiB request cap; decompression ratio ≤ 10×; these only change with documented version policy.
* **Headers are normative**: `ETag`, `Accept-Ranges`, `Vary: Accept-Encoding` are always correct when applicable.
* **Idempotency**: `PUT /o/{b3}` is idempotent; `POST /o` is a helper that returns the same address for the same content.
* **Auth**: privileged ops require **macaroon** capabilities; there are no ambient bypass headers.
* **Backward compatibility**: unknown JSON fields are ignored; new optional fields do not break clients.
* **Auditability**: interop drills are encoded as integration tests; logs carry `corr_id` consistently.

---

## 7) Reference Snippets

### 7.1 `curl` Recipes

**GET with Range and cache validator**

```bash
curl -v "$BASE/o/$B3" \
  -H "Range: bytes=0-65535" \
  -H 'If-None-Match: "'$B3'"' \
  -H "X-Corr-ID: $(uuidgen)"
```

**PUT idempotent**

```bash
curl -v -X PUT "$BASE/o/$B3" \
  -H "Authorization: Macaroon $CAP" \
  --data-binary @object.bin
```

**POST digest-on-write**

```bash
curl -v -X POST "$BASE/o" \
  -H "Authorization: Macaroon $CAP" \
  --data-binary @object.bin
```

### 7.2 Minimal Client Digest (Rust)

```rust
// Compute BLAKE3 and decide PUT vs POST.
// (Use the blake3 crate; address = format!("b3:{:x}", blake3::hash(&bytes)))
```

---

## 8) Cross-Refs

* **IDB.md** — invariants (content addressing, caps, readiness, capability auth)
* **CONFIG.md** — tunables (timeouts, caps, pacing)
* **SECURITY.md** — auth, key handling, amnesia mode
* **OBSERVABILITY.md** — correlation/metrics names
* **API.md** — HTTP contract and SemVer rules
* **OpenAPI** — `/docs/openapi/svc-storage.yaml` (HTTP source of truth)

---

✅ With this INTEROP, all external SDKs and sibling services share a **single, precise wire-level contract** for `svc-storage`: routes, caps, DTOs, bus events, and reproducible drills.
