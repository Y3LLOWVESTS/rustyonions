````markdown
# 🔗 INTEROP.md — svc-overlay

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-overlay` so independently implemented peers/SDKs and sibling crates compose safely:

- **Wire**: OAP/1 framed session protocol over TLS/QUIC (optionally Tor via `ron-transport`), admin HTTP.
- **Messages**: envelope, control/gossip payloads, handshakes, errors.
- **Schemas/DTOs**: canonical types, encodings, and **deterministic** serialization rules.
- **Bus topics**: events published/subscribed.
- **Vectors**: golden examples for conformance.

This aligns with **GMI-1.6 Omni-Gate** and the project’s IDB invariants (max frame size, readiness truth, zero-trust, amnesia).

---

## 1) Protocols & Endpoints

### 1.1 Transport stack (ingress)

- **Primary**: **OAP/1** (Overlay Application Protocol v1) over **TLS 1.3** (rustls).
- **Optional**: OAP/1 over **QUIC** (`quinn`) using a **single bidirectional stream** per session.  
  *Anti-amplification MUST be enforced; 0-RTT is DISABLED unless explicitly enabled and tested.*
- **Optional**: OAP/1 over **Tor** (`arti`) for onion services.  
  *Do not trust source IP; rate/quotas MUST key on identity/capability, not address.*

**Transport invariants**
- **Endianness**: **Big-endian** network order for numeric fields.
- **`max_frame`** = **1 MiB** (hard cap, post-decompression).  
- **`chunk_bytes`** ≈ **64 KiB** application payload chunks (recommended).
- **TLS**: `tokio_rustls::rustls::ServerConfig` only; **ALPN = `ron/oap1`** required.  
- **mTLS (optional, cluster)**: SAN pinning; wildcard CAs forbidden in prod.
- **PQ**: Hybrid KEX via `ron-kms` when `pq_mode=hybrid` (opt-in, interop-gated).

### 1.2 Admin HTTP (local / behind gateway)

- `GET /healthz` → liveness (200 if process running).
- `GET /readyz` → readiness truth (200 when safe to accept **writes**; else 503 with JSON body).
- `GET /metrics` → Prometheus exposition.
- `GET /version` → semver + build info.

> Admin surface is **stable API** (see `API.md` for SemVer rules).

---

## 2) Wire Envelope & Messages

### 2.1 OAP/1 Envelope

| Field         | Type  | Size | Description                                                        |
|---------------|-------|-----:|--------------------------------------------------------------------|
| `len`         | u32   |    4 | **Remaining length** (all bytes after this field)                  |
| `ver`         | u8    |    1 | Protocol version (**1**)                                           |
| `flags`       | u16   |    2 | Bitfield: `REQ(0) RESP(1) EVENT(2) COMP(3) ACKREQ(4)` … reserved  |
| `tenant_id`   | u128  |   16 | ULID/UUID; **0** if unused                                         |
| `corr_id`     | u64   |    8 | Correlation ID (see §4); `0` if none                               |
| `payload`     | bytes |   N  | Application-opaque; CBOR/MsgPack/JSON; may be **COMP** (Zstd)      |

**Rules**
- A frame is **valid** iff `len == 1+2+16+8 + payload.len()` and the total on-wire size is `4 + len`.
- If the **COMP** flag is set, `payload` is **Zstd** compressed; post-decompression size MUST NOT exceed **1 MiB** and ratio MUST NOT exceed **10×**.
- Unknown **flags** MUST be **ignored** by receivers (mask off reserved bits).
- Fields are **fixed width**; there is **no** varint packing.

### 2.2 Payload Kinds (CBOR-first)

Payload is a **typed message** in one of the supported encodings; **DAG-CBOR (deterministic)** is RECOMMENDED. Each message carries a top-level field `"kind"`.

Common kinds:

```cbor-diag
{
  "kind": "hello",
  "versions": [1],
  "features": {"pq":"off|hybrid","comp":["zstd"]},
  "max_frame": 1048576,
  "token": "macaroon:BASE64..."  ; optional, capability auth
}
````

```cbor-diag
{
  "kind": "hello_ack",
  "chosen": {"ver":1,"pq":"off|hybrid","comp":"zstd"},
  "limits": {"max_frame":1048576,"chunk":65536}
}
```

```cbor-diag
{
  "kind": "gossip",
  "topic": "overlay/feed/v1",
  "ts": 1730592000,
  "fanout": 3,
  "data": h'…'               ; opaque user bytes (already chunked/encoded upstream)
}
```

```cbor-diag
{
  "kind": "ack",
  "for":  1234567890,        ; corr_id of request
  "ok":   true
}
```

