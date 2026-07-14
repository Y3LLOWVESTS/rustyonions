// crates/svc-admin/ui/src/routes/NodeStoragePage.tsx
//
// WHAT:
//   Read-only storage summary and database inventory for one node.
// WHY:
//   Gives operators useful storage posture without fabricating missing
//   capacities, databases, permissions, or filesystem information.
// INVARIANTS:
//   - Only node-reported storage and database DTOs are displayed.
//   - Missing endpoints render unavailable state.
//   - No raw filesystem browsing and no deterministic fallback records.

import React, { useMemo } from 'react'
import { Link, useParams } from 'react-router-dom'

import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { EmptyState } from '../components/shared/EmptyState'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'

import { fmtBps, fmtBytes, clamp01 } from './node-storage/format'
import {
  computeMetricsHealth,
  deriveOverallHealthOrNull,
} from './node-storage/helpers'
import {
  DbIcon,
  RingGauge,
  gaugeColor,
  gaugeLevelFromPct,
} from './node-storage/ringGauge'
import { useNodeStorage } from './node-storage/useNodeStorage'

type MetricsHealth = 'fresh' | 'stale' | 'unreachable'

function safeLocaleInt(value: unknown): string {
  return typeof value === 'number' && Number.isFinite(value)
    ? value.toLocaleString()
    : 'Not reported'
}

