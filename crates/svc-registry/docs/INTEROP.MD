
# 🔗 INTEROP.md — svc-registry

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **wire-level interop surface** of `svc-registry`:

* Protocols & endpoints (HTTP/S + SSE; UDS optional)
* DTOs & schemas (descriptor sets, approvals, signed artifacts)
* Bus topics & event shapes
* Canonical behaviors & test vectors
* Versioning & compatibility rules

All of this is aligned with the project’s **Omni-Gate / GMI-1.6** guidance: stable contracts, content-addressed payloads, and zero ambient authority.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

| Surface          | Transport                             | Media Types                | Notes                               |
| ---------------- | ------------------------------------- | -------------------------- | ----------------------------------- |
| HTTP/1.1         | TCP + optional TLS 1.3 (tokio-rustls) | `application/json` (UTF-8) | Primary API                         |
| HTTP/1.1 (SSE)   | TCP + TLS                             | `text/event-stream`        | Lossy event stream for updates      |
| UDS (local)      | Unix Domain Socket                    | `application/json` / SSE   | Enforced `SO_PEERCRED` allow-list   |
| (Optional) OAP/1 | N/A for registry v1                   | —                          | Not used in v1 (reserve for future) |

**Transport invariants**

* **Max request body:** `1 MiB` (pre-inflate); **decompress ratio cap:** `≤ 10x` and post-inflate absolute cap.
* **Read/write/idle timeouts:** 5s/5s/60s (configurable).
* **TLS:** only via `tokio_rustls::rustls::ServerConfig`.

### 1.2 Exposed Endpoints (authoritative)

| Method | Path                                | Auth                        | Semantics                                                     |
| ------ | ----------------------------------- | --------------------------- | ------------------------------------------------------------- |
| `GET`  | `/version`                          | none                        | Build info, schema versions, deprecations                     |
| `GET`  | `/healthz`                          | none                        | Liveness                                                      |
| `GET`  | `/readyz`                           | none                        | Readiness (write vs read readiness exposed)                   |
| `GET`  | `/registry/head`                    | none                        | Current `{version, payload_b3, committed_at}`                 |
| `GET`  | `/registry/{version}`               | none                        | Immutable descriptor set at version                           |
| `GET`  | `/registry/stream`                  | none                        | SSE of `{version, payload_b3, committed_at, corr_id}` (lossy) |
| `POST` | `/registry/proposals`               | macaroon `registry:propose` | Stage a proposal (payload + hash)                             |
| `POST` | `/registry/approvals/{proposal_id}` | macaroon `registry:approve` | Submit a signer approval                                      |
| `POST` | `/registry/commit/{proposal_id}`    | macaroon `registry:commit`  | Finalize commit after quorum                                  |
| `POST` | `/registry/supersede`               | governance scopes           | Administrative supersede (rare)                               |

**Headers**

* `X-Corr-ID` (ingress/egress) — correlation ID (ULID/UUID).
* `Deprecation`, `Sunset` — surfaced per API policy.
* `Cache-Control: no-store` on writes; immutable GETs may set `Cache-Control: max-age`.

---

## 2) DTOs / Schemas

### 2.1 Descriptor Set (wire schema)

```json
{
  "schema_version": "1.0.0",
  "version": 42,
  "items": [ /* ron-proto descriptors (services, nodes, regions, etc.) */ ],
  "prev_hash": "b3:<hex>",        // optional for genesis
  "payload_b3": "b3:<hex>",       // BLAKE3 over canonicalized payload bytes
  "approvals": [
    {
      "signer_id": "org:alpha#key1",
      "algo": "ed25519",
      "sig": "base64",            // signature over payload_b3 (and fixed context string)
      "signed_at": "2025-10-08T18:20:00Z"
    }
  ],
  "committed_at": "2025-10-08T18:22:11Z"
}
```

**Encoding rules**

