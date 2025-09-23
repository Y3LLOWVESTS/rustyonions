
Save as: `crates/macronode/docs/RUNBOOK.md`

```markdown
---
title: macronode — RUNBOOK
version: 0.4.0
status: draft
last-updated: 2025-09-22
audience: ops, oncall, SRE, release
related: macronode.IDB.md, HARDENING.md, CONCURRENCY.md, PERFORMANCE.md, OBSERVABILITY.md, QUANTUM.md
---

# 0) What this is

**macronode** is the operator-grade, multi-tenant profile of RustyOnions. It composes canonical services (gateway, overlay, registry, naming, storage, index, mailbox, audit, auth, policy, metrics; optional econ: ledger/wallet/rewarder) behind a **capability-first** wall:
**TLS → Auth → Policy → Quota → Handler**, with **OAP/1** framed I/O and strict backpressure.

This runbook is your **day-2 ops bible**: build/run, health, triage, upgrades, and drills—written to match our **IDB invariants** (RES/PERF/GOV), **amnesia mode**, and **ZK/PQ readiness**.

---

# 1) Quickstart (Local, single host)

> Assumes axum 0.7 surfaces on `0.0.0.0:8080` by default; adjust to your config.

## Build
cargo build --release
cargo build --release --features zk,arti
cargo build --release --features amnesia
cargo build --release --features zk,arti,amnesia

## Run (dev TLS off; signed config bundle)
RUST_LOG=info ./target/release/macronode \
  --config ./configs/macronode.bundle.toml

## Health & version
curl -s http://localhost:8080/healthz
curl -s http://localhost:8080/readyz
curl -s http://localhost:8080/version | jq

## Metrics (Prometheus text)
curl -s http://localhost:8080/metrics | head -n 40

## Feature parity check
# Expect flags to reflect the build:
# features.zk, features.amnesia, features.arti → true/false in /version
curl -s http://localhost:8080/version | jq .features

---

# 2) Minimal config (signed bundle) — example

> **Invariant**: config is **signed & versioned**. Every apply emits `KernelEvent::ConfigUpdated{version}` (see journal).

```toml
# macronode.bundle.toml
[meta]
version = 12                  # monotonic
bundle_hash = "blake3:..."    # computed at build
signer = "KMS:key/ops-release"

[server]
bind = "0.0.0.0:8080"
# If TLS terminates here:
# tls = { enabled = true, kms_key = "KMS:key/tls", cert_chain = "KMS:obj/cert_chain" }

[readiness]
required_services = ["svc-gateway","svc-overlay","ron-policy","ron-auth","ron-metrics"]

[oap]
max_frame_bytes = 1048576     # 1 MiB (MUST)
chunk_bytes = 65536           # ~64 KiB (MUST)

[policy]
deny_by_default = true
cache_ttl_ms = 250
bundle_kms = "KMS:obj/policy-bundle@v42"
bundle_hash = "blake3:..."

[zk]                           # ZK seam (optional)
enabled = false               # enable only if /version zk=true
kind = "NoOp"
verifying_key_b64 = ""
param_hash_hex = ""
domain_sep = "domain:policy"

[amnesia]
enabled = false               # true → RAM only caches, zeroize on stop

[econ]                         # optional
enable_ledger = false
````

**Load & verify**

* Start node; tail logs for `ConfigUpdated{version=12}` and bundle hash.
* `curl /version` and confirm `git_sha`, `semver`, feature flags.

---

# 3) Ports & endpoints

| Endpoint   | Purpose                                     | Success Criteria                                      |
| ---------- | ------------------------------------------- | ----------------------------------------------------- |
| `/healthz` | Liveness                                    | HTTP 200 if process alive                             |
| `/readyz`  | Readiness (deps + entropy + /version alive) | HTTP 200 only if **all required** deps healthy        |
| `/metrics` | Prometheus metrics                          | Scrape-safe; histograms, counters, exemplars          |
| `/version` | Build + feature metadata                    | Contains `git_sha`, `semver`, flags {zk,arti,amnesia} |

> **Note**: Readiness must include **entropy ≥ 128 bits** and **/version** parity with binary.

---

# 4) Operational runbooks (common scenarios)

## A) Node won’t become Ready

**Symptoms:** `/readyz` 503, metric `ready_state{state="false"}=1`, logs show missing dependency.
**Checks:**

