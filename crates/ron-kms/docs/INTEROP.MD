

---

# 🔗 INTEROP.md — ron-kms

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-kms`:

* **Wire protocols & endpoints:** HTTP/1.1 (+TLS) canonical; optional gRPC mirroring; OAP/1 framing (disabled by default).
* **DTOs & schemas:** strict JSON with `deny_unknown_fields`, size caps, canonical hashing.
* **Bus topics & events:** readiness, config, integrity, policy hints.
* **Canonical test vectors:** content hashing, sign/verify bundles (Ed25519, ML-DSA, Hybrid), KEM wrap/unwrap (X25519, ML-KEM, Hybrid).
* **Guarantees:** compatibility, SemVer, Omni-Gate alignment (GMI-1.6).

`ron-kms` is **custody-only**: it verifies **capability scopes** and enforces crypto invariants; **policy decisions** live in `ron-policy`.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols (canonical → optional)

| Protocol            | Status       | Notes                                                           |
| ------------------- | ------------ | --------------------------------------------------------------- |
| **HTTP/1.1 + TLS**  | **Required** | Axum/Hyper; JSON DTOs; canonical transport                      |
| gRPC (HTTP/2 + TLS) | Optional     | Messages mirror JSON 1:1 (no gRPC-only fields)                  |
| OAP/1 framed TCP    | Optional/Off | Length-delimited frames (see §1.4); for controlled fabrics only |

> TLS must use `tokio_rustls::rustls::ServerConfig`. Reverse proxies **must not** terminate capability verification — KMS validates tokens end-to-end.

### 1.2 Exposed Endpoints (HTTP base: `/v1/kms`)

| Method | Path                 | Scope(s)      | Description                                                  |
| -----: | -------------------- | ------------- | ------------------------------------------------------------ |
|   POST | `/sign`              | `kms:sign`    | Sign `payload_b3`; may emit **hybrid** (ed25519 + ML-DSA)    |
|   POST | `/verify`            | `kms:verify`* | Verify bundle; may succeed via cached pubkeys during outages |
|   POST | `/wrap`              | `kms:wrap`    | Wrap (AEAD+KEM); default **Hybrid KEM (X25519+ML-KEM)**      |
|   POST | `/unwrap`            | `kms:unwrap`  | Unwrap envelope                                              |
|   POST | `/keys`              | `kms:rotate`  | Create key; returns `kid#v1` + public                        |
|   POST | `/keys/{kid}/rotate` | `kms:rotate`  | New version for existing key                                 |
|    GET | `/keys/{kid}`        | `kms:read`    | Public material + versions (+ attestation class)             |

* Deployments may allow `/verify` with `kms:read`; default expects `kms:verify`.

### 1.3 Transport Invariants

* **Body caps:** request ≤ **1 MiB**; decompression ratio ≤ **10×**.
* **Correlation:** `X-Corr-ID` (UUIDv7) honored or generated; echoed in logs/spans.
* **No raw payloads:** callers send **`payload_b3 = BLAKE3(canonical_payload)`** hex (lowercase).
* **Strict parsing:** JSON DTOs `deny_unknown_fields`; bounded arrays; fixed enums/alg names.

### 1.4 OAP/1 (Optional Framed Transport)

> Disabled by default; only for private fabrics requiring non-HTTP framing.

| Field       | Type  | Description                                 |      |        |
| ----------- | ----- | ------------------------------------------- | ---- | ------ |
| `len`       | u32   | Remaining length (not incl. header)         |      |        |
| `ver`       | u8    | 1                                           |      |        |
| `flags`     | u16   | `REQ                                        | RESP | EVENT` |
| `tenant_id` | u128  | ULID/UUID; `0` if unused                    |      |        |
| `corr_id`   | u64   | Correlation (truncated/derived from UUIDv7) |      |        |
| `payload`   | bytes | JSON payload (optionally COMP)              |      |        |

**Limits:** `max_frame = 1 MiB`; streaming chunk = **64 KiB**; checksums validated when COMP is used.

---

## 2) DTOs / Schemas (canonical JSON)

### 2.1 Common Types

* **Key ID:** `kid = "<tenant>/<purpose>/<alg>/<uuid>#vN"`
* **Algorithms:** `ed25519 | ml-dsa | x25519 | ml-kem`
* **Policies:**

  * Sign: `{ "produce_hybrid": bool }`
  * Verify: `{ "mode": "or"|"and"|"pq_only" }`

> All DTOs enforce **`deny_unknown_fields`**, hard size/length caps, and fixed enums.

### 2.2 Sign / Verify

```jsonc
// POST /sign
{
  "kid_or_set": { "kid": "core/audit/ed25519/0b2e...#v3" },
  "payload_b3": "9f64a747... (hex)",       // BLAKE3(“hello world”)= 9f64a7…
  "policy": { "produce_hybrid": true },
  "op_id": "8f9a2c34-..."                  // idempotency (optional)
}
```

