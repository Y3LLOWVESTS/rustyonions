

````markdown
# 📈 OBSERVABILITY.md — ron-audit

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define **what is observable**, **how it’s exposed (via hosts)**, and **how it’s used** for:

- Metrics (Prometheus/OTEL)
- Health & readiness semantics (host endpoints)
- Logs (JSON schema/fields)
- Tracing (spans, correlation IDs)
- Alerts & SLOs (for the audit pipeline)

> `ron-audit` itself **does not** bind HTTP; hosts must surface the metrics/health endpoints and wire the counters/gauges described here.

---

## 1) Metrics (Prometheus-style)

### 1.1 Golden (host-level, required when using ron-audit)

- `audit_appended_total{service,kind,stream}` (Counter)  
  Records successfully appended (post-canonicalization & chain update).

- `audit_drop_total{service,reason}` (Counter)  
  Best-effort sheds: `audit_backpressure`, `io_backpressure`, `bounds_exceeded`, `shutdown`, `closed`.

- `audit_queue_depth{service,stream}` (Gauge)  
  Current `mpsc` depth for the audit work queue.

- `audit_bytes_total{service}` (Counter)  
  Serialized bytes appended (canonical excluding `self_hash`).

- `audit_export_checkpoints_total{service}` (Counter)  
  Number of Merkle checkpoints produced (Macronode).

- `io_timeouts_total{service,op="wal|export"}` (Counter)  
  WAL/Export operations that exceeded time budgets.

- `audit_verify_fail_total{service,reason}` (Counter)  
  Verification-on-read failures: `hash_mismatch`, `prev_mismatch`, `manifest_gap`.

- `tasks_{spawned,aborted}_total{service,kind="audit_worker"}` (Counters)  
  Concurrency health for sink workers (host).

- `busy_rejections_total{service,endpoint="emit"}` (Counter)  
  Caller-facing rejections due to full queue or enqueue deadline.

### 1.2 Optional/diagnostic

- `audit_dedupe_hits_total{service}` (Counter) — idempotent replays accepted.  
- `audit_pii_{redacted,blocked}_total{service}` (Counters) — privacy instrumentation.  
- `wal_fsync_seconds` (Histogram) — latency of fsync batches (Macronode).  
- `export_batch_seconds` (Histogram) — Merkle + manifest latency.

### 1.3 Registration Discipline

- Hosts **register once** in a `Metrics::new()` and pass handles down.  
- This crate offers **label keys** and helper macros (optional) but **does not** register a global singleton.

---

## 2) Health & Readiness (host endpoints)

### 2.1 Endpoints (host must expose)

- `/healthz` — liveness (process up).
- `/readyz` — **audit pipeline readiness**, must return `200` only if:

### 2.2 Readiness Keys (audit-specific)

- `audit.config_loaded` — config parsed & validated (amnesia/WAL compatibility).
- `audit.queue_bound_ok` — queue constructed with configured bounds.
- `audit.wal_ready` — WAL dir exists, writable (Macronode only).
- `audit.export_ready` — export dir exists, writable (Macronode only).
- `audit.checkpoint_timer_active` — checkpoint scheduler running (if enabled).
- `audit.verify_on_read` — verification hook enabled before serving to auditors.

### 2.3 Failure Semantics (host)