* **JSON** (canonicalized for hashing): UTF-8, stable field order for hash preimage via a fixed canonicalization procedure (e.g., RFC8785-style rules).
* `payload_b3` = `BLAKE3(canonical_payload_bytes)`.
* `sig` = signature over: `context || payload_b3` where `context = "ron:svc-registry:v1\n"`.
* Decoders use `serde(deny_unknown_fields)`. Unknown fields → **reject** (prevents silent drift).

### 2.2 Approval

```json
{
  "signer_id": "org:<realm>#<key-id>",
  "algo": "ed25519",
  "sig": "base64",
  "signed_at": "RFC3339"
}
```

* Multiple approvals allowed per proposal; duplicates by `(signer_id, proposal)` rejected.

### 2.3 Proposal

```json
{
  "schema_version": "1.0.0",
  "payload": { /* descriptor set without approvals/committed_at */ },
  "payload_b3": "b3:<hex>" // optional; server recomputes and must match if supplied
}
```

### 2.4 Event (SSE)

```
event: registry.update
data: {"version":43,"payload_b3":"b3:...","committed_at":"...","corr_id":"01J..."}
```

* **Lossy**: if your client lags, you **must** reconcile using `/registry/head` and then request any missing versions.

---

## 3) Bus Topics (Kernel)

### 3.1 Published

| Topic                   | Shape (JSON)                                                                                  | Semantics                          |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------- |
| `registry.updated`      | `{"version":u64,"payload_b3":"b3:...","committed_at":"timestamp","corr_id":"..."}`            | Emitted post-commit; non-blocking  |
| `kernel.health`         | `{"service":"svc-registry","ok":bool,"components":{"storage":bool,"bus":bool,"commit":bool}}` | Health heartbeat                   |
| `kernel.config.updated` | `{"service":"svc-registry","version":u64}`                                                    | Emitted after config snapshot swap |

### 3.2 Subscribed

| Topic                   | Action                                          |
| ----------------------- | ----------------------------------------------- |
| `kernel.shutdown`       | Begin graceful drain then stop                  |
| `kernel.config.updated` | (Optional) reload if addressed to this instance |

> **Backpressure:** bus publishes are non-blocking; drops are counted (`bus_overflow_dropped_total`) and **must** be observable by consumers through missed sequence detection (reconcile via `/registry/head`).

---

## 4) Canonical Test Vectors

> These vectors pin **behavior**, not private secrets. Store them under `/tests/vectors/registry/`.

### 4.1 Correlation & Headers

* **Ingress**:
  Request `GET /registry/head` with `X-Corr-ID: 01J00000000000000000000000`.
  Response **must** echo the same header.

### 4.2 Minimal Proposal → Approval → Commit

* **Proposal request** (truncated for brevity):

```json
{
  "schema_version":"1.0.0",
  "payload": {
    "version": 100,
    "items": [
      {"kind":"service","id":"svc:alpha","endpoint":"https://alpha.local","meta":{}}
    ],
    "prev_hash": "b3:0000000000000000000000000000000000000000000000000000000000000000"
  }
}
```

* **Server acceptance**:

```json
{ "proposal_id":"p_test01","payload_b3":"b3:<computed>", "expires_at":"<iso8601>" }
```

* **Approval (ed25519)**:

```json
{
  "signer_id":"org:alpha#key1",
  "algo":"ed25519",
  "sig":"<base64(ed25519_sign('ron:svc-registry:v1\n' || payload_b3))>",
  "signed_at":"2025-10-08T18:20:00Z"
}
```

* **Commit response**:

```json
{
  "version":100,
  "payload_b3":"b3:<same-as-proposal>",
  "committed_at":"2025-10-08T18:22:11Z"
}
```

> **Note:** Clients **must not** rely on field order; servers **must** compute `payload_b3` over a canonical byte sequence. Provide a helper in SDKs: `compute_payload_b3(payload) -> "b3:<hex>"`.

### 4.3 SSE Gap Handling

* Start at `version=100`, receive an event for `version=102` (missed 101). Client **must** call `/registry/head`, then GET `/registry/101` if needed, or accept head if immutable history is acceptable for its use case.

---

## 5) Error Taxonomy (wire contract)

