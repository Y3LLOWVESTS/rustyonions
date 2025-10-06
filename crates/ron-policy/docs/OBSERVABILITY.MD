# 📈 OBSERVABILITY — ron-policy

*Audience: developers, operators, auditors*
*msrv: 1.80.0 (Tokio/loom compatible; async only in consumer crates)*

---

## 0) Purpose

`ron-policy` is a **pure library** that validates and evaluates **policy bundles** and returns **decisions + obligations**. It does **not** open sockets, export `/metrics`, or log by itself. This doc defines:

* The **stable metric names** and low-cardinality label sets that consumer services MUST/SHOULD emit when invoking `ron-policy`.
* **Health/readiness semantics** for consumers that depend on a policy snapshot.
* **Log schema** and **tracing spans** for validation/evaluation paths.
* **Alert rules & SLOs** for policy health and performance.

> TL;DR: This is the **contract** for how services make `ron-policy` observable.

---

## 1) Metrics (Prometheus-style)

### 1.0 Emission model

* Library supplies **names + labels** (this doc) and optional helper shims (`feature = "metrics"`).
* **Consumers** (gateway, storage, index, …) actually **record** metrics around:

  1. **Validation** (when bundles change) and
  2. **Evaluation** (on request hot paths).

> Keep label **cardinality low**. Use **route patterns** like `"GET /o/*"`, not raw paths.

### 1.1 Golden Metrics (policy contract)

| Metric                              | Type      | Labels                          | Notes / Cardinality Rules                       |                                                  |                                                              |        |           |                                            |
| ----------------------------------- | --------- | ------------------------------- | ----------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------ | ------ | --------- | ------------------------------------------ |
| `policy_eval_latency_seconds`       | Histogram | `route_class`, `has_cap=("true" | "false")`, `decision=("allow"                   | "deny")`                                         | p50/p95/p99 for hot paths. `reason` is **not** a label here. |        |           |                                            |
| `policy_decisions_total`            | Counter   | `decision=("allow"              | "deny")`, `reason`                              | `reason` is from the **small, fixed set** below. |                                                              |        |           |                                            |
| `policy_changes_total`              | Counter   | `result=("applied"              | "rejected")`, `cause=("tighten_only"            | "churn"                                          | "schema"                                                     | "perf" | "other")` | Increment on every attempted bundle apply. |
| `policy_snapshot_version`           | Gauge     | —                               | Last applied `version` as a numeric gauge.      |                                                  |                                                              |        |           |                                            |
| `policy_snapshot_age_seconds`       | Gauge     | —                               | `now - issued_at_epoch_ms/1000`.                |                                                  |                                                              |        |           |                                            |
| `policy_rules_total`                | Gauge     | `kind=("route"                  | "quota"                                         | "residency")`                                    | Count from the current bundle.                               |        |           |                                            |
| `policy_bundle_size_bytes`          | Gauge     | —                               | Serialized size of canonical JSON.              |                                                  |                                                              |        |           |                                            |
| `policy_perf_budget_breaches_total` | Counter   | —                               | Times eval exceeded p95 budget for the release. |                                                  |                                                              |        |           |                                            |
| `queue_dropped_total`               | Counter   | `queue=("policy_loader"         | "audit")`                                       | For consumer-side queues only.                   |                                                              |        |           |                                            |
| `bus_lagged_total`                  | Counter   | —                               | When broadcast lag forces drop (consumer bus).  |                                                  |                                                              |        |           |                                            |

**Reason codes (stable, low cardinality) used in `policy_decisions_total{reason=…}` and `policy_rejects_total{reason=…}`:**

```
ok
region.denied
region.not_allowed
cap.required
body.too_large
decompress.guard
tighten_only
churn
schema
perf
other
```

> **Rule:** Only **deny** outcomes should carry a specific root cause (e.g., `region.denied`). `allow` SHOULD use `reason="ok"`.

### 1.2 Service-specific joins (examples)

