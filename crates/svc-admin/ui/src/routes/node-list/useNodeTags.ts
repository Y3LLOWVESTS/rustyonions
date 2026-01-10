// crates/svc-admin/ui/src/routes/node-list/useNodeTags.ts
//
// RO:WHAT — UI-only storage for operator-assigned node tags (labels).
// RO:WHY  — Enables NOC-scale filtering/grouping before backend tag APIs ship.
// RO:INVARIANTS —
//   - No conditional hooks.
//   - Tags are normalized (trim, lowercase, safe charset).
//   - Stored deterministically (sorted, unique).
//
// PHASE PLAN:
//   - Phase 1 (current): localStorage key/value map { [nodeId]: string[] }.
//   - Phase 2: replace persistence with svc-admin backend endpoints,
//     keeping this hook API stable for the UI.

import { useCallback, useEffect, useMemo, useState } from 'react'

type TagMap = Record<string, string[]>

const STORAGE_KEY = 'svc-admin.node_tags.v1'

function safeJsonParse<T>(raw: string | null): T | null {
  if (!raw) return null
  try {
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

function uniqSorted(xs: string[]): string[] {
  const out = Array.from(new Set(xs.filter(Boolean)))
  out.sort((a, b) => a.localeCompare(b))
  return out
}

export function normalizeTag(input: string): string {
  let s = String(input ?? '').trim().toLowerCase()
  if (!s) return ''

  // Normalize whitespace → hyphens, keep a conservative charset.
  s = s.replace(/\s+/g, '-')
  s = s.replace(/[^a-z0-9._-]/g, '')
  s = s.replace(/-+/g, '-')
  s = s.replace(/^[-_.]+|[-_.]+$/g, '')

  // Guardrails (operator UX, avoids junk payloads).
  if (s.length > 48) s = s.slice(0, 48)
  return s
}

function loadTagMap(): TagMap {
  if (typeof window === 'undefined') return {}
  const parsed = safeJsonParse<TagMap>(window.localStorage.getItem(STORAGE_KEY))
  if (!parsed || typeof parsed !== 'object') return {}

  const out: TagMap = {}
  for (const [nodeId, tagsAny] of Object.entries(parsed)) {
    if (!nodeId) continue
    const arr = Array.isArray(tagsAny) ? (tagsAny as unknown[]) : []
    const tags = uniqSorted(
      arr
        .map((x) => normalizeTag(String(x ?? '')))
        .filter(Boolean),
    )
    if (tags.length) out[nodeId] = tags
  }
  return out
}

function saveTagMap(map: TagMap) {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(map))
  } catch {
    // Ignore quota / privacy mode failures. UI still works in-memory.
  }
}

export function useNodeTags(knownNodeIds: string[] = []) {
  const [map, setMap] = useState<TagMap>(() => loadTagMap())

  // Persist changes.
  useEffect(() => {
    saveTagMap(map)
  }, [map])

  // Best-effort pruning: drop tags for nodes no longer present in registry.
  // This keeps long-running operator sessions clean in large clusters.
  useEffect(() => {
    if (!knownNodeIds.length) return
    const keep = new Set(knownNodeIds)
    setMap((prev) => {
      let changed = false
      const next: TagMap = {}
      for (const [id, tags] of Object.entries(prev)) {
        if (!keep.has(id)) {
          changed = true
          continue
        }
        next[id] = tags
      }
      return changed ? next : prev
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [knownNodeIds.join('|')])

  const tagsForNode = useCallback(
    (nodeId: string): string[] => {
      const t = map[nodeId]
      return Array.isArray(t) ? t : []
    },
    [map],
  )

  const setTagsForNode = useCallback((nodeId: string, tags: string[]) => {
    const norm = uniqSorted(tags.map(normalizeTag).filter(Boolean))
    setMap((prev) => {
      if (!norm.length) {
        if (!prev[nodeId]) return prev
        const { [nodeId]: _, ...rest } = prev
        return rest
      }
      return { ...prev, [nodeId]: norm }
    })
  }, [])

  const addTagToNode = useCallback((nodeId: string, tag: string) => {
    const t = normalizeTag(tag)
    if (!t) return
    setMap((prev) => {
      const cur = prev[nodeId] ?? []
      if (cur.includes(t)) return prev
      return { ...prev, [nodeId]: uniqSorted([...cur, t]) }
    })
  }, [])

  const removeTagFromNode = useCallback((nodeId: string, tag: string) => {
    const t = normalizeTag(tag)
    if (!t) return
    setMap((prev) => {
      const cur = prev[nodeId] ?? []
      const next = cur.filter((x) => x !== t)
      if (!next.length) {
        if (!prev[nodeId]) return prev
        const { [nodeId]: _, ...rest } = prev
        return rest
      }
      return { ...prev, [nodeId]: next }
    })
  }, [])

  const allTags = useMemo(() => {
    const flat: string[] = []
    for (const tags of Object.values(map)) flat.push(...tags)
    return uniqSorted(flat)
  }, [map])

  return {
    map,
    tagsForNode,
    setTagsForNode,
    addTagToNode,
    removeTagFromNode,
    allTags,
  }
}
