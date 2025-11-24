````md
# ron-app-sdk-php

> **Role:** PHP App SDK (client library for RON-CORE app plane)  
> **Owner:** Stevan White (RustyOnions / RON-CORE)  
> **Status:** draft (pre-beta)  
> **PHP target:** 8.1+  
> **Last reviewed:** 2025-11-23

Badges (future):  
[![Build](https://img.shields.io/badge/build-CI-green)]() [![Packagist](https://img.shields.io/packagist/v/ron-core/ron-app-sdk-php)]() [![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

---

## 1) Overview

**What it is (one paragraph):**  
`ron-app-sdk-php` is the official PHP app SDK for RON-CORE’s **app plane**. It provides an idiomatic `RonClient` that talks to RON-CORE via `svc-gateway`’s `/app/*` surface, handling HTTPS, timeouts, retries, structured error envelopes, and pagination for you. It targets **PHP backends and CLI workers** (Laravel, Symfony, Slim, custom apps), giving existing PHP/LAMP stacks a gentle migration path onto Micronode/Macronode without having to learn Rust or rewrite everything.

**How it fits (RustyOnions topology):**  

- **Pillar:** App plane / Developer Experience (SDKs)  
- **Upstream callers:** PHP web apps, CLI tools, job runners (Laravel commands, Symfony console, cron scripts)  
- **Downstream deps:**  
  - `svc-gateway` (primary, `/app/*`)  
  - `svc-edge` (optional, if fronting gateway)  
  - `svc-admin` (optional, for health checks in tests/tools)  
- **Data it touches:** In-memory PHP objects, HTTP/JSON over network; **no direct DB** access (all storage/indexing flows through RON-CORE).  
- **Security boundary:**  
  - Handles **capability tokens/macaroons** and auth headers.  
  - Must never log secrets.  
  - Assumes trusted server/worker environment (not browser), but still enforces sane timeouts & bounded retries. :contentReference[oaicite:0]{index=0}  

### 1.1 High-Level Architecture (Mermaid REQUIRED)

```mermaid
flowchart LR
  subgraph PHP App / Worker
    A[Caller: Laravel/Symfony CLI/HTTP] -->|calls| B(RonClient)
  end

  B -->|HTTPS /app/*| C[svc-gateway]
  C -->|OAP/HTTP| D[omnigate]
  D -->|RPC/OAP| E[RON-CORE services]

  B -->|Logs/metrics hooks| F[[App Observability (PSR-3, OpenTelemetry)]]

  style B fill:#0b7285,stroke:#083344,color:#fff
````

*The SDK is a thin client: all authority and data live in RON-CORE; PHP apps only see the app-plane HTTP surface.*

---

## 2) Responsibilities & Boundaries

**MUST do (core responsibilities):**

* [x] Provide an idiomatic `RonClient` for PHP backends and CLI tools.
* [x] Attach capability/bearer tokens securely (no logging, no query-string caps).
* [x] Implement canonical error & pagination envelopes shared with other SDKs. 
* [x] Enforce bounded network calls (timeouts, optional bounded retries).
* [x] Expose configuration via shared `RON_SDK_*` env vars with fail-closed validation. 

**MUST NOT do (anti-scope / boundaries):**

* [x] MUST NOT talk directly to `svc-storage`, `svc-index`, or other core services—**all** app traffic goes through `svc-gateway`/omnigate (`/app/*`).
* [x] MUST NOT store caps/macaroons on disk by default (no built-in persistence to files, DB, cookies).
* [x] MUST NOT auto-retry non-idempotent writes without explicit idempotency keys and user opt-in.
* [x] MUST NOT ship browser-facing code or UI widgets (PHP SDK is backend-only; frontends use TS SDK).

**Acceptance Gates (PROOF you did it):**

* [ ] PHPUnit unit tests for:

  * Config/env parsing (including invalid/env-missing cases).
  * Error envelope mapping (`Problem` / canonical error JSON → exceptions).
  * Pagination helper behavior.
* [ ] Interop tests against a local test node (Micronode/Macronode or mock gateway):

  * `/app/hello` round-trip.
  * At least one paginated endpoint.
* [ ] No secrets in logs:

  * Tests/assertions that debug logging masks `Authorization`/`X-Ron-*` headers.
* [ ] Benchmarks for simple `/app/hello` call:

  * Measured overhead vs raw Guzzle/Symfony HTTP client.
  * Documented in `docs/benchmarks.md`.

---

## 3) Public Interfaces

> This is a **PHP SDK**, not a Rust crate. This section documents the main PHP API surface and how it’s typically used.

### 3.1 PHP API (library)

Key namespaces/types (planned):

* `Ron\AppSdkPhp\ClientConfig` — configuration value object:

  * `fromEnv()` factory (reads `RON_SDK_*` env vars). 
  * Validates base URL, timeouts, retry settings; fail-closed on invalid config.
* `Ron\AppSdkPhp\RonClient` — main HTTP client:

  * `__construct(ClientConfig|array $config)`
  * `static fromEnv(): RonClient`
  * `get(string $path, array $query = []): Response`
  * `post(string $path, array|JsonSerializable|null $body = null): Response`
  * `put(string $path, array|JsonSerializable|null $body = null): Response`
  * `delete(string $path, array $query = []): Response`
  * `request(string $method, string $path, array $options = []): Response`
  * `close(): void`
* `Ron\AppSdkPhp\Response` — immutable response wrapper:

  * `getStatusCode(): int`
  * `getHeaders(): array`
  * `json(): array`
  * `getRawBody(): string`
* Exceptions:

  * `Ron\AppSdkPhp\Exception\RonException` (base)
  * `Ron\AppSdkPhp\Exception\RonNetworkException`
  * `Ron\AppSdkPhp\Exception\RonTimeoutException`
  * `Ron\AppSdkPhp\Exception\RonAuthException`
  * `Ron\AppSdkPhp\Exception\RonConfigException`
  * `Ron\AppSdkPhp\Exception\RonProblemException`
* Error envelope DTO:

  * `Ron\AppSdkPhp\Problem` — wraps canonical error / RFC7807. 

Minimal example:

```php
<?php

use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\RonClient;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\Exception\RonNetworkException;

$config = ClientConfig::fromEnv(); // falls back to sane defaults for dev
$client = new RonClient($config);

try {
    $response = $client->get('/app/hello');
    $data = $response->json();
    echo "Hello from RON-CORE: " . json_encode($data) . PHP_EOL;
} catch (RonProblemException $e) {
    error_log('[RON] Problem: ' . $e->getProblem()->getMessage());
} catch (RonNetworkException $e) {
    error_log('[RON] Network error: ' . $e->getMessage());
} finally {
    $client->close();
}
```

### 3.2 Framework integration (Laravel / Symfony)

**Laravel (planned recipe):**

* Service provider registers `RonClient`:

```php
$this->app->singleton(RonClient::class, function ($app) {
    $config = new ClientConfig([
        'baseUrl' => config('ron.base_url'),
        'token'   => config('ron.token'),
    ]);

    return new RonClient($config);
});
```

* Usage in controllers/services:

```php
public function show(RonClient $ron)
{
    $rsp = $ron->get('/app/users/current')->json();
    return response()->json($rsp);
}
```

**Symfony (planned recipe):**

* Define `RonClient` as a service in `services.yaml` with env-based config.

### 3.3 HTTP Surface (what the SDK is wrapping)

* **Base URL:** `RON_SDK_GATEWAY_ADDR` (e.g., `https://gateway.example.com`) 
* **Primary path:** `/app/*` exposed by `svc-gateway`/omnigate.
* Example gateway endpoints (subject to final app-plane contract):

  * `GET /app/hello`
  * `GET /app/resources?page_token=...`
  * `POST /app/items`
* The SDK only wraps these HTTP endpoints; it does **not** expose lower-level OAP or internal admin APIs by default.

### 3.4 CLI / tools

Typical usage in a PHP CLI script:

```bash
php examples/hello.php
```

Example script:

```php
#!/usr/bin/env php
<?php

require __DIR__ . '/../vendor/autoload.php';

use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\RonClient;

$config = ClientConfig::fromEnv();
$client = new RonClient($config);

$rsp = $client->get('/app/hello')->json();
echo json_encode($rsp, JSON_PRETTY_PRINT) . PHP_EOL;

$client->close();
```

---

## 4) Configuration

> The SDK aligns with the **shared env schema** used by other SDKs. 

| Variable                     | Type   | Default                 | Description                                  |
| ---------------------------- | ------ | ----------------------- | -------------------------------------------- |
| `RON_SDK_GATEWAY_ADDR`       | string | `http://localhost:8080` | App-plane gateway URL (`svc-gateway` / edge) |
| `RON_SDK_OVERALL_TIMEOUT_MS` | int    | `30000`                 | Overall request timeout (ms)                 |
| `RON_SDK_CONNECT_TIMEOUT_MS` | int    | `5000`                  | TCP connect timeout (ms)                     |
| `RON_SDK_READ_TIMEOUT_MS`    | int    | `25000`                 | Read timeout (ms)                            |
| `RON_SDK_WRITE_TIMEOUT_MS`   | int    | `25000`                 | Write timeout (ms)                           |
| `RON_SDK_MAX_RETRIES`        | int    | `0`                     | Max retries for idempotent calls             |
| `RON_SDK_RETRY_BACKOFF_MS`   | int    | `100`                   | Base backoff (ms)                            |
| `RON_SDK_TOKEN`              | string | *(none)*                | Capability / bearer token (not required)     |

**Config objects & validation:**

* `ClientConfig::fromEnv()`:

  * Reads env vars above.
  * Validates URLs (MUST be `https://` unless explicitly allowed via config).
  * Validates timeouts & retry counts (must be >0, sane upper bounds).
  * Throws `RonConfigException` on invalid configuration instead of silently continuing.

**Custom config:**

```php
$config = new ClientConfig([
    'baseUrl'           => 'https://gateway.example.com',
    'token'             => getenv('APP_RON_TOKEN') ?: null,
    'allowInsecureHttp' => false,
    'connectTimeoutMs'  => 5000,
    'readTimeoutMs'     => 25000,
    'writeTimeoutMs'    => 25000,
    'overallTimeoutMs'  => 30000,
]);
```

---

## 5) Build, Run, Test

This is a **PHP Composer** package, not a Rust crate.

**Install (for consumers):**

```bash
composer require ron-core/ron-app-sdk-php
```

**Clone & set up (for contributors):**

```bash
git clone https://github.com/RustyOnions/ron-app-sdk-php.git
cd ron-app-sdk-php
composer install
```

**Run examples:**

```bash
RON_SDK_GATEWAY_ADDR=http://localhost:8080 \
php examples/hello.php
```

**Tests (unit/integration):**

```bash
composer test          # runs PHPUnit test suite
composer test:unit     # if split
composer test:int      # integration tests (may require local node)
```

**Lint & static analysis:**

```bash
composer lint          # phpcs / php-cs-fixer
composer analyse       # phpstan / psalm
```

**Security & deps:**

```bash
composer audit         # or `symfony security:check` / local equivalent
```

**Bench (if provided):**

```bash
composer bench         # simple benchmark scripts (CLI)
```

---

## 6) Observability

The SDK itself does not expose HTTP endpoints, but it integrates cleanly with your app’s observability stack.

**Logging:**

* Supports injection of a PSR-3 logger via `ClientConfig` or setter.
* Logs:

  * Request method + path.
  * Status code and latency.
  * Correlation IDs where available.
* Never logs:

  * `Authorization` header.
  * Any header that looks like a capability/token (`X-Ron-*` caps).
  * Raw response bodies that might contain PII or secrets.

**Metrics (future hooks):**

* Common hook points:

  * Before request → increment `requests_total`.
  * After response → observe latency histogram.
  * On error → increment `errors_total{kind}`.

* Recommended metric names (to align with RustyOnions):

  * `ron_sdk_http_requests_total{sdk="php",method,route}`
  * `ron_sdk_http_request_duration_seconds{sdk="php",method,route}`
  * `ron_sdk_errors_total{sdk="php",kind}`
  * `ron_sdk_retries_total{sdk="php",method,route}`

**Tracing:**

* Support passing correlation-ID headers into the SDK (e.g., `X-Request-Id`).
* Forward gateway-provided correlation IDs into logs where available.

---

## 7) Performance & SLOs

| Metric              | Target           | Notes                                 |
| ------------------- | ---------------- | ------------------------------------- |
| P50 SDK overhead    | < 5% vs raw HTTP | For simple `/app/hello` on dev laptop |
| P99 SDK overhead    | < 10%            | Under moderate load                   |
| Start-to-first-call | < 100 ms         | After autoloader warmup               |
| Max RPS per worker  | “HTTP-bound”     | PHP worker is usually the bottleneck  |

> These are **guidelines**, not hard SLOs for Tier 2; benchmark scripts should demonstrate approximate performance and make regressions visible over time. 

---

## 8) Data & Schema

The SDK does not persist data itself, but it must **honor the shared schema profile** across SDKs:

* DTOs for app-plane payloads, errors, and pagination follow `SDK_SCHEMA_IDB.MD`. 
* `i64`/`u64` identifiers and counters are represented as **strings** in PHP DTOs to avoid overflow and drift across 32/64-bit platforms, with convenience accessors where safe.

Example DTO sketch:

```php
final class ResourceSummary
{
    public function __construct(
        private string $id,          // u64 as string
        private string $createdAt,   // ISO8601 timestamp
        private array $meta = [],    // extra / extensions
    ) {}

    public function getId(): string { return $this->id; }
    public function getCreatedAt(): \DateTimeImmutable { return new \DateTimeImmutable($this->createdAt); }
}
```

No migrations are handled at the SDK level; schema evolution is governed by RON-CORE’s protocol/versioning and reflected in new DTOs over time.

---

## 9) Security & Privacy

* **Threat model (high level):**

  * PHP apps/CLI tools are in a **trusted server environment** (not browser), but:

    * Network traffic and nodes may be untrusted or misconfigured.
    * Logs may be visible to more people than secrets should be.

* **Key/cap handling:**

  * Caps/tokens are passed via headers only.
  * SDK never writes caps to logs or disk by default.
  * Config encourages short-lived, scoped tokens and rotation patterns. 

* **PII:**

  * The SDK does not inspect payload semantics; it treats DTOs as opaque JSON.
  * Docs must emphasise that **if apps include PII in DTOs**, they must:

    * Avoid logging full payloads.
    * Use redaction hooks in middleware/middleware-like interceptors.

* **AuthN/Z:**

  * Auth is handled by RON-CORE via capability tokens/macaroons (`Authorization` header or similar).
  * SDK only attaches tokens; it does not make authorization decisions.

* **Input validation:**

  * Validates configuration (env, baseUrl, timeouts).
  * Validates HTTP methods / path shapes to avoid obviously malformed requests.
  * Leaves domain-level validation to RON-CORE and app logic.

* **Supply chain:**

  * Depends on widely used libraries (Guzzle or Symfony HTTP client).
  * Composer constraints are pinned to stable versions; `composer audit` must be clean in CI.

---

## 10) Error Taxonomy

| Class / Code           | When it happens                            | User hint                                  | Retries                           |
| ---------------------- | ------------------------------------------ | ------------------------------------------ | --------------------------------- |
| `RonConfigException`   | Bad env/config (URL, timeouts, etc.)       | Fix env or config and redeploy             | no                                |
| `RonNetworkException`  | DNS/TCP/TLS failure, unreachable gateway   | Check network, gateway address, TLS cert   | yes (with backoff, if idempotent) |
| `RonTimeoutException`  | Request exceeded configured timeout        | Increase timeout or reduce work per req    | yes (idempotent only)             |
| `RonAuthException`     | 401/403 from gateway                       | Refresh or fix token/caps; least-privilege | no (unless token refreshed)       |
| `RonProblemException`  | 4xx/5xx with Problem / error envelope      | Inspect `code`/`kind` and `details`        | maybe (depends on `kind`)         |
| Generic `RonException` | Unexpected response or SDK invariant issue | File issue / bug if reproducible           | no                                |

---

## 11) Concurrency Model

PHP is **synchronous** and process-based, but deployment modes differ.

* **PHP-FPM / Apache / Nginx+FPM:**

  * Each request runs in its own process/worker.
  * Typically, a `RonClient` is created per request or injected per controller; overhead is primarily HTTP, not the SDK.
* **Long-lived workers (Swoole, RoadRunner, Laravel Octane):**

  * The same `RonClient` instance may be reused across many requests.
  * Responsibility moves to the app to:

    * Rotate tokens periodically.
    * Handle config reloads where needed.

Concurrency rules:

* No global mutable singletons that carry caps; clients are explicit.
* No shared mutable state across workers beyond what the PHP runtime and app framework manage.
* No background threads; if future async variants are added, they will live in a separate package with explicit documentation.

```mermaid
stateDiagram-v2
  [*] --> Unconfigured
  Unconfigured --> Configured: new ClientConfig()
  Configured --> Active: new RonClient(config)
  Active --> Active: request()/response()
  Active --> Closed: close()
  Closed --> [*]
```

---

## 12) Compatibility & Requirements

* **PHP:** 8.1+ (typed properties, enums, improved error handling).
* **Extensions:**

  * `ext-json` (required)
  * `ext-curl` (or HTTP client of choice)
  * `ext-openssl` (TLS)
* **OS:** Linux, macOS, Windows (where PHP is supported).
* **Runtime:** FPM, CLI, Swoole/RoadRunner (with appropriate docs/recipes).

---

## 13) Examples

**1) Simple sync call (CLI or controller):**

```php
$config = Ron\AppSdkPhp\ClientConfig::fromEnv();
$client = new Ron\AppSdkPhp\RonClient($config);

$data = $client->get('/app/hello')->json();
echo "Hello: " . json_encode($data) . PHP_EOL;

$client->close();
```

**2) Laravel controller action:**

```php
use Ron\AppSdkPhp\RonClient;

class HelloController
{
    public function __construct(private RonClient $ron) {}

    public function __invoke()
    {
        $data = $ron->get('/app/hello')->json();
        return response()->json($data);
    }
}
```

**3) Worker loop (very simple poller):**

```php
$config = ClientConfig::fromEnv();
$client = new RonClient($config);

while (true) {
    $page = $client->get('/app/jobs/next')->json();
    foreach ($page['items'] as $job) {
        // process job...
        $client->post('/app/jobs/' . $job['id'] . '/ack', ['status' => 'done']);
    }
    sleep(1);
}

$client->close();
```

---

## 14) Troubleshooting

* **“Connection refused” / timeouts:**

  * Check `RON_SDK_GATEWAY_ADDR`.
  * Verify gateway is listening and reachable from the PHP host.
* **TLS errors:**

  * Ensure `https://` URL uses valid certificates.
  * Verify PHP `openssl` extension and CA bundle configuration.
* **Auth errors (401/403):**

  * Ensure `RON_SDK_TOKEN` or config token is set and scoped correctly.
  * Check that token isn’t expired; consider using `tokenProvider`.
* **Strange numeric behavior (IDs truncated):**

  * Ensure you treat IDs as strings in application code; avoid casting `u64` to `int` on 32-bit environments.
* **High latency:**

  * Verify gateway & node health.
  * Check for unnecessary per-request client construction in hot paths; reuse `RonClient` where safe, especially in long-lived workers.

---

## 15) Development Notes

**Project conventions (RustyOnions SDKs):**

* Use `Ron\AppSdkPhp\...` namespace and PSR-4 autoloading.
* Exceptions are in `Ron\AppSdkPhp\Exception`.
* DTOs and envelopes map directly to schemas defined in `SDK_SCHEMA_IDB.MD`. 
* Shared env var names (`RON_SDK_*`) align with other language SDKs.

**Pre-commit quickcheck (for contributors):**

```bash
composer lint
composer analyse
composer test
composer audit
```

---

## 16) Mermaid Diagrams — Policy & Tooling (REQUIRED)

Diagrams in this README are written inline using Mermaid. For more complex diagrams, keep `.mmd` sources under `docs/` and render SVGs in CI if desired.

**Render locally (SVG) with Mermaid CLI:**

```bash
npm i -g @mermaid-js/mermaid-cli
mmdc -i docs/arch.mmd -o docs/arch.svg
```

**Render via Docker (no Node install):**

```bash
docker run --rm -v "$PWD":/data minlag/mermaid-cli \
  mmdc -i docs/arch.mmd -o docs/arch.svg
```

**Optional CI snippet (GitHub Actions):**

```yaml
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          mkdir -p docs
          for f in $(git ls-files '*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
```

---

## 17) Roadmap & TODO

* [ ] **M1 — Core client MVP**

  * `RonClient`, `ClientConfig`, basic request/response, env config, canonical error mapping.
* [ ] **M2 — Interop & framework adapters**

  * Laravel/Symfony recipes.
  * Interop tests against test node (hello + pagination).
* [ ] **M3 — Hardening**

  * Timeouts, retries, idempotency helpers.
  * Security checks (no caps in logs), basic fuzzing of error envelopes.
* [ ] **M4 — Streaming & workers (optional)**

  * SSE-based `subscribe` API for CLI workers.
  * Examples for long-lived worker modes (Swoole/RoadRunner).
* [ ] **M5 — Codegen & advanced schema tooling**

  * DTO generation from canonical schema / OpenAPI.
  * Schema drift checks in CI.

---

## 18) Changelog

See [CHANGELOG.md](./CHANGELOG.md).
Follow SemVer; document breaking changes with migration notes and link them from release tags.

---

## 19) License

Dual-licensed under **MIT** or **Apache-2.0**.
See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE).

---

## 20) Contributing

PRs welcome! Please:

* Keep PHP code PSR-12 compliant.
* Run `composer lint`, `composer analyse`, and `composer test` before pushing.
* Update docs and examples when public APIs change.
* Include:

  * Tests for new behavior or bugfixes.
  * Notes in `CHANGELOG.md`.
  * Any new or updated diagrams (`docs/*.mmd` → `docs/*.svg` if used).

Also see the **IDBs** in this repo:

* [`SDK_IDB.MD`](./SDK_IDB.MD) — invariant-driven blueprint for this SDK. 
* [`SDK_SECURITY.MD`](./SDK_SECURITY.MD) — security checklist and threat model.
* [`SDK_SCHEMA_IDB.MD`](./SDK_SCHEMA_IDB.MD) — unified schema & facet contract across SDKs. 

```
```
