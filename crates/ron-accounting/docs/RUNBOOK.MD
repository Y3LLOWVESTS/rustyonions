

---

```markdown
title: RUNBOOK — ron-accounting
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-09-19
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — ron-accounting

## 0) Purpose
Operational manual for `ron-accounting`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
This crate tracks **transient usage counters** (requests, bytes, time-slices) and periodically **exports idempotent batches to `ron-ledger`** (the sole durable economic truth). It must not accumulate business-critical state.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview
- **Name:** `ron-accounting`
- **Role:** Transient metering + exporter (idempotent batches) to `ron-ledger`
- **Criticality Tier:** 2 (supporting) — outages must not lose durable truth
- **Dependencies (runtime):**
  - `ron-bus` (events/supervision)
  - `ron-metrics` (golden metrics)
  - `ron-ledger` (append-only truth; UDS link)
  - `ron-policy` (quota/governance knobs)
  - (optional) `svc-rewarder` (downstream econ)
- **Ports / Sockets (defaults; tune per env):**
  - `127.0.0.1:9600` — metrics + `/healthz`, `/readyz`
  - `$XDG_RUNTIME_DIR/ron/ledger.sock` — UDS to `ron-ledger`
- **Data Flows:** ingest meter events → aggregate in RAM → export idempotent batches → ledger; on failure, buffer in RAM with backoff/jitter; bounded queues.
- **Version Constraints:** uses kernel public API (`Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`).

---

## 2) Startup / Shutdown

### Startup
```

ronctl config check --crate ron-accounting --file ./configs/ron-accounting.toml
cargo run -p ron-accounting -- --config ./configs/ron-accounting.toml

# or

