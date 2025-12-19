// crates/svc-admin/ui/src/routes/NodeStoragePage.tsx
//
// RO:WHAT — Node “Data & Storage” page (Storage summary + database inventory).
// RO:WHY  — Give operators a read-only, curated view of disk/DB health without
//           turning svc-admin into a remote file browser.
// RO:HOW  — Uses svc-admin backend storage endpoints when present; falls back
//           to deterministic mock data when endpoints are missing (404/405/501).
// RO:INVARIANTS —
//   - Read-only: no mutations.
//   - No raw filesystem browsing; storage/DB details must be curated DTOs.

import React, { useMemo } from 'react'
import { Link, useParams } from 'react-router-dom'

import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { EmptyState } from '../components/shared/EmptyState'

import { useNodeStorage } from './node-storage/useNodeStorage'
import { deriveOverallHealthOrNull, computeMetricsHealth } from './node-storage/helpers'
import { fmtBytes, fmtBps, clamp01 } from './node-storage/format'
import { mockStorageSummary, mockDatabaseDetail } from './node-storage/mock'
import { RingGauge, DbIcon, gaugeColor, gaugeLevelFromPct } from './node-storage/ringGauge'

type MetricsHealth = 'fresh' | 'stale' | 'unreachable'

export function NodeStoragePage() {
  const params = useParams()
  const nodeId = params.id ?? ''

  const {
    status,
    statusLoading,
    statusError,

    facets,
    facetsLoading,
    facetsError,

    storage,
    storageLoading,
    storageError,
    storageSource,

    databases,
    dbLoading,
    dbError,
    dbSource,

    selectedDb,
    setSelectedDb,
    dbDetail,
    dbDetailLoading,
    dbDetailError,
  } = useNodeStorage(nodeId)

  const overallHealth = useMemo(
    () => deriveOverallHealthOrNull(status?.planes),
    [status?.planes],
  )

  const metricsHealth: MetricsHealth = useMemo(
    () => computeMetricsHealth(facets, facetsError),
    [facets, facetsError],
  )

  const title = status?.display_name ?? nodeId

  // Storage should always be present (hook falls back), but keep a hard-safe default.
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

  // ------------------------ renders (safe early returns) --------------------

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
                    [
                      'Health',
                      sidebarDb.health.charAt(0).toUpperCase() + sidebarDb.health.slice(1),
                    ],
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
