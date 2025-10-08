

---

title: RUNBOOK — omnigate
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-07
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — omnigate

## 0) Purpose

Operational manual for **omnigate**: startup, health, diagnostics, failure modes, recovery, scaling, security ops, and alerting.
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `omnigate`
* **Role:** BFF-style hydration/service coordinator (behind gateway). Stateless; orchestrates `svc-index`, `svc-storage`, `svc-mailbox`; streams results with strict backpressure.
* **Criticality Tier:** 1 (critical service impacting all read/hydration paths)
* **Dependencies:** `ron-bus` (events), `svc-index`, `svc-storage`, `svc-mailbox`. Optional pilots: `svc-mod` (mods), econ/ZK stack (ledger/zk) when enabled.
* **Ports Exposed:**

  * `service_addr` (internal HTTP/OAP): `0.0.0.0:9444`
  * `metrics_addr` (Prometheus/health): `0.0.0.0:9909`
  * `admin_addr` (optional): `127.0.0.1:9910`
* **Data Flows:** Gateway → Omnigate (join/hydration planner) → downstream (index/storage/mailbox) → streamed response → Gateway. Emits health/config events on bus.
* **Version Constraints:** workspace pins: `tokio 1.x`, `axum 0.7.x`, `tower-http 0.6.x`, `tokio-rustls 0.26.x` (rustls preferred; native-tls only via ADR).

---

## 2) Startup / Shutdown

### Startup

```bash
# From workspace root (debug)
cargo run -p omnigate -- --config ./configs/omnigate.toml

# From a packaged binary (release)
./target/release/omnigate --config /etc/ron/omnigate.toml
```

**Environment variables (subset):**

* `RON_CONFIG` — config path (overrides CLI).
* `OMNIGATE_INFLIGHT_MAX` — global inflight cap (default 512).
* `OMNIGATE_INDEX_SEMAPHORE` / `OMNIGATE_STORAGE_SEMAPHORE` / `OMNIGATE_MAILBOX_SEMAPHORE` — per-downstream concurrency (e.g., 128/128/64).
* `OMNIGATE_STREAM_CHUNK_BYTES` — stream chunk size (default 65536).
* `SERVICE_ADDR`, `METRICS_ADDR`, `ADMIN_ADDR` — listen overrides.
* `RUST_LOG=info,omnigate=debug` — structured logs.

**Flags / pilots (config or CLI):**

* `--amnesia` (Micronode: RAM-only; no disk artifacts).
* `--hedge-read` (read-path hedging with budget).
* `--pilot-pq` (Kyber+X25519 handshake profiling; OFF by default).
* `--pilot-zk` (econ/ZK hydration; OFF by default).

**Verification:**

* Logs include `ready=1`.
* `curl -sf http://127.0.0.1:9909/readyz` → `200`.
* `curl -sf http://127.0.0.1:9909/healthz` → `200`.

### Shutdown

* **Interactive:** SIGINT (Ctrl-C) → graceful drain; bus `Shutdown` emitted.
* **systemd:** `systemctl stop omnigate`
* **Kubernetes:** preStop hook (SIGTERM); readiness probe grace ≥10s; ensure in-flight drains.

---

## 3) Health & Readiness

* `GET /healthz` — process alive; invariants loaded.
* `GET /readyz` — bus subscribed; semaphores active; downstream warm checks passed. Omnigate flips **writes first** when shedding.
* **Time-to-ready:** 2–5s after start.

**If not ready after 10s:**

* Check bus `ServiceCrashed{ service }` and `ConfigUpdated{ version }`.
* Metrics to inspect:

  * `bus_overflow_dropped_total` (should be 0)
  * `downstream_connect_errors_total{service}`
  * `*_latency_seconds` (downstream warmups)
* Validate config logs (no `deny_unknown_fields` errors).

---

## 4) Common Failure Modes

