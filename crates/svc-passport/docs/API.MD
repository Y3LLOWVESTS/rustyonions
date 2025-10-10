````markdown
---
title: API Surface & SemVer Reference — svc-passport
status: draft
msrv: 1.80.0
last-updated: 2025-10-09
audience: contributors, auditors, API consumers
---

# API.md — svc-passport

## 0. Purpose

This document captures the **public API surface** of `svc-passport`:

- Snapshot of exported **Rust** symbols (if any) and the **HTTP surface**.
- SemVer discipline for both **Rust** and **HTTP** (path/DTO/versioning).
- CHANGELOG alignment and CI gates (`cargo public-api`, OpenAPI diff).
- “Spec you can build to” for integrators, with examples and error model.

> `svc-passport` is primarily a **service crate** (bin-first). The authoritative integration surface is **HTTP**; the **Rust** surface is intentionally minimal.

---

## 1. Public API Surface

### 1.1 Rust surface (crate `svc-passport`)

`svc-passport` ships as a binary. A tiny `lib.rs` may exist for testing, but **no stable Rust API is guaranteed** to external users. CI still snapshots any public items to prevent accidental exposure.

Generate via:

```bash
cargo public-api -p svc-passport --simplified --deny-changes
````

#### Current Surface (expected)

```text
# Binary crate: no stable public Rust items intended for external use.
# If present (internal-use only, guarded by #[doc(hidden)]):
pub fn run() -> impl Future<Output = anyhow::Result<()>>   # NOT STABLE
pub mod metrics                                            # NOT STABLE
```

> If any new public items appear, CI will fail until they are **documented here** and acknowledged in CHANGELOG.

---

### 1.2 HTTP surface (stable contract)

Base URL: internal cluster via `svc-gateway`
Versioned path prefix: **/v1**
Content-Type: `application/json; charset=utf-8`
Authn: caller must be **trusted upstream** (e.g., via gateway policy/MTLS).
Rate/size: governed by config (RPS, inflight, body cap, ratio cap).

#### Endpoints

1. **POST /v1/passport/issue** — mint a capability token

* **Request**

  ```json
  {
    "subject_ref": "opaque-user-or-service-handle",
    "audience": "svc-mailbox",
    "ttl_s": 900,
    "caveats": [
      "svc=svc-mailbox",
      "route=/mailbox/send",
      "budget.bytes=1048576",
      "rate.rps=5"
    ],
    "accept_algs": ["ed25519+ml-dsa","ed25519"],
    "proof": null
  }
  ```
* **Response 200**

  ```json
  {
    "token": "base64url(...)",       // macaroon-style envelope (opaque)
    "kid": "issuer-v1",
    "alg": "ed25519+ml-dsa",
    "exp": "2025-10-09T14:30:00Z",
    "caveats": ["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"]
  }
  ```
* **Common 4xx**

  * `400` `{"reason":"ttl_too_long"}` (exceeds policy)
  * `400` `{"reason":"unknown_caveat"}` (not in registry)
  * `400` `{"reason":"no_acceptable_alg"}` (PQ negotiation)
  * `413` `{"reason":"over_limit"}` (body cap)
  * `429` `{"reason":"busy","retry_after":2}`
* **5xx**

  * `504` `{"reason":"kms_timeout"}`
  * `503` `{"reason":"degraded"}` (readiness shed)

2. **POST /v1/passport/verify** — non-authoritative preflight decode/echo

* **Request**

  ```json
  {"token":"base64url(...)"}
  ```
* **Response 200**

  ```json
  {
    "ok": true,
    "parsed": {
      "alg": "ed25519+ml-dsa",
      "kid": "issuer-v1",
      "epoch": 42,
      "aud": "svc-mailbox",
      "sub": "opaque-subject",
      "exp": "2025-10-09T14:30:00Z",
      "caveats": ["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"]
    }
  }
  ```

> **Note:** Authoritative verification is in **`ron-auth`**. This endpoint is a diagnostic/preflight convenience for integrators.

3. **POST /v1/passport/revoke** — revoke via epoch/key policy

* **Request**

  ```json
  {"epoch": 43, "reason": "compromise"}  // or {"kid":"issuer-v2", "reason":"rotation"}
  ```
* **Response 200**

  ```json
  {"current_epoch": 43}
  ```

4. **GET /healthz** — liveness
5. **GET /readyz** — readiness (issuance may fail-closed while verify preflight stays OK)
6. **GET /metrics** — Prometheus text

#### Error envelope (all 4xx/5xx)

```json
{
  "reason": "ttl_too_long | unknown_caveat | no_acceptable_alg | kms_timeout | degraded | busy | over_limit | ratio_cap",
  "message": "human-friendly diagnostic",
  "corr_id": "ULID"
}
```

#### Headers

* `X-Corr-ID`: correlation id (ingest or generate)
* `Retry-After`: on `429` or `503` (seconds)
* `Cache-Control: no-store`

---

## 2. SemVer Discipline

We treat **Rust** and **HTTP** surfaces distinctly.

### 2.1 Rust (binary-first)

* **Breaking (Major)**: expose/remove/rename public items; change signatures/types/traits.
* **Additive (Minor)**: add new `pub` items marked `#[non_exhaustive]` where applicable.
* **Patch**: impl/behavioral changes without public surface change.

