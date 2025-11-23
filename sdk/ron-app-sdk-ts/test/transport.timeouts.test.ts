import { describe, it, expect, vi } from 'vitest';
import { Ron } from '../src/client';

describe('transport timeouts', () => {
  it('maps AbortError into local_timeout RonProblem', async () => {
    vi.useFakeTimers();

    const abortingFetch = vi.fn<
      [string | URL, RequestInit?],
      Promise<Response>
    >((_url, init) => {
      return new Promise<Response>((_resolve, reject) => {
        const signal = init?.signal as AbortSignal | undefined;
        if (signal) {
          signal.addEventListener('abort', () => {
            const err = new Error('Aborted');
            (err as { name: string }).name = 'AbortError';
            reject(err);
          });
        }
      });
    });

    const g = globalThis as typeof globalThis & { fetch: typeof fetch };
    g.fetch = abortingFetch as unknown as typeof fetch;

    const ron = new Ron({
      baseUrl: 'https://example.com',
      overallTimeoutMs: 50,
    });

    const promise = ron.get('/timeout');

    await vi.advanceTimersByTimeAsync(51);

    const res = await promise;

    expect(res.ok).toBe(false);
    expect(res.status).toBe(0);
    expect(res.problem?.code).toBe('local_timeout');
    expect(res.problem?.kind).toBe('transport');
    expect(res.problem?.retryable).toBe(true);

    vi.useRealTimers();
  });
});
