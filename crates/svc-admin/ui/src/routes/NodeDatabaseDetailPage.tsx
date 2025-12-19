// crates/svc-admin/ui/src/routes/NodeDatabaseDetailPage.tsx
//
// RO:WHAT — Node “Database Detail” page (curated DB + file inventory).
// RO:WHY  — Clicking a database from NodeStoragePage must land on a real page,
//           not the SPA 404, and show “God-tier” operator context: DB metadata,
//           permission posture, and a curated file list (safe, read-only).
// RO:HOW  — Calls getNodeStatus + getNodeDatabaseDetail when available; falls back
//           to deterministic mock facts when endpoints are missing (404/405/501).
// RO:INVARIANTS —
//   - Read-only only; no mutations.
//   - No raw filesystem browsing. Paths are aliases, and file list is curated
//     (or deterministic safe mock fallback).

import React, { useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'

import { adminClient } from '../api/adminClient'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { EmptyState } from '../components/shared/EmptyState'

import type { AdminStatusView, DatabaseDetailDto } from '../types/admin-api'

type FetchErr = Error & { status?: number }
type OverallHealth = 'healthy' | 'degraded' | 'down' | null

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

function deriveOverallHealth(
  planes: Array<{ health: 'healthy' | 'degraded' | 'down' }> | undefined,
): OverallHealth {
  if (!planes || planes.length === 0) return null
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
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

function modeLooksWorldReadable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return last === '4' || last === '5' || last === '6' || last === '7'
}

function modeLooksWorldWritable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return last === '2' || last === '3' || last === '6' || last === '7'
}

function seedFromString(s: string): number {
  let acc = 0
  for (let i = 0; i < s.length; i++) acc = (acc * 31 + s.charCodeAt(i)) >>> 0
  return acc >>> 0
}

function mulberry32(seed: number) {
  let t = seed >>> 0
  return () => {
    t += 0x6d2b79f5
    let x = Math.imul(t ^ (t >>> 15), 1 | t)
    x ^= x + Math.imul(x ^ (x >>> 7), 61 | x)
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296
  }
}

// ------------------------ deterministic mock fallback ------------------------

function mockDatabaseDetail(nodeId: string, name: string): DatabaseDetailDto {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  const bump = (n: number) => n + (seed % 17) * 1024 * 1024

  const fallback = [
    {
      name: 'svc-index.sled',
      engine: 'sled',
      sizeBytes: bump(1_250_000_000),
      mode: '0750',
      owner: 'ron:ron',
      health: 'ok' as const,
      pathAlias: 'data/db',
      fileCount: 3_200,
      lastCompaction: '2025-12-12T19:19:00Z',
      approxKeys: 12_400_000,
    },
    {
      name: 'svc-storage.cas',
      engine: 'fs-cas',
      sizeBytes: bump(88_500_000_000),
      mode: '0700',
      owner: 'ron:ron',
      health: 'ok' as const,
      pathAlias: 'data/cas',
      fileCount: 128_400,
      lastCompaction: null,
      approxKeys: null,
    },
    {
      name: 'svc-overlay.sled',
      engine: 'sled',
      sizeBytes: bump(4_800_000_000),
      mode: '0755',
      owner: 'ron:ron',
      health: 'degraded' as const,
      pathAlias: 'data/db',
      fileCount: 3_200,
      lastCompaction: '2025-12-12T19:19:00Z',
      approxKeys: 12_400_000,
    },
  ]

  const hit = fallback.find((d) => d.name === name) ?? fallback[0]

  const warnings: string[] = []
  if (modeLooksWorldReadable(hit.mode)) {
    warnings.push('Permissions: database appears world-readable.')
  }
  if (modeLooksWorldWritable(hit.mode)) {
    warnings.push('Permissions: database appears world-writable (high risk).')
  }
  if (hit.health !== 'ok') {
    warnings.push(
      'Health: database reports degraded status (investigate I/O or compaction).',
    )
  }

  return {
    name: hit.name,
    engine: hit.engine,
    sizeBytes: hit.sizeBytes,
    mode: hit.mode,
    owner: hit.owner,
    health: hit.health,
    pathAlias: hit.pathAlias,
    fileCount: hit.fileCount,
    lastCompaction: hit.lastCompaction,
    approxKeys: hit.approxKeys,
    warnings,
  }
}

