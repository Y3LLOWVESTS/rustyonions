Amazing — I combed through our blueprints set (Interop, Omnigate, Microkernel, Scaling, Hardening, App Integration, Developer Suite, Six Concerns, 12 Pillars, Complete Crate List) and tightened the macronode IDB for 100% coverage, including a first-class ZK (zero-knowledge) seam. Below is a paste-ready full replacement.

---

title: macronode — Invariant-Driven Blueprint (IDB)
version: 0.4.0
status: draft
last-updated: 2025-09-22
msrv: 1.80.0
crate-type: service profile (deployment role)
audience: contributors, ops, auditors
concerns: \[RES, PERF, GOV]

---

# 0) Scope & Purpose

**macronode** is the *large-footprint, operator-grade node profile* of RustyOnions. It packages the microkernel (ron-kernel) with a curated, feature-gated bundle of canonical services and operator surfaces for **multi-tenant workloads, high throughput, and strict governance**. Compared to **micronode** (single-tenant, DX-lean), **macronode** emphasizes **Resilience (RES), Performance (PERF), and Governance (GOV)** with hard SLOs, noisy-neighbor controls, capability-first policy, and optional **ZK attest & verify** at trust boundaries.

**In-scope**

* Composition/orchestration of canonical services on a single host or small cluster (feature-gated): **gateway, overlay, registry, naming, storage, index, mailbox, audit, auth, policy, metrics**, optional econ (ledger/wallet/rewarder).
* Operator surfaces: health/readiness, metrics, structured logs, **/version**, runbooks, safe upgrades/rollbacks, policy gates, red-team drills.
* Resource fencing & backpressure at service and node levels.
* Governance hooks: audit trails, policy enforcement, per-tenant quotas, transparency logs (Merkle), **ZK proof verification** (where configured).
* **Facet composition** via omnigate: Graph, Feed, Search, Media are *composed*, not re-implemented.

**Out-of-scope**

* Service business logic (lives in the respective crates).
* Cloud-specific IaC; deployment blue/green scripts (documented per env).
* End-user UX (belongs to app SDKs and higher layers).

**Primary Interfaces**

* **Core**: ron-kernel, ron-bus, ron-metrics, ron-policy, ron-auth, ron-audit, ron-naming, ron-proto, ron-transport, ron-kms.
* **Services (feature-gated)**: svc-gateway, svc-overlay, svc-index, svc-storage, svc-mailbox, svc-registry, svc-wallet, svc-rewarder, svc-mod, svc-ads (as/if canon), plus omnigate for facets.
* **Economics**: ron-ledger (+ wallet/rewarder).
* **Profile Interop**: macronode ↔ micronode via OAP/1.

**Topology (high-level)**

```mermaid
flowchart LR
  Kernel[ron-kernel] --> Bus[ron-bus]
  Bus --> Services[svc-gateway | svc-overlay | svc-index | svc-storage | svc-mailbox | svc-registry | ...]
  Services --> Facets[Graph / Feed / Search / Media via omnigate]
  Services --> Auth[ron-auth] --> Policy[ron-policy] --> Audit[ron-audit]
  Services --> Metrics[ron-metrics]
  Services --> KMS[ron-kms]
  Services --> Ledger[ron-ledger]:::econ
classDef econ fill:#eef,stroke:#99f,stroke-width:1px;
```

---

# 1) Invariants (MUST)

**I-M1 | Kernel Supervision**
Child service panics are contained; jittered backoff restart. **No lock held across `.await`** on supervisory paths. Graceful shutdown; idempotent start/stop.

**I-M2 | Signed Source of Truth (Config)**
macronode consumes **signed, versioned** config bundles; effective config changes emit `KernelEvent::ConfigUpdated { version }` and are audit-logged with hash and signer.

**I-M3 | Health/Readiness Gates**
Node is **Ready** iff: (a) kernel healthy, (b) **all required** services healthy, (c) policy/auth reachable, (d) entropy ≥128 bits, (e) **/version** surface live and consistent with binary metadata.

