// crates/svc-admin/ui/src/routes/NodeListPage.tsx

import React, { useEffect, useState } from 'react'
import { adminClient } from '../api/adminClient'
import type { NodeSummary } from '../types/admin-api'
import { NodeCard } from '../components/nodes/NodeCard'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { EmptyState } from '../components/shared/EmptyState'
import { useI18n } from '../i18n/useI18n'

export function NodeListPage() {
  const { t } = useI18n()
  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    setLoading(true)
    setError(null)

    adminClient
      .getNodes()
      .then((data) => {
        if (cancelled) return
        setNodes(data)
      })
      .catch((err: unknown) => {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load nodes'
        setError(msg)
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [])

  return (
    <div className="svc-admin-page svc-admin-page-nodes">
      <header className="svc-admin-page-header">
        <h1>{t('nav.nodes')}</h1>
        <p>Overview of nodes registered in this svc-admin instance.</p>
      </header>

      {loading && <LoadingSpinner />}

      {!loading && error && <ErrorBanner message={error} />}

      {!loading && !error && nodes.length === 0 && (
        <EmptyState message="No nodes configured. Check svc-admin config for node registry entries." />
      )}

      {!loading && !error && nodes.length > 0 && (
        <div className="svc-admin-node-grid">
          {nodes.map((node) => (
            <NodeCard key={node.id} node={node} />
          ))}
        </div>
      )}
    </div>
  )
}
