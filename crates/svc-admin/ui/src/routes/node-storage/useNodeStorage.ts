// crates/svc-admin/ui/src/routes/node-storage/useNodeStorage.ts
//
// RO:WHAT — Hook for NodeStoragePage (fetching + optional endpoints + safe fallbacks).
// RO:WHY  — Prevent large route files; keep all effects/state in one place.
// RO:INVARIANTS —
//   - Read-only: no mutations.
//   - Missing endpoints (404/405/501) => deterministic mock data.
//   - No conditional hooks; effects are stable.
//   - Selected DB is kept valid as DB list changes.

import { useEffect, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type {
  AdminStatusView,
  FacetMetricsSummary,
  StorageSummaryDto,
  DatabaseEntryDto,
  DatabaseDetailDto,
} from '../../types/admin-api'
import { isMissingEndpoint } from './helpers'
import { mockStorageSummary, mockDatabases, mockDatabaseDetail } from './mock'

type DataSource = 'live' | 'mock'

export function useNodeStorage(nodeId: string) {
  // --- status / metrics ----------------------------------------------------
  const [status, setStatus] = useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] = useState<string | null>(null)

  const [facets, setFacets] = useState<FacetMetricsSummary[] | null>(null)
  const [facetsLoading, setFacetsLoading] = useState(true)
  const [facetsError, setFacetsError] = useState<string | null>(null)

  // --- storage/db endpoints (optional) -------------------------------------
  const [storage, setStorage] = useState<StorageSummaryDto | null>(null)
  const [storageLoading, setStorageLoading] = useState(true)
  const [storageError, setStorageError] = useState<string | null>(null)
  const [storageSource, setStorageSource] = useState<DataSource>('mock')

  const [databases, setDatabases] = useState<DatabaseEntryDto[]>([])
  const [dbLoading, setDbLoading] = useState(true)
  const [dbError, setDbError] = useState<string | null>(null)
  const [dbSource, setDbSource] = useState<DataSource>('mock')

  const [selectedDb, setSelectedDb] = useState<string | null>(null)
  const [dbDetail, setDbDetail] = useState<DatabaseDetailDto | null>(null)
  const [dbDetailLoading, setDbDetailLoading] = useState(false)
  const [dbDetailError, setDbDetailError] = useState<string | null>(null)

  // ------------------------ effects ----------------------------------------

  useEffect(() => {
    if (!nodeId) {
      setStatus(null)
      setStatusLoading(false)
      setStatusError('Missing node id in route.')
      return
    }

    let cancelled = false
    setStatusLoading(true)
    setStatusError(null)

    ;(async () => {
      try {
        const data = await adminClient.getNodeStatus(nodeId)
        if (cancelled) return
        setStatus(data)
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load node status.'
        setStatusError(msg)
      } finally {
        if (!cancelled) setStatusLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setFacets(null)
      setFacetsLoading(false)
      setFacetsError('Missing node id in route.')
      return
    }

    let cancelled = false
    setFacetsLoading(true)
    setFacetsError(null)

    ;(async () => {
      try {
        const data = await adminClient.getNodeFacetMetrics(nodeId)
        if (cancelled) return
        setFacets(data)
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load facet metrics.'
        setFacetsError(msg)
      } finally {
        if (!cancelled) setFacetsLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setStorage(mockStorageSummary(''))
      setStorageSource('mock')
      setStorageLoading(false)
      setStorageError('Missing node id in route.')
      return
    }

    let cancelled = false
    setStorageLoading(true)
    setStorageError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeStorageSummary(nodeId)
        if (cancelled) return
        setStorage(live)
        setStorageSource('live')
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          setStorage(mockStorageSummary(nodeId))
          setStorageSource('mock')
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load storage summary.'
          setStorageError(msg)
          setStorage(mockStorageSummary(nodeId))
          setStorageSource('mock')
        }
      } finally {
        if (!cancelled) setStorageLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setDatabases(mockDatabases(''))
      setDbSource('mock')
      setDbLoading(false)
      setDbError('Missing node id in route.')
      return
    }

    let cancelled = false
    setDbLoading(true)
    setDbError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeDatabases(nodeId)
        if (cancelled) return
        setDatabases(live)
        setDbSource('live')
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          setDatabases(mockDatabases(nodeId))
          setDbSource('mock')
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load databases.'
          setDbError(msg)
          setDatabases(mockDatabases(nodeId))
          setDbSource('mock')
        }
      } finally {
        if (!cancelled) setDbLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setSelectedDb(null)
      return
    }

    if (!databases || databases.length === 0) {
      setSelectedDb(null)
      return
    }

    setSelectedDb((prev) => {
      if (prev && databases.some((d) => d.name === prev)) return prev
      return databases[0].name
    })
  }, [nodeId, databases])

  useEffect(() => {
    if (!nodeId) {
      setDbDetail(null)
      setDbDetailLoading(false)
      setDbDetailError('Missing node id in route.')
      return
    }

    if (!selectedDb) {
      setDbDetail(null)
      setDbDetailError(null)
      setDbDetailLoading(false)
      return
    }

    let cancelled = false
    setDbDetailLoading(true)
    setDbDetailError(null)

    ;(async () => {
      try {
        const live = await adminClient.getNodeDatabaseDetail(nodeId, selectedDb)
        if (cancelled) return
        setDbDetail(live)
      } catch (err) {
        if (cancelled) return

        if (isMissingEndpoint(err)) {
          setDbDetail(mockDatabaseDetail(nodeId, selectedDb))
        } else {
          const msg = err instanceof Error ? err.message : 'Failed to load database detail.'
          setDbDetailError(msg)
          setDbDetail(mockDatabaseDetail(nodeId, selectedDb))
        }
      } finally {
        if (!cancelled) setDbDetailLoading(false)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId, selectedDb])

  return {
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
  }
}
