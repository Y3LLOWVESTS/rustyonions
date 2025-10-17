# 📖 API.md — `svc-wallet`

---

title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-10-16
audience: contributors, auditors, API consumers
-----------------------------------------------

## 0. Purpose

This document captures the **public API surface** of `svc-wallet`:

* Snapshot of exported functions/types (Rust surface) and **HTTP API** (primary contract).
* SemVer discipline: what changes **break** vs. **extend**.
* CI-enforceable via `cargo public-api` (Rust) and **OpenAPI** drift checks (HTTP).
* Single source of truth for external consumers and auditors.

`svc-wallet` is a **service crate**. Its authoritative public contract is the **HTTP API**; the Rust surface is intentionally minimal and not for external embedding.

---

## 1. Public API Surface

### 1.1 Rust surface (binary/service)

`svc-wallet` is a binary crate; it intentionally exposes **no stable Rust API** to other crates.

Generate and assert:

```bash
cargo public-api --simplified --deny-changes -p svc-wallet
```

**Expected snapshot (should be empty or minimal):**

```text
# (binary crate; no exported items intended)
```

> If any symbols appear, treat as **accidental exposure**; make items `pub(crate)` or move behind a private module.

---

### 1.2 HTTP API surface (authoritative)

OpenAPI spec lives at: `docs/openapi/svc-wallet.yaml` (generated from this section).

#### 1.2.1 Conventions

* **Base path:** `/v1`
* **Auth:** `Authorization: Bearer <macaroon>` (capability token)
* **Idempotency:** `Idempotency-Key: <uuid/ulid>` required on **POST** write operations
* **Content-Type:** `application/json; charset=utf-8`
* **Rate limiting headers (if present):** `Retry-After`, `X-RateLimit-*`

#### 1.2.2 Endpoints

| Method | Path            | Auth Scope    | Idempotent | Description                                                             |
| -----: | --------------- | ------------- | ---------- | ----------------------------------------------------------------------- |
|    GET | `/v1/balance`   | `read`        | Yes        | Return effective balance for `{account, asset}` with bounded staleness. |
|   POST | `/v1/issue`     | `issue`       | Via header | Mint asset to account (policy-gated).                                   |
|   POST | `/v1/transfer`  | `transfer`    | Via header | Move amount from `from` → `to`.                                         |
|   POST | `/v1/burn`      | `burn`        | Via header | Destroy (burn) amount from account.                                     |
|    GET | `/v1/tx/{txid}` | `read`        | Yes        | Fetch short receipt by `txid`.                                          |
|    GET | `/healthz`      | none          | Yes        | Liveness.                                                               |
|    GET | `/readyz`       | none          | Yes        | Readiness (fails writes when degraded).                                 |
|    GET | `/metrics`      | none/loopback | Yes        | Prometheus metrics.                                                     |

#### 1.2.3 Schemas (abridged)

```jsonc
// Balance Request: GET /v1/balance?account=<str>&asset=<str>
{
  "account": "acc_abc123",   // query
  "asset":   "ron"           // query
}

// Balance Response
{
  "account": "acc_abc123",
  "asset": "ron",
  "amount_minor": "2500000000000000000", // string-encoded u128
  "as_of": "2025-10-16T16:10:00Z",
  "stale_ms": 120
}

// POST /v1/transfer — Request (serde deny_unknown_fields)
{
  "from": "acc_src",
  "to": "acc_dst",
  "asset": "ron",
  "amount_minor": "250000",   // string u128, >0, ≤ max_amount_per_op
  "nonce": 42                 // u64, per-account, strictly monotonic
}

// Common Receipt (issue/transfer/burn)
{
  "txid": "tx_01JFA.....",
  "op": "transfer",           // enum
  "from": "acc_src",          // optional for issue
  "to": "acc_dst",            // optional for burn
  "asset": "ron",
  "amount_minor": "250000",
  "nonce": 42,
  "idem": "01JFA...ULID",     // echoed Idempotency-Key when present
  "ts": "2025-10-16T16:11:02Z",
  "receipt_hash": "b3-hex..." // BLAKE3(essential fields)
}

// Error model (deterministic)
{
  "code": "INSUFFICIENT_FUNDS", // enum (see table)
  "http": 409,
  "message": "insufficient funds",
  "retryable": false,
  "corr_id": "01JFA...",
  "details": { "asset":"ron" }
}
```

**Error codes (stable enum)**

