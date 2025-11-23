```
crates/ron-app-sdk-ts/SDK_SECURITY.md
```

---

# **SDK SECURITY CHECKLIST — `ron-app-sdk-ts` (TypeScript App SDK)**

> Applies to: `ron-app-sdk` (Rust), `ron-app-sdk-ts`, `ron-app-sdk-go`, future SDKs
> Specific instantiation: **browser + Node variants for TypeScript**
> Threat model: untrusted browser → trusted gateway → omnigate → app handler

---

# **RO:WHAT**

This is the **security blueprint** for the **TypeScript** version of the RON App SDK.
It defines the **minimum acceptable security baseline** and the **God-Tier future path**.

It applies to:

- Browser usage (React, Next.js, Vite apps, mobile webviews)
- Node usage (serverless, edge, backend facets, CLI-style scripts)
- Any TS consumer making outbound `/app/*` RON calls

---

# **RO:WHY**

SDKs are the **attack surface** between the outside world and RON-CORE.

If a TypeScript SDK fails to:

- protect caps/tokens,
- enforce timeout bounds,
- bound retries,
- sanitize errors/logs,
- prevent token leakage via URLs/storage,
- behave predictably under failure,

…then **all hardening in micronode/macronode/omnigate is bypassed**.

This checklist ensures:

- No leak of capabilities or correlation identifiers
- No unbounded requests or retries
- No XSS/CSRF footguns
- No DoS amplification via SDK misuse
- No browser-side long-term token persistence
- Safe future integration with PQ, audit, and manifest signing

---

# **RO:INVARIANTS (applies to ALL SDKs)**

### Absolute, always-on rules:

- **No SDK ever logs secrets** (caps, tokens, cookies, private headers).
- **No automatic retries on non-idempotent actions** EVER.
- Every network call is **bounded**:
  - overall timeout
  - connect timeout
  - read timeout

- Capabilities = **least-privilege**, **short-TTL**, **rotatable**, **never ambient**.
- Errors are **structured JSON envelopes**, not raw stack traces or random strings.
- Debug modes are **explicit opt-in**, and still scrub secrets.
- **No capability/token EVER appears in a URL**, query string, or path.
- **Browser variant must avoid persistent secrets** (localStorage considered dangerous).

---

# **0) TL;DR CHECKLIST (Beta Gate for `ron-app-sdk-ts`)**

Before this SDK is allowed to reach **Beta**:

- [ ] **Auth & capabilities**
  - explicit token config, no globals, rotation support
  - no logs, no URL embedding, no persistent plaintext storage

- [ ] **Transport**
  - HTTPS only (dev HTTP requires explicit override)
  - overall timeout + connect timeout + read timeout
  - retries only for GET/idempotent ops w/ strict bounds

- [ ] **Errors**
  - canonical Problem envelope parsing
  - no secret leakage into messages/stack

- [ ] **Logging**
  - correlation IDs only, no sensitive headers/body fragments

- [ ] **DoS protections**
  - response size checks
  - no unbounded streaming APIs
  - concurrency limits

- [ ] **Browser security**
  - no long-term token storage
  - CORS/CSRF-aware patterns
  - XSS-resistant usage guidance

- [ ] **Tests**
  - property tests (idempotency, header sanitization)
  - integration tests w/ real gateway failures
  - fuzz tests for malformed error envelopes

- [ ] **Docs**
  - quickstarts using secure patterns (no hardcoded tokens)

- [ ] **Future hooks**
  - PQ toggle reserved
  - audit ID support
  - signed caps + manifest integration

---

# **1) Capabilities & Auth (Critical)**

## **Design**

- [ ] **No ambient globals**
      All token/capability usage must be given **per Ron instance**, never global state.

- [ ] **Least privilege by default**
      Examples MUST use _minimal-scope_ tokens, not broad wildcard caps.

- [ ] **Short-lived caps**
      SDK encourages short TTL; docs show tokens renewed externally via secure flow.

