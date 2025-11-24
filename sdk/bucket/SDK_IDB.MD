````md
# IDB — `ron-app-sdk-php`

### SDK Tier & Profile
- **Tier:** 2  
  - Tier 2: Popular/community (Ruby, PHP, Kotlin, Dart, Elixir, Scala)—balanced, grow over time.
- **Usage profile:**  
  * [ ] Front-end  
  * [x] Backend/facet  
  * [x] CLI/tools  
  * [ ] Dual-use  

This tier gates:  
- §3.4 sandboxing expectations  
- §4.3 idempotency helpers (SHOULD, not MUST)  
- §5.3 benchmark rigor (baseline, not full CI gate)  
- §10.3 property/chaos tests (grow over time)  
- §9.3 CI/support levels (standard CI: lint + unit + basic integration)

---

### 0) RO:WHAT / WHY / INTERACTS

**RO:WHAT —**  
`ron-app-sdk-php` is the **PHP app SDK for RON-CORE’s app plane**, providing a small, idiomatic PHP client that talks to `/app/*` via `svc-gateway` and omnigate. It wraps HTTP, error envelopes, and pagination into a simple `RonClient` (plus config and helpers) for PHP backends and CLI tools.

**RO:WHY —**

* PHP still powers a massive share of the web: Laravel, Symfony, WordPress, custom apps, legacy LAMP stacks, and shared hosting.
* Many teams that would benefit from Micronode/Macronode are currently on PHP + MySQL; this SDK gives them a **gentle path** onto RON-CORE:
  * Laravel/Symfony services calling into RON-CORE as their data/infra layer.
  * Existing PHP apps offloading heavy work into RON-CORE facets.
  * CLI tools (maintenance scripts, batch jobs, migrations) using the same client.
* It helps ship apps faster on RON-CORE by:
  * Hiding raw HTTP/Problem-json mapping behind a small, predictable client.
  * Providing consistent auth / timeouts / pagination behavior that matches TS/Python SDKs.
  * Giving framework-friendly patterns (service container binding, config-from-env) so devs can drop it into Laravel/Symfony with minimal glue.

**RO:INTERACTS —**

Core RON-CORE components this SDK talks to:

* `svc-gateway` — **primary** endpoint: `/app/*` surface for app calls. The SDK **does not** talk directly to `svc-storage` / `svc-index` etc.; all app traffic flows through `/app/*`.
* `svc-edge` — potentially fronting `svc-gateway` in some deployments (CDN/edge).
* `svc-admin` — for health/readiness checks during integration tests or ops tooling.
* `svc-registry` — optional future helpers for looking up nodes/facets.
* `svc-passport` / `ron-auth` — **indirectly** through tokens/macaroons provisioned elsewhere; SDK attaches them as headers.
* `svc-storage` / `svc-index` — indirectly via app-plane endpoints that expose storage/index ops.

---

### 1) Scope & Target Profiles

**1.1 Supported usage modes**  

* [ ] Front-end only (browser / mobile)  
* [x] Backend / facets only  
* [x] CLI / tools  
* [ ] Dual-use (browser + Node / server)  

**Explanation**

* **Backend / facets**:  
  * Used inside PHP web apps (Laravel/Symfony/Slim) to call RON-CORE via `svc-gateway`.  
  * Used inside PHP-based app facets running behind omnigate (if a deployment chooses PHP services as upstreams).
* **CLI/tools**:  
  * Long-running workers (queue consumers, job runners).  
  * Maintenance/ops scripts (re-index, snapshot, data migrations).

We **do not** target browser/mobile directly with PHP; any browser usage should go through TS SDKs or plain HTTP.

---

**1.2 Supported node profiles**

* [x] Micronode  
* [x] Macronode  
* [x] Both (default)  

**Profile-specific assumptions**

* Micronode may run in amnesia/RAM-only mode; docs must **warn** that:
  * Any PHP app using persistent tokens/cookies must not assume node persistence.
  * “Admin” APIs may not be enabled on Micronode; SDK should not rely on them.
* Macronode adds:
  * More admin endpoints.
  * Multi-tenant deployments; docs should emphasize tenant-scoped tokens for multi-tenant apps and that some admin APIs may only exist on macronode and should be guarded by configuration/feature flags in the SDK helpers.

