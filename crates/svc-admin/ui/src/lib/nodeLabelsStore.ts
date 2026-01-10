/**
 * RO:WHAT — Local node label (tag) store with subscribe/notify for the Nodes dashboard.
 * RO:WHY — DX/RES. Enables cluster-grade filtering/grouping without blocking on backend wiring.
 * RO:INTERACTS — NodeListPage, NodeLabelsEditor; storage: window.localStorage.
 * RO:INVARIANTS — keys trimmed/lowercased; values trimmed; JSON parse failures must not crash UI.
 * RO:SECURITY — stores only operator labels (no secrets); does not persist IP/PII.
 */

export type NodeId = string
export type NodeLabels = Record<string, string>
export type NodeLabelsByNode = Record<NodeId, NodeLabels>

const LS_KEY = 'ron.svc_admin.node_labels.v1'

// In-tab subscribers (storage event won’t fire within the same tab).
type Listener = () => void
const listeners = new Set<Listener>()

function safeParse(json: string | null): unknown {
  if (!json) return {}
  try {
    return JSON.parse(json)
  } catch {
    return {}
  }
}

function normalizeKey(k: string): string {
  return k.trim().toLowerCase()
}

function normalizeVal(v: string): string {
  return v.trim()
}

function isRecord(x: unknown): x is Record<string, unknown> {
  return typeof x === 'object' && x !== null && !Array.isArray(x)
}

function sanitizeLabels(input: unknown): NodeLabelsByNode {
  if (!isRecord(input)) return {}

  const out: NodeLabelsByNode = {}
  for (const [nodeId, labelsAny] of Object.entries(input)) {
    if (!isRecord(labelsAny)) continue
    const labels: NodeLabels = {}
    for (const [k, v] of Object.entries(labelsAny)) {
      if (typeof k !== 'string') continue
      if (typeof v !== 'string') continue
      const nk = normalizeKey(k)
      const nv = normalizeVal(v)
      if (!nk || !nv) continue
      labels[nk] = nv
    }
    if (Object.keys(labels).length > 0) out[nodeId] = labels
  }
  return out
}

function emit(): void {
  for (const fn of listeners) fn()
}

export function subscribeNodeLabels(fn: Listener): () => void {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

export function loadAllNodeLabels(): NodeLabelsByNode {
  const raw = safeParse(localStorage.getItem(LS_KEY))
  return sanitizeLabels(raw)
}

export function getNodeLabels(nodeId: NodeId): NodeLabels {
  const all = loadAllNodeLabels()
  return all[nodeId] ?? {}
}

export function setNodeLabels(nodeId: NodeId, labels: NodeLabels): void {
  const all = loadAllNodeLabels()
  const cleaned: NodeLabels = {}

  for (const [k, v] of Object.entries(labels)) {
    const nk = normalizeKey(k)
    const nv = normalizeVal(v)
    if (!nk || !nv) continue
    cleaned[nk] = nv
  }

  if (Object.keys(cleaned).length === 0) {
    delete all[nodeId]
  } else {
    all[nodeId] = cleaned
  }

  localStorage.setItem(LS_KEY, JSON.stringify(all))
  emit()
}

export function upsertNodeLabel(nodeId: NodeId, key: string, value: string): void {
  const labels = getNodeLabels(nodeId)
  const nk = normalizeKey(key)
  const nv = normalizeVal(value)
  if (!nk || !nv) return
  setNodeLabels(nodeId, { ...labels, [nk]: nv })
}

export function removeNodeLabel(nodeId: NodeId, key: string): void {
  const labels = getNodeLabels(nodeId)
  const nk = normalizeKey(key)
  if (!nk) return
  // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
  delete labels[nk]
  setNodeLabels(nodeId, labels)
}

export function clearNodeLabels(nodeId: NodeId): void {
  setNodeLabels(nodeId, {})
}

// Cross-tab sync.
window.addEventListener('storage', (ev) => {
  if (ev.key === LS_KEY) emit()
})
