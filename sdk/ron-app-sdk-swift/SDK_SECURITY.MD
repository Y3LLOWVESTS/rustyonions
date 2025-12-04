````md
---

# SDK SECURITY CHECKLIST — `ron-app-sdk-swift`

> Applies to: `ron-app-sdk-swift` (this file), and informs patterns for other `ron-app-sdk-*` SDKs.  
> Scope (Swift): **frontend** (iOS / iPadOS / macOS apps) and **backend** (server-side Swift / CLIs).

---

## RO:WHAT

Security checklist for `ron-app-sdk-swift`, aligned with the global SDK security profile.

Defines the **minimum bar** for “Developer Preview / beta” and the **stretch items** for “God-tier” for Swift:

- how we handle capabilities (macaroons/tokens),
- how we treat errors, logs, timeouts, retries,
- how we integrate with Apple platforms (Keychain, URLSession),
- and how we preserve the same security invariants as Rust/TS on the wire.

---

## RO:WHY

`ron-app-sdk-swift` is how **Apple apps and Swift services** talk to RON-CORE.

If this SDK is sloppy with:

- capabilities (macaroons/tokens/passports),
- error messages (leaking internals),
- logging (printing tokens, full responses),
- timeouts & retries (unbounded waits / unbounded retry storms),

…then all of the hardening in Micronode / omnigate is weakened or bypassed.

This checklist ensures `ron-app-sdk-swift`:

- doesn’t leak secrets,
- doesn’t weaken auth / capability semantics,
- behaves predictably under failure,
- is ready to plug into future PQ / audit / facet hardening work.

---

## RO:INVARIANTS

Across **all** SDKs (Swift included):

- No SDK logs secrets (caps, raw auth headers, private keys).
- No SDK “helpfully” retries in ways that violate idempotency.
- All SDK network calls are **bounded** (timeouts; no infinite retry loops).
- Capabilities are **least-privilege**, **short-lived**, and **rotatable** (SDK supports, app enforces).
- Error surfaces are **structured**, not raw stack traces or arbitrary blobs.
- Debug/trace modes are **explicitly opt-in** and still scrub secrets.

Swift-specific invariants:

- Tokens stored by apps use **Keychain / secure storage** on Apple platforms, never persisted by the SDK.
- URLSession usage always has timeouts attached (via config); no “wait forever” calls.
- `RonProblem` is the **canonical, structured error type**; anything else is wrapped into it or a small Swift error enum.

---

## 0) TL;DR CHECKLIST (Swift)

Before calling `ron-app-sdk-swift` “beta”:

* [x] **Auth & caps**:  
  - Per-client config; no globals; no logging; rotation via header provider; examples use scoped caps.
* [x] **Transport**:  
  - HTTPS by default; `allowInsecureHttp` only for dev; sane timeouts; **no automatic retries in MVP**.
* [x] **Error handling**:  
  - Structured `RonProblem` + `AppResponse`; safe messages only; synthetic “transport” problems for low-level failures.
* [x] **Logging**:  
  - Token-redacted logs; correlation IDs logged; body logging only behind explicit debug flags.
* [ ] **DoS safeguards** (MVP):  
  - No unbounded pagination helpers; guidance on page size; streaming not in 0.1.  
  - Full concurrency tuning & backoff for 429/5xx is **planned**.
* [ ] **Browser variant**:  
  - N/A for JS; for Swift we map this section to **iOS/macOS UI apps**:  
    - in-memory tokens preferred; Keychain usage documented; no insecure storage examples.
* [x] **Tests**:  
  - Unit tests for logging redaction, timeouts, error mapping.  
  - Interop tests planned; property/chaos tests planned as Tier-2 “SHOULD”.
* [x] **Docs**:  
  - Quickstarts use env vars or secure config; no hard-coded tokens in README.
* [x] **Hooks for future**:  
  - Config fields reserved for PQ (`pqMode`), audit identifiers (`actorId`/`auditContext`), and signed caps/manifests.

---

## 1) Capabilities & Auth (ALL SDKs, Swift-specific notes)

**Goal:** Caps/macaroons/passports are treated like cryptographic plutonium.

### Design

* [x] **No ambient caps**

  - `RonClient` requires explicit auth configuration via `RonConfig` per instance.
  - No global singletons like `RonClient.default` that silently carry tokens across modules.
  - Multiple `RonClient` instances with different caps (e.g., per-tenant, per-user) are supported and encouraged.

* [x] **Least privilege by default**

  - All examples in README / docs use **scoped** caps (e.g., “tenant:foo:read/write”) or JWTs with explicit app claims.
  - We avoid examples that imply “root”/`*` capabilities.

