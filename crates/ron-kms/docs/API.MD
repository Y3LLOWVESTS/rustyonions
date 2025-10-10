
---

# 📖 API.md — ron-kms

---

title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-10-09
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md

## 0. Purpose

This document captures the **public API surface** of `ron-kms`:

* Snapshot of exported Rust symbols (traits, types, modules) and **canonical HTTP surface**.
* SemVer discipline (what breaks vs. extends).
* Alignment with CHANGELOG.md (behavioral vs. surface changes).
* CI-enforceable via `cargo public-api` and semver gates.
* Acts as the “spec” for external consumers (services, SDKs, auditors).

---

## 1. Public API Surface

### 1.1 Rust surface (crate exports)

> Generated with:
>
> ```
> cargo public-api --simplified --deny-changes -p ron-kms
> ```

**Current intended surface** (developers: keep this in sync with `cargo public-api` output):

```text
pub mod api          // DTOs, errors, auth scopes, serde types
pub mod service      // server bootstrap & router wiring (feature "bin")
pub mod backend      // KeyStore/Signer traits + adapters (re-exports only)

pub use backend::{KeyStore, Signer, Attestation};
pub use api::{
  Kid, KidRef, KidSelector, Alg, KeyPurpose, SignPolicy, VerifyPolicy,
  SignRequest, SignResponse, VerifyRequest, VerifyResponse,
  WrapRequest, WrapResponse, UnwrapRequest, UnwrapResponse,
  CreateKeyRequest, KeyMaterialPublic, KeyMeta, RotateResponse,
  ErrorEnvelope, ErrorCode
};
```

**Traits** (semantic contracts):

```rust
pub trait KeyStore {
    fn create(&self, req: CreateKeyRequest) -> Result<KeyMeta, ErrorEnvelope>;
    fn rotate(&self, kid: Kid) -> Result<KeyMeta, ErrorEnvelope>;
    fn get(&self, kid: KidRef) -> Result<KeyMeta, ErrorEnvelope>;
    fn attest(&self, kid: KidRef) -> Result<Option<Attestation>, ErrorEnvelope>;
}

pub trait Signer {
    fn sign(
        &self,
        kid_or_set: KidSelector,
        msg: &[u8],
        policy: Option<SignPolicy>
    ) -> Result<SignResponse, ErrorEnvelope>;

    fn verify(
        &self,
        msg: &[u8],
        bundle: &SignResponse,
        policy: Option<VerifyPolicy>
    ) -> Result<VerifyResponse, ErrorEnvelope>;
}
```

> **Stability:** `backend::*` traits are **public** on purpose to allow adapters (`pkcs11`, `tpm`, `file`, `cloud`). We gate breaking edits with SemVer rules (§2).

---

### 1.2 Canonical HTTP API (service interface)

* **Base**: `/v1/kms`
* **Auth**: Bearer macaroon with scopes (`kms:read`, `kms:sign`, `kms:verify`, `kms:rotate`, `kms:wrap`)
* **Transport**: HTTP/JSON (canonical). gRPC (optional feature) mirrors DTOs 1:1 (no extra fields).

#### Endpoints

| Method | Path                 | Scope(s)      | Description                                                                   |
| -----: | -------------------- | ------------- | ----------------------------------------------------------------------------- |
|   POST | `/sign`              | `kms:sign`    | Sign payload (may emit hybrid sigs)                                           |
|   POST | `/verify`            | `kms:verify`* | Verify signatures (reads only; may succeed during backend outage using cache) |
|   POST | `/wrap`              | `kms:wrap`    | Wrap an ephemeral/data key (hybrid KEM)                                       |
|   POST | `/unwrap`            | `kms:unwrap`  | Unwrap a wrapped key                                                          |
|   POST | `/keys`              | `kms:rotate`  | Create a new key (returns `kid#v1`)                                           |
|   POST | `/keys/{kid}/rotate` | `kms:rotate`  | Create a new **version** for an existing key                                  |
|    GET | `/keys/{kid}`        | `kms:read`    | Get **public** material, algorithm, versions, attestation class               |

* `kms:verify` is recommended; if not provided, some deployments allow verify with `kms:read`—check policy.

---

## 2. Request/Response Schemas (canonical JSON)

