
---

## 1) New file: `SDK_SCHEMA_IDB.MD`

````md
# SDK_SCHEMA_IDB — Unified Schema & Facet Contract for SDKs

> Applies to: all `ron-app-sdk-*` (Rust, TS, Go, Python, etc.)  
> Scope: DTO schemas, error shapes, pagination, **facet manifests (TOML)**, and cross-language type mapping.

---

## RO:WHAT

Define a **single schema+contract profile** that every SDK must follow:

- DTOs and envelopes (request/response) for RON-CORE services.
- Error, pagination, and metadata envelopes.
- **Facet manifests** (TOML) and their invariants.
- Cross-language type mappings (Rust ↔ TS ↔ Go ↔ Python, etc.).
- Codegen + CI rules to prevent drift.

This doc is the interop spine for all SDKs.

---

## RO:WHY

Without a unified schema profile:

- Each SDK will slowly invent its own types (e.g. timestamps as `string` vs `number` vs `Date`).
- Error formats drift (TS expects `{ error }`, Rust returns `{ message }`).
- Facet manifests and routing semantics become slightly different per language.

That breaks the core promise of RON-CORE:

- **Same capability + same request ⇒ same behavior**, no matter which SDK or language is used.

This IDB ensures:

- One *canonical schema* (from `ron-proto` and HTTP contracts).
- All SDKs generate or derive types from the same source.
- Facet behavior is consistent and testable across the stack.

---

## RO:INVARIANTS

Across all SDKs and services:

1. **Single source of truth**:
   - All DTOs and envelopes come from **`ron-proto` + HTTP/OpenAPI contracts**.
   - SDKs **never** define conflicting schemas by hand.

2. **Stable envelopes**:
   - One canonical error envelope and pagination envelope is shared across SDKs.

3. **Facet schema is uniform**:
   - All facet manifests (TOML) follow the same shape and invariants.
   - Path semantics `/facets/{facet_id}{route.path}` are identical everywhere.

4. **Predictable type mapping**:
   - For each “tricky” type (e.g. `u64`, timestamps, binary), mapping is consistent across languages.

5. **Forward compatibility**:
   - Adding fields is non-breaking when done according to this profile.
   - Unknown fields are ignored (or safely surfaced as extension metadata), not fatal.

---

## 0) TL;DR

- **Truth:** `ron-proto` (wire contracts) + gateway HTTP spec (OpenAPI/JSON schema).
- **SDKs:** generate or derive DTOs from that truth; never fork schemas.
- **Errors + pagination:** same envelope everywhere.
- **Facets:** TOML manifests with a strict schema; SDKs can generate them but cannot bend invariants.
- **CI:** schema drift is a build-time failure.

---

## 1) Canonical Sources of Truth

### 1.1 `ron-proto` (wire-level DTOs)

- Serves as the **canonical definition** for:
  - OAP/1 frames,
  - Storage objects,
  - Index entries,
  - Auth/Passport/Capability payloads,
  - Audit/Registry entries (where exposed to SDKs).

### 1.2 HTTP / OpenAPI Specification

- `svc-gateway`, `omnigate`, and future gateways expose:
  - HTTP endpoints surface (paths, methods),
  - Request/response bodies,
  - Error codes and envelopes.
- These should be captured in:
  - OpenAPI 3.x or equivalent machine-readable format.

### 1.3 Facet Manifest Schema (TOML)

- The facet plane in Micronode is **always** configured via TOML manifests.
- These manifests are **language-agnostic** and must not diverge.

SDKs may:

- Generate TOML from builders,
- POST manifests into the node (future “register facet” API),
- But must produce TOML that conforms to this schema.

---

## 2) DTO & Type Mapping (Cross-language)

### 2.1 Core Types

For each core type, define mapping:

| Canonical Type | Rust          | TypeScript / JS          | Go          | Notes                             |
|----------------|---------------|--------------------------|-------------|-----------------------------------|
| `bool`         | `bool`        | `boolean`                | `bool`      | -                                 |
| `u32`          | `u32`         | `number`                 | `uint32`    | Non-negative, < 2^32              |
| `i64`          | `i64`         | `string`                 | `int64`     | TS uses `string` to avoid loss    |
| `u64`          | `u64`         | `string` or `bigint`*    | `uint64`    | TS default = `string`; bigints opt-in |
| `string`       | `String`      | `string`                 | `string`    | UTF-8                             |
| `bytes`        | `Vec<u8>`     | `Uint8Array` / `Buffer`  | `[]byte`    | Base64 or raw; document per API   |
| `timestamp`    | `DateTime`    | `string` (ISO8601)       | `time.Time` | Always ISO 8601 in JSON           |
| `id` (UUID-ish)| `String`      | `string`                 | `string`    | No special type; use newtype if needed |

\* Each TS SDK can provide a config flag: “parse `u64` as `bigint`”, but **serialized JSON must use string** to avoid ambiguity.

