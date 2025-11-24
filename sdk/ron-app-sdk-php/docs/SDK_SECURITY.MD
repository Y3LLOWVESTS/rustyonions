````md
---

# SDK SECURITY CHECKLIST — `ron-app-sdk-php`

> Applies to: `ron-app-sdk-php` (PHP backend/CLI SDK).  
> Scope: **backend** (PHP services, workers, CLI tools).  
> Browser-specific items are documented for cross-SDK parity but **implemented in TS SDKs**, not here.

---

## RO:WHAT

Security checklist for `ron-app-sdk-php` within the broader RON app SDK family.  
Defines the **minimum bar** for “ship” and **stretch items** for “God-tier” for PHP:

* How caps/macaroons/tokens are handled.
* How HTTP(S) is configured (TLS, timeouts, retries).
* How errors/logging behave (no leaks, useful to devs).
* How this SDK plugs into future PQ/audit/formal-methods work without breaking.

---

## RO:WHY

This SDK is how PHP apps and CLI tools talk to RON-CORE (`svc-gateway` / `/app/*`).  
If the PHP SDK is sloppy with:

* capabilities (macaroons/tokens),
* error messages,
* logging,
* timeouts & retries,

…then all the hardening in Micronode/Macronode, omnigate, and gateway can be bypassed in practice.

For `ron-app-sdk-php`, “secure by default” means:

* Caps are treated like cryptographic plutonium (short-lived, scoped, never logged).
* Network calls are **bounded** (timeouts + limited retries).
* Errors are **structured** and scrubbed.
* Debug is **opt-in** and still redacts secrets.
* The design leaves room for PQ toggles and audit context later.

---

## RO:INVARIANTS

Across **all** SDKs (including PHP):

* No SDK logs secrets (caps, raw headers, private keys, cookies).
* No SDK “helpfully” retries in ways that violate idempotency.
* All SDK network calls are **bounded** (timeouts, backoff, cap on retries).
* Capabilities are **least-privilege**, **short-lived**, and **rotatable**.
* Error surfaces are **structured**, not random stack traces with secrets.
* Debug/trace modes are **explicitly opt-in** and still scrub secrets.
* For PHP specifically: no accidental leaking of caps into framework logs or exception messages.

---

## 0) TL;DR CHECKLIST (ron-app-sdk-php)

Before calling `ron-app-sdk-php` “beta”:

* [x] **Auth & caps**  
  * Safe in-memory storage only, no logging, support for rotation via `tokenProvider`, examples use least-privilege scopes.
* [x] **Transport**  
  * HTTPS/TLS required by default, insecure HTTP only with explicit `allowInsecureHttp=true`, sane timeouts, bounded retries with backoff for idempotent ops.
* [x] **Error handling**  
  * Structured exceptions (`RonProblemException`, `RonAuthException`, `RonNetworkException`, etc.), no caps or raw stack traces in user-visible messages.
* [x] **Logging**  
  * Optional PSR-3 logger integration with header/body scrubbing; log correlation IDs and error codes, not secrets.
* [x] **DoS safeguards**  
  * Response size and pagination caps enforced by default; no unbounded streaming APIs in MVP; concurrency guidance in docs.
* [ ] **Browser variant**  
  * Not applicable for PHP; browser-specific constraints live in `ron-app-sdk-ts` (frontend flavor).
* [x] **Tests**  
  * Unit + integration tests for failure paths; at least one property/fuzz-style test for error parsing and header construction.
* [x] **Docs**  
  * Quickstarts show env-based caps, timeouts, and secure patterns; any insecure examples are explicitly marked “dev-only”.
* [x] **Hooks for future**  
  * Reserved config fields for PQ mode, audit context, and future signed caps/manifests without breaking APIs.

---

## 1) Capabilities & Auth (ALL SDKs, PHP-specific details)

**Goal:** Caps/macaroons are treated like cryptographic plutonium.

### 1.1 Design (PHP)

* [x] **No ambient caps**  

  * Caps are provided per-**client instance** via `ClientConfig` (`token` or `tokenProvider`), not stored in globals or static singletons.
  * Laravel/Symfony bindings inject a configured `RonClient` via DI; they do not set any global magical caps.

