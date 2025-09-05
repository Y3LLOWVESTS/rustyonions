# RustyOnions — Canonical TODO & Roadmap 
_Last updated: 2025-09-04_

> Single source of truth for near‑term execution. Consolidates **Full_Project_Blueprint**, **Omnigate_Blueprint**, **Microkernel_Blueprint**, **Scaling_Blueprint**, **Interop_Blueprint**, the current **README** and **TODO (old)**.  
> **v3 adds:** explicit doc‑wide terminology enforcement and CI checks for terminology and README “Project Structure”.

---

## 0) Progress Snapshot (now)
- ✅ **Kernel invariants locked**: public API frozen; **monomorphic `Bus`**; **`tokio_rustls::rustls::ServerConfig`**; Axum 0.7 handlers; unsafe‑free.  
- ✅ **Observability**: `/healthz`, `/readyz`, `/metrics`; overload returns **429/503**; backpressure wired.  
- ✅ **Omnigate path stood up**: TLS listener, **OAP/1 HELLO**, Storage **GET** (64 KiB streaming chunks), Mailbox MVP (SEND/RECV/ACK, idempotency), quotas demo.  
- 🔶 **SDK**: Rust client demos exist; **env + retry polish** pending.  
- 🔶 **Docs**: Blueprints unified; **in‑repo OAP/1 stub** pending.  
- ℹ️ **Vest readiness**: Bronze close; Silver after Storage read‑path & SDK DX.

---

## 1) Critical Alignments (drift killers)
- [ ] **Hashing:** Eliminate SHA‑256; only **BLAKE3‑256** addressing `b3:<hex>`. During migration: **dual‑hash read / BLAKE3‑only write**.  
- [ ] **OAP/1 limits:** Protocol **`max_frame = 1 MiB`**; implementation detail: **storage streaming chunk = 64 KiB**.  
- [ ] **Kernel API (frozen)** re‑exports remain: `Bus`, `KernelEvent::{Health, ConfigUpdated, ServiceCrashed{reason}, Shutdown}`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.  
- [ ] **Service observability:** each `ServiceCrashed{..., reason}` increments `rejected_total{reason}`; structured logs carry `reason`.  
- [ ] **No app logic in kernel.**

**DoD:** CI greps pass; conformance tests assert `max_frame` vs chunking; `ServiceCrashed{reason}` visible in metrics/logs.

---

## 2) Immediate Priorities — “Pilot‑Ready Core” (A‑series)
> Locks safety/contract; targets Vest‑ready Bronze

### A1. Compression guard‑rails → **413**
- Enforce **decompressed byte cap** and **compression ratio cap** (e.g., ≤10:1).  
- Emit metrics `rejected_total{reason="decompress_cap"|"ratio_cap"}`.

**DoD:** Red‑team corpus (zstd bomb) rejected with **413**; metrics increment; no memory spikes.

### A2. Error taxonomy + JSON envelope (+ `corr_id`)
- Map **400/404/413/429/503**; add **Retry‑After** on 429/503; echo **`corr_id`**.  
- Typed error body: `{ "code": "...", "message": "...", "retryable": true|false, "corr_id": "..." }`.

**DoD:** Golden tests per path; SDK parses typed errors; `/metrics` labeled by `code`.

### A3. SDK retries + env polish
- Respect **Retry‑After**; bounded jitter; envs: **`RON_NODE_URL`**, **`RON_CAP`**; propagate `corr_id` end‑to‑end.

**DoD:** Quota demo passes with retries; examples succeed under forced 429/503.

---

## 3) Omnigate Rings (Bronze / Silver / Gold)

