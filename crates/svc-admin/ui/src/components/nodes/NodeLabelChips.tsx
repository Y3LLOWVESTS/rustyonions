/**
 * RO:WHAT — Renders node labels as compact chips (key=value).
 * RO:WHY — DX. Makes labels visible for cluster operations (grouping/filtering at a glance).
 * RO:INTERACTS — NodeListPage, NodeLabelsEditor.
 * RO:INVARIANTS — stable ordering; never throws on empty/undefined.
 */

import React from 'react'
import type { NodeLabels } from '../../lib/nodeLabelsStore'

export function NodeLabelChips(props: { labels: NodeLabels; max?: number }) {
  const { labels, max = 6 } = props
  const entries = Object.entries(labels ?? {}).sort(([a], [b]) => a.localeCompare(b))

  if (entries.length === 0) return null

  const shown = entries.slice(0, max)
  const remaining = entries.length - shown.length

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 6 }}>
      {shown.map(([k, v]) => (
        <span
          key={k}
          title={`${k}=${v}`}
          style={{
            fontSize: 12,
            padding: '2px 8px',
            borderRadius: 999,
            border: '1px solid rgba(255,255,255,0.12)',
            background: 'rgba(255,255,255,0.04)',
            lineHeight: 1.6,
          }}
        >
          <span style={{ opacity: 0.75 }}>{k}</span>
          <span style={{ opacity: 0.55, padding: '0 4px' }}>=</span>
          <span>{v}</span>
        </span>
      ))}
      {remaining > 0 && (
        <span
          style={{
            fontSize: 12,
            padding: '2px 8px',
            borderRadius: 999,
            border: '1px solid rgba(255,255,255,0.12)',
            opacity: 0.75,
          }}
          title={`${remaining} more labels`}
        >
          +{remaining}
        </span>
      )}
    </div>
  )
}

export default NodeLabelChips
