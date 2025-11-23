import { describe, it, expect, vi } from 'vitest';
import { Ron } from '../src/client';

interface HelloResponse {
  id: string; // u64 serialized as string
  message: string;
}

describe('schema interop', () => {
  it('preserves string IDs and JSON shape', async () => {
    const payload: HelloResponse = {
      id: '18446744073709551615',
      message: 'hi',
    };

    const fetchMock = vi.fn<[string | URL, RequestInit?], Promise<Response>>(
      async () => {
        return new Response(JSON.stringify(payload), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      },
    );

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = fetchMock as unknown as typeof fetch;

    const ron = new Ron({ baseUrl: 'https://example.com' });
    const res = await ron.get<HelloResponse>('/hello');

    expect(res.ok).toBe(true);
    expect(res.status).toBe(200);
    expect(res.data?.id).toBe(payload.id);
    expect(res.data?.message).toBe(payload.message);
  });

  it('maps error responses into RonProblem consistent with schema', async () => {
    const body = {
      code: 'forbidden',
      message: 'nope',
      kind: 'auth',
      correlation_id: 'corr-123',
      retryable: false,
      retry_after_ms: 0,
      reason: 'missing_cap',
      details: { cap: 'write:thing' },
    };

    const fetchMock = vi.fn<[string | URL, RequestInit?], Promise<Response>>(
      async () => {
        return new Response(JSON.stringify(body), {
          status: 403,
          headers: { 'content-type': 'application/json' },
        });
      },
    );

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = fetchMock as unknown as typeof fetch;

    const ron = new Ron({ baseUrl: 'https://example.com' });
    const res = await ron.get('/forbidden');

    expect(res.ok).toBe(false);
    expect(res.status).toBe(403);
    expect(res.problem).toMatchObject({
      code: 'forbidden',
      message: 'nope',
      kind: 'auth',
      correlationId: 'corr-123',
      retryable: false,
      retryAfterMs: 0,
      reason: 'missing_cap',
      details: { cap: 'write:thing' },
    });
  });
});
