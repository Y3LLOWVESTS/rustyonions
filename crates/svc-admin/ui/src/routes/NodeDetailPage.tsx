// crates/svc-admin/ui/src/routes/NodeDetailPage.tsx
//
// WHAT:
//   Deep-dive Node detail screen. Two-column layout:
//   - Main: utilization tiles + network accounting + facet metrics + actions/debug.
//   - Right: NodeDetailSidebar with "All planes" + storage + uptime.
//
// WHY:
//   Keep this route thin + compositional; heavy logic lives in route-local hooks/modules.
//   (This file used to be 1600+ LOC; we are splitting it into route-local modules.)
//
// INTERACTS:
//   - routes/node-detail/useNodeDetail.ts        (polling/actions/debug)
//   - routes/node-detail/liveUtilization.ts      (live CPU/RAM/Storage/Bandwidth polling)
//   - routes/node-detail/netAccounting.tsx       (network accounting polling + panel)
//   - components/metrics/FacetMetricsPanel       (2-up cards + drilldown + tags + issues filter)
//
// INVARIANTS:
//   - No conditional hooks; polling uses stable effects.
//   - UI supports safe mock fallback patterns.
// SECURITY:
//   - Mutations gated by server-side config + roles (same as before).

import React, { useMemo, useState } from 'react'
import { useParams, Link } from 'react-router-dom'

import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { FacetMetricsPanel } from '../components/metrics/FacetMetricsPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { NodeDetailSidebar } from '../components/nodes/NodeDetailSidebar'
import type { MetricsHealth } from '../components/nodes/NodeCard'

import { useNodeDetail } from './node-detail/useNodeDetail'
import { deriveOverallHealth } from './node-detail/health'
import { classifyMetricsHealth, renderMetricsBadge } from './node-detail/metricsHealth'
import {
  seedFromString,
  MiniMetricCard,
  SpeedometerGauge,
  ThermometerGauge,
  StorageWaffleGauge,
  BandwidthBarsGauge,
} from './node-detail/utilization'
import { computePlaneSummary, Pill } from './node-detail/planeSummary'
import { useLiveUtilization } from './node-detail/liveUtilization'
import { NetAccountingPanel, useNetAccounting } from './node-detail/netAccounting'

function fmtGiB(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—'
  const gib = bytes / (1024 * 1024 * 1024)
  return `${gib.toFixed(2)} GiB`
}

/**
 * Heuristic guard:
 * If RAM totals come back implausibly huge AND "cleanly divisible by 1024 GiB",
 * it's usually the /proc/meminfo kB -> bytes conversion bug (multiplied by 1024*1024).
 *
 * Example: 8 GiB machine:
 *  - MemTotal ~ 8,388,608 kB
 *  - correct bytes: kB * 1024  -> 8.00 GiB
 *  - bug bytes:     kB * 1024^2 -> 8192.00 GiB (1024× too big)
 *
 * We fix display by dividing by 1024 when this pattern is detected.
 */
function normalizeRamBytesForDisplay(totalBytes: number, usedBytes: number): { total: number; used: number } {
  if (!Number.isFinite(totalBytes) || totalBytes <= 0) return { total: totalBytes, used: usedBytes }
  if (!Number.isFinite(usedBytes) || usedBytes < 0) return { total: totalBytes, used: usedBytes }

  const gibTotal = totalBytes / (1024 * 1024 * 1024)

  // If it's > 1 TiB and a neat multiple of 1024 GiB, it's almost always the 1024× bug.
  // (This keeps us from touching normal machines, and avoids most legit odd-sized systems.)
  const looksLike1024xBug = gibTotal >= 1024 && Math.abs(gibTotal % 1024) < 1e-9

  if (!looksLike1024xBug) return { total: totalBytes, used: usedBytes }

  const adjTotal = totalBytes / 1024
  const adjUsed = usedBytes / 1024

  // sanity: used should not exceed total by a lot
  if (adjUsed > adjTotal * 1.05) return { total: totalBytes, used: usedBytes }

  return { total: adjTotal, used: adjUsed }
}

function mockCpuTopology(seed: number): { cores: number; threads: number } {
  const options: Array<[number, number]> = [
    [4, 8],
    [6, 12],
    [8, 16],
    [12, 24],
    [16, 32],
  ]
  const pick = options[seed % options.length]
  return { cores: pick[0], threads: pick[1] }
}

