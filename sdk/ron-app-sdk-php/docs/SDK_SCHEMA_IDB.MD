---

## 1) New file: `SDK_SCHEMA_IDB.MD`

````md
# SDK_SCHEMA_IDB — Unified Schema & Facet Contract for SDKs

> Applies to: all `ron-app-sdk-*` (Rust, TS, Go, Python, PHP, etc.)  
> Scope: DTO schemas, error shapes, pagination, **facet manifests (TOML)**, cross-language type mapping, and schema drift rules.

---

## RO:WHAT

Define a **single schema+contract profile** that every SDK must follow:

- DTOs and envelopes (request/response) for RON-CORE services.
- Error, pagination, and metadata envelopes.
- **Facet manifests** (TOML) and their invariants.
- Cross-language type mappings (Rust ↔ TS ↔ Go ↔ Python ↔ PHP, etc.).
- Codegen + CI rules to prevent drift.

This doc is the **interop spine** for all SDKs.

---

## RO:WHY

Without a unified schema profile:

- Each SDK will slowly invent its own types (e.g. timestamps as `string` vs `number` vs `Date`).
- Error formats drift (TS expects `{ error }`, Rust returns `{ message }`, PHP throws plain strings).
- Facet manifests and routing semantics become slightly different per language.

That breaks the core promise of RON-CORE:

> **Same capability + same request ⇒ same behavior**, no matter which SDK or language is used.

This IDB ensures:

- One *canonical schema* (from `ron-proto` and HTTP contracts).
- All SDKs generate or derive types from the same source.
- Facet behavior is consistent and testable across the stack.
- PHP fits into the same interop story as Rust/TS/Go/Python, despite runtime differences (32-bit vs 64-bit, sync vs async, etc.).

---

## RO:INVARIANTS

Across all SDKs and services:

1. **Single source of truth**  
   - All DTOs and envelopes come from **`ron-proto` + HTTP/OpenAPI contracts**.  
   - SDKs **never** define conflicting schemas by hand.

2. **Stable envelopes**  
   - One canonical error envelope and pagination envelope is shared across SDKs.

3. **Facet schema is uniform**  
   - All facet manifests (TOML) follow the same shape and invariants.  
   - Path semantics `/facets/{facet_id}{route.path}` are identical everywhere.

4. **Predictable type mapping**  
   - For each “tricky” type (e.g. `u64`, timestamps, binary), mapping is consistent across languages, including PHP’s 32-bit edge case.

5. **Forward compatibility**  
   - Adding fields is non-breaking when done according to this profile.  
   - Unknown fields are ignored (or safely surfaced as extension metadata), not fatal.

---

## 0) TL;DR

- **Truth:** `ron-proto` (wire contracts) + gateway HTTP spec (OpenAPI/JSON schema).
- **SDKs:** generate or derive DTOs from that truth; never fork schemas.
- **Errors + pagination:** same envelope everywhere.
- **Facets:** TOML manifests with a strict schema; SDKs can generate them but cannot bend invariants.
- **CI:** schema drift is a build-time failure.
- **PHP:** uses the same JSON and envelope shapes as Rust/TS/Go, with clear numeric mapping rules and tests for 32-bit vs 64-bit behavior.

---

## 1) Canonical Sources of Truth

### 1.1 `ron-proto` (wire-level DTOs)

Serves as the **canonical definition** for:

- OAP/1 frames,
- Storage objects,
- Index entries,
- Auth/Passport/Capability payloads,
- Audit/Registry entries (where exposed to SDKs),
- Any app-plane DTO that can be surfaced via `/app/*`.

### 1.2 HTTP / OpenAPI Specification

`svc-gateway`, `omnigate`, and future gateways expose:

- HTTP endpoint surface (paths, methods),
- Request/response bodies (JSON),
- Error codes and envelopes (canonical error schema / RFC7807 variant).

These must be captured in:

- OpenAPI 3.x, JSON schema, or equivalent machine-readable format.

### 1.3 Facet Manifest Schema (TOML)

- The facet plane in Micronode is **always** configured via TOML manifests.
- These manifests are **language-agnostic** and must not diverge.

SDKs may:

- Generate TOML manifests from builders/helpers,
- POST manifests into the node (future “register facet” API),
- But must produce TOML that conforms to this schema profile.

---

## 2) DTO & Type Mapping (Cross-language)

### 2.1 Core Types

For each core type, define mapping:

| Canonical Type | Rust          | TypeScript / JS          | Go          | PHP                            | Notes |
|----------------|---------------|--------------------------|-------------|--------------------------------|-------|
| `bool`         | `bool`        | `boolean`                | `bool`      | `bool`                         | - |
| `u32`          | `u32`         | `number`                 | `uint32`    | `int`                          | Non-negative, < 2^32; PHP `int` is fine on 32/64-bit for this range. |
| `i64`          | `i64`         | `string`                 | `int64`     | `string` (always)              | TS uses `string` to avoid loss; PHP uses `string` for JSON and DTO fields to avoid overflow on 32-bit. Helpers may expose convenience casting on 64-bit. |
| `u64`          | `u64`         | `string` or `bigint`*    | `uint64`    | `string` (JSON & DTOs)         | TS default = `string`; BigInt opt-in. PHP **always** uses string in JSON/DTOs for safety across 32/64-bit; helper methods may cast to `int` on 64-bit if safe. |
| `string`       | `String`      | `string`                 | `string`    | `string`                       | UTF-8; SDKs assume UTF-8 but do not enforce language/locale. |
| `bytes`        | `Vec<u8>`     | `Uint8Array` / `Buffer`  | `[]byte`    | `string` (raw or base64)       | Over HTTP+JSON these are base64url strings; internally PHP may hold raw binary `string`. |
| `timestamp`    | `DateTime`    | `string` (ISO8601)       | `time.Time` | `string` / `\DateTimeImmutable`| JSON wire format is ISO8601 string. PHP SDK may hydrate to `\DateTimeImmutable` in DTOs or leave as string with helper. |
| `id` (UUID-ish)| `String`      | `string`                 | `string`    | `string`                       | No special type; SDKs may add newtypes/wrappers locally. |

\* TS SDKs may provide a config flag: “parse `u64` as `bigint`”, but **serialized JSON must use string** to avoid ambiguity and interop issues.

#### 2.1.1 PHP Numeric Mapping Notes

- **JSON wire:** always uses strings for `i64`/`u64` to avoid ambiguity and float truncation.
- **DTO fields in PHP:**
  - Represented as `string` in value objects for `i64` and `u64`.
  - Convenience accessors can expose `asInt()` or `asFloat()` that:
    - Throw if value would overflow on the current platform.
    - Are clearly documented as lossy or platform-specific when used.
- This ensures **lossless roundtrip** between PHP and Rust/TS/Go even on 32-bit PHP.

### 2.2 JSON Encoding Rules

- **Nulls vs missing fields:**
  - Absent optional fields are **omitted**, not set to `null`, unless `null` has specific semantics in the canonical schema.
  - PHP DTOs should reflect this:
    - Distinguish between “not present” and “explicitly set to null” where relevant.

- **Unknown fields:**
  - SDKs must not crash on unknown fields:
    - Rust: `#[serde(default)]`, optional `extra: HashMap<String, Value>` if needed.
    - TS: store unknown fields in a loose `Record<string, unknown>` or drop them by design.
    - Go: Go’s JSON decoder already ignores unknown fields.
    - PHP: `json_decode` will surface all fields; DTO hydration must either:
      - Ignore unknown keys, or
      - Store them in an `extensions`/`extra` map, but never treat unexpected keys as fatal.
  - For security-sensitive DTOs, we may choose stricter behavior (deny unknown) if it’s safe and documented.

- **Binary:**
  - When transported in JSON, `bytes` are **base64url** encoded strings.
  - All SDKs must agree on:
    - Encoding: base64url (no `+`/`/`, no padding where feasible).
    - Where binary appears:
      - e.g., content hashes, small blobs.
  - PHP:
    - Receives base64url string in JSON.
    - Can expose helper to decode to raw binary `string` or `resource`.

---

## 3) Error Envelope Schema

### 3.1 Canonical Error Shape