```jsonc
// 200 OK
{
  "sigs": [
    { "alg": "ed25519", "signature_b64": "..." },
    { "alg": "ml-dsa",  "signature_b64": "..." }   // if hybrid
  ],
  "kid": "core/audit/ed25519/0b2e...#v3",
  "attestation": { "class": "pkcs11-yubihsm2", "evidence": "..." } // optional
}
```

```jsonc
// POST /verify
{
  "payload_b3": "9f64a747...",
  "sigs": [
    { "alg": "ed25519", "signature_b64": "..." },
    { "alg": "ml-dsa",  "signature_b64": "..." }
  ],
  "policy": { "mode": "and" }
}
// 200 OK
{ "ok": true, "details": { "checked": ["ed25519","ml-dsa"], "satisfied_mode": "and" }, "cache_age_ms": 742 }
```

### 2.3 Wrap / Unwrap (Hybrid KEM default)

```jsonc
// POST /wrap
{
  "kid": "core/kem/x25519/3c8b...#v2",
  "plaintext_key_b64": "uMo9a...==",
  "kem_mode": "hybrid"
}
```

```jsonc
// POST /unwrap
{
  "kid": "core/kem/x25519/3c8b...#v2",
  "wrapped_b64": "Q2lwaGVyVGV4dA..."        // AEAD+KEM envelope
}
```

### 2.4 Keys (Create / Rotate / Get)

```jsonc
// POST /keys
{ "tenant":"core","purpose":"audit","alg":"ed25519","policy_hints":{"rotate_days":30} }
// 200 OK
{ "kid":"core/audit/ed25519/bf1a...#v1","public":{"alg":"ed25519","pubkey_b64":"..." } }
```

```jsonc
// POST /keys/{kid}/rotate
{ "kid":"core/audit/ed25519/bf1a...#v2","previous":"...#v1" }
```

```jsonc
// GET /keys/{kid}
{
  "kid":"core/audit/ed25519/bf1a...#v3",
  "alg":"ed25519",
  "versions":["#v1","#v2","#v3"],
  "public":{"pubkey_b64":"..."},
  "attestation_class":"pkcs11-yubihsm2"
}
```

---

## 3) Bus Topics

> Bus is optional; used for **state signaling** (config, readiness, integrity). No secret material is ever published.

### 3.1 Events Published

| Topic                    | Payload                                               | When                               |                                 |                                    |
| ------------------------ | ----------------------------------------------------- | ---------------------------------- | ------------------------------- | ---------------------------------- |
| `kms.health`             | `{ service:"ron-kms", ok:bool, reason?:string }`      | On liveness transitions            |                                 |                                    |
| `kms.ready`              | `{ ready:bool, missing:[string], degraded:[string] }` | Readiness flips / degraded clarity |                                 |                                    |
| `kms.audit.checkpoint`   | `{ head: "<hex>", index: u64, ok: bool }`             | After checkpoint attempt           |                                 |                                    |
| `kms.attestation.status` | `{ backend:"pkcs11                                    | tpm                                | ...", class?:string, ok:bool }` | Refresh cycle                      |
| `kms.config.updated`     | `{ version:u64 }`                                     | After config swap                  |                                 |                                    |
| `kms.policy.hint`        | `{ pq_mode:"or                                        | and                                | pq_only", rollout_percent:u8 }` | For dashboards (no decisions here) |

### 3.2 Events Subscribed

| Topic            | Action                               |
| ---------------- | ------------------------------------ |
| `config.updated` | Reload config snapshot (atomic swap) |
| `bus.shutdown`   | Begin graceful drain/abort sequence  |

> Policy **decisions** are not taken from bus; `ron-kms` calls `ron-policy` directly (in-process client or HTTP/gRPC) and caches hints.

---

## 4) Canonical Test Vectors

> Store these under `tests/vectors/kms/` and publish for SDK authors.

### 4.1 Content Hash (BLAKE3)

* Input (UTF-8): `"hello world"`
* `payload_b3` (hex, lowercase):
  `9f64a747e1...` (full 32 bytes hex; canonical reference in repo)

### 4.2 Ed25519 Sign/Verify

* **Public key (b64):** `...`
* **Signature (b64):** `...`
* **Verify:** `ok=true` with `policy.mode="or"` (or `and` if hybrid contains both).

*(Provide concrete byte strings in repo vectors; keep constants stable across releases.)*

### 4.3 ML-DSA (Dilithium) Sign/Verify

* **Parameters:** ML-DSA-2 (documented in vectors).
* **Signature (b64):** `...`
* **Verify:** `ok=true`.

### 4.4 Hybrid Sign Bundle

