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
export function buildFacetManifest(facets: FacetDefinition[]): FacetManifest {
  return { facets };
}