1. `curl /healthz` → 200?
2. `curl /version` → semver + `git_sha` match rollout; feature flags as expected.
3. Logs: `KernelEvent::Health{ service, ok:false }` for which service?
4. Policy/Auth reachable? DNS/certs OK?
5. Entropy check in readiness details.
   **Fix:**

* Restore failing service or remove it from `[readiness.required_services]` (signed bundle!) and redeploy.
* If config updated: confirm signature + bundle\_hash; restart graceful drain if needed.

## B) 429/503 storms under load

**Symptoms:** p99 latency grows; 429/503% > budget.
**Checks:** Prometheus: `request_latency_seconds`, `service_restarts_total`, `bus_lagged_total`, 429/503 counters.
**Fix:**

* Confirm **bounded mailboxes** and connection caps; adjust within envelopes.
* Enable circuit breakers; shed **facets** first (Search/Feed/Media) per IDB.
* Verify **OAP/1** limits enforced (oversize → 413).

## C) TLS failures

**Checks:**

* We **must** use `tokio_rustls::rustls::ServerConfig`.
* Check KMS material & allowed ciphers; OCSP if enabled.
  **Fix:** rotate keys; correct chain; re-load config; watch `ConfigUpdated`.

## D) Amnesia hygiene

**Checks:**

* `/version.features.amnesia == true`.
* Shutdown → run a disk scan of profile-owned paths → **empty**.
  **Fix:** disable fs caches; ensure zeroization hooks executed; recheck.

## E) ZK seam sanity (if enabled)

**Checks:**

* `/version.features.zk == true`.
* Health detail shows ZK seam smoke test pass.
* KMS has verifier key; `param_hash_hex` matches doc.
  **Fix:** pin VK & params; domain separation tag correct; ensure no witness logs (grep).

## F) Policy cache poisoning / TTL storms

**Checks:**

* Verify policy **signed bundle hash** and signer.
* TTL not too long; cache invalidation on rotate.
  **Fix:** reduce TTL; rotate bundle; prefer deny-secure on backend outage.

---

# 5) Incident response (SEV taxonomy)

**SEV-1:** macronode `/readyz` false > 5m **OR** core p99 SLOs violated > 15m across gateway/overlay.
**SEV-2:** Single facet outage (Search/Feed/Media) without core impact > 30m.
**SEV-3:** Intermittent 429 spikes; degraded but within SLO after retries.

**Escalation order:** oncall-primary → platform → security (if TLS/PQ/ZK) → econ (if ledger/wallet).

**Attach to ticket:**

* `/version` JSON, last 15m `/metrics` snapshot, `git_sha`, config bundle hash, recent `KernelEvent::*`.

---

# 6) Upgrades & rollbacks

**Invariant:** blue/green or rolling with **version-skew tolerance**.

## Rolling upgrade (one node)

1. `curl /version` and record `git_sha` + features.
2. Pre-flight: `curl /readyz` must be 200 for N minutes; error budget OK.
3. Drain: stop accepting new ingress; wait until in-flight <= threshold.
4. Deploy new binary + signed config; start process.
5. Verify `/version` shows new `git_sha`; readiness returns 200.
6. Watch SLOs for 15m; if healthy, finalize.

## Rollback

* Re-deploy last known good binary + config bundle.
* Confirm `/version` reverted; SLOs stable.

---

# 7) Red-team & chaos drills (quarterly)

* **Malformed OAP frames** → expect 413; no meltdown.
* **Quota storms** → policy 429 with `Retry-After`; core stays inside budgets.
* **Policy backend outage** → **fail-secure deny**; `/readyz` false; recover cleanly.
* **Service crash** → median restart < 1s; no lock-across-await deadlocks (loom test parity).
* **Facet outage** → degrade features only; core stays green.
* **ZK bad proof** (if enabled) → verifier rejects; no witness logged; metrics increment “zk\_verify\_failures”.

---

# 8) Observability cheat-sheet

## Key metrics (Prometheus)

* `request_latency_seconds{service=...,route=...}` (Histogram)
* `service_restarts_total{service=...}` (Counter)
* `bus_lagged_total{service=...}` (Counter)
* `zk_verify_success_total` / `zk_verify_fail_total` (optional)
* `ready_state{state="true"|"false"}` (Gauge)

## Handy PromQL

