import React from 'react'
import { Link } from 'react-router-dom'

export function Sidebar() {
  return (
    <aside className="svc-admin-sidebar">
      <h2>RON-CORE</h2>
      <nav>
        <ul>
          <li><Link to="/">Nodes</Link></li>
          <li><Link to="/settings">Settings</Link></li>
        </ul>
      </nav>
    </aside>
  )
}
