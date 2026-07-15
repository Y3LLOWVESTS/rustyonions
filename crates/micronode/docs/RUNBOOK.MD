

---

title: RUNBOOK — micronode
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-09-23
audience: operators, SRE, auditors

---

# 🛠️ RUNBOOK — micronode

## BUILD_PLAN_Z Phase 24 private-beta posture

This section controls the current CrabLink private-beta product posture.

Authoritative product guide:

```text
docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md
```

The current `micronode` product role is the **CrabLink User Node**:

```text
passive background verifier
managed by the CrabLink desktop application
loopback-only
amnesia-first
resource bounded
non-serving by default
pauseable
private by default
```

The CrabLink desktop owns the User Node lifecycle. A normal user does not
manage ports, public listeners, routes, RPC URLs, server logs, or admin
dashboards.

The supported local inspection commands are:

```bash
cargo run -p micronode -- check
cargo run -p micronode -- status
cargo run -p micronode -- --help
```

Direct runtime use is a development and diagnostic surface:

```bash
cargo run -p micronode -- serve
```

Direct `serve` use does not convert the User Node into a public Service Node.

The User Node must not:

```text
bind public inbound listeners
serve public content by default
publish the user's residential IP
invent provider addresses
mint ROC
approve its own rewards
mutate a wallet
mutate the ledger
invent receipts
invent confirmed ROC
claim finality
```

A paused User Node rejects new verification work without consuming replay
identity for the rejected request. Read-only pending evidence may remain
visible.

Pending evidence is not a reward. A pending reward plan is not a balance.
Confirmed ROC appears only after backend-derived wallet and ledger receipt
truth.

Older material below remains an engineering reference. Any older description
of `micronode` as a public object, mailbox, edge-server, or general deployment
profile does not override this private-beta User Node posture.


## 0) Purpose

Operational manual for `micronode`: startup, health, diagnostics, failure modes, recovery, scaling, and security ops.
This document satisfies **PERFECTION\_GATES** K (Continuous Vigilance) and L (Black Swan Economics).

---

## 1) Overview

* **Name:** `micronode`
* **Role:** DX-lean **node profile** (single-tenant), exposes admin + minimal object/mailbox surface; **amnesia ON by default** (RAM-only) &#x20;
* **Criticality Tier:** 2 (supporting; used for edge/dev or small single-tenant deployments)
* **Dependencies (feature-dependent):** kernel/bus, config, storage/index (optional), mailbox (optional), KMS (keys/caps)&#x20;
* **Ports Exposed:** Admin HTTP binds to `--http-addr` (default `127.0.0.1:8080`); optional `--metrics-addr` if separate; TLS via rustls if enabled &#x20;
* **Data Flows:** HTTP/OAP ingress with strict limits; DRR/backpressure; bus events (`Health`, `ServiceCrashed`, `Shutdown`) out &#x20;
* **Version Constraints:** uses kernel contracts frozen by canon (KernelEvent etc.); TLS type **must** be `tokio_rustls::rustls::ServerConfig`&#x20;

---

## 2) Startup / Shutdown

### Startup

```bash
# Local (dev)
cargo run -p micronode -- --config ./configs/micronode.toml

# Production-style
./target/release/micronode --config /etc/ron/micronode.toml
```

**Env/flags (common):**

* `RON_CONFIG` (path), `RON_HTTP_ADDR`, `RON_METRICS_ADDR`, `RON_LOG`, `RON_AMNESIA` (+ TLS flags)&#x20;

**Verification**

* Logs show readiness transition; `/readyz` returns **200** once config/deps bound&#x20;

```bash
curl -sS http://127.0.0.1:8080/readyz | jq .
```

### Shutdown

* Ctrl-C (SIGINT) triggers graceful drain; kernel emits `Shutdown`; tasks drain then abort at deadline (3–5s)&#x20;
* systemd: `systemctl stop micronode`

---

## 3) Health & Readiness

* **`/healthz`** = process alive (always **200**)&#x20;
* **`/readyz`** = fully serving once config loaded, listeners bound, deps OK; **degrade before NotReady** under stress &#x20;
* **Hardened defaults** (5s/512/500rps/1 MiB/≤10×) are enforced by the hardening layer across all services&#x20;

**If not ready after 10s**

* Check bus for `ServiceCrashed{reason}`; readiness should **fail early** under restart storms by design&#x20;
* Inspect `bus_lagged_total`, `service_restarts_total` for churn/overflow&#x20;

---

## 4) Common Failure Modes