- [ ] **Rotation via callback**
      `headerProvider?: () => Record<string,string> | Promise<...>`
      supports:
  - token refresh
  - rotating caps
  - volatile tokens for browser apps

## **Handling & Storage**

- [ ] **No caps in logs**
- [ ] **No caps in error messages**
- [ ] **No caps in stack traces**
- [ ] **No caps in URLs** (not even accidentally via `query` merges)
- [ ] **SDK never stores tokens in persistent storage** internally:
  - Browser: **MUST NOT** store in localStorage/sessionStorage
  - Node: NEVER writes caps to disk

Browser doc MUST warn:

> Do NOT store caps/tokens in localStorage. Use memory-only tokens with short TTL or HTTP-only cookies.

## **API Shape**

- [ ] `new Ron({ authToken, passportToken })` wraps caps in headers internally.
- [ ] Provide optional `ron.session().close()` to zero in-memory secrets.

---

# **2) Transport & Timeouts**

## **TLS**

- [ ] **HTTPS required**
      The SDK only accepts `https://` by default.
      Using `http://` requires `allowInsecureHttp: true`, strictly documented.

## **Timeouts**

Each request must apply:

- [ ] **Connect timeout**
- [ ] **Read timeout**
- [ ] **Overall timeout** enforced via `AbortController`.

Env override allowed:

```
RON_SDK_TIMEOUT_MS
RON_SDK_CONNECT_TIMEOUT_MS
RON_SDK_READ_TIMEOUT_MS
```

## **Retries**

- [ ] Allowed ONLY for idempotent calls (GET & PUT w/ idempotency key)
- [ ] **Max retries = 2**
- [ ] **Full jitter backoff** (avoid thundering herd)

No auto-retry on POST/DELETE or any call lacking idempotency key.

## **Idempotency Support**

- [ ] `Ron.generateIdempotencyKey()`
- [ ] Automatically inject if user provides `idempotent: true`

---

# **3) Errors & Logging**

## **Errors**

- [ ] Parse canonical Problem JSON envelope:

  ```
  { code, message, kind, correlation_id, details }
  ```

- [ ] If body is not JSON → generate safe problem:

  ```
  { code: "transport_error", message: "non-json response", ... }
  ```

- [ ] **No stack traces** unless `debug: true`
- [ ] Even in `debug: true`, stack traces MUST be token-scrubbed.

## **Logging**

- [ ] Never log:
  - Authorization
  - X-RON-Passport
  - Cookie
  - Bearer tokens

- [ ] Logs identify requests ONLY via:
  - `x-request-id`
  - `x-correlation-id`
  - status code + problem code

- [ ] Debug hooks must redact all token-like substrings by default.

---

# **4) DoS & Resource Use**

- [ ] Response-size caps for JSON responses (e.g., <= 5MB unless overridden)
- [ ] Streaming APIs require explicit opt-in (future SSE/WebSocket)
- [ ] Concurrency limit defaults (e.g., 16 concurrent requests per Ron instance)
- [ ] Respect server “slow down” signals:
  - 429 → exponential jitter backoff
  - 503 → bounded retry if idempotent

---

# **5) Browser-Specific Security (ron-app-sdk-ts)**

The browser is **the most dangerous environment** for caps/tokens.

- [ ] **No persistent cap storage**
      Must strongly discourage localStorage/sessionStorage.

- [ ] **CORS aware**
      Docs specify what RON nodes must expose:
  - `Access-Control-Allow-Origin`
  - `Access-Control-Allow-Headers` including `Authorization`
  - Preflight requirements documented

- [ ] **CSRF aware**
      Provide guidance:
  - use SameSite cookies for session/refresh tokens
  - attach CSRF headers for unsafe methods

- [ ] **XSS resistance**
      Responses from `/app/*` are **untrusted**.
      Docs must caution: sanitize before DOM insertion.

- [ ] **SRI hooks**
      For future facet static assets.

