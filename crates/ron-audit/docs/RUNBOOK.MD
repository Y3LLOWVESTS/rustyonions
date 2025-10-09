
---

title: RUNBOOK — ron-audit
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-08
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — ron-audit

## 0) Purpose

Operational manual for `ron-audit`: health signals, common failures, recovery, scaling, security ops, chaos drills.
Meets **PERFECTION_GATES**: **K** (Continuous Vigilance) and **L** (Black Swan Economics).

> `ron-audit` is a **library** embedded by host services (e.g., `svc-gateway`, `svc-overlay`, `svc-index`). Hosts own ports, readiness, quotas, and metric registration.

---

## 1) Overview

* **Name:** `ron-audit` (library)
* **Role:** Tamper-evident audit trail — canonicalize → BLAKE3 → hash-chain (`prev→self`) → sign; optional WAL + export/checkpoint for durability.
* **Criticality Tier:** 1 (critical correctness plane). If degraded, hosts fail-closed on writes or shed with explicit counting.
* **Dependencies (via host):** `ron-kernel` (bus events), KMS/custody for signing, WAL/export filesystem when enabled.
* **Ports Exposed:** none directly (use host’s `metrics_addr` like `:9909`).
* **Data Flows (high level):**

```mermaid
flowchart LR
  I[Host emit(record)] --> Q{bounded queue}
  Q -- ok --> A[append: canonicalize → BLAKE3 → chain → sign]
  Q -- full --> S[drop + count busy/shed]
  A -- Micronode --> R[verify on read]
  A -- Macronode --> W[WAL batched fsync] --> E[export + checkpoint]
```

* **Version Constraints:** Workspace MSRV 1.80.0; host pins a compatible `ron-audit` API version.

---

## 2) Startup / Shutdown

Because `ron-audit` is embedded, you start/stop the **host**.

### Startup (example: `svc-gateway` with audit)

```
./target/release/svc-gateway \
  --config /etc/ron/svc-gateway.toml \
  --enable-audit
```

**Typical env/flags:**

* `AUDIT_ENABLED=true`
* `AUDIT_QUEUE_DEPTH=4096`
* `AUDIT_WAL_ENABLED=true|false`            # Macronode vs Micronode
* `AUDIT_WAL_DIR=/var/lib/ron/audit/wal`
* `AUDIT_WAL_BATCH=32`
* `AUDIT_EXPORT_ENABLED=true|false`
* `AUDIT_EXPORT_DIR=/var/lib/ron/audit/export`
* `AUDIT_CHECKPOINT_EVERY=10000`            # records or time window
* `AUDIT_KEY_ID=aud-2025q4`

**Verification**

* Logs: `audit.ready=1` and no `verify_fail` lines.
* Readiness: `curl -sf http://127.0.0.1:9909/readyz` → `200`.
* Healthy metric snapshot (example):
  `audit_appended_total` increasing; `audit_verify_fail_total=0`; `busy_rejections_total` ~0.

**Sample healthy log line**

```
level=INFO ts=2025-10-08T19:10:42Z svc=svc-gateway audit.ready=1 queue_depth=4096 wal=true export=true key=aud-2025q4
```

### Shutdown

* SIGINT/SIGTERM drains audit workers; host emits `KernelEvent::Shutdown`.
* systemd:

```
systemctl stop svc-gateway
```

---

## 3) Health & Readiness

**Host endpoints**

* `/healthz` = process alive.
* `/readyz` = audit plane wired (queue, KMS/FS checks pass).

**Golden metrics (host registers)**

* Core: `audit_appended_total`, `audit_bytes_total`, `audit_queue_depth`
* Shedding: `audit_drop_total{reason}`, `busy_rejections_total{endpoint="emit"}`
* Integrity: `audit_verify_fail_total`
* Durability (Macronode): `wal_fsync_seconds` (histogram), `export_batch_seconds` (histogram), `audit_export_checkpoints_total`

**Expectations**

* Ready ≤ 5s in normal conditions.
* `audit_verify_fail_total` **must stay 0**.
* Shedding (`busy_rejections_total`) ≤ 0.5% at design load.

**If not ready after 10s**

* Check host logs for permission/KMS errors, or bus `ServiceCrashed`.
* Inspect `audit_queue_depth` and `busy_rejections_total` (mis-sized queue or overload).

**Alert rules (PromQL examples)**

