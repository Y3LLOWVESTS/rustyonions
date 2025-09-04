
# RustyOnions — THE PERFECT FINAL BLUEPRINT (v1.0)

_Generated: 2025-08-30 23:59:10_

This document **unifies and supersedes** the prior artifacts — the original Final Blueprint, the “New Internet Blueprint — Microkernel Plan,” and **PERFECTION_PASS.md** — into a single, actionable specification for building a secure, high‑concurrency, high‑fault‑tolerance microkernel platform.

**Status:** This blueprint is the **source of truth** for architecture, acceptance gates, SLOs, and the delivery plan. All future changes must land as PRs against this file with matching updates to CI gates.

---

## 0) Executive Summary

- Keep the kernel **tiny, safe, and stable** — one crate (`ron-kernel`) exporting exactly the public API defined below. Services live **outside** the kernel boundary and can crash, restart, and evolve independently.
- Drive everything with **capabilities**, **content addressing**, and **end‑to‑end encryption**. Make failure a **first‑class input** via supervision and backpressure.
- Bake in **observability**, **supply‑chain integrity**, and **formal/destructive validation**. We ship only when **Perfection Gates** are green.
- Initial service set is **nine** actors (overlay, index, storage, DHT, gateway, mailbox, discovery, payment, accounting) with a lean supervisor and typed IPC.

---


## 0.1) Invariants (source of truth)

- **Addressing (normative):** `b3:<hex>` where `<hex>` is the 32-byte **BLAKE3-256** digest (b3-256) of the **unencrypted** object (or manifest root). Truncated prefixes MAY be used for routing; services **MUST** verify the full digest before returning bytes.
- **OAP/1 default:** `max_frame = 1 MiB` (per **GMI-1.6**). **64 KiB** refers to a **storage streaming chunk** (implementation detail) and must not be conflated with the OAP/1 `max_frame`.
- **Kernel public API (frozen):** `Bus`, `KernelEvent::{ Health {service, ok}, ConfigUpdated {version}, ServiceCrashed {service, reason}, Shutdown }`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
- **Normative spec pointer:** OAP/1 is defined by **GMI-1.6**. Any `/docs/specs/OAP-1.md` in this repo is a **mirror stub** that links to GMI-1.6 to prevent drift.
- **Perfection Gates ↔ Milestones:** The Final Blueprint maps gates **A–O** to **M1/M2/M3** in the **Omnigate Build Plan** (see Appendix).


- **Zero-knowledge proofs:** Feature-gated ZK hooks in `ledger`/`svc-rewarder` for private usage reporting (post-merge).
## 1) Non‑Negotiable Principles

- **Secure by default:** _keys not accounts_, mTLS/Noise, least‑privilege capabilities, zero‑trust boundaries.  
- **Crash‑only + supervision:** processes die fast, restart fast, never corrupt state.
- **Backpressure first:** every boundary enforces limits; **no unbounded queues**.
- **Deterministic interfaces:** strict schemas, stable re‑exports, versioned messages.
- **Content addressing:** integrity by hash; idempotency everywhere.
- **Telemetry is a feature:** `/healthz`, `/readyz`, `/metrics`, tracing baked in.
- **Reproducible builds + SBOM:** release equals proof.
- **Chaos‑ready:** fault injection, partitions, and latency spikes are test modes.
- **Erase by design:** _amnesia mode_ with zeroization and RAM‑only logs.

---

## 2) Kernel (the Diamond)

**Scope:** Bus, Events, Metrics/Health, Transport, Config, Capabilities, Supervisor hooks, Shutdown helper.

### 2.1 Public API (frozen)

```rust
pub use { Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c };
```

**KernelEvent** (exact variants & meaning):

```rust
enum KernelEvent { 
  Health{ service: String, ok: bool },
  ConfigUpdated{ version: u64 },
  ServiceCrashed{ service: String, reason: String },
  Shutdown
}
```

**Breaking change policy:** any semantic change requires a **major** version bump and migration notes.

### 2.2 Bus & IPC

- **Events:** lossy broadcast (`tokio::sync::broadcast`) with **bounded capacity** (default: 4096).  
  On overflow: increment `bus_overflow_dropped_total`, emit a throttled `ServiceCrashed{service:"bus-overflow"}`, **never block** kernel critical paths.
