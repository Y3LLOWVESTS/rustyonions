---

# IDB — `ron-app-sdk-ts` (TypeScript App SDK)

### SDK Tier & Profile

* **Tier:** **1**

  * Tier 1: Core/maintained (Rust, TS, Python, Go, Java, C#)—full rigor on tests, benchmarks, CI. 
* **Usage profile:**

  * [x] Front-end
  * [x] Backend/facet
  * [x] Dual-use
  * [ ] CLI/tools  (CLI helpers are future, not MVP)

This tier gates:

* §3.4 sandboxing expectations
* §4.3 idempotency helpers
* §5.3 benchmark rigor
* §10.3 property/chaos tests
* §9.3 CI/support levels 

---

### 0) RO:WHAT / WHY / INTERACTS

**RO:WHAT —**
`ron-app-sdk-ts` is the **TypeScript/JavaScript client SDK** for RON-CORE. It provides a high-level, ergonomic wrapper over the RON app plane:

````text
browser / Node app
  → https://<node>/app/*
  → svc-gateway
  → omnigate /v1/app/*
  → app handlers via ron-app-sdk-rs
``` :contentReference[oaicite:2]{index=2}

It exposes a `Ron` client with methods like `get`, `post`, and `request`, plus structured error handling and observability headers.

**RO:WHY —**

* TS/JS is the **primary language of the web**; most apps that talk to RON-CORE will have a browser or Node component.
* The SDK removes boilerplate around:
  * building `fetch` URLs and query strings,
  * setting RON headers and IDs,
  * parsing JSON vs raw bodies,
  * mapping RON Problem JSON into typed error values. :contentReference[oaicite:3]{index=3}
* It enforces **God-tier safety defaults** from the SDK security checklist:
  * no logging secrets,
  * bounded timeouts/retries,
  * structured errors instead of raw stack traces. :contentReference[oaicite:4]{index=4}

**RO:INTERACTS —**

Core RON-CORE components this SDK talks to (via HTTP):

* `svc-gateway` — primary entrypoint at `/app/*` (never `/v1/app/*`). :contentReference[oaicite:5]{index=5}
* `omnigate` — indirectly, via gateway’s `/v1/app/*`.
* `micronode` / `macronode` — node profiles behind the gateway (same HTTP contract).
* Future:
  * `svc-passport` / `ron-auth` for auth flows and caps/macaroons.
  * `svc-registry` for facet discovery and app manifests.
  * `svc-storage`, `svc-index` for data-plane APIs once exposed to app plane.

---

### 1) Scope & Target Profiles

**1.1 Supported usage modes** :contentReference[oaicite:6]{index=6}

* [x] Front-end only (browser / mobile via WebView)
* [x] Backend / facets (Node runtimes; serverless handlers)
* [x] Dual-use (browser + Node / server)
* [ ] CLI / tools (future: thin wrapper over `node` for simple scripts)

Explanation:

* **Browser:** primary target; must be CORS- and CSRF-friendly, never store long-lived caps in untrusted storage.
* **Node:** for backend app services and facet-like workers that want to call RON-CORE via HTTP instead of embedding Rust.
* CLIs are a nice-to-have on top of the Node story, not a separate design axis for MVP.

---

**1.2 Supported node profiles** :contentReference[oaicite:7]{index=7}

* [x] Micronode
* [x] Macronode
* [x] Both (default)

Assumptions:

* App-plane `/app/*` is uniformly exposed regardless of node profile; the SDK doesn’t care whether it’s talking to a micronode or macronode.
* Some **admin** / registry / facet-management APIs may be **macronode-only**; those belong to future helpers or a separate admin SDK.
* Docs must warn about:
  * writing persistent data to nodes in **amnesia mode** (micronode variants),
  * not assuming long-term state unless the node advertises persistence.

---

**1.3 Feature scope (MVP vs future)** :contentReference[oaicite:8]{index=8}

**MVP:**

* [ ] Auth (attach macaroon / token) — **basic pass-through**:
  * explicit `authToken` / `passportToken` option at client creation,
  * tokens attached to `Authorization` / `X-RON-Passport` headers only, with no storage. :contentReference[oaicite:9]{index=9}
* [x] Basic request/response calls
  * `ron.request`, `ron.get`, `ron.post`, `ron.put`, `ron.patch`, `ron.delete`. :contentReference[oaicite:10]{index=10}
* [ ] Streaming (SSE/WebSocket) subscribe
  * not in first commit; may be stubbed as “future” for TypeScript.
* [x] Error handling (idiomatic for TS)
  * `Promise` resolves to `AppResponse<T>` with `ok`, `status`, optional `problem`. :contentReference[oaicite:11]{index=11}
* [x] Minimal logging / tracing hooks
  * ability to plug in a callback for each request/response with redacted data.

**Future:**

* [ ] Schema-generated DTOs from `ron-proto` + HTTP/OpenAPI (via codegen) :contentReference[oaicite:12]{index=12}
* [ ] Auto-reconnect & backoff for streaming transports.
* [ ] Built-in metrics hooks (expose timing, request counters).
* [ ] Facet lifecycle helpers (register/heartbeat) once the facet-plane admin APIs are stable.
* [ ] Higher-level convenience APIs (domain-specific “recipes” for common patterns).

Also explicitly **future** per previous notes (do NOT burn time in MVP): :contentReference[oaicite:13]{index=13}

* app authentication flows (full passport/ron-auth dance),
* facet-generation codegen,
* multi-app tenancy,
* token refresh logic,
* retry policies beyond minimal “safe defaults”,
* websocket support.

---

### 2) API Design & DX Basics

**2.1 Primary abstraction** :contentReference[oaicite:14]{index=14}

Main entrypoint: a `Ron` client class.

```ts
import { Ron } from "ron-app-sdk-ts";

const ron = new Ron({
  baseUrl: "https://my-node.example.com",
  authToken: "Bearer <jwt-or-macaroon>",  // optional
  passportToken: "<cap>",                 // optional
});

const res = await ron.get<{ message: string }>("/hello");

if (res.ok && res.data) {
  console.log("Hello:", res.data.message);
} else {
  console.error("RON error:", res.status, res.problem);
}
````

- The snippet shows:
  1. Create a client.
  2. Make a simple call.
  3. Handle error via `ok`/`problem` instead of thrown exceptions.

**2.2 Core types / interfaces**

From the carry-over contract:

```ts
export interface AppRequest {
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  path: string; // "/hello", "/kv/put", "/auth/register"
  query?: Record<string, string>;
  headers?: Record<string, string>;
  body?: unknown; // automatically JSON-encoded if object
}

export interface RonProblem {
  code: string; // "upstream_unavailable", "problem_400", etc
  message?: string;
  retryable?: boolean;
  retry_after_ms?: number;
  reason?: string;
}

export interface AppResponse<T = unknown> {
  status: number;
  ok: boolean; // status in 200–299
  headers: Record<string, string>;
  data?: T; // parsed JSON if content-type = JSON
  raw?: ArrayBuffer; // raw body if non-JSON
  problem?: RonProblem; // present only for 4xx/5xx/0 with problem envelope
}
```

**Design invariants:**

- SDK never prefixes `/app` in `path`; `path` is relative (“/hello”), and the SDK builds `<baseUrl>/app/<path>` itself.
- The SDK **never** talks to `/v1/app/*`—that’s gateway → omnigate internal.
- Server-originated errors **never throw**; they return as `AppResponse` with `ok=false`. Only programmer errors (bad URL, invalid options) reject the promise.

**2.3 Method surface**

- `request<T>(req: AppRequest): Promise<AppResponse<T>>`
- `get<T>(path: string, query?: Record<string, string>): Promise<AppResponse<T>>`
- `post<T>(path: string, body?: unknown, query?: Record<string, string>): Promise<AppResponse<T>>`
- `put`, `patch`, `delete` in same style.

Later, we add `ron.facet("auth").login(...)`-style helpers on top of `request`.

---

### 3) Security & Capabilities (SEC)

**3.1 Threat model & scope**

We adopt the **SDK_SECURITY** checklist across all app SDKs:

Invariants:

- No SDK logs secrets (tokens, caps, private headers).
- No “helpful” unbounded retries—especially not on non-idempotent calls.
- All network calls are bounded (timeouts, capped retries).
- Capabilities are treated like cryptographic plutonium—least privilege, short-lived, rotatable.
- Errors are structured; stack traces never include secrets.
- Debug/trace modes are opt-in and still scrub secrets.

**3.2 Cap / token handling**

Design:

- No global ambient caps; tokens are attached **per client instance**, not globals.

- Client options:

  ```ts
  interface RonOptions {
    baseUrl: string; // required
    authToken?: string; // e.g. "Bearer <jwt>"
    passportToken?: string; // capability token
    headerProvider?: () =>
      | Record<string, string>
      | Promise<Record<string, string>>;
    requestIdFactory?: () => string; // override UUID v4 generator (for tests)
    overallTimeoutMs?: number; // default from env
  }
  ```

- Rotation: encouraged via `headerProvider` or future `CapProvider`:
  - Each request can call a user-provided function to get fresh headers (rotated tokens).

Handling & storage:

- Browser:
  - SDK **does not** prescribe where tokens come from (cookie, memory, storage).
  - Docs strongly discourage long-term secrets in `localStorage`; suggest:
    - HTTP-only cookies for long-lived refresh,
    - in-memory access tokens with short TTL.

- Node:
  - Caps/tokens should live in process memory, not logs or config files.

**3.3 Transport constraints**

- HTTPS/TLS only for production; docs and examples always use `https://` (except local dev).
- Timeouts:
  - global `overallTimeoutMs` (from env `RON_SDK_OVERALL_TIMEOUT_MS` if present) with an `AbortController` around `fetch`.

- Bounded retries:
  - MVP: no automatic retries except maybe for explicitly marked **idempotent** requests and only with strict caps (e.g., max 2, jitter/backoff).
  - Future: full backoff strategy, aware of idempotency.

**3.4 Sandboxing for facets/UI**

- The SDK must be **sandbox-friendly**:
  - No hidden global singletons.
  - Client instances can be created in Web Workers, Service Workers, Node worker threads.

- Tier 1 requirement:
  - Provide docs and at least one helper/wrapper for worker-based usage (e.g., an example `workerClient.ts` demonstrating using `Ron` from a Web Worker).

- All configuration is passed via constructor; no process-wide mutable state.

---

### 4) Reliability & Resilience (RES)

**4.1 Connection behavior**

The TS SDK is **stateless between calls** (no connection pool at HTTP level). Behavior:

- DNS failures, connection errors, or TLS issues:
  - `fetch` rejects → mapped to a synthetic local problem:

    ````ts
    {
      ok: false,
      status: 0,
      problem: { code: "local_network_failure", retryable: true }
    }
    ``` :contentReference[oaicite:30]{index=30}

    ````

- Timeouts triggered by `AbortController` likewise map to a local problem code such as `"local_timeout"`.

**4.2 Streaming resilience**

MVP:

- Streaming (SSE/WebSocket) is **not implemented** yet; design this section as future-facing.
- When added:
  - Each subscription will have:
    - explicit max retry count,
    - randomized backoff,
    - clear semantics (at-least-once vs at-most-once) documented.

  - Cancellation is explicit (subscription handle with `.close()`).

**4.3 Backoff & rate limiting**

MVP:

- No hidden auto-retry loops; any retry helper is opt-in, with:
  - max attempts,
  - jittered backoff,
  - idempotency flag required.

Idempotency:

- Provide helpers for idempotent writes:

  ```ts
  const id = Ron.generateIdempotencyKey(); // UUID v4
  await ron.post('/payments/charge', body, { idempotency_key: id });
  ```

- Tier 1: This helper **must** be present and documented.

---

### 5) Performance (PERF)

**5.1 Expected usage profile**

- Browser:
  - per-user rate: low to medium RPS (UI-driven).
  - payload sizes: typically small (KBs to low MBs).

- Node backend:
  - moderate RPS per process (10s–100s RPS) for typical app servers.
  - streaming may increase concurrency once implemented.

**5.2 Implementation choices**

- HTTP client:
  - Browser: **native `fetch`**.
  - Node: `global.fetch` via a lightweight polyfill (e.g., node-fetch/undici) hidden behind a tiny abstraction.

- Serialization:
  - JSON baseline:
    - objects → `JSON.stringify`,
    - `content-type: application/json` auto-set when the body is an object and header not overridden.

  - Non-JSON:
    - if `body` is `FormData`, `Blob`, `ArrayBuffer`, `Uint8Array`, pass-through and avoid overriding content-type.

  - Proto/CBOR are future.

**5.3 Benchmarks**

Tier 1 requirement: we’ll maintain at least:

- Single-call latency baseline vs raw `fetch`.
- Sustained RPS for a simple call (e.g., `/app/ping`).
- Overhead target: SDK overhead < 5% vs raw HTTP for simple calls.

Bench harness:

- Simple Node script that:
  - warms up,
  - fires N calls through `ron-app-sdk-ts`,
  - then N calls via `fetch` directly to `/app/ping`,
  - compares latency/RPS.

---

### 6) Economics (ECON)

**6.1 Dev productivity**

The SDK should reduce app code to:

1. **Hello world call** — copy-paste snippet above.

2. **“Business” call with JSON, query, and headers:**

   ```ts
   const res = await ron.post(
     '/kv/put',
     { key: 'foo', value: 'bar' },
     { region: 'us' },
   );
   if (!res.ok) {
     console.error(res.problem);
   }
   ```

3. **(Future) subscribe to a stream** — once SSE/WebSockets land.

Goal: You should _never_ hand-build URLs or manually parse errors for routine app-plane work.

**6.2 Operational cost**

- Encourage reuse:
  - One `Ron` client per browser app or per Node service instance.

- No resource leaks:
  - Node: no extra timers/goroutines without cancellation (SSE/WebSocket later).
  - Browser: no hidden intervals or background polling; everything is explicitly created and closed.

---

### 7) Developer Experience (DX)

**7.1 “No manual” quickstart**

Required README snippets:

1. Hello world call (shown above).
2. Basic POST with JSON.
3. (Future) Subscribe to stream.

Each must fit 10–20 lines and be runnable with minimal config.

**7.2 Framework adapters / helpers**

Planned adapters:

- [ ] React hook: `useRonClient(options)` (or simple pattern docs).
- [ ] Next.js / Remix usage notes (focus on server-side `Ron` in loaders/actions).
- [ ] Node serverless recipes (e.g., AWS Lambda, Vercel Edge).
- [ ] Logging/tracing integration:
  - Optionally plug in a logger or tracing callback.

These can live either in this package or a sibling `ron-app-sdk-ts-react` / `ron-app-sdk-ts-next` package once stable.

**7.3 Language-idiomatic docs**

- Use idiomatic TS:
  - generics for typed responses (`AppResponse<T>`),
  - union types for methods, optional chaining.

- No Rust-isms:
  - avoid `Result`-style naming; we use the native Promise and value-based `ok` flag.

---

### 8) Extensibility & Plugins

**8.1 Extension points**

MVP:

- [x] Request/response middleware callbacks:

  ```ts
  const ron = new Ron({
    baseUrl,
    onRequest: (req) => {
      /* logging, tracing, etc. */
    },
    onResponse: (res) => {
      /* metrics, error tracking */
    },
  });
  ```

Future:

- [ ] Dedicated middleware stack (like Axios interceptors).
- [ ] Custom transport adapters (e.g., alt runtime or offline mode).

**8.2 UI widgets / components**

Because TS SDK will be used in GUIs:

- UI helpers should live in **separate packages** (`ron-app-sdk-ts-ui`), not in the core.
- Future widgets:
  - `NodeHealthBadge`
  - `MetricsPanel`
  - `EventLogViewer`

- Design goal: drop a widget into any React app, point it at a gateway base URL, and see metrics/health with minimal wiring.

**8.3 Plugin model (future)**

- Third-party extensions register as either:
  - request/response middleware,
  - or standalone helper libraries built on top of the public `Ron` API.

- Core SDK remains lean; heavy plugins live elsewhere.
- Avoid dynamic plugin loading patterns that create security footguns in browser environments.

---

### 9) Governance & Versioning (GOV)

**9.1 Version scheme**

- SDK uses semver:
  - `0.x` while RON-CORE protocols are pre-1.0.
  - `1.x` once app-plane is stable.

- Policy:
  - Breaking changes → major bump.
  - Deprecation window for Tier 1: at least 2 minor versions before removal.

**9.2 Compatibility matrix**

Example initial row:

| SDK version | Protocol version | Min node version                  | Notes      |
| ----------- | ---------------- | --------------------------------- | ---------- |
| 0.1.x       | v0.1             | macronode/micronode Beta snapshot | MVP TS SDK |

**9.3 Support expectations**

`ron-app-sdk-ts` is **Tier 1**:

- CI: lint, unit, integration, plus basic property tests for idempotency and header stability.
- Strong backward compatibility guarantees when updating across minor versions.

---

### 10) Testing & CI

**10.1 Unit tests**

Cover:

- Header merge logic.
- Query-string assembly.
- Body JSON serialization rules.
- Error mapping (HTTP 4xx/5xx, 502 upstream, network errors).
- Problem JSON parsing and classification.

**10.2 Integration tests**

Test topology:

- Spin up:
  - dummy omnigate (or test handler),
  - real `svc-gateway`.

- Then call SDK:

Test cases:

1. `GET /app/ping` → `ok=true`.
2. `POST` with JSON body → echo or mutate, asserts correct encoding.
3. `PUT` with query parameters → asserts query string correctness.
4. Header propagation: `Authorization`, `X-RON-Passport`, `x-request-id`, `x-correlation-id`.
5. Problem passthrough: forced `400`/`500` from app -> `problem` field filled.
6. 502 `upstream_unavailable` from gateway → `problem.code="upstream_unavailable"`, `retryable=true`.
7. Local network failure → status `0`, `problem.code="local_network_failure"`.

**10.3 Browser tests**

- CORS behavior:
  - preflight with auth headers,
  - responses with correct CORS headers from gateway.

- Test via a simple Vite app that imports `ron-app-sdk-ts` and calls a local node.

---

### 11) Interop & Schema (ron-proto, HTTP, Facets)

We apply the strengthened schema IDB guidance here.

**RO:WHAT —**
Ensure `ron-app-sdk-ts` uses the **same structural truth** (DTOs, errors, pagination, manifests) as Rust and Go SDKs.

**RO:INVARIANTS —**

- DTOs exposed by TS SDK are derived from canonical schemas (`ron-proto` + HTTP/OpenAPI).
- No TS-specific skew in request/response shapes.
- Facet manifests (TOML) follow the shared schema (`SDK_SCHEMA_IDB.MD`) when we later add manifest generation.
- Error and pagination envelopes have identical fields and semantics cross-language.

#### 11.1 Canonical Sources

- `ron-proto` — OAP/1 frames, storage/index DTOs, auth/passport payloads.
- HTTP/OpenAPI spec for gateway/omnigate — path/method/body and error envelopes.
- `SDK_SCHEMA_IDB.MD` — cross-language type mapping, JSON encoding rules, facet manifest schema.

#### 11.2 TS-specific type mapping

TS uses:

- `boolean` for `bool`.
- `number` for `u32` etc where safe; `string` or configurable `bigint` for `u64`/`i64`.
- `string` (ISO 8601) for timestamps.
- `Uint8Array`/`ArrayBuffer` or base64url string for binary payloads (document per API).

JSON encoding rules match SDK_SCHEMA (omit-null semantics, base64 for binary, ignore unknown fields or store in extension map).

#### 11.3 Drift avoidance

- If we add codegen for TS DTOs:
  - CI step regenerates types from the canonical schema and fails if git diff is non-empty.

- Interop tests:
  - For each “interop endpoint” (ping, KV put/get, etc.), we assert that runtime JSON matches compiled types and error envelopes.

---

### 12) Open Questions / Future Work

- Schema/codegen:
  - Decide on TS codegen toolchain (OpenAPI → TS types, or proto → TS).

- Streaming:
  - SSE vs WebSocket vs EventSource; design reconnection semantics and backpressure.

- Facet manifest generation:
  - TS helper to build facet TOML, aligning with shared schema and SRI/limits fields.

- Advanced security:
  - PQ-ready token fields, audit IDs, and signed manifests integration via `ron-kms`.

- Admin plane SDK vs app-plane SDK:
  - Decide if TS admin helpers belong here or in a separate `ron-admin-sdk-ts`.

---
