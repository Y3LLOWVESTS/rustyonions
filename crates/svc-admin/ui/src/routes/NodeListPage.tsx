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

import React from 'react'
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

import { useNodeListData } from './node-list/useNodeListData'
import { buildSummary } from './node-list/helpers'

export function NodeListPage() {
  const { t } = useI18n()

  const {
    nodes,
    loading,
    error,

    statusById,
    metricsById,

    effectiveSelectedId,
    selectedNode,
    selectedStatusState,
    selectedMetricsState,
    setSelectedNodeId,
  } = useNodeListData()

  const selectedSummary: NodeStatusSummary | undefined = buildSummary(
    selectedStatusState?.status ?? null,
  )

  const selectedPlanes = selectedStatusState?.status?.planes ?? null

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