* [x] **Least privilege by default**  

  * Docs and examples use tenant-scoped, least-privilege caps (e.g., `app:todo:read/write`) rather than wildcard `*`.
  * README includes explicit “do not use global admin caps in app code” warnings.

* [x] **Short-lived caps**  

  * Recommended patterns emphasize short TTL caps (minutes/hours), not long-lived tokens.
  * Docs promote “short-lived, refreshable tokens” patterns in CLI workers and long-lived PHP processes (Swoole/RoadRunner).

* [x] **Rotation**  

  * `ClientConfig` supports a `tokenProvider` callable that can be used to dynamically fetch/refresh caps.
  * Docs include a section “Token rotation example” showing 401 → refresh → retry logic implemented in user code or middleware.

### 1.2 Handling & Storage (PHP)

* [x] **No caps in logs** (ever)  

  * PSR-3 logging hooks must redact:
    * `Authorization`, `Cookie`, any `X-Ron-*` headers.
  * Even in debug mode, these headers are replaced with `"[REDACTED]"`.

* [x] **No caps in error messages**  

  * Exception messages never include raw header values or caps.
  * If an auth error needs to report something, it uses error codes and generic messages (“invalid or expired token”) instead of embedding the token.

* [x] **No caps in panic/exception messages**  

  * All `Ron*Exception` classes are audited to ensure no caps are formatted into `message` or `__toString()` accidentally.

* [x] **No caps in URLs (query strings)**  

  * Auth is carried via headers (e.g., `Authorization` or dedicated RON headers), never via query params or path segments.

* [x] **No persistent storage of caps in plaintext from SDK internals**  

  * SDK never writes caps to disk (no caching, no file-based storage, no DB writes).
  * Backend usage:
    * Caps are expected to come from environment variables, secrets managers, or application config—**all outside** the SDK.
  * SDK does not implement any “remember-me” functionality.

### 1.3 API Shape (PHP)

* [x] Helpers so user code rarely manipulates raw caps  

  * Primary pattern: `new RonClient(new ClientConfig(['token' => getenv('RON_SDK_TOKEN')]))`.
  * User code seldom constructs headers manually; the SDK handles token → header mapping.

* [x] “Session” abstraction  

  * `RonClient` acts as a session-like object: once destroyed or `close()`d, it releases references to caps, allowing GC.
  * Future improvements may include explicit “retire session” helper that clears any internal references to tokens (best-effort within PHP constraints).

---

## 2) Transport & Timeouts (PHP)

**Goal:** Every SDK call is bounded in time and resources.

* [x] **TLS/HTTPS required by default**  

  * `baseUrl` must be `https://` unless `allowInsecureHttp=true` is explicitly set in `ClientConfig`.
  * README and docs mark `allowInsecureHttp` as *dev/testing only*, not acceptable for production.

* [x] **Timeouts mandatory**  

  * `ClientConfig` requires or defaults to non-zero timeouts:
    * `connectTimeoutMs`
    * `readTimeoutMs`
    * `writeTimeoutMs`
    * `overallTimeoutMs`
  * Config parsing is **fail-closed**:
    * Negative or zero timeouts cause a `RonConfigException` at startup.

* [x] **Retries bounded**  

  * Optional retry logic (if enabled) is:
    * Bound to a small max (e.g., max 3 retries).
    * Applied **only** to idempotent operations (`GET`, `HEAD`, and `PUT`/`DELETE` if an idempotency key is present).
  * No infinite retry loops; no silent retries on non-idempotent writes.

* [x] **Idempotency support**  

  * Idempotency keys supported via:
    * `IdempotencyKey::generate()` helper.
    * Automatic setting of `X-Idempotency-Key` when requested.
  * Docs include examples:
    * “Safe retries with idempotency keys” for job processing and at-least-once semantics.

---

## 3) Errors & Logging (PHP)

**Goal:** Helpful to developers, useless to attackers.

### 3.1 Errors

* [x] **Structured error types**  

  * All errors are represented via typed exceptions:
    * `RonProblemException` (captures canonical Problem envelope fields: `type`, `title`, `status`, `detail`, `instance`, `extensions`).
    * `RonNetworkException`, `RonTimeoutException`, `RonAuthException`, `RonConfigException`, etc.
  * Each exception carries:
    * Safe human-readable message.
    * Machine-parseable fields (error code, status, correlation ID where available).
    * Optional structured details.

