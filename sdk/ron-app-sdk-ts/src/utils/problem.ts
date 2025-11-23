/**
 * Problem utilities: parse canonical RON Problem JSON into RonProblem.
 */

import type { RonProblem } from '../types';

function fallbackProblem(message: string): RonProblem {
  return {
    code: 'transport_error',
    message,
    kind: 'transport',
    retryable: false,
  };
}

export async function parseProblem(res: Response): Promise<RonProblem> {
  const contentType = res.headers.get('content-type') ?? '';
  if (!contentType.includes('application/json')) {
    return fallbackProblem('Non-JSON problem response');
  }

  let parsed: unknown;
  try {
    parsed = await res.json();
  } catch {
    return fallbackProblem('Failed to parse error body');
  }

  if (parsed === null || typeof parsed !== 'object') {
    return fallbackProblem('Malformed problem body');
  }

  const obj = parsed as Record<string, unknown>;

  const getString = (value: unknown, def?: string): string | undefined => {
    if (typeof value === 'string') {
      return value;
    }
    return def;
  };

  const getBoolean = (value: unknown): boolean | undefined =>
    typeof value === 'boolean' ? value : undefined;

  const getNumber = (value: unknown): number | undefined =>
    typeof value === 'number' ? value : undefined;

  const code = getString(obj['code'], 'unknown_error')!;
  const message = getString(obj['message']) ?? `HTTP ${res.status.toString()}`;
  const kind = getString(obj['kind']);
  const correlationId = getString(obj['correlation_id']);
  const retryable =
    getBoolean(obj['retryable']) ?? (res.status >= 500 && res.status < 600);
  const retryAfterMs = getNumber(obj['retry_after_ms']);
  const reason = getString(obj['reason']);
  const details = obj['details'];

  const problem: RonProblem = {
    code,
    message,
    kind,
    correlationId,
    retryable,
    retryAfterMs,
    reason,
    details,
  };

  return problem;
}
