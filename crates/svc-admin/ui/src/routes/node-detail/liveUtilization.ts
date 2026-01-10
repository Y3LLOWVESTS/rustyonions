// crates/svc-admin/ui/src/routes/node-detail/liveUtilization.ts
//
// WHAT:
//   Route-local hook for live utilization polling (system + storage), with safe fallbacks.
// WHY:
//   Keeps NodeDetailPage compositional and avoids 1k+ LOC route files.
// INVARIANTS:
//   - No conditional hooks.
//   - Optional endpoints: if 404/405/501, treat as capability-missing and fall back to mocks.

import { useEffect, useMemo, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type { StorageSummaryDto, SystemSummaryDto } from '../../types/admin-api'
import { mockNodeUtilization, type TileSource } from './utilization'

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
  const total = (sys as any).ramTotalBytes
  const used = (sys as any).ramUsedBytes
  if (!Number.isFinite(total) || total <= 0) return null
  const pct = (Math.max(0, used) / total) * 100
  return clampPct(pct)
}

function computeStoragePct(st: StorageSummaryDto | null): number | null {
  if (!st) return null
  const total = (st as any).totalBytes
  const used = (st as any).usedBytes
  if (!Number.isFinite(total) || total <= 0) return null
  const pct = (Math.max(0, used) / total) * 100
  return clampPct(pct)
}

/**
 * Live utilization polling:
 * - system summary: optional endpoint
 * - storage summary: optional endpoint
 * - polling controlled by (enabled, intervalMs)
 * - pauses when tab hidden
 */
export function useLiveUtilization(nodeId: string, opts: { enabled: boolean; intervalMs: number }) {
  const [system, setSystem] = useState<SystemSummaryDto | null>(null)
  const [storage, setStorage] = useState<StorageSummaryDto | null>(null)

  useEffect(() => {
    if (!nodeId) {
      setSystem(null)
      setStorage(null)
      return
    }

    let cancelled = false
    let timer: number | null = null

    const shouldPollNow = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'hidden') return false
      return true
    }

    const tick = async () => {
      if (cancelled) return
      if (!shouldPollNow()) return

      try {
        const s = await adminClient.getNodeSystemSummary(nodeId)
        if (!cancelled) setSystem(s)
      } catch (err) {
        if (cancelled) return
        if (isMissingEndpoint(err)) setSystem(null)
      }

      try {
        const st = await adminClient.getNodeStorageSummary(nodeId)
        if (!cancelled) setStorage(st)
      } catch (err) {
        if (cancelled) return
        if (isMissingEndpoint(err)) setStorage(null)
      }
    }

    const start = () => {
      void tick()
      if (opts.enabled) {
        timer = window.setInterval(() => void tick(), Math.max(500, opts.intervalMs))
      }
    }

    start()

    const onVis = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'visible') void tick()
    }
    document.addEventListener?.('visibilitychange', onVis)

    return () => {
      cancelled = true
      if (timer != null) window.clearInterval(timer)
      document.removeEventListener?.('visibilitychange', onVis)
    }
  }, [nodeId, opts.enabled, opts.intervalMs])

  const mock = useMemo(() => mockNodeUtilization(nodeId), [nodeId])

  const cpuPct = useMemo(() => {
    const live = (system as any)?.cpuPercent
    if (typeof live === 'number' && Number.isFinite(live)) return clampPct(live)
    return mock.cpuPct
  }, [system, mock])
  const cpuSource: TileSource = useMemo(() => {
    const live = (system as any)?.cpuPercent
    if (typeof live === 'number' && Number.isFinite(live)) return 'reported'
    return 'mock'
  }, [system])

  const ramPct = useMemo(() => {
    const live = computeRamPct(system)
    if (live != null) return live
    return mock.ramPct
  }, [system, mock])
  const ramSource: TileSource = useMemo(() => {
    const live = computeRamPct(system)
    return live != null ? 'reported' : 'mock'
  }, [system])

  const storagePct = useMemo(() => {
    const live = computeStoragePct(storage)
    if (live != null) return live
    return mock.storagePct
  }, [storage, mock])
  const storageSource: TileSource = useMemo(() => {
    const live = computeStoragePct(storage)
    return live != null ? 'reported' : 'mock'
  }, [storage])

  const rxBps = useMemo(() => {
    const v = (system as any)?.netRxBps
    return typeof v === 'number' && Number.isFinite(v) ? Math.max(0, v) : null
  }, [system])
  const txBps = useMemo(() => {
    const v = (system as any)?.netTxBps
    return typeof v === 'number' && Number.isFinite(v) ? Math.max(0, v) : null
  }, [system])

  const bandwidthSource: TileSource = useMemo(() => {
    if (!system) return 'mock'
    const hasAny = rxBps != null || txBps != null
    return hasAny ? 'reported' : 'warming'
  }, [system, rxBps, txBps])

  const bandwidthActivityPct = useMemo(() => {
    if (!system) return mock.bandwidthPct
    const r = rxBps ?? 0
    const t = txBps ?? 0
    const total = r + t
    const max = 50 * 1024 * 1024
    const v = Math.log10(1 + Math.min(max, total)) / Math.log10(1 + max)
    return clampPct(v * 100)
  }, [system, rxBps, txBps, mock.bandwidthPct])

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