* [x] **Message hygiene**  

  * Raw server error bodies are **not** blindly dumped into exception messages.
  * Where server responses contain a problem envelope, the SDK maps it into `Problem` and renders a **safe summary**.
  * Arbitrary HTML/text bodies (e.g., from proxies) are truncated and sanitized in messages.

* [x] **Separation of concerns**  

  * Exceptions separate:
    * Human message (safe, no secrets).
    * Machine code/status (e.g., HTTP status, RON error code).
    * Meta (`correlationId`, retryability hints, route path, etc).
  * Exception APIs allow application code to log more detail if desired, but default messages are conservative.

* [x] **No stack-trace leakage by default**  

  * Code examples do not dump full stack traces to logs in production.
  * Stack traces are accessible via `$e->getTrace()` in PHP, but not automatically printed by SDK helpers.
  * Any debug helper that prints stack traces is clearly labeled dev-only.

### 3.2 Logging

* [x] **Headers scrubbing**  

  * Logging middleware scrubs:
    * `Authorization`, `Cookie`, `Set-Cookie`, `X-Ron-*` headers.
  * Logs use placeholder `"[REDACTED]"` for these fields.

* [x] **Body scrubbing**  

  * Debug logging for request/response bodies is **opt-in**.
  * SDK provides a pluggable scrubber callback:
    * Used to redact PII-like fields in JSON before logging.
  * Default behavior: bodies not logged at all unless explicitly configured.

* [x] **Correlation IDs**  

  * SDK supports injecting/forwarding a `X-Request-Id` / correlation ID header.
  * Logs emphasize correlation IDs, route paths, and error codes; they avoid logging raw contents.

---

## 4) DoS & Resource Use (PHP)

**Goal:** SDK is a good cluster citizen; it doesn’t accidentally DDoS RON-CORE.

* [x] **Response size caps**  

  * The SDK encourages server-driven pagination (e.g., `limit`, `cursor` patterns).
  * Helpers that fetch lists ensure:
    * Reasonable default limits are used (e.g., 100 items).
    * Any client-side iteration over pages uses sane bound loops, not open-ended recursion.

* [x] **Streaming APIs guarded**  

  * MVP for `ron-app-sdk-php` does **not** include long-lived streaming APIs in the core package.
  * Future SSE/streaming helpers (CLI/worker only) will:
    * Require explicit opt-in methods (`subscribe`).
    * Take explicit sanity settings (max events, idle timeouts, etc).

* [x] **Concurrency settings**  

  * Recommended usage:
    * One `RonClient` per PHP worker; concurrency handled by PHP-FPM or worker framework.
  * For environments where concurrency is explicit (Swoole/ReactPHP, future async package):
    * Helpers will expose a configurable max inflight requests to avoid uncontrolled concurrency.

* [x] **Backoff on error**  

  * For HTTP 429, 503, and other gateway “slow down” signals:
    * Retry logic (if enabled) uses exponential backoff and respects `Retry-After` headers where applicable.
  * Docs call out “backoff best practices” to avoid thundering herds.

---

## 5) Browser-Specific Checklist (`ron-app-sdk-ts` frontend flavor)

> **Note:** `ron-app-sdk-php` is **not** used in the browser.  
> These items are included for cross-SDK parity but are enforced in the TypeScript frontend SDK, not here.

* [N/A] **No raw caps in localStorage by default**  
* [N/A] **CORS-safe defaults**  
* [N/A] **CSRF aware**  
* [N/A] **XSS-resistant patterns**  
* [N/A] **SRI hooks for static bundles**

For RON-CORE as a whole, docs should link PHP backend guidance to TS frontend guidance and clarify that browser-facing security properties live in TS, not PHP.

---

## 6) Backend-Specific Checklist (PHP, Node/TS, Go, Rust)

**Goal:** Hard to shoot yourself in the foot, even in dynamic languages.

For `ron-app-sdk-php` specifically:

* [x] **Safe config patterns**  

  * Sample code uses:
    * Environment variables for caps and base URL (`RON_SDK_GATEWAY_ADDR`, `RON_SDK_TOKEN`).
    * Reasonable timeout defaults.
  * Config is **validated** and fail-closed:
    * Bad URLs, invalid timeouts, or missing mandatory configuration result in `RonConfigException`.

