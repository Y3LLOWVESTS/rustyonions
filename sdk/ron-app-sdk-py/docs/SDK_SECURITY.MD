````md
---

# SDK SECURITY CHECKLIST — `ron-app-sdk-py`

> Applies to: **ron-app-sdk-py** (Tier-1 SDK, aligned with Rust/TS/Go SDKs).  
> Scope: Python backend services, Python facets, CLI tools, async + sync execution.

---

## RO:WHAT

Security hardening blueprint for the **Python** RON app SDK.

Defines the **minimum bar** to ship (`beta-ready`) and the **God-tier bar** for long-term security (PQ readiness, auditability, testability).

---

## RO:WHY

SDKs are how apps actually talk to RON-CORE. If the Python SDK mishandles:

- macaroon/capability tokens,
- timeouts & retries,
- exceptions & error messages,
- logging,
- concurrency and streaming,

…then Micronode/Macronode hardening gets bypassed or weakened.

This checklist ensures `ron-app-sdk-py`:

- does **not leak secrets**,
- does **not weaken auth**,
- behaves **bounded and predictable** under failure,
- integrates cleanly with **future PQ / audit / signed caps** work.

---

## RO:INVARIANTS (Python Edition)

Across all SDKs — and **especially** `ron-app-sdk-py`:

- No SDK logs secrets (caps, macaroons, tokens, Authorization, Cookie, etc.).  
- No SDK stores secrets in exceptions, stack traces, `repr()`, `__str__()`, or logs.  
- All network calls are **bounded** (timeouts, backoff, max retries).  
- Capabilities are **least-privilege**, **short-lived**, and **rotatable**.  
- Errors are **structured** (canonical Problem envelope), not raw stack traces.  
- Debug/trace modes are **explicit opt-in** and still scrub secrets.  
- No global singletons holding caps: **each `RonClient` instance is isolated** and multiprocessing-safe.

---

## 0) TL;DR CHECKLIST (Python-Specific)

Before calling `ron-app-sdk-py` “beta”:

- [ ] **Auth & caps**  
  - Caps stored only in memory (no disk).  
  - Caps wrapped in Pydantic `SecretStr` for automatic redaction.  
  - No caps in logs, errors, or stack traces.  
  - Rotation supported via `token_provider` callbacks.

- [ ] **Transport**  
  - HTTPS/TLS enforced by default; HTTP allowed only with explicit `allow_insecure_http=True`.  
  - Connect/read/write/overall timeouts enforced for every httpx call.  
  - Bounded retries (e.g., max 3) with jitter/backoff; only for safe/idempotent operations.

- [ ] **Errors & logging**  
  - Canonical Problem envelope parsed via Pydantic (`extra='forbid'`).  
  - Structured error types (`RonProblem`, `RonNetworkError`, etc.).  
  - Header/body scrubbing for sensitive values.  
  - Correlation IDs exposed, not secrets.

- [ ] **DoS safeguards**  
  - Reasonable default max response size (e.g., ~16 MiB).  
  - Streaming (SSE) is explicit opt-in and guarded with backoff and disconnect detection.  
  - Optional concurrency limits per client (semaphore).

- [ ] **Backend patterns**  
  - Config via env (`RON_APP_URL`, `RON_APP_TIMEOUT`, `RON_APP_TOKEN`) following CONFIG.md prefixes.  
  - No module-level globals for caps; per-request/per-process client patterns recommended.

- [ ] **Tests**  
  - Unit tests: no cap leaks, timeouts enforced.  
  - Property tests (Hypothesis): header/DTO/error robustness, idempotency keys.  
  - Chaos tests (Toxiproxy): network flakiness, half-open, latency.  
  - Multiprocessing tests: secrets isolated across processes.

- [ ] **Docs**  
  - Quickstarts use env vars for caps.  
  - No examples hardcode caps or plain HTTP for prod.  
  - Clear “don’t do this in prod” callouts.

- [ ] **Future hooks**  
  - PQ mode stub (`tls_pq_mode="off" | "hybrid"`, currently “off”).  
  - Optional `audit_context` hooks.  
  - Room for signed caps/manifests without breaking APIs.

