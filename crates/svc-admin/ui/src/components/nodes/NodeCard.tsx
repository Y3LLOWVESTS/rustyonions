// crates/svc-admin/ui/src/components/nodes/NodeCard.tsx

import React from 'react'
import { Link } from 'react-router-dom'
import type { NodeSummary } from '../../types/admin-api'

type Props = {
  node: NodeSummary
}

/**
 * Compact card used on the Nodes overview page.
 * Clicking it navigates to `/nodes/:id` for the selected node.
 */
export function NodeCard({ node }: Props) {
  return (
    <Link
      to={`/nodes/${encodeURIComponent(node.id)}`}
      className="svc-admin-node-card"
    >
      <h3 className="svc-admin-node-title">{node.display_name}</h3>
      <p className="svc-admin-node-subtitle">
        <span className="svc-admin-node-label">Profile:</span>{' '}
        <span className="svc-admin-node-profile">{node.profile}</span>
      </p>
    </Link>
  )
}
