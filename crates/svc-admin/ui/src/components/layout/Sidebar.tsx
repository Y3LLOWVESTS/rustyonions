// crates/svc-admin/ui/src/components/layout/Sidebar.tsx
//
// RO:WHAT — Sidebar navigation.
// RO:WHY  — Fast operator navigation; Playground is dev-only but should still be discoverable.
// RO:INVARIANTS —
//   - Always show the Playground link alongside Nodes/Settings.
//   - If ui-config fetch fails, assume Playground is disabled (safe posture).
//   - No sideways scroll / layout shift from active styles.
// RO:SECURITY — Dev flag gates behavior; backend endpoints remain hidden when disabled.
// RO:TEST — Manual: link present; when enabled it loads editor; when disabled it shows instructions.

import React, { useEffect, useState } from 'react'
import { NavLink } from 'react-router-dom'
import { adminClient } from '../../api/adminClient'

function linkClassName({ isActive }: { isActive: boolean }) {
  return isActive
    ? 'svc-admin-sidebar-link svc-admin-sidebar-link-active'
    : 'svc-admin-sidebar-link'
}

function devBadgeStyle(enabled: boolean): React.CSSProperties {
  return {
    display: 'inline-block',
    marginLeft: '0.5rem',
    padding: '0.05rem 0.45rem',
    borderRadius: 999,
    fontSize: '0.72rem',
    lineHeight: 1.2,
    opacity: enabled ? 0.9 : 0.8,
    border: '1px solid currentColor',
    transform: 'translateY(-1px)',
    whiteSpace: 'nowrap',
  }
}

export function Sidebar() {
  const [playgroundEnabled, setPlaygroundEnabled] = useState(false)

  useEffect(() => {
    let cancelled = false

    adminClient
      .getUiConfig()
      .then((cfg) => {
        if (cancelled) return
        setPlaygroundEnabled(Boolean(cfg?.dev?.enableAppPlayground))
      })
      .catch(() => {
        if (cancelled) return
        // Safe posture: if we can't fetch ui-config, assume disabled.
        setPlaygroundEnabled(false)
      })

    return () => {
      cancelled = true
    }
  }, [])

  return (
    <aside className="svc-admin-sidebar">
      <h2>🦀 RON-CORE 🧅</h2>

      <nav>
        <ul>
          <li>
            <NavLink to="/" className={linkClassName}>
              Nodes
            </NavLink>
          </li>

          <li>
            <NavLink
              to="/playground"
              className={linkClassName}
              style={{
                opacity: playgroundEnabled ? 1 : 0.65,
              }}
            >
              Playground
              <span style={devBadgeStyle(playgroundEnabled)}>
                {playgroundEnabled ? 'DEV' : 'DEV OFF'}
              </span>
            </NavLink>
          </li>

          <li>
            <NavLink to="/settings" className={linkClassName}>
              Settings
            </NavLink>
          </li>
        </ul>
      </nav>
    </aside>
  )
}
