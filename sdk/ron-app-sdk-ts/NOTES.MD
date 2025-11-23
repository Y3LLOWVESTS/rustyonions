### BEGIN NOTE - NOVEMBER 22 2025 - 19:45 CST

Below is a **massive, crystal-clear, zero-drift, fully exhaustive CARRY-OVER PACK** for the **next major milestone: building `ron-app-sdk-ts`**.

This document assumes:

* **svc-gateway is complete**
* **omnigate App Plane surface is stable**
* **The Rust SDK (ron-app-sdk-rs) will act as the reference contract**
* **We are now moving outward to the TypeScript SDK used by browsers & Node apps**

You can drop these notes directly into `ALLNOTES.md` under the **SDK section**.

---

# 🚀 **CARRY-OVER NOTES — Begin ron-app-sdk-ts (TypeScript SDK)**

**Date:** 2025-11-22
**Status:** svc-gateway App Plane Hop Completed 100% • All Tests Green • Ready for SDK Development
**Verdict:** We are ready to begin the RON App SDK for TypeScript. This carry-over pack is designed to eliminate drift, give us exact requirements, and define the shape, API surface, and behavior of the upcoming SDK.

---

# 0) **TL;DR — Where We Stand**

The svc-gateway now exposes a clean, correct, stable `/app/*` surface:

```
client (browser/Node)
    → https://your-micronode-or-macronode/app/*
    → svc-gateway
    → omnigate /v1/app/*
    → app handlers via ron-app-sdk-rs
```

**Everything the TS SDK needs is now fully ready and guaranteed stable:**

* Correct HTTP status propagation
* Correct error envelope propagation
* Correct header propagation
* Correct request-ID propagation
* Query/body/method integrity
* Fully compliant RON Problem JSON in all failure cases
* No hidden redirects, mutations, or gateway-side rewrites

This gives us a **solid contract** on which to build the TypeScript SDK.

---

# 1) **The Contract the TS SDK MUST Implement**

This is the **single source of truth** for the SDK interface.

## 1.1 Request Model (AppRequest)

The Rust reference SDK uses:

```
struct AppRequest {
    method: String,
    path: String,
    headers: Option<HashMap<String, String>>,
    body: Option<Bytes>,
    query: Option<String>
}
```

The TS equivalent should map as:

```ts
export interface AppRequest {
    method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
    path: string;              // "/hello", "/kv/put", "/auth/register"
    query?: Record<string, string>;
    headers?: Record<string, string>;
    body?: unknown;            // automatically JSON-encoded if object
}
```

**Invariant:**
TS SDK never prefixes `/app/*`.
It always sends the request to:

```
<gateway_base_url>/app/<path>?<query>
```

---

## 1.2 Response Model (AppResponse)

Rust returns:

* Raw HTTP code
* Raw headers
* Raw bytes or JSON
* Or Problem JSON

TS equivalent:

```ts
export interface AppResponse<T = unknown> {
    status: number;
    ok: boolean;            // status in 200–299
    headers: Record<string, string>;
    data?: T;               // parsed JSON if content-type = application/json
    raw?: ArrayBuffer;      // raw body if non-JSON
    problem?: RonProblem;   // present ONLY if is 4xx/5xx returned by server
}
```

---

## 1.3 Problem Model (TS Version)

Matches Rust:

```ts
export interface RonProblem {
    code: string;               // "upstream_unavailable", "problem_400", etc
    message?: string;
    retryable?: boolean;
    retry_after_ms?: number;
    reason?: string;
}
```

**Invariant: The SDK never throws exceptions for server-returned problems.**
It exposes them inside `AppResponse`.

Only SDK misuse throws (e.g., invalid URL).

---

# 2) **Transport Rules the SDK Must Honor**

These come directly from svc-gateway + omnigate contract.

## 2.1 Outbound Headers to Always Send

TS SDK must send (if not already present):

```
x-request-id     (UUID v4)
x-correlation-id (equal to x-request-id unless caller overrides)
content-type     (json if body is object)
accept           (json)
```

Gateway will preserve these and forward them unchanged.

---

## 2.2 Outbound Headers the SDK May Allow User to Set

* Authorization
* X-RON-Token
* X-RON-Passport
* Custom application headers

**The SDK must merge user headers with automatic headers.**

---

## 2.3 Hop-by-Hop Headers the SDK Must Never Set

* Host
* Connection
* Content-Length
* Transfer-Encoding
* Trailer
* TE
* Upgrade

These are controlled by Fetch/Node and forbidden by svc-gateway.

---

# 3) **Base URL Discovery Strategy**

TS SDK initialization should look like:

```ts
import { Ron } from "ron-app-sdk-ts";

const ron = new Ron({
    baseUrl: "https://my-node.example.com",      // required
    passportToken?: string,                       // optional
    authToken?: string,                           // optional
});
```

Derived endpoints:

```
ron.request()   → baseUrl + "/app/<path>"
```

**Invariant:**
SDK never talks to `/v1/app/*`.
That is internal to svc-gateway → omnigate.

---

# 4) **High-Level API Surface for TypeScript**

## 4.1 Simple Request API

```ts
await ron.get("/hello");
await ron.post("/kv/put", { key: "foo", value: "bar" });
await ron.delete("/user/123");
```