**I-M4 | Capability Wall**
Every externally reachable surface (HTTP/gRPC/overlay) is **auth → policy → quota → handler** with deny-by-default; capability tokens/macaroons carry least privilege & expiry.

**I-M5 | Backpressure & OAP/1 Limits**
No unbounded queues. Ingress enforces **OAP/1** limits: `max_frame = 1 MiB`, chunk ≈ 64 KiB (bounded), oversize → 413. Bounded mailboxes, timeouts, circuit breakers.

**I-M6 | Deterministic Observability**
Every ingress path records a **trace\_id**, latency histogram, error counters. `/metrics`, `/healthz`, `/readyz`, `/version` must be consistent and scrape-safe.

**I-M7 | TLS & Crypto Discipline (PQ-Ready)**
TLS uses **tokio\_rustls::rustls::ServerConfig** only; approved ciphers; keys via ron-kms; **hybrid PQ seam** (e.g., Kyber pilot) plumbed through config without coupling.

**I-M8 | Economic Integrity**
Econ flows idempotent, replay-safe; conservation proofs delegated to ron-ledger; audit trails hash-chained.

**I-M9 | Zero-Data Restarts / Amnesia**
When amnesia enabled: no durable state; caches RAM-only; secrets zeroized; restart leaves no residue; disk at rest is empty for profile-owned paths.

**I-M10 | Upgrade Safety**
Blue/green or rolling upgrades with version-skew tolerance; schema/config migrations monotonic and reversible (documented rollback).

**I-M11 | Interop Invariants**
macronode stays wire-compatible with micronode; **overlay excludes DHT** logic (lives where canon dictates); OAP/1 versioned DTOs with `deny_unknown_fields`.

**I-M12 | Safe Decompression**
≤10× expansion *and* absolute output caps for compressed inputs; refuse or sandbox beyond caps.

**I-M13 | Facet SLO Integrity**
Composed facet ops (Graph, Feed, Search, Media) respect stated SLOs and GOV policies; degradation path is defined and tested.

**I-M14 | Operator Intelligibility**
Every alert has a runbook URI and stable taxonomy; refusal codes follow 413/429/503 policy.

**I-M15 | ZK Integrity Boundary**
Where enabled, **zero-knowledge proofs** (ZKPs) validating policy compliance, rate limits, or data predicates **must be verified before privileged actions**. Proof systems and parameters are declared in config, and verifier keys are authenticated via KMS. **No witness material** is ever logged or persisted.

---

# 2) Design Principles (SHOULD)

* **D-M1 | Profile-as-Bundle**: Feature-gated service composition controlled by a signed profile config; minimal default footprint.
* **D-M2 | Hot-Path Minimalism**: Auth → Policy → Router only; side effects go async off hot path with bounded queues.
* **D-M3 | Govern the Edges**: Policy on ingress/egress; transparency log (Merkle) for governance events.
* **D-M4 | Predictable Envelopes**: Document p95/p99 latency & memory envelopes; maintain stability across minor releases.
* **D-M5 | Strict API Surface**: OAP/1 with explicit DTOs; `serde(deny_unknown_fields)` and stable versioning.
* **D-M6 | Fail-Dark**: Shed features before failing the whole node (except security/policy failures).
* **D-M7 | Amnesia-First Option**: One flag flips to ephemeral runtime; ops costs reflected in SLOs.
* **D-M8 | PQ & ZK Friendly**: Keep crypto pluggable (hybrid KEM slots; SNARK/STARK verifier traits).
* **D-M9 | Composed Facets**: Facet capabilities (Graph/Feed/Search/Media) come via omnigate composition; never embed logic locally.
* **D-M10 | CI Teeth**: Fuzz/perf/chaos/loom/sanitizers wired; docs quickstarts are tested; /version surfaced.

---

# 3) Implementation Patterns (HOW)

**Boot/Orchestration**

* Start order: metrics → bus → config watcher → transport(s) → services; subscribe to bus; hook health reporters.
* Expose `/metrics`, `/healthz`, `/readyz`, **`/version`** consistently.

**HTTP & Overlay**

* HTTP via **axum 0.7**; handlers end with `.into_response()`.
* Overlay uses `ron-transport` with TLS, read/write/idle timeouts, connection caps, and framed I/O enforcing OAP/1 limits.

