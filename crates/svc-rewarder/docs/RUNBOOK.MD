
---

title: RUNBOOK — svc-rewarder
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-14
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — `svc-rewarder`

## 0) Purpose

Operational manual for `svc-rewarder`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `svc-rewarder`
* **Role:** Reward computation service (CPU-bound deterministic calculus on sealed inputs) + idempotent **intent emit** to ledger.
* **Criticality Tier:** **1 — critical service** (economic correctness; emits ledger intents).
* **Primary Dependencies:**

  * `ron-bus` (events, health, config updates)
  * `svc-ledger` (egress: `intent {accepted|dup|error}`)
  * `ron-auth` (capability/macaroons; local cache)
  * tmpfs/disk (amnesia artifacts, ephemeral)
* **Ports Exposed (typical):**

  * `http_addr` (service): `:8080`
  * `metrics_addr`: `:9909` (`/metrics`, `/healthz`, `/readyz`)
* **Data Flows:**

  * **Ingress:** `POST /rewarder/epochs/{id}/compute {inputs_cid, policy_id, policy_hash, ...}`
  * **Compute:** bounded worker pool; deterministic math; sealed inputs
  * **Egress:** emit ledger intent with idempotency key (`run_key`) → result `accepted|dup|error`
  * **Bus:** `Health`, `ConfigUpdated`, `ServiceCrashed`, `Shutdown`
* **Version Constraints:** Requires `ron-kernel ≥` the frozen API/Bus version referenced in `Cargo.toml` (see repo root pins).

---

## 2) Startup / Shutdown

### Startup

```bash
# From workspace root (dev)
cargo run -p svc-rewarder -- --config ./configs/svc-rewarder.toml

# From installed binary (prod)
./target/release/svc-rewarder --config /etc/ron/svc-rewarder.toml
```

**Key env vars** (override config):

* `RON_CONFIG` (path to config TOML)
* `HTTP_ADDR` (default `0.0.0.0:8080`)
* `METRICS_ADDR` (default `0.0.0.0:9909`)
* `MAX_INFLIGHT` (default `512`)
* `WORK_QUEUE_MAX` (default `4096`)
* `TENANT_RPS_MAX` (default `100`)
* `CONCURRENCY_COMPUTE_WORKERS` (default `num_cpus-1`)
* `LEDGER_DEADLINE_MS` (default `1000`)
* `LEDGER_MAX_RETRIES` (default `2`)
* `REQ_BODY_MAX_BYTES` (default `1048576`)
* `DECOMP_RATIO_MAX` (default `10`)
* `REQUIRE_PQ_ON` (default `false`)
* `AMNESIA_MODE` (default `true` for Micronode profile)

**Feature toggles / flags** (if compiled or exposed):

* `--profile micronode|macronode`
* `--brownout-allow` (test only; simulates dependency degradation)
* `--trace-burst` (temporarily increases tracing sample rate)

**Verification**

* Logs show: `ready=1` and worker pool sizing.
* `curl -s http://127.0.0.1:9909/readyz` → `200 OK`.
* Metrics present: `reward_compute_latency_seconds`, `ledger_intents_total`, `rejected_total`, `queue_depth`.

### Shutdown

* `Ctrl-C` (SIGINT) → clean **bus `Shutdown`** event; drains in-flight up to deadline.
* systemd: `systemctl stop svc-rewarder`
* K8s: `preStop` should wait for `/readyz` to drop and inflight to drain.

---

## 3) Health & Readiness

* **/healthz** → process up, thread pools started.
* **/readyz** → fully serving (bus subscribed, ledger reachable within budget, local cap cache warm, queue below soft limit).
* **Expected time to ready:** **<2 s** Micronode; **2–5 s** Macronode.

If not ready after **10 s**:

1. `journalctl -u svc-rewarder -e | tail -200` for startup errors.
2. Check bus for `ServiceCrashed{service="svc-rewarder"}`.
3. Verify metrics (look for reasons):

   * `readyz_degraded{cause=*}`
   * `bus_overflow_dropped_total`
   * `ledger_egress_errors_total`
   * `queue_depth`, `rejected_total{reason}`

---

## 4) Common Failure Modes

