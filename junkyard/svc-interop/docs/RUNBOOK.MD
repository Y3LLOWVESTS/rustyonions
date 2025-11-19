
---

title: RUNBOOK — svc-interop
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-12
audience: operators, SRE, auditors

---

# 🛠️ RUNBOOK — svc-interop

## 0) Purpose

Operational manual for `svc-interop`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.
Satisfies **PERFECTION_GATES**: K (Continuous Vigilance), L (Black Swan Economics).

---

## 1) Overview

* **Name:** `svc-interop`
* **Role:** Interop bridge/adapters (REST/webhooks/SDKs/GraphQL) that translate external provider events/requests into internal capability-guarded actions. Bridges are **reversible** and default-deny.
* **Criticality Tier:** 2 (supporting). Read paths should remain green if bridges brown out.
* **Dependencies (runtime):**

  * `ron-auth`/`svc-passport` (macaroon verification & capability translation)
  * `ron-kms` (key material, signing, rotation hooks)
  * `svc-index`/`svc-storage` (object addressing & I/O)
  * `svc-mailbox` (async job/retry)
  * `ron-bus` (events), Prometheus (metrics scrape)
* **Ports Exposed (defaults):**

  * `bind_addr=0.0.0.0:8080` (HTTP ingress)
  * `metrics_addr=127.0.0.1:9909` (Prometheus text)
* **Data Flows:**

  1. HTTP ingress → provider verification (HMAC/ed25519 as applicable; SHA-256 allowed **edge-only**)
  2. Capability mint/validate → internal action (BLAKE3-256 addressing)
  3. Emit audit events and metrics
  4. Brownout degrades **writes first**; safe reads continue
* **Version Constraints:** Built against workspace pins (Axum 0.7.x, reqwest 0.12.x, tokio-rustls 0.26.x). Requires `ron-kernel` public API at current workspace tag.

---

## 2) Startup / Shutdown

### Startup

```bash
RUST_LOG=info \
SVC_INTEROP_BIND_ADDR=0.0.0.0:8080 \
SVC_INTEROP_METRICS_ADDR=127.0.0.1:9909 \
SVC_INTEROP_MAX_CONNS=512 \
SVC_INTEROP_MAX_RPS=500 \
SVC_INTEROP_BURST_RPS=250 \
./target/release/svc-interop --config /etc/ron/svc-interop.toml
```

**Config & env highlights**

* `limits.max_rps`, `limits.burst_rps`, `limits.body_limit=1MiB`
* `decompression.max_ratio` (protect zip-bombs)
* `brownout.thresholds`: `error_rate_5m=0.01`, `upstream_p95_ms=500`
* Provider secrets from env/secret mounts (never in logs)

**Verification**

```bash
curl -sf http://127.0.0.1:9909/readyz      # 200 when deps green & below thresholds
curl -sf http://127.0.0.1:9909/healthz     # 200 when process alive
```

### Shutdown

* SIGINT/SIGTERM triggers: stop listeners, drain inflight, flush audit (memory sink under amnesia), emit `Shutdown` bus event.
* systemd: `systemctl stop svc-interop`

---

## 3) Health, Readiness & SLOs

* **/healthz:** liveness probe only
* **/readyz:** readiness + brownout sentinel (deps connected; error-rate & upstream p95 below thresholds)
* **SLOs (from PERFORMANCE.md):**

  * Availability (monthly): ≥ 99.9% for read endpoints
  * Latency (intra-region): p95 ≤ 120 ms (steady state), p99 ≤ 300 ms
  * Error budget: ≤ 1% 5-minute rolling window (excludes client 4xx)

**If not ready after 10s**

* Check metrics: `rejected_total{reason}`, `latency_seconds{quantile="0.95"}`, `busy_rejections_total`
* Validate deps reachability and queue depths (see §5)

---

## 4) Common Failure Modes

| Symptom                               | Likely Cause                             | Metric/Log Hint                                    | Resolution                                                                               | Alert        |
| ------------------------------------- | ---------------------------------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------- | ------------ |
| 401/403 on bridge routes              | Missing/expired capability; policy deny  | `rejected_total{reason="POLICY_BLOCKED"}`          | Ensure capability mint/translate path OK; review policy/allow lists                      | any          |
| 400 `MALFORMED`                       | DTO parse/unknown fields                 | Structured error `{error, reason, corr_id}`        | Fix client; schemas use `deny_unknown_fields`                                            | any          |
| 413 `BODY_LIMIT` / 400 `DECOMP_LIMIT` | Oversized body / decompression ratio cap | `rejected_total{reason}`                           | Keep ≤ 1 MiB; stream large payloads (64 KiB chunks); verify provider limits              | spike        |
| 429 `RATE_LIMIT` / `BACKPRESSURE`     | RPS limiter or saturated queues          | `quota_exhaustions_total`, `busy_rejections_total` | Scale replicas; raise limits carefully; increase mpsc capacities; confirm CPU headroom   | >10/min      |
| 503 `DOWNSTREAM_UNAVAILABLE`          | index/storage/mailbox slow/unreachable   | `latency_seconds` ↑; error logs                    | Triage downstream; brownout protects writes; fail open for idempotent reads when allowed | any          |
| Invalid webhook signature             | Wrong secret/clock skew                  | audit `decision=deny reason=signature`             | Rotate secret; confirm HMAC base string & headers; fix time sync (±5 min window)         | any          |
| Audit drops                           | Audit ring buffer overflow               | `audit_drop_total`                                 | Increase audit mpsc; ensure amnesia (no file sink); slow producers if sustained          | >0 sustained |