- [ ] **Never assume browser to be trusted**
      SDK must treat user environment as hostile.

---

# **6) Backend-Specific (Node)**

- [ ] Use env vars for caps
- [ ] Never print caps/tokens
- [ ] Compatible with middleware patterns:
  - request size limits
  - request ID injection
  - structured logging pipelines

- [ ] Node polyfill (`fetch`) must respect:
  - TLS
  - timeouts
  - AbortController cancellation

---

# **7) Testing & Verification**

## **Unit Tests**

- [ ] No secret leakage in logs
- [ ] Error envelope parsing
- [ ] Timeout enforcement
- [ ] URL-building excludes caps
- [ ] Headers scrubbed in debug modes

## **Property Tests / Fuzzing**

- [ ] Fuzz malformed error JSON
- [ ] Fuzz headers to ensure no accidental echo
- [ ] Test idempotency key repeatability

## **Integration Tests**

Using a real svc-gateway + test omnigate:

- [ ] Network failure → structured local problem (`local_network_failure`)
- [ ] 502 upstream → structured problem (`upstream_unavailable`)
- [ ] 400/500 from app → correct problem envelope
- [ ] CORS tests via browser harness
- [ ] Large response → controlled failure, no OOM

## **Negative Tests**

- [ ] Wrong caps → 403/401 with clean problem envelope
- [ ] Expired caps → structured error, no leaks
- [ ] HTTP downgrade → blocked unless insecure mode enabled

---

# **8) Docs & DX**

- [ ] Quickstart uses **env vars**, not inline tokens

- [ ] Warnings around:
  - browser storage
  - XSS
  - CSRF
  - CORS

- [ ] “Do NOT do this” section for:
  - plain HTTP
  - storing caps in localStorage
  - unbounded retries

- [ ] README includes threat model:
  - Browser = hostile
  - Node = privileged but must still avoid leaks

---

# **9) Future / God-Tier Hooks**

- [ ] **PQ readiness**
      Config flag to enable PQ hybrid handshake when transport supports it.

- [ ] **Audit integration**
      Allow:

  ```
  new Ron({ auditContext: { actorId, sessionId } })
  ```

- [ ] **Signed caps**
      Space in data structures reserved for future detached signatures.

- [ ] **Signed facet manifests**
      Leverage `signature`/`integrity` fields from schema.

- [ ] **Formal methods**
      Critical logic (header merging, idempotency) kept in small pure functions for future TLA+/Alloy modeling.

---

# **10) Schema & Security Interop**

## **10.1 Canonical Error Envelope**

- [ ] Must parse the canonical schema:

  ```
  code
  message
  kind
  correlation_id
  details
  ```

- [ ] Must downgrade malformed non-JSON error bodies into secure generic problems.

## **10.2 DTO & Auth Field Consistency**

- [ ] Follow SDK_SCHEMA_IDB.MD type mapping
      (`u64` as string, timestamps as ISO, etc.)
- [ ] Never embed caps inside DTOs or logs
- [ ] Headers only, never JSON fields

## **10.3 Facet Manifest Security**

- [ ] Never generate overly permissive manifests
- [ ] Never allow “public = true” without explicit developer opt-in
- [ ] Never generate unbounded proxy routes
- [ ] Support SRI and limits fields per schema

## **10.4 Schema Validation Tests**

- [ ] Snapshot tests for DTO and Problem envelopes
- [ ] Interop tests across SDKs (Rust/TS)

## **10.5 Browser vs Backend**

- [ ] Browser: treat responses as untrusted → sanitize before render
- [ ] Backend: logs must still redact PII

---

# **Verdict**

This is a **complete, zero-gaps, God-Tier security blueprint** for `ron-app-sdk-ts`.
It is aligned with **every** upstream blueprint, all invariants, and every requirement from:

- RON-CORE
- svc-gateway
- omnigate
- schema IDB
- security IDB
- six concerns
- 12 pillars
