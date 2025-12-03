Here’s a JVM-specific instantiation of the schema IDB, parallel to the TS one you already have:

````md
# SDK_SCHEMA_IDB — Unified Schema & Facet Contract (ron-app-sdk-jvm Profile)

> Applies to: all `ron-app-sdk-*` (Rust, TS, Go, Python, JVM, etc.)  
> This document is **instantiated for `ron-app-sdk-jvm`** (Java/Kotlin SDK), with explicit JVM mapping and behavior.

---

## RO:WHAT

Define a **single schema+contract profile** that every SDK must follow, and specify exactly how **`ron-app-sdk-jvm`** maps to it:

- DTOs and envelopes (request/response) for RON-CORE services.
- Error, pagination, and metadata envelopes.
- **Facet manifests** (TOML) and their invariants.
- Cross-language type mappings (Rust ↔ TS ↔ Go ↔ JVM).
- Codegen + CI rules to prevent drift.

For `ron-app-sdk-jvm` this means:

- Java/Kotlin DTO classes derive from the canonical schema (ron-proto + HTTP/OpenAPI).
- Error envelopes and pagination envelopes are parsed into stable JVM types.
- Facet manifests are generated as TOML following the same schema as Rust/TS.
- Any future codegen/regeneration is wired into CI to reject schema drift.

---

## RO:WHY

Without a unified schema profile:

- TS might treat timestamps as `Date`, Rust as `String`, JVM as `Instant`.
- One SDK might expect `{ error }` while gateway returns `{ message }`.
- Facet manifests could subtly differ per language, making nodes behave differently.

That would violate a core RON-CORE promise:

> “Same capability + same request ⇒ same behavior, regardless of SDK or language.”

This IDB ensures:

- One canonical schema from `ron-proto` and the HTTP contracts.
- `ron-app-sdk-jvm` DTOs and envelopes derive from that schema.
- Facet behavior is consistent and testable across Rust/TS/Go/JVM/Python.
- CI catches drift **early**, not in prod.

---

## RO:INVARIANTS

Across all SDKs and services, and **concretely enforced in `ron-app-sdk-jvm`**:

1. **Single source of truth**
   - DTOs and envelopes come from **`ron-proto` + HTTP/OpenAPI contracts**.
   - JVM SDK **never** invents alternative JSON shapes for core RON-CORE DTOs.

2. **Stable envelopes**
   - There is one canonical error envelope and pagination envelope.
   - `ron-app-sdk-jvm` parses them into stable Java/Kotlin types.

3. **Uniform facet schema**
   - All facet manifests (TOML) follow the same shape.
   - JVM helpers that generate TOML **MUST** respect this schema.

4. **Predictable type mapping**
   - For tricky types (`u64`, timestamps, bytes), mapping is consistent:
     - JSON uses **string** for `u64`/`i64` (to be JS-safe).
     - JVM exposes them as `long`/`Long` or `BigInteger` but serializes/deserializes via strings.
     - Timestamps are **always ISO 8601 strings** in JSON.

5. **Forward compatibility**
   - New fields are added as **optional** in JSON and DTO classes.
   - Unknown fields never crash the JVM SDK; they’re either ignored or surfaced via an `extra` map.

---

## 0) TL;DR

- **Truth:** `ron-proto` + gateway/omnigate HTTP/OpenAPI schemas.
- **JVM SDK (`ron-app-sdk-jvm`):**
  - Java/Kotlin DTOs & envelopes **mirror** those schemas via generated or hand-aligned classes.
  - Error & pagination envelopes are unified across languages.
  - Facet TOML is generated in JVM but must follow the shared manifest schema.
- **CI:** regenerating DTOs or updating schemas that cause drift is a **build failure**.

---

## 1) Canonical Sources of Truth

### 1.1 `ron-proto` (wire-level DTOs)

`ron-proto` defines:

- OAP/1 frames.
- Storage objects and index entries.
- Auth/Passport/Capability payloads.
- Audit/Registry entries (where exposed to SDKs).

For `ron-app-sdk-jvm`:

- Prefer codegen from `ron-proto` (via protobufs) or from HTTP/OpenAPI specs.
- Where codegen is not yet wired, DTOs are handwritten but **must** match the canonical schema and be covered by interop tests.

### 1.2 HTTP / OpenAPI Specification

Gateway/omnigate expose:

- HTTP endpoints: paths, methods, status codes.
- Request/response bodies and query parameters.
- Error envelope shape.

For `ron-app-sdk-jvm`:

- We treat the OpenAPI spec as the **primary driver** for client DTOs and service method signatures.
- The SDK should be able to regenerate DTOs from updated OpenAPI and fail CI if there is any drift.

### 1.3 Facet Manifest Schema (TOML)

- Micronode facet configuration is always TOML.
- JVM SDK may:
  - Provide Kotlin/Java builders (`data class FacetManifest`, etc.).
  - Serialize them to TOML using a small TOML writer (hand-rolled or a vetted dependency).
- The emitted TOML **must** follow the shared manifest schema (see §5).

---

## 2) DTO & Type Mapping (Cross-language)

### 2.1 Core Types

Add JVM mapping to the existing cross-language mapping:

| Canonical Type | Rust          | TypeScript / JS          | Go          | JVM (Java)                  | JVM (Kotlin)                      | Notes                                                                      |
|----------------|---------------|--------------------------|-------------|-----------------------------|------------------------------------|----------------------------------------------------------------------------|
| `bool`         | `bool`        | `boolean`                | `bool`      | `boolean` / `Boolean`       | `Boolean`                         | Use primitives in DTO fields, boxed in nullable fields.                    |
| `u32`          | `u32`         | `number`                 | `uint32`    | `long` / `Long`             | `Long`                            | Avoid signed overflow; range-checked when parsing.                         |
| `i64`          | `i64`         | `string`                 | `int64`     | `long` / `Long`             | `Long`                            | JSON encodes as string; SDK parses from string to `Long`.                  |
| `u64`          | `u64`         | `string` or `bigint`*    | `uint64`    | `java.math.BigInteger` or `Long`** | `Long` or `java.math.BigInteger` | JSON encodes as string; safe default = `BigInteger` in DTOs where needed. |
| `string`       | `String`      | `string`                 | `string`    | `String`                    | `String`                          | UTF-8; no surprises.                                                       |
| `bytes`        | `Vec<u8>`     | `Uint8Array` / `Buffer`  | `[]byte`    | `byte[]` / `ByteBuffer`     | `ByteArray`                       | JSON uses base64url; transport-level may be raw bytes.                     |
| `timestamp`    | `DateTime`    | `string` (ISO 8601)      | `time.Time` | `java.time.Instant` (pref) or `OffsetDateTime` | `java.time.Instant` or `kotlinx.datetime.Instant` | JSON always ISO 8601 string; SDK maps to JVM time types with converters. |
| `id` (UUID-ish)| `String`      | `string`                 | `string`    | `java.util.UUID` or `String` | `java.util.UUID` or `String`     | SDK can expose typed UUID helpers but JSON is always string.               |

\* TS default = `string` with optional `bigint` parse mode.  
\** For 64-bit positive values that are known bounded, `Long` is fine; for unbounded counters/ids, `BigInteger` is safer. The SDK may offer both (e.g., `asLong()` / `asBigInteger()` helpers) but JSON stays string.

### 2.2 JSON Encoding Rules

For `ron-app-sdk-jvm`:

- Align with canonical rules:

  - Optional fields:
    - Omitted from JSON when `null`/absence is semantically “unset”.
    - Explicit `null` only when the schema carries a specific meaning for `null`.
  - `u64`/`i64`:
    - Represented as **JSON strings**.
    - JVM SDK parses them into `Long` or `BigInteger` as appropriate.
  - `bytes`:
    - Base64url-encoded strings for JSON; raw bytes for non-JSON content types.
  - `timestamp`:
    - Always ISO 8601 string in JSON (`Instant.parse()` / matching formatter).

- Unknown fields:

  - Kotlin DTOs may have an `extra: Map<String, JsonElement>` or similar to keep unknown fields if needed.
  - Java DTOs may ignore unknown fields by default (typical Jackson behavior).

---

## 3) Error Envelope Schema

### 3.1 Canonical Error Shape

Every SDK-facing error from RON-CORE (over HTTP) uses:

```json
{
  "code": "string",          // short machine code, e.g. "UNAUTH", "NOT_FOUND"
  "message": "string",       // end-user-safe description, no secrets
  "kind": "string",          // category: "auth", "validation", "internal", "rate_limit", etc.
  "correlation_id": "string",
  "details": {               // optional machine-readable detail; may be absent
    "...": "..."
  }
}
```

### 3.2 Mapping in `ron-app-sdk-jvm`

Define a single, canonical error type for the JVM side, e.g.:

- Kotlin:

  ```kotlin
  data class RonProblem(
      val code: String,
      val message: String,
      val kind: String? = null,
      val correlationId: String? = null,
      val details: Map<String, Any?>? = null
  )
  ```

- Java:

  ```java
  public final class RonProblem {
      private final String code;
      private final String message;
      private final String kind;
      private final String correlationId;
      private final Map<String, Object> details;
      // getters, builder, etc.
  }
  ```

SDK invariants:

- HTTP error responses are **always** parsed into `RonProblem`.
- Local transport failures (timeouts, DNS, TLS errors) synthesize a `RonProblem` with:
  - `kind = "transport"`,
  - `code` like `"local_timeout"` / `"local_network_failure"`,
  - no leaking of raw exception messages that might contain secrets.

User-facing APIs can then wrap this in:

- Kotlin:

  ```kotlin
  sealed class RonException(
      message: String,
      val problem: RonProblem? = null,
      val status: Int? = null,
      cause: Throwable? = null
  ) : Exception(message, cause)
  ```

so application code can handle `(status, problem.code)` without parsing strings.

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

- `items` is present and is always an array (possibly empty).
- `next_page_token`:
  - `null`/missing ⇒ no more pages.
  - non-empty string ⇒ feed into `page_token`/`next_page_token` request parameter.

### 4.2 JVM Types & Helpers

In `ron-app-sdk-jvm`:

- Kotlin:

  ```kotlin
  data class Page<T>(
      val items: List<T>,
      val nextPageToken: String? = null
  )
  ```

- Java:

  ```java
  public final class Page<T> {
      private final List<T> items;
      private final String nextPageToken; // nullable
      // getters
  }
  ```

Helper patterns:

- A `listAll` utility that:

  - Accepts a function `(pageToken: String?) -> Page<T>` (Kotlin) or `Function<String, Page<T>>` (Java).
  - Iteratively fetches pages while respecting:
    - `maxItems` limits,
    - caller-provided cancellation (`CoroutineContext` / `CompletableFuture` cancel),
    - backoff rules (429 / rate limit).

- Hard invariant: **never** spin infinite loops on server bugs; always have max pages/iterations or timeout.

---

## 5) Facet Manifest Schema (TOML)

This is a **shared schema**; `ron-app-sdk-jvm` may generate it, but cannot change it.

### 5.1 Canonical TOML Shape

Same as global schema:

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

### 5.2 JVM Representation

In `ron-app-sdk-jvm`, we define **builder types** (Kotlin first-class, Java wrappers):

- Kotlin:

  ```kotlin
  enum class FacetKind { STATIC, ECHO, PROXY }

  data class FacetSecurity(
      val public: Boolean? = null,
      val requiresAuth: Boolean? = null
  )

  data class FacetMeta(
      val description: String? = null,
      val owner: String? = null,
      val version: String? = null
  )

  data class StaticRoute(
      val method: String,   // "GET" | "HEAD"
      val path: String,
      val file: String,
      val integrity: Integrity? = null
  )

  data class Integrity(
      val algo: String,     // "sha256" | "sha384" | "sha512"
      val value: String
  )

  data class FacetLimits(
      val maxRps: Int? = null,
      val maxConcurrency: Int? = null
  )

  data class FacetDefinition(
      val id: String,
      val kind: FacetKind,
      val security: FacetSecurity? = null,
      val meta: FacetMeta? = null,
      val limits: FacetLimits? = null,
      val routes: List<StaticRoute> = emptyList()
      // future: upstream for proxy facets, etc.
  )
  ```

- Java:

  - Provide POJOs or builders mirroring the same fields, so Java users can construct facets without touching TOML directly.

TOML emission:

- A `FacetManifestWriter` converts these JVM types into canonical TOML:

  - `[facet]` / `[facet.security]` / `[facet.meta]` / `[facet.limits]` tables.
  - `[[route]]` entries.
  - `route.integrity` inline tables, etc.

### 5.3 Path & Routing Semantics

The HTTP path exposed by Micronode/gateway is always:

```text
/facets/{facet.id}{route.path}
```

Examples:

- `id = "docs"`, `path = "/hello"` ⇒ `/facets/docs/hello`.

The JVM SDK:

- Documents this rule in Javadoc/KDoc.
- Provides helpers like `facetUrl(baseUrl, facetId, routePath)` so app devs don’t reimplement it incorrectly.

### 5.4 Security Hints & Limits

`[facet.security]`:

- `public = true` ⇒ facet is safe for unauthenticated reads.
- `requires_auth = true` ⇒ enforce caps even in `dev_allow`.

