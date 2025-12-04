

````md
# SDK_SCHEMA_IDB — Unified Schema & Facet Contract for SDKs

> Applies to: all `ron-app-sdk-*` (Rust, TS, Go, Python, Swift, etc.)  
> Scope: DTO schemas, error shapes, pagination, **facet manifests (TOML)**, and cross-language type mapping.  
> This copy lives under `ron-app-sdk-swift` but is **normative** for every SDK.

---

## RO:WHAT

Define a **single schema+contract profile** that every SDK must follow:

- DTOs and envelopes (request/response) for RON-CORE services.
- Error, pagination, and metadata envelopes.
- **Facet manifests** (TOML) and their invariants.
- Cross-language type mappings (Rust ↔ TS ↔ Go ↔ Python ↔ Swift, etc.).
- Codegen + CI rules to prevent drift.

This doc is the interop spine for all SDKs.

---

## RO:WHY

Without a unified schema profile:

- Each SDK will slowly invent its own types (e.g. timestamps as `string` vs `number` vs `Date` vs `Date` in Swift).
- Error formats drift (TS expects `{ code, message, kind }`, another SDK returns `{ message, reason }`).
- Facet manifests and routing semantics become slightly different per language.

That breaks the core promise of RON-CORE:

- **Same capability + same request ⇒ same behavior**, no matter which SDK or language is used.

This IDB ensures:

- One *canonical schema* (from `ron-proto` and HTTP contracts).
- All SDKs generate or derive types from the same source.
- Facet behavior is consistent and testable across the stack.
- Swift SDKs (`ron-app-sdk-swift`) are first-class citizens in the same schema universe as Rust/TS.

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
   - For each “tricky” type (e.g. `u64`, timestamps, binary), mapping is consistent across languages, including Swift.

5. **Forward compatibility**  
   - Adding fields is non-breaking when done according to this profile.  
   - Unknown fields are ignored (or safely surfaced as extension metadata), not fatal.

6. **JSON as the lingua franca**  
   - On the wire, SDK-facing HTTP surfaces use JSON for control DTOs and envelopes.  
   - Binary/bulk content is carried as bytes (non-JSON) or as explicit `base64url`-encoded fields.

---

## 0) TL;DR

- **Truth:** `ron-proto` (wire contracts) + gateway HTTP spec (OpenAPI/JSON schema).
- **SDKs:** generate or derive DTOs from that truth; never fork schemas.
- **Errors + pagination:** same envelope everywhere.
- **Facets:** TOML manifests with a strict schema; SDKs can generate them but cannot bend invariants.
- **Swift:** uses `Codable`/`JSONDecoder`/`JSONEncoder` with explicit date and numeric strategies that honor this schema.
- **CI:** schema drift is a build-time failure.

---

## 1) Canonical Sources of Truth

### 1.1 `ron-proto` (wire-level DTOs)

Serves as the **canonical definition** for:

- OAP/1 frames,
- Storage objects,
- Index entries,
- Auth/Passport/Capability payloads,
- Audit/Registry entries (where exposed to SDKs).

SDKs (including Swift) must treat `ron-proto` as the authoritative source when mapping wire DTOs.

### 1.2 HTTP / OpenAPI Specification

- `svc-gateway`, `omnigate`, and future gateways expose:
  - HTTP endpoints (paths, methods),
  - Request/response bodies,
  - Error codes and envelopes.

These should be captured in:

- OpenAPI 3.x or equivalent machine-readable format (JSON Schema is fine as underlying type system).

### 1.3 Facet Manifest Schema (TOML)

- The facet plane in Micronode is **always** configured via TOML manifests.
- These manifests are **language-agnostic** and must not diverge.

SDKs may:

- Generate TOML from builders,
- POST manifests into the node (future “register facet” API),
- But must produce TOML that conforms to this schema.

Swift-specific note:

- `ron-app-sdk-swift` *may* provide a `FacetManifest` Swift struct with `Codable` + TOML encoder/decoder, but that struct must mirror this TOML schema exactly.

---

## 2) DTO & Type Mapping (Cross-language)

### 2.1 Core Types

For each core type, define mapping:

