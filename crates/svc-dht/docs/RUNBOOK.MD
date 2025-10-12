**File:** `crates/svc-dht/RUNBOOK.md`

````markdown
---
title: RUNBOOK — svc-dht
owner: Stevan White
msrv: 1.80.0
version: 1.1 (post-review 2025-10-11)
next-review: 2026-01-11
last-reviewed: 2025-10-11
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — svc-dht

## 0) Purpose
Operational manual for `svc-dht` (Kademlia/Discv5 discovery service): startup, health, diagnostics, failure modes, recovery, scaling, and security ops.  
This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

> **MSRV policy:** Repo-wide MSRV is `1.80.0`. Review quarterly; bump only after CI is green across all crates.

---

## 1) Overview
- **Name:** `svc-dht`
- **Role:** Provider/peer discovery. Maintains DHT routing table; answers `FIND_NODE`, `GET_PROVIDERS`, `PROVIDE`. **Overlay logic lives elsewhere** (strict separation).
- **Criticality Tier:** 1 (critical service for resolve/read paths)
- **Dependencies:**  
  `ron-kernel` (bus, health), `ron-transport` (TLS/TCP; optional Tor via `arti` feature), `ron-metrics` (Prometheus HTTP), `ron-policy` (rate/geo/abuse), `svc-registry` (signed descriptors)
- **Ports/Endpoints:**  
  - `DHT_ADDR` (UDP/TCP as configured)  
  - `METRICS_ADDR` (HTTP): `/metrics`, `/healthz`, `/readyz`  
  - Optional admin UDS (peer-cred gated)
- **Data Flows & SLOs:**  
  Ingress = DHT RPCs; Egress = discovery results + bus events + golden metrics.  
  Targets: **p99 hops ≤ 5**; **p95 intra-region lookup < 500 ms** (nominal).
- **Version/Interop:**  
  Requires workspace-pinned `ron-*` API surface; OAP/1 framing limits apply end-to-end (max frame **1 MiB**; stream chunk **64 KiB**).  
  **PQ note:** PQ enforcement supported (e.g., ML-DSA-87) when `REQUIRE_PQ_ON=true`; see §12.1.

---

## 2) Startup / Shutdown

### 2.1 Startup
```bash
cargo run -p svc-dht -- --config ./configs/svc-dht.toml
# or
./target/release/svc-dht --config /etc/ron/svc-dht.toml
````

**Verification**

```bash
curl -fsS http://127.0.0.1:9909/readyz
# expect: 200 OK

