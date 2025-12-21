// crates/svc-admin/ui/src/components/nodes/NodeDetailSidebar.tsx
//
// WHAT:
//   Right-hand sidebar on Node detail page.
//   Shows curated shortcuts (storage/db), uptime/identity-ish facts,
//   a quick "data planes" glance, and a playground stub.
//
// WHY:
//   Keeps the main operational view on the left while providing
//   quick-read panels on the right.

import React, { useMemo } from 'react'
import { Link } from 'react-router-dom'
import type { AdminStatusView } from '../../types/admin-api'
import type { MetricsHealth } from './NodeCard'

type PlaneLike = {
  name?: string
  health?: 'healthy' | 'degraded' | 'down' | string
  ready?: boolean | string
  restarts?: number
  restart_count?: number
  restartCount?: number
}

type Props = {
  status: AdminStatusView
  metricsHealth: MetricsHealth | null
  minSampleAgeSecs: number | null
  loading: boolean
  planes?: PlaneLike[] | null
}

function planeRestarts(p: PlaneLike): number {
  const a = p.restarts
  if (typeof a === 'number' && Number.isFinite(a)) return a
  const b = p.restart_count
  if (typeof b === 'number' && Number.isFinite(b)) return b
  const c = p.restartCount
  if (typeof c === 'number' && Number.isFinite(c)) return c
  return 0
}

function planeReady(p: PlaneLike): boolean | null {
  if (typeof p.ready === 'boolean') return p.ready
  if (typeof p.ready === 'string') {
    const s = p.ready.toLowerCase().trim()
    if (s === 'ready' || s === 'true' || s === 'ok') return true
    if (s === 'not_ready' || s === 'not ready' || s === 'false') return false
  }
  return null
}

function healthTone(h: string): 'ok' | 'warn' | 'bad' | 'muted' {
  const s = (h ?? '').toLowerCase()
  if (s === 'healthy') return 'ok'
  if (s === 'degraded') return 'warn'
  if (s === 'down') return 'bad'
  return 'muted'
}

function hasStorageCapability(status: AdminStatusView | null): boolean {
  if (!status) return false
  const caps = (status as any).capabilities
  // If not reported yet, don’t block in dev
  if (!Array.isArray(caps)) return true
  return caps.includes('storage.readonly.v1')
}

function fmtUptimeLong(secs: number | null | undefined): string {
  if (typeof secs !== 'number' || !Number.isFinite(secs) || secs < 0) return '—'
  const s = Math.floor(secs)

  const days = Math.floor(s / 86400)
  const hours = Math.floor((s % 86400) / 3600)
  const mins = Math.floor((s % 3600) / 60)
  const rem = s % 60

  const parts: string[] = []
  if (days > 0) parts.push(`${days}d`)
  if (hours > 0 || days > 0) parts.push(`${hours}h`)
  if (mins > 0 || hours > 0 || days > 0) parts.push(`${mins}m`)
  parts.push(`${rem}s`)
  return parts.join(' ')
}

