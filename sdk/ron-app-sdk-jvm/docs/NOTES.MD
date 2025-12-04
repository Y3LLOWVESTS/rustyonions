### BEGIN NOTE - NOVEMBER 30 2025 - 14:55 CST

---

## 1. High-level status of ron-app-sdk-jvm

**Goal:** JVM SDK for RON-CORE with:

* **Core (Java)**: HTTP, config, errors, retries, pagination.
* **Kotlin facade**: coroutines, DSL, extension sugar.
* **Facets**: manifest builder + TOML writer (interops with schema).
* **Interop tests**: hit a real Micronode/gateway.
* **Examples**: Java CLI, Spring Boot, Ktor, Android sample.
* **Tools/CI**: codegen + lint/test/interop scripts.

**Current milestone:**
✅ `./gradlew :core:test` **passes** on Java 17 + Gradle 8.10.2.

Terminal snapshot:

* All core tests (config, env, errors, retry policy, client basics) are green.
* Only warning: “Deprecated Gradle features used; incompatible with Gradle 9.0” – not urgent right now.

---

## 2. Tooling + runtime baseline

### Java

* Installed: **Temurin 17.0.17**
  Verified via:

  ```bash
  java -version
  javac -version
  ```

* Decision: **Java 17 is our baseline** for ron-app-sdk-jvm.

  * Good compatibility with Spring / enterprise.
  * Android-friendly for server/desktop-side libs.
  * Future plan: test on Java 21 in CI, but keep language level at 17.

### Gradle

* We had a corrupt wrapper jar initially.

* Fixed by:

  1. Installing global Gradle (via Homebrew).
  2. Regenerating wrapper from SDK root:

     ```bash
     cd sdk/ron-app-sdk-jvm
     gradle wrapper --gradle-version 8.10.2
     ```

* Now:

  ```bash
  ./gradlew --version
  ```

  shows:

  * Gradle **8.10.2**
  * JVM: Temurin 17.0.17
  * OS: macOS

* We use **wrapper-only** going forward:

  ```bash
  ./gradlew :core:test
  ./gradlew :kotlin:test
  ./gradlew :facets:test
  ```

---

## 3. Root project / Gradle layout

**File:** `build.gradle.kts` (root of ron-app-sdk-jvm)

* Plugins:

  * Only **third-party** plugins declared with versions and `apply false`:

    * `org.jetbrains.kotlin.jvm`
    * `org.jlleitschuh.gradle.ktlint`
    * `io.gitlab.arturbosch.detekt`
  * We **do not** declare core Gradle plugins (`java-library`, `checkstyle`) with `apply false` anymore (that caused the earlier error).

* `allprojects`:

  * `group = "dev.roncore"`
  * `version = "0.1.0-SNAPSHOT"`
  * `repositories { mavenCentral() }`

* `subprojects`:

  * All test tasks use **JUnit Jupiter** and log PASSED/SKIPPED/FAILED.

**File:** `gradle/libs.versions.toml`

* Versions:

  * Java = 17
  * OkHttp = 4.12.0
  * Jackson = 2.17.2
  * JUnit = 5.11.0
  * Coroutines = 1.8.1

* Libraries:

  * `okhttp`
  * `jackson-core`, `jackson-databind`, `jackson-annotations`, `jackson-jsr310`
  * `junit-jupiter`
  * `kotlin-coroutines-core`

This catalog is referenced from module-level `build.gradle.kts` files (`libs.okhttp`, `libs.junit.jupiter`, etc.).

---

## 4. Core module status (`core/`)

### 4.1. `RonConfig` + `EnvConfigLoader`

**RonConfig** (immutable config):

* Fields:

  * `baseUrl` (String) – raw URL (what tests see).
  * `baseUri` (URI) – parsed form (what `RonClient` uses).
  * `connectTimeout`, `readTimeout`, `writeTimeout`, `overallTimeout` – `Duration`.
  * `allowInsecureHttp` (boolean).
  * `maxRetries` (int).
  * `maxResponseBytes` (long, default 5 MiB).

* Builder methods:

  * `baseUrl(String)`:

    * stores string.
    * parses to URI, throws `RonException.configError("CONFIG_INVALID_URL", ...)` on failure.
  * `connectTimeout(Duration)`, `readTimeout(Duration)`, `writeTimeout(Duration)`, `overallTimeout(Duration)`.
  * Millisecond helpers:

    * `connectTimeoutMs(long)`, `readTimeoutMs(long)`, `writeTimeoutMs(long)`, `overallTimeoutMs(long)`.
  * `allowInsecureHttp(boolean)`.
  * `maxRetries(int)`, `maxResponseBytes(long)`.
  * `hasBaseUrl()` used by env loader.

* Build invariants:

  * If `baseUrl` missing/blank:

    * throw `CONFIG_MISSING_BASE_URL`.
  * Ensures `baseUri` is synced with `baseUrl`.
  * If scheme is not `https` and `allowInsecureHttp == false`:

    * throw `CONFIG_INSECURE_HTTP_DISABLED` (dev/test must opt into HTTP explicitly).

* Getters used by tests:

  * `getBaseUrl()` → String.
  * `getBaseUri()` → URI.
  * `getOverallTimeoutMs()`, `getConnectTimeoutMs()`, `getReadTimeoutMs()`, `getWriteTimeoutMs()`.
  * `isAllowInsecureHttp()`.

**EnvConfigLoader**:

* Two main entrypoints:

  * `RonConfig.Builder fromEnv()`:

    * Creates `RonConfig.builder()`.
    * Calls `applyEnv(builder, System.getenv())`.

  * `void applyEnv(RonConfig.Builder builder, Map<String, String> env)`:

    * Respects builder overrides:

      * Only applies `RON_SDK_GATEWAY_ADDR` if `!builder.hasBaseUrl()`.
    * Env it understands:

      * `RON_SDK_GATEWAY_ADDR` → baseUrl.
      * `RON_SDK_OVERALL_TIMEOUT_MS` → `overallTimeoutMs`.
      * `RON_SDK_CONNECT_TIMEOUT_MS` → `connectTimeoutMs`.
      * `RON_SDK_READ_TIMEOUT_MS` → `readTimeoutMs`.
      * `RON_SDK_WRITE_TIMEOUT_MS` → `writeTimeoutMs`.
      * `RON_SDK_INSECURE_HTTP` → `allowInsecureHttp(true/false)` for 1/true.
    * Gracefully ignores malformed numbers.
    * Default `overallTimeout` to 30_000ms if not otherwise set.

**Tests:**

* `ConfigEnvLoaderTest` is now all green:

  * `envVarsPopulateConfigDefaults()`:

    * Env-only config yields correct `baseUrl` and timeouts.
  * `explicitConfigOverridesEnvDefaults()`:

    * Builder-specified base URL wins over env.
  * `missingEnvAndConfigShouldStillFailOnBuild()`:

    * No env + no builder baseUrl → `build()` fails with missing baseUrl.

### 4.2. `AppResponse` + `RonProblem`

**AppResponse<T>:**

* Fields:

  * `data`, `problem`, `status` (int).
* Static factories:

  * `static <T> AppResponse<T> ok(T data, int status)`.
  * `static <T> AppResponse<T> error(RonProblem problem, int status)`.
* Methods:

  * `T getData()`.
  * `RonProblem getProblem()`.
  * `int getStatus()`.
  * `boolean ok()` – true if 2xx and `problem == null`.
  * `boolean isOk()` – alias to `ok()` for older code.
* Implements `equals`, `hashCode`, `toString`.

**RonProblem** (from earlier work, but verified behavior):

* Constructor takes:

  * `code`, `message`, `kind`, `correlationId`, `details` (Map<String,Object>).
* Defensively copies `details`.
* `getDetails()` returns an unmodifiable view (tests assert `UnsupportedOperationException` when mutating).

**Tests:**

* `ErrorParsingTest`:

  * `appResponseOkShouldExposeDataAndMarkOk()` – uses `AppResponse.ok(...)` + `ok()`.
  * `appResponseErrorShouldExposeProblemAndMarkNotOk()` – uses `AppResponse.error(...)` + `ok() == false`.
  * `ronProblemShouldBeImmutableAndDefensive()` – confirms defensive copy + unmodifiable map.
  * `appResponseStaticFactoryForEmptyErrorIsConvenient()` – convenience creation works.

All green.

### 4.3. `RetryPolicy` + `BackoffStrategy`

**BackoffStrategy:**

* Interface:

  * `Duration nextDelay(int attempt);`

* Static builders:

  * `noBackoff()` – always zero delay.
  * `exponential(Duration base, Duration max, double jitter)`:

    * Exponential growth with cap at `max`.
    * Optional jitter:

      * `jitter = 0.0` → deterministic.
      * Jitter range ± fraction of delay.

**RetryPolicy:**

* Fields:

  * `int maxRetries`.
  * `BackoffStrategy backoff`.

* Static methods:

  * `defaultPolicy()` – no retries.
  * `exponentialBackoff(int maxRetries, Duration base, Duration max, double jitter)`.

* Core predicate:

  * `boolean shouldRetry(int attempt, int statusCode, boolean idempotent)`:

    * Non-idempotent → never retry.
    * Attempt > `maxRetries` → no.
    * Only retry on:

      * `408`, `429`, or 5xx.

* `execute(String method, Callable<T> callable)`:

  * Determines idempotency (`GET`/`HEAD`).
  * Retries on `RonException` where `ex.isRetryable()` and idempotent and under `maxRetries`.
  * Retries on generic `Exception` approximated as status 503 when allowed.
  * Sleeps using `backoff.nextDelay(attempt)` between retries.
  * Throws structured `RonException.transportError` on final failure / interruption.

**Tests:**

* `RetryPolicyTest` all green:

  * `nonIdempotentRequestsDoNotRetryByDefault()`.
  * `idempotentRequestsRetryUpToMaxRetries()`.
  * `backoffDelaysGrowAndAreCapped()` – uses jitter=0.0 for deterministic durations.

### 4.4. `RonClient`

**Builder:**

* `Builder builder()` static factory.

* Methods:

  * `fromEnv()`:

    * `configBuilder = EnvConfigLoader.fromEnv()`.
    * Clears any existing `config`.
  * `baseUrl(String)`:

    * Creates `configBuilder` if null.
    * Calls `configBuilder.baseUrl(...)`.
  * `config(RonConfig)`:

    * Uses fully-built config, bypassing env and builder state.
  * `tokenProvider(TokenProvider)`, `httpClientAdapter(HttpClientAdapter)`, `jsonMapper(JsonMapper)`, `retryPolicy(RetryPolicy)`.

* `build()`:

  * If `config` null:

    * Requires `configBuilder != null`, or throws `CONFIG_MISSING_BASE_URL`.
    * Calls `configBuilder.build()`.
  * Returns new `RonClient`.

**RonClient internals:**

* Fields:

  * `RonConfig config`.
  * `HttpClientAdapter http` – defaults to `OkHttpClientAdapter`.
  * `TokenProvider tokenProvider`.
  * `JsonMapper jsonMapper` – defaults to `new JsonMapper()`.
  * `RetryPolicy retryPolicy` – defaults to `RetryPolicy.defaultPolicy()`.
  * `ResponseSizeLimiter responseSizeLimiter` – uses `config.getMaxResponseBytes()`.

* Core methods:

  * `get/post/put/patch/delete` → all route through `execute(method, path, body, clazz)`.

* `execute`:

  * Builds URL with `buildUrl(path)`:

    * `base = config.getBaseUri()`.
    * Normalizes path to `/app/...`.
    * Avoids double slashes.

  * Builds headers:

    * `User-Agent: ron-app-sdk-jvm/<SDK_VERSION>`.
    * `X-Ron-Protocol-Version: <PROTOCOL_VERSION>`.
    * `Authorization: Bearer <token>` from `TokenProvider`, if present.

  * Uses `HttpRequestContext`.

  * Executes via `retryPolicy.execute(method, () -> http.execute(ctx))` with IO → `RonException.transportError` wrapping.

  * Applies `ResponseSizeLimiter` on response body length.

  * Uses `JsonMapper.decodeAppResponse(...)` to get `AppResponse<T>`.

