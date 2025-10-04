
---

# 🛠️ RUNBOOK — svc-overlay

---

title: RUNBOOK — svc-overlay
owner: Stevan White (with ECON note from Pillar 12)
msrv: 1.80.0
last-reviewed: 2025-10-03
audience: operators, SRE, auditors
----------------------------------

## 0) Purpose

Operational manual for **svc-overlay**: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.
Satisfies **PERFECTION_GATES**: **K** (Continuous Vigilance) and **L** (Black Swan Economics).

---

## 1) Overview

* **Name:** `svc-overlay`
* **Role:** Overlay session & gossip plane (OAP/1 over TLS/QUIC/Tor via `ron-transport`). Not a DHT, not a router.
* **Criticality Tier:** **1 — critical service** (ingress assist & session fabric)
* **Dependencies (runtime):**

  * `ron-bus` (internal broadcast bus)
  * `ron-transport` (TLS/TCP, optional QUIC/Tor)
  * `svc-index` (object address → location lookups) — read-only assist
  * `svc-storage` (blob fetch/stream) — downstream dependency
  * `svc-gateway` (upstream ingress)
  * **Facet assists:** `svc-index` (Graph/Search), `svc-mailbox` (Feed/Media) — integration latency **monitored** but **not owned**
  * **Audit/Econ:** `ron-audit` for quota/cap events; **no economic custody** in overlay (ledger/state belongs to `ron-ledger`)
* **Ports Exposed (defaults):**

  * `oap_addr = 0.0.0.0:9444` (OAP/1 framed)
  * `admin_addr = 127.0.0.1:9600` (HTTP admin: /metrics, /healthz, /readyz, /version)
* **Data Flows:** OAP/1 frames in/out; admin HTTP for health/metrics; bus events for health/config/crash signals.
* **Version Constraints:** requires `ron-kernel` public API (Bus + KernelEvent) compatible with this release; `axum 0.7` admin plane.

**Normative invariants (must hold):**

* `OAP/1 max_frame = 1 MiB`
* **Streaming chunk size ≈ 64 KiB** (no full buffering)
* **Truthful readiness:** `/readyz` flips to not ready under overload/dependency failure
* **QUIC 0-RTT disabled** in prod unless explicitly gated & tested

---

## 2) Startup / Shutdown

### Startup

```bash
# Local dev
cargo run -p svc-overlay -- --config ./configs/svc-overlay.toml

# Release binary
./target/release/svc-overlay --config /etc/ron/svc-overlay.toml

# Example env override
RON_CONFIG=/etc/ron/svc-overlay.toml ./svc-overlay
```

**Key env/flags (examples):**

* `RON_CONFIG` — absolute path to config (wins over `--config`)
* `OVERLAY_OAP_ADDR`, `OVERLAY_ADMIN_ADDR` — override listen sockets
* `OVERLAY_PQ_MODE=classical|hybrid` — handshake mode (perf doc)
* `--tor` (if built with Tor feature) — bind onion service
* `--quic` — enable QUIC listener (0-RTT off by default)

**Verification checklist (must pass to consider “up”):**

* Logs show `ready=1` with bound sockets and bus subscription
* `curl -sf http://127.0.0.1:9600/healthz` → `200 OK`
* `curl -sf http://127.0.0.1:9600/readyz` → `200 OK` (within 2–5s warm-up)
* `curl -sf http://127.0.0.1:9600/version` returns git/tag

### Shutdown

* Foreground: `Ctrl-C` (SIGINT) → clean **KernelEvent::Shutdown** published; graceful drain
* systemd: `systemctl stop svc-overlay` → expect `/readyz` to drop first, then exit
* Docker: `docker stop --time=15 svc-overlay` (graceful)

---

## 3) Health & Readiness

**Admin endpoints (HTTP on `admin_addr`):**

* `/healthz` → process alive (always cheap)
* `/readyz` → **truthful readiness** (bus subscribed, transport bound, backpressure below thresholds, downstream pings OK)
* `/metrics` → Prometheus exposition (latency histograms, counters)

**Expected behavior:**

* Initial ready within **2–5s** (micronode cold-start target <300 ms in ideal dev)
* `/readyz` MUST flip to **503** on:

  * bus overflow or lag beyond cap
  * inflight frames ≥ critical threshold
  * downstream dependency hard-down (if configured as required)

**If not ready after 10s:**

1. `journalctl -u svc-overlay -n 200 | less` — look for bind errors or crash loops
2. `curl -s localhost:9600/metrics | grep -E 'rejected_total|bus_overflow|readyz'`
3. `ronctl bus tail --topic kernel` — watch for `ServiceCrashed` / `Health{ok:false}`