| Symptom                               | Likely Cause                                     | Metric / Log Hint                                        | Resolution                                                                 | Alert Threshold                                                 |                  |
| ------------------------------------- | ------------------------------------------------ | -------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------- | ---------------- |
| 503/429 spikes on ingress             | Quota/inflight caps reached                      | `rejected_total{reason="quota"}`                         | Raise quotas gradually; verify gateway DRR; add replicas                   | >1% of requests for 5m                                          |                  |
| p95 latency > target (hydration)      | Downstream `index`/`storage` slow                | `downstream_latency_seconds{service}`                    | Tighten semaphores; enable hedged reads (budgeted); check storage locality | p95 >150ms for 10m                                              |                  |
| Frequent connect timeouts to storage  | Network path or DNS issue                        | `downstream_connect_errors_total{service="storage"}`     | Fallback endpoint; verify DNS/IP & security groups                         | >10/min                                                         |                  |
| FD leak during soak                   | Streams not closed / response bodies not dropped | `process_open_fds` trend; `lsof`; logs                   | Audit stream lifetimes; ensure `.shutdown()`; add test; restart            | Upward trend over 30m                                           |                  |
| 500s on `/view/*` only                | Policy/capability regression                     | logs `cap=deny` or `policy=error`                        | Roll back policy bundle; clear cap cache; hotfix policy                    | Any sustained elevation                                         |                  |
| Panics with restart loops             | Join planner bug / unhandled error               | bus `ServiceCrashed{service="omnigate"}` + backtrace     | Verify ryker/backoff; capture crash; disable feature flag; patch           | >3 restarts in 5m                                               |                  |
| 413/400 on media (if uploads proxied) | Body cap / decompression guard triggered         | `rejected_total{reason="body_cap"                        | "decompress"}`                                                             | Validate client; raise cap only w/ ADR; always prefer streaming | Any sudden spike |
| 5xx only for inter-region             | TLS/WAN jitter; handshake storms at gateway      | Gateway logs; omnigate inflight queueing                 | Enable handshake caching; tune per-conn limits; scale out                  | p95 inter-region >200ms for 10m                                 |                  |
| ZK timeout spikes (pilot)             | Econ/ZK overhead in hydration                    | `downstream_latency_seconds{service="ledger"}` >50ms p95 | Disable `--pilot-zk`; fallback to non-ZK; file ADR for phased roll-in      | Econ p95 >250ms for 10m                                         |                  |

---

## 5) Diagnostics

**Logs (structured JSON):**

```bash
journalctl -u omnigate -f
journalctl -u omnigate -S -30m | grep corr_id=
RUST_LOG=omnigate=trace,tower=debug ./target/release/omnigate --config ./configs/omnigate.toml
```

**Metrics:**

```bash
curl -s http://127.0.0.1:9909/metrics | grep -E 'omnigate|downstream|rejected_total|latency_seconds'
```

**Health/Readiness:**

```bash
curl -sS http://127.0.0.1:9909/healthz
curl -sS http://127.0.0.1:9909/readyz
```

**Bus Events:**

```bash
ronctl tail --topic omnigate
```

**Tracing & Perf:**

```bash
cargo flamegraph -p omnigate
# async stalls
RUSTFLAGS="--cfg tokio_unstable" RUST_LOG=debug TOKIO_CONSOLE_BIND=127.0.0.1:6669 ./target/release/omnigate --config /etc/ron/omnigate.toml
```

### Dashboards & Alerts (copy-paste starters)

**PromQL (Grafana panels):**

* p95 hydration latency:

```
histogram_quantile(0.95, sum(rate(omnigate_hydration_latency_seconds_bucket[5m])) by (le))
```

* p95 range-start latency:

```
histogram_quantile(0.95, sum(rate(omnigate_range_start_latency_seconds_bucket[5m])) by (le))
```

* Quota sheds:

```
sum(rate(rejected_total{reason="quota"}[5m]))
```

* Downstream p95 by service:

```
histogram_quantile(0.95, sum by (le,service) (rate(downstream_latency_seconds_bucket[5m])))
```

**Alert rules (Prometheus):**

```yaml
groups:
- name: omnigate-slo
  rules:
  - alert: OmnigateHydrationP95High
    expr: histogram_quantile(0.95, sum(rate(omnigate_hydration_latency_seconds_bucket[5m])) by (le)) > 0.150
    for: 10m
    labels: { severity: page, service: omnigate }
    annotations: { summary: "Omnigate hydration p95 high", runbook: "crates/omnigate/docs/RUNBOOK.md#6" }

  - alert: OmnigateRangeStartP95High
    expr: histogram_quantile(0.95, sum(rate(omnigate_range_start_latency_seconds_bucket[5m])) by (le)) > 0.100
    for: 10m
    labels: { severity: ticket, service: omnigate }
    annotations: { summary: "Range-start p95 high" }

  - alert: OmnigateQuotaShedSpikes
    expr: sum(rate(rejected_total{reason="quota"}[5m])) > 0.01 * sum(rate(http_requests_total[5m]))
    for: 5m
    labels: { severity: page, service: omnigate }
    annotations: { summary: "Quota shed >1% requests" }
```

---

## 6) Recovery Procedures

1. **Config Drift / Bad Deploy**

   * *Symptom:* unexpected sheds, rejects, or readiness flaps.
   * *Action:* `ronctl config check omnigate`; revert to last-known-good; SIGHUP or rolling restart; commit ADR for the change.

2. **Downstream Partial Outage**

   * *Symptom:* degraded p95, /view failures.
   * *Action:* Reduce the affected downstream semaphore; prefer cached/range paths; coordinate with owning team; restore and ramp back.

3. **Bus Overflow**

   * *Symptom:* `bus_overflow_dropped_total` > 0.
   * *Action:* Increase bus buffer within policy; reduce event verbosity; ensure consumers are healthy; restart **only after** confirming drain.