### M1 — **Bronze (“Hello Any App”)**
- [ ] **Spec stub** `/docs/specs/OAP‑1.md` mirroring GMI‑1.6 (no drift) + **two hex vectors**.  
- [ ] **Overlay/OAP** frame parser + bounds + **fuzz/property tests**.  
- [ ] **Gateway** `/readyz` capacity gating + **per‑tenant token buckets**.  
- [ ] **Mailbox MVP**: SEND/RECV/ACK/DELETE/SUBSCRIBE; at‑least‑once; **ULID**; **idempotency key**.  
- [ ] **Metrics (golden)**: `requests_total`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`, `quota_exhaustions_total`, `bus_overflow_dropped_total`.  
- [ ] **Red‑team suite**: malformed frames, slow‑loris, partial writes, quota storms, compression bombs.

**DoD:** Echo+Mailbox E2E; overload → **429/503** with **Retry‑After**; metrics complete; fuzz hours clean.

### M2 — **Silver (“Useful Substrate”)**
- [ ] **Storage/DHT read‑path**: `GET`, `HAS`, **64 KiB streaming**; tileserver example.  
- [ ] **Mailbox polish**: ACK levels, visibility timeout, DLQ.  
- [ ] **SDK DX**: env keys, `corr_id` tracing, friendly errors.  
- [ ] **Capability rotation v1**: `ronctl cap mint/rotate`; HELLO advertises revocation sequence.

**DoD:** Vest demo: tiles + mailbox over OAP with backpressure; intra‑AZ latency targets met.

### M3 — **Gold (“Ops‑Ready & Smooth Growth”)**
- [ ] Parser **proptests** in CI; **persist fuzz corpus** & replay.  
- [ ] **Leakage harness v1** (padding/jitter toggles) + docs.  
- [ ] **Registry/Governance**: `docs/GOVERNANCE.md`; SLA & appeal path; JSON mirror + schema CI; **signed banlist exchange** (cosign).  
- [ ] **TLA+ service models**: `specs/mailbox.tla`, `specs/rewarder.tla`; TLC checks in CI.  
- [ ] **Performance simulation**: `testing/performance/` OAP/1 + DHT sim (1k+ nodes). Targets: **p95 < 40 ms**, **p99 < 120 ms** under configured RF.

**DoD:** Multi‑tenant load test stable; TLC green; simulation meets SLOs; new dev integrates in < 30 min.

---

## 4) Platform (B‑series — Backbone)
- [ ] **B1. `ron-proto` crate**: OAP constants, status codes, headers, canonical vectors (single source).  
- [ ] **B2. Overlay Alpha (`svc-overlay`)**: BLAKE3 GET/PUT; verify on read; latency histograms.  
- [ ] **B3. Index Alpha (`svc-index`)**: Namespace→addr map; small federation; resolver API.  
- [ ] **B4. Privacy option (`arti_transport`)**: Tor/Arti transport flag; onion round‑trip smoke test in CI (optional job).  
- [ ] **B5. Accounting v1**: Persistent counters; **DRR** policy quotas; `/readyz` reflects policy (hot reload).  
- [ ] **B6. Rewards hooks**: BLAKE3 **`counters_hash`**; audit trail; ROX/ROC metrics; prep proof‑of‑service.  
- [ ] **B7. Erasure Coding**: Reed–Solomon parity; **repair pacing ≤ 50 MiB/s** per cluster; prioritize hottest content.  
- [ ] **B8. Micronode Offline Sync**: `testing/test_offline_sync.sh` with mock‑mailbox; intermittent connectivity converge.  
- [ ] **B9. Micronode Architecture Spec**: `docs/micronode.md` — roles, manifest, attribution, cache policy, privacy toggles, offline reconciliation, safety limits; **cross‑linked from README**.

**DoD:** Soak tiles + mailbox; RF & latency SLOs; offline write→sync→converge; micronode doc reviewed & referenced by README.

---

## 5) Docs — README Alignment (explicit tasks)
- [ ] **Replace SHA‑256 with BLAKE3‑256** in `README.md` **system diagram** and text.  
  **DoD:** Diagram/text say “content‑addressed bytes, **BLAKE3‑256** verified”; CI grep finds no `sh""a-?256|sh""a256:` in README.  
- [ ] **Standardize two‑plane terminology** in `README.md` **and across all docs** to **Public Plane** (content) and **Private Plane** (messaging/Tor); **remove legacy terms**.  
  **DoD:** Grep for `Overlay Plane` or `Private Message Plane` returns **no matches** in README or `docs/`.  
- [ ] **Add high‑level crate structure** to `README.md` (nine services + core libraries).  
  **DoD:** “Project Structure” lists crates with 1–2‑line summaries; links to per‑crate READMEs.  
- [ ] **Map progress to M1/M2/M3 + Perfection Gates A–O** in README “Status”.  
  **DoD:** Snapshot shows current ring and gate readiness; kept in sync with this TODO.

---

## 6) Migration & Alignment
- [ ] **SHA‑256 → BLAKE3** epoch: dual‑hash read / BLAKE3 write; DHT re‑announce `b3:<hex>`; **410 Gone** for SHA‑256 endpoints post‑window.  
- [ ] **OAP/1 `max_frame = 1 MiB`** everywhere; storage chunk stays **64 KiB**.  
- [ ] **Sanity greps** added to CI pipelines (see §17).

---

## 7) Security & Privacy
- [ ] **Capabilities**: Ed25519 + macaroons v1; TTL/audience/method caveats; ≤30‑day rotation; HELLO revocation probe.  
- [ ] **APP_E2E**: Opaque payloads; redact logs; reject oversize **after decompress**.  
- [ ] **DoS**: per‑conn `max_inflight`, timely ACK; Gateway choke‑point; 429/503 discipline.  
- [ ] **Amnesia mode**: RAM‑only logs; zeroize secrets; optional seccomp profile.  
- [ ] **ZK hooks (feature‑gated; service‑layer)**: commit‑only M1 → commitments/proofs M2 → harden M3; metrics `zk_verify_total`, `zk_verify_failures_total`.

**DoD:** Red‑team suite green; rotation drills succeed; leakage harness report attached.

---

## 8) Observability, SLOs & Runbooks
- [ ] **Metrics**: counters/histograms/gauges per blueprint (latency, inflight, rejected, quotas, DHT lookups, EC repair).  
- [ ] **SLOs**: **p50 < 10 ms**, **p95 < 40 ms**, **p99 < 120 ms** (intra‑AZ; dev box varies).  
- [ ] **Tracing**: `corr_id` propagation; OTel sampling 0.1% steady / ≥5% during incidents.  
- [ ] **Runbooks**: overload, disk‑full, cert‑rotate, rollback, partitions; **DHT failover** & **cross‑region placement** → `docs/runbooks.md`.  
- [ ] **Leakage harness**: measure timing/size correlation; padding/jitter toggles documented.

**DoD:** Dashboards show p50/p95/p99 and RF gauges; operators can restore RF in ≤ 60 min using runbooks.

---

## 9) CI/CD & Supply Chain Gates
- [ ] Workflows: build/test (**nextest**), **llvm‑cov ≥85%**, miri, loom, fuzz (corpus reuse), **TLA+ TLC**, cargo‑deny/audit, SBOM (CycloneDX/Syft), cosign sign.  
- [ ] **Performance sim job**: run OAP/1 + DHT simulation; **fail CI if SLOs violated**.  
- [ ] **Release gates**: block unless protocol tests, fuzz/property, chaos basic, and SLO burn‑rate alarms are green.  
- [ ] **Docs/DX**: Quickstart, SDK Guide, GOVERNANCE, Registry, example walkthroughs; new dev integrates **< 30 min**.

---

## 10) Scripts — Enhancements
- [ ] `scripts/run_quota_demo.sh`: parameterize limits & duration; assert expected 429 band; emit JSON.  
- [ ] `scripts/run_tile_demo.sh`: emit JSON summary; snapshot `/metrics` post‑run.  
- [ ] `scripts/run_mailbox_demo.sh`: emit JSON with `msg_id`s + ack status; optional injected retry.  
- [ ] **`testing/test_offline_sync.sh`**: parameterize mock‑mailbox flow; emit JSON summary (`msg_id`, sync status, retries).  
  **DoD:** Script passes with `testing/mock-mailbox`; JSON output archived by CI; visible in badges/logs.

---

## 11) Repo Hygiene
- [ ] **CRATE_INDEX.md**: role per crate; 1–2‑line summary.  
- [ ] **Per‑crate READMEs** from template in `docs/templates/`.  
- [ ] **`xtask/`**: port heavier shell to Rust progressively; JSON outputs for CI.  
- [ ] Workspace: set `[workspace] default-members` to `svc-omnigate`, `ron-app-sdk` for fast iteration.

---

## 12) Vest Pilot — Definition of Ready / Done
- **Ready:** Bronze complete; echo + mailbox soak pass; quotas & readiness verified; docs live.  
- **Done:** Vest pulls **tiles via OAP** and runs **E2E mailbox chat**; under overload receives **429/503** (no hangs); **APP_E2E** shows no plaintext leaks.

---

## 13) Nice‑to‑Have (defer if risky)
- Polyglot clients (TS/Go/Python/Swift) — spec remains OAP/1.  
- Interop adapters (IPFS/libp2p, S3‑style GET).  
- Placement service + hedged GETs; nightly `ronctl rebalance`.

---

## 14) Owner Map (suggested)
- **Core:** OAP/1 spec stub, overlay/protocol, SDK, examples.  
- **Services:** Mailbox, Storage (read), Index.  
- **Ops:** Gateway quotas/readiness, metrics/alerts.  
- **Security:** Caps, fuzz/property tests, red‑team suite, leakage harness.  
- **Docs/DX:** Specs, SDK guides, registry process, `ronctl` UX, runbooks, **micronode spec**, README alignment.

---

## 15) Command Cheat‑sheet (dev box)
```bash
# Core spec + tests
cargo test -p oap
cargo test -p ron-kernel --tests