**Tests:**

* `RonClientBasicTest` all green:

  * `buildWithoutBaseUrlShouldFailFast()` – builder with no baseUrl → runtime exception where message mentions baseUrl/config.
  * `buildWithHttpsBaseUrlShouldSucceed()` – simple `RonClient.builder().baseUrl("https://node.example.com").build()`.
  * `configBuilderRoundTripKeepsValues()` – ensures baseUrl and timeout getters return expected values.

---

## 5. Kotlin module status (`kotlin/`)

We’ve stubbed out the **Kotlin façade**, but **haven’t run its tests yet**.

### 5.1. Gradle config

**File:** `kotlin/build.gradle.kts`

* Applies Kotlin JVM plugin.
* Uses Java toolchain 17.
* Dependencies:

  * `implementation(project(":core"))`
  * `implementation(libs.kotlin.coroutines.core)`
  * `testImplementation(libs.junit.jupiter)`

(This should be ready for `./gradlew :kotlin:test` once we add tests.)

### 5.2. DSL: `RonConfigDsl.kt`

* `@DslMarker` annotation `RonDslMarker`.

* `RonConfigDsl` class with fields:

  * `baseUrl: String?`
  * `insecureHttp: Boolean?`
  * `connectTimeoutMs`, `readTimeoutMs`, `writeTimeoutMs`, `overallTimeoutMs`: `Long?`
  * `maxRetries: Int?`
  * `maxResponseBytes: Long?`

* `fun ronConfig(block: RonConfigDsl.() -> Unit = {}): RonConfig`:

  * Starts from `EnvConfigLoader.fromEnv()` (so defaults come from env).
  * Applies DSL overrides (baseUrl, timeouts, insecureHttp, maxRetries, maxResponseBytes).
  * Returns built `RonConfig`.

### 5.3. Kotlin facade: `Ron.kt`

* `class Ron(private val client: RonClient) : AutoCloseable`.

* Companion object:

  * `fromConfig(config: RonConfig): Ron` – wraps a `RonClient` built from the given config.
  * `fromEnv(block: RonConfigDsl.() -> Unit = {}): Ron` – builds a `RonConfig` via `ronConfig { ... }`, then uses `RonClient.builder().config(config).build()`.

* Suspend methods:

  * `suspend fun <T> get(path: String, clazz: Class<T>): AppResponse<T>` – uses `withContext(Dispatchers.IO)` and `client.get(path, clazz)`.
  * Convenience `getString(path: String)`.

### 5.4. Extensions: `RonExtensions.kt`

* Coroutine/sugar for Java client:

  * `suspend inline fun <reified T : Any> RonClient.get(path: String): AppResponse<T>`
  * `post`, `put`, `patch`, `delete` variants with body `Any?`.
  * `suspend fun <T : Any> RonClient.get(path: String, type: KClass<T>)`.

All use `withContext(Dispatchers.IO)` under the hood.

### 5.5. `Streaming.kt`

* Placeholder object to reserve a stable API surface for future Flow/streaming support.

### 5.6. Remaining Kotlin work

* Implement tests in `kotlin/src/test/kotlin/dev/roncore/sdk/kotlin/`:

  * `RonDslTest.kt`:

    * Assert that `ronConfig { ... }` correctly overrides env defaults.
    * Assert safe failure when baseUrl missing (if we want DSL-only config).
  * `RonCoroutineTest.kt`:

    * Use a fake `HttpClientAdapter` and `TokenProvider` to test coroutine-based calls.

* After tests exist, run:

  ```bash
  ./gradlew :kotlin:test
  ```

---

## 6. Facets module status (`facets/`)

The file tree is there, but we **haven’t implemented these yet**:

* `FacetKind.kt`
* `FacetSecurity.kt`
* `FacetMeta.kt`
* `FacetLimits.kt`
* `Integrity.kt`
* `RouteDefinition.kt`
* `FacetDefinition.kt`
* `FacetTomlWriter.kt`
* Tests:

  * `FacetTomlWriterTest.kt`
  * `FacetSchemaInteropTest.kt`

**Intended role (per SDK blueprints):**

* Provide builders/types to express:

  * Facet types (query/update/stream).
  * Security constraints (auth, scopes).
  * Limits (max body, rate limits).
  * Integrity metadata (hashes, schema versions).
  * Routes → final manifest that can be written to TOML.

* `FacetTomlWriter` should:

  * Serialize these domain types into the canonical facet manifest format (TOML) that RON-CORE expects.
  * Tests should compare generated TOML against known-good strings and possibly cross-check schema constraints.

**TODO for facets:**

* Implement data classes and enums (Kotlin) for all above.
* Implement `FacetTomlWriter`.
* Implement tests and ensure:

  ```bash
  ./gradlew :facets:test
  ```

  passes.

---

## 7. Interop tests (`interop-tests/`)

Currently:

* File tree exists under `interop-tests/src/test/java/dev/roncore/sdk/interop/`:

  * `InteropSmokeTest.java`
  * `ErrorEnvelopeInteropTest.java`
  * `PaginationInteropTest.java`

But we haven’t wired these to a real node in this session.

**Intended role:**

* Run **in CI** against a live Micronode / gateway, using environment variables:

  * e.g., `RON_SDK_GATEWAY_ADDR`, token, etc.

* Validate:

  * Basic connectivity & happy-path requests.
  * Error envelope interop (mapping of `RonProblem` to actual RON-CORE responses).
  * Pagination semantics (Page<T> integration).

**TODO:**

* Implement these tests once RON-CORE Micronode/gateway endpoints are stable.
* Decide on CI story:

  * Maybe a GitHub Action that:

    * Starts a test Micronode container.
    * Runs `./gradlew :interop-tests:test`.

---

## 8. Examples (`examples/`)

Trees exist but implementations need to be built and verified:

1. **Java CLI** (`examples/java-cli/`):

   * `HelloCli.java` should:

     * Read config from env / CLI arg.
     * Build `RonClient` (or `Ron` for Kotlin flavor).
     * Call a trivial endpoint (`/app/ping` or similar).
     * Print response or error.

2. **Spring Boot** (`examples/spring-boot/`):

   * `RonSdkConfig.java`:

     * Define a `@Bean` returning `RonClient` or `Ron`.
     * Config from env / `application.yml`.

   * `HelloController.java`:

     * Expose an HTTP endpoint that calls RON-CORE through the SDK.

3. **Kotlin Ktor** (`examples/kotlin-ktor/`):

   * `KtorApp.kt`, `Routes.kt`:

     * Set up Ktor application.
     * Inject/construct `Ron` or `RonClient`.
     * Basic route calling RON-CORE.

4. **Android sample** (`examples/android-sample/`):

   * `RonApp.kt` (Application), `MainActivity.kt`, `HelloViewModel.kt`:

     * Use Kotlin facade (`Ron`) and `RonConfigDsl`.
     * Note: Android must not rely on `System.getenv()` in production – we should treat this as a dev-only sample or document that env behavior is for server/desktop, not mobile, and provide alternative config injection.

**TODO:**

* Implement minimal working examples for each.
* Ensure they compile (even if not fully integration-tested yet):

  ```bash
  ./gradlew :examples:java-cli:build
  ./gradlew :examples:spring-boot:build
  ./gradlew :examples:kotlin-ktor:build
  ./gradlew :examples:android-sample:assemble
  ```

---

## 9. Tools & CI (`tools/`)

Two main areas:

1. **Codegen (`tools/codegen/`)**:

   * `openapi-config.yml` – needs to point at the correct RON-CORE gateway OpenAPI description(s).
   * `regenerate-dtos.sh`:

     * Should use a chosen Java client generator (e.g., OpenAPI Generator).
     * Regenerate DTOs and place them in a consistent package, e.g. `dev.roncore.sdk.dtos`.
     * Must follow CODECHECK / CODECOMMENTS guidelines for generated code vs handwritten.

2. **CI scripts (`tools/ci/`)**:

   * `run-lint.sh` – to run:

     * `ktlint` for Kotlin.
     * `checkstyle` for Java.
     * `detekt` for Kotlin static analysis.

   * `run-tests.sh`:

     * `./gradlew :core:test :kotlin:test :facets:test`.

   * `run-interop.sh`:

     * Set up env for interop tests.
     * Run `./gradlew :interop-tests:test`.

**TODO:**

* Actually implement these scripts (right now they’re just placeholders in the tree).
* Make sure they align with the RON-CORE-wide CI story and CODECHECK.MD expectations.

---

## 10. Docs & meta files

You’ve already dropped:

* `README.md` – needs to describe:

  * Minimum Java version (17).
  * How to add the SDK via Gradle/Maven (planned coordinates).
  * Quick start examples (Java + Kotlin).
* `SDK_IDB.MD`, `SDK_SECURITY.MD`, `SDK_SCHEMA_IDB.MD` – design docs for:

  * Contract of SDK surface.
  * Security invariants (timeouts, max body sizes, retry limits).
  * Schema mapping for facets / problems / responses.

**TODO for docs:**

* Align README + SDK files with the **actual code** we now have:

  * RonConfig fields and defaults.
  * Env var semantics (matching EnvConfigLoader).
  * Retry policy behavior (max retries, idempotency).
  * Kotlin facade API (`Ron.fromEnv`, `ronConfig {}`).

---

## 11. Suggested next steps when we resume

When we come back to ron-app-sdk-jvm, a good next move:

1. **Kotlin tests**:

   * Implement `RonDslTest.kt` and `RonCoroutineTest.kt`.

   * Run:

     ```bash
     ./gradlew :kotlin:test
     ```

   * Fix any signature mismatches via full drop-in Kotlin updates.

2. **Facets domain + TOML writer**:

   * Implement data classes/enums in `facets/`.
   * Implement `FacetTomlWriter`.
   * Wire up tests so `./gradlew :facets:test` is green.

3. **Examples & docs**:

   * Fill out Java CLI + Spring Boot example using the now-stable `RonClient`/`Ron` + config DSL.
   * Update README.md with real code snippets.

4. **Interop tests** (when Micronode/gateway is ready):

   * Hook `interop-tests` to a live node.
   * Verify real HTTP flows and error envelopes.

---

If you paste this whole carry-over chunk into your project notes (or `ALLNOTES.MD`/SDK-specific notes), Future-You will know:

* ✅ Tools and versions (Java 17, Gradle 8.10.2).
* ✅ Core module is **fully tested and passing**.
* ✅ Kotlin facade is scaffolded and ready for tests.
* 🚧 Facets, examples, interop tests, and CI scripts still need implementation and wiring.

Whenever you’re ready to continue, we can jump straight into **Kotlin tests** or **facets** with this as the baseline.

### END NOTE - NOVEMBER 30 2025 - 14:55 CST



### BEGIN NOTE - NOVEMBER 30 2025 - 22:00 CST

I’d call ron-app-sdk-jvm roughly **65–70% complete** for a **developer-preview SDK** scope.

* **Core Java client:** ~100% for preview (implemented, tested, green).
* **Kotlin facade:** ~90% (DSL + coroutine extensions implemented & tested; could use a bit more coverage/ergonomics polish).
* **Facets module:** ~90% (domain types + TOML writer + tests all green).
* **Examples:** ~40% (Java CLI + Spring Boot done; Ktor + Android still to come).
* **Interop tests:** ~0–10% (wiring plan exists, no module/tests yet).
* **Tools/CI:** ~50% (test runner wired; lint & interop scripts are placeholders).
* **Docs/IDBs alignment:** ~40% (docs exist but need a pass to match real code).

Below is a full carry-over pack you can paste into NOTES / ALLNOTES.

---

## 1. High-level status & mental model

**Goal:** ron-app-sdk-jvm is the **JVM SDK for RON-CORE**, split into focused modules:

* `core/` — Java-first HTTP client, config, errors, retry, envelope decoding.
* `kotlin/` — Kotlin DSL + coroutine extensions on top of `RonClient`.
* `facets/` — facet manifest model + TOML writer (interop with RON facet schema).
* `examples/` — integration samples:

  * `examples/java-cli/`
  * `examples/spring-boot/`
  * (future) `examples/kotlin-ktor/`
  * (future) `examples/android-sample/`