**Metrics & Tracing**

* Prometheus metrics include: `request_latency_seconds` (histogram), `bus_lagged_total` (IntCounterVec), `service_restarts_total` (IntCounterVec).
* Structured JSON logs: `{trace_id, tenant, service, capability_id, outcome, latency_ms}`.

**Auth/Policy/Quota**

* Middleware stack: **TLS → Auth → Policy → Quota → Router**; deny-by-default; capability tokens/macaroons w/ root of trust pinned in KMS; cache policy decisions with bounded TTLs and signed bundles.

**Config & KMS**

* Config hot-reload is atomic; every apply emits bus event + audit record with bundle hash.
* Secrets and keys obtained via **ron-kms**; rotation supported; amnesia zeroizes material on shutdown.

**Storage & Data Handling**

* WAL/idempotent writes; BLAKE3 digests for integrity; safe decompression (≤10×, absolute caps).
* Optional econ store via **ron-ledger** and **svc-wallet/rewarder**.

**ZK Seam**

* **Verifier abstraction** with support for:

  * SNARK (e.g., BLS12-381) via succinct verification APIs.
  * Transparent STARK (no trusted setup) where circuits permit.
* **Domain separation** for transcripts; Fiat-Shamir rigor; **verifier keys authenticated** from KMS; batch verification path for throughput.
* **Locations**: (a) capability mint/rotate (prove attributes without revealing PII), (b) policy checks (prove “under quota” or “belongs to group”), (c) audit inclusion (prove event inclusion in Merkle transparency log) without log disclosure.
* **Witness hygiene**: never log/persist; in amnesia mode, witnesses remain in RAM and are zeroized.

**Upgrades/Drains**

* Readiness gates; in-flight drain with deadlines; skew-tolerant protocol versioning.

**Ops Surfaces**

* `/version` returns build info (git SHA, semver, feature set, PQ/ZK flags) to assert binary ↔ config parity.
* Runbook links embedded in alert annotations.

---

# 4) Acceptance Gates (PROOF)

**A-M1 | Readiness Correctness**
Kill/restore required service → `/readyz` flips false/true correctly; health reflects dependency status.

**A-M2 | Backpressure**
Sustain **≥1.5× p95** target load: bounded queues hold; 429/503 < **5%** with correct `Retry-After`; p99 within envelope.

**A-M3 | Chaos Drill**
Induce crash; median restart < **1s**; no deadlocks; no lock-across-await on supervisory paths (loom test proof).

**A-M4 | Policy Wall**
Unauthorized requests denied; audit captures rule id and refusal taxonomy (413/429/503).

**A-M5 | TLS & PQ Sanity**
TLS config validated; PQ flags surfaced in `/version` and `/readyz` details; no fallback to non-approved suites.

**A-M6 | Observability**
Exemplars and trace\_id propagate; metric cardinality stays within budget; scrape is stable for 24h.

**A-M7 | Upgrade Drill**
Blue/green with zero data loss; error budgets respected; skew tests pass.

**A-M8 | Amnesia Mode**
Disk scan shows no profile-owned residues; secret pages zeroized; cold-start parity confirmed.

**A-M9 | Facet SLOs**
Feed p95 ≤ **2s @10k followers**; Graph p95 ≤ **50ms** intra-AZ; Search p95 ≤ **150ms**; Media range start p95 ≤ **100ms**.

**A-M10 | Concurrency Proofs**
Loom model covering supervision/readiness passes; no await-holding-lock violations; bounded channels verified.

**A-M11 | Fuzz/Red-Team**
Fuzz policy parser/eval; send malformed OAP frames, quota storms, policy outage; system fails securely without meltdown.

**A-M12 | Profile Interop**
Round-trip macronode ↔ micronode; DTO versioning honored; deny drift.

**A-M13 | /version Surface**
Endpoint present & stable; contains git SHA, semver, enabled features, PQ/ZK flags; validated by CI smoke rig.

**A-M14 | Feature Matrix**
CI builds/tests with `--features arti` ON/OFF and amnesia ON/OFF; all green.

