
````markdown
---
title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-10-12
audience: contributors, auditors, API consumers
---

# API.md — svc-interop

## 0. Purpose

This document captures the **public API surface** of `svc-interop`:

- Snapshot of exported **Rust** items (if any) and the canonical **HTTP** surface.
- SemVer discipline: what constitutes **breaking**, **minor**, and **patch** changes for both Rust and HTTP layers.
- CI-enforceable diffs via `cargo public-api` (Rust) and OpenAPI snapshot (HTTP).
- Source of truth for auditors and external integrators.

> Design intent: `svc-interop` is a **service** crate. The public contract is its **HTTP interface**. Rust APIs remain minimal and internal by default.

---

## 1. Public API Surface

### 1.1 Rust surface (intentionally minimal)

`svc-interop` is primarily a binary. To support tooling (`cargo public-api`) and internal embedding, we expose a tiny **optional** library API behind the `libapi` feature. Consumers should **not** depend on these for cross-crate use unless explicitly approved.

Generated via:

```bash
cargo public-api -p svc-interop --features libapi --simplified --deny-changes
````

**Current surface (expected when `libapi` is enabled):**

```text
pub mod prelude
pub struct Service
pub struct ServiceOptions
pub fn run(opts: ServiceOptions) -> Result<(), anyhow::Error>
```

Notes:

* `ServiceOptions` mirrors a **stable subset** of `Config` (bind/metrics/timeouts). New fields are additive and non-breaking.
* `run()` is a convenience to start the HTTP server. It is **not** a stability anchor for third parties; the HTTP contract is.

If the crate is built **without** `libapi`, the Rust surface is intentionally empty (binary-only).

### 1.2 HTTP surface (canonical)

Base URL: the service bind (`bind_addr`) as configured. All JSON is UTF-8. All error bodies follow the **error taxonomy** in the IDB/config (e.g., `reason: "BODY_LIMIT"`).

**Common headers**

* `Content-Type: application/json`
* `X-Corr-ID` (optional; generated if missing, echoed on response)
* `Cache-Control` as documented per route
* For webhooks: provider-specific signature headers required (edge-only verification)

#### Endpoints

1. `POST /webhooks/:provider`

* Purpose: ingress for signed webhooks (`github|stripe|slack_webhook`).
* Request: JSON or provider-native form; body size ≤ 1 MiB (streaming > 1 MiB with 64 KiB chunks).
* Headers (examples):

  * GitHub: `X-Hub-Signature-256: sha256=<hex>`
  * Stripe: `Stripe-Signature: t=<ts>,v1=<sig>[,v0=<sig>]`
  * Slack Webhook: `X-Slack-Signature: v0=<sig>`, `X-Slack-Request-Timestamp: <ts>`
* Responses:

  * `202 Accepted` `{ "accepted": true, "corr_id": "<ulid>" }`
  * `4xx/5xx` with body `{ "error": "...", "reason": "BAD_ORIGIN|UNAUTH|DECOMP_LIMIT|..." }`

2. `POST /put`

* Purpose: normalized content ingress → internal BLAKE3 addressing + enqueue/forward.
* Request body: JSON or binary (when `Content-Type` is not JSON). For JSON:

  ```json
  { "payload": "<base64|opaque>", "meta": { "type": "…" } }
  ```
* Response:

  * `202 Accepted` `{ "address": "b3:<hex>", "corr_id": "<ulid>" }`
  * `413` if `max_body_bytes` exceeded; `429` for `Busy`; `503` on brownout.

3. `GET /o/:addr`

* Purpose: served object by **BLAKE3** content address.
* Path: `:addr` must be `b3:<hex>`.
* Response:

  * `200` with bytes (or JSON if negotiated)
  * `404` if not found; `400` if malformed address.

4. Health/Meta

* `GET /healthz` → `200` when process is live (no deps).
* `GET /readyz` → `200` when deps met and no brownout for requested class; else `503`:

  ```json
  { "degraded": true, "missing": ["passport_reachable"], "retry_after": 5 }
  ```
* `GET /metrics` → Prometheus text format (bind should be localhost).
* `GET /version` → build/scm metadata `{ "version": "…", "git": "…", "rustc": "…" }`.

> **Correlation:** All responses include `X-Corr-ID`. Metrics/trace exemplars may attach `trace_id` to latency series.

---

## 2. SemVer Discipline

We maintain **two layers** of compatibility:

### 2.1 Rust (feature `libapi`)

**Additive (minor / non-breaking)**

* New free functions, structs, modules (not publicly reexported as defaults).
* New fields on `ServiceOptions` behind `#[non_exhaustive]`.
* New error variants with enums marked `#[non_exhaustive]`.

