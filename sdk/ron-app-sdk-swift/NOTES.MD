### BEGIN NOTE - DECEMBER 4 2025 - 12:35 CST

---

## 1. High-level status

**ron-app-sdk-swift** is now a real Swift Package with:

* ✅ `Package.swift` defining:

  * Library target: `RonAppSdkSwift`
  * Test target: `RonAppSdkSwiftTests`
  * Example executable target: `HelloRon` (under `examples/hello-ron`)
* ✅ Core SDK sources under `Sources/RonAppSdkSwift`:

  * `RonClient.swift` – high-level API wrapper.
  * `RonConfig.swift` – SDK configuration (env + programmatic).
  * `RonProblem.swift` – problem+error model.
  * `AppRequest.swift` / `AppResponse.swift` – wire DTOs.
  * `Env/EnvConfig.swift` – environment loader (RON_SDK_*).
  * `Internal/RequestBuilder.swift` / `Internal/ResponseParser.swift`.
  * `Transport/HttpTransport.swift` / `Transport/UrlSessionConfig.swift`.
  * `Types/` – `CodableValue`, `Page`, `StringInt64`, `StringUInt64`.
  * `Logging/RonLogger.swift` – pluggable logger surface.
* ✅ It **builds far enough** that the compiler is doing full module builds, and we’re down to **API polish / signature mismatches**, not structural issues.
* ✅ Macronode integration is wired:

  * You successfully ran:

    ```bash
    RUST_LOG=info \
    RON_GATEWAY_ADDR=127.0.0.1:8090 \
    cargo run -p macronode
    ```
  * `svc-gateway` is listening on `127.0.0.1:8090`, which is exactly what the Swift SDK expects via `RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090`.

Rough completion estimate for **Developer Preview** of the Swift SDK: **~65–70%**.
Core pieces exist; what’s left is fixing compile issues, adding minimal tests, and proving “HelloRon” round-trip works end-to-end.

---

## 2. What we tried in this run

### 2.1. Macronode (Terminal A)

You started the macronode successfully:

* `svc-gateway` bound to `127.0.0.1:8090`
* `svc-storage` and `svc-index` are up
* No errors / panics; node is ready to accept app-plane traffic

So **the backend side was good** for this test.

### 2.2. HelloRon example (Terminal B/C)

From `examples/hello-ron`:

```bash
cd /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-swift/examples/hello-ron

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
swift run HelloRon
```

This is the correct invocation pattern: env vars → `swift run HelloRon` from the example package directory.

The failure is purely **compile-time inside the SDK module**, not in the example itself, and not in RON-CORE.

---

## 3. Why the build failed (root causes)

You hit **two clusters** of compile errors:

1. **`AppRequest` initializer mismatch**
2. **`UrlSessionConfig` / `RonConfig` API mismatch**

Let’s break those down.

---

### 3.1. `AppRequest` initializer mismatch

Error:

```text
/…/RonClient.swift:51:23: error: missing argument for parameter 'queryItems' in call
 49 |         let req = AppRequest(
 50 |             method: "GET",
 51 |             path: path,
    |                       `- error: missing argument for parameter 'queryItems' in call
 52 |             headers: ["Accept": "application/json"],
 53 |             body: nil
```

And similar at line 79 for POST.

The initializer is now:

```swift
// AppRequest.swift
struct AppRequest {
    let method: String
    let path: String
    let queryItems: [URLQueryItem]?
    let headers: [String: String]
    let body: Data?
}
```

So its synthesized init is effectively:

```swift
init(method: String,
     path: String,
     queryItems: [URLQueryItem]?,
     headers: [String: String],
     body: Data?)
