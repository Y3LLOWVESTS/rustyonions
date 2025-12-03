### SDK Tier & Profile

* **Tier:** **1**

  * Justification: Java is explicitly Tier-1 in the global SDK plan; Kotlin is Tier-2 but here it’s *bundled* into a single JVM SDK with Tier-1 expectations (tests, benchmarks, CI, property/chaos tests).
* **Usage profile:**

  * [x] Front-end (Android apps, desktop/UI clients)
  * [x] Backend/facet (Spring/Ktor services, Micronode/macronode app backends)
  * [x] Dual-use
  * [x] CLI/tools

This tier gates:

* §3.4 sandboxing expectations (must be sandbox-friendly)
* §4.3 idempotency helpers (must exist)
* §5.3 benchmark rigor (benchmarks in CI)
* §10.3 property/chaos tests (must exist)
* §9.3 CI/support levels (Tier-1 pipeline)

---

### 0) RO:WHAT / WHY / INTERACTS

**RO:WHAT —**
`ron-app-sdk-jvm` is the official JVM app SDK for RON-CORE’s **app plane**. It provides a shared core for Java and Kotlin that makes type-safe, secure calls to `/app/*` on a RON-CORE node from **Android, desktop, server, and CLI** JVM applications.

**RO:WHY —**

* **Why JVM:**

  * Java remains dominant in backend (Spring Boot, Micronaut, Quarkus), enterprise, and big-data workloads.
  * Kotlin is the modern default for Android and increasingly for server and multiplatform apps.
  * A single JVM SDK lets you target **Android front-ends**, desktop/front-end UIs, and JVM backends/facets with one consistent surface.
* **Primary use cases:**

  * **Front-end (Android / desktop)**:

    * Android apps using Kotlin coroutines or Java APIs to call RON-CORE (e.g., auth, profile, real-time updates).
    * Desktop/JavaFX tools that talk to a developer’s Micronode.
  * **Backend/facets:**

    * Spring Boot / Micronaut / Ktor services acting as app handlers, gateways, or internal tools calling RON’s app plane.
  * **CLI/tools:**

    * JVM-based CLIs (Picocli, plain `main`) to manage nodes, run migrations, inspect registries.
* **How it helps ship apps faster:**

  * Encapsulates base URL routing conventions (`/app/*`), headers, timeouts, and error envelopes so Java/Kotlin code can focus on **business logic** and Android UX.
  * Shared core types across Java/Kotlin reduce duplication; Android and server can share DTOs and request code.
  * Aligns with canonical schemas and security invariants from `SDK_SECURITY.MD` and `SDK_SCHEMA_IDB.MD`.

**RO:INTERACTS —**

Primarily via `svc-gateway` and omnigate, mirroring the TS SDK.

* `svc-gateway` — HTTPS `/app/*` entrypoint from JVM clients.
* `svc-edge` — future: edge-optimized entrypoints for mobile/latency-sensitive flows.
* `svc-admin` — optional admin/ops calls (likely as a separate `admin` surface in the SDK).
* `svc-registry` — for service discovery, app/facet metadata lookup.
* `svc-passport` / `ron-auth` — auth flows (login, token/macaroon issuance, refresh).
* `svc-storage` / `svc-index` — app-plane storage/index helpers (e.g., key/value, search) once exposed via app APIs.

---

### 1) Scope & Target Profiles

**1.1 Supported usage modes**

* [x] Front-end only (browser / mobile)

  * **Android (Kotlin/Java)**: first-class, coroutine-friendly APIs (suspend functions) plus Java callbacks/Futures.
  * **Desktop**: JavaFX/Swing or other UIs can reuse the same async core.
* [x] Backend / facets only

  * JVM services can act as app clients (not handlers) using the SDK to call RON-CORE.
* [x] Dual-use (browser + Node / server)

  * In JVM world: “dual-use” = **Android + server** plus desktop and CLI.
* [x] CLI / tools

  * Thin wrapper around the core `RonClient` to make CLIs trivial.

Explanation:

* Core module is pure JVM (no Android dependencies) so it works in **server, desktop, and CLI**.
* Android module depends on the core and adds **coroutines adapters**, lifecycle-aware helpers, and network config guidance.

**1.2 Supported node profiles**

* [ ] Micronode
* [ ] Macronode
* [x] Both (default)

Assumptions:

* App-plane APIs exposed via `svc-gateway` are **profile-agnostic** for most calls.
* Some admin routes may be macronode-only; SDK will document these and surface explicit exceptions when called against Micronode.
* SDK docs must warn that any persistence assumptions (e.g., long-term storage) need to respect **amnesia mode** semantics on Micronode (no local state guarantees).

**1.3 Feature scope (MVP vs future)**

**MVP:**

* [x] Auth (attach macaroon / token)
* [x] Basic request/response calls (GET/POST/PUT/PATCH/DELETE to `/app/*`)
* [ ] Streaming (SSE/WebSocket) subscribe

  * Reserved for post-MVP (mirroring TS roadmap where streaming is M2+/future).
* [x] Error handling (idiomatic for Java/Kotlin)
* [x] Minimal logging / tracing hooks

**Future:**

* [x] Schema-generated DTOs from `ron-proto` (see §11)

  * Java/Kotlin DTOs auto-generated from canonical schema; included once codegen tooling is in place.
* [x] Auto-reconnect & backoff (especially for streaming/websocket)
* [x] Built-in metrics hooks (timers, counters)
* [x] Facet lifecycle helpers (register, heartbeat, etc.)
* [x] Higher-level convenience APIs (domain-specific), e.g.:

  * `AuthClient`, `StorageClient`, `IndexClient` wrappers.
  * Android-ready helpers for offline caching + sync.

---

### 2) API Design & DX Basics

**2.1 Primary abstraction**

* **Core entrypoints:**

  * `RonClient` (Java-first) — main client for JVM.
  * `Ron` (Kotlin wrapper) — idiomatic Kotlin facade with `suspend` functions and `Flow` streams.
* Builders:

  * `RonClient.Builder` for JVM (`RonClient.builder().baseUrl("...").overallTimeout(...).build()`).
  * Kotlin DSL: `Ron { baseUrl("...") ; overallTimeoutMs(10_000) }`.

Minimal “hello world” example (Java; copy-paste runnable):

```java
// HelloWorld.java
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonProblem;

public class HelloWorld {
    public static void main(String[] args) {
        // 1) Create client (baseUrl defaults from env if not provided).
        RonClient ron = RonClient.builder()
                .baseUrl(System.getenv("RON_SDK_GATEWAY_ADDR"))
                .build();

        // 2) Make one simple call to /app/hello
        AppResponse<String> res = ron.get("/hello");

        // 3) Handle result in an idiomatic Java way.
        if (res.isOk()) {
            System.out.println("Server said: " + res.getData());
        } else {
            RonProblem problem = res.getProblem();
            System.err.println("RON error [" + problem.getCode() + "]: " + problem.getDetail());
        }

        ron.close();
    }
}
```

Minimal “hello world” example (Kotlin + coroutines):

```kotlin
// HelloWorld.kt
import dev.roncore.sdk.kotlin.Ron
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val ron = Ron {
        baseUrl(System.getenv("RON_SDK_GATEWAY_ADDR") ?: "https://localhost:5304")
    }

    val res = ron.get<String>("/hello")

    if (res.ok) {
        println("Server said: ${res.data}")
    } else {
        println("RON error [${res.problem?.code}]: ${res.problem?.detail}")
    }

    ron.close()
}
```

Goal: matches TS `Ron` semantics (`get/post/...`) but feels Java/Kotlin-native.

**2.2 Core operations**

* `RonClient.builder()` / `Ron { ... }` — construct clients with config.
* `request(AppRequest)` — low-level primitive mirroring TS `request`.
* `get(path, query)`
* `post(path, body, query)`
* `put(path, body, query)`
* `patch(path, body, query)`
* `delete(path, query)`
* Typed high-level helpers (future):

  * `getResource<T>(resourcePath)`
  * `putResource<T>(resourcePath, payload)`
