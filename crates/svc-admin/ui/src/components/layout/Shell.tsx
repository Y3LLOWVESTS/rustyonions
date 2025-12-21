/**
 * RO:WHAT — App shell layout (Sidebar + TopBar + main content).
 * RO:WHY — Keeps operator navigation consistent across all routes; central place
 *         to mount dev-only tooling overlays without polluting pages.
 * RO:INTERACTS — Sidebar, TopBar, (dev) RequestInspectorDrawer.
 * RO:INVARIANTS — No nested routers; no conditional hooks; dev tools must not
 *               affect production behavior when Vite build is PROD.
 * RO:SECURITY — Dev tooling is UI-only; no secrets persisted; bounded logs.
 * RO:TEST — Manual: route navigation + open/close inspector; no layout shift.
 */

import React from 'react'
import { Sidebar } from './Sidebar'
import { TopBar } from './TopBar'
import { RequestInspectorDrawer } from '../dev/RequestInspectorDrawer'

type Props = {
  children: React.ReactNode
}

export function Shell({ children }: Props) {
  return (
    <div className="svc-admin-shell">
      <Sidebar />
      <div className="svc-admin-main">
        <TopBar />
        <main>{children}</main>

        {/* Dev-only request inspector (Vite dev server only). */}
        {import.meta.env.DEV ? <RequestInspectorDrawer /> : null}
      </div>
    </div>
  )
}