* [x] **Short-lived caps**

  - SDK supports short TTL caps by:
    - Accepting caps via `headerProvider` (async closure) so apps can refresh tokens frequently.
  - Docs emphasize short TTLs (minutes-hours) and server-side rotation rather than week-long tokens.

* [x] **Rotation**

  - `RonConfig` includes an optional `headerProvider: () async -> [String: String]` or similar.
  - Rotation pattern:
    - App updates Keychain / in-memory token.
    - `headerProvider` reads latest token on each call.
  - `RonClient` itself never caches old tokens beyond the duration of a single request.

### Handling & Storage

* [x] **No caps in logs** (ever)

  - Logging hooks always scrub headers:
    - `Authorization`, `X-RON-Passport`, `X-RON-Capability`, `Cookie`, etc.
  - Only header names plus safe metadata (e.g., “(redacted)”) ever appear.

* [x] **No caps in error messages**

  - `RonProblem.message` is derived from server’s safe `message` field.
  - SDK never constructs error messages by concatenating raw headers or request bodies.

* [x] **No caps in panic/exception messages**

  - Internal `Error` types never include caps in their `localizedDescription`.
  - If we attach debug metadata, caps are redacted or omitted.

* [x] **No caps in URLs** (query strings)

  - Auth is **always** conveyed via headers.
  - Any attempt by app code to include caps in query params is discouraged and called out in docs.

* [x] **No persistent storage of caps in plaintext from SDK internals**

  - The SDK does not write caps to disk, logs, or caches.
  - Backend (server-side Swift):
    - Caps stored in env vars or configuration, outside the SDK’s responsibility.
  - iOS/macOS apps:
    - SDK docs recommend Keychain or OS-managed secure storage; the SDK itself does not perform Keychain operations.
    - No built-in `UserDefaults` or file-based token storage.

### API Shape

* [x] Provide helpers so user code rarely touches raw caps

  - Example:

    ```swift
    let config = RonConfig(
        baseUrl: ...,
        headerProvider: { ["Authorization": "Bearer \(try tokenStore.current())"] }
    )
    ```

  - Apps can encapsulate token management in `tokenStore` without scattershot header handling.

* [ ] Consider a **“session” abstraction** that can be retired/zeroed

  - Future: `RonSession` or `AuthSession` wrapper with:
    - active caps,
    - `invalidate()` to zero tokens,
    - hooks for re-auth flows.
  - Not part of MVP, but design should leave room for this without breaking API.

---

## 2) Transport & Timeouts

**Goal:** Every SDK call is bounded in time and resources.

* [x] **TLS/HTTPS required**

  - Default: base URL **must** be `https://`.
  - If `http://` is used:
    - `RonConfig.allowInsecureHttp` must be set to `true`, and
    - README marks this pattern as **“dev-only, never production”**.
  - No fallback from HTTPS to HTTP; no “try HTTPS then downgrade” logic.

* [x] **Timeouts mandatory**

  - `RonConfig` contains:
    - `overallTimeoutMs`,
    - `connectTimeoutMs`,
    - `readTimeoutMs`.
  - URLSession tasks are created with these timeouts either via:
    - `URLSessionConfiguration` timeouts, or
    - explicit cancellation using `Task`/`Cancellation` and async timeouts.
  - There is **no** path where a request can block indefinitely.

* [ ] **Retries bounded**

  - MVP for Swift = **no automatic retries inside SDK**:
    - All operations are *single-try* with timeouts.
  - Future (opt-in) retry helper:
    - Only for **idempotent** operations (GET, HEAD, safe PUT with idempotency key).
    - Bounded attempts (e.g., 3) with exponential backoff.
    - Explicit config: app must opt-in per-call or per-client.

* [x] **Idempotency support**

  - `RonConfig` can accept an `idempotencyKeyFactory: () -> String`.
  - For write operations where idempotency is desired, SDK sets an `X-Idempotency-Key` header if the factory is present.
  - Docs emphasize:
    - “If your operation may be retried, enable idempotency keys.”

---

## 3) Errors & Logging

**Goal:** Helpful to developers, useless to attackers.

### Errors

* [x] **Structured error type**

  - Core canonical type: `RonProblem` (see `SDK_SCHEMA_IDB.MD`).
  - SDK also exposes:
    - `AppResponse<T>` (with `problem: RonProblem?`),
    - A small `RonSdkError` enum for local issues (`config`, `encoding`, `transport`).

