# 🔗 INTEROP.md — `svc-wallet`

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-wallet`:

* Wire protocols & message formats (**HTTP/1.1 over TLS 1.3**; optional **UDS**).
* DTOs & schemas (JSON, **string-encoded `u128` amounts**, strict request decoding).
* Bus topics and events (audit, cache invalidation, readiness).
* **Canonical test vectors** (requests, headers, responses, receipts).
* **Error taxonomy** and behavioral guarantees (idempotency, nonces, conservation).

This keeps integrations consistent with **GMI-1.6 Omni-Gate** and the wallet’s IDB/CONFIG/SECURITY/CONCURRENCY/OBSERVABILITY docs.

---

## 1) Protocols & Endpoints

### 1.1 Transport (ingress)

* **HTTP/1.1 + TLS 1.3** via `tokio-rustls` (service may also run behind a terminating edge).

  * **HTTP/2** is permitted **when** an edge proxy terminates and preserves HTTP semantics.
  * **WebSocket:** **not supported** (no subscription endpoints).
* **UDS (Unix Domain Socket)** (optional) for on-host admin plane (metrics/readyz).
* **Content:** `Content-Type: application/json; charset=utf-8`
* **Compression:** Accepts `Content-Encoding: gzip` subject to **decompression ratio cap** (≤10×).

### 1.2 Endpoints (authoritative contract)

Base path: `/v1`

| Method | Path            | Auth Scope | Idempotent | Notes                                                   |
| -----: | --------------- | ---------- | ---------- | ------------------------------------------------------- |
|    GET | `/v1/balance`   | `read`     | Yes        | `?account=<str>&asset=<str>`; bounded staleness window. |
|   POST | `/v1/issue`     | `issue`    | Via header | Requires `Idempotency-Key`.                             |
|   POST | `/v1/transfer`  | `transfer` | Via header | Requires `Idempotency-Key`.                             |
|   POST | `/v1/burn`      | `burn`     | Via header | Requires `Idempotency-Key`.                             |
|    GET | `/v1/tx/{txid}` | `read`     | Yes        | Short receipt lookup.                                   |
|    GET | `/healthz`      | none       | Yes        | Liveness only.                                          |
|    GET | `/readyz`       | none       | Yes        | Readiness gate (writes fail-closed during degradation). |
|    GET | `/metrics`      | none/UDS   | Yes        | Prometheus exposition; prefer loopback/UDS.             |

**Auth:** `Authorization: Bearer <macaroon>` (capability token via `ron-auth`).
**Idempotency:** `Idempotency-Key: <uuid/ulid>` is **required** on all **POST** write operations.
**Correlation:** `X-Corr-ID` accepted and echoed; propagated to upstreams as `x-corr-id`.

### 1.3 Transport invariants

* `max_body_bytes = 1 MiB` → `413 Payload Too Large`.
* `max_decompress_ratio = 10x` → `400 Bad Request` on violation.
* `read_timeout = 5s`, `write_timeout = 5s`, total handler deadline ≤ `5s`.
* Ingress → **bounded work queue** (`mpsc(512)`) → **429 Busy** with `Retry-After` when saturated.
* TLS: **rustls** only; no legacy/renegotiation; mTLS optional **at the edge**.

---

## 2) DTOs / Schemas

> **Encoding:** JSON (UTF-8).
> **Amounts:** **string-encoded `u128`** in **minor units**; **no floats**.
> **Schema hygiene:** all **request** DTOs use `serde(deny_unknown_fields)` (reject unknowns). Responses may add fields over time; SDKs **must** ignore unknown response fields.

### 2.1 Requests

```rust
// POST /v1/issue
struct IssueReq {
  to: String,             // account
  asset: String,          // e.g., "ron"
  amount_minor: String,   // string u128 (>0, ≤ max_amount_per_op)
  nonce: u64,             // per-account, strictly monotonic
}

