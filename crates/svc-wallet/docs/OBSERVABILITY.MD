# 📈 OBSERVABILITY.md — `svc-wallet`

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

This file aligns with `docs/IDB.md` (invariants), `docs/CONFIG.md` (limits/timeouts), and the Concurrency model.

---

## 1) Metrics (Prometheus-style)

> **Cardinality law:** keep label sets small and bounded. No unbounded labels (e.g., `txid`, `account`) on hot-path timeseries.

### 1.1 Golden Metrics (service-agnostic)

* `http_requests_total{route,method,status}` — **Counter**
  Emitted by Axum/Tower layer. `route` uses templated path (`/v1/transfer`, not IDs).

* `request_latency_seconds{route,method}` — **Histogram**
  Buckets: `0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2, 5`. Use exemplars (if OTEL enabled).

* `inflight_requests{route}` — **Gauge**
  From concurrency-limit middleware.

* `bus_lagged_total{bus}` — **Counter**
  Broadcast overflow drops (e.g., `{bus="ledger"|"audit"}`).

* `service_restarts_total{kind}` — **Counter**
  Supervisor restarts of workers; `{kind="worker"|"listener"|"metrics"}`.

* `rejected_total{reason}` — **Counter**
  Reasons include: `unauth`, `policy_denied`, `insufficient_funds`, `limits_exceeded`, `bad_request`, `busy`, `timeout`, `upstream_unavailable`.

### 1.2 Wallet-Specific Metrics

**Requests & Outcomes**

* `wallet_requests_total{op}` — **Counter**
  `{op="issue"|"transfer"|"burn"|"balance"|"receipt"}`.

* `wallet_rejects_total{reason}` — **Counter**
  Mirrors error taxonomy; MUST line up with logs’ `reason` field.

* `wallet_idem_replays_total` — **Counter**
  Replays served from idempotency store (should rise under client retries).

**Ledger/Policy/Auth**

* `upstream_calls_total{svc,op,outcome}` — **Counter**
  `{svc="ledger"|"auth"|"policy", op="verify"|"commit"|"policy_check", outcome="ok"|"error"|"timeout"}`.

* `upstream_latency_seconds{svc,op}` — **Histogram**
  Same buckets as request latency.

* `circuit_open_total{svc}` — **Counter**
  Circuit breaker opens per upstream.

**Economics/Integrity**

* `wallet_conservation_checks_total{outcome}` — **Counter**
  `{outcome="pass"|"fail"}` should be overwhelmingly `pass`.

* `wallet_nonce_conflicts_total` — **Counter**
  Attempts to reuse `(account,nonce)` rejected at reservation.

**Queues & Workers**

* `queue_depth{queue}` — **Gauge**
  `{queue="work"|"events"}` sampled.

* `queue_dropped_total{queue}` — **Counter**
  Drop-on-overflow counts.

* `busy_rejections_total{endpoint}` — **Counter**
  Listener returned 429 due to full queue.

* `worker_inflight` — **Gauge**
  Number of currently active worker tasks.

**Security/TLS**

* `tls_handshake_failures_total` — **Counter**
* `auth_failures_total{stage}` — **Counter**
  `{stage="extract_caps"|"verify_caps"}`.

### 1.3 Registration Discipline

* Metrics **registered once** in `metrics::new()`; pass handles via `Arc<Metrics>`.
* CI checks duplicate registration symbol names.
* **Units**: encode in metric names when ambiguous (e.g., `_seconds`, `_bytes`).

### 1.4 Example `/metrics` Snippet

```
# HELP wallet_requests_total Count of wallet ops by type
# TYPE wallet_requests_total counter
wallet_requests_total{op="transfer"} 12345

# HELP upstream_calls_total Upstream RPCs
# TYPE upstream_calls_total counter
upstream_calls_total{svc="ledger",op="commit",outcome="ok"} 12200
upstream_calls_total{svc="ledger",op="commit",outcome="timeout"} 17
```

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness**: `200 OK` if the process and main loops are alive.
* `GET /readyz` — **readiness**: `200 OK` only when:

  * config loaded & validated
  * listeners bound
  * upstream **probes OK** or breakers **closed** (ledger/auth/policy)
  * work queue not saturated
  * metrics exporter responsive

