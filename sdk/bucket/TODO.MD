

---

## File Tree — `sdk/ron-app-sdk-php/`

```text
sdk/ron-app-sdk-php/
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-MIT
├─ LICENSE-APACHE
├─ composer.json
├─ phpunit.xml.dist
├─ phpstan.neon.dist
├─ psalm.xml.dist
├─ .gitignore
├─ .gitattributes
├─ .editorconfig
├─ .php-cs-fixer.dist.php
├─ SDK_IDB.MD
├─ SDK_SECURITY.MD
├─ SDK_SCHEMA_IDB.MD
├─ CODECHECK.MD
├─ CODECOMMENTS.MD
├─ TODO.md
├─ docs/
│  ├ OVERVIEW.md
│  ├ ARCH.mmd
│  ├ ARCH.svg
│  ├ ERROR_MODEL.md
│  └ SCHEMA_NOTES.md
├─ examples/
│  ├ hello.php
│  ├ pagination.php
│  └ worker.php
├─ src/
│  ├ RonClient.php
│  ├ ClientConfig.php
│  ├ Response.php
│  ├ Problem.php
│  ├ Pagination.php
│  ├ Middleware/
│  │  ├ LoggingMiddleware.php
│  │  ├ RetryMiddleware.php
│  │  └ IdempotencyMiddleware.php
│  ├ Http/
│  │  ├ HttpClientInterface.php
│  │  ├ GuzzleHttpClient.php
│  │  └ SymfonyHttpClient.php
│  ├ Exception/
│  │  ├ RonException.php
│  │  ├ RonConfigException.php
│  │  ├ RonNetworkException.php
│  │  ├ RonTimeoutException.php
│  │  ├ RonAuthException.php
│  │  └ RonProblemException.php
│  └ Util/
│     ├ Env.php
│     ├ IdempotencyKey.php
│     └ Json.php
├─ tests/
│  ├ bootstrap.php
│  ├ Unit/
│  │  ├ ClientConfigTest.php
│  │  ├ RonClientBasicTest.php
│  │  ├ ErrorMappingTest.php
│  │  ├ PaginationTest.php
│  │  └ SecurityHeadersTest.php
│  └ Integration/
│     ├ HelloWorldTest.php
│     └ PaginationLiveTest.php
├─ .github/
│  └ workflows/
│     ├ ci.yml
│     └ security.yml
└─ tools/
   ├ gen-schema-fixtures.php
   └ smoke-app-plane.sh
```

---

## File-by-file Descriptions

### Root files

* **README.md**
  Top-level documentation for the SDK: what it is, how to configure it, quickstart examples, and roadmap (the README we just drafted).

* **CHANGELOG.md**
  Human-readable history of releases and breaking changes, following SemVer and documenting migration notes per version.

* **LICENSE-MIT**
  MIT license text for dual-licensing the SDK.

* **LICENSE-APACHE**
  Apache-2.0 license text for dual-licensing the SDK.

* **composer.json**
  Composer package manifest defining the package name, dependencies (e.g., Guzzle/Symfony HTTP), autoload rules, and scripts (test/lint/analyse).

* **phpunit.xml.dist**
  PHPUnit configuration file specifying test suites, bootstrap, and coverage settings for unit and integration tests.

* **phpstan.neon.dist**
  Baseline configuration for PHPStan static analysis (paths, level, and ignore rules).

* **psalm.xml.dist**
  Optional Psalm static analysis configuration, if you want dual static checking alongside PHPStan.

* **.gitignore**
  Ignores vendor/, build artifacts, IDE metadata, and generated files like `docs/*.svg` or `composer.lock` (if you decide not to commit it).

* **.gitattributes**
  Git attributes file to control line endings, linguist classifications, and export-ignore rules (e.g., omit tests/docs from distribution archives).

* **.editorconfig**
  Shared editor settings (indent style/size, charset, line endings) to keep formatting consistent across IDEs.

