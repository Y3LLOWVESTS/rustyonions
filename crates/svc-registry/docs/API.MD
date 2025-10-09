---

title: API Surface & SemVer Reference — svc-registry
status: draft
msrv: 1.80.0
last-updated: 2025-10-08
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md — svc-registry

## 0. Purpose

This document captures the **public API surface** for `svc-registry`:

* Snapshot of exported Rust symbols (should be minimal for a service crate).
* Full HTTP API (endpoints, request/response schemas, error model, auth).
* SemVer discipline and CI gates for surface changes.
* Alignment with the crate’s IDB, CONFIG, SECURITY, and OBSERVABILITY docs.

> Role reminder: `svc-registry` is the authoritative, **append-only**, **signed** registry of descriptors (nodes/services/regions). It verifies threshold signatures, enforces quorum, commits a tamper-evident chain, and emits non-blocking bus updates.

---

## 1. Public API Surface (Rust)

Service crates keep Rust surface tiny and stable; most functionality is HTTP + config.

Generate on CI (this command fails the build on unintended diffs):

```bash
cargo public-api -p svc-registry --simplified --deny-changes
```

### 1.1 Current Rust Surface (snapshot)

```text
# Root binary entrypoints (not a library API)
pub fn main()

# Re-exports for integration tests (documented, intentionally public)
pub mod config
pub mod observability
pub mod error

# Types intentionally public for embedding/testing (#[non_exhaustive] where applicable)
pub struct Config
pub enum Error           #[non_exhaustive]
pub struct Metrics       // handle cloneable across tasks
pub struct BuildInfo     // { name, version, commit, build_time }

# NOTE: No public commitment to internal storage, bus, or HTTP framework types.
# No public async functions—service API is HTTP, not Rust SDK.
```

> If we ever need a Rust SDK, it will live in a separate crate (`sdk-registry`) with its own API.md.

---

## 2. HTTP API (authoritative)

All endpoints return **JSON**. Errors follow a typed error schema. Durations are RFC3339/ISO8601-friendly where applicable. Body caps and compression caps apply (see CONFIG).

### 2.1 Authentication & Authorization

* **Capability tokens (macaroons)** in `Authorization: Bearer <token>` for write paths:

  * Scopes: `registry:propose`, `registry:approve`, `registry:commit`.
  * Short-lived; attenuable.
* Read paths may be public but **rate-limited**.
* UDS callers are additionally gated by `SO_PEERCRED` allow-list.

### 2.2 Common Headers

* `X-Corr-ID`: request correlation ID (ULID/UUID). Generated if absent and echoed in responses.
* `Deprecation`, `Sunset`: present when calling deprecated surfaces (see CONFIG).
* `Cache-Control: no-store` on write paths; reads may use `max-age` for immutable versions.

### 2.3 Endpoints

#### `GET /version` — build & schema info

**200 OK**

```json
{
  "service": "svc-registry",
  "version": "1.0.0",
  "commit": "abcdef1",
  "schema": { "registry": "1.0.0" },
  "deprecations": [
    { "surface": "/registry/stream", "since": "2025-09-01", "sunset": "2026-03-01", "notes": "Use SSE v2" }
  ]
}
```

---

#### `GET /healthz` — liveness

**200 OK** if process alive.

---

#### `GET /readyz` — readiness

**200 OK** when storage open, last HEAD validated, commit pipeline/bus ready.

**503 Service Unavailable**

```json
{
  "ready": false,
  "write_ready": false,
  "degraded": true,
  "missing": ["storage","bus"],
  "retry_after": 5
}
```

---

#### `GET /registry/head` — current head summary

**200 OK**

```json
{
  "version": 42,
  "payload_b3": "b3:7a0e...",
  "committed_at": "2025-10-08T18:12:00Z"
}
```

---

#### `GET /registry/{version}` — full descriptor set (immutable)

Path: `version: u64` (monotonic).

