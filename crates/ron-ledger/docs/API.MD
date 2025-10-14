

---

# 📄 Paste-ready — `crates/ron-ledger/docs/API.md`

````markdown
---
title: ron-ledger — Public API (Rust + HTTP/UDS)
status: reviewed
version: 1.4.0
msrv: 1.80.0
last-updated: 2025-10-13
audience: consumers, contributors, ops, auditors
pillar: P12 — Economics & Wallets
concerns: [ECON, SEC, RES]
owners: [Stevan White]
links:
  - IDB: ./IDB.md
  - SECURITY: ./SECURITY.md
  - CONCURRENCY: ./CONCURRENCY.md
  - OBSERVABILITY: ./OBSERVABILITY.md
  - CONFIG: ./CONFIG.md
---

# 0) Purpose & Scope

This document is the **normative specification** of `ron-ledger`’s public surface:

- **Rust surface area** (very small, DTOs/types only; crate is primarily a service).
- **HTTP/UDS API** (endpoints, auth, bodies, responses, errors).
- **Wire contracts** (strict JSON, reason taxonomy).
- **Stability policy** (SemVer for Rust + HTTP; MSRV).
- **Tooling & CI gates** (snapshots, diffs, semver checks).
- **Acceptance checklist** for shipping changes.

Everything here is enforced by CI. If it’s not captured here, it isn’t public.

---

# 1) Public Surface

## 1.1 Rust API (intentionally minimal)
Rust exports exist only for DTOs and config/types used by tests and tooling.

```text
crate
├─ pub mod api         // DTOs for wire types; #[non_exhaustive]
├─ pub mod types       // Hash types, IDs, RejectReason; #[non_exhaustive]
└─ pub mod config      // read-only view of runtime config; #[non_exhaustive]
````

**Stability rules:**

* All exported structs/enums are `#[non_exhaustive]`.
* No trait APIs are exposed.
* No `unsafe` in public surface.

CI snapshots are generated with `cargo public-api` and checked with `cargo semver-checks`.

## 1.2 HTTP / UDS endpoints

| Route      | Method | Auth                                 | Body            | Returns            | Notes                                                             |
| ---------- | ------ | ------------------------------------ | --------------- | ------------------ | ----------------------------------------------------------------- |
| `/ingest`  | POST   | Capability (macaroon); mTLS for priv | `IngestRequest` | `IngestResponse`   | UDS variant supports PEERCRED identity in addition to capability. |
| `/roots`   | GET    | none (read)                          | —               | `RootsResponse`    | Pagination via `since=seq`.                                       |
| `/healthz` | GET    | none                                 | —               | `{"ok":true}`      | Shallow liveness only.                                            |
| `/readyz`  | GET    | none                                 | —               | `ReadyzReport`     | Reflects backpressure/breakers (writes fail-closed).              |
| `/metrics` | GET    | network policy restricted            | —               | Prometheus text    | See OBSERVABILITY.md.                                             |
| `/version` | GET    | none                                 | —               | build/version JSON | Includes commit & OpenAPI version.                                |

---

# 2) Wire Contracts (strict JSON)

All JSON uses `serde` with `deny_unknown_fields`. Unknown fields MUST be rejected with a structured `RejectReason`.

## 2.1 DTOs (selected)

```json
// IngestRequest
{
  "batch": [
    {
      "id": "9c7d9bda-1f8a-4f3e-8f7a-1a0e3b6a9a10",
      "ts": 1728794183123,
      "kind": "Credit|Debit|Transfer|Mint|Burn|Hold|Reverse",
      "account": "acct_123",
      "amount": "12345",              // string-encoded integer (atoms)
      "nonce": "base64-16",
      "capability_ref": "cap_abc",
      "v": 1
    }
  ],
  "idem_id": "optional-idempotency-id"
}
```

```json
// IngestResponse (success)
{
  "accepted": true,
  "seq_start": 1024,
  "seq_end": 1031,
  "new_root": "hex-256",
  "reasons": []               // empty on success
}
```

```json
// IngestResponse (partial/none accepted)
{
  "accepted": false,
  "seq_start": null,
  "seq_end": null,
  "new_root": "hex-256-or-prev",
  "reasons": [
    {"idx": 0, "reason": "cap_invalid"},
    {"idx": 3, "reason": "unknown_kind"}
  ]
}
```

```json
// RootsResponse
{
  "roots": [
    {"seq": 1031, "root": "hex-256", "ts": 1728794183999}
  ],
  "next": 1032
}
```

```json
// ReadyzReport
{
  "ready": true,
  "degraded": false,
  "kms_ready": true,
  "policy_ready": true,
  "wallet_ready": true,
  "backpressure_ok": true,
  "committer_lag_ms": 3
}
```