---

## 4) Common Failure Modes

| Symptom                        | Likely Cause                                                                      | Metric / Log                                     | Resolution                                                                | Alert Threshold       |
| ------------------------------ | --------------------------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------- | --------------------- |
| 503 spikes on `/readyz`        | Backpressure guard engaged                                                        | `overlay_inflight_frames{state="high"}`          | Scale out; raise window caps cautiously; verify downstream latency        | >5% of checks over 5m |
| 429/503 on ingress via gateway | Quota/rate exceeded                                                               | `quota_exhaustions_total`                        | Increase quotas or reduce client RPS; enable fair-queue layer             | >10/min               |
| p95 GET latency > targets      | Downstream slow (index/storage); copy amp; **facet assist** pressure (Graph/Feed) | `request_latency_seconds_bucket` distribution    | Ensure streaming; use `bytes::Bytes`; verify **svc-index/mailbox** health | p95 > target for 10m  |
| Frequent frame rejections      | Oversize/ratio-cap or malformed frames                                            | `rejected_total{reason=*}`                       | Confirm `max_frame=1MiB`; reject bad peers; audit client SDK version      | >0.5% of RX frames    |
| TLS/QUIC handshake CPU spikes  | High churn; PQ-hybrid overhead                                                    | `handshake_total`, CPU profiler flamegraph       | Enable connection reuse; consider classical mode; add cores               | CPU >80% for 5m       |
| Bus lag / drops                | Event storm; slow consumer                                                        | `bus_lagged_total`, `bus_overflow_dropped_total` | Increase bus buffer; reduce emit frequency; investigate slow tasks        | any overflow          |
| Crash loop on start            | Bad config; port in use; missing feature gate                                     | `ServiceCrashed{service="svc-overlay"}` in logs  | `ronctl config check`; free port; rebuild with correct features           | >3 restarts in 5m     |
| Admin plane slow               | Heavy GC/allocs; lock held across `.await`                                        | `/metrics` latency; tokio-console stalls         | Remove blocking in async paths; optimize allocations; review spans        | p95 admin >200ms      |

---

## 5) Diagnostics

**Logs (with correlation):**

```bash
journalctl -u svc-overlay -f | grep -E 'corr_id=|ERROR|WARN'
```

**Metrics (quick picks):**

```bash
curl -s localhost:9600/metrics | grep -E 'readyz|request_latency_seconds|rejected_total|bus_(lagged|overflow)'
```

**Bus events (kernel/control):**

```bash
ronctl bus tail --topic kernel
ronctl bus tail --topic overlay
```

**Polyglot SDK tracing (interop latency from client → overlay):**

```bash
ronctl trace client --sdk rust --corr_id <ID>
ronctl trace client --sdk ts   --corr_id <ID>
```

**Tracing / Verbosity:**

```bash
RUST_LOG=info,svc_overlay=debug ./svc-overlay --config /etc/ron/svc-overlay.toml
```

**Async stalls (tokio-console):**

```bash
RUSTFLAGS="--cfg tokio_unstable" tokio-console
```

**Perf hotspots:**

```bash
cargo flamegraph -p svc-overlay
```

**QUIC/TLS sanity (if enabled):**

```bash
ss -ltpn | grep -E '9444|9600'
openssl s_client -connect 127.0.0.1:9444 -tls1_3 </dev/null 2>/dev/null | head -n 3
```

---

## 6) Recovery Procedures

1. **Config Drift / Bad Deploy**
   *Symptom:* rejects valid frames, readiness flaps.
   *Action:* `ronctl config check`; compare to golden; restart with known-good.
   *Verification:* `/readyz` stable 10m; `rejected_total` flat.

2. **Backpressure / Overload**
   *Symptom:* 429/503, inflight near cap.
   *Action:* Temporarily lower ingress RPS at gateway; scale replicas; raise `peer.send_window_frames` cautiously; ensure streaming path.
   *Verification:* p95 recovers; `/readyz` steady.

3. **Downstream Failure (index/storage & facets)**
   *Symptom:* elevated p95, 5xx on fetch.
   *Action:* Mark `/readyz` degraded (auto); fail open/closed per policy; escalate to owning service; consider circuit breaker.
   *Facet check:* verify **svc-index** latency, **svc-mailbox** ingest lag <5s (see PERFORMANCE.md).
   *Verification:* Error budget burn halts; dependency restored.

