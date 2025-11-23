/**
 * URL utilities: validate baseUrl, build /app/* URLs, and attach query strings.
 */

export function joinBaseAndPath(baseUrl: string, path: string): string {
  const base = baseUrl.replace(/\/+$/, '');
  const p = path.startsWith('/') ? path : `/${path}`;
  return `${base}/app${p}`;
}

export function toQueryString(
  query?: Record<string, string>,
): string | undefined {
  if (!query) return undefined;
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) {
    params.set(k, v);
  }
  const s = params.toString();
  return s.length ? `?${s}` : undefined;
}
