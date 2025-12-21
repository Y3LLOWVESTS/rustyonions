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
//
// RO:UX — Add a top “freshness strip” so operators can tell at a glance whether
//        the dashboard is live, stale, or unreachable before deep-diving.
//
// NOTE: This is intentionally lightweight: it only depends on metricsById health
//       which is already computed by the polling hooks.

import React, { useMemo } from 'react'
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

function MetricsPill({
  label,
  count,
  tone,
}: {
  label: string
  count: number
  tone: 'ok' | 'warn' | 'bad' | 'muted'
}) {
  const style: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    padding: '6px 10px',
    borderRadius: 999,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background:
      tone === 'ok'
        ? 'rgba(16,185,129,0.14)'
        : tone === 'warn'
          ? 'rgba(251,146,60,0.14)'
          : tone === 'bad'
            ? 'rgba(244,63,94,0.14)'
            : 'rgba(255,255,255,0.06)',
    color:
      tone === 'ok'
        ? 'rgba(167,243,208,0.95)'
        : tone === 'warn'
          ? 'rgba(254,215,170,0.95)'
          : tone === 'bad'
            ? 'rgba(253,164,175,0.95)'
            : 'rgba(226,232,240,0.92)',
    fontSize: 13,
    lineHeight: 1.2,
    whiteSpace: 'nowrap',
  }

  const bubbleStyle: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    minWidth: 22,
    height: 18,
    padding: '0 6px',
    borderRadius: 999,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(0,0,0,0.18)',
    fontVariantNumeric: 'tabular-nums',
  }

  return (
    <span style={style}>
      <span style={bubbleStyle}>{count}</span>
      {label}
    </span>
  )
}

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

  const metricsCounts = useMemo(() => {
    let fresh = 0
    let stale = 0
    let unreachable = 0
    let unknown = 0

    for (const n of nodes) {
      const h = (metricsById[n.id]?.health ?? null) as MetricsHealth | null
      if (h === 'fresh') fresh += 1
      else if (h === 'stale') stale += 1
      else if (h === 'unreachable') unreachable += 1
      else unknown += 1
    }

    return { fresh, stale, unreachable, unknown }
  }, [metricsById, nodes])

  return (
    <div className="svc-admin-page svc-admin-page-nodes">
      <header className="svc-admin-page-header">
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 16, flexWrap: 'wrap' }}>
          <div>
            <h1>{t('nav.nodes')}</h1>
            <p>
              Overview of nodes registered in this svc-admin instance, including
              quick health, restart, and metrics freshness summaries.
            </p>
          </div>

          {!loading && !error && nodes.length > 0 && (
            <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', alignItems: 'center' }}>
              <MetricsPill label="Fresh" count={metricsCounts.fresh} tone="ok" />
              <MetricsPill label="Stale" count={metricsCounts.stale} tone="warn" />
              <MetricsPill label="Unreachable" count={metricsCounts.unreachable} tone="bad" />
              {metricsCounts.unknown > 0 && (
                <MetricsPill label="Unknown" count={metricsCounts.unknown} tone="muted" />
              )}
            </div>
          )}
        </div>
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
