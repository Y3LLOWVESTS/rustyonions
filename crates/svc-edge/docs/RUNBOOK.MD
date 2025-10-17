

---

# 🛠️ RUNBOOK — svc-edge

---

title: RUNBOOK — svc-edge
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-17
audience: operators, SRE, auditors
----------------------------------

## 0) Purpose

Operational manual for `svc-edge`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `svc-edge`
* **Role:** Stateless edge asset server for static/ranged bytes with optional **live fill** from allow-listed HTTPS origins into a content-addressed store (CAS). Strong ETag semantics, byte-range reads, observability endpoints.
* **Criticality Tier:** 2 (supporting ingress/edge plane; sheds early under pressure by design).
* **Dependencies:** Static packs/CAS for hot assets; outbound HTTPS client for live fill; Prometheus scrape target.
* **Ports Exposed (typical):**

  * `bind_addr` (serves API, e.g., `0.0.0.0:8080`)
  * `metrics_addr` (Prometheus, e.g., `127.0.0.1:9909`)
* **Data Flows:** HTTP GET → cache/CAS hit (ETag + Range) **or** live fill (allow-list → fetch → hash/verify → optional CAS write) → bytes out.
* **Version Constraints:** HTTP contract & headers are semver-governed; ETag/Range behavior is normative.

---

## 2) Startup / Shutdown

### Startup (bare metal / dev)

```bash
SVC_EDGE_BIND_ADDR=0.0.0.0:8080 \
SVC_EDGE_METRICS_ADDR=127.0.0.1:9909 \
SVC_EDGE_EDGE__MODE=offline \
SVC_EDGE_EDGE__PACKS=./data/world.pmtiles \
SVC_EDGE_SECURITY__AMNESIA=true \
RUST_LOG=info \
cargo run -p svc-edge
```

### Startup (binary + config file)

```bash
./target/release/svc-edge --config ./configs/svc-edge.toml
```

### Startup (docker)

```bash
docker run --rm -p 8080:8080 -p 9909:9909 \
  -v "$PWD/configs/svc-edge.toml:/etc/ron/svc-edge.toml:ro" \
  -v "$PWD/data:/data:ro" \
  -e RUST_LOG=info \
  --name svc-edge ron/svc-edge:latest \
  --config /etc/ron/svc-edge.toml
```

### systemd unit (example)

```
[Unit]
Description=svc-edge
After=network.target

[Service]
ExecStart=/usr/local/bin/svc-edge --config /etc/ron/svc-edge.toml
Environment=RUST_LOG=info
Restart=always
RestartSec=2
LimitNOFILE=1048576
AmbientCapabilities=CAP_NET_BIND_SERVICE
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

**Verification**

* Logs show mode (offline/live), bind addresses, and `ready=1`.
* `curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9909/readyz` → `200`

### Shutdown

* SIGINT/SIGTERM or `systemctl stop svc-edge` → listeners close, workers drain until deadline; process exits non-zero on fatal init errors.

---

## 3) Health & Readiness

* **/healthz**: process liveness.
* **/readyz**: serving readiness (packs verified, CAS reachable, allow-list parsed, inflight/queue below thresholds, amnesia constraints satisfied).
* **Expectation:** ready within 2–5s on warm start.

**If not ready after 10s**

1. Check metrics: `rejected_total{reason}`, `edge_queue_depth`, `io_timeouts_total`.
2. Inspect logs for init errors.
3. Validate config paths & allow-list, then restart.

**Kubernetes probes (example)**

```yaml
livenessProbe:
  httpGet: { path: /healthz, port: 8080 }
  initialDelaySeconds: 3
  periodSeconds: 5
readinessProbe:
  httpGet: { path: /readyz, port: 8080 }
  initialDelaySeconds: 3
  periodSeconds: 5
  failureThreshold: 6
