

````md
---

# SDK SECURITY CHECKLIST — `ron-app-sdk-jvm` (Java/Kotlin App SDK)

> Applies to: `ron-app-sdk-jvm` (Java + Kotlin, single JVM artifact).  
> Scope: **backend** (Spring Boot, Ktor, Quarkus, Micronaut, CLI/tools) and **frontend** (Android, desktop UI).

---

## RO:WHAT

Security checklist for the **JVM App SDK** that lets Java and Kotlin apps talk to RON-CORE via `svc-gateway`/omnigate.

Defines the **minimum bar** for “ship” and the **stretch items** for “God-tier” security on:

* JVM servers (Spring, Ktor, etc.),
* Android apps (Kotlin/Java),
* Desktop JVM apps (Swing/JavaFX/Kotlin Compose, etc.).

---

## RO:WHY

For a large chunk of the world, **Java/Kotlin is the default backend stack** and **Kotlin is a first-class mobile stack** (Android). The JVM SDK will sit in:

* Backend services (Spring Boot/Ktor/Quarkus),
* Android apps calling Micronodes/Macronodes,
* Desktop tools and CLIs.

If the JVM SDK is sloppy with:

* capabilities (macaroons/tokens),
* error messages (leaking internals into logs/logcat),
* logging (debug logging secrets),
* timeouts & retries (unbounded OkHttp calls, coroutines that never time out),

…then all hardening in Micronode / omnigate is bypassed.

This checklist ensures `ron-app-sdk-jvm`:

* doesn’t leak secrets (even in logcat),
* doesn’t weaken auth (no ambient global caps),
* behaves predictably under failure (timeouts, backoff),
* is ready for PQ/audit/formal work when core RON-CORE moves there.

---

## RO:INVARIANTS

Across **all** SDKs (JVM included):

* No SDK logs secrets (caps, raw headers, private keys).
* No SDK “helpfully” retries in ways that violate idempotency.
* All SDK network calls are **bounded** (timeouts, backoff, cap on retries).
* Capabilities are **least-privilege**, **short-lived**, and **rotatable**.
* Error surfaces are **structured**, not raw stack traces with secrets.
* Debug/trace modes are **explicitly opt-in** and still scrub secrets.

For `ron-app-sdk-jvm` specifically:

* No tokens appear in Java/Kotlin exception messages or stack traces by default.
* No secrets are ever logged via `java.util.logging`, SLF4J, Logback, Log4j, or Android’s `Log.*`.
* Defaults are safe for **both**:
  * server environments (full control, but high blast radius),
  * Android apps (hostile environments, device loss, logcat scraping).

---

## 0) TL;DR CHECKLIST

Before calling `ron-app-sdk-jvm` “beta”:

* [ ] **Auth & caps**:  
  * safe in-memory handling, no logging, rotation hooks, least-privilege scopes.
* [ ] **Transport**:  
  * HTTPS/TLS only by default, sane timeouts, bounded retries, explicit backoff.
* [ ] **Error handling**:  
  * structured errors, no secrets in messages or stack traces, correlation IDs.
* [ ] **Logging**:  
  * header/body scrubbing, no tokens in logs/logcat, correlation IDs + error codes only.
* [ ] **DoS safeguards**:  
  * response size caps where applicable, concurrency limits, guarded streaming (SSE/WS).
* [ ] **Mobile/UI variant**:  
  * no long-term cap storage by default, Android-safe storage guidance, XSS-like issues for WebViews documented.
* [ ] **Tests**:  
  * JUnit/Kotest property tests for idempotency, unit/integration tests for failure paths, basic fuzzing for error parsing.
* [ ] **Docs**:  
  * show secure usage by default (env-based caps on server, Keystore/ephemeral caps on Android, no hardcoded tokens).
* [ ] **Hooks for future**:  
  * config fields/flags reserved for PQ, audit IDs, signed manifests/caps.

---

## 1) Capabilities & Auth (ALL JVM SURFACES)

**Goal:** Caps/macaroons are treated like cryptographic plutonium.

### Design

* [ ] **No ambient caps**:

  * No global static `RonClient` or static token fields.
  * Caps are provided **per client instance** via builder/config object.

* [ ] **Least privilege by default**:

  * Code samples use **scoped capabilities** (e.g., `bucket:todo:read/write`, tenant-scoped) instead of `*`.
  * Android examples use **per-user** short-scope caps, not node-root admin caps.