* [x] **Message hygiene**

  - `RonProblem.message` is always sourced from the server’s safe `message` field.
  - When the server returns non-JSON or malformed bodies, SDK:
    - Creates a synthetic `RonProblem` with `kind = "transport"`, `status = 0`, and a generic message like `"transport_error"`.
    - Does **not** embed raw response body in `message`.

* [x] **Separation of concerns**

  - For Swift:

    ```swift
    enum RonSdkError: Error {
        case config(String)         // Misconfiguration
        case transport(RonProblem)  // Wrapping canonical problem
        case decoding(String)       // JSON decoding issues
        case unknown
    }
    ```

  - `RonProblem`:
    - `code` ⇒ machine code,
    - `kind` ⇒ category (`auth`, `policy`, `validation`, `transport`, etc.),
    - `message` ⇒ human-safe string,
    - `correlationId`, `retryable`, `retryAfterMs`, etc. as meta.

* [x] **No stack-trace leakage by default**

  - Swift stack traces exist in debug, but:
    - `localizedDescription` of errors does not dump entire stack.
    - Only safe fields (message, code, correlationId) surface to user-facing contexts.
  - For CLI / server logs:
    - If we log `error`, we rely on structured logging and optional debug flag; we don’t inject raw internals into responses.

### Logging

* [x] **Headers scrubbing**

  - Logging hook is aware of “sensitive headers”:
    - `Authorization`, `Cookie`, `X-RON-*` auth headers, etc.
  - These are replaced with `"(redacted)"` in any debug output.

* [x] **Body scrubbing**

  - By default, bodies are **not logged**.
  - If a debug mode is enabled (e.g., `RonConfig.debug = true`):
    - Logging is still capped (size-limited).
    - For JSON bodies, we may log **structure only** (keys, approximate sizes) rather than full values.
    - Strong warning in docs: “Do not enable debug logging in production.”

* [x] **Correlation IDs**

  - `RonClient` supports correlation IDs:
    - Either from server (`X-RON-Correlation-Id`) or generated locally.
  - Logs include correlationId to tie together client/server traces without exposing data payloads.

---

## 4) DoS & Resource Use

**Goal:** SDKs are good cluster citizens; they don’t accidentally DDoS RON-CORE.

* [x] **Response size caps (logical)**

  - Recommended patterns:
    - Use pagination; avoid endpoints returning large unbounded lists.
  - Swift SDK:
    - `listAll` helpers (once implemented) will:
      - Respect server-side `page_size` caps,
      - Stop when `next_page_token` is nil,
      - Optionally accept explicit max item limits.

* [x] **Streaming APIs guarded**

  - Streaming (SSE/WebSocket) is **not** part of MVP.
  - When added:
    - Streaming APIs will be opt-in,
    - Will allow specifying:
      - Max events,
      - Idle timeouts,
      - Manual cancellation.

* [ ] **Concurrency settings**

  - MVP: We rely on URLSession’s internal connection/concurrency limits.
  - Future:
    - `RonConfig` might gain `maxConcurrentRequests` or similar.
    - Helpers to run calls through a simple semaphore or TaskGroup to cap concurrency.

* [ ] **Backoff on error (429/503)**

  - MVP: No built-in backoff; app code decides.
  - Future:
    - Retry helper that:
      - Inspects `RonProblem` (`code`, `kind`, `retryable`, `retryAfterMs`),
      - Sleeps/backoffs before re-issuing calls.

---

## 5) “Browser” Checklist, Mapped to Apple Client Apps

**Goal:** Running inside iOS / iPadOS / macOS UI apps without betraying users.

In Swift, “browser SDK” maps to **client-side apps** (mobile/desktop UI):

* [x] **No raw caps in insecure storage**

  - Docs strongly recommend:
    - Keychain (iOS/macOS) or secure enclave-backed storage for tokens.
  - We **never** show examples that store caps in `UserDefaults` or arbitrary files.
  - `ron-app-sdk-swift` itself does not persist tokens; that responsibility is left to app code with strong guidance.

* [x] **CORS-safe / app-safe defaults**

  - Not strictly CORS (since we are not running in a browser), but:
    - We document required gateway settings for mobile/desktop apps:
      - Allowed origins, cross-site security expectations are less relevant, but TLS is mandatory.
  - All examples use `https://` base URLs; no mixed-content or downgrade patterns.

* [x] **CSRF-aware patterns**

  - CSRF is primarily a browser cookie problem; native apps are typically token-based.
  - Docs recommend:
    - Use bearer-style tokens or RON capabilities, not ambient cookies.
    - Avoid bridging user-browser cookies into mobile apps.

