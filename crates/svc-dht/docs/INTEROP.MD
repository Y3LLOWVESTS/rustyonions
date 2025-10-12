````markdown
# 🔗 INTEROP — svc-dht

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-dht`:

- Wire protocols & message formats (DHT over `ron-transport`: TLS/TCP or QUIC; optional Tor/relay).
- DTOs & schemas (lookup requests/responses, provider records).
- Bus topics and events (what we publish/subscribe).
- Canonical **test vectors** and invariants.

This keeps integrations consistent with **GMI-1.6 Omni-Gate** and prevents drift across crates and SDKs.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

- **DHT peer protocol** over **`ron-transport`**:
  - **QUIC/TLS 1.3** (preferred) or **TLS 1.3/TCP**.
  - Optional **Tor/relay** hop via transport adapter.
  - **ALPN**:
    - `ron/kad-cbor` (default binary CBOR)
    - `ron/kad-msgpack` (optional MsgPack)
- **Operational HTTP**:
  - `GET /metrics` (OpenMetrics)
  - `GET /healthz` (liveness)
  - `GET /readyz` (readiness)
  - `GET /version` (build info)

### 1.2 Exposed DHT Operations (OpCodes)

| OpCode        | Name         | Direction     | Purpose                                |
|---------------|--------------|---------------|----------------------------------------|
| `0x01`        | FIND_NODE    | bi-directional| Return closest `k` nodes to target key |
| `0x02`        | FIND_VALUE   | bi-directional| Return value (provider) or closest `k` |
| `0x03`        | PROVIDE      | inbound       | Submit signed provider record          |
| `0x04` (opt)  | STORE        | inbound       | Store KV (feature-gated)               |

### 1.3 Transport Invariants

- **Max frame**: **1 MiB** (hard cap)  
- **Default RPC timeout**: **1500 ms** (per probe); hedge after **250 ms**  
- **Hops SLO**: p99 **≤ 5** hops (budget enforced)  
- **Streaming chunk** (if used): 64 KiB  
- **TLS**: `tokio_rustls::rustls::ServerConfig` only; TLS 1.3 minimum

---

## 2) DTOs / Schemas

Encoding is **binary CBOR** (canonical, deterministic). MsgPack is allowed when ALPN negotiates `ron/kad-msgpack`. Unknown fields **must be ignored** by readers (forward-compatible).

> All messages share a **common envelope** and an **op-specific payload**.

### 2.1 Common Envelope (binary; shown as Rust for clarity)

```rust
#[non_exhaustive]
struct Envelope<'a> {
  proto_ver: u16,          // = 1
  opcode: u8,              // 0x01..0x04
  corr_id: u64,            // correlation / tracing
  ts: u64,                 // unix seconds (sender clock)
  hops_seen: u8,           // monotonically non-decreasing
  flags: u16,              // bitfield: REQ=0x1, RESP=0x2, HEDGE=0x4
  payload: &'a [u8],       // CBOR of one of the op-specific DTOs below
}
````

### 2.2 Node & Routing DTOs

```rust
/// 256-bit node id (BLAKE3 keyed or transport-native id, canonicalized as bytes)
struct NodeId([u8; 32]);

struct NodeInfo {
  id: NodeId,
  asn: u32,                // max 32-bit
  addrs: Vec<MultiAddr>,   // e.g., "quic://host:port", "tcp://host:port"
  last_seen: u64,          // unix sec
}
```

### 2.3 Lookup DTOs

```rust
struct FindNodeReq { target_key: [u8; 32] }       // BLAKE3 digest
struct FindNodeResp { closest: Vec<NodeInfo> }    // len ≤ k

struct FindValueReq { content_key: [u8; 32] }
struct FindValueResp {
  value: Option<ProviderRecord>,
  closest: Option<Vec<NodeInfo>>                  // present if not found
}
```

### 2.4 ProviderRecord (signed, expiring)