---

**1.3 Feature scope (MVP vs future)**

**MVP:**

* [x] Auth (attach macaroon / bearer token)  
* [x] Basic request/response calls (`get`, `post`, `put`, `delete`, `request`)  
* [ ] Streaming (SSE/WebSocket) subscribe — **not in MVP**  
* [x] Error handling (idiomatic PHP exceptions + Problem value objects)  
* [x] Minimal logging / tracing hooks (PSR-3 compatible hooks)  

**Future:**

* [x] Schema-generated DTOs from `ron-proto` (see §11) — **handwritten initially, codegen later**  
* [x] Auto-reconnect & backoff for streaming (CLI workers / long-lived connections)  
* [x] Built-in metrics hooks  
* [x] Facet lifecycle helpers (register, heartbeat, etc.)  
* [x] Higher-level convenience APIs (domain-specific helpers, e.g., “simple KV”, “events”, etc.)  
* [x] Event publishing (`emitEvent`) and streaming subscriptions (`subscribe`) as fully supported APIs.

---

### 2) API Design & DX Basics

**2.1 Primary abstraction**

* Main entrypoint: `RonClient` class.  
* Config via a `ClientConfig` value object or associative array (`RonClient::fromEnv()` helper).

Minimal “hello world” example (copy-paste runnable):

```php
<?php

require __DIR__ . '/vendor/autoload.php';

use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\RonClient;
use Ron\AppSdkPhp\RonProblemException;
use Ron\AppSdkPhp\RonNetworkException;

$config = new ClientConfig([
    'baseUrl'            => getenv('RON_SDK_GATEWAY_ADDR') ?: 'http://localhost:8080',
    'allowInsecureHttp'  => true, // only for local dev!
    'token'              => null, // or 'macaroon/bearer token string'
]);

$client = new RonClient($config);

try {
    $response = $client->get('/app/hello');
    var_dump($response->json()); // array decoded from JSON
} catch (RonProblemException $e) {
    // HTTP 4xx/5xx with Problem Details
    error_log('RON problem: ' . $e->getProblem()->getTitle());
} catch (RonNetworkException $e) {
    error_log('RON network error: ' . $e->getMessage());
} finally {
    $client->close();
}
````

Goal: A typical PHP dev sees this and immediately understands how to wire the client and handle errors.

---

**2.2 Core operations**

**MVP operations:**

* `__construct(ClientConfig|array $config)` — configure client (base URL, timeouts, token, etc.).
* `RonClient::fromEnv()` — convenience factory using env vars (`RON_SDK_*`).
* `get(string $path, array $queryParams = []): Response`
* `post(string $path, array|JsonSerializable $body = null): Response`
* `put(string $path, array|JsonSerializable $body = null): Response`
* `delete(string $path, array $queryParams = []): Response`
* `request(string $method, string $path, array $options = []): Response` — low-level escape hatch.
* `close(): void` — release any persistent resources (e.g., underlying client if needed).

**Future operations (non-MVP):**

* `emitEvent(string $topic, array $payload): void` — thin wrapper around `POST /app/events/{topic}` (future).
* `subscribe(string $path, callable $onEvent): void` — SSE subscription (CLI/worker only, future).

Response abstraction:

* `Response` value object exposing:

  * `getStatusCode(): int`
  * `json(): array`
  * `getHeaders(): array`
  * `getRawBody(): string`

`Response` objects are **immutable** to avoid accidental aliasing/mutation across different layers.

---

**2.3 Error model**

* **Representation**:

  * Base `RonException` extends `\RuntimeException`.
  * Subclasses:

    * `RonNetworkException` — connection errors, DNS failures, timeouts.
    * `RonTimeoutException` — explicit timeout classification.
    * `RonAuthException` — 401/403 with problem details.
    * `RonConfigException` — invalid config/env (e.g., malformed URL, negative timeouts).
    * `RonProblemException` — wraps RFC7807-style problem responses.
* A `Problem` value object mirrors the canonical error envelope:

  * `type`, `title`, `status`, `detail`, `instance`, plus `extensions`.

**Mapping**

* Non-2xx responses with `application/problem+json` → `RonProblemException(Problem)`.
* Non-2xx responses with unknown body → `RonNetworkException` or generic `RonException`.
* Network failures (DNS, TCP, TLS) → `RonNetworkException`.
* Timeouts → `RonTimeoutException`.

**Idiomatic PHP error handling**

* Apps `try/catch` specific `Ron*Exception` types.
* For frameworks, we can provide middleware to map `RonProblemException` into framework-native HTTP responses or logs.

---

**2.4 Configuration**

Configurable fields:

* `baseUrl` — gateway/base URL (e.g., `https://gateway.example.com`).
* `allowInsecureHttp` — bool, **default false**, must be explicit for `http://`.
* `token` — string macaroon/bearer token.
* `tokenProvider` — callable `fn(): string` for token rotation.
* Timeouts:

  * `connectTimeoutMs`
  * `readTimeoutMs`
  * `writeTimeoutMs`
  * `overallTimeoutMs`
