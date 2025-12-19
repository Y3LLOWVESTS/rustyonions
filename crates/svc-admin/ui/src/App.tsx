/**
 * RO:WHAT — Top-level route table for the svc-admin SPA.
 * RO:WHY — Keeps navigation explicit and stable as the dashboard grows (DX).
 * RO:INTERACTS — Shell layout; routes/* pages; BrowserRouter in main.tsx.
 * RO:INVARIANTS — NotFound is last; no nested Routers; routes are client-side only.
 * RO:SECURITY — No privileged actions here; pages enforce dev gating/read-only.
 * RO:TEST — Manual smoke: /, /nodes/:id, /settings, /playground.
 */

// crates/svc-admin/ui/src/App.tsx

import React from 'react'
import { Routes, Route } from 'react-router-dom'
import { Shell } from './components/layout/Shell'
import { NodeListPage } from './routes/NodeListPage'
import { NodeDetailPage } from './routes/NodeDetailPage'
import { NodeStoragePage } from './routes/NodeStoragePage'
import { NodeDatabaseDetailPage } from './routes/NodeDatabaseDetailPage'
import { PlaygroundPage } from './routes/PlaygroundPage'
import { SettingsPage } from './routes/SettingsPage'
import { LoginPage } from './routes/LoginPage'
import { NotFoundPage } from './routes/NotFoundPage'

export default function App() {
  return (
    <Shell>
      <Routes>
        <Route path="/" element={<NodeListPage />} />
        <Route path="/nodes/:id" element={<NodeDetailPage />} />
        <Route path="/nodes/:id/storage" element={<NodeStoragePage />} />
        <Route
          path="/nodes/:id/storage/databases/:name"
          element={<NodeDatabaseDetailPage />}
        />
        <Route path="/playground" element={<PlaygroundPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="/login" element={<LoginPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </Shell>
  )
}