---

## 1) Capabilities & Auth (ALL SDKs, Python-Specialized)

**Goal:** Treat caps/macaroons like cryptographic plutonium.

### 1.1 Design

- [ ] **No ambient/global caps**  
  - `RonClient` requires explicit `token` / `macaroon` / `token_provider` per-instance.  
  - No hidden module-level variables storing caps.

- [ ] **Least privilege by default**  
  - All docs and examples use **minimal scope** caps, not `*`.  
  - Encourage “one facet / one cap” patterns.

- [ ] **Short-lived caps**  
  - Examples emphasize TTL in minutes (or hours), not days/weeks.  
  - Docs recommend rotating caps frequently for long-running services.

- [ ] **Rotation**  
  - Provide a standard `token_provider: Callable[[], str]` or async variant, so caps can be refreshed automatically.  
  - Client implementation must be safe to swap caps mid-flight (for future requests only).

### 1.2 Handling & Storage

- [ ] **No caps in logs**  
  - Logging filter must scrub any cap-like value from headers or DTOs.  

- [ ] **No caps in errors**  
  - `RonProblem` and other error types never embed caps.  
  - Network errors (e.g., 401/403) return sanitized messages.

- [ ] **No caps in exceptions/tracebacks**  
  - If an exception is raised due to auth, it uses sanitized messages.  
  - We do not append raw request headers to exception text.

- [ ] **No caps in URLs**  
  - Strictly reject or warn if caps are passed as query parameters.  
  - SDK never constructs URLs with caps in query strings.

- [ ] **No persistent cleartext cap storage**  
  - SDK never writes caps to disk (config files, logs, caches).  
  - If user persists caps, it’s an explicit choice outside SDK.

- [ ] **Use Pydantic `SecretStr` for caps**  
  - Any internal data model storing caps uses `SecretStr` so `repr()` and `str()` auto-redact.

- [ ] **Multiprocessing safety**  
  - Docs instruct: in multiprocessing, pass caps via process-safe channels (`multiprocessing.Queue`/Manager) rather than module-level globals or shared memory.  
  - SDK itself does not try to share `RonClient` instances across processes.

### 1.3 API Shape

- [ ] Provide helpers so user code rarely touches raw caps:

  ```python
  client = RonClient(
      base_url=...,
      token="cap:....",  # or token_provider=...
  )
````

* [ ] `RonClient` exposes:

  * `close()` to drop references to caps and close connections
  * `zeroize()` best-effort method to overwrite cap buffers.

* [ ] **Zeroization guidance**

  * `zeroize()` should overwrite any internal buffers and drop references.
  * Docs should note Python cannot guarantee perfect wiping due to GC and copies.
  * For high-sensitivity deployments, suggest pairing with libraries like **`zeroize-python`** (or equivalent) for stronger guarantees, while acknowledging interpreter limits.

---

## 2) Transport & Timeouts

**Goal:** Every SDK call is bounded and safe.

### 2.1 HTTPS/TLS

* [ ] **TLS required by default**

  * `base_url` must be `https://` in production.
  * `http://` allowed only when `allow_insecure_http=True` set explicitly (e.g., for local dev).

* [ ] **TLS verification on**

  * `verify=True` by default for httpx.
  * Allow custom CA bundles for corporate PKI (optional parameter or ENV).

* [ ] **PQ toggle stub**

  * Configurable `tls_pq_mode: Literal["off", "hybrid"]` in `RonClientConfig`.
  * Currently “off”; “hybrid” reserved for future PQ-KEM support.

### 2.2 Timeouts

* [ ] **Mandatory timeouts** for every request:

  * Connect timeout
  * Read timeout
  * Write timeout
  * Overall request timeout

* [ ] Reasonable defaults (e.g., connect 3s, read 10s) and overridable per-client/per-call.

### 2.3 Retries & Idempotency

* [ ] **Bounded retries**

  * Max retries at a small integer (e.g., 3).
  * Jittered exponential backoff (e.g., base 100–200ms).
  * No infinite loops.

* [ ] **Safe retry policy**

  * Retries allowed on GET and other clearly idempotent operations by default.
  * For writes (POST/PUT/PATCH), retry only if an explicit idempotency key is used.