| Canonical Type | Rust             | TypeScript / JS           | Go          | Swift                                      | Notes |
|----------------|------------------|---------------------------|-------------|-------------------------------------------|-------|
| `bool`         | `bool`           | `boolean`                 | `bool`      | `Bool`                                    | - |
| `u32`          | `u32`            | `number`                  | `uint32`    | `UInt32`                                  | Non-negative, < 2³² |
| `i64`          | `i64`            | `string`                  | `int64`     | `Int64` (JSON as string)                  | JSON encodes as string; Swift decodes via custom `Codable` from string |
| `u64`          | `u64`            | `string` or `bigint`*     | `uint64`    | `UInt64` (JSON as string) **or** `String` | On the wire: JSON string; Swift may expose `UInt64` with custom decoding or `String` for lossless representation |
| `string`       | `String`         | `string`                  | `string`    | `String`                                  | UTF-8 |
| `bytes`        | `Vec<u8>`        | `Uint8Array` / `Buffer`   | `[]byte`    | `Data`                                    | If in JSON: base64url; otherwise raw body bytes |
| `timestamp`    | `DateTime`       | `string` (ISO8601)        | `time.Time` | `Date`                                    | Always ISO 8601 in JSON; Swift uses `ISO8601DateFormatter` |
| `id` (UUID-ish)| `String`         | `string`                  | `string`    | `String` (optionally wrapped in `UUID`)   | No special type required; can wrap in a domain type in SDK surface |

\* Each TS SDK can provide a config flag: “parse `u64` as `bigint`”, but **serialized JSON must use string** to avoid ambiguity.

Swift invariants:

- On-wire representation for `i64`/`u64` is always a JSON string.
- `ron-app-sdk-swift` uses custom `Codable` helpers to decode from string into `Int64`/`UInt64` where safe.
- If a numeric value cannot be represented in Swift’s integer range, decoding **must fail with a clear error** rather than silently truncate.

### 2.2 JSON Encoding Rules

- **Nulls vs missing fields**:
  - Absent optional fields are **omitted**, not set to `null`, unless explicitly meaningful in the schema.
- **Unknown fields**:
  - SDKs:
    - Rust: may use `#[serde(default)]` + `#[serde(deny_unknown_fields)]` when safe, or a catch-all `extra` map.
    - TS: may keep unknown fields in a loose `Record<string, unknown>` or ignore them.
    - Go: default JSON decoder ignores unknown fields.
    - Swift:
      - Default `JSONDecoder` ignores unknown keys by default.  
      - **Invariant**: SDK DTO types must *not* use custom `CodingKeys` that throw on unknown keys unless explicitly required; forward compatibility beats strictness.
- **Binary**:
  - If transported in JSON, bytes are **base64url** encoded (documented per API).
  - If transported as raw HTTP body, content-type should be non-JSON (e.g., `application/octet-stream`), and SDKs expose raw bytes (`Data` in Swift).

### 2.3 Swift-Specific Codable Guidance

- All Swift DTOs should conform to `Codable` and prefer:

  - `JSONEncoder` / `JSONDecoder` with:
    - `dateEncodingStrategy = .iso8601`,
    - `dateDecodingStrategy = .iso8601`.
- For `i64`/`u64`:

  - Implement helper wrappers like:

    ```swift
    struct StringInt64: Codable {
        let value: Int64
        // encode/decode from JSON string
    }

    struct StringUInt64: Codable {
        let value: UInt64
    }
    ```

  - Use them for fields that are canonically `i64`/`u64` to guarantee string-on-wire, integer-in-memory mapping.

- DTOs must **never** change field names from the canonical JSON (use `CodingKeys` to map snake_case ↔ camelCase only if strictly necessary; default should be exact match).

---

## 3) Error Envelope Schema

### 3.1 Canonical Error Shape

Every *SDK-facing* error from RON-CORE (over HTTP) should follow this **core** shape:

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
````

Canonical **optional** fields (already used in TS and recommended across SDKs):

```json
{
  "retryable": true,          // whether client may safely retry
  "retry_after_ms": 1000,     // backoff hint in ms (if provided)
  "reason": "string"          // machine-readable reason phrase
}
```

* These optional fields **must not** be required in the schema, but SDKs should understand them if present.

### 3.2 Mapping in SDKs

* **Rust**:

  * `SdkError` with fields `(code, message, kind, correlation_id, details, retryable, retry_after_ms, reason)`.

* **TS/JS**:

  * `class RonError extends Error` with:

    * `code`, `kind`, `correlationId`, `details`,
    * `retryable?`, `retryAfterMs?`, `reason?`.

* **Go**:

  * `type SdkError struct { Code, Kind, Message, CorrelationID string; Retryable bool; RetryAfterMs *int; Reason string; Details map[string]any }`.

* **Swift**:

  ```swift
  public struct RonProblem: Codable, Error {
      public let code: String
      public let message: String
      public let kind: String?
      public let correlationId: String?
      public let details: [String: CodableValue]?

      public let retryable: Bool?
      public let retryAfterMs: Int?
      public let reason: String?
  }
  ```

  * `CodableValue` can be a `enum` wrapper over `String`, `Int`, `Double`, `Bool`, arrays, and dictionaries, used where type is unknown but JSON must be preserved.
  * `RonProblem` conforms to `Error` so Swift code can throw it directly or attach it to higher-level error types.

