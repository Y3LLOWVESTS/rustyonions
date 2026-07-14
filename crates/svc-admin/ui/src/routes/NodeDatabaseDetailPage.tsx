// crates/svc-admin/ui/src/routes/NodeDatabaseDetailPage.tsx
//
// WHAT:
//   Read-only detail page for one node-reported database.
// WHY:
//   Shows curated database metadata without generating fake files,
//   capacities, permissions, timestamps, or filesystem paths.
// INVARIANTS:
//   - No mutations.
//   - No raw filesystem browser.
//   - Missing database detail remains unavailable.
//   - File inventory is shown only when a real DTO exists.

import React, { useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'

import { adminClient } from '../api/adminClient'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { EmptyState } from '../components/shared/EmptyState'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import type {
  AdminStatusView,
  DatabaseDetailDto,
} from '../types/admin-api'

type OverallHealth =
  | 'healthy'
  | 'degraded'
  | 'down'
  | null

type FetchError = Error & { status?: number }

function isMissingEndpoint(err: unknown): boolean {
  const error = err as FetchError

  return (
    error?.status === 404 ||
    error?.status === 405 ||
    error?.status === 501
  )
}

function deriveOverallHealth(
  planes:
    | Array<{
        health: 'healthy' | 'degraded' | 'down'
      }>
    | undefined,
): OverallHealth {
  if (!planes || planes.length === 0) {
    return null
  }

  if (planes.some((plane) => plane.health === 'down')) {
    return 'down'
  }

  if (
    planes.some((plane) => plane.health === 'degraded')
  ) {
    return 'degraded'
  }

  return 'healthy'
}

function fmtBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) {
    return 'Not reported'
  }

  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  let value = bytes
  let index = 0

  while (value >= 1024 && index < units.length - 1) {
    value /= 1024
    index += 1
  }

  const digits = index === 0 ? 0 : index <= 2 ? 1 : 2
  return `${value.toFixed(digits)} ${units[index]}`
}

function modeLooksWorldReadable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return ['4', '5', '6', '7'].includes(last)
}

function modeLooksWorldWritable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return ['2', '3', '6', '7'].includes(last)
}

function DbIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        d="M12 2c4.97 0 9 1.79 9 4s-4.03 4-9 4-9-1.79-9-4 4.03-4 9-4Z"
        fill="var(--svc-admin-color-accent, #3b82f6)"
        opacity="0.35"
      />
      <path
        d="M3 6v12c0 2.21 4.03 4 9 4s9-1.79 9-4V6"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
      />
    </svg>
  )
}