```

---

## 4) Common Failure Modes

| Symptom                            | Likely Cause                                  | Metric / Log                                     | Resolution                                                                              | Alert                                                                          |                                                                                       |               |
| ---------------------------------- | --------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | ------------- |
| 429 Too Many Requests              | Inflight/RPS caps; queue full                 | `rejected_total{reason="rate_limit               | busy"}`; queue hist                                                                     | Raise capacity or replicas; confirm bounded queue; verify CDN/gateway in front | >1% 15m                                                                               |               |
| 503 Service Unavailable            | I/O timeout to CAS/packs or live fill         | `io_timeouts_total{op="fill                      | read                                                                                    | connect"}`                                                                     | Validate storage reachability; increase `timeout_secs`; apply limited retries/hedging | any sustained |
| 416 Range Not Satisfiable          | Invalid/unsatisfiable `Range` header          | `edge_range_requests_total{result="416"}`        | Client bug; confirm header parse; provide sample header in docs                         | n/a                                                                            |                                                                                       |               |
| High cache miss on hot paths       | Misconfigured pack path / cold CAS            | `edge_cache_{hits,misses}_total` ratio           | Fix `edge.packs`, pre-warm frequently requested assets, verify ETag stability           | trend ↑                                                                        |                                                                                       |               |
| Live fill frequently failing       | Origin not allow-listed or upstream 4xx       | `edge_live_fill_{attempts,success}_total` + logs | Add domain to allow-list; check upstream; tune bounded retry/jitter                     | success <99%/h                                                                 |                                                                                       |               |
| Ready but slow (local p95 > 40ms)  | Saturation / disk contention / CPU throttling | `request_latency_seconds`, `edge_inflight`       | Add replicas; raise concurrency prudently; profile I/O; confirm CPU governor/perf state | p95>40ms 10m                                                                   |                                                                                       |               |
| Readiness flaps under amnesia mode | Operation would persist state                 | `rejected_total{reason="amnesia"}`               | Disable live-write features, run offline profile, or relocate to non-amnesia node       | any flap                                                                       |                                                                                       |               |

---

## 5) Diagnostics

**Logs (JSON)**

* Core fields: `ts, level, service, event, route, method, status, reason, corr_id, peer_addr, latency_ms, bytes_out, etag/hash, mode`.
* Patterns to watch: `reason=rate_limit|busy|amnesia|timeout|upstream_4xx|upstream_5xx`.

**Quick commands**

```bash
curl -s http://127.0.0.1:9909/metrics
curl -s http://127.0.0.1:9909/metrics | rg 'rejected_total|edge_queue_(depth|dropped)|busy_rejections_total'
journalctl -u svc-edge -f | jq -r '.ts+" "+.level+" "+.event+" "+(.reason//"")+" "+(.latency_ms|tostring)+"ms"'
docker logs -f svc-edge | jq '.'
```

**Tracing**

* Spans: `svc.edge.http|cache|fill|range` (fields: `corr_id`, `range`, `host`, `attempt`, `reason`).
* Optional OTEL export: set `OTEL_EXPORTER_OTLP_ENDPOINT` and enable tracing feature flags.

**Perf debug**

```bash
RUST_LOG=info cargo flamegraph -p svc-edge
```

---

## 6) Recovery Procedures

1. **Config Drift / Bad Overrides**
   *Symptom:* rejects or unexpected caps.
   *Action:* Validate TOML; check `[ingress]` (`timeout_secs`, `max_inflight`, `rps_limit`, `max_body_bytes`, `max_decompress_ratio`). Reload (SIGHUP) or restart.

2. **Packs Not Found / Not Verified**
   *Symptom:* cache misses surge; readiness degrades.
   *Action:* Fix `edge.packs` paths; verify file permissions; restart; confirm ETag/Range on a sample asset.

3. **Live Fill Failing**
   *Symptom:* `edge_live_fill_success_total` declines; 403 from origin.
   *Action:* Add origin to `edge.allow`; confirm TLS reachability; inspect retries; temporarily disable live fill to restore service.

4. **Amnesia Mode Violation**
   *Symptom:* `rejected_total{reason="amnesia"}`; `/readyz` flaps.
   *Action:* Turn off features that persist; run in offline profile; re-test readiness.

5. **Overload / Shedding**
   *Symptom:* spike in 429; `busy_rejections_total` increases.
   *Action:* Add replicas; ensure CDN or svc-gateway in front; consider per-tenant RPS shaping.

6. **Post-Recovery Verification (for all above)**
   Hold steady state ≥ 5m with: p95 latency within baseline, 5xx < 0.1%, rejects < 1% (15m), `/readyz` stable.

---

## 7) Backup / Restore

* `svc-edge` holds **no durable state** by default.
* Backups apply to CAS/storage/index services hosting packs/blobs.
* Integrity checks: verify BLAKE3/ETag on sampled assets after restores.

---

## 8) Upgrades

**Blue/Green or Rolling (preferred)**

* Drain instance (remove from LB or set `/readyz` to fail-closed via flag).
* Deploy new version to a subset (20–30%), observe 10m:

  * p95 ≤ baseline + 10%
  * 5xx ≤ 0.1%
  * rejects ≤ 1%
* Roll remainder if green.

**Canary + Auto-Rollback**

* Route 1–5% traffic to canary; monitor the same SLOs.
* **Rollback trigger:** any metric breaching above thresholds for > 5m (or readiness fails).
* Keep last 2 known-good images/tags available for instant revert.

**One-liner rollback (docker example)**

```bash
docker service update --image ron/svc-edge:<LAST_GOOD> svc-edge
```

---

## 9) Chaos Testing

**Success Criteria (must all hold within 3m of fault clear)**

* `/readyz` returns 200 steadily.
* p95 latency returns to baseline ±10%.
* `rejected_total{reason}` back to pre-fault rates.
* No error burst > 0.1% 5xx over 10m window.

**Tiny chaos harness (latency injection via tc/netem, Linux)**

```bash
sudo tc qdisc add dev eth0 root netem delay 80ms 20ms distribution normal
sleep 120
sudo tc qdisc del dev eth0 root netem
```

Run quarterly and record before/after metrics snapshots.

---

## 10) Scaling Notes

**Vertical knobs**

* `max_inflight` (concurrency), `rps_limit` (shaping), `timeout_secs` (tail protection). Keep queues **bounded**.

**Horizontal**

* Stateless design ⇒ scale replicas; front with CDN/gateway; share read-only packs/CAS.

**Autoscaling signals (K8s HPA examples)**

* p95 latency target (derived from `request_latency_seconds`).
* `edge_inflight` ≥ 80% of configured cap for 3m.
* `rejected_total` growth (rate) over 1%/15m.

**SLO reference**

* Local warm-hit: p95 < 40ms, p99 < 80ms
* 5xx < 0.1% (rolling 10m)
* Shedding (429/rejects) < 1% (15m)

---

## 11) Security Ops

* **Default stance:** deny-by-default CORS; sanitize/redact error bodies; never log secrets.
* **TLS:** rustls for local termination where applicable; prefer TLS upstreams for live fill.
* **Live Fill:** enforce hostname allow-list; optionally audit `{origin_host, hash, size}` on fills.
* **Abuse detection:** watch `tls_handshake_failures_total`, `io_timeouts_total`, `rejected_total{reason}`.
* **Rotation:** rotate allow-list and any caps via ops change control; keep diffs auditable.

---

## 12) Alert → Action Runbooks

### A) EdgeHighErrorRate

* **Trigger:** 5xx > 0.1% for 10m.
* **Triage:** requests by status, storage timeouts, recent deploy diff.
* **Action:** Fix storage/upstream; widen timeouts modestly; rollback if deploy-related.

### B) EdgeExcessiveShedding

* **Trigger:** `rejected_total{reason=rate_limit|busy|draining}` / requests > 1% for 15m.
* **Triage:** `edge_inflight`, `edge_queue_depth`, node CPU/IO.
* **Action:** Add replicas; tune caps; confirm CDN/gateway cache hit rate.

### C) EdgeNotReady

* **Trigger:** `/readyz` failing > 5m.
* **Triage:** Packs path/permissions, CAS reachability, amnesia violations.
* **Action:** Correct config; restart; verify steady state ≥ 5m.

---

## 13) ARM64 / Edge Hardware Notes

* ARM often has different perf profile: higher perf/Watt but slower single-thread peak.
* Recommendations:

  * Prefer `edge.packs` on fast local SSD/NVMe; avoid network FS for hot paths.
  * Keep TLS ciphers modern but efficient; consider terminating at gateway/CDN.
  * Validate governor: set `performance` for latency-critical nodes.
  * Benchmark with realistic ranges (16–256 KiB) and many small files (stress syscall/IO path).

---

## 14) References

* CONFIG.md — knobs & defaults
* OBSERVABILITY.md — metrics/logs/tracing/alerts
* SECURITY.md — STRIDE, hardening defaults
* INTEROP.md — HTTP contract, ETag/Range examples
* CONCURRENCY.md — bounded queues/backpressure
* PERFORMANCE.md — SLOs & perf harness

---

## ✅ Perfection Gates Checklist

* [ ] Gate A: Metrics green (latency histogram present; counters rising; no scrape gaps)
* [ ] Gate J: Chaos drill passed (criteria in §9 met; artifacts attached)
* [ ] Gate K: Continuous vigilance (alerts wired; corr_id present in logs; dashboards linked)
* [ ] Gate N: ARM/edge perf profiled (p95 ≤ 40ms warm-hit; perf/Watt noted)
* [ ] Gate O: Security audit clean (deny-by-default CORS; secret-safe logs; strong ETag/Range invariants)

---

# Appendices

## Appendix A — Request Flow (Mermaid)

```mermaid
flowchart LR
  C[Client] -->|GET /edge/assets/{path}| E(svc-edge)
  subgraph HotPath
    P[(Packs/CAS)]
  end
  E -->|ETag match & Range OK| P
  E -->|miss or stale| L[Live Fill: HTTPS (allow-listed)]
  L -->|200 OK| V[Verify hash/ETag]
  V -->|OK| P
  P -->|bytes + Accept-Ranges| E
  E -->|200/206 + strong ETag| C
  E -.->|/metrics| M[(Prometheus)]
  E -.->|/healthz /readyz| O[(Probes)]