* `sigs = [{alg:"ed25519",...},{alg:"ml-dsa",...}]`
* **Verify (OR):** `ok=true` if either valid.
* **Verify (AND):** `ok=true` only if both valid.

### 4.5 X25519 / ML-KEM Wrap/Unwrap

* **Plaintext key (32B, b64):** `uMo9a...==`
* **Wrapped (b64):** `Q2lwaGVyVGV4dA...`
* **Unwrap → Plaintext** matches original.

> Each vector records: `kid#vN`, algs, signatures, envelope bytes, and expected results under `verify_mode ∈ {or,and,pq_only}`.

---

## 5) Error Taxonomy (wire level)

|    HTTP | Code               | When                                       | Notes                            |
| ------: | ------------------ | ------------------------------------------ | -------------------------------- |
|     400 | `BadRequest`       | DTO validation, size caps, bad alg         | `deny_unknown_fields`, hex check |
| 401/403 | `NoCapability`     | Missing/insufficient scope                 | Bearer macaroon                  |
|     404 | `NotFound`         | Unknown `kid` / version                    |                                  |
|     409 | `Conflict`         | Idempotent mutation collision; rotate race |                                  |
|     413 | `TooLarge`         | Body over cap                              |                                  |
|     429 | `Backpressure`     | Queue full / RPS limit                     | Prefer reject-new                |
|     500 | `Internal`         | Unexpected; audited                        |                                  |
|     503 | `StoreUnavailable` | Backend/attestation gate                   | Include `Retry-After`            |
|     504 | `Timeout`          | Backend op deadline                        |                                  |

**Envelope (stable):**

```json
{ "error": { "code":"Backpressure","message":"queue full","retryable":true,"ts":"...","request_id":"r-abc123" } }
```

---

## 6) Interop Guarantees

* **No Kernel Drift:** Transport surfaces (HTTP, optional gRPC) are versioned and frozen per SemVer.
* **SemVer Discipline:** Additive fields/endpoints are **minor**; any removal/requirement change is **major**.
* **Backward Compatibility:**

  * JSON: unknown fields **rejected** (security posture); **new optional** fields added only when clients can ignore them safely.
  * **KID verification** remains backward compatible across versions (`v1..vN`).
* **Auditability:** All vectors live in `tests/vectors/` with README and sha256 sums.
* **No secret leakage:** `kid` replaced by `kid_hash` in logs/events; payloads referenced via `payload_b3`.

---

## 7) References

* **Omni-Gate / Interop Blueprint (GMI-1.6):** `docs/Interop_Blueprint.md`
* **API Contract:** `docs/API.md`, `docs/openapi/ron-kms.yaml`
* **Observability:** `docs/OBSERVABILITY.md` (IDs, spans, metrics)
* **Security:** `docs/SECURITY.md` (threats, redaction, attestation)
* **Concurrency:** `docs/CONCURRENCY.md` (readiness & degraded semantics)

---

## 8) Quick Examples (for SDKs)

### 8.1 `curl` Sign (hybrid on)

```bash
curl -sS -H "Authorization: Bearer $MACAROON" \
  -H "X-Corr-ID: $(uuidgen)" \
  -H "Content-Type: application/json" \
  -d '{"kid_or_set":{"kid":"core/audit/ed25519/0b2e...#v3"},"payload_b3":"9f64a747...","policy":{"produce_hybrid":true}}' \
  https://kms.internal/v1/kms/sign
```

### 8.2 `curl` Verify (AND mode)

```bash
curl -sS -H "Authorization: Bearer $MACAROON" \
  -H "Content-Type: application/json" \
  -d '{"payload_b3":"9f64a747...","sigs":[{"alg":"ed25519","signature_b64":"..."},{"alg":"ml-dsa","signature_b64":"..."}],"policy":{"mode":"and"}}' \
  https://kms.internal/v1/kms/verify
```

### 8.3 gRPC (proto note)

* Service: `ron.kms.v1.Kms`
* RPCs: `Sign`, `Verify`, `Wrap`, `Unwrap`, `CreateKey`, `Rotate`, `GetKey`
* Messages mirror JSON fields exactly; **no** gRPC-only metadata.

---

## 9) Compatibility Matrix (policy & PQ)

| Client                    | `produce_hybrid` | `verify_mode` | Expected                |
| ------------------------- | ---------------: | ------------: | ----------------------- |
| Legacy (classical only)   |            false |          `or` | OK (classical sig only) |
| Hybrid-OR rollout         |             true |          `or` | OK if either sig valid  |
| Hybrid-AND (default prod) |             true |         `and` | Requires both sigs      |
| PQ-only                   |       true/false |     `pq_only` | Requires PQ sigs only   |

---

✅ With this interop contract, SDKs and peer services have **exact, testable** wire semantics for `ron-kms` — no drift, no surprises, fully aligned with Omni-Gate and your invariants.
