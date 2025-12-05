````md
---
title: RUNBOOK — svc-admin
owner: RustyOnions / RON-CORE
msrv: 1.80.0
last-reviewed: 2025-12-04
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — svc-admin

## 0) Purpose

Operational manual for `svc-admin`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.

`svc-admin` is the **admin-plane GUI + HTTP proxy** for RON-CORE nodes. It talks only to **documented admin endpoints** on nodes (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`, future `/api/v1/reload`, `/api/v1/shutdown`) and never to data-plane or shell interfaces.

This document satisfies **PERFECTION_GATES**:

- **K – Continuous Vigilance:** clear runbook & alerts.
- **L – Black Swan Economics:** tested behavior under partial node failure, slow metrics, and auth outages.

---

## 1) Overview

- **Name:** `svc-admin`
- **Role:**  
  Admin-plane service providing:
  - HTML/JS **operator dashboard** (SPA),
  - JSON API for node health, readiness, metrics, and environment view,
  - optional one-click admin actions (reload/shutdown) behind strict gates.

- **Criticality Tier:**  
  - **Tier 1 – Critical service** for operators (loss is painful but **does not** directly corrupt data or stop the data plane).
  - Nodes (gateway/overlay/storage) continue serving apps if `svc-admin` is down; you “lose eyes” but not the system.

- **Dependencies (runtime):**
  - One or more RON-CORE nodes (e.g., `macronode`, `micronode`) exposing admin HTTP endpoints.
  - Optional: identity provider (Passport / OIDC) or ingress identity headers, depending on `auth.mode`.
  - Local environment: filesystem for config and TLS keys, network connectivity to nodes & IdP.

- **Ports Exposed (typical example; see CONFIG.md for actual values):**
  - `bind_addr` — e.g., `0.0.0.0:5300` for UI + JSON API.
  - `metrics_addr` — e.g., `0.0.0.0:5310` for Prometheus `/metrics` + `/healthz` + `/readyz`.
  - Optional: Unix domain socket instead of TCP for internal deployments.

- **Data Flows:**
  - **Ingress:**
    - Operator browser → `svc-admin` via HTTP(S).
    - Monitoring systems → `/healthz`, `/readyz`, `/metrics`.
  - **Egress:**
    - `svc-admin` → nodes (HTTP(S) to admin endpoints).
    - `svc-admin` → IdP (JWKS / metadata / token checks), if `auth.mode="passport"`.
    - Logs/metrics → stdout/err, Prometheus, log shipping stack.

- **Version / API Constraints:**
  - Expects nodes implementing the **RON-CORE admin plane v1**:
    - `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`.
  - Changes in node admin API must be coordinated with `svc-admin` upgrades.

---

## 2) Startup / Shutdown

### 2.1 Startup

Common patterns:

```bash
# Dev: run from workspace
RUST_LOG=info \
SVC_ADMIN_CONFIG=./configs/svc-admin.dev.toml \
cargo run -p svc-admin -- --config ./configs/svc-admin.dev.toml

# Prod: systemd or container
/usr/local/bin/svc-admin --config /etc/ron/svc-admin.toml
````

**Config & env overrides (examples; see CONFIG.md)**:

* `--config /path/to/svc-admin.toml`
* Env-style overrides:

  * `SVC_ADMIN_CONFIG=/etc/ron/svc-admin.toml`
  * `SVC_ADMIN_BIND_ADDR=0.0.0.0:5300`
  * `SVC_ADMIN_METRICS_ADDR=0.0.0.0:5310`
  * `SVC_ADMIN_AUTH_MODE=passport|ingress|none`
  * `SVC_ADMIN_LOG_LEVEL=info|debug|trace`

**Verification (happy path):**

1. Logs:

   * Look for lines such as:

     * `svc-admin: config loaded`
     * `svc-admin: HTTP listening on ...`
     * `svc-admin: readiness achieved` or similar.

2. Health checks:

   ```bash
   curl -sS http://127.0.0.1:5310/healthz
   curl -sS http://127.0.0.1:5310/readyz | jq
   ```

   * `/healthz` → `200 OK` immediately if process is up.
   * `/readyz` → `200 OK` once:

     * config parsed,
     * listeners bound,
     * auth initialized,
     * node registry loaded,
     * metrics sampler (if enabled) has started successfully.

3. UI:

   ```bash
   open http://127.0.0.1:5300/
   ```

   * You should see the dashboard loading (even if nodes are down).

### 2.2 Shutdown

**Manual (dev):**

* `Ctrl-C` in the terminal:

  * triggers graceful shutdown of HTTP server and background tasks (metrics sampler, auth refresh).
  * `/readyz` flips to non-ready during drain.

**systemd:**

```bash
systemctl stop svc-admin
journalctl -u svc-admin -f
```

* Expect logs indicating graceful shutdown:

  * `svc-admin: shutdown requested`
  * `svc-admin: background tasks stopped`
  * `svc-admin: exit 0`

**Containers (K8s, Nomad, etc.):**

* Use standard SIGTERM → graceful shutdown flow.
* Ensure terminationGracePeriodSeconds ≥ 10–30s to allow clean drain.

---

## 3) Health & Readiness

### 3.1 Endpoints

* `GET /healthz`

  * Liveness: returns `200 OK` if the process and supervisor loop are alive.
  * Does **not** check nodes or IdP.

* `GET /readyz`

  * Readiness: returns `200 OK` only when core components are ready:

    * config loaded,
    * HTTP listener bound,
    * metrics listener bound (if configured),
    * auth mode initialized (`none`, `ingress`, or `passport`),
    * node registry constructed from config,
    * metrics sampler (if enabled) has at least one successful cycle or is disabled by configuration.

* `GET /metrics`

  * Prometheus metrics for svc-admin itself (not nodes).
  * Used to monitor latency, error rates, upstream failures, and sampler behavior.

### 3.2 Expected Behavior

* **Startup readiness:**

  * `/healthz` should be ready within **< 1s** of process start.
  * `/readyz` typically ready within **2–10s**, depending on:

    * metrics sampler initial polls,
    * initial auth/JWK fetch (passport mode).

* **Degraded but serving:**

  * If some nodes are down:

    * `/readyz` for svc-admin may still be `200 OK` (svc-admin itself is fine).
    * Node cards should show per-node errors and metrics gaps.

* **Not ready after 30s+:**

  Investigate:

  * Config issues:

    * mis-typed `bind_addr`, port already in use,
    * broken node URLs,
    * malformed TLS/Passport config.
  * Auth issues:

    * passport mode cannot reach IdP / JWKS.

  Use diagnostics in §5.

---

## 4) Common Failure Modes

| Symptom                                           | Likely Cause                                                      | Metric / Log Clues                                       | Resolution                                                                                | Alert Threshold                     |
| ------------------------------------------------- | ----------------------------------------------------------------- | -------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ----------------------------------- |
| `/readyz` stuck in 503 on startup                 | Config parse failure, listener bind error, auth/JWK fail          | startup logs; no `ready=1` log; `/metrics` missing       | Fix config (addresses, TLS, auth); restart.                                               | `/readyz` non-200 for >5m           |
| Dashboard shows **all nodes unavailable**         | Bad node URLs/DNS, network ACL, TLS handshake failures            | `svc_admin_upstream_errors_total{kind="connect"}` ↑      | Check node endpoints from same host via `curl`; fix config/DNS/TLS; restart.              | >0 upstream errors for >5m          |
| Some nodes red / others green                     | Per-node outage / node misconfig                                  | upstream errors for specific `node_id` only              | Investigate affected node(s) directly (`curl /healthz,/readyz`); fix node; not svc-admin  | 1+ nodes `ready=false` in prod      |
| Frequent 401/403 to operators                     | Misconfigured `auth.mode` or ingress/Passport headers             | logs with `action=auth_validate result=unauth/forbidden` | Fix IdP/Passport config; or fix ingress header mapping; double-check `auth.mode`.         | auth failures spike >1% of requests |
| Ops see **stale charts** (no updates for minutes) | Metrics sampler stuck, timeouts, or oversized `/metrics`          | `svc_admin_metrics_staleness_seconds` trending upward    | Inspect metrics sampler logs; maybe reduce sampling frequency or drop large metrics sets. | staleness > 3× interval for >10m    |
| High CPU on svc-admin                             | Aggressive metrics parsing, too many nodes, very frequent polling | flamegraph hotspots in parsing tasks                     | Reduce node count per instance, raise polling interval, or limit parsed metrics.          | CPU >70–80% sustained >10m          |
| 429/503 `busy` from svc-admin                     | Inbound concurrency limit hit or internal queue full              | `svc_admin_rejected_total{reason="busy"}` ↑              | Check load; scale horizontally or adjust concurrency limit carefully.                     | busy rejections >1% of requests     |
| Nodes show “amnesia” but ops unaware              | Amnesia flag not surfaced/understood                              | `amnesia=true` in `AdminStatusView`; UI badge            | Educate ops: amnesia nodes are ephemeral; adjust expectations and run policies.           | N/A (informational, not error)      |

---

## 5) Diagnostics

### 5.1 Logs

**Systemd:**

```bash
journalctl -u svc-admin -f
```

**Containers:**

```bash
docker logs -f svc-admin
# or
kubectl logs -f deploy/svc-admin
```

Filter by correlation ID:

```bash
journalctl -u svc-admin -f | grep "corr_id="
```

Look for:

* `action=auth_validate`, `result=unauth/forbidden`.
* `action=metrics_sample`, `result=error`, `kind=timeout/connect/oversized`.
* `action=node_status_proxy`, `node_id=..., result=upstream_unavailable`.

### 5.2 Metrics

Via Prometheus-style `/metrics`:

```bash
curl -s http://127.0.0.1:5310/metrics | grep svc_admin
curl -s http://127.0.0.1:5310/metrics | grep http_request_duration_seconds
```

Useful series:

* `http_request_duration_seconds_bucket{route="/api/nodes/{id}/status",...}`
* `svc_admin_upstream_errors_total{kind="connect"|"timeout"|"parse"}`
* `svc_admin_rejected_total{reason="busy"|"unauth"|"forbidden"|"oversized"}`
* `svc_admin_metrics_staleness_seconds{node_id="..."}`

### 5.3 Direct Node Probing

To distinguish svc-admin vs node issues, **bypass svc-admin** and query nodes directly from the same host:

```bash
# Example: node admin on 127.0.0.1:8090
curl -sS http://127.0.0.1:8090/readyz | jq
curl -sS http://127.0.0.1:8090/api/v1/status | jq
curl -sS http://127.0.0.1:8090/metrics | head -100
```

If these fail from the svc-admin host, the problem is node or network, not svc-admin.

### 5.4 Auth / Passport Diagnostics

* Check Passport / OIDC metadata:

  ```bash
  curl -sS "$SVC_ADMIN_PASSPORT_ISSUER/.well-known/openid-configuration" | jq '.issuer,.jwks_uri'
  curl -sS "$(jq -r '.jwks_uri' <<< "$...")" | jq '.keys | length'
  ```

* Look in svc-admin logs for:

  * `action=auth_refresh` (JWK updates).
  * `result=error`, `details=...` when IdP unreachable.

### 5.5 Perf / Concurrency Debug

* `RUST_LOG=debug` and optionally `trace` (dev only) for detailed traces.

* `tokio-console` (dev/stage):

  * Run svc-admin under `TOKIO_CONSOLE_BIND` to inspect:

    * long-running tasks,
    * blocked futures,
    * tasks that never yield.

* `cargo flamegraph -p svc-admin --bin svc-admin` under synthetic load to find CPU hotspots.

---

## 6) Recovery Procedures

### 6.1 Config Drift / Misconfig

**Symptom:** svc-admin refuses to start or `/readyz` stays 503.

1. Validate config file syntax:

   ```bash
   svc-admin --config /etc/ron/svc-admin.toml --check-config
   # (if/when we add a dedicated check; otherwise run in a staging environment)
   ```

2. Check for:

   * invalid `bind_addr` or `metrics_addr`,
   * malformed node URLs,
   * TLS cert/key path issues,
   * incorrect `auth.mode` values.

3. Fix config → restart svc-admin.

### 6.2 Auth Outage (Passport / Ingress)

**Symptom:** operators get 401/403; `/api/me` fails.

1. Confirm IdP availability (curl `.well-known` and JWKS).
2. Check svc-admin logs for `auth_validate`, `auth_refresh` errors.
3. If IdP is down and this blocks critical visibility:

   * Short-term emergency option (dev/test only): switch `auth.mode` to `none` or `ingress` where safe, **documenting the security waiver**.
   * Long-term: repair IdP or ingress headers.

Restart svc-admin after config change.

### 6.3 Node or Network Outage

**Symptom:** some nodes show as unavailable in UI; upstream errors in metrics.

1. Confirm connectivity with `curl` directly to node admin endpoints.
2. If only some nodes are down:

   * treat as **node incident**, not svc-admin.
   * svc-admin should remain healthy; they just show errors for those nodes.
3. Escalate to node owners; fix node configuration or restart node processes.

### 6.4 Metrics Overload / High CPU

**Symptom:** CPU pegged, charts laggy, `svc_admin_metrics_staleness_seconds` increasing.

1. Check node count and polling frequency:

   * In `svc-admin.toml`, locate `polling.metrics_interval` and `polling.metrics_window`.
2. Apply reductions:

   * Increase interval (e.g., from 10s → 30s).
   * Reduce number of nodes per svc-admin instance (split config, run multiple instances).
3. Consider limiting metrics parsing to the core golden metrics engaged by the UI.
4. Restart svc-admin and verify perf.

### 6.5 Overload / Backpressure

**Symptom:** many 429/503 from svc-admin itself.

1. Inspect `/metrics` for `svc_admin_rejected_total{reason="busy"}` and concurrency gauges.
2. Check load:

   * Are many automated clients polling `/api/nodes` / metrics too aggressively?
3. Actions:

   * Reduce polling rate on clients (especially automation).
   * Scale svc-admin horizontally (more replicas behind LB).
   * Carefully increase concurrency caps if well within resource headroom.

### 6.6 Roll Back Upgrade

1. Identify the last known good version (git tag and binary).
2. Remove svc-admin instance(s) from LB / traffic.
3. Deploy previous version:

   * `git checkout vX.Y.Z && cargo build --release -p svc-admin`.
4. Restart svc-admin with the previous binary.
5. Verify:

   * `/healthz`, `/readyz`.
   * Node dashboards.
   * No new errors in logs.

---

## 7) Backup / Restore

`svc-admin` is **stateless** at runtime:

* No local DB or durable application state.
* State consists of:

  * Config files (`svc-admin.toml` + environment-specific overrides).
  * TLS certificates/keys for incoming HTTPS (if used).
  * Any local Passport/IdP config (client IDs, secrets).

### 7.1 Backup

* Config and secrets should be stored in:

  * Git (for non-secret parts),
  * Secret manager (for IDs, tokens, private keys),
  * or infra-as-code (K8s Secrets, Vault, etc.).

### 7.2 Restore

* To rebuild svc-admin after a node failure:

  1. Recreate config + secret materials on a new host.
  2. Deploy binary/container.
  3. Start `svc-admin` and rejoin LB/monitoring.

No special runtime restore steps are required beyond configuration.

---

## 8) Upgrades

### 8.1 Rolling Upgrade

For HA deployments behind a load balancer:

1. **Drain one instance:**

   * Remove from LB or set drain/termination on that pod/VM.
2. **Stop svc-admin:**

   * `systemctl stop svc-admin` or equivalent.
3. **Deploy new binary/container.**
4. **Start svc-admin.**
5. **Verify readiness:**

   * `/healthz`, `/readyz`, smoke-check UI and basic API calls.
6. **Re-add instance to LB.**
7. Repeat for remaining instances.

### 8.2 Post-Upgrade Checks

* Confirm:

  * SPA loads and can fetch `/api/ui-config`, `/api/me`.
  * Node status pages work (some nodes green).
  * No new error patterns in logs (e.g., schema mismatch with nodes).

If upgrade includes **API changes** between svc-admin and nodes:

* Ensure node versions and svc-admin versions are compatible per release notes.

---

## 9) Chaos Testing

Quarterly or per-major release, run a **chaos drill** targeting svc-admin + nodes:

### 9.1 Scenarios

* **Node latency injection:**

  * Introduce 500–1000 ms latency to `/metrics` and `/api/v1/status` on a subset of nodes.
  * Expect:

    * svc-admin remains responsive,
    * charts show gaps or degraded states,
    * clear upstream error metrics.

* **Partial node outage:**

  * Take 1/5 nodes fully offline.
  * Ensure:

    * svc-admin dashboards show some nodes red but still load,
    * svc-admin remains `ready` with correct per-node errors.

* **Auth failures:**

  * Break IdP/JWKS endpoint.
  * Expect:

    * 401/403 patterns for operators,
    * `auth_refresh` errors in logs,
    * svc-admin itself staying up (though effectively locked down).

### 9.2 Success Criteria

* No crashes or panic loops in svc-admin.
* `/healthz` remains 200 and `/readyz` reflects svc-admin’s state truthfully.
* Alerts fire as expected:

  * upstream error rates,
  * stale metrics,
  * auth failure spikes.

Document each drill run and updates needed to runbooks / configs.

---

## 10) Scaling Notes

### 10.1 Vertical Scaling

* If metrics parsing or JSON mapping dominate CPU:

  * Increase CPU allocation.
  * Consider enabling more worker threads via Tokio runtime settings (if tuned).
* If memory pressure appears:

  * Reduce metrics window size,
  * Lower max number of nodes per instance.

### 10.2 Horizontal Scaling

* `svc-admin` is stateless; easiest scale-up is **more replicas**.
* Guidelines:

  * 1 instance per:

    * up to ~25–50 nodes,
    * up to ~10 concurrent operators.
  * For larger fleets, either:

    * run multiple svc-admin clusters (per environment or per node group),
    * or scale up and carefully tune metrics sampling.

### 10.3 Capacity Planning

* For per-instance target of ~200 req/s:

  * 1–2 vCPUs and 512 MiB–1 GiB RAM are sufficient in most environments.
* Monitor:

  * `http_request_duration_seconds`,
  * `svc_admin_upstream_errors_total`,
  * `svc_admin_metrics_staleness_seconds`,
  * CPU and memory via host/container metrics.

Scale out **before** hitting steady-state CPU >70% or frequent `busy` rejections.

---

## 11) Security Ops

### 11.1 Secrets Handling

* Never log:

  * node macaroon/token contents,
  * Passport/IdP secrets,
  * JWT raw bodies.
* Config:

  * keep secrets in env variables or secret stores,
  * avoid plaintext secrets in git.

### 11.2 TLS & Certificates

* For HTTPS ingress:

  * rotate certificates using standard tooling (Certbot, ACME, or operator-managed PKI).
* For outbound TLS:

  * rely on system or embedded trust roots as configured.
  * treat node admin endpoints as internal; rotate as per node’s TLS strategy.

### 11.3 Auth Modes

* `auth.mode="none"` is **dev-only**:

  * strongly discouraged in production; treat as security waiver.
* `auth.mode="ingress"`:

  * rely on trusted ingress to authenticate users and inject identity headers.
* `auth.mode="passport"`:

  * treat IdP as source of truth.
  * monitor refresh failures and 401/403 patterns.

### 11.4 Audit Trail

* Ensure centralized logging (ELK, Loki, etc.) captures:

  * operator actions,
  * node-level admin operations triggered via svc-admin (reload/shutdown),
  * correlation IDs for cross-service tracing.

If/when `ron-audit` is deployed, integrate svc-admin logs and metrics into the audit pipeline.

---

## 12) References

* [CONFIG.md](./CONFIG.md) — configuration schema, env vars, examples.
* [SECURITY.md](./SECURITY.md) — threat model, auth modes, surfaces.
* [OBSERVABILITY.md](./OBSERVABILITY.md) — metrics, logs, traces, alerts.
* [CONCURRENCY.md](./CONCURRENCY.md) — task model, channels, shutdown.
* [PERFORMANCE.md](./PERFORMANCE.md) — SLOs, benchmarks, scaling knobs.
* RON-CORE Blueprints:

  * Hardening Blueprint
  * Concurrency & Aliasing Blueprint
  * Scaling Blueprint
  * Omni-Gate / App Plane Blueprints

---

## ✅ Perfection Gates Checklist (svc-admin)

* [ ] **Gate A:** Metrics green (`http_request_duration_seconds`, `svc_admin_*`).
* [ ] **Gate F:** No unapproved perf regressions (see PERFORMANCE.md).
* [ ] **Gate J:** Chaos drill for svc-admin + nodes passed in last 90 days.
* [ ] **Gate K:** Alerts wired for auth failures, upstream errors, stale metrics.
* [ ] **Gate L:** Scaling behavior validated under degraded node conditions.
* [ ] **Gate O:** Security audit clean (auth modes, logging, secrets).

```
```