```cbor-diag
{
  "kind": "error",
  "code": "FrameTooLarge|BadVersion|QuotaExceeded|NotReady|Unauth|Timeout",
  "msg":  "human-compatible detail"
}
```

**Encoding rules (determinism)**

* **Map keys sorted** lexicographically.
* No float NaNs/Inf; use integers/strings for codes.
* **Unknown fields ignored** on read and **preserved only** if acting as a pass-through (not typical here).

### 2.3 Handshake

1. Client sends **`hello`** (`REQ` flag).
2. Server replies **`hello_ack`** (`RESP` flag) or `error`.

Negotiation fields:

* `versions`: list including **1**.
* `features`: `pq ∈ {"off","hybrid"}`, `comp ⊇ ["zstd"]?`.
* `max_frame`: MUST be **≤ 1 MiB**.
* `token`: macaroon capability (optional; else send later with first `REQ`).

Receivers MUST:

* Choose **lowest compatible risk** (e.g., `pq=off` if `hybrid` unsupported).
* Reject unknown **mandatory** features (if any) with `error{code="BadVersion"}`.

---

## 3) DTOs / Schemas

### 3.1 Capability Token (Macaroon)

```json
{
  "typ": "macaroon",
  "v": 1,
  "caveats": ["ttl=60s","allow=topic:overlay/feed/*","rate<=500rps"],
  "sig": "BASE64URL(...)"
}
```

* **Transport**: carried in `hello.token` or per-message as a CBOR field `"cap"`.
* **Verification**: server validates caveats; tokens are **never** logged; only `cap_id` or fingerprint appears in logs.

### 3.2 GossipEnvelope (CBOR schema)

```cbor-diag
{
  "kind":"gossip",
  "topic": tstr,
  "ts": uint,               ; seconds since epoch
  "fanout": uint,           ; suggested relay fanout (bounded by server)
  "data": bstr              ; application bytes
}
```

### 3.3 Error (CBOR schema)

```cbor-diag
{
  "kind":"error",
  "code": "BadVersion|FrameTooLarge|QuotaExceeded|NotReady|Unauth|Timeout",
  "msg": tstr
}
```

---

## 4) Correlation & Tracing

* **`corr_id (u64)`** in the OAP envelope propagates request lineage.

  * If an external HTTP ingress provides `X-Corr-ID` (UUID/ULID), the gateway SHOULD derive `corr_id` by hashing (e.g., xxhash64 of the 128-bit value).
  * Within overlay, services generate **random 64-bit** IDs when absent.
* The full higher-entropy ID MAY still appear in **logs** (not on wire).

---

## 5) Bus Topics

### 5.1 Published (outbound)

* `overlay.session.opened`

  ```rust
  struct SessOpened { peer: String, pq: &'static str, comp: &'static str }
  ```
* `overlay.session.closed`

  ```rust
  struct SessClosed { peer: String, reason: String, dur_ms: u64 }
  ```
* `overlay.ready.changed`

  ```rust
  struct ReadyChanged { ready: bool, degraded: bool, missing: Vec<String> }
  ```
* `overlay.security.auth_fail`

  ```rust
  struct AuthFail { reason: String, cap_id: Option<String> }
  ```
* `overlay.gossip.rx`, `overlay.gossip.tx` (low-cardinality summaries)

  ```rust
  struct GossipEvt { topic: String, bytes: u32, fanout: u8 }
  ```

### 5.2 Subscribed (inbound)

* `config.updated` → swap validated snapshot; emit `ready.changed`.
* `bus.shutdown` → graceful drain (see `CONCURRENCY.md`).

> **No Kernel Drift**: topic names are stable; payloads use `#[non_exhaustive]` structs to allow additive evolution.

---

## 6) Canonical Test Vectors

> Golden files live in `tests/vectors/` and are used by CI interop tests. The inline examples below are normative.

### 6.1 OAP/1 “hello” frame (header example)

For payload `b"hi"` with:

* `ver=1`
* `flags=0x0001` (`REQ`)
* `tenant_id=0`
* `corr_id=0x1122334455667788`

**Hex (big-endian):**

```
0000001d 01 0001 00000000000000000000000000000000 1122334455667788 6869
└len=0x1d┘ └v┘ └f┘ └────────────tenant_id (16B)────────────┘ └corr┘ └hi┘
```

### 6.2 Handshake (CBOR diagnostic)

Client `hello` (DAG-CBOR, deterministic):

```cbor-diag
{"features":{"comp":["zstd"],"pq":"off"},"kind":"hello","max_frame":1048576,"token":"macaroon:BASE64...","versions":[1]}
```

Server `hello_ack`:

```cbor-diag
{"chosen":{"comp":"zstd","pq":"off","ver":1},"kind":"hello_ack","limits":{"chunk":65536,"max_frame":1048576}}
```