4. **Overload / Hotspot**

   * *Symptom:* CPU pegged, 429/503 rising.
   * *Action:* Temporarily raise quotas within SLO; add replicas; verify gateway DRR fairness; tune inflight caps; consider hedged reads.

5. **Faulty Policy or Cap Store**

   * *Symptom:* spike in denies.
   * *Action:* Roll back policy bundle / cap issuer; clear caches; run smoke tests.

6. **Rollback Upgrade**

   * Drain from LB; redeploy previous tag; verify `/readyz` and p95 within targets for 10m; diff metrics.

7. **Post-Incident Retro (Blameless)**

   * Open an ADR with root-cause summary (systemic); add any runbook deltas; create toil-reduction tasks; link to dashboards and alerts.

---

## 7) Backup / Restore

* **Omnigate:** stateless; **no backups** required.
* **Downstream (reference pointers):**

  * `svc-index`, `svc-storage`, `svc-mailbox`: hot copy every 15m; RTO target <15m; after restore, verify parity/repair jobs and data SLAs.

---

## 8) Upgrades

1. Drain connections (LB or `/drain` if enabled).
2. Apply config migrations if required (`ronctl migrate omnigate`).
3. Deploy new binary; start; verify: `/readyz` 200, no `ServiceCrashed` for 10m, p95 within SLO.
4. Canary 30m; then fleet rollout; watch alerts/dashboards.

---

## 9) Chaos Testing

* Quarterly drills (Gate J):

  * Inject downstream latency/jitter to `storage` and `index`.
  * Introduce 1% 5xx for 10m.
  * Attempt slow-loris & decompression-bomb at gateway.
* **Pass criteria:** shed-before-collapse (429/503 ≤1% steady), p95 within +10% baseline, no FD leak/memory creep, alerts fire and auto-resolve.

---

## 10) Scaling Notes

* **Vertical:**

  * `OMNIGATE_INFLIGHT_MAX` (default 512); increase cautiously.
  * Per-downstream semaphores (`index`/`storage`/`mailbox`: start 128/128/64).
  * Stream chunk 32–128 KiB (default 64 KiB) based on IO profile.

* **Horizontal:**

  * Stateless; add replicas; ensure gateway DRR weights updated and quota partitions correct.

* **When to scale:**

  * `inflight_requests` sustained >80% of cap for 10m.
  * p95 above SLO for 10m with downstream stable.
  * CPU >70% per core for 10m at steady load.
  * **Toil gate:** manual scale/limits tweaks >5/week → automate or add capacity.

* **Reference capacity (illustrative):**

  * 4c/8GiB: ~500 rps read-heavy within SLOs.
  * 16c/32GiB: ≥2000 rps read-heavy (intra-AZ) within SLOs.

* **ARM/Edge Notes:**

  * Cross-compile: `cargo build --target aarch64-unknown-linux-gnu -p omnigate --release`
  * Validate `tokio` timers and I/O latency; re-baseline p95 on ARM.

---

## 11) Security Ops

* **Secrets hygiene:** redact; never log secrets.
* **Capability rotation:**

  * Rotate: `ronctl cap rotate --service omnigate`
  * Verify: `ron-audit query --service omnigate --since 24h`
* **Ingress hardening:** 1 MiB body cap; ≤10× decompress; 5s timeouts; streaming over buffering.
* **Amnesia mode (Micronode):** assert zero disk artifacts during soak (fs watchers/audit).
* **PQ readiness (pilot):** keep `--pilot-pq` OFF in prod unless canaried; collect handshake deltas and error rates; ADR for rollout.
* **Native-TLS escape hatch:** only via ADR with explicit risk assessment (rustls is default).

---

## 12) References

* `CONFIG.md` — knobs, envs, defaults
* `SECURITY.md` — capabilities, hardening, PQ pilots
* `OBSERVABILITY.md` — metrics, logs, tracing, alert fields
* `CONCURRENCY.md` — semaphores, backpressure, zero-copy
* `PERFORMANCE.md` — SLOs, gates, harnesses, chaos criteria
* `TESTS.md` — smoke/load/soak/chaos taxonomy
* Blueprints: **Hardening**, **Concurrency & Aliasing**, **Scaling**, **Omnigate**

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A**: Metrics steady and within SLOs (latency, requests_total, rejected_total).
* [ ] **Gate D**: Dashboards & alerts verified (Grafana panels present; alert rules fire in chaos drills).
* [ ] **Gate J**: Chaos drill passed (quarterly), all alerts fired & auto-resolved.
* [ ] **Gate K**: Continuous vigilance (dashboards, alerts, log routing operational).
* [ ] **Gate L**: Black Swan readiness (shed-before-collapse verified under burst).
* [ ] **Gate N**: ARM/edge profile captured (Micronode amnesia assertions green).
* [ ] **Gate R**: Regression gates in CI (latency/throughput/CPU/MEM thresholds honored).
* [ ] **Post-Incident**: Retro complete; ADR updated; toil tickets filed.

---