// POST /v1/transfer
struct TransferReq {
  from: String,
  to: String,
  asset: String,
  amount_minor: String,
  nonce: u64,
}

// POST /v1/burn
struct BurnReq {
  from: String,
  asset: String,
  amount_minor: String,
  nonce: u64,
}

// GET /v1/balance?account=...&asset=... (query params)
```

### 2.2 Responses

```rust
// Common short receipt (issue/transfer/burn)
struct Receipt {
  txid: String,            // "tx_<ulid>"
  op: String,              // "issue"|"transfer"|"burn"
  from: Option<String>,
  to: Option<String>,
  asset: String,
  amount_minor: String,    // string u128
  nonce: Option<u64>,      // present if client supplied nonce
  idem: Option<String>,    // echoed Idempotency-Key
  ts: String,              // RFC3339 UTC
  receipt_hash: String,    // "b3:<hex>" — see §2.4
}

// GET /v1/balance
struct Balance {
  account: String,
  asset: String,
  amount_minor: String,    // string u128
  as_of: String,           // RFC3339 UTC
  stale_ms: u32            // effective staleness bound used on read
}

// Deterministic error
struct ApiError {
  code: String,            // enum (see §5)
  http: u16,
  message: String,         // human-readable; do not parse
  retryable: bool,
  corr_id: String,         // echoed correlation id
  details: Option<serde_json::Value>
}
```

### 2.3 Capability Token (macaroon) — minimal claims

* `iss` (issuer), `aud` (service), `exp` (expiry)
* `scope`:
  `{ actions: ["issue"|"transfer"|"burn"|"read"], accounts: [..], assets: [..], ceilings: { daily: "<u128>" } }`

> Nonces are **not** inside the token; they are enforced by `svc-wallet` per account.

### 2.4 Receipt hash canonicalization (auditor-ready)

* Construct **canonical JSON** over keys (sorted, compact separators):
  `{txid, op, from?, to?, asset, amount_minor, nonce?, idem?, ts}`
* `receipt_hash = "b3:" + hex( BLAKE3( utf8(canonical_json) ) )`

**Worked demo (procedure):**

1. Build the object with the exact fields shown above (omit `from/to/nonce/idem` if logically absent).
2. Serialize with **sorted keys** and **no spaces** (e.g., JSON separators `,` and `:`).
3. Run **BLAKE3** over the resulting bytes; hex-encode and prefix with `b3:`.

*A machine-readable example with inputs and expected `receipt_hash` is stored at:*
`/tests/vectors/receipt_hash.json`
*(contains the canonical JSON bytes and expected hex for SDK self-tests).*

---

## 3) Bus Topics

### 3.1 Events Published (best-effort, lossy-OK)

**Format:** `application/json` line events.

* `wallet.transfer.accepted`

  ```json
  {"txid":"tx_...","from":"acc_src","to":"acc_dst","asset":"ron","amount_minor":"250000","nonce":42,"idem":"01JFA...","ts":"2025-10-16T16:11:02Z"}
  ```
* `wallet.issue.accepted` / `wallet.burn.accepted`
  (same shape; `from` or `to` may be omitted accordingly)
* `wallet.request.rejected`

  ```json
  {"op":"transfer","reason":"BUSY","http":429,"corr_id":"01JFA...","ts":"2025-10-16T16:12:07Z"}
  ```
* `wallet.balance.invalidated`

  ```json
  {"account":"acc_dst_hash","asset":"ron","reason":"ledger_change","ts":"2025-10-16T16:12:10Z"}
  ```
* `kernel.health`

  ```json
  {"service":"svc-wallet","ok":true,"ts":"2025-10-16T16:12:10Z"}
  ```

> **Durability:** Bus is **not** durable audit; authoritative truth is `ron-ledger`.

### 3.2 Events Subscribed

* `config.updated` → reload config snapshot; recompute readiness.
* `bus.shutdown` → graceful drain/abort sequence.
* `ledger.balance.changed` → invalidate balance cache entries.

---

## 4) Canonical Test Vectors

*(Machine-readable copies live under `/tests/vectors/`)*

### 4.1 Idempotent Transfer (happy path)

**Request**

```
POST /v1/transfer
Authorization: Bearer <macaroon>
Idempotency-Key: 01JFA1KQ2Q9G2VE8W7
Content-Type: application/json

