

````md
---

# SDK SECURITY CHECKLIST — `ron-app-sdk-*`

> Applies to: `ron-app-sdk` (Rust), `ron-app-sdk-ts`, `ron-app-sdk-go`, future SDKs.
> Scope: both **backend** (Node/Go/Rust services) and **frontend** (browser) variants.

---

## RO:WHAT

Security checklist for all RON app SDKs (Rust, TS, Go, etc.).
Defines **minimum bar** for “ship” and **stretch items** for “God-tier”.

---

## RO:WHY

SDKs are how apps *actually* talk to RON-CORE. If SDKs are sloppy with:

* capabilities (macaroons/tokens),
* error messages,
* logging,
* timeouts & retries,

…then all of the hardening in Micronode / omnigate gets bypassed.

This checklist ensures each SDK:

* doesn’t leak secrets,
* doesn’t weaken auth,
* behaves predictably under failure,
* and is ready to plug into future PQ / audit / formal methods work.

---

## RO:INVARIANTS

Across **all** SDKs:

* No SDK logs secrets (caps, raw headers, private keys).
* No SDK “helpfully” retries in ways that violate idempotency.
* All SDK network calls are **bounded** (timeouts, backoff, cap on retries).
* Capabilities are **least-privilege**, **short-lived**, and **rotatable**.
* Error surfaces are **structured**, not random stack traces with secrets.
* Debug/trace modes are **explicitly opt-in** and still scrub secrets.

---

## 0) TL;DR CHECKLIST

Before calling any SDK “beta”:

* [ ] **Auth & caps**: safe storage, no logging, refresh flows, least-privilege scopes.
* [ ] **Transport**: HTTPS/TLS only, sane timeouts, bounded retries, explicit backoff.
* [ ] **Error handling**: structured errors, no secrets in messages or stack traces.
* [ ] **Logging**: scrub headers/body; log correlation IDs + error codes only.
* [ ] **DoS safeguards**: response size caps, concurrency limits, no unbounded streaming without guard rails.
* [ ] **Browser variant**: no long-term cap storage, CORS/CSRF compatible flows, XSS-resilient patterns.
* [ ] **Tests**: property tests for idempotency, unit/integration tests for failure paths, basic fuzzing where possible.
* [ ] **Docs**: show secure usage by default (no “quickstart” that hardcodes tokens, etc.).
* [ ] **Hooks for future**: fields/flags reserved for PQ, audit IDs, and signed manifests/caps.

---

## 1) Capabilities & Auth (ALL SDKs)

**Goal:** Caps/macaroons are treated like cryptographic plutonium.

### Design

* [ ] **No ambient caps**:

  * The SDK requires explicit cap configuration per client instance, not global variables.
* [ ] **Least privilege by default**:

  * Default examples use **minimal** scopes (e.g., “bucket:todo:read/write”), not “*”.
* [ ] **Short-lived caps**:

  * SDK supports and encourages short TTL caps (e.g., minutes, not days).
* [ ] **Rotation**:

  * Provide a standard way to rotate caps:

    * e.g. a `CapProvider` or callback that can refresh tokens.

### Handling & Storage

* [ ] **No caps in logs** (ever).
* [ ] **No caps in error messages**.
* [ ] **No caps in panic/exception messages**.
* [ ] **No caps in URLs** (query strings).
* [ ] **No persistent storage of caps in plaintext** from SDK internals:

  * Backend: Prefer environment variables or process memory; never write caps to disk by default.
  * Browser: Avoid `localStorage`/`sessionStorage` if possible; if unavoidable, strongly documented with warnings and rotate aggressively.

### API Shape

* [ ] Provide helpers so user code rarely touches raw caps:

  * e.g. `new RonClient({ capability: "..." })`, SDK handles headers.
* [ ] Consider a **“session” abstraction** that can be retired/zeroed.

---

## 2) Transport & Timeouts

**Goal:** Every SDK call is bounded in time and resources.

* [ ] **TLS/HTTPS required**:

  * Disallow plain HTTP by default; require explicit “unsafe” flag for dev-only.
* [ ] **Timeouts mandatory**:

  * Every network call has:

    * connect timeout,
    * read timeout,
    * overall request timeout.
* [ ] **Retries bounded**:

  * Retries only on safe/idempotent operations (GET, maybe PUT with idempotency keys).
  * Bounded max retries (e.g., 3) with backoff.
  * No infinite retry loops.
* [ ] **Idempotency support**:

  * For write operations, **optional idempotency key** support baked in:

    * e.g., the Rust SDK already leans this way; TS/Go should mirror it.

