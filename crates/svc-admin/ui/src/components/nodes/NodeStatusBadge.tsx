import React from 'react'

type Props = {
  status: 'healthy' | 'degraded' | 'down'
}

export function NodeStatusBadge({ status }: Props) {
  return (
    <span className={`svc-admin-node-status svc-admin-node-status-${status}`}>
      {status}
    </span>
  )
}
