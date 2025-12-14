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
//   - OPTIONAL: adminClient.getNodeStorageSummary(nodeId) (preview storage ring)
//              falls back to deterministic mock when endpoint missing (404/405/501)
//
// NOTE:
//   - This panel supports an optional "Planes" table, matching the mock.
//   - To populate it, pass `planes` from the selected node's status view.

import React, { useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import type { NodeSummary, StorageSummaryDto } from '../../types/admin-api'
import { adminClient } from '../../api/adminClient'
import { NodeStatusBadge } from './NodeStatusBadge'
import type { NodeStatusSummary, MetricsHealth, Health } from './NodeCard'
import { renderMetricsLabel } from './NodeCard'

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
  statusSummary?: NodeStatusSummary
  metricsHealth?: MetricsHealth | null
  metricsLoading?: boolean
  metricsError?: string | null
  planes?: PlaneLike[] | null
}

type FetchErr = Error & { status?: number }
type DataSource = 'live' | 'mock'

function isMissingEndpoint(err: unknown): boolean {
  const e = err as FetchErr
  const s = e && typeof e.status === 'number' ? e.status : undefined
  if (s === 404 || s === 405 || s === 501) return true

  const msg = e?.message ?? ''
  return (
    msg.includes(' 404 ') ||
    msg.includes(' 405 ') ||
    msg.includes(' 501 ') ||
    msg.toLowerCase().includes('not found') ||
    msg.toLowerCase().includes('not implemented')
  )
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

function fmtBps(bps: number): string {
  if (!Number.isFinite(bps) || bps < 0) return 'n/a'
  const units = ['bps', 'Kbps', 'Mbps', 'Gbps', 'Tbps'] as const
  let v = bps
  let i = 0
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000
    i++
  }
  const digits = i === 0 ? 0 : i <= 2 ? 1 : 2
  return `${v.toFixed(digits)} ${units[i]}`
}

// ---- deterministic mock fallback (storage + ram + cpu + bandwidth) ----------

function mockStorageSummary(nodeId: string): StorageSummaryDto {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  const total = 512 * 1024 * 1024 * 1024 // 512 GiB
  const used = (96 + (seed % 220)) * 1024 * 1024 * 1024 // 96..316 GiB
  const clampedUsed = Math.min(used, total - 8 * 1024 * 1024 * 1024)
  const free = total - clampedUsed

  return {
    fsType: 'ext4',
    mount: '/',
    totalBytes: total,
    usedBytes: clampedUsed,
    freeBytes: free,
    ioReadBps: null,
    ioWriteBps: null,
  }
}

type MemSummary = { totalBytes: number; usedBytes: number }

function mockMemorySummary(nodeId: string): MemSummary {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  const totalsGiB = [16, 24, 32, 48, 64, 96, 128] as const
  const totalGiB = totalsGiB[seed % totalsGiB.length]
  const total = totalGiB * 1024 * 1024 * 1024

  // used: 35%..88% deterministic
  const pct = 35 + (seed % 54)
  const used = Math.round((pct / 100) * total)

  return { totalBytes: total, usedBytes: used }
}

function mockCpuPct(nodeId: string): number {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  // 18%..92%
  return 18 + (seed % 75)
}

type BandwidthSummary = { totalBps: number; usedBps: number }

function mockBandwidthSummary(nodeId: string): BandwidthSummary {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)

  // deterministic "link size": 250 Mbps, 500 Mbps, 1 Gbps, 2.5 Gbps
  const totals = [250e6, 500e6, 1e9, 2.5e9] as const
  const total = totals[seed % totals.length]

  // used: 12%..90%
  const pct = 12 + (seed % 79)
  const used = Math.round((pct / 100) * total)

  return { totalBps: total, usedBps: used }
}

// ---- ring pill --------------------------------------------------------------