curl -fsS http://127.0.0.1:9909/metrics | grep -E 'ron_dht_(lookup|hops|bootstrap)'
journalctl -u svc-dht -n 100 --no-pager | grep 'ready=1'
```

### Common startup errors

| Error                    | Likely Cause                     | Fix                                                    |
| ------------------------ | -------------------------------- | ------------------------------------------------------ |
| `DhtError::Bind`         | Port in use / insufficient perms | Change `DHT_ADDR` or free port; check `ulimit -n`      |
| `BootstrapTimeout`       | Seed unreachable / egress issue  | Verify bootstrap IPs/SG/routes; retry with jitter      |
| `ConfigInvalid(key=...)` | Bad TOML / unknown flag          | Run `ronctl config check`; rollback to last-known-good |

### 2.2 Shutdown

* `Ctrl-C` (SIGINT): graceful drain; readiness flips before listeners close.
* systemd:

```bash
systemctl stop svc-dht
```

**Crash-only safety:** supervised tasks restart with jittered backoff; `ServiceCrashed{service,reason}` broadcast on bus.

---

## 3) Health & Readiness

* **/healthz:** process alive (liveness).
* **/readyz:** listeners bound, bus subscribed, bootstrap succeeded (≥1 **live** bootstrap; i.e., pingable and responsive to handshake), routing table warmed.
* **SLA:** p99 time-to-ready ≤ **10s** after start; 99% of restarts ready within **15s**.

**If not ready after 15s**

1. Check bus for `ServiceCrashed`.
2. Verify outbound egress to bootstraps.
3. Inspect metrics: `ron_dht_bootstrap_peers == 0`, `ron_dht_inflight_lookup` pinned.

**Degraded control (ops drill):**

```bash
ronctl set-degraded --service=svc-dht --on
# rollback:
ronctl set-degraded --service=svc-dht --off
```

**PromQL ready probe:**

```promql
probe_success{job="svc-dht-readyz"} == 1
```

---

## 4) Environment / Flags (Ops Surface)

| Name                 | Type   |                             Default | Description                                            |
| -------------------- | ------ | ----------------------------------: | ------------------------------------------------------ |
| `RON_CONFIG`         | path   |                             (unset) | Path to config file (TOML). Overrides per-flag values. |
| `DHT_ADDR`           | addr   |                      `0.0.0.0:9444` | Bind address (UDP/TCP depending on config).            |
| `METRICS_ADDR`       | addr   |                    `127.0.0.1:9909` | Prometheus/health endpoints.                           |
| `TRANSPORT_FEATURES` | csv    |                             (unset) | `arti` enables Tor via `ron-transport`.                |
| `RON_AMNESIA`        | on/off | `on` (micronode), `off` (macronode) | RAM-only caches / ephemeral logs when `on`.            |
| `DHT_ALPHA`          | int    |                                 `3` | Parallel lookups degree (α).                           |
| `DHT_BETA`           | int    |                                 `2` | Hedged extra queries (β).                              |
| `DHT_TIMEOUT_MS`     | ms     |                               `750` | Per-hop timeout.                                       |
| `POLICY_PROFILE`     | enum   |                           `default` | Rate/geo/abuse policy profile.                         |
| `REQUIRE_PQ_ON`      | bool   |                             `false` | Enforce PQ-only signatures/handshakes (see §12.1).     |

> See `CONFIG.md` for full matrix (peer ceilings, retries, jitter, TLS ciphers, bucket writer guarantees).

---

## 5) Common Failure Modes

| Symptom                    | Likely Cause                | What to Check                          | Resolution                                                                          | Severity | Alert         |
| -------------------------- | --------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------- | :------: | ------------- |
| Lookups slow (p95 > 500ms) | Lost bootstrap / partition  | `ron_dht_bootstrap_peers==0`, hop p99↑ | Reseed bootstraps; verify egress; temporarily raise α, then tighten                 |     M    | p95>500ms 15m |
| Hop count > 5 p99          | Eclipse/Sybil; churn spike  | `ron_dht_hops_p99>5`, peer diversity↓  | `ronctl policy update --profile=strict`; rotate bootstrap set; raise β to 3 for 15m |     H    | any           |
| PROVIDE failures           | Policy/Quota rejects        | `rejected_total{reason}`               | Update `ron-policy`; adjust DRR; client backoff+jitter                              |     M    | >5/min        |
| Table thrash + CPU pegged  | α/β too high; no semaphores | `inflight_lookup` ~ ceiling            | Reduce α/β; enforce semaphores; add replica                                         |     M    | sustained 5m  |
| Ready flapping             | Crash loop; bus overload    | `service_restarts_total`↑              | Examine last panic; widen backoff; cut subscribers                                  |     H    | >3/5m         |
| 5xx on `/get_providers`    | Downstream index/storage    | 5xx logs; degraded=true                | Enter read-only degraded; coordinate with owners                                    |     H    | >1% 10m       |

---

## 6) Diagnostics

* **Logs (structured JSON):**

```bash
journalctl -u svc-dht -f | jq -r 'select(.corr_id!=null) | [.ts,.level,.msg,.corr_id,.hops,.peer_id] | @tsv'
# requires: jq
```

* **Metrics (Prometheus):**

```bash
curl -s http://127.0.0.1:9909/metrics | grep -E 'ron_dht|requests_total|latency_seconds'
```

* **Bus events:**

```bash
ronctl tail --topic svc-dht
```

* **Perf (hotpath):**

```bash
cargo install flamegraph
cargo flamegraph -p svc-dht
```

**Mermaid — Startup flow**

```mermaid
flowchart LR
A[Start svc-dht] --> B[Bind listeners]
B --> C[Subscribe to bus]
C --> D[Bootstrap to peers]
D -->|ok| E[Warm routing table]
D -->|timeout| F[Retry + backoff]
E --> G[/readyz=200]
```

---

## 7) Recovery Procedures

### 7.1 DHT Failover / Missing Providers

**Signals:** `ron_dht_missing_records>0`, `rf_observed<rf_target`, p95↑
**Action:**

```bash
ronctl repair --hash b3:<HEX> --rf 3 --prefer-region <REGION>
```

**Exit:** Alerts clear ≥30m; p95 < 500ms; hop p99 ≤ 5.

### 7.2 Eclipse / Sybil Suspicion

* Enforce per-op rate limits (policy profile → `strict`).
* Rotate bootstrap set (publish new seeds).
* Temporarily raise β (hedging) to `3`; revert after stability.

### 7.3 Routing Table Corruption

* Restart service (crash-only).
* Ensure **single-writer per k-bucket** and backpressure are enabled.

### 7.4 Overload

* Lower α/β; enforce semaphores.
* Horizontal scale: add 1+ replicas; enable gateway early shedding.

### 7.5 Rollback Upgrade

```bash
git checkout vX.Y.Z
cargo build --release
systemctl restart svc-dht
curl -fsS http://127.0.0.1:9909/readyz
```

**Escalation:** If unresolved in 15m, page Pillar 9 (Ops) and Pillar 10 (Security). Contacts: see `GOVERNANCE.md` on-call rota.

---

## 8) Backup / Restore

Stateless in durable terms; routing table regenerates.
If a persistent cache is enabled (macronode), treat as **disposable**.

**!Amnesia cache ops (if present):**

```bash
# backup
rsync -a /var/ron/dht-cache host:/backup/svc-dht-cache/