* [ ] **Short-lived caps**:

  * SDK config supports expiry metadata (e.g., `expiresAtInstant`).
  * Docs strongly encourage **minute-level TTLs** for user/session caps.
  * Examples show flows where caps are issued by a backend and rotated often, not “forever” tokens baked into apps.

* [ ] **Rotation**:

  * Provide a `CapabilityProvider` / `TokenProvider` abstraction:

    * Java: `Supplier<String>` or functional interface.
    * Kotlin: suspendable provider (`suspend () -> String` or Flow-based for advanced use).

  * Clients can be constructed with a provider so a stale token is automatically replaced without restarting the process/app.

### Handling & Storage

* [ ] **No caps in logs** (ever):

  * Internal logging uses redactors; any `Authorization`/`Cookie` header is replaced with `"***REDACTED***"`.

* [ ] **No caps in error messages**:

  * Exception messages never embed full request headers or bodies containing tokens.

* [ ] **No caps in panic/exception messages**:

  * Even in unexpected failures (serialization, HTTP client exceptions), the SDK wraps raw exceptions so tokens do not appear in `toString()` output.

* [ ] **No caps in URLs** (query strings):

  * `Authorization` is **always** via headers.
  * Any attempt to set caps via query string options is rejected at build-time or runtime with a clear error.

* [ ] **No persistent storage of caps in plaintext from SDK internals**:

  * JVM server:
    * SDK never writes caps to disk.
    * Config encourages `RON_SDK_*` env vars or secret stores; no `.properties` or `.yml` examples where tokens are in source.
  * Android:
    * SDK never writes caps to `SharedPreferences` on its own.
    * If host app chooses to persist, docs recommend Android Keystore + `EncryptedSharedPreferences` and aggressive rotation.
    * SDK assumes caps are held in memory (ViewModel/repository layer); loss of process clears them.

### API Shape

* [ ] Provide helpers so user code rarely touches raw caps:

  * Example:

    ```kotlin
    val client = RonClient.builder()
      .baseUrl("https://my-node.example.com")
      .capabilityProvider { currentCapability() } // app-defined provider
      .build()
    ```

  * Java-friendly builder variant with method references / lambdas.

* [ ] Consider a **“session” abstraction**:

  * `RonSession` for per-user/per-tenant context, with:
    * `close()` method that zeroes in-memory token material (best-effort).
    * Android: session bound to lifecycle (e.g., tied to an Activity/Fragment ViewModel).

---

## 2) Transport & Timeouts

**Goal:** Every JVM SDK call is bounded in time and resources.

* [ ] **TLS/HTTPS required**:

  * Default `baseUrl` parsing rejects `http://` unless:
    * An explicit `insecureHttp` / `allowInsecureHttpForDev` flag is set, or
    * A test hook is used (annotated clearly as non-prod).
  * Android examples **never** use plain HTTP.

* [ ] **Timeouts mandatory**:

  * Each call made through the underlying HTTP client (e.g., OkHttp/Ktor client) has:
    * connect timeout,
    * read timeout,
    * overall call timeout (or per-request budget with `withTimeout` in coroutines).
  * Defaults wired from shared env vars (`RON_SDK_OVERALL_TIMEOUT_MS`, etc.) where appropriate, with sane fallbacks.

* [ ] **Retries bounded**:

  * Retries are **off** by default for non-idempotent operations (POST/DELETE).
  * For idempotent ops (GET, HEAD, some PUT with idempotency keys):
    * At most 3 retries, with exponential backoff and jitter.
    * Rely on exception types / HTTP status (e.g., 502/503/504) to decide retry vs fail.
  * No infinite loops or unbounded recursion; all retry loops check a global attempt counter.

* [ ] **Idempotency support**:

  * Write operations support optional **idempotency keys**:

    * Server: builder can auto-generate UUIDv4 (Java `UUID.randomUUID()` / Kotlin `UUID.randomUUID()`).
    * Android: expose same helpers; document that idempotency is per client instance, not global.

  * Keys go into a standard header (e.g., `x-idempotency-key`), not into the body.

---

## 3) Errors & Logging

**Goal:** Helpful to developers, useless to attackers.

### Errors