* Eventing/streaming (future):

  * `subscribe(topic, handler)` — via WebSockets/SSE; Kotlin `Flow` wrapper.
* `facet(name)` — returns a `FacetClient` that prefixes paths (like TS `ron.facet("auth")`).
* `close()` — closes underlying HTTP/WebSocket resources, cleans up thread pools/executors.

**2.3 Error model**

* Representation:

  * Java: `AppResponse<T>` holds either `data` or `RonProblem`; network/config errors may throw `RonClientException` (unchecked) or `IOException`, depending on call variant.
  * Kotlin: `AppResponse<T>` plus extension helpers (`getOrThrow()`, `getOrNull()`) and convenience functions that throw typed exceptions for error cases if desired.
* RON-CORE error mapping:

  * HTTP/Problem JSON is parsed into a canonical `RonProblem` DTO, consistent with `SDK_SCHEMA_IDB.MD` and TS.
  * Distinguish:

    * **Network errors** → `RonTransportException` (e.g., timeouts, DNS failure).
    * **Auth/policy errors** → `RonAuthException` / `RonPolicyException` (401/403 with problem details).
    * **App errors** → result with `AppResponse.problem != null` mapped from server problem envelope.
* Idiomatic handling patterns:

  * Java: either pattern-match on exception type or inspect `AppResponse`.
  * Kotlin: `try { ron.get<T>("/foo").getOrThrow() } catch (e: RonException) { ... }`.

**2.4 Configuration**

* Configurable options:

  * `baseUrl` (derived from `RON_SDK_GATEWAY_ADDR` env var if not set).
  * Timeouts: overall, connect, read, write, mapping to canonical env vars:

    * `RON_SDK_OVERALL_TIMEOUT_MS`
    * `RON_SDK_CONNECT_TIMEOUT_MS`
    * `RON_SDK_READ_TIMEOUT_MS`
    * `RON_SDK_WRITE_TIMEOUT_MS`
  * TLS:

    * TLS **enabled by default**; explicit flag required for insecure HTTP (mirrors security checklist).
    * Optional: custom trust store / cert pinning (especially for Android).
  * Auth:

    * Static token/macaroon string.
    * Token provider callback `TokenProvider { String currentToken(); }`.
  * Retries/backoff:

    * Configurable max retries + backoff strategy; off by default for non-idempotent operations.
  * Logging/tracing hooks:

    * Interfaces for pluggable loggers (SLF4J/Android Log) and tracing (OpenTelemetry).

* Config loading rules:

  * Server/CLI: environment variables provide defaults but code options always override them (mirrors TS behavior where env defaults are Node-only).
  * Android: **no env usage**; must be passed via config or Android resource/config API.
  * Optional config file: `ron-sdk.properties` for JVM backends (future).

**2.5 Language-idiomatic patterns**

* Async model:

  * Java:

    * Synchronous methods (`AppResponse<T> get(path)`), plus async variants returning `CompletableFuture<AppResponse<T>>`.
  * Kotlin:

    * All key methods exposed as `suspend fun` and `Flow` for streaming, built on the same underlying engine.

* Naming conventions:

  * Packages: `dev.roncore.sdk` (core), `dev.roncore.sdk.kotlin`, `dev.roncore.sdk.android`.
  * Methods: `get`, `post`, `request`, `facet`, `close` (short, HTTP-like, same as TS).

* Integration patterns:

  * Spring Boot: configure `RonClient` as a singleton `@Bean`.
  * Ktor server: inject `Ron` into routes via DI or service locator.
  * Android: hold a `Ron` instance in a `ViewModel` or even Hilt/DI singleton.

---

### 3) Security & Capabilities (SEC)

**3.1 Auth flow**

* Accept credentials via:

  * Macaroon/token string (`setToken(String)`),
  * Token provider callback (`TokenProvider`/Kotlin lambda) so rotation is centralized.
