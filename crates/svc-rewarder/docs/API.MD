---

title: API Surface & SemVer Reference — svc-rewarder
status: draft
msrv: 1.80.0
last-updated: 2025-10-14
audience: contributors, auditors, API consumers
-----------------------------------------------

# 📖 API.md — `svc-rewarder`

## 0. Purpose

This document captures the **public API surface** of `svc-rewarder`:

* HTTP/REST endpoints (primary contract for consumers).
* Wire-level semantics (idempotency, auth, rate limits, headers).
* Rust surface (if any public library API is exported).
* SemVer discipline: what changes break vs. extend.
* CI enforcement via `cargo public-api` (library surface) and OpenAPI diff (HTTP surface).

`svc-rewarder` computes deterministic reward manifests from sealed inputs and emits **idempotent** settlement intents to the ledger.

---

## 1. Public API Surface

> `svc-rewarder` is a **service crate**. Its stable surface is the **HTTP API**.
> Any Rust types exposed are internal-only unless explicitly stated here. The crate does **not** publish a reusable Rust library API by default.

### 1.1 HTTP Endpoints (v1)

Base path: `/rewarder`
All endpoints require **TLS** and **capability tokens (macaroons)** unless explicitly noted.

#### 1) `POST /rewarder/epochs/{epoch_id}/compute`

**Purpose:** Compute rewards for an epoch using sealed inputs and a signed policy. Emits **zero** ledger effects if `dry_run=true`.

* **Auth:** macaroon with scope `rewarder.run`, tenant-scoped.
* **Idempotency:** Natural, via `run_key = hash(epoch_id || policy_hash || inputs_cid)`.
  Re-submitting the same triple is safe; ledger sees a duplicate emit (`dup`) with **no new effects**.
* **Rate limits:** Per-tenant and per-instance (see headers).
* **Request (JSON):**

```json
{
  "inputs_cid": "b3:7d9c...c1",          // BLAKE3 or content-addressed handle
  "policy_id": "rev42",
  "policy_hash": "b3:3aa1...f83",
  "dry_run": false,
  "notes": "optional operator notes"
}
```

* **Success (200 OK):**

```json
{
  "epoch_id": "2025-10-01",
  "run_key": "e91a6e1b1f2c9c0d",
  "commitment": "b3:1ac4...9b0",
  "status": "ok",                         // ok | quarantined | fail
  "totals": {
    "pool_minor_units": "123456789",
    "payout_minor_units": "123456700",
    "residual_minor_units": "89"
  },
  "policy": { "id": "rev42", "hash": "b3:3aa1...f83", "signed": true },
  "invariants": { "conservation": true, "overflow": false, "idempotent": true },
  "ledger": { "emitted": true, "result": "accepted" }, // dry_run => emitted=false
  "metrics": { "compute_ms": 418, "cost_estimate_ms": 377 }
}
```

* **Quarantine (409 Conflict):** Invariant breach; **no** ledger effects.

```json
{
  "status": "quarantined",
  "reason": "conservation",
  "details": "Σ payouts exceeds pool_total",
  "run_key": "e91a6e1b1f2c9c0d",
  "commitment": "b3:1ac4...9b0"
}
```

* **Common Errors:**

  * `400 BAD_REQUEST` — schema invalid, oversize, decompress cap, stale policy hash.
  * `401 UNAUTHENTICATED` — missing/invalid macaroon.
  * `403 UNAUTHORIZED` — missing scope/caveats failed.
  * `409 CONFLICT` — same `epoch_id` with **different** `(policy_hash, inputs_cid)` commitment.
  * `429 TOO_MANY_REQUESTS` — rate/concurrency caps.
  * `503 SERVICE_UNAVAILABLE` — degraded (dependency or backpressure).

#### 2) `GET /rewarder/epochs/{epoch_id}`

**Purpose:** Fetch the sealed manifest for an epoch (read-only).

* **Auth:** macaroon with scope `rewarder.inspect` (read-only).
* **Response (200 OK):**

```json
{
  "epoch_id": "2025-10-01",
  "run_key": "e91a6e1b1f2c9c0d",
  "commitment": "b3:1ac4...9b0",
  "status": "ok",
  "policy": { "id": "rev42", "hash": "b3:3aa1...f83", "signed": true },
  "totals": { "pool_minor_units": "123456789", "payout_minor_units": "123456700", "residual_minor_units": "89" },
  "attestation": {
    "sig_ed25519": "base64:...",
    "sig_pq": "base64:...",                   // present if PQ dual-sign enable
    "signed_at": "2025-10-01T12:34:56Z"
  }
}
```

* **Errors:** `404 NOT_FOUND` (no manifest), plus standard auth/rate/degraded responses.

#### 3) `GET /rewarder/policy/{policy_id}`

**Purpose:** Fetch a **signed** policy document by ID for audit/reproducibility.

* **Auth:** macaroon with scope `rewarder.inspect` (or public if configured).
* **Response (200 OK):**