* [ ] **Structured error type**:

  * Single JVM error representation, e.g. `RonException` with:

    ```kotlin
    class RonException(
      val kind: String,          // "transport" | "auth" | "policy" | "app"
      val code: String,          // stable, machine-readable
      override val message: String, // user-friendly, scrubbed
      val correlationId: String?,
      val retryable: Boolean,
      val details: Map<String, Any?> = emptyMap(),
      cause: Throwable? = null
    ) : RuntimeException(message, cause)
    ```

  * For Java users: methods to access fields (`getKind()`, `getCode()`, etc.).

* [ ] **Message hygiene**:

  * Human-readable `message` is always generic (e.g., “Request timed out”, “Unauthorized”, “Server returned problem: <code>”).
  * Raw server stack traces never directly appear in the message.

* [ ] **Separation of concerns**:

  * Distinguish:
    * **transport errors** (DNS, TLS, timeouts),
    * **auth/capability errors** (401/403),
    * **policy/quota** (429/limited),
    * **app problems** (4xx/5xx with canonical problem envelope).
  * `retryable` flag indicates whether SDK thinks an automatic retry is safe (for user orchestration).

* [ ] **No stack-trace leakage by default**:

  * For backend:
    * Stack traces stay in logs when explicitly logged by the app, but user-visible messages exposed through API layers are cleaned.
  * For Android:
    * `RonException.toString()` does not dump nested stack traces or raw HTTP bodies.
    * Crash reporting integrations can access full details via explicit APIs, not by default.

### Logging

* [ ] **Headers scrubbing**:

  * Built-in logging interceptors (if provided) remove or mask:
    * `Authorization`,
    * `Cookie`,
    * any header that matches token-like patterns.

* [ ] **Body scrubbing**:

  * Default logs show **metadata only** (method, URL path, status, duration).
  * Optional debug mode exposes truncated bodies, but:
    * runs through a redactor that masks obvious secrets (JWT-like strings, long hex, etc),
    * is clearly labeled non-prod.

* [ ] **Correlation IDs**:

  * Each request injects or preserves a `x-request-id`/`x-correlation-id`.
  * Logging hooks log these IDs, not payloads, so operators can cross-reference node logs without leaking data.

---

## 4) DoS & Resource Use

**Goal:** JVM SDK is a good cluster and device citizen; it doesn’t accidentally DDoS RON-CORE or freeze UIs.

* [ ] **Response size caps**:

  * For list/paginated endpoints, the SDK:
    * honors `limit`/`pageSize` parameters,
    * encourages **small pages** in examples,
    * exposes streaming or paging helpers rather than “read all into memory” defaults.

* [ ] **Streaming APIs guarded**:

  * SSE/WebSocket support (when added) requires explicit methods like `subscribeToTopic(...)`.
  * No hidden background subscriptions created by default.
  * API surface allows caller to:
    * cancel subscriptions,
    * set per-subscription backpressure/timeouts.

* [ ] **Concurrency settings**:

  * Underlying HTTP client is shared and connection-pooled per baseUrl.
  * SDK exposes config for **max concurrent requests** per client and global:
    * For servers: recommended defaults tuned for typical thread pools.
    * For Android: recommended lower concurrency to avoid saturating resources.

* [ ] **Backoff on error**:

  * For 429 and known gateway “slow down” responses:
    * parse `Retry-After` if present,
    * apply backoff and bubble a structured error.

---

## 5) Mobile / UI-Specific Checklist (Android & Desktop)

**Goal:** Running in user devices without betraying them.

* [ ] **No raw caps in persistent storage by default**:

  * SDK keeps caps **only in memory** by default.
  * Docs recommend:
    * Android: use Android Keystore + `EncryptedSharedPreferences` **if** the app must persist tokens.
    * Desktop: use OS secret stores (Keychain, Credential Manager) or encrypted files; never store raw caps in `.properties` visible in repo.

* [ ] **Secure network stack**:

  * Android examples pin TLS (e.g., certificate pinning) or at least enforce system trust store.
  * WebView integrations (if any) explicitly document CORS/auth flows and do **not** expose caps via JS bridges.

* [ ] **CSRF-analog awareness (if used with cookies)**:

  * If apps use cookie-based session with nodes:
    * Document safe patterns (same-site cookies, CSRF tokens) for backends.
    * Strongly discourage using browser-like cookie flows in Android unless the full threat model is understood.