* [x] **Framework adapters**  

  * Guidance for Laravel:
    * Service provider that reads from env/config and builds a single `RonClient`.
    * Avoids per-request reconfiguration and mixing caps from multiple tenants in one client instance.
  * Guidance for Symfony:
    * Services configuration, environment overrideable parameters, no global statics.

* [x] **HTTP client invariants**  

  * Underlying PSR-18 clients (e.g., Guzzle or Symfony HTTP Client) are always configured with:
    * TLS verification enabled.
    * Timeouts and optional retry logic.
    * No disabled certificate validation unless explicitly configured for dev.

* [x] **Rust/Go/TS cross-reference**  

  * `ron-app-sdk-php` docs refer to common patterns:
    * correlation ID propagation,
    * consistent header naming,
    * same error envelope semantics as Rust/TS/Go.

---

## 7) Testing & Verification (PHP)

**Goal:** We don’t just hope it’s safe; we prove a lot of it.

* [x] **Unit tests**  

  * Verify that:
    * Caps are not present in logs when PSR-3 logger is used.
    * Error messages are scrubbed and do not contain tokens.
    * Timeouts are enforced and misconfigurations result in `RonConfigException`.
    * Header building never places tokens in query strings.

* [x] **Property tests / fuzz** (Tier 2, minimal but present)  

  * Fuzz `application/problem+json` bodies:
    * Ensure they always map to valid `Problem` objects or produce a safe `RonProblemException`, never a fatal error.
  * Fuzz header construction:
    * Ensure that no arbitrary header building path ever includes raw caps in unexpected places.

* [x] **Integration tests**  

  * SDK tested against:
    * A local Micronode/Macronode test instance or test gateway.
    * Misconfigured endpoints (TLS errors, 5xx, slow responses) to verify:
      * Proper error mapping.
      * Bounded retries.
      * No unbounded hangs.

* [x] **Negative tests**  

  * Invalid/broken caps:
    * Produce structured auth errors (e.g., `RonAuthException` wrapping an appropriate Problem).
  * Giant payloads:
    * Are rejected or surface predictable errors without OOMs or runaway allocations.
  * Insecure HTTP without `allowInsecureHttp`:
    * Raises `RonConfigException` or equivalent at startup, not at runtime.

---

## 8) Docs & DX (PHP)

**Goal:** Secure by default, even if devs only copy-paste.

* [x] **Quickstarts show secure patterns**  

  * Examples:
    * Use env vars for caps and base URL.
    * Use `ClientConfig::fromEnv()` which sets timeouts and secure defaults.
  * No quickstart example uses hardcoded tokens in code without a bold “don’t do this in production” warning.

* [x] **Explicit “don’t do this in prod” callouts**  

  * Any example using:
    * `allowInsecureHttp=true`,
    * hardcoded tokens,
    * disabling TLS checks
  * …is marked clearly as “development only / not for production.”

* [x] **Threat model call-out**  

  * README includes a short section:
    * Assumptions:
      * PHP backend/CLI runs in a *trusted* environment (server-side).
      * Caps are managed by the app using secure storage (not by the SDK).
    * Prohibitions:
      * Do not embed `ron-app-sdk-php` in browser or untrusted client code.
      * Do not log caps; do not share `RonClient` across tenants unless caps are correctly isolated.

---

## 9) Future / “God-Tier” Hooks (PHP)

Not required for the first release, but we keep hooks ready so we can evolve without breaking APIs:

* [x] **PQ toggle hooks**  

  * `ClientConfig` reserves (or can easily add) fields like `pqMode` (`off`/`hybrid`) once transport libraries support PQ.
  * Naming aligns with RON-CORE PQ profile.

* [x] **Audit integration**  

  * Requests can carry an `X-Ron-Audit-Context` or similar header, sourced from config or per-call overrides.
  * SDK design allows this header to be wired into future `ron-audit` services without API breakage.

* [x] **Signed config/caps**  

  * SDK treats caps as opaque strings and does not assume any particular format.
  * This leaves room for signed caps/macaroons or signed facet manifests to be introduced, verified, and rotated outside of the SDK.