```

But `RonClient` is still calling the older 4-parameter flavor:

```swift
let req = AppRequest(
    method: "GET",
    path: path,
    headers: ["Accept": "application/json"],
    body: nil
)
```

**Root cause:** We evolved `AppRequest` to support query parameters but didn’t update `RonClient` call-sites.

**Fix (next instance):**

In `RonClient.swift`, you need to add `queryItems: nil` (or real items later) everywhere you build an `AppRequest`.

For example, for GET:

```swift
let req = AppRequest(
    method: "GET",
    path: path,
    queryItems: nil,
    headers: ["Accept": "application/json"],
    body: nil
)
```

For POST:

```swift
let req = AppRequest(
    method: "POST",
    path: path,
    queryItems: nil,
    headers: headers,
    body: data
)
```

Later we can introduce a higher-level API that takes something like `query: [String: String]?` and map it into `[URLQueryItem]`.

---

### 3.2. `UrlSessionConfig` and `RonConfig` mismatches

Errors:

```text
UrlSessionConfig.swift:13:43: error: cannot call value of non-function type 'URLSessionConfiguration'
13 |         let cfg = URLSessionConfiguration.ephemeral()
   |                                           `- error

UrlSessionConfig.swift:16:48: error: value of type 'RonConfig' has no member 'requestTimeout'
16 |         cfg.timeoutIntervalForRequest = config.requestTimeout

UrlSessionConfig.swift:17:49: error: value of type 'RonConfig' has no member 'overallTimeout'
17 |         cfg.timeoutIntervalForResource = config.overallTimeout

UrlSessionConfig.swift:21:35: error: cannot infer contextual base in reference to member 'reloadIgnoringLocalAndRemoteCacheData'
21 |         cfg.requestCachePolicy = .reloadIgnoringLocalAndRemoteCacheData
```

#### 3.2.1. `URLSessionConfiguration.ephemeral()`

In modern Swift, `ephemeral` is a **static property**, not a function.

* ❌ `URLSessionConfiguration.ephemeral()`
* ✅ `URLSessionConfiguration.ephemeral`

So the line should be:

```swift
let cfg = URLSessionConfiguration.ephemeral
```

Once that’s fixed, the weird “cannot infer contextual base” on `.reloadIgnoringLocalAndRemoteCacheData` probably resolves itself because the type system stops being confused. If it doesn’t, we can be explicit:

```swift
cfg.requestCachePolicy = .reloadIgnoringLocalAndRemoteCacheData
// or:
cfg.requestCachePolicy = URLRequest.CachePolicy.reloadIgnoringLocalAndRemoteCacheData
```

#### 3.2.2. `RonConfig` timeouts

`UrlSessionConfig` is expecting `RonConfig` to expose:

```swift
config.requestTimeout   // per-request timeout
config.overallTimeout   // resource / total timeout
```

But the actual `RonConfig` doesn’t define these yet (it likely has some other fields like `connectTimeoutSeconds`, `readTimeoutSeconds`, etc.).

**We have two options:**

* **Option A (preferred, consistent with other SDKs):**
  Add timeout properties to `RonConfig` that match what we want here.

  For example:

  ```swift
  public struct RonConfig {
      public var baseUrl: URL
      public var insecureHttp: Bool
      public var requestTimeout: TimeInterval
      public var overallTimeout: TimeInterval
      // ...
  }
  ```

  And make sure `EnvConfig` maps `RON_SDK_TIMEOUT_REQUEST_SECONDS` / `RON_SDK_TIMEOUT_OVERALL_SECONDS` (or similar) to those.

* **Option B (adjust UrlSessionConfig to match existing fields):**
  If `RonConfig` already has say:

  ```swift
  public var timeoutSeconds: TimeInterval
  ```

  we change `UrlSessionConfig` to use that single value for both:

  ```swift
  cfg.timeoutIntervalForRequest  = config.timeoutSeconds
  cfg.timeoutIntervalForResource = config.timeoutSeconds
  ```

**Either way:** we must make `RonConfig` and `UrlSessionConfig` agree on the names. Right now they’re out of sync.

---

## 4. Earlier access-level issue (for context)

