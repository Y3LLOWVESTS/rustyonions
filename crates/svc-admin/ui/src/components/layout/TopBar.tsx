// crates/svc-admin/ui/src/components/layout/TopBar.tsx
//
// WHAT: Top navigation bar for the svc-admin SPA.
// WHY:  Central place for brand, theme + language toggles, and the
//       current operator identity (from `/api/me`).

import React, { useEffect, useState } from 'react'
import { ThemeToggle } from './ThemeToggle'
import { LanguageSwitcher } from './LanguageSwitcher'
import { adminClient } from '../../api/adminClient'
import type { MeResponse } from '../../types/admin-api'

type MeState = {
  loading: boolean
  error: string | null
  me: MeResponse | null
}

export function TopBar() {
  const [state, setState] = useState<MeState>({
    loading: true,
    error: null,
    me: null
  })

  useEffect(() => {
    let cancelled = false

    adminClient
      .getMe()
      .then((me) => {
        if (cancelled) return
        setState({
          loading: false,
          error: null,
          me
        })
      })
      .catch((_err) => {
        if (cancelled) return
        setState({
          loading: false,
          error: 'identity-unavailable',
          me: null
        })
      })

    return () => {
      cancelled = true
    }
  }, [])

  const { loading, error, me } = state

  let identityContent: React.ReactNode = null

  if (loading) {
    identityContent = (
      <span className="svc-admin-topbar-identity svc-admin-topbar-identity-loading">
        Loading&hellip;
      </span>
    )
  } else if (error) {
    identityContent = (
      <span className="svc-admin-topbar-identity svc-admin-topbar-identity-error">
        Identity unavailable
      </span>
    )
  } else if (me) {
    const rolesLabel =
      me.roles && me.roles.length > 0 ? ` · ${me.roles.join(', ')}` : ''

    identityContent = (
      <span className="svc-admin-topbar-identity">
        {me.displayName}{' '}
        <span className="svc-admin-topbar-identity-auth">
          ({me.authMode}
          {rolesLabel})
        </span>
      </span>
    )
  } else {
    identityContent = (
      <span className="svc-admin-topbar-identity svc-admin-topbar-identity-anon">
        Anonymous
      </span>
    )
  }

  return (
    <header className="svc-admin-topbar">
      <div className="svc-admin-topbar-left">
        <span>RON-CORE Admin</span>
      </div>
      <div className="svc-admin-topbar-right">
        {identityContent}
        <LanguageSwitcher />
        <ThemeToggle />
      </div>
    </header>
  )
}