* Retries/backoff (optional):

  * `maxRetries`
  * `retryBackoffMs`
* Logging/tracing:

  * Optional PSR-3 logger or callback hooks.

How config is loaded:

* **Env vars** (canonical names from shared schema):

  * `RON_SDK_GATEWAY_ADDR`
  * `RON_SDK_OVERALL_TIMEOUT_MS`
  * `RON_SDK_CONNECT_TIMEOUT_MS`
  * `RON_SDK_READ_TIMEOUT_MS`
  * `RON_SDK_WRITE_TIMEOUT_MS`
* `ClientConfig` from array/constructor.
* Optional config file (e.g., `ron_sdk.php` or `.env` via existing frameworks).

**Fail-closed behavior:**

* Config values are validated eagerly:

  * Malformed URLs, negative or zero timeouts, invalid retry counts, etc. cause a **`RonConfigException`** on construction (`ClientConfig`/`RonClient`) rather than silently falling back.
  * Insecure HTTP (`http://`) is rejected unless `allowInsecureHttp=true` is explicitly set.

---

**2.5 Language-idiomatic patterns**

* Async model:

  * PHP is predominantly sync/blocking; core API is synchronous.
  * For async/event-loop environments (Swoole, ReactPHP), a future extension may provide an async client in a **separate package** (e.g., `ron-app-sdk-php-async`) to avoid surprising behavior or oversubscription in the main sync SDK.
* Naming conventions:

  * PSR-4 namespacing: `Ron\AppSdkPhp\RonClient`, `Ron\AppSdkPhp\ClientConfig`.
  * Methods: `camelCase`, classes: `StudlyCaps`.
* Integration patterns:

  * Laravel:

    * Service provider that binds a singleton `RonClient` into the container.
    * Config via `config/ron.php` mapping env vars → `ClientConfig`.
  * Symfony:

    * Service definition in `services.yaml`.
    * Environment-overrideable parameters.

Rule: it should “feel like” a normal PHP HTTP client library, not a Rust transplant.

---

### 3) Security & Capabilities (SEC)

**3.1 Auth flow**

* SDK accepts credentials via:

  * `token` option in `ClientConfig`: macaroon/bearer token string.
  * `tokenProvider` callable for dynamic token fetching/rotation.
* Tokens are attached as:

  * `Authorization: Bearer <token>` or a dedicated `X-Ron-Cap` header (final wire decision must match Rust/TS SDKs).
* Tokens are stored **only in memory** inside `ClientConfig`/`RonClient`; never persisted by the SDK.

**Secrets hygiene**

* No secrets should be logged:

  * Logger hooks must **redact** headers like `Authorization`, `Cookie`, and any `X-Ron-*` caps.
  * Debug dumping of requests/responses must mask tokens.
* Future improvement (optional, not required for MVP):

  * If practical, clear token strings on `close()` where the PHP runtime and extensions (e.g., sodium) provide facilities.

---

**3.2 Capability boundaries**

* Encourage **scoped tokens**:

  * Documentation shows examples like `bucket:todo:read/write` instead of `*`.
* Helpers (future):

  * `CapRequestBuilder` helper to describe desired scopes; actual issuance still happens via `svc-passport`/`ron-auth`.
