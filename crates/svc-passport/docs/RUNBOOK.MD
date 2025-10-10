````markdown
---
title: RUNBOOK — svc-passport
owner: Platform / Identity (Pillar 3)
msrv: 1.80.0
last-reviewed: 2025-10-09
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — svc-passport

## 0) Purpose
Operational manual for `svc-passport`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics) and is aligned with `PERFORMANCE.md`, `OBSERVABILITY.md`, `SECURITY.md`, and `CONFIG.md`.

---

## 1) Overview
- **Name:** `svc-passport`
- **Role:** Passport issuance/verification and revocation propagation (control-plane identity service).
- **Criticality Tier:** **1 — critical service** (authn/authz plane; issuance must fail-closed).
- **Dependencies (northbound/southbound):**
  - `ron-bus` (event bus): consume revocation/epoch events; publish health/service events.
  - `ron-kms` (key operations): signing/verification and PQ-hybrid if enabled.
  - `ron-auth` (policy/capability checks) — local or remote depending on deployment.
  - `svc-registry` (optional): policy/identity registry lookups.
  - System DNS/TLS (outbound), clock (NTP) for time-based validation.
- **Ports Exposed (typical):**
  - `http_addr` (API): e.g., `0.0.0.0:9446`
  - `metrics_addr`: e.g., `0.0.0.0:9909` (`/metrics`, `/healthz`, `/readyz`)
- **Data Flows (summary):**
  - **Ingress:** HTTP/1.1 + TLS; `POST /v1/passport/issue`, `POST /v1/passport/verify`, `POST /v1/passport/revoke`
  - **Egress:** bus events (revocation acks, health), RPC/UDS to KMS/Auth, optional reads from Registry.
- **Version Constraints:** Workspace MSRV 1.80.0; requires `ron-kernel`/`ron-transport` matching minor (frozen API per release branch).

---

## 2) Startup / Shutdown

### Startup
```bash
./target/release/svc-passport --config /etc/ron/svc-passport.toml

cargo run -p svc-passport -- --config ./configs/svc-passport.toml
````

**Important config/env:**

* Env overrides:
  `RON_CONFIG=/etc/ron/svc-passport.toml`
  `RON_BUS_SOCK=/var/run/ron/bus.sock` (or TCP addr)
  `SVC_PASSPORT_HTTP_ADDR=0.0.0.0:9446`
  `SVC_PASSPORT_METRICS_ADDR=0.0.0.0:9909`
  `SVC_PASSPORT_PQ_MODE=mirror|require|enforce`
* Feature toggles (if compiled): `--pq`, `--amnesia`, `--uds-kms`, `--uds-auth`.

**Verification**

```bash
curl -fsS http://127.0.0.1:9909/readyz
curl -fsS http://127.0.0.1:9909/healthz
```

Logs should show `ready=1 service=svc-passport` and `subscribed topic=revocation`.

### Container/K8s quickstart

```bash
docker run --rm \
  -e RON_CONFIG=/etc/ron/svc-passport.toml \
  -e SVC_PASSPORT_HTTP_ADDR=0.0.0.0:9446 \
  -e SVC_PASSPORT_METRICS_ADDR=0.0.0.0:9909 \
  -p 9446:9446 -p 9909:9909 \
  -v /etc/ron:/etc/ron:ro \
  ghcr.io/rustyonions/svc-passport:<version>
```

```yaml
apiVersion: apps/v1
kind: Deployment
metadata: { name: svc-passport }
spec:
  replicas: 2
  selector: { matchLabels: { app: svc-passport } }
  template:
    metadata: { labels: { app: svc-passport } }
    spec:
      containers:
      - name: svc-passport
        image: ghcr.io/rustyonions/svc-passport:<version>
        args: ["--config","/etc/ron/svc-passport.toml"]
        ports:
          - { name: http, containerPort: 9446 }
          - { name: metrics, containerPort: 9909 }
        env:
          - { name: SVC_PASSPORT_PQ_MODE, value: "mirror" }
        volumeMounts:
          - { name: ron-config, mountPath: /etc/ron, readOnly: true }
      volumes:
        - name: ron-config
          configMap: { name: svc-passport-config }
