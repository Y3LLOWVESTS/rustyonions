
---

````markdown
---
title: RUNBOOK — ron-transport
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-01
audience: operators, SRE, auditors
---

# 🛠️ RUNBOOK — ron-transport

> ron-transport is a **library crate**. It does not expose ports or HTTP endpoints by itself. This runbook explains how to operate services that **embed** the library (e.g., gateway, overlay) and how to diagnose transport-layer issues (TCP/TLS/Arti).

This document satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 0) Purpose

Operational manual for `ron-transport`: startup wiring, health semantics, diagnostics, failure modes, recovery, scaling, and security ops when embedded into a service. All guidance assumes the service surfaces metrics/logs/readiness derived from the library.

---

## 1) Overview

- **Name:** `ron-transport`
- **Role:** Unified transport library (TCP/TLS, optional Tor via Arti) with strict I/O ceilings and rate limits. No overlay/DHT logic.
- **Criticality Tier:** **0 (kernel)** — foundational; multiple services depend on it for ingress/egress.
- **Dependencies:**
  - Required: `tokio`, `tokio-rustls` (when TLS enabled)
  - Optional: `arti` (Tor) under `--features arti`
  - OS: entropy source; Unix perms enforcement for key files and UDS
- **Ports Exposed:** **None (library).** Consumer services expose their own ports (e.g., `oap_addr`, `metrics_addr`).
- **Data Flows:** Byte streams carrying OAP/1 frames (the library **does not parse OAP**); emits structured logs/metrics via the consumer.
- **Version Constraints:** MSRV 1.80.0; compatible with rustls 0.22+/tokio-rustls re-exports; Arti versions per consumer lockfile.

---

## 2) Startup / Shutdown

Because `ron-transport` is a library, startup is through the **embedding service**.

### Startup (example service)

