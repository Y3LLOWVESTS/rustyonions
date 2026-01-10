// crates/svc-admin/ui/src/routes/NodeListPage.tsx
//
// RO:WHAT — Nodes overview page (multi-node “NOC” view).
// RO:WHY  — Show all registered nodes plus quick health, restart, and
//           metrics freshness summaries.
// RO:INTERACTS —
//   - adminClient.getNodes()           → NodeSummary list
//   - adminClient.getNodeStatus(id)    → AdminStatusView per node
//   - adminClient.getNodeFacetMetrics  → FacetMetricsSummary per node
//   - NodeCard                         → presentational card (selection-aware)
//   - NodePreviewPanel                 → right-hand preview pane
//   - LoadingSpinner / ErrorBanner / EmptyState
//   - i18n/useI18n for copy
//
// RO:UX — Add a top “freshness strip” so operators can tell at a glance whether
//        the dashboard is live, stale, or unreachable before deep-diving.
//
// NOTE: This is intentionally lightweight: it only depends on metricsById health
//       which is already computed by the polling hooks.
//
// NEW (2026-01): Operator tags + cluster-scale filtering.
//   - Tags are stored locally (localStorage) for Phase 1.
//   - Filters are computed purely client-side.
//   - Group-by uses *primary tag* = first tag to avoid duplicating nodes in multiple groups.
//   - Phase 2: swap useNodeTags storage to svc-admin backend endpoints.

import React, { useEffect, useMemo, useState } from 'react'
import {
  NodeCard,
  type NodeStatusSummary,
  type MetricsHealth,
} from '../components/nodes/NodeCard'
import { NodePreviewPanel } from '../components/nodes/NodePreviewPanel'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { EmptyState } from '../components/shared/EmptyState'
import { useI18n } from '../i18n/useI18n'

import { useNodeListData } from './node-list/useNodeListData'
import { buildSummary } from './node-list/helpers'
import { useNodeTags } from './node-list/useNodeTags'

function MetricsPill({
  label,
  count,
  tone,
}: {
  label: string
  count: number
  tone: 'ok' | 'warn' | 'bad' | 'muted'
}) {
  const style: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    padding: '6px 10px',
    borderRadius: 999,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background:
      tone === 'ok'
        ? 'rgba(16,185,129,0.14)'
        : tone === 'warn'
          ? 'rgba(251,146,60,0.14)'
          : tone === 'bad'
            ? 'rgba(244,63,94,0.14)'
            : 'rgba(255,255,255,0.06)',
    color:
      tone === 'ok'
        ? 'rgba(167,243,208,0.95)'
        : tone === 'warn'
          ? 'rgba(254,215,170,0.95)'
          : tone === 'bad'
            ? 'rgba(253,164,175,0.95)'
            : 'rgba(226,232,240,0.92)',
    fontSize: 13,
    lineHeight: 1.2,
    whiteSpace: 'nowrap',
  }

  const bubbleStyle: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    minWidth: 22,
    height: 18,
    padding: '0 6px',
    borderRadius: 999,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(0,0,0,0.18)',
    fontVariantNumeric: 'tabular-nums',
  }

  return (
    <span style={style}>
      <span style={bubbleStyle}>{count}</span>
      {label}
    </span>
  )
}

function FilterChip({
  label,
  value,
}: {
  label: string
  value: string
}) {
  const style: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    padding: '6px 10px',
    borderRadius: 999,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: 'rgba(255,255,255,0.04)',
    color: 'rgba(226,232,240,0.92)',
    fontSize: 12,
    lineHeight: 1.1,
    whiteSpace: 'nowrap',
  }

  const bubbleStyle: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    minWidth: 22,
    height: 18,
    padding: '0 6px',
    borderRadius: 999,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(0,0,0,0.18)',
    fontVariantNumeric: 'tabular-nums',
    opacity: 0.92,
  }

  return (
    <span style={style}>
      <span style={bubbleStyle}>{value}</span>
      {label}
    </span>
  )
}