```

---

## Appendix B — Kubernetes Deployment (paste-ready)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: svc-edge
  labels: { app: svc-edge }
spec:
  replicas: 2
  strategy:
    type: RollingUpdate
    rollingUpdate: { maxUnavailable: 0, maxSurge: 1 }
  selector:
    matchLabels: { app: svc-edge }
  template:
    metadata:
      labels: { app: svc-edge }
    spec:
      containers:
        - name: svc-edge
          image: ron/svc-edge@sha256:PUT_REAL_DIGEST_HERE
          args: ["--config", "/etc/ron/svc-edge.toml"]
          ports:
            - name: http
              containerPort: 8080
            - name: metrics
              containerPort: 9909
          env:
            - name: RUST_LOG
              value: info
            - name: SVC_EDGE_BIND_ADDR
              value: "0.0.0.0:8080"
            - name: SVC_EDGE_METRICS_ADDR
              value: "0.0.0.0:9909"
          volumeMounts:
            - name: cfg
              mountPath: /etc/ron
              readOnly: true
            - name: packs
              mountPath: /data
              readOnly: true
          readinessProbe:
            httpGet: { path: /readyz, port: http }
            initialDelaySeconds: 3
            periodSeconds: 5
            failureThreshold: 6
          livenessProbe:
            httpGet: { path: /healthz, port: http }
            initialDelaySeconds: 3
            periodSeconds: 5
          resources:
            requests: { cpu: "250m", memory: "256Mi" }
            limits: { cpu: "1000m", memory: "512Mi" }
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            runAsNonRoot: true
            runAsUser: 10001
            capabilities: { drop: ["ALL"] }
      volumes:
        - name: cfg
          configMap: { name: svc-edge-config }
        - name: packs
          persistentVolumeClaim: { claimName: svc-edge-packs-pvc }
---
apiVersion: v1
kind: Service
metadata:
  name: svc-edge
spec:
  selector: { app: svc-edge }
  ports:
    - name: http
      port: 80
      targetPort: http
    - name: metrics
      port: 9909
      targetPort: metrics
```

