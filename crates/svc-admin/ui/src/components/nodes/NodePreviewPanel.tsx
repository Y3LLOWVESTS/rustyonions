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
//     falls back to deterministic mock when endpoint missing (404/405/501)
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
  NodeSummary,
  StorageSummaryDto,
  SystemSummaryDto,
} from '../../types/admin-api'
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

function fmtBps(bitsPerSec: number): string {
  if (!Number.isFinite(bitsPerSec) || bitsPerSec < 0) return 'n/a'
  const units = ['bps', 'Kbps', 'Mbps', 'Gbps', 'Tbps'] as const
  let v = bitsPerSec
  let i = 0
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000
    i++
  }
  const digits = i === 0 ? 0 : i <= 2 ? 1 : 2
  return `${v.toFixed(digits)} ${units[i]}`
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
  const pct = clampPct(props.pct)
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
  const [storageSource, setStorageSource] = useState<DataSource>('mock')

  const [system, setSystem] = useState<SystemSummaryDto | null>(null)
  const [systemLoading, setSystemLoading] = useState(false)
  const [systemSource, setSystemSource] = useState<DataSource>('mock')

  // NEW: tag input draft
  const [tagDraft, setTagDraft] = useState('')

  // Fetch storage summary (already supported)
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
        void err
      } finally {
        if (!cancelled) setStorageLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  // Fetch system summary (CPU/RAM/NET) — optional rollout
  useEffect(() => {
    if (!nodeId) {
      setSystem(null)
      setSystemLoading(false)
      setSystemSource('mock')
      return
    }

    let cancelled = false
    setSystemLoading(true)

    ;(async () => {
      try {
        const live = await adminClient.getNodeSystemSummary(nodeId)
        if (cancelled) return
        setSystem(live)
        setSystemSource('live')
      } catch (err) {
        if (cancelled) return
        setSystem(null)
        setSystemSource('mock')
        if (!isMissingEndpoint(err)) void err
      } finally {
        if (!cancelled) setSystemLoading(false)
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

    // Prefer live
    if (system && system.ramTotalBytes > 0) {
      const totalRaw = system.ramTotalBytes
      const usedRaw = Math.max(0, system.ramUsedBytes)

      // Normalize likely “×1024” reporting bug
      const total = normalizeMaybeOffBy1024(totalRaw)
      const used = normalizeMaybeOffBy1024(usedRaw)

      const pct = total > 0 ? Math.round((used / total) * 100) : 0
      return { pct, used, total, source: systemSource as DataSource }
    }

    // Fallback deterministic mock
    const m = mockMemorySummary(nodeId)
    const total = m.totalBytes > 0 ? m.totalBytes : 0
    const used = m.usedBytes >= 0 ? m.usedBytes : 0
    const pct = total > 0 ? Math.round((used / total) * 100) : 0
    return { pct, used, total, source: 'mock' as DataSource }
  }, [nodeId, system, systemSource])

  const cpuComputed = useMemo(() => {
    if (!nodeId) return null

    // Prefer live
    const live = system?.cpuPercent
    if (typeof live === 'number' && Number.isFinite(live)) {
      return { pct: clampPct(live), source: systemSource as DataSource }
    }

    // Fallback mock
    return { pct: clampPct(mockCpuPct(nodeId)), source: 'mock' as DataSource }
  }, [nodeId, system, systemSource])

  const bandwidthComputed = useMemo(() => {
    if (!nodeId) return null

    const rx = system?.netRxBps
    const tx = system?.netTxBps
    const rxOk = typeof rx === 'number' && Number.isFinite(rx)
    const txOk = typeof tx === 'number' && Number.isFinite(tx)

    // Live path: netRxBps/netTxBps are bytes/sec.
    if (rxOk || txOk) {
      const rxBpsBytes = rxOk ? Math.max(0, rx as number) : 0
      const txBpsBytes = txOk ? Math.max(0, tx as number) : 0

      const usedBits = (rxBpsBytes + txBpsBytes) * 8

      // NOTE: This is *interface activity*, not “internet speed”.
      // Until we expose link speed, use a soft ring (assumed 1 Gbps) just for a vibe.
      const assumedLinkBits = 1e9
      const pct = assumedLinkBits > 0 ? Math.round((usedBits / assumedLinkBits) * 100) : 0

      return {
        available: true,
        pct: clampPct(pct),
        rxBytesPerSec: rxBpsBytes,
        txBytesPerSec: txBpsBytes,
        source: systemSource as DataSource,
      }
    }

    // If system summary is live but it doesn't expose net counters,
    // DO NOT show mock bandwidth (it’s misleading). Show n/a instead.
    if (systemSource === 'live') {
      return {
        available: false,
        pct: 0,
        rxBytesPerSec: null as number | null,
        txBytesPerSec: null as number | null,
        source: 'live' as DataSource,
      }
    }

    // If we have no system endpoint at all, keep the old deterministic mock fallback.
    const bw = mockBandwidthSummary(nodeId)
    const total = bw.totalBps > 0 ? bw.totalBps : 0
    const used = bw.usedBps >= 0 ? bw.usedBps : 0
    const pct = total > 0 ? Math.round((used / total) * 100) : 0
    return {
      available: true,
      pct: clampPct(pct),
      usedBitsPerSec: used,
      totalBitsPerSec: total,
      source: 'mock' as DataSource,
    }
  }, [nodeId, system, systemSource])

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
        {/* Top Left: CPU */}
        {cpuComputed && (
          <RingPill
            label="CPU"
            pct={cpuComputed.pct}
            line1="Utilization"
            line2={systemSource === 'live' ? `Updated: ${system?.updatedAt ?? 'now'}` : 'Preview'}
            source={cpuComputed.source}
            loading={systemLoading && cpuComputed.source === 'live'}
            title="CPU utilization (live when node exposes /api/v1/system/summary; else deterministic mock)."
          />
        )}

        {/* Top Right: RAM */}
        {ramComputed && (
          <RingPill
            label="RAM"
            pct={ramComputed.pct}
            line1={`${fmtBytes(ramComputed.used)} /`}
            line2={`${fmtBytes(ramComputed.total)}`}
            source={ramComputed.source}
            loading={systemLoading && ramComputed.source === 'live'}
            title="Total node RAM used (live when node exposes /api/v1/system/summary; else deterministic mock)."
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
            title="Total node storage used (curated preview; live when node exposes /api/v1/storage/summary)."
          />
        )}

        {/* Bottom Right: Bandwidth / Net I/O */}
        {bandwidthComputed && (
          <RingPill
            label="Bandwidth"
            pct={bandwidthComputed.pct}
            line1={
              (bandwidthComputed as any).available === false
                ? 'n/a'
                : (bandwidthComputed as any).rxBytesPerSec != null
                  ? `RX ${fmtBytesPerSec((bandwidthComputed as any).rxBytesPerSec ?? 0)}`
                  : `${fmtBps((bandwidthComputed as any).usedBitsPerSec ?? 0)} /`
            }
            line2={
              (bandwidthComputed as any).available === false
                ? 'Expose netRxBps/netTxBps'
                : (bandwidthComputed as any).txBytesPerSec != null
                  ? `TX ${fmtBytesPerSec((bandwidthComputed as any).txBytesPerSec ?? 0)}`
                  : `${fmtBps((bandwidthComputed as any).totalBitsPerSec ?? 0)}`
            }
            source={(bandwidthComputed as any).source}
            loading={systemLoading && (bandwidthComputed as any).source === 'live'}
            title={
              (bandwidthComputed as any).available === false
                ? 'Node system summary is live but does not expose netRxBps/netTxBps yet.'
                : (bandwidthComputed as any).rxBytesPerSec != null
                  ? 'Network interface activity (live RX/TX bytes/sec). Ring % uses an assumed 1 Gbps link until link speed is exposed.'
                  : 'Bandwidth utilization (mock until nodes expose network rate DTOs).'
            }
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
