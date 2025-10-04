```markdown
---
title: 🔗 INTEROP — ron-auth
audience: developers, auditors, external SDK authors
msrv: 1.80.0
status: draft
last-updated: 2025-10-04
---

# INTEROP.md — ron-auth

## 0) Purpose

Define the **interop surface** of `ron-auth`, the capability **verification/attenuation** library:

- **Wire format** for capability tokens (deterministic CBOR → Base64URL).
- **Algorithm contract** for MAC chaining (keyed BLAKE3).
- **DTOs & schemas** (Capability, Scope, Caveat, RequestCtx).
- **Header/field conventions** for HTTP/gRPC/OAP1 carriers.
- **Canonical test vector** layout and rules.
- **Trait contracts** (`MacKeyProvider`, optional `SigAdapter`) for cross-crate integration.

This document ensures consistent behavior across services (`svc-gateway`, `svc-overlay`, `svc-index`, `svc-storage`, `svc-mailbox`), `svc-passport` (issuance), `ron-kms` (key custody), and any external SDKs—aligned with Omni-Gate principles.

---

## 1) Protocols & Endpoints

`ron-auth` itself exposes **no network endpoints**. Tokens travel **within** other protocols as bearer credentials.

### 1.1 Carrier Conventions

- **HTTP/1.1+ / HTTP/2 / gRPC**  
  Prefer `Authorization` header with a registered auth-scheme:

```

Authorization: Capability <base64url-token>

````

Fallback (when intermediaries interfere with `Authorization`):  
`X-RON-Capability: <base64url-token>`