SDK invariants:

* SDKs **never** blindly print the raw HTTP body as an error string.
* They always attempt to parse this envelope; if parse fails, they wrap it in a generic “transport error” with a safe message and attached debug metadata (status code, content-type, etc.).
* Logging must include `correlation_id` (if present) but **never** raw secrets (auth headers, tokens, capabilities).

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
  * Non-empty string can be fed back to the `page_token` query param.

### 4.2 SDK Helpers

Each SDK should provide:

* A typed `Page<T>` type or equivalent.
* Convenience helpers like `listAll` that:

  * Call `list(page_size)` repeatedly,
  * Respect timeouts and rate limits,
  * Never unboundedly buffer infinite streams (bounded by caller opts).

Swift example:

```swift
public struct Page<T: Codable>: Codable {
    public let items: [T]
    public let nextPageToken: String?
}
```

And a helper like:

```swift
public func listAll<T>(
    pageSize: Int,
    fetchPage: @escaping (String?) async throws -> Page<T>
) async throws -> [T] {
    var all: [T] = []
    var token: String? = nil
    repeat {
        let page = try await fetchPage(token)
        all.append(contentsOf: page.items)
        token = page.nextPageToken
    } while token != nil
    return all
}
```

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

Invariants:

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

Routes forward to an **upstream** service.

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

Swift note:

* If `ron-app-sdk-swift` offers facet builders, they must map 1:1 to this TOML and never silently set `public = true` or relax limits.

---

## 6) SDK Config & Env Schema (Shared)

To avoid each SDK inventing env names:

**Canonical env vars** (already in use; must be reused):

* `RON_SDK_GATEWAY_ADDR` – base URL of gateway/omnigate.
* `RON_SDK_OVERALL_TIMEOUT_MS`
* `RON_SDK_CONNECT_TIMEOUT_MS`
* `RON_SDK_READ_TIMEOUT_MS`
* `RON_SDK_WRITE_TIMEOUT_MS`

SDKs:

* MUST support these env vars as defaults.
* MAY allow override via code options.

Swift specifics:

* For server-side Swift (Vapor, CLI tools), env is read via `ProcessInfo.processInfo.environment`.
* For iOS/macOS apps, env usage is less common; config is normally passed via code (e.g., `RonConfig(baseUrl: ...)`). `ron-app-sdk-swift` should still support env for parity but treat it as *best-effort*, not the only configuration path.

---

## 7) Versioning & Compatibility

* DTOs and manifests follow **semantic versioning**:

  * Backward-compatible additions: new optional fields.
  * Breaking changes: removing fields or changing types.

Each SDK must:

* Declare which **protocol/schema version** it targets (e.g., `protocol = "app-plane v0.1"`).
* Fail fast if it detects incompatible gateway/proto versions (where possible):

  * e.g., if gateway reports an unsupported protocol version in a header.

Swift:

* The Swift package version (SemVer) should track compatibility with RON-CORE protocol versions in its README and `SDK_IDB.MD` matrix.

---

## 8) Codegen & CI Rules

* **Codegen is preferred** over handwritten DTOs wherever tooling allows:

  * For Swift, codegen can be:

    * OpenAPI → Swift `Codable` types,
    * or a small custom generator that emits DTO structs and `Codable` glue.

CI should include:

* A **schema drift** check:

  * For codegen-based SDKs:

    * Regenerate DTOs from `ron-proto`/OpenAPI,
    * Fail if git diff is non-empty.
  * For handwritten mapping (Tier 2 Swift SDK in MVP):

    * At minimum, maintain golden JSON fixtures:

      * Serialize DTOs and compare to expected JSON,
      * Deserialize canonical JSON fixtures and assert DTO fields.
    * If schema changes, fixture updates must be reviewed carefully.

* At least one test that hits a local node and asserts:

  * DTO shapes (fields match, types decode correctly),
  * Error envelopes (canonical problem shape, optional fields, correlation IDs),
  * Pagination behavior (list endpoints, `next_page_token`).

Example CI step for Swift:

```sh
# Run unit + interop tests
swift test --enable-test-discovery
```

---

## 9) Open Questions / Future Extensions

* `kind = "wasm"` facets:

  * Referencing WASM modules and sandboxed execution.

* Signed manifests:

  * Integration with `ron-kms` to sign facet bundles.
  * Swift SDKs may eventually offer helpers to sign manifests offline.

* ZK-capable caps:

  * DTO changes to support proving capabilities without exposing raw secrets.

* Stronger numeric guarantees:

  * Introduce canonical wrapper types for `Money`, `Duration`, etc., across languages, and map them to Swift domain types.

These are future-facing; keep schema flexible to add them without breaking existing SDKs.

```
```
