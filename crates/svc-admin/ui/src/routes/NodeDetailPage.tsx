// crates/svc-admin/ui/src/routes/NodeDetailPage.tsx
//
// RO:WHAT — Detail page for a single node (status, planes, metrics, actions).
// RO:WHY  — Give operators a single place to understand and (optionally)
//           control a node, while keeping read-only as the default.
// RO:INTERACTS — api/adminClient.ts, PlaneStatusTable, NodeStatusBadge,
//                FacetMetricsPanel, i18n provider.
// RO:INVARIANTS — All mutating actions are gated by UiConfig.readOnly + roles
//                 and again on the backend (ActionsCfg + AuthCfg).

import React, { useEffect, useMemo, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { adminClient } from '../api/adminClient'
import type {
  AdminStatusView,
  FacetMetricsSummary,
  PlaneStatus,
  NodeActionResponse,
  UiConfigDto,
  MeResponse
} from '../types/admin-api'
import { PlaneStatusTable } from '../components/nodes/PlaneStatusTable'
import { NodeStatusBadge } from '../components/nodes/NodeStatusBadge'
import { FacetMetricsPanel } from '../components/metrics/FacetMetricsPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { useI18n } from '../i18n/useI18n'

type Health = 'healthy' | 'degraded' | 'down'

function deriveOverallHealth(planes: PlaneStatus[]): Health {
  if (!planes.length) return 'degraded'
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
}

export function NodeDetailPage() {
  const { t } = useI18n()
  const params = useParams<{ id: string }>()
  const nodeId = params.id ?? ''

  // --- Node status --------------------------------------------------------

  const [status, setStatus] = useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] = useState<string | null>(null)

  // --- Facet metrics ------------------------------------------------------

  const [facets, setFacets] = useState<FacetMetricsSummary[] | null>(null)
  const [facetsLoading, setFacetsLoading] = useState(true)
  const [facetsError, setFacetsError] = useState<string | null>(null)

  // --- Identity + UI config (for action gating) ---------------------------

  const [readOnlyUi, setReadOnlyUi] = useState(true)
  const [roles, setRoles] = useState<string[]>([])
  const [identityError, setIdentityError] = useState<string | null>(null)
  const [identityLoading, setIdentityLoading] = useState(true)

  // --- Action state -------------------------------------------------------

  const [actionInFlight, setActionInFlight] =
    useState<'reload' | 'shutdown' | null>(null)
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  const overallHealth: Health | null = useMemo(() => {
    if (!status) return null
    return deriveOverallHealth(status.planes)
  }, [status])

  const pageTitle = status?.display_name ?? nodeId

  // Fetch node status whenever id changes.
  useEffect(() => {
    if (!nodeId) return

    let cancelled = false

    setStatus(null)
    setStatusError(null)
    setStatusLoading(true)

    adminClient
      .getNodeStatus(nodeId)
      .then((data) => {
        if (cancelled) return
        setStatus(data)
      })
      .catch((err: unknown) => {
        if (cancelled) return
        const msg =
          err instanceof Error ? err.message : 'Failed to load node status'
        setStatusError(msg)
      })
      .finally(() => {
        if (!cancelled) {
          setStatusLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [nodeId])

  // Fetch facet metrics whenever id changes.
  useEffect(() => {
    if (!nodeId) return

    let cancelled = false

    setFacets(null)
    setFacetsError(null)
    setFacetsLoading(true)

    adminClient
      .getNodeFacetMetrics(nodeId)
      .then((data) => {
        if (cancelled) return
        setFacets(data)
      })
      .catch((err: unknown) => {
        if (cancelled) return
        const msg =
          err instanceof Error ? err.message : 'Failed to load facet metrics'
        setFacetsError(msg)
      })
      .finally(() => {
        if (!cancelled) {
          setFacetsLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [nodeId])

  // Fetch UiConfig + Me once, to know read-only + roles.
  useEffect(() => {
    let cancelled = false

    setIdentityLoading(true)
    setIdentityError(null)

    Promise.all<[UiConfigDto, MeResponse]>([
      adminClient.getUiConfig(),
      adminClient.getMe()
    ])
      .then(([cfg, me]) => {
        if (cancelled) return
        setReadOnlyUi(cfg.readOnly)
        setRoles(me.roles ?? [])
      })
      .catch((err: unknown) => {
        if (cancelled) return
        const msg =
          err instanceof Error ? err.message : 'Failed to load identity'
        setIdentityError(msg)
      })
      .finally(() => {
        if (!cancelled) {
          setIdentityLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [])

  const canAct = useMemo(
    () =>
      !readOnlyUi &&
      roles.some((r) => r === 'admin' || r === 'ops'),
    [readOnlyUi, roles]
  )

  const showActionsSection =
    !!status && !identityLoading && !identityError

  async function runAction(kind: 'reload' | 'shutdown') {
    if (!status) return

    setActionError(null)
    setActionMessage(null)
    setActionInFlight(kind)

    try {
      let resp: NodeActionResponse

      if (kind === 'reload') {
        resp = await adminClient.reloadNode(status.node_id)
      } else {
        resp = await adminClient.shutdownNode(status.node_id)
      }

      if (!resp.accepted) {
        setActionError(resp.message ?? t('node.actions.genericError'))
      } else {
        const fallback =
          kind === 'reload'
            ? t('node.actions.reloadAccepted')
            : t('node.actions.shutdownAccepted')
        setActionMessage(resp.message ?? fallback)
      }
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : t('node.actions.genericError')
      setActionError(msg)
    } finally {
      setActionInFlight(null)
    }
  }

  return (
    <div className="svc-admin-page svc-admin-page-node">
      <header className="svc-admin-page-header svc-admin-page-header-node">
        <div>
          <h1>{pageTitle}</h1>
          {status && (
            <p className="svc-admin-node-meta">
              <span className="svc-admin-node-id">ID: {status.node_id}</span>{' '}
              <span className="svc-admin-node-profile">
                Profile: {status.profile}
              </span>{' '}
              <span className="svc-admin-node-version">
                Version: {status.version}
              </span>
            </p>
          )}
        </div>
        <div className="svc-admin-page-header-actions">
          <Link to="/" className="svc-admin-link-muted">
            ← {t('nav.nodes')}
          </Link>
          {overallHealth && <NodeStatusBadge status={overallHealth} />}
        </div>
      </header>

      {statusLoading && (
        <div className="svc-admin-section">
          <LoadingSpinner />
        </div>
      )}

      {statusError && (
        <div className="svc-admin-section">
          <ErrorBanner message={statusError} />
        </div>
      )}

      {!statusLoading && !statusError && status && (
        <>
          <section className="svc-admin-section svc-admin-section-node-overview">
            <h2>Planes</h2>
            <PlaneStatusTable planes={status.planes} />
          </section>

          {showActionsSection && (
            <section className="svc-admin-section svc-admin-section-node-actions">
              <h2>{t('node.actions.title')}</h2>

              {identityError && (
                <ErrorBanner message={identityError} />
              )}

              {!identityError && !canAct && (
                <p className="svc-admin-node-actions-help">
                  {readOnlyUi
                    ? t('node.actions.readOnlyHelp')
                    : t('node.actions.insufficientRole')}
                </p>
              )}

              <div className="svc-admin-node-actions-buttons">
                <button
                  type="button"
                  className="svc-admin-node-action-button"
                  disabled={!canAct || actionInFlight === 'reload'}
                  onClick={() => runAction('reload')}
                >
                  {actionInFlight === 'reload'
                    ? t('node.actions.reloadInProgress')
                    : t('node.actions.reload')}
                </button>

                <button
                  type="button"
                  className="svc-admin-node-action-button svc-admin-node-action-button-danger"
                  disabled={!canAct || actionInFlight === 'shutdown'}
                  onClick={() => runAction('shutdown')}
                >
                  {actionInFlight === 'shutdown'
                    ? t('node.actions.shutdownInProgress')
                    : t('node.actions.shutdown')}
                </button>
              </div>

              {actionMessage && (
                <p className="svc-admin-node-actions-message">
                  {actionMessage}
                </p>
              )}

              {actionError && (
                <ErrorBanner message={actionError} />
              )}
            </section>
          )}
        </>
      )}

      <section className="svc-admin-section svc-admin-section-node-metrics">
        <FacetMetricsPanel
          facets={facets}
          loading={facetsLoading}
          error={facetsError}
        />
      </section>
    </div>
  )
}