---

## 3) Errors & Logging

**Goal:** Helpful to developers, useless to attackers.

### Errors

* [ ] **Structured error type**:

  * e.g. `{ kind, code, message, correlationId? }`, plus optional `details`.
* [ ] **Message hygiene**:

  * No raw server error bodies blindly dumped into error messages if they might contain stack traces or sensitive payloads.
* [ ] **Separation of concerns**:

  * Error has:

    * *human message* (safe, no secrets),
    * *machine code* (for programmatic handling),
    * *meta* (correlation ID, retryable yes/no, optional `details`).
* [ ] **No stack-trace leakage by default**:

  * For TS/Node: hide stack traces unless in explicit debug mode.
  * For other languages: keep full stack traces out of user-facing messages; reserve them for debug logs only.

### Logging

* [ ] **Headers scrubbing**:

  * Never log `Authorization`, `Cookie`, or anything that looks like a token.
* [ ] **Body scrubbing**:

  * Provide a flag/hook to scrub PII or token-like substrings if bodies are logged in debug mode.
* [ ] **Correlation IDs**:

  * SDK should support correlation IDs / request IDs and log those instead of sensitive contents.

---

## 4) DoS & Resource Use

**Goal:** SDKs are good cluster citizens; they don’t accidentally DDoS RON-CORE.

* [ ] **Response size caps**:

  * For APIs that can list blobs or datasets, enforce sensible default page sizes.
* [ ] **Streaming APIs guarded**:

  * For long streams (logs, SSE), require explicit opt-in API calls.
* [ ] **Concurrency settings**:

  * SDK supports global or per-client limits on concurrent requests.
* [ ] **Backoff on error**:

  * For 429 or gateway-level “slow down” signals, back off appropriately.

---

## 5) Browser-Specific Checklist (`ron-app-sdk-ts` frontend flavor)

**Goal:** Running in the browser without betraying users.

* [ ] **No raw caps in localStorage by default**:

  * Preferred: in-memory, short-lived tokens, refreshed via a secure flow.
* [ ] **CORS-safe defaults**:

  * Document which Micronode/omnigate CORS settings are expected for browser apps.
* [ ] **CSRF aware**:

  * Patterns for unsafe methods (`POST/PUT/DELETE`) that expect:

    * CSRF token headers, or
    * same-site cookie settings, depending on app’s auth scheme.
* [ ] **XSS-resistant patterns**:

  * Avoid patterns that encourage injecting server data directly into HTML without sanitization.
* [ ] **SRI hooks for static bundles**:

  * If SDK helps manage facet static assets, keep fields ready for SRI hashes.

---

## 6) Backend-Specific Checklist (Node/TS/Go/Rust server)

**Goal:** Hard to shoot yourself in the foot, even in dynamic languages.

* [ ] **Safe config patterns**:

  * Encourage env-based config with clear separation of:

    * caps,
    * base URLs,
    * timeouts.
* [ ] **Express/Fastify/Koa middlewares (TS)**:

  * Provide hardened middlewares:

    * request-ID injection,
    * input size limits,
    * sane defaults (no body parser for gigantic payloads).
* [ ] **Go handlers**:

  * Wrap `http.Client` with:

    * TLS + timeouts + context cancellation wired in.
* [ ] **Rust backend**:

  * Use same invariants as core Rust crates:

    * no panics in normal flow,
    * error types that integrate with tracing and metrics.

---

## 7) Testing & Verification

**Goal:** We don’t just hope it’s safe; we prove a lot of it.

* [ ] **Unit tests**:

  * Caps not logged.
  * Error messages scrubbed.
  * Timeouts enforced.
* [ ] **Property tests / fuzz**:

  * For idempotency helpers: same input → same idempotency key.
  * For header construction: never includes raw caps in unexpected places.
* [ ] **Integration tests**:

  * SDK talking to:

    * a local Micronode/omnigate,
    * intentionally misconfigured endpoints (simulate TLS errors, 5xx, timeouts).
* [ ] **Negative tests**:

  * Broken caps → proper auth errors, no leaks.
  * Giant payloads → fail predictably, not OOM.

---

## 8) Docs & DX

**Goal:** Secure by default, even if devs only copy-paste.

* [ ] **Quickstarts show secure patterns**:

  * Using env vars for caps.
  * Using timeouts, not bare `fetch` with unbounded waits.
* [ ] **Explicit “don’t do this in prod” callouts**:

  * For any example that uses plain HTTP or hardcoded caps.
