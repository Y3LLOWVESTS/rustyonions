
---

title: RUNBOOK — ron-kms
owner: RustyOnions Core · Identity & Policy (ron-kms)
msrv: 1.80.0
last-reviewed: 2025-10-09
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — ron-kms

## 0) Purpose

Operational manual for `ron-kms`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.
Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `ron-kms`
* **Role:** Key custody & policy; sign/verify, wrap/unwrap, key lifecycle; classical + PQ/hybrid.
* **Criticality Tier:** **1 — critical service** (gates signing for the whole fabric).
* **Dependencies (runtime):**

  * `ron-bus` (control/events)
  * Backends: **PKCS#11 HSM**/**TPM**, or sealed-file backend (profile-specific)
  * `ron-audit` (cap mint/audit trails)
  * `ron-policy` (optional external policy lookups)
* **Ports Exposed (default):**

  * API (HTTP/JSON): `:9446`
  * gRPC (optional): `:9445`
  * Metrics/health: `:9909`
* **Data Flows:**

  * Ingress: HTTP/gRPC requests (`/v1/kms/sign|verify|wrap|unwrap|keys|rotate`).
  * Egress: bus events (health, config), audit records, backend calls (HSM/TPM/file).
* **Version Constraints:**

  * Requires `ron-kernel ≥ vX.Y` (broadcast bus API frozen).
  * TLS via rustls, axum 0.7 stack (workspace pins).
  * MSRV 1.80.0.

---

## 2) Startup / Shutdown

### Startup

```bash
# Dev
cargo run -p ron-kms -- --config ./configs/ron-kms.toml

# Prod
/usr/local/bin/ron-kms --config /etc/ron/ron-kms.toml
```

**Key env vars (override config):**

* `RON_CONFIG=/etc/ron/ron-kms.toml`
* `KMS_API_ADDR=0.0.0.0:9446`
* `KMS_GRPC_ADDR=0.0.0.0:9445`     # optional
* `KMS_METRICS_ADDR=0.0.0.0:9909`
* `KMS_BACKEND=pkcs11|tpm|file`
* `KMS_PKCS11_MODULE=/usr/lib/pkcs11.so`
* `KMS_PQ_PRODUCE_HYBRID=true|false`
* `KMS_PQ_VERIFY_MODE=or|and|pq_only`
* `KMS_PQ_ROLLOUT_PERCENT=0..100`
* `KMS_VERIFY_CACHE_MAX=262144`
* `KMS_VERIFY_CACHE_TTL_MS=300000`
* `KMS_MAX_INFLIGHT_SIGN=512`
* `KMS_MAX_INFLIGHT_VERIFY=1024`
* `KMS_TENANT_SHARE_MIN=0.05`

**Verification**

* Logs show `service=ron-kms ready=1`.
* `curl -fsS http://127.0.0.1:9909/readyz` → `200 OK`.
* Metrics include `kms_latency_seconds*` and `verify_cache_*_total`.

### Shutdown

* SIGINT/SIGTERM → graceful drain, bus `Shutdown` event.
* systemd: `systemctl stop ron-kms` (ensure watchdog not flapping).

---

## 3) Health & Readiness

* `GET /healthz` → process alive (always cheap, no backends).
* `GET /readyz` → fully serving (bus subscribed, backend pools warm, policy loaded).
* Expected ready within **2–5s**. If not **ready after 10s**:

  1. `journalctl -u ron-kms -n 200 --no-pager` (backend/policy init errors)
  2. `curl -s :9909/metrics | grep -E 'backend_reconnects|ServiceCrashed|rejected_total'`
  3. `ronctl tail --topic kms --since 10m` (bus events)

**Red flags**:

* `backend_reconnects_total` growing fast
* `rejected_total{reason="backpressure"}` sustained > 1%
* `kms_audit_integrity_failed_total > 0`

---

## 4) Common Failure Modes

