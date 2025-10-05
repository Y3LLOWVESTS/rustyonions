
---

````markdown
---
title: RUNBOOK — svc-storage
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-04
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — svc-storage

## 0) Purpose
Operational manual for `svc-storage`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview
- **Name:** `svc-storage`
- **Role:** Content-addressed blob store (BLAKE3 `b3:<hex>`); HTTP service for `GET/HEAD/Range` & `PUT` (digest-on-write).
- **Criticality Tier:** 1 (critical service)
- **Dependencies:**
  - **Required:** Local filesystem volumes (data dirs), ron-bus (for health/events), OS ulimit settings
  - **Optional:** Placement/repair coordinator (cluster pacing), hedged-read policy
- **Ports Exposed (defaults):**
  - `bind_addr=0.0.0.0:8080` (HTTP)
  - `metrics_addr=127.0.0.1:9909` (Prometheus; recommended fixed port, default may be `:0`)
  - Optional UDS: `/run/svc-storage.sock` (internal planes)
- **Data Flows:** 
  - **Ingress:** HTTP requests (`GET/HEAD/Range/PUT`) with body cap **1 MiB** (decompress ratio cap ≤ **10×** + absolute cap)
  - **Egress:** Filesystem I/O, repair/placement operations, bus health events, metrics
- **Version Constraints:** Workspace MSRV **1.80.0**; service API is HTTP; no extra public Rust surface

---

## 2) Startup / Shutdown

### Startup
```bash
# With config file
cargo run -p svc-storage -- --config ./configs/svc-storage.toml

# Release binary
./target/release/svc-storage --config /etc/ron/svc-storage.toml

# Minimal env-only boot (overrides TOML); env prefix: SVC_STORAGE_
SVC_STORAGE_BIND_ADDR=0.0.0.0:8080 \
SVC_STORAGE_METRICS_ADDR=127.0.0.1:9909 \
SVC_STORAGE_MAX_CONNS=1024 \
SVC_STORAGE_LIMITS_MAX_BODY_BYTES=1MiB \
SVC_STORAGE_LIMITS_DECOMPRESS_RATIO_CAP=10 \
SVC_STORAGE_STORAGE_CHUNK_SIZE=64KiB \
SVC_STORAGE_STORAGE_DATA_DIRS='["/var/lib/ron/storage"]' \
svc-storage
````

**Verification**

```bash
# readiness must be green before admitting traffic
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9909/readyz   # expect 200
curl -s http://127.0.0.1:9909/metrics | head                              # metrics stream
```

Logs should include an explicit `ready=1` message with service, route counts, and data-dir summary.

### Shutdown

```bash
# foreground
Ctrl-C