| code                   | HTTP | Notes                          |
| ---------------------- | ---: | ------------------------------ |
| `BAD_REQUEST`          |  400 | DTO parse/validation failure   |
| `UNAUTHORIZED`         |  401 | Missing/invalid token          |
| `FORBIDDEN`            |  403 | Capability scope denied        |
| `LIMITS_EXCEEDED`      |  403 | Amount/body/ratio ceiling      |
| `INSUFFICIENT_FUNDS`   |  409 | Non-negativity guard           |
| `NONCE_CONFLICT`       |  409 | Nonce reservation failed       |
| `IDEMPOTENT_REPLAY`    |  200 | Prior receipt (same body)      |
| `BUSY`                 |  429 | Backpressure (queue full)      |
| `RETRY_LATER`          |  503 | Degraded/readiness false       |
| `UPSTREAM_UNAVAILABLE` |  503 | Ledger/Auth/Policy unavailable |

> **Determinism:** same input + same `Idempotency-Key` ⇒ **byte-identical** receipt.

#### 1.2.4 Headers

* **Required on POST:** `Idempotency-Key` (ULID/UUID; ≤ 64 bytes).
* **Always accepted:** `X-Corr-ID` (echoed), `Content-Encoding` (subject to ratio cap), `Accept: application/json`.

#### 1.2.5 Examples

```bash
# Transfer
curl -sS -X POST https://wallet/api/v1/transfer \
  -H "Authorization: Bearer <macaroon>" \
  -H "Idempotency-Key: 01JFA1KQ2Q9G2VE8W7" \
  -H "Content-Type: application/json" \
  -d '{"from":"acc_src","to":"acc_dst","asset":"ron","amount_minor":"250000","nonce":42}'
```

```bash
# Balance
curl -sS "https://wallet/api/v1/balance?account=acc_dst&asset=ron" \
  -H "Authorization: Bearer <macaroon>"
```

---

## 2. SemVer Discipline

### Service-facing (HTTP)

**Minor / Non-breaking (safe to do):**

* Add new **optional** fields to responses.
* Add new endpoints or query params with defaults.
* Add new error **codes** while keeping existing behavior.

**Breaking (requires major version bump & `/v2`):**

* Change required request fields or types.
* Remove fields or change semantics.
* Change error **codes**/HTTP statuses for existing cases.
* Make idempotency behavior less permissive (e.g., rejecting previously accepted keys).

**Patch:**

* Documentation clarifications, performance improvements, additional metrics, log/trace fields.

### Rust-facing

* Goal is **no** public Rust surface. If any appears, treat changes under standard Rust SemVer rules, but prefer to remove exposure.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0`.
* **Idempotency:** POSTs are idempotent with `Idempotency-Key` within configured TTL.
* **Error determinism:** Error code taxonomy is stable; messages are informational.
* **Amounts:** Always string-encoded `u128` in **minor units**; never floats.
* **Auth:** Capability tokens (macaroons) must include correct scope (`issue|transfer|burn|read`), TTL, and account/asset caveats.

---

## 4. Invariants → API Mapping

* **No doublespends (I-1):** `nonce` required for debit-side ops; 409 on conflict; replays return same receipt.
* **Non-negativity (I-2):** `INSUFFICIENT_FUNDS` (409).
* **Conservation (I-3):** Committed receipt reflects append-only ledger entries; `receipt_hash` ties to batch proof.
* **Capability-only (I-4):** 401/403 on missing/insufficient scope.
* **DTO hygiene (I-5):** `deny_unknown_fields` ⇒ 400 `BAD_REQUEST`.
* **Transport bounds (I-11):** 413 for oversize; 400 for ratio cap/decompress errors; 429 for queue backpressure.

---

## 5. Tooling

* **OpenAPI**: source of truth — `docs/openapi/svc-wallet.yaml`.

  * Validated in CI via `openapi-cli`/`spectral`.
  * Contract tests ensure handler ↔ spec parity.

* **Rust surface**: `cargo public-api --deny-changes`.

* **SemVer checks**: optional `cargo semver-checks` on any library subcrates.

* **Doc tests**: curl examples validated via CI smoke (against local harness).

---

## 6. CI & Gates

* **Gate A:** OpenAPI validation passes; diffs are reviewed on PR (bot comment with `/openapi diff`).
* **Gate B:** `cargo public-api` shows **no new public Rust symbols** unless explicitly approved.
* **Gate C:** Backward compatibility tests (golden receipts, error codes) pass.
* **Gate D:** CHANGELOG contains an entry for **any OpenAPI diff**.
* **Gate E:** Idempotency replay suite proves byte-identical receipts.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current **OpenAPI** generated & committed (`docs/openapi/svc-wallet.yaml`).
* [ ] Rust public API snapshot: empty/minimal; CI gate green.
* [ ] Error code table reflected in both docs & handlers.
* [ ] `Idempotency-Key` enforced on POST routes.
* [ ] Examples (`curl`) tested against local harness.
* [ ] CHANGELOG updated for any contract change.
* [ ] Versioned path (`/v1`) correct; plans for `/v2` noted if needed.

---

## 8. Appendix

### 8.1 Minimal OpenAPI skeleton (drop-in)

```yaml
openapi: 3.0.3
info:
  title: svc-wallet API
  version: "1.0.0"