| Symptom                   | Likely Cause                                    | Metric / Log                                                       | Resolution                                                                                                                              | Alert          |
| ------------------------- | ----------------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| 503/429 spikes under load | Inflight caps saturated (burst), noisy neighbor | `rejected_total{reason="backpressure"}`; p95 up                    | Raise `KMS_MAX_INFLIGHT_*` 10–20%, enable **per-tenant fair-share** (`KMS_TENANT_SHARE_MIN`), scale out                                 | 429/503 >1% 5m |
| 5xx on sign               | HSM/TPM flap / pool exhaustion                  | `backend_reconnects_total`, error logs                             | Reduce pool concurrency 30%, widen backoff; if policy allows, **verify-only** failover to sealed-file; block sign if attestation needed | 5xx >0.1% 5m   |
| Verify p95 > SLO          | Verify cache miss storm, stale cache            | `verify_cache_{hit,miss,stale}_total`, `cache_age_ms`              | Increase cache size 25% or shorten TTL; pre-warm; address rotate flood                                                                  | p95>5ms (miss) |
| `pq_policy` rejects       | Misconfigured rollout / wrong phase             | `kms_reject_total{reason="pq_policy"}`                             | Temporarily set `KMS_PQ_VERIFY_MODE=or` or lower `KMS_PQ_ROLLOUT_PERCENT`; notify tenant owner                                          | any            |
| Not ready post-deploy     | Policy/attestation not ready; bus not connected | `/readyz` false; logs show `attestation_ready=false` or bus errors | Wait for deps; if stuck, restart; verify creds, time sync                                                                               | >5m not ready  |
| Clock skew                | NTP drift breaks attestation/policy             | logs with `clock_skew`                                             | Fix NTP/hwclock; restart                                                                                                                | any            |
| Memory growth             | Verify cache too large; backend leak            | RSS climbs; `verify_cache_size` high                               | Cap `KMS_VERIFY_CACHE_MAX`, shorten TTL, restart with heap snapshot; update driver                                                      | RSS>512MiB     |

---

## 5) Diagnostics

**Logs**

```
journalctl -u ron-kms -f | sed -n 's/\x1b\[[0-9;]*m//gp'
journalctl -u ron-kms -n 200 | grep 'corr_id='
```

**Metrics**

```
curl -s :9909/metrics | grep -E 'kms_|verify_cache_|rejected_total|backend_reconnects'
# Quick SLO snapshot (p95 sign)
curl -s :9909/metrics | grep 'kms_latency_seconds_p95{op="sign"'
```

**Bus Events**

```
ronctl tail --topic kms --since 10m
```

**Tracing**

```
RUST_LOG=info,ron_kms=debug /usr/local/bin/ron-kms --config /etc/ron/ron-kms.toml
```

**Perf**

```
cargo flamegraph -p ron-kms
# Async stalls
# (start tokio-console if enabled and attach to PID)
```

**Heap / Memory**

* Jemalloc (if enabled): `MALLOC_CONF=prof:true,prof_active:true` before start; dump on SIGHUP.
* One-shot: run `heaptrack` under load to confirm.

**Tenant triage (one-liners)**

```
# Top talkers (tenants) last 5m by ops
curl -s :9909/metrics | grep 'kms_ops_total' \
 | sed 's/.*tenant="\([^"]*\)".*op="\([^"]*\)".*} \([0-9.]*\)/\1 \2 \3/' | sort

# Per-tenant p95 sign (if precomputed quantiles exposed)
curl -s :9909/metrics | grep 'kms_latency_seconds_p95{op="sign"'

# Policy/PQ state for a tenant
ronctl policy inspect --tenant TENANT_ID | grep -E 'pq_|verify_mode|rollout_percent'
```

**Concurrency / loom hooks**

```
# Repro suspected pool/fairness race locally
RUSTFLAGS="--cfg loom" cargo test -p ron-kms --test loom_pools -- --include-ignored
```

---

## 6) Recovery Procedures

### 6.1 Config Drift / Bad Deploy

1. `ronctl config get ron-kms > /tmp/kms.cfg`
2. Validate: `ronctl config check /tmp/kms.cfg`
3. Rollback: deploy previous known-good; restart service.
4. Postmortem: diff config; add regression test.