- **OAP/1 (framed)**  
Include capability as a metadata field in the envelope payload:
```cbor
{ "cap": <bstr base64url bytes as text or raw bstr>, ... }
````

(See §2 for the token itself.)

* **Message size**
  The **token** (decoded) must be ≤ **4096 bytes** (§Invariants). Carriers must apply their own frame limits separately.

### 1.2 Transport Invariants (host services)

* TLS termination and readiness gates belong to **host services** (not `ron-auth`).
* Services **must not** rely on central introspection for token validity—verification is **offline** using this library.

---

## 2) DTOs / Schemas (Wire-Level)

### 2.1 Capability (CBOR → Base64URL)

**Media type (recommended):** `application/ron-cap+cbOR;v=1`
**On the wire:** Base64URL **without padding** of the canonical CBOR serialization of the structure below.

CBOR logical model (Rust-ish for clarity):

```rust
struct Capability {
  v:   u8,              // version = 1
  tid: String,          // tenant identifier (ASCII, [-._a-zA-Z0-9], 1..64)
  kid: String,          // key id (ASCII, [-._a-zA-Z0-9], 1..64)
  r:   Scope,           // root scope
  c:   Vec<Caveat>,     // ordered, conjunctive caveats
  s:   [u8; 32],        // MAC tag (final chain)
}
```

**Deterministic CBOR rules (canonical):**

* **Maps** use text keys exactly `"v","tid","kid","r","c","s"`.
* **Definite** lengths only; **shortest** integer encoding; **no** floating-point.
* Byte strings (`s`) are length-32 `bstr`.
* Strings are UTF-8; no alternate forms.
* No additional/unknown top-level fields.

> **Compatibility:** Unknown **caveat** variants are handled via `Custom` (see below), not by adding top-level fields.

### 2.2 Scope (CBOR map)

```rust
struct Scope {
  prefix:    Option<String>,   // resource prefix (e.g., "/o/b3:…")
  methods:   Vec<String>,      // e.g., ["GET","PUT"]
  max_bytes: Option<u64>,      // per-request cap
}
```

### 2.3 Caveats (CBOR tagged union as map: `{"t": <str>, "v": <any>}`)

Canonical tag strings and value shapes:

| `t`                   | Value `v`                                   | Notes                            |
| --------------------- | ------------------------------------------- | -------------------------------- |
| `"exp"`               | `u64` (unix seconds)                        | deny if now > exp (± skew)       |
| `"nbf"`               | `u64`                                       | deny if now < nbf (± skew)       |
| `"aud"`               | `tstr`                                      | exact match against audience     |
| `"method"`            | `[tstr]`                                    | subset of verbs/ops              |
| `"path_prefix"`       | `tstr`                                      | normalized path prefix           |
| `"ip_cidr"`           | `tstr`                                      | CIDR in text form                |
| `"bytes_le"`          | `u64`                                       | max allowed body size            |
| `"rate"`              | `{ "per_s": u32, "burst": u32 }`            | host-enforced rate               |
| `"tenant"`            | `tstr`                                      | must == `tid`                    |
| `"amnesia"`           | `bool`                                      | require host amnesia mode        |
| `"gov_policy_digest"` | `tstr` (hex, 64 chars)                      | bind to governance policy digest |
| `"custom"`            | `{ "ns": tstr, "name": tstr, "cbor": any }` | namespaced extension             |

> **Extensibility:** New standard caveats MAY be added in v1.x without breaking wire compatibility. Unknown **standard** tags should cause **deny** unless specifically allowed by config (default is deny).

### 2.4 Request Context (host → library, not on the wire)

```rust
struct RequestCtx<'a> {
  now_unix_s: u64,
  method: &'a str,
  path: &'a str,
  peer_ip: Option<IpAddr>,
  object_addr: Option<&'a str>,
  tenant: &'a str,
  amnesia: bool,
  policy_digest_hex: Option<&'a str>,
  extras: serde_cbor::Value,
}
```

`RequestCtx` is **not** serialized; it is an input to verification.

---

## 3) Algorithms (Normative)

### 3.1 MAC Primitive

* **BLAKE3** keyed mode (`KDF`/MAC) with the symmetric key referenced by `kid` (per `tid`).
* **Constant-time** equality compare for final tag.

### 3.2 Domain Separation & Chain

Let `DS_INIT = "ron-auth/v1\0init"` and `DS_CAV = "ron-auth/v1\0caveat"` (ASCII bytes).

1. **Initial link:**

```
sig_0 = BLAKE3_key( key, DS_INIT || tid || kid || canonical_cbor(r) )
```

2. **For each caveat `c_i` in order (i=0..n-1):**

```
sig_{i+1} = BLAKE3_key( key, DS_CAV || sig_i || canonical_cbor(c_i) )
```

3. **Final tag:** `s = sig_n` (32 bytes).

> **Notes:**
>
> * No random nonce is required in v1 (the chain is bound to content and order).
> * Reordering caveats **invalidates** the token.
> * Any change to `r` or any `c_i` produces a different `s`.

### 3.3 Verification

* Decode Base64URL → CBOR (canonical, strict).
* Enforce **bounds**: size ≤ 4096 B, caveats ≤ 64.
* Resolve `(tid,kid) → key handle` via `MacKeyProvider`.
* Recompute chain; constant-time compare with `s`.
* Evaluate caveats with **conjunctive** semantics (all must pass).
* Fail-closed on: parse errors, bounds, unknown KID, unknown caveat where not explicitly allowed, context failure (amnesia/policy digest).

---

## 4) Traits (Integration Contracts)

### 4.1 `MacKeyProvider` (required)

```rust
trait MacKeyProvider {
  fn mac_handle(&self, tenant: &str, kid: &str) -> Result<Box<dyn MacHandle>, AuthError>;
}

trait MacHandle: Send + Sync {
  fn mac(&self, msg: &[u8]) -> [u8; 32]; // keyed BLAKE3
}
```

* Implemented by **`ron-kms`** or service-local adapters.
* **Never** expose raw key material to `ron-auth`.

### 4.2 `SigAdapter` (optional, `pq-hybrid`)

For cross-org **signature envelopes** (avoid MAC key sharing):

```rust
trait SigAdapter: Send + Sync {
  fn verify_hybrid(&self, payload: &[u8], sig: &[u8], kid: &str) -> Result<(), AuthError>;
}
```

This verifies an outer **envelope** over `payload = canonical_cbor(Capability without 's')`.
Default build excludes this.

---

## 5) Canonical Test Vectors

Vectors live under: `crates/ron-auth/tests/vectors/` and are versioned by **wire version**.

### 5.1 File Layout

```
tests/vectors/
  v1/
    readme.md                # provenance and generation instructions
    capability_roundtrip.json
    mac_chain.json
    deny_cases.json
    interop_suite.csv        # tabular summary for SDKs
