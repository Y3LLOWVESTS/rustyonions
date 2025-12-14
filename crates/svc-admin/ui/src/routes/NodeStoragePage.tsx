// crates/svc-admin/ui/src/routes/NodeStoragePage.tsx
//
// RO:WHAT — Node “Data & Storage” page (Storage summary + database inventory).
// RO:WHY  — Give operators a read-only, curated view of disk/DB health without
//           turning svc-admin into a remote file browser.
// RO:HOW  — Uses svc-admin backend storage endpoints when present; falls back
//           to deterministic mock data when endpoints are missing (404/405/501).
// RO:INTERACTS —
//   - adminClient.getNodeStatus(id)               → AdminStatusView
//   - adminClient.getNodeFacetMetrics(id)         → FacetMetricsSummary[]
//   - adminClient.getNodeStorageSummary(id)       → StorageSummaryDto (optional)
//   - adminClient.getNodeDatabases(id)            → DatabaseEntryDto[] (optional)
//   - adminClient.getNodeDatabaseDetail(id, name) → DatabaseDetailDto (optional)
// RO:INVARIANTS —
//   - Read-only: no mutations.
//   - No raw filesystem browsing; storage/DB details must be curated DTOs.

import React, { useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'

import { adminClient } from '../api/adminClient'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { EmptyState } from '../components/shared/EmptyState'
import type {
  AdminStatusView,
  FacetMetricsSummary,
  StorageSummaryDto,
  DatabaseEntryDto,
  DatabaseDetailDto,
} from '../types/admin-api'

type MetricsHealth = 'fresh' | 'stale' | 'unreachable'
type DataSource = 'live' | 'mock'

type FetchErr = Error & { status?: number }

function isMissingEndpoint(err: unknown): boolean {
  const e = err as FetchErr
  const s = e && typeof e.status === 'number' ? e.status : undefined
  if (s === 404 || s === 405 || s === 501) return true

  // Fallback heuristic: handleResponse embeds status code into the message.
  const msg = e?.message ?? ''
  return (
    msg.includes(' 404 ') ||
    msg.includes(' 405 ') ||
    msg.includes(' 501 ') ||
    msg.toLowerCase().includes('not found') ||
    msg.toLowerCase().includes('not implemented')
  )
}

function deriveOverallHealth(
  planes: Array<{ health: 'healthy' | 'degraded' | 'down' }> | undefined,
): 'healthy' | 'degraded' | 'down' | null {
  if (!planes || planes.length === 0) return null
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
}

function computeMetricsHealth(
  facets: FacetMetricsSummary[] | null,
  error: string | null,
): MetricsHealth {
  if (error) return 'unreachable'

  if (!facets || facets.length === 0) {
    // Node may be idle or just starting; treat as stale for now.
    return 'stale'
  }

  const ages = facets
    .map((f) => f.last_sample_age_secs)
    .filter((v): v is number => v !== null && Number.isFinite(v))

  if (ages.length === 0) return 'stale'

  const minAge = Math.min(...ages)
  const FRESH_THRESHOLD_SECS = 30
  return minAge <= FRESH_THRESHOLD_SECS ? 'fresh' : 'stale'
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

function fmtBps(bps: number | null): string {
  if (bps === null) return 'n/a'
  return `${fmtBytes(bps)}/s`
}

function clamp01(x: number): number {
  if (!Number.isFinite(x)) return 0
  return Math.max(0, Math.min(1, x))
}

// ------------------------ ring gauge (DB disk share) ------------------------

type GaugeLevel = 'ok' | 'warn' | 'near' | 'crit'

// Interprets "how full" as "how much of node disk is consumed by this DB".
function gaugeLevelFromPct(pct: number): GaugeLevel {
  if (pct >= 60) return 'crit'
  if (pct >= 40) return 'near'
  if (pct >= 20) return 'warn'
  return 'ok'
}

function gaugeColor(level: GaugeLevel): string {
  // Prefer theme vars; provide safe fallbacks so we don’t break if a var is missing.
  switch (level) {
    case 'ok':
      return 'var(--svc-admin-color-accent, #3b82f6)'
    case 'warn':
      return 'var(--svc-admin-color-warning-text, #facc15)'
    case 'near':
      return 'var(--svc-admin-color-warning, #fb923c)'
    case 'crit':
      return 'var(--svc-admin-color-danger-text, #ef4444)'
  }
}

function RingGauge(props: {
  pct: number // 0..100
  label: string
  sublabel?: string
  color: string
  onClick?: () => void
}) {
  const pct = Math.max(0, Math.min(100, props.pct))
  const radius = 36
  const stroke = 8
  const c = 2 * Math.PI * radius
  const dash = (pct / 100) * c
  const gap = c - dash

  return (
    <div
      className="svc-admin-card"
      style={{
        padding: '0.9rem 1rem',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: '0.55rem',
        cursor: props.onClick ? 'pointer' : 'default',
        userSelect: 'none',
      }}
      onClick={props.onClick}
      title={props.label}
    >
      <div style={{ position: 'relative', width: 96, height: 96 }}>
        <svg width="96" height="96" viewBox="0 0 96 96" aria-hidden="true">
          <circle
            cx="48"
            cy="48"
            r={radius}
            fill="none"
            stroke="var(--svc-admin-color-border, rgba(255,255,255,0.12))"
            strokeWidth={stroke}
            opacity={0.9}
          />
          <circle
            cx="48"
            cy="48"
            r={radius}
            fill="none"
            stroke={props.color}
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={`${dash} ${gap}`}
            transform="rotate(-90 48 48)"
          />
        </svg>

        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '0.1rem',
          }}
        >
          <div style={{ fontSize: '1.05rem', fontWeight: 800, lineHeight: 1 }}>
            {pct.toFixed(0)}%
          </div>
          <div style={{ fontSize: '0.75rem', opacity: 0.85, lineHeight: 1 }}>
            of disk
          </div>
        </div>
      </div>

      <div style={{ textAlign: 'center', width: '100%' }}>
        <div style={{ fontWeight: 750, fontSize: '0.95rem', overflowWrap: 'anywhere' }}>
          {props.label}
        </div>
        {props.sublabel && (
          <div style={{ fontSize: '0.8rem', opacity: 0.8, marginTop: '0.15rem' }}>
            {props.sublabel}
          </div>
        )}
      </div>
    </div>
  )
}

function DbIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      aria-hidden="true"
      style={{ opacity: 0.9 }}
    >
      <path
        d="M12 2c4.97 0 9 1.79 9 4s-4.03 4-9 4-9-1.79-9-4 4.03-4 9-4Z"
        fill="var(--svc-admin-color-accent, #3b82f6)"
        opacity="0.35"
      />
      <path
        d="M3 6v6c0 2.21 4.03 4 9 4s9-1.79 9-4V6"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
        opacity="0.95"
      />
      <path
        d="M3 12v6c0 2.21 4.03 4 9 4s9-1.79 9-4v-6"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
        opacity="0.75"
      />
      <path
        d="M3 6c0 2.21 4.03 4 9 4s9-1.79 9-4"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
        opacity="0.95"
      />
    </svg>
  )
}

// ------------------------ deterministic mock fallback ------------------------

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
    ioReadBps: 12_500_000 + (seed % 7_500_000),
    ioWriteBps: 8_500_000 + (seed % 6_000_000),
  }
}

function modeLooksWorldReadable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return last === '4' || last === '5' || last === '6' || last === '7'
}

function modeLooksWorldWritable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return last === '2' || last === '3' || last === '6' || last === '7'
}

function mockDatabases(nodeId: string): DatabaseEntryDto[] {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  const bump = (n: number) => n + (seed % 17) * 1024 * 1024

  const list: DatabaseEntryDto[] = [
    {
      name: 'svc-index.sled',
      engine: 'sled',
      sizeBytes: bump(1_250_000_000),
      mode: '0750',
      owner: 'ron:ron',
      health: 'ok',
      notes: 'Name → ContentId resolution indexes.',
    },
    {
      name: 'svc-storage.cas',
      engine: 'fs-cas',
      sizeBytes: bump(88_500_000_000),
      mode: '0700',
      owner: 'ron:ron',
      health: 'ok',
      notes: 'Content-addressed object store (b3:*).',
    },
    {
      name: 'svc-overlay.sled',
      engine: 'sled',
      sizeBytes: bump(4_800_000_000),
      mode: '0755',
      owner: 'ron:ron',
      health: 'degraded',
      notes: '⚠ world-readable (policy warning).',
    },
  ]

  return list.map((d) => ({
    ...d,
    worldReadable: modeLooksWorldReadable(d.mode),
    worldWritable: modeLooksWorldWritable(d.mode),
  }))
}

