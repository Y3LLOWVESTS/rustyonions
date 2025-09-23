

# RustyOnions — THE PERFECT FINAL BLUEPRINT (v2.0)

*Generated: 2025-09-21*

This document **unifies and supersedes** v1.0 and remains the **single source of truth** for architecture, acceptance gates, SLOs, and the delivery plan. All future changes land as PRs against this file with matching CI gate updates.&#x20;

---

## 0) Executive Summary

* Keep the kernel **tiny, safe, and stable** — one crate (`ron-kernel`) exporting the frozen public API. Services live **outside** the kernel boundary and may crash/restart independently.&#x20;
* Drive everything with **capabilities**, **content addressing**, and **end-to-end encryption**. Treat failure as a **first-class input** via supervision and backpressure.&#x20;
* Ship with **observability**, **supply-chain integrity**, and **formal/destructive validation** baked in. We only ship when **Perfection Gates** are green.&#x20;
* The architecture is organized by **12 Pillars** and a fixed **33-crate** canon. All blueprints and CI checks are aligned to this structure.&#x20;

### 0.1) Canon Deltas (refactor updates)

* **Overlay/DHT split:** `svc-dht` now owns Kademlia/Discv5; `svc-overlay` handles sessions/gossip only (no DHT logic).&#x20;
* **Transport merge:** `svc-arti-transport` is **folded into `ron-transport`** under an `arti` feature (library, not service).
* **Naming cleanup:** `tldctl` folded into **`ron-naming`** (schemas/types); runtime lookups live in `svc-index`.
* **Profiles:** `macronode` (full mesh) and `micronode` (amnesia-first, RAM-heavy defaults) are first-class node compositions.&#x20;
* **Six Concerns spine:** SEC / RES / PERF / ECON / DX / GOV → cross-cutting invariants with CI hooks and PR labels.&#x20;

---

## 1) Non-Negotiable Principles

* **Secure by default:** *keys not accounts*, mTLS/Noise, least-privilege capabilities, zero-trust boundaries.
* **Crash-only + supervision:** processes die fast, restart fast, never corrupt state.
* **Backpressure first:** every boundary enforces limits; **no unbounded queues**.
* **Deterministic interfaces:** strict schemas, stable re-exports, versioned messages.
* **Content addressing:** integrity by hash; idempotency everywhere.
* **Telemetry is a feature:** `/healthz`, `/readyz`, `/metrics`, tracing baked in.
* **Reproducible builds + SBOM:** release equals proof.
* **Chaos-ready:** partitions, latency spikes, injected panics are test modes.
* **Erase by design:** **amnesia mode** with zeroization and RAM-only logs.&#x20;

---

## 2) Invariants (source of truth)

* **Addressing:** `b3:<hex>` = **BLAKE3-256** of the **unencrypted** object or manifest root; truncated prefixes MAY route; **full digest MUST verify** before serving.
* **OAP/1 defaults:** `max_frame = 1 MiB`; **64 KiB** is a **streaming chunk** detail (do not conflate with frame size).
* **Kernel public API (frozen):** `Bus`, `KernelEvent::{ Health{service,ok}, ConfigUpdated{version}, ServiceCrashed{service,reason}, Shutdown }`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* **TLS type:** `tokio_rustls::rustls::ServerConfig`.
* **Normative spec pointer:** OAP/1 defined by **GMI-1.6**; any local copy is a **mirror stub** to avoid drift.&#x20;

---

## 3) Kernel (the Diamond)

**Scope:** Bus, Events, Metrics/Health, Transport, Config, Capabilities, Supervisor hooks, Shutdown helper.&#x20;

### 3.1 Public API (frozen)

```rust
pub use { Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c };
```

```rust
enum KernelEvent { 
  Health{ service: String, ok: bool },
  ConfigUpdated{ version: u64 },
  ServiceCrashed{ service: String, reason: String },
  Shutdown
}
```

**Breaking change policy:** semantic changes require a **major** bump plus migration notes.&#x20;

### 3.2 Bus & IPC

* **Events:** lossy broadcast (`tokio::sync::broadcast`) with **bounded capacity** (default: 4096). On overflow: increment `bus_overflow_dropped_total`, emit a throttled crash event, **never block** kernel critical paths.
* **Commands:** per-service bounded `mpsc` (default: 128) + `oneshot` for req/rep.&#x20;

