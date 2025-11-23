/**
 * Timeout utilities: create AbortSignal for overall request timeout.
 */

export function withTimeout(
  ms: number,
  controller?: AbortController,
): { signal: AbortSignal; cancel: () => void } {
  const c = controller ?? new AbortController();
  const id = setTimeout(() => c.abort(), ms);
  const cancel = () => clearTimeout(id);
  return { signal: c.signal, cancel };
}