## 4.2 Fully Manual Raw Request

```ts
ron.request<T>({
    method: "POST",
    path: "/kv/put",
    body: { key: "foo", value: "bar" },
    headers: { "X-Whatever": "1" },
    query: { region: "us" }
});
```

## 4.3 Helpers for Typed App Facets (generated later)

```ts
ron.facet("auth").login({ user, pass })
ron.facet("kv").get("foo")
```

Facets come later after initial SDK completion.

---

# 5) **Behavior Rules for Error Handling (Mandatory)**

## 5.1 If svc-gateway returns 4xx or 5xx with JSON:

SDK must NOT throw.
SDK must return:

```ts
{
  ok: false,
  status: 400,
  problem: { code: "problem_400" },
}
```

## 5.2 If svc-gateway returns 502 "upstream_unavailable":

SDK should expose:

```ts
{
  ok: false,
  status: 502,
  problem: {
    code: "upstream_unavailable",
    retryable: true,
    reason: "connect" | "read"
  }
}
```

## 5.3 If fetch() throws (e.g., network offline):

SDK wraps into a local problem:

```ts
{
  ok: false,
  status: 0,
  problem: { code: "local_network_failure", retryable: true }
}
```

**Invariant:**
No Promise rejection for server-originated errors.
The SDK is fully error-value-based.

---

# 6) **TypeScript SDK Implementation Plan**

A clear 10-step implementation roadmap.

### Step 1 — Project Bootstrap

* Create package scaffold `ron-app-sdk-ts/`
* Set up TypeScript, Vite (or NPM), ES modules, Node compat
* Set up Jest or Vitest for tests
* Configure ESLint + Prettier

### Step 2 — Build the RonClient Class

```ts
export class Ron {
    constructor(options: RonOptions) {...}
    request<T>(req: AppRequest): Promise<AppResponse<T>>;
    get<T>(path: string, query?: object): ...
    post<T>(path: string, body?: object, query?: object): ...
}
```

### Step 3 — Auto-injected headers

* x-request-id
* x-correlation-id
* content-type if absent

### Step 4 — Build the Fetch Wrapper

Implements:

* Assemble URL
* Attach query parameters
* Attach merged headers
* Handle JSON encoding
* Attach AbortController for timeouts

### Step 5 — Response parser

* Read status
* Check content-type
* Attempt JSON parse
* Detect if body is Problem JSON
* Fill `AppResponse`

### Step 6 — Error semantic mapping

* Fetch failure → local Problem
* HTTP error → Problem passthrough
* JSON parse fail → raw buffer fallback

### Step 7 — End-to-end Integration Tests

Using a real svc-gateway + omnigate just like we did for Rust tests.

### Step 8 — Browser Compatibility

* Ensure `fetch()` polyfill for Node.js
* Ensure HTTPS/TLS works with gateway

### Step 9 — Developer Ergonomics

* Hooks for token injection
* Hooks for custom headers
* Automatic renew-extension (future)

### Step 10 — Publish Ready

* Build dist bundles
* Type definitions
* README
* Examples

---

# 7) **SDK MUST Mirror Observability Guarantees**

The SDK will be used by developers. It must forward:

### ✔ x-request-id

Automatically generated if absent.

### ✔ x-correlation-id

Used for tracing user sessions.

### ✔ Caller-supplied headers

Auth / passport / custom.

### ✔ No mutation of response headers

We return exactly what svc-gateway delivered.

### ✔ Timing information (future)

Expose timing hooks for dashboards.

---

# 8) **Minimum Test Matrix for SDK**

### ✔ Unit Tests

* Header merge
* Query string assembly
* Body JSON serialization
* Error mapping logic
* Problem JSON parsing

### ✔ Integration Tests (critical)

Spin up:

* dummy omnigate
* real svc-gateway
* then call the SDK:

Tests:

1. GET /app/ping
2. POST with JSON
3. PUT with query
4. Header propagation
5. Problem passthrough
6. 502 upstream unavailable
7. local network failure

### ✔ Browser Tests

Real-world:

* Vite dev server → gateway → omnigate
* CORS behavior

---

# 9) **Things We Do NOT Need Yet**

(do not waste time here)

* App authentication (requires ron-auth + svc-passport)
* Facet generation (comes after base SDK is stable)
* Multi-app tenancy
* Token refresh logic
* Retry policies
* Websocket support

These belong to future SDK phases.

---

# 10) **Ready for Next Session — Files to Attach**

When we begin coding the SDK:

### You will attach:

```
ron-app-sdk-ts/package.json
ron-app-sdk-ts/src/index.ts
ron-app-sdk-ts/src/client.ts
ron-app-sdk-ts/src/types.ts
ron-app-sdk-ts/src/errors.ts
ron-app-sdk-ts/test/*.ts
```

If these do not exist yet, you will paste blank initial scaffolds and I will generate full production versions.

---

# 11) **Final Summary**

We are now fully ready to build `ron-app-sdk-ts` because:

* Gateway is complete
* App-plane contract is stable
* Problem semantics are stable
* Header propagation is correct
* Query + body + method forwarding is tested
* End-to-end topology replicates real node behavior

The TypeScript SDK now has a **zero-ambiguity target contract** and a complete plan.

---

### END NOTE - NOVEMBER 22 2025 - 19:45 CST