### 3.3 Transport

* Acceptors for HTTP/2 + WebSocket; QUIC optional via features.
* **TLS:** Rustls via `tokio_rustls`; timeouts and per-peer ceilings exported to metrics.
* **Arti/Tor:** implemented as a **feature of `ron-transport`**, not a service.&#x20;

### 3.4 Metrics & Health

`/metrics`, `/healthz`, `/readyz` everywhere; `/readyz` **fail-open reads / fail-closed writes** during incidents; golden histograms/gauges standardized.&#x20;

### 3.5 Config & Hot-Reload

Validated snapshots; on validation failure, **automatic rollback**; emits `ConfigUpdated`.&#x20;

### 3.6 Capabilities & Secrets

Macaroon-style capabilities; zeroize on drop; sealed storage; rotation ≤ 30d; amnesia disables persistence and redirects logs to RAM.&#x20;

### 3.7 Supervisor

Exponential backoff (100ms→30s) with jitter; restart intensity cap ≤ 5/60s; quarantine flappers; publish `ServiceCrashed` with reason.&#x20;

---

## 4) Pillars & Canon (12 Pillars, 33 crates)

**This section is normative.** A crate must appear **exactly once**. Libs (`ron-*`) are pure (no network side-effects); services (`svc-*`) are runtime actors.&#x20;

* **P1 Kernel & Orchestration:** `ron-kernel`, `ron-bus`, `ryker`.
* **P2 Policy & Governance:** `ron-policy`, `svc-registry`.
* **P3 Identity & Keys:** `ron-kms`, `ron-auth`, `svc-passport`.
* **P4 Audit & Compliance:** `ron-audit`.
* **P5 Observability:** `ron-metrics`.
* **P6 Ingress & Edge:** `svc-gateway`, `omnigate`, `svc-edge`.
* **P7 App BFF & SDK:** `ron-app-sdk`, `oap`, `ron-proto`.
* **P8 Node Profiles:** `macronode`, `micronode`.
* **P9 Content Addressing & Naming:** `ron-naming`, `svc-index`.
* **P10 Overlay, Transport & Discovery:** `svc-overlay`, `ron-transport`, `svc-dht`.
* **P11 Messaging & Extensions:** `svc-mailbox`, `svc-mod`, `svc-sandbox`.
* **P12 Economics & Wallets:** `ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`, `svc-ads`, `svc-interop`.&#x20;

> The **canonical list (33)** is:
> `macronode, micronode, oap, omnigate, ron-accounting, ron-app-sdk, ron-audit, ron-auth, ron-bus, ron-kernel, ron-kms, ron-ledger, ron-metrics, ron-naming, ron-policy, ron-proto, ron-transport, ryker, svc-ads, svc-dht, svc-edge, svc-gateway, svc-index, svc-interop, svc-mailbox, svc-mod, svc-overlay, svc-passport, svc-registry, svc-rewarder, svc-sandbox, svc-storage, svc-wallet` (no extras; no renames).&#x20;

---

## 5) Data Plane & Formats

* **Content addressing:** `b3:<hex>` everywhere; services MUST verify full digest before serving.
* **OAP/1 envelopes:** 1 MiB frame limit; stream data in 64 KiB chunks; explicit error taxonomy; HELLO negotiation.
* **DTO hygiene:** `ron-proto` uses pure types with `#[serde(deny_unknown_fields)]`; schema compatibility tests in CI.
* **Interop/federation:** allow-lists; `svc-registry` signed descriptors; capability translation only (no ambient trust).&#x20;

---

## 6) Concurrency & Backpressure

* **Never** hold locks across `.await`.
* **Single-writer** per DHT k-bucket; **bounded** mailboxes (Ryker); **bounded** bus with overflow counters.
* Deadline/`JoinSet` per request; semaphores at dials, inbound, CPU crypto/hashing; zero-copy hot paths (`Bytes`).&#x20;

---

## 7) Security, Privacy & Amnesia

* **Capabilities only** (macaroons or equivalent); short TTL; revocation paths.
* **Key custody** in `ron-kms` with zeroization and sealed storage; **PQ-hybrid readiness** (X25519+Kyber, etc.).
* **Amnesia mode** (kernel flag honored by all services): RAM-only caches, ephemeral logs, timed purge of keys, **no disk spill** in micronode.