**Breaking (major)**

* Remove/rename exports.
* Change function signatures/return types or trait bounds.
* Make a previously `#[non_exhaustive]` enum exhaustive.

**Patch**

* Documentation/comments.
* Internal perf improvements, bug fixes without surface change.

### 2.2 HTTP

We treat the HTTP contract like an external API with its own SemVer-like policy:

**Additive (non-breaking)**

* Add new endpoints.
* Add optional request fields with defaults.
* Add new response fields that clients can ignore.
* Expand `reason` taxonomy with a documented meaning.

**Breaking**

* Remove or rename endpoints.
* Change response semantics for existing codes.
* Tighten validation in a way that rejects previously valid requests.
* Change admitted media types without negotiation.

**Patch**

* Better error messages; more specific reason codes.
* Performance characteristics without semantic change.

**Versioning strategy**

* Current API is **unversioned paths** with strong stability. If we must break, introduce `/v2/…` while keeping `/v1` (or current paths) available during a deprecation window, or use **media type versioning**:

  * `Accept: application/vnd.ron.interop+json;v=1`

---

## 3. Stability Guarantees

* **MSRV**: `1.80.0` (Rust) — enforced in CI.
* **Safety**: no `unsafe` in public APIs unless justified and reviewed.
* **No leakage of internal types**: e.g., we do not expose `tokio` sockets from public signatures.
* **Error bodies**: stable JSON shape `{ error, reason, corr_id }`; `reason` is from a documented **closed** taxonomy (with additive-only growth).

---

## 4. Invariants (API-specific)

* Service is the **contract**; Rust `libapi` is convenience, not a public SDK.
* Requests > 1 MiB must be **streamed**; server enforces body and decompression caps.
* All **internal addresses and audits use BLAKE3**; SHA-256 exists only for **edge verification** and is never persisted.
* Readiness **fails closed** for bridging if SLOs are breached; read paths may remain ready under brownout.
* Every non-2xx response includes a stable `reason` and echoes `X-Corr-ID`.

---

## 5. Tooling

* `cargo public-api` on `svc-interop` with `--features libapi` to guard Rust surface.
* `cargo semver-checks` (optional) to detect semver violations.
* **OpenAPI source**: maintained at `/docs/openapi/svc-interop.openapi.yaml`; PRs that change HTTP add/update this file and the snapshot under `/docs/api-history/http/`.
* `cargo doc` must build with `--features libapi` to keep docs coherent.

---

## 6. CI & Gates

* Pipeline jobs:

  * **Public API gate (Rust):**

    ```bash
    cargo public-api -p svc-interop --features libapi --simplified --deny-changes
    ```

    Fails if the Rust surface changes without explicit approval.
  * **HTTP snapshot gate:**

    * Validate `/docs/openapi/svc-interop.openapi.yaml` with spectral/openapi-lint.
    * Diff against previous snapshot (stored in `/docs/api-history/http/<version>.yaml`).
    * If diffs exist, require CHANGELOG entry with SemVer classification (breaking/minor/patch).
  * **Doc gate:** `#![deny(missing_docs)]` for any `pub` items when `libapi` is on.

* The PR bot posts:

  * Rust symbol diff (added/removed).
  * OpenAPI diff summary (paths/verbs/schemas).

---

## 7. Acceptance (Definition of Done)

* Current **Rust** API snapshot generated and stored under `/docs/api-history/rust/<crate>/<version>.txt`.
* Current **HTTP** OpenAPI spec updated with route/field changes and snapshot recorded.
* CHANGELOG updated with a clearly labeled section:

  * `### API (HTTP)` and/or `### API (Rust/libapi)`.
* All public items have rustdoc and examples where applicable.
* Release notes mention any new `reason` codes or deprecations.

---

## 8. HTTP Reference (concise)

### `POST /webhooks/:provider`

* **Auth**: provider signature (edge).
* **On success**: `202` with `{ accepted, corr_id }`.
* **Errors**: `400 BAD_ORIGIN|MALFORMED`, `401 UNAUTH`, `413 BODY_LIMIT`, `429 BACKPRESSURE|RATE_LIMIT`, `503 DOWNSTREAM_UNAVAILABLE|BACKPRESSURE`.

### `POST /put`

* **Body**: JSON or binary.
* **On success**: `202` with `{ address: "b3:<hex>", corr_id }`.