### 6.3 Error on oversize

Frame with `payload.len() = 1_048_577` → receiver MUST reply:

```cbor-diag
{"code":"FrameTooLarge","kind":"error","msg":"payload exceeds 1 MiB"}
```

…and increment `rejected_total{reason="oversize"}`.

### 6.4 Capability example (macaroon)

```json
{
  "typ":"macaroon",
  "v":1,
  "caveats":["ttl=60s","allow=topic:overlay/feed/*"],
  "sig":"BASE64URL(...)"
}
```

> **Digest vectors** (content addressing via BLAKE3) are provided in `tests/vectors/digests.json` with `{"input":"…","b3":"b3:<hex32|64>"}`; SDKs MUST match exactly.

---

## 7) Error Taxonomy & Mapping

### 7.1 OAP/1 Codes (CBOR `error.code`)

| Code            | Meaning                            | Typical HTTP mapping      |
| --------------- | ---------------------------------- | ------------------------- |
| `BadVersion`    | Unsupported protocol/features      | `400 Bad Request`         |
| `FrameTooLarge` | Payload > 1 MiB post-decompress    | `413 Payload Too Large`   |
| `RatioCap`      | Decompression ratio exceeded (10×) | `400 Bad Request`         |
| `QuotaExceeded` | Per-peer/tenant quota exceeded     | `429 Too Many Requests`   |
| `NotReady`      | Readiness gate failed              | `503 Service Unavailable` |
| `Unauth`        | Missing/invalid capability         | `401/403`                 |
| `Timeout`       | Operation deadline exceeded        | `504 Gateway Timeout`     |

**Rules**

* Errors are **responses** (`RESP` flag) to `REQ` frames with same `corr_id` (if non-zero).
* **EVENT** frames (unsolicited) MUST NOT carry `RESP`.

---

## 8) Interop Guarantees

* **Versioning**

  * **OAP/1** is fixed; future changes use `ver=2+` and/or **new payload kinds**.
  * Payload schemas are **additively extensible**: unknown fields ignored; **never** repurpose keys.
* **Compression**

  * If `COMP` set, compression MUST be **Zstd** with standard dictionaryless stream. Receivers MUST enforce **ratio** and **absolute** caps.
* **Security**

  * Capability tokens are optional in `hello` but MUST be honored when present. A missing capability on subsequent REQ MUST yield `Unauth`.
* **Readiness**

  * When not ready, peers MUST **fail closed**: send `NotReady`, flip `/readyz` to 503.
* **Bus**

  * Topic names are stable; event payloads may gain optional fields over time.

---

## 9) Conformance Checklist (for SDKs & peers)

* [ ] Emit/parse OAP/1 envelope with **big-endian** fields; respect `max_frame=1 MiB`.
* [ ] Support **REQ/RESP/EVENT** flags; ignore unknown bits; enforce `COMP` → Zstd.
* [ ] Implement **hello/hello_ack** negotiation; down-negotiate **pq** and **comp** sanely.
* [ ] Propagate/echo `corr_id`; generate when absent.
* [ ] Enforce decompression **ratio ≤ 10×** and **absolute cap** post-inflate.
* [ ] Handle errors per §7 and mirror `corr_id`.
* [ ] Avoid high-cardinality labels/IDs on your metrics/logs.
* [ ] Pass golden vectors in `tests/vectors/`.

---

## 10) References

* `docs/specs/OAP-1.md` (wire format details, ABNF/BNF).
* `docs/Interop_Blueprint.md` (GMI-1.6 Omni-Gate).
* `OBSERVABILITY.md` (correlation IDs, metrics).
* `CONCURRENCY.md` (readiness, backpressure & single-writer invariant).
* `SECURITY.md` (capabilities, TLS/mTLS, PQ hybrid).

---

### Appendix A — Flag Bits (reference)

```
bit 0: REQ
bit 1: RESP
bit 2: EVENT
bit 3: COMP (payload is zstd)
bit 4: ACKREQ (sender requests ack)
bits 5..15: reserved (ignore if unset, drop if set unless negotiated)
```

### Appendix B — QUIC mapping (if enabled)

* One **bidirectional stream** per session; stream ID negotiated implicitly on accept.
* **Keepalive** via OAP `EVENT {kind:"keep"}` every ≤30s.
* Disable **0-RTT** by default; only enable with explicit feature + interop test.
* Enforce QUIC **anti-amplification** (3× rule) and path MTU discovery.

---

✅ With this spec, `svc-overlay`’s **wire contract is explicit**: envelope shape, negotiation, errors, and events are stable and testable. Independent SDKs can implement once and interoperate indefinitely without guessing.

```
```
