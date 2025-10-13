````markdown
# 📈 OBSERVABILITY.md — svc-mailbox

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

- Metrics (Prometheus/OTEL)
- Health & readiness semantics
- Logs (JSON schema, fields)
- Tracing spans & correlation
- Alerts & SLOs

Everything here maps 1:1 to the IDB: at-least-once delivery, bounded queues, integrity checks, capability gating, replay window, visibility timers, and DLQ policy.

---

## 1) Metrics (Prometheus-style)

> **Label policy (critical):** keep **low cardinality**. Allowed labels: `route`, `method`, `status`, `queue`, `shard`, `endpoint`, `reason`, `op`, `topic_class` (coarse bucket like `user_inbox`, **not** raw topic), `profile` (`micronode|macronode`). Never put `msg_id`, `idem_key`, `corr_id`, or raw `topic` in labels.

### 1.1 Golden Metrics (every service)

- `http_requests_total{route,method,status}` (Counter)
- `request_latency_seconds_bucket{route,method,le}` (Histogram + `_sum`, `_count`)
- `inflight_requests{route}` (Gauge)
- `service_restarts_total` (Counter) — supervisor restarts
- `rejected_total{reason}` (Counter) — `unauth|oversize|ratio_cap|saturated|schema|integrity`
- `bus_lagged_total` (Counter) — broadcast backlog dropped (loss-tolerant control bus)

### 1.2 Mailbox-Specific

**Queues & Backpressure**
- `queue_depth{queue,shard}` (Gauge) — `queue ∈ {work,requeue}`
- `saturation{queue,shard}` (Gauge 0..1)
- `busy_rejections_total{endpoint}` (Counter) — 429s at ingress
- `queue_dropped_total{queue}` (Counter) — if any internal shed happens (should be rare/never on work)

**Delivery & Visibility**
- `mailbox_enqueued_total{topic_class}` (Counter)
- `mailbox_delivered_total{topic_class}` (Counter) — successful ACKs
- `mailbox_redelivered_total{topic_class}` (Counter) — post-visibility reappearances (attempt > 1)
- `mailbox_visibility_timeout_total{topic_class}` (Counter)
- `mailbox_attempts_histogram{topic_class}` (Histogram, integer buckets 1..N)

**Latency**
- `enqueue_latency_seconds` (Histogram) — accept → persisted/ready
- `dequeue_latency_seconds` (Histogram) — RECV request → first envelope emitted
- `ack_commit_latency_seconds` (Histogram) — ACK call → removal from inflight

**Integrity & DLQ**
- `integrity_fail_total{reason}` (Counter) — `payload_hash|hash_chain|sig`
- `mailbox_dlq_total{topic_class,reason}` (Counter)
- `mailbox_dlq_reprocess_total{topic_class}` (Counter)

**Auth & Capabilities**
- `auth_failures_total{scope}` (Counter) — `scope` like `send|recv|ack|nack|admin`
- `cap_cache_entries` (Gauge)
- `cap_cache_evictions_total{reason}` (Counter) — `expired|revoked|error`

**Ordering (Acceptance Gate)**
- `reorder_depth_max{shard}` (Gauge) — CI asserts `≤ 32`

**Scanner & Housekeeping**
- `scanner_wake_total` (Counter)
- `scanner_wake_jitter_ms` (Summary)
- `requeue_backoff_total` (Counter)

**Example registration discipline**
- All metrics constructed once in `Metrics::new()` and **cloned** into tasks; CI test greps for a single constructor path to prevent duplicate registration.

### 1.3 PromQL Snippets