* Clear docs/examples for:

  * Tenant-scoped tokens in multi-tenant environments.
  * Read-only vs admin tokens.
  * Short-lived tokens and rotation patterns using `tokenProvider`.

---

**3.3 Secure defaults**

Checklist:

* [x] TLS on by default; `https://` required unless `allowInsecureHttp=true`.
* [x] Credentials never logged (mask in any debug logs).
* [x] Reasonable timeouts; no infinite hangs.
* [x] Debug-only features (wire dumps, extra logs) are opt-in and still scrub secrets.
* [ ] PQ-ready toggle (`pq_mode: off|hybrid`) — **reserved**, implemented later once RON-CORE PQ profile and PHP TLS stack support are stable.

---

**3.4 Sandboxing for Facets/UI**

* Even though PHP is mostly backend, we **still** require:

  * No global singletons that hold caps; `RonClient` instances are explicit objects.
  * SDK is safe to create in isolated processes (FPM workers, CLI workers).
* Docs should include patterns for:

  * Spawning separate PHP workers that each create their own `RonClient` with scoped tokens.
  * Avoid sharing tokens via global/static variables.
* Tier 2: we focus on **docs + patterns**, not heavy sandbox helpers.

---

### 4) Reliability & Resilience (RES)

**4.1 Connection behavior**

* Default behavior:

  * Block on HTTP calls.
  * On network error/timeout, throw `RonNetworkException` / `RonTimeoutException`.
* Optional auto-retry:

  * For **idempotent** methods (`GET`, `HEAD`) we can provide an opt-in retry with exponential backoff.
  * `POST/PUT` retries must be explicit and documented with idempotency keys (see §4.3).

Failure handling:

* DNS failures, TLS errors → `RonNetworkException`.
* Timeouts → `RonTimeoutException`.

Note: in Micronode **amnesia mode**, retries may hit nodes that have lost prior state; docs must warn that app-level semantics (e.g., at-least-once writes) depend on the node’s configuration, not just the SDK.

---

**4.2 Streaming resilience**

* Not in MVP for typical request/response PHP apps.
* Future:

  * SSE-based streaming subscriptions for CLI/worker processes.
  * Simple backoff/reconnect loop:

    * On disconnect, call `onDisconnect` hook, then reconnect with backoff.
    * No guarantee beyond **at-least-once** delivery where the server supports replay; docs must be explicit here.

---

**4.3 Backoff & rate limiting**

* SDK **SHOULD** include:

  * Basic client-side throttling to avoid busy loops in misconfigured code:

    * e.g., minimum delay between retries, simple token bucket for repeated calls in a loop.
* Recommendations:

  * Reuse a single `RonClient` per PHP worker / process; don’t instantiate per request in hot loops.
  * For frameworks, ideally bind `RonClient` as a service and share where safe.
* Idempotency helpers:

  * Provide a function `IdempotencyKey::generate()` (UUID v4 or cryptographically random).
  * Document pattern for writes:

    * `X-Idempotency-Key` header set by the SDK when requested.
  * As a Tier 2 SDK, **idempotency helpers are strongly recommended** and should be included in the initial implementation, but deep property tests for idempotency can arrive later.

---

### 5) Performance (PERF)

**5.1 Expected usage profile**

* Typical request rates:

  * **Low to medium** per PHP worker (tens to low hundreds RPS).
  * Aggregate across many workers/containers, RON-CORE will still see high RPS.
* Payload sizes:

  * Usually small JSON payloads (< 64 KiB).
  * Occasional medium payloads for bulk operations or search results.

---

**5.2 Implementation choices**

* HTTP client library:

  * Implementation **should** be based on a **PSR-18** compatible client; recommended stack:

    * Guzzle (`guzzlehttp/guzzle`) or Symfony HTTP Client.
  * Use PSR-7 for requests/responses internally where convenient.
* Serialization:

  * JSON baseline via `json_encode`/`json_decode`.
  * DTO hydration into PHP arrays / simple value objects.
  * Future support for CBOR/protobuf as RON-CORE exposure grows.

---

**5.3 Benchmarks**

* Baseline benchmarks (simple scripts run under CLI):

  * Single call latency vs raw Guzzle/Symfony HTTP client.
  * Sustained RPS for a simple `/app/hello` request.