## 2.2 Error taxonomy (one source of truth)

`RejectReason` values (wire + metrics):
`unauth`, `cap_invalid`, `unknown_kind`, `policy_denied`, `busy`, `too_large`, `timeout`, `internal`

* These strings **exactly** match `rejected_total{reason}` label values (see OBSERVABILITY.md).
* Additions are **minor** (see SemVer table); changes/removals are **major**.

---

# 3) Usage Examples (curl)

## 3.1 Ingest (HTTPS)

```bash
curl -sS -X POST https://ledger.example.com/ingest \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${MACAROON}" \
  --data @request.json
```

## 3.2 Ingest (UDS with PEERCRED)

```bash
curl --unix-socket /var/run/ron-ledger.sock -sS -X POST http://localhost/ingest \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${MACAROON}" \
  --data @request.json
```

## 3.3 Fetch roots since a sequence

```bash
curl -sS "https://ledger.example.com/roots?since=1032"
```

---

# 4) Stability & SemVer Policy

We maintain **two** surfaces with coordinated versioning:

* **Rust library (DTOs/types)**: Semantic Versioning (SemVer) with MSRV guard.
* **HTTP/JSON API**: Versioned via OpenAPI; additive within a major.

## 4.1 Rust SemVer (crate)

* **Minor**: add fields to `#[non_exhaustive]` structs, add enum variants to `#[non_exhaustive]` enums, add new DTOs/modules.
* **Major**: remove/rename public items; change types/visibility; tighten trait bounds (if any in the future).
* **MSRV**: 1.80.0 (bumping MSRV is **minor** if build-only impact; **major** if it impacts runtime behavior).

CI: `cargo public-api --deny-changes` + `cargo semver-checks --workspace` are mandatory.

## 4.2 HTTP/JSON SemVer (serde-specific contract)

| Change type                                | Class     | Notes                                                                                                        |
| ------------------------------------------ | --------- | ------------------------------------------------------------------------------------------------------------ |
| Add **optional** field to request/response | Minor     | Must default safely; `deny_unknown_fields` still rejects unknown input from client → document opt-in fields. |
| Add **new** `RejectReason` value           | Minor     | Consumers must treat unknown reasons as generic error.                                                       |
| Add **new endpoint**                       | Minor     | Backwards-compatible.                                                                                        |
| Tighten validation (e.g., stricter range)  | Minor*    | Marked with release note; must not break well-formed historical payloads.                                    |
| Change scale/encoding of `amount`          | **Major** | E.g., atoms → millis; breaking.                                                                              |
| Remove field / change field type           | **Major** | Breaking.                                                                                                    |
| Change status codes semantics              | **Major** | Breaking.                                                                                                    |
| Break reason strings (rename/remove)       | **Major** | Breaking (observability taxonomy change).                                                                    |

* Minor “tightening” allowed only if prior payloads remain valid.

---

# 5) Guarantees & Anti-Footguns

* **Idempotency**: `(account, amount, nonce[, idem_id])` tuple guarantees no divergence on retries.
* **Deterministic roots**: `/roots` reflects the same accumulator values as internal checkpoints.
* **No glob imports (consumer guidance)**: avoid `use ron_ledger::types::*;` in clients—future additive changes can shadow names. Import explicitly.
* **No `unsafe` in public surface**: if ever introduced, it requires an audited major release (see SECURITY.md §10).

---

# 6) Tooling & Artifacts (CI-enforced)

* **Rust surface snapshot**: `cargo public-api --diff-git-check` against `docs/api-history/ron-ledger/<version>.txt`.
* **Semantic lint**: `cargo semver-checks --workspace` (mandatory).
* **OpenAPI**: `openapi/ron-ledger.v<major>.<minor>.yaml` (generated+reviewed); `oasdiff` runs in CI to assert **non-breaking** diffs unless major bump.
* **Rustdoc JSON (provenance)**: tools consume `rustdoc-json` output pinned via CI matrix to ensure deterministic snapshots.

---

# 7) Acceptance Checklist (Definition of Done)

* [ ] Updated OpenAPI file and example payloads.
* [ ] `cargo public-api` snapshot updated and diff reviewed.
* [ ] `cargo semver-checks --workspace` green.
* [ ] Tests proving:

  * [ ] Unknown JSON fields **reject** with `RejectReason="unknown_kind"` or specific mapped reason.
  * [ ] New optional fields are ignored by older servers (where applicable) and do not change behavior.
  * [ ] Metrics taxonomy unchanged or documented if adding new reasons.
* [ ] CHANGELOG includes migration notes (if any).
* [ ] Observability updated if reasons/endpoints changed (dashboards/alerts).