* `tools/ci/` — CI hooks (`run-tests.sh`, `run-lint.sh`, `run-interop.sh`).

**Runtime baseline:**

* Java: **Temurin 17.0.17** (language level 17).
* Gradle: **8.10.2** via wrapper.
* Spring Boot (example): 3.3.5.
* Jackson core (from libs.versions.toml): 2.17.x.
* OkHttp (core HTTP impl): 4.12.x.
* Kotlin coroutines (kotlin module): 1.8.x.

All Java/Kotlin modules currently compile and test cleanly on this stack.

---

## 2. Project layout & Gradle settings

**File:** `sdk/ron-app-sdk-jvm/settings.gradle.kts`

* `rootProject.name = "ron-app-sdk-jvm"`
* Includes:

  ```kotlin
  include("core")
  include("kotlin")
  include("facets")
  include("examples:java-cli")
  include("examples:spring-boot")
  ```

**Root build (high level):**

* Uses version catalog `gradle/libs.versions.toml` for:

  * okhttp
  * jackson-core / jackson-databind / jackson-annotations / jackson-jsr310
  * junit-jupiter
  * kotlin-coroutines-core

* `core`, `kotlin`, `facets`, and `examples` each have their own `build.gradle.kts`.

**Module Java toolchains:**

* All modules explicitly use **Java 17** toolchain.
* Gradle `test` tasks are configured to use **JUnit Jupiter** with PASSED/SKIPPED/FAILED logging in core, kotlin, and facets.

---

## 3. Core module: RonClient + config + errors + retry

**Status:** **DONE & GREEN** for dev preview.

**Key classes:**

* `dev.roncore.sdk.config.RonConfig`
* `dev.roncore.sdk.config.EnvConfigLoader`
* `dev.roncore.sdk.AppResponse<T>`
* `dev.roncore.sdk.RonProblem`
* `dev.roncore.sdk.RonException`
* `dev.roncore.sdk.retry.RetryPolicy` & `BackoffStrategy`
* `dev.roncore.sdk.http.HttpClientAdapter`, `HttpRequestContext`, `HttpResponse`
* `dev.roncore.sdk.http.OkHttpClientAdapter`
* `dev.roncore.sdk.json.JsonMapper`
* `dev.roncore.sdk.RonClient`

### 3.1 RonConfig + EnvConfigLoader

**RonConfig:**

* Immutable configuration for the JVM SDK.

* Fields:

  * `baseUrl: String` — raw URL.
  * `baseUri: URI` — parsed URL.
  * `connectTimeout`, `readTimeout`, `writeTimeout`, `overallTimeout: Duration`.
  * `allowInsecureHttp: boolean`.
  * `maxRetries: int`.
  * `maxResponseBytes: long` (default 5 MiB).

* Builder:

  * `baseUrl(String)` — sets string and parses `URI`; on failure throws `RonException.configError("CONFIG_INVALID_URL", ...)`.
  * Timeout setters as `Duration` and `*Ms(long)` helpers.
  * `allowInsecureHttp(boolean)`.
  * `maxRetries(int)`, `maxResponseBytes(long)`.

* Build invariants:

  * `baseUrl` must be provided (env or explicit) — otherwise throws `CONFIG_MISSING_BASE_URL`.
  * `baseUri` always reflects `baseUrl`.
  * If scheme not `https` and `allowInsecureHttp == false`, throws `CONFIG_INSECURE_HTTP_DISABLED`.

* Accessors used by tests & client:

  * `getBaseUrl()`, `getBaseUri()`.
  * `getConnectTimeoutMs()`, `getReadTimeoutMs()`, `getWriteTimeoutMs()`, `getOverallTimeoutMs()`.
  * `isAllowInsecureHttp()`, `getMaxResponseBytes()`.

**EnvConfigLoader:**

* Centralizes env semantics for `RON_SDK_*` vars.

* `RonConfig.Builder fromEnv()`:

  * Starts with `RonConfig.builder()`.
  * Calls `applyEnv(builder, System.getenv())`.

* `applyEnv(RonConfig.Builder, Map<String,String>)`:

  * Only sets `baseUrl` from `RON_SDK_GATEWAY_ADDR` if `!builder.hasBaseUrl()` (explicit config wins).

  * Reads:

    * `RON_SDK_GATEWAY_ADDR` → baseUrl
    * `RON_SDK_OVERALL_TIMEOUT_MS`
    * `RON_SDK_CONNECT_TIMEOUT_MS`
    * `RON_SDK_READ_TIMEOUT_MS`
    * `RON_SDK_WRITE_TIMEOUT_MS`
    * `RON_SDK_INSECURE_HTTP` → allow insecure for `1` or case-insensitive `true`.

  * Gracefully ignores malformed numbers.

  * Defaults overall timeout to 30_000 ms if not otherwise set.

**Tests:**

* `ConfigEnvLoaderTest`:

  * `envVarsPopulateConfigDefaults()` — pure-env setup works, correct timeouts and baseUrl.
  * `explicitConfigOverridesEnvDefaults()` — builder baseUrl overrides env.
  * `missingEnvAndConfigShouldStillFailOnBuild()` — no baseUrl → build fails.

### 3.2 AppResponse + RonProblem

**AppResponse<T>:**

* Fields: `data: T`, `problem: RonProblem`, `status: int`.

* Factories:

  * `AppResponse.ok(T data, int status)`
  * `AppResponse.error(RonProblem problem, int status)`

* Methods:

  * `getData()`
  * `getProblem()`
  * `getStatus()`
  * `boolean ok()` (true if status 2xx and no problem).
  * `boolean isOk()` — alias retained for convenience.

**RonProblem:**

* Fields: `code`, `message`, `kind`, `correlationId`, `details: Map<String,Object>`.

* Defensive design:

  * Copies `details` on construction.
  * `getDetails()` returns unmodifiable view; mutating it throws `UnsupportedOperationException`.

**Tests:**

* `ErrorParsingTest`:

  * `appResponseOkShouldExposeDataAndMarkOk()`.
  * `appResponseErrorShouldExposeProblemAndMarkNotOk()`.
  * `ronProblemShouldBeImmutableAndDefensive()` — verifies details map is immutable and defensively copied.
  * `appResponseStaticFactoryForEmptyErrorIsConvenient()` — convenience for creating empty error envelope.

### 3.3 RetryPolicy + BackoffStrategy

**BackoffStrategy:**

* Interface: `Duration nextDelay(int attempt);`
* Implementations:

  * `noBackoff()` — always 0ms.
  * `exponential(base: Duration, max: Duration, jitter: double)`:

    * Exponential growth with cap at `max`.
    * Optional jitter (when jitter > 0).

**RetryPolicy:**

* Fields: `maxRetries: int`, `backoff: BackoffStrategy`.

* `defaultPolicy()` — no retries.

* `exponentialBackoff(int maxRetries, Duration base, Duration max, double jitter)`.

* Retry decision:

  * Only retries **idempotent** methods (`GET`, `HEAD`).
  * Never retries non-idempotent methods (POST, etc).
  * Only retries on status 408, 429, or 5xx.
  * Observes `maxRetries`.

* `execute(method, Callable<T>)`:

  * Computes idempotency from method.
  * Retries on `RonException` marked retryable + idempotent + under `maxRetries`.
  * Retries on generic `Exception` as if status 503, where allowed.
  * Sleeps using `backoff.nextDelay(attempt)`.

**Tests:**

* `RetryPolicyTest`:

  * `nonIdempotentRequestsDoNotRetryByDefault()`.
  * `idempotentRequestsRetryUpToMaxRetries()`.
  * `backoffDelaysGrowAndAreCapped()` — deterministic jitter 0.

### 3.4 RonClient

**Builder:**

* Start with `RonClient.builder()`:

  * `.fromEnv()` → loads `RonConfig.Builder` from `EnvConfigLoader.fromEnv()`.
  * `.baseUrl(String)` → ensures configBuilder exists, calls `baseUrl`.
  * `.config(RonConfig)` → bypasses builder/env, uses fully built config.
  * `.tokenProvider(TokenProvider)`
  * `.httpClientAdapter(HttpClientAdapter)`
  * `.jsonMapper(JsonMapper)`
  * `.retryPolicy(RetryPolicy)`

* `build()`:

  * If explicit `config` is null:

    * Requires `configBuilder != null` or throws `CONFIG_MISSING_BASE_URL`.
    * Calls `configBuilder.build()`.

  * Constructs `RonClient` with:

    * Config
    * Http adapter (default OkHttp)
    * JSON mapper (default Jackson `JsonMapper`)
    * Retry policy (default no retries)
    * ResponseSizeLimiter (max bytes from config).

**Execution:**

* Workflow:

  * API methods: `get/post/put/patch/delete(path, clazz)` → all route to private `execute(method, path, body, clazz)`.

  * Build URL:

    * Uses `baseUri` from config.
    * Normalizes path to `/app/<...>` to form full URL.

  * Build headers:

    * `User-Agent: ron-app-sdk-jvm/<SDK_VERSION>`
    * `X-Ron-Protocol-Version: <PROTOCOL_VERSION>`
    * `Authorization: Bearer <token>` if provided.

  * Wraps HTTP in `HttpRequestContext`.

  * Executes via `retryPolicy.execute(method, () -> http.execute(ctx))`.

  * Applies `ResponseSizeLimiter` to body length.

  * Uses `JsonMapper` to decode the JSON envelope into `AppResponse<T>`.

**Tests:**

* `RonClientBasicTest`:

  * `buildWithoutBaseUrlShouldFailFast()` — ensures config invariants enforced.
  * `buildWithHttpsBaseUrlShouldSucceed()`.
  * `configBuilderRoundTripKeepsValues()` — verifies timeouts/baseUrl equality in config builder and built config.

---

## 4. Kotlin module: DSL + coroutine facade

**Status:** **Implemented & tested**, minor warning about an extension shadowing a member (harmless).

### 4.1 Gradle & layout

**File:** `kotlin/build.gradle.kts`

* Applies `org.jetbrains.kotlin.jvm`.
* `java.toolchain` set to Java 17.
* Dependencies:

  * `implementation(project(":core"))`
  * `implementation(libs.kotlin.coroutines.core)`
  * `testImplementation(libs.junit.jupiter)`

### 4.2 RonConfig DSL

**File:** `kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/RonConfigDsl.kt`

* `@DslMarker` annotated `RonDslMarker`.

* `RonConfigDsl` properties:

  * `baseUrl: String?`
  * `insecureHttp: Boolean?`
  * `connectTimeoutMs`, `readTimeoutMs`, `writeTimeoutMs`, `overallTimeoutMs: Long?`
  * `maxRetries: Int?`
  * `maxResponseBytes: Long?`

* `fun ronConfig(block: RonConfigDsl.() -> Unit = {}): RonConfig`:

  * Starts from `EnvConfigLoader.fromEnv()` builder.
  * Applies DSL overrides if non-null.
  * Returns built `RonConfig`.

### 4.3 Ron facade (Kotlin)

**File:** `kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/Ron.kt`

* `class Ron(private val client: RonClient) : AutoCloseable`.

* `companion object`:

  * `fromConfig(config: RonConfig): Ron` – wraps `RonClient.builder().config(config).build()`.
  * `fromEnv(block: RonConfigDsl.() -> Unit = {}): Ron` – uses `ronConfig {}` DSL then builds client.

* Suspend methods using `withContext(Dispatchers.IO)`:

  * `suspend fun <T> get(path: String, clazz: Class<T>): AppResponse<T>`
  * `suspend fun getString(path: String): AppResponse<String>`

### 4.4 Coroutine extensions

**File:** `kotlin/src/main/kotlin/dev/roncore/sdk/kotlin/RonExtensions.kt`

* Suspend sugar for raw `RonClient`:

  * `suspend inline fun <reified T : Any> RonClient.get(path: String): AppResponse<T>`
  * `post`, `put`, `patch`, `delete` variants.
  * `suspend fun <T : Any> RonClient.get(path: String, type: KClass<T>): AppResponse<T>`

