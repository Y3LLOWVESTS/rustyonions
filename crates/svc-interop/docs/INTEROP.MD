

````markdown
# 🔗 INTEROP.md — svc-interop

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-interop`:

* **Wire protocols & endpoints**: canonical HTTP surface; internal OAP/1 framing if/when used; TLS posture.
* **DTOs & schemas**: request/response shapes (deny unknown fields), error taxonomy.
* **Bus topics & events**: what we emit/consume and their payload contracts.
* **Canonical test vectors**: reproducible inputs/outputs for SDKs and conformance tests.

This document keeps `svc-interop` consistent with the **GMI-1.6 Omni-Gate** ethos: **capability-first**, **content-addressed (BLAKE3)**, **bounded/streamed**, and **reversible**.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

- **HTTP/1.1 over TCP** (default); TLS termination typically upstream. If enabled in-proc, only via `tokio_rustls::rustls::ServerConfig`.
- **Content addressing**: Internal addresses are **BLAKE3-256** (`"b3:<hex>"`).
- **Provider signatures @edge**: Third-party webhooks use **HMAC-SHA256** verification **only at ingress**; no SHA digests are persisted.

### 1.2 Exposed Endpoints (external contract)

1) `POST /webhooks/:provider` — signed ingest  
   Providers: `github | stripe | slack_webhook`  
   Body: provider-native JSON/form.  
   Limits: `≤ 1 MiB` body, otherwise stream with `64 KiB` chunks (≤ 2 buffered).

   **Required headers (examples):**
   - GitHub: `X-Hub-Signature-256: sha256=<hex>`
   - Stripe: `Stripe-Signature: t=<ts>,v1=<sig>[,v0=<sig>]`
   - Slack Webhooks: `X-Slack-Signature: v0=<sig>`, `X-Slack-Request-Timestamp: <ts>`

   **Responses**
   - `202 Accepted` — `{ "accepted": true, "corr_id": "<ulid>" }`
   - Error (stable taxonomy): `{ "error": "<human>", "reason": "BAD_ORIGIN|UNAUTH|DECOMP_LIMIT|BODY_LIMIT|RATE_LIMIT|BACKPRESSURE|MALFORMED|DOWNSTREAM_UNAVAILABLE|POLICY_BLOCKED", "corr_id": "<ulid>" }`

2) `POST /put` — normalized content ingress → BLAKE3 address  
   Content-Type: `application/json` or `application/octet-stream`  
   - JSON: `{ "payload": "<opaque|base64>", "meta": { ... } }`  
   - Binary: raw body (streamed as needed)  
   **Response**: `202` `{ "address": "b3:<hex>", "corr_id": "<ulid>" }`

3) `GET /o/:addr` — addressed object fetch  
   `:addr` must match `^b3:[0-9a-fA-F]+$`  
   **Response**: `200` (bytes) or `404/400` (malformed/missing)

4) Ops
   - `GET /healthz` — liveness
   - `GET /readyz` — readiness (brownout degrades bridging first)
   - `GET /metrics` — Prometheus (bind to localhost)
   - `GET /version` — build metadata

### 1.3 Transport Invariants

- **Body/Frame caps**: `max_body = 1 MiB` for non-streaming HTTP; **streaming chunk = 64 KiB**, ≤ 2 buffered at a time.  
- **Decompression guard**: expansion ≤ `10×` and absolute cap respected; reject on overflow.  
- **Timeouts**: `read=5s`, `write=5s`, `idle=60s` (configurable).  
- **No ambient trust**: ingress tokens/signatures never become internal authority; capabilities minted via `svc-passport`, TTL ≤ `60s`.

---

## 2) DTOs / Schemas

> All JSON DTOs use `#[serde(deny_unknown_fields)]` semantics and stable field names. Unknown fields **must** cause rejection unless the DTO is explicitly marked additive-tolerant.

### 2.1 PUT Request (JSON mode)