# restore (service stopped)
rsync -a host:/backup/svc-dht-cache/ /var/ron/dht-cache/
```

---

## 9) Upgrades

### 9.1 Canary / Blue-Green

* Bring up **green** at 10% traffic; confirm for 10m:

  * `/readyz` stable
  * `service_restarts_total` flat
  * p95 lookup < 500ms; hop p99 ≤ 5
* Promote to 100%; keep **blue** hot for 30m.

### 9.2 Config Migrations

* Apply atomically; on invalid config, **auto-rollback** to last-known-good (LKG).
* **Governance:** CR approval required for Tier-1 changes.

### 9.3 PQ Rollout (feature gate)

1. Set `REQUIRE_PQ_ON=true` on **canary**.
2. Watch `pq_downgrade_attempt_total` and client error rate for **30m**.
3. Promote to 100%; rollback by toggling flag (audited path only).

---

## 10) Chaos Testing (Gate K)

**Quarterly drill** (CI + staging):

| Fault                               | Method                                                                | Success Criteria                                                                |
| ----------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| 50% packet loss on bootstraps (15m) | `ronctl chaos inject --fault=loss --percent=50 --targets=bootstrap`   | `/readyz` flips before collapse; hop p99 ≤ 7; **MTTR < 5m** after fault removal |
| +300ms latency (regional link)      | `ronctl chaos inject --fault=latency --ms=300 --targets=inter-region` | p95 < 800ms; zero data loss; proper shedding under load                         |
| Partition a replica                 | iptables drop (or `ronctl chaos`)                                     | Remaining replicas sustain p95 < 700ms; alerts fire < 2m                        |
| CPU saturation (1 core pegged)      | `stress-ng`                                                           | RPS throttled (not crashed); **MTTR < 5m**                                      |

**Fallback (no `ronctl chaos`):**
Use Linux `tc` netem:

```bash
tc qdisc add dev <iface> root netem loss 50%
tc qdisc change dev <iface> root netem delay 300ms
tc qdisc del dev <iface> root netem
```

---

## 11) Scaling Notes

* **Knobs:** α (parallelism), β (hedging), per-peer ceilings, per-hop timeout, `ron-transport` read/write/idle timeouts.
* **Nominal churn:** < **10%** peer turnover per hour. If ≥10% for 10m+, treat as degraded and raise β temporarily.
* **When to scale:** if `inflight_requests > 80% capacity` for 10m, or hop p99 > 5 for 10m without partition explanation.
* **Rule of thumb:** Each replica sustains ~**500 rps** @ 4c/8GB under nominal churn.

---

## 12) Security Ops

* **Zero-trust:** capability-gated admin; no ambient authority. Reject unauthenticated control requests.
* **Transport:** TLS1.3 via `ron-transport`. `arti` (Tor) is a **library feature** toggle; ensure read/write/idle timeouts.
* **Amnesia Mode:** RAM-only caches; ephemeral logs; no disk spill when enabled.
* **Observability Security:** Structured rejects (400/404/413/429/503) with no secret leakage; redact PII.

### 12.1 Post-Quantum (PQ) Hooks

* When `REQUIRE_PQ_ON=true`:

  * Reject classical-only signatures/handshakes.
  * Emit `pq_downgrade_attempt_total` and structured security log.
  * Rollback toggle only via audited, capability-gated path.

**Vulnerability hygiene**

```bash
cargo deny check advisories
# optional if enabled in workspace
cargo audit
```

**Tor mode monitoring:** watch `ron_transport_tor_errors` (gauge/counter) for stability.

---

## 13) References

* **Internal:** `CONFIG.md`, `SECURITY.md`, `OBSERVABILITY.md`, `CONCURRENCY.md`, `PERFORMANCE.md`, `INTEROP.md`, `IDB.md`, blueprints (App/Omnigate/Scaling/Hardening)
* **External:** Maymounkov & Mazieres (2002), *Kademlia: A Peer-to-Peer Information System Based on the XOR Metric*; IETF **QUIC RFC 9000** (transport context)

---

## ✅ Perfection Gates Checklist (svc-dht)

* [ ] **A — Metrics:** `/metrics` exposes golden set; `build_info` present
* [ ] **B — Interop:** wire/DTOs validated vs `INTEROP.md`
* [ ] **C — Config:** schema validated; `deny_unknown_fields` enforced; LKG set
* [ ] **D — Concurrency:** single-writer buckets; no locks across `.await`
* [ ] **E — Observability:** exemplars; RED metrics exported; structured logs
* [ ] **F — Perf Gates:** p99 hops ≤ 5; p95 intra < 500ms; perf CI regression gates
* [ ] **G — Governance:** CR approval recorded for last upgrade/migration
* [ ] **H — Economics:** quotas/rate limits active (policy profile)
* [ ] **I — Invariants:** IDB invariants green (pre-deploy)
* [ ] **J — Chaos:** quarterly drill passed; **MTTR < 5m**
* [ ] **K — Vigilance:** alerts wired; runbook reviewed on schedule
* [ ] **L — Black Swan:** eclipse/failover playbooks tested
* [ ] **M — Portability:** runs on ARM64 & x86_64 with same SLOs
* [ ] **N — Edge Perf:** profiled on low-core edge targets

---

# Appendices

## A) Prometheus — Suggested Alerts

```promql
# Ready stuck false for 15s (fast-fire)
ALERT SvcDhtNotReady
  IF probe_success{job="svc-dht-readyz"} == 0
  FOR 15s
  LABELS { severity="page" }
  ANNOTATIONS { summary="svc-dht not ready", runbook="RUNBOOK.md#3-health-readiness" }

