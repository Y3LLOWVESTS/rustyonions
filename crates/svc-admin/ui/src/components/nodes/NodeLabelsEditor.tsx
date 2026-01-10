/**
 * RO:WHAT — Inline editor to add/remove node labels (key/value).
 * RO:WHY — DX/RES. Operator-grade node categorization for large clusters.
 * RO:INTERACTS — nodeLabelsStore; used by NodeListPage selection panel.
 * RO:INVARIANTS — keys lowercased/trimmed; empty keys/vals rejected; optimistic UI.
 */

import React from 'react'
import {
  clearNodeLabels,
  getNodeLabels,
  removeNodeLabel,
  subscribeNodeLabels,
  upsertNodeLabel,
} from '../../lib/nodeLabelsStore'
import type { NodeId, NodeLabels } from '../../lib/nodeLabelsStore'

function useNodeLabels(nodeId: NodeId | null): NodeLabels {
  const [labels, setLabels] = React.useState<NodeLabels>(() => (nodeId ? getNodeLabels(nodeId) : {}))

  React.useEffect(() => {
    if (!nodeId) {
      setLabels({})
      return
    }
    setLabels(getNodeLabels(nodeId))
    return subscribeNodeLabels(() => setLabels(getNodeLabels(nodeId)))
  }, [nodeId])

  return labels
}

export function NodeLabelsEditor(props: { nodeId: NodeId | null }) {
  const { nodeId } = props
  const labels = useNodeLabels(nodeId)

  const [k, setK] = React.useState('')
  const [v, setV] = React.useState('')
  const [err, setErr] = React.useState<string | null>(null)

  const entries = Object.entries(labels).sort(([a], [b]) => a.localeCompare(b))

  function onAdd() {
    if (!nodeId) return
    const key = k.trim()
    const val = v.trim()
    if (!key || !val) {
      setErr('Key and value are required.')
      return
    }
    setErr(null)
    upsertNodeLabel(nodeId, key, val)
    setK('')
    setV('')
  }

  if (!nodeId) {
    return (
      <div style={{ padding: 12, border: '1px solid rgba(255,255,255,0.10)', borderRadius: 12 }}>
        <div style={{ fontWeight: 600, marginBottom: 6 }}>Tags</div>
        <div style={{ opacity: 0.75, fontSize: 13 }}>Select a node to view/edit tags.</div>
      </div>
    )
  }

  return (
    <div style={{ padding: 12, border: '1px solid rgba(255,255,255,0.10)', borderRadius: 12 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ fontWeight: 600 }}>Tags</div>
        <button
          type="button"
          onClick={() => clearNodeLabels(nodeId)}
          style={{
            fontSize: 12,
            padding: '6px 10px',
            borderRadius: 10,
            border: '1px solid rgba(255,255,255,0.14)',
            background: 'transparent',
            cursor: 'pointer',
          }}
          title="Clear all tags for this node"
        >
          Clear
        </button>
      </div>

      {entries.length === 0 ? (
        <div style={{ opacity: 0.75, fontSize: 13, marginTop: 8 }}>No tags yet.</div>
      ) : (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 10 }}>
          {entries.map(([key, val]) => (
            <span
              key={key}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 8,
                fontSize: 12,
                padding: '4px 10px',
                borderRadius: 999,
                border: '1px solid rgba(255,255,255,0.12)',
                background: 'rgba(255,255,255,0.04)',
              }}
              title={`${key}=${val}`}
            >
              <span style={{ opacity: 0.75 }}>{key}</span>
              <span style={{ opacity: 0.55 }}>=</span>
              <span>{val}</span>
              <button
                type="button"
                onClick={() => removeNodeLabel(nodeId, key)}
                style={{
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  opacity: 0.75,
                  fontSize: 14,
                  lineHeight: 1,
                }}
                aria-label={`Remove ${key}`}
                title="Remove"
              >
                ×
              </button>
            </span>
          ))}
        </div>
      )}

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr auto', gap: 8, marginTop: 12 }}>
        <input
          value={k}
          onChange={(e) => setK(e.target.value)}
          placeholder="key (e.g., env)"
          style={{
            padding: '8px 10px',
            borderRadius: 10,
            border: '1px solid rgba(255,255,255,0.12)',
            background: 'rgba(0,0,0,0.15)',
            color: 'inherit',
          }}
        />
        <input
          value={v}
          onChange={(e) => setV(e.target.value)}
          placeholder="value (e.g., prod)"
          style={{
            padding: '8px 10px',
            borderRadius: 10,
            border: '1px solid rgba(255,255,255,0.12)',
            background: 'rgba(0,0,0,0.15)',
            color: 'inherit',
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') onAdd()
          }}
        />
        <button
          type="button"
          onClick={onAdd}
          style={{
            padding: '8px 12px',
            borderRadius: 10,
            border: '1px solid rgba(255,255,255,0.14)',
            background: 'rgba(255,255,255,0.06)',
            cursor: 'pointer',
            fontWeight: 600,
          }}
        >
          Add
        </button>
      </div>

      {err && <div style={{ marginTop: 8, color: '#ff6b6b', fontSize: 13 }}>{err}</div>}

      <div style={{ marginTop: 10, opacity: 0.7, fontSize: 12 }}>
        Tip: use stable keys like <code>env</code>, <code>region</code>, <code>team</code>, <code>tier</code>.
      </div>
    </div>
  )
}

export default NodeLabelsEditor