- **Backpressure headroom**
  ```promql
  avg_over_time(1 - saturation{queue="work"}[5m]) * 100
````

* **Visibility health**

  ```promql
  rate(mailbox_visibility_timeout_total[5m]) / rate(mailbox_enqueued_total[5m])
  ```
* **Integrity incidents**

  ```promql
  sum(integrity_fail_total) by (reason)
  ```
* **DLQ rate**

  ```promql
  rate(mailbox_dlq_total[15m]) > 0
  ```

---

## 2) Health & Readiness

### 2.1 Endpoints

* `GET /healthz` — **liveness** only; returns `200 OK` if process is responsive.
* `GET /readyz` — **readiness**; returns `200 OK` only when all keys are good; otherwise `503`.

### 2.2 Readiness Keys (mailbox-specific)

* `config_loaded`
* `listeners_bound` (HTTP/UDS)
* `metrics_ready`
* `cap_authority_ok` (capability validator reachable, cache warmed)
* `profile_ok` (DLQ backend matches profile: durable for macronode, ephemeral for micronode)
* `scanner_ok` (scanner loop ticker alive)
* `clock_skew_ok` (monotonic + wall-clock sanity within ±2m window)
* `shards_ready` (all shard workers running; mpsc capacity initialized)

### 2.3 Failure Semantics

* **Degraded mode**: **fail-open reads; fail-closed writes** (IDB).
  `/readyz` returns `503` and body:

  ```json
  { "degraded": true, "missing": ["cap_authority_ok","shards_ready"], "retry_after": 30 }
  ```
* Write endpoints (`/v1/send`) return `429/503` with `Retry-After` when saturation or a readiness key fails. Read endpoints may continue if safe.

---

## 3) Logs

### 3.1 Format

* JSON lines (`application/jsonl`), one event per line.
* **Required fields**

  * `ts` (RFC3339 with millis)
  * `level` (`TRACE|DEBUG|INFO|WARN|ERROR`)
  * `service` (`svc-mailbox`)
  * `event` (`send.accepted`, `recv.batch`, `ack.ok`, `ack.unknown`, `dlq.move`, `auth.denied`, `scanner.requeue`, `integrity.fail`, `readyz.state`)
  * `reason` (aligned with `rejected_total{reason}`)
  * `corr_id` (UUIDv7; generated or propagated)
  * `latency_ms` (when applicable)
  * `shard` (integer)
  * `topic_class` (coarse bucket — never raw topic)
  * `remote_addr`
  * `profile` (`micronode|macronode`)
* **Never log**: macaroon contents, payloads, keys, raw `topic`, `msg_id`, `idem_key`. Emit them only as **hashed/truncated** debug under a feature flag if absolutely necessary.

**Examples**

* Successful enqueue:

  ```json
  {"ts":"2025-10-12T18:00:01.234Z","level":"INFO","service":"svc-mailbox","event":"send.accepted","corr_id":"018f1a2a-6b45-7c7c-b80e-9ef2a5b1f22a","latency_ms":3,"topic_class":"user_inbox","shard":12,"profile":"macronode","remote_addr":"203.0.113.10:53211"}
  ```
* Rejection (saturated):

  ```json
  {"ts":"2025-10-12T18:00:06.011Z","level":"WARN","service":"svc-mailbox","event":"send.rejected","reason":"saturated","corr_id":"018f1a2a-6b45-7c7c-b80e-9ef2a5b1f22a","saturation":0.94,"queue":"work","shard":3}
  ```
* Integrity failure:

  ```json
  {"ts":"2025-10-12T18:00:10.500Z","level":"ERROR","service":"svc-mailbox","event":"integrity.fail","reason":"hash_chain","corr_id":"018f1a2a-6b45-7c7c-b80e-9ef2a5b1f22a","shard":7,"topic_class":"user_inbox"}
  ```

### 3.2 Redaction & Secrets

* Payload bytes are **never** logged.
* Cap tokens/keys are never logged. Errors reference capability **scope** only.
* Config diffs in readiness/rotate logs redact secret values and paths.

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` (JSON) with optional OTEL exporter (feature `otel`).
* **Span naming**

  * `svc.mailbox.send`
  * `svc.mailbox.recv`
  * `svc.mailbox.ack`
  * `svc.mailbox.nack`
  * `svc.mailbox.scanner`
  * `svc.mailbox.dlq.reprocess`
* **Span attributes (stable keys)**

  * `corr_id`, `shard`, `topic_class`, `attempt`, `visibility_ms`, `profile`
  * `auth.scope` (`send|recv|ack|nack|admin`), `auth.result`
* **Correlation IDs**

  * Ingress: accept `X-Corr-Id` or generate UUIDv7; always emit & propagate in responses/logs.
  * Propagate across mailbox → downstream (e.g., cap authority) via header or metadata.

---

## 5) Alerts & SLOs

### 5.1 SLOs (Mailbox)

* **Local latency**: `p95(enqueue_latency_seconds) < 50ms` and `p95(dequeue_latency_seconds) < 50ms` (intra-region).
* **Fanout budget** (via svc-mod): `p95 < 2s` for 10→10k follower push.
* **Durability**: `mailbox_visibility_timeout_total / mailbox_enqueued_total < 0.01` (steady-state).
* **Error budget**

  * 5xx rate `< 0.1%`
  * 429/503 `< 1%` sustained
  * `integrity_fail_total` **= 0** (any occurrence is critical)

### 5.2 Alert Rules (PromQL examples)

* **Sustained saturation**

  ```promql
  avg_over_time(saturation{queue="work"}[10m]) > 0.75
  ```

  *Warn at 0.75 for 10m; Critical at 0.9 for 5m.*

* **Visibility leak**

  ```promql
  rate(mailbox_visibility_timeout_total[15m]) / ignoring(reason) rate(mailbox_enqueued_total[15m]) > 0.02
  ```

* **DLQ spike**

  ```promql
  increase(mailbox_dlq_total[10m]) > 100
  ```

* **Auth failures**

  ```promql
  rate(auth_failures_total[5m]) > 5
  ```

