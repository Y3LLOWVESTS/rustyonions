````markdown
---
title: API Surface & SemVer Reference — svc-mailbox
status: draft
msrv: 1.80.0
last-updated: 2025-10-12
audience: contributors, auditors, API consumers
---

# API.md — svc-mailbox

## 0. Purpose

This document is the **canonical spec** for the public API surface of `svc-mailbox`.

- Enumerates the **HTTP/REST** and **gRPC** endpoints (wire contracts).
- States **idempotency**, **auth**, **error taxonomy**, and **rate-limit/backpressure** semantics.
- Defines **SemVer** discipline for changes (what’s breaking vs. additive).
- CI-enforceable with `cargo public-api` (Rust symbol surface) and OpenAPI diff checks (wire surface).
- Aligns with `docs/IDB.md`, `docs/CONFIG.md`, and Observability/Hardening blueprints.

> `svc-mailbox` is a **service crate**. Its stable API is the **network surface**; the Rust symbol surface is deliberately minimal and **not** the primary integration target.

---

## 1. Public API Surface

### 1.1 Rust (symbol) surface — intentionally tiny

Produced by:

```bash
cargo public-api -p svc-mailbox --simplified --deny-changes
````

**Current snapshot (expected)**

```text
pub mod cfg            // configuration types (unstable)
pub fn main()          // binary entrypoint (not for embedding)
pub fn run(cfg: cfg::Config) -> Result<(), anyhow::Error>   // optional for tests
```

> Note: The Rust surface is **not** a stable SDK. Consumer integrations must use the **HTTP** or **gRPC** APIs below. If an official Rust SDK exists, it will live in a separate crate (`ron-mailbox-sdk`).

---

### 1.2 HTTP (REST) — canonical wire API

**Base URL:** `/v1`
**Auth:** `Authorization: Bearer <macaroon>` (scoped capabilities)
**Correlation:** Optional `X-Corr-Id: <uuidv7>` header; always echoed (generated if missing).
**Content Types:** `application/json` (requests and responses), `application/x-ndjson` for streaming.

#### Endpoints

1. **Enqueue (SEND)**
   `POST /v1/send`

**Request**

```json
{
  "topic": "user:42:inbox",
  "idem_key": "email-2025-10-12-01",
  "payload_b64": "eyJzdWJqZWN0IjoiSGkifQ==",
  "attrs": { "content-type": "application/json" }
}
```

**Response (default mode: 200 OK)**

```json
{ "msg_id": "01JAE6ZQX5...", "duplicate": false }
```

**Status codes**

* `200 OK` — enqueued (or duplicate acknowledged with `"duplicate": true`)
* `409 Conflict` — **optional compatibility mode** for duplicates (see header below)
* `400 Bad Request` — schema/unknown fields/invalid sizes
* `401/403` — auth/authz failure
* `413 Payload Too Large` — > 1 MiB frame
* `429 Too Many Requests` — saturation (see `Retry-After`)
* `503 Service Unavailable` — degraded

**Headers**

* `X-Corr-Id` — correlation id (request/response)
* `Retry-After` — seconds (429/503)
* `X-Idempotency-Mode` (optional request): `200-flag|409-conflict` (default `200-flag`)

2. **Receive (lease)**
   `POST /v1/recv`

**Request**

```json
{
  "topic": "user:42:inbox",
  "visibility_ms": 5000,
  "max_messages": 32,
  "max_bytes": 524288
}
```

**Response (200 OK)**

```json
{ "messages": [ { "msg_id":"01JAE6...", "topic":"user:42:inbox",
  "ts":"2025-10-12T18:02:41Z", "idem_key":"email-2025-10-12-01",
  "payload_hash":"b3:...", "attrs":{}, "corr_id":"018f1a2a-...",
  "shard":7, "attempt":1, "hash_chain":"b3:...", "sig":null } ] }