* **Gateway:** add `endpoint_class` and reuse `policy_*` metrics to correlate with `http_requests_total`.
* **Storage:** include `content_class=("small"|"large")` when measuring eval latency effects on GET/PUT decision paths.

### 1.3 Registration Discipline

* Register metrics **once** at service start (e.g., `Metrics::new()`), then pass handles.
* CI grep in services MUST ensure **no duplicate registration** and **no free-form `reason` values** (enforced via enum mapping).

---

## 2) Health & Readiness (for consumers)

### 2.1 Endpoints

* `/healthz` — liveness (process alive).
* `/readyz` — readiness: **OK (200)** only when:

  * A **valid policy snapshot** is loaded (`policy_snapshot_version > 0`);
  * Last **validation** was **applied** (or a previous good snapshot exists);
  * **Perf headroom** is healthy (recent p95 ≤ budget).

### 2.2 Readiness Keys (suggested JSON)

```json
{
  "policy_snapshot_loaded": true,
  "policy_snapshot_version": 43,
  "policy_snapshot_age_seconds": 128,
  "last_validation_result": "applied",
  "perf_headroom_ok": true
}
```

### 2.3 Failure Semantics

* If snapshot missing or last good rejected with no fallback → `/readyz` = **503** with body:

  ```json
  { "degraded": true, "missing": ["policy_snapshot"], "retry_after": 10 }
  ```
* Writes SHOULD fail-closed; reads MAY degrade per obligations (see IDB).

---

## 3) Logs

### 3.1 Format (JSON lines)

**Required fields (service logs around policy):**

* `ts` (ISO8601), `level`, `service`, `event`
* `corr_id` (inherited), `peer_addr` (if applicable)
* `policy_version`, `policy_id_b3` (for validation/reload)
* `route_class`, `has_cap`, `decision` (`allow|deny`), `reason` (deny only)
* `latency_ms` (evaluation duration)

### 3.2 Event taxonomy

* `policy.validate.ok` — bundle accepted; include counts: `{routes, quotas, residency}`.
* `policy.validate.reject` — include `cause`.
* `policy.reload.apply` — include `from_version`, `to_version`.
* `policy.eval.decision` — **TRACE** (on hot paths) or **INFO** on deny only.

### 3.3 Redaction & Secrets

* Never log tokens, caps, or full bundles in prod.
* Redact long strings; log only **counts** and **ids** (`policy_id_b3`).
* If logging diffs, prefer **keys changed** and **cardinality deltas**, not raw bodies.

**Example (deny):**

```json
{"ts":"2025-10-06T14:32:21Z","level":"INFO","service":"svc-gateway",
 "event":"policy.eval.decision","corr_id":"01J...",
 "route_class":"GET /o/*","has_cap":false,"decision":"deny","reason":"region.denied",
 "latency_ms":0.42,"policy_version":43,"policy_id_b3":"b3:89ab...cdef"}
```

---

## 4) Tracing & Correlation

* Use `tracing` JSON formatter with spans:

  * **`lib.ron_policy.validate`** — fields: `new_version`, `id_b3`, `result`, `cause?`.
  * **`lib.ron_policy.evaluate`** — fields: `route_class`, `has_cap`, `decision`, `reason?`.

* Correlation ID:

  * Inject on ingress (e.g., HTTP `X-Corr-ID`) and **propagate** into both spans.

* OTEL (optional): map spans to OTEL with `tracing-opentelemetry`.

  * Attributes mirror the fields above; NEVER attach payloads or tokens.

---

## 5) Alerts & SLOs

### 5.1 SLOs (policy path)

* **Eval p95** `< 1 ms` for hot paths (≤1k rules).
* **Rejected changes** `< 1%` of attempts over 1h (sustained).
* **Snapshot age** `< 10 min` after an approved change is published (staleness guard).

### 5.2 PromQL alert examples

* **Rejected burst (warning/critical):**

  ```promql
  increase(policy_changes_total{result="rejected"}[5m]) > 0
  ```
