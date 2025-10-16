
````markdown
# 🔗 INTEROP.md — ron-accounting
*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-accounting`:

- Wire protocols & message formats (HTTP + TLS, OAP/1 framing for bus, optional MsgPack).
- DTOs & schemas for **sealed usage slices**.
- Bus topics and events.
- Canonical test vectors (hashes, frames, signatures).

This ensures all inter-crate and external integrations remain consistent with **GMI-1.6 Omni-Gate** and the crate’s **transient, export-focused** north star.

---

## 1) Protocols & Endpoints

### 1.1 Ingress (adapter feature only)

- **HTTP/1.1 over TLS 1.3** (via `tokio_rustls`), or **UDS** with `SO_PEERCRED` allowlist.
- **Endpoints**
  - `POST /export` — Accepts **SealedSliceV1** (see §2.1). Content-Type:
    - `application/dag-cbor` (canonical) or `application/msgpack` (feature `msgpack`).
  - `GET /metrics` — Prometheus text (local/gateway only).
  - `GET /readyz`, `GET /healthz` — liveness/readiness (see OBSERVABILITY).

**Transport Invariants**
- Max request body (**frame**) ≤ **1 MiB** (post-decompress).
- Chunked streaming **64 KiB** segments.
- Strict timeouts: read=5s, write=5s, idle=60s.
- **Authentication**: macaroon capability token (SECURITY.md), or UDS allowlist.

### 1.2 Egress (to `ron-ledger`)

- **HTTP/1.1 over TLS 1.3** (preferred) or **UDS**.
- **Endpoint**
  - `PUT /slices/{tenant}/{dimension}/{seq}`  
    Body = **SealedSliceV1** (same canonical encoding).  
    Idempotent: duplicate PUT returns `200 OK` with `{"ack":"dup"}`.
- **Ordering**: client (ron-accounting) preserves per-stream order `(tenant,dimension)`; ledger rejects gaps or mis-order (returns 409/422).

### 1.3 Bus (Kernel)

- **OAP/1 framed events** over the Kernel Bus for health/config signals.
- Envelope (see §2.3 Canonical Envelope).

---

## 2) DTOs / Schemas

> **Canonical serialization** is required for digest stability and idempotence.  
> The content address (b3) is computed over the **canonical bytes** of the DTO (DAG-CBOR or MsgPack in canonical map order).

### 2.1 `SealedSliceV1`

```rust
// Canonical, versioned payload used on-wire and for digest calculation.
#[derive(Serialize, Deserialize)]
struct SealedSliceV1 {
  // Identity
  tenant: u128,                 // opaque tenant id
  dimension: DimensionV1,       // "bytes" | "requests" | "cpu"  (closed string enum for V1)
  seq: u64,                     // per-(tenant,dimension) monotonically increasing

  // Window (UTC aligned)
  window_start_s: u64,          // unix secs inclusive
  window_end_s:   u64,          // unix secs exclusive

  // Metered rows (hot-path compact)
  rows: Vec<RowV1>,             // bounded; each Row adds a non-negative increment

  // Integrity / provenance
  b3: [u8; 32],                 // BLAKE3 digest of canonical bytes of this struct with b3 field set to zeroes during preimage
  prev_b3: [u8; 32],            // BLAKE3 of previous sealed slice for same (tenant,dimension), or zeros for seq==0
  sealed_at_ms: u64,            // monotone wall time (ms)
  codec: CodecV1,               // "dag-cbor" | "msgpack"
}

#[derive(Serialize, Deserialize)]
struct RowV1 {
  ns: u32,          // namespace (small integer domain for hot path)
  id: u128,         // object/account id within namespace
  inc: u64,         // saturating add; non-negative
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DimensionV1 { Bytes, Requests, Cpu }

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CodecV1 { DagCbor, Msgpack }
````

**Canonical rules (V1):**

* **Map key ordering**: lexical; no duplicate keys.
* **DAG-CBOR** preferred; MsgPack allowed if feature `msgpack` is enabled **and** `codec = "msgpack"`.
* `b3` computed over the canonical bytes of the struct **with `b3` set to 32 zero bytes**; then persisted with the resulting `b3`. This makes `b3` the *content address*.
* `prev_b3` links slices creating a per-stream hash chain for forensics.
* Unknown fields → **reject** (strict schema) to avoid silent drift.
* Size caps: whole DTO ≤ **1 MiB** after encoding.

### 2.2 Export ACK (ledger response)

```json
// 200 OK on success (including idempotent duplicate)
{ "ack": "ok",  "seq": 128, "b3": "d8e2...4a" }
{ "ack": "dup", "seq": 128, "b3": "d8e2...4a" }
```

**Errors** (see §5) use JSON problem style with `code`, `message`.

### 2.3 Canonical Envelope (OAP/1) — Bus events

| Field       | Type  | Description                                      |
| ----------- | ----- | ------------------------------------------------ |
| `len`       | u32   | Remaining length                                 |
| `ver`       | u8    | Protocol version (1)                             |
| `flags`     | u16   | `REQ, RESP, EVENT`                               |
| `tenant_id` | u128  | Opaque; `0` if not applicable                    |
| `corr_id`   | u64   | Correlation ID / tracing                         |
| `payload`   | bytes | CBOR-encoded `KernelEvent` or crate event struct |

---

## 3) Bus Topics

### 3.1 Events Published

* `accounting.rollover`

