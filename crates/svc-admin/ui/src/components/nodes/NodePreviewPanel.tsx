// crates/svc-admin/ui/src/components/nodes/NodePreviewPanel.tsx
//
// WHAT:
//   Right-hand “preview” panel on the Nodes overview page.
//   Shows richer details for the currently selected node plus a
//   metrics pill and an "Open →" button to jump to the detail page.
//
// WHY:
//   Mirrors the God-tier mock: cards on the left, focused preview on the
//   right so operators can scan quickly but still get depth on hover/click.
//
// INTERACTS:
//   - NodeListPage (passes selected node + status + metrics)
//   - NodeStatusBadge for overall health
//   - NodeCard.renderMetricsLabel for metrics freshness pill
//   - adminClient.getNodeStorageSummary(nodeId) (preview storage ring)
//   - adminClient.getNodeSystemSummary(nodeId)  (preview CPU/RAM/Network)
//
// TRUTH BOUNDARY:
//   Missing telemetry remains unavailable. The preview never fabricates
//   capacity, utilization, network rates, hardware totals, or link speed.
//
// NOTE:
//   - This panel supports an optional "Planes" table, matching the mock.
//   - To populate it, pass `planes` from the selected node's status view.
//
// NEW (UI sprint):
//   - Optional operator tag editor (chips + add/remove).
//   - Tag storage lives upstream (NodeListPage), this panel just renders/edit UI.

import React, { useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import type {
  AdminStatusView,
  NodeSummary,
  StorageSummaryDto,
  SystemSummaryDto,
} from '../../types/admin-api'
import { adminClient } from '../../api/adminClient'
import { NodeStatusBadge } from './NodeStatusBadge'
import type { NodeStatusSummary, MetricsHealth, Health } from './NodeCard'
import { renderMetricsLabel } from './NodeCard'
import { buildNodeCapabilityView, postureValue } from '../../lib/nodeCapabilities'

type PlaneLike = {
  name?: string
  health?: 'healthy' | 'degraded' | 'down' | string
  ready?: boolean | string
  restarts?: number
  restart_count?: number
  restartCount?: number
}

type Props = {
  node: NodeSummary | null
  status?: AdminStatusView | null

  // NEW: operator tags
  tags?: string[]
  onAddTag?: (tag: string) => void
  onRemoveTag?: (tag: string) => void

  statusSummary?: NodeStatusSummary
  metricsHealth?: MetricsHealth | null
  metricsLoading?: boolean
  metricsError?: string | null
  planes?: PlaneLike[] | null
}

type DataSource = 'live' | 'unavailable'

function clampPct(p: number): number {
  if (!Number.isFinite(p)) return 0
  return Math.max(0, Math.min(100, p))
}

function fmtBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return 'n/a'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'] as const
  let v = bytes
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  const digits = i === 0 ? 0 : i <= 2 ? 1 : 2
  return `${v.toFixed(digits)} ${units[i]}`
}

function fmtBytesPerSec(bytesPerSec: number): string {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec < 0) return 'n/a'
  return `${fmtBytes(bytesPerSec)}/s`
}

// ---- RAM unit bug guard ----------------------------------------------------
//
// If a node mistakenly reports bytes that are off by exactly *1024* (e.g. GiB
// becomes TiB), we can normalize for display so the UI doesn’t mislead operators.
// We STILL want to fix this at the source in macronode, but this prevents the UI
// from looking insane during rollout.
function normalizeMaybeOffBy1024(bytes: number): number {
  if (!Number.isFinite(bytes) || bytes <= 0) return bytes

  // If it’s already < 1 TiB, don’t touch it.
  const oneTiB = 1024 * 1024 * 1024 * 1024
  if (bytes < oneTiB) return bytes

  // If dividing by 1024 yields something < 1 TiB, this is a strong hint that an extra
  // KiB factor was applied somewhere (bytes treated as KiB).
  if (bytes % 1024 === 0) {
    const div = bytes / 1024
    if (div > 0 && div < oneTiB) return div
  }

  return bytes
}

// ---- ring pill --------------------------------------------------------------