**Stable reason codes → status**
`BODY_LIMIT`→413, `DECOMP_LIMIT`→400, `RATE_LIMIT`→429, `BACKPRESSURE`→429/503, `DOWNSTREAM_UNAVAILABLE`→503, `POLICY_BLOCKED`→403, `MALFORMED`→400.
All non-2xx return `{error, reason, corr_id}`.

---

## 5) Diagnostics

**Logs (structured JSON)**

```bash
journalctl -u svc-interop -f | jq -rc 'select(.corr_id!=null) | [.ts,.level,.reason,.provider,.corr_id] | @tsv'
```

**Metrics quick checks**

```bash
curl -s http://127.0.0.1:9909/metrics | grep -E 'svc_interop_(requests_total|latency_seconds|rejected_total|busy_rejections_total|audit_drop_total|inflight)'
```

**Bus tail**

```bash
ronctl tail --topic svc-interop --since 15m
```

**Perf**

```bash
cargo flamegraph --bin svc-interop -- --config ./configs/svc-interop.toml
TOKIO_CONSOLE_BIND=127.0.0.1:6669 RUSTFLAGS="-Zinstrument-mcount" cargo run -p svc-interop
```

**Smoke tests (post-deploy)**

```bash
# Liveness/Readiness
curl -sf http://127.0.0.1:9909/healthz
curl -sf http://127.0.0.1:9909/readyz
# Signed webhook sample (provider fixture)
curl -sS -X POST http://127.0.0.1:8080/webhooks/providerX \
  -H "X-Signature: <fixture>" -d @tests/fixtures/providerX_event.json
```

---

## 6) Recovery Procedures

1. **Bridge Brownout Triggered**

* Confirm upstream error/p95 spikes.
* Action: scale replicas; temporarily lower `max_rps` & `burst_rps`; when steady, restore.
* Verify `/readyz` returns 200 within 2–5 min.

2. **Invalid Signatures / Clock Skew**

* Check NTP skew (target ≤ ±30s, budget ≤ ±5m).
* Rotate provider secret; deploy with **dual secret window** (old+new accepted for overlap period).

3. **Policy Blocked**

* Ensure SHA-256 appears **only at edge**; internal hashing remains BLAKE3-256.
* Re-run payload through adapter to drop foreign hashes before persistence.

4. **Queue Saturation**

* Inspect queue depths; increase mpsc capacity; add workers; prefer `try_send` with typed Busy.
* Horizontal scale (stateless) if CPU < 70% yet backpressure persists.

5. **Audit Drops**

* Raise audit queue size; confirm amnesia mode (no file sink); if sustained, apply producer throttling.

6. **Config Drift**

* Validate with `ronctl config check`; hot-reload if supported or restart service.
* Keep a **known-good** config snapshot for rollback.

---

## 7) Backup / Restore

* **Stateless.** Back up only configuration and dashboards.
* Secrets live in external secret manager; rotate per schedule and after incidents.

---

## 8) Upgrades & Rollbacks

**Blue/Green (preferred)**

1. Bring up `svc-interop@new` behind the gateway, no traffic.
2. Run smoke tests.
3. Shift traffic gradually (10% → 50% → 100%).
4. Watch `rejected_total`, `latency_seconds` p95, `audit_drop_total` for ≥10 minutes.
5. Decommission old.

**Rollback**

* Keep N-1 binary and config; `systemctl start svc-interop@n-1` and reroute.
* If auth/hash changes were deployed, maintain **dual-algo**/dual-secret accept for the overlap window.

---

## 9) Chaos & GameDays

* Faults to inject quarterly:

  * +300 ms upstream latency (index/storage), 5% error bursts
  * Queue saturation (reduce capacities), burst RPS 10×
  * Clock skew ±10 min on one node
* Expected: write brownout without read outage; alerts fire; recovery ≤ 5 min after fault removed.

---

## 10) Scaling Notes

