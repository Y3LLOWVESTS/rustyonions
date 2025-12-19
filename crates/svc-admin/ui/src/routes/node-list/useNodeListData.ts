// crates/svc-admin/ui/src/routes/node-list/useNodeListData.ts
//
// RO:WHAT — Data hook for NodeListPage (nodes + per-node status + per-node metrics freshness).
// RO:WHY  — Future-proof: polling, refresh buttons, concurrency limits, caching, etc.
// RO:INVARIANTS —
//   - No conditional hooks.
//   - Default selection: first node after initial registry load.
//   - Background loads are concurrency-limited (avoid stampeding nodes).
//   - Cancels state updates on unmount.

import { useEffect, useMemo, useRef, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type {
  AdminStatusView,
  NodeSummary,
  FacetMetricsSummary,
} from '../../types/admin-api'
import type { MetricsHealth } from '../../components/nodes/NodeCard'
import { classifyMetricsHealth } from './helpers'

type NodeStatusState = {
  loading: boolean
  error: string | null
  status: AdminStatusView | null
}

type NodeMetricsState = {
  loading: boolean
  error: string | null
  health: MetricsHealth | null
}

async function runWithConcurrency<T>(
  items: T[],
  limit: number,
  worker: (item: T) => Promise<void>,
) {
  const n = Math.max(1, Math.floor(limit))
  const queue = items.slice()
  const runners = Array.from({ length: Math.min(n, queue.length) }).map(async () => {
    while (queue.length > 0) {
      const item = queue.shift()
      if (item === undefined) return
      await worker(item)
    }
  })
  await Promise.all(runners)
}

export function useNodeListData(opts?: { concurrency?: number }) {
  const concurrency = opts?.concurrency ?? 4

  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const [statusById, setStatusById] = useState<Record<string, NodeStatusState>>({})
  const [metricsById, setMetricsById] = useState<Record<string, NodeMetricsState>>({})

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)

  const mountedRef = useRef(true)
  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
    }
  }, [])

  // --- Load node registry (once) -------------------------------------------

  useEffect(() => {
    let cancelled = false

    setLoading(true)
    setError(null)

    ;(async () => {
      try {
        const data = await adminClient.getNodes()
        if (cancelled || !mountedRef.current) return

        setNodes(data)

        // Default selection: first node in the list (only once on initial load).
        setSelectedNodeId((prev) => {
          if (prev) return prev
          return data.length > 0 ? data[0].id : null
        })
      } catch (err) {
        if (cancelled || !mountedRef.current) return
        const msg = err instanceof Error ? err.message : 'Failed to load nodes.'
        setError(msg)
      } finally {
        if (!cancelled && mountedRef.current) setLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [])

  // --- Load per-node status in background ----------------------------------

  useEffect(() => {
    if (!nodes.length) return

    let cancelled = false

    ;(async () => {
      // Mark all nodes as "loading" if we don't already have a state.
      setStatusById((prev) => {
        const next = { ...prev }
        for (const n of nodes) {
          if (!next[n.id]) next[n.id] = { loading: true, error: null, status: null }
        }
        return next
      })

      await runWithConcurrency(nodes, concurrency, async (node) => {
        const id = node.id
        try {
          const data = await adminClient.getNodeStatus(id)
          if (cancelled || !mountedRef.current) return

          setStatusById((prev) => ({
            ...prev,
            [id]: { loading: false, error: null, status: data },
          }))
        } catch (err) {
          if (cancelled || !mountedRef.current) return
          const msg = err instanceof Error ? err.message : 'Failed to load node status'
          setStatusById((prev) => ({
            ...prev,
            [id]: { loading: false, error: msg, status: null },
          }))
        }
      })
    })()

    return () => {
      cancelled = true
    }
  }, [nodes, concurrency])

  // --- Load per-node metrics freshness in background ------------------------

  useEffect(() => {
    if (!nodes.length) return

    let cancelled = false

    ;(async () => {
      setMetricsById((prev) => {
        const next = { ...prev }
        for (const n of nodes) {
          if (!next[n.id]) next[n.id] = { loading: true, error: null, health: null }
        }
        return next
      })

      await runWithConcurrency(nodes, concurrency, async (node) => {
        const id = node.id
        try {
          const data: FacetMetricsSummary[] = await adminClient.getNodeFacetMetrics(id)
          if (cancelled || !mountedRef.current) return

          const health = classifyMetricsHealth(data, null)

          setMetricsById((prev) => ({
            ...prev,
            [id]: { loading: false, error: null, health },
          }))
        } catch (err) {
          if (cancelled || !mountedRef.current) return
          const msg = err instanceof Error ? err.message : 'Failed to load facet metrics'
          const health: MetricsHealth = 'unreachable'

          setMetricsById((prev) => ({
            ...prev,
            [id]: { loading: false, error: msg, health },
          }))
        }
      })
    })()

    return () => {
      cancelled = true
    }
  }, [nodes, concurrency])

  // --- Derived selections ---------------------------------------------------

  const effectiveSelectedId = useMemo(() => {
    return selectedNodeId || (nodes.length > 0 ? nodes[0].id : null)
  }, [selectedNodeId, nodes])

  const selectedNode = useMemo(() => {
    if (!effectiveSelectedId) return null
    return nodes.find((n) => n.id === effectiveSelectedId) ?? null
  }, [effectiveSelectedId, nodes])

  const selectedStatusState = selectedNode ? statusById[selectedNode.id] : undefined
  const selectedMetricsState = selectedNode ? metricsById[selectedNode.id] : undefined

  return {
    nodes,
    loading,
    error,

    statusById,
    metricsById,

    selectedNodeId,
    setSelectedNodeId,

    effectiveSelectedId,
    selectedNode,
    selectedStatusState,
    selectedMetricsState,
  }
}
