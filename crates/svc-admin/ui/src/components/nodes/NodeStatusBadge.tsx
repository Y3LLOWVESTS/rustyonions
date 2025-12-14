// crates/svc-admin/ui/src/components/nodes/NodeStatusBadge.tsx
//
// RO:WHAT  — Small pill badge for overall node health (healthy / degraded / down).
// RO:WHY   — Used on the Nodes list and Node detail header for quick visual status.
// RO:NOTE  — Uses inline styles instead of Tailwind so it works with our current CSS
//            pipeline and looks good in both light and dark themes.

import React, { type CSSProperties } from 'react'

type Props = {
  status: 'healthy' | 'degraded' | 'down'
}

const BASE_STYLE: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  padding: '0 0.6rem',
  height: '1.4rem',
  borderRadius: 9999,
  fontSize: '0.75rem',
  fontWeight: 600,
  textTransform: 'capitalize',
  letterSpacing: '0.03em',
  lineHeight: 1,
}

const VARIANTS: Record<Props['status'], CSSProperties> = {
  healthy: {
    backgroundColor: 'rgba(22, 163, 74, 0.16)', // emerald-ish
    border: '1px solid rgba(34, 197, 94, 0.7)',
    color: 'rgb(134, 239, 172)',
  },
  degraded: {
    backgroundColor: 'rgba(245, 158, 11, 0.16)', // amber
    border: '1px solid rgba(251, 191, 36, 0.7)',
    color: 'rgb(253, 224, 171)',
  },
  down: {
    backgroundColor: 'rgba(248, 113, 113, 0.18)', // red
    border: '1px solid rgba(248, 113, 113, 0.85)',
    color: 'rgb(254, 202, 202)',
  },
}

export function NodeStatusBadge({ status }: Props) {
  const style: CSSProperties = { ...BASE_STYLE, ...VARIANTS[status] }
  return <span style={style}>{status}</span>
}