function mockDatabaseDetail(nodeId: string, name: string): DatabaseDetailDto {
  const list = mockDatabases(nodeId)
  const hit = list.find((d) => d.name === name) ?? list[0]
  const warnings: string[] = []

  if (hit.worldReadable) warnings.push('Permissions: database appears world-readable.')
  if (hit.worldWritable) warnings.push('Permissions: database appears world-writable (high risk).')
  if (hit.health !== 'ok') {
    warnings.push('Health: database reports degraded status (investigate I/O or compaction).')
  }

  return {
    name: hit.name,
    engine: hit.engine,
    sizeBytes: hit.sizeBytes,
    mode: hit.mode,
    owner: hit.owner,
    health: hit.health,
    pathAlias: hit.engine === 'fs-cas' ? 'data/cas' : 'data/db',
    fileCount: hit.engine === 'fs-cas' ? 128_400 : 3_200,
    lastCompaction: hit.engine === 'sled' ? '2025-12-12T19:19:00Z' : null,
    approxKeys: hit.engine === 'sled' ? 12_400_000 : null,
    warnings,
  }
}

// ------------------------------- page ---------------------------------------

export function NodeStoragePage() {
  const params = useParams()
  const nodeId = params.id ?? ''

  // --- status / metrics ----------------------------------------------------
  const [status, setStatus] = useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] = useState<string | null>(null)

  const [facets, setFacets] = useState<FacetMetricsSummary[] | null>(null)
  const [facetsLoading, setFacetsLoading] = useState(true)
  const [facetsError, setFacetsError] = useState<string | null>(null)

  // --- storage/db endpoints (optional) -------------------------------------
  const [storage, setStorage] = useState<StorageSummaryDto | null>(null)
  const [storageLoading, setStorageLoading] = useState(true)
  const [storageError, setStorageError] = useState<string | null>(null)
  const [storageSource, setStorageSource] = useState<DataSource>('mock')

  const [databases, setDatabases] = useState<DatabaseEntryDto[]>([])
  const [dbLoading, setDbLoading] = useState(true)
  const [dbError, setDbError] = useState<string | null>(null)
  const [dbSource, setDbSource] = useState<DataSource>('mock')

  const [selectedDb, setSelectedDb] = useState<string | null>(null)
  const [dbDetail, setDbDetail] = useState<DatabaseDetailDto | null>(null)
  const [dbDetailLoading, setDbDetailLoading] = useState(false)
  const [dbDetailError, setDbDetailError] = useState<string | null>(null)

  // ------------------------ effects ----------------------------------------

  useEffect(() => {
    if (!nodeId) {
      setStatus(null)
      setStatusLoading(false)
      setStatusError('Missing node id in route.')
      return
    }

    let cancelled = false
    setStatusLoading(true)
    setStatusError(null)

    ;(async () => {
      try {
        const data = await adminClient.getNodeStatus(nodeId)
        if (cancelled) return
        setStatus(data)
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load node status.'
        setStatusError(msg)
      } finally {
        if (!cancelled) setStatusLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setFacets(null)
      setFacetsLoading(false)
      setFacetsError('Missing node id in route.')
      return
    }

    let cancelled = false
    setFacetsLoading(true)
    setFacetsError(null)

    ;(async () => {
      try {
        const data = await adminClient.getNodeFacetMetrics(nodeId)
        if (cancelled) return
        setFacets(data)
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load facet metrics.'
        setFacetsError(msg)
      } finally {
        if (!cancelled) setFacetsLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setStorage(mockStorageSummary(''))
      setStorageSource('mock')
      setStorageLoading(false)
      setStorageError('Missing node id in route.')
      return
    }

    let cancelled = false
    setStorageLoading(true)
    setStorageError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeStorageSummary(nodeId)
        if (cancelled) return
        setStorage(live)
        setStorageSource('live')
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          setStorage(mockStorageSummary(nodeId))
          setStorageSource('mock')
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load storage summary.'
          setStorageError(msg)
          setStorage(mockStorageSummary(nodeId))
          setStorageSource('mock')
        }
      } finally {
        if (!cancelled) setStorageLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setDatabases(mockDatabases(''))
      setDbSource('mock')
      setDbLoading(false)
      setDbError('Missing node id in route.')
      return
    }

    let cancelled = false
    setDbLoading(true)
    setDbError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeDatabases(nodeId)
        if (cancelled) return
        setDatabases(live)
        setDbSource('live')
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          const mock = mockDatabases(nodeId)
          setDatabases(mock)
          setDbSource('mock')
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load databases.'
          setDbError(msg)
          setDatabases(mockDatabases(nodeId))
          setDbSource('mock')
        }
      } finally {
        if (!cancelled) setDbLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setSelectedDb(null)
      return
    }

    if (!databases || databases.length === 0) {
      setSelectedDb(null)
      return
    }

    setSelectedDb((prev) => {
      if (prev && databases.some((d) => d.name === prev)) return prev
      return databases[0].name
    })
  }, [nodeId, databases])

  useEffect(() => {
    if (!nodeId) {
      setDbDetail(null)
      setDbDetailLoading(false)
      setDbDetailError('Missing node id in route.')
      return
    }

    if (!selectedDb) {
      setDbDetail(null)
      setDbDetailError(null)
      setDbDetailLoading(false)
      return
    }

    let cancelled = false
    setDbDetailLoading(true)
    setDbDetailError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeDatabaseDetail(nodeId, selectedDb)
        if (cancelled) return
        setDbDetail(live)
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          setDbDetail(mockDatabaseDetail(nodeId, selectedDb))
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load database detail.'
          setDbDetailError(msg)
          setDbDetail(mockDatabaseDetail(nodeId, selectedDb))
        }
      } finally {
        if (!cancelled) setDbDetailLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId, selectedDb])

  // ------------------------ derived state (ALL HOOKS ABOVE RETURNS) --------

  const overallHealth = useMemo(() => deriveOverallHealth(status?.planes), [status?.planes])

  const metricsHealth: MetricsHealth = useMemo(
    () => computeMetricsHealth(facets, facetsError),
    [facets, facetsError],
  )

  const title = status?.display_name ?? nodeId

  // storage should always be present (we default to mock on failures)
  const s = useMemo(() => storage ?? mockStorageSummary(nodeId), [storage, nodeId])

  const usedPct = useMemo(() => {
    return s.totalBytes ? Math.round((s.usedBytes / s.totalBytes) * 100) : 0
  }, [s.totalBytes, s.usedBytes])

  const freePct = useMemo(() => 100 - usedPct, [usedPct])
  const lowDisk = useMemo(() => freePct <= 10, [freePct])

  const sourceLabel = useMemo(() => {
    return storageSource === 'live' && dbSource === 'live'
      ? 'Live'
      : storageSource === 'mock' && dbSource === 'mock'
        ? 'Mock'
        : 'Mixed'
  }, [storageSource, dbSource])

  const sidebarDb = useMemo(() => {
    return dbDetail ?? (selectedDb ? mockDatabaseDetail(nodeId, selectedDb) : null)
  }, [dbDetail, selectedDb, nodeId])

  const topDbGauges = useMemo(() => {
    const total = s.totalBytes > 0 ? s.totalBytes : 1
    const top = [...databases].sort((a, b) => b.sizeBytes - a.sizeBytes).slice(0, 3)

    return top.map((db) => {
      const share = clamp01(db.sizeBytes / total)
      const pct = share * 100
      const lvl = gaugeLevelFromPct(pct)
      return {
        name: db.name,
        size: fmtBytes(db.sizeBytes),
        pct,
        color: gaugeColor(lvl),
      }
    })
  }, [databases, s.totalBytes])

  const dbOpenHref = useMemo(() => {
    return selectedDb && status?.id
      ? `/nodes/${encodeURIComponent(status.id)}/storage/databases/${encodeURIComponent(
          selectedDb,
        )}`
      : null
  }, [selectedDb, status?.id])

  // ------------------------ renders (safe early returns now) ---------------

  if (!nodeId) {
    return (
      <div className="svc-admin-page svc-admin-page-node-storage">
        <div className="svc-admin-section">
          <ErrorBanner message="Missing node id in route." />
          <div style={{ marginTop: '0.75rem' }}>
            <Link to="/" className="svc-admin-link-muted">
              ← Nodes
            </Link>
          </div>
        </div>
      </div>
    )
  }

  if (statusLoading) {
    return (
      <div className="svc-admin-page svc-admin-page-node-storage">
        <div className="svc-admin-section">
          <LoadingSpinner />
        </div>
      </div>
    )
  }

  if (statusError || !status) {
    return (
      <div className="svc-admin-page svc-admin-page-node-storage">
        <div className="svc-admin-section">
          <ErrorBanner message={statusError ?? 'Node not found.'} />
          <div style={{ marginTop: '0.75rem' }}>
            <Link to="/" className="svc-admin-link-muted">
              ← Nodes
            </Link>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="svc-admin-page svc-admin-page-node-storage">
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <h1>{title}</h1>
          <p className="svc-admin-page-subtitle">
            Data &amp; storage inventory (read-only).{' '}
            <span style={{ opacity: 0.85 }}>Source: {sourceLabel}</span>
          </p>
          <p className="svc-admin-node-meta">
            <span className="svc-admin-node-id">
              <strong>ID:</strong> {status.id}
            </span>{' '}
            {status.profile && (
              <span className="svc-admin-node-profile">
                <strong>Profile:</strong> {status.profile}
              </span>
            )}
          </p>
        </div>

        <div className="svc-admin-page-header-actions">
          <Link
            to={`/nodes/${encodeURIComponent(status.id)}`}
            className="svc-admin-link-muted"
          >
            ← Node
          </Link>
          <Link to="/" className="svc-admin-link-muted">
            ← Nodes
          </Link>

          <div className="svc-admin-node-preview-pills">
            {overallHealth && <NodeStatusBadge status={overallHealth} />}
            <span
              className={`svc-admin-metrics-pill svc-admin-metrics-pill--${metricsHealth}`}
              title={
                metricsHealth === 'fresh'
                  ? 'Metrics sampled recently.'
                  : metricsHealth === 'stale'
                    ? 'Metrics are stale or node is idle.'
                    : 'Metrics unreachable from svc-admin.'
              }
            >
              Metrics: {metricsHealth}
            </span>
          </div>
        </div>
      </header>

      {facetsLoading && (
        <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
          <LoadingSpinner />
        </section>
      )}
      {!facetsLoading && facetsError && (
        <ErrorBanner message="Metrics unreachable for this node. Storage data may be mocked." />
      )}

      {(storageError || dbError || dbDetailError) && (
        <ErrorBanner message="Some storage endpoints failed; falling back to safe mock data where needed." />
      )}

      {lowDisk && (
        <ErrorBanner message={`Low disk headroom: ~${freePct}% free on ${s.mount}.`} />
      )}

      <div className="svc-admin-node-detail-layout">
        <div className="svc-admin-node-detail-main">
          <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'baseline',
                justifyContent: 'space-between',
                gap: '1rem',
              }}
            >
              <h2 style={{ marginBottom: 0 }}>Top databases</h2>
              <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>
                Disk-share gauges (curated proxy until per-DB headroom exists)
              </div>
            </div>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                gap: '0.75rem',
                marginTop: '0.75rem',
              }}
            >
              {dbLoading ? (
                <>
                  <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                    <LoadingSpinner />
                  </div>
                  <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                    <LoadingSpinner />
                  </div>
                  <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                    <LoadingSpinner />
                  </div>
                </>
              ) : topDbGauges.length === 0 ? (
                <EmptyState message="No databases available for gauges." />
              ) : (
                topDbGauges.map((g) => (
                  <RingGauge
                    key={g.name}
                    pct={g.pct}
                    label={g.name}
                    sublabel={g.size}
                    color={g.color}
                    onClick={() => setSelectedDb(g.name)}
                  />
                ))
              )}
            </div>
          </section>

          <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
            <h2>Storage summary</h2>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
                gap: '0.75rem',
                marginTop: '0.75rem',
              }}
            >
              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Disk</div>
                <div style={{ fontSize: '1.1rem', fontWeight: 700, marginTop: '0.2rem' }}>
                  {fmtBytes(s.usedBytes)} used
                </div>
                <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                  / {fmtBytes(s.totalBytes)} ({usedPct}%)
                </div>
              </div>

              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Free</div>
                <div style={{ fontSize: '1.1rem', fontWeight: 700, marginTop: '0.2rem' }}>
                  {fmtBytes(s.freeBytes)}
                </div>
                <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                  {freePct}% headroom
                </div>
              </div>

              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>I/O</div>
                <div style={{ fontSize: '0.95rem', marginTop: '0.2rem' }}>
                  Read: <strong>{fmtBps(s.ioReadBps)}</strong>
                </div>
                <div style={{ fontSize: '0.95rem' }}>
                  Write: <strong>{fmtBps(s.ioWriteBps)}</strong>
                </div>
              </div>

              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Databases</div>
                <div style={{ fontSize: '1.1rem', fontWeight: 700, marginTop: '0.2rem' }}>
                  {dbLoading ? '…' : databases.length}
                </div>
                <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                  Mount: <strong>{s.mount}</strong> · FS: <strong>{s.fsType}</strong>
                </div>
              </div>
            </div>

            <p style={{ marginTop: '0.75rem', fontSize: '0.9rem', opacity: 0.85 }}>
              Storage/DB facts are curated and read-only. When node support is not present, this
              page safely falls back to deterministic mock data.
            </p>

            {storageLoading && (
              <div style={{ marginTop: '0.5rem' }}>
                <LoadingSpinner />
              </div>
            )}
          </section>

          <section className="svc-admin-section">
            <h2>Databases</h2>

            {dbLoading && (
              <div style={{ marginTop: '0.75rem' }}>
                <LoadingSpinner />
              </div>
            )}

            {!dbLoading && databases.length === 0 ? (
              <EmptyState message="No databases reported by this node." />
            ) : (
              <div
                className="svc-admin-card"
                style={{ padding: 0, overflow: 'hidden', marginTop: '0.75rem' }}
              >
                <table className="svc-admin-plane-table" style={{ marginTop: 0 }}>
                  <thead>
                    <tr>
                      <th style={{ width: '42%' }}>Name</th>
                      <th>Engine</th>
                      <th>Size</th>
                      <th>Perms</th>
                      <th>Health</th>
                    </tr>
                  </thead>
                  <tbody>
                    {databases.map((db) => {
                      const selected = db.name === selectedDb
                      const rowStyle: React.CSSProperties = selected
                        ? { background: 'var(--svc-admin-color-accent-soft)' }
                        : {}

                      return (
                        <tr
                          key={db.name}
                          style={{ cursor: 'pointer', ...rowStyle }}
                          onClick={() => setSelectedDb(db.name)}
                          title={db.notes ?? ''}
                        >
                          <td style={{ fontWeight: 650 }}>{db.name}</td>
                          <td>{db.engine}</td>
                          <td>{fmtBytes(db.sizeBytes)}</td>
                          <td>{db.mode}</td>
                          <td>
                            <span
                              style={{
                                fontWeight: 650,
                                textTransform: 'capitalize',
                                color:
                                  db.health === 'ok'
                                    ? 'var(--svc-admin-color-text)'
                                    : db.health === 'degraded'
                                      ? 'var(--svc-admin-color-text-muted)'
                                      : 'var(--svc-admin-color-danger-text)',
                              }}
                            >
                              {db.health}
                            </span>
                            {(db.worldReadable || db.worldWritable) && (
                              <span style={{ marginLeft: '0.5rem', opacity: 0.75 }}>
                                {db.worldWritable ? '⚠ perms' : '⚠ read'}
                              </span>
                            )}
                          </td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        </div>

        <aside className="svc-admin-node-detail-sidebar">
          <div className="svc-admin-node-detail-sidebar-card">
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'baseline',
                gap: '0.75rem',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <DbIcon />
                <div>
                  <div className="svc-admin-node-detail-sidebar-title">Database details</div>
                  <div className="svc-admin-node-detail-sidebar-caption">
                    Read-only curated node facts (or safe mock fallback).
                  </div>
                </div>
              </div>

              <div style={{ fontSize: '0.8rem', opacity: 0.75 }}>
                {selectedDb ? (dbDetailLoading ? 'Loading…' : sidebarDb?.engine ?? '') : ''}
              </div>
            </div>

            {selectedDb && dbOpenHref && (
              <div style={{ marginTop: '0.6rem' }}>
                <Link
                  to={dbOpenHref}
                  style={{
                    color: 'var(--svc-admin-color-warning-text, #facc15)',
                    fontWeight: 750,
                    textDecoration: 'none',
                  }}
                  title="Open the database details page (permissions / files / curated inventory)."
                >
                  Open →
                </Link>
              </div>
            )}

            {!selectedDb && (
              <div className="svc-admin-node-detail-sidebar-empty">
                Select a database from the table to inspect details.
              </div>
            )}

            {selectedDb && sidebarDb && (
              <>
                <div style={{ fontSize: '1.15rem', fontWeight: 750, margin: '0.75rem 0' }}>
                  {sidebarDb.name}
                </div>

                <div className="svc-admin-node-detail-sidebar-table">
                  {[
                    ['Size', fmtBytes(sidebarDb.sizeBytes)],
                    ['Owner', sidebarDb.owner],
                    ['Mode', sidebarDb.mode],
                    ['Path alias', sidebarDb.pathAlias],
                    ['Files', sidebarDb.fileCount.toLocaleString()],
                    [
                      'Keys',
                      sidebarDb.approxKeys === null
                        ? 'n/a'
                        : sidebarDb.approxKeys.toLocaleString(),
                    ],
                    ['Compaction', sidebarDb.lastCompaction ?? 'n/a'],
                    ['Health', sidebarDb.health.charAt(0).toUpperCase() + sidebarDb.health.slice(1)],
                  ].map(([k, v]) => (
                    <div key={k} className="svc-admin-node-detail-sidebar-row">
                      <div className="svc-admin-node-detail-sidebar-row-main">
                        <div className="svc-admin-node-detail-sidebar-row-name">{k}</div>
                      </div>
                      <div className="svc-admin-node-detail-sidebar-row-aside">
                        <div style={{ fontSize: '0.9rem', fontWeight: 650 }}>{v}</div>
                      </div>
                    </div>
                  ))}
                </div>

                {sidebarDb.warnings.length > 0 && (
                  <div style={{ marginTop: '0.9rem' }}>
                    <div style={{ fontWeight: 750, marginBottom: '0.35rem' }}>Warnings</div>
                    <ul style={{ margin: 0, paddingLeft: '1.1rem' }}>
                      {sidebarDb.warnings.map((w) => (
                        <li key={w} style={{ marginBottom: '0.35rem' }}>
                          {w}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                <div style={{ marginTop: '0.9rem', fontSize: '0.9rem', opacity: 0.85 }}>
                  This panel is read-only and displays curated node facts (or mock fallback).
                </div>
              </>
            )}
          </div>
        </aside>
      </div>
    </div>
  )
}
