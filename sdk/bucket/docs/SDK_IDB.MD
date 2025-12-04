
---

# SDK_IDB — ron-app-sdk-swift

---

### SDK Tier & Profile

* **Tier:** **2**

  * Swift is strategically important (Apple platforms + growing server-side ecosystem), but not in the Tier 1 “core set” (Rust, TS, Python, Go, Java, C#). We’ll treat it like Kotlin/PHP: **popular + growing**, with solid CI and interop tests, but not the *first* SDK updated on every protocol bump.

* **Usage profile:**

  * [x] Front-end (iOS / iPadOS / macOS apps, potentially watchOS / tvOS)
  * [x] Backend/facet (server-side Swift: Vapor, Hummingbird, etc.)
  * [x] Dual-use
  * [x] CLI/tools (SwiftPM executables for ops, migrations, demos)

This tier gates:

* §3.4: SDK must be **sandbox-friendly** and documented; helper tooling is nice-to-have, not mandatory.
* §4.3: **Idempotency helpers are strongly recommended** (simple header/key helper).
* §5.3: Baseline benchmarks are expected; perf CI need not be as heavy as Tier 1.
* §10.3: Property + chaos tests are “SHOULD” — we’ll plan them, but can phase in.
* §9.3: CI = lint + tests + interop; perf/chaos can come later.

---

### 0) RO:WHAT / WHY / INTERACTS

**RO:WHAT —**
`ron-app-sdk-swift` is the **Swift client SDK for the RON-CORE App Plane**, providing an async/await-friendly `RonClient` to call `/app/*` endpoints via `svc-gateway` from iOS/macOS apps, Swift server apps, and Swift CLI tools, mirroring the semantics of the TypeScript SDK (`Ron`) and Rust reference client.

**RO:WHY —**

* Swift is the **native language of the Apple ecosystem**:

  * iOS / iPadOS / macOS / watchOS / tvOS applications.
  * Native-feeling client SDK means RON-CORE can be used as *the* backend for mobile and desktop apps.
* Swift is also increasingly used for **servers and CLIs**:

  * Vapor/Hummingbird services.
  * SwiftPM-based tools and migration utilities.
  * This makes the SDK inherently **dual-use**: one client for both apps and backend services.
* `ron-app-sdk-swift`:

  * Hides all `/app/*` URL building, headers, timeouts, and error envelope mapping.
  * Enforces **HTTPS-by-default** and consistent error modeling (via `RonProblem` + `AppResponse<T>`).
  * Gives Swift devs “one import and one client” instead of re-implementing URLSession + JSON + auth headers for each project.

**RO:INTERACTS —**

Via `svc-gateway`’s App Plane, this SDK ultimately talks to:

* `svc-gateway` — **only** via HTTP(S) on the app plane (`/app/*`).
* `omnigate` — behind the gateway; Swift SDK does not care, but all App Plane semantics flow from here.
* App handlers (usually Rust) — the Swift client sees their outputs as JSON and Problem envelopes.
* Auth plane backend:

  * `svc-passport` / `ron-auth` via app endpoints for login, capability issuance, and passports.
* Storage/index (indirectly):

  * `svc-storage` / `svc-index` are consumed by backend handlers; Swift only sees the app endpoints that wrap them.
* Potential future admin UX:

  * Selected `svc-admin` / `svc-registry` app endpoints, if/when they expose App Plane-compatible APIs.

**RO:INVARIANTS —**

* **App Plane only**: SDK never talks to internal service ports; always via `svc-gateway` `/app/*`.
* **OAP/HTTP limits respected**:

  * Effective payloads per HTTP call stay within the frame constraints (≤ 1 MiB effective body).
  * Large logical payloads (e.g., file-like blobs) must be chunked or offloaded; the SDK won’t pretend to support unbounded blobs.
* **No global singletons**:

  * No global mutable shared state; no static “default client” needed to function.
  * `RonClient` instances are cheap to create and safe to share within a process.
* **Forward compatibility**:

  * Unknown JSON fields are ignored when decoding DTOs, not fatal.
  * Error envelope layout remains compatible with `RonProblem` canonical shape.

---

### 1) Scope & Target Profiles

#### 1.1 Supported usage modes

* [x] Front-end (mobile/desktop)

  * iOS / iPadOS / macOS apps using `async`/`await` and URLSession.
* [x] Backend / facets

  * Server-side Swift (e.g., Vapor routes) calling micronode/macronode App Plane endpoints.
* [x] Dual-use

  * Same SDK usable from app targets and SwiftPM libraries/executables.
* [x] CLI / tools

  * SwiftPM executables for ops tooling, migrations, and test harnesses.

We explicitly **avoid** dependencies on UI frameworks (UIKit, SwiftUI) in the core library for maximum portability (Linux, macOS CLI, server).

#### 1.2 Supported node profiles

* [x] Micronode
* [x] Macronode
* [x] Both (default)

Profile notes:

* Core `/app/*` APIs behave the same for both profiles; the SDK is **profile-agnostic** for standard calls.
* Certain future admin-style app endpoints may exist only on macronodes; SDK doesn’t special-case but documentation will call that out.
* **Amnesia mode**:

  * Node may aggressively discard local state.
  * SDK itself does not hold server-side assumptions; persistence (Keychain, UserDefaults) is client-side.
  * Docs must warn: long-lived state should be modeled in external systems or via re-hydratable data, not assumptions about node memory.

#### 1.3 Feature scope (MVP vs future)

**MVP (0.1.x):**

* [x] Auth: attach macaroon / bearer / passport tokens via headers.
* [x] Basic request/response calls:

  * `request<T>(...)`
  * `get/post/put/patch/delete` for JSON App Plane calls.
* [ ] Streaming (SSE/WebSocket) subscribe:

  * **Out-of-scope for 0.1**, reserved for a later minor once gateway streaming semantics are locked.
* [x] Error handling:

  * Idiomatic Swift `Error` for config/local failure.
  * Canonical `RonProblem` + `AppResponse<T>` for server-originated errors/problems.
* [x] Minimal logging/tracing hooks:

  * Pluggable hooks for request/response, correlation IDs, SwiftLog integration.

**Future (≥ 0.2 / gated by RON-CORE milestones):**

* [x] Schema-generated DTOs from `ron-proto` (see §11).
* [x] Streaming support with auto-reconnect/backoff.
* [x] Built-in metrics hooks:

  * Call timing, error codes, surfacing to SwiftLog or custom collector.
* [x] Facet lifecycle helpers (register/heartbeat) once a facet registration API exists.
* [x] Higher-level convenience APIs:

  * `FacetClient` sugar (`client.facet("auth")`) mirroring TS `FacetClient`.

---

### 2) API Design & DX Basics

#### 2.1 Primary abstraction

Primary entrypoint: **`RonClient`** created from **`RonConfig`**.

Minimal “hello world” example (copy-paste, Swift 5.9+):

```swift
import Foundation
import RonAppSdkSwift

struct Greeting: Codable {
    let message: String
}

@main
struct HelloRon {
    static func main() async {
        do {
            // Prefer env var for base URL in dev:
            //   RON_SDK_GATEWAY_ADDR=https://my-node.example.com
            let config = try RonConfig.fromEnvironment(
                overrides: RonConfig(
                    allowInsecureHttp: true, // local dev only
                    debug: true
                )
            )

            let client = try RonClient(config: config)

            let res: AppResponse<Greeting> = await client.get("/hello")

            if res.ok, let data = res.data {
                print("Server says:", data.message)
            } else if let problem = res.problem {
                print("Call failed:", problem.code, "-", problem.message)
            }
        } catch {
            // SDK-local errors: misconfiguration, invalid URL, etc.
            print("RON SDK error:", error)
        }
    }
}
```

**DX goals:**

* You should **not** need to read the manual to understand this.
* APIs feel like “URLSession + Codable + typed error envelopes,” not a foreign TS/Rust wrapper.

#### 2.2 Core operations

`RonClient` will expose:

* Constructor:

  * `init(config: RonConfig) throws`
* Low-level primitive:

  * `func request<T: Decodable>(_ req: AppRequest) async -> AppResponse<T>`
* HTTP helpers:

  * `func get<T: Decodable>(_ path: String, query: [String: String]? = nil) async -> AppResponse<T>`
  * `func post<T: Decodable>(_ path: String, body: Encodable? = nil, query: [String: String]? = nil) async -> AppResponse<T>`
  * `put`, `patch`, `delete` with similar signatures.
* Facet sugar (0.1 or 0.2 depending on complexity):

  * `func facet(_ id: String) -> FacetClient`
  * `FacetClient` prefixes `/facets/{id}` to paths under the hood.
* Streaming (future):

  * `func subscribe(...)` returning `AsyncSequence<Event>` or Combine `Publisher`.
* Cleanup:

  * `func close()` — currently likely a no-op (URLSession-managed), but important for future custom transports.

#### 2.3 Error model

**Two layers of errors:**

1. **SDK-local errors** (`Error`-conforming types):

   * `SdkConfigError` — invalid config (no base URL, insecure HTTP without opt-in, etc.).
   * `LocalNetworkError` — catastrophic local failures that can’t be mapped into a Problem (e.g., we couldn’t even make a request).
2. **Server-originated problems** (canonical RON-CORE problem envelope):

   * `RonProblem` struct:

     * `code: String`
     * `message: String`
     * `kind: String` (e.g., `auth`, `policy`, `app`, `transport`)
     * `status: Int`
     * `correlationId: String?`
     * `retryable: Bool?`
     * `retryAfterMs: Int?`
     * `reason: String?`
     * `details: [String: AnyCodable]?` (or similar JSON-ish blob)
   * `AppResponse<T>`:

     * `status: Int`
     * `ok: Bool`
     * `headers: [String: String]`
     * `data: T?`
     * `raw: Data?`
     * `problem: RonProblem?`

**Mapping rules:**

* Valid Problem JSON → decode to `RonProblem` and attach to `AppResponse.problem`.
* Non-JSON or malformed error bodies:

  * Synthesize a `RonProblem` with:

    * `kind = "transport"`
    * `status = 0`
    * `retryable = true` (where appropriate)
    * safe `message` like `"transport_error"` (no leaking low-level details).
* Network timeouts / DNS failures:

  * Prefer mapping to synthetic `AppResponse` with `status = 0` + `problem.kind = "transport"` and `retryable = true`.
  * Only throw `LocalNetworkError` for truly local misbehavior (e.g., programming error, invalid URL, encode crash).

**Idiomatic usage:**

* Most app code:

  * `let res = await client.get("/foo")`
  * Inspect `res.ok` / `res.data` / `res.problem`.
* Config issues:

  * `try RonClient(config:)` inside `do { } catch { ... }`.

#### 2.4 Configuration

Config object (conceptual):

```swift
public struct RonConfig {
    public var baseUrl: URL?
    public var overallTimeoutMs: Int?
    public var connectTimeoutMs: Int?
    public var readTimeoutMs: Int?

    public var authToken: String?
    public var passportToken: String?
    public var headerProvider: (() async -> [String: String])?

    public var requestIdFactory: (() -> String)?
    public var idempotencyKeyFactory: (() -> String)?

    public var allowInsecureHttp: Bool
    public var debug: Bool

    // Optional: custom URLSession or transport
    public var urlSession: URLSession?
    // Optional: logger integration
    public var logger: Logger?
}
```

Config sources:

* **Environment variables** (primary for server/CLI):

  * `RON_SDK_GATEWAY_ADDR` → default base URL.
  * `RON_SDK_OVERALL_TIMEOUT_MS` → default overall timeout.
* **Code-based config**:

  * Explicit `RonConfig(...)` initialization.
  * `RonConfig.fromEnvironment(overrides:)` merging env + code.
* **TLS & security**:

  * HTTPS-only by default; `http://` baseUrl is **rejected** unless `allowInsecureHttp` is true (for local dev).
  * Cert pinning examples in docs via `URLSessionDelegate`; not hard-coded into SDK.
* **Retries/backoff**:

  * No implicit retries in MVP; we provide timeouts and idempotency header hooks, apps optionally implement their own backoff.
* **Logging/tracing hooks**:

  * Request/response hooks and a `Logger` integration:

    * Redact tokens in log output.
    * Log `correlationId` for tracing.

#### 2.5 Language-idiomatic patterns

* **Async model**: Swift Concurrency (`async/await`) throughout. No callback-based APIs in the core library.
* **Naming conventions**:

  * Types: `RonClient`, `RonConfig`, `RonProblem`, `AppResponse`, `AppRequest`.
  * Methods: lowerCamelCase (`get`, `post`, `facet`, `subscribe`, `close`).
* **Integration patterns**:

  * iOS/macOS apps:

    * Use `Task` and `await` from view models / controllers; handle `AppResponse` cleanly.
  * Server-side Swift:

    * Call from Vapor route handlers or similar; map `RonProblem` to HTTP responses.
  * CLI:

    * Use `@main` async entrypoints or `Task` wrappers.

Rule of thumb: this SDK should **feel like it was written by a Swift developer who’s never seen our TS code**, but the behavior should match TS precisely.

---

### 3) Security & Capabilities (SEC)

#### 3.1 Auth flow

* SDK accepts credentials via `RonConfig`:

  * `authToken` → `Authorization` header (`Bearer`, macaroon, or other scheme).
  * `passportToken` → `X-RON-Passport` header.
  * `headerProvider` → async function to attach or rotate headers per call (useful for session-based app auth).
* Storage:

  * Tokens are **never persisted** by the SDK.
  * Mobile apps should store them in Keychain or secure storage; server apps can use env vars or config files.
* Logging:

  * All logs redact:

    * `Authorization`
    * `X-RON-Token`
    * `X-RON-Passport`
  * Only correlation IDs, statuses, and safe metadata appear in logs.

#### 3.2 Capability boundaries

* SDK itself does **not** mint capabilities; it only carries them.
* Patterns showcased in docs:

  * Tenant-scoped capabilities (per-customer tokens).
  * Read-only tokens for metrics dashboards.
  * Short-lived, session-bound tokens for mobile apps.
* `headerProvider` makes it easy to:

  * Swap tokens based on active user.
  * Refresh tokens without reconstructing `RonClient`.

#### 3.3 Secure defaults

Checklist:

* [x] HTTPS enforced unless `allowInsecureHttp` is explicitly true.
* [x] Credentials never logged; logs are token-redacted.
* [x] Reasonable default timeouts; no infinite-hang calls.
* [x] Debug mode gated by `debug` flag; off by default.
* [x] PQ-ready toggle placeholder:

  * Optional `pqMode: PQMode = .off` reserved in config; no-op until RON-CORE’s PQ profile + Swift TLS stack are ready.

#### 3.4 Sandboxing for Facets/UI

* **No global state**; all configuration flows through `RonConfig` and `RonClient`.
* **Injectable transport**:

  * `urlSession` can be overridden for tests/sandboxed runtimes.
* Examples (docs):

  * Running `RonClient` in an isolated actor or process for untrusted facet logic.
  * Using separate `RonClient` instances with strictly scoped tokens for untrusted modules vs core app.

Sandbox decisions like seccomp/WASM/containerization happen at the node or host level, not in the SDK, but the SDK must **not** make them harder.

---

### 4) Reliability & Resilience (RES)

#### 4.1 Connection behavior

* Each `request` call is independent; **no stateful connection** required.

* Handling:

  * DNS failures / TLS handshake issues / low-level socket errors:

    * Prefer mapping to synthetic `AppResponse` with:

      * `status = 0`
      * `problem.kind = "transport"`
      * `problem.retryable = true`
    * Only throw if we cannot construct a valid request at all.
  * Timeouts:

    * Controlled via `overallTimeoutMs` and per-stage timeouts.
    * Timeouts yield `AppResponse` with `status = 0` and `transport` kind.

* PQ-hybrid handshakes are future work; no special logic in MVP.

#### 4.2 Streaming resilience

* **Not in MVP**, but when added:

  * Subscriptions will likely use `AsyncSequence` or Combine.
  * On disconnect:

    * Transparent reconnect with backoff for durable, idempotent streams.
    * Surface `retryable` hints via `RonProblem` when a stream fails.

#### 4.3 Backoff & rate limiting

* Built-in in MVP:

  * `idempotencyKeyFactory` to generate an `X-Idempotency-Key` header for writes.
* Not built-in in MVP:

  * Automatic retries or rate limiting; we’ll document recommended patterns (exponential backoff around `request`).
* Guidance:

  * **Share `RonClient` instances per base URL** rather than creating one per call.
  * Avoid unbounded loops that call the node as fast as possible; always incorporate delays/timeouts.

---

### 5) Performance (PERF)

#### 5.1 Expected usage profile

* Mobile/desktop:

  * Low to moderate RPS per client (few calls per screen/action).
* Server/CLI:

  * Medium RPS; typically invoked from HTTP handlers or scheduled jobs.
* Payload sizes:

  * Typical JSON payloads in the 1–64 KiB range.
  * Hard upper bound ~1 MiB per call, aligned with RON-CORE frame constraints.

#### 5.2 Implementation choices

* HTTP client:

  * Standard `URLSession`, using the built-in connection pool and platform TLS.
* Streaming (future):

  * `URLSession` WebSockets or SSE implementation.
* Serialization:

  * JSON via `JSONEncoder`/`JSONDecoder`.
  * `Codable` for DTOs; custom encoders for special numeric handling (e.g., i64/u64 represented as strings over the wire).

#### 5.3 Benchmarks

Tier 2 “should” level:

* Microbenchmarks:

  * Single `GET /app/hello` latency vs raw URLSession call; aim for SDK overhead < 5% vs a bare-bones implementation.
* Sustained RPS:

  * From a Swift CLI hitting a local node on loopback, maintain stable memory and CPU usage at, e.g., 100–500 req/s.
* Streaming (future):

  * Simple tests measuring overhead per event.

Benchmarks can be run locally and optionally in CI as a non-blocking job.

---

### 6) Economics (ECON)

#### 6.1 Dev productivity

Compared to raw URLSession + JSON:

* Removes repetitive boilerplate:

  * Building `/app/*` URLs, query strings, and headers (auth, passports, correlation IDs).
  * Mapping HTTP codes to `AppResponse.ok` vs `problem`.
  * Handling timeouts and idempotency keys.
* Result:

  * Typical “call an endpoint” scenario: 3–6 lines of actual app code instead of 25–40 lines of plumbing.

#### 6.2 Operational cost

* Recommend **one `RonClient` per base URL per process**, shared across request handlers where appropriate.
* URLSession’s internal connection pool is leveraged; we don’t spin up our own threads/pools.
* No internal caches or long-lived buffers in MVP; footprint is mostly per-request allocations.

---

### 7) Developer Experience (DX) — Onboarding & Familiarity

#### 7.1 “No manual” quickstart

Three must-have README snippets:

1. **“Hello world” call**
   (Already shown in §2.1)

2. **“Subscribe to a stream”** (future, but shape reserved):

   ```swift
   struct Event: Codable {
       let kind: String
       let payload: String
   }

   let stream = try await client.subscribe(Event.self, path: "/events/logs")

   Task {
       do {
           for try await evt in stream {
               print("Event:", evt.kind, evt.payload)
           }
       } catch {
           print("Stream failed:", error)
       }
   }
   ```

3. **“Facet-style worker loop” (backend)**:

   ```swift
   struct LoginRequest: Codable { let user: String; let pass: String }
   struct LoginResponse: Codable { let token: String }

   func login(user: String, pass: String, client: RonClient) async throws -> LoginResponse {
       let auth = client.facet("auth")
       let res: AppResponse<LoginResponse> = await auth.post(
           "/login",
           body: LoginRequest(user: user, pass: pass)
       )

       if res.ok, let data = res.data {
           return data
       }
       // Prefer mapping problem → local Error type
       if let problem = res.problem {
           throw RonSdkError.problem(problem)
       }
       throw RonSdkError.unknown
   }
   ```

All three:

* Fit in ~10–20 lines.
* Assume configuration via env or minimal explicit config.

#### 7.2 Framework adapters / helpers

Planned adapters:

* [x] CLI helpers:

  * Example SwiftPM executable in `examples/cli-hello/`.
* [ ] Vapor integration:

  * Small helper or middleware to map `RonProblem` to HTTP responses and propagate correlation IDs.
* [ ] Admin GUI helpers:

  * Future SwiftUI example app using the SDK to render node health/metrics.
* [x] Logging/tracing:

  * Integration with SwiftLog (`import Logging`), optionally wired via `RonConfig.logger`.

#### 7.3 Language-idiomatic docs

* Code samples use:

  * Swift 5.9 syntax.
  * `async/await` and `Codable`.
* Error docs clearly split:

  * What throws (`SdkConfigError`, rare `LocalNetworkError`).
  * What is expressed as `AppResponse.problem` (`RonProblem`).
* No TS or Rust jargon; we explain things in “Swift dev” language.

---

### 8) Extensibility & Plugins

#### 8.1 Extension points

**MVP:**

* [x] Request/response hooks:

  * `onRequest: ((AppRequest, URL) -> Void)?`
  * `onResponse: ((AppResponse<Any>) -> Void)?`
  * Intended for logging/tracing/custom headers.
  * Hooks must **never** crash the SDK (exceptions caught and ignored).

**Future:**

* [ ] Middleware pipeline:

  * Pluggable `RequestInterceptor` / `ResponseInterceptor` chain for advanced cross-cutting concerns.
* [ ] Custom transports:

  * Optionally allow replacing URLSession with a custom HTTP client for specialized environments.

#### 8.2 UI widgets/components

* Core SDK remains UI-agnostic.
* Future: `ron-app-sdk-swift-ui` for SwiftUI helpers:

  * `NodeHealthBadgeView`
  * `MetricsPanelView`
  * `EventLogListView`

Widgets accept a `RonClient` or config and keep wiring minimal for GUI builders.

#### 8.3 Plugin model (future)

* Third-party extensions are expected to live as **utility libraries** built on top of the SDK, not as dynamic plugins loaded at runtime.
* Core constraints:

  * Keep the core SDK lean, predictable, and secure.
  * Avoid dynamic plugin loading that could become a security footgun, especially on mobile.

---

### 9) Governance & Versioning (GOV)

#### 9.1 Version scheme

* Package: `ron-app-sdk-swift` (SwiftPM).
* SemVer mapping:

  * `0.1.x` — Developer Preview / MVP aligned with RON-CORE protocol v0.x App Plane.
  * Major bumps reserved for:

    * Breaking API changes.
    * Deep behavior changes that affect wire compatibility.
  * Deprecation:

    * Use `@available(*, deprecated, message: "...")` for at least 2 minor versions before removal.

#### 9.2 Compatibility matrix

| SDK version | Protocol version | Min node version | Notes                            |
| ----------- | ---------------- | ---------------- | -------------------------------- |
| 0.1.x       | v0.x (App Plane) | TBD (M-series)   | MVP: HTTP JSON, no streaming yet |

#### 9.3 Support expectations

* [ ] Tier 1
* [x] Tier 2
* [ ] Tier 3

CI expectations:

* **Required**:

  * Lint/format (SwiftLint or swift-format).
  * Unit tests via XCTest.
  * Interop tests hitting a local micronode/macronode.
* **Nice-to-have**:

  * Perf benchmarks and chaos tests as optional jobs.

---

### 10) Testing & QA

#### 10.1 Test types

* **Unit tests:**

  * Config resolution:

    * baseUrl sourcing from env vs explicit.
    * HTTPS enforcement logic.
    * Timeout defaults.
  * Error mapping:

    * Canonical Problem JSON → `RonProblem`.
    * Non-JSON body / 502 / timeouts → synthetic `transport` problems.
  * URL building:

    * Path and query encoding.
    * Header composition (automatic + user headers).

* **Integration tests:**

  * Hit a local node (micronode/macronode) via `svc-gateway`:

    * `GET /app/healthz`.
    * Echo endpoint (send JSON, get it back).
  * Validate shape of `AppResponse` and `RonProblem` matches shared schema examples.

* **E2E examples:**

  * SwiftPM example in `examples/` directory:

    * Reads env config.
    * Calls `/app/hello`.
    * Exits non-zero on failure (usable in CI as smoke test).

#### 10.2 CI requirements

* `swift format` / SwiftLint passes (no style violations).
* `swift test` completes and passes.
* Optional job:

  * Bring up a test node (Docker or local `cargo run`) and run Swift interop tests.

#### 10.3 Property/Chaos Tests

Tier 2 “SHOULD”:

* **Property-based testing** (e.g., SwiftCheck):

  * URL building:

    * For all valid combinations of path + query map, `buildUrl(path, query)` produces a valid URL and round-trips query parameters.
  * Error decoding:

    * For a wide range of valid/invalid Problem JSON variants, decoding yields a `RonProblem` that honors invariants:

      * Unknown fields ignored.
      * Required fields present yield `ok` mapping.
      * Missing `status` defaults to `0` when synthetic.

* **Chaos testing**:

  * Mock `URLSession` implementation that simulates:

    * Timeouts.
    * Connection resets.
    * Partial writes.
  * Assertions:

    * Timeouts and network errors produce `AppResponse` with `status = 0` and `problem.kind = "transport"`.
    * `retryable` flags are set appropriately.
    * No tasks leak or hang indefinitely.

Property + chaos tests can start as local scripts and graduate into CI once stable.

---

### 11) Interop & Schema (ron-proto, HTTP, Facets)

**RO:WHAT —**
Ensure that the Swift SDK adheres to the **same canonical schemas** for DTOs, errors, pagination, and facets as Rust, TS, and other SDKs, so that **same capability + same request ⇒ same behavior** regardless of language.

**RO:INVARIANTS —**

* DTO shapes, error envelopes (`RonProblem`), and pagination layouts in Swift match the canonical definitions in `SDK_SCHEMA_IDB.MD`.
* Swift treats facet manifests as **data**, not behavior, when we add helpers for generating them.
* No drift: Wire JSON field names remain exactly as specified; only in-memory representations may differ (e.g., numeric vs string).

#### 11.1 Canonical Sources

* `ron-proto` — message definitions for App Plane DTOs, storage/index payloads, auth/passport structures, etc.
* Gateway/OpenAPI spec — for `/app/*` request/response shapes, HTTP semantics, and Problem envelopes.
* `SDK_SCHEMA_IDB.MD` — cross-language mapping rules and facet manifest schema.

#### 11.2 SDK Requirements

As a **Tier 2** SDK:

* **Codegen preferred**:

  * If we add a viable Swift generator for `ron-proto`/OpenAPI, use it for DTOs and error types.
* **If handwritten**:

  * JSON field names and types **must** match canonical schema.
  * All canonical fields (`code`, `message`, `kind`, `correlation_id`, etc.) must exist in `RonProblem`.
  * Shared interop tests must validate that responses from a reference node can be deserialized and round-tripped.

Pagination:

* Implement canonical pagination envelope (e.g., `items`, `next_page_token` or similar) once that spec is finalized, mirroring TS behavior.

#### 11.3 Type Mapping & Drift Avoidance

For core types (per SDK_SCHEMA_IDB):

* `bool` → `Bool`
* `u32` → `UInt32` (wire = JSON number)
* `i64` → in-memory `Int64`, **wire = JSON string**
* `u64` → in-memory `UInt64` or wrapper, **wire = JSON string**

  * If a value is out of range for `UInt64`, decoding must fail explicitly; no silent truncation.
* `string` → `String` (UTF-8)
* `bytes` → `Data` (wire = base64 or raw, per API docs)
* `timestamp` → `Date` using ISO8601 string via `ISO8601DateFormatter`
* `id` (UUID-ish) → `String` or `UUID` wrapper, but JSON remains `string`.

Rules:

* **Wire format always follows canonical schema**:

  * Even if in-memory representation uses `Int64`/`UInt64`, JSON encodes `i64`/`u64` as strings where the schema says so.
* Unknown fields:

  * Ignored on decode to retain forward compatibility.
* Lossless representation:

  * Wherever possible, map values exactly; where not (e.g., numeric overflow), treat as decoding error and document.

**CI drift checks:**

* Maintain JSON fixture examples representing canonical DTOs, problems, and pagination envelopes.
* Interop test suite:

  * Start a local node (micronode/macronode).
  * Hit known endpoints (`/app/healthz`, echo, sample list API).
  * Compare decoded shapes against fixture expectations (e.g., key sets, type checks).

Example CI harness sketch (conceptual):

```sh
#!/bin/sh
set -e

# Start local test node (e.g., micronode in Docker or via cargo)
# NODE_PID=...
# trap "kill $NODE_PID" EXIT

# Run Swift interop tests
swift test --filter InteropTests
```

Goal: if the canonical schema changes without corresponding Swift updates, tests fail.

#### 11.4 Facet Interop

* Path semantics:

  ```text
  /facets/{facet_id}{route.path}
  ```

  e.g. `id = "docs"`, `path = "/hello"` → `/facets/docs/hello`.

* Swift `FacetClient` helpers (when added) must:

  * Generate correct paths using this convention.
  * Respect all security/limit/integrity hints (as far as they’re reflected in App Plane endpoints).

* Facet manifests:

  * When we add Swift helpers for generating TOML manifests, they must produce TOML that validates against Micronode’s facet loader and matches `SDK_SCHEMA_IDB` schema constraints (kinds, limits, security hints, etc.).

---

### 12) Future Extensions & Nice-to-Haves

(Each gated by specific RON-CORE milestones.)

* **Post-M3**:

  * Local mock App Plane server in Swift for offline dev (responds with canned JSON/Problem data).
* **Post-M4**:

  * `ron-swift-cli` that scaffolds minimal Swift apps (iOS + server) using the SDK.
* **Post-M5**:

  * Domain-specific helpers: e.g., a “taxi app” demo library built on top of the SDK.
* **Post-M3+**:

  * SwiftUI UI kit (`ron-app-sdk-swift-ui`) with health, metrics, and logs widgets.

---

### 13) Platform-Specific Gotchas

* **iOS / iPadOS / tvOS / watchOS**:

  * No real env vars at runtime; `RonConfig.fromEnvironment` must gracefully accept explicit baseUrl overrides from Info.plist or in-app configuration.
  * Cert pinning must be done via `URLSessionDelegate` in the consuming app; SDK will provide examples but no hard-coded pinning.
  * Background tasks must respect OS constraints (no long-running polling loops without throttling).

* **macOS (desktop) & server-side Swift**:

  * Env vars work normally; these environments are ideal for using `RON_SDK_GATEWAY_ADDR` and other env-based config.
  * Long-running processes should share `RonClient` instances for performance.

* **Linux**:

  * SDK must not depend on Apple-only frameworks; stick to Foundation and cross-platform libraries.

* **Amnesia mode**:

  * When talking to a micronode in amnesia mode, apps must treat node-side state as volatile.
  * Docs should explicitly warn against assuming long-lived sessions or cached server-side state; apps should rely on capabilities and idempotent operations.

---