function RingPill(props: {
  label: string
  pct: number | null
  line1: string
  line2: string
  source: DataSource
  loading: boolean
  title: string
}) {
  const available =
    typeof props.pct === 'number' &&
    Number.isFinite(props.pct)
  const pct = clampPct(props.pct ?? 0)
  const radius = 16
  const stroke = 5
  const c = 2 * Math.PI * radius
  const dash = available ? (pct / 100) * c : 0
  const gap = c - dash

  const color = !available
    ? 'rgba(148,163,184,0.38)'
    : pct >= 90
      ? 'var(--svc-admin-color-danger-text, #ef4444)'
      : pct >= 75
        ? 'var(--svc-admin-color-warning, #fb923c)'
        : 'var(--svc-admin-color-accent, #3b82f6)'

  return (
    <div
      title={props.title}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '0.55rem',
        padding: '0.38rem 0.60rem',
        borderRadius: 999,
        border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
        background: 'var(--svc-admin-color-panel, rgba(255,255,255,0.02))',
        width: '100%',
        minWidth: 190,
        maxWidth: 240,
      }}
    >
      <div style={{ position: 'relative', width: 40, height: 40, flex: '0 0 auto' }}>
        <svg width="40" height="40" viewBox="0 0 40 40" aria-hidden="true">
          <circle
            cx="20"
            cy="20"
            r={radius}
            fill="none"
            stroke="var(--svc-admin-color-border, rgba(255,255,255,0.12))"
            strokeWidth={stroke}
            opacity={0.9}
          />
          <circle
            cx="20"
            cy="20"
            r={radius}
            fill="none"
            stroke={color}
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={`${dash} ${gap}`}
            transform="rotate(-90 20 20)"
          />
        </svg>

        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontWeight: 850,
            fontSize: '0.78rem',
            lineHeight: 1,
          }}
        >
          {props.loading
            ? '…'
            : available
              ? `${pct.toFixed(0)}%`
              : '—'}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.08rem' }}>
        <div style={{ fontWeight: 900, fontSize: '0.92rem', lineHeight: 1.05 }}>
          {props.label}
        </div>
        <div style={{ fontSize: '0.80rem', opacity: 0.9, lineHeight: 1.1 }}>
          {props.line1}
        </div>
        <div style={{ fontSize: '0.78rem', opacity: 0.8, lineHeight: 1.1 }}>
          {props.line2}{' '}
          <span style={{ opacity: 0.75 }}>
            · {props.source === 'live' ? 'Live' : 'Unavailable'}
          </span>
        </div>
      </div>
    </div>
  )
}

// ---- planes helpers ---------------------------------------------------------

function planeRestarts(p: PlaneLike): number {
  const a = p.restarts
  if (typeof a === 'number' && Number.isFinite(a)) return a
  const b = p.restart_count
  if (typeof b === 'number' && Number.isFinite(b)) return b
  const c = p.restartCount
  if (typeof c === 'number' && Number.isFinite(c)) return c
  return 0
}

function planeHealth(p: PlaneLike): Health | 'unknown' {
  const h = (p.health ?? '').toString().toLowerCase()
  if (h === 'healthy') return 'healthy'
  if (h === 'degraded') return 'degraded'
  if (h === 'down') return 'down'
  return 'unknown'
}

function planeReady(p: PlaneLike): boolean | null {
  if (typeof p.ready === 'boolean') return p.ready
  if (typeof p.ready === 'string') {
    const s = p.ready.toLowerCase()
    if (s === 'ready') return true
    if (s === 'not_ready' || s === 'not ready') return false
  }
  return null
}

function normalizeTag(input: string): string {
  let s = String(input ?? '').trim().toLowerCase()
  if (!s) return ''
  s = s.replace(/\s+/g, '-')
  s = s.replace(/[^a-z0-9._-]/g, '')
  s = s.replace(/-+/g, '-')
  s = s.replace(/^[-_.]+|[-_.]+$/g, '')
  if (s.length > 48) s = s.slice(0, 48)
  return s
}

// ---- component --------------------------------------------------------------