A previous run (before this snippet) showed:

```text
error: method cannot be declared public because its parameter uses an internal type
public func request<T: Decodable>(_ req: AppRequest) async -> AppResponse<T> {
              ^ AppRequest is internal
```

This is just Swift’s access control rule:

* A `public` method cannot take a parameter whose type is less visible (`internal`).

We have two design options here:

1. **Keep `AppRequest` internal (preferred)**

   * Make `request` internal (drop `public`).
   * This keeps the low-level wire primitive hidden; external users just call `getApp`, `postJson`, or whatever high-level methods we expose.

2. **Expose `AppRequest` as public**

   * `public struct AppRequest { … }`
   * This gives power users direct raw access (like our TS/Py “request” primitives), but increases the public API surface.

For a first dev preview, I’d lean toward option 1: **keep `AppRequest` internal** and make `request` non-public. That removes the error and keeps the API small.

---

## 5. What is already working / implemented

Here’s what we *know* is present and basically correct at a code-structure level:

* ✅ **SwiftPM wiring**:

  * `swift test -v` reaches deep into compilation of `RonAppSdkSwift`.
  * `swift run HelloRon` finds the target when invoked from `examples/hello-ron`.

* ✅ **Core DTOs and helpers**:

  * `AppRequest` / `AppResponse<T>` typed decode path.
  * `RonProblem` modeled similarly to the other SDKs (status, code, message, details).
  * `CodableValue`, `Page<T>`, `StringInt64`, `StringUInt64` mirror the cross-language types.

* ✅ **HTTP transport**:

  * `HttpTransport` wraps `URLSession` and is async/await-based.
  * Transport returns `AppResponse<T>` or maps local errors into a `RonProblem`-typed response.

* ✅ **Env loading and config**:

  * `EnvConfig` reads `RON_SDK_GATEWAY_ADDR`, `RON_SDK_INSECURE_HTTP`, etc., and builds a `RonConfig`.
  * That config is used to create `RonClient`.

* ✅ **Macronode integration path**:

  * We have a canonical dev loop:

    1. `cargo run -p macronode` (gateway on 127.0.0.1:8090)
    2. From `examples/hello-ron`, run HelloRon with env pointing at that gateway.

Once the compile issues are fixed, that round-trip should be able to hit `GET /version` or `GET /healthz` and decode.

---

## 6. What remains to finish Dev Preview

### 6.1. Fix current compile errors

1. **Align `AppRequest` usage in `RonClient.swift`:**

   * Add `queryItems:` argument to all `AppRequest` initializations.
   * Decide whether we want to make `queryItems` part of the public API for users now, or keep it internal.

2. **Fix `UrlSessionConfig`:**

   * Change `URLSessionConfiguration.ephemeral()` → `URLSessionConfiguration.ephemeral`.
   * Align field names with `RonConfig`:

     * Either add `requestTimeout` + `overallTimeout` to `RonConfig`.
     * Or update `UrlSessionConfig` to use whatever timeout properties `RonConfig` currently has.

3. **Resolve access control mismatch:**

   * Either:

     * Make `AppRequest` public, **or**
     * Make `request<T>(_ req: AppRequest)` internal.
   * For now, prefer internal (to keep a small public surface).

Once these are fixed, `swift test` and `swift run HelloRon` should at least **compile**.

---

### 6.2. Make HelloRon actually call RON-CORE

Right now we haven’t seen a runtime log yet (because it didn’t compile). Next steps once it builds:

* Confirm `HelloRon`:

  * Creates a `RonClient` via `EnvConfig.load()` or similar.
  * Calls something like `client.getJson("/version", into: VersionResponse.self)` or `client.route("healthz")`.
  * Prints either:

    * Success (decoded JSON), or
    * Failure (pretty printed `RonProblem`).

* Verify with macronode logs that:

  * Requests hit `svc-gateway` and `svc-index` (or whichever demo endpoint).
  * We see HTTP 200/4xx/5xx in gateway logs matching what the SDK sees.