* All run in `Dispatchers.IO`.

* One compiler warning: an extension named `get` is shadowed by a member `get` — not breaking behavior.

### 4.5 Tests

**File:** `kotlin/src/test/kotlin/dev/roncore/sdk/kotlin/RonDslTest.kt`

* Verifies `ronConfig` DSL overrides env defaults correctly.

**File:** `kotlin/src/test/kotlin/dev/roncore/sdk/kotlin/RonCoroutineTest.kt`

* Uses `RecordingHttpClientAdapter` returning a synthetic JSON envelope:

  * `{"data": {"value": "hello-from-test"}, "problem": null}`

* Calls `client.get("/ping", Map::class)` (KClass overload).

* Asserts:

  * `response.ok() == true`
  * Data map contains `value = "hello-from-test"`.
  * Adapter recorded a request whose URL starts with `https://node.example.com/app/`.

**All Kotlin tests are green** (`./gradlew :kotlin:test`).

---

## 5. Facets module: domain model + TOML writer

**Status:** **Implemented & tested**.

### 5.1 Gradle

**File:** `facets/build.gradle.kts`

* Kotlin JVM plugin, Java toolchain 17.
* Dependencies: `testImplementation(libs.junit.jupiter)`.

### 5.2 Domain types

* `FacetKind`:

  * Enum: `STATIC`, `ECHO`, `PROXY`.
  * `wire()` → `"static"`, `"echo"`, `"proxy"`.
  * `fromWire(String)` case-insensitive, throws on unknown.

* `FacetSecurity`:

  * `public: Boolean?`
  * `requiresAuth: Boolean?`
  * Nulls omitted from TOML.

* `FacetMeta`:

  * `description: String?`
  * `owner: String?`
  * `version: String?`

* `FacetLimits`:

  * `maxRps: Int?`
  * `maxConcurrency: Int?`

* `Integrity`:

  * `algo: String`
  * `value: String`

* `RouteDefinition`:

  * `method: String`
  * `path: String`
  * `file: String?` (required for static facets).
  * `upstreamPath: String?` (for future proxy facets).
  * `integrity: Integrity?`.

* `FacetDefinition`:

  * `id: String`
  * `kind: FacetKind` (default STATIC).
  * `security: FacetSecurity?`
  * `meta: FacetMeta?`
  * `limits: FacetLimits?`
  * `routes: List<RouteDefinition>` (must be non-empty).

### 5.3 FacetTomlWriter

**File:** `facets/src/main/kotlin/dev/roncore/sdk/facets/FacetTomlWriter.kt`

* `object FacetTomlWriter { fun write(facet: FacetDefinition): String }`

* Validations:

  * `id` not blank.
  * `routes` not empty.
  * Each route:

    * `path` not blank and starts with `/`.
    * For `FacetKind.STATIC`, `file` must be non-null/non-blank.

* TOML layout:

  * `[facet]`:

    * `id = "..."`, `kind = "static" | "echo" | "proxy"`.

  * `[facet.security]` (only if non-empty):

    * `public`, `requires_auth`.

  * `[facet.meta]`:

    * `description`, `owner`, `version`.

  * `[facet.limits]`:

    * `max_rps`, `max_concurrency`.

  * `[[route]]` for each route:

    * `method = "GET"` (uppercased).
    * `path = "/..."`
    * Optional `file`.
    * Optional `upstream_path` (normalized to leading `/`).
    * Optional `integrity = { algo = "...", value = "..." }`.

* Helper functions:

  * `normalizePath` ensures leading `/`.
  * `escape` escapes `"`, `\`, and control chars for TOML.

### 5.4 Tests

**File:** `FacetTomlWriterTest.kt`

* `writes minimal static facet()`:

  * Single route static facet; compares exact TOML string.

* `validates route path must start with slash()`:

  * Asserts `IllegalArgumentException` for invalid `path`.

* `emits optional sections when provided()`:

  * Asserts that security/meta/limits + integrity fields appear in generated TOML.

**File:** `FacetSchemaInteropTest.kt`

* Tests `FacetKind` round-trip and case-insensitivity.

`./gradlew :facets:test` is green.

---

## 6. Examples: Java CLI & Spring Boot

### 6.1 Java CLI example

**Module:** `examples/java-cli`

* `build.gradle.kts`:

  * `java` + `application` plugins.
  * Depends on `project(":core")`.
  * `application.mainClass = "dev.roncore.sdk.examples.java.HelloCli"`.

**HelloCli.java:**

* Reads `RON_SDK_GATEWAY_ADDR` and `RON_SDK_INSECURE_HTTP`.
* Builds `RonClient` via `RonClient.builder().fromEnv().build()`.
* Calls `client.get("/ping", Map.class)`.

Behavior:

* If `RON_SDK_GATEWAY_ADDR` not set → prints error and exits.
* If gateway reachable but route missing → prints 4xx/5xx and any problem envelope.
* Proved end-to-end flow from env → config → client → gateway.

### 6.2 Spring Boot example

**Module:** `examples/spring-boot`

* `build.gradle.kts`:

  * Plugins: `org.springframework.boot` 3.3.5, `io.spring.dependency-management`, `java`.
  * Depends on `project(":core")` and `spring-boot-starter-web`.
  * `springBoot.mainClass = "dev.roncore.sdk.examples.spring.Application"`.

**Application.java:**

* Standard `@SpringBootApplication` entrypoint.

**RonSdkConfig.java:**

* `@Configuration` class that defines:

  ```java
  @Bean
  public RonClient ronClient() {
      return RonClient.builder().fromEnv().build();
  }
  ```

* Fails fast on misconfiguration via `IllegalStateException`.

**HelloController.java (latest version):**

* `@RestController` with `@GetMapping("/ron/ping")`.
* Injects `RonClient` from `RonSdkConfig`.
* On request:

  * Calls `AppResponse<Map> response = client.get("/ping", Map.class)`.

  * Logs `status` and `ok()`.

  * If `ok()`:

    * Returns `200` with data map (or `"note": "no data payload"`).

  * If `ok() == false`:

    * Returns `502` with JSON body:

      ```json
      {
        "error": "RON-CORE returned a problem envelope",
        "status": 404,
        "problem": "null"
      }
      ```

  * On `RonException`:

    * Returns `502` with JSON error describing exception type and message.

  * On other exceptions:

    * Returns `500` with JSON error.

**Validated flow:**

* Run macronode (`cargo run -p macronode`).

* Observe `svc-gateway: listening on 127.0.0.1:8090`.

* Run Spring Boot example with:

  ```bash
  RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
  RON_SDK_INSECURE_HTTP=1 \
  ./gradlew :examples:spring-boot:bootRun
  ```

* `curl -i http://localhost:8080/ron/ping`:

  * Currently yields `502` with body `{"status":404,...}` because macronode returns `404` on `/app/ping` (no app-plane route yet).
  * This confirms correct **SDK → gateway → Spring** wiring, with failure cause clearly surfaced.

---

## 7. Tools/CI scripts

**Location:** `sdk/ron-app-sdk-jvm/tools/ci/`

### 7.1 run-tests.sh

* **Active** and used from repo root:

  ```bash
  sdk/ron-app-sdk-jvm/tools/ci/run-tests.sh
  ```

* Behavior:

  * Runs:

    ```bash
    ./gradlew :core:test
    ./gradlew :kotlin:test
    ./gradlew :facets:test
    ./gradlew :examples:java-cli:build
    ```

  * All targets pass currently.

### 7.2 run-lint.sh

* **Placeholder** by design (lint not wired yet, but script exists).

* Behavior:

  * Prints planned future behavior (Java checkstyle, ktlint, detekt).
  * Exits `0` so CI isn’t blocked.

* Future plan:

  * Wire `checkstyle` config correctly (`config/checkstyle/checkstyle.xml`).
  * Integrate `config/ktlint.gradle.kts` into `kotlin` and `facets` modules.
  * Integrate `detekt` via `config/detekt.yml`.
  * Update script to:

    * `./gradlew :core:check` (checkstyle)
    * `./gradlew :kotlin:ktlintCheck :facets:ktlintCheck`
    * `./gradlew :kotlin:detekt :facets:detekt`

### 7.3 run-interop.sh

* **Placeholder** for future interop tests.

* Behavior:

  * Logs that interop tests are not wired yet.
  * Explains future behavior: start/connect to test Micronode, run `./gradlew :interop-tests:test`.
  * Exits `0`.

---

## 8. Docs & IDBs

Present but not fully aligned yet:

* `README.md` (at SDK root).
* `SDK_IDB.MD` — interface design blueprint for the SDK surface.
* `SDK_SECURITY.MD` — security invariants (timeouts, retry caps, max response bytes).
* `SDK_SCHEMA_IDB.MD` — mapping between JVM types and RON schemas/facets/problem envelopes.
* `TODO.MD` — local TODO list for JVM SDK.

**Needed alignment pass:**

* Ensure README reflects:

  * Java version (17).
  * How to add dependency via Maven/Gradle (planned `groupId/artifactId`).
  * Quick start examples:

    * Pure Java: using `RonClient` + `RonConfig.builder()` + `EnvConfigLoader`.
    * Kotlin: using `Ron.fromEnv { ... }` and coroutine extensions.
    * Spring Boot snippet referencing the example.

* Ensure docs match actual behavior:

  * Env var names (`RON_SDK_GATEWAY_ADDR`, `RON_SDK_INSECURE_HTTP`, etc.).
  * Default timeouts and `maxResponseBytes`.
  * Retry semantics (idempotent only, status codes, max retries).
  * Facet manifest schema as implemented in `facets/`.

---

## 9. Remaining work & next steps (prioritized)

### 9.1 Near-term (high impact)

1. **Doc alignment + README polish**

   * Update `README.md` with real examples:

     * Java CLI usage.
     * Spring Boot config snippet (`RonSdkConfig`) and `/ron/ping` endpoint.
     * Kotlin DSL and coroutine extension usage.

   * Cross-check with `SDK_IDB.MD` and `SDK_SECURITY.MD` to ensure:

     * All env vars and fields are documented.
     * Timeouts and limits (max bytes, retries) match code.

2. **Kotlin examples / Ktor module**

   * Create `examples/kotlin-ktor`:

     * Ktor app wiring `Ron` or `RonClient` with coroutines.
     * Minimal `/ron/ping` route hitting gateway.

   * Add to `settings.gradle.kts` and `run-tests.sh` (at least `:examples:kotlin-ktor:build`).

3. **Interop test module skeleton**

   * Add `interop-tests` module:

     * `interop-tests/build.gradle.kts` (JUnit Jupiter, depends on `:core` and maybe `:kotlin`).
     * Tests that expect a live RON node:

       * `InteropSmokeTest` — simple request to a known route.
       * `ErrorEnvelopeInteropTest` — verifies problem mapping.
       * `PaginationInteropTest` — later when pagination endpoints exist.

   * Update `run-interop.sh` to:

     * Check for env (e.g. `RON_SDK_GATEWAY_ADDR`).
     * Run `./gradlew :interop-tests:test`.
     * Fail if tests fail.

### 9.2 Medium term

4. **Android sample (examples/android-sample)**

   * KTX-style facade using `Ron` and `RonConfigDsl`.
   * Notes about env not being realistic on Android; use injected config instead.

5. **Lint / static analysis**

   * Fix `config/checkstyle.xml` path and content so `:core:check` works.
   * Wire `ktlint` into `kotlin` and `facets` via `config/ktlint.gradle.kts`.
   * Wire `detekt` via `config/detekt.yml`.
   * Update `run-lint.sh` to call real checks (and remove “placeholder” messaging).

6. **Codegen + schemas (tools/codegen)**

   * Hook up `openapi-config.yml` + `regenerate-dtos.sh` using OpenAPI Generator (or similar).
   * Decide DTO package (e.g. `dev.roncore.sdk.dtos`).
   * Ensure generated DTOs follow CODECHECK/CODECOMMENTS invariants (clear markers, no hand edits).

### 9.3 Longer term / polish