* Hardware + env:

  * Document results on a dev laptop and on a modest server (e.g., 4 vCPU).

**Performance expectation (Tier 2, non-hard gate):**

* For a simple `/app/hello` call, the SDK should add at most a small overhead vs the underlying HTTP client:

  * Target: **<5% additional median latency** on a typical dev machine, measured in simple micro-benchmarks.
  * This is an **aspirational guideline**, not a strict CI gate for Tier 2.

* Rigor for Tier 2:

  * Include **benchmark scripts** and recommended settings, but not required to run in CI initially.
  * Over time, we can wire a slim benchmark into CI on a small sample.

---

### 6) Economics (ECON)

**6.1 Dev productivity**

* Remove boilerplate compared to raw HTTP:

  * No need to remember `/app` prefix, headers, timeouts, error mapping.
  * Built-in parsing of Problem envelopes and mapping to exceptions.
* Minimal LOC for:

  * Calling an API: ~5–10 lines.
  * Handling auth: config + env.
  * Paging through results using a helper.

---

**6.2 Operational cost**

* Recommended:

  * One `RonClient` per process (or per worker) to minimize overhead.
  * For PHP-FPM, create a client in a container/service and inject it into controllers/services.
* No resource leaks:

  * Ensure any persistent connections are closed on `close()` and not left dangling.
* Document any tradeoffs:

  * Long-lived PHP processes (Swoole, RoadRunner) can reuse the same client; FPM may recreate clients per request but this is acceptable for modest RPS.

---

### 7) Developer Experience (DX) — Onboarding & Familiarity

**7.1 “No manual” quickstart**

Three README snippets:

1. **Hello world call**

```php
$config = Ron\AppSdkPhp\ClientConfig::fromEnv();
$client = new Ron\AppSdkPhp\RonClient($config);

$response = $client->get('/app/hello');
var_dump($response->json());

$client->close();
```

2. **Subscribe to a stream** (future, CLI worker with SSE)

```php
$client = new Ron\AppSdkPhp\RonClient(ClientConfig::fromEnv());

$client->subscribe('/app/events', function (Ron\AppSdkPhp\Event $event): void {
    echo "EVENT: " . json_encode($event->getData()) . PHP_EOL;
});

// Blocks until interrupted
```

3. **Facet-style worker loop** (backend job)

```php
$config = ClientConfig::fromEnv();
$client = new RonClient($config);

while (true) {
    $jobs = $client->get('/app/jobs/next')->json();

    foreach ($jobs['items'] as $job) {
        // process job...
        $client->post('/app/jobs/' . $job['id'] . '/ack', ['status' => 'done']);
    }

    sleep(1); // simple backoff
}
```

Each snippet should be <=20 lines and easy to paste into a Laravel command or standalone script.

---

**7.2 Framework adapters / helpers**

Planned adapters:

* [x] Laravel:

  * Service provider to bind `RonClient` into the container (`app()->make(RonClient::class)`).
  * Config file `config/ron.php` mapping env → `ClientConfig`.
* [x] Symfony:

  * Service definition with constructor-injected `ClientConfig`.
* [ ] CLI helpers:

  * A `bin/ron-sdk-php-example` script showing common operations.
* [x] Logging/tracing:

  * PSR-3 logger integration; optional injection via `ClientConfig`.

---

**7.3 Language-idiomatic docs**

* Code samples match common PHP style (PSR-12).
* Docs show:

  * How to install via Composer.
  * How to configure using env variables.
  * How to integrate with Laravel/Symfony.
* No Rust-specific terminology leaks into the public PHP API; we talk in terms of “client”, “request”, “response”, “exceptions”.

---

### 8) Extensibility & Plugins

**8.1 Extension points**

* Request middleware/interceptors:

  * Allow registering callbacks to modify outgoing requests:

    * `beforeRequest(Request $request): Request`
    * `afterResponse(Response $response): Response`
* Event hooks:

  * `onConnect`, `onDisconnect`, `onError`, `onRetry`.
* Pluggable transports (future):

  * Potential support for alt transports (e.g., HTTP/2, gRPC) if/when RON-CORE exposes them.

