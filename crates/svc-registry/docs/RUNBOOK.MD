

---

````markdown
---
title: RUNBOOK — svc-registry
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-08
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — svc-registry

## 0) Purpose
Operational manual for `svc-registry`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
Satisfies **PERFECTION_GATES**: **K** (Continuous Vigilance), **L** (Black Swan Economics).

---

## 1) Overview

- **Name:** `svc-registry`
- **Role:** Authoritative, append-only registry of signed node/service/region descriptors (quorum-gated writes, fast read plane).
- **Criticality Tier:** **1 (critical service)** — kernel is tier 0; registry is the control-plane source of truth.
- **Dependencies:**
  - **Internal:** `ron-bus` (broadcast/notifications), storage backend (sled/sqlite), system clock (monotonic).
  - **External (logical):** signer set (quorum of approvals), metrics collector (Prometheus), log sink.
- **Ports Exposed:**
  - HTTP API: `REGISTRY_ADDR` (e.g., `0.0.0.0:9444`)
  - Metrics/health: `METRICS_ADDR` (e.g., `0.0.0.0:9909`)
  - (Optional) UDS for local bus: `RON_BUS_SOCK` (e.g., `/run/ron/bus.sock`)
- **Data Flows:**
  - Ingress: HTTP(S) reads (`/registry/head`, `/registry/{version}`), SSE stream (`/registry/stream`), write path (`/propose`, `/approve`, `/commit`).
  - Egress: Non-blocking bus events (commit notifications), metrics, structured logs.
- **Version Constraints:** Requires `ron-kernel` (workspace pin); `tokio`/`axum` pins; `ed25519-dalek v2`.

---

## 2) Startup / Shutdown

### Startup

```bash
./svc-registry --config /etc/ron/svc-registry.toml
# dev
cargo run -p svc-registry -- --config ./configs/svc-registry.toml
````

**Key env vars (override config):**

* `RON_CONFIG=/etc/ron/svc-registry.toml`
* `REGISTRY_ADDR=0.0.0.0:9444`
* `METRICS_ADDR=0.0.0.0:9909`
* `RON_BUS_SOCK=/run/ron/bus.sock`
* `RUST_LOG=info,svc_registry=info`

**Feature toggles / flags (if built):**

* `--amnesia` (Micronode; tmpfs, no disk persistence)
* `--uds-only` (bind UDS only; no TCP)
* `--read-only` (maintenance; write plane sheds)

**Verification after start:**

* Logs contain `ready=1 service=svc-registry role=read-plane`.
* `curl -fsS http://127.0.0.1:9909/readyz` → `200 OK`.
* `curl -fsS http://127.0.0.1:9444/registry/head` → JSON `{version: "...", ...}`.

### Shutdown

* SIGINT / SIGTERM → graceful drain; emits bus `KernelEvent::Shutdown`.
* systemd: `systemctl stop svc-registry` (watch for `shutdown_complete=1`).

---

## 3) Health & Readiness

* **/healthz**: process liveness.
* **/readyz** (true when): HTTP bound; storage mounted / checkpoint restored; bus connected; metrics alive; (if write-enabled) signer set hydrated & commit critical section ready.

**Expected timings:**

* Read plane ready **< 800 ms** after start.
* Full write readiness **< 2.5 s** (checkpoint restore **< 3.5 s** after crash/upgrade).

**If not ready after 10s:**

* Check bus `ServiceCrashed{service,reason}` events.
* Metrics: `registry_checkpoint_age_seconds`, `bus_lagged_total`, `rejected_total{reason="busy|deps"}`.

---

## 4) Common Failure Modes