```

### Shutdown

* `SIGINT`/`Ctrl-C`: graceful stop (emits `KernelEvent::Shutdown` on bus).
* systemd: `systemctl stop svc-passport`
* Drains: stops accepting new requests; completes in-flight (bounded by `graceful_shutdown_timeout`).

---

## 3) Health & Readiness

* **/healthz** — process liveness (cheap).
* **/readyz** — full readiness: bus subscribed; outbound deps healthy (`ron-kms`, optional `ron-auth`/`svc-registry`); concurrency caps set; cold caches warmed.
* **Expected time-to-ready:** **2–5s**.
* **If not ready after 10s:**

  1. Logs for `ServiceCrashed{service,reason}`.
  2. Metrics:

     * `svc_passport_dependency_up{dep="kms|auth|registry"} == 1`
     * `bus_overflow_dropped_total == 0`
     * `passport_issuance_blocked{reason}` no spikes.
* **Burn-rate alerts:** tie SLO alerts to burn rates (fast 2h ≥ **14×**, slow 24h ≥ **6×**). Alert annotations must link to §6 Runbook steps.

---

## 4) Common Failure Modes

| Symptom                           | Likely Cause                               | Metric / Log                                                          | Resolution                                                                             | Alert Threshold          |
| --------------------------------- | ------------------------------------------ | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------ |
| 429/503 spikes on `/issue`        | Concurrency cap hit / ingress backpressure | `busy_rejections_total{endpoint="/v1/passport/issue"}`                | Scale out; raise `limits.rps` cautiously; verify CPU headroom policy                   | 429/503 > 1% for 5m      |
| p95 issuance > 40ms, p99 > 100ms  | KMS RTT or retries; semaphore too wide     | `svc_passport_issue_latency_seconds_*`, KMS client error/latency logs | Tighten KMS semaphore; reduce retry attempts; check network/DNS; add replica           | p95 breach for 3 windows |
| Verify p95 > 10ms with low CPU    | Verify cache misses / revocation churn     | `passport_verify_cache_hit_ratio`, revocation event rate              | Increase cache TTL (1–3s); investigate epoch bumps; regionalize callers                | Hit-ratio < 70% for 10m  |
| Readiness flaps                   | Dependency flapping (KMS/Auth/Registry)    | `svc_passport_dependency_up{dep=...}`, connection error logs          | Stabilize dep; enable circuit breaker; increase backoff jitter                         | >3 flaps/30m             |
| Frequent panics / restart loop    | Bug or misconfig                           | `ServiceCrashed` events; systemd restart count                        | Capture core/trace; rollback to last known good; open incident                         | >3 restarts/5m           |
| Revocation propagation > 5s (p99) | Bus lag or registry latency                | `revocation_propagation_seconds_*`, `bus_lagged_total`                | Check bus depth; throttle issuance; reindex registry; consider temporary cache tighten | p99 > 5s for 10m         |
| “Signature invalid” after deploy  | Clock skew or key rotation drift           | Verification error logs; NTP status; KMS key id mismatch              | NTP re-sync; confirm active key IDs; roll epoch bump                                   | any sustained increase   |
| 5xx bursts                        | Dependency outage or serialization bombs   | `http_requests_total{status=~"5.."}; body_size_bytes`                 | Enforce body cap (1 MiB); block offending clients; fail-closed issuance                | 5xx > 0.1% for 5m        |
| Inter-region latency > budget     | WAN path chosen by client                  | region-labeled `request_count` + `verify p95/p99`                     | Nudge callers to regional endpoint; enable edge verify cache; add nearby replica       | p95 > 120ms for 10m      |

---

## 5) Diagnostics

**Logs**

```bash
journalctl -u svc-passport -f
journalctl -u svc-passport -p warning -S -10m
```

**Metrics (Prometheus text)**

```bash
curl -s http://127.0.0.1:9909/metrics | grep -E 'svc_passport|busy_rejections|revocation|dependency_up'
```

**PromQL Snippets**

* Issuance p95:
  `histogram_quantile(0.95, sum(rate(svc_passport_issue_latency_seconds_bucket[5m])) by (le))`
* Backpressure (429):
  `sum(rate(busy_rejections_total{endpoint="/v1/passport/issue"}[5m]))`
* Revocation p99:
  `histogram_quantile(0.99, sum(rate(revocation_propagation_seconds_bucket[5m])) by (le))`

**Bus Events (tail)**

```bash
ronctl tail --topic svc-passport --since 10m
```

**No-ronctl fallback**

```bash
socat - UNIX-CONNECT:/var/run/ron/bus.sock | jq -c 'select(.topic=="svc-passport")'
curl -fsS localhost:9909/readyz
curl -fsS localhost:9909/healthz
```

**Async/Perf**

```bash
RUST_LOG=info,svc_passport=debug tokio-console
cargo flamegraph -p svc-passport
```

**Config inspection**

```bash
ronctl config print svc-passport
ronctl config check svc-passport --file /etc/ron/svc-passport.toml
```

---

## 6) Recovery Procedures

### Incident Workflow (template)

1. **Declare**: set severity, page on-call, open incident doc (corr_id, time, version).
2. **Contain**: enforce fail-closed issuance; add replica or RPS cap to stabilize.
3. **Diagnose**: confirm symptom (metrics), identify bottleneck (flamegraph/tokio-console).
4. **Remediate**: apply smallest-safe fix (rollback, config revert, scale).
5. **Validate**: SLOs green for 30m; close incident.
6. **Post-mortem**: within 72h, capture timeline, root cause, and lasting fixes.

1) **Overload / Backpressure**

   * *Symptom:* 429/503 increase, p95 breaches.
   * *Do:* Add one replica; reduce `issue_concurrency` by 25%; confirm CPU < 70%/core; verify recovery.
   * *Then:* Re-tune KMS semaphore (smaller) and retry budgets.

2) **KMS Degradation**

   * *Symptom:* Issuance p95↑, errors on sign/verify.
   * *Do:* Check `dependency_up{dep="kms"}`; open circuit; fail-closed issuance; route verify to cached/regional if configured.
   * *Then:* Engage KMS owners; consider temporary RPS cap.

3) **Revocation Lag**

   * *Symptom:* p99 propagation > 5s.
   * *Do:* Inspect bus depth/lag; throttle issuance; verify registry responsiveness; issue *epoch bump* to invalidate caches.
   * *Then:* Monitor `revocation_propagation_seconds`.

4) **Verify Cache Pathology**

   * *Symptom:* Hit-rate < 70%, verify p95↑ with normal CPU.
   * *Do:* Increase TTL to 3s; ensure revocation events not storming; consider per-region cache.

5) **Clock Skew / Signature Invalid**

   * *Symptom:* Post-deploy verify failures.
   * *Do:* Re-sync NTP; verify KMS active key ID; roll back to last good or rotate keys; re-run readiness.

6) **Config Drift / Bad Rollout**

   * *Symptom:* Valid requests rejected; readiness flaps.
   * *Do:* `ronctl config check`; revert to last known good config; SIGHUP or restart; confirm `/readyz = 200`.

7) **Crash Loop**

   * *Symptom:* >3 restarts/5m.
   * *Do:* Capture logs/core; scale out another replica to maintain availability; rollback binary to `vX.Y.Z`.

---

## 7) Backup / Restore

* **Statefulness:** `svc-passport` is **stateless** (no persistent DB). No regular backups required.
* **Secrets/Keys:** Private keys live in `ron-kms`. Only configuration files need backup per standard ops.
* **Restore:** Redeploy binary and config; verify readiness; keys remain managed by KMS.

---

## 8) Upgrades

1. Announce drain; set instance to *draining* (stop accepting new requests).
2. Deploy new binary; run automated smoke (`/healthz`, `/readyz`, canary issue/verify).
3. **Canary gates:** promote only if issuance p95 within **+10%** of baseline and 5xx < **0.1%** over **10m**.
4. If regressions breach CI gates, rollback immediately to prior tag (`vX.Y.Z`).

---

## 9) Chaos Testing

* **Quarterly drill (Gate J/K):**

  * Inject **latency** on KMS (+50/100/200ms); expect controlled p95 increase and **no** unbounded queues.
  * Simulate **revocation storm** (epoch bump + N events) and verify p99 propagation ≤ **5s**.
  * Run **slow-loris** on ingress; confirm early 429 and stable CPU.
* **ARM/edge profile:** run same injects on ARM node; record cold-start delta (expect ≤ +50ms) and power notes if available.
* **Pass criteria:** SLOs within budgets, alerts with runbook links fired, autoscale/manual playbook recovered.

---

## 10) Scaling Notes

* **Targets (see `PERFORMANCE.md`):** issuance p95 ≤ **40ms**, p99 ≤ **100ms**; verify p95 ≤ **10ms**. Reference throughput: **~500 rps** per 4c/8GB node (issuance-heavy).
* **Vertical:** More cores reduce JSON/crypto latency; pin crypto workers if introduced.
* **Horizontal:** Stateless — add replicas behind gateway; scale when CPU > **70%** for **15m** or p95 breaches for **3** windows.
* **Headroom:** Maintain ≥ **30%** CPU headroom; pre-emptively scale for PQ `enforce` mode.

**K8s HPA (CPU + custom p95)**

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata: { name: svc-passport }
spec:
  scaleTargetRef: { apiVersion: apps/v1, kind: Deployment, name: svc-passport }
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource: { name: cpu, target: { type: Utilization, averageUtilization: 70 } }
    - type: Pods
      pods:
        metric:
          name: svc_passport_issue_latency_p95_ms
        target:
          type: AverageValue
          averageValue: 40
```

