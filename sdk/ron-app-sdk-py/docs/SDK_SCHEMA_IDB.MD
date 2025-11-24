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

This doc is the **interop spine** for all SDKs, including `ron-app-sdk-py`.

---

## RO:WHY

Without a unified schema profile:

- Each SDK will slowly invent its own types (e.g. timestamps as `string` vs `number` vs `Date`).
- Error formats drift (TS expects `{ error }`, Rust returns `{ message }`).
- Facet manifests and routing semantics become slightly different per language.

That breaks the core promise of RON-CORE:

> **Same capability + same request ⇒ same behavior**, no matter which SDK or language is used.

This IDB ensures:

- One *canonical schema* (from `ron-proto` and HTTP contracts).
- All SDKs generate or derive types from the same source.
- Facet behavior is consistent and testable across the stack.
- `ron-app-sdk-py` is just another “skin” on the same contract, not a fork.

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
   - For each “tricky” type (e.g. `u64`, timestamps, binary), mapping is consistent across languages.

5. **Forward compatibility**  
   - Adding fields is non-breaking when done according to this profile.  
   - Unknown fields are ignored (or safely surfaced as extension metadata), not fatal.

---

## 0) TL;DR

- **Truth:** `ron-proto` (wire contracts) + gateway HTTP spec (OpenAPI/JSON schema).
- **SDKs:** generate or derive DTOs from that truth; never fork schemas by hand.
- **Errors + pagination:** same envelope everywhere.
- **Facets:** TOML manifests with a strict schema; SDKs can generate them but cannot bend invariants.
- **CI:** schema drift is a build-time failure.

`ron-app-sdk-py` must treat this doc as its **schema bible**.

---

## 1) Canonical Sources of Truth

### 1.1 `ron-proto` (wire-level DTOs)

`ron-proto` serves as the **canonical definition** for:

- OAP/1 frames,
- Storage objects,
- Index entries,
- Auth/Passport/Capability payloads,
- Audit/Registry entries (where exposed to SDKs).

SDKs (Rust, TS, Go, Python, …) either:

- Bind to generated code (where tools exist), or  
- Use JSON/OpenAPI views derived from these proto definitions.

### 1.2 HTTP / OpenAPI Specification

`svc-gateway`, `omnigate`, and future gateways expose:

- HTTP endpoints surface (paths, methods),
- Request/response bodies (JSON),
- Error codes and canonical error envelopes.

These must be captured in:

- OpenAPI 3.x (preferred), or
- A JSON Schema profile that can be used for codegen & validation.

SDKs consume these specs to:

- Generate or verify DTO shapes.
- Assert error and pagination envelopes in tests.

### 1.3 Facet Manifest Schema (TOML)

The facet plane in Micronode is **always** configured via TOML manifests.

- Manifests are **language-agnostic**.
- SDKs may:
  - Generate TOML from builders, or
  - POST manifests into the node (future “register facet” API),
- But must produce TOML that conforms to the schema in §5.

`ron-app-sdk-py` facet helpers (future) must **not** introduce Python-specific schema variants.

---

## 2) DTO & Type Mapping (Cross-language)

### 2.1 Core Types

For each core type, define mapping:

| Canonical Type | Rust               | TypeScript / JS             | Go             | Python                    | Notes                                                                 |
|----------------|--------------------|-----------------------------|----------------|---------------------------|-----------------------------------------------------------------------|
| `bool`         | `bool`             | `boolean`                   | `bool`         | `bool`                    | -                                                                     |
| `u32`          | `u32`              | `number`                    | `uint32`       | `int`                     | Non-negative, < 2^32                                                  |
| `i64`          | `i64`              | `string`                    | `int64`        | `int`                     | TS uses `string` to avoid loss; Python uses `int` but JSON as string* |
| `u64`          | `u64`              | `string` (or `bigint` opt.) | `uint64`       | `int`                     | TS default = `string` in JSON; Python uses `int` but encodes as string|
| `string`       | `String`           | `string`                    | `string`       | `str`                     | UTF-8                                                                 |
| `bytes`        | `Vec<u8>`          | `Uint8Array` / `Buffer`     | `[]byte`       | `bytes`                   | JSON transport = base64url or per-API rule                            |
| `timestamp`    | `DateTime<Utc>`    | `string` (ISO8601)          | `time.Time`    | `datetime.datetime` (tz)  | Always ISO 8601 in JSON, UTC or offset-aware                          |
| `id` (UUID-ish)| `String` / newtype | `string`                    | `string`       | `str`                     | Canonical textual ID (UUID/ULID/etc.), no special wire type           |