export function NodeListPage() {
  const { t } = useI18n()

  const {
    nodes,
    loading,
    error,

    statusById,
    metricsById,

    effectiveSelectedId,
    selectedNode,
    selectedStatusState,
    selectedMetricsState,
    setSelectedNodeId,
  } = useNodeListData()

  // Tags storage (Phase 1 = localStorage)
  const nodeIds = useMemo(() => nodes.map((n) => n.id), [nodes])
  const { tagsForNode, addTagToNode, removeTagFromNode, allTags } = useNodeTags(nodeIds)

  // Filters
  const [search, setSearch] = useState('')
  const [profileFilter, setProfileFilter] = useState<string>('__all__')
  const [tagFilter, setTagFilter] = useState<string>('__all__')
  const [onlyProblems, setOnlyProblems] = useState(false)
  const [groupBy, setGroupBy] = useState<'none' | 'tag'>('none')

  const selectedSummary: NodeStatusSummary | undefined = buildSummary(
    selectedStatusState?.status ?? null,
  )

  const selectedPlanes = selectedStatusState?.status?.planes ?? null

  // New: best-effort uptime (seconds) from AdminStatusView.
  const selectedUptimeSeconds =
    (selectedStatusState?.status as any)?.uptime_seconds ?? null

  const metricsCounts = useMemo(() => {
    let fresh = 0
    let stale = 0
    let unreachable = 0
    let unknown = 0

    for (const n of nodes) {
      const h = (metricsById[n.id]?.health ?? null) as MetricsHealth | null
      if (h === 'fresh') fresh += 1
      else if (h === 'stale') stale += 1
      else if (h === 'unreachable') unreachable += 1
      else unknown += 1
    }

    return { fresh, stale, unreachable, unknown }
  }, [metricsById, nodes])

  const profiles = useMemo(() => {
    const set = new Set<string>()
    for (const n of nodes) {
      const p = (n.profile ?? '').trim()
      if (p) set.add(p)
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b))
  }, [nodes])

  const problemById = useMemo(() => {
    const out: Record<string, boolean> = {}
    for (const n of nodes) {
      const statusState = statusById[n.id]
      const metricsState = metricsById[n.id]
      const summary = buildSummary(statusState?.status ?? null)

      const overall = summary?.overallHealth ?? 'healthy'
      const metrics = (metricsState?.health ?? null) as MetricsHealth | null

      const statusProblem = overall !== 'healthy'
      const metricsProblem = metrics === 'stale' || metrics === 'unreachable'
      out[n.id] = statusProblem || metricsProblem
    }
    return out
  }, [metricsById, nodes, statusById])

  const filteredNodes = useMemo(() => {
    const q = search.trim().toLowerCase()
    const wantProfile = profileFilter !== '__all__' ? profileFilter : null
    const wantTag = tagFilter !== '__all__' ? tagFilter : null

    return nodes.filter((n) => {
      if (wantProfile && (n.profile ?? '') !== wantProfile) return false

      if (onlyProblems && !problemById[n.id]) return false

      if (wantTag) {
        const tags = tagsForNode(n.id)
        if (!tags.includes(wantTag)) return false
      }

      if (q) {
        const dn = (n.display_name ?? n.displayName ?? '').toLowerCase()
        const id = (n.id ?? '').toLowerCase()
        const p = (n.profile ?? '').toLowerCase()
        const hay = `${dn} ${id} ${p}`
        if (!hay.includes(q)) return false
      }

      return true
    })
  }, [nodes, onlyProblems, problemById, profileFilter, search, tagFilter, tagsForNode])

  // If the current selection gets filtered out, snap selection to first filtered node.
  useEffect(() => {
    if (!filteredNodes.length) return
    if (!effectiveSelectedId) return
    const stillVisible = filteredNodes.some((n) => n.id === effectiveSelectedId)
    if (!stillVisible) setSelectedNodeId(filteredNodes[0].id)
  }, [effectiveSelectedId, filteredNodes, setSelectedNodeId])

  const grouped = useMemo(() => {
    if (groupBy !== 'tag') return null as null | Array<{ key: string; nodes: typeof nodes }>
    const by = new Map<string, typeof nodes>()
    const untaggedKey = 'untagged'

    for (const n of filteredNodes) {
      const tags = tagsForNode(n.id)
      const primary = tags.length ? tags[0] : untaggedKey
      const bucket = by.get(primary) ?? []
      bucket.push(n)
      by.set(primary, bucket)
    }

    const keys = Array.from(by.keys()).sort((a, b) => {
      if (a === untaggedKey) return 1
      if (b === untaggedKey) return -1
      return a.localeCompare(b)
    })

    return keys.map((k) => ({ key: k, nodes: by.get(k) ?? [] }))
  }, [filteredNodes, groupBy, tagsForNode])

  const selectedTags = selectedNode ? tagsForNode(selectedNode.id) : []

  const filtersBar: React.CSSProperties = {
    display: 'flex',
    gap: 10,
    flexWrap: 'wrap',
    alignItems: 'center',
    padding: '10px 12px',
    borderRadius: 14,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
    background: 'rgba(255,255,255,0.03)',
    margin: '10px 0 14px',
  }

  const inputStyle: React.CSSProperties = {
    height: 34,
    borderRadius: 12,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: 'rgba(0,0,0,0.18)',
    color: 'rgba(226,232,240,0.92)',
    padding: '0 10px',
    outline: 'none',
    minWidth: 220,
  }

  const selectStyle: React.CSSProperties = {
    height: 34,
    borderRadius: 12,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: 'rgba(0,0,0,0.18)',
    color: 'rgba(226,232,240,0.92)',
    padding: '0 10px',
    outline: 'none',
  }

  const buttonStyle: React.CSSProperties = {
    height: 34,
    borderRadius: 12,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: 'rgba(255,255,255,0.06)',
    color: 'rgba(226,232,240,0.92)',
    padding: '0 10px',
    cursor: 'pointer',
    fontWeight: 700,
  }

  const toggleStyle: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    padding: '0 10px',
    height: 34,
    borderRadius: 12,
    border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.14))',
    background: onlyProblems ? 'rgba(251,146,60,0.14)' : 'rgba(255,255,255,0.06)',
    cursor: 'pointer',
    userSelect: 'none',
    color: 'rgba(226,232,240,0.92)',
    fontSize: 12,
    fontWeight: 800,
  }

  const filteredCount = filteredNodes.length

  return (
    <div className="svc-admin-page svc-admin-page-nodes">
      <header className="svc-admin-page-header">
        <div
          style={{
            display: 'flex',
            alignItems: 'baseline',
            justifyContent: 'space-between',
            gap: 16,
            flexWrap: 'wrap',
          }}
        >
          <div>
            <h1>{t('nav.nodes')}</h1>
            <p>
              Overview of nodes registered in this svc-admin instance, including
              quick health, restart, and metrics freshness summaries.
            </p>
          </div>

          {!loading && !error && nodes.length > 0 && (
            <div
              style={{
                display: 'flex',
                gap: 10,
                flexWrap: 'wrap',
                alignItems: 'center',
              }}
            >
              <MetricsPill label="Fresh" count={metricsCounts.fresh} tone="ok" />
              <MetricsPill label="Stale" count={metricsCounts.stale} tone="warn" />
              <MetricsPill
                label="Unreachable"
                count={metricsCounts.unreachable}
                tone="bad"
              />
              {metricsCounts.unknown > 0 && (
                <MetricsPill
                  label="Unknown"
                  count={metricsCounts.unknown}
                  tone="muted"
                />
              )}
            </div>
          )}
        </div>
      </header>

      {loading && <LoadingSpinner />}

      {!loading && error && <ErrorBanner message={error} />}

      {!loading && !error && nodes.length === 0 && (
        <EmptyState message="No nodes configured. Check svc-admin config for node registry entries." />
      )}

      {!loading && !error && nodes.length > 0 && (
        <>
          {/* Filters bar (cluster-scale operator workflow) */}
          <div style={filtersBar}>
            <input
              style={inputStyle}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search nodes (name/id/profile)…"
              aria-label="Search nodes"
            />

            <select
              style={selectStyle}
              value={profileFilter}
              onChange={(e) => setProfileFilter(e.target.value)}
              aria-label="Filter by node profile"
            >
              <option value="__all__">All profiles</option>
              {profiles.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>

            <select
              style={selectStyle}
              value={tagFilter}
              onChange={(e) => setTagFilter(e.target.value)}
              aria-label="Filter by tag"
            >
              <option value="__all__">All tags</option>
              {allTags.map((tag) => (
                <option key={tag} value={tag}>
                  {tag}
                </option>
              ))}
            </select>

            <span
              role="button"
              tabIndex={0}
              style={toggleStyle}
              onClick={() => setOnlyProblems((v) => !v)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') setOnlyProblems((v) => !v)
              }}
              aria-pressed={onlyProblems}
              title="Show only nodes with degraded/down planes or stale/unreachable metrics"
            >
              {onlyProblems ? 'Only problems: ON' : 'Only problems: OFF'}
            </span>

            <select
              style={selectStyle}
              value={groupBy}
              onChange={(e) => setGroupBy(e.target.value as any)}
              aria-label="Group nodes"
              title="Group by primary tag (first tag)"
            >
              <option value="none">No grouping</option>
              <option value="tag">Group by tag</option>
            </select>

            <button
              style={buttonStyle}
              onClick={() => {
                setSearch('')
                setProfileFilter('__all__')
                setTagFilter('__all__')
                setOnlyProblems(false)
                setGroupBy('none')
              }}
            >
              Clear
            </button>

            <div style={{ marginLeft: 'auto', display: 'flex', gap: 10, flexWrap: 'wrap' }}>
              <FilterChip label="Visible" value={String(filteredCount)} />
              <FilterChip label="Total" value={String(nodes.length)} />
            </div>
          </div>

          <div className="svc-admin-node-layout">
            <div className="svc-admin-node-grid">
              {grouped ? (
                grouped.map((g) => (
                  <section key={g.key} style={{ gridColumn: '1 / -1' }}>
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'baseline',
                        justifyContent: 'space-between',
                        gap: 10,
                        padding: '6px 4px 10px',
                      }}
                    >
                      <div style={{ fontWeight: 900, letterSpacing: '0.02em' }}>
                        {g.key === 'untagged' ? 'Untagged' : `Tag: ${g.key}`}
                      </div>
                      <div style={{ opacity: 0.72, fontSize: 12 }}>
                        {g.nodes.length} node{g.nodes.length === 1 ? '' : 's'}
                      </div>
                    </div>

                    <div className="svc-admin-node-grid">
                      {g.nodes.map((node) => {
                        const statusState = statusById[node.id]
                        const metricsState = metricsById[node.id]

                        const summary = buildSummary(statusState?.status ?? null)
                        const isSelected = node.id === effectiveSelectedId

                        return (
                          <NodeCard
                            key={node.id}
                            node={node}
                            tags={tagsForNode(node.id)}
                            statusSummary={summary}
                            statusLoading={statusState?.loading}
                            statusError={statusState?.error ?? null}
                            metricsHealth={metricsState?.health ?? null}
                            metricsLoading={metricsState?.loading}
                            metricsError={metricsState?.error ?? null}
                            isSelected={isSelected}
                            onSelect={() => setSelectedNodeId(node.id)}
                          />
                        )
                      })}
                    </div>
                  </section>
                ))
              ) : (
                filteredNodes.map((node) => {
                  const statusState = statusById[node.id]
                  const metricsState = metricsById[node.id]

                  const summary = buildSummary(statusState?.status ?? null)

                  const isSelected = node.id === effectiveSelectedId

                  return (
                    <NodeCard
                      key={node.id}
                      node={node}
                      tags={tagsForNode(node.id)}
                      statusSummary={summary}
                      statusLoading={statusState?.loading}
                      statusError={statusState?.error ?? null}
                      metricsHealth={metricsState?.health ?? null}
                      metricsLoading={metricsState?.loading}
                      metricsError={metricsState?.error ?? null}
                      isSelected={isSelected}
                      onSelect={() => setSelectedNodeId(node.id)}
                    />
                  )
                })
              )}

              {!filteredNodes.length && (
                <div
                  style={{
                    gridColumn: '1 / -1',
                    padding: 14,
                    borderRadius: 14,
                    border: '1px dashed var(--svc-admin-color-border, rgba(255,255,255,0.12))',
                    opacity: 0.78,
                    fontSize: 13,
                  }}
                >
                  No nodes match the current filters.
                </div>
              )}
            </div>

            <NodePreviewPanel
              node={selectedNode}
              tags={selectedTags}
              onAddTag={(tag) => {
                if (!selectedNode) return
                addTagToNode(selectedNode.id, tag)
              }}
              onRemoveTag={(tag) => {
                if (!selectedNode) return
                removeTagFromNode(selectedNode.id, tag)
              }}
              statusSummary={selectedSummary}
              metricsHealth={selectedMetricsState?.health ?? null}
              metricsLoading={selectedMetricsState?.loading}
              metricsError={selectedMetricsState?.error ?? null}
              planes={selectedPlanes}
              uptimeSeconds={selectedUptimeSeconds}
            />
          </div>
        </>
      )}
    </div>
  )
}