- **Commands:** per‑service bounded `mpsc` (default: 128) with **oneshot** replies for request/response.

### 2.3 Transport

- Acceptors for HTTP/2 & HTTP/3 (QUIC), WebSocket, raw QUIC (feature‑gated).  
- **TLS:** `tokio_rustls::rustls::ServerConfig` only.  
- Hard limits exported to metrics: handshake `<2s`, read idle `<15s`, write `<10s`, header/body caps, per‑peer RPS/bytes/sec/streams with 429/503 + `Retry‑After` on exceed.

### 2.4 Metrics & Health

- `/metrics` (Prometheus), `/healthz`, `/readyz`. Health registry with per‑service `HealthState` and SLOs. `/readyz` **fail‑open reads / fail‑closed writes** during incident windows.

### 2.5 Config & Hot‑Reload

- Immutable snapshot struct. File watcher emits `KernelEvent::ConfigUpdated{version}` **after** validation. Failed validation triggers **automatic rollback** to prior snapshot.

### 2.6 Capabilities & Secrets

- Ed25519 identities; macaroon caveats (TTL, path, method, audience).  
- Zeroize on drop; sealed storage; rotation ≤ 30 days; amnesia mode disables persistence and moves logs to RAM‑fs.

### 2.7 Supervisor Hooks

- Exponential backoff with jitter (100ms → 30s), restart‑intensity cap (≤ 5/60s), quarantine flappers; publish `ServiceCrashed`/`Restart` reasons on the Bus.

---

## 3) Services (Nine Actors)

Each service is a **single‑purpose actor** with a typed command enum and event emissions. All queues are **bounded**. Hot paths use `bytes::Bytes` and streaming.

1. **GatewayService (HTTP)** — Axum + Tower; routes: `/o/<addr>`, `/put`, `/status`, `/metrics`; per‑route limiters & timeouts.  
2. **OverlayService (+Chunking)** — streamed PUT/GET; Merkle chunk manifests; content addressing.  
3. **IndexService** — resolver (write quorum / raft later), read‑through cache; bounded `spawn_blocking` to RocksDB (post‑sled).  
4. **StorageService** — DAG store; rolling‑hash chunker (1–4 MiB); Reed‑Solomon erasure; background repair jobs.  
5. **DHTService** — Kademlia core with hedged lookups; onion‑routed queries (feature‑gated); NAT traversal.  
6. **MailboxService** — encrypted store‑and‑forward; delivery receipts; quotas + per‑peer rate limits.  
7. **DiscoveryService** — peer bootstrap, seed rotation, gossip; publishes `PeersUpdated`.  
8. **PaymentService** — price models (per‑request/per‑MiB/flat), internal credits now; LNURL/stablecoin adapters later; PoW fallback for tiny anonymous GETs.  
9. **AccountingService** — atomics for counters; periodic rollups; emits `QuotaExceeded`, `UsageTick`.

**IPC example (Overlay commands):**

```rust
pub enum OverlayCmd { 
  Put{ blob: bytes::Bytes, manifest: ManifestV2, reply: oneshot::Sender<Result<Address, OverlayError>> },
  Get{ addr: Address, reply: oneshot::Sender<Result<bytes::Bytes, OverlayError>> },
  Stat{ addr: Address, reply: oneshot::Sender<Result<ObjectInfo, OverlayError>> },
}
```

---

## 4) Data Plane & Formats

- **Addressing:** `b3:<hex>` (**BLAKE3-256**) over the unencrypted object (or manifest root); manifests use DAG-CBOR. Truncated prefixes permitted for routing; full digest MUST be verified before returning bytes.
- **Schemas:** versioned; unknown fields ignored (forward‑compatible); strict validation on writes.  
- **Encryption:** per‑object envelope keys; group sessions with epoch ratchets; key‑disclosure audits.  
- **CRDTs:** apply only where multi‑writer convergence is required (profiles/presence).

---

## 5) Concurrency & Backpressure

