// crates/svc-admin/ui/src/routes/BenchmarksPage.tsx
//
// RO:WHAT — Benchmarks runner page.
// RO:WHY  — Let operators run node-executed benchmark suites and view results.
// RO:INVARIANTS —
//   - Benchmarks are initiated by svc-admin, but executed by the node.
//   - Missing endpoints => graceful “Not implemented” posture.
// RO:SECURITY — No secrets; read-only display; node enforces auth for POST run.

import React, { useEffect, useMemo, useRef, useState } from 'react'
import { adminClient, isHttpError } from '../api/adminClient'
import type { BenchRunReq, BenchRunResultDto, BenchRunStatusDto, NodeSummary } from '../types/admin-api'

function fmtBytes(n: number) {
  if (!Number.isFinite(n) || n <= 0) return '0 B'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  let v = n
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(i === 0 ? 0 : 2)} ${units[i]}`
}

function clampInt(v: number, min: number, max: number): number {
  if (!Number.isFinite(v)) return min
  const n = Math.trunc(v)
  return Math.max(min, Math.min(max, n))
}

type AnyBenchResult = BenchRunResultDto & {
  // macronode v1 shape (actual today)
  results?: Array<{
    name: string
    method: string
    path: string
    requests: number
    errors: number
    rps: number
    p50Ms?: number
    p95Ms?: number
    p99Ms?: number
    // some DTOs might still be snake-ish; tolerate both
    p50_ms?: number
    p95_ms?: number
    p99_ms?: number
  }>
  notes?: string[]
  // “future/god-tier” shape (kept compatible)
  scenarios?: Array<{
    name: string
    ok: boolean
    p50LatencyMs?: number | null
    p95LatencyMs?: number | null
    p99LatencyMs?: number | null
    throughputOpsPerSec?: number | null
    throughputBytesPerSec?: number | null
    errorRate?: number | null
    notes?: string[] | null
  }>
}

export function BenchmarksPage() {
  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [nodeId, setNodeId] = useState<string>('')

  // IMPORTANT:
  // macronode v1 only supports suite="admin_plane" today; other suite names will 400. 
  const [suite, setSuite] = useState<string>('admin_plane')

  const [durationSecs, setDurationSecs] = useState<number>(10)
  const [concurrency, setConcurrency] = useState<number>(8)
  const [payloadSize, setPayloadSize] = useState<number>(64 * 1024)
  const [seed, setSeed] = useState<number>(1337)

  const [runId, setRunId] = useState<string | null>(null)
  const [status, setStatus] = useState<BenchRunStatusDto | null>(null)
  const [result, setResult] = useState<AnyBenchResult | null>(null)
  const [err, setErr] = useState<string | null>(null)

  const pollingRef = useRef<number | null>(null)

  useEffect(() => {
    let alive = true
    adminClient
      .getNodes()
      .then((ns) => {
        if (!alive) return
        setNodes(ns)
        if (!nodeId && ns.length > 0) setNodeId(ns[0].id)
      })
      .catch((e: any) => {
        if (!alive) return
        setErr(e?.message ? String(e.message) : 'Failed to load nodes')
      })
    return () => {
      alive = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const req: BenchRunReq = useMemo(
    () => ({
      suite: suite.trim() || 'admin_plane',
      durationSecs: clampInt(durationSecs, 1, 120),
      concurrency: clampInt(concurrency, 1, 64),
      payloadSize: clampInt(payloadSize, 0, 256 * 1024 * 1024),
      seed: clampInt(seed, 0, 2 ** 31 - 1),
    }),
    [suite, durationSecs, concurrency, payloadSize, seed],
  )

  function stopPolling() {
    if (pollingRef.current != null) {
      window.clearInterval(pollingRef.current)
      pollingRef.current = null
    }
  }

  async function poll(node: string, run: string) {
    const s = await adminClient.getNodeBenchRunStatus(node, run)
    if (s == null) {
      setErr('Bench endpoints not implemented on this node (or proxy not wired).')
      stopPolling()
      return
    }
    setStatus(s)

    if (s.status === 'done') {
      stopPolling()
      const r = await adminClient.getNodeBenchRunResult(node, run)
      if (r != null) setResult(r as AnyBenchResult)
      return
    }

    if (s.status === 'failed') {
      stopPolling()
      setErr(s.error ?? 'Benchmark failed')
      return
    }
  }

  async function onRun() {
    setErr(null)
    setResult(null)
    setStatus(null)
    setRunId(null)
    stopPolling()

    if (!nodeId) {
      setErr('Select a node first.')
      return
    }

    try {
      const resp = await adminClient.runNodeBench(nodeId, req)
      if (resp == null) {
        setErr('Bench endpoints not implemented on this node (or proxy not wired).')
        return
      }
      setRunId(resp.runId)

      // start polling
      pollingRef.current = window.setInterval(() => {
        poll(nodeId, resp.runId).catch((e) => {
          const msg = e?.message ? String(e.message) : 'Polling failed'
          setErr(msg)
          stopPolling()
        })
      }, 750) as unknown as number

      // do an immediate poll so UI feels snappy
      await poll(nodeId, resp.runId)
    } catch (e: any) {
      if (isHttpError(e)) {
        const body = e.body ? ` — ${e.body}` : ''
        setErr(`Run request failed (status ${e.status ?? 'unknown'})${body}`)
      } else {
        setErr(e?.message ? String(e.message) : 'Run request failed')
      }
    }
  }

  useEffect(() => {
    return () => stopPolling()
  }, [])

  const selectedNode = nodes.find((n) => n.id === nodeId)

  const macronodeResults = (result as AnyBenchResult | null)?.results ?? null
  const scenarioResults = (result as AnyBenchResult | null)?.scenarios ?? null
  const notes = (result as AnyBenchResult | null)?.notes ?? null

  return (
    <div className="page">
      <div className="page-header">
        <h1>Benchmarks</h1>
        <div className="muted">
          Node-executed (stable timing). MVP suite: <code>admin_plane</code>
        </div>
      </div>

      {err ? (
        <div className="svc-admin-banner danger" style={{ marginBottom: 12 }}>
          <strong>Error:</strong> {err}
        </div>
      ) : null}

      <div className="svc-admin-card subtle" style={{ marginBottom: 12 }}>
        <div className="svc-admin-card-body">
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', alignItems: 'center' }}>
            <div>
              <div className="muted" style={{ marginBottom: 4 }}>
                Node
              </div>
              <select value={nodeId} onChange={(e) => setNodeId(e.target.value)}>
                {nodes.map((n) => (
                  <option key={n.id} value={n.id}>
                    {n.name ?? n.id}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <div className="muted" style={{ marginBottom: 4 }}>
                Suite
              </div>
              <select value={suite} onChange={(e) => setSuite(e.target.value)}>
                <option value="admin_plane">admin_plane (implemented)</option>
                <option value="storage_http_put_get" disabled>
                  storage_http_put_get (coming soon)
                </option>
              </select>
            </div>

            <div>
              <div className="muted" style={{ marginBottom: 4 }}>
                Duration (secs) (cap: 120)
              </div>
              <input
                type="number"
                value={durationSecs}
                min={1}
                max={120}
                onChange={(e) => setDurationSecs(Number(e.target.value))}
                style={{ width: 90 }}
              />
            </div>

            <div>
              <div className="muted" style={{ marginBottom: 4 }}>
                Concurrency (cap: 64)
              </div>
              <input
                type="number"
                value={concurrency}
                min={1}
                max={64}
                onChange={(e) => setConcurrency(Number(e.target.value))}
                style={{ width: 90 }}
              />
            </div>

            <div>
              <div className="muted" style={{ marginBottom: 4 }}>
                Payload size (bytes) (placeholder for future suites)
              </div>
              <input
                type="number"
                value={payloadSize}
                min={0}
                max={256 * 1024 * 1024}
                onChange={(e) => setPayloadSize(Number(e.target.value))}
                style={{ width: 140 }}
              />
              <span className="muted" style={{ marginLeft: 8 }}>
                ({fmtBytes(payloadSize)})
              </span>
            </div>

            <div>
              <div className="muted" style={{ marginBottom: 4 }}>
                Seed
              </div>
              <input
                type="number"
                value={seed}
                min={0}
                onChange={(e) => setSeed(Number(e.target.value))}
                style={{ width: 110 }}
              />
            </div>

            <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end' }}>
              <button className="btn" onClick={onRun}>
                Run on {selectedNode?.name ?? selectedNode?.id ?? 'Node'}
              </button>
              {pollingRef.current != null ? (
                <button
                  className="btn secondary"
                  onClick={() => {
                    stopPolling()
                    setErr('Polling stopped (manual).')
                  }}
                >
                  Stop
                </button>
              ) : null}
            </div>
          </div>
        </div>
      </div>

      <div className="svc-admin-card" style={{ marginBottom: 12 }}>
        <div className="svc-admin-card-header">
          <div className="svc-admin-card-title">Status</div>
        </div>
        <div className="svc-admin-card-body">
          {runId ? (
            <>
              <div>
                <span className="muted">Run:</span> <code>{runId}</code>
              </div>
              {status ? (
                <div style={{ marginTop: 8 }}>
                  <div>
                    <span className="muted">State:</span> <strong>{status.status}</strong>{' '}
                    <span className="muted">({Math.round((status.progress ?? 0) * 100)}%)</span>
                  </div>
                  <div className="muted" style={{ marginTop: 2 }}>
                    Phase: {status.phase}
                    {status.startedAt ? ` · startedAt=${status.startedAt}` : ''}
                    {/* some DTOs call this updatedAt; show if present */}
                    {'updatedAt' in status && (status as any).updatedAt
                      ? ` · updatedAt=${(status as any).updatedAt}`
                      : ''}
                  </div>
                </div>
              ) : (
                <div className="muted" style={{ marginTop: 8 }}>
                  Waiting for status…
                </div>
              )}
            </>
          ) : (
            <div className="muted">No active run.</div>
          )}
        </div>
      </div>

      <div className="svc-admin-card">
        <div className="svc-admin-card-header">
          <div className="svc-admin-card-title">Result</div>
        </div>
        <div className="svc-admin-card-body">
          {!result ? (
            <div className="muted">No result yet.</div>
          ) : macronodeResults ? (
            <>
              <table className="svc-admin-table" style={{ width: '100%' }}>
                <thead>
                  <tr>
                    <th style={{ textAlign: 'left' }}>Name</th>
                    <th style={{ textAlign: 'left' }}>Method</th>
                    <th style={{ textAlign: 'left' }}>Path</th>
                    <th style={{ textAlign: 'right' }}>Req</th>
                    <th style={{ textAlign: 'right' }}>Err</th>
                    <th style={{ textAlign: 'right' }}>RPS</th>
                    <th style={{ textAlign: 'right' }}>p50 (ms)</th>
                    <th style={{ textAlign: 'right' }}>p95 (ms)</th>
                    <th style={{ textAlign: 'right' }}>p99 (ms)</th>
                  </tr>
                </thead>
                <tbody>
                  {macronodeResults.map((r, idx) => {
                    const p50 = r.p50Ms ?? r.p50_ms ?? 0
                    const p95 = r.p95Ms ?? r.p95_ms ?? 0
                    const p99 = r.p99Ms ?? r.p99_ms ?? 0
                    return (
                      <tr key={idx}>
                        <td>{r.name}</td>
                        <td className="muted">{r.method}</td>
                        <td className="muted">
                          <code>{r.path}</code>
                        </td>
                        <td style={{ textAlign: 'right' }}>{r.requests}</td>
                        <td style={{ textAlign: 'right' }}>{r.errors}</td>
                        <td style={{ textAlign: 'right' }}>{r.rps.toFixed(1)}</td>
                        <td style={{ textAlign: 'right' }}>{p50.toFixed(3)}</td>
                        <td style={{ textAlign: 'right' }}>{p95.toFixed(3)}</td>
                        <td style={{ textAlign: 'right' }}>{p99.toFixed(3)}</td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>

              {Array.isArray(notes) && notes.length > 0 ? (
                <div style={{ marginTop: 12 }}>
                  <div className="muted" style={{ marginBottom: 6 }}>
                    Notes
                  </div>
                  <ul style={{ margin: 0, paddingLeft: 18 }}>
                    {notes.map((n, i) => (
                      <li key={i} className="muted">
                        {n}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </>
          ) : scenarioResults ? (
            <>
              <div className="muted" style={{ marginBottom: 8 }}>
                (Scenario-format result)
              </div>
              <table className="svc-admin-table" style={{ width: '100%' }}>
                <thead>
                  <tr>
                    <th style={{ textAlign: 'left' }}>Scenario</th>
                    <th style={{ textAlign: 'right' }}>OK</th>
                    <th style={{ textAlign: 'right' }}>p50</th>
                    <th style={{ textAlign: 'right' }}>p95</th>
                    <th style={{ textAlign: 'right' }}>p99</th>
                    <th style={{ textAlign: 'right' }}>Ops/s</th>
                    <th style={{ textAlign: 'right' }}>Bytes/s</th>
                    <th style={{ textAlign: 'right' }}>Err rate</th>
                  </tr>
                </thead>
                <tbody>
                  {scenarioResults.map((s, idx) => (
                    <tr key={idx}>
                      <td>{s.name}</td>
                      <td style={{ textAlign: 'right' }}>{s.ok ? 'yes' : 'no'}</td>
                      <td style={{ textAlign: 'right' }}>{(s.p50LatencyMs ?? 0).toFixed(3)}</td>
                      <td style={{ textAlign: 'right' }}>{(s.p95LatencyMs ?? 0).toFixed(3)}</td>
                      <td style={{ textAlign: 'right' }}>{(s.p99LatencyMs ?? 0).toFixed(3)}</td>
                      <td style={{ textAlign: 'right' }}>
                        {s.throughputOpsPerSec == null ? '—' : s.throughputOpsPerSec.toFixed(1)}
                      </td>
                      <td style={{ textAlign: 'right' }}>
                        {s.throughputBytesPerSec == null ? '—' : s.throughputBytesPerSec.toFixed(0)}
                      </td>
                      <td style={{ textAlign: 'right' }}>
                        {s.errorRate == null ? '—' : `${(s.errorRate * 100).toFixed(2)}%`}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          ) : (
            <div className="muted">Result received, but format is unknown.</div>
          )}
        </div>
      </div>
    </div>
  )
}
