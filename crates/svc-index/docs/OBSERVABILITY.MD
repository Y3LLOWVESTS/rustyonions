---

# 📈 OBSERVABILITY.md — `svc-index`

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus/OTEL)
* Health & readiness semantics
* Logs (JSON schema, fields)
* Tracing spans & correlation
* Alerts & SLOs

`svc-index` is a **read-optimized resolver**: it maps **names / `b3:<hex>` content IDs → manifests → provider sets**. It never serves blob bytes; it depends on `svc-dht` (provider discovery) and the kernel plane (Bus, Metrics, Health, Config). Observability therefore emphasizes: **lookup latency, cache health, DHT interactions, admission control, and truthful readiness**.

---

## 1) Metrics (Prometheus-style)

> **Discipline:** Register metrics once in a `Metrics::new()` (or crate-equivalent), store handles, and clone. Never register in hot paths. Keep **label cardinality low**: prefer `{route="/resolve/:key"}` over embedding the concrete key. Never include `corr_id` as a label.

### 1.1 Golden Metrics (every service)

* `http_requests_total{route,method,status}` (Counter)
  Increment on response write. `route` must be **templated** (e.g., `/resolve/:key`).
* `request_latency_seconds{route,method}` (Histogram)
  Buckets: `= {0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5}`.
* `inflight_requests{route}` (Gauge)
  Increment on admission, decrement on response/early reject.
* `rejected_total{reason}` (Counter)
  Reasons (fixed enum): `unauth|over_capacity|timeout|body_cap|decompress_cap|bad_request|upstream_unready|internal`.
* `bus_lagged_total` (Counter)
  Count of dropped bus events due to backlog (kernel broadcast).
* `service_restarts_total` (Counter)
  Emitted by supervisor on crash/restart.
* `tls_handshake_failures_total` (Counter, if TLS is terminated here).

### 1.2 `svc-index` Service-Specific

* **Resolver path**

  * `index_resolve_latency_seconds{source="cache|db|dht"}` (Histogram)
    End-to-end latency by source of truth.
  * `index_cache_hits_total` / `index_cache_misses_total` (Counter)
    Hit/miss at the manifest/provider cache.
  * `index_providers_returned{class="any|region_match|fresh"}` (Histogram/Gauge)
    Providers returned per query (bucketed: `1,2,3,5,8,13`).
  * `index_integrity_fail_total{stage="manifest|cid"}` (Counter)
    BLAKE3 or manifest validation failures (should be **0**).

* **DHT interactions**

  * `dht_lookup_ms` (Histogram)
    Latency for provider lookup; same buckets as `request_latency_seconds` but in ms when exported via OTEL.
  * `dht_errors_total{kind="timeout|not_found|io"}` (Counter)
  * `dht_hedged_requests_total{result="win|lose|tie"}` (Counter)
    Hedge outcome for α/β parallel requests.
  * `dht_records_missing_total` (Counter)
    Observed missing/expired provider records.

* **Admission & capacity**

  * `admission_inflight{stage="preauth|authorized|in_handler"}` (Gauge)
  * `quota_exhaustions_total{dimension="rps|concurrency|ip"}` (Counter)

* **Amnesia mode (if enabled)**

  * `amnesia_mode{status="on|off"}` (Gauge; 1/0)
  * `amnesia_purge_total{kind="cache|temp"}` (Counter)

### 1.3 Registration & Consistency

* Metrics created in a single constructor; **no duplicate registration**.
* Label sets are **fixed**; CI greps defend against accidental high-cardinality labels.
* Histogram buckets are **identical** across crates for `request_latency_seconds` to enable global SLOs.

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness**. Always `200 OK` if process/event loop is alive.
* `GET /readyz` — **readiness**. `200 OK` only when **all readiness keys** are satisfied.
* `GET /metrics` — Prometheus plaintext exposition.
* `GET /version` — Build metadata (git SHA, crate version, rustc).

### 2.2 Readiness Keys (svc-index)

Readiness is the **AND** of:

* `config_loaded` — configuration parsed, validated, applied.
* `db_open` — index database opened (and migration, if any, complete).
* `bus_attached` — kernel bus receiver attached (to receive control events).
* `dht_client_ready` — client to `svc-dht` initialized, successful ping within `N=5s`.
* `listeners_bound` — all sockets bound (HTTP/UDS) and serving.
* `metrics_live` — metrics endpoint serving (self-scrape passes).

> **Contract:** Until all keys are true, `/readyz` MUST return `503` with a JSON body describing which keys are missing.

### 2.3 Failure & Degradation Semantics

* **Writes/admin** (e.g., backfill, pin/unpin) are **fail-closed**.
* **Reads** degrade depending on capability:

  * If DB is open but DHT is degraded, serve cache/DB results **with freshness flags**; set `Retry-After` when confidence is low.
  * If DB is not open, **do not** serve reads; return `503 upstream_unready`.