* **.php-cs-fixer.dist.php**
  Configuration for PHP-CS-Fixer (or similar) defining the code style rules and which directories to format.

* **SDK_IDB.MD**
  Invariant-Driven Blueprint for `ron-app-sdk-php`, capturing scope, invariants, performance/security expectations, and interop requirements for this SDK.

* **SDK_SECURITY.MD**
  Security checklist and threat model focused on caps handling, timeouts, retries, error/logging hygiene, and future PQ/audit hooks.

* **SDK_SCHEMA_IDB.MD**
  Unified schema + facet contract profile describing DTOs, error envelopes, pagination, and facet manifest rules shared across all SDKs.

* **CODECHECK.MD**
  Local build/lint/test playbook for this SDK (what to run before commit, what counts as a “clean” build, any special checks).

* **CODECOMMENTS.MD**
  Guidelines for how to comment the PHP code (docblock style, invariants in comments, how to document tricky schema mappings).

* **TODO.md**
  Implementation checklist that mirrors the IDB milestones (M1–M5), listing which files/features still need to be filled in or hardened.

---

### `docs/`

* **docs/OVERVIEW.md**
  Higher-level narrative of the SDK: motivation, architecture with RON-CORE, and how PHP fits into the overall polyglot SDK story.

* **docs/ARCH.mmd**
  Source Mermaid diagram describing architecture (PHP app → RonClient → gateway → RON-CORE), used to generate SVGs for README/docs.

* **docs/ARCH.svg**
  Rendered SVG diagram generated from `ARCH.mmd`, suitable for embedding in README or other docs.

* **docs/ERROR_MODEL.md**
  Detailed explanation of the error taxonomy and how canonical error envelopes/RFC7807 map to PHP exceptions.

* **docs/SCHEMA_NOTES.md**
  Notes on schema/type mapping for this SDK (e.g., how `u64` is represented as strings, pagination envelopes, and any PHP-specific nuances).

---

### `examples/`

* **examples/hello.php**
  Minimal “hello world” example using `ClientConfig::fromEnv()` + `RonClient` to call `/app/hello` and print the JSON response.

* **examples/pagination.php**
  Example demonstrating list/pagination helpers: repeatedly fetching a paginated `/app/...` endpoint and iterating through all items.

* **examples/worker.php**
  Simple long-running worker example (CLI-style) polling a hypothetical `/app/jobs/next` and posting results back to the node.

---

### `src/` (main library code)

* **src/RonClient.php**
  Core client class that wraps HTTP calls to `svc-gateway`’s `/app/*` surface, handling headers, caps, timeouts, retries, and error mapping.

* **src/ClientConfig.php**
  Configuration value object for the SDK with constructors like `fromEnv()`, validation for base URL/timeouts, and optional token provider support.

* **src/Response.php**
  Immutable wrapper for responses returning status code, headers, raw body, and convenience `json()` decoding with error handling.

* **src/Problem.php**
  DTO representing the canonical error envelope / RFC7807 problem document and providing normalized getters like `getCode()` and `getCorrelationId()`.

* **src/Pagination.php**
  Pagination helper types (`Page`, `Paginator`) for dealing with `items` + `next_page_token` responses and iterating through multiple pages safely.

#### `src/Middleware/`

* **src/Middleware/LoggingMiddleware.php**
  Optional middleware that logs requests and responses via PSR-3, redacting sensitive headers/body fields according to the security checklist.

* **src/Middleware/RetryMiddleware.php**
  Middleware implementing bounded retry/backoff for idempotent operations, honoring max retry counts and gateway “slow down” signals.

* **src/Middleware/IdempotencyMiddleware.php**
  Helper middleware that generates and attaches idempotency keys for write operations when requested, to make safe retries easier.

#### `src/Http/`

* **src/Http/HttpClientInterface.php**
  Small abstraction for HTTP clients the SDK can use (e.g., Guzzle or Symfony HTTP client), making it easy to swap implementations.

