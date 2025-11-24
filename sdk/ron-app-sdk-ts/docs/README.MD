
````md
# ron-app-sdk-ts

> **Role:** TypeScript / JavaScript App SDK  
> **Owner:** Stevan White (RustyOnions / RON-CORE)  
> **Status:** alpha (app-dev ready; security hardening & integration tests pending)  
> **Runtime targets:** Modern browsers, Node 18+ (with global `fetch`), Deno/Workers (future)  
> **Last reviewed:** 2025-11-23

Badges (future):  
[![Build](https://img.shields.io/badge/build-CI-green)]() [![npm](https://img.shields.io/npm/v/ron-app-sdk-ts)]() [![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

---

## 1) Overview

**What it is (one paragraph):**  
`ron-app-sdk-ts` is the official TypeScript / JavaScript SDK for talking to the RON-CORE **app plane** over HTTP. It wraps `fetch` with a small, opinionated `Ron` client that builds `/app/*` URLs, injects observability headers (`x-request-id`, `x-correlation-id`), encodes/decodes JSON, and maps RON Problem JSON into safe, structured error values instead of thrown exceptions. It also includes **facet helpers** so app developers don’t have to hard-code route prefixes everywhere.

**How it fits (RustyOnions topology):**

- **Pillar:** App Plane + Developer Experience (DX) + Security (SEC)
- **Upstream callers:**
  - Browser apps (React/Next/Vite/etc.)
  - Node backends, serverless functions, edge workers
- **Downstream deps:**
  - `svc-gateway` at `https://<node>/app/*` (primary)
  - Indirectly `omnigate` (`/v1/app/*`) and app handlers implemented via `ron-app-sdk-rs`
- **Data it touches:**
  - In-memory DTOs and HTTP bodies only (no local DB)
  - Canonical RON-CORE DTOs & error envelopes (schema-driven)
- **Security boundary:**
  - Treats browser as **untrusted**; no long-term secret storage
  - Treats Node as **trusted but must not leak caps** (tokens/macaroons)

### 1.1 High-Level Architecture (Mermaid)

```mermaid
flowchart LR
  subgraph App["App code (browser / Node)"]
    A[React/Next/Node service] -->|TS import| B[Ron client<br/>ron-app-sdk-ts]
  end

  B -->|HTTPS /app/*| C[svc-gateway]
  C -->|HTTP /v1/app/*| D[omnigate]
  D -->|App plane| E[App handlers<br/>(ron-app-sdk-rs)]

  style B fill:#0b7285,stroke:#083344,color:#fff
  style C fill:#1f2933,stroke:#111827,color:#fff
  style D fill:#111827,stroke:#020617,color:#fff
````

---

## 2) Quickstart

> For a focused version of this QuickStart, see [`docs/QUICKSTART.MD`](./docs/QUICKSTART.MD).

### 2.1 Install

Using **pnpm** (recommended):

```bash
pnpm add ron-app-sdk-ts
```

Or with **npm**:

```bash
npm install ron-app-sdk-ts
```

The SDK targets:

* Modern browsers (with `fetch`)
* Node 18+ (with global `fetch` or a polyfill like `undici`)

---

### 2.2 Create a client

Point the SDK at your node’s **gateway** (not omnigate directly):

```ts
import { Ron } from "ron-app-sdk-ts";

const ron = new Ron({
  baseUrl: "https://my-node.example.com",
  // optional:
  // overallTimeoutMs: 10_000,
  // authToken: "Bearer CAP_...",
  // passportToken: "PASS_...",
});
```

By default:

* `baseUrl` **must** be HTTPS (unless you opt into `allowInsecureHttp` for local dev).
* `overallTimeoutMs` defaults to 10s if not set.

---

### 2.3 Call an app endpoint

Each call returns a **typed** `AppResponse<T>` with either `data` (ok) or `problem` (error) — it does **not** throw for server-side errors.

```ts
interface HelloResponse {
  id: string;          // u64 encoded as string
  message: string;
}

const res = await ron.get<HelloResponse>("/hello", { lang: "en" });

if (res.ok) {
  console.log("hello:", res.data?.message);
} else {
  console.error("problem:", res.problem?.code, res.problem?.message);
}
```

This hits:

* `GET https://my-node.example.com/app/hello?lang=en`

The SDK:

* Builds `/app/...` for you
* Attaches headers (auth/passport, observability)
* Maps non-2xx JSON bodies into a structured `RonProblem`.

---

### 2.4 Use facets instead of hard-coding paths

Facets are just **namespaces** under `/app`. Instead of sprinkling `/auth/...` or `/users/...` everywhere, use the facet helper:

```ts
const auth = ron.facet("auth");
const users = ron.facet("users");

const loginRes = await auth.post("/login", {
  username: "alice",
  password: "secret",
});

const meRes = await users.get("/me");
```

These calls become:

* `POST https://my-node.example.com/app/auth/login`
* `GET  https://my-node.example.com/app/users/me`

You can still use `ron.get("/foo")` directly for simple cases; facets are pure DX sugar.

---

### 2.5 Handling network errors & timeouts

The SDK never throws for normal HTTP errors. It only throws if something is catastrophically misconfigured. Network problems and timeouts are surfaced as **transport problems**:

```ts
const res = await ron.get("/sometimes-slow");

if (!res.ok && res.status === 0) {
  const problem = res.problem;
  if (problem?.code === "local_timeout") {
    console.warn("Request timed out, consider retrying");
  } else if (problem?.code === "local_network_failure") {
    console.warn("Network error, check connectivity");
  }
}
```

Invariants:

* No auth/passport tokens are ever placed in the URL.
* Local error messages do **not** contain secrets (only generic strings like `"Network error"` / `"Request timed out"`).
* Server-originated problems are available as `res.problem` with a consistent shape.

---

## 3) Responsibilities & Boundaries

**MUST do (core responsibilities):**

* ✅ Provide a small, ergonomic `Ron` client that wraps `fetch` for `/app/*` calls (browser + Node).
* ✅ Enforce key security invariants from `SDK_SECURITY.md`:

  * No secrets in URLs.
  * No caps in error messages produced by the SDK.
  * Bounded timeouts.
* ✅ Implement canonical `AppRequest`, `AppResponse<T>`, and `RonProblem` shapes defined in `SDK_IDB.md` / `SDK_SCHEMA_IDB.md`.
* ✅ Preserve and/or inject observability headers (`x-request-id`, `x-correlation-id`) on every request.
* ✅ Provide facet helpers (`ron.facet("auth")`) to avoid hard-coded prefixes.

**MUST NOT do (anti-scope / boundaries):**

* ❌ Must **not** talk directly to `/v1/app/*` (that’s gateway → omnigate internal).
* ❌ Must **not** store capabilities/tokens in browser `localStorage`/`sessionStorage` internally.
* ❌ Must **not** invent custom JSON schemas for core RON DTOs; must follow `ron-proto`/HTTP schemas.
* ❌ Must **not** auto-retry non-idempotent operations (POST/DELETE) behind the user’s back.

**Acceptance Gates (proof of done):**

* ✅ Unit tests covering URL building, header merge, JSON encode/decode, error mapping, timeouts, and facet behavior.
* ⏳ Integration tests against a real `svc-gateway` (`/app/ping`, JSON POST, header propagation, problem passthrough, 502, and network failure).
* ⏳ Security tests beyond unit scope (fuzzing, header scanning, etc.) — to be added in the “security hardening” phase.
* ⏳ Benchmarks showing SDK overhead vs raw `fetch` is within the target (< ~5% latency overhead on simple calls).

---

## 4) TypeScript API (library)

### 4.1 Core types

```ts
export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export interface AppRequest {
  method: HttpMethod;
  path: string; // "/hello", "/kv/put", "/auth/register"
  query?: Record<string, string>;
  headers?: Record<string, string>;
  body?: unknown; // JSON-encoded if object
}

export interface RonProblem {
  code: string; // "upstream_unavailable", "problem_400", "local_timeout", etc.
  message?: string;
  kind?: string;
  correlationId?: string;
  retryable?: boolean;
  retryAfterMs?: number;
  reason?: string;
  details?: Record<string, unknown>;
}

export interface AppResponse<T = unknown> {
  status: number;
  ok: boolean; // status 200–299
  headers: Record<string, string>;
  data?: T; // parsed JSON, if JSON body
  raw?: ArrayBuffer; // raw bytes, if non-JSON (optional, future)
  problem?: RonProblem; // present when !ok
}
```

These shapes are derived from the canonical `SDK_IDB.md` + `SDK_SCHEMA_IDB.md` contract and the gateway Problem JSON spec.

### 4.2 Ron client

```ts
import { Ron } from "ron-app-sdk-ts";

const ron = new Ron({
  baseUrl: "https://my-node.example.com",
  authToken: "Bearer <jwt-or-macaroon>",
  passportToken: "<capability-token>",
});

async function main() {
  const res = await ron.get<{ message: string }>("/hello");

  if (res.ok && res.data) {
    console.log("Hello from RON:", res.data.message);
  } else {
    console.error("RON error:", res.status, res.problem);
  }
}
```

#### Public surface (MVP)

```ts
class Ron {
  constructor(options: RonOptions);

  request<T = unknown>(req: AppRequest): Promise<AppResponse<T>>;

  get<T = unknown>(
    path: string,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  post<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  put<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  patch<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  delete<T = unknown>(
    path: string,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  facet(facetId: string): FacetClient;
}

class FacetClient {
  get<T = unknown>(
    path: string,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  post<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  put<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  patch<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;

  delete<T = unknown>(
    path: string,
    query?: Record<string, string>
  ): Promise<AppResponse<T>>;
}
```

**Invariants:**

* `path` is **always app-relative** (e.g. `"/hello"`). The SDK constructs `baseUrl + "/app" + path`.
* Facet paths become `"/{facetId}{path}"` (e.g. facet `"auth"` + `"/login"` → `"/auth/login"`).
* The SDK **never** calls `/v1/app/*`.
* Server-originated errors (4xx/5xx/502) **never throw**; they appear as `AppResponse` with `ok === false` and `problem` populated. Only local SDK mis-use (e.g. invalid `baseUrl`) rejects the Promise.

---

## 5) Configuration

> TS SDK uses a mix of **constructor options** and **env vars** (for Node). Browsers ignore env vars.

### 5.1 RonOptions

```ts
export interface RonOptions {
  baseUrl: string; // required; must be https:// in production

  // Auth / caps (optional)
  authToken?: string; // e.g. "Bearer <jwt>"
  passportToken?: string; // capability macaroon

  // Timeouts (ms)
  overallTimeoutMs?: number;
  connectTimeoutMs?: number; // Node/Workers only
  readTimeoutMs?: number;

  // Observability
  requestIdFactory?: () => string;
  onRequest?: (req: AppRequest & { url: string }) => void;
  onResponse?: <T>(res: AppResponse<T>) => void;

  // Dynamic header source (for rotation)
  headerProvider?: () =>
    | Record<string, string>
    | Promise<Record<string, string>>;

  // Dev-only
  allowInsecureHttp?: boolean; // default false
  debug?: boolean; // extra diagnostics w/ scrubbing
}
```

### 5.2 Environment variables (Node only)

| Variable                     | Type   | Default         | Description                                  |
| ---------------------------- | ------ | --------------- | -------------------------------------------- |
| `RON_SDK_GATEWAY_ADDR`       | string | *none*          | Default `baseUrl` if not provided in options |
| `RON_SDK_OVERALL_TIMEOUT_MS` | int    | `10000`         | Overall request timeout                      |
| `RON_SDK_CONNECT_TIMEOUT_MS` | int    | runtime default | Node: socket connect timeout                 |
| `RON_SDK_READ_TIMEOUT_MS`    | int    | runtime default | Node: read inactivity timeout                |

These env vars are defined in the shared schema IDB and must be honored as defaults by all SDKs.

---

## 6) Facet Manifest Helpers (future-facing)

`src/facets/manifest.ts` defines small helpers for building facet manifests consistent with the shared schema (static / proxy facets).

Rough shapes:

```ts
type FacetKind = "static" | "echo" | "proxy";

interface FacetRouteStatic {
  kind?: "static";
  method: "GET" | "HEAD";
  path: string;
  file: string;
}

interface FacetRouteProxy {
  kind?: "proxy" | "echo";
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  path: string;
  upstreamPath?: string;
}

type FacetRoute = FacetRouteStatic | FacetRouteProxy;

interface FacetUpstream {
  scheme: "http" | "https";
  host: string;
  port: number;
  basePath?: string;
}

interface StaticFacetDefinition {
  kind: "static";
  name: string;
  routes: FacetRouteStatic[];
  // meta/security omitted for brevity
}

interface ProxyFacetDefinition {
  kind: "proxy" | "echo";
  name: string;
  upstream: FacetUpstream;
  routes: FacetRouteProxy[];
}

type FacetDefinition = StaticFacetDefinition | ProxyFacetDefinition;

interface FacetManifest {
  facets: FacetDefinition[];
}
```

Helpers:

* `defineStaticFacet(...)`
* `defineProxyFacet(...)`
* `buildFacetManifest([...])`

These are primarily for future tooling / generators and are kept in sync with `SDK_SCHEMA_IDB.md`.

---

## 7) HTTP / Wire Behavior

### 7.1 Base URL & paths

* **Base URL:** `https://<gateway-host>` (the node’s external address)
* **SDK target:** `baseUrl + "/app" + path`
* **Example:**

  * `baseUrl = "https://my-node.example.com"`
  * `path = "/hello"`
  * Request goes to `https://my-node.example.com/app/hello`.

### 7.2 Required outbound headers

The SDK ensures the following headers are present (merging with user headers):

* `x-request-id`: UUID-like string (if caller didn’t set)
* `x-correlation-id`: equals `x-request-id` by default; can be overridden
* `accept`: `application/json` (by default)
* `content-type`: `application/json` when `body` is an object and caller didn’t override

**Forbidden / hop-by-hop headers** (never set by SDK):

* `Host`, `Connection`, `Content-Length`, `Transfer-Encoding`, `Trailer`, `TE`, `Upgrade`

---

## 8) Security & Privacy

Security is driven by the shared `SDK_SECURITY.md` blueprint.

**Threat model (short):**

* **Browser:** hostile environment (XSS, extension snooping, stolen devices). Tokens must be short-lived; storage is dangerous.
* **Node:** trusted environment but may still leak secrets via logs/misconfig.

**Key handling:**

* No caps/tokens are ever placed in URLs, query strings, or facet manifests.
* No caps/tokens are ever included in SDK-generated error messages.
* The SDK does **not** internally persist tokens in browser storage; caller is responsible for secure acquisition/refresh.
* Debug logs (when enabled) must not print raw tokens.

**PII:**

* The SDK is **transport only**; it does not inspect or log bodies. It forwards whatever JSON you send. Apps must handle PII carefully.

---

## 9) Observability

`ron-app-sdk-ts` doesn’t expose `/metrics` itself, but it is **observability-aware**:

* Always forwards / injects:

  * `x-request-id`
  * `x-correlation-id`

* Provides hooks:

```ts
const ron = new Ron({
  baseUrl,
  onRequest: (req) => {
    console.log("RON request", { method: req.method, path: req.path });
  },
  onResponse: (res) => {
    console.log("RON response", { status: res.status, ok: res.ok });
  },
});
```

Downstream services can rely on these IDs to correlate logs and traces.

---

## 10) Error Taxonomy

`ron-app-sdk-ts` surfaces **value-based** errors, not thrown exceptions, for all server-originated problems.

| Scenario                 | AppResponse shape                                                                   | Retry?                    |
| ------------------------ | ----------------------------------------------------------------------------------- | ------------------------- |
| 2xx success              | `ok=true`, `data` set (if JSON), or `raw` for non-JSON                              | n/a                       |
| 4xx app error            | `ok=false`, `status=4xx`, `problem.code="problem_4xx"` (or similar)                 | app-dependent             |
| 5xx app error            | `ok=false`, `status=5xx`, `problem` filled                                          | maybe                     |
| 502 upstream unavailable | `ok=false`, `status=502`, `problem.code="upstream_unavailable"`, `retryable=true`   | maybe with backoff        |
| Network / DNS / timeout  | `ok=false`, `status=0`, `problem.code="local_network_failure"` or `"local_timeout"` | yes, with jitter/backoff  |
| Malformed non-JSON error | `ok=false`, `problem.code="transport_error"`                                        | maybe, depending on cause |

All errors expose a **canonical `RonProblem`** where possible; otherwise, a safe generic problem with no raw stack traces or tokens.

---

## 11) Build, Run, Test

This is a pure TS/JS package; use your preferred package manager.

**Install deps (for the SDK itself):**

```bash
pnpm install
```

**Standard local checks:**

```bash
pnpm format
pnpm lint
pnpm build
pnpm test
```

* `pnpm format` — Prettier
* `pnpm lint` — ESLint (strict, TS-aware)
* `pnpm build` — TypeScript build to `dist/`
* `pnpm test` — Vitest unit tests

Security & deps (TS analog of cargo-deny):

```bash
pnpm audit
pnpm lint
```

---

## 12) Roadmap & Status

High-level roadmap (TS SDK slice):

* ✅ **M1 — Core client MVP**

  * Implement `Ron` client, `AppRequest`, `AppResponse<T>`, `RonProblem`.
  * Implement header injection, timeouts, and baseUrl + `/app` routing.
  * Unit tests for URL building, headers, JSON, error mapping, timeouts.
  * Facet helpers (`ron.facet("auth")`) + tests.

* ⏳ **M2 — Integration & Security hardening**

  * Integration tests with real `svc-gateway` and test app.
  * Extra security tests (fuzz malformed responses, header scanning).
  * Basic benchmarks vs raw `fetch`.

* ⏳ **M3 — Schema & Codegen**

  * Introduce DTO codegen from canonical schemas (`ron-proto`/OpenAPI).
  * Drift-check CI (fail on regenerated type diffs).

* ⏳ **M4 — Framework helpers & DX**

  * React/Next example apps or small helper layer.
  * Richer docs and cookbook for common app patterns.

Streaming, SSE/WebSocket, and more advanced features are reserved for future milestones once core app-plane flows are stable.

---

## 13) License

Dual-licensed under **MIT** or **Apache-2.0**.
Intended for both open-source and commercial use.

---

## 14) Contributing

PRs welcome!

Before opening a PR:

* Run `pnpm format`, `pnpm lint`, `pnpm build`, and `pnpm test`.
* Keep README and diagrams in sync with any topology/protocol changes.
* Ensure new features obey the invariants in:

  * `SDK_IDB.md`
  * `SDK_SECURITY.md`
  * `SDK_SCHEMA_IDB.md`

For deeper design rationale and invariants, see:

* `SDK_IDB.MD`
* `SDK_SCHEMA_IDB.MD`
* `SDK_SECURITY.MD`
* carry-over notes for `ron-app-sdk-ts` in `ALLNOTES.MD` (RustyOnions root)

```

