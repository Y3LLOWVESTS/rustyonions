import React from 'react'
import { Sidebar } from './Sidebar'
import { TopBar } from './TopBar'

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
      </div>
    </div>
  )
}