export function NodePreviewPanel({
  node,
  status = null,
  tags = [],
  onAddTag,
  onRemoveTag,
  statusSummary,
  metricsHealth,
  metricsLoading,
  metricsError,
  planes,
}: Props) {
  // Hooks must run on every render; no early return before hooks.
  const nodeId = node?.id ?? ''

  const [storage, setStorage] = useState<StorageSummaryDto | null>(null)
  const [storageLoading, setStorageLoading] = useState(false)
  const [storageSource, setStorageSource] =
    useState<DataSource>('unavailable')

  const [system, setSystem] = useState<SystemSummaryDto | null>(null)
  const [systemLoading, setSystemLoading] = useState(false)
  const [systemSource, setSystemSource] =
    useState<DataSource>('unavailable')

  // NEW: tag input draft
  const [tagDraft, setTagDraft] = useState('')

  const capabilityView = useMemo(
    () => buildNodeCapabilityView(status, node),
    [status, node],
  )
  const canOpenStorage = capabilityView.canOpenStorage

  // Fetch storage summary without inventing a fallback value.
  useEffect(() => {
    if (!nodeId || !canOpenStorage) {
      setStorage(null)
      setStorageLoading(false)
      setStorageSource('unavailable')
      return
    }

    let cancelled = false

    setStorage(null)
    setStorageLoading(true)
    setStorageSource('unavailable')

    void (async () => {
      try {
        const reported =
          await adminClient.getNodeStorageSummary(nodeId)

        if (cancelled) return

        setStorage(reported)
        setStorageSource('live')
      } catch {
        if (cancelled) return

        setStorage(null)
        setStorageSource('unavailable')
      } finally {
        if (!cancelled) {
          setStorageLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId, canOpenStorage])

  // Fetch system summary without fabricating CPU, RAM, or network data.
  useEffect(() => {
    if (!nodeId) {
      setSystem(null)
      setSystemLoading(false)
      setSystemSource('unavailable')
      return
    }

    let cancelled = false

    setSystem(null)
    setSystemLoading(true)
    setSystemSource('unavailable')

    void (async () => {
      try {
        const reported =
          await adminClient.getNodeSystemSummary(nodeId)

        if (cancelled) return

        if (reported) {
          setSystem(reported)
          setSystemSource('live')
        } else {
          setSystem(null)
          setSystemSource('unavailable')
        }
      } catch {
        if (cancelled) return

        setSystem(null)
        setSystemSource('unavailable')
      } finally {
        if (!cancelled) {
          setSystemLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  const storageComputed = useMemo(() => {
    if (
      !storage ||
      !Number.isFinite(storage.totalBytes) ||
      storage.totalBytes <= 0 ||
      !Number.isFinite(storage.usedBytes) ||
      storage.usedBytes < 0
    ) {
      return null
    }

    return {
      pct: clampPct(
        (storage.usedBytes / storage.totalBytes) * 100,
      ),
      used: storage.usedBytes,
      total: storage.totalBytes,
      source: storageSource,
    }
  }, [storage, storageSource])

  const ramComputed = useMemo(() => {
    if (
      !system ||
      !Number.isFinite(system.ramTotalBytes) ||
      system.ramTotalBytes <= 0 ||
      !Number.isFinite(system.ramUsedBytes) ||
      system.ramUsedBytes < 0
    ) {
      return null
    }

    const total =
      normalizeMaybeOffBy1024(system.ramTotalBytes)
    const used =
      normalizeMaybeOffBy1024(system.ramUsedBytes)

    if (
      !Number.isFinite(total) ||
      total <= 0 ||
      !Number.isFinite(used) ||
      used < 0
    ) {
      return null
    }

    return {
      pct: clampPct((used / total) * 100),
      used,
      total,
      source: systemSource,
    }
  }, [system, systemSource])

  const cpuComputed = useMemo(() => {
    const reported = system?.cpuPercent

    if (
      typeof reported !== 'number' ||
      !Number.isFinite(reported)
    ) {
      return null
    }

    return {
      pct: clampPct(reported),
      source: systemSource,
    }
  }, [system, systemSource])

  const bandwidthComputed = useMemo(() => {
    const rx = system?.netRxBps
    const tx = system?.netTxBps

    const rxReported =
      typeof rx === 'number' && Number.isFinite(rx)
    const txReported =
      typeof tx === 'number' && Number.isFinite(tx)

    if (!rxReported && !txReported) {
      return null
    }

    return {
      // Link capacity is not reported, so no utilization percentage
      // can be truthfully derived from the observed rates.
      pct: null,
      rxBytesPerSec: rxReported ? Math.max(0, rx) : null,
      txBytesPerSec: txReported ? Math.max(0, tx) : null,
      source: systemSource,
    }
  }, [system, systemSource])

  // Safe empty state after hooks.
  if (!node) {
    return (
      <aside className="svc-admin-node-preview svc-admin-node-preview-empty">
        <p>Select a node on the left to see details.</p>
      </aside>
    )
  }

  const planeCount = statusSummary?.planeCount ?? null
  const readyCount = statusSummary?.readyCount ?? null
  const totalRestarts = statusSummary?.totalRestarts ?? null

  const version = (node as any)?.version ?? (node as any)?.build_version ?? null
  const baseUrl = (node as any)?.base_url ?? (node as any)?.baseUrl ?? null
  const metricsUrl =
    typeof baseUrl === 'string' && baseUrl.length > 0
      ? `${baseUrl.replace(/\/+$/, '')}/metrics`
      : null

  const chipStyle: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    height: 24,
    padding: '0 10px',
    borderRadius: 999,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(255,255,255,0.05)',
    fontSize: 12,
    fontWeight: 850,
    letterSpacing: '0.02em',
    opacity: 0.92,
  }

  const chipBtn: React.CSSProperties = {
    border: 'none',
    background: 'transparent',
    color: 'rgba(226,232,240,0.92)',
    cursor: 'pointer',
    padding: 0,
    fontWeight: 950,
    lineHeight: 1,
  }

  const inputStyle: React.CSSProperties = {
    height: 32,
    borderRadius: 12,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: 'rgba(0,0,0,0.18)',
    color: 'rgba(226,232,240,0.92)',
    padding: '0 10px',
    outline: 'none',
    minWidth: 180,
  }

  const addBtnStyle: React.CSSProperties = {
    height: 32,
    borderRadius: 12,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: 'rgba(255,255,255,0.06)',
    color: 'rgba(226,232,240,0.92)',
    padding: '0 10px',
    cursor: 'pointer',
    fontWeight: 900,
  }

  return (
    <aside className="svc-admin-node-preview">
      <header className="svc-admin-node-preview-header svc-admin-node-preview-header--stack">
        <div className="svc-admin-node-preview-header-left">
          <h2 className="svc-admin-node-preview-title">{node.display_name}</h2>
          <p className="svc-admin-node-preview-subtitle">
            <span className="svc-admin-node-label">Role:</span>{' '}
            <span className="svc-admin-node-profile">{capabilityView.roleLabel}</span>
            {' · '}
            <span className="svc-admin-node-label">Profile:</span>{' '}
            <span className="svc-admin-node-profile">
              {capabilityView.nodeProfile ?? node.profile ?? 'not reported'}
            </span>
          </p>
        </div>

        <div className="svc-admin-node-preview-pills">
          {statusSummary && <NodeStatusBadge status={statusSummary.overallHealth} />}
          <div className="svc-admin-node-preview-metrics-pill">
            {renderMetricsLabel(metricsHealth ?? null, metricsLoading, metricsError)}
          </div>
        </div>
      </header>

      <section
        style={{
          marginTop: '0.85rem',
          padding: '0.75rem',
          borderRadius: 16,
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          background: capabilityView.isUserNode
            ? 'rgba(59,130,246,0.08)'
            : 'rgba(255,255,255,0.025)',
        }}
      >
        <div style={{ fontWeight: 950, fontSize: '0.9rem', marginBottom: 5 }}>
          {capabilityView.roleLabel}
        </div>
        <div style={{ fontSize: 12, opacity: 0.82, lineHeight: 1.35 }}>
          {capabilityView.roleDescription}
        </div>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginTop: 10 }}>
          <span style={chipStyle}>Privacy: {postureValue(capabilityView.privacyMode, 'On', 'Off')}</span>
          <span style={chipStyle}>Amnesia: {postureValue(capabilityView.amnesiaMode, 'On', 'Off')}</span>
          <span style={chipStyle}>Public inbound: {postureValue(capabilityView.publicInboundEnabled, 'On', 'Off')}</span>
          <span style={chipStyle}>IP publication: {capabilityView.userIpPublication ?? 'not reported'}</span>
        </div>
      </section>

      {/* NEW: Tags editor */}
      <section style={{ marginTop: '0.85rem' }}>
        <div style={{ fontWeight: 900, fontSize: '0.92rem', marginBottom: 6, opacity: 0.95 }}>
          Tags
        </div>

        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 10 }}>
          {tags.length ? (
            tags.map((t) => (
              <span key={t} style={chipStyle} title={t}>
                {t}
                {onRemoveTag ? (
                  <button
                    type="button"
                    style={chipBtn}
                    aria-label={`Remove tag ${t}`}
                    title="Remove tag"
                    onClick={() => onRemoveTag(t)}
                  >
                    ×
                  </button>
                ) : null}
              </span>
            ))
          ) : (
            <div style={{ opacity: 0.7, fontSize: 12 }}>No tags yet.</div>
          )}
        </div>

        {onAddTag ? (
          <form
            onSubmit={(e) => {
              e.preventDefault()
              const norm = normalizeTag(tagDraft)
              if (!norm) return
              onAddTag(norm)
              setTagDraft('')
            }}
            style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}
          >
            <input
              style={inputStyle}
              value={tagDraft}
              onChange={(e) => setTagDraft(e.target.value)}
              placeholder="Add tag…"
              aria-label="Add tag"
            />
            <button type="submit" style={addBtnStyle}>
              Add
            </button>
          </form>
        ) : null}
      </section>

      {/* ✅ TOP SECTION: 2×2 grid (CPU/RAM + Storage/Bandwidth) */}
      <div
        style={{
          width: '100%',
          display: 'grid',
          gridTemplateColumns: 'repeat(2, minmax(190px, 240px))',
          gap: '0.85rem',
          justifyContent: 'center',
          alignItems: 'center',
          margin: '1.0rem 0 0.35rem 0',
        }}
      >
        <RingPill
          label="CPU"
          pct={cpuComputed?.pct ?? null}
          line1={
            cpuComputed
              ? 'Utilization'
              : 'Not reported'
          }
          line2={
            cpuComputed
              ? `Updated: ${system?.updatedAt ?? 'reported'}`
              : 'Node did not publish CPU usage'
          }
          source={cpuComputed?.source ?? 'unavailable'}
          loading={systemLoading}
          title="CPU utilization reported by the node system-summary endpoint."
        />

        <RingPill
          label="RAM"
          pct={ramComputed?.pct ?? null}
          line1={
            ramComputed
              ? `${fmtBytes(ramComputed.used)} /`
              : 'Not reported'
          }
          line2={
            ramComputed
              ? fmtBytes(ramComputed.total)
              : 'Node did not publish RAM usage'
          }
          source={ramComputed?.source ?? 'unavailable'}
          loading={systemLoading}
          title="RAM usage reported by the node system-summary endpoint."
        />

        {canOpenStorage ? (
          <RingPill
            label="Storage"
            pct={storageComputed?.pct ?? null}
            line1={
              storageComputed
                ? `${fmtBytes(storageComputed.used)} /`
                : 'Not reported'
            }
            line2={
              storageComputed
                ? fmtBytes(storageComputed.total)
                : 'Node did not publish storage usage'
            }
            source={
              storageComputed?.source ?? 'unavailable'
            }
            loading={storageLoading}
            title="Storage usage reported by the node storage-summary endpoint."
          />
        ) : (
          <div
            style={{
              minWidth: 190,
              maxWidth: 240,
              padding: '0.72rem',
              borderRadius: 18,
              border:
                '1px dashed var(--svc-admin-color-border, rgba(255,255,255,0.14))',
              background: 'rgba(255,255,255,0.02)',
              fontSize: 12,
              opacity: 0.82,
              lineHeight: 1.35,
            }}
            title="Private User Nodes are not storage-service operators."
          >
            <div
              style={{
                fontWeight: 950,
                marginBottom: 3,
              }}
            >
              Storage service
            </div>
            <div>
              {capabilityView.isUserNode
                ? 'Hidden for private User Node.'
                : 'Not reported'}
            </div>
          </div>
        )}

        <RingPill
          label="Bandwidth"
          pct={bandwidthComputed?.pct ?? null}
          line1={
            bandwidthComputed?.rxBytesPerSec != null
              ? `RX ${fmtBytesPerSec(
                  bandwidthComputed.rxBytesPerSec,
                )}`
              : 'Not reported'
          }
          line2={
            bandwidthComputed?.txBytesPerSec != null
              ? `TX ${fmtBytesPerSec(
                  bandwidthComputed.txBytesPerSec,
                )}`
              : 'Node did not publish network rates'
          }
          source={
            bandwidthComputed?.source ?? 'unavailable'
          }
          loading={systemLoading}
          title={
            bandwidthComputed
              ? 'Observed network rates. Link capacity was not reported, so no utilization percentage is shown.'
              : 'Network rates were not reported by the node.'
          }
        />
      </div>

      <div className="svc-admin-node-preview-body">
        <p className="svc-admin-node-preview-line">
          <span className="svc-admin-node-label">Node ID:</span>{' '}
          <span className="svc-admin-node-id">{node.id}</span>
        </p>

        {version && (
          <p className="svc-admin-node-preview-line">
            <span className="svc-admin-node-label">Version:</span>{' '}
            <span className="svc-admin-node-id">{String(version)}</span>
          </p>
        )}

        {planeCount !== null && readyCount !== null && totalRestarts !== null && (
          <p className="svc-admin-node-preview-line svc-admin-node-preview-planes">
            <span>
              <strong>
                {readyCount}/{planeCount}
              </strong>{' '}
              planes ready
            </span>
            <span className="svc-admin-node-meta-dot">•</span>
            <span>
              <strong>{totalRestarts}</strong>{' '}
              {totalRestarts === 1 ? 'restart' : 'restarts'}
            </span>
          </p>
        )}
      </div>

      <section className="svc-admin-node-preview-planes-block">
        <div className="svc-admin-node-preview-planes-header">
          <h3 className="svc-admin-node-preview-planes-title">Planes</h3>

          {metricsUrl ? (
            <a
              className="svc-admin-node-preview-planes-link"
              href={metricsUrl}
              target="_blank"
              rel="noreferrer"
            >
              View /metrics
            </a>
          ) : (
            <span className="svc-admin-node-preview-planes-link svc-admin-node-preview-planes-link--disabled">
              View /metrics
            </span>
          )}
        </div>

        {planes && planes.length > 0 ? (
          <div className="svc-admin-node-preview-table-wrap">
            <table className="svc-admin-node-preview-table">
              <thead>
                <tr>
                  <th>Plane</th>
                  <th>Health</th>
                  <th>Ready</th>
                  <th className="svc-admin-node-preview-th-right">Restarts</th>
                </tr>
              </thead>
              <tbody>
                {planes.map((p, idx) => {
                  const name = p.name ?? `plane-${idx}`
                  const h = planeHealth(p)
                  const r = planeReady(p)
                  const restarts = planeRestarts(p)

                  return (
                    <tr key={String(name)}>
                      <td className="svc-admin-node-preview-td-plane">{String(name)}</td>
                      <td>
                        <span
                          className={[
                            'svc-admin-node-preview-health',
                            `svc-admin-node-preview-health--${h}`,
                          ].join(' ')}
                        >
                          {h === 'unknown'
                            ? 'Unknown'
                            : h.charAt(0).toUpperCase() + h.slice(1)}
                        </span>
                      </td>
                      <td>
                        <span
                          className={[
                            'svc-admin-node-preview-ready',
                            r === true
                              ? 'svc-admin-node-preview-ready--ready'
                              : r === false
                                ? 'svc-admin-node-preview-ready--notready'
                                : 'svc-admin-node-preview-ready--unknown',
                          ].join(' ')}
                        >
                          {r === true ? 'Ready' : r === false ? 'Not ready' : 'Unknown'}
                        </span>
                      </td>
                      <td className="svc-admin-node-preview-td-right">
                        {restarts}
                        <span className="svc-admin-node-preview-chev">›</span>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="svc-admin-node-preview-planes-empty">Planes not loaded yet.</div>
        )}
      </section>

      <footer className="svc-admin-node-preview-footer">
        <Link
          to={`/nodes/${encodeURIComponent(node.id)}`}
          className="svc-admin-node-preview-open"
        >
          <span>Open</span>
          <span className="svc-admin-node-preview-open-icon">→</span>
        </Link>
      </footer>
    </aside>
  )
}
