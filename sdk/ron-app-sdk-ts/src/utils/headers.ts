/**
 * Header utilities: merge user headers with required SDK headers,
 * and generate request/correlation IDs.
 */

import type { AppRequest } from '../types';

export function generateRequestId(): string {
  // Simple placeholder; will be replaced with a UUIDv4 implementation.
  return `ron-${Math.random().toString(16).slice(2)}-${Date.now()}`;
}

export function buildHeaders(
  base: Record<string, string> | undefined,
  extra: Record<string, string>,
): Record<string, string> {
  return { ...(base ?? {}), ...extra };
}

export function applyObservabilityHeaders(
  req: AppRequest,
  requestIdFactory: () => string,
): AppRequest {
  const requestId = requestIdFactory();
  const headers = buildHeaders(req.headers, {
    'x-request-id': requestId,
    'x-correlation-id': requestId,
  });
  return { ...req, headers };
}
