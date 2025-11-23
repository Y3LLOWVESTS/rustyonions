import { describe, it, expect, vi } from 'vitest';
import { Ron } from '../src/client';

describe('FacetClient', () => {
  it('prefixes paths with "/{facetId}" when called with a leading slash', async () => {
    const fetchMock = vi.fn<[string | URL, RequestInit?], Promise<Response>>(
      async () => {
        return new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      },
    );

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = fetchMock as unknown as typeof fetch;

    const ron = new Ron({ baseUrl: 'https://example.com' });
    const auth = ron.facet('auth');

    await auth.post('/login', { user: 'alice' });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];

    expect(String(url)).toBe('https://example.com/app/auth/login');
    const initTyped = init as RequestInit;
    expect(initTyped.method).toBe('POST');

    const body = initTyped.body as string | undefined;
    expect(body).toBeDefined();
    if (body) {
      const parsed = JSON.parse(body) as { user?: string };
      expect(parsed.user).toBe('alice');
    }
  });

  it('prefixes paths with "/{facetId}" when called without a leading slash', async () => {
    const fetchMock = vi.fn<[string | URL, RequestInit?], Promise<Response>>(
      async () => {
        return new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      },
    );

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = fetchMock as unknown as typeof fetch;

    const ron = new Ron({ baseUrl: 'https://example.com' });
    const users = ron.facet('users');

    await users.get('me');

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];

    expect(String(url)).toBe('https://example.com/app/users/me');
    const initTyped = init as RequestInit;
    expect(initTyped.method).toBe('GET');
  });
});
