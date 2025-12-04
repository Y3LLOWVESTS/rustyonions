

---

## Proposed file tree for `ron-app-sdk-swift`

```text
ron-app-sdk-swift/
├─ README.md
├─ SDK_IDB.MD
├─ SDK_SCHEMA_IDB.MD
├─ SDK_SECURITY.MD
├─ Package.swift
├─ .gitignore
├─ .editorconfig
├─ CHANGELOG.md
├─ LICENSE-MIT
├─ LICENSE-APACHE
├─ docs/
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  └─ state.mmd
├─ Sources/
│  └─ RonAppSdkSwift/
│     ├─ RonClient.swift
│     ├─ RonConfig.swift
│     ├─ AppRequest.swift
│     ├─ AppResponse.swift
│     ├─ RonProblem.swift
│     ├─ Types/
│     │  ├─ CodableValue.swift
│     │  ├─ StringInt64.swift
│     │  ├─ StringUInt64.swift
│     │  └─ Page.swift
│     ├─ Transport/
│     │  ├─ HttpTransport.swift
│     │  └─ UrlSessionConfig.swift
│     ├─ Logging/
│     │  └─ RonLogger.swift
│     ├─ Env/
│     │  └─ EnvConfig.swift
│     ├─ Facets/
│     │  └─ FacetClient.swift
│     └─ Internal/
│        ├─ RequestBuilder.swift
│        └─ ResponseParser.swift
├─ Tests/
│  └─ RonAppSdkSwiftTests/
│     ├─ RonConfigTests.swift
│     ├─ RonClientTests.swift
│     ├─ ErrorMappingTests.swift
│     ├─ PaginationTests.swift
│     ├─ UrlBuildingTests.swift
│     └─ InteropTests.swift
├─ examples/
│  ├─ hello-ron/
│  │  ├─ Package.swift
│  │  └─ Sources/HelloRon/main.swift
│  └─ vapor-facet-demo/
│     ├─ Package.swift
│     └─ Sources/App/main.swift
└─ .github/
   └─ workflows/
      └─ swift-ci.yml
```

---

## File-by-file descriptions

### Root

* **`README.md`**
  Human-facing entrypoint: explains what `ron-app-sdk-swift` is, how it fits into RON-CORE, quickstart snippets, diagrams, and basic build/test instructions.

* **`SDK_IDB.MD`**
  Invariant-Driven Blueprint for the Swift SDK: scope, tier, responsibilities, testing expectations, and interop rules specific to this package.

* **`SDK_SCHEMA_IDB.MD`**
  Shared schema + facet contract document (canonical DTO, error, pagination, and facet TOML mapping) for Swift and all other SDKs, kept in this repo as the local, normative copy for the SDK.

* **`SDK_SECURITY.MD`**
  Security checklist for `ron-app-sdk-swift`: how caps are handled, transport hardening, error/log hygiene, DoS safeguards, and browser/server threat model notes.

* **`Package.swift`**
  Swift Package Manager manifest that defines the `RonAppSdkSwift` library target, test target, example targets, and external dependencies (e.g., SwiftLog).

* **`.gitignore`**
  Ignores build artifacts, Xcode/SwiftPM junk, and local tooling outputs (DerivedData, `.build/`, `.swiftpm`, etc.) so the repo stays clean.

* **`.editorconfig`**
  Project-wide formatting rules (indent width, line endings, charset) to keep Swift and Markdown files consistent across editors.

* **`CHANGELOG.md`**
  SemVer-aligned change log tracking every notable change in the SDK, especially breaking API or wire-compat changes.

* **`LICENSE-MIT`**
  MIT license text as one half of the dual licensing model.

* **`LICENSE-APACHE`**
  Apache-2.0 license text as the other half of the dual licensing model.

---

### `docs/` — diagrams & design artifacts

* **`docs/arch.mmd`**
  Mermaid architecture diagram showing how `RonClient` talks to `svc-gateway`/App Plane, how logging/metrics hook in, and where apps/CLIs integrate.

* **`docs/sequence.mmd`**
  Mermaid sequence diagram of a typical call flow: Swift app → `RonClient` → URLSession → gateway → app handler → Problem/AppResponse back.

* **`docs/state.mmd`**
  Mermaid state diagram for SDK lifecycle and error handling (e.g., Configured → Requesting → Backoff/Retry → Shutdown) including how `RonClient` behaves on failures.

---

### `Sources/RonAppSdkSwift/` — core SDK implementation

* **`RonClient.swift`**
  Primary public entrypoint: exposes `RonClient` with `request<T>`, `get/post/put/patch/delete`, optional `facet(id:)`, and `close()`, orchestrating config, transport, error mapping, and hooks.

* **`RonConfig.swift`**
  Defines `RonConfig` (base URL, timeouts, auth tokens, header providers, logging hooks, PQ mode placeholder, etc.) and convenience functions like `fromEnvironment(overrides:)`.

* **`AppRequest.swift`**
  Internal-ish request struct that represents an SDK call in a normalized form (method, path, query, headers, body, idempotency key), before being converted into a concrete `URLRequest`.

* **`AppResponse.swift`**
  `AppResponse<T>` implementation that wraps HTTP status, headers, decoded data (`T`), raw body bytes, and an optional `RonProblem`, plus helpers like `ok` and `map`.

* **`RonProblem.swift`**
  Canonical `RonProblem` error type conforming to `Codable` and `Error`, matching the shared error envelope schema (code, message, kind, correlationId, retryable, details, etc.).

#### `Sources/RonAppSdkSwift/Types/`

