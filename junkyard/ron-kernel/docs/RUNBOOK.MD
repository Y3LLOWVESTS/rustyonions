
---

````markdown
---
title: RUNBOOK — ron-kernel
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-09-26
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — ron-kernel

## 0) Purpose
Operational manual for `ron-kernel`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).  

---

## 1) Overview
- **Name:** ron-kernel  
- **Role:** Microkernel (supervision, bus, config, health, metrics)  
- **Criticality Tier:** 0 (foundation; all other crates depend on it)  
- **Dependencies:**  
  - `ron-bus` (internal broadcast)  
  - `ron-metrics` (observability)  
  - `ron-config` (config snapshot/reload)  
- **Ports Exposed:**  
  - None by default (library).  
  - Demo bins: `metrics_addr=127.0.0.1:9909` (Prometheus + healthz/readyz).  
- **Data Flows:**  
  - In: Config/env/CLI → `ConfigUpdated` bus events.  
  - Out: `KernelEvent::{Health,ConfigUpdated,ServiceCrashed,Shutdown}` → subscribers.  
- **Version Constraints:** ron-kernel ≥ v1.0 (API frozen: Bus, KernelEvent, Metrics, HealthState, Config).

---

## 2) Startup / Shutdown
### Startup
```bash
cargo run -p ron-kernel -- --config ./configs/ron-kernel.toml
# or, for demo bin:
./target/release/ron-kernel-demo --config /etc/ron/ron-kernel.toml
````

* Env vars: `RON_CONFIG`, `RON_KERNEL_METRICS_ADDR`
* Feature flags: `--features tls,pq,kameo`

**Verification**:

* Logs: `KernelEvent::Health{ok=true}` appears.
* `curl http://127.0.0.1:9909/readyz` → `200 OK` (demo only).

### Shutdown

* `Ctrl-C` (SIGINT) → kernel emits `KernelEvent::Shutdown`.
* Systemd: `systemctl stop ron-kernel`.
* On shutdown, all bus subscribers receive `Shutdown` before process exit.

---

## 3) Health & Readiness

* **/healthz** = process liveness (demo bin).
* **/readyz** = kernel ready (bus active, config loaded).
* Expected: ready within 2–5s.
* If not ready after 10s:

  * Check bus for `ServiceCrashed{reason}`.
  * Inspect metric `bus_lagged_total`.

---

## 4) Common Failure Modes

| Symptom               | Likely Cause             | Metric / Log               | Resolution                         | Alert Threshold |
| --------------------- | ------------------------ | -------------------------- | ---------------------------------- | --------------- |
| 503 on /readyz        | Config invalid / reload  | log `ConfigUpdated{error}` | Fix config, `SIGHUP` to reload     | any             |
| Slow readiness (>10s) | Bus backpressure         | `bus_lagged_total` > 0     | Scale consumers, reduce queue size | sustained >100  |
| Frequent restarts     | Child crash loop         | `ServiceCrashed` events    | Check restart caps/backoff policy  | >5/5m           |
| Metrics endpoint 500  | Bind conflict / TLS fail | log `bind error`           | Rebind on free port, check certs   | any             |

---

## 5) Diagnostics

* **Logs**:
  `journalctl -u ron-kernel -f | jq .`
* **Metrics**:
  `curl -s http://127.0.0.1:9909/metrics | grep ron_kernel`
* **Bus Events**:
  `ronctl tail --topic kernel`
* **Tracing**:
  `RUST_LOG=debug cargo run -p ron-kernel-demo`
* **Perf Debug**:
  `cargo flamegraph -p ron-kernel`

---

## 6) Recovery Procedures

1. **Config Drift**

   * Symptom: reload fails, degraded readiness.
   * Action: validate config (`ronctl config check`), correct file, send SIGHUP.

2. **Restart Storms**

   * Symptom: repeated panics in child tasks.
   * Action: check `ServiceCrashed` logs, adjust backoff, quarantine misbehaving service.

3. **Overload**

   * Symptom: bus lag counters increasing.
   * Action: increase consumer capacity, tune `bus.capacity`.

4. **Rollback Upgrade**

   * Symptom: new kernel release unstable.
   * Action: redeploy last known good git tag & binary.

---

## 7) Backup / Restore

* **ron-kernel is stateless**.
* No backup required beyond config files.
* Ensure `/etc/ron/ron-kernel.toml` is in version control or secret store.

---

## 8) Upgrades

* Drain: `systemctl stop --no-block ron-kernel-demo`.
* Redeploy binary or update library dependency.
* Restart, validate `/readyz`.
* Confirm no `ServiceCrashed` events for ≥10m.

---

## 9) Chaos Testing

* Run:
  `ronctl chaos inject --target ron-kernel --fault=panic`
* Expect: supervisor enforces backoff/jitter, restart capped.
* Verify: `/readyz` degrades then recovers.
* Must pass quarterly chaos drill (**Perfection Gate J**).

---

## 10) Scaling Notes

* **Vertical**: bus capacity and task concurrency can be tuned in config (`max_conns`, `bus.capacity`).
* **Horizontal**: multiple kernel instances embed in services, each with local bus.
* Monitor scaling trigger: sustained `bus_lagged_total > 0` or `queue_depth > 80%`.

---

## 11) Security Ops

* No persistent secrets in kernel.
* If demo TLS used: rotate certs ≤30d; store with 0600 perms.
* Amnesia mode ON disables secret persistence; recommended in prod.
* Ensure structured logs are shipped to central SIEM.
* Audit trail: subscribe to bus events via `ron-audit`.

---

## 12) References

* [CONFIG.md](./CONFIG.md)
* [SECURITY.md](./SECURITY.md)
* [OBSERVABILITY.md](./OBSERVABILITY.md)
* [CONCURRENCY.md](./CONCURRENCY.md)
* [TESTS.md](./TESTS.md)
* Blueprints: Hardening, Concurrency & Aliasing, Omnigate

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Metrics green (`latency`, `requests_total`)
* [ ] Gate J: Chaos drill passed
* [ ] Gate K: Continuous vigilance (logs, alerts wired)
* [ ] Gate N: ARM/edge perf profiled
* [ ] Gate O: Security audit clean

---

```