{"from":"acc_src","to":"acc_dst","asset":"ron","amount_minor":"250000","nonce":42}
```

**Response (first attempt)**

```json
{
  "txid":"tx_01JFA7Z2A7YQ4QW3EJ7N3N6D1X",
  "op":"transfer",
  "from":"acc_src",
  "to":"acc_dst",
  "asset":"ron",
  "amount_minor":"250000",
  "nonce":42,
  "idem":"01JFA1KQ2Q9G2VE8W7",
  "ts":"2025-10-16T16:11:02Z",
  "receipt_hash":"b3:<hex>"
}
```

**Response (exact replay with same `Idempotency-Key`)**
→ **200 OK** with **byte-identical** body (increments `wallet_idem_replays_total`).

### 4.2 Nonce Conflict (409)

Body differs (e.g., different `to`) but `(account, nonce)` already reserved:

```json
{"code":"NONCE_CONFLICT","http":409,"message":"nonce conflict","retryable":false,"corr_id":"01JFA..."}
```

### 4.3 Insufficient Funds (409)

```json
{"code":"INSUFFICIENT_FUNDS","http":409,"message":"insufficient funds","retryable":false,"corr_id":"01JFA..."}
```

### 4.4 Limits — Body Cap (413)

```json
{"code":"LIMITS_EXCEEDED","http":413,"message":"payload too large","retryable":false,"corr_id":"01JFA..."}
```

### 4.5 Degraded Readiness (503)

```json
{"code":"RETRY_LATER","http":503,"message":"service not ready","retryable":true,"corr_id":"01JFA..."}
```

Header: `Retry-After: 2`

### 4.6 Balance Read (200)

```
GET /v1/balance?account=acc_dst&asset=ron
Authorization: Bearer <macaroon>
```

```json
{"account":"acc_dst","asset":"ron","amount_minor":"250000","as_of":"2025-10-16T16:12:10Z","stale_ms":120}
```

### 4.7 Issue (200) — example

```
POST /v1/issue
Authorization: Bearer <macaroon>
Idempotency-Key: 01JFA-ISSUE-KEY
Content-Type: application/json

{"to":"acc_dst","asset":"ron","amount_minor":"1000000","nonce":1}
```

```json
{"txid":"tx_01JFAISSUE...","op":"issue","to":"acc_dst","asset":"ron","amount_minor":"1000000","nonce":1,"idem":"01JFA-ISSUE-KEY","ts":"2025-10-16T16:05:00Z","receipt_hash":"b3:<hex>"}
```

### 4.8 Burn (200) — example

```
POST /v1/burn
Authorization: Bearer <macaroon>
Idempotency-Key: 01JFA-BURN-KEY
Content-Type: application/json

