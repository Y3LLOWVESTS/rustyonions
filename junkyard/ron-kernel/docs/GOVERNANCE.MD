
---

**File:** `crates/ron-kernel/docs/GOVERNANCE.md`

```markdown
# 🏛 GOVERNANCE.md 

---
title: Governance & Economic Integrity (Kernel-Scoped)
status: draft
msrv: 1.80.0
last-updated: 2025-09-26
audience: contributors, ops, auditors, stakeholders
crate-type: kernel (code/process governance; not economic policy)
---

# GOVERNANCE.md — ron-kernel

## 0. Purpose

This document defines the **rules of engagement** for the `ron-kernel` crate’s governance: how we change it, who can change it, and the **non-negotiable boundaries** that keep the kernel tiny, stable, and orchestration-only.  
It enforces:

- Transparent, auditable decision-making (RFCs/ADRs, CI gates, metrics & events).  
- **Pillar-1** constraints: frozen public API, bounded queues, supervision, observability. :contentReference[oaicite:0]{index=0}  
- Clear **authority boundaries**: kernel is not policy/econ/overlay/storage; those belong to other crates/services. :contentReference[oaicite:1]{index=1}

**Scope note:** Traditional “economic integrity” (issuance, rewards, ledger) is **out of scope** for the kernel and lives in `ron-ledger`, `ron-accounting`, `ron-policy`, `svc-rewarder`. Kernel governance focuses on **code/process** and interop boundaries. :contentReference[oaicite:2]{index=2}

---

## 1. Invariants (MUST)

Non-negotiable rules for `ron-kernel`:

- [I-K1] **Public API is frozen**: re-exports = `{ Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c }`. Any change is a **major** bump with migration notes. :contentReference[oaicite:3]{index=3} :contentReference[oaicite:4]{index=4}  
- [I-K2] **Kernel boundaries**: kernel **is** supervision/config/health/bus/observability; kernel **is not** overlay/DHT/storage/ledger/gateway. :contentReference[oaicite:5]{index=5}  
- [I-K3] **Concurrency**: no locks across `.await`; **bounded** mailboxes/bus; lag/drop metrics present. :contentReference[oaicite:6]{index=6}  
- [I-K4] **Observability, not enforcement**: kernel exposes OAP/BLAKE3/TLS invariants; services enforce wire rules (e.g., `max_frame=1 MiB`, `DATA` header `obj:"b3:<hex>"`). :contentReference[oaicite:7]{index=7}  
- [I-K5] **TLS type**: any TLS touched by kernel uses `tokio_rustls::rustls::ServerConfig` (never `rustls::ServerConfig`). :contentReference[oaicite:8]{index=8}  
- [I-K6] **Amnesia mode surfaced** as a global flag + `amnesia="on|off"` metrics label; services must honor semantics (RAM-only, zeroization). :contentReference[oaicite:9]{index=9} :contentReference[oaicite:10]{index=10}  
- [I-K7] **PQ neutrality & pass-through**: kernel does not block PQ; passes transport/proto PQ flags and surfaces posture in health snapshots. :contentReference[oaicite:11]{index=11}

---

## 2. Roles & Authority

### Roles
- **Kernel Maintainers** — own API stability & boundaries; gate PRs per Pillar-1 checklist. :contentReference[oaicite:12]{index=12}  
- **Release Manager** — tags releases; ensures semver discipline & migration notes on majors. :contentReference[oaicite:13]{index=13}  
- **Security/Hardening Reviewer** — checks TLS type, amnesia surfacing, and observability of rejects. :contentReference[oaicite:14]{index=14}  
- **SRE/Operators** — consume kernel events/metrics; validate readiness behavior under load. :contentReference[oaicite:15]{index=15}  
- **Auditors** — verify invariants via tests/vectors and acceptance gates. :contentReference[oaicite:16]{index=16}

### Authority Boundaries
- Kernel **cannot** add overlay/DHT/storage/ledger logic; reject via review gates. :contentReference[oaicite:17]{index=17}  
- Kernel **may** surface protocol bounds (OAP/TLS/BLAKE3) as metrics/events, **may not** enforce them. :contentReference[oaicite:18]{index=18}  
- Config watcher emits `ConfigUpdated`; rollback for invalid config remains service-side policy. :contentReference[oaicite:19]{index=19}

---

## 3. Rules & SLAs

- **Readiness semantics**: `/healthz` = liveness; `/readyz` = backpressure/supervisor state; services **fail fast** on saturation or restart storms. Kernel must make this state visible. :contentReference[oaicite:20]{index=20}  
- **Bus SLAs**: bounded buffers; lag/drop **observable** via `bus_lagged_total` (and drop counters if present). No blocking in supervisory paths. :contentReference[oaicite:21]{index=21}  
- **Metrics golden set**: `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds` present & tested. :contentReference[oaicite:22]{index=22}

---

## 4. Governance Process

- **Change classes**
  - **Docs/tests only**: maintainer review.  
  - **Non-breaking code**: maintainer + security reviewer sign-off.  
  - **Breaking (public API)**: **RFC/ADR**, quorum of maintainers, **major** version bump, migration notes. :contentReference[oaicite:23]{index=23}

- **Proposal lifecycle**: Draft → Review → Approve → Merge → Release. PRs must pass Pillar-1 acceptance checklist & CI label routing (RES/GOV jobs; loom/fuzz/TLA+ when applicable). :contentReference[oaicite:24]{index=24} :contentReference[oaicite:25]{index=25}

- **Non-goals enforcement**: any PR introducing app/overlay/econ logic is rejected on boundary breach. :contentReference[oaicite:26]{index=26}

---

## 5. Audit & Observability

- **Events**: kernel emits `KernelEvent::{Health,ConfigUpdated,ServiceCrashed{reason},Shutdown}`; crashes include **structured `reason`**. :contentReference[oaicite:27]{index=27} :contentReference[oaicite:28]{index=28}  
- **Metrics**: golden set (above); amnesia label attached when surfaced. :contentReference[oaicite:29]{index=29}  
- **Proofs**: canonical vectors for bus overflow & degraded readiness live under `/tests/vectors/`; formal/perf hooks exist (TLA+ specs, perf harness). :contentReference[oaicite:30]{index=30}

---

## 6. Config & Custody

- **Config**: hot-reload via watcher emits `ConfigUpdated{version}`; invalid configurations are rejected upstream and should not be applied globally. :contentReference[oaicite:31]{index=31}  
- **TLS custody (kernel-adjacent)**: when kernel touches TLS config types, it uses `tokio_rustls::rustls::ServerConfig`. No raw private keys handled by the kernel. :contentReference[oaicite:32]{index=32}  
- **PQ posture**: kernel passes transport/proto PQ flags and surfaces posture—no policy decisions made in kernel. :contentReference[oaicite:33]{index=33}

---

## 7. Appeal Path

1) Raise concern in PR review referencing Pillar-1 acceptance checklist items. :contentReference[oaicite:34]{index=34}  
2) Escalate to Kernel Maintainers quorum; if unresolved, open an ADR capturing the dispute & resolution.  
3) For urgent boundary breaches (e.g., unbounded queues, API drift), **block release** until checklist passes. :contentReference[oaicite:35]{index=35}

---

## 8. Acceptance Checklist (DoD)

- [ ] **API surface unchanged** (or major bump + migration notes if changed). :contentReference[oaicite:36]{index=36}  
- [ ] **No locks across `.await`**; **all channels bounded**. :contentReference[oaicite:37]{index=37}  
- [ ] **Metrics present & exercised**: `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`. :contentReference[oaicite:38]{index=38}  
- [ ] **Readiness degrades/fails** correctly under saturation/restart storms. :contentReference[oaicite:39]{index=39}  
- [ ] **TLS type invariant** satisfied (tokio_rustls wrapper). :contentReference[oaicite:40]{index=40}  
- [ ] **Amnesia surfaced** (flag + label), and services assert honoring. :contentReference[oaicite:41]{index=41}  
- [ ] **OAP exposure only** (no enforcement creep); DATA header rule documented in downstream specs. :contentReference[oaicite:42]{index=42}  
- [ ] **Formal/Perf hooks** present: TLA+ specs, perf harness smoke. :contentReference[oaicite:43]{index=43}

---

## 9. Appendix

- **Blueprints / Canon:**  
  - Microkernel Blueprint (FINAL+) — scope, API, invariants, amnesia, PQ posture. :contentReference[oaicite:44]{index=44} :contentReference[oaicite:45]{index=45}  
  - 12 Pillars — Pillar-1 roles & review gates; 33-crate canon. :contentReference[oaicite:46]{index=46}  
- **References:**  
  - OAP/1 limits & content addressing (exposed by kernel, enforced by services). :contentReference[oaicite:47]{index=47}  
  - CI label routing & verification plan (RES/GOV, loom/fuzz/TLA+). :contentReference[oaicite:48]{index=48}  
- **History:**  
  - Track API changes, boundary exceptions (none allowed), notable readiness incidents and resolutions.

```

---

