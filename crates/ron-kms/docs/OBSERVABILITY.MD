
---

# 📈 OBSERVABILITY.md — ron-kms

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how we expose it**, and **how it’s used** for:

* Metrics (Prometheus/OTEL)
* Health & readiness semantics
* Logs (JSON schema, fields, redaction)
* Tracing spans & correlation
* Alerts, SLOs, dashboards, and runbooks

`ron-kms` is a custody/signing service. Observability must never leak secrets and must make **tamper-evidence**, **attestation**, **hybrid PQ rollout**, and **degraded verify** status explicit.

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden Metrics (service-common)

* `http_requests_total{route,method,status}` (Counter)
* `request_latency_seconds{route,method}` (Histogram)
* `inflight_requests{route}` (Gauge)
* `rejected_total{reason}` (Counter) — `unauth|backpressure|decompress_cap|ratio_cap|pq_policy|attest_missing|store_unavailable`
* `service_restarts_total` (Counter)
* `bus_lagged_total` (Counter) — broadcast backlog dropped

### 1.2 KMS-Specific Canon

**Ops & Latency**

* `kms_ops_total{op,alg,result}` (Counter)
  `op ∈ {sign,verify,wrap,unwrap,rotate,create,attest,checkpoint}`
  `alg ∈ {ed25519,ml-dsa,x25519,ml-kem,aead}`
  `result ∈ {ok,err}`

* `kms_latency_seconds{op,alg}` (Histogram)
  Histograms with SLA buckets:

  * `ed25519`: `[{0.0005,0.001,0.002,0.005,0.01,0.02}]`
  * `ml-dsa`: `[{0.002,0.005,0.01,0.02,0.05,0.1}]`
  * others sized accordingly.

**Audit & Integrity**

* `kms_audit_appends_total{result}` (Counter)
* `kms_audit_checkpoint_total{result}` (Counter)
* `kms_audit_integrity_failed_total` (Counter) — **zero** in healthy state
* `kms_audit_queue_depth` (Gauge)
* `kms_audit_dropped_total{kind}` (Counter) — `kind=non_checkpoint|other`

**Attestation & Backends**

* `kms_attest_total{backend,result}` (Counter)
* `kms_backend_sessions{backend,state}` (Gauge) — `state=connected|connecting|error`
* `kms_backend_reconnects_total{backend}` (Counter)

**Hybrid PQ & Policy**

* `kms_pq_mode{mode}` (Gauge 0/1) — `mode=or|and|pq_only`
* `kms_pq_rollout_percent` (Gauge)
* `kms_reject_total{reason="pq_policy"}` (Counter) *(alias of `rejected_total` for convenience)*

**Verify-during-Outage**

* `kms_verify_cache_age_ms` (Gauge) — sampled
* `kms_verify_cache_hits_total{hit}` (Counter) — `hit=true|false`

**Fairness & Backpressure**

* `queue_depth{queue}` (Gauge) — `queue=sign|audit`
* `queue_dropped_total{queue}` (Counter)
* `busy_rejections_total{queue}` (Counter)

**Keys & Versions**

* `kms_keys_total{tenant,purpose,alg}` (Gauge)
* `kms_key_version_head{kid}` (Gauge) — useful for per-key canaries

### 1.3 Registration Discipline

* All metrics **registered once** in `metrics::new()`.
* Metric handles cloned where needed.
* CI check: grep for duplicate `register_int_counter/…` calls.
* **Naming rule:** prefix KMS-specific metrics with `kms_`; avoid cardinality explosions (no raw UUIDs as labels; use `kid_hash=blake3_8` when necessary).

---

## 2) Health & Readiness

### 2.1 Endpoints

* `/healthz` — **liveness**; returns `200 OK` if process is alive.
* `/readyz` — **readiness**; returns `200 OK` only when all readiness keys are satisfied. Payload includes structured reasons.

### 2.2 Readiness Keys (must all be true)