### 2.2 Readiness Keys (JSON body when not ready)

* `cfg_loaded` (bool)
* `listener_bound` (bool)
* `ledger_ready` (bool)
* `auth_ready` (bool)
* `policy_ready` (bool)
* `queue_ok` (bool)
* `breaker` (list of `{svc, state}` when open)

### 2.3 Failure Semantics

* **Fail-open reads / fail-closed writes**

  * `GET /v1/balance` may remain `200` during partial degradation.
  * `POST /v1/{issue,transfer,burn}` must return `503` when readiness preconditions fail.
* Degraded response body:

  ```json
  { "degraded": true, "missing": ["ledger_ready"], "retry_after": 2 }
  ```

---

## 3) Logs

### 3.1 Format

* JSONL (`application/jsonl`), one event per line.
* **Required fields** (hot path):

  * `ts` — ISO8601 with millis
  * `level` — `INFO|WARN|ERROR|DEBUG|TRACE`
  * `service` — `"svc-wallet"`
  * `event` — e.g., `"wallet.transfer.accepted"`, `"readyz.fail"`
  * `reason` — (when error) aligned with `wallet_rejects_total{reason}`
  * `corr_id` — UUID/ULID; use `X-Corr-ID` or generate
  * `peer_addr` — remote IP:port or UDS peer
  * `method` / `route` / `status`
  * `latency_ms` — request wall time
  * `amnesia` — `true|false`
  * `pq_hybrid` — `true|false` (build/runtime posture)

**Context fields** (safe, no secrets):
`op`, `asset`, `amount_minor`, `nonce`, `idem`, `txid` (only after commit), `account_hash` (hashed/pseudonymous).

### 3.2 Redaction & Secrets

* Never log `Authorization` header or macaroon contents.
* Redact secrets in config diffs.
* Avoid raw `account` or PII; use stable hashed IDs if needed for correlation.

### 3.3 Examples

**Accept (transfer):**

```json
{"ts":"2025-10-16T16:25:41.120Z","level":"INFO","service":"svc-wallet","event":"wallet.transfer.accepted","corr_id":"01JF...","method":"POST","route":"/v1/transfer","status":200,"latency_ms":34,"op":"transfer","asset":"ron","amount_minor":"250000","nonce":42,"idem":"8b4f...","txid":"tx_01JF...","amnesia":false,"pq_hybrid":true}
```

**Reject (busy):**

```json
{"ts":"2025-10-16T16:26:02.003Z","level":"WARN","service":"svc-wallet","event":"wallet.request.rejected","reason":"busy","method":"POST","route":"/v1/transfer","status":429,"corr_id":"01JF...","queue":"work","depth":512}
```

---

## 4) Tracing & Correlation

* Use `tracing`/`tracing-subscriber` with JSON formatter; **no ANSI** in prod.
* **Span naming:** `svc.wallet.<operation>`
  Examples: `svc.wallet.transfer`, `svc.wallet.ledger.commit`.
* **Span fields:** `corr_id`, `op`, `asset`, `nonce`, `idem`, `pq_hybrid`, `amnesia`.
* **Correlation ID:**

  * Ingest from `X-Corr-ID`; if missing, generate ULID and echo as response header.
  * Propagate to upstreams as `x-corr-id` and into audit events.
* **OTEL (feature-flag):**

  * Export spans to OTLP; attach **exemplars** to latency histograms when available.

---

## 5) Alerts & SLOs

### 5.1 SLOs (wallet)

* **Availability (writes):** ≥ 99.9% monthly for `POST /v1/{issue,transfer,burn}`.
* **Latency (reads):** p95 `< 120 ms` intra-region; p99 `< 250 ms`.
* **Error rate:** 5xx `< 0.1%`; `429|503` `< 1%` sustained.
* **Invariant health:** `wallet_conservation_checks_total{outcome="fail"} = 0` (hard SLO).