# systemd
systemctl stop svc-storage
```

Expected behavior: drain intake, finish in-flight up to idle timeout, then emit bus `Shutdown` event.

---

## 3) Health & Readiness

* **/healthz** — process liveness (always 200 if process is up)
* **/readyz** — true readiness: listeners bound, data dirs accessible, bus subscribed, repair/placement pacer ready
  **Target:** ready within **2–5s** under normal cold start.

**If not ready after 10s:**

1. Inspect logs for reasons: volumes inaccessible, pacer not ready, bus subscribe failure.
2. Check metrics:

   * `rejected_total{reason="init"|"not_ready"}`
   * `bus_overflow_dropped_total`
   * `rf_target`, `rf_observed` (during repair)
3. Validate config by echoing effective settings (if exposed) or reviewing TOML/env.

---

## 4) Common Failure Modes

| Symptom                        | Likely Cause                            | Metric / Log Hint                                   | Resolution                                                     | Alert                                                           |                   |
| ------------------------------ | --------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------- | ----------------- |
| 413 on writes                  | Body > 1 MiB or decompress cap exceeded | `rejected_total{reason="body_cap"                   | "decompress_cap"}`                                             | Confirm caps are correct; if false positive, adjust caps; retry | warn              |
| 429/503 bursts                 | Backpressure / rate limit               | `rejected_total{reason="rate_limit"                 | "backpressure"}`                                               | Raise `max_inflight`/`max_rps` cautiously; scale out            | page if sustained |
| p95 > 100 ms on range reads    | Cold cache / disk seeks / no hedging    | Latency histograms; low cache hit; hedging disabled | Enable hedged reads (e.g., 30 ms); warm cache; add replicas    | warn                                                            |                   |
| Frequent 5xx                   | Volume not mounted / disk errors        | `io_errors_total`, logs with `EIO`/`ENOENT`         | Remount/replace disk; verify data-dir ownership/permissions    | page                                                            |                   |
| Not Ready (stuck)              | Placement/repair pacer gating readiness | `/readyz` false with reason; `rf_*` gauges unstable | Wait or lower pacing; ensure target RF available; see §6.3     | warn                                                            |                   |
| Integrity failure on serve     | Digest mismatch / corrupted chunk       | `integrity_fail_total{reason="digest_mismatch"}`    | Re-replicate/restore; block bad object; investigate provenance | page                                                            |                   |
| Repair storms → latency spikes | Aggressive repair pacing                | `repair_bytes_total`, high disk IO                  | Cap to ≤ **50 MiB/s** per repair worker; stagger jobs          | warn                                                            |                   |
| Panics or restart loop         | Bug / bad config                        | `ServiceCrashed` bus events; crash logs             | Rollback config/binary; open incident; attach crash core       | page                                                            |                   |

---

## 5) Diagnostics

**Logs (JSON-lines)**

```bash
journalctl -u svc-storage -f
# or, if running directly:
RUST_LOG=info svc-storage 2>&1 | jq -r '.'
```

Log fields include: `ts, level, service="svc-storage", event, route, method, status, corr_id, latency_ms, b3_prefix, bytes_in/out, reason, amnesia`.

**Metrics (Prometheus)**

```bash
curl -s http://127.0.0.1:9909/metrics | grep -E 'requests_total|latency_seconds|rejected_total|integrity_fail_total|rf_(target|observed)'
```

**Bus Events**

```bash
ronctl tail --topic health --since 10m | grep svc-storage
```

**I/O & System**

```bash
df -h /var/lib/ron/storage
iostat -xm 2 5
lsof +D /var/lib/ron/storage | head
ulimit -n
```

**Async/Perf**

```bash
# When built with the feature / dev profile
tokio-console # attach to running process if enabled
cargo flamegraph -p svc-storage
```

---

## 6) Recovery Procedures

### 6.1 Config Drift / Bad Deploy

* **Symptom:** requests rejected or wrong caps/routes.
* **Action:** validate TOML; ensure env overrides are correct (prefix `SVC_STORAGE_`); redeploy with known-good config.
* **Optional:** SIGHUP live-reload (if implemented); otherwise rolling restart.

### 6.2 Disk / Filesystem Issues

* **Symptom:** 5xx/IO errors; `/readyz` false; integrity failures.
* **Action:**

  1. Drain node; 2) unmount & fix/replace disk; 3) verify ownership/permissions for data dirs;
  2. run filesystem check if required; 5) restart, watch `io_errors_total` drop to baseline.

### 6.3 Repair / RF Mismatch

* **Symptom:** sustained `rf_observed` < `rf_target`, tail latencies.
* **Action:** cap repair pacing to ≤ **50 MiB/s** per worker; schedule during off-peak; confirm replicas availability; monitor `rf_*` gauges.

### 6.4 Decompression Bomb / Abuse

* **Symptom:** 413 spikes; CPU inflation on decode.
* **Action:** verify ratio/absolute caps; ensure fast-reject path; consider tighter caps; confirm attack IPs throttled upstream.

### 6.5 Overload (CPU/mem/fd)

* **Symptom:** 429/503; high CPU; socket errors.
* **Action:** raise `max_inflight`/`max_rps` cautiously; increase `ulimit -n`; scale out replicas; enable hedged reads to reduce tail.

### 6.6 Rollback

* **Trigger:** after crash loop or SLO breach post-deploy.
* **Action:** redeploy previous tagged binary; restore prior TOML; confirm `/readyz`; keep under observation ≥10m.

---

## 7) Backup / Restore

> Applies if the node stores persistent data on local volumes.

* **Backup cadence:** every 15 min incremental; nightly full (filesystem snapshots preferred).
* **Backup scope:** all `storage.data_dirs` (e.g., `/var/lib/ron/storage`).
* **Restore:**

  1. Stop service; 2) restore snapshot into data dir; 3) start service;
  2. verify sample objects by address (`GET /o/{b3}`); 5) run integrity scan if tooling exists.

---

## 8) Upgrades

1. Announce drain; remove from LB or set weight→0.
2. Snapshot config; ensure CHANGELOG reviewed (metrics/labels are SemVer-sensitive).
3. Deploy new binary; run with existing config.
4. Verify: `/readyz` 200, `ServiceCrashed` count **0**, p95s within budget for 10 min.
5. Re-add to LB; monitor error budget.

---

## 9) Chaos Testing (Quarterly, Gate J)

* **Latency injection:** +100 ms on read path; verify SLO ceilings and hedged read efficacy.
* **Slow-loris:** many half-open connections; confirm backpressure & fast close on idle timeout.
* **Zip-bomb:** decompression >10×; require 413 fast-reject, no collapse.
* **Disk full:** simulate volume at 99%; expect `/readyz` to degrade, writes refused; reads OK.

Record results in the chaos log and attach perf graphs.

---

## 10) Scaling Notes

* **Vertical:** CPU for hashing/decompression; increase `max_conns`; check `ulimit -n`.
* **Horizontal:** add replicas behind gateway; enable **hedged reads** (e.g., 30 ms) to reduce tail.
* **Knobs:**

  * `max_rps`, `max_inflight`, `read_buffer_bytes`, `write_buffer_bytes`
  * `storage.chunk_size` ≈ **64 KiB** streaming
  * `limits.max_body_bytes=1MiB`, `limits.decompress_ratio_cap=10`
* **Reference capacity:** ~**500 rps** on 4c/8 GiB profile (mix: reads + 10% 1 MiB writes).

---

## 11) Security Ops

* **Capabilities/Tokens:** store macaroon/cap files securely; **do not log** tokens or full addresses (log first 8 hex only).
* **Rotation:** rotate cap roots ≤ 30 days; reject stale tokens; enforce permission mode on key files.
* **TLS/UDS:** prefer UDS for internal planes; if TLS enabled, validate cert/key paths & permissions.
* **Amnesia Mode (Micronode):** `storage.amnesia=true` → RAM-only; refuse spill and degrade readiness instead of writing to disk.

---

## 12) References

* `docs/CONFIG.md` — keys, defaults, env prefix `SVC_STORAGE_`
* `docs/CONCURRENCY.md` — bounded channels, no await-holding-lock, readiness DAG
* `docs/OBSERVABILITY.md` — metrics catalog (`requests_total`, `latency_seconds`, `rejected_total{reason=…}`, `integrity_fail_total{…}`, `rf_*`)
* `docs/SECURITY.md` — input caps, decompression guard, token handling
* `docs/INTEROP.md` — HTTP contract & invariants
* `docs/PERFORMANCE.md` — SLOs, rigs, regression gates

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A:** Golden metrics green (`latency_seconds`, `requests_total`, error budget)
* [ ] **Gate J:** Chaos drill passed (latency, slow-loris, zip-bomb, disk full)
* [ ] **Gate K:** Continuous vigilance (alerts, dashboards, log hygiene)
* [ ] **Gate L:** Scaling validated under mixed load & repair traffic
* [ ] **Gate N:** Edge/ARM profile captured (if applicable)
* [ ] **Gate O:** Security audit clean (no plaintext secrets, proper file perms)

```

---