* Standard degraded response body:

  ```json
  { "degraded": true, "missing": ["dht_client_ready"], "retry_after": 5 }
  ```

---

## 3) Logs

### 3.1 Format & Transport

* **JSON lines** (`application/jsonl`), one event per line.
* Structured with `tracing` → `tracing-subscriber` (JSON).
* **Never** log secrets, capability tokens, or full payloads.

### 3.2 Required Fields

* `ts` (RFC3339/ISO8601, UTC)
* `level` (`TRACE|DEBUG|INFO|WARN|ERROR`)
* `service` (e.g., `"svc-index"`)
* `version` (crate semver) and `git_sha` (short)
* `event` (taxonomy: `resolve_begin|resolve_ok|resolve_fail|admission_reject|quota_exhausted|audit_emit|dht_query|dht_timeout|cache_hit|cache_miss|readyz_state`)
* `route` (templated), `method`, `status`
* `corr_id` (UUID/ULID; see §4)
* `latency_ms` (when applicable)
* `key_hint` (shortened CID/name hint: **first 12 hex** or sanitized name; never full secrets)
* `reason` (must align with `rejected_total{reason}` enum)

### 3.3 Examples

* **Resolve success**

  ```json
  {"ts":"2025-10-03T20:17:11Z","level":"INFO","service":"svc-index","version":"0.1.0","git_sha":"a1b2c3d","event":"resolve_ok","route":"/resolve/:key","method":"GET","status":200,"corr_id":"01J9W1M0K6Q1GJ9E8Z4Q0T8M5X","latency_ms":27,"key_hint":"b3:7f19a2c0a1b2","source":"cache","providers":5}
  ```
* **Admission reject (over capacity)**

  ```json
  {"ts":"2025-10-03T20:18:05Z","level":"WARN","service":"svc-index","event":"admission_reject","route":"/resolve/:key","method":"GET","status":503,"corr_id":"01J9W1M0ZEF6Y4F3X2","reason":"over_capacity"}
  ```

### 3.4 Redaction

* Redact any `config` or `env` values flagged as secret.
* Truncate `key_hint` to keep lines compact and avoid copyable IDs in logs.
* For errors, prefer **stable reasons** and **opaque IDs** over payload echoes.

---

## 4) Tracing & Correlation

* Use `tracing` spans with the naming convention:
  `svc.index.<operation>` — e.g., `svc.index.resolve`, `svc.index.dht_lookup`, `svc.index.admin.pin`.
* **Span structure (typical request):**

  * Root span: `svc.index.resolve`
    Attributes: `route`, `method`, `client_addr` (if safe), `user_role` (coarse), `key_hint`.
  * Child span: `svc.index.cache_check`
  * Child span: `svc.index.db_read` (if needed)
  * Child span: `svc.index.dht_lookup` (hedged; include `alpha`, `beta`, `winner`)
* **Correlation IDs**

  * Ingest from `X-Corr-ID` header; if missing, generate ULID.
  * Propagate via Bus events and outbound calls (`svc-dht`) using the same field name.
  * Add exemplars to latency histograms when supported by the exporter.
* **OpenTelemetry**

  * Feature-flag optional OTEL exporter (trace + metrics).
  * When enabled, map `corr_id` to `TraceId` if caller provides W3C Trace Context.

---

## 5) Alerts & SLOs

> **Philosophy:** Alerts must be **actionable** and map to a **RUNBOOK.md** entry. SLOs are **externally visible** promises; alerts generally fire when we are on track to **miss** an SLO (burn-rate style).

### 5.1 Standard SLOs (svc-index)

* **Latency (read path):** p95 `< 80 ms` intra-region, `< 200 ms` inter-region.
* **Availability:** 5xx error rate `< 0.1%` (per 10-min windows).
* **Throttle:** 429/503 `< 1%` sustained (excluding intentional load-shed drills).
* **Integrity:** `index_integrity_fail_total == 0`.
* **Cache health:** hit ratio `> 0.80` (rolling 1h) under steady state.
* **DHT health:** `dht_errors_total` near 0; hedge **win rate > 0.2** under contention.

### 5.2 Prometheus Alert Examples