---

## 8) Observability

* **Golden metrics** everywhere (latency histograms, error counters, saturation gauges) and uniform `/metrics`, `/healthz`, `/readyz`, `/version`.
* Readiness degrades **before** collapse; DRR queues and backpressure are observable.&#x20;

---

## 9) Economic Architecture

* **Counters vs Truth:** `ron-accounting` (transient) → `ron-ledger` (append-only truth) → `svc-rewarder` (distribution; ZK phased).
* **Edges:** `svc-ads` (quotas/policy); `svc-interop` bridges are **reversible** and **capability-gated**.
* **Wallet:** `svc-wallet` enforces **no doublespends**, integrates with ledger.&#x20;

---

## 10) Validation Suite (must pass)

* **Property tests:** conservation in ledger, idempotency in mailbox, chunking/rehydration paths.
* **Fuzzing:** OAP frame parse, bus event decode, DTO round-trip.
* **Concurrency models:** `loom` for bus/watcher paths.
* **Model checking:** DHT liveness and quorum safety.
* **Chaos:** network partitions, loss/jitter, DB stalls, injected panics; **graceful-degradation behaviors documented**.&#x20;

---

## 11) Supply Chain & Release Gates

Deterministic builds, minimal features, exact patch pins; **SBOM (CycloneDX/Syft)** per artifact; Cosign signatures with provenance; `cargo-audit`/`cargo-deny` clean; **releases blocked unless all gates are green**.&#x20;

---

## 12) Roadmap & Acceptance Gates (aligned to Six Concerns)

* **M0 (Kernel 0.1)** — Bus/Events/Transport/Metrics/Config; fuzz + loom clean; SBOM + signing.
* **M1 (Overlay + Index)** — content-addressed GET/PUT; resolver; p95 < 50 ms local; chaos basic pass.
* **M2 (Storage + DHT)** — chunked DAG + erasure; Kademlia with hedged lookups; NAT traversal smoke.
* **M3 (Mailbox)** — E2E messaging + receipts; quotas; soak 1M msgs/day.
* **M4 (Supervisor)** — restart policy + breakers; SLO-driven degradation.
* **M5 (Privacy)** — onion routing; cover traffic; metadata-minimized telemetry.
* **M6 (Audit & Bounty)** — external review; bug bounty; red-team report.
  **Gate for each milestone:** caps hold, clean restarts, no data loss for committed ops, observability sufficient for incident diagnosis; **concern labels must pass** (SEC/RES/PERF/ECON/DX/GOV).&#x20;

---

## 13) Perfection Gates (Release Blockers)

* Unsafe-free kernel (`#![forbid(unsafe_code)]`, Miri clean).
* Stable public API (exact re-exports).
* TLS type is `tokio_rustls::rustls::ServerConfig`.
* Axum 0.7 handlers end with `.into_response()` and enforce bounded sizes/timeouts.
* Bus overflow semantics: bounded, drop counter, throttled crash event, never block kernel.
* Transport hard limits (per-peer + global) with 429/503 + `Retry-After`.
* Security validated (capabilities, zeroize, rotation, amnesia).
* Formal/destructive validation (proptest, fuzz, loom, TLA+, chaos).
* Supply chain clean (pins, SBOM, signed artifacts).
* Ops runbooks + burn-rate alerts + canary rollouts.
* Coverage ≥ 85%; soak tests pass; reports attached.
  **CI reference:** `.github/workflows/perfection.yml`.&#x20;

---

## 14) Operational Runbooks (minimum set)

Overload (shed writes first), Disk-full (purge caches, stronger backpressure), Cert expiry (overlapping rotate), Config rollback (atomic snapshots), Partial partitions (degrade write quorum, prefer nearest reads; reconcile).&#x20;

---

## 15) Risks & Mitigations

Actor runtime maturity (time-boxed spike; fallback supervisor), over-instrumentation (sampling + dynamic log levels; disable heavy exporters under amnesia), sandbox portability (feature-gate seccomp), scope creep (enforce **33-crate canon** and milestone SLOs before expansion).&#x20;

---

## 16) Project Layout (by Pillar — canonical)