---

### 6.3. Tests & lint

For a “real” dev preview we still want:

* **Unit tests** in `RonAppSdkSwiftTests`:

  * Config/env loading.
  * Request builder correctness (path+query items).
  * Response parser’s handling of success vs problem envelopes.
* **Interop test** (later step):

  * A minimal test that hits a running macronode (`127.0.0.1:8090`) and asserts we can decode `/version`.
* **Lint / style**:

  * Decide on a SwiftLint config (basic rules only).
  * Run `swift test` + `swift format` / `swiftlint` as our basic gate.

We don’t need all of this before **dev preview**, but we do need at least **compile-clean + passing core tests**.

---

## 7. Suggested dev loop for next instance

When we pick this back up:

1. **Open / paste full sources for these files (if we’re editing):**

   * `Sources/RonAppSdkSwift/AppRequest.swift`
   * `Sources/RonAppSdkSwift/RonClient.swift`
   * `Sources/RonAppSdkSwift/RonConfig.swift`
   * `Sources/RonAppSdkSwift/Transport/UrlSessionConfig.swift`

2. **Apply the fixes:**

   * Add `queryItems:` param in `RonClient`.
   * Fix `URLSessionConfiguration.ephemeral`.
   * Align `RonConfig` timeout fields with `UrlSessionConfig`.
   * Resolve `public` vs `internal` for `AppRequest` + `request()`.

3. **Rebuild & run tests:**

   From repo root (macronode):

   ```bash
   RUST_LOG=info \
   RON_GATEWAY_ADDR=127.0.0.1:8090 \
   cargo run -p macronode
   ```

   From `sdk/ron-app-sdk-swift`:

   ```bash
   swift test -v
   ```

   From `sdk/ron-app-sdk-swift/examples/hello-ron`:

   ```bash
   RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
   RON_SDK_INSECURE_HTTP=1 \
   swift run HelloRon
   ```

4. **If HelloRon runs:**

   * Capture the stdout output.
   * Capture the macronode log line that corresponds to the SDK call.
   * Then we can tighten API naming and docs.


### END NOTE - DECEMBER 4 2025 - 12:35 CST




### BEGIN NOTE - DECEMBER 4 2025 - 13:24 CST

## 1. Completion estimate for `ron-app-sdk-swift`

**Rough overall completion (toward Developer Preview): *~70–75%***

Why that range:

* ✅ Core library compiles cleanly.
* ✅ Core types (config, client, error model, pagination, codable value) are implemented.
* ✅ HTTP transport + URLSession wiring is in place.
* ✅ A first pass of unit tests is in and all pass (`swift test -v`).
* ✅ Basic error mapping logic is implemented and tested.

But:

* ❌ No real **interop test** against a running RON-CORE node yet (InteropTests is just a placeholder).
* ❌ No **examples** yet (CLI/Vapor/app-plane demos).
* ❌ No **CI workflow** wired up.
* ❌ Docs (`README.md`, `SDK_IDB.MD`, `SDK_SECURITY.MD`, `SDK_SCHEMA_IDB.MD`) are still essentially templates / stubs.

So: core is in good shape; the “outer ring” (interop, examples, CI, docs) is what’s mostly left.

---

## 2. Current snapshot (as of this test run)

**Command used:**

```bash
cd ~/Desktop/RustyOnions/sdk/ron-app-sdk-swift
swift build
swift test -v
```

**Result:**

* Build: **success** (`Build complete!`)
* Tests: **10/10 passing**

Test bundles:

* `ErrorMappingTests` – 2 tests, 0 failures
* `InteropTests` – 1 test (placeholder), 0 failures
* `PaginationTests` – 2 tests, 0 failures
* `RonClientTests` – 2 tests, 0 failures
* `RonConfigTests` – 1 test, 0 failures
* `UrlBuildingTests` – 2 tests, 0 failures

