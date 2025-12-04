

````markdown
# ron-app-sdk-jvm

> **Role:** JVM app SDK (Java + Kotlin client for RON-CORE app plane)  
> **Owner:** Stevan White / RustyOnions  
> **Status:** dev preview (interop tests green against Micronode/Macronode gateway)  
> **JVM target:** Java 17+, Kotlin 1.9+  
> **Last reviewed:** 2025-12-03

Badges (future):

[![Build](https://img.shields.io/badge/build-CI-green)]() [![Maven Central](https://img.shields.io/maven-central/v/dev.roncore/ron-app-sdk-jvm-core)]() [![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

---

## 1. Overview

`ron-app-sdk-jvm` is the official JVM SDK for RON-CORE’s **app plane**. It gives Java and Kotlin applications a single, type-safe way to talk to `/app/*` endpoints on a Micronode/Macronode via the gateway (svc-gateway/omnigate).

It is designed for:

- **Android apps** (Kotlin/Java)
- **Server-side JVM** (Spring Boot, Ktor, Micronaut, Quarkus, etc.)
- **CLI / desktop tools** (plain Java/Kotlin)

Key ideas:

- All HTTP calls are normalized to **`/app/*`**.
- Responses are wrapped in a single `AppResponse<T>` type (data + `RonProblem` + HTTP status).
- Config is **env-first** on servers/CLIs (`RON_SDK_*`) and **code-first** on Android/desktop.

### 1.1 Architecture (high-level)

```mermaid
flowchart LR
  subgraph JVM_App[Your JVM app]
    A[Android / Server / CLI] -->|RonClient / Ron| B(ron-app-sdk-jvm)
  end

  B -->|HTTPS /app/*| C[svc-gateway / omnigate]
  C --> D[(RON-CORE Micronode / Macronode)]

  style B fill:#0b7285,stroke:#083344,color:#fff
````

---

## 2. Project layout

This SDK is a **multi-module Gradle project**:

```text
ron-app-sdk-jvm/
├─ core/             # Java core: config, HTTP, AppResponse, RonProblem, retries
├─ kotlin/           # Kotlin facade: Ron, DSL (ronConfig), coroutine wrappers
├─ facets/           # Higher-level facet manifest builder + TOML writer
├─ examples/
│  ├─ java-cli/      # Minimal Java CLI: env-based /app/ping
│  ├─ spring-boot/   # Spring Boot app exposing /ron/ping → RON-CORE
│  ├─ kotlin-ktor/   # Ktor server with health + demo route
│  └─ android-sample/# Android app using the SDK
├─ interop-tests/    # Interop tests against a real Micronode/Macronode gateway
├─ tools/
│  └─ ci/
│     ├─ run-tests.sh    # Core + Kotlin + facets + interop unit tests
│     ├─ run-interop.sh  # Interop tests against running node
│     └─ run-lint.sh     # (placeholder) future lint wiring
├─ README.md
├─ SDK_IDB.MD
├─ SDK_SCHEMA_IDB.MD
└─ SDK_SECURITY.MD
```

**Core packages:**

* `dev.roncore.sdk.*` — Java core (RonClient, AppResponse, RonProblem, RonConfig, etc.)
* `dev.roncore.sdk.kotlin.*` — Kotlin facade and DSL
* `dev.roncore.sdk.facets.*` — Facet manifest builder (TOML)

---

## 3. Getting the SDK into your project

> The SDK is **not yet published** to Maven Central. For now, treat it as a local multi-module Gradle project.

### 3.1 As part of the RustyOnions repository

If you are already working inside the RustyOnions monorepo, the project is under:

```text
sdk/ron-app-sdk-jvm
```

In your **JVM app module** (for example `apps/my-service/build.gradle.kts`):

```kotlin
dependencies {
    implementation(project(":sdk:ron-app-sdk-jvm:core"))
    implementation(project(":sdk:ron-app-sdk-jvm:kotlin"))
    // Optional:
    // implementation(project(":sdk:ron-app-sdk-jvm:facets"))
}
```

### 3.2 As a standalone clone (composite build)

If you clone this SDK somewhere else and want to use it from your app:

1. Clone:

   ```bash
   git clone https://github.com/your-org/RustyOnions.git
   cd RustyOnions/sdk/ron-app-sdk-jvm
   ```

2. In your app’s **`settings.gradle.kts`**, add:

   ```kotlin
   includeBuild("../path/to/RustyOnions/sdk/ron-app-sdk-jvm")
   ```

3. In your app’s module:

   ```kotlin
   dependencies {
       implementation("dev.roncore:core:0.1.0-SNAPSHOT")
       implementation("dev.roncore:kotlin:0.1.0-SNAPSHOT")
   }
   ```

> Exact published coordinates may change when the SDK is pushed to Maven Central; for now, the above is the canonical usage inside the monorepo.

---

## 4. Configuration (RON_SDK_* and builders)

### 4.1 Environment variables (server / CLI)

On servers and CLIs, **env-first** configuration is recommended. `EnvConfigLoader.fromEnv()` and `RonClient.builder().fromEnv()` understand the following:

| Env var                      | Type   | Default                   | Meaning                                              |
| ---------------------------- | ------ | ------------------------- | ---------------------------------------------------- |
| `RON_SDK_GATEWAY_ADDR`       | string | **required** (no default) | Base URL of the gateway, e.g. `https://node:5304`.   |
| `RON_SDK_INSECURE_HTTP`      | bool   | `false`                   | Allow `http://` (dev/test only). Any non-empty = on. |
| `RON_SDK_CONNECT_TIMEOUT_MS` | int    | 5000                      | Connect timeout in ms.                               |
| `RON_SDK_READ_TIMEOUT_MS`    | int    | 30000                     | Read timeout in ms.                                  |
| `RON_SDK_WRITE_TIMEOUT_MS`   | int    | 30000                     | Write timeout in ms.                                 |
| `RON_SDK_OVERALL_TIMEOUT_MS` | int    | 30000                     | Overall timeout in ms (connect+read+write).          |

If a timeout env var is **missing**, the defaults from `RonConfig.Builder` apply:

* `connectTimeout = 5s`
* `readTimeout = 30s`
* `writeTimeout = 30s`
* `overallTimeout = 30s`
* `maxResponseBytes = 5 MiB` (upper bound on JSON body size)

### 4.2 Java: `RonClient.Builder`

Typical server/CLI configuration:

```java
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.config.RonConfig;

RonClient client = RonClient.builder()
        // Start from env if available: RON_SDK_GATEWAY_ADDR, RON_SDK_*_TIMEOUT_MS, RON_SDK_INSECURE_HTTP
        .fromEnv()
        // Optional overrides:
        .baseUrl("https://node.example.com:5304")
        .tokenProvider(() -> currentCapability())  // your TokenProvider
        .build();
```

Key builder methods:

* `fromEnv()` — load `RonConfig` from `RON_SDK_*` env vars.
* `baseUrl(String)` — override the base URL.
* `config(RonConfig)` — provide a fully-built config (bypasses env).
* `tokenProvider(TokenProvider)` — attach a per-request capability/macaron.
* `httpClientAdapter(HttpClientAdapter)` — swap HTTP engine if needed.
* `retryPolicy(RetryPolicy)` — control local retries/backoff.

### 4.3 Kotlin: `ronConfig {}` DSL + `Ron`

Kotlin apps can build config from env + overrides via the DSL:

```kotlin
import dev.roncore.sdk.kotlin.Ron
import dev.roncore.sdk.kotlin.ronConfig

val config = ronConfig {
    // Start from env, then override:
    baseUrl = System.getenv("RON_SDK_GATEWAY_ADDR") ?: "http://127.0.0.1:8090"
    insecureHttp = true        // dev only; never in prod
    connectTimeoutMs = 5_000
    readTimeoutMs = 30_000
    overallTimeoutMs = 30_000
}

val ron = Ron.fromConfig(config)
```

Or directly from env:

```kotlin
val ron = Ron.fromEnv {
    // Optional overrides on top of EnvConfigLoader.fromEnv()
    insecureHttp = true
}
```

`Ron` wraps a `RonClient` internally and implements `AutoCloseable`, so you can use `use { }`:

```kotlin
Ron.fromEnv {
    insecureHttp = true
}.use { ron ->
    // ...
}
```

### 4.4 Android / desktop

On Android/desktop:

* **Do not** rely on environment variables.
* Build `RonConfig` explicitly (e.g., from `BuildConfig`, DI, or secure storage).
* Do not bake long-lived admin capabilities into the client; fetch short-lived tokens from a backend.

---

## 5. Core semantics: URLs, envelopes, errors

### 5.1 Path normalization: `/ping` → `/app/ping`

The SDK always talks to the **app plane**. When you pass a path to `RonClient`:

* If it **already** starts with `/app/`, it is used as-is.
* Otherwise it is normalized to `"/app" + path` (with the leading slash handled).

Examples:

* `client.get("/ping", ...)` → `GET <base>/app/ping`
* `client.get("jobs", ...)` → `GET <base>/app/jobs`
* `client.get("/app/jobs", ...)` → `GET <base>/app/jobs`

### 5.2 `AppResponse<T>`

Every call returns an `AppResponse<T>`:

```java
public final class AppResponse<T> {

    private final T data;
    private final RonProblem problem;
    private final int status;

    public boolean ok() {
        return status >= 200 && status < 300 && problem == null;
    }

    public boolean isOk() { return ok(); }

    public T getData() { ... }
    public RonProblem getProblem() { ... }
    public int getStatus() { ... }
}
```

* On success, `ok()` is `true`, `status` is 2xx, and `data` is non-null.
* On application or transport errors, `ok()` is `false` and `problem` describes what went wrong.

### 5.3 `RonProblem`

`RonProblem` is the canonical problem envelope:

```java
public final class RonProblem {

    private final String code;
    private final String message;
    private final String kind;
    private final String correlationId;
    private final Map<String, Object> details;
    private final Map<String, Object> extra; // unknown fields
}
```

Common usage:

```java
if (!response.ok()) {
    RonProblem p = response.getProblem();
    System.err.println(
        "RON error [" + p.getCode() + "] " + p.getMessage()
        + " (kind=" + p.getKind() + ", correlation_id=" + p.getCorrelationId() + ")"
    );
}
```

Any unexpected JSON fields are captured in `extra` and never break parsing.

---

## 6. Quick start examples

### 6.1 Java CLI (minimal)

This is effectively what `examples/java-cli` does.

```java
package dev.roncore.sdk.examples.java;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonException;

import java.util.Map;

public final class HelloCli {
    public static void main(String[] args) {
        String baseUrl = System.getenv("RON_SDK_GATEWAY_ADDR");
        if (baseUrl == null || baseUrl.isBlank()) {
            System.err.println("ERROR: RON_SDK_GATEWAY_ADDR is not set.");
            System.err.println("Set it to your Micronode/Macronode gateway URL, e.g.:");
            System.err.println("  RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090");
            System.err.println("  RON_SDK_INSECURE_HTTP=1");
            System.exit(1);
        }

        try (RonClient client = RonClient.builder().fromEnv().build()) {
            @SuppressWarnings("unchecked")
            AppResponse<Map<String, Object>> response =
                    client.get("/ping", (Class<Map<String, Object>>) (Class<?>) Map.class);

            System.out.println("HTTP status: " + response.getStatus());
            if (response.ok()) {
                System.out.println("--- OK envelope ---");
                System.out.println(response.getData());
            } else {
                System.out.println("--- Problem envelope ---");
                System.out.println(response.getProblem());
            }
        } catch (RonException ex) {
            ex.printStackTrace(System.err);
            System.exit(2);
        }
    }
}
```

Run (from `sdk/ron-app-sdk-jvm`):

```bash
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:java-cli:run
```

### 6.2 Kotlin server (using `Ron`)

```kotlin
import dev.roncore.sdk.kotlin.Ron
import dev.roncore.sdk.kotlin.ronConfig
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val ronConfig = ronConfig {
        baseUrl = System.getenv("RON_SDK_GATEWAY_ADDR") ?: "http://127.0.0.1:8090"
        insecureHttp = true   // dev only
    }

    Ron.fromConfig(ronConfig).use { ron ->
        val response = ron.getString("/ping")
        if (response.ok()) {
            println("OK (${response.status}): ${response.data}")
        } else {
            println("Problem: ${response.problem}")
        }
    }
}
```

---

## 7. Running tests & examples

All commands below assume:

```bash
cd sdk/ron-app-sdk-jvm
```

### 7.1 Unit tests (core + kotlin + facets)

Quick path:

```bash
./tools/ci/run-tests.sh
```

That script runs:

* `./gradlew projects`
* `./gradlew :core:test`
* `./gradlew :kotlin:test`
* `./gradlew :facets:test`
* `./gradlew :interop-tests:test` (unit-style sanity)
* `./gradlew :examples:java-cli:build`

You can also run individual modules:

```bash
./gradlew :core:test
./gradlew :kotlin:test
./gradlew :facets:test
```

### 7.2 Interop tests against a real Micronode/Macronode

To exercise end-to-end behavior against a **running gateway** (typically `http://127.0.0.1:8090`):

#### 7.2.1 Standard interop sweep

```bash
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./tools/ci/run-interop.sh
```

* If `RON_SDK_GATEWAY_ADDR` is **missing**, the script prints guidance and exits 0.
* If set, it runs all tests in `interop-tests`:

  * `InteropSmokeTest` — basic `/ping` sanity.
  * `HappyPathInteropTest` — strict success envelope (when enabled).
  * `ErrorEnvelopeInteropTest` — strict problem envelope (when enabled).
  * `PaginationInteropTest` — placeholder for future list APIs.

#### 7.2.2 Strict “happy path” + error envelope mode

If your node is wired to:

* Return **success envelopes** on `/app/ping`, and
* Return proper **problem envelopes** for a known error route,

you can enable stricter assertions:

```bash
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
RON_SDK_INTEROP_EXPECT_PING_OK=1 \
RON_SDK_INTEROP_EXPECT_ERROR_ENVELOPE=1 \
./tools/ci/run-interop.sh
```

Env knobs:

* `RON_SDK_INTEROP_EXPECT_PING_OK=1` — `HappyPathInteropTest` asserts 2xx status, `ok() == true`, no `problem`, `data` present.
* `RON_SDK_INTEROP_EXPECT_ERROR_ENVELOPE=1` — `ErrorEnvelopeInteropTest` asserts presence and shape of `RonProblem` for a known error case.

### 7.3 Examples

**Java CLI:**

```bash
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:java-cli:run
```

**Spring Boot:**

```bash
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:spring-boot:bootRun
```

Then:

```bash
curl -v http://127.0.0.1:8080/ron/ping
```

**Kotlin Ktor:**

```bash
RON_SDK_GATEWAY_ADDR=http://127.0.0.1:8090 \
RON_SDK_INSECURE_HTTP=1 \
./gradlew :examples:kotlin-ktor:run
```

Then:

```bash
curl -v http://127.0.0.1:8080/healthz
```

(You can extend this example to add `/ron/ping` that proxies to RON-CORE via `Ron`.)

**Android sample:**

```bash
./gradlew :examples:android-sample:assembleDebug
```

Then install/run via Android Studio or:

```bash
./gradlew :examples:android-sample:installDebug
```

(Requires your local Android SDK configured via `local.properties`.)

---

## 8. Security notes (short version)

> Full details: see `SDK_SECURITY.MD`.

* **No secret logging:** `RonClient` never logs headers or bodies by default; do not add logging interceptors that dump tokens.
* **Tokens are in-memory only:** `TokenProvider` is per-client; the SDK does **not** persist tokens.
* **Android:** assume the device and logs are hostile. Fetch short-lived capabilities from a backend; store them in memory only where possible.
* **Insecure HTTP:** `RON_SDK_INSECURE_HTTP` and `insecureHttp = true` are **dev-only** switches; do not enable in production.

---

## 9. Advanced: facets module

The `facets/` module provides builders for **facet manifests** (TOML) aligned with `SDK_SCHEMA_IDB.MD`. This is useful if your JVM app wants to emit facet definitions that match other SDKs.

Rough shape:

* `FacetDefinition` — core facet metadata (`id`, `name`, `kind`, etc.).
* `FacetLimits` — optional limits & policies.
* `FacetTomlWriter` — writes manifests to TOML.

This is advanced usage and not required for basic HTTP + JSON calls. See `facets/src/test` for working examples.

---

## 10. Troubleshooting

* **`CONFIG_MISSING_BASE_URL` / “No config or baseUrl provided”**

  * `RON_SDK_GATEWAY_ADDR` is missing and you didn’t call `.baseUrl(...)`.
  * Fix by setting the env var or calling `.baseUrl()` on the builder.

* **Connection refused / timeout**

  * Confirm your Micronode/Macronode gateway is running and reachable.
  * Check `RON_SDK_GATEWAY_ADDR` and timeout settings.

* **TLS / certificate errors**

  * For development, you may temporarily use `http://` + `RON_SDK_INSECURE_HTTP=1`.
  * For production, use a valid TLS certificate and **do not** enable insecure HTTP.

* **Unauthorized / forbidden**

  * Check your `TokenProvider` and capability scope.
  * Ensure tokens are valid for the tenant and not expired.

* **High connection count / resource usage**

  * Reuse `RonClient` instances (one per base URL) rather than creating them per request.

---

## 11. Roadmap (short)

Near-term items (subject to change):

* **Metrics & tracing hooks** — examples showing integration with Micrometer/OpenTelemetry via custom `HttpClientAdapter`.
* **Pagination helpers** — use `Page<T>` + `PaginationInteropTest` once canonical list endpoints are stable.
* **Streaming APIs** — flow-based Kotlin APIs via `Streaming` placeholder.
* **Published artifacts** — `ron-app-sdk-jvm-core`, `ron-app-sdk-jvm-kotlin`, `ron-app-sdk-jvm-facets` on Maven Central.

For deep design invariants and security posture, see:

* `SDK_IDB.MD` — invariant-driven design blueprint for this SDK.
* `SDK_SCHEMA_IDB.MD` — schema & facet contracts.
* `SDK_SECURITY.MD` — JVM SDK security checklist.

---

## 12. License & contributing

Dual-licensed under **MIT** or **Apache-2.0**, consistent with the rest of RustyOnions / RON-CORE.

Pull requests and issues are welcome. Please:

* Keep behavior aligned with TS/Python/PHP SDKs (conceptually) while staying idiomatic to Java/Kotlin.
* Run `./tools/ci/run-tests.sh` and, when relevant, `./tools/ci/run-interop.sh` before pushing.
* Update this README and the SDK_* docs when you change public behavior.

```
```
