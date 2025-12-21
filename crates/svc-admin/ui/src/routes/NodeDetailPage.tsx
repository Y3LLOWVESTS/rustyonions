// crates/svc-admin/ui/src/routes/NodeDetailPage.tsx
//
// WHAT:
//   Deep-dive Node detail screen. Two-column layout:
//   - Left: planes table, facet metrics, actions, debug.
//   - Right: NodeDetailSidebar with "Data & storage" + "Playground" stubs.
// WHY:
//   Keep this route thin + compositional; heavy logic lives in a hook and
//   route-local helpers to improve maintainability and reduce 1k+ LOC files.
// INTERACTS:
//   - routes/node-detail/useNodeDetail.ts        (polling/actions/debug)
//   - routes/node-detail/*                       (pure helpers + gauges)
//   - components/nodes/*                         (tables/badges/sidebar)
// INVARIANTS:
//   - No conditional hooks; polling uses stable effects.
//   - UI supports safe mock fallback patterns.
// SECURITY:
//   - Mutations gated by server-side config + roles (same as before).

import React, { useEffect, useMemo, useState } from 'react'
import { useParams, Link } from 'react-router-dom'

import { PlaneStatusTable } from '../components/nodes/PlaneStatusTable'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { FacetMetricsPanel } from '../components/metrics/FacetMetricsPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { NodeDetailSidebar } from '../components/nodes/NodeDetailSidebar'
import type { MetricsHealth } from '../components/nodes/NodeCard'

import { adminClient } from '../api/adminClient'
import type { StorageSummaryDto, SystemSummaryDto } from '../types/admin-api'

import { useNodeDetail } from './node-detail/useNodeDetail'
import { deriveOverallHealth } from './node-detail/health'
import { classifyMetricsHealth, renderMetricsBadge } from './node-detail/metricsHealth'
import {
  mockNodeUtilization,
  seedFromString,
  MiniMetricCard,
  SpeedometerGauge,
  ThermometerGauge,
  StorageWaffleGauge,
  BandwidthBarsGauge,
} from './node-detail/utilization'
import { computePlaneSummary, Pill } from './node-detail/planeSummary'

type FetchErr = Error & { status?: number }

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

function computeRamPct(sys: SystemSummaryDto | null): number | null {
  if (!sys) return null
  const total = sys.ramTotalBytes
  const used = sys.ramUsedBytes
  if (!Number.isFinite(total) || total <= 0) return null
  const pct = (Math.max(0, used) / total) * 100
  return clampPct(pct)
}

function computeStoragePct(st: StorageSummaryDto | null): number | null {
  if (!st) return null
  const total = st.totalBytes
  const used = st.usedBytes
  if (!Number.isFinite(total) || total <= 0) return null
  const pct = (Math.max(0, used) / total) * 100
  return clampPct(pct)
}

function computeBandwidthPct(sys: SystemSummaryDto | null): number | null {
  if (!sys) return null
  const rx = sys.netRxBps
  const tx = sys.netTxBps
  const rxB = typeof rx === 'number' && Number.isFinite(rx) ? Math.max(0, rx) : 0
  const txB = typeof tx === 'number' && Number.isFinite(tx) ? Math.max(0, tx) : 0

  // If both are missing/null, treat as no live bandwidth.
  if (!rx && !tx) return null

  // Soft utilization: assume a 1 Gbps link until we expose link speed.
  const usedBits = (rxB + txB) * 8
  const assumedLinkBits = 1e9
  const pct = (usedBits / assumedLinkBits) * 100
  return clampPct(pct)
}