Target environment (from the logs):

* Swift version: **6.1.x**
* Target: **x86_64-apple-macos14.0**
* macOS SDK: **macOS 15.5**
* OS at runtime: **15.7.2**

So on your dev box, the SDK is **fully green** in a modern Swift/Xcode toolchain.

---

## 3. What’s implemented (code-level)

### 3.1 Package + layout

We have the core minimal structure:

* `Package.swift`

  * Defines `RonAppSdkSwift` library target.
  * Defines test target `RonAppSdkSwiftTests`.
* `Sources/RonAppSdkSwift/…`

  * Core types, config, transport, logging, facets.
* `Tests/RonAppSdkSwiftTests/…`

  * Unit test suites for config, client, URL building, pagination, and error mapping.

Doc + meta files exist but are mostly templates:

* `README.md`
* `SDK_IDB.MD`
* `SDK_SCHEMA_IDB.MD`
* `SDK_SECURITY.MD`
* `.editorconfig`, `.gitignore`, `CHANGELOG.md` etc.

### 3.2 Core public surface

**RonConfig.swift**

* Holds the user-facing configuration:

  * `baseUrl: URL`
  * `appPrefix: String` (e.g. `/app`).
  * Timeouts, retry/backoff-related knobs (where applicable).
  * Toggle for plain HTTP vs HTTPS (default: **disallow plain HTTP**).
* Invariants:

  * Building with a plain `http://` URL should fail unless `allowInsecureHttp` (or equivalent) is explicitly enabled.
  * Defaults are “safe”: `https` expected, reasonable timeout, sane app prefix.

**RonClient.swift**

* Thin, ergonomic client that app-facing code will use.
* Wraps:

  * `RonConfig`
  * `HttpTransport`
  * `ResponseParser`
* Provides:

  * A core “execute app request” function that:

    * Builds a `URLRequest` from `AppRequest`.
    * Uses `HttpTransport` to perform the HTTP call.
    * Uses `ResponseParser` to decode a `RonProblem` or typed success value.
  * Helper methods (e.g. `get`, `post`) for common patterns (depending on how far we went in the code you have).

The tests confirm:

* Config actually gets stored and used by `RonClient`.
* Plain HTTP is rejected by default.

**AppRequest.swift**

* Encapsulates what we send to the App Plane:

  * HTTP method (GET / POST / etc.).
  * Path under `appPrefix`.
  * Query parameters.
  * Optional JSON body (using `CodableValue`).
* Designed so faceted/typed clients can build requests without caring about `URLRequest` details.

**AppResponse.swift**

* Generic result wrapper: `AppResponse<T: Decodable>`

  * `ok: Bool`
  * `value: T?` (for successful responses)
  * `problem: RonProblem?` (for error cases)
* Invariants:

  * Non-2xx responses are **not treated as thrown errors** – they become `ok == false` with `problem` filled.
  * 2xx + decodable body → `ok == true`, `value` set, `problem == nil`.
  * Decoding failures surface as a **local** `RonProblem` while still returning an `AppResponse`.

**RonProblem.swift**

* Represents both remote and local problems in a RON-CORE-compatible way.
* Fields include:

  * `code: String`
  * `message: String`
  * `kind: String` (e.g. `"remote"`, `"local"`)
  * `correlationId: String?`
  * `details: [String: CodableValue]?`
* Helper constructors:

  * `localDecodingError(underlying: Error)` – for JSON decode failures.
  * `localHttpStatus(_ status: Int, reason: String?, source: String)` – for non-JSON, non-2xx HTTP responses.
* These are the same “shape” as other SDKs (TS/py/php/jvm) so the app layer can treat problems uniformly.

**Types/CodableValue.swift**

* A JSON “sum type” that can represent:

  * `string`, `number`, `bool`, `null`, `array`, `object`.