```

### 5.2 Vector Schema (examples)

* **`capability_roundtrip.json`**

  ```json
  {
    "name": "allow_get_prefix",
    "cap_cbor_hex": "a66576... (canonical CBOR hex)",
    "cap_b64url": "p2V2... (no padding)",
    "tid": "tenant-1",
    "kid": "kid-2025-10",
    "scope": { "prefix": "/o/b3:abcd", "methods": ["GET"], "max_bytes": 1048576 },
    "caveats": [
      { "t": "exp", "v": 1767225600 },
      { "t": "method", "v": ["GET"] },
      { "t": "path_prefix", "v": "/o/b3:abcd" }
    ],
    "mac_hex": "b3b3...32bytes",
    "ctx": {
      "now_unix_s": 1767225599,
      "method": "GET",
      "path": "/o/b3:abcd/some",
      "tenant": "tenant-1"
    },
    "expect": "allow"
  }
  ```
* **`deny_cases.json`**: table of `{name, reason, token, ctx}` using canonical **deny reason strings** (`"parse.b64"`, `"kid.unknown"`, `"caveat.exp"`, …).
* **`mac_chain.json`**: lists intermediate `sig_i` values for SDK implementers.

### 5.3 Generation & Validation

* Vectors are generated by rust tests and **re-validated** in CI.
* External SDKs must reproduce `s` and decisions exactly for the same inputs.
* Any change to vectors → **major** if it alters v1 semantics.

---

## 6) Error / Deny Taxonomy (Interop-Stable)

The library exposes a **non-exhaustive** `DenyReason` enum plus **stable** strings via `as_str()` that SDKs/log pipelines must use:

```
parse.b64 | parse.cbor | parse.bounds | schema.unknown_field
mac.mismatch | kid.unknown | tenant.mismatch
caveat.exp | caveat.nbf | caveat.aud | caveat.method
caveat.path | caveat.ip | caveat.bytes | caveat.rate
caveat.tenant | caveat.amnesia | caveat.policy_digest
caveat.custom.unknown | caveat.custom.failed
```

> These strings are part of the **observability contract** (see `OBSERVABILITY.md`).
> Adding **new** reason strings is **minor**; renaming existing ones is **major**.

---

## 7) Interop Guarantees

* **Wire stability:** CBOR map keys (`"v","tid","kid","r","c","s"`), canonical encoding rules, and MAC chain domain strings are **stable within v1**.
* **Extensibility:** `Caveat` is open via `"custom"` and new standard tags; unknown standard tags → **deny** unless explicitly allowed.
* **Offline verification:** No network calls; `MacKeyProvider` is the only dependency.
* **Tenant/KID binding:** Decisions are **namespaced** by `(tid,kid)`; cross-tenant acceptance is impossible unless a host implements explicit cross-trust.
* **SemVer discipline:** Any change that affects wire encoding, domain constants, or evaluation semantics is **major**.
* **Auditability:** Vectors and their generator source are checked in; CI enforces determinism.

---

## 8) Cross-Crate Touchpoints

* **`svc-passport` (issuance/rotation):**

  * Mints tokens conforming to §2 and computes `s` using §3.
  * Publishes active `KID` windows; removal immediately invalidates tokens with that KID.
* **`ron-kms` (keys):**

  * Implements `MacKeyProvider` returning opaque `MacHandle`s.
  * Enforces rotation policy and tenant isolation.
* **Consumers (`svc-gateway`, `svc-*`):**

  * Extract token from header (HTTP) or metadata (OAP/1/gRPC).
  * Construct `RequestCtx` from the live request and call `verify()`.

---

## 9) References

* `docs/IDB.md` — invariants & proofs
* `docs/CONFIG.md` — verifier config & bounds
* `docs/SECURITY.md` — threat model & handling
* `docs/OBSERVABILITY.md` — deny reason strings & metrics
* Omni-Gate Interop principles (GMI-1.6)

---

✅ With this spec, `ron-auth` remains **portable and exact**: any SDK can implement the same CBOR + MAC chain + caveat evaluation and achieve bit-for-bit compatibility with Rust services—without calling back to a central authority.

```
```
