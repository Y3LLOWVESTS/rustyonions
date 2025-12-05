// crates/svc-admin/ui/src/routes/NodeDetailPage.tsx

import React, { useEffect, useMemo, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { adminClient } from '../api/adminClient'
import type { AdminStatusView, FacetMetricsSummary, PlaneStatus } from '../types/admin-api'
import { PlaneStatusTable } from '../components/nodes/PlaneStatusTable'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { FacetMetricsPanel } from '../components/metrics/FacetMetricsPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { useI18n } from '../i18n/useI18n'

type Health = 'healthy' | 'degraded' | 'down'

function deriveOverallHealth(planes: PlaneStatus[]): Health {
  if (!planes.length) return 'degraded'
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
}

export function NodeDetailPage() {
  const { t } = useI18n()
  const params = useParams<{ id: string }>()
  const nodeId = params.id ?? ''

  const [status, setStatus] = useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] = useState<string | null>(null)

  const [facets, setFacets] = useState<FacetMetricsSummary[] | null>(null)
  const [facetsLoading, setFacetsLoading] = useState(true)
  const [facetsError, setFacetsError] = useState<string | null>(null)

  useEffect(() => {
    if (!nodeId) return

    let cancelled = false

    setStatus(null)
    setStatusError(null)
    setStatusLoading(true)

    adminClient
      .getNodeStatus(nodeId)
      .then((data) => {
        if (cancelled) return
        setStatus(data)
      })
      .catch((err: unknown) => {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load node status'
        setStatusError(msg)
      })
      .finally(() => {
        if (!cancelled) {
          setStatusLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) return

    let cancelled = false

    setFacets(null)
    setFacetsError(null)
    setFacetsLoading(true)

    adminClient
      .getNodeFacetMetrics(nodeId)
      .then((data) => {
        if (cancelled) return
        setFacets(data)
      })
      .catch((err: unknown) => {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load facet metrics'
        setFacetsError(msg)
      })
      .finally(() => {
        if (!cancelled) {
          setFacetsLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [nodeId])

  const overallHealth: Health | null = useMemo(() => {
    if (!status) return null
    return deriveOverallHealth(status.planes)
  }, [status])

  if (!nodeId) {
    return (
      <div className="svc-admin-page svc-admin-page-node">
        <ErrorBanner message="Missing node id in route." />
      </div>
    )
  }

  const pageTitle = status ? status.display_name : nodeId

  return (
    <div className="svc-admin-page svc-admin-page-node">
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <h1>{pageTitle}</h1>
          {status && (
            <p className="svc-admin-node-meta">
              <span className="svc-admin-node-id">ID: {status.node_id}</span>{' '}
              <span className="svc-admin-node-profile">Profile: {status.profile}</span>{' '}
              <span className="svc-admin-node-version">Version: {status.version}</span>
            </p>
          )}
        </div>
        <div className="svc-admin-page-header-actions">
          <Link to="/" className="svc-admin-link-muted">
            ← {t('nav.nodes')}
          </Link>
          {overallHealth && (
            <NodeStatusBadge status={overallHealth} />
          )}
        </div>
      </header>

      {statusLoading && (
        <div className="svc-admin-section">
          <LoadingSpinner />
        </div>
      )}

      {!statusLoading && statusError && (
        <div className="svc-admin-section">
          <ErrorBanner message={statusError} />
        </div>
      )}

      {!statusLoading && !statusError && status && (
        <section className="svc-admin-section svc-admin-section-node-overview">
          <h2>Planes</h2>
          <PlaneStatusTable planes={status.planes} />
        </section>
      )}

      <section className="svc-admin-section svc-admin-section-node-metrics">
        <FacetMetricsPanel
          facets={facets}
          loading={facetsLoading}
          error={facetsError}
        />
      </section>
    </div>
  )
}