> Default stance: **no stable Rust API**. If exporting helpers for tests, hide under `#[doc(hidden)]` and feature flags.

### 2.2 HTTP (path + DTOs)

* **Path versioning**: `/v1/...` is the stable contract for DTOs and semantics.
* **Additive (Non-breaking)**:

  * Add **optional** request fields with defaults.
  * Add response fields (consumers must ignore unknown fields).
  * Add new caveat families in registry (issuance still clamps to policy).
* **Breaking (Requires `/v2`)**:

  * Remove or rename fields without aliases.
  * Change field semantics or types incompatibly.
  * Tighten defaults in a way that rejects previously valid conformant requests (outside policy config).
* **Policy-driven changes**:

  * Changing **default alg** set, rotation cadence, or PQ rollout **does not** change HTTP version if DTOs unchanged; must be announced via `svc-registry` policy and CHANGELOG.

---

## 3. Stability Guarantees

* **MSRV**: 1.80.0.
* DTOs: `#[serde(deny_unknown_fields)]` for requests; responses may gain fields (consumers must ignore unknowns).
* Unsafe code: forbidden unless explicitly justified and reviewed.
* Tokens are **opaque** strings; envelope structure may evolve behind the stable contract fields (`alg`, `kid`, `exp`, `caveats` advertised).
* **No** durable PII fields exist; `subject_ref` is opaque and caller-provided.

---

## 4. Invariants (API-aligned)

* Issued artifacts are **capability tokens** with **short TTLs**; max TTL enforced by policy.
* Keys live in **`ron-kms`**; this service never exposes key material.
* PQ-hybrid negotiation via `accept_algs`; explicit `no_acceptable_alg` on mismatch (no silent downgrade).
* Revocation via **epoch bump**; propagation SLO ≤ 5s p99 is observable (metrics).

---

## 5. Tooling

### 5.1 Rust surface

```bash
cargo public-api -p svc-passport --simplified --deny-changes
```

Artifacts:

* Store snapshots in `docs/api-history/svc-passport/<version>.txt`.

### 5.2 HTTP surface

* `docs/openapi/svc-passport.v1.yaml` generated & linted.

**Generate (for maintainers using oas tooling):**

```bash
npx @redocly/cli lint docs/openapi/svc-passport.v1.yaml
```

**Diff on PRs:**

```bash
npx openapi-diff docs/openapi/svc-passport.v1.yaml origin/main:docs/openapi/svc-passport.v1.yaml
```

---

## 6. CI & Gates

* **cargo-public-api**: fails PR if Rust surface changes without doc/CHANGELOG updates.
* **OpenAPI lint/diff**: PR fails if spec drifts versus main.
* **Docs-sync**: DTO structs ↔ OpenAPI schema checked by script (field parity).
* **SemVer checks** (optional): `cargo semver-checks` job for library surfaces.

---

## 7. Acceptance Checklist (Definition of Done)

* [ ] Current Rust API snapshot generated & committed under `docs/api-history/...`.
* [ ] `docs/openapi/svc-passport.v1.yaml` updated; lints pass.
* [ ] CHANGELOG updated for any surface-affecting change.
* [ ] Integration examples (`curl`) updated.
* [ ] CI gates green (`cargo public-api`, OpenAPI diff).
* [ ] For breaking HTTP changes, a **/v2** path is introduced with migration notes.

---

## 8. OpenAPI (excerpt, normative for integrators)