**200 OK**

```json
{
  "schema_version": "1.0.0",
  "version": 40,
  "items": [ /* descriptors from ron-proto DTOs */ ],
  "payload_b3": "b3:2f9a...",
  "approvals": [
    {"signer_id":"org:alpha#key1","algo":"ed25519","sig":"base64","signed_at":"2025-10-07T10:11:12Z"}
  ],
  "prev_hash": "b3:1aa3...",
  "committed_at": "2025-10-07T12:00:00Z"
}
```

**404 Not Found** if version does not exist.

---

#### `GET /registry/stream` — event stream (SSE)

Sends `text/event-stream` lines for committed updates. **Lossy** under lag; consumers must reconcile by calling `/registry/head` when gaps detected.

Event payload example:

```json
{ "version": 43, "payload_b3": "b3:...", "committed_at": "2025-10-08T18:30:01Z", "corr_id": "01J..." }
```

---

#### `POST /registry/proposals` — submit a proposal (write)

**Auth:** `registry:propose`

**Request**

```json
{
  "schema_version": "1.0.0",
  "payload": { "version": 42, "items": [/* descriptors */] },
  "payload_b3": "b3:7a0e..."  // optional; server recomputes and verifies
}
```

**202 Accepted**

```json
{ "proposal_id": "p_7f3ce2", "payload_b3": "b3:7a0e...", "expires_at": "2025-10-09T00:00:00Z" }
```

**429 Too Many Requests** when queue full (`Error::Busy`).

**400 Bad Request** for malformed/oversized request.

---

#### `POST /registry/approvals/{proposal_id}` — add an approval

**Auth:** `registry:approve`

**Request**

```json
{
  "signer_id": "org:alpha#key1",
  "algo": "ed25519",
  "sig": "base64(signature_over_payload_b3)",
  "signed_at": "2025-10-08T18:20:00Z"
}
```

**200 OK**

```json
{ "status": "accepted", "approvals": 2, "quorum": { "m": 2, "n": 3 } }
```

**409 Conflict** if signer revoked or duplicate approval (`err.code=DuplicateApproval`).

---

#### `POST /registry/commit/{proposal_id}` — finalize commit after quorum

**Auth:** `registry:commit`

Server verifies:

* Quorum **M-of-N** satisfied,
* Signer set & revocations current,
* `payload_b3` integrity,
* CAS on `HEAD{version,hash}` (single-writer).

**201 Created**

```json
{
  "version": 42,
  "payload_b3": "b3:7a0e...",
  "committed_at": "2025-10-08T18:22:11Z"
}
```

**409 Conflict** (`err.code=ChainMismatch`) if HEAD advanced concurrently → client should refetch and possibly re-propose.

**400/401/403** for invalid signatures or missing scopes.

---

#### `POST /registry/supersede` — administrative supersede (rare)

Used to replace current HEAD with a signed **supersede** record (see IDB). Requires governance capabilities and reason.

**Auth:** governance-scoped macaroon, multi-approver flow.

**Request**

```json
{
  "supersede_of": "b3:oldhead...",
  "supersede_reason": "KeyCompromise",
  "note": "Rotating compromised signer set"
}
```

**201 Created** with new HEAD summary, or **409** if `supersede_of` != current HEAD.

---

### 2.4 Error Model (typed)

All error responses use this envelope (HTTP status reflects category):

```json
{
  "error": {
    "code": "InvalidSig|QuorumFailed|RevokedKey|Busy|Timeout|Malformed|ChainMismatch|Unauthorized|Forbidden",
    "message": "human readable (stable-ish)",
    "corr_id": "01J...",
    "details": { }
  }
}
```

* `code` is stable for dashboards and automation.
* `message` is for humans; avoid parsing it.
* `details` may carry structured hints (e.g., which signer was revoked).

---

## 3. DTOs (schemas)

DTOs are defined in `ron-proto` and referenced here. The **wire schema** is **non-breaking additive** within a major series:

* Top-level envelope: `SignedArtifact { schema_version, payload, payload_b3, approvals[], prev_hash, committed_at }`
* `DescriptorSet { version: u64, items: Vec<Descriptor>, prev_hash?, created_at, expiry? }`
* `Approval { signer_id, algo, sig, signed_at }`

**Schema rules**

* `deny_unknown_fields` on all decoders.
* Additive-only within a major version; removals/renames require a major bump.
* Include `schema_version` in every payload.

---

## 4. SemVer Discipline

### 4.1 Service (HTTP) surface

* **Non-breaking (minor):**

  * Adding new endpoints.
  * Adding **optional** request/response fields.
  * Adding new enum values **when marked** `#[non_exhaustive]` in docs (clients must ignore unknown).
  * Introducing new headers that are optional (`Warning`, `Sunset`, etc.).

* **Breaking (major):**

  * Removing/renaming endpoints or fields.
  * Changing types/meaning of existing fields.
  * Tightening validation that rejects previously-valid inputs (unless behind an opt-in flag through a deprecation window).
  * Changing error `code` strings.

* **Patch:**

  * Doc clarifications, perf improvements, additional metrics, stricter **server-side** defaults that do not cause request rejections.

> **Deprecations:** minimum **2 minor releases** **or** **180 days** window, announced via `/version`, `Deprecation`/`Sunset` headers, and CHANGELOG.

### 4.2 Rust surface

* This crate’s Rust surface is intentionally small. Any new public types must be `#[non_exhaustive]` where reasonable.
* **No** public export of internal HTTP/storage/bus types.

---

## 5. Stability Guarantees

* **MSRV:** `1.80.0`.
* **Unsafe:** forbidden unless justified with a doc comment + tests.
* **Content hashes:** `payload_b3` remains stable for the same payload bytes.
* **Idempotency:** `POST /registry/commit/{proposal_id}` is idempotent—retries return the same committed version if already committed.

---

## 6. Tooling

* `cargo public-api` — detects Rust symbol diffs.
* `cargo semver-checks` (optional) — semantic checks for crate API.
* Contract tests for HTTP surface:

  * `tests/http_contract.rs` spins the service and asserts endpoint existence, headers, and error codes.
* API snapshots stored under: `/docs/api-history/svc-registry/{version}.txt`.

---

## 7. CI & Gates

* **Gate A:** `cargo public-api -p svc-registry --deny-changes` (service should have no or few public symbols).
* **Gate B:** `tests/http_contract.rs` must pass (endpoints & headers).
* **Gate C:** CHANGELOG entry required whenever HTTP surface changes.
* **Gate D:** Deprecation policy enforced (check `/version` emits the deprecation table; CI asserts presence when PR marks deprecations).

Example GitHub Actions step:

```yaml
- name: Public API surface
  run: cargo public-api -p svc-registry --simplified --deny-changes

- name: HTTP contract tests
  run: cargo test -p svc-registry --test http_contract -- --nocapture
```

---

## 8. Acceptance Checklist (DoD)

* [ ] Current **Rust** API snapshot generated & stored in `/docs/api-history/...`.
* [ ] HTTP endpoints documented here match `tests/http_contract.rs`.
* [ ] SemVer review performed; changes classified (patch/minor/major).
* [ ] Deprecations (if any) have `Deprecation`/`Sunset` headers and appear in `/version`.
* [ ] CI gates (`cargo public-api`, contract tests) are green.
* [ ] CHANGELOG updated with behavior and surface changes.
* [ ] OpenAPI document updated (below) and committed under `/docs/openapi/registry.yaml`.

---

## 9. OpenAPI (authoritative excerpt)

> Full spec lives at `/docs/openapi/registry.yaml`. Keep excerpt in sync.