```

**Status**: `200 OK` (empty list if none), `401/403`, `429/503` on saturation/degrade.

3. **Receive (streaming)**
   `POST /v1/recv/stream` → `Content-Type: application/x-ndjson`
   Body identical to `/v1/recv` (visibility applies). Each line is a JSON `Envelope`. Ack via standard endpoints.

4. **Acknowledge**
   `POST /v1/ack/{msg_id}` → `{ "ok": true }`

* Idempotent. `404` if unknown/expired.

5. **Negative-ack (requeue with backoff)**
   `POST /v1/nack/{msg_id}` (optional JSON body `{ "reason": "transient_error" }`) → `{ "ok": true }`

6. **DLQ reprocess (operator)**
   `POST /v1/dlq/reprocess` (guarded scope)
   Body: `{ "topic": "user:42:inbox", "limit": 100 }` → moves DLQ→ready with policy.

7. **Health/Readiness/Metrics**

* `GET /healthz` → 200
* `GET /readyz` → 200 or 503 `{ "degraded": true, "missing": [...] }`
* `GET /metrics` → Prometheus exposition (usually localhost-bound)

#### Error model (JSON)

```json
{ "code": "E_SATURATED", "message": "work queue full", "corr_id": "018f1a2a-..." }
```

**Codes:** `E_SCHEMA`, `E_CAP_AUTH`, `E_CAP_SCOPE`, `E_DUPLICATE`, `E_FRAME_TOO_LARGE`, `E_SATURATED`, `E_UNAVAILABLE`, `E_INTEGRITY`.

---

### 1.3 gRPC (Proto) — efficiency path

**Service:** `ron.mailbox.v1.Mailbox`
**RPCs:**

* `Send(SendRequest) -> SendResponse`
* `Recv(RecvRequest) -> RecvResponse`
* `RecvStream(RecvRequest) -> stream Envelope`
* `Ack(AckRequest) -> AckResponse`
* `Nack(NackRequest) -> AckResponse`

**Envelope fields (subset):** `msg_id, topic, ts, idem_key, payload_hash (b3:<hex>), attrs, corr_id, shard, attempt, hash_chain, sig, payload?`

> Transport framing follows OAP/1 constraints: `max_frame=1 MiB`, chunks ≈ 64 KiB.

---

### 1.4 Authentication & Authorization

* **Bearer macaroon** with scoped caveats:

  * `op ∈ {send,recv,ack,nack,admin}`
  * `topic` or `topic_class` scope
  * `quota` (bytes/ops), `expiry`
* Revocation is honored within **`cap_cache_ttl` (default 30s)**. Fail-closed on errors.

---

### 1.5 Idempotency

* `idem_key` is **required** on `SEND`.
* Duplicate defined as **same** `(topic, idem_key, payload_hash)` within `T_replay` (default **300s**).
* Default behavior: `200 OK` with `"duplicate": true` and **same `msg_id`**.
  Compatibility mode (opt-in): `409 Conflict` + same body; signal via `X-Idempotency-Mode: 409-conflict`.

---

### 1.6 Backpressure & Rate Limits

* Ingress may respond `429 Too Many Requests` with `Retry-After` (seconds).
* `503` indicates degraded readiness; treat as retryable unless told otherwise.

---

### 1.7 Integrity

* Envelopes include `payload_hash = blake3_256(payload)` and `hash_chain = H(topic, ts, idem_key, payload_hash, attrs)`.
* Integrity failure yields **`422 Unprocessable Entity`** with `code="E_INTEGRITY"`, and the message enters DLQ after `max_attempts`.

---

### 1.8 Examples

**Enqueue (curl)**

```bash
curl -sS -X POST http://localhost:8080/v1/send \
  -H "Authorization: Bearer $MACAROON" \
  -H "Content-Type: application/json" \
  -H "X-Corr-Id: $(uuidgen)" \
  -d '{"topic":"user:42:inbox","idem_key":"email-2025-10-12-01","payload_b64":"eyJzdWJqZWN0IjoiSGkifQ=="}'
```

**Receive batch**

```bash
curl -sS -X POST http://localhost:8080/v1/recv \
  -H "Authorization: Bearer $MACAROON" \
  -H "Content-Type: application/json" \
  -d '{"topic":"user:42:inbox","visibility_ms":5000,"max_messages":32}'
```

**Ack**

```bash
curl -sS -X POST http://localhost:8080/v1/ack/01JAE6ZQX5... \
  -H "Authorization: Bearer $MACAROON"
```

**Streaming receive (NDJSON)**

```bash
curl -N -sS -X POST http://localhost:8080/v1/recv/stream \
  -H "Authorization: Bearer $MACAROON" \
  -H "Content-Type: application/json" \
  -d '{"topic":"user:42:inbox","visibility_ms":5000,"max_messages":32}'
```

---

## 2. SemVer Discipline

**Scope:** both **Rust symbol** surface and **wire** surface (HTTP/gRPC). We treat **HTTP paths, schemas, status codes**, and **gRPC service/field numbers** as the *public API*.

### 2.1 Additive (Minor / Non-Breaking)

* Adding **optional** request fields (with defaults) and **new response fields**.
* Adding **new endpoints** or **new gRPC RPCs**.
* Adding **new error codes** (with fallbacks).
* Extending enums/flags marked `#[non_exhaustive]`.
* Adding **headers** that do not change existing semantics.

### 2.2 Breaking (Major)

* Removing or renaming endpoints/fields/headers.
* Changing JSON shapes or field types; changing wire-level defaults.
* Altering status codes in a way that breaks existing clients’ logic.
* Changing gRPC field **numbers**, message names, or RPC signatures.
* Changing idempotency semantics (e.g., default 200→409) without opt-in header.