# Bootstrap lost for 2m
ALERT DhtBootstrapZero
  IF max_over_time(ron_dht_bootstrap_peers[2m]) == 0
  FOR 2m
  LABELS { severity="page" }
  ANNOTATIONS { summary="svc-dht lost all bootstraps", runbook="RUNBOOK.md#7-recovery-procedures" }

# Lookup latency regression
ALERT DhtLookupP95High
  IF histogram_quantile(0.95, sum by(le) (rate(ron_dht_lookup_ms_bucket[5m]))) > 0.5
  FOR 10m
  LABELS { severity="ticket" }
  ANNOTATIONS { summary="svc-dht p95 lookup > 500ms" }

# Hop budget exceeded (if recorded as gauge)
ALERT DhtHopP99High
  IF ron_dht_hops_p99 > 5
  FOR 10m
  LABELS { severity="ticket" }
  ANNOTATIONS { summary="svc-dht hop p99 > 5" }

# Crash loop
ALERT DhtRestartsBurst
  IF increase(service_restarts_total{service="svc-dht"}[5m]) > 3
  LABELS { severity="page" }
  ANNOTATIONS { summary="svc-dht crash loop detected" }

# PQ downgrade attempts (if PQ enforced)
ALERT DhtPqDowngradeAttempts
  IF increase(pq_downgrade_attempt_total[10m]) > 0
  LABELS { severity="ticket" }
  ANNOTATIONS { summary="PQ downgrade attempts seen" }
```

> If `ron_dht_hops_p99` is derived via recording rules, include the recording expression in your ruleset.

---

## B) On-Call Quick Reference

* **Page triggers:** `SvcDhtNotReady`, `DhtBootstrapZero`, `DhtRestartsBurst`
* **First 5 minutes:**

  1. Check `/readyz`, then logs for `ServiceCrashed`.
  2. Verify egress to bootstraps (security groups, routes).
  3. If eclipse suspected, switch to `strict` policy + rotate bootstraps.
* **If unresolved @15m:** escalate (Pillar 9/10), initiate rollback or failover.

---

## C) Post-Incident Template

```
Incident: <id / date / duration>
Impact: <user-visible effects, SLOs breached>
Timeline:
- t0: detection (alert X)
- t1: triage start, actions A/B/C
- t2: mitigation applied
- t3: resolved, verification complete

Root Cause:
Contributing Factors:
What Went Well:
What Went Poorly:
Action Items (owner → due):
- [ ] ...
```

---

## D) Glossary

* **α (alpha):** parallelism for lookup queries.
* **β (beta):** hedged extra queries beyond alpha.
* **hop:** a DHT step to a peer closer to target key.
* **amnesia mode:** RAM-only operation; no persistent logs/caches.

```
```
