import React, { useEffect, useMemo, useState } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { adminClient, isHttpError } from '../api/adminClient'

type Phase = 'idle' | 'checking' | 'submitting'

type NavState =
  | {
      from?: string
    }
  | {
      from?: { pathname?: string; search?: string }
    }

function normalizeFrom(state: unknown): string {
  const st = (state ?? {}) as NavState

  // string form: { from: "/path?x=1" }
  if (typeof (st as any).from === 'string') {
    const s = String((st as any).from).trim()
    return s.length > 0 ? s : '/'
  }

  // location-like form: { from: { pathname, search } }
  const obj = (st as any).from
  if (obj && typeof obj === 'object') {
    const pn = typeof obj.pathname === 'string' ? obj.pathname : ''
    const qs = typeof obj.search === 'string' ? obj.search : ''
    const out = (pn + qs).trim()
    return out.length > 0 ? out : '/'
  }

  return '/'
}

export function LoginPage() {
  const nav = useNavigate()
  const loc = useLocation()

  const [phase, setPhase] = useState<Phase>('checking')
  const [username, setUsername] = useState('admin')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)

  const from = useMemo(() => normalizeFrom(loc.state), [loc.state])

  const canSubmit = useMemo(() => {
    return phase !== 'submitting' && username.trim().length > 0 && password.length > 0
  }, [phase, username, password])

  // If already authenticated, bounce to "from".
  useEffect(() => {
    let alive = true
    ;(async () => {
      setPhase('checking')
      setError(null)
      try {
        await adminClient.getMe()
        if (!alive) return
        nav(from, { replace: true })
      } catch (err: any) {
        if (!alive) return

        // 401 is expected: show the login form.
        if (isHttpError(err) && err.status === 401) {
          setPhase('idle')
          return
        }

        // Anything else: surface context but still allow login.
        if (isHttpError(err)) {
          const statusNum = typeof err.status === 'number' ? err.status : undefined
          setError(statusNum ? `Unable to check session (HTTP ${statusNum}).` : `Unable to check session.`)
        } else {
          setError(`Unable to check session.`)
        }

        setPhase('idle')
      }
    })()
    return () => {
      alive = false
    }
  }, [nav, from])

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!canSubmit) return

    setPhase('submitting')
    setError(null)

    const u = username.trim()

    try {
      await adminClient.login(u, password)

      // Confirm session is live (cookie round-trip).
      await adminClient.getMe()

      nav(from, { replace: true })
    } catch (err: any) {
      if (isHttpError(err)) {
        const statusNum = typeof err.status === 'number' ? err.status : undefined

        if (statusNum === 404) {
          setError(
            `Login endpoint not found (404). This usually means local auth routes are not mounted, or the server is not running in local auth mode.`
          )
        } else if (statusNum === 401) {
          setError(`Login failed (401). Check username/password.`)
        } else if (statusNum) {
          setError(`Login failed (${statusNum}). Check username/password and server auth mode.`)
        } else {
          setError(`Login failed. Check username/password and server auth mode.`)
        }
      } else {
        setError(`Login failed: ${err?.message ? String(err.message) : 'unknown error'}`)
      }
      setPhase('idle')
    }
  }

  const disabled = phase === 'checking' || phase === 'submitting'

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
          width: 'min(520px, 100%)',
          borderRadius: 14,
          background: 'var(--svc-admin-color-bg-elevated)',
          boxShadow: 'var(--svc-admin-shadow-soft)',
          border: '1px solid var(--svc-admin-color-border-subtle)',
          padding: 18,
        }}
      >
        <div style={{ marginBottom: 14 }}>
          {/* Centered brand header */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 10,
              fontSize: 18,
              fontWeight: 800,
              letterSpacing: 0.2,
              marginBottom: 8,
              width: '100%',
              textAlign: 'center',
            }}
          >
            <span aria-hidden>🦀</span>
            <span>RON-CORE</span>
            <span aria-hidden>🧅</span>
          </div>

          <h1 style={{ margin: 0, fontSize: 22 }}>Login</h1>
          <p style={{ margin: '8px 0 0 0', color: 'var(--svc-admin-color-text-muted)' }}>
            Local session login (cookie-based). If you’re using SSO/ingress auth, this page may not be used.
          </p>
          {from !== '/' ? (
            <p style={{ margin: '8px 0 0 0', fontSize: 12, color: 'var(--svc-admin-color-text-muted)' }}>
              After sign-in, you’ll return to: <code>{from}</code>
            </p>
          ) : null}
        </div>

        {error ? (
          <div
            style={{
              marginBottom: 12,
              padding: 12,
              borderRadius: 10,
              background: 'var(--svc-admin-color-danger-soft)',
              border: '1px solid var(--svc-admin-color-danger-border)',
              color: 'var(--svc-admin-color-danger-text)',
            }}
          >
            {error}
          </div>
        ) : null}

        <form onSubmit={onSubmit}>
          <div style={{ display: 'grid', gap: 10 }}>
            <label style={{ display: 'grid', gap: 6 }}>
              <span style={{ fontSize: 13, fontWeight: 600 }}>Username</span>
              <input
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                autoComplete="username"
                spellCheck={false}
                disabled={disabled}
                style={{
                  padding: '10px 12px',
                  borderRadius: 10,
                  border: '1px solid var(--svc-admin-color-border-subtle)',
                  background: 'transparent',
                  color: 'var(--svc-admin-color-text)',
                  outline: 'none',
                  opacity: disabled ? 0.85 : 1,
                }}
              />
            </label>

            <label style={{ display: 'grid', gap: 6 }}>
              <span style={{ fontSize: 13, fontWeight: 600 }}>Password</span>
              <input
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                type="password"
                autoComplete="current-password"
                disabled={disabled}
                style={{
                  padding: '10px 12px',
                  borderRadius: 10,
                  border: '1px solid var(--svc-admin-color-border-subtle)',
                  background: 'transparent',
                  color: 'var(--svc-admin-color-text)',
                  outline: 'none',
                  opacity: disabled ? 0.85 : 1,
                }}
              />
            </label>

            <button
              type="submit"
              disabled={!canSubmit || phase === 'checking'}
              style={{
                marginTop: 6,
                padding: '10px 12px',
                borderRadius: 10,
                border: '1px solid var(--svc-admin-color-border-subtle)',
                background: 'var(--svc-admin-color-accent-soft)',
                color: 'var(--svc-admin-color-text)',
                fontWeight: 700,
                cursor: canSubmit && phase !== 'checking' ? 'pointer' : 'not-allowed',
                opacity: canSubmit && phase !== 'checking' ? 1 : 0.6,
              }}
            >
              {phase === 'checking' ? 'Checking session…' : phase === 'submitting' ? 'Signing in…' : 'Sign in'}
            </button>

            <div style={{ marginTop: 10, fontSize: 12, color: 'var(--svc-admin-color-text-muted)' }}>
              Tip: for a fresh local install, set the bootstrap env var you configured (commonly{' '}
              <code>SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD</code>) so the server creates the initial <code>admin</code> user.
            </div>
          </div>
        </form>
      </div>
    </div>
  )
}