* **Stateless:** scale horizontally behind gateway.
* **Knobs:** `max_conns`, `limits.{max_rps,burst_rps}`, queue sizes, timeouts (connect/read/write).
* **When to scale:** `busy_rejections_total` > 0 with CPU < 70% or p95 drifting > 120 ms for 10 min.

**Reference throughput (guide, 4c/8GB, intra-region)**

* ~500 rps steady with p95 ~80–120 ms under typical provider verification.

---

## 11) Security Ops

* **Zero-trust:** short-TTL macaroons; revocation latency ≤ 1s.
* **Hashing policy:** BLAKE3-256 internal; SHA-256 **edge-only** for transient verification; never persist.
* **Amnesia:** RAM-only caches; no file sinks; timed purge for secrets/keys.
* **Provider signatures:** Validate headers & base strings; replay window ≤ 5m.
* **Rotation playbooks:** Dual secret acceptance; record KID/version in audit; cutover after 2× TTL.

---

## 12) Alerting & Dashboards

**Prometheus rules (suggested)**

```yaml
groups:
- name: svc-interop
  rules:
  - alert: InteropHighErrorRate
    expr: sum(rate(svc_interop_rejected_total{reason!="MALFORMED"}[5m])) 
          / sum(rate(svc_interop_requests_total[5m])) > 0.01
    for: 10m
    labels: {severity: page}
    annotations: {summary: "svc-interop high error rate (>1%/5m)"}

  - alert: InteropHighLatencyP95
    expr: histogram_quantile(0.95, rate(svc_interop_latency_seconds_bucket[5m])) > 0.12
    for: 10m
    labels: {severity: ticket}
    annotations: {summary: "svc-interop p95 latency > 120ms"}

  - alert: InteropBackpressure
    expr: rate(svc_interop_busy_rejections_total[5m]) > 1
    for: 5m
    labels: {severity: ticket}
    annotations: {summary: "svc-interop backpressure 429s rising"}
```

**Dashboards (golden panels)**

* Requests & rejects by reason
* Latency (p50/p95/p99)
* Inflight & queue depths
* Audit drops
* Brownout state (% time degraded)

---

## 13) Deployment Manifests

**systemd unit (example)**

```ini
[Unit]
Description=svc-interop
Wants=network-online.target
After=network-online.target

[Service]
User=ron
ExecStart=/opt/ron/bin/svc-interop --config /etc/ron/svc-interop.toml
Environment=RUST_LOG=info
Environment=SVC_INTEROP_METRICS_ADDR=127.0.0.1:9909
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
```

**Kubernetes (minimal example)**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata: { name: svc-interop }
spec:
  replicas: 2
  selector: { matchLabels: { app: svc-interop } }
  template:
    metadata: { labels: { app: svc-interop } }
    spec:
      containers:
      - name: svc-interop
        image: ghcr.io/rustyonions/svc-interop:{{TAG}}
        args: ["--config","/etc/ron/svc-interop.toml"]
        ports: [{containerPort: 8080}, {containerPort: 9909, name: metrics}]
        env:
        - name: RUST_LOG
          value: info
        readinessProbe:
          httpGet: { path: /readyz, port: 9909 }
          periodSeconds: 5
        livenessProbe:
          httpGet: { path: /healthz, port: 9909 }
          periodSeconds: 10
        resources:
          requests: { cpu: "250m", memory: "256Mi" }
          limits:   { cpu: "1000m", memory: "512Mi" }
---
apiVersion: v1
kind: Service
metadata: { name: svc-interop }
spec:
  selector: { app: svc-interop }
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: metrics
    port: 9909
    targetPort: 9909
```

---

## 14) References

* CONFIG.md — knobs & thresholds
* OBSERVABILITY.md — metrics taxonomy & dashboards
* CONCURRENCY.md — queues, backpressure, invariants
* SECURITY.md — hashing, capabilities, amnesia, rotations
* INTEROP.md — provider surface & reversible bridges
* PERFORMANCE.md — SLOs & bench baselines

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Metrics green (`latency_seconds`, `requests_total`, rejects flat)
* [ ] Gate J: Chaos drill passed (quarterly)
* [ ] Gate K: Alerts wired; dashboards current; on-call playbook linked
* [ ] Gate N: ARM/edge profiled; blake3-simd validated where available
* [ ] Gate O: Security audit clean (hash/signature/policy posture)

---

### Appendix A — Dependency Cascade

* **svc-index/storage**: write brownout; reads may serve from cache if policy allows
* **svc-passport/ron-auth**: all capability checks fail-closed → 401/403
* **ron-kms**: rotations delayed; keep last good KID; enable dual verify window
* **svc-mailbox**: async jobs delayed; retries resume on recovery

### Appendix B — DTO & Reason Code Stability

* Error envelope `{error, reason, corr_id}` is stable; reason code table above is normative.

---