* [ ] **Idempotency helpers**

  * `await client.post("/x", json=data, idem=True)` attaches an idempotency key header (UUID v4 by default).
  * Option to provide custom idempotency key function.

### 2.4 Asyncio Best Practices

* [ ] No long-running busy loops on the event loop.
* [ ] Use `asyncio.create_task` and `asyncio.Semaphore` (or anyio equivalents) for concurrency.
* [ ] No blocking sync calls inside async context — use `anyio.to_thread.run_sync` when needed.
* [ ] Clean cancellation handling for long-running operations (SSE/subscriptions).

---

## 3) Errors & Logging

**Goal:** Helpful to developers, useless to attackers.

### 3.1 Error Model

* [ ] **Canonical error envelope** (from `SDK_SCHEMA_IDB.MD`):

  ```json
  {
    "code": "string",
    "kind": "string",
    "message": "string",
    "correlation_id": "string",
    "details": {}
  }
  ```

* [ ] Implemented as Pydantic model (`RonProblem`):

  * `extra='forbid'` so unknown fields are ignored or rejected as per schema.
  * Used across SDK for server-side problem responses.

* [ ] If parsing fails (non-JSON or malformed JSON):

  * Wrap in `RonTransportError` or `RonNetworkError` with **safe**, high-level message.
  * Raw body not exposed unless explicit debug mode is enabled.

### 3.2 Message Hygiene

* [ ] Human message: high-level, safe, localizable.
* [ ] Machine code/kind: stable, for programmatic handling.
* [ ] `correlation_id`: included when available (for tracing/log correlation).
* [ ] `details`: structured but sanitized; no secrets or full payload dumps.

### 3.3 Logging

* [ ] Use Python `logging` or `structlog` with structured fields.

* [ ] **Headers scrubbing** — never log:

  * `Authorization`
  * `Cookie` / `Set-Cookie`
  * `X-Api-Key`
  * Any configured “secret-like” header names.

* [ ] **Body scrubbing** — if debug mode logs bodies:

  * Apply regex-based redaction for cap-like patterns.
  * Provide hook so users can plug their own PII redaction logic.

* [ ] Prefer logging:

  * request/response direction,
  * correlation ID,
  * status code,
  * retry counters,
  * durations,

  over raw payload contents.

---

## 4) DoS & Resource Use

**Goal:** SDK is a good citizen; it doesn’t accidentally DDoS RON-CORE or its host.

### 4.1 Response Size & Pagination

* [ ] Default max response size per call (e.g., ~16 MiB) with overridable config.
* [ ] Encourage paginated list APIs (SDK can provide helpers for `page`/`limit`).
* [ ] If server returns more than max size, SDK fails cleanly with a specific error.

### 4.2 Streaming/SSE Guardrails

* [ ] Streaming subscription requires explicit call:

  ```python
  async for event in client.subscribe("topic"):
      ...
  ```

* [ ] No automatic “subscribe all streams” or similar footguns.

* [ ] Automatic disconnect detection and optional backoff/reconnect logic with bounded attempts.

* [ ] Ability to set a max duration or max events per subscription.

### 4.3 Concurrency & Backpressure

* [ ] Optional per-client concurrency limit:

  ```python
  client = RonClient(..., max_concurrency=10)
  ```

* [ ] Implementation uses `asyncio.Semaphore`/anyio for concurrency control.

* [ ] No unbounded task creation; all tasks are tracked and cancellable.

### 4.4 Backoff on Error

* [ ] On 429 or server “slow down” signals, SDK waits before retrying.
* [ ] On persistent 5xx/connection failures, SDK stops retrying after configured max.

---

## 5) Browser-Specific Checklist

Python SDK is **backend-only** (no browser runtime).
This section applies only if we ever embed Python in the browser (Pyodide/WebAssembly); for now:

* [ ] Explicit statement in README: `ron-app-sdk-py` is not intended for untrusted browser contexts.

If a browser variant ever appears:

* Avoid persistent cleartext caps (no `localStorage`/`sessionStorage` by default).
* Mirror TS SDK’s CORS/CSRF guidance.
* Treat all DOM usage as untrusted data sinks.