### 5.2 Example Alert Rules

* **Readiness flip:**

  * `readyz_status == 0 for 2m` → page.

* **Queue pressure:**

  * `rate(busy_rejections_total[5m]) > 50` **AND** `queue_depth{queue="work"} > 400` → warn; > 480 → critical.

* **Upstream instability:**

  * `rate(upstream_calls_total{outcome="error"}[5m]) / rate(upstream_calls_total[5m]) > 0.05` → warn.
  * `increase(circuit_open_total{svc="ledger"}[10m]) > 0` → critical.

* **Invariant breach (should be impossible):**

  * `increase(wallet_conservation_checks_total{outcome="fail"}[1h]) > 0` → **critical**.

* **Security posture:**

  * `increase(auth_failures_total[10m]) > 100` → investigate flood/attack.

### 5.3 Runbooks

Each alert links to `RUNBOOK.md` with:

* **What it means** (plain language)
* **Immediate actions** (flip traffic/readiness, scale, restart)
* **Diagnostics** (graphs, logs, commands)
* **Rollback/mitigation** steps

---

## 6) CI / Enforcement

* CI asserts presence of `/metrics`, `/healthz`, `/readyz` handlers.
* Tests scrape metrics in integration and assert movement:

  * `wallet_requests_total` increments on happy path
  * `wallet_rejects_total{reason="insufficient_funds"}` increments on overdraft tests
  * `wallet_idem_replays_total` increments on replay suite
* Lints ensure **no secrets** in logs and forbid `await_holding_lock`.
* Docs review every **90 days** or on material changes to metrics/logs/traces.

---

## 7) Dashboards (starter pack)

* **Overview:** request rate, error rate, p95 latency, inflight, queue depth.
* **Writes health:** per-op rate/latency, 4xx/5xx split, busy rejections, circuit opens.
* **Upstreams:** ledger/auth/policy latency and error rate; breaker state.
* **Invariant panel:** conservation checks, nonce conflicts, idem replays.
* **Security:** auth failures, TLS handshakes.

---

## 8) Sampling & Privacy

* **Trace sampling:** head-based 1–5% on **reads**, 100% on **writes** (until volume requires tail sampling).
* **PII hygiene:** do not place raw `account` in logs/labels; if necessary, use a stable salted hash only in logs (never metrics).
* **Receipts:** `txid` appears in logs **only after** successful commit; never in metric labels.

---

## 9) Implementation Notes

* **Metrics crate:** centralized `metrics::Metrics` with constructors for counters/gauges/histograms; expose via Axum `/metrics`.
* **Tracing:** set global default subscriber at main; configure JSON, UTC timestamps, and field filters by level.
* **Middleware order:** request ID → metrics → timeouts → decompression cap → body limit → auth → handler.

---

## 10) Examples (copy/paste)

**Add a counter and latency timer in handler:**

```rust
let _guard = metrics.latency_transfer.start_timer(); // RAII histogram timer
metrics.requests_total.with_label_values(&["transfer"]).inc();
```

**Expose /readyz with JSON detail on failure:**

```rust
if !ready.ready() {
    return (StatusCode::SERVICE_UNAVAILABLE, Json(ready.snapshot())).into_response();
}
```

**Inject correlation ID and echo:**

```rust
let corr = header_or_generate_corr_id(&headers);
res.headers_mut().insert("X-Corr-ID", corr.clone());
tracing::Span::current().record("corr_id", &tracing::field::display(&corr_str));
```

---

## 11) Versioning & Governance

* Any **metric name** or **label set** change is a semver-impacting change for dashboards/alerts; document in `CHANGELOG.md`.
* Keep this file in PR scope for:

  * new endpoints
  * new invariants
  * changes to limits/backpressure/queues

---

**With this observability spec, `svc-wallet` is debuggable under fire, auditable for economics, and measurable against SLOs—without leaking secrets or exploding cardinality.**
