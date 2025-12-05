// crates/svc-admin/ui/src/components/nodes/PlaneStatusTable.tsx

import React from 'react'
import type { PlaneStatus } from '../../types/admin-api'
import { NodeStatusBadge } from './NodeStatusBadge'

type Props = {
  planes: PlaneStatus[]
}

/**
 * Tabular view of per-plane health for a node.
 * All health / ready values are derived directly from `AdminStatusView`.
 */
export function PlaneStatusTable({ planes }: Props) {
  if (!planes.length) {
    return <p>No plane status reported by this node yet.</p>
  }

  return (
    <table className="svc-admin-plane-table">
      <thead>
        <tr>
          <th>Plane</th>
          <th>Health</th>
          <th>Ready</th>
          <th>Restarts</th>
        </tr>
      </thead>
      <tbody>
        {planes.map((plane) => (
          <tr key={plane.name}>
            <td>{plane.name}</td>
            <td>
              <NodeStatusBadge status={plane.health} />
            </td>
            <td>{plane.ready ? 'Ready' : 'Not ready'}</td>
            <td>{plane.restart_count}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}