{"from":"acc_src","asset":"ron","amount_minor":"50000","nonce":43}
```

```json
{"txid":"tx_01JFABURN...","op":"burn","from":"acc_src","asset":"ron","amount_minor":"50000","nonce":43,"idem":"01JFA-BURN-KEY","ts":"2025-10-16T16:06:00Z","receipt_hash":"b3:<hex>"}
```

### 4.9 Malformed DTO → BAD_REQUEST (400)

Unknown field triggers strict decoder:

```json
{"from":"acc_src","to":"acc_dst","asset":"ron","amount_minor":"250000","nonce":42,"oops":"unexpected"}
```

---

## 5) Error Taxonomy (wire-stable)

| `code`                 |    HTTP | Retryable | Description                                               |
| ---------------------- | ------: | :-------: | --------------------------------------------------------- |
| `BAD_REQUEST`          |     400 |     ✗     | Malformed JSON, schema violation (`deny_unknown_fields`). |
| `UNAUTHORIZED`         |     401 |     ✗     | Missing/invalid capability token.                         |
| `FORBIDDEN`            |     403 |     ✗     | Capability scope denied by policy.                        |
| `LIMITS_EXCEEDED`      | 403/413 |     ✗     | Amount/policy or transport limits exceeded (body/ratio).  |
| `INSUFFICIENT_FUNDS`   |     409 |     ✗     | Non-negativity guard.                                     |
| `NONCE_CONFLICT`       |     409 |     ✗     | `(account, nonce)` already reserved/committed.            |
| `IDEMPOTENT_REPLAY`    |     200 |     —     | Prior receipt (same body + same `Idempotency-Key`).       |
| `BUSY`                 |     429 |     ✓     | Queue saturated; `Retry-After` provided.                  |
| `UPSTREAM_UNAVAILABLE` |     503 |     ✓     | Ledger/Auth/Policy down or breaker open.                  |
| `RETRY_LATER`          |     503 |     ✓     | Readiness false (shed writes; reads may remain green).    |
| `INTERNAL_ERROR`       |     500 |     ✗     | Unexpected server failure; sensitive details redacted.    |

> Clients **must** branch on `code` (not `message`). Messages are human-readable and non-stable.

---

## 6) Interop Guarantees

* **SemVer:** Breaking wire/schema changes require a **major** bump and `/v{n}` path.
* **Idempotency:** Same `Idempotency-Key` and **semantically identical** request body ⇒ **byte-identical** receipt within TTL.
* **Nonces:** Per-account strictly monotonic; reservation is **linearizable** (atomic) to prevent doublespends.
* **Unknown fields:**

  * **Requests:** rejected (hard fail) to prevent silent drift.
  * **Responses:** ignore unknowns (enable additive evolution).
* **Transport bounds:** Caps enforced **before** upstream calls (defense-in-depth).
* **Auditability:** Deterministic `receipt_hash` derivation (see §2.4) and canonical vectors in `/tests/vectors/`.
* **Bus semantics:** Best-effort broadcast; do **not** rely on it for durable audit (use `ron-ledger`).
* **PQ posture:** When `pq.mode=hybrid`, capability verification may use PQ-hybrid signatures. **Wire format, DTOs, and error taxonomy are unchanged.**

---

## 7) References

* **Interop Blueprint GMI-1.6** — `../../docs/Interop_Blueprint.md`
* **IDB (svc-wallet)** — invariants & proof gates
* **CONFIG.md** — timeouts, caps, retry/circuit breaker, PQ posture
* **CONCURRENCY.md** — backpressure and readiness semantics
* **OBSERVABILITY.md** — metrics, tracing, correlation
* **SECURITY.md** — auth/capabilities, redaction, amnesia
* **API.md** — HTTP surface & OpenAPI

---

### ✅ Notes for SDK Authors

* Always include **`Idempotency-Key`** on POSTs; retry safely on `BUSY`/`RETRY_LATER` with exponential backoff + jitter.
* Handle **amounts** as big integers (`BigInt`/decimal) parsed from strings; never use floats.
* Preserve and propagate **`X-Corr-ID`**; log both `code` and `corr_id` on failures.
* Pre-validate capability scopes client-side where feasible to reduce 403s.
* Recompute and store **`receipt_hash`** for audit; adhere to canonicalization rules in §2.4.
* Treat **`INTERNAL_ERROR`** as non-retryable without operator guidance (page/alert).

---

With these updates (HTTP/2 note, WebSocket N/A, PQ posture, bus payload schemas, complete vectors including **issue/burn/malformed**, and a **reserved `INTERNAL_ERROR`**), `svc-wallet`’s interop contract is **precise, provable, and exhaustive**—ready for SDKs, auditors, and platform integration.
