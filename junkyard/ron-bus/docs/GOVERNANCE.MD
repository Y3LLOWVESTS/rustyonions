

---

Here’s the **refactored, GOVERNANCE.md for ron-bus**, scoped to its role:

```markdown
# 🏛 GOVERNANCE.md — ron-bus

---
title: Governance & Integrity Guarantees
status: draft
msrv: 1.80.0
last-updated: 2025-09-27
audience: contributors, ops, auditors, stakeholders
crate-type: lib
---

# GOVERNANCE.md

## 0. Purpose

`ron-bus` is a **pure in-process broadcast bus** (Tokio `broadcast` abstraction).  
It does not mint, settle, or distribute value — but it is the **substrate through which governance events flow**.

This document defines:

- The **rules of engagement** for bus-mediated governance.  
- How we prevent the bus from becoming an **unbounded authority vector**.  
- How bus-delivered governance actions are **auditable, bounded, and observable**.  
- SLA expectations for propagation of governance-critical events.

It ties into:  
- **Microkernel Blueprint** (Bus as frozen API).  
- **Hardening Blueprint** (bounded authority, no secrets).  
- **Perfection Gates I & M** (bounded invariants, appeal paths).

---

## 1. Invariants (MUST)

- [I-B1] Bus is **bounded, lossy, observable**: all drops accounted in metrics.  
- [I-B2] **No hidden control channels**; all governance events use the canonical bus.  
- [I-B3] **No unbounded authority escalation**: bus only carries events, it does not enforce policy.  
- [I-B4] **All governance events are auditable**: ConfigUpdated, Shutdown, ServiceCrashed must be emitted and traceable.  
- [I-B5] **Neutrality**: bus cannot privilege one subscriber over another; fairness is at Tokio broadcast semantics.  

---

## 2. Roles & Authority

- **Policy crates** (e.g., ron-policy): publish governance proposals.  
- **Ledger/rewarder services**: subscribe to enforce economic invariants.  
- **Auditors**: subscribe read-only, verify governance event flow.  
- **ron-bus**: neutral substrate; cannot veto, mint, or override — only fan-out events.

---

## 3. Rules & SLAs

- **Propagation SLA:** governance events must be observable by subscribers within **1 tick** of publish.  
- **Audit SLA:** overflow/loss must increment `bus_lagged_total` counter; zero “silent drops.”  
- **Authority SLA:** bus never generates events on its own; only relays producer messages.  
- **Appeal SLA:** disputed governance actions surface as events (`disputed=true` in payload), never silent rollbacks.  

---

## 4. Governance Process (Bus Role)

- **Proposal lifecycle:** proposals flow *through* bus (`KernelEvent::ConfigUpdated`, etc.); bus does not judge.  
- **Emergency powers:** Freeze/Shutdown are represented as events; bus cannot initiate them.  
- **Quorum rules:** enforced by consumers (policy/ledger), not by bus. Bus ensures all subscribers see the same events.  

---

## 5. Audit & Observability

- **Audit logs:** rely on upstream (svc-index, ron-ledger). Bus exposes counters (`bus_lagged_total`, `queue_depth`).  
- **Metrics:**  
  - `bus_lagged_total` — dropped messages.  
  - `bus_queue_depth` — instantaneous backlog.  
  - `service_crashed_total` — if subscribers report crash.  
- **Verifiability:** subscribers can replay event streams; lag is measurable.  
- **Red-team drill:** simulate subscriber overload → verify drops are accounted, not silent.  

---

## 6. Config & Custody

- **Config:** capacity, overflow policy. No economic parameters.  
- **Custody:** no keys or secrets; bus must never carry macaroon/capability tokens directly.  
- **Rotation policy:** N/A — handled in upstream auth services.  

---

## 7. Appeal Path (Bus-Specific)

- **If a governance event is disputed:**  
  - Publish `KernelEvent::ServiceCrashed { service, reason="dispute" }` or domain-specific “disputed” event.  
  - No hidden retractions — disputes must be explicit.  
- **Escalation:** auditors subscribe to bus; external governance crates enforce multi-sig overrides.  

---

## 8. Acceptance Checklist (DoD)

- [ ] Bus invariants enforced (bounded, lossy, observable).  
- [ ] Governance events published via canonical API.  
- [ ] Overflow/drops observable in metrics.  
- [ ] No authority escalation inside bus code.  
- [ ] Appeal path documented & tested.  

---

## 9. Appendix

- **Blueprints:**  
  - Microkernel (Bus invariants).  
  - Hardening (bounded authority, auditability).  
- **References:**  
  - `KernelEvent` enum (ConfigUpdated, ServiceCrashed, Shutdown).  
  - Perfection Gates I (bounded invariants), M (appeal paths).  
- **History:**  
  - Record any governance disputes where bus delivery was implicated.  

---
```

---

🔑 **My view:** This reframing makes `ron-bus`’s GOVERNANCE.md *canon-pure*:

* No drift into economic issuance (not its job).
* Locks in its true governance role: **neutral, bounded, auditable transport of governance events.**
* Perfectly aligned with your Six Concerns spine: GOV = appeal paths, RES = boundedness, OBS = metrics, SEC = no secrets.