```yaml
openapi: 3.0.3
info:
  title: svc-registry
  version: "1.0.0"
servers:
  - url: https://registry.local
components:
  securitySchemes:
    macaroon:
      type: http
      scheme: bearer
  schemas:
    Error:
      type: object
      required: [error]
      properties:
        error:
          type: object
          required: [code, message, corr_id]
          properties:
            code:
              type: string
              enum: [InvalidSig, QuorumFailed, RevokedKey, Busy, Timeout, Malformed, ChainMismatch, Unauthorized, Forbidden]
            message: { type: string }
            corr_id: { type: string }
            details: { type: object, additionalProperties: true }
paths:
  /registry/head:
    get:
      summary: Current HEAD
      responses:
        "200":
          description: OK
        "503":
          $ref: "#/components/schemas/Error"
  /registry/{version}:
    get:
      parameters:
        - in: path
          name: version
          required: true
          schema: { type: integer, format: int64, minimum: 0 }
      responses:
        "200": { description: OK }
        "404": { $ref: "#/components/schemas/Error" }
  /registry/proposals:
    post:
      security: [{ macaroon: [] }]
      responses:
        "202": { description: Accepted }
        "400": { $ref: "#/components/schemas/Error" }
        "401": { $ref: "#/components/schemas/Error" }
        "429": { $ref: "#/components/schemas/Error" }
  /registry/approvals/{proposal_id}:
    post:
      security: [{ macaroon: [] }]
      parameters:
        - in: path
          name: proposal_id
          required: true
          schema: { type: string }
      responses:
        "200": { description: OK }
        "401": { $ref: "#/components/schemas/Error" }
        "409": { $ref: "#/components/schemas/Error" }
  /registry/commit/{proposal_id}:
    post:
      security: [{ macaroon: [] }]
      parameters:
        - in: path
          name: proposal_id
          required: true
          schema: { type: string }
      responses:
        "201": { description: Created }
        "400": { $ref: "#/components/schemas/Error" }
        "401": { $ref: "#/components/schemas/Error" }
        "409": { $ref: "#/components/schemas/Error" }
```

---

## 10. Examples (consumer-friendly)

### 10.1 Read the head

```bash
curl -sS http://localhost:8080/registry/head \
  -H 'X-Corr-ID: 01JBC4XYA8...' | jq .
```

### 10.2 Propose → Approve → Commit

```bash
# 1) Propose
curl -sS -X POST http://localhost:8080/registry/proposals \
  -H 'Authorization: Bearer eyJ...macaroon' \
  -H 'Content-Type: application/json' \
  -d @proposal.json

# -> { "proposal_id": "p_7f3ce2", "payload_b3": "b3:..." }

# 2) Approve (repeat for each signer)
curl -sS -X POST http://localhost:8080/registry/approvals/p_7f3ce2 \
  -H 'Authorization: Bearer eyJ...macaroon' \
  -H 'Content-Type: application/json' \
  -d @approval-signer1.json

# 3) Commit
curl -sS -X POST http://localhost:8080/registry/commit/p_7f3ce2 \
  -H 'Authorization: Bearer eyJ...macaroon'
```

### 10.3 SSE stream

```bash
curl -N http://localhost:8080/registry/stream
```

Handle gaps by calling `/registry/head` and diffing your local state.

---

## 11. History (examples of future entries)

* **1.1.0** — Added `POST /registry/supersede` (governance-only); non-breaking.
* **1.0.0** — Initial GA: `head`, `version`, `stream`, propose/approve/commit; error envelope standardized.

---

### Notes to reviewers

* **Service-first**: treat HTTP surface as the canonical API. Rust surface is incidental and intentionally small.
* **Invariant alignment**: every change should be checked against IDB invariants (MUSTs), especially multi-sig, CAS on HEAD, non-blocking bus, DTO hygiene, and deprecation windows.
* **CI**: keep `/docs/openapi/registry.yaml` and this document in sync; PRs that alter endpoints must update both and the contract tests.