* **src/Http/GuzzleHttpClient.php**
  Concrete `HttpClientInterface` implementation using Guzzle, wired with timeouts, TLS verification, and error normalization.

* **src/Http/SymfonyHttpClient.php**
  Alternative `HttpClientInterface` implementation using Symfony HTTP Client for users in Symfony-heavy stacks.

#### `src/Exception/`

* **src/Exception/RonException.php**
  Base exception class for all SDK-specific errors, extending PHP’s `\RuntimeException` (or similar) and carrying common metadata.

* **src/Exception/RonConfigException.php**
  Thrown when SDK configuration is invalid (bad URL, missing env var, invalid timeout values, etc.).

* **src/Exception/RonNetworkException.php**
  Thrown when network-level errors occur (DNS errors, TCP failures, TLS issues) before a valid HTTP response is obtained.

* **src/Exception/RonTimeoutException.php**
  Thrown when a request exceeds configured timeouts, signaling that the call was aborted instead of hanging forever.

* **src/Exception/RonAuthException.php**
  Thrown for 401/403 auth errors when the gateway rejects caps/tokens or when tokens are expired/mis-scoped.

* **src/Exception/RonProblemException.php**
  Thrown when the gateway returns a canonical error/problem envelope (4xx/5xx), wrapping a `Problem` DTO for structured inspection.

#### `src/Util/`

* **src/Util/Env.php**
  Small utility for reading/normalizing environment variables (`RON_SDK_*`), handling defaults, integer parsing, and fail-closed validation.

* **src/Util/IdempotencyKey.php**
  Utility to generate and validate idempotency keys (UUID-like strings), used by idempotent write operations and retry logic.

* **src/Util/Json.php**
  JSON encode/decode helpers that throw well-typed exceptions instead of returning null or silently failing, and help with canonical encoding.

---

### `tests/`

* **tests/bootstrap.php**
  Test bootstrap file to set up autoloading, environment variables for tests, and any test-specific configuration.

#### `tests/Unit/`

* **tests/Unit/ClientConfigTest.php**
  Unit tests for `ClientConfig`, ensuring correct env parsing, validation errors, and defaults behavior.

* **tests/Unit/RonClientBasicTest.php**
  Unit-style tests for `RonClient` using a fake HTTP client to verify basic GET/POST behavior and header construction.

* **tests/Unit/ErrorMappingTest.php**
  Tests that canonical error and RFC7807 responses are mapped into `Problem` and `RonProblemException` correctly, including edge cases.

* **tests/Unit/PaginationTest.php**
  Tests for `Pagination` helpers, verifying iteration over `items` and `next_page_token`, and enforcing “no infinite unbounded fetch” behavior.

* **tests/Unit/SecurityHeadersTest.php**
  Tests that ensure tokens/caps are never logged or placed into query strings, and that logging middleware redacts sensitive headers.

#### `tests/Integration/`

* **tests/Integration/HelloWorldTest.php**
  Integration test that calls a real or dockerized test node’s `/app/hello` endpoint using the SDK, asserting shapes and basic connectivity.

* **tests/Integration/PaginationLiveTest.php**
  Integration test for live pagination against a test endpoint, verifying page token semantics and resilience under mild flaky conditions.

---

### `.github/workflows/`

* **.github/workflows/ci.yml**
  Main CI pipeline running lint, static analysis, unit tests, and (optionally) integration tests against a local test node.

* **.github/workflows/security.yml**
  Optional workflow that runs dependency audits, static analysis at a stricter level, and any security-focused checks (if you split them).

---

### `tools/`

* **tools/gen-schema-fixtures.php**
  Developer tool for generating or updating JSON fixtures from a live node to keep schema tests in sync with real responses.

* **tools/smoke-app-plane.sh**
  Shell script that runs a simple smoke test: calls `/app/hello` (or a similar endpoint) via CLI using `hello.php` and checks for non-error output.

---