- **Fail-closed** for write paths: if WAL/export broken or queue saturated persistently, expose `503` from write/emit edges and set `/readyz` to `503` with payload:
  ```json
  { "degraded": true, "missing": ["audit.wal_ready"], "retry_after": 5 }
````

* **Fail-open** only for **read-only** diagnostic surfaces *not* used for compliance.

---

## 3) Logs

### 3.1 Format & Required Fields

* JSON Lines; one event per line; UTC timestamps.
* Required keys (merge with host logging schema):

  * `ts` (RFC3339), `level`, `service`, `event`
  * `corr_id` (UUID/ULID), `span_id`, `trace_id` (if OTEL)
  * **Audit-specific:** `stream`, `writer_id`, `seq`, `prev`, `self_hash`, `reason`
  * Optional: `content_id`, `ledger_txid`, `policy_version`

### 3.2 Canonical Events

* `audit.append.ok` — appended; include `stream`, `seq`, `self_hash`.
* `audit.append.reject` — include `reason` ∈ {`bounds_exceeded`,`tamper`,`busy`,`schema`}.
* `audit.verify.fail` — include `reason` ∈ {`hash_mismatch`,`prev_mismatch`}.
* `audit.checkpoint.created` — include `range`, `root`, `signer_key_id`.
* `audit.export.error` — include error and retry posture.

### 3.3 Redaction & Secrets

* Never log secrets, tokens, raw IPs, or WAL contents.
* If `privacy.ip_mode=hash`, do **not** log the salt or raw IPs; at most log prefix.

---

## 4) Tracing & Correlation

* Use `tracing` with JSON formatter; optional OTEL exporter behind a host feature.
* Span naming: `svc.<host>.<plane>.audit.<op>`

  * Examples: `svc.gateway.ingress.audit.emit`, `svc.storage.audit.wal_append`, `svc.storage.audit.checkpoint`.
* Attach fields:

  * `stream`, `writer_id`, `seq`, `prev`, `self_hash`, `reason`, `queue_depth`.
* Correlation:

  * `corr_id` injected by host at ingress; propagated on all audit spans/logs.

---

## 5) Alerts & SLOs (Audit Pipeline)

### 5.1 SLOs (recommended)

* **Emit p95 latency**: `< 25 ms` intra-region (enqueue path).
* **Backpressure rate**: `audit_drop_total{reason="audit_backpressure"} / audit_appended_total < 0.5%` (5m window).
* **Verification failures**: `audit_verify_fail_total == 0` (hard SLO).
* **Checkpoint cadence**: mean interval within `±20%` of configured target.

### 5.2 Alerts (Prometheus-style examples)

* **Queue Saturation (warning)**

  ```
  avg_over_time(audit_queue_depth{service="$svc"}[5m]) 
    > 0.8 * on(configured_depth) configured_depth
  ```
* **Shed Spike (critical)**

  ```
  increase(audit_drop_total{reason="audit_backpressure",service="$svc"}[5m]) > 100
  ```
* **Verification Failure (critical)**

  ```
  increase(audit_verify_fail_total{service="$svc"}[10m]) > 0
  ```
* **Checkpoint Stale (warning/critical)**

  ```
  time() - max_over_time(audit_export_checkpoints_total{service="$svc"}[30m]) > 1800
  ```

### 5.3 Runbooks (link from alerts)

* `RUNBOOK_audit_queue_saturation.md` — tune `queue_depth`, check slow WAL/FS.
* `RUNBOOK_audit_verify_fail.md` — isolate segment range; re-derive Merkle roots; audit for tamper.
* `RUNBOOK_audit_checkpoint_stale.md` — inspect scheduler, disk space, signer availability.

---

## 6) CI / Enforcement

* **Compile-time:** enable `-W clippy::await_holding_lock`.
* **Unit/property:** prove chain continuity, dedupe idempotency, bounds enforcement (no panics).
* **Golden metrics check:** a CI grep ensures the host exposes all **required** audit metrics.
* **Evil vectors:** CI test vectors validate canonicalization stability (NFC, key order, integer forms).
* **cargo-deny:** advisories/licenses pinned; SBOM generated at release.

---

## 7) Example (Host Wiring)

### 7.1 Metrics Registration (host)

```rust
#[derive(Clone)]
pub struct AuditMetrics {
  pub appended: prometheus::IntCounterVec,
  pub drops: prometheus::IntCounterVec,
  pub q_depth: prometheus::IntGaugeVec,
  pub bytes: prometheus::IntCounter,
  pub verify_fail: prometheus::IntCounterVec,
  pub checkpoints: prometheus::IntCounter,
  pub io_timeouts: prometheus::IntCounterVec,
  pub busy_rej: prometheus::IntCounterVec,
}

impl AuditMetrics {
  pub fn new(reg: &prometheus::Registry) -> Self { /* register vectors once */ /* ... */ Self { /* ... */ } }
}
```

### 7.2 Emit Path (host)

```rust
fn try_enqueue(tx: &mpsc::Sender<Job>, job: Job, m: &AuditMetrics) -> Result<(), Error> {
    match tx.try_send(job) {
        Ok(_) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => {
            m.drops.with_label_values(&["audit_backpressure", SERVICE]).inc();
            m.busy_rej.with_label_values(&["emit", SERVICE]).inc();
            Err(Error::Busy)
        }
        Err(mpsc::error::TrySendError::Closed(_)) => Err(Error::Closed),
    }
}
```

### 7.3 Append Success (host worker)

```rust
fn on_append_ok(m: &AuditMetrics, stream: &str, kind: &str, bytes: usize) {
    m.appended.with_label_values(&[SERVICE, kind, stream]).inc();
    m.bytes.inc_by(bytes as u64);
}
```

---

## 8) Dashboards (suggested panels)

* **Audit Throughput:** `sum by (service,kind)(rate(audit_appended_total[5m]))`
* **Queue Health:** `audit_queue_depth` with threshold line; overlay `busy_rejections_total` rate.
* **Shedding:** stacked `rate(audit_drop_total[5m]) by (reason)`
* **Verification:** `rate(audit_verify_fail_total[5m])` (should be zero)
* **Durability:** `rate(audit_export_checkpoints_total[15m])`, `wal_fsync_seconds` p95

---

## 9) Field Guide for Auditors

* **Proving a chain segment:** export records `n..m`, recompute `self_hash` and `(prev→self)` continuity; compare with manifest ranges and checkpoint Merkle root.
* **Detecting tamper:** any recomputed root mismatch or `prev` discontinuity is a red flag; cross-check `audit_verify_fail_total` and error logs around the same time window.

---

## 10) FAQ

* **Why shed instead of block?** Protects host SLOs; audit is best-effort evidence with strong tamper-evidence when present. Drops are **measured** and alarmed.
* **Why verification on read?** Defense-in-depth vs. at-rest tamper or bitrot; zero trust in storage media.
* **Where are the endpoints?** In the **host** (service) that embeds `ron-audit`. This crate defines the **contract**.

---

```

