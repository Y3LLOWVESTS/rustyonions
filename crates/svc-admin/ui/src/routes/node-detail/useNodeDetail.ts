// crates/svc-admin/ui/src/routes/node-detail/useNodeDetail.ts
//
// WHAT:
//   Single hook that owns NodeDetail data fetching, polling, actions, and dev debug controls.
// WHY:
//   Prevent 1k+ LOC route files by separating orchestration (effects/state) from UI composition.
// INTERACTS:
//   - api/adminClient
//   - dto mirrors in types/admin-api
//   - serviceMap for debug crash mapping
// INVARIANTS:
//   - No overlapping in-flight status/facets polls.
//   - No conditional hooks.
//   - Mutations are derived from ui-config + roles (read-only safe default).

import { useEffect, useRef, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type {
  AdminStatusView,
  FacetMetricsSummary,
  NodeActionResponse,
} from '../../types/admin-api'
import { serviceForPlane } from './serviceMap'

export function useNodeDetail(nodeId: string) {
  const [status, setStatus] = useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] = useState<string | null>(null)

  const [facets, setFacets] = useState<FacetMetricsSummary[] | null>(null)
  const [facetsLoading, setFacetsLoading] = useState(true)
  const [facetsError, setFacetsError] = useState<string | null>(null)

  const [facetHistory, setFacetHistory] = useState<Record<string, number[]>>({})

  const [readOnlyUi, setReadOnlyUi] = useState(true)
  const [roles, setRoles] = useState<string[]>([])
  const [identityError, setIdentityError] = useState<string | null>(null)
  const [identityLoading, setIdentityLoading] = useState(true)

  const [actionInFlight, setActionInFlight] =
    useState<'reload' | 'shutdown' | null>(null)
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  const devDebugEnabled = import.meta.env.DEV
  const [debugPlane, setDebugPlane] = useState<string>('')
  const [debugInFlight, setDebugInFlight] = useState(false)
  const [debugMessage, setDebugMessage] = useState<string | null>(null)
  const [debugError, setDebugError] = useState<string | null>(null)

  const mountedRef = useRef(true)
  const statusInFlightRef = useRef(false)
  const facetsInFlightRef = useRef(false)

  const STATUS_POLL_MS = 5_000
  const FACETS_POLL_MS = 5_000
  const MAX_SPARK_POINTS = 40

  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
    }
  }, [])

  // keep debug default plane seeded when planes arrive
  useEffect(() => {
    const planes: any[] = ((status as any)?.planes as any[]) ?? []
    if (planes.length > 0 && !debugPlane) {
      setDebugPlane(planes[0].name)
    }
  }, [status, debugPlane])

  async function refreshStatus(opts?: { initial?: boolean }) {
    if (!nodeId) return
    if (statusInFlightRef.current) return
    statusInFlightRef.current = true

    const initial = opts?.initial ?? false

    try {
      if (initial) {
        setStatus(null)
        setStatusError(null)
        setStatusLoading(true)
      } else {
        setStatusError(null)
      }

      const data = await adminClient.getNodeStatus(nodeId)
      if (!mountedRef.current) return
      setStatus(data)
    } catch (err) {
      if (!mountedRef.current) return
      setStatusError(
        err instanceof Error ? err.message : 'Failed to load node status.',
      )
    } finally {
      statusInFlightRef.current = false
      if (!mountedRef.current) return
      if (initial) setStatusLoading(false)
    }
  }

  async function refreshFacets(opts?: { initial?: boolean }) {
    if (!nodeId) return
    if (facetsInFlightRef.current) return
    facetsInFlightRef.current = true

    const initial = opts?.initial ?? false

    try {
      if (initial) {
        setFacets(null)
        setFacetsError(null)
        setFacetsLoading(true)
      } else {
        setFacetsError(null)
      }

      const data = await adminClient.getNodeFacetMetrics(nodeId)
      if (!mountedRef.current) return

      setFacets(data)

      setFacetHistory((prev) => {
        const next: Record<string, number[]> = { ...prev }

        for (const facet of data) {
          const key = facet.facet
          const prevSeries = prev[key] ?? []
          const updated = [...prevSeries, facet.rps]
          next[key] =
            updated.length > MAX_SPARK_POINTS
              ? updated.slice(updated.length - MAX_SPARK_POINTS)
              : updated
        }

        return next
      })
    } catch (err) {
      if (!mountedRef.current) return
      setFacetsError(
        err instanceof Error
          ? err.message
          : 'Failed to load facet metrics for this node.',
      )
    } finally {
      facetsInFlightRef.current = false
      if (!mountedRef.current) return
      if (initial) setFacetsLoading(false)
    }
  }

  // status polling
  useEffect(() => {
    if (!nodeId) return

    void refreshStatus({ initial: true })
    const t = window.setInterval(() => void refreshStatus(), STATUS_POLL_MS)
    return () => window.clearInterval(t)
  }, [nodeId])

  // facet polling
  useEffect(() => {
    if (!nodeId) return

    void refreshFacets({ initial: true })
    const t = window.setInterval(() => void refreshFacets(), FACETS_POLL_MS)
    return () => window.clearInterval(t)
  }, [nodeId])

  // identity + ui config (once)
  useEffect(() => {
    let cancelled = false

    setIdentityError(null)
    setIdentityLoading(true)

    ;(async () => {
      try {
        const [uiConfig, me] = await Promise.all([
          adminClient.getUiConfig(),
          adminClient.getMe(),
        ])
        if (cancelled) return
        setReadOnlyUi(uiConfig.readOnly)
        setRoles(me.roles)
      } catch (err) {
        if (cancelled) return
        setIdentityError(
          err instanceof Error
            ? err.message
            : 'Failed to load identity / UI configuration.',
        )
      } finally {
        if (!cancelled) setIdentityLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [])

  const canMutate =
    !readOnlyUi && roles.some((role) => role === 'admin' || role === 'ops')

  async function runAction(kind: 'reload' | 'shutdown') {
    if (!status) return

    setActionInFlight(kind)
    setActionError(null)
    setActionMessage(null)

    try {
      let response: NodeActionResponse
      if (kind === 'reload') {
        response = await adminClient.reloadNode(status.id)
      } else {
        response = await adminClient.shutdownNode(status.id)
      }

      setActionMessage(response.message ?? 'Action completed successfully.')
      await Promise.allSettled([refreshStatus(), refreshFacets()])
    } catch (err) {
      setActionError(
        err instanceof Error
          ? err.message
          : 'Action failed. See logs for more detail.',
      )
    } finally {
      setActionInFlight(null)
    }
  }

  async function runDebugCrash() {
    if (!status || !debugPlane) return

    setDebugInFlight(true)
    setDebugError(null)
    setDebugMessage(null)

    try {
      const service = serviceForPlane(debugPlane)
      const response = await adminClient.debugCrashNode(status.id, service)
      setDebugMessage(
        response.message ?? `Synthetic crash event sent for service "${service}".`,
      )
      await Promise.allSettled([refreshStatus(), refreshFacets()])
    } catch (err) {
      setDebugError(
        err instanceof Error
          ? err.message
          : 'Failed to trigger synthetic crash for this node.',
      )
    } finally {
      setDebugInFlight(false)
    }
  }

  return {
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
  }
}