* [ ] **Threat model call-out**:

  * One short section in each SDK README:

    * “Here’s what this SDK assumes (trusted host / untrusted browser).”
    * “Here’s what you must not do.”

---

## 9) Future / “God-Tier” Hooks

Not required for first SDKs, but keep hooks ready:

* [ ] **PQ toggle hooks**:

  * Config options reserved for PQ-hybrid mode when transport layer is ready.
* [ ] **Audit integration**:

  * Include a way to attach an `audit_context`/`actor_id` so ron-audit can log operations meaningfully.
* [ ] **Signed config/caps**:

  * Keep structure flexible so signed caps or signed facet manifests can be plugged in without breaking APIs.
* [ ] **Formal methods**:

  * For the Rust SDK, keep core auth & idempotency logic in small enough modules that we can later model them (TLA+/Alloy).

---

## 10) Schema & Security (Interop Guarantees)

**Goal:** Security properties (no leaks, bounded calls, correct auth handling) are preserved identically across SDKs via **shared schemas and envelopes**, not ad hoc per language.

### 10.1 Canonical Error Envelope

All SDKs must treat the **canonical error envelope** (as defined in `SDK_SCHEMA_IDB.MD`) as the source of truth, typically:

```json
{
  "code": "string",
  "message": "string",
  "kind": "string",
  "correlation_id": "string",
  "details": { }
}
````

* [ ] SDKs MUST attempt to parse error responses into this canonical shape.
* [ ] If parsing fails (e.g., non-JSON body), SDKs wrap the failure in a **generic transport error** with a safe message and optional debug metadata, never exposing raw bodies by default.
* [ ] User-visible errors (exceptions/results) expose:

  * `code`, `kind`, `message`, `correlationId`, and sanitized `details`.
* [ ] Raw HTTP bodies and headers are only exposed in explicitly opt-in debug paths, with scrubbing applied.

### 10.2 DTO & Auth Field Consistency

* [ ] DTOs (request/response types) follow the canonical schemas from `ron-proto` / OpenAPI:

  * Same field names,
  * Same JSON shapes,
  * Same types as mapped in `SDK_SCHEMA_IDB.MD` (e.g., `u64` as JSON string in TS).

* [ ] Auth fields (headers, JSON fields, cookies) are **consistent** across SDKs:

  * `Authorization` header usage is uniform,
  * Capability/macroon-bearing headers are never copied into DTO payloads or logs.

* [ ] SDKs **never** store capabilities/macaroons inside DTOs or error envelopes; they are tracked separately in client/session state.

### 10.3 Facet Manifest Security

If an SDK helps generate or manage facet manifests (TOML):

* [ ] It MUST emit manifests that follow the shared facet schema in `SDK_SCHEMA_IDB.MD`:

  * `facet.id`, `facet.kind` (`static`, `echo`, future `proxy`),
  * `route.method`, `route.path`,
  * optional `facet.security`, `facet.limits`, `route.integrity`, etc.

* [ ] It MUST respect security hints:

  * Never silently set `public = true` or `requires_auth = false` for convenience.
  * Never generate routes that map outside the configured facet file root (for `static`).

* [ ] For proxy facets, it MUST:

  * Require explicit upstream config (host/port/protocol),
  * Avoid “open proxy” defaults,
  * Recommend or enforce mutual auth or restricted networks where applicable.

### 10.4 Schema-Driven Validation in Tests

* [ ] Unit tests ensure:

  * Error parsing correctly handles the canonical envelope,
  * DTO serialization/deserialization matches the expected schema (e.g., snapshot tests or golden fixtures).

* [ ] Interop tests (per SDK_IDB §11) are also **security tests**:

  * Verify that incorrect caps produce the right structured auth error (401/403 with proper `code`/`kind`),
  * Verify that unknown fields in responses do not crash SDKs or leak secrets.

* [ ] Property tests where possible:

  * Fuzz error payloads and ensure the SDK never panics or leaks raw internals,
  * Fuzz header values and ensure caps are never echoed back.

### 10.5 Browser vs Backend Surface

* [ ] Browser-facing SDK variants MUST:

  * Avoid exposing raw schemas directly in the DOM or logs,
  * Treat all received DTOs as untrusted data (suitable for sanitization before rendering).

* [ ] Backend SDK usage MUST:

  * Assume a **trusted environment** but still avoid logging raw DTOs that may contain secrets or PII,
  * Offer hooks to integrate with redaction / structured logging pipelines.

---

```
```