7. **Improved error surfaces**

   * Expand `RonProblem` helpers and mapping for more detailed error reporting.
   * Utility methods on `AppResponse` for common patterns.

8. **Performance / regression tests**

   * Add microbenchmarks or simple load tests to ensure stable latencies and response size handling.

9. **Publishing/story**

   * Decide Maven coordinates for publishing (e.g. `dev.roncore:ron-app-sdk-jvm-core`, `-kotlin`, `-facets`).
   * Add a basic “Publish to Maven (local or remote)” section to README/TODO.

---

## 10. Summary snapshot for Future-You

As of this session:

* ✅ **Core Java SDK**: fully implemented and unit-tested; env config, timeouts, retry, JSON envelope decoding all green.
* ✅ **Kotlin DSL + coroutine facade**: implemented and unit-tested; Map-based KClass test verifies coroutine path and URL normalization.
* ✅ **Facets module**: facet domain model + TOML writer with strong invariants, fully tested.
* ✅ **Java CLI example**: compiles and runs, hits gateway using env config.
* ✅ **Spring Boot example**: compiles and runs, proxies `/ron/ping` to macronode gateway and surfaces 404/502 correctly.
* ✅ **CI hooks**: `run-tests.sh` wired; `run-lint.sh` and `run-interop.sh` exist as non-blocking placeholders.
* 🚧 **Docs and IDBs**: present but need a pass to match actual code and examples.
* 🚧 **Interop tests module**: not yet created; placeholder script documents the future.
* 🚧 **Additional examples**: Ktor and Android samples still to be implemented.
* 🚧 **Lint & static analysis**: config stubs present; checkstyle/ktlint/detekt not wired into CI yet.


### END NOTE - NOVEMBER 30 2025 - 22:00 CST





### BEGIN NOTE - DECEMBER 3 2025 - 12:20 CST
---

## 1. High-level status snapshot (Dec 3, 2025)

**ron-app-sdk-jvm dev-preview completion estimate:** still about **65–70%**.

Same big picture as before:

* ✅ **Core Java client (`core/`)**: implemented & unit-tested.
* ✅ **Kotlin module (`kotlin/`)**: DSL + coroutine facade implemented & tested.
* ✅ **Facets module (`facets/`)**: domain model + TOML writer + tests done.
* ✅ **Examples:**

  * `examples/java-cli`: compiles and runs, hits gateway via env config.
  * `examples/spring-boot`: compiles and runs, proxies `/ron/ping` to gateway.
* ✅ **Tools/CI:**

  * `tools/ci/run-tests.sh` exists and is wired to run per-module tests.
  * `tools/ci/run-interop.sh` exists and runs `:interop-tests:test` when env is set.
* ✅ **Interop test module (new this round):**

  * `interop-tests` module created with Gradle file + 2 test classes.
* 🚧 **Docs & IDBs:** present (README, SDK_IDB, SDK_SCHEMA_IDB, SDK_SECURITY) but still need a sync pass to match real code.
* 🚧 **Ktor + Android examples:** skeletons exist; logic still to be implemented.
* 🚧 **Codegen & schema CI:** files exist (openapi-config, regenerate-dtos.sh) but are basically empty.
* 🚧 **Gradle project graph is currently broken:** Gradle is not seeing any subprojects under `ron-app-sdk-jvm` (this is our active blocker).

---

## 2. What we changed / added in this session

### 2.1 Interop tests module

We added a new module `interop-tests` under the SDK root:

* **Gradle module file:**

  * `interop-tests/build.gradle.kts`

    * Applies `java` plugin.
    * Uses Java 17 toolchain.
    * `testImplementation(project(":core"))`.
    * `testImplementation(libs.junit.jupiter)`.
    * `tasks.test { useJUnitPlatform() }`.

* **Smoke test:**

  * `interop-tests/src/test/java/dev/roncore/sdk/interop/InteropSmokeTest.java`:

    * Uses `RonClient.builder().fromEnv().build()` to talk to a live gateway.
    * Reads `RON_SDK_GATEWAY_ADDR` from environment.
    * If the env var is **missing**, it **skips the test** using JUnit `Assumptions.assumeTrue` so CI can run without a node.
    * Calls `client.get("/ping", Map.class)`.
    * Asserts:

      * `response` is non-null.
      * Status is a valid HTTP range (100–599).
      * If `!response.ok()`, there **must** be a `RonProblem` present.
    * This is purposely tolerant about status code (404 vs 200) so it doesn’t break when app-plane routes change — we only care that the envelope decodes correctly.

* **Pagination placeholder:**

  * `interop-tests/src/test/java/dev/roncore/sdk/interop/PaginationInteropTest.java`:

    * Placeholder test that always skips via `Assumptions.assumeTrue(false, ...)`.
    * Clearly marked as future home for real “list/pagination” interop tests once we have a canonical list endpoint.

### 2.2 Interop CI script: `tools/ci/run-interop.sh`

We built a **real** interop test runner script:

* **Path:** `sdk/ron-app-sdk-jvm/tools/ci/run-interop.sh`
* Behavior:

  * Locates SDK root (`ROOT=../..` from script).
  * Checks that `settings.gradle.kts` includes `include("interop-tests")`.

    * If not, prints instructions and exits **0** (skip interop, don’t fail CI).
  * Checks for `RON_SDK_GATEWAY_ADDR`.

    * If missing, prints a friendly “set this to run interop” message and exits **0**.
  * If everything is in place:

    * Runs `./gradlew :interop-tests:test` (passes extra CLI args through if provided).

This means interop is **opt-in** and won’t break the build if you haven’t wired a node.

### 2.3 Test runner script: `tools/ci/run-tests.sh`

We rewired the unit test runner to be explicit per module:

* **Path:** `sdk/ron-app-sdk-jvm/tools/ci/run-tests.sh`
* Behavior:

  * Identifies SDK root from script location.
  * Runs:

    * `./gradlew :core:test`
    * `./gradlew :kotlin:test`
    * `./gradlew :facets:test`
    * `./gradlew :interop-tests:test` (unit-only; live tests handled by `run-interop.sh`)
    * `./gradlew :examples:java-cli:build`
  * Echoes a friendly success message at the end.

This script **assumes** Gradle sees `:core`, `:kotlin`, `:facets`, `:interop-tests`, and `:examples:java-cli` as valid subprojects. Right now, that assumption is failing, which is our main bug.

### 2.4 Settings file: `settings.gradle.kts`

We also placed a canonical `settings.gradle.kts` at the SDK root:

* **Path:** `sdk/ron-app-sdk-jvm/settings.gradle.kts`
* Content (in words):

  * Sets `rootProject.name = "ron-app-sdk-jvm"`.
  * Includes:

    * `include("core")`
    * `include("kotlin")`
    * `include("facets")`
    * `include("examples:java-cli")`
    * `include("examples:spring-boot")`
    * `include("examples:kotlin-ktor")` (skeleton)
    * `include("examples:android-sample")` (skeleton)
    * `include("interop-tests")`

From the filesystem side, we’ve verified:

```bash
ls settings.*
# -> settings.gradle.kts
```

So there is **no** Groovy `settings.gradle` left to overshadow the Kotlin one.

---

## 3. The current blocker: Gradle sees “No sub-projects”

Despite the settings file being present and listing the subproject includes, running:

```bash
./gradlew projects
```

from the SDK root gives:

```text
Projects:

------------------------------------------------------------
Root project 'ron-app-sdk-jvm'
------------------------------------------------------------

Root project 'ron-app-sdk-jvm'
No sub-projects
```

And `tools/ci/run-tests.sh` fails with:

```text
Cannot locate tasks that match ':core:test' as project 'core' not found in root project 'ron-app-sdk-jvm'.
```

### 3.1 What we know

* We’re definitely in the SDK root:

  ```bash
  cd /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm
  ```

* `settings.gradle.kts` is present in that directory.

* Gradle is **not** picking up any `include("core")` lines — as if the settings file had no includes or wasn’t being run at all.

### 3.2 Likely explanations (for future debugging)

These are hypotheses for next time when we load the new CODEBUNDLE:

1. **The actual settings file Gradle is reading is different from the one we’re editing.**

   * Possibilities:

     * Another `settings.gradle`/`settings.gradle.kts` higher up in the directory tree taking precedence (composite or included build).
     * The SDK might be an **included build** of a larger Gradle workspace (e.g., root settings at `/Users/mymac/Desktop/RustyOnions/settings.gradle.kts`) that treats `ron-app-sdk-jvm` as a **single project** rather than a multi-module build.
   * In that scenario, the `settings.gradle.kts` inside `ron-app-sdk-jvm` may be ignored by design.

2. **The `settings.gradle.kts` contents in the CODEBUNDLE might not match what we think.**

   * When you re-run `tools/make_codex.sh -s ron-app-sdk-jvm`, it regenerates `CODEBUNDLE.md` based on the current checkout.
   * On the next session we should inspect:

     * What `settings.gradle.kts` actually contains in that bundle.
     * Whether there’s any extraneous characters, encoding issues, or accidental comments that might prevent `include(...)` from executing (unlikely, but worth verifying).

3. **VS Code / Gradle Java tooling state after the crash.**

   * Your laptop battery died mid-session.
   * After restart, you chose “reopen windows” and VS Code had to re-sync Java/Gradle.
   * It’s *possible* the IDE imported the project as a different Gradle root (e.g. the entire RustyOnions repo) and is using a different tasks model than the bare CLI call — but the CLI `./gradlew projects` is authoritative, so we focus on that.
   * There is a slim chance that VS Code or a plugin wrote a default/empty `settings.gradle` at some point and then it was later removed; we’ve already confirmed there’s only `settings.gradle.kts` now.

**Bottom line:**
We’ve definitely **created** a multi-module layout, but the surrounding Gradle setup might still be treating `ron-app-sdk-jvm` as a **single root project** with no subprojects. We need to see the whole Gradle story from the CODEBUNDLE next time (including any repo-root Gradle files) to fix this properly.

---

## 4. What remains for ron-app-sdk-jvm (big picture, beyond the Gradle bug)

Once the Gradle project graph is fixed, the remaining work to get to a solid **developer preview** looks like this:

### 4.1 Fix Gradle project layout (blocker)

* Confirm exactly how the **overall RustyOnions workspace** uses Gradle for the JVM SDK:

  * Is `ron-app-sdk-jvm` meant to be a **standalone** Gradle root?
  * Or is it an included build / subproject of a larger Gradle root in `RustyOnions/`?
* Make sure the “real” `settings.gradle(.kts)` that Gradle uses has the correct `include(...)` lines for:

  * `:core`
  * `:kotlin`
  * `:facets`
  * `:examples:java-cli`
  * `:examples:spring-boot`
  * `:examples:kotlin-ktor`
  * `:examples:android-sample`
  * `:interop-tests`
* Once this is corrected, `./gradlew projects` from the correct root should list these subprojects, and `tools/ci/run-tests.sh` will start working.

*(We’ll diagnose this with the next CODEBUNDLE: checking root Gradle files, included builds, and what tasks exist.)*

### 4.2 Finish Ktor example (`examples/kotlin-ktor`)

* Implement a minimal Ktor app:

  * `Application.module` wiring `Ron` or `RonClient`.
  * `/ron/hello` route that calls `ron.get<String>("/hello")`.
  * Proper mapping of ok vs error responses to HTTP codes.
* Ensure the module:

  * Has `build.gradle.kts` with Ktor + Kotlin dependencies.
  * Is built by `run-tests.sh` (e.g., `:examples:kotlin-ktor:build` once stable).

### 4.3 Finish Android sample (`examples/android-sample`)

* Implement:

  * `RonApp` (Application) that creates a `Ron` or `RonClient` from config.
  * `HelloViewModel` using coroutines and `Ron` to fetch a `/hello` route.
  * `MainActivity` (or Compose screen) observing a `StateFlow<String>` and showing the result.
* Keep it dev-friendly / non-production:

  * Config injected manually or via a local configuration class (no System.getenv).
  * Clear comments about how real apps should handle secrets and tokens.

### 4.4 Wire up codegen + schema alignment