Notes:

* **Digest pinning:** use `image@sha256:<digest>` to align with hardening.
* **Env precedence:** environment variables override TOML—reflect this in CONFIG.md.

**Optional HPA**

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: svc-edge
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: svc-edge
  minReplicas: 2
  maxReplicas: 12
  metrics:
    - type: Pods
      pods:
        metric:
          name: edge_inflight
        target:
          type: AverageValue
          averageValue: "80"
```

---

## Appendix C — PromQL / Dashboards (paste-ready)

**Golden Signals**

```
sum by (status) (rate(http_requests_total[5m]))
```

```
histogram_quantile(0.95, sum by (le) (rate(request_latency_seconds_bucket[5m])))
```

```
sum(rate(rejected_total[5m]))
sum by (reason) (rate(rejected_total[5m]))
```

```
sum(rate(edge_live_fill_success_total[5m])) 
/ clamp_min(sum(rate(edge_live_fill_attempts_total[5m])), 1)
```

```
avg(edge_queue_depth)
sum(rate(edge_queue_dropped_total[5m]))
```

```
sum by (op) (rate(io_timeouts_total[5m]))
```

**Suggested Grafana panels**

1. Requests/sec by status (stacked)
2. Latency p50/p95/p99
3. Rejects by reason
4. Live fill success ratio
5. Queue depth & drops
6. I/O timeouts by operation
7. Readiness pass rate (probe success)

---

## Appendix D — Cross-Platform Chaos

**Linux (latency via netem)**

```
sudo tc qdisc add dev eth0 root netem delay 80ms 20ms distribution normal
sleep 120
sudo tc qdisc del dev eth0 root netem
```

**Portable (Toxiproxy latency/error injection)**

```
docker run -d --name toxiproxy -p 8474:8474 -p 8666:8666 shopify/toxiproxy
```

Create a proxy mapping a local port to your upstream and add latency/timeout toxics via the Toxiproxy REST API or CLI, then point `svc-edge` live-fill to the proxy.

---

## Appendix E — ARM64 Notes (extra-practical)

**Governor**

```
sudo cpupower frequency-set -g performance
```

**Build & test hints**

* Benchmark realistic byte-ranges (16–256 KiB) and many small files.
* Prefer local NVMe/SSD for packs; avoid network FS for hot paths.
* For perf on ARM:

```
perf record -F 99 -g -- <svc-edge-binary-and-args>
perf report
```

---

## Appendix F — Rollback Patterns (generalized)

**Kubernetes**

```
kubectl rollout undo deploy/svc-edge
kubectl rollout status deploy/svc-edge
```

**Docker (compose/swarm)**

```
docker service update --image ron/svc-edge:<LAST_GOOD> svc-edge
```

**Success/abort guardrails**

* Abort if: p95 > baseline + 10% for 5m, or 5xx > 0.1% for 5m, or `/readyz` fails > 2 consecutive probes.
* Hold steady ≥ 10m after recovery before declaring green.

---

## Appendix G — systemd Hardening (tighten a bit more)

Add to your unit:

```
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ProtectKernelTunables=true
ProtectControlGroups=true
MemoryDenyWriteExecute=true
RestrictSUIDSGID=true
LockPersonality=true
SystemCallFilter=@system-service
```

---