Uniform error envelope (HTTP status conveys class; `code` is stable for automations):

| HTTP | `error.code`                                 | Meaning                         | Client Action                 |
| ---- | -------------------------------------------- | ------------------------------- | ----------------------------- |
| 400  | `Malformed`                                  | Invalid JSON/fields/size/ratio  | Fix request                   |
| 401  | `Unauthorized`                               | Missing/invalid macaroon        | Acquire/refresh token         |
| 403  | `Forbidden`                                  | Capability lacks required scope | Use correct scope             |
| 409  | `ChainMismatch`                              | HEAD changed; CAS failed        | Refetch head; reconsider      |
| 409  | `DuplicateApproval`                          | Same signer approval replay     | Stop replay                   |
| 422  | `InvalidSig` | `RevokedKey` | `QuorumFailed` | Crypto/governance errors        | Correct keys or gather quorum |
| 429  | `Busy`                                       | Queue full/backpressure         | Backoff + retry with jitter   |
| 503  | `NotReady`                                   | Readiness gate failed           | Retry after `Retry-After`     |

Error envelope:

```json
{
  "error": {
    "code":"InvalidSig",
    "message":"signature verification failed",
    "corr_id":"01J...",
    "details": { "signer_id":"org:alpha#key1" }
  }
}
```

---

## 6) Interop Guarantees

* **Schema versioning:** `schema_version: "1.x.y"` in payloads. Minor/patch are **additive only**. Breaking changes bump **major** and a new endpoint path **may** be introduced (e.g., `/v2/registry/...`) during migration windows.
* **Content addressing:** `payload_b3` is deterministic for identical `payload`. Any server recomputation **must** match client-provided value when present.
* **Quorum enforcement:** Commits require M-of-N per the signed signer-set artifact. Config **cannot** override this.
* **No kernel drift:** Bus topic names and event shapes are frozen for a major; additions are additive.
* **Backcompat on reads:** New fields added to responses are optional; clients **must** ignore unknown fields. Requests remain strict (`deny_unknown_fields`) to avoid silent drift in write paths.
* **Auditability:** Canonical vectors & OpenAPI live under `/docs/openapi/registry.yaml` and `/tests/vectors/`.

---

## 7) Field & Header Reference (cheat sheet)

* **Headers:** `X-Corr-ID`, `Authorization: Bearer <macaroon>`, `Deprecation`, `Sunset`, `Retry-After`.
* **Stable fields:** `version (u64)`, `payload_b3 ("b3:<hex>")`, `prev_hash ("b3:<hex>")`, `committed_at (RFC3339)`.
* **Approval keys:** `signer_id (string)`, `algo ("ed25519")`, `sig (base64)`, `signed_at (RFC3339)`.

---

## 8) SDK Guidance (external)

* Provide helpers:

  * `compute_payload_b3(payload_json_bytes) -> "b3:<hex>"`
  * `sign_approval(signer_key, payload_b3) -> Approval`
  * `stream_updates(starting_version, on_gap=ReconcileWithHead)`
* Always set/propagate `X-Corr-ID`.
* Treat **429/503** with exponential backoff + jitter; respect `Retry-After`.

---

## 9) References

* **Omni-Gate / GMI-1.6** (Interop Blueprint)
* **OBSERVABILITY.md** — correlation IDs, metrics, and span names
* **CONCURRENCY.md** — readiness semantics & backpressure
* **SECURITY.md** — macaroon scopes, UDS allow-lists
* **API.md** — complete endpoint descriptions & OpenAPI

---

## 10) Compliance Checklist (PR Gate)

* [ ] Any change to DTO fields documented here + `API.md` + OpenAPI.
* [ ] New bus topics/events documented with schema.
* [ ] Canonical test vectors updated/added.
* [ ] Contract tests cover new/changed surfaces.
* [ ] Backward compatibility assessed (SemVer discipline).

---

✅ This document is the **wire-contract** for `svc-registry`. If your client can produce `payload_b3`, create a valid `Approval`, and handle SSE gaps with head reconciliation, you are interoperable.