* [x] **XSS-resistant patterns**

  - Native UI is less prone to XSS, but:
    - We emphasize: treat all server data as untrusted when rendering in WebViews / HTML contexts.
    - No examples show direct injection of server HTML into WKWebView without sanitization.

* [ ] **SRI hooks for static bundles**

  - Hooks for SRI (Subresource Integrity) are mainly for JS bundles in facet static hosting.
  - For Swift, we may:
    - Offer helpers to validate `integrity` fields from facet manifests before using downloaded assets.
  - Not part of MVP.

---

## 6) Backend-Specific Checklist (Server-side Swift / CLI)

**Goal:** Hard to shoot yourself in the foot, even in dynamic/noisy environments.

* [x] **Safe config patterns**

  - README patterns:
    - Base URL from `RON_SDK_GATEWAY_ADDR`.
    - Caps/tokens from env or secret management systems, not hard-coded in source.
    - Timeouts from `RON_SDK_*_TIMEOUT_MS` env vars or explicit config.

* [ ] **Web framework middleware (Vapor, etc.)**

  - Future: Provide small integration helpers:
    - Logging correlation IDs per request.
    - Binding `RonClient` into Vapor’s `Request` container with appropriate lifetime.
  - These helpers should:
    - Enforce request body size limits,
    - Encourage short timeouts for upstream calls.

* [x] **Server lifecycle integration**

  - `RonClient` is safe to:
    - Construct during application startup,
    - Share across route handlers using dependency injection.
  - Shutdown:
    - `close()` is a no-op or gracefully cancels outstanding requests.

---

## 7) Testing & Verification

**Goal:** We don’t just hope it’s safe; we prove a lot of it.

* [x] **Unit tests**

  - Confirm:
    - Caps are not included in log strings (explicit tests for logging hooks).
    - Error messages do not contain raw response bodies or internal stack traces.
    - Timeouts cause:
      - Request cancellation,
      - Synthetic `RonProblem` with `kind = "transport"` and `status = 0`.

* [ ] **Property tests / fuzz**

  - Using SwiftCheck (or similar) for:
    - Idempotency key helpers:
      - Same input (operation + natural key) ⇒ same idempotency key,
      - Different inputs ⇒ high probability of different keys.
    - Header construction:
      - Fuzz header maps and ensure no capability-like substrings leak into logs.
    - Error decoding:
      - Fuzz valid/invalid JSON error payloads to ensure SDK never crashes and always yields a safe error.

* [x] **Integration tests**

  - In CI or local scripts:
    - Start a test Micronode/Macronode.
    - Run Swift interop tests:
      - Valid caps ⇒ success path.
      - Invalid caps ⇒ proper 401/403 RonProblem with non-leaky message.
      - Misconfigured base URL / TLS error ⇒ synthetic `transport` problems.

* [ ] **Negative tests**

  - Giant payloads:
    - Ensure decode errors are graceful (no OOM via unbounded allocations).
  - Broken caps:
    - Validate correct error codes and `kind = "auth"` or `kind = "policy"`.

---

## 8) Docs & DX

**Goal:** Secure by default, even if devs only copy-paste.

* [x] **Quickstarts show secure patterns**

  - Use env vars or config structs for:
    - `RON_SDK_GATEWAY_ADDR`,
    - tokens pulled from Keychain or secrets.
  - Use `https://` for all examples.
  - Show timeouts via `RonConfig` even in simple snippets.

* [x] **Explicit “don’t do this in prod” callouts**

  - Any example that:
    - uses `allowInsecureHttp = true`,
    - prints full responses,
    - uses long-lived tokens,
    - or stores caps in memory for debugging,
    - must have a **big, loud** “DEV ONLY” banner in comments and docs.

* [x] **Threat model call-out**

  - README section for Swift SDK states:
    - Assumes:
      - Node is reachable over TLS on a trusted network.
      - Client app process is trusted (no attacker controlling the binary).
    - Warns:
      - Do not embed permanent admin caps in mobile binaries.
      - Do not share `RonClient` with untrusted plugin modules without re-scoping caps.

---

## 9) Future / “God-Tier” Hooks

Not required for first release, but keep hooks ready in design:

* [x] **PQ toggle hooks**

  - `RonConfig` reserves a `pqMode: PQMode = .off` field.
  - When RON-CORE + Swift TLS stacks support PQ hybrids, we can wire this into URLSession or a custom transport without breaking API.

* [ ] **Audit integration**

  - Future: allow apps to set:
    - `actorId`,
    - `auditContext`,
    - or similar on `RonConfig`/call options.
  - SDK will propagate these as headers so `ron-audit` can log them.