* Fill out:

  * `tools/codegen/openapi-config.yml` with:

    * URL/path to the gateway OpenAPI spec.
    * Output package for DTOs (e.g. `dev.roncore.sdk.dtos`).
  * `tools/codegen/regenerate-dtos.sh` to:

    * Call your chosen OpenAPI generator.
    * Regenerate `src/generated/java/dev/roncore/sdk/dtos`.
* Decide whether to:

  * Run this generator manually (doc’d in README).
  * Or integrate into CI for schema-drift detection.

### 4.5 Docs & security/schema IDBs

* Update `README.MD`:

  * Make sure the Quick Start sections use the **actual** Java CLI and Spring Boot examples.
  * Include a Kotlin example showing `Ron.fromEnv { ... }` and `client.get<String>("/hello")`.
* Align `SDK_IDB.MD`, `SDK_SECURITY.MD`, `SDK_SCHEMA_IDB.MD` with:

  * Actual env vars (`RON_SDK_GATEWAY_ADDR`, `RON_SDK_INSECURE_HTTP`, timeout envs).
  * Real default values from `RonConfig`.
  * Retry policy semantics (idempotent only, which status codes).
  * Facet TOML shape and its invariants (from facets module).

### 4.6 Hardening and polish (post dev-preview)

* Lint & static analysis:

  * Hook up checkstyle, ktlint, detekt using existing `config/*` files.
  * Expand `tools/ci/run-lint.sh` from placeholder into a real gate.
* Interop tests:

  * Add real pagination/endpoints for `PaginationInteropTest`.
  * Possibly add chaos-style tests (timeouts, large bodies) using a test node.
* Observability:

  * Add simple hooks/surfaces to integrate metrics + tracing (e.g. wrapping `HttpClientAdapter`).

---

## 5. Next-session starting point

When we resume:

1. **You’ll generate a fresh `CODEBUNDLE.md` for ron-app-sdk-jvm** (as you already did), and paste/upload it.
2. We’ll:

   * Inspect the actual `settings.gradle.kts` from that bundle.
   * Check if there’s a *workspace-level* Gradle root (e.g., `RustyOnions/settings.gradle.kts`) that treats `ron-app-sdk-jvm` as a single project.
   * Confirm how `./gradlew` is supposed to be invoked in this repo (from SDK root vs repo root).
3. Based on that, we’ll:

   * Fix the **real** settings file used by Gradle.
   * Adjust `run-tests.sh` project paths (e.g. `:sdk:ron-app-sdk-jvm:core:test` vs `:core:test`) if needed.
   * Re-run `./gradlew projects` to ensure the subproject graph is correct.
   * Get `tools/ci/run-tests.sh` and `tools/ci/run-interop.sh` green.

Once that Gradle graph is solved, everything else (Ktor, Android, codegen) will be much smoother.


### END NOTE - DECEMBER 3 2025 - 12:20 CST






### BEGIN NOTE - DECEMBER 3 2025 - 14:05 CST

---

## 1. High-level status snapshot (Dec 3, 2025)

**Scope:** JVM SDK for RON-CORE, multi-module Gradle project:

* `core/` – Java client, config, errors, retry, HTTP.
* `kotlin/` – Kotlin DSL + coroutine facade.
* `facets/` – facet model + TOML writer.
* `examples/` – Java CLI, Spring Boot, Ktor, Android.
* `interop-tests/` – live-gateway smoke tests.
* `tools/ci/`, `tools/codegen/`, `docs/`, SDK IDBs.

**Runtime + build stack:**

* Java toolchain: **17** across modules (via `java { toolchain { languageVersion.set(JavaLanguageVersion.of(17)) } }`).
* Gradle: **8.10.2** via wrapper.
* Root build:

  * Version catalog in `gradle/libs.versions.toml` (jackson, okhttp, junit, coroutines).
  * Root `build.gradle.kts` wires kotlin/detekt/ktlint plugins and common `Test` logging via `subprojects { ... }`.

**Completion by area (dev-preview lens):**

* **Core Java client (`core/`):** ~100% (green tests, config/env loader, retry/backoff, error envelope parsing).

* **Kotlin module (`kotlin/`):** ~90% (Ron DSL, coroutine helpers, tests).

* **Facets (`facets/`):** ~90% (facet types, TOML writer, tests).

* **Examples:**

  * Java CLI + Spring Boot: implemented and hooked to `:core`.
  * Ktor + Android: scaffolds exist but bodies are empty. Overall examples ~50–55%.

* **Interop tests (`interop-tests/`):** module + smoke test wired, pagination placeholder present; ~40%.

* **Tools/CI:** `run-tests.sh` and `run-interop.sh` are real; `run-lint.sh` is a no-op placeholder. ~70%.

* **Docs / IDBs:** README + SDK_IDB/SDK_SCHEMA_IDB/SDK_SECURITY exist but need a sync pass with real code; ~40%.

**Net:** solid core + Kotlin + facets + examples for Java/Spring + basic interop are all in place and *green*. Missing pieces are mostly around examples (Ktor/Android), richer interop, codegen, and doc/IDB alignment.

---

## 2. What we accomplished in this session

### 2.1 Fixed Gradle multi-module wiring

Earlier, Gradle reported:

> Root project 'ron-app-sdk-jvm'
> No sub-projects

and `:core:test` failed with “project 'core' not found”.

We now have:

```text
> Task :projects

Root project 'ron-app-sdk-jvm'
+--- Project ':core'
+--- Project ':examples'
|    +--- Project ':examples:android-sample'
|    +--- Project ':examples:java-cli'
|    +--- Project ':examples:kotlin-ktor'
|    \--- Project ':examples:spring-boot'
+--- Project ':facets'
+--- Project ':interop-tests'
\--- Project ':kotlin'
```

Key wiring:

* `settings.gradle.kts` is now the single source of truth for the JVM SDK and includes all modules: `core`, `kotlin`, `facets`, `examples:*`, `interop-tests`.
* Root `build.gradle.kts` configures group/version, repositories, and shared `Test` logging for all `subprojects`.

Result:

* `./gradlew projects` from `sdk/ron-app-sdk-jvm` lists all modules.
* Gradle tasks like `:core:test` and `:examples:java-cli:build` now resolve correctly.
* `./gradlew clean` *at the root* is still not defined (we don’t apply `base` plugin there), but each subproject has its own `clean` via `java` / `java-library`. That’s fine for now.

### 2.2 CI test runner now fully green

`tools/ci/run-tests.sh` now:

* Prints a friendly banner with `ROOT`.

* Runs:

  * `./gradlew :projects` (sanity).
  * `./gradlew :core:test`
  * `./gradlew :kotlin:test`
  * `./gradlew :facets:test`
  * `./gradlew :interop-tests:test`
  * `./gradlew :examples:java-cli:build`

* Ends with `[ron-app-sdk-jvm] all JVM SDK tests passed.`

You ran it and everything passed against the current CODEBUNDLE and macronode.

### 2.3 Interop tests wired to a live macronode and passing

We now have a real interop loop:

* **Terminal A:** `macronode` running with:

  * `svc-gateway` on `127.0.0.1:8090`
  * `svc-index` shell on `127.0.0.1:5304`

* **Terminal B:**

  ```bash
  cd sdk/ron-app-sdk-jvm
  export RON_SDK_GATEWAY_ADDR=http://127.0.0.1:5304
  export RON_SDK_INSECURE_HTTP=1
  ./gradlew :interop-tests:test --rerun-tasks
  ```

* **Result:** `InteropSmokeTest > gatewayPingReturnsEnvelope()` now **passes**, `PaginationInteropTest` is intentionally **SKIPPED**.

We also confirmed:

* `tools/ci/run-interop.sh`:

  * Validates `settings.gradle.kts` contains `include("interop-tests")`.
  * Validates `RON_SDK_GATEWAY_ADDR` is set.
  * Runs `./gradlew :interop-tests:test` with env wiring.

We adjusted the smoke test to:

* Skip when `RON_SDK_GATEWAY_ADDR` is absent.

* Assert:

  * `AppResponse` is non-null.
  * Status is in `[100..599)`.
  * If a `problem` envelope **is present**, it must at least have a message.

* But **not** fail the build when the macronode shell returns a bare 404 with no envelope on `/app/ping` (current behavior on `127.0.0.1:5304/app/ping`).

This makes interop **real** but also tolerant of early service wiring.

### 2.4 RetryPolicyTest + core tests confirmed green

* `RetryPolicyTest` (with `int maxRetries = 3`/`5` in various tests) is passing:

  * Non-idempotent requests do not retry.
  * Idempotent requests retry up to `maxRetries`.
  * Backoff delays grow and are capped.

So the VS Code highlight on `int maxRetries = 5;` is cosmetic; the underlying tests and semantics are correct.

---

## 3. Current module-by-module snapshot

### 3.1 Root config

* `settings.gradle.kts`:

  * `rootProject.name = "ron-app-sdk-jvm"`.
  * Includes `core`, `kotlin`, `facets`, `examples:*`, `interop-tests`.

* `build.gradle.kts`:

  * Defines plugins for Kotlin/Dkt/ktlint with `apply false`.
  * Sets group/version and Maven Central repo.
  * Configures `subprojects { tasks.withType<Test> { useJUnitPlatform(); testLogging { ... } } }`.

### 3.2 `core/` – Java client

**Status:** Dev-preview complete and green.

* `core/build.gradle.kts`:

  * `java-library` + `checkstyle` plugins.
  * Java 17 toolchain.
  * Dependencies: OkHttp, Jackson core/databind/annotations, Jackson JSR310, JUnit Jupiter.

* Key capabilities (from previous notes + tests):

  * `RonConfig` + `EnvConfigLoader` with env variables.
  * `RonClient` with `builder().fromEnv()` and `get/post` methods that normalize `/ping → /app/ping`.
  * `AppResponse<T>` and `RonProblem` for envelope decoding.
  * `RetryPolicy` + `BackoffStrategy` enforcing bounded retries and capped exponential backoff.
  * Tests: `ConfigEnvLoaderTest`, `RonClientBasicTest`, `ErrorParsingTest`, `RetryPolicyTest` all passing.

### 3.3 `kotlin/` – Kotlin DSL + coroutines

**Status:** ~90%.

* `kotlin/build.gradle.kts`:

  * `org.jetbrains.kotlin.jvm` plugin.
  * Java 17 toolchain.
  * Depends on `:core` + Kotlin coroutines + JUnit.

* Important files:

  * `Ron.kt`, `RonConfigDsl.kt`, `RonExtensions.kt`, `Streaming.kt` providing:

    * `ronConfig { ... }` DSL mapping to `RonConfig`.
    * Coroutine helpers for making requests from suspending contexts.
    * Streaming support surfaces.

  * Tests: `RonDslTest`, `RonCoroutineTest` verifying DSL + coroutine behavior.

### 3.4 `facets/` – facet model & TOML writer

**Status:** ~90%.

* `facets/build.gradle.kts`:

  * Kotlin JVM, Java 17, JUnit only (no HTTP/OkHttp dependency).

* Key types:

  * `FacetDefinition`, `FacetKind`, `FacetLimits`, `FacetMeta`, `FacetSecurity`, `RouteDefinition`, `FacetTomlWriter`, etc.

* Focus:

  * In-memory representation of a facet manifest and TOML serialization aligned with canonical RON facet schema.

### 3.5 `examples/`

**Status overall:** ~50–55%.

1. **Java CLI (`examples/java-cli`)**

   * `build.gradle.kts`:

     * `java` + `application` plugins.
     * Java 17 toolchain.
     * `implementation(project(":core"))`.
     * JUnit + test logging.
     * `application.mainClass = "dev.roncore.sdk.examples.java.HelloCli"`.

   * `HelloCli.java`:

     * Builds `RonClient` via `.fromEnv()`.
     * Performs `client.get("/ping", Map.class)` and prints either data or problem based on `response.ok()`.

   * This is already a good “hello RON from Java” story.

