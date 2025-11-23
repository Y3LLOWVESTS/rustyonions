/**
 * Ron client
 *
 * High-level client for calling /app/* on a RON-CORE node.
 *
 * Facet-level helpers:
 *   const ron = new Ron({ baseUrl: "https://node" });
 *   const auth = ron.facet("auth");
 *   const res = await auth.post("/login", { user, pass });
 *
 * This keeps app code from hard-coding "/auth/..." paths everywhere.
 */

import type { AppRequest, AppResponse, RonOptions } from './types';
import { resolveConfig } from './config';
import { sendRequest } from './transport';

/**
 * Primary SDK client for talking to the RON app plane.
 */
export class Ron {
  private readonly options: RonOptions;

  constructor(options: RonOptions) {
    // Enforce config invariants (baseUrl, HTTPS-by-default, env overrides).
    this.options = resolveConfig(options);
  }

  /**
   * Low-level request primitive.
   *
   * Responsibilities:
   * - Use resolved config (baseUrl, timeouts, auth/passport, hooks).
   * - Delegate to the transport layer (URL building, headers, fetch, problems).
   * - Never throw for server-originated problems; those land in AppResponse.problem.
   */
  async request<T = unknown>(req: AppRequest): Promise<AppResponse<T>> {
    return sendRequest<T>(this.options, req);
  }

  /**
   * Convenience HTTP helpers operating on app-plane paths.
   *
   * Paths are always relative to /app on the gateway:
   *   ron.get("/hello")   → GET https://node/app/hello
   */

  async get<T = unknown>(
    path: string,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'GET', path, query });
  }

  async post<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'POST', path, body, query });
  }

  async put<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'PUT', path, body, query });
  }

  async patch<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'PATCH', path, body, query });
  }

  async delete<T = unknown>(
    path: string,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'DELETE', path, query });
  }

  /**
   * Facet helper:
   *
   *   const auth = ron.facet("auth");
   *   await auth.post("/login", { user, pass });
   *
   * This prefixes all paths with "/{facetId}" so app code doesn’t have to
   * repeat facet segments by hand. The resulting URL becomes:
   *   baseUrl + "/app/" + facetId + path
   */
  facet(facetId: string): FacetClient {
    return new FacetClient(this, facetId);
  }
}

/**
 * Facet-scoped client that prefixes all paths with "/{facetId}" and
 * forwards to the underlying Ron instance.
 *
 * It is intentionally thin sugar on top of Ron’s HTTP methods.
 */
export class FacetClient {
  private readonly ron: Ron;
  private readonly facetId: string;

  constructor(ron: Ron, facetId: string) {
    this.ron = ron;
    this.facetId = facetId;
  }

  private buildPath(path: string): string {
    const normalized = path.startsWith('/') ? path : `/${path}`;
    // Result: "/facetId/normalizedPath"
    return `/${this.facetId}${normalized}`;
  }

  async get<T = unknown>(
    path: string,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.ron.get<T>(this.buildPath(path), query);
  }

  async post<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.ron.post<T>(this.buildPath(path), body, query);
  }

  async put<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.ron.put<T>(this.buildPath(path), body, query);
  }

  async patch<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.ron.patch<T>(this.buildPath(path), body, query);
  }

  async delete<T = unknown>(
    path: string,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.ron.delete<T>(this.buildPath(path), query);
  }
}
