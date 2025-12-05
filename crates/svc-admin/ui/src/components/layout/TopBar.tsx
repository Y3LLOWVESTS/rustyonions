import React from 'react'
import { ThemeToggle } from './ThemeToggle'
import { LanguageSwitcher } from './LanguageSwitcher'

export function TopBar() {
  return (
    <header className="svc-admin-topbar">
      <div className="svc-admin-topbar-left">
        <span>RON-CORE Admin</span>
      </div>
      <div className="svc-admin-topbar-right">
        <LanguageSwitcher />
        <ThemeToggle />
      </div>
    </header>
  )
}