* Tokens are stored **only in memory**, in client instance fields, never logged, never written to disk by the SDK.
* SDK ensures:

  * Auth headers use canonical names/fields consistent across SDKs (`Authorization`, capability header) as per schema/security docs.

**3.2 Capability boundaries**

* Design encourages **scoped tokens**:

  * Example docs use read/write tokens for specific app buckets instead of wildcard admin tokens.
  * Android examples explicitly show short-lived tokens from login flow, not long-term secrets embedded in app.

* Helpers (future):

  * `CapRequest`/`CapBuilder` helpers for asking RON-CORE to mint scoped caps with TTLs.
  * Token provider integration for rotation (e.g., refreshing from `svc-passport`).

**3.3 Secure defaults**

Checklist alignment with `SDK_SECURITY.MD`:

* [x] TLS on by default; insecure HTTP requires explicit `allowInsecureHttp(true)` or similar.
* [x] Credentials never logged; log scrubbing for headers/body.
* [x] Reasonable timeouts; no infinite hangs.
* [x] Debug logging explicitly opt-in; even then, tokens are redacted.
* [x] PQ-ready toggle (future): placeholder config for PQ/hybrid ciphers once JVM TLS stack and RON profile stabilize.

**3.4 Sandboxing for Facets/UI**

* No global client singletons; `RonClient`/`Ron` are **ordinary objects** that can be scoped per sandbox (thread, coroutine, Android component).

* Must be safe for:

  * Android WorkManager jobs.
  * Kotlin coroutines on different dispatchers.
  * JVM isolates/worker threads.

* Docs will include examples:

  * “Sandboxed worker” pattern: create a client in a worker thread/coroutine, pass only safe payloads back, never share tokens with UI.

* Any facet manifest helpers must avoid global mutable state and be pure builders (data-only), in line with schema invariants.

---

### 4) Reliability & Resilience (RES)

**4.1 Connection behavior**

* HTTP client:

  * Likely OkHttp or similar engine that works on JVM + Android; actual HTTP engine is hidden behind an internal `HttpTransport` interface.

* SDK behavior:

  * DNS failures, timeouts, and transport errors map to `RonTransportException`.
  * No silent retries by default; idempotent operations (GET, some PUT) may opt-in to limited retries.
  * Configurable connect/read/write/overall timeouts using shared env var names.

* PQ-hybrid handshake:

  * Reserved configuration hooks for future PQ ciphers; blocked on RON-CORE transport profile.

**4.2 Streaming resilience**

* Once streaming is added:

  * WebSocket/SSE layer with automatic reconnect and exponential backoff, subject to maximum retry limits.
  * Consumers will see either a `Flow` (Kotlin) or callback API that may emit duplicate messages; semantics documented as **at-least-once** unless underlying protocol provides stronger guarantees.
  * Backpressure: streaming APIs must not leak threads or unbounded queues.

**4.3 Backoff & rate limiting**

* Built-in helpers:

  * Retry/backoff policies (e.g., exponential with jitter) for idempotent requests.
  * Client-side rate limiter option (basic token bucket) for high-throughput backends.

* Anti-patterns to avoid:

  * “One client per request” — docs will emphasize a shared `RonClient` per process or per key scope.

* Idempotency helpers (Tier-1 requirement):

  * Support for idempotency keys:

    * `IdempotencyKeyProvider` that generates UUIDs.
    * Optional `withIdempotencyKey(String)` on request builder to set header/field.

---

### 5) Performance (PERF)

**5.1 Expected usage profile**

* Typical server rates:

  * 10–1,000 RPS per process; high-throughput services may run multiple instances.
* Typical Android rates:

  * Low RPS; focus on latency, battery, and network efficiency over raw throughput.
* Payload sizes:

  * Small JSON requests/responses (<100 KB) for most operations; streaming or storage operations may move larger payloads, but those are considered advanced.

**5.2 Implementation choices**

* HTTP client:

  * OkHttp (or equivalent) to support connections on both JVM servers and Android.