function RingPill(props: {
  label: string
  pct: number // 0..100
  line1: string
  line2: string
  source: DataSource
  loading: boolean
  title: string
}) {
  const pct = Math.max(0, Math.min(100, props.pct))
  const radius = 16
  const stroke = 5
  const c = 2 * Math.PI * radius
  const dash = (pct / 100) * c
  const gap = c - dash

  const color =
    pct >= 90
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

        // keep pill sizing consistent (grid controls placement)
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
          {props.loading ? '…' : `${pct.toFixed(0)}%`}
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
            · {props.source === 'live' ? 'Live' : 'Mock'}
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

// ---- component --------------------------------------------------------------

export function NodePreviewPanel({
  node,
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
  const [storageSource, setStorageSource] = useState<DataSource>('mock')

  useEffect(() => {
    if (!nodeId) {
      setStorage(null)
      setStorageLoading(false)
      setStorageSource('mock')
      return
    }

    let cancelled = false
    setStorageLoading(true)

    ;(async () => {
      try {
        const live = await adminClient.getNodeStorageSummary(nodeId)
        if (cancelled) return
        setStorage(live)
        setStorageSource('live')
      } catch (err) {
        if (cancelled) return
        setStorage(mockStorageSummary(nodeId))
        setStorageSource('mock')
        // preview remains stable even on real errors
        if (!isMissingEndpoint(err)) {
          // optional: could surface a tooltip later
        }
      } finally {
        if (!cancelled) setStorageLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  const storageComputed = useMemo(() => {
    if (!nodeId) return null
    const s = storage ?? mockStorageSummary(nodeId)
    const total = s.totalBytes > 0 ? s.totalBytes : 0
    const used = s.usedBytes >= 0 ? s.usedBytes : 0
    const pct = total > 0 ? Math.round((used / total) * 100) : 0
    return { pct, used, total }
  }, [storage, nodeId])

  const ramComputed = useMemo(() => {
    if (!nodeId) return null
    const m = mockMemorySummary(nodeId)
    const total = m.totalBytes > 0 ? m.totalBytes : 0
    const used = m.usedBytes >= 0 ? m.usedBytes : 0
    const pct = total > 0 ? Math.round((used / total) * 100) : 0
    return { pct, used, total }
  }, [nodeId])

  const cpuComputed = useMemo(() => {
    if (!nodeId) return null
    const pct = mockCpuPct(nodeId)
    return { pct }
  }, [nodeId])

  const bandwidthComputed = useMemo(() => {
    if (!nodeId) return null
    const bw = mockBandwidthSummary(nodeId)
    const total = bw.totalBps > 0 ? bw.totalBps : 0
    const used = bw.usedBps >= 0 ? bw.usedBps : 0
    const pct = total > 0 ? Math.round((used / total) * 100) : 0
    return { pct, used, total }
  }, [nodeId])

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

  return (
    <aside className="svc-admin-node-preview">
      <header className="svc-admin-node-preview-header svc-admin-node-preview-header--stack">
        <div className="svc-admin-node-preview-header-left">
          <h2 className="svc-admin-node-preview-title">{node.display_name}</h2>
          <p className="svc-admin-node-preview-subtitle">
            <span className="svc-admin-node-label">Profile:</span>{' '}
            <span className="svc-admin-node-profile">{node.profile}</span>
          </p>
        </div>

        <div className="svc-admin-node-preview-pills">
          {statusSummary && <NodeStatusBadge status={statusSummary.overallHealth} />}
          <div className="svc-admin-node-preview-metrics-pill">
            {renderMetricsLabel(metricsHealth ?? null, metricsLoading, metricsError)}
          </div>
        </div>
      </header>

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
        {/* Top Left: CPU */}
        {cpuComputed && (
          <RingPill
            label="CPU"
            pct={cpuComputed.pct}
            line1="Utilization"
            line2="Preview"
            source="mock"
            loading={false}
            title="CPU utilization (mock preview until nodes expose CPU DTOs)."
          />
        )}

        {/* Top Right: RAM */}
        {ramComputed && (
          <RingPill
            label="RAM"
            pct={ramComputed.pct}
            line1={`${fmtBytes(ramComputed.used)} /`}
            line2={`${fmtBytes(ramComputed.total)}`}
            source="mock"
            loading={false}
            title="Total node RAM used (mock preview until nodes expose memory DTOs)."
          />
        )}

        {/* Bottom Left: Storage */}
        {storageComputed && (
          <RingPill
            label="Storage"
            pct={storageComputed.pct}
            line1={`${fmtBytes(storageComputed.used)} /`}
            line2={`${fmtBytes(storageComputed.total)}`}
            source={storageSource}
            loading={storageLoading}
            title="Total node storage used (curated preview)."
          />
        )}

        {/* Bottom Right: Bandwidth */}
        {bandwidthComputed && (
          <RingPill
            label="Bandwidth"
            pct={bandwidthComputed.pct}
            line1={`${fmtBps(bandwidthComputed.used)} /`}
            line2={`${fmtBps(bandwidthComputed.total)}`}
            source="mock"
            loading={false}
            title="Bandwidth utilization (mock preview until nodes expose bandwidth DTOs)."
          />
        )}
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
                      <td className="svc-admin-node-preview-td-plane">
                        {String(name)}
                      </td>
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
          <div className="svc-admin-node-preview-planes-empty">
            Planes not loaded yet.
          </div>
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
