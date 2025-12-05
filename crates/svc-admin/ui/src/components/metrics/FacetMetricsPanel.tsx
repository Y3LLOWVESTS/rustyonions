// crates/svc-admin/ui/src/components/metrics/FacetMetricsPanel.tsx

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
}

/**
 * Node-scoped facet metrics panel.
 *
 * The NodeDetail page controls fetching; this component focuses on
 * rendering the state (loading, error, or metrics).
 */
export function FacetMetricsPanel({ facets, loading, error }: Props) {
  return (
    <section className="svc-admin-panel svc-admin-panel-metrics">
      <header className="svc-admin-panel-header">
        <h2>Facet Metrics</h2>
        <p>Recent traffic and errors per facet on this node.</p>
      </header>

      {loading && (
        <div className="svc-admin-panel-body">
          <LoadingSpinner />
        </div>
      )}

      {!loading && error && (
        <div className="svc-admin-panel-body">
          <ErrorBanner message={error} />
        </div>
      )}

      {!loading && !error && (!facets || facets.length === 0) && (
        <div className="svc-admin-panel-body">
          <EmptyState message="No facet metrics yet. Check that this node exports RON facet metrics on /metrics." />
        </div>
      )}

      {!loading && !error && facets && facets.length > 0 && (
        <div className="svc-admin-panel-body svc-admin-metric-list">
          {facets.map((facet) => (
            <MetricChart
              key={facet.facet}
              facet={facet.facet}
              rps={facet.rps}
              errorRate={facet.error_rate}
              p95={facet.p95_latency_ms}
              p99={facet.p99_latency_ms}
            />
          ))}
        </div>
      )}
    </section>
  )
}
