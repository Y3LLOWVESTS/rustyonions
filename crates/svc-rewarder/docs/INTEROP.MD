---

title: 🔗 INTEROP — svc-rewarder
status: draft
msrv: 1.80.0
last-updated: 2025-10-14
audience: developers, auditors, external SDK authors
----------------------------------------------------

# 🔗 INTEROP.md — `svc-rewarder`

## 0) Purpose

Define the **interop surface** of `svc-rewarder` so that internal crates, external SDKs, and third-party integrators can interact without drift:

* **Wire protocols & endpoints** (HTTP/1.1 + TLS as primary).
* **DTOs & schemas** (JSON with strict validation; OpenAPI + JSON Schema).
* **Bus topics & events** (service lifecycle and economic outcomes).
* **Canonical test vectors** (golden inputs/outputs that must round-trip identically).
* **Transport invariants** (timeouts, caps, TLS) aligned with **GMI-1.6 Omni-Gate**.

This document complements: `API.md`, `OBSERVABILITY.md`, `SECURITY.md`, and the repo Interop & Hardening blueprints.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

* **HTTP/1.1 + TLS (rustls)** — primary surface (Axum 0.7).
* **UDS (Unix Domain Socket)** — optional single-host profile; `SO_PEERCRED` allow-list.
* **mTLS** — recommended for intra-plane deployments (econ plane).

> QUIC/gRPC are not currently exposed by `svc-rewarder`. If added later, the schemas, invariants, and canonical vectors here still apply.

### 1.2 Exposed Endpoints (v1)

Base path: `/rewarder`

| Method | Path                                  | Purpose                                                                                  | Auth (capability / scope)                    |
| ------ | ------------------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------- |
| POST   | `/rewarder/epochs/{epoch_id}/compute` | Deterministic compute for an epoch; emits idempotent ledger intent unless `dry_run=true` | `rewarder.run` (tenant-scoped)               |
| GET    | `/rewarder/epochs/{epoch_id}`         | Read sealed manifest & attestation                                                       | `rewarder.inspect`                           |
| GET    | `/rewarder/policy/{policy_id}`        | Fetch signed policy document                                                             | `rewarder.inspect` (or public if configured) |
| GET    | `/healthz`                            | Liveness                                                                                 | none                                         |
| GET    | `/readyz`                             | Readiness JSON                                                                           | none                                         |
| GET    | `/metrics`                            | Prometheus metrics                                                                       | none                                         |
| GET    | `/version`                            | Build info                                                                               | none                                         |
| GET    | `/openapi.json`                       | OpenAPI 3.1 document (machine-readable)                                                  | none                                         |
| GET    | `/schema/compute.json`                | JSON Schema for compute request                                                          | none                                         |

### 1.3 Transport Invariants (enforced)

* **Timeouts:** 5s per request (edge), 2s connect downstream; total budget ≤ 10s.
* **Concurrency limit:** 512 inflight per instance; bounded work queue.
* **Rate limits:** 500 RPS/instance (token bucket; burst 200) + **per-tenant** caps.
* **Payload cap:** 1 MiB request body; **decompression ratio ≤ 10×**; post-inflate cap ≤ 8 MiB.
* **TLS:** `tokio_rustls::rustls::ServerConfig` only; TLS 1.3; cipher suites pinned by deployment policy.
* **Idempotency:** Natural by content triple `(epoch_id, policy_hash, inputs_cid)` → `run_key`.

---

## 2) DTOs / Schemas

All JSON DTOs are strict: `additionalProperties=false` (`serde(deny_unknown_fields)`).

### 2.1 Request — `ComputeRequest` (JSON)

```json
{
  "inputs_cid": "b3:7d9c0f...c1",          // BLAKE3 content ID or gateway handle
  "policy_id": "rev42",                     // human-readable policy version
  "policy_hash": "b3:3aa1f0...f83",         // signed policy content hash
  "dry_run": false,                         // optional, default false
  "notes": "optional operator notes"        // <= 1024 chars
}
```

**Constraints**

* `inputs_cid`: string, min 8; must parse as supported content address (e.g., `b3:<hex>`).
* `policy_id`: non-empty; `policy_hash`: string matching policy.
* `dry_run`: boolean; default `false`.
* `notes`: optional; ≤ 1024 chars.

**JSON Schema (excerpt)**

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "inputs_cid": { "type": "string", "minLength": 8 },
    "policy_id":  { "type": "string", "minLength": 1 },
    "policy_hash":{ "type": "string", "minLength": 8 },
    "dry_run":    { "type": "boolean", "default": false },
    "notes":      { "type": "string", "maxLength": 1024 }
  },
  "required": ["inputs_cid","policy_id","policy_hash"]
}
```

### 2.2 Response — `ComputeResult` (JSON)

```json
{
  "epoch_id": "2025-10-01",
  "run_key": "e91a6e1b1f2c9c0d",
  "commitment": "b3:1ac4...9b0",
  "status": "ok",                          // ok | quarantined | fail
  "totals": {
    "pool_minor_units": "123456789",
    "payout_minor_units": "123456700",
    "residual_minor_units": "89"
  },
  "policy": { "id": "rev42", "hash": "b3:3aa1...f83", "signed": true },
  "invariants": {
    "conservation": true,
    "overflow": false,
    "negative": false,
    "idempotent": true
  },
  "ledger": {
    "emitted": true,                       // false if dry_run or quarantine
    "result": "accepted"                   // accepted | dup | error | none
  },
  "metrics": {
    "cost_estimate_ms": 377,
    "compute_ms": 418
  }
}
```

### 2.3 Manifest — `EpochManifest` (GET `/epochs/{id}`)

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
    "sig_pq": "base64:...",                 // present if PQ enabled
    "signed_at": "2025-10-01T12:34:56Z"
  }
}
```

