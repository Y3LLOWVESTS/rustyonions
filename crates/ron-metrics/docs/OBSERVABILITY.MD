
---

````markdown
# 📈 OBSERVABILITY — ron-metrics
*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- Metrics (Prometheus default; OTEL optional via feature `otel`)
- Health & readiness semantics (`/healthz`, `/readyz`)
- Logs (JSON schema/fields)
- Tracing spans & correlation (exemplars optional)
- Alerts & SLOs (benchmarks + rules)

**Labels contract (canonical):** every first-party metric includes  
`service`, `instance`, `build_version`, `amnesia` (plus optional low-cardinality `extra`).

---

## 1) Metrics (Prometheus-style)

### 1.0 Taxonomy & discipline
- Suffixes: `*_seconds`, `*_bytes`, `*_total` only.  
- **Register once** in `Metrics::new()`; **clone handles** everywhere (see `IDB.md` [I-3]).  
- Hot path recording is **lock-free** and **allocation-free** (IDB [I-12]).  
- Cardinality: labels are **static**; forbid per-request/user IDs.

### 1.1 Golden Metrics (every service)
| Name | Type | Labels | Unit | Purpose |
|---|---|---|---|---|
| `http_requests_total{route,method,status}` | Counter | +contract | 1 | Traffic/accounting |
| `request_latency_seconds{route,method}` | Histogram | +contract | seconds | p95/p99 latency |
| `inflight_requests{route}` | Gauge | +contract | 1 | Backpressure surface (or implied by semaphore) |
| `rejected_total{reason}` | Counter | +contract | 1 | Shed/deny reasons (quota/timeouts/method/body/cardinality) |
| `service_restarts_total{service}` | Counter | +contract | 1 | Crash-only observability |
| `bus_lagged_total` | Counter | +contract | 1 | Broadcast lag/drop observed |

> **For the ron-metrics exposer itself** add:  
> `exposition_latency_seconds{endpoint="/metrics"}` (Histogram; p95 < **10ms** local target).

### 1.2 ron-metrics Specific Families (forward-proofed)
| Name | Type | Labels | Unit | Notes |
|---|---|---|---|---|
| `io_timeouts_total{op="read"|"write"}` | Counter | +contract | 1 | Exposer I/O deadline hits |
| `tls_handshake_failures_total` | Counter | +contract | 1 | If TLS enabled |
| `pq_kex_failures_total{algo,role}` | Counter | +contract | 1 | Always registered; may be 0 |
| `pq_verify_failures_total{algo,role}` | Counter | +contract | 1 | — |
| `zk_verify_failures_total{scheme}` | Counter | +contract | 1 | — |
| `zk_proof_latency_seconds{scheme}` | Histogram | +contract | seconds | Pre-registered buckets |

### 1.3 Exporters
- **Default:** Prometheus text exposition at `GET /metrics`.  
- **Optional:** OpenTelemetry (`--otel` + `feature = "otel"`). Mapping stays 1:1 (Counter→Counter, Histogram→Histogram with unit `s`/`By`).  
- **Public API does not change.** Prom remote_write or OTEL collector choice is **deployment** concern.

### 1.4 Scrape guidance
- Prefer **loopback** or **UDS** (CONFIG). If TCP+TLS, scrape via gateway/sidecar.  
- Response headers: `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`.  
- 200 OK body is **small** and bounded; no request bodies are accepted.

### 1.5 Registration snippet (reference)
See `IDB.md` → Implementation 3.2. Buckets for latency:  
`[0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0]`.

---

## 2) Health & Readiness

### 2.1 Endpoints
- `GET /healthz` — **liveness** (200 when process is running).  
- `GET /readyz` — **readiness** (200 only when minimal dependencies are up).