* Serialization:

  * JSON as the baseline; DTOs match canonical schema from `SDK_SCHEMA_IDB.MD`.
  * Protobuf/CBOR (future) once app-plane supports alternative encodings.

* WebSocket/SSE:

  * OkHttp WebSockets or Ktor client channels (for Kotlin) when streaming is implemented.

**5.3 Benchmarks**

* Include simple benchmarks in CI:

  * Single call latency vs raw HTTP client (OkHttp).
  * Sustained RPS for simple GET/POST on dev laptop and CI runner.
  * Streaming overhead benchmarks once streaming lands.

* Target:

  * SDK overhead <5% vs raw HTTP for simple JSON calls, mirroring expectations from TS SDK benchmarks guidance.

* Bench harnesses:

  * JMH for JVM microbenchmarks.
  * Kotlin benchmark suites for coroutine variants.

---

### 6) Economics (ECON)

**6.1 Dev productivity**

* Removes boilerplate:

  * No manual URL concatenation for `/app/*`.
  * Consistent error envelope handling instead of ad-hoc `try/catch` + JSON parsing.
  * Built-in timeouts and env config prevent copy-pasted configuration per project.

* Minimal LOC to:

  * Call an API: ~5–10 lines in Java, fewer in Kotlin DSL.
  * Subscribe (future): one `Flow` or callback binding.
  * Handle auth: passing a token provider instead of scattering headers.

**6.2 Operational cost**

* Guidance:

  * Use a shared `RonClient` per process (server) or per DI scope (Android), not per request.
  * Use connection pooling from underlying HTTP engine.
* Memory/CPU:

  * Keep DTOs lightweight; avoid heavy reflection frameworks.
  * No hidden background threads beyond what’s required for HTTP/streaming; document them.

---

### 7) Developer Experience (DX) — Onboarding & Familiarity

**7.1 “No manual” quickstart**

Three snippets required in README:

1. **Hello world call (Java)** — as shown in §2.1.
2. **Hello world call (Kotlin + coroutines)** — as shown in §2.1.
3. **Facet-style worker loop (backend)**:

   ```kotlin
   // Simple worker loop polling a RON app endpoint
   suspend fun runWorker(ron: Ron) {
       while (true) {
           val res = ron.get<List<JobDto>>("/jobs/pending")
           if (res.ok) {
               res.data?.forEach { job ->
                   // process job...
               }
           } else {
               // log structured problem, maybe backoff
           }
           kotlinx.coroutines.delay(1_000)
       }
   }
   ```

Each snippet fits within 10–20 lines and relies on env defaults + one `Ron` construction, mirroring TS SDK quickstart philosophy.

**7.2 Framework adapters / helpers**

Planned adapters:

* [x] Web framework middleware / helpers

  * Spring Boot: `RonClient` auto-config (starter) and `@ConfigurationProperties` for env-driven config.
  * Micronaut/Quarkus: simple Bean producers and injection examples.
  * Ktor: feature/plugin to inject `Ron` into route handlers.
* [x] CLI helper(s)

  * Thin wrapper for building CLI tools with Picocli or plain `main` using env defaults.
* [x] Admin GUI hooks/widgets (future)

  * Backend SDK pieces used as data source for JavaFX/Compose Desktop admin tooling.
* [x] Logging/tracing integration

  * Integration snippets for SLF4J and OpenTelemetry so RON calls carry spans/traces.

**7.3 Language-idiomatic docs**

* All code examples use modern patterns:

  * Java 17+ style (var sparingly, `try-with-resources`).
  * Kotlin 1.9+ idioms (coroutines, `data class`, extension functions).

* Error and return types:

  * Feel like typical Java/Kotlin HTTP clients and coroutines wrappers, not Rust-isms.

* Docs explicitly link to `SDK_SECURITY.MD` and `SDK_SCHEMA_IDB.MD` as “deeper rationale” for invariants.

---

### 8) Extensibility & Plugins

**8.1 Extension points**

