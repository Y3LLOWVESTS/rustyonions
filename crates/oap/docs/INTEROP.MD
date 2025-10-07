---

title: INTEROP — oap
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: developers, auditors, external SDK authors
----------------------------------------------------

# 🔗 INTEROP.md — `oap`

## 0) Purpose

Define the **interop surface** for the OAP/1 framed protocol as implemented by the `oap` crate:

* Wire protocol & envelope grammar (HELLO/START/DATA/ACK/END/ERROR).
* Header fields & constraints (what is normative vs. host-defined).
* DTO boundaries (what lives in `ron-proto`, not here).
* Canonical interop **test vectors** and error taxonomy.
* Expectations for **polyglot SDKs** (Rust/TS/Python) and service embeddings.

> `oap` is **framing + validation only**. Sockets/TLS/auth/DTOs live outside this crate. This file keeps different languages and services in lock-step.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols (this crate)

* **OAP/1 framed** over an arbitrary transport (typically TCP/TLS via `ron-transport`).
* `oap` itself exposes **no HTTP/gRPC endpoints**. Those belong to embedding services (e.g., `svc-gateway`, `svc-overlay`).

### 1.2 Envelope Grammar (normative)

* **HELLO** `{ version: u32, max_frame: u64, pq_hybrid: bool }`
* **START** `{ stream: String }`
* **DATA** `{ obj: Option<String>, seq: u64, len: u32 }` + `<len bytes>`
* **ACK** `{ up_to: u64 }`
* **END** `{ seq: u64, status: u16 }`
* **ERROR** `{ code: u16, reason: String }`

### 1.3 Transport Invariants (normative)

* **Frame cap** `MAX_FRAME_BYTES = 1_048_576` (1 MiB).
* **Streaming chunk size** = **64 KiB** is a **host I/O knob** (does **not** change the 1 MiB frame cap).
* **TLS** is **host-owned** (tokio-rustls), not part of `oap`.

### 1.4 Version & Feature Negotiation

* OAP **version** carried in `HELLO.version`. For OAP/1, `version == 1`.
* **Feature bits** (e.g., `pq_hybrid`) in `HELLO` are **opaque** capability flags. Unknown flags **must not** alter parsing/semantics.

> Interop rule: peers MUST agree on version; feature bits are purely advisory for lower layers (e.g., transport/crypto).

---

## 2) DTOs / Schemas

### 2.1 What’s **not** here

* Application DTOs (manifests, tickets, ledger proofs) are **not defined** by `oap`. They live in `ron-proto` (strictly versioned schemas).

### 2.2 Header conventions (normative to `oap`)

* **`obj` header (DATA):** when the DATA bytes represent a content-addressed object, include `obj: "b3:<hex>"` (BLAKE3-256).

  * `oap` **requires** the header presence (when applicable) but does **not** hash; verification occurs at materialization points (storage/overlay).
* Duplicate/unknown headers are **rejected** when `strict_headers=true` (default).

### 2.3 Correlation identifiers

* Correlation, tenant, tracing IDs are **host fields** (HTTP headers, bus metadata). `oap` does not carry tracing fields in its envelope.

---

## 3) Bus Topics

`oap` publishes no bus events. For **hosts** embedding `oap`, recommended topics:

### 3.1 Events Published (host examples)

* `gateway.oap.reject` → `{ reason, peer, corr_id }`
* `gateway.oap.frame` → `{ kind, bytes, seq, ack_up_to }` (sampled)
* `overlay.object.stored` → (storage-layer DTO; outside `oap`)

### 3.2 Events Subscribed (host examples)

* `config.updated` → rebuild `ParserConfig`/`WriterConfig` and swap.
* `bus.shutdown` → drain/abort according to host policy.

---

## 4) Canonical Test Vectors (polyglot)

> These vectors are **language-agnostic** and stored under `/tests/vectors/oap1/`.
> All SDKs (Rust/TS/Python) must round-trip these **byte-identically** and/or **struct-identically** as noted.

### 4.1 HELLO ↔ HELLO (negotiate)

**Input (struct):**

```json
{ "HELLO": { "version": 1, "max_frame": 1048576, "pq_hybrid": false } }
```

**Expected:** encode→decode yields the same struct. Peers that answer with `version != 1` → `BadHello`.

### 4.2 START → DATA → ACK → END (happy path)

1. `START { "stream": "objects/v1" }`
2. `DATA  { "obj": "b3:<hex-OBJECT>", "seq": 1, "len": 3 } + "abc"`
3. `ACK   { "up_to": 1 }`
4. `END   { "seq": 1, "status": 0 }`
   **Expected:**