> **General rules**: All DTOs use `deny_unknown_fields`. Sizes capped (e.g., body ≤ 1 MiB). Payloads are **not** sent raw; callers send `payload_b3 = BLAKE3(canonical_form)`.

### 2.1 Common types

```jsonc
// Kid = "<tenant>/<purpose>/<alg>/<uuid>#vN"
"kid": "core/audit/ed25519/0b2e...#v3"

// Key selector (exact kid or a set)
"kid_or_set": {
  "kid": "core/audit/ed25519/0b2e...#v3"
  // or
  // "kid_set": ["...#v3", "...#v2"]
}

// Algorithms
"alg": "ed25519" | "ml-dsa" | "x25519" | "ml-kem"

// Policies (server may ignore local fields and consult ron-policy)
"sign_policy": { "produce_hybrid": true }
"verify_policy": { "mode": "or|and|pq_only" }
```

### 2.2 POST `/sign`

**Request**

```json
{
  "kid_or_set": { "kid": "core/audit/ed25519/0b2e...#v3" },
  "payload_b3": "7f3c... (hex)",
  "policy": { "produce_hybrid": true },
  "op_id": "8f9a2c34-..." // idempotency (optional but strongly encouraged)
}
```

**Response**

```json
{
  "sigs": [
    { "alg": "ed25519", "signature_b64": "..." },
    { "alg": "ml-dsa",  "signature_b64": "..." } // present if hybrid
  ],
  "kid": "core/audit/ed25519/0b2e...#v3",
  "attestation": { "class": "pkcs11-yubihsm2", "evidence": "..." } // optional
}
```

**Errors** (ErrorEnvelope, §2.7): `NoCapability`, `NotFound`, `StoreUnavailable`, `AttestationMissing`, `Backpressure`, `BadRequest`.

### 2.3 POST `/verify`

**Request**

```json
{
  "payload_b3": "7f3c... (hex)",
  "sigs": [
    { "alg": "ed25519", "signature_b64": "..." },
    { "alg": "ml-dsa",  "signature_b64": "..." }
  ],
  "policy": { "mode": "and" }
}
```

**Response**

```json
{
  "ok": true,
  "details": {
    "checked": ["ed25519","ml-dsa"],
    "satisfied_mode": "and"
  },
  "cache_age_ms": 742 // present if using cached pubkeys
}
```

### 2.4 POST `/wrap` and `/unwrap`

**Wrap Request**

```json
{
  "kid": "core/kem/x25519/3c8b...#v2",
  "plaintext_key_b64": "...",      // small symmetric key to wrap (or use "size": 32 for KEM-generated)
  "kem_mode": "hybrid"              // default
}
```

**Wrap Response**

```json
{
  "wrapped_b64": "...",            // AEAD+KEM envelope
  "alg": "hybrid(x25519+ml-kem)",
  "kid": "core/kem/x25519/3c8b...#v2"
}
```

**Unwrap Request**

```json
{
  "kid": "core/kem/x25519/3c8b...#v2",
  "wrapped_b64": "..."
}
```

**Unwrap Response**

```json
{
  "plaintext_key_b64": "...",
  "alg": "hybrid(x25519+ml-kem)",
  "kid": "core/kem/x25519/3c8b...#v2"
}
```

### 2.5 POST `/keys` (create)

**Request**

```json
{
  "tenant": "core",
  "purpose": "audit",
  "alg": "ed25519",                 // or "ml-dsa", "hybrid" (implies dual tracks)
  "policy_hints": { "rotate_days": 30 } // opaque to KMS; forwarded to policy
}
```

**Response**

```json
{
  "kid": "core/audit/ed25519/bf1a...#v1",
  "public": { "alg": "ed25519", "pubkey_b64": "..." }
}
```

### 2.6 POST `/keys/{kid}/rotate`

**Response**

```json
{
  "kid": "core/audit/ed25519/bf1a...#v2",
  "previous": "core/audit/ed25519/bf1a...#v1"
}
```

### 2.7 GET `/keys/{kid}`

**Response**

```json
{
  "kid": "core/audit/ed25519/bf1a...#v3",
  "alg": "ed25519",
  "versions": ["#v1","#v2","#v3"],
  "public": { "pubkey_b64": "..." },
  "attestation_class": "pkcs11-yubihsm2" // optional
}
```

