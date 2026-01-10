// crates/svc-admin/ui/src/components/layout/TopBar.tsx
//
// WHAT: Top navigation bar for the svc-admin SPA.
// WHY:  Central place for brand, theme + language toggles, and the
//       current operator identity (from `/api/me` or local `/api/auth/me`).

import React, { useEffect, useMemo, useState } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { ThemeToggle } from './ThemeToggle'
import { LanguageSwitcher } from './LanguageSwitcher'
import { adminClient, isHttpError } from '../../api/adminClient'
import type { MeResponse } from '../../types/admin-api'

type MeState = {
  loading: boolean
  error: string | null
  me: MeResponse | null
}

function displayNameOf(me: any): string {
  if (me && typeof me.displayName === 'string' && me.displayName.trim().length > 0) return me.displayName
  if (me && typeof me.username === 'string' && me.username.trim().length > 0) return me.username
  if (me && typeof me.subject === 'string' && me.subject.trim().length > 0) return me.subject
  return 'Operator'
}

function rolesOf(me: any): string[] {
  return me && Array.isArray(me.roles) ? me.roles : []
}

function authModeOf(me: any): string {
  // /api/me shape includes authMode; /api/auth/me does not, but implies local session.
  if (me && typeof me.authMode === 'string' && me.authMode.trim().length > 0) return me.authMode
  if (me && typeof me.expiresAtUnixS === 'number') return 'local'
  return 'unknown'
}

function isLocalSession(me: any): boolean {
  // If we came from /api/auth/me we likely have expiresAtUnixS.
  if (me && typeof me.expiresAtUnixS === 'number') return true
  return me && typeof me.authMode === 'string' && me.authMode === 'local'
}

export function TopBar() {
  const nav = useNavigate()
  const loc = useLocation()

  const [state, setState] = useState<MeState>({
    loading: true,
    error: null,
    me: null
  })

  const [logoutBusy, setLogoutBusy] = useState(false)

  useEffect(() => {
    let cancelled = false

    async function refresh() {
      setState((s) => ({ ...s, loading: true, error: null }))

      try {
        const me = await adminClient.getMe()
        if (cancelled) return
        setState({
          loading: false,
          error: null,
          me
        })
      } catch (err: any) {
        if (cancelled) return

        // 401 means "not logged in".
        if (isHttpError(err) && err.status === 401) {
          setState({
            loading: false,
            error: null,
            me: null
          })
          return
        }

        setState({
          loading: false,
          error: 'identity-unavailable',
          me: null
        })
      }
    }

    // Refresh on mount and whenever route changes (covers post-login redirect).
    refresh()

    return () => {
      cancelled = true
    }
  }, [loc.pathname, loc.search])

  const { loading, error, me } = state

  const identityContent = useMemo((): React.ReactNode => {
    if (loading) {
      return (
        <span className="svc-admin-topbar-identity svc-admin-topbar-identity-loading">
          Loading&hellip;
        </span>
      )
    }

    if (error) {
      return (
        <span className="svc-admin-topbar-identity svc-admin-topbar-identity-error">
          Identity unavailable
        </span>
      )
    }

    if (me) {
      const nm = displayNameOf(me as any)
      const mode = authModeOf(me as any)
      const roles = rolesOf(me as any)
      const rolesLabel = roles.length > 0 ? ` · ${roles.join(', ')}` : ''

      return (
        <span className="svc-admin-topbar-identity">
          {nm}{' '}
          <span className="svc-admin-topbar-identity-auth">
            ({mode}
            {rolesLabel})
          </span>
        </span>
      )
    }

    return (
      <span className="svc-admin-topbar-identity svc-admin-topbar-identity-anon">
        Anonymous
      </span>
    )
  }, [loading, error, me])

  const showLogout = Boolean(me && isLocalSession(me as any))

  async function onLogout() {
    if (logoutBusy) return
    setLogoutBusy(true)

    try {
      await adminClient.logout()
    } catch {
      // Best-effort.
    } finally {
      setLogoutBusy(false)
      setState({ loading: false, error: null, me: null })

      const from = `${loc.pathname}${loc.search}`
      nav('/login', { replace: true, state: { from } })
    }
  }

  return (
    <header className="svc-admin-topbar">
      <div className="svc-admin-topbar-left">
        <span>RON-CORE Admin</span>
      </div>
      <div className="svc-admin-topbar-right">
        {identityContent}

        {showLogout ? (
          <button
            type="button"
            onClick={onLogout}
            disabled={logoutBusy}
            title="End local session"
            aria-label="Logout"
            style={{ marginLeft: 10 }}
          >
            {logoutBusy ? 'Logging out…' : 'Logout'}
          </button>
        ) : null}

        <LanguageSwitcher />
        <ThemeToggle />
      </div>
    </header>
  )
}