> **Encoding:** JSON (UTF-8). If/when a binary representation is required, manifests may also be published as **DAG-CBOR** under a content address (same fields and constraints).

---

## 3) Bus Topics

`svc-rewarder` integrates with the kernel bus (tokio broadcast) for service lifecycle and audit notifications. Topic naming is `rewarder.*`.

### 3.1 Events Published

| Topic                      | Payload (JSON)                                                                            | When                                     |                           |
| -------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------- |
| `rewarder.run.started`     | `{ "epoch_id", "tenant", "run_key", "policy_hash", "corr_id" }`                           | Immediately after request admission      |                           |
| `rewarder.run.quarantined` | `{ "epoch_id", "tenant", "run_key", "reason", "corr_id" }`                                | Invariant breach; **no** ledger emit     |                           |
| `rewarder.run.completed`   | `{ "epoch_id", "tenant", "run_key", "status", "ledger_result", "compute_ms", "corr_id" }` | On compute completion                    |                           |
| `rewarder.ledger.emit`     | `{ "epoch_id", "tenant", "run_key", "result", "corr_id" }`                                | After attempting ledger emit             |                           |
| `rewarder.health`          | Kernel-style: `{ "service":"svc-rewarder","ok":true                                       | false }`                                 | Periodic health heartbeat |
| `rewarder.crash`           | `{ "service":"svc-rewarder","reason":"<text>" }`                                          | On fatal error before supervisor restart |                           |

**Cardinality guard:** Do **not** include raw `inputs_cid` or secrets in bus payloads; include `run_key` in **truncated** form only (e.g., 8-byte hex).

### 3.2 Events Subscribed

| Topic               | Action                                                                      |
| ------------------- | --------------------------------------------------------------------------- |
| `config.updated`    | Reload dynamic config (caps, caps for per-tenant RPS, cost caps, endpoints) |
| `bus.shutdown`      | Graceful shutdown (drain queue, stop accepting new work)                    |
| `policy.invalidate` | Future: cache busting for policy registry (if used)                         |

---

## 4) Canonical Test Vectors

Canonical vectors live under: `crates/svc-rewarder/tests/vectors/`.
They must be **stable** across versions unless a documented breaking change occurs.

### 4.1 Run-Key Derivation

* **Input:**
  `epoch_id="2025-10-01"`, `policy_hash="b3:000102..."` (32 bytes), `inputs_cid="b3:abcdef..."`
* **Process:** `run_key = BLAKE3(epoch_id || policy_hash || inputs_cid)` → hex, truncated to 8 bytes for logs.
* **Expected `run_key`:** `"3af8c2e1b9d47210"` (example; include real hex in repo vector).

### 4.2 Conservation (Happy Path)

* **inputs_cid:** `b3:.../happy-001.json`
* **policy_hash:** `b3:.../policy-1.2.0.json`
* **Expect:** `status=ok`, `Σ payouts ≤ pool_total`, `ledger.result="accepted"`.

### 4.3 Idempotency (Duplicate Submit)

* Submit same triple twice.
* **First:** `ledger.result="accepted"`
* **Second:** `ledger.result="dup"`; manifest identical.

### 4.4 Quarantine (Invariant Breach)

* Vector with payouts intentionally exceeding pool.
* **Expect:** HTTP `409`, body `{ "status":"quarantined","reason":"conservation" }`; **no** `ledger.emit`.

### 4.5 Cost Cap Enforcement

* Vector that triggers estimator > `MAX_EPOCH_COST_MS`.
* **Expect:** HTTP `400` with `details.reason="cost_cap"` (or forced `dry_run` if configured).

> Each vector includes: request JSON, expected response JSON, and a short log/metrics snapshot to verify observability alignment.

---

## 5) Error Taxonomy (Wire Semantics)