- Tokio multi‑thread runtime for I/O; **bounded CPU pool** (max `cores-1`) for hashing/compression/crypto.  
- `JoinSet` per request, cancelling children on parent drop; deadlines on all awaits.  
- Semaphores at: dials, inbound requests, CPU jobs; Tower global concurrency cap to avoid collapse.  
- Zero‑copy paths (`Bytes`, vectored I/O); avoid `String` on hot paths.

**Initial budgets (tunable):** transport inbound ~1k msgs/listener; overlay queue ~2× cores; CPU pool max(2, cores‑1).

---

## 6) Fault Tolerance & Recovery

- **Crash‑only:** WAL + atomic snapshots; idempotent write paths.  
- **Supervision:** actor links + backoff; circuit breakers; retries with jitter at edges.  
- **Chaos:** netem packet loss (0–30%), jitter (100–1000ms), partitions (A|B, A|B, A|B|C), DB stalls, injected panics.  
- **Shutdown:** stop accepting; drain with deadlines; flush journals; final ACK on Bus.

---

## 7) Security & Privacy

- Mutual auth (mTLS/Noise) on non‑public planes.  
- Nonces + short TTL for replay defense; constant‑time checks where applicable; padded envelopes for sensitive ops.  
- Sandbox: optional seccomp profile in amnesia mode (Linux).  
- Logging: structured, redacted; **no secrets**.

---

## 8) Observability

- **Metrics:** requests, 4xx/5xx, latency histograms, queue depths, restarts, bytes in/out, open conns, backpressure drops.  
- **Tracing:** OpenTelemetry spans across RPC boundaries; correlation IDs propagated.  
- **Profiling:** `pprof` gated by capability; periodic flamegraphs in staging.  
- **Anomaly hooks:** light statistical triggers for queue spikes or restart storms.

---
Note: Kernel emits `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`; services emit `rejected_total{reason=...}` tied to `ServiceCrashed{reason}`.
## 9) Validation Suite (must pass)

- **Property tests:** capability checks, chunking/rehydration, validators.  
- **Fuzzing:** CBOR/JSON/TLV parsers (≥ 72h cumulative corpus, zero crashes/UB).  
- **Concurrency models:** `loom` for Bus + watcher (no deadlocks or missed notifications).  
- **Model checking:** TLA+/Apalache for DHT liveness and quorum safety.  
- **Jepsen‑style chaos:** assert linearizability where claimed; document graceful‑degradation behaviors.

---

## 10) Supply Chain & Release Gates

- Deterministic builds; minimal features; exact patch pins.  
- SBOM (CycloneDX/Syft) generated per artifact; Cosign signatures with provenance.  
- `cargo-audit` / `cargo-deny` clean; license policy enforced.  
- Releases **blocked** unless all gates are green.

---

## 11) SLOs & SLIs (targets)

- **Kernel uptime:** ≥ 99.99% monthly.  
- **Local admin latency:** `/healthz` p99 < 15ms; `/metrics` p99 < 25ms.  
- **Transport:** sustain ≥ 10k TLS connections on a dev box; large GETs 2–5 Gbit/s localhost.  
- **Overlay small object latency:** p95 < 10ms local; via Tor < 150ms.  
- **Memory:** tens of MiB steady‑state; bounded peaks.

On SLO burn: **auto‑degrade** (shed writes first, keep cached reads; reduce fan‑out; raise sampling).

---

## 12) Roadmap & Acceptance Gates

**M0 (Kernel 0.1):** stable API; Bus/Events/Transport/Metrics/Config; fuzz + loom clean; SBOM + signing.  
**M1 (Overlay + Index):** content‑addressed GET/PUT; resolver; p95 < 50ms local; chaos basic pass.  
**M2 (Storage + DHT):** chunked DAG + erasure; Kademlia w/ hedged lookups; NAT traversal smoke tests.  
**M3 (Mailbox):** E2E messaging + receipts; quotas; soak 1M msgs/day.  
**M4 (Supervisor):** restart policies; circuit breakers; SLO‑driven degradation.  
**M5 (Privacy):** onion routing; cover traffic; metadata‑minimized telemetry.  
**M6 (Audit & bounty):** external review; bug bounty; red‑team report.

**Gate for each milestone:** concurrency caps hold; clean restarts; no data loss for committed ops; metrics/traces adequate for incident diagnosis.

---

## 13) Perfection Gates (Release Blockers)