* p99 HTTP:
  `histogram_quantile(0.99, sum by (le, route) (rate(request_latency_seconds_bucket[5m])))`
* 429/503 ratio:
  `sum(rate(http_responses_total{code=~"429|503"}[5m])) / sum(rate(http_responses_total[5m]))`
* Restart spike:
  `increase(service_restarts_total[15m]) > 0`
* Bus lag growth:
  `deriv(bus_lagged_total[10m]) > 0`

---

# 9) SLOs (targets; see IDB §5)

| Surface   | SLI                        | Target               |
| --------- | -------------------------- | -------------------- |
| Gateway   | HTTP p99 latency           | ≤ 120 ms             |
| Overlay   | Hop p99 latency            | ≤ 200 ms             |
| Policy    | Eval p95                   | ≤ 10 ms              |
| Media     | Range start p95 (intra-AZ) | ≤ 100 ms             |
| ZK Verify | Per-proof p95 (batched)    | ≤ 15 ms (if enabled) |

**Error budgets**: 99.9% availability monthly for core surfaces.

---

# 10) Security & hardening (ops sidebars)

* **TLS** via `tokio_rustls::rustls::ServerConfig` only; approved ciphers; staple OCSP if supported.
* **OAP/1**: `max_frame=1 MiB`, chunks≈64 KiB; reject oversize (413).
* **Decompression**: ≤10× expansion **and** absolute caps.
* **Policy**: deny-by-default; signed bundles; bounded TTL; rotate safely.
* **Audit**: append-only, hash-chained; Merkle checkpoints; no PII.
* **Amnesia**: RAM-only caches; zeroize on stop; disk empty after shutdown.
* **ZK**: verifier keys from KMS; domain separation tags; **no witness logs**.
* **Deny-drift**: overlay has **no DHT** logic; crate canon enforced in CI.

---

# 11) Day-1 deployment recipes

## A) systemd (single host)

Create `/etc/systemd/system/macronode.service`:

```
[Unit]
Description=RustyOnions Macronode
After=network-online.target
Wants=network-online.target

[Service]
User=macronode
Group=macronode
Environment=RUST_LOG=info
ExecStart=/opt/macronode/macronode --config /opt/macronode/macronode.bundle.toml
Restart=on-failure
RestartSec=1
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
```

systemctl daemon-reload
systemctl enable --now macronode
journalctl -u macronode -f

## B) Container (docker/podman)

docker run --rm -p 8080:8080&#x20;
-v \$PWD/configs\:/configs\:ro&#x20;
ghcr.io/rustyonions/macronode\:latest&#x20;
\--config /configs/macronode.bundle.toml

## C) Kubernetes probes (snippet)

```yaml
livenessProbe:
  httpGet: { path: /healthz, port: 8080 }
  initialDelaySeconds: 5
  periodSeconds: 10
readinessProbe:
  httpGet: { path: /readyz, port: 8080 }
  initialDelaySeconds: 5
  periodSeconds: 10
```

---

# 12) Verification checklist (pre-prod)

* Build matrix passes:

  * `cargo build --release`
  * `cargo build --features zk,arti`
  * `cargo build --features amnesia`
  * `cargo build --features zk,arti,amnesia`
* `/version` reflects feature flags correctly.
* `/metrics` scrape passes Prometheus lint; cardinality within budget.
* `/readyz` false when a required dep is down; true on recovery.
* OAP limits enforced (`413` on oversize).
* Chaos drill: crash & restart `< 1s` median.
* Amnesia: disk is clean post-shutdown; secrets zeroized.
* ZK seam (if enabled): verifier self-test passes; no witness logs.

---

# 13) Appendix — Useful one-liners

## Tail recent errors

journalctl -u macronode -S -15min -p err

## Top endpoints by p99 (requires route label)

curl -s localhost:8080/metrics | grep request\_latency\_seconds\_bucket | wc -l

## Sanity: 413 on oversize

python - <<'PY'
import socket,os
s=socket.create\_connection(("127.0.0.1",8080))
payload=b"X"*(1024*1024+1)
s.sendall(b"POST /ingress HTTP/1.1\r\nHost: x\r\nContent-Length: "+str(len(payload)).encode()+b"\r\n\r\n"+payload)
print(s.recv(1024))
PY

## Confirm amnesia (no residue)

find /var/lib/rustyonions/macronode -maxdepth 2 -type f | wc -l

```

