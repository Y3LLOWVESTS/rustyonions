// crates/svc-admin/ui/src/routes/node-storage/mock.ts
//
// RO:WHAT — Deterministic mock fallback for storage + DB inventory.
// RO:WHY  — When node endpoints are missing, keep UI usable without lying about raw FS access.
// RO:INVARIANTS —
//   - Deterministic per nodeId.
//   - Read-only.
//   - Curated fields only.

import type { StorageSummaryDto, DatabaseEntryDto, DatabaseDetailDto } from '../../types/admin-api'

export function mockStorageSummary(nodeId: string): StorageSummaryDto {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  const total = 512 * 1024 * 1024 * 1024 // 512 GiB
  const used = (96 + (seed % 220)) * 1024 * 1024 * 1024 // 96..316 GiB
  const clampedUsed = Math.min(used, total - 8 * 1024 * 1024 * 1024)
  const free = total - clampedUsed

  return {
    fsType: 'ext4',
    mount: '/',
    totalBytes: total,
    usedBytes: clampedUsed,
    freeBytes: free,
    ioReadBps: 12_500_000 + (seed % 7_500_000),
    ioWriteBps: 8_500_000 + (seed % 6_000_000),
  }
}

function modeLooksWorldReadable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return last === '4' || last === '5' || last === '6' || last === '7'
}

function modeLooksWorldWritable(mode: string): boolean {
  const last = mode.trim().slice(-1)
  return last === '2' || last === '3' || last === '6' || last === '7'
}

export function mockDatabases(nodeId: string): DatabaseEntryDto[] {
  const seed = Array.from(nodeId).reduce((acc, c) => acc + c.charCodeAt(0), 0)
  const bump = (n: number) => n + (seed % 17) * 1024 * 1024

  const list: DatabaseEntryDto[] = [
    {
      name: 'svc-index.sled',
      engine: 'sled',
      sizeBytes: bump(1_250_000_000),
      mode: '0750',
      owner: 'ron:ron',
      health: 'ok',
      notes: 'Name → ContentId resolution indexes.',
    },
    {
      name: 'svc-storage.cas',
      engine: 'fs-cas',
      sizeBytes: bump(88_500_000_000),
      mode: '0700',
      owner: 'ron:ron',
      health: 'ok',
      notes: 'Content-addressed object store (b3:*).',
    },
    {
      name: 'svc-overlay.sled',
      engine: 'sled',
      sizeBytes: bump(4_800_000_000),
      mode: '0755',
      owner: 'ron:ron',
      health: 'degraded',
      notes: '⚠ world-readable (policy warning).',
    },
  ]

  return list.map((d) => ({
    ...d,
    worldReadable: modeLooksWorldReadable(d.mode),
    worldWritable: modeLooksWorldWritable(d.mode),
  }))
}

export function mockDatabaseDetail(nodeId: string, name: string): DatabaseDetailDto {
  const list = mockDatabases(nodeId)
  const hit = list.find((d) => d.name === name) ?? list[0]
  const warnings: string[] = []

  if (hit.worldReadable) warnings.push('Permissions: database appears world-readable.')
  if (hit.worldWritable) warnings.push('Permissions: database appears world-writable (high risk).')
  if (hit.health !== 'ok') {
    warnings.push('Health: database reports degraded status (investigate I/O or compaction).')
  }

  return {
    name: hit.name,
    engine: hit.engine,
    sizeBytes: hit.sizeBytes,
    mode: hit.mode,
    owner: hit.owner,
    health: hit.health,
    pathAlias: hit.engine === 'fs-cas' ? 'data/cas' : 'data/db',
    fileCount: hit.engine === 'fs-cas' ? 128_400 : 3_200,
    lastCompaction: hit.engine === 'sled' ? '2025-12-12T19:19:00Z' : null,
    approxKeys: hit.engine === 'sled' ? 12_400_000 : null,
    warnings,
  }
}