export function NodeDatabaseDetailPage() {
  const params = useParams<{
    id: string
    name: string
  }>()

  const nodeId = params.id ?? ''
  const databaseName = params.name ?? ''

  const [status, setStatus] =
    useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] =
    useState<string | null>(null)

  const [detail, setDetail] =
    useState<DatabaseDetailDto | null>(null)
  const [detailLoading, setDetailLoading] = useState(true)
  const [detailError, setDetailError] =
    useState<string | null>(null)

  useEffect(() => {
    if (!nodeId) {
      setStatus(null)
      setStatusLoading(false)
      setStatusError('Missing node id in route.')
      return
    }

    let cancelled = false

    setStatus(null)
    setStatusLoading(true)
    setStatusError(null)

    void (async () => {
      try {
        const data = await adminClient.getNodeStatus(nodeId)

        if (!cancelled) {
          setStatus(data)
        }
      } catch (err) {
        if (!cancelled) {
          setStatusError(
            err instanceof Error
              ? err.message
              : 'Failed to load node status.',
          )
        }
      } finally {
        if (!cancelled) {
          setStatusLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId || !databaseName) {
      setDetail(null)
      setDetailLoading(false)
      setDetailError(
        !nodeId
          ? 'Missing node id in route.'
          : 'Missing database name in route.',
      )
      return
    }

    let cancelled = false

    setDetail(null)
    setDetailLoading(true)
    setDetailError(null)

    void (async () => {
      try {
        const data =
          await adminClient.getNodeStorageDatabaseDetail(
            nodeId,
            databaseName,
          )

        if (!cancelled) {
          setDetail(data)
        }
      } catch (err) {
        if (!cancelled) {
          setDetail(null)
          setDetailError(
            isMissingEndpoint(err)
              ? 'Database detail is not reported by this node.'
              : err instanceof Error
                ? err.message
                : 'Failed to load database detail.',
          )
        }
      } finally {
        if (!cancelled) {
          setDetailLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId, databaseName])

  const overallHealth = useMemo(
    () => deriveOverallHealth(status?.planes),
    [status?.planes],
  )

  const nodeTitle =
    status?.display_name ?? status?.id ?? nodeId

  if (!nodeId || !databaseName) {
    return (
      <div className="svc-admin-page">
        <div className="svc-admin-section">
          <ErrorBanner
            message="Missing node id or database name in route."
          />
          <Link to="/" className="svc-admin-link-muted">
            ← Nodes
          </Link>
        </div>
      </div>
    )
  }

  if (statusLoading || detailLoading) {
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
          <Link to="/" className="svc-admin-link-muted">
            ← Nodes
          </Link>
        </div>
      </div>
    )
  }

  if (detailError || !detail) {
    return (
      <div className="svc-admin-page">
        <div className="svc-admin-section">
          <ErrorBanner
            message={
              detailError ??
              'Database detail is unavailable.'
            }
          />
          <p>
            svc-admin does not generate placeholder database
            metadata or file paths.
          </p>
          <Link
            to={`/nodes/${encodeURIComponent(nodeId)}/storage`}
            className="svc-admin-link-muted"
          >
            ← Storage
          </Link>
        </div>
      </div>
    )
  }

  const worldReadable =
    modeLooksWorldReadable(detail.mode)
  const worldWritable =
    modeLooksWorldWritable(detail.mode)
  const warnings = Array.isArray(detail.warnings)
    ? detail.warnings
    : []

  return (
    <div className="svc-admin-page svc-admin-page-node-db-detail">
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <Link
            to={`/nodes/${encodeURIComponent(nodeId)}/storage`}
            className="svc-admin-link-muted"
          >
            ← Back
          </Link>

          <h1
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
            }}
          >
            <DbIcon />
            <span>{nodeTitle}</span>
          </h1>

          <p className="svc-admin-page-subtitle">
            Database detail (read-only). Source: Live
          </p>

          <p className="svc-admin-node-meta">
            <span className="svc-admin-node-id">
              <strong>DB:</strong> {detail.name}
            </span>{' '}
            <span className="svc-admin-node-profile">
              <strong>Engine:</strong> {detail.engine}
            </span>
          </p>
        </div>

        <div className="svc-admin-page-header-actions">
          {overallHealth && (
            <NodeStatusBadge status={overallHealth} />
          )}
        </div>
      </header>

      {(worldReadable || worldWritable) && (
        <section
          className="svc-admin-section"
          style={{ marginBottom: '1rem' }}
        >
          <ErrorBanner
            message={
              worldWritable
                ? 'Permission warning: database appears world-writable.'
                : 'Permission warning: database appears world-readable.'
            }
          />
        </section>
      )}

      {warnings.length > 0 && (
        <section
          className="svc-admin-section"
          style={{ marginBottom: '1rem' }}
        >
          <strong>Backend warnings</strong>
          <ul>
            {warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      )}

      <section
        className="svc-admin-section"
        style={{ marginBottom: '1rem' }}
      >
        <h2>Database summary</h2>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns:
              'repeat(2, minmax(0, 1fr))',
            gap: '0.75rem',
          }}
        >
          <div
            className="svc-admin-card"
            style={{ padding: '0.9rem 1rem' }}
          >
            <strong>Size</strong>
            <div>{fmtBytes(detail.sizeBytes)}</div>
            <div>
              Files:{' '}
              {typeof detail.fileCount === 'number'
                ? detail.fileCount.toLocaleString()
                : 'Not reported'}
            </div>
          </div>

          <div
            className="svc-admin-card"
            style={{ padding: '0.9rem 1rem' }}
          >
            <strong>Permissions</strong>
            <div>{detail.mode}</div>
            <div>Owner: {detail.owner}</div>
          </div>

          <div
            className="svc-admin-card"
            style={{ padding: '0.9rem 1rem' }}
          >
            <strong>Path alias</strong>
            <div>{detail.pathAlias || 'Not reported'}</div>
            <div>Health: {detail.health}</div>
          </div>

          <div
            className="svc-admin-card"
            style={{ padding: '0.9rem 1rem' }}
          >
            <strong>Engine facts</strong>
            <div>
              Keys:{' '}
              {detail.approxKeys == null
                ? 'Not reported'
                : detail.approxKeys.toLocaleString()}
            </div>
            <div>
              Compaction:{' '}
              {detail.lastCompaction ?? 'Not reported'}
            </div>
          </div>
        </div>
      </section>

      <section className="svc-admin-section">
        <h2>Curated file inventory</h2>
        <EmptyState
          message={
            'No file-inventory DTO is published for this database. ' +
            'svc-admin does not synthesize filenames, paths, sizes, ' +
            'permissions, or modification timestamps.'
          }
        />
      </section>
    </div>
  )
}
