// crates/svc-admin/ui/src/components/nodes/NodeCard.tsx
//
// RO:WHAT — Compact card for a node on the Nodes overview page.
// RO:WHY  — Give operators a quick glance at profile + overall health +
//           plane/restart summary + metrics freshness, without clicking in.
// RO:INTERACTS —
//   - NodeListPage (passes NodeSummary + status + metrics health)
//   - NodeStatusBadge for overall health pill.
//
// NOTE:
//   - This component is now *selection-aware*.
//   - When `onSelect` is provided, the card renders as a <button> that
//     selects the node (used on NodeListPage + right-hand preview).
//   - When `onSelect` is not provided, we fall back to a simple <Link>
//     to `/nodes/:id` (useful for any future usages).

import React from 'react'
import { Link } from 'react-router-dom'
import type { NodeSummary } from '../../types/admin-api'
import { NodeStatusBadge } from './NodeStatusBadge'

export type Health = 'healthy' | 'degraded' | 'down'

export type NodeStatusSummary = {
  overallHealth: Health
  planeCount: number
  readyCount: number
  totalRestarts: number
}

export type MetricsHealth = 'fresh' | 'stale' | 'unreachable'

export type NodeCardProps = {
  node: NodeSummary
  statusSummary?: NodeStatusSummary
  statusLoading?: boolean
  statusError?: string | null
  metricsHealth?: MetricsHealth | null
  metricsLoading?: boolean
  metricsError?: string | null

  // New for selection-aware layout on NodeListPage:
  isSelected?: boolean
  onSelect?: () => void
}

function metricsPillClass(kind: 'loading' | 'na' | MetricsHealth) {
  const base = 'svc-admin-metrics-pill'
  switch (kind) {
    case 'loading':
      return `${base} ${base}--loading`
    case 'na':
      return `${base} ${base}--na`
    case 'fresh':
      return `${base} ${base}--fresh`
    case 'stale':
      return `${base} ${base}--stale`
    case 'unreachable':
      return `${base} ${base}--unreachable`
    default:
      return `${base} ${base}--na`
  }
}

/**
 * Small metrics pill used both on Node cards *and* in the Node preview panel.
 */
export function renderMetricsLabel(
  health: MetricsHealth | null | undefined,
  loading: boolean | undefined,
  error: string | null | undefined,
): JSX.Element {
  if (loading) {
    return <span className={metricsPillClass('loading')}>Metrics: loading…</span>
  }

  if (error || health === 'unreachable') {
    return (
      <span className={metricsPillClass('unreachable')}>
        Metrics: unreachable
      </span>
    )
  }

  if (!health) {
    return <span className={metricsPillClass('na')}>Metrics: n/a</span>
  }

  if (health === 'fresh') {
    return <span className={metricsPillClass('fresh')}>Metrics: fresh</span>
  }

  // stale
  return <span className={metricsPillClass('stale')}>Metrics: stale</span>
}

/**
 * Compact card used on the Nodes overview page.
 *
 * When `onSelect` is provided, the card acts like a selectable tile (button),
 * and we highlight the currently selected one.
 *
 * When `onSelect` is omitted, we fall back to a simple <Link> to the node
 * detail page.
 */
export function NodeCard({
  node,
  statusSummary,
  statusLoading = false,
  statusError = null,
  metricsHealth,
  metricsLoading,
  metricsError,
  isSelected = false,
  onSelect,
}: NodeCardProps) {
  const hasStatus = !!statusSummary && !statusLoading && !statusError

  const classNameBase = [
    'svc-admin-node-card',
    onSelect ? 'svc-admin-node-card--clickable' : '',
    isSelected ? 'svc-admin-node-card--selected' : '',
  ]
    .filter(Boolean)
    .join(' ')

  const inner = (
    <>
      <div className="svc-admin-node-card-header">
        <div>
          <h3 className="svc-admin-node-title">{node.display_name}</h3>
          <p className="svc-admin-node-subtitle">
            <span className="svc-admin-node-label">Profile:</span>{' '}
            <span className="svc-admin-node-profile">{node.profile}</span>
          </p>
        </div>

        {hasStatus && statusSummary && (
          <NodeStatusBadge status={statusSummary.overallHealth} />
        )}
      </div>

      <div className="svc-admin-node-card-body">
        {statusLoading && (
          <p className="svc-admin-node-meta svc-admin-node-meta-muted">
            Loading status…
          </p>
        )}

        {!statusLoading && statusError && (
          <p className="svc-admin-node-meta svc-admin-node-meta-error">
            Status unavailable
          </p>
        )}

        {!statusLoading && !statusError && hasStatus && statusSummary && (
          <p className="svc-admin-node-meta">
            <span>
              {statusSummary.readyCount}/{statusSummary.planeCount} planes ready
            </span>
            <span className="svc-admin-node-meta-dot">•</span>
            <span>
              {statusSummary.totalRestarts}{' '}
              {statusSummary.totalRestarts === 1 ? 'restart' : 'restarts'}
            </span>
          </p>
        )}

        {!statusLoading && !statusError && !hasStatus && (
          <p className="svc-admin-node-meta svc-admin-node-meta-muted">
            Status not loaded yet.
          </p>
        )}

        <div className="svc-admin-node-meta-metrics">
          {renderMetricsLabel(metricsHealth, metricsLoading, metricsError)}
        </div>
      </div>
    </>
  )

  // Selection-aware variant (used by NodeListPage with right-hand preview)
  if (onSelect) {
    return (
      <button type="button" className={classNameBase} onClick={onSelect}>
        {inner}
      </button>
    )
  }

  // Fallback: plain navigation card
  return (
    <Link
      to={`/nodes/${encodeURIComponent(node.id)}`}
      className={classNameBase}
    >
      {inner}
    </Link>
  )
}
