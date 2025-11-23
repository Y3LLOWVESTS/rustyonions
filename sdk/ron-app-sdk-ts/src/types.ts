/**
 * Core types for ron-app-sdk-ts.
 *
 * These types are aligned with the SDK_IDB and SDK_SCHEMA_IDB documents.
 */

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface AppRequest {
  method: HttpMethod;
  path: string;
  query?: Record<string, string>;
  headers?: Record<string, string>;
  body?: unknown;
}

export interface RonProblem {
  code: string;
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
  ok: boolean;
  headers: Record<string, string>;
  data?: T;
  raw?: ArrayBuffer;
  problem?: RonProblem;
}

export interface RonOptions {
  baseUrl: string;
  authToken?: string;
  passportToken?: string;
  overallTimeoutMs?: number;
  connectTimeoutMs?: number;
  readTimeoutMs?: number;
  requestIdFactory?: () => string;
  onRequest?: (req: AppRequest & { url: string }) => void;
  onResponse?: <T>(res: AppResponse<T>) => void;
  headerProvider?: () =>
    | Record<string, string>
    | Promise<Record<string, string>>;
  allowInsecureHttp?: boolean;
  debug?: boolean;
}
