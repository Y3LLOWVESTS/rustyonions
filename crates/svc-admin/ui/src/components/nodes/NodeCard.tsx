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
//
// NEW (UI sprint):
//   - Optional operator tags (chips) rendered on the card.
//   - This remains purely presentational; storage lives upstream (NodeListPage).

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

  // Optional: best-effort uptime seconds (from node status).
  uptime_seconds?: number | null
}

export type MetricsHealth = 'fresh' | 'stale' | 'unreachable'

export type NodeCardProps = {
  node: NodeSummary

  // Optional operator tags.
  tags?: string[]

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

function fmtUptimeShort(secs: number | null | undefined): string {
  if (typeof secs !== 'number' || !Number.isFinite(secs) || secs < 0) return '—'
  const s = Math.floor(secs)

  const days = Math.floor(s / 86400)
  const hours = Math.floor((s % 86400) / 3600)
  const mins = Math.floor((s % 3600) / 60)

  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${mins}m`
  if (mins > 0) return `${mins}m`
  return `${Math.max(0, s)}s`
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
    return <span className={metricsPillClass('unreachable')}>Metrics: unreachable</span>
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

function TagChips({ tags }: { tags: string[] }) {
  if (!tags.length) return null

  const chip: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    height: 20,
    padding: '0 8px',
    borderRadius: 999,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(255,255,255,0.05)',
    fontSize: 11,
    fontWeight: 850,
    letterSpacing: '0.02em',
    opacity: 0.92,
    maxWidth: 160,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  }

  const more: React.CSSProperties = {
    ...chip,
    background: 'rgba(0,0,0,0.18)',
    opacity: 0.78,
  }

  const shown = tags.slice(0, 6)
  const hidden = tags.length - shown.length

  return (
    <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap', marginTop: 8 }}>
      {shown.map((t) => (
        <span key={t} style={chip} title={t}>
          {t}
        </span>
      ))}
      {hidden > 0 && <span style={more}>+{hidden}</span>}
    </div>
  )
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
  tags = [],
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

  const profileLabel =
    typeof node.profile === 'string' && node.profile.trim().length > 0 ? node.profile : '—'

  const displayName =
    typeof (node as any).display_name === 'string'
      ? (node as any).display_name
      : typeof (node as any).displayName === 'string'
        ? (node as any).displayName
        : node.id

  const uptimeLabel = fmtUptimeShort(statusSummary?.uptime_seconds ?? null)

  const inner = (
    <>
      <div className="svc-admin-node-card-header">
        <div>
          <h3 className="svc-admin-node-title">{displayName}</h3>
          <p className="svc-admin-node-subtitle">
            <span className="svc-admin-node-label">Profile:</span>{' '}
            <span className="svc-admin-node-profile">{profileLabel}</span>
          </p>

          {/* NEW: tags */}
          <TagChips tags={tags} />
        </div>

        {hasStatus && statusSummary && <NodeStatusBadge status={statusSummary.overallHealth} />}
      </div>

      <div className="svc-admin-node-card-body">
        {statusLoading && (
          <p className="svc-admin-node-meta svc-admin-node-meta-muted">Loading status…</p>
        )}

        {!statusLoading && statusError && (
          <p className="svc-admin-node-meta svc-admin-node-meta-error">Status unavailable</p>
        )}

        {!statusLoading && !statusError && hasStatus && statusSummary && (
          <>
            <p className="svc-admin-node-meta">
              <span>
                {statusSummary.readyCount}/{statusSummary.planeCount} planes ready
              </span>
              <span className="svc-admin-node-meta-dot">•</span>
              <span>
                {statusSummary.totalRestarts}{' '}
                {statusSummary.totalRestarts === 1 ? 'restart' : 'restarts'}
              </span>
              <span className="svc-admin-node-meta-dot">•</span>
              <span title="Best-effort uptime from node status">Uptime: {uptimeLabel}</span>
            </p>
          </>
        )}

        {!statusLoading && !statusError && !hasStatus && (
          <p className="svc-admin-node-meta svc-admin-node-meta-muted">Status not loaded yet.</p>
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
      <button type="button" className={classNameBase} onClick={onSelect} aria-pressed={isSelected}>
        {inner}
      </button>
    )
  }

  // Fallback: plain navigation card
  return (
    <Link to={`/nodes/${encodeURIComponent(node.id)}`} className={classNameBase}>
      {inner}
    </Link>
  )
}
