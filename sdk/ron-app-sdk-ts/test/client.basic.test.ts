import { describe, it, expect, vi } from 'vitest';
import { Ron } from '../src/client';
import type { AppResponse } from '../src/types';

describe('Ron client (basic)', () => {
  it('constructs with baseUrl', () => {
    const ron = new Ron({ baseUrl: 'https://example.com' });
    expect(ron).toBeInstanceOf(Ron);
  });

  it('performs a basic GET and maps JSON response', async () => {
    const payload = { message: 'hello' };

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

    const ron = new Ron({ baseUrl: 'https://my-node.example.com' });

    const res = await ron.get<typeof payload>('/hello');
    const typed = res as AppResponse<typeof payload>;

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];

    expect(String(url)).toBe('https://my-node.example.com/app/hello');
    expect((init as RequestInit).method).toBe('GET');

    expect(typed.ok).toBe(true);
    expect(typed.status).toBe(200);
    expect(typed.data).toEqual(payload);
  });
});
