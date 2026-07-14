// crates/svc-admin/ui/src/routes/node-detail/liveUtilization.ts
//
// WHAT:
//   Route-local hook for live utilization polling.
// WHY:
//   Keeps NodeDetailPage compositional and prevents missing endpoints from
//   becoming fabricated operational telemetry.
// INVARIANTS:
//   - No conditional hooks.
//   - Optional or unreachable endpoints produce unavailable values.
//   - No deterministic CPU, RAM, storage, or bandwidth fallback.
//   - Polling pauses while the browser tab is hidden.

import { useEffect, useMemo, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type {
  StorageSummaryDto,
  SystemSummaryDto,
} from '../../types/admin-api'
import type { TileSource } from './utilization'

function clampPct(p: number): number {
  if (!Number.isFinite(p)) return 0
  return Math.max(0, Math.min(100, p))
}

function computeRamPct(
  system: SystemSummaryDto | null,
): number | null {
  if (!system) return null

  const total = system.ramTotalBytes
  const used = system.ramUsedBytes

  if (
    typeof total !== 'number' ||
    !Number.isFinite(total) ||
    total <= 0 ||
    typeof used !== 'number' ||
    !Number.isFinite(used) ||
    used < 0
  ) {
    return null
  }

  return clampPct((used / total) * 100)
}

function computeStoragePct(
  storage: StorageSummaryDto | null,
): number | null {
  if (!storage) return null

  const total = storage.totalBytes
  const used = storage.usedBytes

  if (
    typeof total !== 'number' ||
    !Number.isFinite(total) ||
    total <= 0 ||
    typeof used !== 'number' ||
    !Number.isFinite(used) ||
    used < 0
  ) {
    return null
  }

  return clampPct((used / total) * 100)
}

export function useLiveUtilization(
  nodeId: string,
  opts: {
    enabled: boolean
    intervalMs: number
  },
) {
  const [system, setSystem] =
    useState<SystemSummaryDto | null>(null)
  const [storage, setStorage] =
    useState<StorageSummaryDto | null>(null)

  useEffect(() => {
    if (!nodeId) {
      setSystem(null)
      setStorage(null)
      return
    }

    let cancelled = false
    let timer: number | null = null

    const shouldPollNow = () =>
      !(
        typeof document !== 'undefined' &&
        document.visibilityState === 'hidden'
      )

    const tick = async () => {
      if (cancelled || !shouldPollNow()) return

      try {
        const nextSystem =
          await adminClient.getNodeSystemSummary(nodeId)

        if (!cancelled) {
          setSystem(nextSystem)
        }
      } catch {
        if (!cancelled) {
          setSystem(null)
        }
      }

      try {
        const nextStorage =
          await adminClient.getNodeStorageSummary(nodeId)

        if (!cancelled) {
          setStorage(nextStorage)
        }
      } catch {
        if (!cancelled) {
          setStorage(null)
        }
      }
    }

    void tick()

    if (opts.enabled) {
      timer = window.setInterval(
        () => void tick(),
        Math.max(500, opts.intervalMs),
      )
    }

    const onVisibilityChange = () => {
      if (
        typeof document !== 'undefined' &&
        document.visibilityState === 'visible'
      ) {
        void tick()
      }
    }

    document.addEventListener?.(
      'visibilitychange',
      onVisibilityChange,
    )

    return () => {
      cancelled = true

      if (timer !== null) {
        window.clearInterval(timer)
      }

      document.removeEventListener?.(
        'visibilitychange',
        onVisibilityChange,
      )
    }
  }, [nodeId, opts.enabled, opts.intervalMs])

  const cpuPct = useMemo<number | null>(() => {
    const value = system?.cpuPercent

    return typeof value === 'number' && Number.isFinite(value)
      ? clampPct(value)
      : null
  }, [system])

  const cpuSource: TileSource =
    cpuPct === null ? 'unavailable' : 'reported'

  const ramPct = useMemo(
    () => computeRamPct(system),
    [system],
  )

  const ramSource: TileSource =
    ramPct === null ? 'unavailable' : 'reported'

  const storagePct = useMemo(
    () => computeStoragePct(storage),
    [storage],
  )

  const storageSource: TileSource =
    storagePct === null ? 'unavailable' : 'reported'

  const rxBps = useMemo<number | null>(() => {
    const value = system?.netRxBps

    return typeof value === 'number' && Number.isFinite(value)
      ? Math.max(0, value)
      : null
  }, [system])

  const txBps = useMemo<number | null>(() => {
    const value = system?.netTxBps

    return typeof value === 'number' && Number.isFinite(value)
      ? Math.max(0, value)
      : null
  }, [system])

  const bandwidthSource: TileSource = useMemo(() => {
    if (!system) return 'unavailable'

    return rxBps !== null || txBps !== null
      ? 'reported'
      : 'warming'
  }, [system, rxBps, txBps])

  const bandwidthActivityPct = useMemo<number | null>(() => {
    if (rxBps === null && txBps === null) {
      return null
    }

    const total = (rxBps ?? 0) + (txBps ?? 0)
    const visualCeiling = 50 * 1024 * 1024

    const normalized =
      Math.log10(1 + Math.min(visualCeiling, total)) /
      Math.log10(1 + visualCeiling)

    return clampPct(normalized * 100)
  }, [rxBps, txBps])

  return {
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
  }
}
