
**Path:** `crates/ron-bus/docs/RUNBOOK.md`

```markdown
---
title: RUNBOOK — ron-bus
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-09-27
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — ron-bus

## 0) Purpose
Operational manual for `ron-bus`: usage in hosts, health heuristics, diagnostics, failure modes, recovery, scaling, and security ops.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview
- **Name:** `ron-bus`
- **Role:** **in-process broadcast bus** for kernel/events; bounded, lossy, observable fan-out.
- **Criticality Tier:** **0 (kernel-adjacent)** — used by kernel/service hosts to coordinate.
- **Dependencies (direct):** `tokio 1.x` (broadcast), optional `tokio-util` in examples, metrics facade (host-provided).
- **Ports Exposed:** **N/A (library)** — hosts expose `/metrics`, `/healthz`, `/readyz`.
- **Data Flows:** in-proc `Event` values → cloned to each **non-lagging** subscriber; lagged receivers report `Lagged(n)`.
- **Version Constraints:** MSRV 1.80.0; Tokio 1.x (pinned in workspace); API semver per `docs/API.md`.

---

## 2) Startup / Shutdown
`ron-bus` is embedded. There is no daemon to start/stop. Hosts construct and drop `Bus`.

### Startup (host usage)
- Construct with validated capacity (see CONFIG.md), register metrics handles, spawn subscriber tasks each with a **unique receiver**.
- Verify:
  - Host `/readyz` includes a key like `bus_attached=true`.
  - Metrics include `bus_overflow_dropped_total` (0 after warmup) and `bus_queue_depth` ≈ baseline.

**Developer smoke (repo-local)**
```

cargo test -p ron-bus --lib
cargo bench -p ron-bus

```

### Shutdown (host usage)
- Signal cancellation (watch or `CancellationToken`), allow subscriber loops to drain, then drop `Bus`.
- On reload requiring different capacity: **create a new Bus**, cut subscribers over, then drop the old (see §6.3).

**Amnesia mode (RAM-only)**
- Behavior is operationally identical for ron-bus (no persistence), but hosts may run with reduced buffers to control RSS.
- Expect faster shutdown (no flush-to-disk); verify `amnesia_label="on"` is present in metrics (host-exported) during smoke tests.

---

## 3) Health & Readiness
- **Library:** no endpoints.  
- **Host readiness gates (recommended):**
  - `bus_attached=true`
  - `bus_queue_depth` under threshold for T seconds after warmup
  - no sustained increase in `bus_overflow_dropped_total`
  - `amnesia_label` present and correct (if enabled); not a gate by itself, but mismatches are a config drift signal

- **Degraded readiness:** if `bus_queue_depth` remains elevated or overflow counter grows monotonically → host `/readyz` returns `503` with reason.

**Amnesia baseline:** In Micronode profiles with `amnesia=on`, maintain a **lower capacity baseline**; alert if `bus_queue_depth` baseline drifts upward (>10% of capacity for 10m) without accompanying traffic growth.

---

## 4) Common Failure Modes

| Symptom / Signal                                          | Likely Cause                                              | Metric / Evidence                                        | Resolution (see §6)                                             | Alert Threshold (host)           |
|-----------------------------------------------------------|-----------------------------------------------------------|----------------------------------------------------------|------------------------------------------------------------------|----------------------------------|
| Sustained growth in `bus_overflow_dropped_total`          | One or more **lagging subscribers**                      | Counter slope > 0; `Lagged(n)` observed in logs/metrics  | §6.1: identify slow consumer, fix or shed; tune capacity if needed | any sustained > 0 for 10m        |
| `bus_queue_depth` stays high after burst                  | Insufficient capacity **or** persistent slow work        | Depth gauge > 50% capacity for > 5m                      | §6.1 + §6.2: reduce handler work, increase capacity (with reload) | depth > 50% for 5m               |
| Missed/duplicated handling in a task                      | **Receiver shared** across tasks (anti-pattern)          | Code review; doctest violations; intermittent behavior   | Refactor: **one receiver per task** (§6.4)                       | n/a (caught in review/tests)     |
| `RecvError::Closed` seen by subscribers unexpectedly      | Sender dropped early (premature Bus drop)                | Logs show drop; host lifecycle mismatch                  | Ensure sender lives ≥ subscribers; tie Bus lifetime to host      | spike in Closed events            |
| High CPU with low throughput                              | Payload copying (`Vec<u8>`), per-event allocations       | Flamegraph shows memcpy/alloc hotspots                   | Switch to `bytes::Bytes`/`Arc<T>`; batch tiny events             | CPU pegged with low RPS           |
| Log spam at WARN on overflow                               | Host logging every `Lagged(n)`                           | High WARN rate; I/O pressure                             | Rate-limit WARNs; rely on counters/gauges (§5, SECURITY.md)      | > 10 WARN/s                       |
| Cutover causes consumer-visible gaps externally           | Reload migration mishandled in host                      | External stream gap; counters spike at cutover           | §6.3: two-bus cutover sequence with cancel + resubscribe         | any gap during migration          |
| RSS climbs / OOM killer in Micronode (amnesia=on)         | Too-large capacity or handlers retaining buffers         | `resident_memory_bytes` ↑; `bus_queue_depth` baseline ↑  | Reduce bus `capacity`; ensure payloads use `Bytes`/`Arc<T>`; audit handlers for lingering buffers | RSS slope > 100MB/10m or any OOM |

---

## 5) Diagnostics

### 5.1 Metrics (host-owned; library-updated)
```

