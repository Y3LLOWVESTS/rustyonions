---

## 1) `sdk/ron-app-sdk-ts/src/facets/manifest.ts`

Paste this over the existing file:

```ts
/**
 * Facet manifest types and helpers for emitting facet configs.
 *
 * This is aligned with the SDK_SCHEMA_IDB manifest schema and is intentionally
 * small and declarative: app code describes *what* routes exist, not how
 * they’re wired internally.
 */

export type FacetKind = 'static' | 'echo' | 'proxy';

export interface FacetSecurity {
  /**
   * If true, facet is world-readable (no auth required).
   * Useful for healthz/status/docs.
   */
  public?: boolean;
  /**
   * If true, facet requires some form of authentication/capability.
   * Exact mechanism is enforced by the node, not the SDK.
   */
  requiresAuth?: boolean;
}

export interface FacetMeta {
  /** Human-readable description for operators / dashboards. */
  description?: string;
  /** Owning team or contact, e.g. "payments" or "search". */
  owner?: string;
  /** Optional semantic version of the facet contract. */
  version?: string;
}

/**
 * Static file route served directly by gateway.
 * Example: GET /docs -> ./public/docs/index.html
 */
export interface FacetRouteStatic {
  kind?: 'static';
  method: 'GET' | 'HEAD';
  path: string;
  file: string;
}

/**
 * Upstream target for proxy/echo facets.
 */
export interface FacetUpstream {
  scheme: 'http' | 'https';
  host: string;
  port: number;
  /** Optional basePath prepended before upstreamPath. */
  basePath?: string;
}

/**
 * Proxy route forwarded to an upstream service via svc-gateway/omnigate.
 * Example: POST /kv/put -> http://kv-svc.internal/put
 */
export interface FacetRouteProxy {
  kind?: 'proxy' | 'echo';
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  path: string;
  /**
   * Optional upstream path override. If omitted, `path` (minus /app prefix)
   * is reused on the upstream.
   */
  upstreamPath?: string;
}

export type FacetRoute = FacetRouteStatic | FacetRouteProxy;

interface FacetBase {
  /** Logical name for this facet (e.g. "docs", "kv", "search"). */
  name: string;
  kind: FacetKind;
  meta?: FacetMeta;
  security?: FacetSecurity;
}

/**
 * A facet that serves static assets only.
 */
export interface StaticFacetDefinition extends FacetBase {
  kind: 'static';
  routes: FacetRouteStatic[];
}

/**
 * A facet that proxies requests to an upstream service.
 * `kind === "proxy"` is the usual case; `"echo"` is reserved for
 * debugging/diagnostic facets.
 */
export interface ProxyFacetDefinition extends FacetBase {
  kind: 'proxy' | 'echo';
  upstream: FacetUpstream;
  routes: FacetRouteProxy[];
}

export type FacetDefinition = StaticFacetDefinition | ProxyFacetDefinition;

/**
 * Top-level manifest: a list of facets.
 * Actual on-disk / wire format (TOML/JSON/etc.) is handled by tooling;
 * this is the in-memory representation apps work with.
 */
export interface FacetManifest {
  facets: FacetDefinition[];
}

/**
 * Helper to define a static facet in a type-safe way.
 */
export function defineStaticFacet(args: {
  name: string;
  routes: FacetRouteStatic[];
  meta?: FacetMeta;
  security?: FacetSecurity;
}): StaticFacetDefinition {
  return {
    kind: 'static',
    ...args,
  };
}

/**
 * Helper to define a proxy (or echo) facet.
 */
export function defineProxyFacet(args: {
  name: string;
  upstream: FacetUpstream;
  routes: FacetRouteProxy[];
  meta?: FacetMeta;
  security?: FacetSecurity;
  kind?: 'proxy' | 'echo';
}): ProxyFacetDefinition {
  const { kind = 'proxy', ...rest } = args;
  return {
    kind,
    ...rest,
  };
}

/**
 * Helper to bundle a set of facets into a manifest.
 * This is primarily for tooling / future generators.
 */
export function buildFacetManifest(
  facets: FacetDefinition[],
): FacetManifest {
  return { facets };
}
```

This keeps all the existing types you already had, and adds small, ergonomic helpers for app devs:

```ts
const docsFacet = defineStaticFacet({
  name: 'docs',
  routes: [{ method: 'GET', path: '/docs', file: './public/docs.html' }],
});

const apiFacet = defineProxyFacet({
  name: 'kv',
  upstream: { scheme: 'http', host: 'kv-svc', port: 8080 },
  routes: [{ method: 'POST', path: '/kv/put', upstreamPath: '/put' }],
});

const manifest = buildFacetManifest([docsFacet, apiFacet]);
```

---

## 2) `sdk/ron-app-sdk-ts/src/index.ts`

Make the public surface a bit richer (while keeping current imports working).

Paste this over `src/index.ts`:

```ts
/**
 * ron-app-sdk-ts
 *
 * Public entrypoint. Re-export the primary Ron client, core types,
 * config helpers, and facet helpers.
 */

// Core types & client
export * from './types';
export * from './client';

// Local error / config helpers (optional use by callers)
export * from './errors';
export { resolveConfig } from './config';

// Facet helpers under a namespaced export to avoid polluting top-level names.
export * as Facets from './facets/manifest';
```

Nothing in your current tests depends on this, but consumers now get a nice top-level:

```ts
import { Ron, Facets } from 'ron-app-sdk-ts';
```

---

## 3) `sdk/ron-app-sdk-ts/docs/QUICKSTART.MD`

Finally, a small focused QuickStart that matches what you’re already doing in the README & tests.

Create or overwrite `docs/QUICKSTART.MD` with:

````md
# ron-app-sdk-ts QuickStart

This is the **short path** from zero to calling your RON-CORE node’s `/app/*`
surface from a TypeScript/JavaScript app.

---

## 1) Install

In your app project:

```bash
pnpm add ron-app-sdk-ts
# or
npm install ron-app-sdk-ts
# or
yarn add ron-app-sdk-ts
```
````

Requirements:

- Node **18+** (global `fetch`, `AbortController`) or a modern browser.
- TypeScript **5.x** recommended (strict mode).

---

## 2) Minimal usage

```ts
import { Ron } from 'ron-app-sdk-ts';

const ron = new Ron({
  baseUrl: 'https://my-node.example.com',
});

async function ping() {
  const res = await ron.get<{ pong: string }>('/ping');

  if (!res.ok || !res.data) {
    console.error('Ping failed:', res.problem);
    return;
  }

  console.log('Ping:', res.data.pong);
}
```

Key points:

- `baseUrl` points at your node’s **gateway** (no `/app` suffix).
- The SDK automatically calls `/app/ping` under the hood.
- You don’t need `try/catch` for app errors — they surface as `AppResponse`.

---

## 3) Adding auth & headers

```ts
const ron = new Ron({
  baseUrl: 'https://my-node.example.com',
  authToken: 'Bearer <access-token>',
  headerProvider: async () => ({
    'x-tenant-id': 'demo',
  }),
});
```

Secrets are **never** placed into URLs, and the SDK won’t log them by default.

---

## 4) Handling errors

```ts
const res = await ron.post<{ ok: boolean }>(
  '/kv/put',
  { key: 'foo', value: 'bar' },
  { region: 'us-west' },
);

if (res.ok && res.data) {
  console.log('KV put ok:', res.data.ok);
} else if (res.problem) {
  switch (res.problem.code) {
    case 'upstream_unavailable':
      // show "please retry later"
      break;
    case 'local_timeout':
      // show "check connection / retry"
      break;
    default:
    // generic error UI
  }
}
```

All app and transport errors flow through a **canonical `RonProblem`**, never raw
stack traces or unstructured exceptions.

---

## 5) Local dev with HTTP

By default, the SDK **requires HTTPS**. For local dev only:

```ts
const ron = new Ron({
  baseUrl: 'http://localhost:9090',
  allowInsecureHttp: true, // dev-only
});
```

In production, leave `allowInsecureHttp` unset so plain `http://` is rejected.

---

For deeper details (options, error taxonomy, SLOs, facet manifests), see the
main [`README.md`](../README.MD).

````

---

If you drop these in and rerun:

```bash
cd sdk/ron-app-sdk-ts
pnpm format
pnpm lint
pnpm build
pnpm test
````

you should stay green, with:

- Facet-level helpers in place,
- A richer public entrypoint,
- And a clean QuickStart doc wired into the docs story.
