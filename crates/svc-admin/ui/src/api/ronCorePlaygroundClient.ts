// crates/svc-admin/ui/src/api/ronCorePlaygroundClient.ts
//
// RO:WHAT — Thin fetch wrapper for svc-admin “App Playground” endpoints.
// RO:WHY  — Keep playground HTTP paths + DTO wiring isolated so UI routes/components
//           stay simple and we preserve the dev-only posture.
// RO:INTERACTS — types/admin-api.ts, Rust router.rs: /api/playground/*
// RO:INVARIANTS —
//   - Playground is dev-only; backend may return 404 when disabled (treat as “off”).
//   - No arbitrary proxying; only curated endpoints are called.
//   - Keep requests small; do not send large payloads.
// RO:SECURITY — No auth headers added here; browser auth/session (if any) is handled by same-origin.
// RO:TEST — Covered indirectly by UI smoke + manual dev flows.

import type {
  PlaygroundExample,
  PlaygroundValidateManifestResponse,
} from '../types/admin-api'

// Base URL strategy:
//
// - In dev, use relative URLs ("/api/...") so Vite proxies to backend (no CORS).
// - In production, SPA is served by svc-admin, so relative URLs still work.
// - If you want remote svc-admin, set VITE_SVC_ADMIN_BASE_URL (or VITE_API_BASE_URL).
const RAW_BASE_URL: string =
  (import.meta as any).env?.VITE_SVC_ADMIN_BASE_URL ??
  (import.meta as any).env?.VITE_API_BASE_URL ??
  ''

function buildUrl(path: string): string {
  if (!RAW_BASE_URL) return path
  const base = RAW_BASE_URL.replace(/\/+$/, '')
  return `${base}${path}`
}

export type FetchError = Error & { status?: number; statusText?: string }

async function handleResponse<T>(res: Response): Promise<T> {
  if (!res.ok) {
    let bodyText = ''
    try {
      bodyText = await res.text()
    } catch {
      // ignore
    }

    const msg =
      bodyText && bodyText.length < 1024
        ? `Request failed: ${res.status} ${res.statusText} - ${bodyText}`
        : `Request failed: ${res.status} ${res.statusText}`

    const err = new Error(msg) as FetchError
    err.status = res.status
    err.statusText = res.statusText
    throw err
  }

  const text = await res.text()
  if (!text) return undefined as unknown as T
  return JSON.parse(text) as T
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(buildUrl(path), init)
  return handleResponse<T>(res)
}

function is404(err: unknown): boolean {
  const e = err as FetchError
  return typeof e?.status === 'number' && e.status === 404
}

export const ronCorePlaygroundClient = {
  /**
   * Returns dev-only examples for the playground.
   *
   * Backend returns 404 when playground is disabled.
   */
  async getExamples(): Promise<PlaygroundExample[]> {
    return fetchJson<PlaygroundExample[]>('/api/playground/examples')
  },

  /**
   * Validate a manifest TOML payload.
   *
   * Backend returns 404 when playground is disabled.
   */
  async validateManifest(
    manifestToml: string,
  ): Promise<PlaygroundValidateManifestResponse> {
    return fetchJson<PlaygroundValidateManifestResponse>(
      '/api/playground/manifest/validate',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ manifestToml }),
      },
    )
  },

  /**
   * Convenience probe: returns true if playground appears enabled on this svc-admin.
   * If the endpoint is hidden (404), returns false.
   */
  async isEnabled(): Promise<boolean> {
    try {
      await ronCorePlaygroundClient.getExamples()
      return true
    } catch (err) {
      if (is404(err)) return false
      // For non-404 errors (network, 500, etc.), treat as “enabled but unhealthy”
      // so callers can still show a useful error state rather than “disabled”.
      return true
    }
  },
}