**A-M15 | UDS Perms (Conditional)**
If UDS used: dir `0700`, socket `0600`, **SO\_PEERCRED** allow-list enforced; else record N/A.

**A-M16 | 24h Soak**
No FD leaks; memory variance < **10%**; p99 within SLOs under ≥10k concurrent session simulation.

**A-M17 | Deny-Drift**
xtask/CI bans legacy/non-canonical crates; overlay verified to contain **no DHT** logic; crate canon pinned.

**A-M18 | ZK Verifier Proofs**

* **Correctness**: Known-good and known-bad proofs tested for each enabled circuit.
* **Performance**: Batch verification meets p95 ≤ **15ms** per proof (target for small policy predicates) under load.
* **Safety**: No witness logged; verifier keys loaded from KMS and pinned; domain-sep tags enforced.

**A-M19 | Econ Integrity**
Double-spend and replay attempts fail; conservation checks pass via ron-ledger; audit inclusion proofs (Merkle) verifiable (optionally with ZK inclusion).

**A-M20 | Docs Build-Run**
`cargo test --doc` passes; RUNBOOK quickstarts validated by bash smoke rig in CI.

---

# 5) SLOs & Envelopes

| Surface   | SLI                            | Target                |
| --------- | ------------------------------ | --------------------- |
| Gateway   | HTTP p99 latency               | ≤ 120 ms              |
| Overlay   | Hop p99 latency                | ≤ 200 ms              |
| Registry  | Lookup p95                     | ≤ 50 ms               |
| Feed      | Fanout p95                     | ≤ 2 s @ 10k followers |
| Graph     | Neighbor fetch p95             | ≤ 50 ms               |
| Search    | Query p95                      | ≤ 150 ms              |
| Media     | Range start p95 (intra-AZ)     | ≤ 100 ms              |
| Policy    | Eval p95                       | ≤ 10 ms               |
| Audit     | Ingest lag                     | ≤ 500 ms              |
| ZK Verify | Per-proof verify p95 (batched) | ≤ 15 ms (small preds) |

---

# 6) Security, PQ & ZK Highlights

* **mTLS/token auth**; macaroons short TTL; capabilities least-privilege.
* **PQ readiness** via ron-kms-backed hybrid KEM seam; **tokio\_rustls::rustls::ServerConfig** only.
* **ZK boundary**: configurable circuits for (a) capability mint, (b) policy predicates (e.g., “under quota”, “member-of-group”), (c) audit inclusion proofs.
* **Transparency log**: append-only, Merkle-root anchored; optional ZK inclusion without revealing full log.
* **Privacy budget** (optional): policy supports bounded decision caches; no PII in logs; redaction rules enforced.
* **Key hygiene**: rotation & zeroization on amnesia; verifier keys authenticated via KMS.

---

# 7) Integration Map

* **Kernel**: bus, metrics, config, health events → `KernelEvent::*`.
* **Transport**: TLS/timeouts/caps via `TransportConfig`.
* **Gateway/Overlay**: ingress → auth/policy/quota → router.
* **Registry/Naming**: service discovery & stable addressing.
* **Storage/Index/Mailbox**: durability & retrieval (with safe decompression & digest checks).
* **Auth/Policy/Audit**: capability wall + enforcement & audit hash chain.
* **Metrics**: Prometheus; exemplars with trace\_id.
* **Econ**: ron-ledger (+ wallet/rewarder) when enabled.
* **ZK**: verifier module behind policy/identity surfaces; KMS-pinned parameters.
* **Facets**: omnigate composes downstreams (Graph/Feed/Search/Media).

---

# 8) Failure Domains & Run Modes

* **Single-host** (default), **Clustered**, **Amnesia**.
* Clear blast-radius: transport, policy, storage/index, econ, zk-verify.
* Degradation ladders defined (shed facets first, keep core auth/policy online).

---

# 9) Risks & Mitigations