This section embeds the **PERFECTION_PASS** acceptance checklist. CI must fail if any box is false.

- Unsafe‑free kernel (`#![forbid(unsafe_code)]`, Miri clean).  
- Stable public API (exact re‑exports).  
- TLS type is `tokio_rustls::rustls::ServerConfig`.  
- Axum 0.7 handlers use `.into_response()` with bounded sizes/timeouts.  
- Bus overflow semantics: bounded channel, drops counter, throttled crash event, never block kernel.  
- Transport hard limits (per‑peer and global) with 429/503 + `Retry‑After`.  
- Observability complete (metrics, tracing, health/ready semantics).  
- Security validated (capabilities, zeroize, rotation, amnesia).  
- Formal/destructive validation (proptest, fuzz, loom, TLA+, chaos).  
- Supply chain clean (pins, SBOM, signed artifacts).  
- Ops runbooks + SLO burn‑rate alerts + canary rollouts.  
- Coverage ≥ 85% lines/branches; soak tests pass; reports attached.

**Reference CI**: see `.github/workflows/perfection.yml` (build/test, nextest, llvm‑cov, audit/deny, miri, loom, fuzz, sbom‑sign).

---

## 14) Operational Runbooks (minimum set)

- Overload (shed writes, rate‑limit; open circuit breakers; raise sampling).  
- Disk‑full (rotate logs, purge caches, backpressure stronger; resume after headroom).  
- Cert expiry (auto‑renew or planned rotate with overlapping validity).  
- Config rollback (atomic snapshots, revert, emit `ConfigUpdated`).  
- Partial partitions (degrade write quorum, prefer nearest reads; monitor reconciliation).

---

## 15) Risks & Mitigations

- **Actor runtime maturity:** time‑boxed spike; clean fallback to in‑house supervisor.  
- **Over‑instrumentation:** sampling + dynamic log levels; disable heavy exporters in amnesia.  
- **Sandbox portability:** feature‑gate seccomp; document OS caveats.  
- **Scope creep:** enforce nine‑service MVP; expand only after SLOs are green.

---

## 16) Project Layout

```
crates/
  ron-kernel/        # bus, events, transport, metrics/health, config, caps, supervisor hooks
  svc-gateway/       # axum handlers; per-route limiters; public edge
  svc-overlay/       # streamed PUT/GET; manifests; content addressing
  svc-index/         # resolver; quorum write; cache read
  svc-storage/       # DAG store; chunker; erasure coding; repair
  svc-dht/           # kademlia + hedged lookups; nat traversal
  svc-mailbox/       # encrypted store-and-forward; quotas
  svc-discovery/     # peer bootstrap & gossip
  svc-payment/       # internal credits now; adapters later
  svc-accounting/    # usage meters, rollups, quotas
supervisor/          # shared actor policies, circuits
cli/                 # admin & debug tools (caps, tracing, chaos)
testing/             # chaos harness, jepsen-style, perf rigs
specs/               # TLA+, schemas, docs
```

---

## 17) Appendix — Defaults We Ship With

- Timeouts: handshake 2s, read idle 15s, write 10s, client body 16 MiB, headers 64 KiB.  
- SLO targets: internal API p95 < 40ms, p99 < 120ms.  
- Amnesia mode: disk disabled; logs to RAM; secrets zeroized on exit.  
- Overload responses: 429 or 503 with randomized `Retry‑After: 1–5s`.  
- Tracing sampling: 0.1% steady‑state, ≥ 5% during incidents.

## 18) Appendix — Perfection Gates ↔ Milestones (A–O)

| Milestone | Gates (subset) | Summary |
|-----------|-----------------|---------|
| **M1 (Bronze)** | A–E | OAP/1 baseline, Gateway quotas, Mailbox MVP, metrics & healthz, fuzz smoke |
| **M2 (Silver)** | F–J | Storage/DHT, mailbox reliability, revocation, latency targets, streaming |
| **M3 (Gold)** | K–O | Proptests, chaos/leakage harness, governance, polyglot SDKs, multi-tenant tests |


---

### Provenance

This blueprint synthesizes: the original Final Blueprint, the New Internet Microkernel Plan, and the **Perfection Pass** acceptance gates, aligning them into one implementable specification.
