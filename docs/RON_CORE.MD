

````md
---
title: RON-CORE — Scope, Invariants, and Launch Plan
status: draft
msrv: 1.80.0
last-updated: 2025-10-17
audience: contributors, reviewers, ops
owners: [Stevan White]
profile: RON-CORE            # Core Plane (no value-plane/Web3)
---

![Profile: RON-CORE](https://img.shields.io/badge/profile-RON--CORE-blue)
> **RON-CORE** = Micronode + Macronode substrate (kernel, transport/overlay/DHT, identity/KMS, gateway/edge, storage/index, ops).  
> **Out of scope:** Value plane (wallet/ledger/accounting/rewarder), ads, sandbox/modules, and external webhooks/interop.

---

# 0) Purpose

Keep the entire project **laser-focused** on shipping the **Core Plane** first—without drift into Web3/economics or other deferred facets.  
This document defines:
- **Scope** (what’s in vs. out)  
- **Architectural invariants** (non-negotiables across crates)  
- **Acceptance gates** (what must be true before we call it “done”)  
- **Minimal demos** (what users/devs can do day-one)  
- **Milestones** (tight path to ship)  
- **Communication rule** (abstain from suggestions on deferred crates unless explicitly requested)

---

# 1) Scope (Authoritative)

## 1.1 Included Crates (RON-CORE)

**Runtime & Protocol**
- `crates/ron-kernel` — microkernel (actors, supervision, shutdown)
- `crates/ron-bus` — process/event bus (bounded, backpressure)
- `crates/ron-proto` — DTOs/types; wire-safe shapes
- `crates/oap` — OAP/1 frame/stream constants & codec
- `crates/ron-app-sdk` — app-facing SDK (client/runtime helpers)

**Networking & Overlay**
- `crates/ron-transport` — I/O, timeouts, TLS(rustls)
- `crates/svc-overlay` — sessions, gossip, peer mgmt
- `crates/svc-dht` — discovery (Kademlia-like)

**Ingress / Edge**
- `crates/svc-gateway` — HTTP ingress/BFF
- `crates/omnigate` — policy/routing/composition
- `crates/svc-edge` — caching, byte-range reads

**Content & Naming**
- `crates/svc-storage` — content-addressable store (CAS)
- `crates/svc-index` — name/manifest → providers/index
- `crates/ron-naming` — name & address types

**Identity & Security**
- `crates/svc-passport` — identities, capability tokens
- `crates/ron-auth` — authZ primitives
- `crates/ron-kms` — key custody, zeroization, PQ-ready hooks

**Ops & Control**
- `crates/ron-metrics` — golden metrics, /metrics
- `crates/ron-policy` — policy engine, deny-by-default
- `crates/ron-audit` — audit trail/event digests
- `crates/svc-registry` — signed topology & service registry

> **Profiles:** Same client API across **Micronode** (single binary, amnesia mode) and **Macronode** (multi-service).  

## 1.2 Deferred (Explicitly Not in RON-CORE)

- **Value/Web3:** `crates/svc-wallet`, `crates/ron-ledger`, `crates/ron-accounting`, `crates/svc-rewarder`
- **Ads:** `crates/svc-ads`
- **Sandbox/Modules:** `crates/svc-sandbox`, `crates/svc-mod`
- **Interop bridges:** `crates/svc-interop` (3rd-party webhooks/providers)

> **Communication rule:** Do **not** propose designs or suggestions for deferred crates unless the owner explicitly requests it.

---

# 2) Architectural Invariants (Non-Negotiable)

- **I-1 Content Addressing:** All blobs addressed by **BLAKE3** digest (`b3:<hex>`). Reads verify digest before serve.
- **I-2 Transport Limits:** OAP/1 `max_frame = 1 MiB`; streaming chunks ~64 KiB; strict timeouts and backpressure.
- **I-3 Amnesia Mode (Micronode):** RAM-only, no persistence by default; explicit opt-in to persist.
- **I-4 Observability Contract:** Every service exposes `/metrics`, `/healthz`, `/readyz`. Histogram latencies; error counts; backpressure gauges.
- **I-5 Capability-Scoped Access:** Macaroon/cap tokens with least-privilege scopes (read, write-once, admin).
- **I-6 PQ-Ready Edge:** TLS/KMS support hybrid/PQ toggle points; no PQ requirement at launch, but no dead-ends.
- **I-7 No Locks Across `.await`:** Concurrency model avoids holding locks across await points; prefer bounded channels.
- **I-8 Fail-open Reads / Fail-closed Writes (Content):** Serving remains resilient; mutation requires strict validation.
- **I-9 Deterministic Errors:** Stable error codes/messages for SDK resilience; no semantic breaking changes within a minor.
- **I-10 Reproducible Builds:** Pinned workspace deps; cargo-deny green; SBOM emitted in CI.

---

# 3) Acceptance Gates (Ship Criteria)

- **G-OBS (Observability):**
  - `/metrics`, `/healthz`, `/readyz` on all included services
  - Golden histograms: request latency, request size, CAS hit/miss, DHT ops
- **G-INT (Interop):**
  - OAP/1 compliance; DTOs versioned; canonical test vectors
  - Micronode ↔ Macronode clients unchanged
- **G-SEC (Security):**
  - Cap tokens enforced on write/mutate paths
  - KMS zeroization paths tested; PQ toggle proven in TLS/KMS smoke tests
- **G-PERF (Performance):**
  - p95 GET (intra-region, cached) ≤ target X ms (fill per env)
  - Sustained PUT/GET throughput targets recorded with harness
- **G-RUN (Runbooks):**
  - Minimal runbooks for bootstrap, rotate keys, drain/upgrade, incident triage
- **G-DOC (Docs):**
  - README + CONFIG + SECURITY + OBSERVABILITY + PERFORMANCE + RUNBOOK populated for each included crate
- **G-TEST (Testing):**
  - Unit tests for invariants
  - Integration smoke: PUT/GET; DHT discover; Gateway byte-range; Passport issue/verify
  - Chaos: transport timeouts/backpressure; CAS corruption detection (negative test)

> **Release tag when all gates pass:** `core-vX.Y.Z`

---

# 4) Minimal Demos (Day-One)

### D-1 Zero-Trust File Drop
- Put a blob to Micronode CAS → serve by `b3:` from Macronode via Gateway with byte-range support.

### D-2 Peer Discovery & Transfer
- Two Micronodes discover via DHT and exchange chunks automatically under cap-scoped session.

### D-3 Signed Artifacts Channel
- Producer signs & publishes artifact; consumers fetch by content address; integrity verified on read.

---

# 5) Milestones (Tight Path)

- **M1 Kernel & Ops (Week 1–2):**
  - Kernel/bus hardened; /metrics, /healthz, /readyz everywhere
  - CAS write/read path + digest verify; Index minimal resolve
- **M2 Network & Ingress (Week 3–4):**
  - Transport with timeouts/backpressure; Overlay sessions; DHT basic
  - Gateway GET + byte-range + caching; SDK happy path
- **M3 Identity & Profiles (Week 5):**
  - Passport/Auth/KMS wired; Micronode amnesia parity; Macronode packaging
- **M4 Perf & PQ Hooks (Week 6):**
  - Perf passes (p95/p99); PQ toggle smoke (TLS/KMS)
- **M5 Tests/Docs/Runbooks (Week 7):**
  - Integration + chaos smokes; crate docs/runbooks
- **M6 Polish & Ship (Week 8):**
  - CI gates green; examples; `core-v1.0.0` tag

*(If sprinting to a preview in ~4–5 weeks, push M4–M5 items into a post-preview patch plan.)*

---

# 6) Interfaces (Concise)

- **OAP/1**: framed up to 1 MiB; streaming ~64 KiB; content-addressed pulls
- **HTTP ingress**: GET with byte-range; PUT guarded by caps
- **CAS API**: put(blob) → b3, get(b3) → bytes/stream; head(b3) → meta
- **Index API**: resolve(name/version) → {b3, providers}
- **Passport/KMS**: issue(cap_specs) → token; verify(token) → scopes

---

# 7) Configuration Knobs (Core Only)

- `RON_AMNESIA=on|off` (Micronode default: on)
- `RON_TLS_PQ=off|hybrid` (toggle PQ-ready ciphers when supported)
- `RON_DHT_PROFILE=lite|standard`
- `RON_CACHE_MAX_BYTES=<size>`
- `RON_TIMEOUT_CLIENT_MS=<n>` / `RON_TIMEOUT_BACKEND_MS=<n>`
- `RON_LOG_LEVEL=info|debug|trace`

---

# 8) Quality Bars & CI

- **cargo fmt/clippy** must pass with zero new warnings
- **cargo-deny**: licenses/bans/advisories/sources green
- **SBOM** emitted on release; artifact digests recorded
- **perf harness** runs on PRs that touch transport/CAS/gateway
- **semver checks**: public API drift detected for SDK/proto

---

# 9) Versioning & Labels

- **Profile tag:** `RON-CORE`
- **Release tags:** `core-vMAJOR.MINOR.PATCH`
- **Issue/PR labels:** `plane:RON-CORE`, `profile:micronode`, `profile:macronode`, `gate:perf`, `gate:security`, `gate:interop`, `gate:observability`

---

# 10) FAQ (Short)

- **Q: Is PQ mandatory at launch?**  
  A: No. We require **PQ-ready toggles** at the TLS/KMS edge; full PQ rollout can follow.

- **Q: Does Micronode behave differently from Macronode?**  
  A: Same APIs; Micronode defaults to **amnesia** (RAM-only). Macronode is multi-service/persistent.

- **Q: Can I build apps now without the value plane?**  
  A: Yes—content apps, signed artifact channels, secure file distribution, private sharing, and more.

---

# 11) Example Commands (No leading “#”)

**Build everything in RON-CORE workspace (example subset):**
cargo build -p ron-kernel -p ron-bus -p ron-app-sdk -p ron-proto -p oap -p ron-transport -p svc-overlay -p svc-dht -p svc-gateway -p omnigate -p svc-edge -p svc-storage -p svc-index -p ron-naming -p svc-passport -p ron-auth -p ron-kms -p ron-metrics -p ron-policy -p ron-audit -p svc-registry

**Run a Micronode in amnesia mode (example envs):**
RON_AMNESIA=on RON_LOG_LEVEL=info cargo run -p micronode

**Smoke: put/get via Gateway (pseudo):**
curl -X PUT --data-binary @file.bin "http://localhost:8080/put?cap=<token>"
curl -L "http://localhost:8080/o/b3:<hex>"

---

# 12) LOC & Timeline Reality (for planning)

- **Implementation LOC (typical):** ~30k across RON-CORE  
- **Total with tests/docs (typical):** ~57k–65k  
- **Sustainable timeline (solo):** ~8–10 weeks to “Gold-candidate” with gates

---

# 13) Change Log Header (Template)

## [core-vX.Y.Z] — YYYY-MM-DD
- Kernel/Bus: …
- Transport/Overlay/DHT: …
- Gateway/Edge: …
- Content/Index: …
- Identity/KMS: …
- Ops: metrics/policy/audit/registry
- Docs/Runbooks: …

---

# 14) Mermaid (Overview)

```mermaid
flowchart LR
  subgraph Client/App
    SDK[ron-app-sdk]
  end

  subgraph Micronode/Macronode
    KRN[ron-kernel]
    BUS[ron-bus]
    TRN[ron-transport]
    OVY[svc-overlay]
    DHT[svc-dht]
    GTW[svc-gateway]
    EDGE[svc-edge]
    IDX[svc-index]
    CAS[svc-storage]
    PAS[svc-passport]
    KMS[ron-kms]
    MET[ron-metrics]
    POL[ron-policy]
    AUD[ron-audit]
    REG[svc-registry]
  end

  SDK --> GTW
  GTW --> CAS
  GTW --> IDX
  IDX --> DHT
  DHT --> OVY
  OVY --> TRN
  KRN --> BUS
  PAS --> GTW
  KMS --> PAS
  MET -.-> GTW
  MET -.-> TRN
  POL -.-> GTW
  AUD -.-> GTW
  REG -.-> OVY
````

---

# 15) Enforcement

* Treat this doc as **source of truth** for what belongs in the **RON-CORE** launch.
* If a proposed change touches deferred areas, label it `plane:VALUE` and park it until Core hits `core-v1.0.0`.

---

```

