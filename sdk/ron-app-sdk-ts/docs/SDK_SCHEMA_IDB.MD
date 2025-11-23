

# SDK_SCHEMA_IDB — Unified Schema & Facet Contract (ron-app-sdk-ts Profile)

> Applies to: all `ron-app-sdk-*` (Rust, TS, Go, Python, etc.)
> This document is **instantiated for `ron-app-sdk-ts`** (TypeScript SDK), with explicit TS mapping and behavior.

---

## RO:WHAT

Define a **single schema+contract profile** that every SDK must follow, and specify exactly how **`ron-app-sdk-ts`** maps to it:

* DTOs and envelopes (request/response) for RON-CORE services.
* Error, pagination, and metadata envelopes.
* **Facet manifests** (TOML) and their invariants.
* Cross-language type mappings (Rust ↔ TS ↔ Go ↔ Python).
* Codegen + CI rules to prevent drift.

For `ron-app-sdk-ts` this means:

* TS types/interfaces for DTOs derive from the canonical schema.
* Error envelopes and pagination envelopes are parsed into well-known TS shapes.
* Facet manifests are generated as TOML following the same schema as Rust.
* Any future TS codegen/regeneration is wired into CI to reject schema drift.

---

## RO:WHY

Without a unified schema profile:

* TS might treat timestamps as `Date`, Rust as `String`, Go as `time.Time`.
* One SDK might expect `{ error }` while gateway returns `{ message }`.
* Facet manifests could subtly differ per language, making nodes behave differently.

That would violate a core RON-CORE promise:

> “Same capability + same request ⇒ same behavior, regardless of SDK or language.”

This IDB ensures:

* One canonical schema from `ron-proto` and the HTTP contracts.
* `ron-app-sdk-ts` types and envelopes derive from that schema.
* Facet behavior is consistent and testable across Rust/TS/Go/Python.
* CI catches drift **early**, not in prod.

---

## RO:INVARIANTS

Across all SDKs and services, and **concretely enforced in `ron-app-sdk-ts`**:

1. **Single source of truth**

   * DTOs and envelopes come from **`ron-proto` + HTTP/OpenAPI contracts**.
   * TS **never** invents alternative JSON shapes for core RON-CORE DTOs.

2. **Stable envelopes**

   * There is one canonical error envelope and pagination envelope.
   * `ron-app-sdk-ts` parses them into stable TS types.

3. **Uniform facet schema**

   * All facet manifests (TOML) follow the same shape.
   * TS helpers that generate TOML **MUST** respect this schema.

4. **Predictable type mapping**

   * For tricky types (`u64`, timestamps, bytes), mapping is consistent:

     * TS uses `string` for `u64` in JSON by default, with an opt-in `bigint` parse mode.
     * Timestamps are **always ISO 8601 strings** in JSON.

5. **Forward compatibility**

   * New fields are added as **optional** in JSON and TS interfaces.
   * Unknown fields never crash the TS SDK; they’re either ignored or surfaced via an `extra` map.

---

## 0) TL;DR

* **Truth:** `ron-proto` + gateway/omnigate HTTP/OpenAPI schemas.
* **TS SDK (`ron-app-sdk-ts`):**

  * DTOs & envelopes **mirror** those schemas via explicit TS interfaces (and future codegen).
  * Error & pagination envelopes are unified across languages.
  * Facet TOML is generated in TS but must follow the shared manifest schema.
* **CI:** regenerating DTOs or updating schemas that cause drift is a **build failure**.

---

## 1) Canonical Sources of Truth

### 1.1 `ron-proto` (wire-level DTOs)

`ron-proto` defines:

* OAP/1 frames (internal; not directly exposed to TS).
* Storage/index DTOs (e.g., blobs, keys, cursor tokens).
* Auth/passport/capability payloads.
* Audit/registry DTOs exposed to app-plane endpoints.

For `ron-app-sdk-ts`, these show up as:

* JSON DTOs returned by `/app/*` facets that wrap OAP/1 or storage primitives.
* Future codegen: `ron-proto` → JSON schema → TS type definitions.

### 1.2 HTTP / OpenAPI Specification

`svc-gateway`/`omnigate` define:

* Paths, methods, query parameters, request/response bodies.
* Error envelopes (HTTP 4xx/5xx).
* Pagination fields (where applicable).

For `ron-app-sdk-ts`:

* The SDK’s `AppResponse<T>` and `RonProblem` types are aligned with these schemas.
* The TypeScript SDK’s tests treat OpenAPI schemas as **golden truth**.

### 1.3 Facet Manifest Schema (TOML)

Micronode facet plane is configured via TOML manifests.

`ron-app-sdk-ts`:

* May provide **builders** (e.g., `createStaticFacetManifest`) that emit TOML.
* These builders must emit TOML that conforms **exactly** to the shared facet schema.
* TS code never redefines schema; it just maps to it.

---

## 2) DTO & Type Mapping (Cross-language, TS-focused)

### 2.1 Core Types Mapping

| Canonical Type | Rust       | TypeScript / JS                             | Go          | Notes                                                              |
| -------------- | ---------- | ------------------------------------------- | ----------- | ------------------------------------------------------------------ |
| `bool`         | `bool`     | `boolean`                                   | `bool`      | Straightforward                                                    |
| `u32`          | `u32`      | `number`                                    | `uint32`    | Must be in JS safe integer range                                   |
| `i32`          | `i32`      | `number`                                    | `int32`     | Same as above                                                      |
| `i64`          | `i64`      | `string`                                    | `int64`     | TS: `string` to avoid precision issues                             |
| `u64`          | `u64`      | `string` (JSON) + optional `bigint` parsing | `uint64`    | TS JSON always string; optional runtime parse to bigint            |
| `string`       | `String`   | `string`                                    | `string`    | UTF-8                                                              |
| `bytes`        | `Vec<u8>`  | `Uint8Array` / `ArrayBuffer` / `Buffer`     | `[]byte`    | Over HTTP JSON: base64url string; raw body: ArrayBuffer/Uint8Array |
| `timestamp`    | `DateTime` | `string` (ISO 8601, RFC 3339)               | `time.Time` | TS: no `Date` in JSON; app can convert to `Date` as needed         |
| `uuid`/id      | `String`   | `string`                                    | `string`    | TS: treat as opaque string; helpers may validate format            |

**TS policy:**

* All JSON **wire types** for `u64` / `i64` are `string`.
* `ron-app-sdk-ts` may expose **helper functions** to parse `u64` as `bigint`, but the **JSON → TS interface** still uses `string` to avoid a breaking schema.

### 2.2 JSON Encoding Rules (ron-app-sdk-ts)

* **Optional fields**:

  * Missing optional fields are omitted from JSON and typed as `foo?: T` in TS.
  * `null` is reserved for semantically meaningful “explicit null”, not just “no value”.
* **Unknown fields**:

  * TS type definitions only include known fields.
  * At runtime, unknown fields are ignored or optionally collected in an `extra?: Record<string, unknown>` property for advanced clients.
* **Binary**:

  * HTTP JSON: base64url `string`.
  * Raw HTTP body: `ArrayBuffer` / `Uint8Array` via `AppResponse.raw`.
* **Canonical naming**:

  * Field names follow JSON snake_case or lowerCamelCase exactly as defined in the canonical schema.
  * `ron-app-sdk-ts` does **not** rename fields when mapping (no `userId` ↔ `user_id` translation behind the scenes).

---

## 3) Error Envelope Schema

### 3.1 Canonical Error Shape

Gateway/app-plane errors that the TS SDK sees must follow:

```json
{
  "code": "string",
  "message": "string",
  "kind": "string",
  "correlation_id": "string",
  "details": {}
}
```

Where:

* `code` – machine code (`"auth_unauthorized"`, `"problem_400"`, `"upstream_unavailable"`, etc.).
* `message` – safe human-readable message (no secrets).
* `kind` – category (`"auth"`, `"validation"`, `"internal"`, `"rate_limit"`, etc.).
* `correlation_id` – log/trace correlation.
* `details` – optional structured detail (e.g., per-field validation errors).

### 3.2 TS Mapping (`RonProblem`)

`ron-app-sdk-ts` unifies this with app-plane semantics (`retryable`, `retry_after_ms`, `reason`) into:

```ts
export interface RonProblem {
  code: string;
  message?: string;
  kind?: string;
  correlationId?: string;
  retryable?: boolean;
  retryAfterMs?: number;
  reason?: string;
  details?: Record<string, unknown>;
}
```

Rules:

* On parse:

  * `correlation_id` → `correlationId`
  * `retry_after_ms` → `retryAfterMs`
* If body parses as canonical error → set `problem` on `AppResponse`.
* If body is not JSON or not canonical → generate `problem` like:

```ts
{
  code: "transport_error",
  message: "Non-JSON or unknown error format",
  kind: "transport",
  retryable: false
}
```

### 3.3 Exception vs Value

* Server-originated errors are **never thrown** as exceptions in TS.
  They are represented as:

```ts
interface AppResponse<T = unknown> {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  data?: T;
  raw?: ArrayBuffer;
  problem?: RonProblem;
}
```

* TS only throws:

  * programmer errors (invalid base URL, invalid config),
  * catastrophic local errors (no `fetch` implementation, etc.).

---

## 4) Pagination & Collections

### 4.1 Canonical Pagination Envelope

For list endpoints, JSON:

```json
{
  "items": [ ... ],
  "next_page_token": "string or null"
}
```

Semantics:

* `items` – always present (possibly `[]`).
* `next_page_token`:

  * missing or `null` → no more pages.
  * non-empty string → feed back to `page_token` query param.

### 4.2 TS Types & Helpers

TS mapping:

```ts
export interface Page<T> {
  items: T[];
  nextPageToken?: string | null;
}
```

SDK helper pattern:

```ts
async function listAll<T>(
  listFn: (pageToken?: string) => Promise<Page<T>>,
  options?: { maxItems?: number; signal?: AbortSignal }
): Promise<T[]> { ... }
```

Constraints:

* `listAll` must:

  * respect `maxItems` and `signal`,
  * never spin infinite loops,
  * respect rate limiting and backoff (esp. when hitting 429).

---

## 5) Facet Manifest Schema (TOML)

This is a **shared schema**; `ron-app-sdk-ts` may generate it, but can’t change it.

### 5.1 Canonical TOML Shape

```toml
[facet]
id   = "docs"          # unique within node
kind = "static"        # "static" | "echo" | "proxy" (future others)

[facet.security]
public        = false  # safe for unauthenticated reads?
requires_auth = true   # always require caps?

[facet.meta]
description = "Docs facet"
owner       = "team-or-service-name"
version     = "1.0.0"

[[route]]
method = "GET"
path   = "/hello"
file   = "crates/micronode/dev_facets/hello.txt"
```

### 5.2 TS Representation

In `ron-app-sdk-ts`, we define **builder types**:

```ts
export type FacetKind = "static" | "echo" | "proxy";

export interface FacetSecurity {
  public?: boolean;
  requiresAuth?: boolean;
}

export interface FacetMeta {
  description?: string;
  owner?: string;
  version?: string;
}

export interface FacetRouteStatic {
  method: "GET" | "HEAD";
  path: string;
  file: string;
  integrity?: {
    algo: "sha256" | "sha384" | "sha512";
    value: string;
  };
}

export interface FacetUpstream {
  scheme: "http" | "https"; // future: "oap"
  host: string;
  port: number;
  basePath?: string;
}

export interface FacetRouteProxy {
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  path: string;
  upstreamPath?: string;
}

export interface FacetManifestStatic {
  facet: {
    id: string;
    kind: "static";
  };
  "facet.security"?: FacetSecurity;
  "facet.meta"?: FacetMeta;
  route: FacetRouteStatic[];
}

export interface FacetManifestProxy {
  facet: {
    id: string;
    kind: "proxy";
  };
  "facet.security"?: FacetSecurity;
  "facet.meta"?: FacetMeta;
  "facet.upstream": FacetUpstream;
  route: FacetRouteProxy[];
}
```

