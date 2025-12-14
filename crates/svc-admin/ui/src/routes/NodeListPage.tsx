// crates/svc-admin/ui/src/routes/NodeListPage.tsx
//
// RO:WHAT — Nodes overview page (multi-node “NOC” view).
// RO:WHY  — Show all registered nodes plus quick health, restart, and
//           metrics freshness summaries.
// RO:INTERACTS —
//   - adminClient.getNodes()           → NodeSummary list
//   - adminClient.getNodeStatus(id)    → AdminStatusView per node
//   - adminClient.getNodeFacetMetrics  → FacetMetricsSummary per node
//   - NodeCard                         → presentational card (selection-aware)
//   - NodePreviewPanel                 → right-hand preview pane
//   - LoadingSpinner / ErrorBanner / EmptyState
//   - i18n/useI18n for copy

import React, { useEffect, useState } from 'react'
import { adminClient } from '../api/adminClient'
import type {
  AdminStatusView,
  NodeSummary,
  PlaneStatus,
  FacetMetricsSummary,
} from '../types/admin-api'
import {
  NodeCard,
  type NodeStatusSummary,
  type MetricsHealth,
} from '../components/nodes/NodeCard'
import { NodePreviewPanel } from '../components/nodes/NodePreviewPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { EmptyState } from '../components/shared/EmptyState'
import { useI18n } from '../i18n/useI18n'

type Health = 'healthy' | 'degraded' | 'down'

type NodeStatusState = {
  loading: boolean
  error: string | null
  status: AdminStatusView | null
}

type NodeMetricsState = {
  loading: boolean
  error: string | null
  health: MetricsHealth | null
}

function deriveOverallHealth(planes: PlaneStatus[]): Health {
  if (!planes.length) return 'degraded'
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
}

function buildSummary(
  status: AdminStatusView | null,
): NodeStatusSummary | undefined {
  if (!status) return undefined

  const planeCount = status.planes.length
  const readyCount = status.planes.filter((p) => p.ready).length
  const totalRestarts = status.planes.reduce(
    (sum, p) => sum + (p.restart_count ?? 0),
    0,
  )

  return {
    overallHealth: deriveOverallHealth(status.planes),
    planeCount,
    readyCount,
    totalRestarts,
  }
}

function classifyMetricsHealth(
  facets: FacetMetricsSummary[] | null,
  error: string | null,
): MetricsHealth | null {
  if (error) {
    return 'unreachable'
  }

  if (!facets || facets.length === 0) {
    // Node may be idle or just starting; treat as stale for now.
    return 'stale'
  }

  const ages = facets
    .map((f) => f.last_sample_age_secs)
    .filter((v): v is number => v !== null && Number.isFinite(v))

  if (ages.length === 0) {
    return 'stale'
  }

  const minAge = Math.min(...ages)
  const FRESH_THRESHOLD_SECS = 30

  if (minAge <= FRESH_THRESHOLD_SECS) {
    return 'fresh'
  }

  return 'stale'
}