### `GET /o/:addr`

* **Path**: `:addr = b3:<hex>`.
* **On success**: `200` with bytes.
* **Errors**: `400`, `404`.

### Health/Meta

* `/healthz`, `/readyz`, `/metrics`, `/version` per Observability & Config docs.

> All endpoints return `X-Corr-ID` and log/audit with `b3_hash` (when applicable).

---

## 9. Example cURL

```bash
# Webhook (GitHub)
curl -i -X POST http://localhost:8080/webhooks/github \
  -H 'X-Hub-Signature-256: sha256=<sig>' \
  -H 'X-Corr-ID: demo-123' \
  -d '{"action":"opened","…":…}'

# Put (JSON)
curl -i -X POST http://localhost:8080/put \
  -H 'Content-Type: application/json' \
  -d '{"payload":"…","meta":{"type":"example"}}'

# Get by BLAKE3 address
curl -i http://localhost:8080/o/b3:7f1a…c0de
```

---

## 10. Appendix

### 10.1 OpenAPI (starter sketch)

```yaml
openapi: 3.0.3
info:
  title: svc-interop API
  version: "1"
servers:
  - url: http://localhost:8080
paths:
  /webhooks/{provider}:
    post:
      parameters:
        - name: provider
          in: path
          required: true
          schema: { type: string, enum: [github, stripe, slack_webhook] }
      requestBody:
        required: true
        content:
          application/json: { schema: { type: object } }
          application/x-www-form-urlencoded: { schema: { type: object, additionalProperties: true } }
      responses:
        "202": { description: Accepted, content: { application/json: { schema: { $ref: "#/components/schemas/Accepted" } } } }
        "4XX": { $ref: "#/components/responses/Error" }
        "5XX": { $ref: "#/components/responses/Error" }
  /put:
    post:
      requestBody:
        required: true
        content:
          application/json: { schema: { $ref: "#/components/schemas/PutRequest" } }
          application/octet-stream: { schema: { type: string, format: binary } }
      responses:
        "202": { description: Accepted, content: { application/json: { schema: { $ref: "#/components/schemas/PutAccepted" } } } }
        "4XX": { $ref: "#/components/responses/Error" }
        "5XX": { $ref: "#/components/responses/Error" }
  /o/{addr}:
    get:
      parameters:
        - name: addr
          in: path
          required: true
          schema: { type: string, pattern: '^b3:[0-9a-fA-F]+$' }
      responses:
        "200": { description: OK, content: { application/octet-stream: {} } }
        "400": { $ref: "#/components/responses/Error" }
        "404": { $ref: "#/components/responses/Error" }
  /healthz:
    get: { responses: { "200": { description: OK } } }
  /readyz:
    get:
      responses:
        "200": { description: Ready }
        "503": { $ref: "#/components/responses/Error" }
  /metrics:
    get: { responses: { "200": { description: Prometheus metrics } } }
components:
  schemas:
    Accepted: { type: object, properties: { accepted: { type: boolean }, corr_id: { type: string } }, required: [accepted, corr_id] }
    PutRequest:
      type: object
      properties:
        payload: { type: string }
        meta: { type: object, additionalProperties: true }
      required: [payload]
    PutAccepted:
      type: object
      properties:
        address: { type: string, pattern: '^b3:' }
        corr_id: { type: string }
      required: [address, corr_id]
    ErrorBody:
      type: object
      properties:
        error: { type: string }
        reason: { type: string, enum: [UNAUTH, BAD_ORIGIN, EXPIRED_TOKEN, SCOPE_MISMATCH, BODY_LIMIT, DECOMP_LIMIT, RATE_LIMIT, BACKPRESSURE, DOWNSTREAM_UNAVAILABLE, POLICY_BLOCKED, MALFORMED] }
        corr_id: { type: string }
      required: [error, reason, corr_id]
  responses:
    Error:
      description: Error with stable reason code
      content:
        application/json:
          schema: { $ref: '#/components/schemas/ErrorBody' }
```

### 10.2 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 10.3 Perfection Gates tie-in

* Gate G: No undocumented API surface (Rust & HTTP).
* Gate H: Breaking changes require major bump (Rust) or versioned HTTP (path/media type).
* Gate J: CHANGELOG alignment required when diff detected.

### 10.4 History

* v1 (current): initial HTTP set (`/webhooks/:provider`, `/put`, `/o/:addr`, health/meta) with closed error taxonomy; Rust `libapi` gated behind feature.

```