\* For `i64` / `u64`:

- **Wire/JSON representation:**  
  - Always serialize as **string** in JSON to avoid precision issues for JS.  
  - TS SDK parses this string into `string` (always) or `bigint` (opt-in).  
  - Python SDK (`ron-app-sdk-py`) parses JSON string into `int`, and serializes back to string when sending.

### 2.2 JSON Encoding Rules

**Nulls vs missing fields**

- Absent optional fields are **omitted** in JSON by default.  
- `null` is used only when semantically necessary (e.g., tri-state, explicit “no value”).

**Unknown fields**

- SDKs must tolerate forward-compatible additions:
  - Rust: prefer `#[serde(default)]` + `#[serde(deny_unknown_fields)]` only when DTO is strictly controlled; otherwise use a catch-all `extra: Map<String, Value>`.
  - TS: allow unknown fields, optionally storing them in `extra?: Record<string, unknown>`.
  - Go: default behavior already ignores unknown fields.
  - Python:
    - Use Pydantic models with `extra='ignore'` or an `extras: dict[str, Any]` field where forward compatibility is needed.
    - For security-critical models (e.g., Problems), we can use `extra='forbid'` but must be careful not to break on non-breaking additions; in those cases, an `extra` map can be used instead.

**Binary**

- If transported in JSON, binary values are encoded as **base64url** strings (document this per API).
- SDKs:
  - TS: `Uint8Array` ↔ base64url string converter.
  - Go: `[]byte` ↔ base64url.
  - Python: `bytes` ↔ base64url (helper functions, possibly in `ron_app_sdk_py.codec`).

**Timestamps**

- Always ISO 8601 strings in JSON (`YYYY-MM-DDTHH:MM:SS[.fraction]Z` or with offset).
- Python:
  - Parsed to `datetime.datetime` with `tzinfo` (UTC or offset).
  - Serialized back to ISO 8601, normalized to UTC or preserving offset per schema.

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
  "details": {               // optional machine-readable detail; MAY be omitted
    "...": "..."
  }
}
````

Invariants:

* `code`, `message`, `kind` are always present (non-empty strings).
* `correlation_id` is present whenever the gateway/omnigate can generate it.
* `details` is optional; when present, it is a JSON object with machine-friendly fields.

### 3.2 Mapping in SDKs

**Rust**

* `struct SdkError { code: String, message: String, kind: String, correlation_id: Option<String>, details: Option<Value> }`
* Integrates with `thiserror` and `tracing`.

**TS/JS**

* `class RonError extends Error { code: string; kind: string; correlationId?: string; details?: unknown; }`
* `message` field on `Error` is the safe `message` field from the envelope.

**Go**

* `type SdkError struct { Code, Kind, Message, CorrelationID string; Details map[string]any }`.

**Python (`ron-app-sdk-py`)**

* Pydantic model:

  ```python
  from pydantic import BaseModel, Field
  from typing import Any, Dict, Optional

  class Problem(BaseModel):
      code: str
      message: str
      kind: str
      correlation_id: Optional[str] = None
      details: Optional[Dict[str, Any]] = None

      class Config:
          extra = "ignore"  # or 'forbid' + explicit extras map when needed
  ```

* Exception classes:

  ```python
  class RonProblemError(Exception):
      def __init__(self, problem: Problem):
          self.problem = problem
          super().__init__(problem.message)
  ```

**SDK invariants for errors**

* Always attempt to parse the canonical envelope.
* If parsing fails (invalid JSON or shape):

  * Wrap in a generic transport/network error (`RonNetworkError`, `RonTransportError`) with a **safe**, high-level message.
  * Raw body only exposed in opt-in debug fields, never as `str(error)`.