# Demos
cargo run -p gateway --example demo
cargo run -p gateway --example tcp_demo

# Soak & quota
N=80 P=80 bash scripts/run_quota_demo.sh
bash scripts/run_tile_demo.sh
bash scripts/run_mailbox_demo.sh

# Offline sync (once implemented)
bash testing/test_offline_sync.sh --duration 120 --jitter 200ms --loss 0.5%

# Performance sim (once implemented)
cargo run -p testing --bin oap_dht_sim -- --nodes 1000 --rf 3 --duration 300s
```

---

## 16) Acceptance Gate Reminders (release blockers)
- Unsafe‑free kernel; stable public API & TLS type; Axum 0.7 `.into_response()`; bounded queues; **429/503 + Retry‑After**.  
- Observability complete; security validated (caps, rotation, amnesia).  
- Formal/destructive validation (**proptest, fuzz, TLA+, loom, chaos**).  
- Supply chain clean; SBOM & signatures; runbooks present; governance doc live.

---

## 17) CI Sanity Greps (copy into CI)
```bash
# Kill SHA‑256 anywhere in docs/code
rg -n "sh""a-?256|sh""a256:" README.md docs/ *.md crates/ -S

# Ensure BLAKE3 addressing appears
rg -n "b3:" -S

# Protocol vs storage chunk confusion
rg -n "max_frame\\s*=\\s*64\\s*Ki?B" -S

# Enforce doc‑wide terminology (no legacy terms)
rg -n "Overlay Plane|Private Message Plane" README.md docs/ *.md -S

# README must include a Project Structure with at least these crates (proxy check)
rg -n "Project Structure" README.md -n
rg -n "ron-kernel" README.md -n
rg -n "svc-omnigate|svc-overlay|svc-index|svc-mailbox" README.md -n
```