---

## 3. Error Envelope (stable)

All non-2xx responses use a **stable** envelope:

```json
{
  "error": {
    "code": "NoCapability" | "NotFound" | "BadRequest" | "StoreUnavailable" |
            "AttestationMissing" | "Backpressure" | "Timeout" | "Conflict" |
            "TooLarge" | "Unsupported" | "Internal",
    "message": "human-readable, redacted",
    "retryable": true,
    "hint": "attach macaroon with kms:sign",
    "ts": "2025-10-09T20:33:11Z",
    "request_id": "r-abc123"
  }
}
```

**Semantics**

* `StoreUnavailable` → sign path returns **503** + `Retry-After`.
* `AttestationMissing` (when required by config/policy) → **403**.
* `Backpressure` → **429**.
* `TooLarge` / `Unsupported` → **4xx**.
* `Internal` → **500** (rare; audited).

---

## 4. OpenAPI (excerpt)

> A full spec lives at `/docs/openapi/ron-kms.yaml`. This excerpt shows the canonical schemas/names.

```yaml
openapi: 3.0.3
info:
  title: ron-kms
  version: 1.0.0
servers:
  - url: /v1/kms
paths:
  /sign:
    post:
      operationId: sign
      security: [{ bearerAuth: [] }]
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/SignRequest" }
      responses:
        "200": { description: OK, content: { application/json: { schema: { $ref: "#/components/schemas/SignResponse" }}}}
        "429": { $ref: "#/components/responses/Backpressure" }
        "503": { $ref: "#/components/responses/StoreUnavailable" }
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
  schemas:
    SignRequest:
      type: object
      additionalProperties: false
      required: [kid_or_set, payload_b3]
      properties:
        kid_or_set: { oneOf: [ { $ref: "#/components/schemas/Kid" }, { type: object, properties: { kid_set: { type: array, items: { $ref: "#/components/schemas/Kid" }}}}]}
        payload_b3: { type: string, pattern: "^[0-9a-f]+$" }
        policy: { $ref: "#/components/schemas/SignPolicy" }
        op_id: { type: string, format: uuid }
    SignResponse:
      type: object
      additionalProperties: false
      required: [sigs, kid]
      properties:
        sigs:
          type: array
          items:
            type: object
            required: [alg, signature_b64]
            properties:
              alg: { $ref: "#/components/schemas/Alg" }
              signature_b64: { type: string }
        kid: { $ref: "#/components/schemas/Kid" }
        attestation:
          type: object
          additionalProperties: true
    Alg:
      type: string
      enum: [ed25519, ml-dsa, x25519, ml-kem]
    Kid:
      type: string
      pattern: "^[^/]+/[^/]+/[^/]+/[a-f0-9-]+#v[0-9]+$"
  responses:
    Backpressure: { description: Queue full; retry later }
    StoreUnavailable: { description: Signing store unavailable; retry later }
```

---

## 5. SemVer Discipline

### 5.1 Additive (Minor / Non-Breaking)

* Adding **new** endpoints (e.g., `/attest`) or optional response fields.
* Adding algorithms to the **allow-list** while keeping default behavior unchanged.
* Adding enum variants when types are `#[non_exhaustive]`.
* Adding headers/metadata that clients can ignore.

### 5.2 Breaking (Major)

* Removing/renaming public Rust exports (`api::…`, `backend::…`), or changing trait method signatures.
* Changing endpoint paths, required fields, or error codes.
* Making previously `#[non_exhaustive]` enums exhaustive.
* Changing DTO field meaning or default behavior (e.g., changing default verify mode).

### 5.3 Patch-Level

* Doc fixes, log message tweaks, dependency bumps that don’t alter API.
* Performance improvements with identical inputs/outputs.
* Tightening validation **that already matched the documented contract**.

---

## 6. Stability Guarantees

* **MSRV**: `1.80.0`.
* **DTOs** use `deny_unknown_fields`; clients must not rely on unspecified behavior.
* **KID semantics**: verification remains **backward compatible across versions** (`v1..vN`).
* **Unsafe code**: forbidden unless justified and reviewed.
* **No internal types leak** (e.g., no `hyper::Request` in public APIs).
* **gRPC (optional)** mirrors JSON DTOs **1:1** (no gRPC-only fields).

