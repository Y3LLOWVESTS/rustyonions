````markdown
# 📈 OBSERVABILITY.md — svc-admin

*Audience: developers, operators, auditors*  
*MSRV: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

`svc-admin` is the **admin-plane GUI** and API façade for RON-CORE nodes. It talks only to node **admin HTTP surfaces** (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`, and documented admin endpoints), and it MUST NOT invent hidden control channels or shadow state.

This document defines **what we observe** and **how we expose it** for:

* Metrics (Prometheus / OTEL)
* Health & readiness (for svc-admin itself, and how we reflect node health truthfully)
* Logs (JSON schema, fields, redaction rules)
* Tracing spans & correlation across svc-admin ↔ nodes
* Alerts & SLOs (what “healthy” looks like for operators at 3 a.m.)

Key constraints:

* **Short-horizon metrics only** — svc-admin keeps an in-memory window of node metrics; no TSDB, no long-lived historical store.  
* **Truthful readiness** — svc-admin **never** reports a node/plane as “ready” if that node’s `/readyz` says otherwise.  
* **Read-only by default** — observability-first, no actions unless explicitly enabled.  

---

## 1) Metrics (Prometheus-style)

### 1.0 Metric namespace & exposure

* All metrics are exposed via **Prometheus text** on `/metrics` bound to `metrics_addr` (`SVC_ADMIN_METRICS_ADDR`).  
* We follow a **global golden-metrics schema** across services, with service-specific metrics prefixed with `svc_admin_…`.
* Metrics server binding is **separate** from the UI/API bind (`bind_addr`) to allow localhost-only scrapes in production.  

Cardinality constraints:

* `route` labels should be **templated paths** (e.g., `/api/nodes/{id}/status`), not raw IDs.
* Metrics that carry `node_id` are limited to the set of **configured nodes** (small, explicitly curated set).  

---

### 1.1 Golden Metrics (every service, svc-admin flavor)

These exist for **svc-admin itself**, independent of node metrics:

* `http_requests_total{route,method,status}` (Counter)  
  * All inbound HTTP requests to svc-admin (SPA assets + JSON APIs).
* `request_latency_seconds{route,method}` (Histogram)  
  * Duration from request accept to response send, used for p50/p95/p99.
* `inflight_requests{route}` (Gauge)  
  * Requests currently in-flight; can be implied if we rely on concurrency limits instead.
* `bus_lagged_total` (Counter)  
  * For any internal broadcast channels we use (e.g., `events_tx`), increment when listeners lag and we drop messages.  
* `service_restarts_total{service="svc-admin"}` (Counter)  
  * Incremented by the supervisor on restart loops (if/when we supervise svc-admin as a child, or from a higher-level supervisor).
* `rejected_total{reason}` (Counter)  
  * Global rejection counter, with reasons aligned to API error mapping (`"busy"`, `"timeout"`, `"invalid"`, etc.).  
  * **Security-relevant reasons** like `"unauth"`, `"forbidden"`, `"oversized"` use the more specific `svc_admin_rejected_total{reason=…}` below.  

We also inherit **security metrics** from `SECURITY.md` as first-class golden metrics for svc-admin:

* `svc_admin_rejected_total{reason="unauth"|"forbidden"|"oversized"|"timeout"}`  
* `svc_admin_auth_failures_total{scope="ui"|"node"|"passport"}`  
* `svc_admin_tls_handshake_failures_total{peer="node"|"client"}`  
* `svc_admin_csrf_failures_total` (future)  
* `svc_admin_upstream_errors_total{kind="connect"|"timeout"|"parse"}`  

These are the **primary alerting signals** for auth and upstream/node problems (see §5).

---

### 1.2 Service-Specific Metrics

svc-admin’s job is to **visualize nodes**. Internally we track:

#### Node inventory & status

* `svc_admin_nodes_total{environment}` (Gauge)  
  * Count of configured nodes by environment (dev/staging/prod).  
* `svc_admin_node_up{node_id,environment}` (Gauge: 0/1)  
  * Derived from node `/healthz` responses.
* `svc_admin_node_ready{node_id,environment}` (Gauge: 0/1)  
* `svc_admin_node_degraded{node_id,plane}` (Gauge: 0/1)  
  * Derived strictly from node `/readyz` and status JSON; svc-admin **never overrides** node readiness.  
* `svc_admin_node_amnesia{node_id}` (Gauge: 0/1)  
  * Reflects `amnesia: bool` in `AdminStatusView` and is surfaced clearly in the UI.  

#### Metrics sampling (short-horizon rings)

Based on the **per-node metrics sampler** design:  

* `svc_admin_metrics_samples_total{node_id}` (Counter)  
  * Successful polls of `/metrics` per node.
* `svc_admin_metrics_parse_errors_total{node_id}` (Counter)  
  * Prometheus parsing failures (malformed or unexpected content).
* `svc_admin_metrics_staleness_seconds{node_id}` (Gauge)  
  * Time since last successful update of the ring buffer; used to detect stale charts.
* `svc_admin_metrics_window_seconds` (Gauge)  
  * Effective horizon of the in-memory ring, derived from `polling.metrics_window`.  
* `svc_admin_metrics_body_rejected_total{reason="too_large"|"decompression_ratio"}` (Counter)  
  * Rejections due to size/ratio caps.  

#### Auth & config

* `svc_admin_auth_mode{mode}` (Gauge: 0/1 per mode, exactly one 1)  
  * Exposes current auth configuration (`none | ingress | passport`).  
* `svc_admin_auth_refresh_total{result="ok"|"error"}` (Counter)  
  * Success/failure of JWK/issuer metadata refresh.
* `svc_admin_config_reloads_total{result="ok"|"error"}` (Counter; future when reload exists)  
  * Correlates with optional config-watcher.  

#### Internal queues & backpressure

From the channel/backpressure inventory:  

* `svc_admin_busy_rejections_total{queue}` (Counter)  
  * Increments when `try_send` on bounded queues fails (e.g., job queue, auth-refresh queue).  
* `svc_admin_events_lagged_total` (Counter)  
  * When `events_tx` drops messages for slow listeners, mirror this into `bus_lagged_total` while also keeping a service-specific counter.

#### Dev-only app-plane playground

If `[ui.dev.enable_app_playground = true]`, the dev playground (backed by `ron-app-sdk-ts`) may emit **app-plane request metrics** (e.g., `svc_admin_app_playground_requests_total`). :contentReference[oaicite:23]{index=23}  

These are clearly labeled as **dev-only** and MUST NOT be relied on for production alerting.

---

### 1.3 Registration Discipline

* All metrics are **registered exactly once** in `Metrics::new()` for svc-admin.
* Metric handles (Counters, Histograms, Gauges) are cloned into handlers, samplers, and auth middleware; no duplicate registration in “leaf” code.
* CI **greps for `register_` / `lazy_static!` / `once_cell::sync::Lazy`** to ensure:
  * only one registration site,
  * no per-request registration,
  * no dynamic metric names from user input.

---

## 2) Health & Readiness

### 2.1 Endpoints

svc-admin exposes the standard triad:

* `GET /healthz` — **liveness**: returns `200 OK` if the process is up and the supervisor loop is running.
* `GET /readyz` — **readiness**: returns `200 OK` only once core dependencies are ready:
  * config loaded,
  * HTTP listener bound,
  * metrics listener bound (if configured),
  * auth mode initialized,
  * node registry loaded,
  * shutdown not in progress.  
* `GET /metrics` — Prometheus endpoint for svc-admin’s own metrics (as described in §1), bound to `metrics_addr`.  

**Important:**

* `/readyz` reflects **svc-admin’s own ability** to serve admin requests, not whether all nodes are healthy.
* Node health/readiness is surfaced via **admin APIs** (e.g., `/api/nodes/{id}/status`) and UI tiles, derived strictly from node endpoints (no heuristic overrides).  

---

### 2.2 Readiness Keys (svc-admin)

Internally, we model readiness as a small `HealthState` with boolean keys like:

* `config_loaded` — initial config parsed and validated.  
* `http_listener_bound` — main Axum listener is bound to `bind_addr`.  
* `metrics_listener_bound` — metrics listener bound (if `metrics_addr` != `0`).  
* `auth_ready` — auth middleware initialized according to `auth.mode`, with any required keys cached.  
* `node_registry_loaded` — Node registry built from `[nodes.*]` config entries.  
* `samplers_running` — if metrics sampling is enabled, at least one successful poll occurred per configured node within `polling.metrics_window`.  

`/readyz` encodes:

```jsonc
{
  "ready": true | false,
  "missing": ["config_loaded", "auth_ready"],
  "degraded": ["samplers_running"],
  "details": {
    "config_loaded": true,
    "http_listener_bound": true,
    "metrics_listener_bound": true,
    "auth_ready": false,
    "node_registry_loaded": true,
    "samplers_running": false
  }
}
````

* `ready = true` only when **all non-optional keys are true**.
* Optional components (like samplers) can be expressed in `degraded` without flipping readiness to false.

---

### 2.3 Failure Semantics

* **Fail-open reads / fail-closed writes**:

  * Read-only admin views (default) tolerate sampler failures and node connectivity issues by returning explicit upstream errors without crashing the service.
  * Any future mutating actions (reload/shutdown) follow node-level policy and fail closed on auth or upstream errors.
* When svc-admin itself is degraded but still partially useful:

  * `/readyz` returns `503` with JSON:

    ```jsonc
    {
      "ready": false,
      "degraded": true,
      "missing": ["auth_ready"],
      "retry_after": 30
    }
    ```
* When nodes are degraded:

  * `/readyz` for svc-admin can still be `200`, but **admin APIs** and UI clearly show which nodes/planes are failing, based on node `/readyz` and `AdminStatusView`.

---

## 3) Logs

### 3.1 Format & Sink

* **Format:** JSON lines, one event per line (`application/jsonl`).
* Controlled via `log.format` (`json` | `text`); **JSON is strongly recommended in prod.**
* `log.level` (`trace` → `error`) defaults to `info`; `trace` only in dev.

Each log event MUST include at least:

* `ts` — ISO8601 timestamp.
* `service = "svc-admin"`.
* `level` — `INFO|WARN|ERROR|DEBUG|TRACE`.
* `corr_id` — correlation ID for this request/operation (see §4).
* `peer_addr` — client IP where available/appropriate.
* `node_id` — for outbound node calls.
* `action` — high-level operation name:

  * `"proxy_health"`, `"proxy_ready"`, `"proxy_status"`, `"proxy_metrics"`,
  * `"auth_validate"`, `"auth_refresh"`, `"reload"`, `"shutdown"`, `"metrics_sample"`, etc.
* `result` — `"ok"`, `"timeout"`, `"unauth"`, `"forbidden"`, `"error"`, `"upstream_unavailable"`, etc.
* For authenticated operators:

  * `subject` — canonical user ID / email / UUID from SSO.
  * `roles` — coarse roles/groups, e.g. `"admin"`, `"viewer"`.

Where relevant, we also include:

* `latency_ms` — for request spans (mirrors `request_latency_seconds` histogram).
* `reason` — machine-readable reason matching `svc_admin_rejected_total{reason=…}` and `svc_admin_upstream_errors_total{kind=…}`.
* `status_code` — HTTP status returned by svc-admin to the client.
* `upstream_status` — HTTP code returned by the node (if any).
* `metrics_body_bytes` — size of `/metrics` responses that we accept or reject.

---

### 3.2 Redaction & Secrets

**Never log:**

* Access tokens, macaroons, or auth headers.
* Full passports / JWTs.
* Node credentials (paths are safe, contents are not).

Redaction rules:

* Where we must mention a credential, log a **tag** and a truncated fingerprint, e.g.:

  ```json
  {
    "node_id": "macronode-prod",
    "action": "proxy_status",
    "credential_ref": "macaroon_path",
    "credential_fingerprint": "abc123…7890"
  }
  ```

* For env/config snapshots, log **only non-secret fields** (e.g., `auth.mode`, `bind_addr`, `metrics_addr`, `polling.*`, `ui.*`). Values like `client_id`, `issuer`, `audience`, cookie names are okay; tokens and private keys are not.

---

### 3.3 Event Taxonomy

We standardize `action` and `result` fields:

* **Ingress/auth:**

  * `auth_validate` — evaluating `auth.mode`, headers, or passport token.
  * `auth_refresh` — periodic JWK/issuer refresh.
* **Node proxying:**

  * `proxy_health`, `proxy_ready`, `proxy_status`, `proxy_metrics`.
  * `node_action_reload`, `node_action_shutdown` (future).
* **Metrics sampling:**

  * `metrics_sample` — one `/metrics` poll for a node (success).
  * `metrics_sample_error` — poll failed (connect, timeout, parse, oversized), with `reason` and `kind`.
* **Config & lifecycle:**

  * `config_loaded`, `config_reload`, `shutdown_begin`, `shutdown_complete`.

Each error-like event increments the **aligned metric**:

* `result = "unauth"` → `svc_admin_rejected_total{reason="unauth"}`.
* `result = "timeout"` from node → `svc_admin_upstream_errors_total{kind="timeout"}`.

This ensures **metric ↔ log correlation** during incident triage.

---

## 4) Tracing & Correlation

svc-admin uses the `tracing` crate with a JSON `tracing-subscriber` backend (compatible with log shipping systems and optional OTEL exporters).

### 4.1 Span structure

For each inbound HTTP request:

* Root span: `svc.admin.http`

  * Attributes: `route`, `method`, `corr_id`, `peer_addr`, `auth_mode`, `subject` (if any), `roles`, `status_code`, `latency_ms`.
* Child spans:

  * `svc.admin.auth.validate` — auth middleware, before handler.
  * `svc.admin.node.proxy` — for node admin API calls, with `node_id`, `target_url`, `upstream_status`.
  * `svc.admin.metrics.sample` — background sampler spans, with `node_id` and `metrics_body_bytes`.
  * `svc.admin.config.reload` — when we add reload support.

### 4.2 Correlation IDs

* Ingress:

  * Extract `X-Corr-ID` from inbound requests if present; otherwise generate a ULID/UUID.
  * Inject into both **logs** (`corr_id`) and **spans**.
* Egress (to nodes):

  * Forward `X-Corr-ID` header to node admin endpoints.
  * Node logs (and metrics) can then be stitched with svc-admin’s logs using this ID.

### 4.3 OTEL / external tracing (optional)

* An optional `otel` feature can export spans to a collector.
* Export pipeline is **best-effort**:

  * tracing failures must **never** affect request handling or readiness.
* No node credentials or PII are placed in OTEL attributes.

---

## 5) Alerts & SLOs

These are **recommended defaults** for operators; concrete values can be tuned per deployment.

### 5.1 Standard SLOs (svc-admin itself)

* **Availability (UI/API):**

  * 99.9% of `GET /` (SPA) and `/api/*` calls return non-5xx responses.
* **Latency:**

  * p95 `request_latency_seconds{route="/api/nodes/{id}/status"}` < **250 ms** for intra-region nodes, under normal load (small fleets).
* **Error rates:**

  * 5xx rate (`status >= 500`) < **0.5%** of requests.
  * `svc_admin_rejected_total{reason="unauth"|"forbidden"}` rate should be small and stable; spikes often indicate misconfig or auth outage.

### 5.2 Node coverage SLOs

* **Metrics freshness:**

  * `svc_admin_metrics_staleness_seconds{node_id}` ≤ **3 × polling.metrics_interval** for ≥ 99% of samples.
* **Node visibility:**

  * For any production node, `svc_admin_node_up{node_id}` = 1 for ≥ 99.9% of time; if not, treat as an upstream/node issue.
* **Amnesia awareness:**

  * Nodes reporting `amnesia = true` should have visible badges in UI and clear annotations in logs/metrics so operators know persistence guarantees are different.

### 5.3 Example Alerts

Operators are encouraged to wire PromQL alerts along these lines:

* **Auth / Security:**

  * `increase(svc_admin_auth_failures_total[5m]) > 0` (warning), sustained for `> 30m` (critical).
  * `rate(svc_admin_rejected_total{reason="unauth"|"forbidden"}[5m]) > 0.05` (5% of traffic) — investigate SSO/ingress/passport.
* **Upstream / Node health:**

  * `rate(svc_admin_upstream_errors_total[5m]) > 1` ⇒ nodes misbehaving or network issues.
  * `svc_admin_node_ready{environment="prod"} == 0` for any node for > N minutes ⇒ critical incident.
* **Metrics sampling:**

  * `max_over_time(svc_admin_metrics_staleness_seconds[10m]) > (metrics_window / 2)` ⇒ charts are stale; metrics samplers may be failing.
* **Resource & concurrency:**

  * `increase(svc_admin_busy_rejections_total[5m]) > 0` ⇒ admin traffic exceeding configured queue capacity.
  * Frequent restarts (`increase(service_restarts_total{service="svc-admin"}[1h]) > 0`) ⇒ investigate crashes or OOM issues.

Each alert should reference **`RUNBOOK.md`** with:

* how to inspect /healthz, /readyz,
* how to check node connectivity,
* how to examine logs filtered by `corr_id`, `node_id`, `action`.

---

## 6) CI / Enforcement

To keep observability **non-rotted and consistent**, CI should enforce:

* **Endpoint presence:**

  * Integration tests that start svc-admin and assert:

    * `/healthz` returns 200 with minimal JSON.
    * `/readyz` transitions from non-ready → ready once config & listeners are initialized.
    * `/metrics` returns parseable Prometheus text and includes:

      * at least one `http_requests_total` series,
      * `svc_admin_nodes_total`,
      * security metrics like `svc_admin_rejected_total`.
* **Log schema:**

  * Tests that exercise a simple request and assert:

    * `service="svc-admin"`,
    * `corr_id` present,
    * `action` and `result` set,
    * no secrets/PII-like fields in the log JSON.
* **Metrics parser & staleness:**

  * Unit/property tests over synthetic Prometheus inputs to ensure we never panic on malformed lines and we safely skip bad metrics.
  * Tests that simulate sampler timeouts and ensure we:

    * log `metrics_sample_error` events,
    * increment `svc_admin_upstream_errors_total`,
    * increase `svc_admin_metrics_staleness_seconds`,
    * retain bounded memory (ring buffer).
* **Auth mode sanity:**

  * Tests confirming `auth.mode="none"` is dev-only and emits a warning log in non-dev environments, and that ingress/passport modes properly surface 401/403 with aligned security metrics.
* **Loom / concurrency checks (targeted):**

  * Loom models for:

    * metrics store + HTTP handler + shutdown,
    * `AuthState` read/write patterns,
    * verifying no deadlocks and no use-after-shutdown.

Docs:

* `OBSERVABILITY.md`, `SECURITY.md`, `CONFIG.md`, and `CONCURRENCY.md` should be **reviewed at least every 90 days** or whenever we add a major plane/view to svc-admin, to keep observability behavior aligned with the actual code.

---

```
```
