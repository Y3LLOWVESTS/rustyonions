
---
title: RUNBOOK — <crate>
owner: <owner>
msrv: 1.80.0
last-reviewed: 2025-09-19
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — <crate>

## 0) Purpose
Operational manual for `<crate>`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).  

---

## 1) Overview
- **Name:** `<crate>`
- **Role:** (e.g., gateway, overlay, index)
- **Criticality Tier:** (0=kernel, 1=critical service, 2=supporting)
- **Dependencies:** (e.g., ron-bus, svc-index)
- **Ports Exposed:** e.g., `oap_addr=9444`, `metrics_addr=9909`
- **Data Flows:** ingress/egress summary (e.g., OAP/1 frames in, bus events out)
- **Version Constraints:** requires ron-kernel ≥ vX.Y (frozen API)

---

## 2) Startup / Shutdown
### Startup
```bash
cargo run -p <crate> -- --config ./configs/<crate>.toml
# or
./target/release/<crate> --config /etc/ron/<crate>.toml
```

* Env vars: `RON_CONFIG`, `RON_INDEX_SOCK`, `<crate>_ADDR`
* Feature flags: `--tor`, `--legacy-pay`, etc.

**Verification**:

* Logs show `ready=1` event.
* `curl http://127.0.0.1:9909/readyz` → `200 OK`.

### Shutdown

* `Ctrl-C` (SIGINT) → clean bus `Shutdown` event.
* systemd: `systemctl stop <crate>`

---

## 3) Health & Readiness

* **/healthz** = process alive
* **/readyz** = fully serving (bus subscribed, deps connected)
* Expected: ready within 2–5s.
* If not ready after 10s:

  * Check `ServiceCrashed{reason}` on bus.
  * Inspect metrics `bus_overflow_dropped_total`.

---

## 4) Common Failure Modes

| Symptom                 | Likely Cause          | Metric / Log                | Resolution                        | Alert Threshold |
| ----------------------- | --------------------- | --------------------------- | --------------------------------- | --------------- |
| 503 on ingress          | Quota exhausted       | `quota_exhaustions_total`   | Increase limits or throttle       | >10/min         |
| Slow responses (>100ms) | Bus lag               | `bus_lagged_total`          | Scale workers; check backpressure | p95 >100ms      |
| Frequent panics         | Child restart loop    | bus `ServiceCrashed` events | ryker backoff policy              | >3 restarts/5m  |
| 500 errors on /resolve  | svc-index unavailable | log `connect ECONNREFUSED`  | Restart svc-index                 | any             |
| Corrupt chunk reads     | svc-storage failure   | `repair_errors_total`       | Trigger re-replication            | any             |

---

## 5) Diagnostics

* **Logs**:
  `journalctl -u <crate> -f | grep corr_id=`
* **Metrics**:
  `curl -s http://127.0.0.1:9909/metrics | grep <crate>`
* **Bus Events**:
  `ronctl tail --topic <crate>`
* **Tracing**:
  `RUST_LOG=debug <crate>` (uses `tracing-subscriber`)
* **Perf Debug**:
  `cargo flamegraph -p <crate>`

---

## 6) Recovery Procedures

1. **Config Drift**

   * Symptom: rejects valid requests.
   * Action: validate config (`ronctl config check`), reload with SIGHUP.
2. **Data Corruption** (svc-index, svc-storage)

   * Symptom: bundle unreadable.
   * Action: restore from backup, trigger re-replication.
3. **Key Loss (onion/tor)**

   * Symptom: clients cannot connect.
   * Action: rotate via `ronctl cap rotate`, update `RO_HS_KEY_FILE`.
4. **Overload**

   * Symptom: CPU pegged.
   * Action: drain connections, scale horizontally.
5. **Rollback Upgrade**

   * Use `git tag vX.Y.Z`, cargo build, redeploy.

---

## 7) Backup / Restore

* **Stateful crates** (svc-index, svc-storage):

  * Backup sled DB every 15m (hot copy).
  * Restore: stop service, replace `.data/` dir, restart.
* **Stateless crates** (gateway, overlay):

  * No backup required.

---

## 8) Upgrades

* Drain traffic (`systemctl stop --no-block`)
* Apply migrations (`ronctl migrate <crate>`) if present.
* Redeploy binary, restart.
* Verify `/readyz` + zero `ServiceCrashed` events for 10m.

---

## 9) Chaos Testing

* Run `ronctl chaos inject --target <crate> --fault=latency`
* Verify recovery and alert firing.
* Must pass quarterly chaos drill (gate J in PERFECTION\_GATES).

---

## 10) Scaling Notes

* Vertical: tune `--oap-concurrency`, default 1024.
* Horizontal: run multiple replicas, share bus (UDS).
* Monitor: scale when `inflight_requests > 80% capacity`.
* Benchmarks: handles \~500rps @ 4c/8GB.

---

## 11) Security Ops

* No plaintext secrets in logs.
* Rotate caps with `ronctl cap rotate`.
* PQ readiness: KMS supports `ed25519-dalek v2`.
* Audit trail: check `ron-audit` for minted caps.

---

## 12) References

* [CONFIG.md](./CONFIG.md)
* [SECURITY.md](./SECURITY.md)
* [OBSERVABILITY.md](./OBSERVABILITY.md)
* [CONCURRENCY.md](./CONCURRENCY.md)
* [TESTS.md](./TESTS.md)
* Blueprints: [Hardening](../../docs/Hardening_Blueprint.md), [Concurrency](../../docs/Concurrency_And_Aliasing_Blueprint.md), [Omnigate](../../docs/Omnigate_Blueprint.md)

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Metrics green (`latency`, `requests_total`)
* [ ] Gate J: Chaos drill passed
* [ ] Gate K: Continuous vigilance (logs, alerts wired)
* [ ] Gate N: ARM/edge perf profiled
* [ ] Gate O: Security audit clean



---

