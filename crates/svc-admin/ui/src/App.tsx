/**
 * RO:WHAT — Top-level route table for the svc-admin SPA.
 * RO:WHY — Keeps navigation explicit and stable as the dashboard grows (DX).
 * RO:INVARIANTS — NotFound is last; no nested Routers; routes are client-side only.
 */

// crates/svc-admin/ui/src/App.tsx

import React, { useEffect, useMemo, useState } from 'react'
import { Routes, Route, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { Shell } from './components/layout/Shell'
import { NodeListPage } from './routes/NodeListPage'
import { NodeDetailPage } from './routes/NodeDetailPage'
import { NodeStoragePage } from './routes/NodeStoragePage'
import { NodeDatabaseDetailPage } from './routes/NodeDatabaseDetailPage'
import { PlaygroundPage } from './routes/PlaygroundPage'
import { SettingsPage } from './routes/SettingsPage'
import { LoginPage } from './routes/LoginPage'
import { NotFoundPage } from './routes/NotFoundPage'
import { BenchmarksPage } from './routes/BenchmarksPage'
import { adminClient, isHttpError } from './api/adminClient'

type GatePhase = 'checking' | 'authed' | 'redirecting' | 'error'

function ShellLayout() {
  return (
    <Shell>
      <Outlet />
    </Shell>
  )
}

function FullPageStatus(props: { title: string; body?: string }) {
  return (
    <div
      style={{
        minHeight: 'calc(100vh - 40px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
      }}
    >
      <div
        style={{
          width: 'min(720px, 100%)',
          borderRadius: 14,
          background: 'var(--svc-admin-color-bg-elevated)',
          boxShadow: 'var(--svc-admin-shadow-soft)',
          border: '1px solid var(--svc-admin-color-border-subtle)',
          padding: 18,
        }}
      >
        <h2 style={{ margin: 0, fontSize: 18 }}>{props.title}</h2>
        {props.body ? (
          <p style={{ marginTop: 10, color: 'var(--svc-admin-color-text-muted)' }}>{props.body}</p>
        ) : null}
      </div>
    </div>
  )
}

/**
 * RequireAuth:
 * - Runs a session check via adminClient.getMe().
 * - If 401: redirects to /login and preserves the original destination in state.from
 * - Other errors: show a helpful error panel
 */
function RequireAuth() {
  const nav = useNavigate()
  const loc = useLocation()

  const [phase, setPhase] = useState<GatePhase>('checking')
  const [errMsg, setErrMsg] = useState<string | null>(null)

  const from = useMemo(() => {
    const path = loc.pathname + (loc.search || '')
    return path.trim().length > 0 ? path : '/'
  }, [loc.pathname, loc.search])

  useEffect(() => {
    let alive = true

    ;(async () => {
      setPhase('checking')
      setErrMsg(null)

      try {
        await adminClient.getMe()
        if (!alive) return
        setPhase('authed')
      } catch (err: any) {
        if (!alive) return

        if (isHttpError(err) && err.status === 401) {
          setPhase('redirecting')
          nav('/login', {
            replace: true,
            state: {
              // store both string and location-like (LoginPage accepts either)
              from: { pathname: loc.pathname, search: loc.search || '' },
            },
          })
          return
        }

        const status = isHttpError(err) && typeof err.status === 'number' ? ` (HTTP ${err.status})` : ''
        setErrMsg(
          `Unable to verify session${status}. If you are running local auth, you may need to log in. If you are running ingress/SSO auth, check required identity headers/proxy setup.`
        )
        setPhase('error')
      }
    })()

    return () => {
      alive = false
    }
  }, [nav, loc.key, loc.pathname, loc.search])

  if (phase === 'checking') {
    return <FullPageStatus title="Checking session…" />
  }

  if (phase === 'redirecting') {
    return <FullPageStatus title="Redirecting to login…" />
  }

  if (phase === 'error') {
    return <FullPageStatus title="Session check failed" body={errMsg ?? undefined} />
  }

  return <Outlet />
}

export function App() {
  return (
    <Routes>
      {/* Public */}
      <Route path="/login" element={<LoginPage />} />

      {/* Protected (auth gate first, then shell layout) */}
      <Route element={<RequireAuth />}>
        <Route element={<ShellLayout />}>
          <Route path="/" element={<NodeListPage />} />
          <Route path="/nodes/:id" element={<NodeDetailPage />} />
          <Route path="/nodes/:id/storage" element={<NodeStoragePage />} />
          <Route path="/nodes/:id/storage/databases/:name" element={<NodeDatabaseDetailPage />} />
          <Route path="/benchmarks" element={<BenchmarksPage />} />
          <Route path="/playground" element={<PlaygroundPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<NotFoundPage />} />
        </Route>
      </Route>
    </Routes>
  )
}