### 2.2 JSON Encoding Rules

- **Nulls vs missing fields**:
  - Absent optional fields are **omitted**, not set to `null`, unless explicitly meaningful.
- **Unknown fields**:
  - SDKs:
    - Rust: `#[serde(default)]` + `#[serde(deny_unknown_fields)]` when safe, or a catch-all `extra` map.
    - TS: keep unknown fields in a loose `Record<string, unknown>` or ignore.
    - Go: allow unknown fields (Go’s JSON decoder does this by default).
- **Binary**:
  - If transported in JSON, bytes are **base64url** encoded (documented per API).

---

## 3) Error Envelope Schema

### 3.1 Canonical Error Shape

Every *SDK-facing* error from RON-CORE (over HTTP) should follow:

```json
{
  "code": "string",          // short machine code, e.g. "UNAUTH", "NOT_FOUND"
  "message": "string",       // end-user-safe description, no secrets
  "kind": "string",          // category: e.g., "auth", "validation", "internal", "rate_limit"
  "correlation_id": "string",// traceable ID for logs
  "details": {               // optional machine-readable detail; may be absent
    "...": "..."
  }
}
````

### 3.2 Mapping in SDKs

* **Rust**:

  * Define a `SdkError` with fields `(code, message, kind, correlation_id, details)`.
  * Always include `correlation_id` in logs (but not secrets).

* **TS/JS**:

  * `class RonError extends Error` with:

    * `code`, `kind`, `correlationId`, `details`.
  * `error.message` is the safe `message` (no raw server stack traces).

* **Go**:

  * `type SdkError struct { Code, Kind, Message, CorrelationID string; Details map[string]any }`.

SDK invariants:

* They **never** blindly print the raw HTTP body as an error string.
* They always attempt to parse this envelope; if parse fails, they wrap it in a generic “transport error” with safe message and attached debug metadata.

---

## 4) Pagination & Collections

### 4.1 Canonical Pagination Envelope

All list-style endpoints use a common pattern:

```json
{
  "items": [ ... ],
  "next_page_token": "string or null"
}
```

Invariants:

* `items` is always present (empty array if no results).
* `next_page_token`:

  * `null` or missing means “no more pages”.
  * Non-empty string can be fed back to `page_token` query param.

### 4.2 SDK Helpers

Each SDK should provide:

* A typed `Page<T>` type or equivalent.
* Helpers like `listAll` that:

  * Call `list(page_size)` repeatedly,
  * Respect timeouts and rate limits,
  * Never unboundedly buffer infinite streams (bounded by caller opts).

---

## 5) Facet Manifest Schema (TOML)

This is the **contract** Micronode enforces for facets, regardless of what SDK generates the manifests.

### 5.1 Basic Shape

Each `.toml` file describes one facet:

```toml
[facet]
id   = "docs"            # unique within the node
kind = "static"          # "static" | "echo" | "proxy"

[facet.security]
# Optional: facet-level security hints (overlays global SecurityMode)
public = false           # if true, route is allowed without caps where safe
requires_auth = true     # if true, always require caps (even in dev_allow)

[facet.meta]
description = "Docs facet"
owner       = "team-or-service-name"
version     = "1.0.0"

[[route]]
method = "GET"
path   = "/hello"
file   = "crates/micronode/dev_facets/hello.txt"
```

### 5.2 Kind Semantics

#### `kind = "static"`

* Routes must include `file` pointing to a readable file.
* Invariants:

  * `method` is usually `GET` (others allowed only if explicitly documented).
  * `file` must be a path **inside a configured facet root or whitelisted dir**.
  * Micronode responds with `200` and file body, `404` if file missing.

#### `kind = "echo"`

* Routes have no `file` or upstream.
* Micronode responds with a simple JSON payload, e.g.:

```json
{ "echo": "ok" }
```

Used for health/debug.

#### `kind = "proxy"` (future, not yet implemented in core)

* Routes forward to an **upstream** service.

Design (future schema):

```toml
[facet]
id   = "todo"
kind = "proxy"

[facet.upstream]
scheme = "http"                          # "http" | "https" | future: "oap"
host   = "todo-backend"                  # name or IP
port   = 4000
base_path = "/"                          # prefix added before route.path

[[route]]
method = "GET"
path   = "/list"                         # Micronode path: /facets/todo/list
upstream_path = "/list"                  # Upstream path; default: same as path

[[route]]
method = "POST"
path   = "/create"
upstream_path = "/create"
```

Invariants:

* Micronode route: `/facets/{id}{route.path}`.
* Upstream URI: `${scheme}://${host}:${port}${base_path}${upstream_path}`.
* Security:

  * Upstream requests must include:

    * Minimal caps (if used),
    * Or be on a private network with mTLS or equivalent.

### 5.3 Path & Routing Semantics

* **Actual HTTP path** exposed by Micronode is always:

  ```text
  /facets/{facet.id}{route.path}
  ```

  Example:

  * `id = "docs"`, `path = "/hello"` → `/facets/docs/hello`.