### 2.3 Patch-Level

* Documentation, examples, and observability naming (non-breaking).
* Performance improvements that don’t change external behavior.

---

## 3. Stability Guarantees

* **MSRV**: `1.80.0`.
* **Paths** prefixed with `/v1/` (major in the URL).
* **Error taxonomy** is versioned; new codes only **add**, not repurpose.
* **Unsafe Rust** not used unless documented.
* **No internal type leakage** in public Rust surface (service crate).

---

## 4. Invariants (API-facing)

* **At-least-once**: duplicates possible; **exactly-once is not promised**.
* **Best-effort FIFO per shard**; no global ordering.
* **Auth required** for all ops (no ambient trust).
* **Strict sizes**: frame ≤ **1 MiB**, deny unknown JSON fields.
* **Integrity** enforced/observable; failures yield `E_INTEGRITY`.
* **Backpressure** is explicit (`429/503` + `Retry-After`).

---

## 5. Tooling

* **Rust symbol surface**

  ```bash
  cargo public-api -p svc-mailbox --simplified --deny-changes
  ```
* **OpenAPI diff** (HTTP)

  * Source of truth: `api/openapi.yaml`
  * CI: run a spec diff (`oasdiff`, `schemathesis`) against previous release.
* **gRPC/Proto** diff

  * Source of truth: `proto/ron/mailbox/v1/mailbox.proto`
  * CI: `buf breaking` (if using Buf) or `protolock` equivalent.
* **Docs**

  * `cargo doc --no-deps -p svc-mailbox` (internal docs)
  * API snapshots in `/docs/api-history/svc-mailbox/{version}.txt`

---

## 6. CI & Gates

* PR pipeline must:

  * ✅ Run `cargo public-api` and fail on unacknowledged symbol changes.
  * ✅ Diff `api/openapi.yaml` vs previous tag; block breaking without major bump.
  * ✅ Diff `.proto` with Buf/Protolock; block breaking without major bump.
  * ✅ Ensure `CHANGELOG.md` contains an entry matching any API diff.
  * ✅ Run contract tests (Vectors in `docs/IDB.md` Appendix C).

---

## 7. Acceptance Checklist (DoD)

* [ ] **Current API snapshot** generated & stored in `/docs/api-history/svc-mailbox/<ver>.txt`.
* [ ] **OpenAPI** updated and passes CI diffs.
* [ ] **Proto** updated and passes Buf/Protolock.
* [ ] **CHANGELOG** includes surface or behavior changes.
* [ ] **Examples** (curl/SDK snippets) compile or run as part of doctests.
* [ ] **Observability** names match `OBSERVABILITY.md` (metrics/logs/spans).
* [ ] **Security**: error codes for auth/integrity present and documented.

---

## 8. Appendix

### 8.1 Error Codes (canonical)

| HTTP | Code                | Meaning                       | Client action                 |
| ---: | ------------------- | ----------------------------- | ----------------------------- |
|  400 | `E_SCHEMA`          | invalid JSON / unknown fields | fix request                   |
|  401 | `E_CAP_AUTH`        | invalid/expired token         | refresh token                 |
|  403 | `E_CAP_SCOPE`       | scope mismatch / forbidden op | request proper scope          |
|  409 | `E_DUPLICATE`       | duplicate (compat mode only)  | treat as success with same id |
|  413 | `E_FRAME_TOO_LARGE` | > 1 MiB                       | reduce payload                |
|  422 | `E_INTEGRITY`       | integrity check failed        | investigate/repair            |
|  429 | `E_SATURATED`       | backpressure                  | retry after `Retry-After`     |
|  503 | `E_UNAVAILABLE`     | degraded / not ready          | retry with backoff            |

### 8.2 Headers

* `Authorization: Bearer <macaroon>`
* `X-Corr-Id: <uuidv7>` (optional input; always output)
* `Retry-After: <seconds>` (429/503)
* `X-Idempotency-Mode: 200-flag|409-conflict` (optional request)

### 8.3 Field Shapes (summary)

**Envelope**:
`{ msg_id, topic, ts, idem_key, payload_hash, attrs, corr_id, shard, attempt, hash_chain, sig? }`

**SendRequest**:
`{ topic, idem_key, payload_b64, attrs? }`

**RecvRequest**:
`{ topic, visibility_ms, max_messages?, max_bytes? }`

---

### 8.4 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* Buf (breaking checks): [https://buf.build/](https://buf.build/)

---

✅ With this API spec + CI gates, `svc-mailbox` maintains a **predictable, versioned** wire contract while allowing additive evolution without breaking existing clients.

```
```