* **Integrity failure**

  ```promql
  increase(integrity_fail_total[5m]) > 0
  ```

  *Immediate critical page.*

* **Readiness flapping**

  ```promql
  changes(up{job="svc-mailbox"}[10m]) > 2
  ```

### 5.3 Runbooks (link placeholders)

* **Saturation** → scale shards / raise capacity / check downstream; see `RUNBOOK.md#saturation`.
* **Visibility leak** → inspect scanner logs; verify monotonic clock; check long GC pauses.
* **DLQ spike** → identify `reason` breakdown; triage consumer parse errors; consider reprocess run.
* **Auth failures** → check cap authority reachability; verify rotation; `cap_cache_ttl` behavior.

---

## 6) CI / Enforcement

* Unit test ensures `/metrics`, `/healthz`, `/readyz` are registered.
* Golden-metrics test: verify presence of:

  * `queue_depth`, `saturation`, `busy_rejections_total`, `mailbox_*`, `integrity_fail_total`, `reorder_depth_max`.
* No duplicate metric registration (grep-based CI).
* Lints: `await_holding_lock` warn; readiness won’t rely on `sleep`.
* Docs refresh check (commit hook) every 90 days.

---

## 7) Example `/metrics` Exposition (truncated)

```
# HELP queue_depth Current queue depth
# TYPE queue_depth gauge
queue_depth{queue="work",shard="0"} 12
queue_depth{queue="requeue",shard="0"} 0

# HELP saturation Queue saturation (0..1)
# TYPE saturation gauge
saturation{queue="work",shard="0"} 0.38

# HELP mailbox_dlq_total Messages quarantined into DLQ
# TYPE mailbox_dlq_total counter
mailbox_dlq_total{topic_class="user_inbox",reason="parse_error"} 42

# HELP integrity_fail_total Integrity verification failures
# TYPE integrity_fail_total counter
integrity_fail_total{reason="hash_chain"} 1
```

---

## 8) Dashboards (suggested panels)

1. **Traffic & Errors**

   * `http_requests_total` by `route,status`
   * 5xx %, 429/503 %
2. **Latency**

   * `request_latency_seconds` p50/p95 by route
   * `enqueue_latency_seconds` & `dequeue_latency_seconds` p95
3. **Queues**

   * `queue_depth{queue,shard}`
   * `saturation{queue,shard}` with thresholds
4. **Delivery Health**

   * `mailbox_delivered_total` vs `mailbox_enqueued_total`
   * `mailbox_visibility_timeout_total` rate
   * `mailbox_redelivered_total` (watch for regressions)
5. **Integrity & DLQ**

   * `integrity_fail_total` (singlestat)
   * `mailbox_dlq_total` by `reason`
6. **Auth**

   * `auth_failures_total` by `scope`
   * `cap_cache_entries`, `cap_cache_evictions_total`

---

## 9) OTEL (optional)

* Feature `otel` enables:

  * Trace export (OTLP/gRPC) to collector; map `corr_id` to `trace_id` when provided.
  * Metric export for histograms as OTel exponential histograms (experimental).
* Resource attributes: `service.name=svc-mailbox`, `service.version`, `service.instance.id`.

---

## 10) Field Reference (stable names)

| Domain    | Name                              | Type         | Notes                                           |
| --------- | --------------------------------- | ------------ | ----------------------------------------------- |
| Log       | `event`                           | string       | canonical set listed above                      |
| Log/Trace | `corr_id`                         | string       | UUIDv7                                          |
| Log/Trace | `topic_class`                     | string       | coarse bucket only                              |
| Log/Trace | `shard`                           | int          |                                                 |
| Metric    | `queue_depth{queue,shard}`        | gauge        |                                                 |
| Metric    | `saturation{queue,shard}`         | gauge        | 0..1                                            |
| Metric    | `busy_rejections_total{endpoint}` | counter      |                                                 |
| Metric    | `mailbox_*`                       | counter/hist | enqueued/delivered/redelivered/dlq/visibility/… |
| Metric    | `integrity_fail_total{reason}`    | counter      |                                                 |
| Metric    | `reorder_depth_max{shard}`        | gauge        | CI checks ≤ 32                                  |

---

## 11) Red/Yellow/Green (quick triage)

* **Green:** saturation < 0.5; p95 enqueue/dequeue < 40ms; 5xx < 0.05%; DLQ flat; integrity=0
* **Yellow:** saturation 0.5–0.75; visibility timeouts rising; small DLQ growth; 429s > 0.5%
* **Red:** saturation > 0.9 (≥ 5m); integrity > 0; DLQ spikes; 5xx > 0.5%; readiness flapping

---

✅ With this spec, `svc-mailbox` emits **consistent, low-cardinality, SLO-grade** telemetry. It is directly mappable to Grafana dashboards and CI assertions, and it provides auditors the end-to-end proof chain from invariant → metric → alert → runbook.

```
```