| Symptom                         | Likely Cause                                  | Metric / Log Indicators                                | Resolution                                                                                     | Alert Threshold                                                                             |                                 |
| ------------------------------- | --------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------- |
| Many **429/503** on compute     | Inflight or queue caps reached (overload)     | `rejected_total{reason=queue_full                      | over_capacity}`, `queue_depth`                                                                 | Throttle tenants; increase `TENANT_RPS_MAX` cautiously; raise capacity; verify backpressure | 429/503 > **1%** for **15 min** |
| p95 compute > **1 s**           | Hot loop regression / serialization churn     | `reward_compute_latency_seconds`, flamegraph hotspots  | Reduce allocations; reuse buffers; tune `CONCURRENCY_COMPUTE_WORKERS`; profile & fix           | p95 > **1 s** for **>5 min** (page)                                                         |                                 |
| `dup` rate > **1%**             | Idempotency key collisions / repeated submits | `ledger_intents_total{result="dup"}`                   | Audit run_key formation; ensure retries use same run_key; check tenant client behavior         | dup > **1%** sustained                                                                      |                                 |
| 5xx spikes (server errors)      | Bug, invariants breach, or dependency crash   | 5xx rate, logs: `invariant=fail`, `panic`, backtrace   | **Quarantine mode**; roll back; run golden vectors; open incident                              | 5xx > **0.1%** sustained                                                                    |                                 |
| Stuck in **degraded ready**     | Ledger brownout / slow egress                 | `readyz_degraded{cause="ledger_degraded"}`, egress p99 | Enter quarantine-only (no emit); 202 Accepted w/ `quarantine=true`; backoff; page if prolonged | p99 > **250 ms** for **>10 min** (page)                                                     |                                 |
| Growth in RSS over soak         | Leak / unbounded structure by mistake         | RSS in host monitor; `heaptrack`/`dhat` samples        | Capture heap profile; bisect recent changes; fix; restart                                      | Any monotonic growth                                                                        |                                 |
| Compression/size bomb attempted | Malicious input                               | `rejected_total{reason="decompress_ratio"              | "body_too_big"}`                                                                               | Confirm caps enforced; 400 the request; consider tenant quarantine if repeated              | any (security ticket)           |

---

## 5) Diagnostics

**Logs**

* Follow with correlation IDs (search `corr_id=` if present):

  * `journalctl -u svc-rewarder -f`
* Increase verbosity temporarily:

  * `RUST_LOG=info,svc_rewarder=debug ./target/release/svc-rewarder --config /etc/ron/svc-rewarder.toml`

**Metrics**

* Full dump:

  * `curl -s http://127.0.0.1:9909/metrics | tee /tmp/metrics.txt`
* Quick greps:

  * `curl -s http://127.0.0.1:9909/metrics | grep reward_compute_latency_seconds`
  * `curl -s http://127.0.0.1:9909/metrics | egrep 'ledger_intents_total|rejected_total|queue_depth|readyz_degraded'`

**Bus Events**

* Tail events (health/config/crash):

  * `ronctl tail --topic kernel`
  * `ronctl tail --topic svc-rewarder`

**Profiling**

* CPU hotspots:

  * `cargo flamegraph -p svc-rewarder`
* Async stalls:

  * Run under `tokio-console`; inspect long polls, blocked tasks.

**Golden vectors (sanity)**

* Run S/M/L sealed vectors locally to validate correctness/perf after changes.

---

## 6) Recovery Procedures

### 6.1 Overload (429/503, queue_full)

1. Confirm via metrics: `queue_depth`, `rejected_total{reason}`, inflight.
2. Apply tenant throttles: set lower `TENANT_RPS_MAX` or enforce per-tenant limits.
3. Scale up:

   * Increase `CONCURRENCY_COMPUTE_WORKERS` (watch CPU < 70% mean).
   * Add replicas; ensure LB honors `/readyz`.
4. Verify recovery: 429/503 drop below 1%, p95 returns to target.

### 6.2 Ledger Brownout (egress degraded)

1. Observe `readyz_degraded{cause="ledger_degraded"}` and egress p99 > 250 ms.
2. **Enter quarantine-only mode** (automatic if configured): compute result returned with `202 Accepted`, `quarantine=true`; **do not emit**.
3. Backoff retries on pending emits; raise `LedgerEgressDegraded` page.
4. Exit quarantine after 10-min egress p95 < 100 ms and queue below soft limit.
5. Reconcile any quarantined results: re-emit with same idempotency keys.

### 6.3 Invariant Failure (economic safety)

1. Logs include `invariant=fail` and `reward_invariant_fail_total` > 0.
2. **Immediately quarantine**: block further emits; only allow dry-run if enabled.
3. Pull manifest and inputs CIDs; compare against golden vectors.
4. Roll back to prior good build; open incident; attach flamegraph and console traces.
5. RCA; add regression test; re-enable emits only after sign-off.

### 6.4 Memory Leak / RSS Growth