### 6.2 Backend Outage (HSM/TPM)

1. Confirm via metrics (`backend_reconnects_total`) and logs.
2. Reduce pool concurrency by 30%, increase backoff jitter.
3. If policy allows: switch to sealed-file backend for **verify-only** continuity (`KMS_BACKEND=file`) and **block sign** if attestation required.
4. Notify security; record incident.

### 6.3 Verify Cache Staleness

1. Check `verify_cache_stale_max_ms` against **2×TTL**.
2. Force cache sweep; lower TTL by 25%; increase size by 25% (temporary).
3. Warm cache by replaying hot key verifies (scripted).
4. Restore defaults after tail settles.

### 6.4 PQ Policy Rollback (per-tenant)

1. If `kms_reject_total{reason="pq_policy"} > 0`:

   * Set `KMS_PQ_VERIFY_MODE=or` **or** lower `KMS_PQ_ROLLOUT_PERCENT` for the tenant.
2. Verify with `ronctl policy inspect --tenant TENANT_ID`.
3. Communicate change with tenant; schedule fix; record in audit & history.

### 6.5 Overload / Burst

1. Confirm 429/503 due to backpressure (not 5xx).
2. Raise `KMS_MAX_INFLIGHT_*` by **10–20%** cautiously.
3. Enable `KMS_TENANT_SHARE_MIN=0.05` to protect small tenants.
4. Scale horizontally; validate p95 recovers to target.

### 6.6 Clock Skew

1. `timedatectl status` and `hwclock --show`.
2. Fix NTP/hwclock; restart if attestation/policy latched.

---

## 7) Backup / Restore

**Sealed-file backend (stateful)**

* **Backup:** snapshot encrypted key store dir (quiesce or fs freeze), every 15 minutes, encrypted off-box.
* **Restore:** stop → replace dir → start → verify `/readyz` → run KAT verifies.
* **Note:** Don’t restore across incompatible policy/attestation modes without re-attesting.

**HSM/TPM (host stateless)**

* Keys live in module; host backups limited to config & policy.

---

## 8) Upgrades

1. Announce window; set maintenance page if any.
2. Drain new conns (keep-alive timeout < 5s).
3. Deploy new binary; run migrations (`ronctl migrate ron-kms`) if present.
4. Start; verify `/readyz`; ensure zero `ServiceCrashed` for 10 minutes.
5. Smoke: sign/verify (classical + PQ), wrap/unwrap.
6. Update **last-reviewed** if procedures changed.

**Rollback**: keep prior binary staged; `systemctl stop` → swap symlink → `systemctl start`.

---

## 9) Chaos Testing (Quarterly, Gate J)

* **HSM flap drill:** inject reconnect storms for 3 minutes; shed-before-work; zero audit integrity failures.
* **Burst drill:** 10× RPS for 60s every 5m; p95 within ±15% baseline; 429/503 < 1%.
* **Rotate flood:** simulate PQ migration; verify cache miss ratio and staleness alerts; no `pq_policy` rejects.
* **Network jitter:** +100ms RTT + 1% loss; verify inter-region dashboards.

**Artifact handling:** store flamegraph SVG, tokio-console capture, perf JSON, ghz/bombardier outputs as CI build artifacts and attach to incident ticket. Naming: `kms-chaos-YYYYMMDD-HHMM/<artifact>`.

---

## 10) Scaling Notes

* **Vertical:** more cores help PQ ops; pin pools roughly to core count.
* **Horizontal:** multiple replicas behind L4; stateless unless file backend.
* **Per-tenant fairness:** enable `KMS_TENANT_SHARE_MIN` to cap noisy neighbors.
* **Scale triggers:** p95 sign edging to SLO; inflight > 80% for 10m; or 429/503 > 0.5% sustained.
* **Reference capacity (baseline host):** ~11.5k sign/s (Ed25519), ~1.8k sign/s (ML-DSA), mixed 70/20/10 ≈ ~6k rps.