* Request/response middleware/interceptors:

  * Java interface: `RonInterceptor { AppRequest before(AppRequest); AppResponse<?> after(AppResponse<?>); }`.
  * Kotlin DSL: `interceptor { req, next -> /* modify req or inspect res */ }`.

* Event hooks:

  * `onConnect`, `onDisconnect`, `onError`, `onReconnect` callbacks at client level.

* Pluggable transports (future):

  * Select HTTP engine (OkHttp vs Ktor client), without changing public API.

MVP:

* [x] Basic request middleware / callback hooks (logging, headers, tracing).

Future:

* [x] Fully pluggable pipeline (stack of interceptors like OkHttp).
* [x] Custom transport adapters (alternate HTTP engines, QUIC, etc.).

**8.2 UI widgets / components (for GUI-friendly SDKs)**

* Core SDK stays headless; UI helpers live in separate artifacts:

  * `ron-app-sdk-jvm-ui` (future) for JavaFX/Compose Desktop.
  * Android-specific samples (Jetpack Compose screens) likely live in sample apps, not core SDK.

Planned primitives:

* `NodeHealthViewModel` that uses `Ron` to poll health/metrics.
* `EventLogViewModel` that subscribes to event streams (future).
* Goal: drop-in viewmodels so Android/desktop UI can quickly wire live node views.

**8.3 Plugin model (future)**

* Third-party extensions register as interceptors or helper libraries, **not** dynamic class loading in the core SDK.
* Keep core light; heavy functionality (e.g., advanced retry plugins, custom metrics sinks) live in separate modules.
* Avoid plugin loading mechanisms that could introduce security issues (no arbitrary reflection-based plugin loading from untrusted sources).

---

### 9) Governance & Versioning (GOV)

**9.1 Version scheme**

* Maven coordinates: `dev.roncore:ron-app-sdk-jvm`.

* Versioning:

  * `0.x` tracks RON-CORE protocol v0.x.
  * `1.x` once protocol stabilizes.

* Breaking changes:

  * Require major version bump (or at least minor with clear migration notes in pre-1.0).
  * Deprecations: warn for at least 2 minors before removal, especially for widely used methods.

**9.2 Compatibility matrix**

| SDK version | Protocol version | Min node version | Notes                         |
| ----------- | ---------------- | ---------------- | ----------------------------- |
| 0.1.x       | v0.1             | X.Y.Z            | MVP app-plane client          |
| 0.2.x       | v0.1             | X.Y.Z            | Adds codegen DTOs, interop CI |
| 0.3.x       | v0.2             | A.B.C            | Streaming + advanced helpers  |

**9.3 Support expectations**

* [x] Tier 1 (core, always up to date)

  * CI includes:

    * Lint/format, unit tests, integration tests (micronode), schema-drift check, basic benchmarks, property tests for auth/error handling, and minimal chaos testing.

---

### 10) Testing & QA

**10.1 Test types**

* Unit tests:

  * Config resolution (env + builder).
  * URL building (`/app/*` semantics).
  * Error mapping from canonical problem envelope.
  * Interceptor behavior, token provider behavior.

* Integration tests:

  * Use a local test Micronode/Macronode + `svc-gateway`.
  * End-to-end calls: `GET /app/ping`, simple JSON echo.
  * Verify DTO, error, and pagination shapes match schema (`SDK_SCHEMA_IDB.MD`).

* E2E samples:

  * Sample Spring Boot service using SDK in CI.
  * Sample Android instrumentation tests (optional, might be separate pipeline).

**10.2 CI requirements**

* Lint/format:

  * Use ktlint/Detekt for Kotlin; Checkstyle/SpotBugs for Java (minimal).
* Unit tests:

  * JUnit 5/Kotest.
* Integration tests:

  * Flagged tests that spin up Micronode via Testcontainers or docker compose.
* Build/distribution:

  * JARs published to Maven Central (or equivalent), signed and versioned.

**10.3 Property/Chaos Tests**

* Property-based tests:

  * Use Kotlin/Kotest property or jqwik to fuzz error payloads, headers, and ensure no secret leaks, no crashes on malformed responses.