```json
{
  "payload": "<opaque or base64>",
  "meta": {
    "type": "example",
    "content_encoding": "identity|gzip|zstd",
    "provider": "github|stripe|slack_webhook|none"
  }
}
````

**Rules**

* `payload` is required for JSON mode. If binary, use `application/octet-stream` and omit this DTO.
* `content_encoding` drives safe decompression guardrails (≤10× + absolute cap).
* On accept, the server computes **BLAKE3-256** on the **decompressed canonical bytes** and returns `b3:<hex>`.

**Response (202)**

```json
{
  "address": "b3:<hex>",
  "corr_id": "<ulid>"
}
```

### 2.2 Error Body (stable)

```json
{
  "error": "human-friendly message",
  "reason": "UNAUTH|BAD_ORIGIN|EXPIRED_TOKEN|SCOPE_MISMATCH|BODY_LIMIT|DECOMP_LIMIT|RATE_LIMIT|BACKPRESSURE|DOWNSTREAM_UNAVAILABLE|POLICY_BLOCKED|MALFORMED",
  "corr_id": "<ulid>"
}
```

### 2.3 Webhook Envelope (normalized, internal)

> For auditing and internal re-emission. **Not** exposed publicly.

```json
{
  "provider": "github|stripe|slack_webhook",
  "ts": "<rfc3339>",
  "corr_id": "<ulid>",
  "edge": {
    "sig_alg": "hmac-sha256",
    "verified": true,
    "req_ip": "<redacted>"
  },
  "b3_hash": "b3:<hex>",      // computed from canonical body
  "raw_meta": { "id": "...", "event": "...", "delivery": "..." }
}
```

### 2.4 ObjectManifestV2 (internal content descriptor)

```rust
// Encoding: DAG-CBOR (strict); deny unknown; versioned.
struct ObjectManifestV2 {
  id: String,           // "b3:<hex>"
  size: u64,            // bytes (post-decompression canonical length)
  chunks: Vec<Chunk>,   // 64 KiB; last may be short
}
```

**Validation**

* The concatenated chunk BLAKE3 **must** equal `id`.
* Serving `GET /o/:addr` requires full digest check or a proven equivalent proof.

### 2.5 Capability (passport minted)

```json
{
  "typ": "macaroon",
  "ttl": "60s",
  "scope": ["write:put", "webhook:accept:github"],
  "sig": "<opaque>",
  "aud": "svc-interop",
  "nbf": "<rfc3339>",
  "exp": "<rfc3339>"
}
```

**Rule**: cap verification is mandatory for mutating paths; caps are **not** stored by `svc-interop`.

---

## 3) Bus Topics

> Bus semantics are **at-least-once**; recipients must be idempotent. Topic names are stable; payloads are versioned.

### 3.1 Events Published

* `interop.webhook.accepted.v1`
  **Payload**:

  ```json
  {
    "provider":"github|stripe|slack_webhook",
    "b3_hash":"b3:<hex>",
    "corr_id":"<ulid>",
    "received_at":"<rfc3339>",
    "meta":{"event":"...", "id":"..."}
  }
  ```

* `interop.put.accepted.v1`
  **Payload**:

  ```json
  { "address":"b3:<hex>", "corr_id":"<ulid>", "received_at":"<rfc3339>" }
  ```

* `audit.event.v1`
  **Payload** (stable subset):

  ```json
  { "kind":"allow|deny", "reason":"<taxonomy>", "corr_id":"<ulid>", "b3_hash":"b3:<hex>", "when":"<rfc3339>" }
  ```

* `interop.bridge.brownout.v1`
  **Payload**:

  ```json
  { "active":true, "reason":"p95|error_rate|queue_pressure", "since":"<rfc3339>" }
  ```

### 3.2 Events Subscribed

* `config.updated.v1` — apply config snapshot atomically.
* `bus.shutdown.v1` — graceful shutdown procedure.

**Delivery contracts**

* **No ordering guarantees** across topics.
* **Idempotency**: duplicates can occur; consumers must dedupe via `b3_hash`/`address` + `corr_id`.

---

## 4) Canonical Test Vectors

> Canonical vectors are stored under `/tests/vectors/` and regenerated by CI to prevent drift. This section documents **procedures** and expected **shapes**. Where exact hashes are listed as `TBD_BY_TESTS`, the repo’s generator fills them in.

### 4.1 BLAKE3 digest — “hello world”

* **Input bytes**: ASCII `"hello world"`
* **Procedure**: compute BLAKE3-256 over canonical bytes; prefix with `"b3:"`.
* **Expected**: `b3:TBD_BY_TESTS_HELLO_WORLD`
* **Notes**: repository test `vectors::hello_world_b3()` asserts this value and writes it to `vectors.json`.

### 4.2 Webhook — GitHub ping (edge verify + normalize)

* **Headers**:

  ```
  X-Hub-Signature-256: sha256=<sig>
  Content-Type: application/json
  ```
* **Body**:

  ```json
  { "zen": "Keep it logically awesome.", "hook_id": 123 }
  ```
* **Procedure**:

  1. Verify `HMAC-SHA256` per provider rule (transient; not persisted).
  2. Compute `b3_hash` on canonical JSON bytes (UTF-8, no BOM).
  3. Emit `interop.webhook.accepted.v1`.
* **Expected envelope fields**: `provider=github`, `edge.sig_alg=hmac-sha256`, `verified=true`, `b3_hash="b3:TBD_BY_TESTS_GITHUB_PING"`.

### 4.3 PUT — JSON payload

* **Request**:

  ```json
  { "payload": "Zm9vYmFy", "meta": { "type":"blob", "content_encoding":"identity" } }
  ```

  (payload = `"foobar"` bytes)
* **Response (202)**:

  ```json
  { "address":"b3:TBD_BY_TESTS_FOOBAR", "corr_id":"<ulid>" }
  ```

### 4.4 GET — address fetch (happy & not found)

* **Request**: `GET /o/b3:TBD_BY_TESTS_FOOBAR`
* **Expected**: `200` with `foobar` bytes, `Content-Type: application/octet-stream`.
* **Negative**: `GET /o/b3:deadbeef` → `404`.

> The CI job regenerates `vectors.json` and validates SDKs against it.

---

## 5) Error Taxonomy (wire-stable)

The following **`reason`** codes are **closed-set** and **additive-only**. Changing semantics is **breaking**; adding a new code is **minor** (documented here and in the OpenAPI).

| Reason                   | Typical HTTP | Meaning & Interop Note                                     |
| ------------------------ | ------------ | ---------------------------------------------------------- |
| `UNAUTH`                 | 401          | Capability/token missing or invalid (mutating paths)       |
| `BAD_ORIGIN`             | 400          | Origin/host not in allow-list                              |
| `EXPIRED_TOKEN`          | 401          | Capability outside TTL window                              |
| `SCOPE_MISMATCH`         | 403          | Capability scope insufficient                              |
| `BODY_LIMIT`             | 413          | Body exceeds configured cap                                |
| `DECOMP_LIMIT`           | 400          | Decompression ratio or absolute cap exceeded               |
| `RATE_LIMIT`             | 429          | RPS limiter engaged                                        |
| `BACKPRESSURE`           | 429/503      | Queue saturation or brownout (writes)                      |
| `DOWNSTREAM_UNAVAILABLE` | 503          | Required internal dependency unavailable/slow              |
| `POLICY_BLOCKED`         | 403          | Violates local policy (e.g., SHA256 disallowed internally) |
| `MALFORMED`              | 400          | DTO/schema parse failed (deny-unknown-fields, bad types)   |

**Wire rule**: every non-2xx JSON error includes `{ error, reason, corr_id }`.

---

## 6) Interop Guarantees

* **BLAKE3-only internal**: All internal addresses, manifests, and audit hashes use **BLAKE3-256**. SHA-256 appears only at ingress for **third-party verification** and is not persisted or exposed as an internal identifier.
* **Streaming over buffering**: >1 MiB requests must stream in `64 KiB` chunks, ≤2 buffered; decompression guard enforced.
* **No kernel drift**: bus topics follow the kernel’s naming and versioning conventions; topic payloads are versioned (`.v1`).
* **SemVer discipline**:

  * **HTTP**: breaking changes require a new path version or media-type version; additive fields are OK.
  * **Bus**: breaking payloads use new topic versions (e.g., `.v2`), old topics retained through migration window.
* **Backward compatibility**: Unknown fields are **rejected** for external DTOs (prevents silent drift); internal CBOR manifests use version tags to evolve.
* **Auditability**: Every allow/deny decision emits an `audit.event.v1` with `b3_hash`, `reason`, and `corr_id`.

---

## 7) References

* **Interop Blueprint (GMI-1.6)** — repo docs
* **OAP/1 Spec** — internal framing reference (if adopted for inter-service paths)
* **OBSERVABILITY.md** — correlation IDs, spans, metrics names
* **CONCURRENCY.md** — readiness/brownout semantics and backpressure behavior
* **SECURITY.md** — hashing/signature policy (BLAKE3 internal; SHA-256 edge-only)

---

## Appendix A — OpenAPI Pointers (concise)

See `/docs/openapi/svc-interop.openapi.yaml` for the canonical HTTP contract. Any change here must update that spec and the snapshot under `/docs/api-history/http/`.

---

## Appendix B — Reproducibility Notes

* **Hash vectors** (`TBD_BY_TESTS_*`) are generated by `tests/build_vectors.rs` using the workspace’s pinned `blake3` crate and written to `/tests/vectors/vectors.json`.
* Providers’ signature examples are generated using deterministic fixtures (fixed secrets, timestamps) so SDKs can verify HMAC procedures without internet access.
* CI verifies SDK conformance by replaying vectors and comparing outputs.

```