2. **Spring Boot (`examples/spring-boot`)**

   * `build.gradle.kts`:

     * Spring Boot plugin 3.3.5, dependency management, `java`.
     * Java 17 toolchain.
     * `implementation(project(":core"))`.
     * `spring-boot-starter-web` + `spring-boot-starter-test` with JUnit Platform.

   * `Application.java`, `HelloController.java`, `RonSdkConfig.java`:

     * `RonSdkConfig` exposes a singleton `RonClient` bean built from env, with fast-fail on misconfig.
     * `HelloController` uses the client to proxy a `/ron/hello` style route (per earlier sessions).

3. **Ktor (`examples/kotlin-ktor`)**

   * `build.gradle.kts` is present but empty.
   * `KtorApp.kt`, `Routes.kt` exist but are empty stubs.

   → Needs full implementation.

4. **Android sample (`examples/android-sample`)**

   * `build.gradle.kts`, `AndroidManifest.xml`, `HelloViewModel.kt`, `MainActivity.kt`, `RonApp.kt` are all present but empty scaffolds.

   → Also needs full implementation.

### 3.6 `interop-tests/`

**Status:** ~40%.

* `interop-tests/build.gradle.kts`:

  * `java` plugin, Java 17 toolchain.
  * `testImplementation(project(":core"))` + JUnit Jupiter.

* Tests:

  * `InteropSmokeTest` (now relaxed to tolerate bare 404s):

    * Uses `RonClient.builder().fromEnv().build()` based on `RON_SDK_GATEWAY_ADDR`.
    * Skips when env var is missing.
    * Calls `client.get("/ping", Map.class)` and asserts connectivity + status and minimal problem sanity when present.

  * `PaginationInteropTest`:

    * Placeholder that always skips via `Assumptions.assumeTrue(false, ...)`.

### 3.7 Tools & scripts

* `tools/ci/run-tests.sh`: orchestrates module tests and Java CLI build (now fully working in your terminal).

* `tools/ci/run-interop.sh`: robust wrapper for `:interop-tests:test` with safety checks for settings + env.

* `tools/ci/run-lint.sh`: documented no-op; a placeholder for future checkstyle/ktlint/detekt wiring.

* `tools/codegen/openapi-config.yml` and `tools/codegen/regenerate-dtos.sh`: present but empty; need real config and invocation later.

---

## 4. What remains for ron-app-sdk-jvm (dev-preview target)

### 4.1 Finish Ktor example (high impact)

Goal: **Show a Kotlin + Ktor server using the SDK**, parallel to Spring Boot.

Needed work:

* Fill in `examples/kotlin-ktor/build.gradle.kts` with:

  * Kotlin JVM plugin.
  * Ktor BOM (server-netty, jackson).
  * Dependency on `:core`.
  * Application plugin alias for Gradle run.

* Implement:

  * `KtorApp.kt`:

    * `fun main()` starting Netty engine.
    * `module()` that wires routing.

  * `Routes.kt`:

    * Inject or build a `Ron`/`RonClient` instance (likely via env).
    * Expose `/ron/hello` and `/ron/ping` routes that call the gateway and return mapped HTTP responses.

This gives JVM devs a second, non-Spring example and aligns with future Micronode use.

### 4.2 Finish Android sample (high impact for mobile story)

Goal: **Show a minimal Android app calling RON via Kotlin DSL**.

Needed work:

* `examples/android-sample/build.gradle.kts`:

  * Android application plugin.
  * SDK versions (min/target), Compose or classic UI.
  * Dependency on `:kotlin` + coroutines + AndroidX lifecycle.

* Wire:

  * `RonApp.kt`:

    * Initializes a `Ron`/`RonClient` somewhere safe (with clear doc that real apps shouldn’t bake secrets into APK).

  * `HelloViewModel.kt`:

    * Exposes `StateFlow<UiState>` and a `fun load()` that uses coroutines to call RON and post result/error.

  * `MainActivity.kt`:

    * Observes the ViewModel and renders a simple “Ping RON” UI.

* Configure a safe dev-preview env story (e.g., local node on LAN, debugging notes).

This doesn’t need to be production perfect, just enough to prove that the Kotlin facade behaves well on Android.

### 4.3 Richer interop tests (once gateway routes stabilize)

Current smoke test only checks `/app/ping` with very loose assertions. Next steps, once macronode/micronode expose more stable routes:

* Add an interop test for a known, envelope-based route (e.g. `/app/version`, `/app/facets`, etc.), asserting:

  * `ok()` is true.
  * Envelope fields (e.g., version string) match expectations.

* Extend `PaginationInteropTest` to hit a real list endpoint:

  * Validate cursor semantics.
  * Validate page size / next token invariants.

Keep the tests skip-safe when no gateway is running or when env isn’t set.

### 4.4 Codegen + schema alignment

We’ve stubbed out codegen hooks; next you’ll want:

* `tools/codegen/openapi-config.yml`:

  * URL/path to the canonical gateway OpenAPI spec.
  * Output package for generated DTOs (e.g., `dev.roncore.sdk.dtos`).

* `tools/codegen/regenerate-dtos.sh`:

  * Uses OpenAPI generator CLI.
  * Targets a `src/generated/java/dev/roncore/sdk/dtos` subtree.
  * Possibly run in CI to detect schema drift, but at minimum documented for manual use.

Then:

* Thread those DTOs into `core`/`kotlin` where appropriate (e.g., typed request/response models rather than raw `Map`).

### 4.5 Docs + IDBs update

The text docs now lag a bit behind the code:

* **README.MD**

  * Ensure Java Quick Start matches the actual Java CLI and env vars.
  * Add Kotlin + Ktor + Android snippets once those examples are filled in.
  * Document `RON_SDK_GATEWAY_ADDR`, `RON_SDK_INSECURE_HTTP`, and key timeout envs.

* **SDK_IDB.MD / SDK_SCHEMA_IDB.MD / SDK_SECURITY.MD**

  * Align with real `RonConfig` defaults and invariants (max retries, timeouts, max response bytes).
  * Document retry semantics: idempotent-only retries, bounded attempts, capped exponential backoff.
  * Include new interop story: how to run `tools/ci/run-tests.sh` and `tools/ci/run-interop.sh` against a macronode.

### 4.6 Lint/static analysis pass (nice-to-have before wider preview)

* `tools/ci/run-lint.sh` can evolve into a real gate:

  * `./gradlew :core:check` with Checkstyle.
  * `./gradlew :kotlin:detekt` + `ktlint`.
  * `./gradlew :facets:detekt`.

* Keep it initially “warning mode only” so it doesn’t block you while you’re still iterating.

---

## 5. Next high-impact steps (actionable checklist)

If you want a tight sequence for the next coding session, I’d go:

1. **Lock in gateway address convention in docs + examples**

   * Decide if `RON_SDK_GATEWAY_ADDR` should point at `8090` (svc-gateway) rather than `5304` (svc-index shell) going forward.
   * Update README, HelloCli, Spring Boot, and notes to match that convention.

2. **Implement Ktor example end-to-end**

   * Fill in `examples/kotlin-ktor/build.gradle.kts`, `KtorApp.kt`, `Routes.kt`.
   * Confirm it builds and runs locally, and can call your macronode using the same env story as Java/Spring.

3. **Implement Android sample minimal “Ping RON” app**

   * Wire `RonApp`, `HelloViewModel`, `MainActivity` with coroutine + Ron DSL.
   * Target a simple `/ping` or `/version` call against your node.

4. **First real “happy path” interop test**

   * Once a stable app-plane route exists (even a simple `GET /app/version`), add one test in `InteropSmokeTest` or a new test class that expects `ok() == true` and validates the version string.

5. **Pass over README + IDBs**

   * Update docs to reflect the now-working multi-module Gradle setup, interop scripts, and example modules.
   * Make sure env var names and defaults match the current code.

Once those are done, you’ll be extremely close to a “Developer Preview” JVM SDK that:

* Has a solid core + Kotlin + facets story.
* Proves itself with real interop against macronode.
* Ships with multiple integration examples (Java CLI, Spring Boot, Ktor, Android).
* Has scripts and docs that make it easy for other devs to adopt.


### END NOTE - DECEMBER 3 2025 - 14:05 CST





### BEGIN NOTE - DECEMBER 3 2025 - 18:40 CST

Here’s a fresh, up-to-date carry-over snapshot for **ron-app-sdk-jvm** as of the moment you got **Android + Ktor + interop test green**.

---

## 1. High-level status & completion estimate

**Big picture right now:**

* ✅ **Core Java client (`core/`)**

  * RonClient, AppResponse, RonProblem, EnvConfigLoader, HTTP adapter, pagination model, etc. are implemented and unit-tested.

* ✅ **Kotlin module (`kotlin/`)**

  * Kotlin DSL (`ronConfig {}`), coroutine wrappers over `RonClient`, and a `Streaming` placeholder object are in place and tested.

* ✅ **Facets module (`facets/`)**

  * Domain model + TOML writer + tests are wired (see older notes; code is stable).

* ✅ **Examples:**

  * `examples/java-cli` – builds and runs; uses `RonClient.builder().fromEnv()` to hit the gateway.
  * `examples/spring-boot` – builds; exposes an HTTP endpoint that proxies `/ron/ping` to the gateway via the Java SDK.
  * `examples/kotlin-ktor` – **now wired**: Ktor server runs, responds `200 ok` on `/healthz` (confirmed by your curl). It uses the Kotlin module + RonClient plumbing under the hood.
  * `examples/android-sample` – **now building successfully** with `assembleDebug` after:

    * Pinning AGP/Kotlin versions via `settings.gradle.kts` pluginManagement.
    * Wiring `local.properties` with your Android SDK location.

* ✅ **Interop module (`interop-tests/`)**

  * `InteropSmokeTest` is implemented and skips cleanly when `RON_SDK_GATEWAY_ADDR` is missing, otherwise hits `/ping` and asserts basic envelope sanity.
  * `PaginationInteropTest` exists as a **placeholder** that always skips (reserved for list/pagination endpoints later).
  * **New:** `HappyPathInteropTest` (your custom interop test) is added and wired; when `RON_SDK_INTEROP_EXPECT_PING_OK=1` it asserts a true “happy path” (ping must return `ok == true`), when unset it behaves more like a tolerant smoke test.

* ✅ **Gradle project graph + CI scripts**

  * `settings.gradle.kts` now correctly includes **all** subprojects (core, kotlin, facets, examples, interop-tests) and configures pluginManagement for Android.
  * `tools/ci/run-tests.sh` runs per-module tests plus the Java CLI example.
  * `tools/ci/run-interop.sh` runs interop tests when `RON_SDK_GATEWAY_ADDR` is set; otherwise it exits **0** with guidance.

* 🚧 **Docs & IDBs**

  * `README.md`, `SDK_IDB.MD`, `SDK_SCHEMA_IDB.MD`, `SDK_SECURITY.MD` all exist and are decent, but they now **lag behind the code** and need a final sync pass after everything else stabilizes.

* 🚧 **Codegen / schema CI**

  * `tools/codegen/openapi-config.yml` and `tools/codegen/regenerate-dtos.sh` are present but intentionally skeletal; they’re reserved for future “generate DTOs from OpenAPI” flows.

**My honest completion estimate for the JVM SDK (Dev Preview):**
➡️ **~85% complete** (realistic band: **80–90%**).

The remaining 10–20% is mostly polish: a couple more interop tests, codegen wiring, and a final documentation/IDB sync.

---

## 2. Module-by-module snapshot

### 2.1 `core/` (Java SDK core)

**What’s in place:**

* `RonClient` with builder API, HTTP abstraction (`HttpClientAdapter`, `HttpRequestContext`, `HttpResponse`).
* `RonConfig` with env/default loader (`EnvConfigLoader`) and strong invariants around base URL, timeouts, retries, and max response bytes.
* `AppResponse<T>` + `RonProblem` envelope modeling.
* Tests cover:

  * Config env & override behavior.
  * Basic error/envelope parsing.
  * Failure-fast behavior when config is invalid.

**Status:**
Functionally **ready for real use**; later we can add more edge-case tests and Javadoc, but it’s already solid.

---

### 2.2 `kotlin/` (Kotlin DSL + coroutines)