* [ ] **Injection/XSS-style risks in UI**:

  * DTOs returned by SDK should be treated as untrusted content before showing in WebViews or rich text views.
  * Docs call out the need to escape/sanitize content before rendering.

* [ ] **SRI hooks for static bundles (desktop tooling)**:

  * If JVM tools help manage facet static assets, keep SRI fields in manifest builders (aligning with `SDK_SCHEMA_IDB.MD`).

---

## 6) Backend-Specific Checklist (JVM Servers & CLI)

**Goal:** Hard to shoot yourself in the foot in long-lived JVM processes.

* [ ] **Safe config patterns**:

  * Guide developers to configure:
    * `RON_SDK_GATEWAY_ADDR`,
    * `RON_SDK_OVERALL_TIMEOUT_MS`,
    * `RON_SDK_CONNECT_TIMEOUT_MS`,
    * `RON_SDK_READ_TIMEOUT_MS`,
    * `RON_SDK_WRITE_TIMEOUT_MS`
  * via environment variables or secret providers, not hardcoded constants.

* [ ] **Framework integrations**:

  * Provide or document:
    * Spring Boot auto-config snippet for wiring a single `RonClient` bean per app.
    * Ktor plugin / Micronaut bean configuration that respects timeouts and TLS.
  * Enforce:
    * one client per baseUrl per process (shared connection pool),
    * no “new client per request” anti-pattern.

* [ ] **HTTP client invariants**:

  * Use a modern async-friendly client (e.g., OkHttp, Ktor client) configured with:
    * enforced timeouts,
    * connection pooling limits,
    * cancellation support via coroutines or `CompletableFuture`/`Executor`.

* [ ] **Rust-like invariants carried over**:

  * No panics in normal flow; in JVM this means:
    * no `Error` throws for expected conditions,
    * checked/unchecked exceptions are controlled and documented.
  * Error types integrate with:
    * logging frameworks,
    * metrics (counters/histograms for failures vs successes).

---

## 7) Testing & Verification

**Goal:** We don’t just hope it’s safe; we prove it where we can.

* [ ] **Unit tests** (JUnit 5 / Kotest):

  * Construct requests and ensure:
    * `Authorization` header is set correctly,
    * logs never contain cap substring.
  * Error mapping:
    * canonical error envelopes → `RonException` fields.
  * Timeouts:
    * simulated slow servers cause well-defined `RonException` with `code = "local_timeout"` (or similar), not indefinite blocking.

* [ ] **Property tests / fuzz**:

  * Using tools like jqwik, Kotest property testing, or QuickTheories:
    * randomize error payloads and prove:
      * no parsing panics,
      * no secrets leaked.
    * For idempotency helpers:
      * same input parameters → same idempotency key.
      * random variations produce well-distributed keys.

* [ ] **Integration tests**:

  * JVM SDK → local Micronode/Macronode via `svc-gateway`:
    * `/app/ping`,
    * simple JSON POST/GET,
    * auth errors,
    * 429/5xx handling.
  * Misconfigured endpoints:
    * bad TLS cert,
    * unreachable host,
    * timeouts,
    * ensure they all surface as structured `RonException` with safe messages.

* [ ] **Negative tests**:

  * Broken/expired caps:
    * cause 401/403 with canonical problem envelope parsed,
    * no caps in logs.
  * Huge payloads:
    * the SDK fails gracefully (error + maybe partial body) and doesn’t OOM on typical dev machines.

---

## 8) Docs & DX

**Goal:** Secure by default, even if devs only copy-paste.

* [ ] **Quickstarts show secure patterns**:

  * Server:
    * use env vars for caps,
    * single shared `RonClient` bean,
    * timeouts & baseUrl configured via env or config.
  * Android:
    * fetch caps via a secure backend auth flow,
    * store in memory or in Keystore-backed storage,
    * never hardcode caps in source.

* [ ] **Explicit “don’t do this in prod” callouts**:

  * Any example that:
    * uses `http://localhost:5304`,
    * sets `insecureHttp(true)`,
    * hardcodes tokens,
    * logs raw responses,
    * must be wrapped in “DEV ONLY / DO NOT USE IN PROD” banners.