4. **Handshake Path Degradation (PQ)**
   *Symptom:* CPU spikes, connect latency ↑.
   *Action:* Switch `OVERLAY_PQ_MODE=classical` temporarily if policy allows; add cores; enable keep-alive.
   *Verification:* p95 handshake back under SLO.

5. **Crash Loop**
   *Symptom:* >3 restarts/5m; **escalate to on-call**.
   *Action:* Capture core/logs; roll back to previous tag; open incident & add CHANGELOG entry with root cause.

6. **Tor/Onion Key Loss (optional)**
   *Symptom:* .onion clients can’t connect.
   *Action:* Restore HS key from secure backup; or rotate HS; update config and restart.

---

## 7) Backup / Restore

* **State:** `svc-overlay` is **stateless**. No data backup required beyond configs and optional Tor HS keys.
* **Config backup:** version control `/etc/ron/svc-overlay.toml`; store secrets in vault.
* **Restore:** deploy binary of chosen tag; restore config; start; verify health.

---

## 8) Upgrades

1. Announce drain; set gateway to reduce new sessions.
2. `systemctl stop --no-block svc-overlay` (graceful)
3. Deploy new binary/config; run `ronctl config check`.
4. Start service; verify:

   * `/readyz` stable for 10m
   * No `ServiceCrashed` events
   * Perf canary within ±10% baseline
5. Update **CHANGELOG.md** with perf deltas (if any) and migration notes.

---

## 9) Chaos Testing

**Quarterly (Gate J), per Blueprints: Hardening v1.1; Scaling v1.3.1**

```bash
ronctl chaos inject --target svc-overlay --fault latency  --ms 150 --duration 5m
ronctl chaos inject --target svc-overlay --fault slowloris --conns 50 --duration 10m
```

**Pass criteria:**

* `/readyz` flips appropriately and recovers
* No memory growth post-soak
* Alerts fire within 2m; auto-suppress on recovery

---

## 10) Scaling Notes

* **Vertical:** increase cores for handshake/compression; pin allocator if needed.
* **Horizontal:** add replicas; stateless service—just ensure consistent config & discovery.
* **Knobs:** `peer.inflight_frames`, `peer.send_window_frames`, `peer.max_rps`, `handshake_timeout`, read/write/idle timeouts.
* **When to scale:**

  * `overlay_inflight_frames` > 80% for 10m
  * p95 GET approaching target +10%
  * admin `/metrics` latency >200ms
* **Reference capacity (baseline):** ~**500 rps** per node @ **4 vCPU / 8 GiB** (mixed 64 KiB–1 MiB).

---

## 11) Security Ops

* **Secrets hygiene:** no plaintext secrets in logs; redact by default.
* **Capability rotation:** `ronctl cap rotate --service svc-overlay` (propagate to gateway).
* **TLS/QUIC:** TLS 1.3 only; QUIC 0-RTT **off** unless explicitly approved.
* **PQ mode:** `OVERLAY_PQ_MODE=hybrid` supported; expect ≤30% handshake overhead; document in CHANGELOG when toggled.
* **ECON note:** Overlay holds **no economic state**; quota exhaustions are **audited** via `ron-audit`/`ron-ledger`.
* **Audit trail:** ensure capability mint/revoke events land in `ron-audit`.
* **Amnesia mode (edge):** prefer `amnesia=true` for RAM-only caches; purge timers active.

---

## 12) References

* `CONFIG.md` — knobs, defaults, envs
* `SECURITY.md` — caps, PQ posture, threat model
* `OBSERVABILITY.md` — metrics catalog & dashboards
* `CONCURRENCY.md` — task model, bounded queues
* `PERFORMANCE.md` — SLOs, gates, harness
* `INTEROP.md` — OAP/1 invariants, endpoints
* **Blueprints:** Full Project v2.0 (Perfection Gates), Hardening v1.1, Concurrency & Aliasing, Omnigate, Scaling v1.3.1

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A:** Metrics green (`request_latency_seconds`, `rejected_total`, `bus_*`)
* [ ] **Gate J:** Chaos drill passed (latency & slowloris)
* [ ] **Gate K:** Continuous vigilance (alerts → oncall; log correlation live)
* [ ] **Gate L:** Black swan validated (burst tests, dependency brownouts)
* [ ] **Gate F:** Perf regressions barred (CI baselines within ±10%)
* [ ] **Gate N:** ARM/edge profiled; amnesia mode verified
* [ ] **Gate S:** Security posture verified (TLS1.3, PQ mode noted, caps rotated)

---