* `config_loaded` — config parsed & validated; backends bound to config snapshot
* `listener_bound` — HTTP listener bound and accepting
* `backend_ready` — active custody backend reachable (PKCS#11/TPM/file/cloud)
* `attestation_ready` — attestation evidence present **if required** by config/policy
* `audit_ready` — audit appender writable; last checkpoint verified
* `checkpoint_recent` — last checkpoint time/ops within threshold
* `rng_ok` — OS RNG usable; reseed cadence healthy
* `clock_ok` — monotonic OK and UTC skew within tolerance
* `verify_cache_ok` — cache present and not beyond soft TTL (warn threshold does not flip readiness)

### 2.3 Failure Semantics

* **Fail-closed writes:** sign/rotate/wrap return **503** with `Retry-After` when `backend_ready=false` or `attestation_ready=false` (if required).
* **Fail-open reads (bounded):** verify can succeed with cached public material; response includes `cache_age_ms`; readiness remains **true** unless `checkpoint_recent=false` or cache beyond **hard** TTL.
* `/readyz` payload example:

```json
{
  "ready": false,
  "missing": ["attestation_ready", "checkpoint_recent"],
  "degraded": ["backend_ready"],
  "retry_after": 5,
  "snapshot": {
    "backend": "pkcs11",
    "pq_mode": "and",
    "audit_last_checkpoint_age_s": 42
  }
}
```

---

## 3) Logs

### 3.1 Format (JSON lines)

* `content-type: application/jsonl`
* One event per line; no multi-line fields.

**Required fields (every log):**

* `ts` — ISO8601 UTC
* `level` — `INFO|WARN|ERROR|DEBUG|TRACE`
* `service` — `"ron-kms"`
* `event` — terse semantic name (see catalog below)
* `corr_id` — UUIDv7
* `request_id` — stable per-request ID (HTTP request span)
* `peer_addr` — remote socket (when available)
* `tenant` — short string or hash
* `route` — e.g., `/v1/kms/sign`

**Operation fields (where applicable):**

* `op_id` — client-supplied UUID for idempotent mutations
* `kid` — **never raw**; emit `kid_hash` = `BLAKE3(kid)[:8]`
* `kid_version` — integer `vN`
* `alg` — e.g., `ed25519|ml-dsa|x25519|ml-kem`
* `result` — `ok|err`
* `latency_ms`
* `attestation_class` — when present
* `cache_age_ms` — for verify under degraded mode
* `reason` — aligns with `rejected_total{reason=…}`

**Event catalog (non-exhaustive):**

* `sign_request`, `sign_result`
* `verify_request`, `verify_result`
* `rotate_request`, `rotate_result`
* `audit_append`, `audit_checkpoint_ok`, `audit_checkpoint_failed`
* `attestation_refresh_ok`, `attestation_refresh_failed`
* `backend_session_up`, `backend_session_down`
* `readyz_false`, `readyz_true` (include `missing` keys)
* `config_updated` (redacted diff)

### 3.2 Redaction & Secrets

* **Never log raw payloads, caps, or private keys.**
* Represent payloads solely as `payload_b3` (hex) and **lengths**.
* Redact `kid` to `kid_hash`.
* Redact PKCS#11 pins/env vars entirely.
* Config diffs are **redacted** for secrets and large arrays.

**Example log (sign):**

```json
{"ts":"2025-10-09T20:30:22Z","level":"INFO","service":"ron-kms","event":"sign_result","corr_id":"018f3a6e-...","request_id":"r-abc123","tenant":"core","route":"/v1/kms/sign","kid_hash":"b3afc1d2","kid_version":3,"alg":"ed25519","result":"ok","latency_ms":0.9}
```

---

## 4) Tracing & Correlation

* Use `tracing` + `tracing-subscriber` JSON formatter; optional OTEL export behind `feature="otel"`.

**Span naming:**

* `svc.kms.http` (per-request root)
* `svc.kms.sign`, `svc.kms.verify`, `svc.kms.wrap`, `svc.kms.unwrap`, `svc.kms.rotate`
* `svc.kms.audit.append`, `svc.kms.audit.checkpoint`
* `svc.kms.attest.refresh`
* `backend.pkcs11.sign`, `backend.tpm.sign`, `backend.file.sign`, `backend.cloud.wrap`

**Span attributes (key ones):**

* `tenant`, `route`, `kid_hash`, `kid_version`, `alg`, `pq_mode`, `attestation_required`, `backend`
* `cache_age_ms` (verify)
* `result`, `error.kind` (on failure)

**Correlation IDs:**

* Ingress: `X-Corr-ID` honored or generated (UUIDv7).
* Propagate `corr_id` and `request_id` into all child spans and logs.
* For downstream calls (policy/backends), inject `corr_id` header if HTTP/gRPC; otherwise carry in span attributes.

---

## 5) Alerts & SLOs

### 5.1 SLOs (per environment; tune in prod)

* **Latency (sign):**

  * `ed25519` p95 ≤ **1.0 ms**
  * `ml-dsa` p95 ≤ **10 ms**
* **Availability:**

  * `/readyz` false **0** minutes monthly (error budget excludes planned maintenance windows)
* **Error rates:**

  * 5xx < **0.1%**
  * 429/`backpressure` < **1%** sustained
* **Audit integrity:** `kms_audit_integrity_failed_total` = **0**
* **Attestation:** if required-by-config, `attestation_ready=true` **99.99%** of minutes
* **Verify cache:** `kms_verify_cache_age_ms` ≤ **TTL**; misses tolerated during outage, but alert if stale > **2× TTL**

### 5.2 PromQL Alerts (examples)

**A. Audit Integrity Failure (Critical)**

```promql
increase(kms_audit_integrity_failed_total[5m]) > 0
```

**B. Readiness Flapping (Critical if sustained)**

```promql
max_over_time(kube_probe_ready{service="ron-kms"}[5m]) == 0
```

**C. Sign Latency SLO Violation (Warning/Critical)**

```promql
histogram_quantile(0.95, sum by (le) (rate(kms_latency_seconds_bucket{op="sign",alg="ed25519"}[5m]))) > 0.001
```

(Use `> 0.010` for `alg="ml-dsa"`)

**D. Backpressure Spike (Warning)**

```promql
increase(busy_rejections_total{queue="sign"}[1m]) > 50
```

**E. PKCS#11 Session Flap (Warning)**

```promql
increase(kms_backend_reconnects_total{backend="pkcs11"}[10m]) > 3
```

**F. Attestation Missing when Required (Critical)**

```promql
max_over_time(kms_attest_total{result="missing"}[2m]) > 0 and on() kube_probe_ready{service="ron-kms"} == 1
```

**G. Verify Cache Stale (Warning)**

```promql
avg_over_time(kms_verify_cache_age_ms[5m]) > (2 * on() kms_config_verify_cache_ttl_ms)
```

*(If you export TTL as a gauge; else hardcode milliseconds.)*

### 5.3 Runbooks (link from alerts)

* `RUNBOOK_audit_integrity.md`: verify storage, recompute head, force checkpoint, restore from trusted snapshot if needed.
* `RUNBOOK_attestation.md`: rotate or re-issue attestation; validate device/slot; confirm `attest_require`.
* `RUNBOOK_backpressure.md`: check incoming RPS, tenant quotas, queue depths; scale pool; confirm policy gating.
* `RUNBOOK_latency_pq.md`: inspect alg mix (hybrid), pool CPU, HSM load, PQ rollout percent; consider reverting to `verify_mode=or` temporarily.

---

## 6) CI / Enforcement

* **HTTP surface check:** Ensure `/metrics`, `/healthz`, `/readyz`, `/version` exist and return sane payloads in integration tests.
* **Metric presence tests:** prove `kms_ops_total`, `kms_latency_seconds`, `kms_audit_integrity_failed_total`, `busy_rejections_total`, and `rejected_total{reason}` are **emitted at least once** under a smoke scenario.
* **No duplicate registration:** grep or unit test `metrics::new()` registers exactly once.
* **Log schema test:** golden sample for `sign_result` with required fields; ensure **no raw payload** fields appear.
* **Tracing test:** span tree contains `svc.kms.http` → `svc.kms.sign` → `backend.*` with `corr_id` propagated.
* **Readiness contract test:** flip `attestation_ready=false` in a harness → `/readyz=false`, **writes 503**, **verify returns with `cache_age_ms`** when cached.
* **Redaction test:** config diff logs redact secrets (env/pins), and `kid` never appears raw (only `kid_hash`).

---

## 7) Dashboards (suggested panels)

1. **Service Overview**

   * Requests by route/status; p50/p95 latency; inflight
   * Rejections by reason; backpressure trend
2. **KMS Core**

   * `kms_ops_total` split by `op,alg,result`
   * `kms_latency_seconds` (overlay ed25519 vs ml-dsa)
   * Queue depths (`sign`, `audit`) + drops
3. **Audit/Integrity**

   * Appends/checkpoints; time since last checkpoint
   * Integrity failures (single-stat, red)
4. **Attestation & Backend**

   * Attest result rate
   * Backend sessions & reconnects (pkcs11/tpm/cloud)
5. **Degraded Verify**

   * Cache age distribution; cache hit ratio
   * `/readyz` timeline + degraded flags

---

## 8) OTEL Mapping (optional feature)

* Spans exported via OTLP/HTTP when `--features otel` enabled.
* Resource attributes: `service.name=ron-kms`, `service.version`, `deployment.environment`.
* Span attributes mirror log fields (`kid_hash`, `alg`, `pq_mode`, etc.); redact as in logs.
* Exemplars: attach span links to `kms_latency_seconds` histograms for p95 debugging.

---

## 9) Field Reference (cheat sheet)

| Name           | Type   | Example        | Notes                       |     |          |
| -------------- | ------ | -------------- | --------------------------- | --- | -------- |
| `corr_id`      | UUIDv7 | `018f3a6e-…`   | Ingress header or generated |     |          |
| `request_id`   | string | `r-abc123`     | Stable per request          |     |          |
| `op_id`        | UUID   | `f1d2-…`       | Idempotent mutations        |     |          |
| `kid_hash`     | hex8   | `b3afc1d2`     | `BLAKE3(kid)[:8]`           |     |          |
| `alg`          | enum   | `ed25519`      | Allow-list only             |     |          |
| `pq_mode`      | enum   | `and`          | `or                         | and | pq_only` |
| `cache_age_ms` | int    | `742`          | Verify degraded             |     |          |
| `reason`       | enum   | `backpressure` | Must match metrics label    |     |          |

---

## 10) Example `/metrics` excerpt (sanitized)

```
kms_ops_total{op="sign",alg="ed25519",result="ok"} 132942
kms_latency_seconds_sum{op="sign",alg="ed25519"} 92.1
kms_latency_seconds_count{op="sign",alg="ed25519"} 130000
kms_audit_integrity_failed_total 0
busy_rejections_total{queue="sign"} 4
kms_verify_cache_age_ms 238
rejected_total{reason="attest_missing"} 2
```

---

## 11) Governance

* This document is **reviewed every 90 days** or whenever telemetry names change.
* Any new metric or log field must follow **naming guidelines** and be accompanied by a dashboard/alert addition or an explicit rationale for omission.

---

✅ With this, `ron-kms` emits **actionable, leak-free** telemetry that proves custody safety (audit/attestation), preserves latency SLOs under PQ, and makes degraded states **explicit** for operators and auditors.