| Symptom                               | Likely Cause                                        | Metric / Log Hint                                                        | Resolution                                                                              | Alert Threshold                      |
| ------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- | ------------------------------------ |
| `503` on write endpoints              | Write plane shed (maintenance / deps not ready)     | `rejected_total{reason="deps"}` ↑, `/readyz` false for write-plane       | Verify signer set, storage, bus; clear maintenance; confirm `/readyz`; re-enable writes | `/readyz` write=false > 5m           |
| `429` spike on reads                  | Concurrency cap reached (traffic surge)             | `rejected_total{reason="busy"}`, inflight gauge high                     | Scale replicas; cautiously raise concurrency; push upstream backpressure                | >1% of requests for >10m             |
| Slow reads (p95 > 100ms intra-region) | Storage/GC pause; checkpointing                     | `registry_checkpoint_age_seconds` ↑, flamegraph shows fsync              | Move checkpoint window off-peak; tune interval; faster disk/tmpfs                       | p95 breach >15m                      |
| Commit latency > 750ms p95            | Signature verify or fsync bound                     | `commit_latency_seconds_bucket`; per-approval verify timer               | Add CPU; parallelize approvals; batch fsyncs (guardrails); tune checkpoint window       | p95 > 750ms for >5m                  |
| SSE clients miss events               | Bus backpressure / slow consumers (lossy by design) | `bus_lagged_total` ↑, client gaps                                        | Expected; clients reconcile via `/registry/head`. Tune bus buffer if chronic.           | Any sustained rise without reconcile |
| 5xx increase                          | Panic/bug, dep outage                               | Logs `level=error`, `ServiceCrashed`; `http_requests_total{code=~"5.."}` | Roll back/hotfix; triage; see §6 Recovery                                               | >0.1% 5xx for 5m                     |
| Stuck not-ready after restart         | Corrupt checkpoint, bad config                      | `readyz=false`, “checkpoint restore failed”                              | Restore last good checkpoint; validate config; restart                                  | any                                  |

---

## 5) Diagnostics

**Logs**

```bash
journalctl -u svc-registry -f
journalctl -u svc-registry -f | grep -E 'corr_id=|level=error|ready='
```

**Metrics (Prometheus text)**

```bash
curl -fsS http://127.0.0.1:9909/metrics | grep -E 'request_latency_seconds|rejected_total|bus_lagged_total|registry_checkpoint_age_seconds'
```

**Quick PromQL**

```promql
# p95 read (/registry/head), last 5m
histogram_quantile(0.95,
  sum by (le) (rate(request_latency_seconds_bucket{route="/registry/head"}[5m]))
)

# commit p95
histogram_quantile(0.95, sum by (le) (rate(commit_latency_seconds_bucket[5m])))

# error rate
sum(rate(http_requests_total{code=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))

# bus drops
rate(bus_lagged_total[5m])

# checkpoint staleness
max_over_time(registry_checkpoint_age_seconds[5m])
```

**Bus events**

```bash
ronctl tail --topic registry
```

**Tracing / Profiling**

```bash
RUST_LOG=debug ./svc-registry --config /etc/ron/svc-registry.toml
cargo flamegraph -p svc-registry -- --config ./configs/svc-registry.toml
# dev only
TOKIO_CONSOLE_BIND=127.0.0.1:6669 cargo run -p svc-registry -- --config ./configs/svc-registry.toml
```

---

## 6) Recovery Procedures

1. **Write-plane shedding (deps)**

   * Verify bus connectivity & signer hydration. Exit maintenance. Confirm `/readyz`.

2. **Config Drift**

   * `ronctl config print svc-registry` → diff vs git; fix; `SIGHUP` or restart.

3. **Checkpoint Corruption / Storage Fault**

   * Stop → restore last good checkpoint dir → start → ensure `registry_checkpoint_age_seconds` drops.

4. **Commit Latency Regression**

   * Capture flamegraph; check per-approval verify time; increase CPU / approvals concurrency; evaluate fsync batching; tune checkpoint window.

5. **High 5xx**

   * Roll back to last good; attach logs/graphs to incident; open hotfix PR.

6. **Blue/Green or Rollback**

   * Start new alongside old; require `/readyz=true` for ≥10m; flip; monitor p95 reads/commits, 5xx, bus drops; retire old.

7. **Micronode anomalies (amnesia)**

   * Confirm `--amnesia` + tmpfs; expect lower fsync costs, higher CPU sensitivity on verify.

---

## 7) Backup / Restore

* **State:** append-only log + periodic checkpoints.
* **Macronode:**

  * Backup every **15m** (hot copy of checkpoint dir; rotate daily).
  * Restore: stop → replace checkpoint dir → start → verify `/readyz` & `/registry/head`.
* **Micronode (`--amnesia`)**: no persistent backup; re-sync from upstream authoritative nodes.

---

## 8) Upgrades

* Pre-check: dashboards green; error budget OK.
* Plan: **blue/green** or **rolling**; single-writer semantics per instance.
* Steps:

  1. Bring up new; keep old until new `/readyz` true for 10m.
  2. Flip traffic; monitor SLOs.
  3. Remove old.
* Migrations: none expected (append-only); keep `deny_unknown_fields` on.

---

## 9) Chaos Testing

