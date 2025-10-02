```markdown
# 🏛 GOVERNANCE.md 

---
title: Governance & Operational Integrity — ron-transport
status: draft
msrv: 1.80.0
last-updated: 2025-10-01
audience: contributors, ops, auditors, stakeholders
crate-type: policy|ops
---

# GOVERNANCE.md

> **Scope note:** `ron-transport` is a **transport library** (TCP/TLS with optional Tor via Arti).  
> It holds **no monetary state**. Throughout this document, “economic integrity” maps to
> **operational integrity of scarce resources** (FDs, CPU, bandwidth) and **bounded authority**
> over safety-critical limits (frame sizes, compression guards, rate caps, readiness semantics).

---

## 0. Purpose

This document defines the **rules of engagement** for `ron-transport` policy and operations.  
It ensures:

- Transparent, auditable decision-making for **defaults and hard limits**.
- Enforcement of **operational invariants** (no unbounded buffers, bounded inflation, single writer).
- Clear **authority boundaries**, change control, and **appeal paths**.
- SLA-backed commitments to embedding services.

It ties into:  
- **IDB (v0.1.2)**: I-1…I-18 invariants (caps, readiness, amnesia, Arti in-proc).  
- **Hardening Blueprint**: quotas-before-work, fixed buckets, decompression guard.  
- **Scaling & Concurrency Blueprints**: single writer, no locks across `.await`.  
- **Perfection Gates A–O** (esp. A, I, J, K, L, M, O).  

---

## 1. Invariants (MUST)

Non-negotiable, CI-enforced, and **release-gated**:

- **[I-G1] Hard caps cannot be exceeded at runtime.**  
  OAP/1 **frame ≤ 1 MiB**, chunk ≈ **64 KiB**, decompression guard **≤ 8×** (stricter than global).  
  Config may **lower** caps but may not raise beyond these limits.

- **[I-G2] Quotas before work.**  
  All I/O applies timeouts, rate limits (per-conn, per-peer, global) and connection caps **before** allocating/buffering.

- **[I-G3] Single writer; no locks across `.await`.**  
  Concurrency model is invariant; violations are release blockers.

- **[I-G4] Stable readiness contract.**  
  `Ready | Degraded{reason} | NotReady{reason}` semantics and reason strings are **stable identifiers**.

- **[I-G5] Error taxonomy stability.**  
  Errors come from a `#[non_exhaustive]` enum with **stable `kind()` strings**; changes require major version bump.

- **[I-G6] In-proc Arti only.**  
  No management of external `tor` daemons; Arti runs in-proc under `arti` feature; amnesia forbids on-disk caches.

- **[I-G7] Buckets are fixed.**  
  Histogram bucket edges for handshake/size/throughput are defined in IDB and **not configurable**.

- **[I-G8] Config reload is atomic with rollback.**  
  On invalid reload, **retain last-good**; emit structured audit event.

- **[I-G9] PQ toggles are config-only.**  
  Enabling PQ-hybrid (when stack supports) must **not** alter public API; annotate metrics with `pq="hybrid"`.

- **[I-G10] No `unsafe` on hot paths.**  
  Any `unsafe` must be cold-path, justified, and covered by tests; violations block release.

---

## 2. Roles & Authority

### Roles (RACI-Style)
| Role                     | Responsibilities                                                          | R | A | C | I |
|--------------------------|---------------------------------------------------------------------------|---|---|---|---|
| **Maintainers**          | Own IDB/CONFIG defaults, invariants, releases                            | ✅ | ✅ | ✅ | ✅ |
| **Security (SECOps)**    | Key custody policy, amnesia/PQ posture, review hardening changes         |   | ✅ | ✅ | ✅ |
| **SRE / Service Owners** | Operate embedding services; tune **runtime** knobs within hard caps      | ✅ |   | ✅ | ✅ |
| **Release Manager**      | Cut releases, ensure gates (Bronze→Gold) are green                       |   | ✅ | ✅ | ✅ |
| **Auditors**             | Verify invariants, buckets, readiness reasons; review logs/metrics       |   |   | ✅ | ✅ |

### Authority Boundaries
- **Maintainers** may adjust **defaults** and **hard caps** via governed releases only.  
  Raising hard caps (e.g., frame > 1 MiB or guard > 8×) requires **major version bump + quorum** (see §4).
- **SREs** may change **runtime config** (timeouts, rate limits, per-peer caps), **never** beyond hard caps.
- **Security** must sign off on any change affecting TLS/Arti/amnesia/PQ semantics.

---

## 3. Rules & SLAs

### Runtime SLAs (as surfaced by embedding services)
- **Readiness:** TCP/TLS path **< 2 s** typical; Arti path **< 30 s** (bounded by `arti.bootstrap_timeout`).
- **Handshake latency (TLS, intra-AZ):** **p95 < 80 ms**, **p99 < 150 ms** on reference rig.  
- **Reject health:** `rejected_total{reason}` **< 1%** at target load.

### Governance Rules
- **No silent drift:** bucket edges, reason strings, and error kinds are **frozen identifiers**.
- **Strict backward compatibility:** public error kinds/reason labels require **semver discipline**; removals = major bump.
- **Kill-switch discipline:** Disabling accept/dial must be done via service controls (listener drain) — **not** by relaxing invariants.

### Compliance Signals (Operators must alert on)
- **Degraded > 60 s**, **NotReady transition**, handshake p95 SLA breach, `too_large` spikes, sustained `rate_limited` > 1%.

---

## 4. Governance Process