---

## 11) Security Ops

* **Log Hygiene:** No plaintext secrets; redact tokens; include `corr_id` in all structured logs.
* **Key Custody:** All keys via `ron-kms`; rotate with `ronctl cap rotate` (or KMS-native flow).
* **PQ Readiness:** Modes: `mirror` (measure), `require` (negotiate), `enforce` (strict). Measure CPU impact before `enforce`.
* **Revocation Integrity:** On incident, execute **epoch bump**; verify caches invalidate. Track `revocation_propagation_seconds` until p99 ≤ **5s**.
* **Audit Hooks:** All issuance events mirrored to `ron-audit` (append-only); verify audit ingestion lag < **5s**.

---

## 12) References

* [`CONFIG.md`](./CONFIG.md) — limits, addresses, PQ mode, cache knobs
* [`PERFORMANCE.md`](./PERFORMANCE.md) — SLOs, gates, triage
* [`OBSERVABILITY.md`](./OBSERVABILITY.md) — metrics, dashboards, alert names
* [`SECURITY.md`](./SECURITY.md) — hardening caps, PQ rollout, redaction policy
* [`CONCURRENCY.md`](./CONCURRENCY.md) — semaphores, graceful shutdown
* [`HISTORY.md`](./HISTORY.md) — incidents, regressions, and fixes (post-mortems)
* **Blueprints:** Hardening, Concurrency & Aliasing, Scaling, Omni-Gate (GMI-1.6)

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A:** Metrics green (issuance/verify latency, dependency_up, backpressure)
* [ ] **Gate F:** Perf regressions barred (CI gates enforced)
* [ ] **Gate J:** Chaos drill passed (KMS latency, revocation storm, slow-loris)
* [ ] **Gate K:** Continuous vigilance (burn-rate alerts wired; runbook links in alerts)
* [ ] **Gate L:** Scaling validated under chaos (autoscale or manual SRE playbook)
* [ ] **Gate N:** ARM/edge profile captured (cold-start, power where applicable)
* [ ] **Gate O:** Security audit clean (redaction, key custody, revocation integrity)

---

## 13) History (Post-Mortems & Fixes)

* **2025-MM-DD — <title>**

  * Impact: <who/what/where>
  * SLO breach: <metric + duration>
  * Root cause: <brief>
  * Fix: <rollback/config/patch>
  * Prevention: <tests/alerts/process>
  * Links: <PRs, dashboards, incident doc>

```
```