* Used in:

  * `RonProblem.details`
  * Generic “unknown JSON” fields that we still want to pass through.

**Types/Page.swift**

* Generic pagination envelope, `Page<T: Decodable>`:

  * `items: [T]`
  * `nextPageToken: String?`
* Tests show:

  * Correct decoding of `nextPageToken: null`.
  * Correct decoding when `nextPageToken` is a string.

**Types/StringInt64.swift / StringUInt64.swift**

* 64-bit ints represented as **strings** in JSON (to avoid JS/JSON precision issues).
* Used wherever the schema expects string-encoded 64-bit numeric IDs.

### 3.3 Transport + env

**Env/EnvConfig.swift**

* Helpers to decode configuration from environment variables:

  * `string(key: String, default: String?)`
  * `int(key: String, default: Int?)`
  * `bool(key: String, default: Bool?)`
* Intended to back a `RonConfig.fromEnvironment()` helper, similar to other SDKs.

**Transport/UrlSessionConfig.swift**

* Houses the `URLSessionConfiguration` setup:

  * Timeouts.
  * Caching strategy.
  * TLS/HTTP2 behavior (as much as we configured).
* Goal: one place to tune `URLSession` knobs.

**Transport/HttpTransport.swift**

* Thin wrapper over `URLSession`.
* Responsibilities:

  * Take a `URLRequest`.
  * Return `(Data, HTTPURLResponse)` or throw for true client-side failures (network down, TLS failure, etc.).
  * Apply configured timeouts from `RonConfig`.

### 3.4 Logging

**Logging/RonLogger.swift**

* Protocol + simple default logger:

  * Allows plugging in a custom logging backend per app.
  * SDK itself logs important information (errors, responses) through this abstraction.

### 3.5 Facets

**Facets/FacetClient.swift**

* Early scaffolding for a “facets-aware” client:

  * Probably something like `FacetClient<RQ: Encodable, RS: Decodable>` or equivalent helper.
  * Goal: let apps bind to app-plane facets in a strongly-typed way with minimal boilerplate.
* Currently present but not deeply tested or fully “ergonomic” yet.

---

## 4. Test coverage & what each suite proves

All of these are **green** in your latest run:

### ErrorMappingTests

* **`testDecodesBareRonProblemOnNon2xx`**

  * Feeds in a body that is a raw `RonProblem` JSON for a non-2xx status.
  * Asserts:

    * `ok == false`
    * `value == nil`
    * `problem` is present and correctly decoded (`code`, `message`, etc.)

* **`testLocalHttpStatusFallbackWhenBodyIsNotJson`**

  * Simulates a non-2xx HTTP status with a non-JSON/plain-text body.
  * Asserts:

    * We **do not crash** trying to parse JSON.
    * We create a **local** `RonProblem` via `localHttpStatus`.
    * `problem.code` is the expected local code (e.g. `"swift_sdk_http_status"`).
    * `ok == false`, `value == nil`.

These confirm our error-mapping invariants and keep Swift in lockstep with other SDKs conceptually.

### PaginationTests

* `testPageDecodes`

  * Ensures that a JSON page with `nextPageToken: null` decodes as `nil`.
* `testPageDecodesWithNextPageToken`

  * Ensures that a JSON page with a string `nextPageToken` decodes correctly.

### RonClientTests

* `testClientStoresConfig`

  * Verifies that `RonClient` retains and exposes its `RonConfig`.
* `testInitDisallowsPlainHttpByDefault`

  * Ensures that a config with a `http://` baseUrl (without explicit allowance) is rejected / fails to initialize the client.
  * This enforces the “TLS by default” invariant.

### RonConfigTests

* `testInitWithDefaults`

  * Confirms default values (prefix, timeouts, etc.) are set as expected when you construct `RonConfig` with minimal parameters.

### UrlBuildingTests

* `testBuildsAppPathUnderAppPrefix`

  * Asserts that paths are correctly formed under the configured `appPrefix`.