// ------------------------ curated file inventory (safe) ---------------------

type DbFileKind = 'manifest' | 'log' | 'sst' | 'lock' | 'segment' | 'chunk' | 'other'

type DbFileEntry = {
  path: string // alias path only (no absolute)
  kind: DbFileKind
  sizeBytes: number
  mode: string
  owner: string
  modified: string // ISO-ish for display
  worldReadable: boolean
  worldWritable: boolean
}

function isoFromSeed(rand: () => number): string {
  const year = 2025
  const month = 1 + Math.floor(rand() * 12)
  const day = 1 + Math.floor(rand() * 28)
  const hour = Math.floor(rand() * 24)
  const min = Math.floor(rand() * 60)
  const sec = Math.floor(rand() * 60)
  const mm = String(month).padStart(2, '0')
  const dd = String(day).padStart(2, '0')
  const hh = String(hour).padStart(2, '0')
  const mi = String(min).padStart(2, '0')
  const ss = String(sec).padStart(2, '0')
  return `${year}-${mm}-${dd}T${hh}:${mi}:${ss}Z`
}

function pickModeForFile(dbMode: string, rand: () => number): string {
  // Keep consistent with the DB’s posture but allow a little variety.
  // (Still safe: this is a curated *fact surface*, not a raw fs view.)
  const base = dbMode.trim()
  const roll = rand()
  if (roll < 0.08) return '0600'
  if (roll < 0.18) return '0640'
  if (roll < 0.30) return '0644'
  return base.length ? base : '0640'
}

