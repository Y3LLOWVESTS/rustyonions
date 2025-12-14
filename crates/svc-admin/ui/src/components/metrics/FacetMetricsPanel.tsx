// crates/svc-admin/ui/src/components/metrics/FacetMetricsPanel.tsx
//
// RO:WHAT — Facet metrics panel for a single node.
// RO:WHY  — Show a small, “ops-friendly” view of facet RPS/error/latency
//           with clear loading/error/empty states. This is where operators
//           will look first when a node’s behavior is weird.
// RO:INTERACTS —
//   - adminClient.getNodeFacetMetrics(id)  (via NodeDetailPage)
//   - MetricChart (SVG sparkline + summary)
//   - LoadingSpinner / ErrorBanner / EmptyState
//
// RO:INVARIANTS —
//   - Never throw; always render *something* (even in error).
//   - Keep a clear mental model for operators:
//       loading   → “svc-admin is fetching from node…”
//       error     → “svc-admin can’t reach facet metrics; node/admin plane
//                    or /metrics endpoint may be offline.”
//       no facets → “Node is up but no facet metrics observed (yet).”
//   - This component stays *purely presentational*; all fetching and
//     history accumulation happens in NodeDetailPage.

import React from 'react'
import type { FacetMetricsSummary } from '../../types/admin-api'
import { MetricChart } from './MetricChart'
import { LoadingSpinner } from '../shared/LoadingSpinner'
import { ErrorBanner } from '../shared/ErrorBanner'
import { EmptyState } from '../shared/EmptyState'

type Props = {
  facets: FacetMetricsSummary[] | null
  loading: boolean
  error?: string | null
  // Optional per-facet RPS history (oldest → newest), supplied by the page.
  historyByFacet?: Record<string, number[]>
}

export function FacetMetricsPanel({
  facets,
  loading,
  error,
  historyByFacet,
}: Props) {
  const hasFacets = !!facets && facets.length > 0

  return (
    <section className="svc-admin-section svc-admin-section-node-metrics">
      <header className="svc-admin-section-header">
        <div className="svc-admin-section-title-row">
          <h2 className="svc-admin-section-title">Facet metrics</h2>
        </div>
        <p className="svc-admin-section-subtitle">
          Recent request rate, error rate, and latency percentiles per facet.
        </p>
      </header>

      {loading && (
        <div className="svc-admin-section-body svc-admin-section-body-centered">
          <LoadingSpinner />
          <p className="svc-admin-section-body-note">
            Loading facet metrics from node&hellip;
          </p>
        </div>
      )}

      {!loading && error && (
        <div className="svc-admin-section-body">
          <ErrorBanner
            message={
              // We soften the raw “Request failed: XXX” into something more
              // operator-friendly but keep the raw message for debugging.
              `Failed to load facet metrics from node. The node's admin plane or /metrics endpoint may be offline or refusing connections. (${error})`
            }
          />
          <p className="svc-admin-section-body-note">
            svc-admin will keep retrying on a short interval. If this is your
            dev environment and no node is actually running on the configured
            admin URL, this warning is expected.
          </p>
        </div>
      )}

      {!loading && !error && !hasFacets && (
        <div className="svc-admin-section-body">
          <EmptyState message="No facet metrics observed yet. The node may be starting up or has not emitted facet metrics in the recent sampling window." />
        </div>
      )}

      {!loading && !error && hasFacets && facets && (
        <div className="svc-admin-section-body">
          <div className="svc-admin-metric-chart-grid">
            {facets.map((facet) => {
              const history = historyByFacet?.[facet.facet]

              return (
                <MetricChart
                  key={facet.facet}
                  facet={facet.facet}
                  rps={facet.rps}
                  errorRate={facet.error_rate}
                  p95={facet.p95_latency_ms}
                  p99={facet.p99_latency_ms}
                  history={history}
                />
              )
            })}
          </div>
        </div>
      )}
    </section>
  )
}