* `testDoesNotDoublePrefixAppPath`

  * Ensures that if the caller gives a path already under `/app`, we don’t end up with `/app/app/...`.

### InteropTests

* `testInteropPlaceholder`

  * Currently a **placeholder** that always passes.
  * No real network I/O yet; reserved for future **macronode gateway** integration testing.

---

## 5. Invariants to remember (for future changes)

When we resume, we should **preserve** these:

1. **Security:**

   * `RonClient` must **not** default to plain HTTP.
   * A clear, explicit override (e.g. config flag or env var) must be required for `http://`.

2. **Error handling:**

   * Non-2xx responses:

     * Should **not** throw.
     * Should map to `AppResponse<T>` with `ok == false`, `value == nil`, and a filled `RonProblem`.
   * Remote errors:

     * If the body decodes to a `RonProblem`, that must win.
   * Local errors:

     * JSON decode failures → `RonProblem.localDecodingError(...)`.
     * Non-JSON / unparseable bodies on non-2xx → `RonProblem.localHttpStatus(...)`.

3. **Kotlin/JVM/TS/Python alignment:**

   * Problem codes and fields should stay as close as possible to other SDKs:

     * Code names like `"upstream_unavailable"`, `"swift_sdk_http_status"`, etc., should line up conceptually with `"php_sdk_http_status"`, `"py_sdk_http_status"`, etc.

4. **Config:**

   * `RonConfig` should be constructible:

     * Programmatically (explicit parameters).
     * From environment.
   * Environment defaults must be safe, with opt-in for insecure/testing modes.

5. **No surprise throws on “normal” failure paths:**

   * Application-level callers should see `AppResponse` and inspect `ok` / `problem`.
   * Only genuinely exceptional conditions (e.g. network failure, TLS failure) should `throw`.

---

## 6. What remains for `ron-app-sdk-swift` (toward Developer Preview)

### 6.1 Must-haves (code + tests)

1. **Real Interop Test(s)**

   * Replace `InteropTests.testInteropPlaceholder` with a real test that:

     * Reads gateway base URL from env (e.g. `RON_SDK_GATEWAY_ADDR`).
     * Optionally allows insecure HTTP for local dev:

       * `RON_SDK_INSECURE_HTTP=1`
     * Sends a simple request to a RON-CORE app-plane route:

       * e.g. some canonical “ping” or “echo” endpoint shared by other SDKs.
     * Asserts:

       * 2xx status.
       * Decoded body matches expectations (e.g. `{"ok":true}` or an `echo`ed payload).
   * Also consider a **negative interop** case:

     * Gateway not reachable → verify thrown error → map to local `RonProblem` or test-level failure; we’ll decide pattern when we see how other SDKs behave.

2. **EnvConfig + RonConfig.fromEnvironment tests**

   * Add tests equivalent to what we have in JVM/Python/PHP:

     * With env vars set:

       * `RON_SDK_GATEWAY_ADDR`
       * `RON_SDK_APP_PREFIX`
       * `RON_SDK_INSECURE_HTTP`
       * `RON_SDK_TIMEOUT_MS`
     * Ensure `RonConfig.fromEnvironment()`:

       * Correctly picks up values.
       * Applies defaults when missing.
       * Enforces security (e.g. refusing plain HTTP unless `RON_SDK_INSECURE_HTTP=1`).

3. **FacetClient solidification**

   * Decide the final ergonomic API, e.g.:

     * `func call<RQ: Encodable, RS: Decodable>(_ facet: String, method: HttpMethod, path: String, body: RQ?) async throws -> AppResponse<RS>`
   * Add unit tests:

     * Happy path with a mock `HttpTransport`.
     * Error path ensures it surfaces `RonProblem` correctly.
   * Ensure no unnecessary generics complexity that would confuse Swift devs.

