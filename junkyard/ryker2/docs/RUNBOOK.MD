
````markdown
---
title: RUNBOOK — ryker
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-09-28
audience: operators, SRE, auditors
concerns: [GOV, RES, PERF, SEC, OPS]
---

# 🛠️ RUNBOOK — ryker

## 0) Purpose
Operational manual for `ryker`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan / Resilience Readiness).

> Note: `ryker` is a **library crate**, not a standalone service. Operators interact with ryker through **host services** (e.g., gateway, overlay) that embed it. This runbook documents **library-specific observability/failure patterns** and how to manage them via hosts.

---

## 1) Overview
- **Name:** `ryker`
- **Role:** Actor/mailbox runtime library
- **Criticality Tier:** Tier 0 (foundation library; embedded in higher services)
- **Dependencies:** None directly; relies on host runtime (Tokio), metrics backend, and host supervision/bus
- **Ports Exposed:** N/A (library only)
- **Data Flows:** In-process message enqueue/dequeue with bounded mailboxes; host may emit bus events informed by ryker state
- **Version Constraints:** SemVer with API freeze discipline (see `docs/API.md`)
- **SLO Pointers:** See `docs/OBSERVABILITY.md §5` (e.g., p95 handler latency < configured deadline; Busy/429 < 1%; config reload errors = 0)

---