### Change Classes & Required Approvals
| Class | Examples                                             | Approvals (Quorum)                              | Release |
|------:|------------------------------------------------------|--------------------------------------------------|---------|
| **0** | Hard caps / invariants (1 MiB, 8×, readiness model) | **2/3 Maintainers + Security**                  | **Major** |
| **1** | Defaults (timeouts, buckets list)                    | **2 Maintainers** + FYI Security                | Minor   |
| **2** | Docs/runbook/tests only                              | 1 Maintainer                                    | Patch   |
| **3** | Runtime config (SRE knob within caps)               | Service Owner                                   | N/A     |

### Proposal Lifecycle
1. **Draft (PR)** with motivation, blast radius, and test deltas.  
2. **Review Window:** 72 h (business days) minimum for Class 0/1.  
3. **Approval:** Quorum per table; Security required for TLS/Arti/amnesia/PQ.  
4. **Execute:** Merge + release; **post-release audit** (10 min) per runbook.  
5. **Fallback:** Revert policy (Class 0/1) only via a second governed PR.

### Emergency Powers
- **Emergency freeze** (e.g., CVE): Service owners **drain and stop accepting**; Maintainers may hotfix **lowering** caps.  
- Any emergency action must land an **audit entry within 24 h**, and a normal proposal within 72 h.

### Parameter Changes
- PQ enablement, per-peer caps defaults, bootstrap timeouts: Class 1.  
- Bucket edges or reason labels: Class 0 (frozen identifiers).

---

## 5. Audit & Observability

- **Audit Events** (structured JSON; emitted by embedding service using library hooks):
  - `TransportPolicyChanged{class, old, new, actor, pr, commit, ts}`
  - `TransportConfigReloaded{result, version}`
  - `TransportReadinessTransition{from, to, reason}`
- **Metrics (required exposure by service):**
  - `transport_dials_total{backend,result}`
  - `transport_accepts_total{backend}`
  - `handshake_latency_seconds{backend}` (fixed buckets)
  - `rejected_total{reason}` with canonical reasons:
    `timeout|tls|too_large|rate_limited|peer_limit|arti_bootstrap`
- **Verifiability:** Nightly **Gold** job runs: fuzz ≥4 h, loom models, 24 h soak, perf snapshot; artifacts retained 30 days.
- **Red-team drills:** Quarterly simulation: rogue admin attempts to raise caps beyond hard limits → rejected by library + CI.

---

## 6. Config & Custody

- **Config** (see CONFIG.md): precedence, atomic reload, rollback on failure.
- **Custody:**
  - TLS private keys: KMS/HSM or secrets store (**0600** on disk if mounted).
  - CA bundles: mounted read-only; prefer `trust_store="system"`.
  - **Amnesia=ON**: no persistent artifacts (incl. Arti caches).
- **Rotation:**
  - TLS cert/keys: as per org policy (≤90 days recommended).
  - Capability tokens (macaroons): ≤30 days, never logged.

---

## 7. Appeal Path

- **Trigger:** Disagreement on Class 0/1 change, or observed SLA harm.
- **Steps:**
  1. Open **Governance Issue** with impact evidence (metrics/logs).  
  2. Request review on governance bus topic (stakeholders pinged).  
  3. If unresolved in 72 h, escalate to Security + Release Manager for **tie-break**.  
  4. Final decision recorded in `GOVERNANCE_LOG.md` with rationale and links.

- **No silent rollbacks.** All reversions follow the same governed path (Class 0/1).

---

## 8. Acceptance Checklist (Definition of Done)

- [ ] **Invariants enforced** in code (caps, guard, concurrency, readiness).  
- [ ] **Authority boundaries** documented (this file) and honored in CI.  
- [ ] **Governed buckets & reason strings** snapshotted in tests.  
- [ ] **Audit events & metrics** wired in embedding services.  
- [ ] **SLA thresholds** tested (perf/soak) and alerting configured.  
- [ ] **Appeal path** validated in a quarterly tabletop or chaos drill.  

---

## 9. Appendix

### 9.1 References
- `docs/IDB.md` (v0.1.2), `docs/CONFIG.md`, `docs/RUNBOOK.md`, `docs/TESTS.md`  
- Blueprints: Hardening, Concurrency & Aliasing, Scaling, Omnigate  
- TLS 1.3 + PQ hybrids (when available via rustls stack)  
- Macaroons (capabilities) for operator/API access, when used by services

### 9.2 History (example entries)
- 2025-09-18 — Adopt **8×** decompression guard (stricter than global); Class 0; quorum 3/3 + Security ✅  
- 2025-09-25 — Added per-peer defaults (Class 1); Maintainers 2/2; perf canary ✅

### 9.3 Risk Matrix (summary)

| Risk                                      | Class | Guardrail                                           |
|-------------------------------------------|:----:|-----------------------------------------------------|
| Frame cap increase > 1 MiB                |  0   | Major release + quorum; CI snapshot tests fail     |
| Disable decompression guard               |  0   | Forbidden; test hard-fails                         |
| Change reason labels/buckets              |  0   | Frozen IDs; breaking change tagged major           |
| PQ hybrid toggle default                  |  1   | Security sign-off; canary with rollback plan       |
| Runtime caps (within hard limits)         |  3   | SRE-owned; alerts on regressions                   |

---

**Governance Golden Rule:**  
When in doubt, **prefer stricter defaults** and **explicit canaries** over silent widening of limits.  
`ron-transport` is the substrate—keep it predictable, bounded, and boring.
```