Every *SDK-facing* error from RON-CORE (over HTTP) should follow this canonical error envelope, or an RFC7807-compatible variant with equivalent fields:

```json
{
  "code": "string",           // short machine code, e.g. "UNAUTH", "NOT_FOUND"
  "message": "string",        // end-user-safe description, no secrets
  "kind": "string",           // category: e.g., "auth", "validation", "internal", "rate_limit"
  "correlation_id": "string", // traceable ID for logs
  "details": {                // optional machine-readable detail; may be absent
    "...": "..."
  }
}
```

In some cases, an RFC7807 problem document may be returned:

```json
{
  "type": "string",
  "title": "string",
  "status": 400,
  "detail": "string",
  "instance": "string",
  "extensions": { "...": "..." }
}
```

The canonical mapping for SDKs must treat both forms consistently.

### 3.2 Mapping in SDKs

- **Rust:**
  - `SdkError` / `RonError` struct with fields `(code, message, kind, correlation_id, details)` or wraps an RFC7807 struct with a consistent façade.
  - Always logs `correlation_id` for debugging, not secrets.

- **TS/JS:**
  - `class RonError extends Error` with:
    - `code`, `kind`, `correlationId`, `details`.
  - `error.message` is the safe `message` or `title`.

- **Go:**
  - `type SdkError struct { Code, Kind, Message, CorrelationID string; Details map[string]any }`.

- **PHP:**
  - `RonProblemException` wraps a `Problem` value object:
    - For canonical error envelope: `code`, `message`, `kind`, `correlationId`, `details`.
    - For RFC7807: `type`, `title`, `status`, `detail`, `instance`, plus `extensions`.
  - Convenience getters normalize the two into an SDK-level **canonical view**:
    - `getCode()`, `getKind()`, `getMessage()`, `getCorrelationId()`, `getDetails()`.

SDK invariants:

- They **never** blindly print the raw HTTP body as an error string.
- They always attempt to parse a known envelope:
  - If parse fails, they wrap the response in a generic “transport error” / `NetworkException` with a safe message and optional debug metadata.
- PHP must ensure that exception messages do **not** include raw body contents or caps.

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

- `items` is always present (empty array if no results).
- `next_page_token`:
  - `null` or missing means “no more pages”.
  - Non-empty string can be fed back to `page_token` query param.

### 4.2 SDK Helpers

Each SDK should provide:

- A typed `Page<T>` type or equivalent (`struct Page<T>` in Rust/Go, generic interface in TS, class in PHP).
- Helpers like `listAll` that:
  - Call `list(page_size)` repeatedly using `next_page_token`.
  - Respect timeouts and rate limits.
  - Never unboundedly buffer infinite streams (bounded by caller options).

For PHP:

- `Page<T>` can be a value object with:
  - `getItems(): array`
  - `getNextPageToken(): ?string`
- A helper like `Paginator::iterate(callable $fetchPage): \Generator` may provide lazy iteration while still respecting backoff and concurrency hints.

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
public        = false    # if true, route is allowed without caps where safe
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

PHP SDKs that offer facet builders must emit TOML that matches this schema exactly.

### 5.2 Kind Semantics

#### `kind = "static"`

- Routes must include `file` pointing to a readable file.
- Invariants:
  - `method` is usually `GET` (others allowed only if explicitly documented).
  - `file` must be a path **inside a configured facet root or whitelisted dir**.
  - Micronode responds with `200` and file body, `404` if file missing.

#### `kind = "echo"`

- Routes have no `file` or upstream.
- Micronode responds with a simple JSON payload, e.g.:

```json
{ "echo": "ok" }
```

Used for health/debug.

#### `kind = "proxy"` (future, not yet implemented in core)

Design (future schema):

```toml
[facet]
id   = "todo"
kind = "proxy"

[facet.upstream]
scheme    = "http"       # "http" | "https" | future: "oap"
host      = "todo-backend"
port      = 4000
base_path = "/"

[[route]]
method        = "GET"
path          = "/list"   # Micronode path: /facets/todo/list
upstream_path = "/list"   # Upstream path; default: same as path

[[route]]
method        = "POST"
path          = "/create"
upstream_path = "/create"
```

Invariants:

- Micronode route: `/facets/{id}{route.path}`.
- Upstream URI: `${scheme}://${host}:${port}${base_path}${upstream_path}`.
- Security:
  - Upstream requests must be explicitly configured and never default to “open proxy”.
  - SDKs must not silently generate proxy facets without explicit config.

### 5.3 Path & Routing Semantics

- **Actual HTTP path** exposed by Micronode is always:

  ```text
  /facets/{facet.id}{route.path}
  ```

  Example:

  - `id = "docs"`, `path = "/hello"` → `/facets/docs/hello`.

- SDKs must reflect this rule in docs and client helpers, including PHP facet-building helpers.

### 5.4 Security Hints

`[facet.security]` is advisory but may influence Micronode behavior:

- `public = true`:
  - This facet is safe for unauthenticated reads (e.g., docs, static assets).
- `requires_auth = true`:
  - Even in `dev_allow`, this facet should enforce caps.

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

- SDKs that generate manifests **may** populate these, but Micronode will:
  - Enforce them when implemented (limits),
  - Verify integrity before serving static assets if `integrity` is present.
- PHP SDK facet helpers must not silently disable integrity/limits; defaults should lean secure.

---

## 6) SDK Config & Env Schema (Shared)

To avoid each SDK inventing env names:

**Canonical env vars** (already in use for Rust; must be reused in TS/Go/Python/PHP):

- `RON_SDK_GATEWAY_ADDR` – base URL of gateway/omnigate.
- `RON_SDK_OVERALL_TIMEOUT_MS`
- `RON_SDK_CONNECT_TIMEOUT_MS`
- `RON_SDK_READ_TIMEOUT_MS`
- `RON_SDK_WRITE_TIMEOUT_MS`

SDKs:

- MUST support these env vars as defaults.
- MAY allow override via code options or config files.
- PHP:
  - `ClientConfig::fromEnv()` must read these names and apply the same validation (fail-closed on invalid values).

---

## 7) Versioning & Compatibility

- DTOs and manifests follow **semantic versioning**:
  - Backward-compatible additions: new optional fields.
  - Breaking changes: removing fields or changing types.

- Each SDK must:
  - Declare which **protocol/schema version** it targets (e.g., `RON-CORE schema v0.1`).
  - Fail fast or warn loudly if it detects incompatible gateway/proto versions where possible (e.g., via version negotiation endpoints).

PHP:

- `composer.json` and docs must state which protocol/schema versions `ron-app-sdk-php` supports.
- When protocol changes introduce breaking schema changes, a **major SDK version bump** is required.

---

## 8) Codegen & CI Rules

- **Codegen is preferred** over handwritten DTOs wherever tooling allows.
  - Rust/Go/TS likely use direct codegen more heavily.
  - PHP (Tier 2) may start with handcrafted DTOs but must be wired to the same schema.

- CI should include:

  - A **schema drift** check:
    - Regenerate DTOs from `ron-proto`/OpenAPI where codegen exists.
    - Fail if git diff is non-empty or fixtures no longer match.

  - At least one test that hits a local node and asserts:
    - DTO shapes for representative endpoints (e.g., `/app/hello`, storage/list endpoints).
    - Error envelopes match the canonical schema.
    - Pagination envelope semantics (`items`, `next_page_token`).

For PHP:

- Use **golden fixture tests**:
  - Store JSON responses from a test node.
  - Hydrate PHP DTOs and assert that fields roundtrip to JSON according to this schema.

---

## 9) Open Questions / Future Extensions

- `kind = "wasm"` facets:
  - Referencing WASM modules and sandboxed execution.
  - Will introduce additional manifest fields (e.g., `module_path`, `memory_limits`).

- Signed manifests:
  - Integration with `ron-kms` to sign facet bundles.
  - May add fields like `[facet.signature]` with `algo`, `value`, `key_id`.

- ZK-capable caps:
  - DTO changes to support proving capabilities without exposing raw secrets.
  - Likely adds new fields to auth/capability payloads and optional headers.

These are future-facing; keep schema and DTOs flexible to add them without breaking existing SDKs and without breaking `ron-app-sdk-php`’s public surface.

---
````