---

## 6) Backend-Specific Checklist (Python)

**Goal:** Safe defaults for typical Python backends (FastAPI, Flask, Django, CLI).

### 6.1 Config Patterns

* [ ] Use environment variables for core settings:

  * `RON_APP_URL`
  * `RON_APP_TIMEOUT`
  * `RON_APP_TOKEN`

  following conventions in `CONFIG.md` (same prefixes used across RON-CORE).

* [ ] Explicitly discourage reading caps from config files in docs.

* [ ] Optional `.env` support (via `python-dotenv`) clearly marked as **dev convenience only**.

### 6.2 Framework Adapters

* [ ] **FastAPI**:

  * Provide a dependency/helper that yields a `RonClient` per request or per app.
  * No global caps stored at module import time.
  * Example: `get_client()` that uses env-driven config.

* [ ] **Flask**:

  * Optional extension to create `RonClient` on app init or per-request, not as global.

* [ ] **Django**:

  * Provide patterns for attaching a client to the request or app registry;
  * Avoid storing caps in Django settings as plain strings (document alternatives like env + secrets managers).

### 6.3 Runtime Considerations

* [ ] Multiprocessing: treat each process as owning its own `RonClient` instances.
* [ ] Async contexts: clean shutdown, closing httpx clients and subscriptions gracefully.
* [ ] No unbounded thread pools; use `anyio.to_thread.run_sync` and `max_workers` caps if threads are used.

---

## 7) Testing & Verification

**Goal:** We don’t just hope it’s safe; we prove a lot of it.

### 7.1 Unit Tests

* [ ] Ensure caps never appear in stringified logs or exception messages.
* [ ] Ensure redaction filters cover `Authorization`, `Cookie`, etc.
* [ ] Ensure timeouts config is actually used by httpx client.
* [ ] Ensure idempotency key headers are attached correctly when `idem=True`.

### 7.2 Property Tests / Fuzzing

Using **Hypothesis**:

* [ ] Fuzz header values to ensure no caps accidentally mirrored or logged.
* [ ] Property-test idempotency helpers: same inputs → same key, random inputs → high collision resistance expectations.
* [ ] Fuzz Problem envelopes with unknown fields to ensure robust parsing and no crashes.

### 7.3 Chaos / Network Tests

Using **Toxiproxy** or equivalent:

* [ ] Latency injection: verify timeouts behave as expected.
* [ ] Half-open connections: verify clean error handling.
* [ ] Intermittent packet drops: verify retry policy and no infinite loops.

### 7.4 Integration & Negative Tests

* [ ] Integration with local Micronode/omnigate:

  * Invalid caps → 401/403 with proper Problem envelope, no leaks.
  * Large responses near max size → bounded behavior.
  * Streaming disconnects → clean cancellation or reconnect with limits.

* [ ] Negative tests:

  * Garbled JSON body —> fails with safe `RonTransportError`.
  * Gigantic payload —> does not OOM; fails with safe “too large” error.
  * Non-JSON error body —> Problem parsing fails gracefully.

### 7.5 Multiprocessing/Concurrency Tests

* [ ] Test patterns with `multiprocessing` or `gunicorn`-style model:

  * Ensure no shared global client with caps across processes.
  * Validate that caps are not accidentally logged from child processes.
  * Validate concurrency guard (max_concurrency) under stress.

---

## 8) Docs & DX

**Goal:** Secure by default, even if devs only copy-paste.

### 8.1 Quickstarts & Examples

* [ ] All quickstart examples:

  * Load caps from env (`RON_APP_TOKEN`), not hardcoded.
  * Use `async with RonClient(...)` or `RonClientSync` for proper resource management.
  * Use HTTPS URLs in all examples; HTTP only in clearly marked “local dev only” blocks.

### 8.2 “Don’t do this in prod” Callouts

* [ ] For any sample using:

  * plain HTTP (`http://localhost:8080`),
  * hardcoded caps,
  * `allow_insecure_http=True`,

  there must be a bold note: **“Dev only; never do this in production.”**

### 8.3 Threat Model Section