### 2.2 Readiness truth (JSON shape)
```json
{ "degraded": true, "missing": ["<keys>"], "retry_after": 5 }
````

**Keys** (examples; host-dependent): `config_loaded`, `port_bound`, `uds_ready`, `kernel_bus_attached`, `<service_dep_ready>`.

### 2.3 Failure semantics

* **Fail-open reads**, **fail-closed writes** (Final Blueprint).
* `/readyz` transitions to `503` **before** collapse/saturation; `/healthz` may stay 200 until exit.
* Headers: include `Retry-After: <seconds>` when degraded.

---

## 3) Logs

### 3.1 Format (JSON lines)

Required fields:

* `ts` (RFC3339), `level` (`TRACE|DEBUG|INFO|WARN|ERROR`)
* `service`, `instance`, `build_version`, `amnesia`
* `event` (e.g., `metrics.scrape`, `ready.degraded`, `tls.handshake_fail`)
* `endpoint` (`/metrics|/healthz|/readyz`), `status`
* `reason` (aligns with `rejected_total{reason}`)
* `peer_addr`, `user_agent` (if available)
* `corr_id` (UUID/ULID; injected by gateway/host)
* `latency_ms` (when applicable)

**Redaction:** no PII/secrets; `labels.extra` is allowlisted; config diffs are **redacted**.

### 3.2 Examples

```json
{"ts":"2025-10-05T14:03:21Z","level":"INFO","service":"gateway","event":"metrics.scrape","endpoint":"/metrics","status":200,"latency_ms":4,"peer_addr":"127.0.0.1:54122","build_version":"1.2.3+abc","instance":"edge-a1","amnesia":"on"}
{"ts":"2025-10-05T14:04:10Z","level":"WARN","service":"gateway","event":"ready.degraded","status":503,"reason":"dep:kernel_bus","missing":["kernel_bus_attached"],"retry_after":5}
```

---

## 4) Tracing & Correlation

* Use `tracing` + JSON subscriber; span names: `svc.ron_metrics.expose`, `svc.<host>.request`.
* **Correlation ID:** accept/propagate header `X-Corr-ID`; include as span field and log field.
* **Exemplars (optional):** when enabled, attach exemplar trace IDs to `*_latency_seconds` observations (keep cardinality bounded).
* **Span fields (min):** `service`, `instance`, `corr_id`, `endpoint`, `amnesia`, `build_version`.

---

## 5) Alerts & SLOs

### 5.1 SLOs (ron-metrics exposer)

* **p95 `exposition_latency_seconds` < 10ms (local)**; < 25ms (intra-cluster).
* **Scrape errors** (`5xx` or timeouts) < **0.1%** 5m-rate.
* **Sustained `/readyz` 503** window < **60s** under planned restarts.
* **TLS handshakes failures** burst alert (if TLS enabled).

### 5.2 Standard platform SLOs (for host services)

* Public GET p95 < 80ms intra-region, < 200ms inter-region.
* 5xx rate < 0.1%; 429/503 < 1%.
* `bus_lagged_total` not increasing for > 5m.

### 5.3 Prometheus rules (examples)

```yaml
groups:
- name: ron-metrics-exposer
  rules:
  - alert: MetricsExposerSlow
    expr: histogram_quantile(0.95, sum by (le) (rate(exposition_latency_seconds_bucket[5m]))) > 0.025
    for: 10m
    labels: { severity: warning }
    annotations:
      summary: "Exposer p95 latency > 25ms for 10m"
      runbook: "RUNBOOK.md#metrics-exposer-slow"

  - alert: ReadyzDegraded
    expr: sum(rate(rejected_total{reason="ready_degraded"}[5m])) > 0
    for: 5m
    labels: { severity: critical }
    annotations:
      summary: "Service readiness degraded"
      runbook: "RUNBOOK.md#readiness-degraded"

  - alert: TlsHandshakeFailures
    expr: rate(tls_handshake_failures_total[5m]) > 0
    for: 10m
    labels: { severity: warning }
    annotations:
      summary: "TLS handshake failures observed on metrics listener"
      runbook: "RUNBOOK.md#tls-handshake-failures"
```

> Replace `rejected_total{reason="ready_degraded"}` with your host’s chosen readiness-reject metric if different.

---

## 6) CI / Enforcement

* **Taxonomy gate:** CI regex ensures `(_seconds|_bytes|_total)\b` suffixes and presence of labels `service,instance,amnesia,build_version`.
* **Registration gate:** unit test guarantees **single registration**; duplicates fail.
* **Endpoint gate:** integration tests ensure `GET /metrics`, `/healthz`, `/readyz` exist and are bounded.
* **Perf gate:** Criterion bench: p95 `exposition_latency_seconds` < **10ms** local.
* **Lints:** `clippy::await_holding_lock` (deny), `-D warnings`.
* **Chaos:** restart under scrape; `/readyz` flips before exit; `service_restarts_total` increments.
* **Docs freshness:** file review every **90 days** or on change to observability surface.

---

### Appendix A — Dashboard sketch (panels)

* **Exposer latency:** p50/p95/p99 `exposition_latency_seconds`.
* **Scrape health:** rate of 5xx/timeouts; tls handshake failures.
* **Readiness:** success ratio + last degraded reasons.
* **Restarts:** `service_restarts_total` by `service`.
* **Bus health:** `bus_lagged_total` (rate).
* **PQ/ZK:** failure counters and proof latency (if features enabled).

### Appendix B — Quick curl checks

```bash
curl -fsS localhost:9600/metrics | head
curl -fsS -o /dev/null -w "%{http_code}\n" localhost:9600/healthz
curl -fsS -o /dev/null -w "%{http_code}\n" localhost:9600/readyz
```

> Expect `200/200/200` when ready; `readyz` returns `503` with JSON when degraded.

```
```