function Badge(props: {
  tone: 'ok' | 'warn' | 'bad' | 'muted'
  children: React.ReactNode
}) {
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
        padding: '3px 8px',
        borderRadius: 999,
        fontSize: 11,
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

function Card(props: {
  title: string
  children: React.ReactNode
  footer?: React.ReactNode
}) {
  return (
    <div
      style={{
        borderRadius: 18,
        border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
        background:
          'linear-gradient(180deg, rgba(255,255,255,0.03), rgba(255,255,255,0.012))',
        boxShadow: '0 14px 34px rgba(0,0,0,0.20)',
        padding: 16,
      }}
    >
      <div style={{ fontSize: 14, fontWeight: 900, marginBottom: 8 }}>{props.title}</div>
      <div>{props.children}</div>
      {props.footer && <div style={{ marginTop: 12 }}>{props.footer}</div>}
    </div>
  )
}

export function NodeDetailSidebar({
  status,
  planes: planesProp,
  metricsHealth,
  minSampleAgeSecs,
  loading,
}: Props) {
  // ✅ ~10% narrower without touching layout CSS
  const wrapperStyle: React.CSSProperties = {
    width: '90%',
    marginLeft: 'auto',
  }

  const planes: PlaneLike[] = useMemo(() => {
    const fromProp = planesProp ?? null
    const fromStatus = ((status as any)?.planes as PlaneLike[]) ?? []
    return (fromProp && fromProp.length > 0 ? fromProp : fromStatus) ?? []
  }, [planesProp, status])

  const dataPlanes = useMemo(() => {
    const isDataPlane = (name: string) => {
      const s = name.toLowerCase()
      // Include both svc-* and friendly names
      return (
        s.startsWith('svc-') ||
        s.includes('storage') ||
        s.includes('index') ||
        s.includes('mailbox') ||
        s.includes('overlay') ||
        s.includes('dht') ||
        s.includes('db') ||
        s.includes('kv')
      )
    }

    return planes
      .map((p) => ({
        name: String(p.name ?? ''),
        health: String(p.health ?? 'unknown'),
        ready: planeReady(p),
        restarts: planeRestarts(p),
      }))
      .filter((p) => p.name.length > 0 && isDataPlane(p.name))
  }, [planes])

  const metricsPill = useMemo(() => {
    if (metricsHealth === 'fresh') return <Badge tone="ok">Metrics: fresh</Badge>
    if (metricsHealth === 'stale') return <Badge tone="warn">Metrics: stale</Badge>
    if (metricsHealth === 'unreachable') return <Badge tone="bad">Metrics: unreachable</Badge>
    return <Badge tone="muted">Metrics: unknown</Badge>
  }, [metricsHealth])

  // ✅ restore working storage link behavior
  const nodeId = status?.id ? String(status.id) : ''
  const canOpenStorage = nodeId.length > 0 && hasStorageCapability(status)
  const storageHref = `/nodes/${encodeURIComponent(nodeId)}/storage`

  const uptimeSecs =
    typeof (status as any)?.uptime_seconds === 'number' && Number.isFinite((status as any).uptime_seconds)
      ? ((status as any).uptime_seconds as number)
      : null

  return (
    <div style={wrapperStyle}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        <Card title="Uptime">
          <div style={{ fontSize: 13, opacity: 0.85, lineHeight: 1.35 }}>
            <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', alignItems: 'center' }}>
              <Badge tone={uptimeSecs != null ? 'ok' : 'muted'}>
                {uptimeSecs != null ? 'Reported' : 'Not reported'}
              </Badge>
              <span style={{ fontWeight: 900 }}>
                {uptimeSecs != null ? fmtUptimeLong(uptimeSecs) : '—'}
              </span>
            </div>
            <div style={{ marginTop: 8, opacity: 0.78 }}>
              This is best-effort from <code>/api/v1/status</code>. Next step is to add launch
              metadata (startedAt, launchedBy, bootId/pid) once macronode exposes it.
            </div>
          </div>
        </Card>

        <Card
          title="Data & storage"
          footer={
            canOpenStorage ? (
              <Link to={storageHref} className="svc-admin-node-preview-open">
                <span>Open storage &amp; databases</span>
                <span className="svc-admin-node-preview-open-icon">→</span>
              </Link>
            ) : (
              <div style={{ fontSize: 13, opacity: 0.75 }}>
                {nodeId.length === 0 ? 'Node id unavailable.' : 'Not supported by this node yet.'}
              </div>
            )
          }
        >
          <div style={{ fontSize: 13, opacity: 0.85, lineHeight: 1.35 }}>
            Databases, disk usage, and safe storage facts (read-only).
          </div>
        </Card>

        <Card title="Data planes">
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: 10,
              marginBottom: 10,
            }}
          >
            <div style={{ fontSize: 13, opacity: 0.85 }}>
              Quick read on storage-ish planes (health / readiness / restarts).
            </div>
            {metricsPill}
          </div>

          {loading && <div style={{ fontSize: 13, opacity: 0.75 }}>Loading…</div>}

          {!loading && dataPlanes.length === 0 && (
            <div style={{ fontSize: 13, opacity: 0.75 }}>
              No data-plane services reported for this node yet.
            </div>
          )}

          {!loading && dataPlanes.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {dataPlanes.map((p) => {
                const tone = healthTone(p.health)
                return (
                  <div
                    key={p.name}
                    style={{
                      display: 'grid',
                      gridTemplateColumns: '1fr auto',
                      gap: 10,
                      alignItems: 'center',
                      padding: '10px 10px',
                      borderRadius: 14,
                      background: 'rgba(255,255,255,0.02)',
                      border: '1px solid rgba(255,255,255,0.08)',
                    }}
                  >
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontWeight: 900, fontSize: 13 }}>{p.name}</div>
                      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginTop: 6 }}>
                        <Badge tone={tone}>
                          {p.health.toLowerCase() === 'unknown'
                            ? 'Unknown'
                            : p.health.charAt(0).toUpperCase() + p.health.slice(1)}
                        </Badge>

                        <Badge
                          tone={p.ready === true ? 'ok' : p.ready === false ? 'warn' : 'muted'}
                        >
                          {p.ready === true ? 'Ready' : p.ready === false ? 'Not ready' : 'Unknown'}
                        </Badge>

                        <Badge tone="muted">{p.restarts} restarts</Badge>
                      </div>
                    </div>

                    <div
                      style={{ fontSize: 12, opacity: 0.75, textAlign: 'right' }}
                      title="Minimum facet sample age (proxy for metrics freshness)."
                    >
                      {minSampleAgeSecs != null ? `${minSampleAgeSecs.toFixed(1)}s` : '—'}
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </Card>

        <Card
          title="Playground"
          footer={
            <button
              type="button"
              disabled
              style={{
                width: '100%',
                borderRadius: 999,
                padding: '9px 12px',
                background: 'rgba(255,255,255,0.03)',
                border: '1px solid rgba(255,255,255,0.10)',
                color: 'rgba(255,255,255,0.55)',
                fontWeight: 800,
                cursor: 'not-allowed',
              }}
            >
              Run (coming soon)
            </button>
          }
        >
          <div style={{ fontSize: 13, opacity: 0.85, lineHeight: 1.35 }}>
            Future home for a read-only playground (safe queries, targeted metrics, and structured
            logs) scoped to this node. For now this is just a stub.
          </div>

          <pre
            style={{
              marginTop: 10,
              padding: 12,
              borderRadius: 14,
              background: 'rgba(0,0,0,0.28)',
              border: '1px solid rgba(255,255,255,0.08)',
              fontSize: 12,
              overflowX: 'auto',
              opacity: 0.9,
            }}
          >
{`# Example (stub)
# SELECT * FROM jobs LIMIT 25;`}
          </pre>
        </Card>
      </div>
    </div>
  )
}
