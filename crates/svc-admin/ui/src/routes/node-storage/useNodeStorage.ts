// crates/svc-admin/ui/src/routes/node-storage/useNodeStorage.ts
//
// WHAT:
//   Read-only loader for the node storage inventory.
// WHY:
//   Keeps storage endpoint state outside the route and prevents missing
//   endpoints from becoming fabricated storage or database facts.
// INVARIANTS:
//   - Read-only; no node mutations.
//   - Missing endpoints produce explicit unavailable state.
//   - No deterministic storage, database, or database-detail fallback.
//   - Selected database remains valid as the reported inventory changes.

import { useEffect, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type {
  AdminStatusView,
  DatabaseDetailDto,
  DatabaseEntryDto,
  FacetMetricsSummary,
  StorageSummaryDto,
} from '../../types/admin-api'
import { isMissingEndpoint } from './helpers'

type DataSource = 'live' | 'unavailable'

function messageFor(
  err: unknown,
  missingMessage: string,
  failureMessage: string,
): string {
  if (isMissingEndpoint(err)) {
    return missingMessage
  }

  return err instanceof Error ? err.message : failureMessage
}

export function useNodeStorage(nodeId: string) {
  const [status, setStatus] =
    useState<AdminStatusView | null>(null)
  const [statusLoading, setStatusLoading] = useState(true)
  const [statusError, setStatusError] =
    useState<string | null>(null)

  const [facets, setFacets] =
    useState<FacetMetricsSummary[] | null>(null)
  const [facetsLoading, setFacetsLoading] = useState(true)
  const [facetsError, setFacetsError] =
    useState<string | null>(null)

  const [storage, setStorage] =
    useState<StorageSummaryDto | null>(null)
  const [storageLoading, setStorageLoading] = useState(true)
  const [storageError, setStorageError] =
    useState<string | null>(null)
  const [storageSource, setStorageSource] =
    useState<DataSource>('unavailable')

  const [databases, setDatabases] =
    useState<DatabaseEntryDto[]>([])
  const [dbLoading, setDbLoading] = useState(true)
  const [dbError, setDbError] =
    useState<string | null>(null)
  const [dbSource, setDbSource] =
    useState<DataSource>('unavailable')

  const [selectedDb, setSelectedDb] =
    useState<string | null>(null)
  const [dbDetail, setDbDetail] =
    useState<DatabaseDetailDto | null>(null)
  const [dbDetailLoading, setDbDetailLoading] =
    useState(false)
  const [dbDetailError, setDbDetailError] =
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
    if (!nodeId) {
      setFacets(null)
      setFacetsLoading(false)
      setFacetsError('Missing node id in route.')
      return
    }

    let cancelled = false

    setFacets(null)
    setFacetsLoading(true)
    setFacetsError(null)

    void (async () => {
      try {
        const data =
          await adminClient.getNodeFacetMetrics(nodeId)

        if (!cancelled) {
          setFacets(data)
        }
      } catch (err) {
        if (!cancelled) {
          setFacetsError(
            err instanceof Error
              ? err.message
              : 'Failed to load facet metrics.',
          )
        }
      } finally {
        if (!cancelled) {
          setFacetsLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setStorage(null)
      setStorageSource('unavailable')
      setStorageLoading(false)
      setStorageError('Missing node id in route.')
      return
    }

    let cancelled = false

    setStorage(null)
    setStorageSource('unavailable')
    setStorageLoading(true)
    setStorageError(null)

    void (async () => {
      try {
        const data =
          await adminClient.getNodeStorageSummary(nodeId)

        if (!cancelled) {
          setStorage(data)
          setStorageSource('live')
        }
      } catch (err) {
        if (!cancelled) {
          setStorage(null)
          setStorageSource('unavailable')
          setStorageError(
            messageFor(
              err,
              'Storage summary is not reported by this node.',
              'Failed to load storage summary.',
            ),
          )
        }
      } finally {
        if (!cancelled) {
          setStorageLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId) {
      setDatabases([])
      setDbSource('unavailable')
      setDbLoading(false)
      setDbError('Missing node id in route.')
      return
    }

    let cancelled = false

    setDatabases([])
    setDbSource('unavailable')
    setDbLoading(true)
    setDbError(null)

    void (async () => {
      try {
        const data =
          await adminClient.getNodeStorageDatabases(nodeId)

        if (!cancelled) {
          setDatabases(data)
          setDbSource('live')
        }
      } catch (err) {
        if (!cancelled) {
          setDatabases([])
          setDbSource('unavailable')
          setDbError(
            messageFor(
              err,
              'Database inventory is not reported by this node.',
              'Failed to load database inventory.',
            ),
          )
        }
      } finally {
        if (!cancelled) {
          setDbLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [nodeId])

  useEffect(() => {
    if (!nodeId || databases.length === 0) {
      setSelectedDb(null)
      return
    }

    setSelectedDb((previous) => {
      if (
        previous &&
        databases.some((database) => database.name === previous)
      ) {
        return previous
      }

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
      setDbDetailLoading(false)
      setDbDetailError(null)
      return
    }

    let cancelled = false

    setDbDetail(null)
    setDbDetailLoading(true)
    setDbDetailError(null)

    void (async () => {
      try {
        const data =
          await adminClient.getNodeStorageDatabaseDetail(
            nodeId,
            selectedDb,
          )

        if (!cancelled) {
          setDbDetail(data)
        }
      } catch (err) {
        if (!cancelled) {
          setDbDetail(null)
          setDbDetailError(
            messageFor(
              err,
              'Database detail is not reported by this node.',
              'Failed to load database detail.',
            ),
          )
        }
      } finally {
        if (!cancelled) {
          setDbDetailLoading(false)
        }
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
