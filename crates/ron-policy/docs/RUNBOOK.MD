

---

title: RUNBOOK — ron-policy
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-09-19
audience: operators, SRE, auditors

---

# 🛠️ RUNBOOK — ron-policy

## 0) Purpose

Operational manual for `ron-policy` as **embedded logic** inside services (gateway, index, storage, overlay). Covers: how to validate/roll out policy bundles, health and diagnostics surfaced by consumers, common failure modes, recovery/rollback, and scaling guidance.
Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `ron-policy`
* **Role:** Deterministic, side-effect-free library for **validating** and **evaluating** declarative policy bundles (capability checks, route classes, obligations).
* **Criticality Tier:** **2 (supporting)** library; its misuse or regressions **impact Tier-1 services** on the request path.
* **Direct Dependencies:** None at runtime (pure lib). Validation uses serde/DTOs in-process.
* **Consumers (examples):** `svc-gateway`, `svc-index`, `svc-storage`, `svc-overlay`.
* **Ports Exposed:** **N/A** (no process). Metrics and health are **surfaced by consumers**.
* **Data Flows:**

  * In: versioned **policy bundle** (JSON/CBOR) loaded by the consumer service.
  * In-process call: `evaluate(bundle, ctx, clock) -> Decision`.
  * Out: Prometheus metrics/logs from the **consumer** (latency histogram, decision counters).
* **Version Constraints:** Keep **DTO version** in consumer services aligned with `ron-policy` minor; roll via **canary bundle** first (see §6.3).

> **Anti-Scope (guardrails)**
> `ron-policy` MUST NOT perform I/O; MUST be deterministic; MUST NOT depend on global state/timezones; MUST fail-closed on invalid bundles (validation stage).

---

## 2) Startup / Shutdown

**Library — no standalone startup.** Operators interact through consumers.

### 2.1 Consumer Startup Checklist (embedding `ron-policy`)

* Ensure the consumer has a **policy bundle path/URL** configured and a **last-good snapshot** available.
* Start consumer (examples):

  * systemd: `systemctl start svc-gateway`
  * dev: `cargo run -p svc-gateway -- --config ./configs/gateway.toml`
* **Verification (consumer):**

  * `curl -s http://127.0.0.1:9909/readyz` → `200 OK`
  * Logs contain: `policy_snapshot_loaded=true version=<N>`
  * Metrics include buckets for `policy_eval_latency_seconds`.

### 2.2 Shutdown (consumer)

* `systemctl stop svc-gateway`
* Expect a clean **Shutdown** bus event (if bus is enabled). Library has no shutdown hooks.

---

## 3) Health & Readiness (observed via consumers)

* **/healthz** (consumer): process liveness.
* **/readyz** (consumer): ready only if **policy snapshot is valid and loaded**.
* **Expected time to ready:** 2–5s on warm disk; add validation time for large bundles.
* **If not ready after 10s:**

  * Check logs for `policy_validation_error` or `policy_snapshot_load_failed`.
  * Inspect metrics: spikes in `policy_eval_latency_seconds` (mis-sized candidate set) or zero samples (snapshot missing).

---

## 4) Common Failure Modes

| Symptom                            | Likely Cause                                    | What to Check (Logs/Metrics)                    | Resolution                                                                               | Alert |
| ---------------------------------- | ----------------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------- | ----- |
| Sudden spike in 403/deny           | Policy change too broad; wrong route class      | `policy_decisions_total{decision="deny"}` surge | **Rollback to last-good bundle** (see §6.1). Triage rule diffs; narrow class; re-canary. | yes   |
| Consumer `/readyz` = 503 on deploy | Bundle failed validation / schema mismatch      | `policy_validation_error` with field path       | Fix bundle, re-validate offline (see §5.3), redeploy.                                    | yes   |
| p95 request latency ↑              | Candidate set explosion (over-broad rules)      | `policy_eval_latency_seconds` p95 > 1ms         | Precompute indices enabled? Lint/narrow route classes; ship trimmed bundle.              | yes   |
| Frequent snapshot reloads (churn)  | Misconfigured watcher or noisy config pipeline  | `policy_snapshot_version` flapping              | Debounce watcher; pin update cadence; verify only intended source mutates the bundle.    | warn  |
| Deny reasons missing/garbled       | DTO/version skew between consumer & bundle      | `policy_reason_parse_error`                     | Align consumer to `ron-policy` minor; rebuild; re-emit canonical bundle.                 | yes   |
| Panic in eval (should not happen)  | Logic bug or malformed data bypassed validation | crash log + backtrace                           | Immediate rollback; capture inputs; open sev-A; run fuzz corpus; patch & hotfix.         | page  |

---

## 5) Diagnostics

### 5.1 Quick Status (consumer)

* **Ready:**
  `curl -s http://127.0.0.1:9909/readyz`
* **Metrics snapshot:**
  `curl -s http://127.0.0.1:9909/metrics | grep -E 'policy_eval_latency_seconds|policy_decisions_total'`
* **Tail logs:**
  `journalctl -u svc-gateway -f | grep policy`

### 5.2 PromQL (consumer dashboards)

* p95 evaluation latency:
  `histogram_quantile(0.95, sum(rate(policy_eval_latency_seconds_bucket[5m])) by (le))`
* Deny rate:
  `sum(rate(policy_decisions_total{decision="deny"}[5m]))`