---

## 4) Pagination & Collections

### 4.1 Canonical Pagination Envelope

All list-style endpoints use:

```json
{
  "items": [ ... ],
  "next_page_token": "string or null"
}
```

Invariants:

* `items` is **always** present (empty array if no results).
* `next_page_token`:

  * Omitted or `null` means “no more pages”.
  * Non-empty string can be fed to a `page_token` query param on the same endpoint.

### 4.2 SDK Helpers

Each SDK should provide a helper abstraction (e.g., `Page<T>`):

* Rust: `struct Page<T> { items: Vec<T>, next_page_token: Option<String> }`.
* TS: `interface Page<T> { items: T[]; nextPageToken?: string | null; }`.
* Go: `type Page[T any] struct { Items []T; NextPageToken string }` (empty string = no more).
* Python:

  ```python
  from typing import Generic, List, Optional, TypeVar
  from pydantic import BaseModel

  T = TypeVar("T")

  class Page(BaseModel, Generic[T]):
      items: List[T]
      next_page_token: Optional[str] = None
  ```

**Convenience helpers**

* `list_all` or `iter_pages` functions that:

  * Repeatedly call the underlying `list` endpoint with `page_token`,
  * Respect timeouts and rate limits,
  * Do **not** load unbounded data into memory unless explicitly requested.

---

## 5) Facet Manifest Schema (TOML)

This is the **contract** Micronode enforces for facets, regardless of which SDK generates the manifests.

### 5.1 Basic Shape

Each `.toml` file describes **one facet**:

```toml
[facet]
id   = "docs"            # unique within the node
kind = "static"          # "static" | "echo" | "proxy"

[facet.security]
public        = false    # if true, facet is allowed unauthenticated reads (where safe)
requires_auth = true     # if true, always require caps

[facet.meta]
description = "Docs facet"
owner       = "team-or-service-name"
version     = "1.0.0"

[[route]]
method = "GET"
path   = "/hello"
file   = "crates/micronode/dev_facets/hello.txt"
```

Invariants:

* `facet.id` is unique across all facets on a node.
* `facet.kind` determines which other sections/fields are valid.
* There is at least one `[[route]]` defined.

### 5.2 Kind Semantics

#### `kind = "static"`

* `[[route]]` entries must include `file` pointing to a readable file within a facet root.
* Invariants:

  * `method` is typically `GET`.
  * `file` path must not escape the configured facet root (no `../` traversal).
  * Micronode returns file contents, or 404 if missing.

#### `kind = "echo"`

* `[[route]]` entries have:

  * `method` and `path`,
  * No `file` or `upstream` settings.

* Micronode returns simple JSON echo payloads, used for health & smoke tests.

#### `kind = "proxy"` (future)

Future schema (reserved):

```toml
[facet]
id   = "todo"
kind = "proxy"

[facet.upstream]
scheme    = "http"       # "http" | "https" | future: "oap"
host      = "todo-backend"
port      = 4000
base_path = "/api"

[[route]]
method        = "GET"
path          = "/list"
upstream_path = "/list"

[[route]]
method        = "POST"
path          = "/create"
upstream_path = "/create"
```

Invariants:

* Micronode route: `/facets/{facet.id}{route.path}`.
* Upstream URL: `${scheme}://${host}:${port}${base_path}${upstream_path}`.
* Proxy facets must **not** default to open proxy behavior; upstream must be explicit.

### 5.3 Path & Routing Semantics

**Canonical HTTP path** exposed by Micronode:

```text
/facets/{facet.id}{route.path}
```

Examples:

* `id = "docs"`, `path = "/hello"` → `/facets/docs/hello`.
* `id = "todo"`, `path = "/list"` → `/facets/todo/list`.

SDKs:

* Must treat this as fixed behavior in their docs and helper functions.
* If helpers build URLs, they must follow this pattern exactly.

### 5.4 Security Hints

`[facet.security]` is a declarative hint for Micronode/policy:

* `public = true`:

  * Indicates facet is safe to expose to unauthenticated readers (static docs, etc.).

* `requires_auth = true`:

  * Indicates facet should always enforce caps, even in `dev_allow` or permissive modes.

SDKs (`ron-app-sdk-py` included) that generate manifests:

* Must not silently set `public = true` or `requires_auth = false`.
* Must require explicit user choice for these flags.

### 5.5 Limits & Integrity (Optional Fields)

To model rate limits and integrity:

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

* `facet.limits` describes facet-level concurrency and rate caps.
* `route.integrity` enables integrity checks (e.g., SRI-like validations) before serving static content.

SDKs:

* May expose these as optional fields in builder APIs.
* Must not invent additional limit semantics without updating this schema.

---

## 6) SDK Config & Env Schema (Shared)

To avoid each SDK inventing its own env names, we define **canonical env vars**:

* `RON_SDK_GATEWAY_ADDR` – base URL of gateway/omnigate (e.g., `https://localhost:8080`).
* `RON_SDK_OVERALL_TIMEOUT_MS`
* `RON_SDK_CONNECT_TIMEOUT_MS`
* `RON_SDK_READ_TIMEOUT_MS`
* `RON_SDK_WRITE_TIMEOUT_MS`

SDKs:

* MUST support these env vars as defaults.
* MAY allow override via code options or SDK-specific config files.

`ron-app-sdk-py`:

* Reads these env vars when `RonClient` is constructed without explicit config.
* Uses them as default timeouts and base URL.

---

## 7) Versioning & Compatibility

* DTOs and manifests follow **semantic versioning** at the protocol/schema level:

  * Backward-compatible additions: new optional fields.
  * Breaking changes: removing fields, changing types, or changing semantics.

* Each SDK must:

  * Declare in its README/metadata which **protocol/schema version** it targets (e.g., `RON-CORE protocol v0.1`).
  * Where feasible, detect incompatible gateway/proto versions and fail fast with a clear error.

`ron-app-sdk-py`:

* Should expose a `PROTOCOL_VERSION` constant and surface mismatches in errors/logs.

---

## 8) Codegen & CI Rules

**Codegen preferred** over handwritten DTOs wherever tooling exists:

* Rust:

  * `prost` or equivalent from `ron-proto`.
  * Serde structs for HTTP JSON DTOs derived from same definitions.

* TS/JS:

  * OpenAPI codegen or Zod schemas derived from canonical spec.

* Go:

  * OpenAPI / JSON schema codegen.

* Python (`ron-app-sdk-py`):

  * Pydantic models generated from OpenAPI / JSON schema where possible.
  * Handwritten models must be minimal and checked against schema fixtures.

**CI requirements**

* A **schema drift check**:

  * Regenerate DTOs from `ron-proto`/OpenAPI.
  * Fail the build if the generated files differ from checked-in versions.

* Interop tests:

  * At least one test suite that hits a local node and asserts:

    * DTO shapes (`items`, fields, type mapping),
    * Error envelopes (canonical Problem),
    * Pagination behavior (`next_page_token` semantics).

---

## 9) Open Questions / Future Extensions

These are not part of MVP but must be anticipated:

1. **`kind = "wasm"` facets**

   * Facets that load and run WASM modules in a sandbox.
   * Schema likely needs fields like `module_path`, `memory_limit`, `time_limit_ms`.

2. **Signed manifests**

   * Integration with `ron-kms` to sign facet manifests.
   * Additional TOML sections, e.g.:

     ```toml
     [signature]
     algo = "ed25519"
     value = "base64-signature"
     ```

3. **ZK-capable caps**

   * Capability payloads that embed zero-knowledge proofs instead of raw secrets.
   * DTOs for such proofs must be added carefully and consistently.

4. **PQ-aware TLS endpoint hints**

   * Schema additions to advertise PQ capabilities from nodes to SDKs.

As these features land:

* This schema doc must be updated first.
* All Tier-1 SDKs (Rust, TS, Go, Python) must add matching DTOs and tests before a protocol version is considered stable.

---

```
```