* Chaos:

  * Use Toxiproxy or similar to simulate latency spikes, drops, and partial writes; ensure timeouts and backoff operate as designed.

* Tier-1 requirement:

  * At least one property test suite around auth/errors.
  * At least one chaos scenario in CI or a script that’s easy to run locally.

---

### 11) Interop & Schema (ron-proto, HTTP, Facets)

**RO:WHAT —**
As a Tier-1 SDK, `ron-app-sdk-jvm` must share a **single structural truth** with Rust, TS, Go, etc., for DTOs, errors, pagination, and facets.

**RO:INVARIANTS —**

* All DTOs exposed are derived from `ron-proto` + HTTP/OpenAPI, not hand-invented types.
* Error and pagination envelopes match the canonical envelopes (`RonProblem`, `Page<T>`).
* Facet manifests (TOML) generated by JVM builders must match shared facet schema (kind, routes, security hints, limits, SRI).

#### 11.1 Canonical Sources

* `ron-proto` — canonical DTOs for app/storage/index/auth.
* HTTP/OpenAPI spec for gateway/omnigate — path/method/body, error envelope.
* `SDK_SCHEMA_IDB.MD` — cross-language type mapping and facet manifest schema.

#### 11.2 SDK Requirements (JVM)

* As Tier-1:

  * **MUST** use codegen/bindings for DTOs and error envelopes where tooling allows.
  * **MUST** implement canonical error/pagination envelopes.
  * **MUST** treat facet manifests as data (TOML builders that align with schema):

    ```kotlin
    FacetManifestBuilder("docs") {
        kind = "static"
        route("GET", "/hello") {
            file = "dist/hello.html"
        }
    }
    ```

  producing TOML consistent with shared rules.

* Interop tests:

  * Shared “interop test” suite across SDKs hits the same endpoints and asserts the same shapes (JVM variant must pass that suite).

#### 11.3 Type Mapping & Drift Avoidance

* Mapping decisions:

  * `u64` → `long` (`Long`) in JVM, JSON string if required by wire contract (documented).
  * timestamps → `Instant` (Java) / `Instant` (`kotlinx-datetime`), encoded as ISO8601 strings.
  * binary → `byte[]` or `ByteString`.

* CI schema drift:

  * Regenerate Java/Kotlin DTOs from schema and fail build if diff is non-empty.

#### 11.4 Facet Interop

* JVM facet helpers (future):

  * Builders that generate TOML for Micronode facets with:

    * `facet.id`, `kind`, `routes`, `limits`, `security`, `route.integrity`.

* Path semantics are fixed:

  * `/facets/{facet_id}{route.path}` (same as other SDKs).

---

### 12) Future Extensions & Nice-to-Haves

* Post-M3: local mock/testing server in JVM for offline dev (embedded H2 + stub gateway).
* Post-M4: `ron-cli-jvm` generator that scaffolds Spring/Ktor/Android examples.
* Post-M5: domain-specific helpers (`TaxiApiClient`, `ChatApiClient`) for dogfooding apps.
* Post-M3: more powerful UI widgets/controllers for desktop admin dashboards or Android monitoring tools.

---

### 13) Platform-Specific Gotchas

* Android:

  * Must not block main thread; all calls from UI should be via coroutines or async.
  * Network security config: handle TLS, certificate pinning; explicit docs for dev vs prod.
  * No environment variables; configuration via DI, resource files, or build configs.

* JVM Server:

  * Thread pool sizing: avoid one thread per request; rely on HTTP client pooling and async APIs.
  * Ensure amnesia mode warnings for persistence assumptions.

* Desktop:

  * Same guidance as Android for not blocking UI thread.
  * Clean shutdown required (close client on application exit).

This filled IDB template now nails down the **shape, scope, and invariants** for `ron-app-sdk-jvm` as a unified Java+Kotlin, Android+server, Tier-1 SDK that mirrors the TS SDK’s philosophy while staying idiomatic to the JVM world.
