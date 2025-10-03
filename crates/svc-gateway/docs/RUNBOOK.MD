

---

````markdown
---
title: RUNBOOK — svc-gateway
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-03
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — svc-gateway

## 0) Purpose
Operational manual for `svc-gateway`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan / Chaos). :contentReference[oaicite:0]{index=0} :contentReference[oaicite:1]{index=1}

---

## 1) Overview
- **Name:** `svc-gateway`
- **Role:** Hardened ingress (HTTP→OAP/1): TLS termination, quotas/backpressure, early shed; forwards to overlay/index/storage. :contentReference[oaicite:2]{index=2} :contentReference[oaicite:3]{index=3}
- **Criticality Tier:** 1 (critical service in front of Omnigate/overlay).
- **Dependencies:** `ron-bus` (events), `svc-overlay` (data fetch), `svc-index` (resolve), `svc-storage` (bytes). :contentReference[oaicite:4]{index=4}
- **Ports Exposed:** HTTP bind (ingress), `/metrics`, `/healthz`, `/readyz`, `/version` (control surface). :contentReference[oaicite:5]{index=5}
- **Data Flows:** client → gateway (HTTP) → overlay (bus RPC) ↘ index ↘ storage. :contentReference[oaicite:6]{index=6}
- **Version Constraints:** OAP/1 envelope; `max_frame=1 MiB`, streaming ~64 KiB; DTOs via `ron-proto`. :contentReference[oaicite:7]{index=7}

---

## 2) Startup / Shutdown

