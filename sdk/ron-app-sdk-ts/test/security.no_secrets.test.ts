import { describe, it, expect, vi } from 'vitest';
import { Ron } from '../src/client';

describe('security invariants — no secrets in URLs or error messages', () => {
  it('never places authToken/passportToken in request URL', async () => {
    const calls: Array<[string | URL, RequestInit | undefined]> = [];

    const fetchMock = vi.fn<[string | URL, RequestInit?], Promise<Response>>(
      async (url, init) => {
        calls.push([url, init]);
        return new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      },
    );

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = fetchMock as unknown as typeof fetch;

    const authToken = 'Bearer CAP_123_SECRET';
    const passportToken = 'PASS_456_SECRET';

    const ron = new Ron({
      baseUrl: 'https://example.com',
      authToken,
      passportToken,
    });

    const res = await ron.get('/secure', { q: '1' });
    expect(res.ok).toBe(true);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];

    const urlStr = String(url);
    expect(urlStr).toContain('/app/secure');
    expect(urlStr).not.toContain(authToken);
    expect(urlStr).not.toContain(passportToken);

    const headers = (init as RequestInit).headers as Record<string, string>;
    expect(headers.authorization).toBe(authToken);
    expect(headers['x-ron-passport']).toBe(passportToken);
  });

  it('does not leak tokens into local error messages', async () => {
    const authToken = 'Bearer CAP_TOKEN_ABC';
    const passportToken = 'PASS_TOKEN_DEF';

    const fetchMock = vi.fn<[string | URL, RequestInit?], Promise<Response>>(
      () => {
        return new Promise((_resolve, reject) => {
          const err = new Error('Network failed');
          reject(err);
        });
      },
    );

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = fetchMock as unknown as typeof fetch;

    const ron = new Ron({
      baseUrl: 'https://example.com',
      authToken,
      passportToken,
    });

    const res = await ron.get('/will-fail');

    expect(res.ok).toBe(false);
    expect(res.problem?.code).toBe('local_network_failure');
    expect(res.problem?.kind).toBe('transport');

    const msg = res.problem?.message ?? '';
    expect(msg).toBe('Network error');
    expect(msg).not.toContain(authToken);
    expect(msg).not.toContain(passportToken);
  });
});