**ARM/edge notes:**

* ARM/Graviton often shows **10–25% lower** PQ throughput vs x86 at same vCPU; classical near-parity. Start with `KMS_MAX_INFLIGHT_*` ~10% lower; recalc baselines on ARM and append to PERFORMANCE.md §8.

---

## 11) Security Ops

* **Secrets hygiene:** no plaintext secrets in logs; verify redaction on error paths.
* **Attestation:** block sign if attestation required and missing; alert if posture < 99.99%.
* **Key rotation:** `POST /v1/kms/keys/rotate` or `ronctl cap rotate`; verify audit entries in `ron-audit`.
* **PQ readiness:** manage phases with env knobs; hybrid-AND in prod unless exception approved.
* **Access control:** capability checks precede all cryptographic work; 401/403 must be cheap.

---

## 12) References

* [`CONFIG.md`](./CONFIG.md) — env & flags
* [`SECURITY.md`](./SECURITY.md) — custody, attestation, PQ policy
* [`OBSERVABILITY.md`](./OBSERVABILITY.md) — metrics, SLOs, alerts
* [`CONCURRENCY.md`](./CONCURRENCY.md) — pools, semaphores, aliasing rules
* [`PERFORMANCE.md`](./PERFORMANCE.md) — SLOs, baselines, drift gates
* [`INTEROP.md`](./INTEROP.md) — API surfaces, DTOs
* Blueprints: Hardening, Concurrency & Aliasing, Scaling

---

## 13) Alerts (PromQL)

```yaml
groups:
- name: ron-kms.slo
  rules:
  - alert: KMS_Backpressure_Excess
    expr: sum(rate(rejected_total{reason="backpressure"}[5m])) > 0.01
    for: 5m
    labels: {severity: page}
    annotations:
      summary: "ron-kms: backpressure >1% for 5m"

  - alert: KMS_5xx_Excess
    expr: sum(rate(http_requests_total{code=~"5..",service="ron-kms"}[5m]))
          / sum(rate(http_requests_total{service="ron-kms"}[5m])) > 0.001
    for: 5m
    labels: {severity: page}
    annotations:
      summary: "ron-kms: 5xx >0.1% for 5m"

  - alert: KMS_VerifyCache_Stale
    expr: max(verify_cache_stale_max_ms{service="ron-kms"})
          > (2 * on() max(verify_cache_ttl_ms{service="ron-kms"}))
    for: 2m
    labels: {severity: ticket}
    annotations:
      summary: "ron-kms: verify cache staleness > 2× TTL"

  - alert: KMS_PQ_Policy_Rejects
    expr: increase(kms_reject_total{reason="pq_policy"}[10m]) > 0
    for: 10m
    labels: {severity: page}
    annotations:
      summary: "ron-kms: PQ policy rejects observed (Phase 0–2 tenants)"
```

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A**: Metrics green (`kms_latency_seconds`, `rejected_total`, `verify_cache_*`).
* [ ] **Gate J**: Chaos drill artifacts attached; thresholds met.
* [ ] **Gate K**: Continuous vigilance (alerts, runbook reviewed/dated).
* [ ] **Gate L**: Black swan validated (backend flap + burst + rotate flood).
* [ ] **Gate N**: ARM/edge profile tested; notes appended to PERFORMANCE.md.
* [ ] **Gate O**: Security audit (attestation, access control) clean.

---

## 14) Runbook History

* **2025-10-09**: Initial GA runbook with PQ rollback, fair-share, chaos artifacts.
* **2025-10-09**: Added PromQL alerts, tenant triage one-liners, loom hooks, ARM notes, artifact storage guidance.

---

**Repo nits (optional but recommended):**

* Add helper scripts under `scripts/runbooks/`:

  * `kms_dump_slos.sh` (metrics slice)
  * `kms_pq_rollback.sh` (per-tenant rollback)
  * `kms_failover_file_backend.sh` (verify-only failover)

These mirror the procedures above and reduce on-call toil.