servers:
  - url: https://wallet
paths:
  /v1/balance:
    get:
      operationId: getBalance
      parameters:
        - in: query; name: account; required: true; schema: { type: string }
        - in: query; name: asset;   required: true; schema: { type: string }
      security: [ { bearerAuth: [] } ]
      responses:
        "200": { description: OK, content: { application/json: { schema: { $ref: "#/components/schemas/Balance" }}}}
        "401": { $ref: "#/components/responses/Unauthorized" }
  /v1/transfer:
    post:
      operationId: postTransfer
      security: [ { bearerAuth: [] } ]
      parameters:
        - in: header; name: Idempotency-Key; required: true; schema: { type: string, maxLength: 64 }
      requestBody:
        required: true
        content: { application/json: { schema: { $ref: "#/components/schemas/TransferReq" }}}
      responses:
        "200": { description: OK, content: { application/json: { schema: { $ref: "#/components/schemas/Receipt" }}}}
        "400": { $ref: "#/components/responses/BadRequest" }
        "401": { $ref: "#/components/responses/Unauthorized" }
        "403": { $ref: "#/components/responses/Forbidden" }
        "409": { $ref: "#/components/responses/Conflict" }
        "429": { $ref: "#/components/responses/TooManyRequests" }
        "503": { $ref: "#/components/responses/ServiceUnavailable" }
components:
  securitySchemes:
    bearerAuth: { type: http, scheme: bearer, bearerFormat: JWT }
  schemas:
    AmountU128: { type: string, pattern: "^[0-9]+$" }
    TransferReq:
      type: object
      additionalProperties: false
      required: [from, to, asset, amount_minor, nonce]
      properties:
        from: { type: string }
        to: { type: string }
        asset: { type: string }
        amount_minor: { $ref: "#/components/schemas/AmountU128" }
        nonce: { type: integer, format: int64, minimum: 1 }
    Receipt:
      type: object
      required: [txid, op, asset, amount_minor, ts, receipt_hash]
      properties:
        txid: { type: string }
        op: { type: string, enum: [issue, transfer, burn] }
        from: { type: string }
        to: { type: string }
        asset: { type: string }
        amount_minor: { $ref: "#/components/schemas/AmountU128" }
        nonce: { type: integer, format: int64 }
        idem: { type: string }
        ts: { type: string, format: date-time }
        receipt_hash: { type: string }
    Balance:
      type: object
      required: [account, asset, amount_minor, as_of]
      properties:
        account: { type: string }
        asset: { type: string }
        amount_minor: { $ref: "#/components/schemas/AmountU128" }
        as_of: { type: string, format: date-time }
        stale_ms: { type: integer }
  responses:
    BadRequest:         { description: Bad request }
    Unauthorized:       { description: Unauthorized }
    Forbidden:          { description: Forbidden }
    Conflict:           { description: Conflict }
    TooManyRequests:    { description: Rate limited }
    ServiceUnavailable: { description: Not ready / degraded }
```

### 8.2 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 8.3 Perfection Gates tie-in

* **Gate G:** No undocumented API surface (Rust + HTTP).
* **Gate H:** Breaking HTTP changes require `/v2` and major bump.
* **Gate J:** CHANGELOG alignment enforced for any API diff.

### 8.4 History

* `v1` (2025-10-16): Initial stable surface with idempotent POSTs, deterministic errors, and receipts.

---

**TL;DR:** `svc-wallet` is a **binary service**. Keep the Rust surface **private**, and treat the **HTTP contract** as the public API, with idempotent writes, deterministic errors, and OpenAPI as the source of truth.