In `README.md` for `ron-app-sdk-py`:

* [ ] Explain assumptions:

  * Running in a trusted backend / server context.
  * Local machine compromise is out-of-scope for SDK (but still discouraged from storing secrets in logs).

* [ ] Explain responsibilities:

  * SDK ensures network-level bounds and secret handling.
  * Application must manage its own local privilege separation and PII handling.

---

## 9) Future / “God-Tier” Hooks

Not required for the first release, but design must keep space for:

* [ ] **PQ toggle hooks**

  * `tls_pq_mode` in config, currently `"off"`, reserved for `"hybrid"` when Python TLS libs support PQ-KEM.

* [ ] **Audit integration**

  * Optional `audit_context` parameter (e.g., actor_id, reason, tenant) that travels in headers, so `ron-audit` can log operations meaningfully.

* [ ] **Signed caps/manifests**

  * Data structures and config leave room for adding signatures/verifiers without breaking API (e.g., `signed_cap`, `signature`, `alg` fields).

* [ ] **Formal methods / model checking (Rust side)**

  * Keep Python auth/idempotency logic conceptually aligned with small Rust core modules that can be model-checked; docs cross-link the invariants.

---

## 10) Schema & Security (Interop Guarantees)

**Goal:** Security properties are enforced the same way across SDKs by **shared schemas + envelopes**, not per-language improvisation.

### 10.1 Canonical Error Envelope

* [ ] `RonProblem` matches the canonical schema from `SDK_SCHEMA_IDB.MD`.
* [ ] SDK attempts to parse any error body as Problem; on failure, returns controlled transport error.
* [ ] User-facing exceptions expose safe `code`, `kind`, `message`, `correlation_id`, `details`.
* [ ] Raw HTTP bodies and headers only exposed in opt-in debug flows with scrubbing.

### 10.2 DTO & Auth Field Consistency

* [ ] DTOs for any exposed app-plane types follow canonical `ron-proto`/OpenAPI shapes:

  * Same field names.
  * Same JSON structure.
  * Type mappings per `SDK_SCHEMA_IDB.MD` (e.g., `u64` → Python `int` or string as specified).

* [ ] Auth headers:

  * `Authorization` usage uniform across SDKs.
  * Capability/macroon headers never copied into DTO payloads or logs.

* [ ] SDK **never** stores caps inside DTOs or error envelopes; caps live in client/session state only.

### 10.3 Facet Manifest Security (Future Python Facet Builder)

If/when `ron-app-sdk-py` helps generate facet manifests (TOML):

* [ ] Manifests must follow the shared facet schema from `SDK_SCHEMA_IDB.MD`:

  * `facet.id`, `facet.kind` (`static`, `echo`, future `proxy`),
  * `route.method`, `route.path`,
  * optional `facet.security`, `facet.limits`, `route.integrity`, etc.

* [ ] Respect security hints:

  * Never silently set `public = true` or `requires_auth = false`.
  * Never generate paths escaping the configured static root (no directory traversal).

* [ ] Proxy facets:

  * Require explicit upstream configuration (host/port/protocol).
  * Do not default to open proxy behavior.
  * Encourage mTLS or restricted network scopes where appropriate.

### 10.4 Schema-Driven Validation in Tests

* [ ] Unit tests:

  * Confirm Problem parsing for canonical and edge-case error bodies.
  * Confirm DTO serialization/deserialization matches golden fixtures.

* [ ] Interop tests:

  * Verify that invalid caps produce the same structured auth errors across Rust/TS/Python SDKs.
  * Verify unknown fields in responses do not crash the SDK or leak internals.

* [ ] Property tests:

  * Fuzz Problem envelopes and confirm no uncaught exceptions or leaked internal stack traces.
  * Fuzz header values and confirm caps never appear in user-visible error fields.

### 10.5 Backend vs Browser Surface

Since `ron-app-sdk-py` is backend-only:

* [ ] Treat all inbound DTOs as untrusted; docs suggest sanitization before logging or exposing to users.
* [ ] Encourage redaction and structured logging for any PII fields.
* [ ] Provide hooks to integrate with redaction middleware or logging pipelines.

---

```
```