function mockTotalGiB(seed: number): number {
  const totals = [8, 16, 24, 32, 48, 64, 96, 128] as const
  return totals[(seed >>> 8) % totals.length]
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

  const planes: any[] = useMemo(() => ((status as any)?.planes as any[]) ?? [], [status])

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
      .map((f: any) => f.last_sample_age_secs)
      .filter((v: any): v is number => v !== null && Number.isFinite(v))
    if (ages.length === 0) return null
    return Math.min(...ages)
  }, [facets])

  const pageTitle =
    (status as any)?.display_name ?? (status as any)?.displayName ?? (status as any)?.id ?? nodeId

  const utilSeed = useMemo(() => seedFromString(nodeId || 'node'), [nodeId])

  const [tilesLive, setTilesLive] = useState(true)
  const [tilesIntervalMs, setTilesIntervalMs] = useState(2000)

  const {
    system,
    storage,

    cpuPct,
    cpuSource,
    ramPct,
    ramSource,
    storagePct,
    storageSource,
    bandwidthActivityPct,
    bandwidthSource,
    rxBps,
    txBps,
  } = useLiveUtilization(nodeId, { enabled: tilesLive, intervalMs: tilesIntervalMs })

  const { net, missing: netMissing } = useNetAccounting(nodeId, {
    enabled: tilesLive,
    intervalMs: Math.max(1500, tilesIntervalMs),
  })

  const planeSummary = useMemo(() => computePlaneSummary(planes), [planes])

  // ---- Extra basic info lines (facts) ------------------------------------

  const cpuFacts = useMemo(() => {
    const cores = (system as any)?.cpuCores
    const threads = (system as any)?.cpuThreads
    if (typeof cores === 'number' && Number.isFinite(cores) && cores > 0) {
      if (typeof threads === 'number' && Number.isFinite(threads) && threads > 0) {
        return `${cores}c / ${threads}t`
      }
      return `${cores} cores`
    }
    const m = mockCpuTopology(utilSeed)
    return `${m.cores}c / ${m.threads}t`
  }, [system, utilSeed])

  const ramFacts = useMemo(() => {
    const totalRaw = (system as any)?.ramTotalBytes
    const usedRaw = (system as any)?.ramUsedBytes

    if (Number.isFinite(totalRaw) && totalRaw > 0 && Number.isFinite(usedRaw) && usedRaw >= 0) {
      const norm = normalizeRamBytesForDisplay(totalRaw, usedRaw)
      return `${fmtGiB(norm.used)} / ${fmtGiB(norm.total)}`
    }

    const totalGiB = mockTotalGiB(utilSeed)
    const totalBytes = totalGiB * 1024 * 1024 * 1024
    const usedBytes = Math.round((ramPct / 100) * totalBytes)
    return `${fmtGiB(usedBytes)} / ${fmtGiB(totalBytes)}`
  }, [system, utilSeed, ramPct])

  const storageFacts = useMemo(() => {
    const total = (storage as any)?.totalBytes
    const used = (storage as any)?.usedBytes
    if (Number.isFinite(total) && total > 0 && Number.isFinite(used) && used >= 0) {
      return `${fmtGiB(used)} / ${fmtGiB(total)}`
    }
    // deterministic fallback: 512 GiB volume
    const totalBytes = 512 * 1024 * 1024 * 1024
    const usedBytes = Math.round((storagePct / 100) * totalBytes)
    return `${fmtGiB(usedBytes)} / ${fmtGiB(totalBytes)}`
  }, [storage, storagePct])

  if (statusLoading && facetsLoading && identityLoading) {
    return (
      <div className="svc-admin-page svc-admin-page-node-detail" style={{ paddingBottom: 140 }}>
        <div className="svc-admin-section">
          <LoadingSpinner />
        </div>
      </div>
    )
  }

  if (statusError || identityError) {
    return (
      <div className="svc-admin-page svc-admin-page-node-detail" style={{ paddingBottom: 140 }}>
        <div className="svc-admin-section">
          <ErrorBanner
            message={statusError ?? identityError ?? 'Something went wrong while loading the node.'}
          />
        </div>
      </div>
    )
  }

  if (!status) {
    return (
      <div className="svc-admin-page svc-admin-page-node-detail" style={{ paddingBottom: 140 }}>
        <div className="svc-admin-section">
          <ErrorBanner message="Node status is unavailable." />
        </div>
      </div>
    )
  }

  return (
    <div className="svc-admin-page svc-admin-page-node-detail" style={{ paddingBottom: 160 }}>
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <div style={{ marginBottom: 8 }}>
            <Link to="/" className="svc-admin-link-muted">
              ← Back
            </Link>
          </div>

          <h1>{pageTitle}</h1>
          <p className="svc-admin-page-subtitle">Detailed status, planes, and metrics for this node.</p>
          <p className="svc-admin-node-meta">
            <span className="svc-admin-node-id">
              <strong>ID:</strong> {(status as any)?.id}
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
                flexWrap: 'wrap',
              }}
            >
              <h2 style={{ margin: 0 }}>Overview</h2>

              <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', alignItems: 'center' }}>
                <Pill tone={planeSummary.ready === planeSummary.total ? 'ok' : 'warn'}>
                  {planeSummary.ready}/{planeSummary.total} Ready
                </Pill>
                {planeSummary.degraded > 0 && <Pill tone="warn">{planeSummary.degraded} Degraded</Pill>}
                {planeSummary.down > 0 && <Pill tone="bad">{planeSummary.down} Down</Pill>}
                <Pill tone="muted">{planeSummary.restarts} Restarts</Pill>

                <div
                  className="svc-admin-card"
                  style={{
                    padding: '0.35rem 0.5rem',
                    borderRadius: 14,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                  }}
                >
                  <label style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
                    <input
                      type="checkbox"
                      checked={tilesLive}
                      onChange={(e) => setTilesLive(e.target.checked)}
                      style={{ transform: 'translateY(1px)' }}
                    />
                    <span style={{ fontSize: '0.9rem', opacity: 0.9 }}>Live tiles</span>
                  </label>

                  <select
                    value={tilesIntervalMs}
                    onChange={(e) => setTilesIntervalMs(Number(e.target.value))}
                    disabled={!tilesLive}
                    style={{
                      borderRadius: 10,
                      padding: '6px 8px',
                      border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
                      background: 'rgba(255,255,255,0.03)',
                      color: 'var(--svc-admin-color-text)',
                    }}
                    title="Refresh interval for the utilization tiles and network accounting panel."
                  >
                    <option value={1000}>1s</option>
                    <option value={2000}>2s</option>
                    <option value={5000}>5s</option>
                    <option value={10000}>10s</option>
                  </select>
                </div>
              </div>
            </div>

            <div
              style={{
                borderRadius: 18,
                padding: 14,
                border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
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

              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                <div
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(2, minmax(220px, 1fr))',
                    gap: '1rem',
                    alignItems: 'stretch',
                  }}
                >
                  <MiniMetricCard title="CPU" source={cpuSource}>
                    <SpeedometerGauge pct={cpuPct} compact facts={cpuFacts} />
                  </MiniMetricCard>

                  <MiniMetricCard title="RAM" source={ramSource}>
                    <ThermometerGauge pct={ramPct} compact facts={ramFacts} />
                  </MiniMetricCard>

                  <MiniMetricCard title="Storage" source={storageSource}>
                    <StorageWaffleGauge pct={storagePct} compact facts={storageFacts} />
                  </MiniMetricCard>

                  <MiniMetricCard title="Bandwidth" source={bandwidthSource}>
                    <BandwidthBarsGauge
                      pct={bandwidthActivityPct}
                      seed={utilSeed}
                      rxBps={rxBps}
                      txBps={txBps}
                      compact
                    />
                  </MiniMetricCard>
                </div>

                <NetAccountingPanel net={net} missing={netMissing} />
              </div>
            </div>
          </section>

          <section className="svc-admin-section svc-admin-section-node-metrics">
            <h2>Facet metrics</h2>
            <p style={{ marginTop: 6, opacity: 0.75 }}>
              Large thumbnail cards (2-up). Search by name or tag. Toggle “Issues only” to surface
              facets with errors/staleness/latency.
            </p>

            <FacetMetricsPanel
              nodeId={nodeId}
              facets={facets}
              loading={facetsLoading}
              error={facetsError}
              historyByFacet={facetHistory}
            />
          </section>

          <section className="svc-admin-section svc-admin-section-node-actions">
            <h2>Actions</h2>
            <p className="svc-admin-node-actions-caption">
              Node actions are gated by server-side config and roles. In dev, svc-admin defaults to
              read-only mode.
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
                    {planes.map((plane: any) => (
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