* Verify failures (page): `sum(audit_verify_fail_total) > 0`
* Shedding budget: `sum(rate(busy_rejections_total[5m])) / sum(rate(audit_appended_total[5m])) > 0.005`
* WAL p95 breach: `histogram_quantile(0.95, sum(rate(wal_fsync_seconds_bucket[5m])) by (le)) > 0.008`

---

## 4) Common Failure Modes

| Symptom                                                                        | Likely Cause                                     | What to Check                                          | First Response                                                                                                                     | Alert Threshold     |
| ------------------------------------------------------------------------------ | ------------------------------------------------ | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| `audit_verify_fail_total > 0`                                                  | Bitrot/tamper; buggy canonicalization; bad disk  | Host logs around `verify_fail`; last export checkpoint | **SEV-1:** isolate range; run chain verification (§6.2); switch to **amnesia** (disable WAL/export) while restoring; page Security | any                 |
| High `busy_rejections_total` / `audit_drop_total{reason="audit_backpressure"}` | Queue too small; CPU saturated; burst > capacity | `audit_queue_depth` near cap; CPU usage; incoming RPS  | Double `AUDIT_QUEUE_DEPTH`, add workers; consider replica scale-out                                                                | >0.5% of emits (5m) |
| WAL p95 > 8ms sustained                                                        | Slow/contended disk; batch too small             | FS/mount; `AUDIT_WAL_BATCH`; I/O contention            | Move WAL to NVMe; raise batch (8→32→64); re-check p95                                                                              | p95 > 8ms (10m)     |
| Export backlog                                                                 | Cadence too aggressive; export CPU bound         | `export_batch_seconds`, missed checkpoints             | Reduce cadence; add export worker; ensure disk not full                                                                            | >2 missed intervals |
| Host not ready                                                                 | KMS offline; wrong `AUDIT_KEY_ID`; FS perms      | KMS status; key alias; dir ownership                   | Fix IAM/key; chown/chmod dirs; restart host                                                                                        | >2m not-ready       |
| Throughput drops >10% vs baseline                                              | Regression in host/dep; CPU throttle             | Compare Criterion; `dmesg` for throttle                | Roll back host; set performance governor; file regression                                                                          | >10%                |

---

## 5) Diagnostics

**Metrics**
`curl -s http://127.0.0.1:9909/metrics | grep -E 'audit_|busy_rejections|wal_fsync|export_batch'`

**Logs**
`journalctl -u <host> -f | grep -E 'audit|verify|checkpoint|busy_rejections'`

**Bus events**
`ronctl tail --topic kernel --since 10m`

**Perf (benches or host under load)**
`cargo flamegraph -p ron-audit`

**Chain spot-check (field guide tool)**
`ronctl audit verify --since "<ts1>" --until "<ts2>" --export-dir /var/lib/ron/audit/export`

---

## 6) Recovery Procedures

### 6.1 Overload / Backpressure

1. Validate shedding within budget (≤0.5%).
2. Increase queue + workers; redeploy host:

```
export AUDIT_QUEUE_DEPTH=$(( AUDIT_QUEUE_DEPTH * 2 ))
export AUDIT_WORKERS=$(( AUDIT_WORKERS + 2 ))
systemctl restart <host>
```

3. If still pegged, add replicas and rebalance traffic.

**Target MTTR:** < 2 minutes to stabilize shedding.

### 6.2 Verification Failures (SEV-1)

1. Isolate time window and recompute chain & Merkle against last checkpoint:
   `ronctl audit range --from "<ts1>" --to "<ts2>" --export-dir /var/lib/ron/audit/export`
2. If mismatch persists:

   * Switch host to **Micronode (amnesia)**:

```
export AUDIT_WAL_ENABLED=false
export AUDIT_EXPORT_ENABLED=false
systemctl restart <host>
```

* Replace/restore export set from last good snapshot; if WAL intact, replay to head; re-verify full set.

3. Re-enable WAL/export and confirm `audit_verify_fail_total==0`.

**Target MTTR:** < 15 minutes to isolate & contain.

### 6.3 Disk / Filesystem

* Move WAL/export to healthy NVMe, increase `AUDIT_WAL_BATCH`, recheck p95.
* Clear disk-full; confirm export resumes.

### 6.4 KMS / Key Issues

* Rotate key and cross-sign next checkpoint:

```
ronctl kms rotate --key-alias aud-2025q4
export AUDIT_KEY_ID=aud-2025q4
systemctl restart <host>
```

* Keep old key until two checkpoints verified with new key.

### 6.5 Config Drift

* `ronctl config check /etc/ron/<host>.toml`
* Apply hot-reload (HUP) if host supports; otherwise rolling restart.

### 6.6 Rollback Upgrade

```
systemctl stop <host>
/usr/local/bin/deploy <host> --version vX.Y.Z
systemctl start <host>
```

Verify `/readyz`, throughput near baseline, and `audit_verify_fail_total==0`.

---

## 7) Backup / Restore (Macronode only)

**Backup**

* `AUDIT_EXPORT_DIR` (manifests/checkpoints).
* Optionally `AUDIT_WAL_DIR` (for replay).

**Cadence & Retention**

* Snapshots every 15m; retain 24h of checkpoints + daily for 30d.
* Encrypt at rest per SECURITY (e.g., LUKS/volume encryption).

**Restore**

1. Stop host; restore last good export (and WAL if used).
2. Start host; run `ronctl audit verify --full --export-dir ...`
3. Admit traffic after `audit_verify_fail_total==0`.

---

## 8) Upgrades

**Rollout**

* Staging: run Criterion; compare to baseline (±10%).
* Canary 1–5% for 30m; watch `audit_*`, `wal_fsync_seconds`.
* Fleet: rolling; keep surge capacity.

**Schema / Interop growth**

* Any `AuditRecord` size increase → **re-baseline perf** and update SLOs/gates in PERFORMANCE.md.

**Downgrade**

* Symmetric to §6.6; verify schema compatibility per INTEROP before rolling back.

---

## 9) Chaos Testing (Quarterly — Gate J)

1. **Latency injection (WAL):** add 20ms±10ms for 5m → p95 fsync rises; shedding still ≤ budget.
2. **Disk-full:** quota the export volume; host goes degraded (amnesia) without crashing; alerts fire.
3. **Message storm:** 2× design load for 10m → bounded queue grows, drops counted, no OOM or panics.
4. **Bitrot drill:** corrupt a single exported chunk in staging → `audit_verify_fail_total` > 0, range isolated, recovery run.

Attach Grafana screenshots and log excerpts to the drill report.

---

## 10) Scaling Notes

* **Vertical:** add CPU; bump `AUDIT_WORKERS` up to physical cores (watch memory bandwidth in flamegraphs).
* **Horizontal:** add replicas; load-balance emits; decide centralized vs per-instance exports; budget for downstream compaction/merge.
* **Knobs to tune:** `AUDIT_QUEUE_DEPTH`, `AUDIT_WAL_BATCH`, export cadence.
* **Scale triggers:**

  * `busy_rejections_total` > 0.5% (5m), or
  * `wal_fsync_seconds` p95 > 8ms for 10m, or
  * CPU > 85% while below perf baseline.

---

## 11) Security Ops

* No secrets in logs; redact by default at host.
* Key rotation: quarterly or on incident; cross-sign checkpoints.
* Export directory perms: `root:ron` `0750` (ops group read).
* **PQ readiness:** enabling ML-DSA/Dilithium may 2× CPU cost; run perf re-baseline and temporarily relax throughput gates during rollout.

---

## 12) References

**Crate docs:** CONFIG.md · OBSERVABILITY.md · CONCURRENCY.md · PERFORMANCE.md · SECURITY.md · IDB.md
**Blueprints:** Hardening_Blueprint.md · Concurrency_And_Aliasing_Blueprint.md · Scaling_Blueprint.md · Omnigate_Blueprint.md

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A:** Metrics green (`audit_verify_fail_total==0`, shedding ≤ budget, fsync p95 ≤ target)
* [ ] **Gate F:** CI perf gates pass (no >10% regress vs baseline)
* [ ] **Gate J:** Chaos drill docs attached (latency, disk-full, storm, bitrot)
* [ ] **Gate K:** Alerts wired; on-call rota recorded; dashboards linked
* [ ] **Gate L:** Black-swan playbook tested (amnesia flip, export restore)
* [ ] **Gate N:** ARM/edge Micronode profile captured
* [ ] **Gate O:** Security audit clean; key rotation exercised

**Sign-off:** *Date*: ________  ·  *Owner*: __________________  ·  *All gates green*: ☐

---