## 2) Startup / Shutdown
### Startup (via host)
```rust
let cfg = ryker::config::RykerConfig::from_env_validated()?;
let rt  = ryker::Runtime::new(cfg);
````

**Verification (via host):**

* Logs: `crate="ryker" event="config_applied"` (diff redacted)
* Metrics: `ryker_config_reload_total` increments; `ryker_config_reload_errors_total == 0`
* (Optional bus) `ryker.config.applied` published by host

### Shutdown (via host)

* Bus: `kernel.shutdown { deadline_ms }` OR host calls `Runtime` drain method with deadline
* Expected:

  * Intake stops immediately
  * In-flight drained within deadline
  * Drops counted under `ryker_mailbox_dropped_total{reason="shutdown"}`

> Amnesia: if `amnesia.enabled=true`, transient buffers are zeroized on shutdown; no disk sinks.

---

## 3) Health & Readiness

Ryker has **no** `/healthz` or `/readyz`; hosts expose them.

**Healthy when (host signals):**

* `ryker_config_reload_errors_total == 0` over the last 10m
* No recent `ServiceCrashed` events for ryker-owned actors

**Degraded when (suggested thresholds):**

* `ryker_busy_rejections_total` rate > 100/min for 5m
* Handler `timeout` outcome > 0.1% or p95 latency ≥ 90% of deadline for 10m

---

## 4) Common Failure Modes

| Symptom (host view)     | Likely Cause                       | Ryker Signal / Evidence                            | Resolution                                                               | Alert Threshold |
| ----------------------- | ---------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------ | --------------- |
| 429 / Busy errors       | Mailbox saturation (backpressure)  | `ryker_busy_rejections_total` rising               | Throttle producers; increase `mailbox_capacity` (cold); re-tune fairness | >100/min 5m     |
| Frequent actor restarts | Faulty handler / panics            | host bus `ServiceCrashed`; restart counters        | Fix handler; widen `rapid_fail_window`; verify backoff base/cap          | >3/5m           |
| Deadline timeouts       | Handler slow or deadline too tight | `ryker_handler_latency_seconds{outcome="timeout"}` | Optimize handler; raise `defaults.deadline`; reconsider batch/yield      | p95 ≥ 0.9×DL    |
| Config reload failures  | Invalid env/file                   | `ryker_config_reload_errors_total` > 0; ERROR log  | Validate config; correct env/file; re-apply; keep aliases WARN-only      | any             |
| Drops on shutdown       | Drain deadline too short           | `ryker_mailbox_dropped_total{reason="shutdown"}`   | Increase shutdown deadline; ensure handlers are cancel-safe              | >10 per drain   |

> Cross-tie alerts: see `docs/OBSERVABILITY.md §5`.

---

## 5) Diagnostics

**Logs**

```bash
journalctl -u <host-service> -f | jq 'select(.crate=="ryker")'
```

**Metrics**

```bash
curl -s http://<host>:<metrics_port>/metrics | grep '^ryker_'
# or with promtool:
promtool query instant http://<prom>/api/v1/query 'rate(ryker_busy_rejections_total[5m])'
```

**Bus Events (if host emits)**

```bash
ronctl tail --topic ryker.config.applied
```

**Tracing**

```bash
RUST_LOG=info,ryker=debug <host-service-binary>
```

**Concurrency/Property Debug**

```bash
RUSTFLAGS="--cfg loom" cargo test -p ryker -- --ignored
# See docs/CONCURRENCY.md §10 for loom/property tests
```

---

## 6) Recovery Procedures

1. **Mailbox Saturation**

* Detect: spike in `ryker_busy_rejections_total`; high depth gauge
* Act: throttle producers; increase `mailbox_capacity` (cold); ensure `yield_every_n_msgs >= batch_messages`
* Escalate: if persists >15m or affects SLOs → **page on-call**

2. **Handler Timeouts**

* Detect: `timeout` outcome in latency histogram increases
* Act: profile/optimize; raise default deadline in config; reduce batch size to improve fairness
* Escalate: sustained p95 ≥ 0.9×deadline for 30m → **page on-call**

3. **Config Drift**

* Detect: reload errors; WARN about deprecated env keys
* Act: run `RykerConfig::from_env_validated()` locally with same env; fix and redeploy
* Escalate: repeated failures block readiness → **page on-call**

4. **Restart Storms**

* Detect: bus `ServiceCrashed` rate > 3/5m
* Act: widen `rapid_fail_window`; inspect crash logs; feature-flag problematic actors if supported by host
* Escalate: if not stabilized in 30m → **page on-call**

5. **Shutdown Drops**

* Detect: `reason="shutdown"` drops non-zero
* Act: raise drain deadline; ensure cancel-safe handlers (`select!` pattern)
* Escalate: if still dropping at next deploy → **change freeze** for the service

---

## 7) Backup / Restore

* Ryker is **stateless**. No backups or restores.
* Any persistence lives in host services (see their runbooks).

---

## 8) Upgrades

* Bump ryker dependency in host crate; follow SemVer rules.
* Gates:

  * `cargo public-api --deny-changes` (or snapshot update per `docs/API.md`)
  * `cargo doc --all-features -D warnings`
  * Host regression covering: env→config snapshot, hot/cold reload, backpressure behavior
* Optional: canary one host replica before fleet rollout.

---

## 9) Chaos Testing

* Quarterly drill:

  * Inject panics in a test actor under load; verify host supervision restarts
  * Simulate queue saturation; confirm Busy→429 mapping and alerts
  * Force config reload error; confirm fail-closed & alert
* Tie to tests:

  * See `docs/CONCURRENCY.md §10` (loom) and `tests/` property tests

---

## 10) Scaling Notes

* In-process knobs:

  * `defaults.mailbox_capacity`, `defaults.max_msg_bytes` (cold)
  * `fairness.batch_messages`, `fairness.yield_every_n_msgs` (hot)
* Scale-up trigger examples:

  * `ryker_mailbox_depth` sustained near capacity
  * `ryker_busy_rejections_total` > threshold
* Horizontal scaling is at the **host** layer (replicas).

---

## 11) Security Ops

* No secrets stored/logged by ryker; redact payloads; labels contain no PII.
* Amnesia mode: set `amnesia.enabled=true` and `zeroize_on_drop=true`; verify via logs/metrics.
* PQ crypto: **N/A** in ryker; see repo-wide quantum posture (e.g., `docs/QUANTUM.md` or `docs/HARDENING_BLUEPRINT.MD` section on timing/side-channels).

---

## 12) References

* [CONFIG.md](./CONFIG.md)
* [SECURITY.md](./SECURITY.md)
* [OBSERVABILITY.md](./OBSERVABILITY.md)
* [CONCURRENCY.md](./CONCURRENCY.md)
* [API.md](./API.md)
* Blueprints: [HARDENING_BLUEPRINT.MD](../../docs/HARDENING_BLUEPRINT.MD), [CONCURRENCY_AND_ALIASING_BLUEPRINT.MD](../../docs/CONCURRENCY_AND_ALIASING_BLUEPRINT.MD), [OMNIGATE_BLUEPRINT.MD](../../docs/OMNIGATE_BLUEPRINT.MD)

---

## 13) Failure Flow (Mermaid)

```mermaid
flowchart TD
  A[Ingress at Host] -->|enqueue try_send| B[ryker mailbox (bounded)]
  B -->|Full| C[Busy]
  C -->|map by Host| D[HTTP 429 / gRPC RESOURCE_EXHAUSTED]
  B -->|Dequeued| E[Actor Handler]
  E -->|>deadline| F[Timeout Outcome + Metrics]
  E -->|panic| G[Host Supervision Restart]
  style B fill:#0ea5e9,stroke:#0369a1,color:#fff
```

**Text alt:** Host attempts `try_send` into a bounded ryker mailbox; on full, host returns Busy/429. Messages dequeued flow into the actor handler; timeouts or panics are surfaced via metrics and host supervision.

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Metrics green on ryker counters/histograms (`busy_rejections`, `handler_latency`, reloads)
* [ ] Gate J: Chaos drill passed (panic storm, saturation, reload error)
* [ ] Gate K: Continuous vigilance (alerts wired, dashboards live)
* [ ] Gate L: Resilience/Black-Swan readiness reviewed (escalations, drain, rollback)
* [ ] Gate N: Perf profiled vs. SLOs (p95 < deadline; 429 < 1%)

```
```