```yaml
groups:
- name: svc-index
  rules:
  - alert: SvcIndexLatencyP95High
    expr: histogram_quantile(0.95, sum(rate(request_latency_seconds_bucket{route="/resolve/:key"}[5m])) by (le)) > 0.2
    for: 10m
    labels: {severity: warning}
    annotations:
      summary: "svc-index p95 latency high"
      runbook_url: "/docs/RUNBOOK.md#latency-spike"

  - alert: SvcIndexErrorRateHigh
    expr: sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m])) > 0.001
    for: 10m
    labels: {severity: critical}
    annotations:
      summary: "svc-index 5xx error rate above SLO"
      runbook_url: "/docs/RUNBOOK.md#error-surge"

  - alert: SvcIndexThrottleBurst
    expr: sum(rate(rejected_total{reason=~"over_capacity|timeout"}[1m])) > 100
    for: 5m
    labels: {severity: warning}
    annotations:
      summary: "Admission rejections spiking"
      runbook_url: "/docs/RUNBOOK.md#admission"

  - alert: SvcIndexCacheMissSpike
    expr: rate(index_cache_misses_total[5m]) / clamp_min(rate(index_cache_hits_total[5m]), 1) > 0.5
    for: 15m
    labels: {severity: warning}
    annotations:
      summary: "Cache miss ratio degraded"
      runbook_url: "/docs/RUNBOOK.md#cache"

  - alert: SvcIndexDHTUnhealthy
    expr: rate(dht_errors_total[5m]) > 0
    for: 30m
    labels: {severity: critical}
    annotations:
      summary: "Persistent DHT lookup errors"
      runbook_url: "/docs/RUNBOOK.md#dht"
```

> Replace runbook URLs with your repo’s path.

### 5.3 Runbooks

Each alert maps to a **concrete** set of operator steps: collection, isolation, mitigation, validation, and rollback. Include canned queries and a **decision tree** in `RUNBOOK.md`.

---

## 6) CI / Enforcement

* **Endpoints contract:** CI asserts `/metrics`, `/healthz`, `/readyz`, `/version` are served and return expected shapes.
* **Metric presence:** `rejected_total{reason}`, `request_latency_seconds`, `http_requests_total` must exist and tick under synthetic load.
* **Label lint:** Static analyzer (grep/script) rejects label keys outside the approved set `{route,method,status,reason,source,class}`.
* **Cardinality guard:** Canary test fires `1000` unique keys and asserts **no** metric label cardinality explosion (routes must be templated).
* **No sleeps for readiness:** Lints/tests deny `sleep`-based readiness; only readiness DAG controls `/readyz`.
* **Docs hygiene:** This doc reviewed at **≤ 90-day** cadence; PRs that change observability require **doc update** in the diff.

---

## 7) Dashboards (starter layout)

1. **Service Overview**

   * p95/p99 `request_latency_seconds` by `route`
   * 5xx and reject rates stacked
   * Inflight vs. concurrency cap (gauge & horizontal line)
2. **Resolver Health**

   * `index_cache_hits_total` vs `index_cache_misses_total` (rates + ratio)
   * `index_providers_returned` distribution
   * `index_integrity_fail_total` (should be flat at 0)
3. **DHT Panel**

   * `dht_lookup_ms` (heatmap)
   * `dht_errors_total` and `dht_hedged_requests_total{result=*}`
4. **Admission & Quotas**

   * `rejected_total{reason}` (stacked area)
   * `quota_exhaustions_total{dimension=*}`
5. **Readiness Timeline**

   * Derived timeseries of readiness keys (`0/1`), with annotations for flips and deploys.

---

## 8) Scrape & Exporter Tips

* Prefer **pull** via `/metrics`. If running behind a gateway, ensure `svc-index`’s metrics endpoint is **not** cached and returns `Content-Type: text/plain; version=0.0.4`.
* If OTEL is enabled, keep **metric names identical** and add OTEL attributes as a **superset**; do not fork naming.
* Scrape intervals: `15s` production; `5s` for perf tests.
* Set `honor_labels: true` and disallow relabeling that injects high-cardinality labels.

---

## 9) Field Guide (common “why is X happening?”)

* **High 5xx but low rejects** → suspect downstream (`svc-dht` timing out) or DB corruption. Check `dht_errors_total`, `index_integrity_fail_total`.
* **Reject spike (`over_capacity`)** → admission too tight or load spike. Check `inflight_requests`, raise caps if healthy, or enable shed.
* **Cache miss ratio climbing** → manifest churn or cache eviction too aggressive. Tune TTL/size; confirm BLAKE3 keys stable.
* **p95 > SLO** → look for hedge **losses**, slow `dht_lookup_ms`, or noisy neighbor (CPU). Consider widening hedge or enabling priority lanes.

---

## 10) Appendix — JSON Shapes

### 10.1 `/readyz` (ready)

```json
{ "ready": true, "keys": ["config_loaded","db_open","bus_attached","dht_client_ready","listeners_bound","metrics_live"] }
```

### 10.2 `/readyz` (degraded)

```json
{ "ready": false, "missing": ["dht_client_ready"], "degraded": true, "retry_after": 5 }
```

---

**Definition of Done (Observability, `svc-index`):**

* Golden metrics + `svc-index` specifics are implemented with **stable label sets**.
* `/healthz` and `/readyz` semantics match §2.
* Logs are JSON-only, secrets-free, and correlate via `corr_id`.
* Traces follow the span schema; DHT hedging is visible.
* SLO dashboards + alerts exist with linked runbooks.