`[facet.limits]`:

- `max_rps`, `max_concurrency` provide rate-limit hints.
- JVM SDK **may** include these builders, but enforcement happens in Micronode/gateway, not the SDK.

Integrity:

- `route.integrity` used for SRI-like checks for static assets.
- JVM `Integrity` type maps 1:1 to TOML fields.

---

## 6) SDK Config & Env Schema (Shared)

To avoid each SDK inventing names, `ron-app-sdk-jvm` reuses canonical env vars:

- `RON_SDK_GATEWAY_ADDR` – base URL of gateway/omnigate (`https://host:port`).
- `RON_SDK_OVERALL_TIMEOUT_MS`
- `RON_SDK_CONNECT_TIMEOUT_MS`
- `RON_SDK_READ_TIMEOUT_MS`
- `RON_SDK_WRITE_TIMEOUT_MS`

JVM specifics:

- For **backend** (Spring Boot, Micronaut, Ktor server, etc.):

  - Read from `System.getenv()` first, with code-level overrides via a builder:

    ```kotlin
    val client = RonClient.fromEnv()
        .withBaseUrl("https://override.example.com")
        .build()
    ```

- For **Android** / frontend:

  - Avoid depending on `System.getenv()`.
  - Use explicit configuration objects injected from app config or BuildConfig, but keep env names for dev tools and tests.

---

## 7) Versioning & Compatibility

- DTOs and manifests follow **semantic versioning** at the protocol/schema level.
- `ron-app-sdk-jvm`:

  - Declares the RON-CORE protocol/schema version it targets (`protocolVersion = "0.x"`).
  - May expose it via `RonClient.protocolVersion` for debugging.
  - Should fail fast (or log loudly) if it detects an incompatible gateway version via health/version endpoints.

Backward-compatible changes:

- New fields added as optional in DTOs.
- Old fields remain until deprecation window expires.

Breaking changes:

- Removing fields or changing types requires a **major** bump in the JVM SDK (`1.x` → `2.x`).

---

## 8) Codegen & CI Rules

### 8.1 Codegen Strategy

For `ron-app-sdk-jvm`:

- Preferred:

  - Use OpenAPI Generator or similar to generate base DTOs (Java/Kotlin).
  - Wrap generated classes in thin, stable facades (so generator output can change without breaking public API).
- Alternative:

  - Handwritten DTOs tightly covered by schema-based tests.

### 8.2 Schema Drift Check

CI must include:

1. **DTO regeneration step** (if using codegen):

   - Regenerate DTOs from the current OpenAPI/ron-proto spec.
   - `git diff` must be empty; non-empty diff = schema drift = CI failure.

2. **Interop tests**:

   - Spin up a local Micronode/gateway in a test profile.
   - Send a known set of requests (`/app/ping`, basic KV put/get, etc.).
   - Assert:
     - DTO shapes match (fields, envelope).
     - Error envelopes parse correctly.
     - Pagination envelope behaves as expected.

3. **Android-specific tests** (if feasible):

   - A small Android test harness using `ron-app-sdk-jvm` to ensure no surprises with Android networking stacks.

---

## 9) Open Questions / Future Extensions

- **JVM-specific codegen pipeline**:
  - Decide between:
    - OpenAPI → Java/Kotlin DTOs,
    - protobuf → gRPC/HTTP clients,
    - combination with manual facades.

- **Kotlin-first vs Java-first API**:
  - Likely: “Kotlin-first API surface, Java-friendly wrappers”.
  - Ensure all public types are Java-usable (no inline/value classes that are hostile to Java without care).

- **Android offline / test server**:
  - Provide a lightweight local mock node or fixture server in JVM to allow Android emulators to run tests without a full RON node.

- **Facet manifest tooling**:
  - CLI or Gradle plugin that:
    - Generates TOML from Kotlin DSL.
    - Validates manifests against `SDK_SCHEMA_IDB` and node expectations.

- **Signed manifests / caps**:
  - Future integration with `ron-kms` to sign facet bundles and capabilities.
  - JVM SDK would:
    - Accept/sign manifests using a KMS client,
    - Attach signatures in a forward-compatible way.

- **WASM/`kind = "wasm"` facets**:
  - Future extension where facets reference WASM modules.
  - JVM SDK should keep facet builder APIs flexible enough to add `kind = "wasm"` without breaking API (e.g., sealed hierarchy with an `Unknown`/`Custom` kind or extension fields).

---

````