* [x] **Formal methods**  

  * Auth & idempotency logic is kept in small, coherent components:
    * header construction,
    * retry+idempotency logic.
  * This allows the possibility of modeling/verification (e.g., TLA+/Alloy) on critical pieces later, even if formal proofs are not a requirement for the PHP SDK.

---

## 10) Schema & Security (Interop Guarantees)

**Goal:** Security properties (no leaks, bounded calls, correct auth handling) are preserved identically across SDKs via **shared schemas and envelopes**, not ad hoc per language.

### 10.1 Canonical Error Envelope

All SDKs, including PHP, treat the **canonical error envelope** (as defined in `SDK_SCHEMA_IDB.MD`) and/or the RFC7807 Problem envelope as the source of truth. A typical shape:

```json
{
  "code": "string",
  "message": "string",
  "kind": "string",
  "correlation_id": "string",
  "details": { }
}
```

* [x] `ron-app-sdk-php` MUST attempt to parse error responses into this canonical shape (or the canonical Problem envelope where applicable).
* [x] If parsing fails (e.g., non-JSON body), the SDK wraps the failure in a **generic transport error** with a safe message and optional debug metadata, never exposing raw bodies by default.
* [x] Exceptions expose:

  * `code`, `kind`, `message`, `correlationId`, and sanitized `details` (through `Problem` or equivalent value object).

* [x] Raw HTTP bodies and headers are only exposed via **explicitly opt-in** debug paths with scrubbing applied.

### 10.2 DTO & Auth Field Consistency

* [x] DTOs (request/response types) follow canonical schemas from `ron-proto` / OpenAPI:

  * Same field names,
  * Same JSON shapes,
  * Same type mappings as in `SDK_SCHEMA_IDB.MD` (e.g., `u64` as string on 32-bit PHP).

* [x] Auth fields (headers, JSON fields, cookies) are **consistent** across SDKs:

  * `Authorization` header usage is uniform.
  * Capability/macroon-bearing headers are never:
    * Copied into DTO payloads,
    * Reflected into error messages or logs.

* [x] SDK never stores caps/macaroons inside DTOs or error envelopes; they are tracked separately in client/session state only.

### 10.3 Facet Manifest Security

If `ron-app-sdk-php` eventually helps generate or manage facet manifests (TOML):

* [x] It MUST emit manifests that follow the shared facet schema in `SDK_SCHEMA_IDB.MD`:

  * `facet.id`, `facet.kind` (`static`, `echo`, future `proxy`),
  * `route.method`, `route.path`,
  * optional `facet.security`, `facet.limits`, `route.integrity`, etc.

* [x] It MUST respect security hints:

  * Never silently set `public = true` or `requires_auth = false`.
  * Never generate routes that map outside the configured facet file root (for `static`).

* [x] For proxy facets:

  * Require explicit upstream config (host/port/protocol).
  * Avoid “open proxy” defaults.
  * Recommend or enforce mutual auth or restricted networks where applicable.

### 10.4 Schema-Driven Validation in Tests

* [x] Unit tests ensure:

  * Error parsing correctly handles the canonical envelope (and RFC7807 variants).
  * DTO serialization/deserialization matches the expected schema (golden fixtures, JSON snapshots).

* [x] Interop tests (per SDK_IDB §11) double as **security tests**:

  * Incorrect caps → structured auth errors (401/403) with proper `code`/`kind`.
  * Unknown fields in responses do not crash the SDK or cause leaks; they are either ignored or safely stored in extension fields.

* [x] Property tests where possible:

  * Fuzz error payloads to ensure the SDK never fatally errors or leaks raw internals.
  * Fuzz header values to ensure caps are never echoed back accidentally.

### 10.5 Browser vs Backend Surface

* [x] `ron-app-sdk-php` assumes a **backend/CLI** context:

  * DTOs are treated as untrusted data but are not rendered directly to browser/HTML from within the SDK.
  * Rendering concerns (XSS, DOM injection) are the responsibility of the app, not the SDK, but docs remind devs not to blindly dump DTOs into HTML.

* [x] Backend SDK usage still avoids logging raw DTOs that may contain secrets or PII.

* [x] Hooks for redaction / structured logging are available so that logs can be integrated into centralized observability stacks without leaking sensitive fields.

---

````