* [ ] **Signed config/caps**

  - Leave room in DTOs/config for:
    - Signed caps (`signed_cap` field),
    - Signed facet manifests (TOML signature metadata).
  - When available, Swift SDK will:
    - Validate signatures (if configured),
    - Surface verification results in `RonProblem.details` on failure.

* [ ] **Formal methods**

  - Keep auth, idempotency, and error-mapping logic in small, testable modules:
    - Future: these modules can be modeled or fuzzed more heavily.
  - For now, ensure functions are:
    - Pure where possible,
    - Easy to reason about and property-test.

---

## 10) Schema & Security (Interop Guarantees)

**Goal:** Security properties (no leaks, bounded calls, correct auth handling) are preserved identically across SDKs via **shared schemas and envelopes**, not ad hoc per language.

### 10.1 Canonical Error Envelope

All SDKs (including Swift) must treat the **canonical error envelope** (as defined in `SDK_SCHEMA_IDB.MD`) as the source of truth:

```json
{
  "code": "string",
  "message": "string",
  "kind": "string",
  "correlation_id": "string",
  "details": { }
}
````

Plus optional `retryable`, `retry_after_ms`, and `reason`.

* [x] Swift SDK MUST attempt to parse error responses into `RonProblem` using this shape.
* [x] If parsing fails (non-JSON or schema drift):

  * SDK emits a synthetic `RonProblem` with:

    * `kind = "transport"`,
    * `status = 0`,
    * safe `message`.
* [x] User-visible surfaces expose:

  * `code`, `kind`, `message`, `correlationId` and sanitized `details` (if any).
* [x] Raw HTTP bodies and headers are exposed only:

  * In explicitly opt-in debug/diagnostic APIs, and
  * With redaction/scrubbing applied.

### 10.2 DTO & Auth Field Consistency

* [x] DTOs (request/response types) follow canonical schemas from `ron-proto` / OpenAPI:

  * Same field names and JSON structure.
  * Type mappings follow `SDK_SCHEMA_IDB.MD` (e.g., `u64` as JSON string decoded to `UInt64` via wrapper).

* [x] Auth fields are consistent:

  * `Authorization` header usage mirrors other SDKs.
  * Passport/cap headers (`X-RON-Passport`, etc.) are identical.
  * SDK never copies caps into DTO payloads or error structures.

* [x] SDK never serializes tokens inside DTOs:

  * Tokens are strictly part of transport-layer headers, not user data.

### 10.3 Facet Manifest Security

If `ron-app-sdk-swift` helps generate facet manifests (future):

* [x] It MUST emit TOML matching the shared schema (`SDK_SCHEMA_IDB.MD`):

  * `facet.id`, `facet.kind`,
  * `route.method`, `route.path`,
  * optional `facet.security`, `facet.limits`, `route.integrity`.

* [x] It MUST respect security hints:

  * No silent `public = true` defaults.
  * No implicit `requires_auth = false` just for convenience.

* [x] It MUST validate paths:

  * For `kind = "static"`, file paths must be relative to an allowed root; no `../` escapes.

* [ ] For future `kind = "proxy"`:

  * SDK facet builder must require:

    * Explicit upstream `host`, `port`, `scheme`.
  * Strongly discourage open Internet upstreams without mTLS or ACLs.

### 10.4 Schema-Driven Validation in Tests

* [x] Unit tests:

  * Error parsing:

    * Known canonical error JSON fixtures decode to `RonProblem` with all expected fields.
  * DTO (de)serialization:

    * Round-trip tests for key DTOs; JSON snapshots match expected canonical shapes.

* [x] Interop tests:

  * Hitting a real node:

    * `GET /app/healthz` ⇒ decode into known DTO.
    * A forced auth failure ⇒ decode into `RonProblem` with `kind = "auth"` or `kind = "policy"`.
  * Validate that tests fail if schema drifts (e.g., missing fields, renamed keys).

* [ ] Property tests:

  * Fuzz error payloads to ensure:

    * SDK never crashes.
    * Malformed errors still return a safe `RonProblem`.
  * Fuzz header values to ensure:

    * Sensitive header names are always redacted in log strings.

### 10.5 Client vs Backend Surface

* [x] Client-side Swift (iOS/macOS):

  * Treat all DTOs as untrusted data when used to render UI.
  * Provide examples that sanitize or constrain dynamic content before showing it in WebViews/HTML.

* [x] Backend Swift / CLI:

  * Assume a more trusted environment, but:

    * Still avoid logging full DTOs when they may contain PII.
    * Provide hooks to integrate with structured logging and redaction pipelines.

---

```
```