4. **More AppResponse & RonProblem Tests**

   * Explicit tests for:

     * `localDecodingError` shape (code, message, kind, details).
     * `localHttpStatus` shape (code, message, status, tagging source as `swift_sdk_http_status` or similar).
   * Round-trip tests:

     * Encode → decode `RonProblem` and `Page<T>` where `T` uses `StringInt64` / `StringUInt64`.

### 6.2 Developer experience / tooling

5. **CI workflow (`.github/workflows/swift-ci.yml`)**

   * Minimal pipeline:

     * `swift build`
     * `swift test -v`
   * Later: add a matrix (different macOS/Xcode/Swift versions) if feasible.

6. **Examples**

   * `examples/hello-cli`:

     * Simple CLI that:

       * Loads config from env.
       * Calls a simple app-plane endpoint.
       * Prints either the decoded value or problem (with correlation ID).
   * `examples/vapor-facet-demo`:

     * Vapor web app that:

       * Exposes a route that itself uses `RonAppSdkSwift` to talk to RON-CORE.
       * Demonstrates mapping `RonProblem` → HTTP status and JSON error payload.
       * Demonstrates correlation ID passthrough in headers.

### 6.3 Documentation

7. **README.md – dev-preview level**

   * Include:

     * “What is RonAppSdkSwift?”
     * Supported platforms (macOS/iOS versions).
     * Basic usage:

       * SwiftPM dependency snippet.
       * `RonConfig` construction (env + code).
       * Creating `RonClient`.
       * Making a simple call.
     * Example code snippets for:

       * Small app-plane call.
       * Error handling via `AppResponse`.

8. **SDK_IDB.MD**

   * Fill in:

     * SDK role.
     * Supported platforms + minimum Swift version.
     * Alignment with other SDKs.
     * Current completion / status (Dev Preview vs Beta).
     * “Six concerns” style notes (SEC/RES/PERF/ECON/DX/GOV) if we mirror your other IDBs.

9. **SDK_SCHEMA_IDB.MD**

   * Document:

     * The app-plane schema expectations this SDK uses.
     * How `Page<T>`, `RonProblem`, and `CodableValue` map onto JSON.
     * Any differences from other SDKs (ideally none).

10. **SDK_SECURITY.MD**

    * Capture:

      * TLS-by-default policy.
      * Behavior of `RON_SDK_INSECURE_HTTP`.
      * How problems are mapped (no information leaks beyond what other SDKs expose).
      * Logging best practices (don’t log secrets, correlation IDs are safe, etc.).

---

## 7. Nice-to-haves (post Dev Preview / pre-Beta)

* **Platform matrix support**

  * Extend beyond macOS to iOS/tvOS/watchOS if needed.
* **More ergonomic sugar**

  * Convenience helpers for:

    * Common patterns (CRUD on app-plane resources).
    * Facet-specific clients.
* **swift-format / SwiftLint integration**

  * Add config + CI gating on style if you want the same “God-tier” cleanliness as other SDKs.

---

## 8. Next time we pick this up – suggested starting steps

When we come back to `ron-app-sdk-swift`, I’d suggest this order:

1. **Re-open InteropTests.swift**

   * Replace the placeholder with a real network test wired to macronode/gateway.
   * Use env-driven config to avoid hard-coding URLs.
   * Verify basic happy-path interop.

2. **Add `RonConfig.fromEnvironment()` + tests**

   * Mirror behavior from JVM/PHP/Python as closely as possible.

3. **Lock down FacetClient**

   * Decide its final public API.
   * Add tests with a mock transport.

4. **Draft README.md**

   * Enough for a Swift dev to:

     * Add the package.
     * Configure it.
     * Do one real call.
   * Full IDB / SECURITY docs can remain “last step,” but let’s at least have a working README stub.

5. **Add `.github/workflows/swift-ci.yml`**

   * Hook it into the monorepo CI story so Swift gets exercised along with the other SDKs.


### END NOTE - DECEMBER 4 2025 - 13:24 CST