* **`Types/CodableValue.swift`**
  JSON “any” wrapper enum (string/int/double/bool/array/object) used for `RonProblem.details` and other fields with open-ended schemas.

* **`Types/StringInt64.swift`**
  Helper wrapper that encodes/decodes `Int64` as a JSON string, enforcing our canonical mapping for `i64` while giving Swift a strongly-typed integer in memory.

* **`Types/StringUInt64.swift`**
  Helper wrapper that encodes/decodes `UInt64` as a JSON string, with explicit failure if the decoded value cannot fit in a `UInt64`.

* **`Types/Page.swift`**
  Generic `Page<T>` type representing the canonical pagination envelope (`items`, `nextPageToken`) and possibly including convenience methods or protocol conformances.

#### `Sources/RonAppSdkSwift/Transport/`

* **`Transport/HttpTransport.swift`**
  Small abstraction over HTTP transport that `RonClient` uses to send `AppRequest` and receive raw `Data`; owns `URLSession` usage, timeout handling, and mapping of low-level errors to a neutral shape.

* **`Transport/UrlSessionConfig.swift`**
  Helpers for building `URLSessionConfiguration` with timeouts, connection limits, and debug TLS options (like allowing self-signed certs in dev), plus a way to inject a custom `URLSession`.

#### `Sources/RonAppSdkSwift/Logging/`

* **`Logging/RonLogger.swift`**
  Lightweight logging facade that either wraps SwiftLog’s `Logger` or a simple protocol so the SDK can log structured events with token-redaction, without hard-coding a logging backend.

#### `Sources/RonAppSdkSwift/Env/`

* **`Env/EnvConfig.swift`**
  Utilities to read and parse standard SDK env vars (`RON_SDK_GATEWAY_ADDR`, `RON_SDK_*_TIMEOUT_MS`), used by `RonConfig.fromEnvironment` and kept separate for testability.

#### `Sources/RonAppSdkSwift/Facets/`

* **`Facets/FacetClient.swift`**
  Small helper type that holds a `RonClient` + facet ID and exposes convenience `get/post/...` methods that automatically prefix paths with `/facets/{id}`, aligning with the shared facet path semantics.

#### `Sources/RonAppSdkSwift/Internal/`

* **`Internal/RequestBuilder.swift`**
  Internal utility responsible for turning `AppRequest` + `RonConfig` into a fully-formed `URLRequest` (base URL, path joining, query encoding, headers, idempotency keys, request IDs).

* **`Internal/ResponseParser.swift`**
  Internal component that takes raw `HTTPURLResponse` + `Data` and decodes them into `AppResponse<T>`, including Problem envelope parsing, synthetic transport problems, and fallback behavior for malformed bodies.

---

### `Tests/RonAppSdkSwiftTests/` — tests & interop

* **`Tests/RonAppSdkSwiftTests/RonConfigTests.swift`**
  Unit tests for `RonConfig` and `EnvConfig`: env var resolution, HTTPS enforcement, default timeouts, and behavior of insecure HTTP flags.

* **`Tests/RonAppSdkSwiftTests/RonClientTests.swift`**
  Tests for high-level `RonClient` behavior using fake/mocked transport: verifies request/response integration, header composition, and correct mapping of transport errors into `AppResponse`.

* **`Tests/RonAppSdkSwiftTests/ErrorMappingTests.swift`**
  Focused tests to ensure canonical Problem JSON (and malformed variants) decode into `RonProblem` correctly, and that non-JSON/timeout cases yield synthetic `transport` problems with `status = 0`.

* **`Tests/RonAppSdkSwiftTests/PaginationTests.swift`**
  Tests for `Page<T>` and `listAll`-style helpers, making sure pagination envelopes are decoded correctly and concatenation logic respects `nextPageToken`.

* **`Tests/RonAppSdkSwiftTests/UrlBuildingTests.swift`**
  Property-style or table-driven tests for URL construction: path joining, query parameter encoding, and correct formation of `/app/*` and `/facets/{id}{path}` URLs.

* **`Tests/RonAppSdkSwiftTests/InteropTests.swift`**
  Integration tests that talk to a real local micronode/macronode via `svc-gateway` (when available), exercising a small set of endpoints to validate DTO shapes, error envelopes, and pagination against live responses.

---

### `examples/` — runnable samples

* **`examples/hello-ron/Package.swift`**
  SwiftPM manifest for a tiny example executable that depends on `RonAppSdkSwift` and demonstrates the “hello world” call used in the README.

* **`examples/hello-ron/Sources/HelloRon/main.swift`**
  Minimal executable example: reads config from env and/or inline, constructs a `RonClient`, calls `/hello` or `/app/hello`, and prints either the greeting or a Problem for smoke testing.

* **`examples/vapor-facet-demo/Package.swift`**
  SwiftPM manifest for a Vapor-based demo server that uses `RonAppSdkSwift` to proxy/augment RON-CORE calls, showing how to integrate the SDK into typical server-side Swift frameworks.

* **`examples/vapor-facet-demo/Sources/App/main.swift`**
  Entrypoint for the Vapor demo: sets up a basic HTTP server, wires in `RonClient` for one or two routes, and demonstrates mapping `RonProblem` to HTTP responses and propagating correlation IDs.

---

### `.github/workflows/`

* **`.github/workflows/swift-ci.yml`**
  GitHub Actions workflow that runs `swift build`, `swift test`, lint/format (SwiftLint or swift-format), and (optionally) interop tests against a local RON-CORE node, enforcing the gates we laid out in SDK_IDB/SDK_SECURITY.

