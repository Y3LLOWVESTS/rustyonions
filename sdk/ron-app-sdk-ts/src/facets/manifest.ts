/**
 * Facet manifest types and future helpers for emitting TOML facet configs.
 * This is a scaffold aligned with the SDK_SCHEMA_IDB manifest schema.
 */

export type FacetKind = 'static' | 'echo' | 'proxy';

export interface FacetSecurity {
  public?: boolean;
  requiresAuth?: boolean;
}

export interface FacetMeta {
  description?: string;
  owner?: string;
  version?: string;
}

export interface FacetRouteStatic {
  method: 'GET' | 'HEAD';
  path: string;
  file: string;
}

export interface FacetUpstream {
  scheme: 'http' | 'https';
  host: string;
  port: number;
  basePath?: string;
}

export interface FacetRouteProxy {
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  path: string;
  upstreamPath?: string;
}
