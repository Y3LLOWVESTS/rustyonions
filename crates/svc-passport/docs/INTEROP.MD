````markdown
# 🔗 INTEROP.md — svc-passport

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-passport`:

- Wire protocols & endpoints (HTTP/1.1 over TLS via gateway; optional UDS).
- DTOs & schemas (issue/verify/revoke + error envelope).
- Bus topics & event payloads (revocation/policy/config).
- Canonical test vectors (requests, responses, error cases).
- Compatibility guarantees with **GMI-1.6 Omni-Gate** and project canon.

> `svc-passport` **mints** capability-scoped tokens; authoritative verification is in `ron-auth`. Key custody is in `ron-kms`.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

- **HTTP/1.1 + JSON** over **TLS** (typically terminated at `svc-gateway`).
- **UDS** (optional intra-host) with `SO_PEERCRED` allowlist (same JSON).
- All endpoints are **idempotent** except `issue` (mint is effectful but safe under duplicate replay by identical body within TTL boundaries).

### 1.2 Exposed Endpoints (v1)

Base path prefix: `/v1/passport/*`

| Method | Path                   | Purpose                             |
|--------|------------------------|-------------------------------------|
| POST   | `/v1/passport/issue`   | Mint a capability token             |
| POST   | `/v1/passport/verify`  | Preflight parse/echo (non-authz)    |
| POST   | `/v1/passport/revoke`  | Revoke via epoch/key policy         |
| GET    | `/healthz`             | Liveness                            |
| GET    | `/readyz`              | Readiness (issuance may fail-closed)|
| GET    | `/metrics`             | Prometheus                          |

### 1.3 Transport Invariants

- `Content-Type: application/json; charset=utf-8`
- `Cache-Control: no-store`
- **Request body cap:** `≤ 1 MiB`
- **Decompression ratio cap:** `≤ 10×` (when compression used)
- **Timeouts:** `read=5s`, `write=5s` (default)
- **Rate/Inflight caps:** RPS 500, inflight 512 (tunable)
- **Retry-After** header on `429/503`

---

## 2) DTOs / Schemas

> Requests enforce `#[serde(deny_unknown_fields)]`. Responses **may** gain fields; clients MUST ignore unknown response fields.

### 2.1 JSON Schemas (informative)

#### IssueRequest

```json
{
  "type": "object",
  "required": ["subject_ref", "audience", "ttl_s"],
  "additionalProperties": false,
  "properties": {
    "subject_ref": { "type": "string", "minLength": 1 },     // opaque, caller-provided
    "audience":    { "type": "string", "pattern": "^svc-[a-z0-9-]+$" },
    "ttl_s":       { "type": "integer", "minimum": 1 },      // clamped by policy.max_ttl
    "caveats":     { "type": "array", "items": { "type": "string" } },
    "accept_algs": { "type": "array", "items": { "type": "string", "enum": ["ed25519","ed25519+ml-dsa"] } },
    "proof":       { "nullable": true }                      // reserved for future binding
  }
}
````

#### IssueResponse

```json
{
  "type": "object",
  "required": ["token","kid","alg","exp"],
  "additionalProperties": false,
  "properties": {
    "token":   { "type": "string" },                        // opaque, macaroon-style
    "kid":     { "type": "string" },
    "alg":     { "type": "string", "enum": ["ed25519","ed25519+ml-dsa"] },
    "exp":     { "type": "string", "format": "date-time" },
    "caveats": { "type": "array", "items": { "type": "string" } }
  }
}
```

#### VerifyRequest / VerifyResponse (preflight)

```json
// Request
{ "type":"object", "required":["token"], "additionalProperties":false,
  "properties": { "token":{"type":"string"} }
}

// Response
{ "type":"object", "required":["ok","parsed"],
  "properties": {
    "ok": { "type":"boolean" },
    "parsed": { "type":"object",
      "properties": {
        "alg":{"type":"string"}, "kid":{"type":"string"}, "epoch":{"type":"integer"},
        "aud":{"type":"string"}, "sub":{"type":"string"},
        "exp":{"type":"string","format":"date-time"},
        "caveats":{"type":"array","items":{"type":"string"}}
}}}
```

#### RevokeRequest (oneOf)

```json
{ "oneOf": [
  { "type":"object","required":["epoch"],"properties":{"epoch":{"type":"integer","minimum":0},"reason":{"type":"string"}} },
  { "type":"object","required":["kid"],  "properties":{"kid":  {"type":"string"},"reason":{"type":"string"}} }
]}
```

#### RevokeResponse

```json
{ "type":"object","required":["current_epoch"],"properties":{"current_epoch":{"type":"integer"}} }
```

#### Error (envelope for 4xx/5xx)

```json
{
  "type":"object",
  "required":["reason"],
  "additionalProperties": false,
  "properties": {
    "reason": { "type":"string", "enum":[
      "ttl_too_long","unknown_caveat","no_acceptable_alg",
      "over_limit","ratio_cap","busy","degraded","kms_timeout","registry_unavailable","bad_request"
    ]},
    "message": { "type":"string" },
    "corr_id": { "type":"string" }
  }
}
```

### 2.2 Capability Token (interoperability notes)

* **`token` is opaque.** Verifiers MUST NOT parse internals unless using official libraries; rely on `ron-auth`.
* Exposed/advertised attributes (via verify preflight or `ron-auth` output):

  * `alg` (`ed25519` or `ed25519+ml-dsa`)
  * `kid` (issuer key id)
  * `epoch` (revocation epoch)
  * `aud`, `sub`, `exp`, `caveats[]`
* **Caveat grammar** (string pairs; all lower-case):

  * `svc=<svc-name>` (`svc-mailbox`, `svc-storage`, etc.)
  * `route=/path/prefix`
  * `region=<region-code>`
  * `budget.bytes=<u64>`
  * `budget.reqs=<u32>`
  * `rate.rps=<u32>`
  * `pq.fallback=true` (set by issuer when negotiated down)

Clients/SDKs MUST treat unknown caveats as **errors** when requesting issuance; services MUST **reject** unknown caveats at ingress.

---

## 3) Bus Topics

### 3.1 Events Published (by svc-passport)

| Topic                     | Payload (JSON)                                                                      | Notes                              |
| ------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------- |
| `passport.revoked`        | `{"epoch":43,"kid":"issuer-v2","reason":"rotation","ts":"<rfc3339>"}`               | Either `epoch` or `kid` present    |
| `passport.policy_updated` | `{"version":177,"alg_set":["ed25519","ed25519+ml-dsa"],"ts":"<rfc3339>"}`           | Signed registry descriptor applied |
| `kernel.config_updated`   | `{"version":177,"changed":["issuer_policy.max_ttl","pq.mode"],"ts":"<rfc3339>"}`    | Redacted/enum-only fields          |
| `kernel.health`           | `{"service":"svc-passport","ready":true,"components":{"kms":true,"registry":true}}` | Heartbeat / readiness reflectors   |

### 3.2 Events Subscribed

| Topic                   | Action                                                           |
| ----------------------- | ---------------------------------------------------------------- |
| `kernel.config_updated` | Recompute config snapshot; swap atomically; emit audit metric    |
| `registry.policy`       | Refresh issuer/policy descriptors; swap snapshot; notify workers |
| `kernel.shutdown`       | Begin graceful drain (stop intake → drain → abort)               |

> **Propagation SLO:** Downstream services should observe `passport.revoked` and refuse old `epoch` within **≤ 5s p99**.

---

## 4) Canonical Test Vectors

> Place machine-readable copies under `tests/vectors/passport/`.

### 4.1 Issue — happy path

**Request**

```json
{
  "subject_ref":"sub-abc123",
  "audience":"svc-mailbox",
  "ttl_s":900,
  "caveats":["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"],
  "accept_algs":["ed25519+ml-dsa","ed25519"]
}
```

**Response (example)**

```json
{
  "token":"b64u:eyJ2ZXIiOjEsImFsZyI6ImVkMjU1MTkrbWwtZHNhIiwiLi4uIn0", 
  "kid":"issuer-v1",
  "alg":"ed25519+ml-dsa",
  "exp":"2030-01-01T00:15:00Z",
  "caveats":["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"]
}
```

### 4.2 Issue — TTL too long

**Request**: same, but `"ttl_s": 999999`
**Response**

```json
{"reason":"ttl_too_long","message":"ttl exceeds policy.max_ttl","corr_id":"01J9..."}
```

HTTP: `400`

### 4.3 PQ negotiation fail

**Request**: `"accept_algs": ["ml-dsa-only"]`
**Response**

```json
{"reason":"no_acceptable_alg","message":"issuer cannot satisfy requested algorithms","corr_id":"01J9..."}
```

HTTP: `400`

### 4.4 Revoke by epoch

**Request**

```json
{"epoch": 43, "reason":"compromise"}
```

**Response**

```json
{"current_epoch":43}
```

**Bus**: emit `passport.revoked` with `epoch:43`.

### 4.5 Verify preflight

**Request**

```json
{"token": "b64u:..."}
```

**Response**

```json
{"ok":true,"parsed":{"alg":"ed25519+ml-dsa","kid":"issuer-v1","epoch":42,"aud":"svc-mailbox","sub":"sub-abc123","exp":"2030-01-01T00:15:00Z","caveats":["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"]}}
```

---

## 5) Error Taxonomy (wire-visible)

| HTTP | reason                 | When                                      | Client action                 |
| ---- | ---------------------- | ----------------------------------------- | ----------------------------- |
| 400  | `bad_request`          | Malformed JSON, missing required field    | Fix payload                   |
| 400  | `ttl_too_long`         | `ttl_s` exceeds policy.max_ttl            | Reduce TTL                    |
| 400  | `unknown_caveat`       | Caveat not recognized by policy           | Remove/rename caveat          |
| 400  | `no_acceptable_alg`    | PQ negotiation cannot be satisfied        | Include acceptable alg set    |
| 413  | `over_limit`           | Body exceeds cap                          | Reduce request size           |
| 400  | `ratio_cap`            | Decompression ratio exceeded              | Adjust compression            |
| 429  | `busy`                 | Bounded queue full                        | Backoff per `Retry-After`     |
| 503  | `degraded`             | Readiness shed (issuance path)            | Retry later                   |
| 504  | `kms_timeout`          | Upstream KMS deadline exceeded            | Retry with jitter; check KMS  |
| 503  | `registry_unavailable` | Registry fetch failed during strict phase | Retry or fail open per policy |

> Error envelope is **uniform** (see §2.2). `Retry-After` may be present on 429/503.

---

## 6) Interop Guarantees

* **Path SemVer:** `/v1` is stable. Breaking DTO changes require `/v2`.
* **Policy vs API:** Policy toggles (alg sets, rotation) are **not** API breaks if DTOs stable; changes are announced via `svc-registry` and bus.
* **Unknown fields:** Requests reject unknown fields; responses may add fields (clients ignore unknowns).
* **Token opacity:** Treat `token` as **opaque**; use `ron-auth` for authoritative verification.
* **Canonical events:** `passport.revoked` and `passport.policy_updated` payloads are **semver-stable** (additive-only fields allowed).
* **Auditability:** Vectors stored under `/tests/vectors/passport/` and referenced by conformance tests.

---

## 7) References

* **Interop Blueprint GMI-1.6** — Omni-Gate contract
* **IDB.md** — invariants mapping (SEC/GOV)
* **CONFIG.md** — limits, PQ mode, amnesia posture
* **OBSERVABILITY.md** — correlation fields and metrics
* **API.md** — full OpenAPI excerpt for `/v1`
* **SECURITY.md** — threat model and hardening

---

## 8) SDK Notes & Pseudocode

### 8.1 Issue flow (client)

```ts
// TypeScript-ish
const req = {
  subject_ref: "sub-abc123",
  audience: "svc-mailbox",
  ttl_s: 900,
  caveats: ["svc=svc-mailbox","route=/mailbox/send","budget.bytes=1048576","rate.rps=5"],
  accept_algs: ["ed25519+ml-dsa","ed25519"]
};
const res = await http.post("/v1/passport/issue", req, { timeout: 5000 });
const { token, alg, kid, exp } = res.data;
// Attach to downstream call:
headers["Authorization"] = `Bearer ${token}`;
```

### 8.2 Backoff policy on 429/503

* Exponential backoff with jitter: 100ms → 200ms → 400ms (cap 2s), respect `Retry-After` if present.

### 8.3 PQ negotiation rule of thumb

* Prefer `["ed25519+ml-dsa","ed25519"]`.
* If server returns classical due to policy, it may add caveat `pq.fallback=true` (informational).

---

## 9) Conformance Checklist

* [ ] Client ignores unknown **response** fields; rejects unknown **request** fields before send.
* [ ] Client honors `Retry-After` on 429/503.
* [ ] Client sets `X-Corr-ID` (ULID/UUID); propagates through SDK.
* [ ] Client bounds request body to `≤ 1 MiB`; avoids pathological compression.
* [ ] Client supplies acceptable `accept_algs`; gracefully handles `no_acceptable_alg`.
* [ ] Integrations subscribe to `passport.revoked` and enforce epoch within **≤ 5s p99**.

---

✅ This interop spec pins the **wire contract** for `svc-passport`: endpoints, DTOs, events, and vectors. It is designed to be **testable, additive-friendly, and governance-aware**, preventing drift across services and SDKs.

```
```
