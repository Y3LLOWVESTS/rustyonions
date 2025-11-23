/**
 * Core types for ron-app-sdk-ts.
 *
 * These types are aligned with the SDK_IDB and SDK_SCHEMA_IDB documents.
 */

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface AppRequest {
  method: HttpMethod;
  /**
   * App-relative path, e.g. "/hello" or "/auth/login".
   * The SDK will prepend "/app" and join with the baseUrl.
   */
  path: string;
  /**
   * Query parameters to append to the URL.
   */
  query?: Record<string, string>;
  /**
   * Request body. For non-GET/HEAD methods, this will be encoded as JSON
   * by default unless the caller passes a BodyInit-like type.
   */
  body?: unknown;
  /**
   * Additional headers to include on the request.
   */
  headers?: Record<string, string>;
}

/**
 * Canonical Problem envelope returned by gateway/omnigate or synthesized by
 * the SDK for local failures.
 */
export interface RonProblem {
  code: string;
  message: string;
  kind?: string;
  correlationId?: string;
  retryable?: boolean;
  retryAfterMs?: number;
  reason?: string;
  details?: unknown;
}

/**
 * Generic app response wrapper.
 *
 * - On success: ok=true, data/raw set, problem undefined.
 * - On failure: ok=false, problem set, data/raw usually undefined.
 */
export interface AppResponse<T = unknown> {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  data?: T;
  raw?: ArrayBuffer;
  problem?: RonProblem;
}

/**
 * Top-level configuration for a Ron client instance.
 */
export interface RonOptions {
  /**
   * Base URL of the gateway (e.g. "https://node.example.com").
   * The SDK will call `${baseUrl}/app/...`.
   */
  baseUrl: string;

  /**
   * Overall timeout in milliseconds (connect + read).
   * If exceeded, the SDK returns a synthetic AppResponse with
   * status=0 and problem.code = "local_timeout".
   */
  overallTimeoutMs?: number;

  /**
   * Optional lower-level timeouts (not yet wired for all transports).
   */
  connectTimeoutMs?: number;
  readTimeoutMs?: number;

  /**
   * Factory for request IDs. If omitted, the SDK will generate its own
   * opaque ID. The same value is used for x-request-id and, by default,
   * x-correlation-id when the caller does not set one.
   */
  requestIdFactory?: () => string;

  /**
   * Optional callback invoked before each request is sent.
   *
   * NOTE: Named `_req` so ESLint does not consider it unused.
   */
  onRequest?: (_req: AppRequest & { url: string }) => void;

  /**
   * Optional callback invoked after each response (success or failure).
   *
   * NOTE: Named `_res` so ESLint does not consider it unused.
   */
  onResponse?: <T>(_res: AppResponse<T>) => void;

  /**
   * Optional header provider for dynamic/rotating headers.
   * This is evaluated per-call.
   */
  headerProvider?:
    | (() => Record<string, string>)
    | (() => Promise<Record<string, string>>);

  /**
   * Auth token (e.g. "Bearer <capability>") attached as Authorization header.
   */
  authToken?: string;

  /**
   * Passport token attached as X-RON-Passport header.
   */
  passportToken?: string;

  /**
   * Allow plain HTTP in development. In production this should be false.
   */
  allowInsecureHttp?: boolean;

  /**
   * Enable additional debug behaviour (reserved for future use).
   */
  debug?: boolean;
}
