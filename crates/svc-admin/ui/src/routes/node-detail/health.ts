// crates/svc-admin/ui/src/routes/node-detail/health.ts
//
// WHAT:
//   Route-local health derivation helpers.
// WHY:
//   Keep NodeDetailPage compositional; pure logic lives here and is reusable/testable.
// INVARIANTS:
//   - If any plane is down => overall down.
//   - Else if any plane is degraded => overall degraded.
//   - Else healthy (when planes exist).

export type Health = 'healthy' | 'degraded' | 'down'

export function deriveOverallHealth(planes: any[]): Health {
  if (!planes.length) return 'degraded'

  if (planes.some((p) => String(p.health ?? '').toLowerCase() === 'down')) {
    return 'down'
  }

  if (planes.some((p) => String(p.health ?? '').toLowerCase() === 'degraded')) {
    return 'degraded'
  }

  return 'healthy'
}
