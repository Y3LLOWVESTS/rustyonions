// crates/svc-admin/ui/src/routes/node-detail/serviceMap.ts
//
// WHAT:
//   Map plane name -> service name (debug crash helper).
// WHY:
//   Centralize the mapping so debug tooling stays consistent as planes evolve.

export function serviceForPlane(plane: string): string {
  const trimmed = plane.trim()
  if (!trimmed) return trimmed
  if (trimmed.startsWith('svc-')) return trimmed

  switch (trimmed) {
    case 'gateway':
      return 'svc-gateway'
    case 'storage':
      return 'svc-storage'
    case 'index':
      return 'svc-index'
    case 'mailbox':
      return 'svc-mailbox'
    case 'overlay':
      return 'svc-overlay'
    case 'dht':
      return 'svc-dht'
    default:
      return trimmed
  }
}