```yaml
openapi: 3.0.3
info:
  title: svc-passport API
  version: "1.0.0"
servers:
  - url: https://gateway.internal
paths:
  /v1/passport/issue:
    post:
      summary: Mint a capability token
      operationId: issuePassport
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [subject_ref, audience, ttl_s]
              additionalProperties: false
              properties:
                subject_ref: { type: string, minLength: 1 }
                audience: { type: string, pattern: "^svc-[a-z0-9-]+$" }
                ttl_s: { type: integer, minimum: 1 }
                caveats:
                  type: array
                  items: { type: string }
                accept_algs:
                  type: array
                  items: { type: string, enum: [ed25519, ed25519+ml-dsa] }
                proof:
                  nullable: true
      responses:
        "200":
          description: Capability minted
          content:
            application/json:
              schema:
                type: object
                required: [token, kid, alg, exp]
                additionalProperties: false
                properties:
                  token: { type: string }
                  kid: { type: string }
                  alg: { type: string }
                  exp: { type: string, format: date-time }
                  caveats:
                    type: array
                    items: { type: string }
        "400":
          $ref: "#/components/responses/BadRequest"
        "413":
          $ref: "#/components/responses/OverLimit"
        "429":
          $ref: "#/components/responses/Busy"
        "503":
          $ref: "#/components/responses/Degraded"
        "504":
          $ref: "#/components/responses/UpstreamTimeout"

  /v1/passport/verify:
    post:
      summary: Preflight decode/echo (non-authoritative)
      operationId: verifyPassport
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [token]
              additionalProperties: false
              properties:
                token: { type: string }
      responses:
        "200":
          description: Parsed view
          content:
            application/json:
              schema:
                type: object
                required: [ok, parsed]
                properties:
                  ok: { type: boolean }
                  parsed:
                    type: object
                    properties:
                      alg: { type: string }
                      kid: { type: string }
                      epoch: { type: integer }
                      aud: { type: string }
                      sub: { type: string }
                      exp: { type: string, format: date-time }
                      caveats:
                        type: array
                        items: { type: string }
        "400":
          $ref: "#/components/responses/BadRequest"

  /v1/passport/revoke:
    post:
      summary: Revoke via epoch/key policy
      operationId: revokePassport
      requestBody:
        required: true
        content:
          application/json:
            schema:
              oneOf:
                - type: object
                  required: [epoch]
                  properties:
                    epoch: { type: integer, minimum: 0 }
                    reason: { type: string }
                - type: object
                  required: [kid]
                  properties:
                    kid: { type: string }
                    reason: { type: string }
      responses:
        "200":
          description: Current epoch after update
          content:
            application/json:
              schema:
                type: object
                required: [current_epoch]
                properties:
                  current_epoch: { type: integer }

components:
  responses:
    BadRequest:
      description: Validation error
      content:
        application/json:
          schema:
            $ref: "#/components/schemas/Error"
    Busy:
      description: Backpressure
      headers:
        Retry-After:
          schema: { type: integer }
      content:
        application/json:
          schema: { $ref: "#/components/schemas/Error" }
    OverLimit:
      description: Payload too large or ratio cap
      content:
        application/json:
          schema: { $ref: "#/components/schemas/Error" }
    Degraded:
      description: Service degraded; issuance shed
      content:
        application/json:
          schema: { $ref: "#/components/schemas/Error" }
    UpstreamTimeout:
      description: Timeout while contacting KMS/Registry
      content:
        application/json:
          schema: { $ref: "#/components/schemas/Error" }

  schemas:
    Error:
      type: object
      required: [reason]
      additionalProperties: false
      properties:
        reason:
          type: string
          enum:
            - ttl_too_long
            - unknown_caveat
            - no_acceptable_alg
            - kms_timeout
            - degraded
            - busy
            - over_limit
            - ratio_cap
        message: { type: string }
        corr_id: { type: string }
security: []
```

> The full OpenAPI spec lives at `docs/openapi/svc-passport.v1.yaml` and MUST be kept in sync with DTO code and this document.

---

## 9. Examples

### 9.1 Mint a token

```bash
curl -sS -X POST http://localhost:8080/v1/passport/issue \
  -H "Content-Type: application/json" \
  -H "X-Corr-ID: 01J9XYZABCDEF" \
  -d '{
    "subject_ref":"sub-abc123",
    "audience":"svc-mailbox",
    "ttl_s":900,
    "caveats":["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"],
    "accept_algs":["ed25519+ml-dsa","ed25519"]
  }'
```

### 9.2 Revoke by epoch

```bash
curl -sS -X POST http://localhost:8080/v1/passport/revoke \
  -H "Content-Type: application/json" \
  -d '{"epoch":43,"reason":"compromise"}'
```

### 9.3 Verify (preflight)

```bash
curl -sS -X POST http://localhost:8080/v1/passport/verify \
  -H "Content-Type: application/json" \
  -d '{"token":"<opaque>"}'
```

---

## 10. History (selected)

* **v1.0.0** — Initial `/v1` with `issue`, `verify`, `revoke`. PQ negotiation via `accept_algs`. Error reasons standardized.

(Record future notable shifts here: new caveat families, response field additions, etc.)

---

## 11. Perfection Gates tie-in

* **Gate G:** No undocumented API (Rust or HTTP).
* **Gate H:** Breaking changes require **major** (Rust) or **/v2** (HTTP).
* **Gate J:** CHANGELOG entry required when public surface diff exists.
* **Gate K:** OpenAPI and DTOs **must** match (CI docs-sync).

---

```
```