```json
{
  "id": "rev42",
  "hash": "b3:3aa1...f83",
  "version": "1.2.0",
  "signed": true,
  "body": { "weights": { "views": 0.3, "subs": 0.7 }, "rounding": "bankers" }
}
```

#### 4) Health/Meta

* `GET /healthz` — liveness (200 if process alive).
* `GET /readyz` — readiness (200 when deps OK; else 503 with JSON `{degraded, missing, retry_after}`).
* `GET /metrics` — Prometheus.
* `GET /version` — build info (git SHA, features, msrv).

---

### 1.2 HTTP Headers & Conventions

* **Auth:** `Authorization: Bearer <macaroon>` (preferred).
  Legacy interop: gateway may present JWT; service **converts** to scoped macaroon internally (JWT is not forwarded downstream).
* **Correlation:** `X-Corr-ID` propagated end-to-end; server generates if missing and echoes it.
* **Idempotency:** Natural by content triple; clients **do not** need a separate `Idempotency-Key` header.
* **Rate Limit Feedback:**

  * `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `Retry-After` (on 429/503).
* **Content:** `Content-Type: application/json`; gzip/zstd accepted; decompression **ratio ≤ 10×**; post-inflate hard cap enforced.

---

### 1.3 Error Schema (Uniform)

All errors return this envelope with appropriate HTTP status:

```json
{
  "error": {
    "code": "UNAUTHORIZED|BAD_REQUEST|CONFLICT|QUARANTINED|DEPENDENCY_UNAVAILABLE|INTERNAL",
    "message": "human-readable summary",
    "corr_id": "01JC...ULID",
    "details": { "reason": "schema|invariant|oversize|decompress_cap|cost_cap|..." }
  }
}
```

**Do not** echo secrets, tokens, or raw input payloads in error messages.

---

### 1.4 Example cURL Invocations

```bash
# Compute (production)
curl -sS -X POST "https://node.example.com/rewarder/epochs/2025-10-01/compute" \
  -H "Authorization: Bearer ${MACAROON}" \
  -H "X-Corr-ID: demo-123" \
  -H "Content-Type: application/json" \
  --data '{
    "inputs_cid":"b3:7d9c...c1",
    "policy_id":"rev42",
    "policy_hash":"b3:3aa1...f83",
    "dry_run":false
  }'

# Fetch manifest
curl -sS -H "Authorization: Bearer ${MACAROON}" \
  "https://node.example.com/rewarder/epochs/2025-10-01"

# Policy (read-only)
curl -sS -H "Authorization: Bearer ${MACAROON}" \
  "https://node.example.com/rewarder/policy/rev42"
```

---

## 2. SemVer Discipline

The **HTTP API** adheres to semantic versioning via a **versioned OpenAPI** and URL stability guarantees.

### 2.1 Additive (Minor / Non-Breaking)

* Add new optional request fields with defaults.
* Add new response fields (must not break existing parsers).
* Add new endpoints under the same base path.
* Extend enums marked `#[non_exhaustive]` (documented as such in OpenAPI using `x-non-exhaustive: true`).

### 2.2 Breaking (Major)

* Remove or rename endpoints/paths.
* Change required fields or field types.
* Change response codes for the same condition (e.g., 409 → 400).
* Make previously optional fields required.
* Tighten validation beyond documented constraints in a way that rejects previously valid requests.

### 2.3 Patch-Level

* Doc-only updates, example clarifications.
* Performance improvements without behavior change.
* Additional error details inside `error.details` that do not change `code`.

**Rule of thumb:** If an existing (well-behaved) client would start failing, it’s **breaking**.

---

## 3. Stability Guarantees

* **MSRV:** 1.80.0.
* **TLS:** TLS 1.3 required; cipher suites pinned in deployment.
* **Auth:** macaroon-based capabilities are the stable model; JWT accepted only at the edge and converted internally.
* **Idempotency:** `run_key` semantics are stable: same triple → same outcome; double-emit is prevented by design.
* **Error taxonomy:** codes above are stable; we may add **new** codes (additive).

---

## 4. Rust Public Surface (Library Exports)

By default, `svc-rewarder` is built as a **binary service** and does not commit to a stable Rust API. If a library target is exposed in the future, it will be documented here and subjected to `cargo public-api` gates.

**Current:** *No stable Rust exports.*

You may still generate a snapshot (will be empty or minimal for service crates):

```bash
cargo public-api --simplified --deny-changes
```

Store results in: `docs/api-history/svc-rewarder/<version>.txt`.

---

## 5. OpenAPI & Schemas

We publish a machine-readable contract for the HTTP API.

* `GET /openapi.json` — OpenAPI 3.1 (current minor version).
* `GET /schema/compute.json` — JSON Schema for the `POST /compute` request.

### 5.1 OpenAPI Sketch (excerpt)