function useLiveUtilization(nodeId: string) {
  const [system, setSystem] = useState<SystemSummaryDto | null>(null)
  const [storage, setStorage] = useState<StorageSummaryDto | null>(null)

  useEffect(() => {
    if (!nodeId) {
      setSystem(null)
      setStorage(null)
      return
    }

    let cancelled = false

    ;(async () => {
      // System summary is optional; missing endpoint -> ignore
      try {
        const s = await adminClient.getNodeSystemSummary(nodeId)
        if (!cancelled) setSystem(s)
      } catch (err) {
        if (!cancelled) {
          if (!isMissingEndpoint(err)) {
            // keep quiet for now; we can surface later if desired
            void err
          }
          setSystem(null)
        }
      }

      // Storage summary is also optional in general (but you already have it)
      try {
        const st = await adminClient.getNodeStorageSummary(nodeId)
        if (!cancelled) setStorage(st)
      } catch (err) {
        if (!cancelled) {
          if (!isMissingEndpoint(err)) {
            void err
          }
          setStorage(null)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  const mock = useMemo(() => mockNodeUtilization(nodeId), [nodeId])

  const cpuPct = useMemo(() => {
    const live = system?.cpuPercent
    if (typeof live === 'number' && Number.isFinite(live)) return clampPct(live)
    return mock.cpuPct
  }, [system, mock])

  const ramPct = useMemo(() => {
    const live = computeRamPct(system)
    if (live != null) return live
    return mock.ramPct
  }, [system, mock])

  const storagePct = useMemo(() => {
    const live = computeStoragePct(storage)
    if (live != null) return live
    return mock.storagePct
  }, [storage, mock])

  const bandwidthPct = useMemo(() => {
    const live = computeBandwidthPct(system)
    if (live != null) return live
    return mock.bandwidthPct
  }, [system, mock])

  return { cpuPct, ramPct, storagePct, bandwidthPct }
}

export function NodeDetailPage() {
  const params = useParams<{ id: string }>()
  const nodeId = params.id ?? ''

  const {
    status,
    statusLoading,
    statusError,

    facets,
    facetsLoading,
    facetsError,
    facetHistory,

    identityLoading,
    identityError,

    canMutate,
    actionInFlight,
    actionMessage,
    actionError,
    runAction,

    devDebugEnabled,
    debugPlane,
    setDebugPlane,
    debugInFlight,
    debugMessage,
    debugError,
    runDebugCrash,
  } = useNodeDetail(nodeId)

  const planes: any[] = useMemo(
    () => ((status as any)?.planes as any[]) ?? [],
    [status],
  )

  const overallHealth = useMemo(() => {
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

  // ✅ Live utilization (CPU/RAM/NET + storage when available) with mock fallback
  const { cpuPct, ramPct, storagePct, bandwidthPct } = useLiveUtilization(nodeId)

  const planeSummary = useMemo(() => computePlaneSummary(planes), [planes])

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
        <div className="svc-admin-node-detail-main">
          <section className="svc-admin-section svc-admin-section-node-overview">
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
                <Pill
                  tone={planeSummary.ready === planeSummary.total ? 'ok' : 'warn'}
                >
                  {planeSummary.ready}/{planeSummary.total} Ready
                </Pill>
                {planeSummary.degraded > 0 && (
                  <Pill tone="warn">{planeSummary.degraded} Degraded</Pill>
                )}
                {planeSummary.down > 0 && <Pill tone="bad">{planeSummary.down} Down</Pill>}
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
                <div style={{ flex: '0 0 340px', maxWidth: 440, minWidth: 300 }}>
                  <PlaneStatusTable planes={planes} />
                </div>

                <div style={{ flex: '1 1 520px', minWidth: 520 }}>
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
                      <BandwidthBarsGauge pct={bandwidthPct} seed={utilSeed} compact />
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
                {actionInFlight === 'reload' ? 'Reloading…' : 'Reload node configuration'}
              </button>

              <button
                type="button"
                className="svc-admin-node-action-button svc-admin-node-action-button-danger"
                disabled={!canMutate || actionInFlight !== null}
                onClick={() => runAction('shutdown')}
              >
                {actionInFlight === 'shutdown' ? 'Shutting down…' : 'Shutdown node'}
              </button>
            </div>

            {actionMessage && <p className="svc-admin-node-actions-message">{actionMessage}</p>}
            {actionError && <ErrorBanner message={actionError} />}
          </section>

          {devDebugEnabled && planes.length > 0 && (
            <section className="svc-admin-section svc-admin-section-node-debug">
              <h2>Debug controls</h2>
              <p className="svc-admin-node-actions-caption">
                Dev-only synthetic crash tool. This emits a crash event for the selected plane
                without killing a real worker. Do not expose in production.
              </p>

              <div className="svc-admin-node-debug-controls">
                <label>
                  Plane to crash:{' '}
                  <select value={debugPlane} onChange={(e) => setDebugPlane(e.target.value)}>
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
                  {debugInFlight ? 'Triggering crash…' : 'Trigger synthetic crash'}
                </button>
              </div>

              {debugMessage && <p className="svc-admin-node-actions-message">{debugMessage}</p>}
              {debugError && <ErrorBanner message={debugError} />}
            </section>
          )}
        </div>

        <aside className="svc-admin-node-detail-sidebar">
          <NodeDetailSidebar
            status={status}
            planes={planes}
            metricsHealth={metricsHealth}
            minSampleAgeSecs={minSampleAgeSecs}
            loading={statusLoading || facetsLoading}
          />
        </aside>
      </div>
    </div>
  )
}