curl -s [http://127.0.0.1](http://127.0.0.1):<metrics_port>/metrics | grep -E 'bus_overflow_dropped_total|bus_queue_depth|bus_subscribers_total'

```
Interpretation:
- `bus_overflow_dropped_total` → should plateau; sustained slope indicates slow consumers.
- `bus_queue_depth` → should return to baseline post-burst.

Also check memory:
```

ps -o pid,rss,comm -p $(pgrep -n <host-bin>)

```

### 5.2 Tracing (host)
- Enable `tracing` with JSON; add span fields for event variant (`event_kind`).
- Include `amnesia_label` as a span field or log field to confirm profile in traces/logs.

### 5.3 Flamegraph / Perf (dev)
```

cargo flamegraph -p <host-crate> --bin <host-bin>

```
- Look for payload copies in publish/handle paths.

### 5.4 Targeted tests
```

cargo test -p ron-bus -- --ignored lagged_overflow_smoke
cargo test -p <host-crate> -- --ignored bus_reload_migration

```

---

## 6) Recovery Procedures

### 6.1 Slow Consumer / Sustained Overflow
1. Identify subscriber tasks with long handlers (profiling/tracing spans).
2. Apply one or more:
   - Make handlers **idempotent & quick**; move heavy work to separate queues.
   - Use `Bytes`/`Arc<T>` to avoid copies; avoid sync locks in handlers.
   - Temporarily **increase capacity** (see §6.3) to absorb bursts while you fix the handler.
3. Ensure host **rate-limits WARN**; rely on counters/gauge for alerting.

### 6.2 Depth Persists After Burst
- If handlers are already fast, bump capacity by a modest factor (e.g., ×2), **measure latency tradeoff**, and plan to revert when root cause is resolved.

### 6.3 Reload Migration (Capacity Change) — **Two-Bus Cutover**
1. **Create** `bus_new = Bus::new(new_cfg)`.
2. **Prepare** subscribers to accept an injected cancel token.
3. **Publish** `Event::ConfigUpdated { version: v+1 }` on old bus.
4. **Cancel** old subscribers; **spawn** new subscribers using `bus_new.subscribe()`.
5. **Swap** publisher handle to `bus_new.sender()`.
6. **Drop** old bus when no receivers remain.
7. Verify:
   - No consumer-visible gaps (see INTEROP.md §5.5).
   - No spike in `bus_overflow_dropped_total` attributable to migration.
   - `bus_queue_depth` returns to baseline.

### 6.4 Receiver-Sharing Anti-Pattern
- Symptom: missed events, weird ordering; code shares one `Receiver` across tasks.
- Fix: each task **owns its own** `Receiver` from `bus.subscribe()`. See CONCURRENCY.md §11.1.

### 6.5 Unexpected `RecvError::Closed`
- Ensure Bus (sender) outlives receivers. Tie Bus lifetime to host app context (e.g., in a struct alongside cancel tokens).

### 6.6 Amnesia memory pressure (Micronode)
1. Confirm `amnesia_label="on"` and capture RSS over 5–10 minutes.
2. Reduce `capacity` (e.g., 256 → 128), execute two-bus cutover (§6.3), and observe `bus_queue_depth` + RSS.
3. Replace large `Vec<u8>` payloads with `Bytes`/`Arc<T>`; avoid retaining buffers in handlers.
4. Re-run soak; target RSS slope ≈ 0 and overflow counter slope ≈ 0.

---

## 7) Backup / Restore
- **Stateless library** — no data to back up.  
- Hosts remain responsible for their own stateful stores.

---

## 8) Upgrades
- Follow **SemVer** (API.md).  
- Before releasing:
  - Run `cargo public-api -p ron-bus --simplified`.
  - Update `/docs/api-history/ron-bus/<ver>.txt` and CHANGELOG.
  - Ensure new `Event` variants are **additive**; changes to existing variants require **major**.
- Post-deploy verification (host):
  - `/readyz` stable for 10m.
  - No increased slope on overflow counter.

---

## 9) Chaos Testing (Quarterly)
- **Lag churn:** randomly delay N% of subscribers; ensure publishers remain non-blocking and counters behave.
- **Reload under load:** execute §6.3 while publishing at target rate; assert no external gaps.
- **Subscriber flapping:** join/leave at 100Hz; ensure no leaks and stable depth.
- **E2E propagation under reload:** Mirror events to an external stream (HTTP/OAP) while executing §6.3. Assert **no external gaps**, no artificial spike in `overflow_dropped_total`, and depth returns to baseline (see INTEROP §5.5).
- **Amnesia on/off delta:** Run identical load with `amnesia=on` and `amnesia=off` (host). Assert p95 publish→recv delta <5% and no RSS slope in amnesia mode.

Suggested invocations:
```

cargo test -p <host-crate> -- --ignored chaos_reload
cargo test -p <host-crate> -- --ignored chaos_lag_churn
cargo test -p <host-crate> -- --ignored chaos_e2e_reload
cargo test -p <host-crate> -- --ignored chaos_amnesia_profile

```

---

## 10) Scaling Notes
- **Capacity:** default 256; increase cautiously (more buffering → higher tail latency).
- **Subscribers:** expect near-linear scaling to ~32; beyond that, measure because `broadcast` is per-subscriber overhead.
- **Payloads:** prefer `bytes::Bytes`/`Arc<T>`; avoid large `Vec<u8>` clones.
- **CPU:** recv loops should be tight; avoid `.await` while holding any lock.
- **Dashboards:** plot overflow counter slope, depth gauge, subscriber count, and (if amnesia=on) memory RSS vs capacity.

---

## 11) Security Ops
- Do **not** place secrets or PII in event payloads.
- The library never logs payloads; hosts must avoid logging event bodies.
- If events are mirrored externally, hosts must validate **capabilities/macaroons** before emission (see INTEROP.md §4).
- Amnesia label (host metric) can tag RAM-only mode; it’s **observability-only**, not a policy surface.
- **Tamper-evident externalization:** If hosts mirror bus events to external sinks, record them in an append-only, hash-chained audit log (e.g., `ron-audit`). Store chain heads and verify during incident review.

---

## 12) References
- [`CONFIG.md`](./CONFIG.md) — capacity, migration guidance
- [`CONCURRENCY.md`](./CONCURRENCY.md) — one receiver per task; patterns
- [`OBSERVABILITY.md`](./OBSERVABILITY.md) — metrics and alerts
- [`SECURITY.md`](./SECURITY.md) — hardening/overflow logging discipline
- [`API.md`](./API.md) — surface & semver
- [`INTEROP.md`](./INTEROP.md) — host mapping & externalization
- [`PERFORMANCE.md`](./PERFORMANCE.md) — SLOs, benchmarks, runbook triage

---

## ✅ Perfection Gates Checklist
- [ ] Gate F: Perf regression guardrails green (see PERFORMANCE.md)
- [ ] Gate J: Chaos drills (reload + lag churn + E2E + amnesia) pass in CI
- [ ] Gate K: Continuous vigilance — overflow & depth alerts wired and tested
- [ ] Gate L: Scaling validated at target load (≥1M events/s baseline)
- [ ] Gate N: ARM/edge profile captured (if applicable)
- [ ] Gate O: Security audit (no payload logging; capability checks in hosts; tamper-evident externalization)

```

---