```rust
/// Canonical provider record; signatures cover the canonical CBOR of {key,publisher,addrs,ttl,ts}
struct ProviderRecord {
  proto_ver: u16,          // = 1
  key: [u8; 32],           // BLAKE3 content id (b3)
  publisher: NodeId,       // publisher node id
  addrs: Vec<MultiAddr>,   // how to reach the publisher/provider
  ttl: u32,                // seconds (max enforced by policy)
  ts: u64,                 // unix seconds (issuer clock)
  sigs: Vec<Sig>           // one or more signatures (see below)
}

enum SigAlg { Ed25519, MLDSA87 }   // ML-DSA-87 optional during PQ migration

struct Sig {
  alg: SigAlg,
  pk: Vec<u8>,               // public key bytes; format depends on alg
  sig: Vec<u8>               // signature over canonical record
}
```

**Signature policy (migration)**

* During PQ migration, **dual-sign** is recommended (Ed25519 + ML-DSA-87).
* After `pq.require_pq_on`, classical-only records may warn/reject per policy.

---

## 3) Bus Topics

### 3.1 Published Events

| Topic                       | Payload (serde JSON)                                  | Notes                                     |
| --------------------------- | ----------------------------------------------------- | ----------------------------------------- |
| `svc-dht.lookup.done`       | `{ corr_id, op, hops, latency_ms, success, target? }` | Audit/metrics fanout                      |
| `svc-dht.provider.accepted` | `{ key:"b3:<…>", publisher, ttl }`                    | On PROVIDE accept                         |
| `svc-dht.provider.rejected` | `{ key:"b3:<…>", reason }`                            | reason∈{bad_sig,stale,ttl_exceeded,quota} |
| `svc-dht.bootstrap.state`   | `{ seeds_up, required, ready_bucket_fill_pct }`       | Drives `/readyz`                          |
| `kernel.health`             | `KernelEvent::Health { service:"svc-dht", ok:bool }`  | Canonical health                          |
| `kernel.crash`              | `KernelEvent::ServiceCrashed { service, reason }`     | Crash loop signal                         |

### 3.2 Subscribed Events

| Topic            | Action                                    |
| ---------------- | ----------------------------------------- |
| `config.updated` | Swap config snapshot (atomic); log diff   |
| `bus.shutdown`   | Begin graceful drain (see CONCURRENCY.md) |

> **Inter-crate contract:** event names are **stable**; payloads are additive (unknown fields ignored).

---

## 4) Canonical Test Vectors

Test vectors live in `tests/vectors/svc-dht/` and are consumed by SDKs and conformance tests.

### 4.1 BLAKE3 Content ID

* **Input**: UTF-8 bytes `"hello world"`
* **Output (truncated)**: `b3:9f64a7…`
* **Notes**: Authoritative full hex is stored in `vectors/b3-helloworld.txt`.
* **Verifier**: `cargo test -p svc-dht --test vectors::b3_roundtrip`

### 4.2 FIND_NODE Round-Trip (CBOR, proto_ver=1)

* **Request (CBOR hex, truncated):**

  ```
  a5 69 70 72 6f 74 6f 5f 76 65 72 01 66 6f 70 63 6f 64 65 01
  67 63 6f 72 72 5f 69 64 1a de ad be ef 62 74 73 1a 67 2b 2e 80
  69 68 6f 70 73 5f 73 65 65 6e 00 66 66 6c 61 67 73 00
  67 70 61 79 6c 6f 61 64 58 23 <CBOR of FindNodeReq>
  ```
* **Response**: `FindNodeResp.closest` contains **≤ k** `NodeInfo` entries; sample under `vectors/find_node_resp.cbor`.

### 4.3 PROVIDE Verification

* **Record**: `ProviderRecord` with `key = b3:<hello world>`, `ttl=86400`, `ts=1731264000`
* **Sigs**:

  * `Ed25519`: `pk`/`sig` files under `vectors/ed25519/`
  * `ML-DSA-87` (optional): under `vectors/mldsa87/`
