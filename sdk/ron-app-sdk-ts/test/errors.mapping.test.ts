import { describe, it, expect } from 'vitest';
import { parseProblem } from '../src/utils/problem';

describe('error mapping → RonProblem', () => {
  it('parses canonical Problem JSON', async () => {
    const body = {
      code: 'bad_request',
      message: 'Nope',
      kind: 'validation',
      correlation_id: 'corr-123',
      retryable: false,
      retry_after_ms: 0,
      reason: 'bad_input',
      details: { field: 'name' },
    };

    const res = new Response(JSON.stringify(body), {
      status: 400,
      headers: { 'content-type': 'application/json' },
    });

    const problem = await parseProblem(res);

    expect(problem).toMatchObject({
      code: 'bad_request',
      message: 'Nope',
      kind: 'validation',
      correlationId: 'corr-123',
      retryable: false,
      retryAfterMs: 0,
      reason: 'bad_input',
      details: { field: 'name' },
    });
  });

  it('falls back to transport_error for non-JSON bodies', async () => {
    const res = new Response('oops', {
      status: 502,
      headers: { 'content-type': 'text/plain' },
    });

    const problem = await parseProblem(res);

    expect(problem.code).toBe('transport_error');
    expect(problem.kind).toBe('transport');
  });

  it('handles malformed JSON gracefully', async () => {
    const res = new Response('not-json', {
      status: 500,
      headers: { 'content-type': 'application/json' },
    });

    const problem = await parseProblem(res);

    expect(problem.code).toBe('transport_error');
    expect(problem.kind).toBe('transport');
  });
});