```
# P1 Kernel & Orchestration
crates/ron-kernel/
crates/ron-bus/
crates/ryker/

# P2 Policy & Governance
crates/ron-policy/
crates/svc-registry/

# P3 Identity & Key Management
crates/ron-kms/
crates/ron-auth/
crates/svc-passport/

# P4 Audit & Compliance
crates/ron-audit/

# P5 Observability
crates/ron-metrics/

# P6 Ingress & Edge
crates/svc-gateway/
crates/omnigate/
crates/svc-edge/

# P7 App BFF & SDK
crates/ron-app-sdk/
crates/oap/
crates/ron-proto/

# P8 Node Profiles
crates/macronode/
crates/micronode/

# P9 Content Addressing & Naming
crates/ron-naming/
crates/svc-index/

# P10 Overlay, Transport & Discovery
crates/svc-overlay/
crates/ron-transport/
crates/svc-dht/

# P11 Messaging & Extensions
crates/svc-mailbox/
crates/svc-mod/
crates/svc-sandbox/

# P12 Economics & Wallets
crates/ron-ledger/
crates/svc-wallet/
crates/ron-accounting/
crates/svc-rewarder/
crates/svc-ads/
crates/svc-interop/

# Storage service (cross-cuts pillars P9/P12 via data plane)
crates/svc-storage/
```

This layout replaces the older, partial list and reflects the overlay/DHT split and transport merge under `ron-transport`.

---

## 17) Six Concerns (constitution with teeth)

The **Six Concerns** are the enforceable backbone that absorb the updated blueprints. Every PR must tag **one or more** concerns; CI routes jobs accordingly.&#x20;

* **SEC — Security & Privacy:** capabilities only; PQ-hybrid posture; amnesia; STRIDE; DTO deny-unknown.
* **RES — Resilience & Concurrency:** crash-only, jittered backoff, bounded queues, no locks across `.await`, chaos.
* **PERF — Performance & Scaling:** latency/throughput SLOs, DHT hop bounds (p99 ≤ 5), replication budgets, profiling.
* **ECON — Economic Integrity:** conservation proofs, no doublespends, transient accounting→ledger, rewards (ZK phased).
* **DX — DX & Interop:** SDK retries/idempotency/tracing; OAP constants; reversible bridges; federation allow-lists.
* **GOV — Governance & Ops:** policy versioning & registry multi-sig; runbooks; metrics/alerts; quarterly reviews.

**CI & Review Glue:**
PR labels `concern:SEC|RES|PERF|ECON|DX|GOV` + `pillar:<#>`; `xtask check-concerns` verifies docs + invariants; perf/chaos jobs cover DHT sims, overlay backpressure, mailbox idempotency, storage decompression caps; schema guard for `ron-proto`; crypto build matrix (PQ/Arti on/off).&#x20;

---

## 18) Hardening Defaults (factored from Hardening Blueprint)

* **Limits:** timeout 5s; 512 inflight; 500 rps; 1 MiB body cap; decompression ≤ 10× + absolute cap.
* **Ops:** `/metrics`, `/healthz`, `/readyz`, `/version`; DRR and backpressure observable.
* **Security:** UDS dir `0700`, socket `0600`, **SO\_PEERCRED** allow-list; zeroize secrets; capability-only privileged ops.
* **Perf:** streaming I/O; **no unbounded buffers**; memory ceilings.
* **Tests:** unit + property + fuzz + chaos; metrics assertions.
  These defaults are shared via a small `hardening::layer()` pattern (Axum/Tower) per service.&#x20;

---

## 19) Delta Log (what changed vs v1.0)

* Removed references to `svc-arti-transport`; Arti is now a **feature** in `ron-transport`.
* Ensured **no DHT logic** in `svc-overlay`; added hop SLOs and bucket invariants in `svc-dht`.
* Consolidated naming under `ron-naming`; runtime lookups remain in `svc-index`.
* Integrated **Six Concerns** into milestones and CI labels; added amnesia-mode obligations across services.&#x20;

---

### Provenance

This v2.0 blueprint synthesizes v1.0 with your **12 Pillars**, **33-crate canon**, the updated **Hardening Blueprint**, and the carry-over notes that define the Six Concerns spine. It replaces residual legacy terms, corrects the overlay/DHT boundary, and aligns transport with `ron-transport` (Arti feature) — making the whole spec **drift-proof** and CI-enforceable.