./target/release/ron-accounting --config /etc/ron/ron-accounting.toml
ronctl bus ping --topic ron-accounting
test -S "${RON_LEDGER_SOCK:-$XDG_RUNTIME_DIR/ron/ledger.sock}" && echo "ledger sock OK"
curl -fsS [http://127.0.0.1:9600/readyz](http://127.0.0.1:9600/readyz)

```

**Environment variables**
- `RON_CONFIG` (path override)
- `RON_LEDGER_SOCK` (UDS path to ledger)
- `AMNESIA=1|0` (Micronode default: `1`)
- `FLUSH_TIMEOUT=5s` (graceful flush cap)
- `DRAIN_TIMEOUT=10s` (shutdown drain cap)

**Build features** (only if needed): `arti` (when dialing through transport layers).

**Verification**: Logs show `ready=1`, `/readyz` → `200 OK`.

### Shutdown
```

env FLUSH_TIMEOUT=5s DRAIN_TIMEOUT=10s systemctl stop ron-accounting

```
SIGINT/SIGTERM triggers flush of any in-flight export batch with a hard cap; emits `KernelEvent::Shutdown` then exits cleanly.

---

## 3) Health & Readiness
- **/healthz**: process liveness
- **/readyz**: fully serving (config loaded, bus subscribed, ledger reachable)  
  If ledger is unreachable, `/readyz` returns **503 (degraded)** while reads remain available; writes may be temporarily shed with bounded backlogs.
- **Expected time to ready:** 2–5s.  
  If not ready after 10s:
  - Inspect `ServiceCrashed{service,reason}` on the bus
  - Check `ron_accounting_rejected_total{reason="degraded"}` and `bus_overflow_dropped_total`

---

## 4) Common Failure Modes

| Symptom                                   | Likely Cause                          | Metric / Log Hint                                        | Resolution                                                                 | Alert                         |
|-------------------------------------------|---------------------------------------|-----------------------------------------------------------|----------------------------------------------------------------------------|-------------------------------|
| Export backlog grows                       | Ledger UDS down / wrong perms         | `ron_accounting_export_backlog_gauge`, `ECONNREFUSED`     | Fix UDS perms (dir 0700, sock 0600); confirm peer-cred allow-list; restart ledger | warn @ 5m, crit @ 30m        |
| /readyz = 503 (reads OK, writes degraded) | Ledger slow/unreachable               | `rejected_total{reason="degraded"}`, backlog ↑            | Keep serving reads; auto-retry exports; page if sustained                  | warn @ 5m, crit @ 30m        |
| Frequent restarts                          | Crash loop / unbounded queue          | `ServiceCrashed` events; `service_restarts_total`         | Ensure bounded queues; supervisor backoff+jitter; fix offending worker      | >3 restarts/5m               |
| 503s attributed to metering                | Quota exhaustion (expected under load)| `quota_exhaustions_total`                                 | Adjust policy limits or throttle by tenant                                 | >10/min                      |
| Cardinality explosion in metrics           | Unscoped labels (tenant/id)           | scrape size ↑; TSDB head series ↑                         | Reduce labels to canonical dimensions                                       | warn                          |
| WAL corruption (if WAL feature enabled)    | Disk glitch / capacity                | `ron_accounting_wal_corrupt_total` > 0; `wal_size_bytes` ↑| Quarantine WAL; re-export from RAM; rotate disk if needed                   | any                           |
| OOM risk                                   | Backlog cap misconfigured             | RSS ↑; backlog slope positive for prolonged period        | Enforce backlog caps; shed writes; scale horizontally                       | crit when slope>0 for 10m    |

---

## 5) Diagnostics

### Logs (structured JSON; filter by correlation ID)
```

journalctl -u ron-accounting -f | grep corr_id=

```

### Metrics (Prometheus)
```

curl -fsS [http://127.0.0.1:9600/metrics](http://127.0.0.1:9600/metrics) | egrep '^(ron_accounting_|process_|tokio_)'
curl -fsS [http://127.0.0.1:9600/metrics](http://127.0.0.1:9600/metrics) | grep '^ron_accounting_export_backlog_gauge'
curl -fsS [http://127.0.0.1:9600/metrics](http://127.0.0.1:9600/metrics) | grep '^ron_accounting_rejected_total'
curl -fsS [http://127.0.0.1:9600/metrics](http://127.0.0.1:9600/metrics) | grep '^ron_accounting_export_latency_seconds'
curl -fsS [http://127.0.0.1:9600/metrics](http://127.0.0.1:9600/metrics) | egrep '^ron_accounting_wal_(size_bytes|corrupt_total)'

```

### Bus Events (supervision + config)
```

ronctl tail --topic ron-accounting

```
Look for: `KernelEvent::ConfigUpdated{version}`, `ServiceCrashed{service,reason}`, `Shutdown`.

### Readiness / Health
```

curl -fsS [http://127.0.0.1:9600/healthz](http://127.0.0.1:9600/healthz)
curl -fsS [http://127.0.0.1:9600/readyz](http://127.0.0.1:9600/readyz)

```

### Perf Debug (local)
```

cargo flamegraph -p ron-accounting

````

---

## 6) Recovery Procedures

1. **Ledger Link Down**
   - Export backlog rises; `/readyz` degraded.
   - Fix UDS perms (dir `0700`, socket `0600`), confirm `SO_PEERCRED` allow-list, restart `ron-ledger`.
   - `ron-accounting` retries with jitter; backlog should drain when restored.

2. **Config Drift**
   - Unexpected tenant mapping/quota behavior.
   - Validate config and reload:
     ```
     ronctl config check --crate ron-accounting --file ./configs/ron-accounting.toml
     systemctl restart ron-accounting
     ```
   - Kernel emits `ConfigUpdated{version}`.

3. **Overload (CPU/mem)**
   - Export latency p95 spikes; backlog grows.
   - Enable shedding (writes degrade first), increase replicas, confirm queue bounds are enforced.

4. **Duplicate Exports (suspected)**
   - Verify idempotency keys on exports; reconcile with ledger (append-only truth). Re-run reconciliation job if needed.

5. **Clock Skew**
   - Slice boundaries misaligned.
   - Restore NTP; widen reconciliation window temporarily; re-export impacted slices (idempotent).

---

## 7) Backup / Restore
- Default profile is **stateless** (Micronode `AMNESIA=1`): **no backups required**.
- If an optional on-disk cache/WAL is enabled, snapshot only for warm-up improvements; **do not** treat it as truth (ledger remains the sole durable source).

---

## 8) Upgrades
````

systemctl stop --no-block ron-accounting

# wait until export backlog drains:

until [ "$(curl -fsS [http://127.0.0.1:9600/metrics](http://127.0.0.1:9600/metrics) | awk -F' ' '/^ron_accounting_export_backlog_gauge/{print $2}')" = "0" ]; do sleep 2; done

# deploy new binary, then:

systemctl start ron-accounting
curl -fsS [http://127.0.0.1:9600/readyz](http://127.0.0.1:9600/readyz)

```
Verify stable `service_restarts_total` and no spike in `rejected_total{reason="degraded"}` for 10 minutes.

---

## 9) Chaos Testing

### Drill 1 — Ledger Outage
```

rm -f "${RON_LEDGER_SOCK:-$XDG_RUNTIME_DIR/ron/ledger.sock}"
systemctl restart ron-ledger

```
Expected: `/readyz=503`, `export_backlog_gauge` rises (bounded), no OOM; automatic recovery within 2m after ledger returns.

### Drill 2 — Export Path Latency
```

tc qdisc add dev lo root netem delay 200ms 50ms
sleep 60
tc qdisc del dev lo root

```
Expected: `export_latency_seconds` p95 within budget; backlog growth bounded.

### Drill 3 — Kill Under Load
```

pkill -9 ron-accounting
systemctl start ron-accounting

```
Expected: supervised restart with jitter; no double-count thanks to idempotency.

### Drill 4 — Time Skew (Black Swan)
```

date -s "+2 minutes"
sleep 5
ntpdate -u pool.ntp.org

```
Expected: slice-boundary tolerance; reconciliation corrects any drift.

### CI Hooks (optional)
```

RUSTFLAGS='--cfg loom' cargo test -p ron-accounting -- --ignored
cargo test -p ron-accounting --test chaos_export -- --nocapture

```

---

## 10) Scaling Notes
- **Vertical:** CPU helps aggregation; watch alloc churn from metric labels.
- **Horizontal:** Stateless; run N replicas; shard by tenant/key; exports idempotent, so double-dials are safe during failover.
- **Scale triggers:** `inflight_requests > 80% capacity` for 10m or export p95 above SLO for 10m rolling.

---

## 11) Security Ops

- **Capabilities only;** never trust ambient identity. Short TTL; revocation paths in auth.
- **Log hygiene:** no plaintext secrets; redact tokens; log key IDs only.
- **UDS hardening (ledger link):**
```

stat -c "%a %n" "$XDG_RUNTIME_DIR/ron"
stat -c "%a %n" "${RON_LEDGER_SOCK:-$XDG_RUNTIME_DIR/ron/ledger.sock}"
ronctl auth probe --crate ron-accounting --cap write:usage --tenant TENANT_ID

````
- **PQ posture:** `ron-accounting` holds no durable keys; honors node-level PQ toggles (KMS/transport); must not impede PQ rollout.

---

## 12) References
- `CONFIG.md`, `SECURITY.md`, `OBSERVABILITY.md`, `CONCURRENCY.md`, `PERFORMANCE.md`, `INTEROP.md`, `IDB.md`
- Blueprints: Hardening, Scaling, Six Concerns, Omni-Gate, Complete Crate List

---

## 13) Visual: Export Flow (for auditors)

```mermaid
flowchart LR
Ingress[Ingress Services] -- meter events --> A[ron-accounting]
A -- idempotent batch export --> L[ron-ledger (append-only truth)]
A <-- supervision/events --> K[ron-kernel Bus]
A -- metrics/health --> P[(Prometheus/SRE)]
````

---

## ✅ Perfection Gates Checklist (self-verifying)

* [ ] **Gate A — Observability:**
  `curl /metrics` shows:
  - `ron_accounting_export_backlog_gauge`
  - `ron_accounting_export_latency_seconds`
  - `ron_accounting_rejected_total`
  - `service_restarts_total`
* [ ] **Gate J — Chaos:**
  Drills (socket drop, latency, kill, skew) pass; recovery ≤ 2m; no OOM.
* [ ] **Gate K — Vigilance:**
  Prometheus alerts loaded:
  - `RonAccountingBacklogHigh`
  - `RonAccountingDegradedWritesSustained`
  - `RonAccountingRestartStorm`
* [ ] **Gate N — Profiles:**
  `AMNESIA=1` leaves no disk artifacts (verify with `lsof | grep deleted`).
* [ ] **Gate O — Security:**
  UDS perms dir `0700`, socket `0600`; token redaction confirmed (`grep -i 'Bearer ' logs` returns empty).

---

## SLOs

* Export latency **p95 < 150ms** (steady), **< 500ms** (peak 5m)
* `ron_accounting_export_backlog_gauge < 1000` for **99.9%** of minutes/day
* **Zero data loss:** nightly idempotent reconciliation passes 100%

```

---

