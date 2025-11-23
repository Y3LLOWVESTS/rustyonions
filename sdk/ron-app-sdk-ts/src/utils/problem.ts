/**
 * Problem utilities: parse canonical RON Problem JSON into RonProblem.
 */

import type { RonProblem } from '../types';

export async function parseProblem(
  res: Response,
): Promise<RonProblem | undefined> {
  const contentType = res.headers.get('content-type') ?? '';
  if (!contentType.includes('application/json')) {
    return {
      code: 'transport_error',
      message: 'Non-JSON error response',
      kind: 'transport',
      retryable: false,
    };
  }

  try {
    const body = (await res.json()) as any;
    const problem: RonProblem = {
      code: String(body.code ?? 'unknown_error'),
      message: typeof body.message === 'string' ? body.message : undefined,
      kind: typeof body.kind === 'string' ? body.kind : undefined,
      correlationId:
        typeof body.correlation_id === 'string'
          ? body.correlation_id
          : undefined,
      retryable:
        typeof body.retryable === 'boolean' ? body.retryable : undefined,
      retryAfterMs:
        typeof body.retry_after_ms === 'number'
          ? body.retry_after_ms
          : undefined,
      reason: typeof body.reason === 'string' ? body.reason : undefined,
      details:
        body.details && typeof body.details === 'object'
          ? (body.details as Record<string, unknown>)
          : undefined,
    };
    return problem;
  } catch {
    return {
      code: 'transport_error',
      message: 'Failed to parse error body',
      kind: 'transport',
      retryable: false,
    };
  }
}