---

## 7. Invariants (API-level)

* **Payloads** are identified by `payload_b3` (BLAKE3 hash of canonical form). Raw payloads are never transmitted to KMS.
* **Hybrid PQ**: verify modes `or|and|pq_only` are controlled by policy/config; server may include hints in responses but never leaks secrets.
* **Idempotent mutations** accept `op_id` and return the same result on replay.
* **Error envelope** shape is stable; codes are additive (new codes may appear but existing ones don’t change meaning).

---

## 8. Tooling

* `cargo public-api` → detects Rust surface diffs.
* `cargo semver-checks` (optional) → compile-time SemVer validation.
* `cargo doc` + doctests for examples.
* API snapshots stored in `docs/api-history/ron-kms/{version}.txt`.

**Makefile targets (suggested)**

```bash
make api-snapshot   # runs cargo public-api and writes docs/api-history/...
make api-verify     # runs public-api --deny-changes
```

---

## 9. CI & Gates

* PR pipeline runs:

  * `cargo public-api --deny-changes -p ron-kms`
  * (optional) `cargo semver-checks check -p ron-kms`
* Fail CI if any **breaking** changes are unacknowledged.
* Bot posts the symbol diff as a PR comment.
* If a surface diff exists, a **CHANGELOG.md** entry is **required** (CI enforces).

---

## 10. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & stored under `docs/api-history/`.
* [ ] SemVer review performed; if breaking, **major** bump planned and documented.
* [ ] CI gate passes (`cargo public-api`).
* [ ] CHANGELOG updated for any surface changes (endpoints/DTOs/errors).
* [ ] OpenAPI updated (`/docs/openapi/ron-kms.yaml`) and rendered.
* [ ] Examples/tests updated (SDK snippets).

---

## 11. History (notable shifts)

* **1.0.0**: Canonical `/v1/kms` surface established; hybrid PQ support with `verify_mode` and `produce_hybrid`; ErrorEnvelope stabilized.
* **1.1.0 (planned)**: Optional `/attest` read-only endpoint for device evidence export (non-breaking).
* **2.0.0 (reserved)**: Any change to trait signatures or endpoint compatibility (none planned).

---

### Appendix A — Error Codes (quick ref)

| Code                 | HTTP | Notes                                    |
| -------------------- | ---: | ---------------------------------------- |
| `NoCapability`       |  403 | Missing/insufficient scope               |
| `NotFound`           |  404 | Unknown KID/version                      |
| `BadRequest`         |  400 | DTO validation, size caps                |
| `StoreUnavailable`   |  503 | Backend/HSM down; includes `Retry-After` |
| `AttestationMissing` |  403 | Required attestation absent              |
| `Backpressure`       |  429 | Queue full / RPS limit                   |
| `Timeout`            |  504 | Backend op deadline exceeded             |
| `TooLarge`           |  413 | Body over limit                          |
| `Unsupported`        |  400 | Disallowed algorithm/mode                |
| `Conflict`           |  409 | Idempotency clash / rotate race          |
| `Internal`           |  500 | Unexpected; audited                      |

---

### Appendix B — Headers

* **Request**

  * `Authorization: Bearer <macaroon>`
  * `X-Corr-ID: <uuidv7>` (optional; server generates if absent)
* **Response**

  * `Retry-After: <seconds>` on `503`
  * `X-KMS-Cache-Age: <ms>` on verify via cache (duplicate of JSON `cache_age_ms`)

---

### Appendix C — gRPC Notes (optional feature)

* Service `ron.kms.v1.Kms` with RPCs: `Sign`, `Verify`, `Wrap`, `Unwrap`, `CreateKey`, `Rotate`, `GetKey`.
* Protos live under `proto/ron/kms/v1/`.
* Messages mirror JSON DTOs **field-for-field**. No gRPC-only fields allowed.

---

✅ With this API spec:

* External consumers know exactly which endpoints/DTOs to use (and **never** send raw payloads).
* Contributors have a CI-enforced snapshot to prevent accidental breakage.
* Auditors can compare implementation vs. this contract and the OpenAPI file for compliance.