* **Policy latency spikes** → bounded caches with signed bundles; strict TTL; fail-secure (deny) on backend outage.
* **Queue blowups** → bounded channels + circuit breakers; drop non-essentials first.
* **Version skew** → monotonic schema; pre-flight checks; blue/green scripts.
* **TLS misconfig** → boot validation; `/readyz` details; CI fixtures.
* **Operator error** → runbook dry-runs; label-routed CI for targeted checks.
* **ZK circuit drift/toxic waste** → prefer **transparent** systems when possible; if SNARK, documented ceremony & custody; parameter hashes pinned in config; batch verify limits enforced.
* **Witness leakage** → forbid persistence/logging; memory zeroization gates tested.
* **Audit log growth** → Merkle checkpoints + compaction; inclusion proofs stay fast.

---

# 10) Work Plan (Executable TODO)

**A. Wiring & Boot**
\[ ] Macronode bin boots kernel, metrics, transport; wires `/metrics`, `/healthz`, `/readyz`, `/version`.
\[ ] Bus subscriptions for health/config; structured logging baseline.

**B. Bundle Composition**
\[ ] Feature-gate gateway/overlay/registry/storage/index/mailbox/naming/audit/auth/policy/metrics/econ.
\[ ] Health reporters per service; readiness dependencies declared.

**C. Security/Policy**
\[ ] Capability tokens + macaroons; policy cache with signed bundles; KMS for secrets.

**D. Backpressure/Perf**
\[ ] Bounded mailboxes; connection caps; latency histograms; perf harness (Criterion + flamegraph).

**E. Observability/Ops**
\[ ] Prometheus registry + exemplars; tracing spans; dashboards & alerts; runbook links.

**F. Governance/Audit**
\[ ] Append-only audit sink; Merkle transparency roots; per-tenant quotas; refusal taxonomy.

**G. Upgrades/Rollbacks**
\[ ] Drain orchestration; skew tests; blue/green pipeline.

**H. Amnesia Mode**
\[ ] Global flag; disable disk writes; zeroization tests; cold-start parity.

**I. ZK Enablement**
\[ ] Verifier trait + batch path; KMS-pinned parameters; circuits catalog (capability, policy predicate, audit inclusion).
\[ ] `/version` exposes ZK feature flags; health includes verify-self test.

**J. CI & Verification**
\[ ] Loom (supervision/readiness); fuzz (policy/econ/overlay frames); sanitizers (ASan/TSan/UBSan); chaos drills.
\[ ] cargo-deny/clippy/fmt; docs quickstart smoke rig; label-routed CI by concern/pillar/profile/facet/amnesia.
\[ ] Feature matrix: `--features arti` ON/OFF + amnesia ON/OFF.
\[ ] 24h soak with leak checks & SLO assertions; deny-drift checks for crate canon and overlay/DHT separation.

**K. Facet Integration Proofs**
\[ ] Omnigate wiring for Graph/Feed/Search/Media; SLO records; degradation runbooks.

**L. Econ Hooks**
\[ ] ron-ledger conservation checks; wallet/rewarder integration behind feature flags.

---

# 11) Appendix — Reference Types & Events

```rust
pub enum KernelEvent {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    ServiceCrashed { service: String },
    Shutdown,
}

pub struct TransportConfig {
    pub addr: std::net::SocketAddr,
    pub name: &'static str,
    pub max_conns: usize,
    pub read_timeout: std::time::Duration,
    pub write_timeout: std::time::Duration,
    pub idle_timeout: std::time::Duration,
}
// TLS configs MUST use tokio_rustls::rustls::ServerConfig.
```

---

# 12) Change Log

* **0.4.0** — Added **ZK seam** (verifier abstraction, batch verify, KMS-pinned params, inclusion proofs), transparency log, privacy budget note, stronger PQ/ZK surfacing in `/version`, upgraded acceptance gates, clarified canon/overlay separation, expanded CI teeth.
* **0.3.0** — Developer Suite alignment: `/version`, docs build-run CI, ronctl gate, feature matrix, UDS (conditional), 24h soak, deny-drift, label-routed CI, red-team storms.
* **0.2.0** — Added Amnesia, PQ, OAP constants, Facet SLOs, CI teeth, Profile interop.
* **0.1.0** — Initial draft.

---