* **Expected**: `accepted=true` in response; on altered `addrs` → `accepted=false`, `reason="bad_sig"`.

### 4.4 Frame Limits

* **Input**: payload size 1,048,577 bytes (1 MiB + 1)
* **Expected**: `413 FrameTooLarge` (see §5), connection remains usable thereafter.

> All vectors have both **CBOR** and **MsgPack** encodings when applicable.

---

## 5) Error Taxonomy (Wire / HTTP)

### 5.1 Wire Errors (u16 code → reason)

| Code | Reason          | When                              |
| ---: | --------------- | --------------------------------- |
| 1000 | `Ok`            | Success                           |
| 1400 | `BadVersion`    | `proto_ver` unsupported           |
| 1413 | `FrameTooLarge` | > 1 MiB frame                     |
| 1429 | `QuotaExceeded` | RPS/inflight/semaphore exhausted  |
| 1440 | `BadSignature`  | Record signatures invalid         |
| 1441 | `StaleRecord`   | `ts`/`ttl` invalid                |
| 1450 | `NotReady`      | Bootstrap/readiness not satisfied |
| 1508 | `Timeout`       | RPC timed out                     |
| 1501 | `Busy`          | Backpressure shed                 |

> **Mapping**: Wire errors map to `DhtError` variants and to metrics `rejected_total{reason}`.

### 5.2 HTTP Errors

* `503 NotReady` on `/readyz` with JSON `{ degraded:true, missing:[…], retry_after }`
* `/metrics` and `/healthz` always `200` on handler success (liveness).

---

## 6) Interop Guarantees

* **Proto stability**: `proto_ver=1` baseline. Unknown fields are **ignored**. Incompatible changes bump **major** and are feature/flag-gated before default.
* **SemVer discipline**: Schema-breaking changes require **major** crate version and deprecation window.
* **No Kernel drift**: Kernel bus topics and health/crash events are frozen; payloads only extend additively.
* **Auditability**: All canonical vectors checked in under `tests/vectors/` and exercised in CI.

---

## 7) Conformance Checklist (for SDKs/peers)

* [ ] Negotiate ALPN (`ron/kad-cbor` or `ron/kad-msgpack`).
* [ ] Send `Envelope{ proto_ver=1, corr_id≠0 }`; ignore unknown fields on receive.
* [ ] Enforce **1 MiB** frame cap; close just the stream on violation, not the whole conn.
* [ ] Honor **hop budget**; refuse to forward if budget exceeded.
* [ ] Verify **ProviderRecord** signatures (Ed25519; ML-DSA-87 when present).
* [ ] Treat policy tightenings (PQ require) via advertised flags/handshake fields when present; otherwise infer from errors.
* [ ] Respect backpressure (`Busy`) and retry with jitter; do not spin.
* [ ] Carry correlation (`corr_id`) end-to-end for tracing.

---

## 8) References

* **Interop Blueprint GMI-1.6** — `docs/Interop_Blueprint.md`
* **OBSERVABILITY.md** — correlation ids, metric contracts
* **CONCURRENCY.md** — readiness & shutdown semantics
* **SECURITY.md** — signatures, PQ migration, quotas

---

## 9) Appendix — Field & Encoding Rules

* **CBOR**: use canonical ordering; shortest form integers; byte strings for digests/keys.
* **MsgPack**: no extension types; binary for digests/keys.
* **MultiAddr**: ASCII; scheme in `{tcp,quic,tor}`; authority `host:port`.
* **BLAKE3**: `b3:<hex-64>` (truncate only for display; on-wire is 32-byte binary).

---

✅ With this INTEROP spec, `svc-dht` exposes a clear, versioned **wire contract**, stable **bus events**, and **canonical vectors** that any SDK or peer can use to implement and verify compatibility—without guesswork or drift.

```
```