* **Policy stale (critical):**

  ```promql
  policy_snapshot_age_seconds > 600
  ```
* **Eval p95 budget breach (warning):**

  ```promql
  histogram_quantile(0.95, sum(rate(policy_eval_latency_seconds_bucket[5m])) by (le)) > 0.001
  ```
* **Deny spike by root cause (investigate):**

  ```promql
  increase(policy_decisions_total{decision="deny"}[5m]) > 100
  ```

### 5.3 Runbooks (link from alerts)

* **Rejected change:** check CI gates (`tighten_only`, `churn`, `schema`), fix bundle, republish.
* **Stale snapshot:** inspect bus health, loader logs; retry fetch; fail `/readyz` if no last good exists.
* **p95 breach:** inspect rule count growth; shrink route scope; precompute indices.

---

## 6) CI / Enforcement

* **Library tests** assert:

  * Decision reasons are from the **allowed set** above.
  * Determinism and performance budgets (criterion) meet SLO.

* **Consumer CI** (recommendations):

  * Grep for **metric names** listed in §1 to ensure consistent emission.
  * `rejected_total{reason}` or `policy_decisions_total{decision="deny"}` must appear on deny paths.
  * `/healthz`, `/readyz`, `/metrics` endpoints present (services only).
  * Lints for `await_holding_lock` (no guards across `.await` around policy spans).
  * Dashboards and alerts are versioned and linked in `RUNBOOK.md`.

---

### Appendix A — Metric Name Guide (copy-paste)

```rust
// Low-cardinality labels (recommended)
const L_ROUTE: &str = "route_class";    // e.g., "GET /o/*"
const L_HAS_CAP: &str = "has_cap";      // "true" | "false"
const L_DECISION: &str = "decision";    // "allow" | "deny"
const L_REASON: &str = "reason";        // see fixed set
const L_RESULT: &str = "result";        // "applied" | "rejected"
const L_CAUSE: &str  = "cause";         // "tighten_only" | "churn" | "schema" | "perf" | "other"
```

**Allowed reasons (enum in code):**

```rust
// Deny reasons and validation causes (stable, auditable)
pub const REASON_OK: &str = "ok";
pub const REASON_REGION_DENIED: &str = "region.denied";
pub const REASON_REGION_NOT_ALLOWED: &str = "region.not_allowed";
pub const REASON_CAP_REQUIRED: &str = "cap.required";
pub const REASON_BODY_TOO_LARGE: &str = "body.too_large";
pub const REASON_DECOMPRESS_GUARD: &str = "decompress.guard";
// validation-only causes:
pub const REASON_TIGHTEN_ONLY: &str = "tighten_only";
pub const REASON_CHURN: &str = "churn";
pub const REASON_SCHEMA: &str = "schema";
pub const REASON_PERF: &str = "perf";
pub const REASON_OTHER: &str = "other";
```

**Recording pattern (consumer side):**

```rust
use metrics::{counter, histogram};

fn observe_eval(route: &str, has_cap: bool, decision: &str, nanos: u128) {
    histogram!("policy_eval_latency_seconds",
        (nanos as f64) / 1_000_000_000.0,
        "route_class" => route,
        "has_cap" => if has_cap { "true" } else { "false" },
        "decision" => decision,
    );
    counter!("policy_decisions_total", 1,
        "decision" => decision,
        // add reason only for deny; use "ok" for allow if you must record it
    );
}
```

---

### Appendix B — Trace Fields (reference)

* Validate span: `new_version`, `id_b3`, `result`, `cause?`, `bundle_bytes`, `route_rules`, `quota_rules`.
* Evaluate span: `route_class`, `has_cap`, `decision`, `reason?`, `policy_version`, `id_b3`.

---

With this contract in place, every consumer can emit **uniform, low-cardinality telemetry** for `ron-policy`, making dashboards portable and incidents fast to triage.