* SDKs must reflect this rule in docs and client helpers.

### 5.4 Security Hints

`[facet.security]` is advisory but may influence Micronode behavior:

* `public = true`:

  * This facet is safe for unauthenticated reads (e.g., docs).
* `requires_auth = true`:

  * Even in `dev_allow`, this facet should enforce caps.

For now, these are **hints**; later they can plug directly into SecurityMode/ron-policy.

### 5.5 Limits & SRI (Optional Fields)

To support rate limiting and integrity:

```toml
[facet.limits]
max_rps         = 100
max_concurrency = 16

[[route]]
method = "GET"
path   = "/app.js"
file   = "dist/app.js"

[route.integrity]
algo  = "sha256"
value = "base64-encoded-hash"
```

Invariants:

* SDKs that generate manifests **may** populate these, but Micronode will:

  * Respect them when implemented (limits),
  * Verify integrity before serving static assets if `integrity` is present.

---

## 6) SDK Config & Env Schema (Shared)

To avoid each SDK inventing env names:

* **Canonical env vars** (already in use for Rust, must be reused):

  * `RON_SDK_GATEWAY_ADDR` – base URL of gateway/omnigate.
  * `RON_SDK_OVERALL_TIMEOUT_MS`
  * `RON_SDK_CONNECT_TIMEOUT_MS`
  * `RON_SDK_READ_TIMEOUT_MS`
  * `RON_SDK_WRITE_TIMEOUT_MS`

SDKs:

* MUST support these env vars as defaults.
* MAY allow override via code options.

---

## 7) Versioning & Compatibility

* DTOs and manifests follow **semantic versioning**:

  * Backward-compatible additions: new optional fields.
  * Breaking changes: removing fields or changing types.

* Each SDK must:

  * Declare which **protocol/schema version** it targets.
  * Fail fast if it detects incompatible gateway/proto versions (where possible).

---

## 8) Codegen & CI Rules

* **Codegen is preferred** over handwritten DTOs wherever tooling allows.
* CI should include:

  * A “schema drift” check:

    * Regenerate DTOs from `ron-proto`/OpenAPI,
    * Fail if git diff is non-empty.
  * At least one test that hits a local node and asserts:

    * DTO shapes,
    * Error envelopes,
    * Pagination behavior.

---

## 9) Open Questions / Future Extensions

* `kind = "wasm"` facets:

  * Referencing WASM modules and sandboxed execution.
* Signed manifests:

  * Integration with `ron-kms` to sign facet bundles.
* ZK-capable caps:

  * DTO changes to support proving capabilities without exposing raw secrets.

These are future-facing; keep schema flexible to add them without breaking existing SDKs.

````

---

## 2) Patches for `SDK_IDB.MD` (interop upgrades)

You don’t need a whole new IDB beyond the schema one; just **strengthen interop sections** in the existing SDK_IDB. Here are drop-in edits.

### 2.1 Strengthen the “Interop & Schema” section

Find your existing **Interop/Schema** section in `SDK_IDB.MD` and replace/augment it with something like:

```md
## 11) Interop & Schema (ron-proto, HTTP, Facets)

RO:WHAT — Ensure all SDKs (Rust, TS, Go, etc.) share a single structural truth for DTOs, errors, pagination, and facets.

RO:INVARIANTS —
- All DTOs exposed by SDKs are derived from canonical schemas (`ron-proto` + HTTP/OpenAPI).
- No SDK invents conflicting request/response shapes.
- Facet manifests (TOML) follow a unified schema profile (see `SDK_SCHEMA_IDB.MD`), even if generated by SDK builders.
- Error and pagination envelopes have the same fields and semantics across languages.

### 11.1 Canonical Sources

- `ron-proto` — message definitions for OAP/1, storage/index DTOs, auth/passport payloads.
- HTTP/OpenAPI spec for gateway/omnigate — path/method/body and error envelopes.
- `SDK_SCHEMA_IDB.MD` — cross-language type mapping + facet schema.

### 11.2 SDK Requirements

- Tier-1 SDKs (Rust, TS, Go):
  - MUST use codegen or direct bindings from `ron-proto`/OpenAPI for DTOs and error envelopes.
  - MUST implement the canonical error shape and pagination envelope.
  - MUST treat facet manifests (TOML) as **data**, not behavior; any code that generates TOML must comply with the facet schema.

- Tier-2/3 SDKs:
  - MAY handwrite DTOs **but**:
    - MUST match the same JSON field names and types,
    - MUST pass the shared compatibility test suite against a local node.

### 11.3 Drift Avoidance

- CI MUST include a schema drift check:
  - For languages with codegen: regenerate DTOs and fail build if diff appears.
  - For all SDKs: run a shared “interop tests” suite that:
    - Hits known endpoints (ping, simple KV put/get),
    - Asserts DTO, error, and pagination shapes.
````