```yaml
openapi: 3.1.0
info:
  title: svc-rewarder API
  version: 1.0.0
servers:
  - url: https://node.example.com
paths:
  /rewarder/epochs/{epoch_id}/compute:
    post:
      operationId: computeEpoch
      security: [{ bearerAuth: [] }]
      parameters:
        - in: path
          name: epoch_id
          required: true
          schema: { type: string, pattern: "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/ComputeRequest"
      responses:
        "200": { $ref: "#/components/responses/ComputeOk" }
        "409": { $ref: "#/components/responses/Quarantined" }
        "4XX": { $ref: "#/components/responses/Error" }
        "5XX": { $ref: "#/components/responses/Error" }
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: macaroon
  schemas:
    ComputeRequest:
      type: object
      additionalProperties: false
      properties:
        inputs_cid: { type: string, minLength: 8 }
        policy_id:  { type: string, minLength: 1 }
        policy_hash:{ type: string, minLength: 8 }
        dry_run:    { type: boolean, default: false }
        notes:      { type: string, maxLength: 1024 }
      required: [inputs_cid, policy_id, policy_hash]
    ErrorEnvelope:
      type: object
      properties:
        error:
          type: object
          properties:
            code: { type: string }
            message: { type: string }
            corr_id: { type: string }
            details: { type: object, additionalProperties: true }
          required: [code, message, corr_id]
  responses:
    Error:
      description: Error
      content:
        application/json: { schema: { $ref: "#/components/schemas/ErrorEnvelope" } }
```

---

## 6. Behavior & Invariants (Wire Semantics)

* **Determinism:** Given identical `(epoch_id, policy_hash, inputs_cid)`, outputs are byte-for-byte stable (manifest + totals).
* **Conservation:** Σ payouts ≤ pool_total; violation ⇒ `409` with `status=quarantined` and **no** ledger emission.
* **Idempotency:** Multiple `POST .../compute` with same triple produce **at most one** ledger intent (`accepted` first time, then `dup`).
* **Cost Guard:** If estimated compute cost exceeds `MAX_EPOCH_COST_MS`, request is rejected with `400` (`details.reason="cost_cap"`) or forced `dry_run` if configured.
* **Compression:** gzip/zstd accepted; decompression ratio ≤ 10× and post-inflate cap < 8 MiB; exceeding caps ⇒ `400` with `details.reason="decompress_cap"`.

---

## 7. Deprecation Policy

* We version the **OpenAPI** document and annotate deprecated fields/endpoints with `"deprecated": true`.
* A deprecation requires:

  * CHANGELOG entry with migration notes.
  * Minimum **one minor release** grace period before removal.
  * If removal is breaking, the **major** version increments and old path is removed only then.

---

## 8. CI & Gates

* **OpenAPI Diff Gate:** Pull Requests that change `openapi.json` must include a rendered diff; CI blocks without a CHANGELOG entry and acceptance checklist.
* **`cargo public-api`:** Runs for any library target (if/when present).
* **Contract Tests:** Integration tests call `/compute` and `/epochs/{id}` with canonical vectors; CI asserts:

  * Determinism: re-running vectors yields identical manifests.
  * Idempotency: first emit `accepted`, subsequent `dup`, never double-accepted.
  * Invariant handling: known-bad vectors yield `409 quarantined`.

---

## 9. Acceptance Checklist (DoD)

* [ ] Current **OpenAPI** generated and stored under `docs/openapi/svc-rewarder.json`.
* [ ] API snapshot stored at `docs/api-history/svc-rewarder/<version>.txt` (if library target exists).
* [ ] CI gates pass (`openapi-diff`, `cargo public-api` if applicable).
* [ ] CHANGELOG updated for any surface/behavior changes.
* [ ] Canonical test vectors updated (good & bad cases).
* [ ] Docs & examples updated; error codes documented.

---

## 10. History (notable API shifts)

* **1.0.0** — Initial stable release: `/epochs/{id}/compute`, `/epochs/{id}`, `/policy/{id}`, `/healthz`, `/readyz`, `/metrics`, `/version`.
* *Future changes recorded here with migration notes.*

---

## 11. Appendix

**References**

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

**Perfection Gates tie-in**

* **Gate G:** No undocumented surface (OpenAPI + this file must match).
* **Gate H:** Breaking changes require major version bump.
* **Gate J:** CHANGELOG alignment enforced.

**Security Cross-Refs**

* Auth model: macaroons with caveats (tenant, path, method, TTL, IP).
* Idempotency: `run_key` = hash(epoch_id || policy_hash || inputs_cid).
* Invariants: conservation, determinism, integer-only money math.

---

### Quick Generator Snippets (optional)

Generate an API snapshot (library target only):

```bash
cargo public-api --simplified --deny-changes > docs/api-history/svc-rewarder/$(git describe --tags --always).txt
```

Validate OpenAPI (if you keep it in-repo at `docs/openapi/svc-rewarder.json`):

```bash
lint-openapi docs/openapi/svc-rewarder.json
openapi-diff previous.json docs/openapi/svc-rewarder.json
```

This API spec is **consumer-focused, invariant-aware, and SemVer-disciplined**, matching the economic safety guarantees of `svc-rewarder`.