MVP:

* [x] Basic “request middleware” hooks (for logging, headers, tracing)

Future:

* [x] More structured middleware stack
* [x] Custom transports if needed

---

**8.2 UI widgets / components**

* For PHP, UI typically lives in HTML/JS; **UI helpers** belong with TS or frontend stacks.
* `ron-app-sdk-php` stays backend-only; no PHP UI widgets.

---

**8.3 Plugin model (future)**

* 3rd-party extensions:

  * Should register as middleware or helpers, not modify core behavior.
  * Example: `ron-app-sdk-php-monolog` package adding opinionated logging.
* Keep core light; heavy/language-specific plugins (framework bundles, advanced tracing) live in separate Composer packages.
* Avoid dynamic plugin loading that pulls in arbitrary code without explicit install.

---

### 9) Governance & Versioning (GOV)

**9.1 Version scheme**

* SDK versioning:

  * `0.x` tracks `RON-CORE protocol v0.x`.
  * When RON-CORE protocol bumps in a breaking way, SDK major version must also bump.
* Breaking changes:

  * Remove or change public method signatures → major bump.
  * Add new optional config fields or helpers → minor bump.
  * Deprecations:

    * Mark deprecated methods with `@deprecated` tags and document replacement.
    * Aim for at least **2 minor versions** before removal.

---

**9.2 Compatibility matrix**

| SDK version | Protocol version | Min node version | Notes         |
| ----------- | ---------------- | ---------------- | ------------- |
| 0.1.x       | v0.1             | TBD              | MVP app-plane |

(TBD node version will be updated once Micronode/Macronode app-plane contracts are frozen.)

---

**9.3 Support expectations**

* [ ] Tier 1 (core, always up to date)
* [x] Tier 2 (popular but not core)
* [ ] Tier 3 (niche / experimental, community-heavy)

CI expectations:

* Tier 2:

  * Standard CI:

    * Composer lint (`phpcs`/`phpstan`).
    * Unit tests (PHPUnit).
    * Basic integration tests against a local node (when available).
  * Over time, we can add property tests and limited chaos tests (network flakiness).

Additionally:

* Security fixes should be backported to at least the last **2 minor versions** where practical.
* Community contributions are encouraged; governance docs should specify how to propose breaking changes.

---

### 10) Testing & QA

**10.1 Test types**

* Unit tests:

  * Config parsing from env (including fail-closed behavior for invalid values).
  * Error mapping from HTTP responses to exceptions.
  * Serialization/deserialization helpers.
* Integration tests:

  * Run against a local Micronode/Macronode (or test gateway).
  * Basic `/app/hello` roundtrip.
  * Pagination roundtrip.
* E2E samples:

  * Example Laravel command that calls `/app/hello` and prints output.
  * Simple CLI script for streaming events (future).

---

**10.2 CI requirements**

* Lint/format:

  * `phpcs` / `php-cs-fixer` / `phpstan` (exact stack to be chosen).
* Unit tests:

  * PHPUnit suite run in CI.
* Integration tests:

  * Optional job that spins up a test node (when available).
* Build/dist:

  * Verify composer metadata, autoloading, and package integrity.

---

**10.3 Property/Chaos Tests**

* Property-based:

  * MVP SHOULD include at least **one** property-style test:

    * Fuzz `application/problem+json` payloads within the canonical schema and assert they always map to a valid `Problem` object or a well-defined `RonProblemException` shape.
  * Additional property tests (e.g., idempotency key format/uniqueness) can be added over time.
* Chaos:

  * Simulate network flakes:

    * Use Toxiproxy or equivalent to test timeouts/retries.
* Tier 2:

  * Start with scripts/manual runs; later fold minimal variants into CI (non-blocking).

---

### 11) Interop & Schema (ron-proto, HTTP, Facets)

**RO:WHAT —**
Ensure `ron-app-sdk-php` shares the same **DTOs, error envelopes, pagination shapes, and facet semantics** as the Rust/TS/Python SDKs, so that **same capability + same request ⇒ same behavior**, regardless of language.

**RO:INVARIANTS —**