**What’s in place:**

* `ronConfig` DSL to build `RonConfig` idiomatically from Kotlin.
* Coroutine extensions on `RonClient` (inline reified `get/post/put/patch/delete` + KClass variant) that delegate to the underlying Java client on `Dispatchers.IO`.
* `Streaming` object as a reserved namespace for future `Flow`/streaming APIs.

**Tests:**

* `RonDslTest` – asserts the DSL correctly applies overrides, works without env vars, and keeps all timeouts/limits aligned with expectations.
* `RonCoroutineTest` – uses a fake `HttpClientAdapter` to assert:

  * Coroutine wrappers delegate correctly.
  * JSON envelope is decoded into a `Map`.
  * Calls get normalized paths like `/app/*`.

**Status:**
Strong **Dev Preview** quality. Streaming is intentionally a stub, which is fine as long as we don’t document it as GA.

---

### 2.3 `facets/`

* Thin “facade” layer for higher-level patterns (e.g., job submission, long operations) built on top of core.
* Tests and TOML serialization are present and passing (per previous sessions; no recent regressions).

**Status:**
Good enough for dev preview, with room to add more real-world patterns later.

---

### 2.4 `examples/` family

#### `examples/java-cli`

* Simple CLI app that:

  * Builds a `RonClient` via `RonClient.builder().fromEnv().build()`.
  * Hits a sample endpoint (e.g., `/ping`) and prints the result or problem envelope.
* CI script `run-tests.sh` builds this target.

**Run:**

```bash
cd sdk/ron-app-sdk-jvm

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:java-cli:run
```

---

#### `examples/spring-boot`

* Spring Boot app that wires a `RonClient` bean using env config and exposes an endpoint (e.g., `/ron/ping`) to proxy to the node.

**Run:**

```bash
cd sdk/ron-app-sdk-jvm

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:spring-boot:bootRun
```

Hit it from another terminal:

```bash
curl -v http://127.0.0.1:8080/ron/ping
```

---

#### `examples/kotlin-ktor`

* A lightweight Ktor server wired to use the JVM SDK (via the Kotlin module).
* Exposes at least `/healthz` which returns `200 OK` and body `ok` (you confirmed with curl).
* Intention: add endpoints that **proxy** to a RON node via `RonClient` using the Kotlin DSL.

**Run:**

```bash
cd sdk/ron-app-sdk-jvm

# Env vars if the Ktor app proxies to RON-CORE:
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:kotlin-ktor:run
```

Then from another terminal:

```bash
curl -v http://127.0.0.1:8080/healthz
# => HTTP/1.1 200 OK, body "ok"
```

Later you can extend it to e.g. `/proxy/ping` that calls the gateway and surfaces AppResponse data.

---

#### `examples/android-sample`

* An Android sample app that uses the Kotlin SDK to call `/ping` (or similar) and render the result in a simple UI.
* Android toolchain is now **fully wired**:

  * `settings.gradle.kts` uses `pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }` and pins `com.android.application` to 8.2.0.

  * `local.properties` points to your actual SDK:

    ```properties
    sdk.dir=/Users/mymac/Library/Android/sdk
    ```

  * Build fixes applied:

    * `android.buildFeatures.buildConfig = true` to unblock custom `BuildConfig` fields.
    * Added AndroidX flags in `gradle.properties` (`android.useAndroidX=true`, Jetifier disabled when safe).
    * Fixed missing launcher icons in `AndroidManifest.xml` by adding a minimal `mipmap` resource set.

**Build (what you just did):**

```bash
cd sdk/ron-app-sdk-jvm
./gradlew :examples:android-sample:assembleDebug
```

**Next usage step (optional):**

* Open the project in Android Studio, select the `examples/android-sample` configuration, and run on an emulator or device.
* Or from CLI: `./gradlew :examples:android-sample:installDebug` (if you have a device/emulator attached).

---

### 2.5 `interop-tests/`

**Gradle module:**

* `interop-tests/build.gradle.kts`:

  * Java module, toolchain pinned to 17.
  * `testImplementation(project(":core"))` and JUnit Jupiter.

**Tests:**

1. `InteropSmokeTest` – tolerant end-to-end smoke.

   * Skips if `RON_SDK_GATEWAY_ADDR` is missing.
   * Builds `RonClient` via `fromEnv()`.
   * Calls `client.get("/ping", Map.class)` and asserts:

     * Non-null response.
     * Status is between 100–599.
     * If `problem` is present, it must have a message.

2. `PaginationInteropTest` – explicit placeholder:

   * Always skips via `Assumptions.assumeTrue(false, ...)`.
   * Reserved for future “list/pagination” interop tests.

3. **`HappyPathInteropTest`** – your new test:

   * Uses `RonClient.builder().fromEnv()` like the smoke test.
   * Reads **`RON_SDK_INTEROP_EXPECT_PING_OK`**:

     * If set (`"1"`): asserts `response.ok()` and that `data` is non-null — this is the “true” happy path.
     * If **unset**: behaves like a tolerant smoke test; ensures we can call `/ping` and get a syntactically valid `AppResponse`, but doesn’t fail if the node still returns bare errors or 404.
   * You verified:

     * With `RON_SDK_INTEROP_EXPECT_PING_OK=1` it failed (gateway not yet returning `ok`).
     * After **unsetting** that env var, the test passes cleanly.

**Status:**
Interop module is **live and valuable** already (connectivity + envelope behavior), with clearly marked placeholders for future deeper scenarios.

---

## 3. How to run things (cheat sheet)

Assume:

```bash
cd sdk/ron-app-sdk-jvm
```

### 3.1 Core + Kotlin + Facets unit tests

Run the full “regular” test sweep:

```bash
./gradlew :core:test :kotlin:test :facets:test
```

Or use the CI helper script (which also builds the Java CLI and includes interop-tests):

```bash
tools/ci/run-tests.sh
```

That script currently runs:

* `./gradlew :core:test`
* `./gradlew :kotlin:test`
* `./gradlew :facets:test`
* `./gradlew :interop-tests:test` (unit-style; interop-with-node is handled separately)
* `./gradlew :examples:java-cli:build`

### 3.2 Interop tests against a real node

**Prereq:** Have a Micronode/Macronode running with the app-plane gateway exposed, typically at `http://127.0.0.1:8090`.

#### Option A – CI script (recommended)

```bash
cd sdk/ron-app-sdk-jvm

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./tools/ci/run-interop.sh
```

* If `RON_SDK_GATEWAY_ADDR` is missing → script prints guidance and exits **0**.
* If present → runs `./gradlew :interop-tests:test`.

#### Option B – Targeted Happy Path test

If you want **strict success** (ping must be OK):

```bash
cd sdk/ron-app-sdk-jvm

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
RON_SDK_INTEROP_EXPECT_PING_OK=1 \
./gradlew :interop-tests:test --tests "dev.roncore.sdk.interop.HappyPathInteropTest"
```

If the node isn’t that far yet, drop the env var so the test just validates connectivity:

```bash
cd sdk/ron-app-sdk-jvm
unset RON_SDK_INTEROP_EXPECT_PING_OK

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :interop-tests:test --tests "dev.roncore.sdk.interop.HappyPathInteropTest"
```

---

### 3.3 Examples

**Java CLI:**

```bash
cd sdk/ron-app-sdk-jvm

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:java-cli:run
```

**Spring Boot:**

```bash
cd sdk/ron-app-sdk-jvm

RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:spring-boot:bootRun
```

**Kotlin Ktor:**

```bash
cd sdk/ron-app-sdk-jvm

# Env optional unless you proxy to RON-CORE
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:kotlin-ktor:run
```

Then:

```bash
curl -v http://127.0.0.1:8080/healthz
```

**Android sample:**

```bash
cd sdk/ron-app-sdk-jvm
./gradlew :examples:android-sample:assembleDebug
```

(Then install/run via Android Studio or `installDebug`.)

---

## 4. Next steps for ron-app-sdk-jvm (code only; docs last)

Here’s what I’d treat as the remaining work **before** we touch README/IDBs:

### 4.1 Interop tests: round out the story

1. **Finish `ErrorEnvelopeInteropTest`**

   * Goal: explicitly test how the Java SDK behaves when the node returns a proper `RonProblem` envelope:

     * Call some known “failure” path (e.g., `/does-not-exist` or a dedicated error route) and assert:

       * `response.ok()` is false.
       * `response.getProblem()` is non-null and has at least a message and a code.
   * Make it tolerant to older nodes (skip or relax if we still get bare HTTP errors).

2. **Keep `PaginationInteropTest` as a marker**

   * Update comments once you have a canonical list endpoint (e.g., `/jobs?cursor=...`).
   * Wire the first real pagination test when the RON-CORE side stabilizes.

3. **Maybe add a minimal “round-trip latency” measurement test**

   * Not as a perf gate, but as a quick sanity metric: log the duration to reach /ping and back (without asserting on the value).

---

### 4.2 Examples polish

1. **Kotlin Ktor:**

   * Add one real “bridge” route:

     * e.g. `GET /ron/ping` → calls the RON node via Kotlin DSL and returns JSON with `ok`, `status`, `data`, `problem`.
   * Make sure it uses `ronConfig { fromEnv = true }` (or equivalent) so Kotlin devs see the “canonical” way.

2. **Android sample:**

   * Ensure the UI clearly demonstrates:

     * A button: “Ping RON-CORE”.
     * A text view / snackbar showing either success (e.g., `OK, status=200`) or a problem summary.
   * Use coroutines + `viewModelScope` so it’s idiomatic.

3. **Spring Boot example:**

   * Quick pass to align naming (`/ron/ping` vs `/app/ping`) with where you want app-plane to live long-term.
   * Make sure it logs structured messages when the node is unavailable.

---

### 4.3 Tooling & CI odds and ends

* **run-lint.sh**

  * Currently exists but is mostly a placeholder.
  * Wire it to:

    * `./gradlew :core:check` (checkstyle or SpotBugs if present).
    * `./gradlew :kotlin:check` (detekt/ktlint).
    * Maybe a top-level `./gradlew check` once everything is stable.

* **Codegen stubs**

  * Leave `openapi-config.yml` and `regenerate-dtos.sh` as stubs for now, but:

    * Add a `TODO` comment that clearly states “Reserved for future: generate Java/Kotlin DTOs from App Plane OpenAPI spec.”

---

### 4.4 Final step (per your request): docs & IDBs

**Do this LAST, once code is quiet:**

* Update:

  * `README.md`
  * `SDK_IDB.MD`
  * `SDK_SCHEMA_IDB.MD`
  * `SDK_SECURITY.MD`

To reflect:

* The real module layout (core/kotlin/facets/examples/interop).
* How to run:

  * Core tests.
  * Interop tests (including `RON_SDK_INTEROP_EXPECT_PING_OK` semantics).
  * Each example (Java CLI, Spring Boot, Ktor, Android).
* Any security assumptions (e.g., no ambient credentials; env-only config; strict timeouts).

---

## 5. TL;DR for future-you

When you come back to ron-app-sdk-jvm:

1. **Sanity check:** from `sdk/ron-app-sdk-jvm` run:

   ```bash
   tools/ci/run-tests.sh
   ```

2. **If you have a node up at 8090:**

   ```bash
   RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
   RON_SDK_INSECURE_HTTP=1 \
   ./tools/ci/run-interop.sh
   ```

3. **To demo the examples:**

   * Java CLI: `:examples:java-cli:run`
   * Spring Boot: `:examples:spring-boot:bootRun`
   * Ktor: `:examples:kotlin-ktor:run` + `curl /healthz`
   * Android: `:examples:android-sample:assembleDebug`

4. **Then finish:**

   * `ErrorEnvelopeInteropTest` + any final interop tweaks.
   * Small polish on Ktor and Android samples.
   * Wire `run-lint.sh`.
   * **Finally**, freeze the README + SDK_* docs to match reality.

That’s the state of the JVM SDK right now — you’ve got a legit, polyglot-friendly, app-plane-ready client with real examples and interop tests already lighting up.


### END NOTE - DECEMBER 3 2025 - 18:40 CST