* [ ] **Threat model call-out**:

  * README section:

    * For **servers**:
      * assume host is trusted,
      * treat data as sensitive/PII,
      * integrate with structured logging and secrets management.
    * For **Android/desktop**:
      * assume device can be lost,
      * logs can be scraped,
      * attackers may interact with the app,
      * never embed long-lived admin caps.

---

## 9) Future / “God-Tier” Hooks

Not required for first JVM SDK, but keep hooks ready in config and types.

* [ ] **PQ toggle hooks**:

  * Config options like `pqMode: OFF|HYBRID` reserved, to be wired once JVM HTTP clients can use PQ-hybrid TLS, mirroring core RON-CORE PQ profile.

* [ ] **Audit integration**:

  * Optional `auditContext`/`actorId` fields in request options, forwarded as headers so `ron-audit` can record who did what.

* [ ] **Signed config/caps**:

  * Structures flexible enough to later:
    * accept signed caps,
    * accept signed facet manifests,
    * verify signatures via `ron-kms` or similar.

* [ ] **Formal methods**:

  * Core auth & idempotency logic is isolated in small, pure modules (no global state), so we can:
    * translate them into TLA+/Alloy in the future,
    * reason about failures independently of HTTP stack.

---

## 10) Schema & Security (Interop Guarantees)

**Goal:** Security properties are preserved identically across SDKs via shared schemas and envelopes, not ad hoc JVM-only behavior.

### 10.1 Canonical Error Envelope

`ron-app-sdk-jvm` must treat the **canonical error envelope** from `SDK_SCHEMA_IDB.MD` as source of truth:

```json
{
  "code": "string",
  "message": "string",
  "kind": "string",
  "correlation_id": "string",
  "details": { }
}
````

* [ ] JVM SDK MUST parse error responses into this shape wherever possible.
* [ ] If parsing fails (non-JSON, HTML, plain text), SDK wraps it in a **generic transport error**:

  * message is safe (“Unexpected error response from gateway”),
  * optional debug field exists only in opt-in paths.
* [ ] User-visible exceptions expose:

  * `code`, `kind`, `message`, `correlationId`, sanitized `details`.

### 10.2 DTO & Auth Field Consistency

* [ ] DTOs follow canonical schemas (`ron-proto` / OpenAPI):

  * Same field names,
  * Canonical JSON mapping (e.g., `u64` as string, timestamps as ISO 8601 strings),
  * Types mapped via `SDK_SCHEMA_IDB.MD`.

* [ ] Auth fields consistent:

  * `Authorization` header usage same as other SDKs.
  * No capability/macroon fields leak into DTO payloads or logs.

* [ ] SDK never stores caps inside DTOs or error envelopes; they live only in client/session state.

### 10.3 Facet Manifest Security

If/when JVM SDK helps generate facet manifests (TOML):

* [ ] It MUST emit manifests matching `SDK_SCHEMA_IDB.MD` schema:

  * `facet.id`, `facet.kind`,
  * `route.method`, `route.path`,
  * optional `facet.security`, `facet.limits`, `route.integrity`, etc.

* [ ] It MUST respect security hints:

  * never silently set `public = true` / `requires_auth = false`,
  * never allow route paths that escape the configured static root (`../` etc.).

* [ ] For proxy facets:

  * require explicit upstream config,
  * avoid “open proxy” defaults,
  * recommend mutual TLS or restricted networks.

### 10.4 Schema-Driven Validation in Tests

* [ ] Unit tests:

  * error envelope parsing from JSON samples,
  * DTO serialization/deserialization roundtrip tests (golden fixtures).

* [ ] Interop tests:

  * same suite as other SDKs:

    * check that auth errors produce proper structured envelopes,
    * ensure unknown fields in responses are ignored or surfaced as extensions, never crash.

* [ ] Property tests:

  * fuzz error payloads and ensure:

    * no NPEs/IndexOutOfBounds,
    * no raw internal state leaked in messages.

### 10.5 Frontend vs Backend Surface

* [ ] Android/desktop usage:

  * Treat all DTOs as untrusted data from network.
  * Docs emphasize sanitization before rendering in WebView / rich text.

* [ ] Server usage:

  * Assume environment is trusted but data may include PII.
  * Provide hooks for structured logging/redaction (e.g., pluggable logger or interceptor interface).

---

```
```