---

# 8) Endpoint Details

## 8.1 `POST /ingest`

* **Auth**: `Authorization: Bearer <macaroon>`, optional mTLS (required for privileged ops).
* **Body**: `IngestRequest`.
* **Responses**:

  * `200 IngestResponse` (accepted).
  * `207 IngestResponse` (partial accept with indexed `reasons`).
  * `400/401/403` (bad/unauth/forbidden) with `RejectReason` mapping.
  * `413` for too large.
  * `429` busy (backpressure).
  * `503` degraded (safe-mode).
* **Semantics**:

  * Idempotent by tuple; replays safe.
  * Backpressure policy aligns with CONCURRENCY (§3).

## 8.2 `GET /roots?since=<seq>`

* **Returns**: ordered list of roots from `since` (inclusive/exclusive depending on `since` definition; we use **exclusive**).
* **Degradation**: still serves during safe-mode (read path); see SECURITY/IDB.

## 8.3 `GET /readyz` / `GET /healthz` / `GET /metrics` / `GET /version`

* As specified in OBSERVABILITY and SECURITY. `/metrics` requires network policy.

---

# 9) OpenAPI Sketch (abbreviated)

```yaml
openapi: 3.0.3
info:
  title: ron-ledger
  version: "1.4.0"
paths:
  /ingest:
    post:
      security:
        - macaroonAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/IngestRequest" }
      responses:
        "200": { $ref: "#/components/schemas/IngestResponse" }
        "207": { $ref: "#/components/schemas/IngestResponse" }
        "400": { description: Bad Request }
        "401": { description: Unauthorized }
        "403": { description: Forbidden }
        "413": { description: Payload Too Large }
        "429": { description: Too Many Requests }
        "503": { description: Service Unavailable }
  /roots:
    get:
      parameters:
        - name: since
          in: query
          schema: { type: integer, format: int64 }
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema: { $ref: "#/components/schemas/RootsResponse" }
components:
  securitySchemes:
    macaroonAuth:
      type: http
      scheme: bearer
  schemas:
    IngestRequest:
      type: object
      additionalProperties: false
      properties:
        batch:
          type: array
          items: { $ref: "#/components/schemas/Entry" }
        idem_id: { type: string }
      required: [batch]
    Entry:
      type: object
      additionalProperties: false
      properties:
        id: { type: string, format: uuid }
        ts: { type: integer, format: int64 }
        kind: { type: string, enum: [Credit,Debit,Transfer,Mint,Burn,Hold,Reverse] }
        account: { type: string }
        amount: { type: string, pattern: "^[0-9]+$" }
        nonce: { type: string }
        capability_ref: { type: string }
        v: { type: integer, minimum: 1 }
      required: [id,ts,kind,account,amount,nonce,capability_ref,v]
    IngestResponse:
      type: object
      properties:
        accepted: { type: boolean }
        seq_start: { type: integer, format: int64, nullable: true }
        seq_end: { type: integer, format: int64, nullable: true }
        new_root: { type: string }
        reasons:
          type: array
          items:
            type: object
            properties:
              idx: { type: integer }
              reason:
                type: string
                enum: [unauth,cap_invalid,unknown_kind,policy_denied,busy,too_large,timeout,internal]
            required: [idx,reason]
      required: [accepted,new_root,reasons]
    RootsResponse:
      type: object
      properties:
        roots:
          type: array
          items:
            type: object
            properties:
              seq: { type: integer, format: int64 }
              root: { type: string }
              ts: { type: integer, format: int64 }
            required: [seq,root,ts]
        next: { type: integer, format: int64 }
      required: [roots,next]
```

---

# 10) History

* **1.4.0** (2025-10-13): Added curl examples; mandated `cargo semver-checks`; added serde SemVer table; clarified `/roots` `since` semantics (exclusive); aligned reason taxonomy with OBSERVABILITY.
* **1.3.0**: Reason taxonomy stabilized; OpenAPI schemas expanded.
* **1.2.0**: Initial public snapshot with `public-api` CI gates.

```
```

---

## Why this addresses Grok’s nits

* **Serde wire SemVer table** (4.2) removes ambiguity about what’s minor vs. major on JSON changes.
* **Mandatory `cargo semver-checks`** and **rustdoc JSON provenance** strengthen CI guarantees beyond `public-api`.
* **Concrete curl examples** improve integrator ergonomics.
* **Runtime tests** for “reject unknown fields” are now part of DoD.
* **Explicit warning about glob imports** heads off consumer breakage.
* **OpenAPI sketch** stays aligned with the RejectReason taxonomy and the observability labels.