| HTTP | `error.code`             | `details.reason`                                                | Notes                                  |          |             |               |                |
| ---- | ------------------------ | --------------------------------------------------------------- | -------------------------------------- | -------- | ----------- | ------------- | -------------- |
| 400  | `BAD_REQUEST`            | `schema` / `oversize` / `decompress_cap` / `cost_cap` / `stale` | Validation, caps, or stale policy hash |          |             |               |                |
| 401  | `UNAUTHENTICATED`        | `unauth`                                                        | Missing/invalid capability             |          |             |               |                |
| 403  | `UNAUTHORIZED`           | `scope` / `caveat`                                              | Scope/caveat failure                   |          |             |               |                |
| 409  | `QUARANTINED`            | `invariant` (`conservation                                      | overflow                               | negative | idempotency | dust_policy`) | No ledger emit |
| 429  | `RATE_LIMITED`           | `quota`                                                         | Global or per-tenant cap               |          |             |               |                |
| 503  | `DEPENDENCY_UNAVAILABLE` | `dep_outage` / `backpressure`                                   | `/readyz` degraded                     |          |             |               |                |
| 500  | `INTERNAL`               | —                                                               | Unexpected; includes `corr_id`         |          |             |               |                |

Error JSON envelope is uniform (see `API.md`). Providers **must not** echo secrets or full payloads.

---

## 6) Interop Guarantees

* **Stable Contracts:**

  * JSON field names and types are stable within the **major** API version.
  * Additional fields may be added (additive) and must be ignored by older clients.
* **Idempotency & Determinism:**

  * Given identical `(epoch_id, policy_hash, inputs_cid)`, results are byte-stable; double-emit is prevented by design.
* **Backwards Compatibility:**

  * Unknown fields are ignored on read (never silently **dropped** on write).
  * Deprecations are marked in OpenAPI; removals only in a major version.
* **No Kernel Drift:**

  * Bus event shapes remain compatible; lifecycle topics are versionless and minimal.
* **Auditability:**

  * Canonical vectors live in `/tests/vectors/`; manifests may also be published under content addresses for independent verification.

---

## 7) Security & Transport Notes (Interop-Relevant)

* **Auth:** macaroons with caveats: `tenant`, `method`, `path`, `TTL`, optional IP. Gateway JWTs (legacy) are converted to macaroons internally and never forwarded downstream.
* **TLS:** TLS 1.3 only; rustls server; optional **mTLS** for intra-plane.
* **Compression:** gzip/zstd accepted; ratio cap ≤ 10×; post-inflate cap ≤ 8 MiB; nested archives refused.
* **Secrets & PII:** Never appear in wire payloads, bus events, or logs; identifiers should be opaque tokens or hashes from upstream systems.

---

## 8) OpenAPI & Schema Publishing

* `GET /openapi.json` — authoritative API contract (OpenAPI 3.1).
* `GET /schema/compute.json` — JSON Schema for compute request (useful for client-side validation).
* Schema changes must pass: **OpenAPI diff CI gate** + `CHANGELOG.md` entry.

---

## 9) Reference CLI & SDK Guidance

* **CLI (optional):** If a thin operator CLI is provided (e.g., `ronctl rewarder compute`), it must simply wrap the HTTP endpoints and preserve idempotency semantics.
* **SDKs:**

  * Always send/propagate `X-Corr-ID`.
  * Treat **409 QUARANTINED** as terminal (do not retry).
  * Treat **dup** results as **success** for idempotent replays.
  * Respect `Retry-After` for `429/503`.

---

## 10) Versioning & SemVer (Interop)

* **HTTP API versioning** is bound to OpenAPI version (e.g., `1.0.0`).
* **Additive**: new optional fields/endpoints → **minor** version bump.
* **Breaking**: field removals/renames/type changes or required field additions → **major** version bump.
* **Canonical vectors** are updated only on additive changes or with new folders per major version (e.g., `vectors/v1/`, `vectors/v2/`).

---

## 11) Example Interactions (cURL)

```bash
# Compute
curl -sS -X POST "https://node.example.com/rewarder/epochs/2025-10-01/compute" \
  -H "Authorization: Bearer ${MACAROON}" \
  -H "X-Corr-ID: demo-123" \
  -H "Content-Type: application/json" \
  --data '{"inputs_cid":"b3:7d9c...c1","policy_id":"rev42","policy_hash":"b3:3aa1...f83","dry_run":false}'

# Manifest
curl -sS -H "Authorization: Bearer ${MACAROON}" \
  "https://node.example.com/rewarder/epochs/2025-10-01"

# Policy
curl -sS -H "Authorization: Bearer ${MACAROON}" \
  "https://node.example.com/rewarder/policy/rev42"
```

---

## 12) Files & Locations

* **OpenAPI:** `crates/svc-rewarder/docs/openapi/svc-rewarder.json`
* **JSON Schemas:** `crates/svc-rewarder/docs/schemas/compute.json`
* **Canonical Vectors:** `crates/svc-rewarder/tests/vectors/v1/*.json`
* **Bus Event Fixtures:** `crates/svc-rewarder/tests/vectors/v1/events/*.json`

---

## 13) References

* GMI-1.6 Omni-Gate Interop Blueprint (repo docs)
* `API.md` (endpoint behavior & error taxonomy)
* `OBSERVABILITY.md` (corr_id, metrics names for SDK exemplars)
* `SECURITY.md` (auth, invariants, caps, idempotency)

---

✅ With this spec, `svc-rewarder` exposes a **tight, invariant-aware contract**: stable JSON DTOs; predictable transport behavior; versioned OpenAPI; and runnable canonical vectors to prove compatibility across crates and external SDKs.