export function NodeListPage() {
  const { t } = useI18n()

  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const [statusById, setStatusById] = useState<Record<string, NodeStatusState>>(
    {},
  )
  const [metricsById, setMetricsById] = useState<
    Record<string, NodeMetricsState>
  >({})

  // which node is currently selected for the right-hand preview.
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)

  // --- Load node registry --------------------------------------------------

  useEffect(() => {
    let cancelled = false

    setLoading(true)
    setError(null)

    ;(async () => {
      try {
        const data = await adminClient.getNodes()
        if (cancelled) return

        setNodes(data)

        // Default selection: first node in the list.
        if (!selectedNodeId && data.length > 0) {
          setSelectedNodeId(data[0].id)
        }
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load nodes.'
        setError(msg)
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
    // deliberately *not* including selectedNodeId here; we only want to
    // apply the default once when the nodes first load.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // --- Load per-node status in the background -----------------------------

  useEffect(() => {
    if (!nodes.length) return

    let cancelled = false

    async function loadStatuses() {
      for (const node of nodes) {
        const id = node.id

        setStatusById((prev) => ({
          ...prev,
          [id]: prev[id] ?? { loading: true, error: null, status: null },
        }))

        try {
          const data = await adminClient.getNodeStatus(id)
          if (cancelled) return

          setStatusById((prev) => ({
            ...prev,
            [id]: { loading: false, error: null, status: data },
          }))
        } catch (err) {
          if (cancelled) return
          const msg =
            err instanceof Error ? err.message : 'Failed to load node status'
          setStatusById((prev) => ({
            ...prev,
            [id]: { loading: false, error: msg, status: null },
          }))
        }
      }
    }

    loadStatuses()

    return () => {
      cancelled = true
    }
  }, [nodes])

  // --- Load per-node metrics freshness in the background -------------------

  useEffect(() => {
    if (!nodes.length) return

    let cancelled = false

    async function loadMetrics() {
      for (const node of nodes) {
        const id = node.id

        setMetricsById((prev) => ({
          ...prev,
          [id]: prev[id] ?? { loading: true, error: null, health: null },
        }))

        try {
          const data = await adminClient.getNodeFacetMetrics(id)
          if (cancelled) return

          const health = classifyMetricsHealth(data, null)

          setMetricsById((prev) => ({
            ...prev,
            [id]: { loading: false, error: null, health },
          }))
        } catch (err) {
          if (cancelled) return
          const msg =
            err instanceof Error ? err.message : 'Failed to load facet metrics'

          const health: MetricsHealth = 'unreachable'

          setMetricsById((prev) => ({
            ...prev,
            [id]: { loading: false, error: msg, health },
          }))
        }
      }
    }

    loadMetrics()

    return () => {
      cancelled = true
    }
  }, [nodes])

  // --- Derived view-model for selected node --------------------------------

  const effectiveSelectedId =
    selectedNodeId || (nodes.length > 0 ? nodes[0].id : null)

  const selectedNode =
    effectiveSelectedId != null
      ? nodes.find((n) => n.id === effectiveSelectedId) ?? null
      : null

  const selectedStatusState =
    selectedNode != null ? statusById[selectedNode.id] : undefined
  const selectedMetricsState =
    selectedNode != null ? metricsById[selectedNode.id] : undefined

  const selectedSummary = buildSummary(selectedStatusState?.status ?? null)

  // ✅ This is what the preview needs in order to render the Planes table.
  const selectedPlanes = selectedStatusState?.status?.planes ?? null

  // --- Render ---------------------------------------------------------------

  return (
    <div className="svc-admin-page svc-admin-page-nodes">
      <header className="svc-admin-page-header">
        <h1>{t('nav.nodes')}</h1>
        <p>
          Overview of nodes registered in this svc-admin instance, including
          quick health, restart, and metrics freshness summaries.
        </p>
      </header>

      {loading && <LoadingSpinner />}

      {!loading && error && <ErrorBanner message={error} />}

      {!loading && !error && nodes.length === 0 && (
        <EmptyState message="No nodes configured. Check svc-admin config for node registry entries." />
      )}

      {!loading && !error && nodes.length > 0 && (
        <div className="svc-admin-node-layout">
          <div className="svc-admin-node-grid">
            {nodes.map((node) => {
              const statusState = statusById[node.id]
              const metricsState = metricsById[node.id]

              const summary = buildSummary(statusState?.status ?? null)

              const isSelected = node.id === effectiveSelectedId

              return (
                <NodeCard
                  key={node.id}
                  node={node}
                  statusSummary={summary}
                  statusLoading={statusState?.loading}
                  statusError={statusState?.error ?? null}
                  metricsHealth={metricsState?.health ?? null}
                  metricsLoading={metricsState?.loading}
                  metricsError={metricsState?.error ?? null}
                  isSelected={isSelected}
                  onSelect={() => setSelectedNodeId(node.id)}
                />
              )
            })}
          </div>

          <NodePreviewPanel
            node={selectedNode}
            statusSummary={selectedSummary}
            metricsHealth={selectedMetricsState?.health ?? null}
            metricsLoading={selectedMetricsState?.loading}
            metricsError={selectedMetricsState?.error ?? null}
            planes={selectedPlanes}
          />
        </div>
      )}
    </div>
  )
}