function generateCuratedFiles(
  nodeId: string,
  detail: DatabaseDetailDto,
  limit: number,
): DbFileEntry[] {
  const seed = seedFromString(`${nodeId}::${detail.name}::${detail.engine}`)
  const rand = mulberry32(seed)

  const mk = (path: string, kind: DbFileKind, sizeBytes: number): DbFileEntry => {
    const mode = pickModeForFile(detail.mode, rand)
    return {
      path,
      kind,
      sizeBytes: Math.max(0, Math.floor(sizeBytes)),
      mode,
      owner: detail.owner,
      modified: isoFromSeed(rand),
      worldReadable: modeLooksWorldReadable(mode),
      worldWritable: modeLooksWorldWritable(mode),
    }
  }

  const out: DbFileEntry[] = []
  const base = detail.pathAlias || 'data/db'

  // Some well-known “always present” style entries.
  out.push(mk(`${base}/${detail.name}/MANIFEST`, 'manifest', 220_000 + rand() * 900_000))
  out.push(mk(`${base}/${detail.name}/LOCK`, 'lock', 4_096))
  out.push(mk(`${base}/${detail.name}/LOG`, 'log', 1_500_000 + rand() * 12_000_000))

  if (detail.engine === 'sled') {
    // Generate “sst-like” segments as a curated inventory sample.
    const count = Math.max(0, limit - out.length)
    for (let i = 0; i < count; i++) {
      const n = i + 1
      const hex = (seed + n * 2654435761) >>> 0
      const id = hex.toString(16).padStart(8, '0')
      const size = 2_000_000 + rand() * 28_000_000
      out.push(mk(`${base}/${detail.name}/tables/${id}.sst`, 'sst', size))
    }
  } else if (detail.engine === 'fs-cas') {
    // CAS “chunks” (alias paths only).
    const count = Math.max(0, limit - out.length)
    for (let i = 0; i < count; i++) {
      const a = Math.floor(rand() * 256)
        .toString(16)
        .padStart(2, '0')
      const b = Math.floor(rand() * 256)
        .toString(16)
        .padStart(2, '0')
      const c = Math.floor(rand() * 256)
        .toString(16)
        .padStart(2, '0')
      const tail = Math.floor(rand() * 0xffffffff)
        .toString(16)
        .padStart(8, '0')
      const size = 64_000 + rand() * 6_000_000
      out.push(mk(`${base}/b3/${a}/${b}/${c}/${tail}.chunk`, 'chunk', size))
    }
  } else {
    const count = Math.max(0, limit - out.length)
    for (let i = 0; i < count; i++) {
      const size = 80_000 + rand() * 4_500_000
      out.push(mk(`${base}/${detail.name}/seg/${i.toString().padStart(6, '0')}.dat`, 'segment', size))
    }
  }

  return out.slice(0, limit)
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

// ------------------------------- page ---------------------------------------

export function NodeDatabaseDetailPage() {
  const params = useParams<{ id: string; name: string }>()
  const nodeId = params.id ?? ''
  const dbName = params.name ?? ''

  const [status, setStatus] = useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] = useState<string | null>(null)

  const [detail, setDetail] = useState<DatabaseDetailDto | null>(null)
  const [detailLoading, setDetailLoading] = useState(true)
  const [detailError, setDetailError] = useState<string | null>(null)
  const [source, setSource] = useState<'live' | 'mock'>('mock')

  const [filter, setFilter] = useState('')
  const FILES_LIMIT = 60

  // Status (for title + health badge)
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

  // Database detail (optional endpoint; safe mock fallback)
  useEffect(() => {
    if (!nodeId || !dbName) {
      setDetail(null)
      setDetailLoading(false)
      setDetailError(!nodeId ? 'Missing node id in route.' : 'Missing database name in route.')
      return
    }

    let cancelled = false
    setDetailLoading(true)
    setDetailError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeDatabaseDetail(nodeId, dbName)
        if (cancelled) return
        setDetail(live)
        setSource('live')
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          setDetail(mockDatabaseDetail(nodeId, dbName))
          setSource('mock')
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load database detail.'
          setDetailError(msg)
          setDetail(mockDatabaseDetail(nodeId, dbName))
          setSource('mock')
        }
      } finally {
        if (!cancelled) setDetailLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId, dbName])

  // Derived
  const overallHealth: OverallHealth = useMemo(
    () => deriveOverallHealth(status?.planes),
    [status?.planes],
  )

  const nodeTitle = status?.display_name ?? status?.id ?? nodeId

  const effectiveDetail = useMemo(() => {
    if (!nodeId || !dbName) return null
    return detail ?? mockDatabaseDetail(nodeId, dbName)
  }, [detail, nodeId, dbName])

  const filesAll = useMemo(() => {
    if (!nodeId || !effectiveDetail) return []
    return generateCuratedFiles(nodeId, effectiveDetail, FILES_LIMIT)
  }, [nodeId, effectiveDetail])

  const filesFiltered = useMemo(() => {
    const q = filter.trim().toLowerCase()
    if (!q) return filesAll
    return filesAll.filter((f) => f.path.toLowerCase().includes(q))
  }, [filesAll, filter])

  if (!nodeId || !dbName) {
    return (
      <div className="svc-admin-page">
        <div className="svc-admin-section">
          <ErrorBanner message="Missing node id or database name in route." />
          <div style={{ marginTop: '0.75rem' }}>
            <Link to="/" className="svc-admin-link-muted">
              ← Nodes
            </Link>
          </div>
        </div>
      </div>
    )
  }

  if (statusLoading && detailLoading) {
    return (
      <div className="svc-admin-page">
        <div className="svc-admin-section">
          <LoadingSpinner />
        </div>
      </div>
    )
  }

  if (statusError) {
    return (
      <div className="svc-admin-page">
        <div className="svc-admin-section">
          <ErrorBanner message={statusError} />
          <div style={{ marginTop: '0.75rem' }}>
            <Link to="/" className="svc-admin-link-muted">
              ← Nodes
            </Link>
          </div>
        </div>
      </div>
    )
  }

  if (!effectiveDetail) {
    return (
      <div className="svc-admin-page">
        <div className="svc-admin-section">
          <ErrorBanner message="Database detail is unavailable." />
          <div style={{ marginTop: '0.75rem' }}>
            <Link to={`/nodes/${encodeURIComponent(nodeId)}/storage`} className="svc-admin-link-muted">
              ← Storage
            </Link>
          </div>
        </div>
      </div>
    )
  }

  const worldReadable = modeLooksWorldReadable(effectiveDetail.mode)
  const worldWritable = modeLooksWorldWritable(effectiveDetail.mode)

  return (
    <div className="svc-admin-page svc-admin-page-node-db-detail">
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <div style={{ marginBottom: 8 }}>
            <Link to={`/nodes/${encodeURIComponent(nodeId)}/storage`} className="svc-admin-link-muted">
              ← Back
            </Link>
          </div>

          <h1 style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <DbIcon />
            <span>{nodeTitle}</span>
          </h1>

          <p className="svc-admin-page-subtitle">
            Database detail (read-only).{' '}
            <span style={{ opacity: 0.85 }}>
              Source: {source === 'live' ? 'Live' : 'Mock'}
            </span>
          </p>

          <p className="svc-admin-node-meta">
            <span className="svc-admin-node-id">
              <strong>DB:</strong> {effectiveDetail.name}
            </span>{' '}
            <span className="svc-admin-node-profile">
              <strong>Engine:</strong> {effectiveDetail.engine}
            </span>
          </p>
        </div>

        <div className="svc-admin-page-header-actions">
          {overallHealth && <NodeStatusBadge status={overallHealth} />}
        </div>
      </header>

      {detailError && (
        <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
          <ErrorBanner message="Some database endpoints failed; falling back to safe mock data where needed." />
        </section>
      )}

      {(worldReadable || worldWritable) && (
        <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
          <ErrorBanner
            message={
              worldWritable
                ? 'Permission warning: database appears world-writable (high risk).'
                : 'Permission warning: database appears world-readable.'
            }
          />
        </section>
      )}

      {effectiveDetail.warnings.length > 0 && (
        <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
          <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
            <div style={{ fontWeight: 800, marginBottom: 6 }}>Warnings</div>
            <ul style={{ margin: 0, paddingLeft: '1.1rem' }}>
              {effectiveDetail.warnings.map((w) => (
                <li key={w} style={{ marginBottom: '0.35rem' }}>
                  {w}
                </li>
              ))}
            </ul>
          </div>
        </section>
      )}

      <div className="svc-admin-node-detail-layout">
        {/* MAIN */}
        <div className="svc-admin-node-detail-main">
          <section className="svc-admin-section" style={{ marginBottom: '1rem' }}>
            <h2 style={{ marginBottom: 10 }}>Database summary</h2>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
                gap: '0.75rem',
              }}
            >
              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Size</div>
                <div style={{ fontSize: '1.1rem', fontWeight: 750, marginTop: '0.2rem' }}>
                  {fmtBytes(effectiveDetail.sizeBytes)}
                </div>
                <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                  Files: <strong>{effectiveDetail.fileCount.toLocaleString()}</strong>
                </div>
              </div>

              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Permissions</div>
                <div style={{ fontSize: '1.1rem', fontWeight: 750, marginTop: '0.2rem' }}>
                  {effectiveDetail.mode}
                </div>
                <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                  Owner: <strong>{effectiveDetail.owner}</strong>
                </div>
              </div>

              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Path alias</div>
                <div style={{ fontSize: '1.05rem', fontWeight: 700, marginTop: '0.2rem' }}>
                  {effectiveDetail.pathAlias}
                </div>
                <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                  Health:{' '}
                  <strong style={{ textTransform: 'capitalize' }}>
                    {effectiveDetail.health}
                  </strong>
                </div>
              </div>

              <div className="svc-admin-card" style={{ padding: '0.9rem 1rem' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>Sled facts</div>
                <div style={{ fontSize: '0.95rem', marginTop: '0.2rem' }}>
                  Keys:{' '}
                  <strong>
                    {effectiveDetail.approxKeys == null
                      ? 'n/a'
                      : effectiveDetail.approxKeys.toLocaleString()}
                  </strong>
                </div>
                <div style={{ fontSize: '0.95rem' }}>
                  Compaction:{' '}
                  <strong>{effectiveDetail.lastCompaction ?? 'n/a'}</strong>
                </div>
              </div>
            </div>

            {detailLoading && (
              <div style={{ marginTop: '0.75rem' }}>
                <LoadingSpinner />
              </div>
            )}
          </section>

          <section className="svc-admin-section">
            <div
              style={{
                display: 'flex',
                alignItems: 'baseline',
                justifyContent: 'space-between',
                gap: '1rem',
              }}
            >
              <h2 style={{ marginBottom: 0 }}>Curated file inventory</h2>
              <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>
                Showing {filesFiltered.length} of {Math.min(FILES_LIMIT, effectiveDetail.fileCount).toLocaleString()}
                {effectiveDetail.fileCount > FILES_LIMIT ? ' (sample)' : ''}
              </div>
            </div>

            <div style={{ marginTop: '0.75rem', display: 'flex', gap: '0.75rem', flexWrap: 'wrap' }}>
              <div className="svc-admin-card" style={{ padding: '0.6rem 0.75rem', flex: '1 1 380px' }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8, marginBottom: 6 }}>Filter (path contains)</div>
                <input
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  placeholder="e.g. MANIFEST, tables/, .sst, b3/"
                  style={{
                    width: '100%',
                    padding: '10px 12px',
                    borderRadius: 12,
                    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
                    background: 'rgba(255,255,255,0.03)',
                    color: 'var(--svc-admin-color-text)',
                    outline: 'none',
                  }}
                />
              </div>

              <div className="svc-admin-card" style={{ padding: '0.6rem 0.75rem', flex: '0 0 auto', minWidth: 220 }}>
                <div style={{ fontSize: '0.85rem', opacity: 0.8, marginBottom: 6 }}>Safety note</div>
                <div style={{ fontSize: '0.9rem', opacity: 0.9 }}>
                  Aliases only. No raw paths. Read-only curated facts (or safe deterministic mock).
                </div>
              </div>
            </div>

            {filesAll.length === 0 ? (
              <div style={{ marginTop: '0.75rem' }}>
                <EmptyState message="No curated file entries available." />
              </div>
            ) : (
              <div
                className="svc-admin-card"
                style={{ padding: 0, overflow: 'hidden', marginTop: '0.75rem' }}
              >
                <table className="svc-admin-plane-table" style={{ marginTop: 0 }}>
                  <thead>
                    <tr>
                      <th style={{ width: '54%' }}>Path (alias)</th>
                      <th>Kind</th>
                      <th>Size</th>
                      <th>Owner</th>
                      <th>Mode</th>
                      <th>Modified</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filesFiltered.map((f) => {
                      const warn = f.worldWritable || f.worldReadable
                      return (
                        <tr key={f.path} title={warn ? 'Permission posture flagged (world readable/writable).' : ''}>
                          <td style={{ fontWeight: 650, overflowWrap: 'anywhere' }}>{f.path}</td>
                          <td style={{ textTransform: 'uppercase', opacity: 0.9 }}>{f.kind}</td>
                          <td>{fmtBytes(f.sizeBytes)}</td>
                          <td>{f.owner}</td>
                          <td style={{ fontWeight: 650 }}>
                            {f.mode}
                            {warn && (
                              <span style={{ marginLeft: '0.5rem', opacity: 0.75 }}>
                                {f.worldWritable ? '⚠ perms' : '⚠ read'}
                              </span>
                            )}
                          </td>
                          <td style={{ opacity: 0.9 }}>{f.modified}</td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        </div>

        {/* SIDEBAR */}
        <aside className="svc-admin-node-detail-sidebar">
          <div className="svc-admin-node-detail-sidebar-card">
            <div className="svc-admin-node-detail-sidebar-title">Navigation</div>
            <div className="svc-admin-node-detail-sidebar-caption">
              Jump back to node or storage inventory.
            </div>

            <div style={{ marginTop: '0.75rem', display: 'grid', gap: '0.5rem' }}>
              <Link to={`/nodes/${encodeURIComponent(nodeId)}`} className="svc-admin-link-muted">
                ← Node overview
              </Link>
              <Link to={`/nodes/${encodeURIComponent(nodeId)}/storage`} className="svc-admin-link-muted">
                ← Storage & databases
              </Link>
              <Link to="/" className="svc-admin-link-muted">
                ← Nodes
              </Link>
            </div>

            <div style={{ marginTop: '0.9rem', fontSize: '0.9rem', opacity: 0.85 }}>
              Next step: wire macronode to expose a <strong>curated</strong> file inventory DTO
              (not a filesystem browser) so this list becomes live.
            </div>
          </div>
        </aside>
      </div>
    </div>
  )
}