| Symptom                       | Likely Cause                     | Metric / Log                                       | Resolution                                                 | Alert Threshold                                            |          |
| ----------------------------- | -------------------------------- | -------------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------- | -------- |
| 503 on ingress (`/put`/`/o/`) | Quota exhausted / degraded ready | \`rejected\_total{reason="429"                     | "503"}\`                                                   | Throttle/raise quota; verify `/readyz` deps; shed non-core | >10/min  |
| Slow p95 (>100 ms)            | Bus lag/backpressure             | `bus_lagged_total`, latency histogram              | Scale workers/replicas; tune caps; verify DRR              | p95 >100 ms                                                |          |
| Frequent restarts             | Crash loop                       | `ServiceCrashed{reason}`; `service_restarts_total` | Inspect last panic; enable backoff; fix hot path           | >5/5m                                                      |          |
| 413 on `/put`                 | Body/frame too large             | Logs: refusal; `rejected_total{reason="413"}`      | Use **64 KiB** streaming; keep non-stream frames ≤1 MiB    | any                                                        |          |
| 502 on GET `/o/{addr}`        | Integrity failure on read        | `integrity_fail_total`                             | Re-replicate/repair object; verify BLAKE3 end-to-end       | any                                                        |          |
| TLS handshake errors          | Bad cert/chain/ciphers           | handshake failures, kernel TLS type check          | Rotate keys; ensure **rustls ServerConfig**; reload config | any                                                        |          |

---

## 5) Diagnostics

**Logs**

```bash
journalctl -u micronode -f | grep -E "corr_id|reason|ServiceCrashed"
```

**Metrics**

```bash
curl -s http://127.0.0.1:8080/metrics | grep -E "request_latency_seconds|rejected_total|bus_lagged_total"
```

**Bus events**

```bash
ronctl tail --topic micronode
```

**Tracing / Perf**

```bash
RUST_LOG=debug ./target/release/micronode --config /etc/ron/micronode.toml
cargo flamegraph -p micronode
```

Admin surface & media types are standard (JSON for health/version; Prometheus text for `/metrics`)&#x20;

---

## 6) Recovery Procedures

1. **Config Drift**

   * **Symptom:** Rejections on valid requests or wrong limits
   * **Action:** Validate/print effective config; reload
     `micronode config validate <PATH>` / `micronode config print` (if available); precedence is flags > env > file > defaults &#x20;

2. **Data Corruption (when storage/index enabled)**

   * **Symptom:** Object unreadable; integrity fails (502)
   * **Action:** Trigger re-replication/repair; verify BLAKE3; ensure UDS + peer-cred; integrity metric increments on failure&#x20;

3. **Key/TLS Issues**

   * **Symptom:** TLS failures on connect
   * **Action:** Rotate keys, check KMS material & ciphers, ensure kernel uses rustls ServerConfig; reload config; watch `ConfigUpdated`&#x20;

4. **Overload**

   * **Symptom:** CPU pegged, rising p95, rising rejects
   * **Action:** Degrade before NotReady; increase replicas; tighten per-route caps; confirm DRR/queue depths; limits remain 5s/512/500/1 MiB/≤10× &#x20;

5. **Rollback Upgrade**

   * **Action:** Follow rolling upgrade/rollback checklist (record `/version`, drain, deploy, verify, watch SLOs; revert if needed)&#x20;

---

## 7) Backup / Restore

`micronode` is **stateless by default** (amnesia mode). If persistence features are enabled (e.g., local storage/index):

* Snapshot/restore the data dir with service **stopped**; prefer UDS with `0700/0600` and peer-cred when restoring and reattaching&#x20;

---

## 8) Upgrades

* Drain new intake, wait for inflight to settle, deploy binary + signed config, verify `/version` and `/readyz` remain green for ≥10–15m, then roll to next instance; **version-skew tolerant** by design&#x20;

---

## 9) Chaos Testing

* Run fault injection (latency/oversize frames/quota storms); expect **413** on 2 MiB frames, decompression refusals with `rejected_total{reason="decompress"}`, policy outage → **fail-secure** + `/readyz` false; quarterly drill required &#x20;

---

## 10) Scaling Notes

* **Vertical:** adjust worker threads and hashing/compression pools
* **Horizontal:** add replicas behind gateway; stateless profile simplifies scaling
* **When to scale:** sustained `inflight_requests > ~80%` capacity; queue depth rising; bus lag non-zero &#x20;

---

## 11) Security Ops

* **No plaintext secrets** in logs; redaction enforced
* **Rotate caps** via passport/KMS; short TTL; capability-only access
* **PQ readiness:** hybrid posture allowed; expose in `/version`
* **Amnesia hygiene:** `/version.features.amnesia == true`; **no residual files** post-shutdown (run disk scan) &#x20;

---

## 12) References

* [CONFIG.md](./CONFIG.md)
* [SECURITY.md](./SECURITY.md)
* [OBSERVABILITY.md](./OBSERVABILITY.md)
* [CONCURRENCY.md](./CONCURRENCY.md)
* [TESTS.md](./TESTS.md)
* Blueprints: [Hardening](../../docs/HARDENING_BLUEPRINT.MD) , [Microkernel](../../docs/MICROKERNEL_BLUEPRINT.MD) , [Interop](../../docs/INTEROP_BLUEPRINT.MD)&#x20;

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A:** Metrics green (`request_latency_seconds`, `http_requests_total`, `rejected_total`)&#x20;
* [ ] **Gate J:** Chaos drill passed (oversize frames, quota storms, policy outage)&#x20;
* [ ] **Gate K:** Continuous vigilance (logs structured, alerts wired, `/readyz` degrade semantics)&#x20;
* [ ] **Gate N:** ARM/edge perf profiled (cold start <2s / power budget)&#x20;
* [ ] **Gate O:** Security audit clean (cap-only, DTO `deny_unknown_fields`, UDS perms, amnesia honored)&#x20;

---

### Operator crib (quick)

* **413 on `/put`?** Switch to **64 KiB streaming**; keep non-stream frame ≤1 MiB.&#x20;
* **p95 up, 429 rising?** Confirm `/readyz` degrades before full NotReady; scale horizontally; verify DRR/queue metrics.&#x20;
* **TLS flapping?** Use rustls ServerConfig; rotate keys; check KMS/chain; reload.&#x20;

---