```bash
RON_TRANSPORT_BACKEND=tls \
RON_TRANSPORT_BIND_ADDR=0.0.0.0:9444 \
RON_TRANSPORT_MAX_CONNS=2048 \
RON_TRANSPORT_READ_TIMEOUT=5s \
RON_TRANSPORT_WRITE_TIMEOUT=5s \
RON_TRANSPORT_IDLE_TIMEOUT=60s \
RON_TRANSPORT_TLS_ENABLED=true \
RON_TRANSPORT_TLS_CERT_PATH=/etc/ron/cert.pem \
RON_TRANSPORT_TLS_KEY_PATH=/etc/ron/key.pem \
my-overlay-service --config /etc/ron/Config.toml
````

**Verification (consumer responsibilities):**

* Logs: a single readiness transition to `Ready` (or `Degraded("arti_bootstrap")` if Arti is bootstrapping).
* `/readyz` (from the service): `200 OK`.
* Metrics (Prometheus): buckets present for `handshake_latency_seconds` and `rejected_total{reason=...}`.

### Shutdown

* `SIGINT` / `SIGTERM`: service should:

  1. stop accepting new connections,
  2. cancel per-conn tasks (deterministic), and
  3. drain gracefully within the service’s shutdown budget (typ. ≤ 5s).
* systemd:

  * `systemctl stop <service>` (ensure `TimeoutStopSec` ≥ idle timeout).

---

## 3) Health & Readiness

**Library readiness contract (surfaced by the service):**

* `Ready` — transport can accept/dial with configured backend.
* `Degraded{ reason }` — transport is usable but impaired. Example: `reason="arti_bootstrap"`.
* `NotReady{ reason }` — transport cannot serve (bind failure, TLS key invalid, Arti exceeded bootstrap timeout).

**Operator expectations (common):**

* Cold start to `Ready`: typically **< 2 s** (TCP/TLS).
* Arti path: `Degraded("arti_bootstrap")` first; transitions to `Ready` usually **< 30 s** (bounded by `arti.bootstrap_timeout`).

**If not ready after expected window:**

* Check service logs for the **exact reason** string.
* Inspect metrics:

  * `transport_dials_total{result="error"}` increasing?
  * `rejected_total{reason="arti_bootstrap"|"tls"|"too_large"|"rate_limited"|"peer_limit"|"timeout"}`
  * `conn_inflight{backend=...}` stuck at 0 with dials attempted?

---

## 4) Common Failure Modes

| Symptom / Signal                                                              | Likely Cause                                     | What to Check (Metric/Log)                                                              | Resolution                                                                                                  | Alert Threshold                                |
| ----------------------------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| Service `/readyz` stays `Degraded("arti_bootstrap")` then flips to `NotReady` | Arti cannot bootstrap in time                    | `rejected_total{reason="arti_bootstrap"}`; Arti logs (consumer should surface)          | Increase `arti.bootstrap_timeout` cautiously; verify egress/network; confirm no disk writes under amnesia.  | Degraded > 60s, or NotReady event fired        |
| TLS start fails at boot                                                       | Key/cert missing or perms too open               | Error log: `tls key read failed` or `insecure permissions`; no `Ready` event            | Fix paths; enforce **0600** on key; restart.                                                                | Any occurrence in production                   |
| Many `TooLarge` rejects                                                       | Peer/software sending frames > 1 MiB             | `rejected_total{reason="too_large"}` increments; p95 handshake latency stable           | Validate peer; ensure OAP framing capped; **do not raise** frame limit; engage producer team.               | > 5/min sustained                              |
| Throughput collapses; rising `rate_limited`                                   | Rate limits too strict vs workload               | `rejected_total{reason="rate_limited"}`; token bucket stats (if exposed by service)     | Increase `rates.*` (global/per_conn/per_peer) with canary; monitor latency & error rates.                   | p95 > target and rate_limited > 1% of requests |
| Many connections, CPU spike, high wakeups                                     | Missing per-peer caps; a hot peer floods         | `conn_inflight` skew per peer (service should export), accept rate burst from single IP | Enable `per_peer.max_conns` and `per_peer_bytes_per_sec`; consider temporary IP-level firewall throttling.  | Any spike lasting > 5 min                      |
| UDS clients cannot connect                                                    | UDS perms or `allow_uids` mismatch               | Error: `uds peer not permitted`; check socket owner/mode                                | Ensure dir 0700, socket 0600; add required UID to `allow_uids`; restart or reload.                          | Any                                            |
| Long tail latency without errors                                              | Upstream congestion or Nagle/ALPN mismatch       | `handshake_latency_seconds` tail; backend="tls" with odd ALPN                           | Pin ALPN as needed in `tls_client.alpn`; validate upstream/load balancer TCP settings (no mid-conn resets). | p99 > SLO for ≥10 min                          |
| Frequent timeouts (`Timeout` kind)                                            | Dead peer or packet loss; timeout too aggressive | `rejected_total{reason="timeout"}`; TCP retransmits (host), VPN/egress metrics          | Increase read/write timeouts moderately; audit network; test via chaos latency injection.                   | > 1% of requests or sustained spikes           |

---

## 5) Diagnostics

### Logs (service emits, library provides reason strings)

* Filter by correlation:
  `journalctl -u <service> -f | grep -E 'conn_id=|corr_id='`
* Look for structured reasons: `reason=arti_bootstrap|tls|too_large|rate_limited|peer_limit|timeout`

### Metrics (typical Prometheus endpoint on the service)

* Handshake latency histogram:
  `handshake_latency_seconds{backend="tls"|"tcp"|"arti"}`
* Counters:
  `transport_dials_total{backend,result}`, `transport_accepts_total{backend}`, `rejected_total{reason}`, `bytes_in_total`, `bytes_out_total`, `conn_inflight`

### Quick grep

```bash
curl -s http://127.0.0.1:9909/metrics | egrep 'transport_(dials|accepts|rejected|bytes_|conn_inflight)|handshake_latency_seconds'
```

### Packet-level

* `tcpdump -i any port 9444 -vv` (ensure no secrets; prefer metadata only)
* For TLS client trust issues: validate CA bundle on host; check SNI/ALPN settings.

### Arti (when enabled)

* Confirm network egress/ports; check service logs for Arti sub-logger.
* If amnesia enabled, verify **no on-disk Arti caches** exist.

---

## 6) Recovery Procedures

1. **Arti bootstrap stall**

* **Symptom:** Long Degraded → NotReady.
* **Action:** Verify connectivity; bump `arti.bootstrap_timeout` (small increments); redeploy. Consider `pre_dial_delay` to avoid stampede after cold starts.

2. **TLS key/cert problems**

* **Symptom:** Boot fails; NotReady with `tls` error.
* **Action:** Fix paths/perms (0600 key); validate PEM; restart.

3. **Hot peer overload**

* **Symptom:** CPU spike; many inflight from one peer; growing `rate_limited`.
* **Action:** Enable/tune `per_peer.max_conns` and `per_peer_bytes_per_sec`; coordinate with producer; if needed, temporary network throttling.

4. **Excessive timeouts**

* **Symptom:** `rejected_total{reason="timeout"}` spikes.
* **Action:** Increase read/write timeouts slightly; validate network; test with controlled latency injection.

5. **Misconfig reload**

* **Symptom:** Reload leads to errors / loss of readiness.
* **Action:** Roll back to prior snapshot (service should keep last-good); fix config; re-validate; reapply.

6. **Rollback upgrade**

* **Action:** Redeploy previous service build; ensure library version and config are compatible; verify readiness and that rejects normalize.

---

## 7) Backup / Restore

* `ron-transport` is **stateless** by default. Under **amnesia=ON**, it guarantees no persistent artifacts.
* If the embedding service persists **TLS keys or CA bundles**, store them in your secret store; restore by re-mounting secrets and restarting.

---

## 8) Upgrades

* Drain traffic on the service (stop accepting, wait for `conn_inflight → 0` or below threshold).
* Redeploy the service with new library build.
* Post-upgrade validation for **10 minutes**:

  * `/readyz` = 200
  * No `NotReady` transitions
  * Stable `handshake_latency_seconds` buckets
  * `rejected_total` baseline unchanged (or lower)

---

## 9) Chaos Testing

Recommended quarterly drills:

* **Latency injection (egress):**

  * Fault: +100–300 ms RTT
  * Expectation: Slight p95 rise; timeouts/rejects remain <1%.
* **Packet loss (1–5%):**

  * Expectation: No panics; controlled increase in timeouts, alert fires if thresholds crossed.
* **Arti bootstrap failure:**

  * Fault: Drop Tor directory access
  * Expectation: `Degraded("arti_bootstrap")` then `NotReady` at `bootstrap_timeout`; alerts fire; roll-forward procedure executed.
* **Hot peer flood:**

  * Fault: One peer opens > per-peer caps
  * Expectation: Early rejects (`peer_limit`/`rate_limited`), service stays healthy.

---

## 10) Scaling Notes

**Vertical**

* Tune:

  * `max_conns` (global)
  * `rates.per_conn_bytes_per_sec` / `rates.global_*`
  * `per_peer.max_conns` and `per_peer_bytes_per_sec`
* Watch CPU wakeups and memory allocator pressure at high connection counts.

**Horizontal**

* Run multiple replicas of the **service**; each embeds the library.
* Use per-replica `bind_addr` or a load balancer (for TCP/TLS).
* Ensure consistent metrics labels (`instance`, `backend`) to compare replicas.

**SLO hints (typical)**

* Intra-AZ handshake p95: **< 80 ms**
* Cross-region handshake p95: **< 200 ms**
* Rejects (`too_large`/`rate_limited`/`peer_limit`) generally **< 1%** under normal load

---

## 11) Security Ops

* **TLS (server):** Key perms must be **0600**; never log key paths in debug dumps; rotate on standard cadence.
* **TLS (client):** Prefer `trust_store="system"`; when `path`, mount read-only; configure SNI/ALPN only as needed.
* **Amnesia:** With `amnesia=ON`, verify no transport-layer caches/keys on disk; ephemeral material should be zeroized on drop.
* **Arti:** In-proc only; **no external tor daemon**. Readiness reasons are auditable (`arti_bootstrap`); ensure logs don’t leak circuit details.
* **PQ readiness:** `pq.mode="hybrid"` only when upstream stack and peers are compatible; roll out via canary; monitor handshake latency and failure modes.
* **UDS:** Enforce `SO_PEERCRED`; socket dir 0700, file 0600; `allow_uids` must be explicit.

---

## 12) References

* `docs/CONFIG.md` — configuration schema & validation
* `docs/IDB.md` — invariants, principles, acceptance gates
* `docs/SECURITY.md` — threat model, hardening posture
* `docs/OBSERVABILITY.md` — metrics/buckets, logging reasons
* `docs/CONCURRENCY.md` — single-writer, cancellation, loom
* `docs/TESTS.md` — property/fuzz/chaos/soak gates
* Blueprints: Hardening, Concurrency & Aliasing, Omnigate

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A (Metrics):** `handshake_latency_seconds` and `rejected_total{reason}` within SLO; buckets populated
* [ ] **Gate J (Chaos):** Quarterly drills pass (latency, loss, Tor bootstrap, hot peer flood)
* [ ] **Gate K (Vigilance):** Alerts wired on: NotReady transition, Degraded > 60s, rejects > 1%, p95 handshake breach
* [ ] **Gate N (Edge Perf):** ARM/edge profile completed; no pathological syscalls/allocations under load
* [ ] **Gate O (Security):** TLS key perms enforced; amnesia verified; PQ canary plan documented

```
---