function safeString(
  value: unknown,
  fallback = 'Not reported',
): string {
  return typeof value === 'string' && value.trim().length > 0
    ? value
    : fallback
}

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

  const usedPct = useMemo(() => {
    if (!storage || storage.totalBytes <= 0) {
      return null
    }

    return Math.round(
      clamp01(storage.usedBytes / storage.totalBytes) * 100,
    )
  }, [storage])

  const freePct =
    usedPct === null ? null : Math.max(0, 100 - usedPct)

  const lowDisk =
    storage !== null &&
    freePct !== null &&
    freePct <= 10

  const sourceLabel = useMemo(() => {
    if (storageSource === 'live' && dbSource === 'live') {
      return 'Live'
    }

    if (
      storageSource === 'unavailable' &&
      dbSource === 'unavailable'
    ) {
      return 'Unavailable'
    }

    return 'Partial'
  }, [storageSource, dbSource])

  const sidebarDb = useMemo(() => {
    if (!dbDetail) {
      return null
    }

    return {
      ...dbDetail,
      approxKeys:
        dbDetail.approxKeys === undefined
          ? null
          : dbDetail.approxKeys,
      warnings: Array.isArray(dbDetail.warnings)
        ? dbDetail.warnings
        : [],
    }
  }, [dbDetail])

  const topDbGauges = useMemo(() => {
    if (!storage || storage.totalBytes <= 0) {
      return []
    }

    return [...databases]
      .sort((left, right) => right.sizeBytes - left.sizeBytes)
      .slice(0, 3)
      .map((database) => {
        const percentage =
          clamp01(database.sizeBytes / storage.totalBytes) * 100
        const level = gaugeLevelFromPct(percentage)

        return {
          name: database.name,
          size: fmtBytes(database.sizeBytes),
          percentage,
          color: gaugeColor(level),
        }
      })
  }, [databases, storage])

  const dbOpenHref = useMemo(() => {
    if (!selectedDb || !status?.id) {
      return null
    }

    return (
      `/nodes/${encodeURIComponent(status.id)}` +
      `/storage/databases/${encodeURIComponent(selectedDb)}`
    )
  }, [selectedDb, status?.id])

  if (!nodeId) {
    return (
      <div className="svc-admin-page svc-admin-page-node-storage">
        <div className="svc-admin-section">
          <ErrorBanner message="Missing node id in route." />
          <Link to="/" className="svc-admin-link-muted">
            ← Nodes
          </Link>
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
          <ErrorBanner
            message={statusError ?? 'Node status is unavailable.'}
          />
          <Link to="/" className="svc-admin-link-muted">
            ← Nodes
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div
      className="svc-admin-page svc-admin-page-node-storage"
      style={{ paddingBottom: 150 }}
    >
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <h1>{title}</h1>
          <p className="svc-admin-page-subtitle">
            Data &amp; storage inventory (read-only).{' '}
            <span style={{ opacity: 0.85 }}>
              Source: {sourceLabel}
            </span>
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
            {overallHealth && (
              <NodeStatusBadge status={overallHealth} />
            )}

            <span
              className={
                `svc-admin-metrics-pill ` +
                `svc-admin-metrics-pill--${metricsHealth}`
              }
            >
              Metrics: {metricsHealth}
            </span>
          </div>
        </div>
      </header>

      {facetsLoading && (
        <section
          className="svc-admin-section"
          style={{ marginBottom: '1rem' }}
        >
          <LoadingSpinner />
        </section>
      )}

      {!facetsLoading && facetsError && (
        <ErrorBanner message="Facet metrics are unavailable for this node." />
      )}

      {(storageError || dbError || dbDetailError) && (
        <ErrorBanner
          message={
            storageError ??
            dbError ??
            dbDetailError ??
            'Storage information is unavailable.'
          }
        />
      )}

      {lowDisk && storage && freePct !== null && (
        <ErrorBanner
          message={
            `Low disk headroom: ${freePct}% free on ` +
            `${storage.mount}.`
          }
        />
      )}

      <div className="svc-admin-node-detail-layout">
        <div className="svc-admin-node-detail-main">
          <section
            className="svc-admin-section"
            style={{ marginBottom: '1rem' }}
          >
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
                Share of the node-reported storage total
              </div>
            </div>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns:
                  'repeat(auto-fit, minmax(180px, 1fr))',
                gap: '0.75rem',
                marginTop: '0.75rem',
              }}
            >
              {dbLoading ? (
                <LoadingSpinner />
              ) : topDbGauges.length === 0 ? (
                <EmptyState
                  message={
                    storage
                      ? 'No database inventory was reported.'
                      : 'Database shares require a reported storage total.'
                  }
                />
              ) : (
                topDbGauges.map((gauge) => (
                  <RingGauge
                    key={gauge.name}
                    pct={gauge.percentage}
                    label={gauge.name}
                    sublabel={gauge.size}
                    color={gauge.color}
                    onClick={() => setSelectedDb(gauge.name)}
                  />
                ))
              )}
            </div>
          </section>

          <section
            className="svc-admin-section"
            style={{ marginBottom: '1rem' }}
          >
            <h2>Storage summary</h2>

            {storageLoading ? (
              <LoadingSpinner />
            ) : !storage ? (
              <EmptyState
                message={
                  'Storage summary was not reported by this node. ' +
                  'No capacity or utilization values are synthesized.'
                }
              />
            ) : (
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns:
                    'repeat(2, minmax(0, 1fr))',
                  gap: '0.75rem',
                  marginTop: '0.75rem',
                }}
              >
                <div
                  className="svc-admin-card"
                  style={{ padding: '0.9rem 1rem' }}
                >
                  <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>
                    Disk
                  </div>
                  <div
                    style={{
                      fontSize: '1.1rem',
                      fontWeight: 700,
                      marginTop: '0.2rem',
                    }}
                  >
                    {fmtBytes(storage.usedBytes)} used
                  </div>
                  <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                    / {fmtBytes(storage.totalBytes)} (
                    {usedPct ?? 'Not reported'}%)
                  </div>
                </div>

                <div
                  className="svc-admin-card"
                  style={{ padding: '0.9rem 1rem' }}
                >
                  <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>
                    Free
                  </div>
                  <div
                    style={{
                      fontSize: '1.1rem',
                      fontWeight: 700,
                      marginTop: '0.2rem',
                    }}
                  >
                    {fmtBytes(storage.freeBytes)}
                  </div>
                  <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                    {freePct ?? 'Not reported'}% headroom
                  </div>
                </div>

                <div
                  className="svc-admin-card"
                  style={{ padding: '0.9rem 1rem' }}
                >
                  <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>
                    I/O
                  </div>
                  <div style={{ fontSize: '0.95rem' }}>
                    Read: <strong>{fmtBps(storage.ioReadBps)}</strong>
                  </div>
                  <div style={{ fontSize: '0.95rem' }}>
                    Write: <strong>{fmtBps(storage.ioWriteBps)}</strong>
                  </div>
                </div>

                <div
                  className="svc-admin-card"
                  style={{ padding: '0.9rem 1rem' }}
                >
                  <div style={{ fontSize: '0.85rem', opacity: 0.8 }}>
                    Databases
                  </div>
                  <div
                    style={{
                      fontSize: '1.1rem',
                      fontWeight: 700,
                      marginTop: '0.2rem',
                    }}
                  >
                    {dbLoading ? '…' : databases.length}
                  </div>
                  <div style={{ fontSize: '0.9rem', opacity: 0.8 }}>
                    Mount: <strong>{storage.mount}</strong> · FS:{' '}
                    <strong>{storage.fsType}</strong>
                  </div>
                </div>
              </div>
            )}

            <p
              style={{
                marginTop: '0.75rem',
                fontSize: '0.9rem',
                opacity: 0.85,
              }}
            >
              All values on this page come from read-only node DTOs.
              Missing storage facts remain unavailable.
            </p>
          </section>

          <section className="svc-admin-section">
            <h2>Databases</h2>

            {dbLoading ? (
              <LoadingSpinner />
            ) : databases.length === 0 ? (
              <EmptyState
                message="No database inventory was reported by this node."
              />
            ) : (
              <div
                className="svc-admin-card"
                style={{
                  padding: 0,
                  overflow: 'hidden',
                  marginTop: '0.75rem',
                }}
              >
                <table
                  className="svc-admin-plane-table"
                  style={{ marginTop: 0 }}
                >
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
                    {databases.map((database) => (
                      <tr
                        key={database.name}
                        style={{
                          cursor: 'pointer',
                          ...(database.name === selectedDb
                            ? {
                                background:
                                  'var(--svc-admin-color-accent-soft)',
                              }
                            : {}),
                        }}
                        onClick={() =>
                          setSelectedDb(database.name)
                        }
                        title={database.notes ?? ''}
                      >
                        <td style={{ fontWeight: 650 }}>
                          {database.name}
                        </td>
                        <td>{database.engine}</td>
                        <td>{fmtBytes(database.sizeBytes)}</td>
                        <td>{database.mode}</td>
                        <td>
                          <span
                            style={{
                              fontWeight: 650,
                              textTransform: 'capitalize',
                            }}
                          >
                            {database.health}
                          </span>
                          {(database.worldReadable ||
                            database.worldWritable) && (
                            <span
                              style={{
                                marginLeft: '0.5rem',
                                opacity: 0.75,
                              }}
                            >
                              {database.worldWritable
                                ? '⚠ perms'
                                : '⚠ read'}
                            </span>
                          )}
                        </td>
                      </tr>
                    ))}
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
                gap: '0.75rem',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.5rem',
                }}
              >
                <DbIcon />
                <div>
                  <div className="svc-admin-node-detail-sidebar-title">
                    Database details
                  </div>
                  <div className="svc-admin-node-detail-sidebar-caption">
                    Read-only node-reported metadata
                  </div>
                </div>
              </div>

              <div style={{ fontSize: '0.8rem', opacity: 0.75 }}>
                {selectedDb
                  ? dbDetailLoading
                    ? 'Loading…'
                    : safeString(sidebarDb?.engine)
                  : ''}
              </div>
            </div>

            {selectedDb && dbOpenHref && (
              <div style={{ marginTop: '0.6rem' }}>
                <Link
                  to={dbOpenHref}
                  className="svc-admin-link-muted"
                >
                  Open →
                </Link>
              </div>
            )}

            {!selectedDb && (
              <div className="svc-admin-node-detail-sidebar-empty">
                Select a reported database to inspect its details.
              </div>
            )}

            {selectedDb &&
              !dbDetailLoading &&
              !sidebarDb && (
                <div className="svc-admin-node-detail-sidebar-empty">
                  Database detail was not reported by this node.
                </div>
              )}

            {selectedDb && sidebarDb && (
              <>
                <div
                  style={{
                    fontSize: '1.15rem',
                    fontWeight: 750,
                    margin: '0.75rem 0',
                  }}
                >
                  {sidebarDb.name}
                </div>

                <div className="svc-admin-node-detail-sidebar-table">
                  {[
                    ['Size', fmtBytes(sidebarDb.sizeBytes)],
                    ['Owner', safeString(sidebarDb.owner)],
                    ['Mode', safeString(sidebarDb.mode)],
                    ['Path alias', safeString(sidebarDb.pathAlias)],
                    ['Files', safeLocaleInt(sidebarDb.fileCount)],
                    [
                      'Keys',
                      sidebarDb.approxKeys == null
                        ? 'Not reported'
                        : safeLocaleInt(sidebarDb.approxKeys),
                    ],
                    [
                      'Compaction',
                      sidebarDb.lastCompaction ?? 'Not reported',
                    ],
                    ['Health', safeString(sidebarDb.health)],
                  ].map(([label, value]) => (
                    <div
                      key={label}
                      className="svc-admin-node-detail-sidebar-row"
                    >
                      <div className="svc-admin-node-detail-sidebar-row-main">
                        <div className="svc-admin-node-detail-sidebar-row-name">
                          {label}
                        </div>
                      </div>
                      <div className="svc-admin-node-detail-sidebar-row-aside">
                        <div
                          style={{
                            fontSize: '0.9rem',
                            fontWeight: 650,
                          }}
                        >
                          {value}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>

                {sidebarDb.warnings.length > 0 && (
                  <div style={{ marginTop: '0.9rem' }}>
                    <strong>Warnings</strong>
                    <ul>
                      {sidebarDb.warnings.map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                  </div>
                )}

                <div
                  style={{
                    marginTop: '0.9rem',
                    fontSize: '0.9rem',
                    opacity: 0.85,
                  }}
                >
                  No database details are synthesized when the
                  backend omits them.
                </div>
              </>
            )}
          </div>
        </aside>
      </div>
    </div>
  )
}