1. Capture heap profile (heaptrack/dhat).
2. Reproduce with S soak for 60 min.
3. Bisect recent changes; fix; redeploy.
4. Verify flat RSS over 24h soak.

### 6.5 Config Drift / Bad Config

1. Validate:

   * `ronctl config check svc-rewarder`
2. Hot-reload:

   * Send `SIGHUP` or `SIGUSR1` to reload clamps (`TENANT_RPS_MAX`, queue caps).
3. If still wrong, restart with known-good config.

### 6.6 Rollback Upgrade

1. Drain:

   * `systemctl stop --no-block svc-rewarder` or cordon pod.
2. Deploy prior tag/binary; restart.
3. Verify `/readyz == 200` and no `ServiceCrashed` for 10 min.
4. Re-run S/M/L vectors; confirm metrics back to baseline.

---

## 7) Backup / Restore

`sfc-rewarder` is **stateless** by design (compute + emit). Persistent state lives in ledger/index/storage.

* **No routine backup** required for `svc-rewarder`.
* If amnesia artifacts are written, they must be **tmpfs/ephemeral**. On crash, start clean.

---

## 8) Upgrades

1. Announce window; ensure error-budget headroom.
2. Canary one instance; compare p95, dup ratio, and error rate vs baseline.
3. Roll forward if green; else roll back.
4. After fleet rollout: watch `reward_compute_latency_seconds` and `ledger_intents_total{result}` for 30 min.

---

## 9) Chaos Testing (Quarterly)

* **Latency injection** on ledger egress (50–500 ms jitter).
* **Slow-loris** against HTTP ingress.
* **Compression/size bombs** (expect clean reject; no perf collapse).
* **Brownout drill:** ensure quarantine-only mode engages and recovers.
* Pass criteria: no invariant failures; burn-rate within policy; alerts fire and clear.

---

## 10) Scaling Notes

* **Vertical:** Tune `CONCURRENCY_COMPUTE_WORKERS = num_cpus-1`; confirm mean CPU < 70%.
* **Horizontal:** Add replicas; LB ejects instance when `/readyz` degrades > 15 s.
* **When to scale:** `queue_depth > 80%` of `WORK_QUEUE_MAX` for >10 min, or sustained p95 > 500 ms.

---

## 11) Security Ops

* **Capabilities:** All compute requires valid macaroon/cap; cache TTL honored. If auth upstream > budget, 503 with `cap_timeout=true`.
* **Idempotency:** Always emit with `run_key`; treat `dup` as **successfully idempotent** (not an error).
* **PQ/Hybrid:** If `REQUIRE_PQ_ON=true`, expect ≤ **+5% p95** CPU/latency overhead; reduce workers by 1 if needed.
* **Amnesia Mode:** Default **on** in Micronode; zeroization on shutdown; verify no artifacts persist.
* **Secrets:** No plaintext in logs. Rotate caps via `ronctl cap rotate`. Rotate ledger/auth creds per schedule.

---

## 12) References

* CONFIG.md — knobs, defaults, hot-reload notes
* SECURITY.md — caps, PQ/hybrid, acceptance criteria
* OBSERVABILITY.md — required metrics & dashboards
* CONCURRENCY.md — bounded work, worker pools, queueing
* PERFORMANCE.md — SLOs, burn-rate, dependency budgets, chaos suite
* INTEROP.md — HTTP surfaces, idempotency details
* IDB.md — invariants (conservation, idempotency, bounded work)
* Blueprints — Hardening, Concurrency & Aliasing, Scaling, Omni-Gate

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A**: Metrics green (latency, dup ratio, error budget)
* [ ] **Gate F**: CI perf regressions barred (thresholds enforced)
* [ ] **Gate J**: Chaos drill passed (brownout, slow-loris, bombs)
* [ ] **Gate K**: Continuous vigilance (alerts routed, dashboards live)
* [ ] **Gate L**: Scaling validated under dependency brownouts
* [ ] **Gate N**: Edge/mobile cold-start profiled (<2 s)
* [ ] **Gate O**: PQ/hybrid overhead ≤ 5% and documented

---

### One-liners (grab-bag)

* Ready check:

  * `curl -sS localhost:9909/readyz -w ' -> %{http_code}\n' -o /dev/null`
* Hot-reload tenant clamps:

  * `pidof svc-rewarder | xargs -n1 kill -USR1`
* Quick perf smoke (local):

  * `bombardier -c 64 -n 5000 -m POST -f testing/performance/payloads/S.json http://localhost:8080/rewarder/epochs/demo/compute`

---
