
---

# 🔗 INTEROP.md — ron-audit

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-audit`:

* **DTOs & schemas** (audit record, checkpoint, manifest).
* **On-disk formats** used by hosts (WAL segments, export manifests).
* **Canonicalization rules** (hashing, dedupe).
* **Bus events** the host service MUST publish when using `ron-audit`.
* **Canonical test vectors** so external implementations (SDKs/tools) can verify parity.

This keeps inter-crate and external integrations consistent with **GMI-1.6 Omni-Gate** (Omni-Gate mandates BLAKE3 addressing, append-only evidence, and host-surfaced observability).

---

## 1) Protocols & Endpoints

`ron-audit` is **lib-only** — it exposes no sockets or HTTP. All wire protocols are the **host’s** concern.

**Ingress Protocols (host examples):** HTTP/1.1 + TLS (tokio-rustls), gRPC, or an internal bus.
**Exposed Endpoints (host examples):** none defined here; hosts MAY expose `/metrics`, `/readyz`, `/healthz` and NOT audit-specific endpoints.

**Transport Invariants (host, if applicable):**

* Max ingress frame **MUST** obey host limits; audit emission is an internal API only.
* TLS endpoints (if any) **MUST** use `tokio_rustls::rustls::ServerConfig`.

> Interop for `ron-audit` is **data-level**, not network-level.

---

## 2) DTOs / Schemas

### 2.1 `AuditRecord` (re-exported from `ron-proto`)

**Role:** Canonical, tamper-evident audit event.

```rust
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRecord {
  pub v: u16,             // schema major
  pub ts_ms: u64,         // advisory wall-clock millis
  pub writer_id: String,  // stable identity: "svc-gateway@inst-123"
  pub seq: u64,           // strictly monotone per (writer_id, stream)
  pub stream: String,     // logical stream, e.g., "ingress", "policy"
  pub kind: AuditKind,    // enum (non_exhaustive recommended)
  pub actor: ActorRef,    // { cap_id?, key_fpr?, passport_id?, anon?:bool }
  pub subject: SubjectRef,// { content_id? "b3:<hex>", ledger_txid?, name? }
  pub reason: ReasonCode, // normalized taxonomy
  pub attrs: serde_json::Value, // sparse; <= 1 KiB canonicalized
  pub prev: String,       // previous record hash or "b3:0" for genesis
  pub self_hash: String,  // "b3:<hex>" = BLAKE3(canon(record_without_self_hash))
}
```

**Encoding:** JSON (UTF-8) with strict canonicalization (see §2.3).
**Versioning:** `v` is **major**; structural changes require a major bump. Additive details go into `attrs`.

#### 2.2 `Checkpoint` (export metadata; host signs)

```json
{
  "range": [12345, 15000],        // inclusive seq range within a stream segment
  "root": "b3:<hex>",             // Merkle root over the batch's canonical bytes
  "signer_key_id": "ops-hsm-1",   // identifier only
  "alg": "ed25519",               // tag only; signing done by host
  "sig": "<base64>"               // host-provided signature bytes
}
```

#### 2.3 `Manifest` (export index; one per export batch)

```json
{
  "version": 1,
  "stream": "ingress",
  "writer_id": "svc-gateway@inst-123",
  "segments": [
    {
      "file": "wal-000042.seg",
      "offset": 0,
      "count": 2048,
      "range": [13000, 15047],
      "root": "b3:<hex>"
    }
  ],
  "checkpoints": [ /* array of Checkpoint */ ],
  "created_ts_ms": 173###        // wall-clock at manifest creation
}
```

> **Note:** Signatures are **not** produced by `ron-audit`; hosts are responsible for signing checkpoints and managing keys.

### 2.3 Canonicalization Law (MUST)

Canonical form for hashing & dedupe:

* **Strings:** UTF-8 **NFC** normalization.
* **No floats:** any floating JSON value → **reject**.
* **Integer format:** base-10; no leading zeros.
* **Field order** (object keys) **exactly:**
  `v, ts_ms, writer_id, seq, stream, kind, actor, subject, reason, attrs, prev`
* **Whitespace:** none significant (minified is assumed).
* **Hashing:**

  * `canonical_bytes = canonicalize(record_without_self_hash)`
  * `self_hash = "b3:" + hex(BLAKE3_256(canonical_bytes))`
  * `dedupe_key = BLAKE3_256(canonical_bytes)` (raw 32-byte key; may be hex/base64 by tooling)

---

## 3) WAL & Export Formats (Host-Writable)

### 3.1 WAL Segment (`*.seg`) — append-only

**Header (fixed 32 bytes):**

```
magic:   0x52 0x4F 0x4E 0x2D 0x41 0x55 0x44 0x01   # "RON-AUD" v1
flags:   u16 (reserved=0)
count:   u32 (records in this segment)             # may be 0 during open
rsvd:    padding to 32 bytes
```

**Record framing (repeated):**

```
u32  len            # length of canonical JSON (record_without_self_hash)
u8   v              # redundant major for quick scan
u64  seq
[ len bytes canonical JSON ]
u32  self_hash_len  # ALWAYS 34 ("b3:" + 32 hex)? if hex encoded -> 66; choose one and fix
[ self_hash bytes ] # we recommend ASCII "b3:<64hex>"
```

**Constraints:**

* **Append-only**; never rewrite prior frames.
* `prev` of the next record MUST equal the prior record’s `self_hash`.
* Segment rotation: host-configurable (e.g., 128 MiB).

### 3.2 Manifest & Checkpoints

* **Manifest** references WAL files, ranges, and per-segment Merkle roots.
* **Checkpoint** is emitted at configured cadence (e.g., every 50k records) and **signed by the host**.
* **Verification tools** MUST be able to reconstruct the roots from WAL records (canonical bytes), detect any tampering, and validate signature authenticity.

---

## 4) Bus Topics (Host Emits / Subscribes)

`ron-audit` itself does not speak on the bus, but hosts MUST integrate:

### 4.1 Events Published (host)

* `audit.checkpoint.created` → `{ stream, writer_id, range, root, signer_key_id }`
* `audit.verify.failed` → `{ stream, writer_id, seq, reason ∈ { "hash_mismatch", "prev_mismatch" } }`
* `kernel.config.updated` (if audit config changed)

### 4.2 Events Subscribed (host)

* `kernel.shutdown` → triggers graceful drain & stop intake.
* `config.updated` → rebuild `AuditConfig` snapshot and swap atomically.

---

## 5) Canonical Test Vectors

> These vectors validate **canonicalization**, **hashing**, and **chaining** across independent implementations.
> Place the JSON and expected outputs under `/tests/vectors/audit/`.

### 5.1 Minimal Record (Genesis)

`record.genesis.json` (canonical, **without** `self_hash`):

```json
{"v":1,"ts_ms":1730246400000,"writer_id":"svc-gateway@inst-1","seq":1,"stream":"ingress","kind":"GetServed","actor":{"anon":true},"subject":{},"reason":"ok","attrs":{},"prev":"b3:0"}
```

Expected:

```
self_hash = b3:<HEX_OF_CANON_BYTES>
dedupe_key = <32-byte BLAKE3 of same bytes (binary/hex)>
```

### 5.2 Next Record (Chained)

`record.next.json` (no `self_hash`; `prev` must be the genesis `self_hash`):

```json
{"v":1,"ts_ms":1730246400100,"writer_id":"svc-gateway@inst-1","seq":2,"stream":"ingress","kind":"QuotaReject","actor":{"passport_id":"u:abcd"},"subject":{"content_id":"b3:1111"},"reason":"audit_backpressure","attrs":{"q":"work"},"prev":"<GENESIS_SELF_HASH>"}
```

Expected:

```
self_hash(next) = b3:<HEX>
verify_link(genesis, next) = Ok
```

### 5.3 Canonicalization “Evil” Inputs

Provide pairs of (input → canonicalized OR reject):

1. **NFD to NFC:** `"writer_id":"s\u0065\u0301rver"` → NFC normalized form → stable hash.
2. **Float reject:** `"seq":2.0` → **reject** (no floats).
3. **Key order drift:** keys shuffled → MUST re-order to fixed key order before hashing.
4. **Oversize attrs:** attrs serialized > 1 KiB → **reject** (`BoundsError::AttrsTooLarge`).

### 5.4 Merkle Root Checkpoint Vector

* Input: a batch of canonical bytes for seq `[100..199]`.
* Expected: `root = b3:<hex>` (computed by the reference tool).
* Include a tampered case (flip 1 byte in one record) → recomputed root MUST mismatch.

> We intentionally do **not** include signature bytes in vectors; signatures are host-specific. Provide **alg** and **signer_key_id** only.

---

## 6) Error Taxonomy (Interop-Visible)

* **Append/Emit (host facing):**

  * `Busy` — mpsc full or enqueue deadline exceeded.
  * `SizeExceeded` — attrs/record exceeds configured bounds (IDB [I-9]).
  * `Schema` — unknown fields / float encountered / non-NFC string.
* **Verification:**

  * `HashMismatch` — recomputed `self_hash` differs.
  * `PrevMismatch` — chain discontinuity (bad or missing `prev`).
* **Export/WAL (host tooling):**

  * I/O errors surfaced with context; `Timeout` counted in `io_timeouts_total{op}`.

---

## 7) Interop Guarantees

* **Canonicalization is Law:** Any change to field order, NFC policy, float policy, or hashing is a **major** schema change.
* **Append-Only:** WAL segments and export manifests are append-only; checkpoints summarize immutable ranges.
* **Dedupe Stability:** `dedupe_key` is stable across versions for identical canonical bytes.
* **Backward Compatibility:**

  * Structural additions go to `attrs`.
  * Unknown top-level fields are rejected (`deny_unknown_fields`).
* **No Kernel Drift:** `ron-audit` does not expose kernel/bus APIs; hosts must keep bus topics stable.
* **Auditability:** All vectors live under `/tests/vectors/audit/`; CI must re-compute to ensure parity.

---

## 8) References

* **GMI-1.6 Omni-Gate** — interop tenets: content addressing (BLAKE3), append-only evidence, host-exposed metrics.
* **IDB.md (ron-audit)** — invariants & acceptance gates.
* **CONFIG.md** — bounds, amnesia, WAL/export controls.
* **CONCURRENCY.md** — bounded queues, shed semantics.
* **OBSERVABILITY.md** — required metrics & readiness keys.

---

✅ With this INTEROP spec, independent implementations (other languages, ops tools, forensic verifiers) can produce **byte-for-byte identical canonical forms**, **hashes**, and **Merkle roots**, and can parse/validate **WAL & Manifest** formats without depending on internal crate code.