TS **never** invents extra semantic fields beyond the schema. It simply presents builders that emit valid TOML.

### 5.3 Path & Routing Semantics

Invariant (enforced globally, and reflected in TS docs/helpers):

* Micronode path = `/facets/{facet.id}{route.path}`

TS code examples must reflect that:

```ts
// If facet.id = "docs" and route.path = "/hello"
// client calls GET /facets/docs/hello
```

### 5.4 Limits & SRI

Optional `limits` and `integrity`:

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

TS builders may expose this as optional fields; if set, generated TOML must adhere to the same field names and types.

---

## 6) SDK Config & Env Schema (TS Integration)

Canonical env vars:

* `RON_SDK_GATEWAY_ADDR`
* `RON_SDK_OVERALL_TIMEOUT_MS`
* `RON_SDK_CONNECT_TIMEOUT_MS`
* `RON_SDK_READ_TIMEOUT_MS`
* `RON_SDK_WRITE_TIMEOUT_MS` (if used)

`ron-app-sdk-ts`:

* Reads these as **defaults** in Node environments.
* Never uses them implicitly in browsers (env isn’t a stable concept there).
* Code-level options always override env defaults:

```ts
new Ron({
  baseUrl: process.env.RON_SDK_GATEWAY_ADDR ?? "https://...",
  overallTimeoutMs: Number(process.env.RON_SDK_OVERALL_TIMEOUT_MS) || 10_000,
  ...
});
```

---

## 7) Versioning & Compatibility

* Schema versioning:

  * New fields → optional, added without breaking TS types.
  * Breaking changes require bump of **protocol/schema version**.

`ron-app-sdk-ts` MUST:

* Announce which protocol/schema version it targets (e.g., in README and `package.json` metadata).
* Provide a way to retrieve at runtime:

```ts
Ron.schemaVersion(): string;
Ron.protocolVersion(): string;
```

If the SDK detects obvious version mismatches (e.g., gateway announces incompatible version via header), it should:

* Log a **structured warning**.
* Optionally set a flag inside `Ron` instances (e.g., `isCompatible: boolean`).

---

## 8) Codegen & CI Rules (TS Focus)

Ideal pipeline for TS DTOs:

1. `ron-proto` / OpenAPI → JSON schema
2. JSON schema → TS types (via codegen)
3. TS SDK uses those generated types directly

CI rules:

* Regenerate TS types in CI (or as a pre-commit).
* Fail build if generated TS types differ from committed ones (drift detection).
* Run **interop tests**:

  * call real gateway/omnigate with `ron-app-sdk-ts`
  * assert shapes of responses, errors, and pagination.

Even before full codegen:

* TS types for central DTOs (`RonProblem`, `Page<T>`, facet manifests) must be treated as **canonical** and updated only in sync with upstream schemas.

---

## 9) Open Questions / Future Extensions

Applies globally, but we call out **TS implications**:

* `kind = "wasm"` facets:

  * TS builder APIs for WASM facets that include:

    * module path,
    * allowed host functions,
    * resource limits.

* Signed manifests:

  * TS must leave space for including signatures in facet manifests (likely as additional TOML blocks).
  * Integration with `ron-kms` for secure signing.

* ZK-capable caps:

  * DTOs may acquire new fields for zero-knowledge proofs.
  * TS must treat them as **opaque** but correctly typed and forwarded.

* Strict schema validation:

  * Potential use of Zod or similar for runtime validation in TS.
  * Optional “strict mode” that validates responses against the canonical schema before exposing them to app code.

---