```bash
ronctl chaos inject --target svc-registry --fault=storage-latency --ms=50
```

* Validate: read SLOs hold, write sheds gracefully, alerts fire, runbook steps resolve.
* Quarterly drill: lossy bus + reconcile, storage jitter, write-plane outage.

---

## 10) Scaling Notes

* **Vertical**: verify-bound → add CPU; fsync-bound → faster disk/tmpfs.
* **Horizontal (reads)**: scale replicas; stateless read plane; shared bus.
* **Writes**: per-instance single logical writer (serialized HEAD advance).
* **Scale triggers**:

  * Inflight > 80% cap for 10m,
  * p95 read > 50 ms intra-region for 15m,
  * commit p95 > 750 ms for 5m at normal cadence.
* **Reference capacity (baseline @ 4c/8GiB)**:

  * Reads: **~2,500 RPS/node** at p95 budget.
  * Commits: **1–5 commits/s** sustained with M-of-N approvals.

---

## 11) Security Ops

* Redact secrets in logs; enable structured logging with redaction.
* **Signer set rotation**: stage keys; overlapping validity; monitor verify timings post-rotation.
* **Capability rotation**: `ronctl cap rotate`.
* **PQ posture (forward-looking)**: if hybrid/PQ signatures piloted, track verify-cost metrics; pre-provision CPU.
* **Audit**: all commits emit audit events; cross-check with `ron-audit`.

---

## 12) References

* `CONFIG.md`, `SECURITY.md`, `OBSERVABILITY.md`, `CONCURRENCY.md`, `PERFORMANCE.md`, `TESTS.md`
* Blueprints: Hardening, Concurrency & Aliasing, Scaling, Omnigate
* Incident template: `/docs/ops/INCIDENT_TEMPLATE.md`
* Dashboards: `/ops/grafana/svc-registry.json`

---

## ✅ Perfection Gates Checklist

* [ ] **A** Metrics green: `request_latency_seconds`, `http_requests_total`, `rejected_total`, `bus_lagged_total`
* [ ] **F** Perf gates enforced in CI (no silent regressions)
* [ ] **J** Chaos drill passed (quarterly)
* [ ] **K** Continuous vigilance: alerts wired, runbook validated
* [ ] **L** Black swan validated: lossy bus + reconcile OK
* [ ] **N** ARM/edge profiled (Micronode & Macronode)
* [ ] **O** Security audit clean; signer rotation test done

---

# 13) Operational Appendices

## 13.1 Mermaid Quick-Ref (Data Flow & Readiness Gates)

```mermaid
flowchart TD
  A[Client / SDK] -->|HTTP: /registry/head| B[svc-registry\nRead Plane]
  A -->|HTTP: /registry/{version}| B
  A -.->|SSE: /registry/stream| B
  subgraph Commit Path (Write Plane)
    C[Propose] --> D[Approve (M-of-N)]
    D --> E[Commit (CAS / HEAD advance)]
  end
  E -->|Bus Event| F[(ron-bus)]
  F --> G[Consumers (gateway, overlay, index...)]
  B -->|Metrics| H[(Prometheus)]
  B -->|/healthz /readyz| I[Probes]
  classDef ok fill:#C6F6D5,stroke:#2F855A,color:#1A202C;
  class B ok; class E ok; class F ok; class I ok
```

## 13.2 Example systemd Unit

```ini
# /etc/systemd/system/svc-registry.service
[Unit]
Description=RustyOnions svc-registry (append-only signed registry)
After=network-online.target
Wants=network-online.target

[Service]
User=ron
Group=ron
Environment=RUST_LOG=info,svc_registry=info
Environment=RON_CONFIG=/etc/ron/svc-registry.toml
Environment=REGISTRY_ADDR=0.0.0.0:9444
Environment=METRICS_ADDR=0.0.0.0:9909
ExecStart=/usr/local/bin/svc-registry --config ${RON_CONFIG}
Restart=always
RestartSec=2
LimitNOFILE=65536
TimeoutStopSec=15
KillSignal=SIGINT

[Install]
WantedBy=multi-user.target
```

## 13.3 Incident Escalation Policy (Template)

* **Severities**

  * **SEV-1**: `5xx > 1%` for 5 min **or** `/readyz=false` (reads) for 2 min.
  * **SEV-2**: commit p95 > 1200 ms for 10 min **or** `bus_lagged_total` monotonic for 15 min.
  * **SEV-3**: intermittent 429/503 spikes (>1% for 10 min).
