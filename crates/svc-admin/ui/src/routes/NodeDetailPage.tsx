// crates/svc-admin/ui/src/routes/NodeDetailPage.tsx
//
// WHAT:
//   Deep-dive Node detail screen. Two-column layout:
//   - Left: planes table, facet metrics, actions, debug.
//   - Right: NodeDetailSidebar with "Data & storage" + "Playground" stubs.
// WHY:
//   Matches the God-tier mock: main operational view on the left, curated
//   data-plane + tinkering surface on the right.

import React, { useEffect, useMemo, useRef, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { adminClient } from '../api/adminClient'
import type {
  AdminStatusView,
  FacetMetricsSummary,
  NodeActionResponse,
} from '../types/admin-api'
import { PlaneStatusTable } from '../components/nodes/PlaneStatusTable'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { FacetMetricsPanel } from '../components/metrics/FacetMetricsPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { useI18n } from '../i18n/useI18n'
import type { MetricsHealth } from '../components/nodes/NodeCard'
import { NodeDetailSidebar } from '../components/nodes/NodeDetailSidebar'

type Health = 'healthy' | 'degraded' | 'down'

function deriveOverallHealth(planes: any[]): Health {
  if (!planes.length) return 'degraded'
  if (planes.some((p) => String(p.health ?? '').toLowerCase() === 'down'))
    return 'down'
  if (planes.some((p) => String(p.health ?? '').toLowerCase() === 'degraded'))
    return 'degraded'
  return 'healthy'
}

function classifyMetricsHealth(
  facets: FacetMetricsSummary[] | null,
  error: string | null,
): MetricsHealth | null {
  if (error) return 'unreachable'
  if (!facets || facets.length === 0) return 'stale'

  const ages = facets
    .map((f) => f.last_sample_age_secs)
    .filter((v): v is number => v !== null && Number.isFinite(v))

  if (ages.length === 0) return 'stale'

  const minAge = Math.min(...ages)
  const FRESH_THRESHOLD_SECS = 30
  return minAge <= FRESH_THRESHOLD_SECS ? 'fresh' : 'stale'
}

function renderMetricsBadge(health: MetricsHealth | null) {
  if (!health) return null

  const base =
    'inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold'

  if (health === 'fresh') {
    return (
      <span className={`${base} bg-emerald-500/10 text-emerald-300`}>
        Metrics: fresh
      </span>
    )
  }

  if (health === 'stale') {
    return (
      <span className={`${base} bg-amber-500/10 text-amber-300`}>
        Metrics: stale
      </span>
    )
  }

  return (
    <span className={`${base} bg-rose-500/10 text-rose-300`}>
      Metrics: unreachable
    </span>
  )
}

function serviceForPlane(plane: string): string {
  const trimmed = plane.trim()
  if (!trimmed) return trimmed
  if (trimmed.startsWith('svc-')) return trimmed

  switch (trimmed) {
    case 'gateway':
      return 'svc-gateway'
    case 'storage':
      return 'svc-storage'
    case 'index':
      return 'svc-index'
    case 'mailbox':
      return 'svc-mailbox'
    case 'overlay':
      return 'svc-overlay'
    case 'dht':
      return 'svc-dht'
    default:
      return trimmed
  }
}

// -------------------- CPU + RAM + Storage + Bandwidth (mock; deterministic per node) --------------------

function seedFromString(s: string): number {
  let acc = 0
  for (let i = 0; i < s.length; i++) acc = (acc * 31 + s.charCodeAt(i)) >>> 0
  return acc >>> 0
}

function clampPct(n: number): number {
  if (!Number.isFinite(n)) return 0
  return Math.max(0, Math.min(100, n))
}

function mockNodeUtilization(nodeId: string): {
  cpuPct: number
  ramPct: number
  storagePct: number
  bandwidthPct: number
} {
  const seed = seedFromString(nodeId || 'node')
  const cpu = 12 + (seed % 79) // 12..90
  const ram = 25 + ((seed >>> 8) % 70) // 25..94
  const storage = 18 + ((seed >>> 16) % 78) // 18..95
  const bw = 8 + ((seed >>> 24) % 88) // 8..95
  return {
    cpuPct: clampPct(cpu),
    ramPct: clampPct(ram),
    storagePct: clampPct(storage),
    bandwidthPct: clampPct(bw),
  }
}

function MiniMetricCard(props: {
  title: 'RAM' | 'CPU' | 'Storage' | 'Bandwidth'
  children: React.ReactNode
}) {
  return (
    <div
      style={{
        borderRadius: 16,
        background:
          'linear-gradient(180deg, rgba(255,255,255,0.035), rgba(255,255,255,0.015))',
        border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
        boxShadow: '0 10px 30px rgba(0,0,0,0.22)',
        padding: '8px 10px',
        minHeight: 80,
        overflow: 'hidden',
      }}
    >
      <div style={{ fontWeight: 900, letterSpacing: '0.02em' }}>
        {props.title}
      </div>
      <div style={{ marginTop: 10 }}>{props.children}</div>
    </div>
  )
}

function ThermometerGauge(props: { pct: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false
  const fill = pct / 100

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <div
        style={{
          width: compact ? 40 : 46,
          height: compact ? 96 : 118,
          borderRadius: 999,
          position: 'relative',
          overflow: 'hidden',
          background: 'rgba(255,255,255,0.04)',
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          boxShadow: '0 0 0 1px rgba(0,0,0,0.18) inset',
          flex: '0 0 auto',
        }}
        aria-hidden="true"
      >
        <div
          style={{
            position: 'absolute',
            inset: 0,
            opacity: 0.25,
            background:
              'linear-gradient(to bottom, rgba(255,255,255,0.12) 1px, transparent 1px)',
            backgroundSize: '100% 14px',
            pointerEvents: 'none',
          }}
        />
        <div
          style={{
            position: 'absolute',
            left: 6,
            right: 6,
            bottom: 6,
            height: `calc(${fill * 100}% - 12px)`,
            minHeight: 10,
            borderRadius: 999,
            background:
              'linear-gradient(180deg, rgba(99,102,241,0.85) 0%, rgba(250,204,21,0.85) 55%, rgba(251,146,60,0.9) 80%, rgba(244,63,94,0.9) 100%)',
            filter: 'saturate(1.15)',
          }}
        />
        <div
          style={{
            position: 'absolute',
            inset: 0,
            background:
              'linear-gradient(90deg, rgba(255,255,255,0.18), rgba(255,255,255,0.02) 55%, rgba(255,255,255,0.08))',
            opacity: 0.12,
            pointerEvents: 'none',
          }}
        />
      </div>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          Memory in use (mock)
        </div>
      </div>
    </div>
  )
}

function SpeedometerGauge(props: { pct: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false
  const angle = -110 + (pct / 100) * 220

  const cx = 78
  const cy = 82
  const r = 58

  const start = polarToCartesian(cx, cy, r, 200)
  const end = polarToCartesian(cx, cy, r, -20)
  const arcPath = describeArc(cx, cy, r, 200, -20)

  const svgW = compact ? 132 : 156
  const svgH = compact ? 98 : 118

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <svg width={svgW} height={svgH} viewBox="0 0 156 118" aria-hidden="true">
        <path
          d={arcPath}
          fill="none"
          stroke="rgba(255,255,255,0.10)"
          strokeWidth="12"
          strokeLinecap="round"
        />
        <path
          d={describeArc(cx, cy, r, 200, 200 - (pct / 100) * 220)}
          fill="none"
          stroke="rgba(99,102,241,0.85)"
          strokeWidth="12"
          strokeLinecap="round"
        />
        {Array.from({ length: 9 }).map((_, i) => {
          const a = 200 - (i / 8) * 220
          const p1 = polarToCartesian(cx, cy, r + 2, a)
          const p2 = polarToCartesian(cx, cy, r - 10, a)
          return (
            <line
              key={i}
              x1={p1.x}
              y1={p1.y}
              x2={p2.x}
              y2={p2.y}
              stroke="rgba(255,255,255,0.12)"
              strokeWidth={i % 2 === 0 ? 2 : 1}
            />
          )
        })}

        <g transform={`rotate(${angle} ${cx} ${cy})`}>
          <line
            x1={cx}
            y1={cy}
            x2={cx + 54}
            y2={cy}
            stroke="rgba(191,219,254,0.95)"
            strokeWidth="3"
            strokeLinecap="round"
          />
          <circle cx={cx} cy={cy} r="7" fill="rgba(255,255,255,0.10)" />
          <circle cx={cx} cy={cy} r="4" fill="rgba(99,102,241,0.95)" />
        </g>

        <circle cx={start.x} cy={start.y} r="3" fill="rgba(251,146,60,0.7)" />
        <circle cx={end.x} cy={end.y} r="3" fill="rgba(16,185,129,0.55)" />
      </svg>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          CPU utilization (mock)
        </div>
      </div>
    </div>
  )
}

function StorageWaffleGauge(props: { pct: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false

  const cols = 10
  const rows = 5
  const total = cols * rows
  const filled = Math.round((pct / 100) * total)

  const size = compact ? 12 : 14
  const gap = compact ? 4 : 5

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${cols}, ${size}px)`,
          gridAutoRows: `${size}px`,
          gap: `${gap}px`,
          padding: 8,
          borderRadius: 14,
          background: 'rgba(255,255,255,0.03)',
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          boxShadow: '0 0 0 1px rgba(0,0,0,0.18) inset',
          flex: '0 0 auto',
        }}
        aria-hidden="true"
      >
        {Array.from({ length: total }).map((_, i) => {
          const on = i < filled
          return (
            <div
              key={i}
              style={{
                width: size,
                height: size,
                borderRadius: 4,
                background: on
                  ? 'rgba(99,102,241,0.70)'
                  : 'rgba(255,255,255,0.07)',
                boxShadow: on ? '0 0 10px rgba(99,102,241,0.18)' : 'none',
              }}
            />
          )
        })}
      </div>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          Storage used (mock)
        </div>
      </div>
    </div>
  )
}

function BandwidthBarsGauge(props: { pct: number; seed: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false

  const bars = 14
  const active = Math.max(1, Math.round((pct / 100) * bars))

  const barW = compact ? 7 : 8
  const barGap = compact ? 5 : 6
  const maxH = compact ? 44 : 52
  const baseH = compact ? 14 : 16

  const colorForIndex = (idx: number) => {
    const t = idx / Math.max(1, bars - 1) // 0..1
    if (t < 0.6) return 'rgba(34,197,94,0.85)'
    if (t < 0.85) return 'rgba(250,204,21,0.85)'
    return 'rgba(244,63,94,0.85)'
  }

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <div
        style={{
          padding: 10,
          borderRadius: 14,
          background: 'rgba(255,255,255,0.03)',
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          boxShadow: '0 0 0 1px rgba(0,0,0,0.18) inset',
          flex: '0 0 auto',
        }}
        aria-hidden="true"
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            gap: barGap,
            height: maxH,
          }}
        >
          {Array.from({ length: bars }).map((_, i) => {
            const jitter = ((props.seed >>> (i % 16)) & 0x7) / 7 // 0..1
            const h = Math.round(baseH + jitter * (maxH - baseH))
            const on = i < active
            const col = colorForIndex(i)

            return (
              <div
                key={i}
                style={{
                  width: barW,
                  height: h,
                  borderRadius: 999,
                  background: on ? col : 'rgba(255,255,255,0.08)',
                  boxShadow: on
                    ? `0 0 10px ${col.replace('0.85', '0.18')}`
                    : 'none',
                  opacity: on ? 1 : 0.85,
                }}
              />
            )
          })}
        </div>
      </div>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          Bandwidth in use (mock)
        </div>
      </div>
    </div>
  )
}

function polarToCartesian(cx: number, cy: number, r: number, angleDeg: number) {
  const a = ((angleDeg - 90) * Math.PI) / 180
  return { x: cx + r * Math.cos(a), y: cy + r * Math.sin(a) }
}

function describeArc(
  cx: number,
  cy: number,
  r: number,
  startAngle: number,
  endAngle: number,
) {
  const s = polarToCartesian(cx, cy, r, startAngle)
  const e = polarToCartesian(cx, cy, r, endAngle)
  const largeArcFlag = Math.abs(endAngle - startAngle) <= 180 ? '0' : '1'
  return `M ${s.x} ${s.y} A ${r} ${r} 0 ${largeArcFlag} 1 ${e.x} ${e.y}`
}

// -------------------- planes helpers (for summary pills) --------------------

function planeRestarts(p: any): number {
  const a = p?.restarts
  if (typeof a === 'number' && Number.isFinite(a)) return a
  const b = p?.restart_count
  if (typeof b === 'number' && Number.isFinite(b)) return b
  const c = p?.restartCount
  if (typeof c === 'number' && Number.isFinite(c)) return c
  return 0
}

function planeReadyBool(p: any): boolean | null {
  const r = p?.ready
  if (typeof r === 'boolean') return r
  if (typeof r === 'string') {
    const s = r.toLowerCase().trim()
    if (s === 'ready' || s === 'true' || s === 'ok') return true
    if (s === 'not_ready' || s === 'not ready' || s === 'false') return false
  }
  return null
}

function planeHealthNorm(p: any): Health | 'unknown' {
  const h = String(p?.health ?? '').toLowerCase().trim()
  if (h === 'healthy') return 'healthy'
  if (h === 'degraded') return 'degraded'
  if (h === 'down') return 'down'
  return 'unknown'
}

function Pill(props: { tone: 'ok' | 'warn' | 'bad' | 'muted'; children: React.ReactNode }) {
  const map: Record<typeof props.tone, { bg: string; fg: string; bd: string }> = {
    ok: {
      bg: 'rgba(16,185,129,0.10)',
      fg: 'rgba(110,231,183,0.95)',
      bd: 'rgba(16,185,129,0.22)',
    },
    warn: {
      bg: 'rgba(251,146,60,0.10)',
      fg: 'rgba(253,186,116,0.95)',
      bd: 'rgba(251,146,60,0.24)',
    },
    bad: {
      bg: 'rgba(244,63,94,0.10)',
      fg: 'rgba(253,164,175,0.95)',
      bd: 'rgba(244,63,94,0.24)',
    },
    muted: {
      bg: 'rgba(255,255,255,0.04)',
      fg: 'rgba(255,255,255,0.85)',
      bd: 'rgba(255,255,255,0.10)',
    },
  }
  const c = map[props.tone]
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        padding: '4px 8px',
        borderRadius: 999,
        fontSize: 12,
        fontWeight: 850,
        background: c.bg,
        color: c.fg,
        border: `1px solid ${c.bd}`,
        whiteSpace: 'nowrap',
      }}
    >
      {props.children}
    </span>
  )
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

  const [facetHistory, setFacetHistory] = useState<Record<string, number[]>>({})

  const [readOnlyUi, setReadOnlyUi] = useState(true)
  const [roles, setRoles] = useState<string[]>([])
  const [identityError, setIdentityError] = useState<string | null>(null)
  const [identityLoading, setIdentityLoading] = useState(true)

  const [actionInFlight, setActionInFlight] =
    useState<'reload' | 'shutdown' | null>(null)
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  const devDebugEnabled = import.meta.env.DEV
  const [debugPlane, setDebugPlane] = useState<string>('')
  const [debugInFlight, setDebugInFlight] = useState(false)
  const [debugMessage, setDebugMessage] = useState<string | null>(null)
  const [debugError, setDebugError] = useState<string | null>(null)

  const mountedRef = useRef(true)
  const statusInFlightRef = useRef(false)
  const facetsInFlightRef = useRef(false)

  const STATUS_POLL_MS = 5_000
  const FACETS_POLL_MS = 5_000
  const MAX_SPARK_POINTS = 40

  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
    }
  }, [])

  const planes: any[] = useMemo(
    () => ((status as any)?.planes as any[]) ?? [],
    [status],
  )

  const overallHealth: Health | null = useMemo(() => {
    if (!status) return null
    return deriveOverallHealth(planes)
  }, [status, planes])

  const metricsHealth: MetricsHealth | null = useMemo(() => {
    if (facetsLoading) return null
    return classifyMetricsHealth(facets, facetsError)
  }, [facets, facetsError, facetsLoading])

  const minSampleAgeSecs: number | null = useMemo(() => {
    if (!facets || facets.length === 0) return null

    const ages = facets
      .map((f) => f.last_sample_age_secs)
      .filter((v): v is number => v !== null && Number.isFinite(v))

    if (ages.length === 0) return null
    return Math.min(...ages)
  }, [facets])

  const pageTitle =
    (status as any)?.display_name ??
    (status as any)?.displayName ??
    status?.id ??
    nodeId

  const utilSeed = useMemo(() => seedFromString(nodeId || 'node'), [nodeId])
  const { cpuPct, ramPct, storagePct, bandwidthPct } = useMemo(
    () => mockNodeUtilization(nodeId),
    [nodeId],
  )

  // planes summary pills
  const planeSummary = useMemo(() => {
    const total = planes.length
    const ready = planes.filter((p) => planeReadyBool(p) === true).length
    const degraded = planes.filter((p) => planeHealthNorm(p) === 'degraded').length
    const down = planes.filter((p) => planeHealthNorm(p) === 'down').length
    const restarts = planes.reduce((acc, p) => acc + planeRestarts(p), 0)
    return { total, ready, degraded, down, restarts }
  }, [planes])

  useEffect(() => {
    if (planes.length > 0 && !debugPlane) {
      setDebugPlane(planes[0].name)
    }
  }, [planes, debugPlane])

  async function refreshStatus(opts?: { initial?: boolean }) {
    if (!nodeId) return
    if (statusInFlightRef.current) return
    statusInFlightRef.current = true

    const initial = opts?.initial ?? false

    try {
      if (initial) {
        setStatus(null)
        setStatusError(null)
        setStatusLoading(true)
      } else {
        setStatusError(null)
      }

      const data = await adminClient.getNodeStatus(nodeId)
      if (!mountedRef.current) return
      setStatus(data)
    } catch (err) {
      if (!mountedRef.current) return
      setStatusError(
        err instanceof Error ? err.message : 'Failed to load node status.',
      )
    } finally {
      statusInFlightRef.current = false
      if (!mountedRef.current) return
      if (initial) setStatusLoading(false)
    }
  }

  async function refreshFacets(opts?: { initial?: boolean }) {
    if (!nodeId) return
    if (facetsInFlightRef.current) return
    facetsInFlightRef.current = true

    const initial = opts?.initial ?? false

    try {
      if (initial) {
        setFacets(null)
        setFacetsError(null)
        setFacetsLoading(true)
      } else {
        setFacetsError(null)
      }

      const data = await adminClient.getNodeFacetMetrics(nodeId)
      if (!mountedRef.current) return

      setFacets(data)

      setFacetHistory((prev) => {
        const next: Record<string, number[]> = { ...prev }

        for (const facet of data) {
          const key = facet.facet
          const prevSeries = prev[key] ?? []
          const updated = [...prevSeries, facet.rps]
          next[key] =
            updated.length > MAX_SPARK_POINTS
              ? updated.slice(updated.length - MAX_SPARK_POINTS)
              : updated
        }

        return next
      })
    } catch (err) {
      if (!mountedRef.current) return
      setFacetsError(
        err instanceof Error
          ? err.message
          : 'Failed to load facet metrics for this node.',
      )
    } finally {
      facetsInFlightRef.current = false
      if (!mountedRef.current) return
      if (initial) setFacetsLoading(false)
    }
  }

  useEffect(() => {
    if (!nodeId) return

    void refreshStatus({ initial: true })
    const t = window.setInterval(() => void refreshStatus(), STATUS_POLL_MS)
    return () => window.clearInterval(t)
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) return

    void refreshFacets({ initial: true })
    const t = window.setInterval(() => void refreshFacets(), FACETS_POLL_MS)
    return () => window.clearInterval(t)
  }, [nodeId])

  useEffect(() => {
    let cancelled = false

    setIdentityError(null)
    setIdentityLoading(true)

    ;(async () => {
      try {
        const [uiConfig, me] = await Promise.all([
          adminClient.getUiConfig(),
          adminClient.getMe(),
        ])
        if (cancelled) return
        setReadOnlyUi(uiConfig.readOnly)
        setRoles(me.roles)
      } catch (err) {
        if (cancelled) return
        setIdentityError(
          err instanceof Error
            ? err.message
            : 'Failed to load identity / UI configuration.',
        )
      } finally {
        if (!cancelled) setIdentityLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [])

  const canMutate =
    !readOnlyUi && roles.some((role) => role === 'admin' || role === 'ops')

  async function runAction(kind: 'reload' | 'shutdown') {
    if (!status) return

    setActionInFlight(kind)
    setActionError(null)
    setActionMessage(null)

    try {
      let response: NodeActionResponse
      if (kind === 'reload') {
        response = await adminClient.reloadNode(status.id)
      } else {
        response = await adminClient.shutdownNode(status.id)
      }

      setActionMessage(response.message ?? 'Action completed successfully.')
      await Promise.allSettled([refreshStatus(), refreshFacets()])
    } catch (err) {
      setActionError(
        err instanceof Error
          ? err.message
          : 'Action failed. See logs for more detail.',
      )
    } finally {
      setActionInFlight(null)
    }
  }

  async function runDebugCrash() {
    if (!status || !debugPlane) return

    setDebugInFlight(true)
    setDebugError(null)
    setDebugMessage(null)

    try {
      const service = serviceForPlane(debugPlane)
      const response = await adminClient.debugCrashNode(status.id, service)
      setDebugMessage(
        response.message ?? `Synthetic crash event sent for service "${service}".`,
      )
      await Promise.allSettled([refreshStatus(), refreshFacets()])
    } catch (err) {
      setDebugError(
        err instanceof Error
          ? err.message
          : 'Failed to trigger synthetic crash for this node.',
      )
    } finally {
      setDebugInFlight(false)
    }
  }

  if (statusLoading && facetsLoading && identityLoading) {
    return (
      <div className="svc-admin-page svc-admin-page-node-detail">
        <div className="svc-admin-section">
          <LoadingSpinner />
        </div>
      </div>
    )
  }

  if (statusError || identityError) {
    return (
      <div className="svc-admin-page svc-admin-page-node-detail">
        <div className="svc-admin-section">
          <ErrorBanner
            message={
              statusError ??
              identityError ??
              'Something went wrong while loading the node.'
            }
          />
        </div>
      </div>
    )
  }

  if (!status) {
    return (
      <div className="svc-admin-page svc-admin-page-node-detail">
        <div className="svc-admin-section">
          <ErrorBanner message="Node status is unavailable." />
        </div>
      </div>
    )
  }

  return (
    <div className="svc-admin-page svc-admin-page-node-detail">
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          {/* ✅ Back link in top-left above the title */}
          <div style={{ marginBottom: 8 }}>
            <Link to="/" className="svc-admin-link-muted">
              ← Back
            </Link>
          </div>

          <h1>{pageTitle}</h1>
          <p className="svc-admin-page-subtitle">
            Detailed status, planes, and metrics for this node.
          </p>
          <p className="svc-admin-node-meta">
            <span className="svc-admin-node-id">
              <strong>ID:</strong> {status.id}
            </span>{' '}
            {(status as any).profile && (
              <span className="svc-admin-node-profile">
                <strong>Profile:</strong> {(status as any).profile}
              </span>
            )}{' '}
            {(status as any).version && (
              <span className="svc-admin-node-version">
                <strong>Version:</strong> {(status as any).version}
              </span>
            )}
          </p>
        </div>

        {/* ✅ Right side: only badges now */}
        <div className="svc-admin-page-header-actions">
          {metricsHealth && renderMetricsBadge(metricsHealth)}
          {overallHealth && <NodeStatusBadge status={overallHealth} />}
        </div>
      </header>

      {metricsHealth === 'stale' && (
        <section className="svc-admin-section svc-admin-node-metrics-banner-wrap">
          <ErrorBanner
            message={
              minSampleAgeSecs != null
                ? `Metrics may be stale (last sample ${minSampleAgeSecs.toFixed(
                    1,
                  )} s ago). Check node /metrics endpoint.`
                : 'Metrics may be stale. Check node /metrics endpoint.'
            }
          />
        </section>
      )}

      {metricsHealth === 'unreachable' && (
        <section className="svc-admin-section svc-admin-node-metrics-banner-wrap">
          <ErrorBanner message="Metrics unreachable. svc-admin cannot reach this node’s /metrics endpoint. Check the node, network path, and /metrics exposure." />
        </section>
      )}

      <div className="svc-admin-node-detail-layout">
        {/* LEFT: main operational view */}
        <div className="svc-admin-node-detail-main">
          <section className="svc-admin-section svc-admin-section-node-overview">
            {/* ✅ “Cooler” planes header + summary pills */}
            <div
              style={{
                display: 'flex',
                alignItems: 'baseline',
                justifyContent: 'space-between',
                gap: 12,
                marginBottom: 10,
              }}
            >
              <h2 style={{ margin: 0 }}>Planes</h2>

              <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}>
                <Pill tone={planeSummary.ready === planeSummary.total ? 'ok' : 'warn'}>
                  {planeSummary.ready}/{planeSummary.total} Ready
                </Pill>
                {planeSummary.degraded > 0 && (
                  <Pill tone="warn">{planeSummary.degraded} Degraded</Pill>
                )}
                {planeSummary.down > 0 && (
                  <Pill tone="bad">{planeSummary.down} Down</Pill>
                )}
                <Pill tone="muted">{planeSummary.restarts} Restarts</Pill>
              </div>
            </div>

            <div
              style={{
                borderRadius: 18,
                padding: 14,
                border:
                  '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
                background:
                  'radial-gradient(1200px 500px at 20% 0%, rgba(99,102,241,0.10), transparent 55%), linear-gradient(180deg, rgba(255,255,255,0.03), rgba(255,255,255,0.012))',
                boxShadow: '0 14px 38px rgba(0,0,0,0.22)',
                position: 'relative',
                overflow: 'hidden',
              }}
            >
              {/* subtle accent rail */}
              <div
                aria-hidden="true"
                style={{
                  position: 'absolute',
                  left: 0,
                  top: 0,
                  bottom: 0,
                  width: 4,
                  background:
                    overallHealth === 'down'
                      ? 'rgba(244,63,94,0.75)'
                      : overallHealth === 'degraded'
                        ? 'rgba(251,146,60,0.65)'
                        : 'rgba(16,185,129,0.55)',
                  boxShadow: '0 0 18px rgba(0,0,0,0.25)',
                }}
              />

              <div
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: '1rem',
                  flexWrap: 'wrap',
                }}
              >
                {/* Plane table */}
                <div
                  style={{
                    flex: '0 0 340px',
                    maxWidth: 440,
                    minWidth: 300,
                  }}
                >
                  <PlaneStatusTable planes={planes} />
                </div>

                {/* 2x2 compact grid: CPU | RAM  then  Storage | Bandwidth */}
                <div
                  style={{
                    flex: '1 1 520px',
                    minWidth: 520,
                  }}
                >
                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: 'repeat(2, minmax(170px, 1fr))',
                      gap: '1rem',
                      alignItems: 'stretch',
                    }}
                  >
                    <MiniMetricCard title="CPU">
                      <SpeedometerGauge pct={cpuPct} compact />
                    </MiniMetricCard>

                    <MiniMetricCard title="RAM">
                      <ThermometerGauge pct={ramPct} compact />
                    </MiniMetricCard>

                    <MiniMetricCard title="Storage">
                      <StorageWaffleGauge pct={storagePct} compact />
                    </MiniMetricCard>

                    <MiniMetricCard title="Bandwidth">
                      <BandwidthBarsGauge
                        pct={bandwidthPct}
                        seed={utilSeed}
                        compact
                      />
                    </MiniMetricCard>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section className="svc-admin-section svc-admin-section-node-metrics">
            <h2>Facet Metrics</h2>
            <FacetMetricsPanel
              facets={facets}
              loading={facetsLoading}
              error={facetsError}
              historyByFacet={facetHistory}
            />
          </section>

          <section className="svc-admin-section svc-admin-section-node-actions">
            <h2>Actions</h2>
            <p className="svc-admin-node-actions-caption">
              Node actions are gated by server-side config and roles. In dev,
              svc-admin defaults to read-only mode.
            </p>

            <div className="svc-admin-node-actions-grid">
              <button
                type="button"
                className="svc-admin-node-action-button"
                disabled={!canMutate || actionInFlight !== null}
                onClick={() => runAction('reload')}
              >
                {actionInFlight === 'reload'
                  ? 'Reloading…'
                  : 'Reload node configuration'}
              </button>

              <button
                type="button"
                className="svc-admin-node-action-button svc-admin-node-action-button-danger"
                disabled={!canMutate || actionInFlight !== null}
                onClick={() => runAction('shutdown')}
              >
                {actionInFlight === 'shutdown'
                  ? 'Shutting down…'
                  : 'Shutdown node'}
              </button>
            </div>

            {actionMessage && (
              <p className="svc-admin-node-actions-message">{actionMessage}</p>
            )}
            {actionError && <ErrorBanner message={actionError} />}
          </section>

          {devDebugEnabled && planes.length > 0 && (
            <section className="svc-admin-section svc-admin-section-node-debug">
              <h2>Debug controls</h2>
              <p className="svc-admin-node-actions-caption">
                Dev-only synthetic crash tool. This emits a crash event for the
                selected plane without killing a real worker. Do not expose in
                production.
              </p>

              <div className="svc-admin-node-debug-controls">
                <label>
                  Plane to crash:{' '}
                  <select
                    value={debugPlane}
                    onChange={(e) => setDebugPlane(e.target.value)}
                  >
                    {planes.map((plane) => (
                      <option key={plane.name} value={plane.name}>
                        {plane.name}
                      </option>
                    ))}
                  </select>
                </label>

                <button
                  type="button"
                  className="svc-admin-node-action-button svc-admin-node-action-button-danger"
                  disabled={debugInFlight}
                  onClick={runDebugCrash}
                >
                  {debugInFlight
                    ? 'Triggering crash…'
                    : 'Trigger synthetic crash'}
                </button>
              </div>

              {debugMessage && (
                <p className="svc-admin-node-actions-message">{debugMessage}</p>
              )}
              {debugError && <ErrorBanner message={debugError} />}
            </section>
          )}
        </div>

        {/* RIGHT: data-plane + playground sidebar */}
        <aside className="svc-admin-node-detail-sidebar">
          <NodeDetailSidebar
            status={status}
            planes={planes} // ✅ new: enables richer Data planes card
            metricsHealth={metricsHealth}
            minSampleAgeSecs={minSampleAgeSecs}
            loading={statusLoading || facetsLoading}
          />
        </aside>
      </div>
    </div>
  )
}