* Parser emits `Frame(DATA)` with body bytes `0x61 0x62 0x63`.
* Sequence algebra: `ACK.up_to` must be **≥** last delivered `seq`.

### 4.3 Oversize frame (reject)

**Input:** `DATA { len: 1_048_577 } + <garbage>`
**Expected error:** `FrameOversize` (reject without allocation).
**Metric label:** `oap_rejects_total{reason="frame_oversize"}`.

### 4.4 Missing `obj` when object bytes present (reject)

**Input:** `DATA { obj: null, seq: 7, len: 1024 } + <looks_like_object>`
**Expected error:** `ObjRequiredMissing`.
**Metric label:** `oap_rejects_total{reason="missing_obj"}`.

### 4.5 Unknown envelope tag (reject)

**Input:** `TAG=0x99 ...`
**Expected error:** `UnknownEnvelope`.
**Metric label:** `oap_rejects_total{reason="unknown_envelope"}`.

### 4.6 Truncated frame (need more)

**Input:** header indicates `len=8192`, only `4096` bytes provided.
**Expected:** `Progress::NeedMore` (no partial allocation, no state corruption). Feeding the remaining bytes must produce **the same** frame as if sent in one buffer.

### 4.7 ACK credit violation (reject)

**Input sequence:** deliver `DATA(seq=10)` then receive `ACK(up_to=9)`.
**Expected error:** `CreditViolation`.
**Note:** algebraic check only; hosts still own backpressure timing.

> Each vector includes a Rust `serde_json` representation and a canonical **hex** fixture for cross-language tests. TS/Python SDKs must pass the same fixtures.

---

## 5) Error Taxonomy (wire-facing)

| Code / Variant       | Condition (normative)                                      | Retry? | Metric reason label |
| -------------------- | ---------------------------------------------------------- | ------ | ------------------- |
| `BadHello`           | Unknown/unsupported version, malformed HELLO               | no     | `bad_hello`         |
| `FrameOversize`      | `len > 1 MiB`                                              | no     | `frame_oversize`    |
| `UnknownEnvelope`    | Tag not in {HELLO, START, DATA, ACK, END, ERROR}           | no     | `unknown_envelope`  |
| `HeaderMalformed`    | Missing/duplicate/invalid header fields                    | no     | `header_malformed`  |
| `ObjRequiredMissing` | DATA bytes represent object but `obj` header absent        | no     | `missing_obj`       |
| `CreditViolation`    | `ACK.up_to` < last delivered seq or otherwise inconsistent | no     | `credit_violation`  |
| `Truncated`          | Buffer ended mid-frame                                     | maybe  | `truncated`         |

> The **metric labels are canonical** across languages and hosts.

---

## 6) Interop Guarantees

* **Version Lock (OAP/1):** `MAX_FRAME_BYTES = 1 MiB`; envelope set is fixed; enums **non_exhaustive** for additive growth.
* **Polyglot Parity:** Rust/TS/Python codecs **must** round-trip canonical vectors; CI runs a **matrix** job to enforce this.
* **Strictness by Default:** `strict_headers=true` rejects unknown/duplicate headers in production builds. Interop labs may loosen to diagnose.
* **Unknown Feature Bits:** `HELLO` feature flags are **opaque**. Unknown bits are preserved/ignored without changing OAP/1 behavior.
* **No Kernel Drift:** `oap` never emits/consumes kernel/bus messages directly; hosts map outcomes to their bus.

**Backward compatibility rules**

* **Additive only** between minor versions (new error variants, new optional fields in `#[non_exhaustive]` enums).
* Any change that alters **length rules**, **envelope tags**, or **semantics** is **major**.

**Auditability**

* Vectors live in `/tests/vectors/oap1/` and are embedded in docs via golden tests.
* A failure in any language binding blocks release.

---

## 7) References

* **OAP/1 IDB** — invariants, acceptance gates, anti-scope.
* **CONFIG.md** — host vs. library knobs; strict headers; ack window hints.
* **CONCURRENCY.md** — split-read guarantees; cancel-safety; backpressure neutrality.
* **OBSERVABILITY.md** — canonical metric names/labels and readiness inputs.
* **Interop Blueprint (GMI-1.6)** — project-wide interop rules and DTO location (`ron-proto`).
* **HARDENING Blueprint** — fuzz/property corpus disciplines.

---

## 8) Appendix — Wire Notes (non-normative encoding guidance)

`oap` does not fix a specific on-the-wire integer endianness here; the canonical fixtures (hex) and parser define it precisely for SDK authors. Implementations **must** pass the provided fixtures byte-for-byte. When in doubt, **trust the fixtures** over prose.

---

✅ With this INTEROP.md, every embedding and SDK can implement OAP/1 **once** and prove correctness via shared vectors, preventing drift across languages and services.