* **Paging**

  * SEV-1: page **Primary SRE** immediately (PagerDuty “RON-Core Primary”).
  * SEV-2: notify **Secondary SRE** and **Service Owner** within 10 min.
  * SEV-3: open ticket; post in `#ron-ops`; review next stand-up.
* **War Room**

  * Channel `#inc-svc-registry-<YYYYMMDD-HHMM>`; pin runbook sections (§5, §6) + Grafana dashboard.
* **Comms**

  * 15-min updates; incident timeline in issue.
* **Exit**

  * Postmortem ≤72h; attach flamegraphs, PromQL, perf diffs.

## 13.4 Quantified Recovery Objectives

* **Cold start to read-ready**: **< 0.8 s** (Micronode), **< 1.2 s** (Macronode).
* **Checkpoint restore (50k descriptors)**: **< 10 s** to read-ready; **< 20 s** to write-ready.
* **Blue/Green cutover validation**: `/readyz=true` for **≥10 min** before flip.
* **RPO (Macronode)**: ≤ **15 min** (checkpoint cadence). **Micronode**: n/a; re-sync upstream.
* **RTO (Macronode)**: ≤ **20 min** for storage faults (automated restore + reheal).

> Refresh: run perf harness “restore-50k” monthly; record timings in `PERFORMANCE.md §8 Perf Evolution`.

## 13.5 Prometheus Alert Rules (Copy-Paste)

```yaml
groups:
- name: svc-registry
  interval: 30s
  rules:
  - alert: RegistryReadP95High
    expr: |
      histogram_quantile(0.95, sum by (le) (rate(request_latency_seconds_bucket{route="/registry/head"}[5m])))
        > 0.05
    for: 15m
    labels: { severity: "page" }
    annotations:
      summary: "svc-registry read p95 high"
      desc: "p95(/registry/head) > 50ms for 15m"

  - alert: RegistryCommitP95High
    expr: |
      histogram_quantile(0.95, sum by (le) (rate(commit_latency_seconds_bucket[5m])))
        > 0.75
    for: 5m
    labels: { severity: "page" }
    annotations:
      summary: "commit p95 high"
      desc: "Commit p95 > 750ms for 5m"

  - alert: RegistryErrorBudgetBurn
    expr: |
      sum(rate(http_requests_total{code=~"5.."}[5m]))
        / sum(rate(http_requests_total[5m])) > 0.001
    for: 5m
    labels: { severity: "page" }
    annotations:
      summary: "5xx over 0.1%"
      desc: "Error budget violating; investigate crashes/deps"

  - alert: RegistryBusLag
    expr: rate(bus_lagged_total[5m]) > 0
    for: 10m
    labels: { severity: "ticket" }
    annotations:
      summary: "bus drops increasing"
      desc: "Lossy bus observed; clients should reconcile via /registry/head"

  - alert: RegistryNotReady
    expr: probe_success{job="svc-registry-readyz"} == 0
    for: 5m
    labels: { severity: "page" }
    annotations:
      summary: "/readyz false"
      desc: "Service not ready for >5m"

  - alert: RegistryCheckpointStale
    expr: max_over_time(registry_checkpoint_age_seconds[10m]) > 120
    for: 10m
    labels: { severity: "ticket" }
    annotations:
      summary: "checkpoint staleness"
      desc: "Checkpoint age >120s; tune cadence or storage"
```

## 13.6 Backup Script (Macronode)

```bash
#!/usr/bin/env bash
set -euo pipefail
SRC="/var/lib/ron/svc-registry/checkpoint"
DST="/var/backups/ron/svc-registry/$(date +%F_%H%M)"
mkdir -p "$DST"
rsync -a --delete "$SRC/" "$DST/"
find /var/backups/ron/svc-registry -maxdepth 1 -type d -mtime +7 -print0 | xargs -0r rm -rf
```

## 13.7 Smoke Tests (Post-Upgrade)

```bash
# Read plane
curl -fsS http://127.0.0.1:9444/registry/head | jq .version

# Commit simulation (staging): propose -> approve (M-of-N) -> commit
ronctl registry propose --file sample.json
ronctl registry approve --id <proposal> --signer <signerA>
ronctl registry approve --id <proposal> --signer <signerB>
ronctl registry commit  --id <proposal>

# Propagation window (p95 < 200ms)
curl -N http://127.0.0.1:9444/registry/stream | head -n 1
```

```

---
```