### Startup
```bash
# Local debug
cargo run -p svc-gateway -- --config ./configs/svc-gateway.toml

# Prod-style binary
./target/release/svc-gateway --config /etc/ron/svc-gateway.toml
````

**Key config knobs** (defaults vary by profile):

* `gateway.timeout_ms = 5000`
* `gateway.max_inflight = 512`
* `gateway.rate_limit_rps = 500`
* `gateway.body_cap_bytes = 1_048_576`         # 1 MiB (OAP bound)
* `gateway.decode_abs_cap_bytes = 8_388_608`   # 8 MiB absolute decoded cap
* `gateway.decode_ratio_max = 10`
* `gateway.amnesia = true|false`
* `gateway.danger_ok = false` (must be true to weaken caps in non-prod) 

**Verification**

```bash
curl -sS http://127.0.0.1:PORT/readyz   # expect 200 when fully ready
curl -sS http://127.0.0.1:PORT/metrics  # golden metrics exposed
```

Uniform control endpoints are required across services. 

### Shutdown

* SIGINT/SIGTERM → graceful drain; readiness flips early under pressure. (Gate asserts degrade-first.)  

---

## 3) Health & Readiness

* **/healthz**: process alive.
* **/readyz**: accepting traffic (bus OK, quotas/DRR healthy). Degrades **before** collapse and sheds writes first when overloaded.  
* **Expected readiness:** 2–5 s typical; investigate if >10 s or flapping.
* **On not-ready (503):** confirm downstreams and check reject reasons & queue depth.

---

## 4) Common Failure Modes

| Symptom                               | Likely Cause                                 | Metric / Log (evidence)                      | Resolution                                                                   | Alert Threshold    |
| ------------------------------------- | -------------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------- | ------------------ |
| 429 spikes with `Retry-After` headers | Quota/DRR limiting working as designed       | `rejected_total{reason="quota"}`; 429 JSON   | Raise per-tenant caps, smooth spikes, or add replicas                        | sustained >1% 5m   |
| 413 `decoded-cap` on uploads          | Decompression ratio/absolute cap exceeded    | `decoded_reject_total`; 413 JSON             | Reject (by design) or increase `decode_*` caps after risk review             | >10/min warn       |
| 503 `/readyz` during traffic spikes   | Degrade-first: write shed or downstream slow | `gateway_degraded{bool}=1`                   | Confirm downstream health; temporarily reduce write load; scale horizontally | >60s critical      |
| Slower p95 on GET                     | TLS/parse hot paths or downstream tail       | latency histograms; flamegraph hotspots      | Keep-alive, tune concurrency cap; investigate overlay/index/storage latency  | p95 > targets      |
| Unexpected 401/403                    | Capability missing/expired                   | error taxonomy code + audit event            | Re-issue macaroons; verify policy & TTLs                                     | any spike          |
| Large body accepted unexpectedly      | Dangerous config override                    | check config; `danger_ok=false` should block | Set `danger_ok=false`; restart; add policy to block unsafe overrides         | any (gate)         |

---

## 5) Diagnostics

**Logs**

```bash
journalctl -u svc-gateway -f | grep X-Corr-ID
```

Redact secrets; use correlation IDs; prefer JSON tracing spans (`svc.gateway.*`).  

**Metrics**

```bash
curl -s http://127.0.0.1:PORT/metrics | grep -E 'latency|rejected_total|decoded|degraded'
```

Golden metrics required across the estate. 

**Bus events**

```bash
ronctl tail --topic svc-gateway.degraded --since 10m
```

Gateway emits health/degraded/crash events on the bus. 

**Perf/Async**

```bash
cargo flamegraph -p svc-gateway
TOKIO_CONSOLE_BIND=127.0.0.1:6669 RUST_LOG=debug ./target/release/svc-gateway --config ...
```

Attach flamegraphs to regressions; inspect stalls with `tokio-console`. 

---

## 6) Recovery Procedures

1. **Config Drift / Unsafe Caps**

   * Symptom: accepting >1 MiB, too many inflight, etc.
   * Action: restore safe defaults (`danger_ok=false`), reload, re-deploy. Gate blocks unsafe overrides in prod. 

2. **Downstream Slowness (overlay/index/storage)**

   * Symptom: `/readyz` degrades, GET tails rise.
   * Action: confirm downstreams; shed writes first; horizontally scale gateway/omnigate tier.  

3. **Decode Bomb / Memory Pressure**

   * Symptom: 413 `decoded-cap`, rising decoded rejects.
   * Action: keep caps; if business-justified, raise ratio/abs-cap incrementally; re-run safety test (`100 KiB → ~12 MiB` must still 413). 

4. **Quota Exhaustion / Tenant Noisy-Neighbor**

   * Symptom: bursty 429s.
   * Action: tune DRR weights/tenant caps; consider separate pool or replica; validate <1% sustained rejects (5m). 

5. **Crash Loop**

   * Symptom: frequent restarts; bus `*.crash` events.
   * Action: inspect last config change; roll back binary; collect flamegraph + logs; file regression gate waiver only with evidence.  

---

## 7) Backup / Restore

* **Gateway is stateless** (beyond counters/metrics): no backups required; verify amnesia mode (RAM-only) on Micronode. 
* **Downstreams (index/storage)** own durability & backup procedures.

---

## 8) Upgrades

1. Drain via LB (readiness off), then stop the unit.
2. Deploy new binary; **no migrations** expected (stateless).
3. Verify `/readyz==200` and no `*.crash` events for 10m; SLO/alerts green. 

---

## 9) Chaos Testing

* Inject burst (e.g., 600 rps for 60s) → expect early 429 with `Retry-After`.
* Induce downstream slowness → `/readyz` degrades and **writes are shed**; reads continue if possible.
* Attach flamegraph to the worst tail run.  

---

## 10) Scaling Notes

* **Horizontal first**: stateless replicas behind LB; health-eject on `/readyz`. 
* **Knobs**: concurrency/inflight caps, rate-limit RPS, decode caps, timeouts; keep within OAP bounds (1 MiB / ~64 KiB).  
* **Scale signal**: sustained inflight >80% of `max_inflight` or rejects approaching 1% (5m). 

---

## 11) Security Ops

* **Capabilities-only** (macaroons); no ambient trust. Rotate short-TTL caps regularly. 
* **Redaction**: never log tokens; hash prefixes only; redact PII headers. 
* **Amnesia mode**: RAM-only artifacts on Micronode; ensure **no disk spill**. 

---

## 12) References

* [CONFIG.md] — caps/timeouts/limits & safety flags. 
* [CONCURRENCY.md] — invariants (no lock across `.await`, bounded channels). 
* [OBSERVABILITY.md] — golden metrics, alerts, exemplars, correlation.  
* **Omnigate Blueprint** — degrade-first & CI gates. 
* **Interop Blueprint** — OAP/1 contract & bounds. 

---

## ✅ Perfection Gates Checklist

* [ ] **Gate F**: perf regressions barred (baseline + flamegraph attached on fail). 
* [ ] **Gate L**: chaos: degrade-first verified (write-shed, early 429). 
* [ ] **Gate K**: continuous vigilance: alerts wired (latency/backpressure/decode). 
* [ ] OAP bounds hold (1 MiB / ~64 KiB); taxonomy stable (400/413/429/503).  

```

---
```