  ```rust
  struct RolloverEvtV1 {
    tenant: u128, dimension: String, seq: u64, b3: [u8;32], prev_b3: [u8;32], rows: u32
  }
  ```
* `accounting.export.status`

  ```rust
  struct ExportStatusV1 {
    tenant: u128, dimension: String, seq: u64, status: "ok"|"dup"|"retry"|"fail", latency_ms: u32
  }
  ```
* `kernel.health` → `KernelEvent::Health { service: "ron-accounting", ok: bool }`
* `kernel.config.updated { version: u64 }`

### 3.2 Events Subscribed

* `kernel.shutdown` → begin graceful drain.
* `kernel.config.updated` → reload where safe (see CONFIG dynamic reload).

> **Cardinality discipline:** Events include opaque tenant/dimension IDs but bus subscribers must avoid exploding Prometheus labels; use logs for top-N diagnosis.

---

## 4) Canonical Test Vectors

> Store under `tests/vectors/ron-accounting/`:
>
> * `sliceV1_dagcbor_ok.json` (human readable),
> * `sliceV1_dagcbor.bin` (canonical bytes),
> * `sliceV1_msgpack.bin` (optional),
> * `sliceV1_digest.txt` (hex of b3),
> * `oap1_rollover_evt.bin` (framed bus event).

### 4.1 SealedSliceV1 — tiny example (deterministic)

Input struct (logical):

```json
{
  "tenant": 1,
  "dimension": "bytes",
  "seq": 0,
  "window_start_s": 1_700_000_000,
  "window_end_s":   1_700_000_300,
  "rows": [
    {"ns":1,"id":"00000000-0000-0000-0000-0000000000aa","inc":42},
    {"ns":1,"id":"00000000-0000-0000-0000-0000000000ab","inc":100}
  ],
  "b3": "0000...0000",                  // 32 zero bytes during preimage
  "prev_b3": "0000...0000",
  "sealed_at_ms": 1_700_000_123_456,
  "codec": "dag-cbor"
}
```

Expected canonical DAG-CBOR bytes: `sliceV1_dagcbor.bin` (provided in repo).
Expected **BLAKE3** digest (hex):

```
b3: 6b8b4567327b23c6643c986966334873f1e289...  (placeholder — replace with generated)
```

Re-encode with `b3` set to the computed digest and verify **round-trip**; `prev_b3` remains zero for `seq=0`.

### 4.2 OAP/1 event frame

* Payload = CBOR of `RolloverEvtV1 { tenant:1, dimension:"bytes", seq:0, ... }`.
* Envelope:

  * `ver=1`, `flags=EVENT`, `tenant_id=1`, `corr_id=0xdead_beef`.
* Hex fixture: `oap1_rollover_evt.bin` (golden).

---

## 5) Error Taxonomy (wire-level)

### 5.1 HTTP adapter (ingress)

| HTTP | Code              | When                               | Notes                 |
| ---- | ----------------- | ---------------------------------- | --------------------- |
| 400  | `BadVersion`      | Unknown DTO version                |                       |
| 400  | `SchemaViolation` | Unknown field / wrong type         | Strict `deny_unknown` |
| 401  | `Unauth`          | Missing/invalid macaroon           | SECURITY.md           |
| 413  | `FrameTooLarge`   | >1 MiB after decode                |                       |
| 415  | `UnsupportedType` | Not DAG-CBOR/MsgPack as negotiated |                       |
| 422  | `OrderOverflow`   | Would violate per-stream ordering  | Lane cap/order hold   |
| 429  | `QuotaExceeded`   | Tenant shaping / backpressure shed |                       |
| 429  | `Busy`            | Queue full (try_send)              |                       |
| 503  | `NotReady`        | `/readyz` false                    | Degraded state        |

### 5.2 Ledger egress (responses mapped to SDK)

| HTTP | ack / code      | SDK `Error` mapping                      |
| ---- | --------------- | ---------------------------------------- |
| 200  | `{"ack":"ok"}`  | —                                        |
| 200  | `{"ack":"dup"}` | `Ack::Duplicate` observed                |
| 409  | `Conflict`      | `Error::OrderOverflow`                   |
| 422  | `Unprocessable` | `Error::SchemaViolation`                 |
| 500  | `Remote5xx`     | `Error::Timeout/Other` with retry policy |

---

## 6) Interop Guarantees

1. **Stable V1 schema**: `SealedSliceV1` is versioned and canonical; breaking changes → **major** schema version (V2) under new content-type or `version` tag.
2. **Deterministic encoding**: Same logical struct → same canonical bytes → same `b3` across platforms/endianness (tested in CI).
3. **Idempotence**: Re-PUT of identical slice returns `dup` without side effects.
4. **Ordering**: Per `(tenant,dimension)` stream enforces `seq(N+1)` not accepted before `seq(N)` unless `seq(N)` is declared **shed** by policy; gaps are explicit errors.
5. **Strictness**: Unknown fields **rejected** on read (no silent drop) to prevent drift.
6. **Size/time caps**: DTO ≤ 1 MiB; read=5s/write=5s/idle=60s enforced.
7. **Auth**: Macaroon caveats restrict tenant/dimension/path and carry expiry; UDS allowlist alternative.
8. **Auditability**: Canonical vectors shipped in `tests/vectors/` and used in CI conformance tests.

---

## 7) Interop Matrix (features & compatibility)

| Feature        | Encoding      | Transport     | Notes                                      |
| -------------- | ------------- | ------------- | ------------------------------------------ |
| (default)      | DAG-CBOR      | TLS 1.3 / UDS | Preferred; canonical                       |
| `msgpack`      | MsgPack (can) | TLS 1.3 / UDS | Canonical map order; set `codec="msgpack"` |
| `http-adapter` | DAG-CBOR      | HTTP/1.1 TLS  | Adds `/export` `/metrics` `/readyz`        |
| `wal`          | —             | FS (bounded)  | On-disk staging; not a wire protocol       |
| `otel`         | —             | OTLP (traces) | Observability; not for slice transport     |

---

## 8) Reference Flows

### 8.1 Adapter Ingress (client → ron-accounting)

```
Client --TLS--> /export  (SealedSliceV1 [DAG-CBOR])
         <----  202 Accepted  (async export)
         <----  409/429/503 on policy/readiness violations
```

### 8.2 Egress (ron-accounting → ron-ledger)

```
put(slice{tenant,dimension,seq,b3,prev_b3}) --ordered--> ron-ledger
<- 200 {"ack":"ok"|"dup"}  or  409/422 on conflict/schema
```

### 8.3 Bus

```
[EVENT] accounting.rollover {tenant,dimension,seq,b3,prev_b3,rows}
[EVENT] accounting.export.status {tenant,dimension,seq,status,latency_ms}
```

---

## 9) SDK Guidance (non-Rust)

* **Canonical bytes win**: Always compute `b3` over canonical encoding (same rules as Rust).
* **`prev_b3` chain**: Keep the previous digest per stream to form a verifiable chain.
* **Strict schema**: Do not add fields; target V1 until V2 is announced.
* **Idempotence**: Treat `dup` as success; do not retry infinitely on 5xx — use capped backoff (global 10s budget).
* **Clock**: `sealed_at_ms` should be monotone; skew not critical but bounded (<5s preferred).

---

## 10) Conformance & Test Harness

* Unit tests assert:

  * Canonical encoding round-trip → expected `b3`.
  * Ledger `dup` semantics recognized as success.
  * OAP/1 envelope parses golden frames.
* Property tests (proptest):

  * Random rows within caps → canonical encoding stable across runs.
* Cross-lang vectors:

  * Minimal Python/Go scripts under `tests/interop/` re-encode vectors and verify digest equality.
* CI:

  * Runs the cross-lang checks on PRs touching DTOs or codec features.
  * Fails if any vector digest changes without bumping schema version.

---

## 11) References

* **GMI-1.6 Omni-Gate** (workspace Interop Blueprint)
* **OAP/1** framing (workspace spec)
* **OBSERVABILITY.md** — tracing IDs, readiness semantics
* **SECURITY.md** — auth, size/ratio caps, no-PII constraints
* **CONFIG.md** — caps for window length, WAL, queue sizes

---

## 12) Appendix — cURL & Code Snippets

### 12.1 cURL (DAG-CBOR payload)

```bash
# SLICE.cbor is canonical DAG-CBOR of SealedSliceV1 with b3 computed
curl -sS --http1.1 --tlsv1.3 \
  -H 'Content-Type: application/dag-cbor' \
  -H "Authorization: Bearer $(cat macaroon.jwt)" \
  --data-binary @SLICE.cbor \
  https://accounting.local/export
```

### 12.2 Rust — compute `b3` (preimage rule)

```rust
fn compute_b3(mut s: SealedSliceV1) -> [u8; 32] {
    let zero = [0u8; 32];
    s.b3 = zero;
    let bytes = dag_cbor::to_vec_canonical(&s).expect("encode");
    blake3::hash(&bytes).into()
}
```

### 12.3 Rust — OAP/1 envelope (bus)

```rust
let payload = minicbor::to_vec(&rollover_evt).unwrap();
let env = Oap1 { ver:1, flags:EVENT, tenant_id, corr_id, len: payload.len() as u32 };
bus.send(env, &payload).await?;
```

---

✅ **Why this matters**: This document freezes the **wire contract** — DTO shapes, encoding, endpoints, and events — so `ron-accounting`, `ron-ledger`, adapters, and non-Rust SDKs stay in lock-step, with **canonical test vectors** to prevent drift. It’s the interop guardrail for **ordered + idempotent** metering at scale.

```
```