### 5.3 Offline Validation (pre-deployment)

Use your CI/job to validate proposed bundles before shipping:

* Structural check (serde/DTO): `cargo test -p ron-policy --test bundle_validations -- --nocapture`
* Golden test replay (recommended): run a small bench/sim that loads the bundle and replays known contexts to diff `Decision` vs. last-good.

---

## 6) Recovery Procedures

### 6.1 **Instant Rollback (bundle)**

1. Point consumer to **last-good bundle** (version `N-1`) or flip a feature flag/env to fallback path.
2. Reload policy (hot): send SIGHUP or hit the consumer’s `/admin/reload` if available.
3. Verify:

   * `/readyz` returns `200`
   * `policy_snapshot_version` decreased to `N-1`
   * Deny rate returns to baseline.

### 6.2 **Fix & Redeploy**

1. Reproduce failure with **offline validation** (5.3).
2. Narrow/patch rules; re-lint route classes (avoid over-broad prefixes/regex).
3. Canary (6.3); then roll to 100%.

### 6.3 **Canary Rollout Playbook**

1. Select **1–5% traffic shard** (one gateway replica or label-based routing).
2. Load canary bundle version `N`.
3. Watch 10–30 min:

   * `policy_eval_latency_seconds` (should remain sub-ms p95)
   * `policy_decisions_total{decision="deny"}`
   * Top deny reasons by label (cardinality bounded).
4. If stable, proceed 25% → 50% → 100%. If not, rollback (6.1).

### 6.4 **Consumer Not Ready After Reload**

* Check logs for validation error path; if schema mismatch, align consumer crate versions.
* If the watcher is flapping, disable auto-reload, pin last-good; file an issue.

---

## 7) Backup / Restore

* `ron-policy` holds **no state**.
* Treat **bundles** as **versioned artifacts** in Git/registry:

  * Keep **last-good** and **current** pinned.
  * Back up the config store that distributes bundles (if used).
* Restore = point consumer at preserved artifact; hot reload.

---

## 8) Upgrades

### 8.1 Library Upgrade (semver)

* Respect semver: minor may add DTO fields (with `deny_unknown_fields` on the consumer side).
* **Order:** upgrade consumers to the new `ron-policy` version **before** shipping bundles that rely on new fields.

### 8.2 Operational Steps

1. Build new consumer with updated `ron-policy`.
2. Deploy one canary replica.
3. Verify `/readyz` + metrics for 10–30 min.
4. Roll fleet.
5. Optionally enable new bundle features; re-canary the bundle.

---

## 9) Chaos & Soak

* Quarterly: inject **latency** and **reload churn** in one consumer to ensure:

  * Ready gate remains stable.
  * Deny rate does not spike unexpectedly.
  * Canary/rollback paths function.
* 24h soak on stable bundle: confirm no memory growth attributable to policy evaluation (should be flat).

---

## 10) Scaling Notes

* `ron-policy` is CPU-bound and **embarrassingly parallel** per request.
* Scale via **consumer replicas**; keep per-replica **concurrency caps** sane (rule of thumb: `min(2 * cores, 64)` evals in flight).
* Maintain **precomputed indices** at bundle load; avoid re-allocations in hot path.
* Expect **p95 eval < 1ms** for ≤1k rules in a route class. Investigate if exceeded (see §5.2).

---

## 11) Security Ops

* Bundles are **configuration artifacts**—treat as code: code review, signatures if available, CI validation.
* No secrets should appear in bundles or policy logs.
* Deny/allow reasons are **user-visible** in some contexts; keep strings concise and non-sensitive.
* PQ posture: library is crypto-agnostic; if consumers add hybrid/PQ checks **before** policy, ensure policy eval remains sub-ms.

---

## 12) References

* `CONFIG.md` — bundle path/envs and reload semantics
* `SECURITY.md` — validation guarantees, fail-closed behavior
* `OBSERVABILITY.md` — metrics schema (`policy_eval_latency_seconds`, `policy_decisions_total`)
* `CONCURRENCY.md` — no locks across `.await` in consumer; atomic snapshot swap
* `PERFORMANCE.md` — SLOs, regression gates, PromQL
* Blueprints: **Hardening**, **Concurrency & Aliasing**, **Scaling**, **Omnigate**

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A**: Metrics green (p95 eval < 1ms; deny rate baseline)
* [ ] **Gate F**: Perf regression guardrails enforced in CI (consumers)
* [ ] **Gate J**: Chaos drill (latency + reload churn) passed this quarter
* [ ] **Gate K**: Continuous vigilance—alerts on deny spikes, readiness flaps
* [ ] **Gate L**: Black swan—rollback rehearsed; last-good artifact pinned
* [ ] **Gate N**: ARM/edge profile captured; no platform-specific cliffs

---

### One-Page Operator Quick-Refs

**Rollback now (bundle):**

1. Point consumer to last-good bundle (`N-1`)
2. Hot reload (SIGHUP or admin endpoint)
3. Verify `/readyz` + deny rate normalized

**Canary a new bundle:**

1. 1–5% shard → load bundle `N`
2. Watch p95 eval latency + deny reasons (10–30 min)
3. Roll forward or rollback

**Investigate perf spike:**

1. Check `policy_eval_latency_seconds` p95
2. Inspect candidate set size / over-broad route classes
3. Revert to last-good; trim rules; re-canary