* DTOs exposed by the PHP SDK are derived from canonical schemas (`ron-proto` + HTTP/OpenAPI).
* No PHP-only re-interpretation of schema (field names/types must match).
* Facet manifests (TOML) generated or handled by helpers must follow `SDK_SCHEMA_IDB.MD`.
* Error and pagination envelopes use the same fields and semantics across SDKs.

---

#### 11.1 Canonical Sources

* `ron-proto` — canonical definitions for:

  * App-plane envelopes, storage/index DTOs, auth payloads.
* HTTP/OpenAPI for gateway/omnigate:

  * `/app/*` routes, error/problem envelopes, pagination.
* `SDK_SCHEMA_IDB.MD`:

  * Cross-language type mapping and facet manifest schema, including env var names and limits.

---

#### 11.2 SDK Requirements (PHP, Tier 2)

* DTOs may be **handwritten** initially but must:

  * Match JSON field names and types from canonical schemas.
  * Be covered by interop tests that compare actual responses from a test node to expected shapes.
* Error envelope:

  * Implement canonical Problem object; test parity with TS/Python fixtures.
* Pagination:

  * Use same envelope (e.g., `{ items, next_cursor, prev_cursor }` or equivalent) as defined in schema.

---

#### 11.3 Type Mapping & Drift Avoidance

* Mapping tricky types:

  * `u64`:

    * On 64-bit PHP builds: may be represented as native `int`, but SDK must still accept/emit string representations where schemas allow, to ensure interoperability.
    * On 32-bit PHP builds: **must** be represented as strings to avoid overflow; tests must verify lossless roundtrip for any identifier/offset that is modelled as `u64` in the canonical schema.
  * Timestamps:

    * ISO8601 strings mapped to `\DateTimeImmutable` where convenient, or left as strings with documented semantics; roundtrip tests ensure no precision loss beyond what the canonical schema allows.
  * Binary data:

    * Represented as base64-encoded strings where applicable; document this in DTOs.
  * Enums:

    * Exposed as string constants in PHP (e.g., class constants), matching canonical enum string values.

* CI:

  * Add a schema drift check:

    * For now, compare sample fixtures (JSON) against PHP DTO hydration and back.
    * Long term, consider using JSON schema or codegen scripts to regenerate DTOs and diff.

---

#### 11.4 Facet Interop

* PHP SDK may later provide helpers for generating facet manifests (TOML), but:

  * Must adhere to the unified facet schema (kind, routes, limits, integrity).
  * Must not change the routing semantics:

    * `/facets/{facet_id}{route.path}`.
* Proxy facets:

  * If helpers are added to configure PHP upstreams, all upstream auth/cap handling must be explicit and secure (no implicit caps).

---

### 12) Future Extensions & Nice-to-Haves

* Post-M3:

  * Local mock server in PHP for offline dev (simulate gateway responses).
* Post-M4:

  * CLI scaffold tool (`ron-app-sdk-php new:app`) that generates a minimal Laravel/Symfony integration sample.
* Post-M5:

  * Domain-specific helpers (e.g., streaming event helpers, simple KV wrapper).
* Post-M3/M4:

  * Async client variant for Swoole/ReactPHP in a **separate package** (`ron-app-sdk-php-async`), to avoid surprising blocking behavior changes in the main sync SDK.
* Post-M3:

  * More advanced interop tooling (DTO codegen from JSON schema/OpenAPI).

---

### 13) Platform-Specific Gotchas

* PHP-FPM:

  * Short-lived processes; repeated client creation is acceptable but documented as less efficient than sharing in long-lived environments.
* Long-lived workers (Swoole/RoadRunner):

  * Be careful to rotate tokens and refresh config; no stale caps.
* TLS:

  * Ensure proper CA bundles; document `openssl` and `curl` dependencies.
* 32-bit PHP:

  * `u64`-ish values must be represented as strings; document this as a known limitation and ensure tests cover it.
* Composer / extensions:

  * Document minimum supported PHP versions (e.g., 8.1+), and required extensions such as `ext-json`, `ext-curl`, and `ext-openssl`.
* Micronode amnesia mode:

  * Warn in docs that node state may not persist across restarts; PHP apps should treat RON-CORE as ephemeral unless otherwise configured and avoid assuming persistent state solely via the SDK.

```
```
