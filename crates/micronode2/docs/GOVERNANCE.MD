
---

```markdown
# 🏛 GOVERNANCE.md — micronode

---
title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-09-24
audience: contributors, ops, auditors, stakeholders
crate: micronode
crate-type: node
pillar: 8
inherits-from: /docs/GOVERNANCE_BLUEPRINT.MD
---

## 0. Purpose

`micronode` is a **single-tenant node profile** with **amnesia ON by default**.  
This document defines the **rules of engagement** for **operational governance** (policy, quotas, security posture, audit) as applied to micronode. It is **not** an economic authority: **no minting, no settlement, no ledger writes**. Micronode **enforces** policy locally and **delegates** economic operations to authoritative services.

Ensures:
- Transparent and auditable **operator actions** and **policy toggles**.
- Enforcement of **economic integrity boundaries** (no doubles, no local mint).
- Clear **authority limits** and **appeal paths** to policy/registry/ledger services.
- SLA-backed commitments for governance surfaces (readiness, audit, metrics).

Ties into:
- **Economic Integrity Blueprint** (no doubles, bounded issuance).
- **Hardening Blueprint** (bounded authority, key custody, amnesia).
- **Scaling Blueprint** (degraded modes, shed-writes).
- **QUANTUM.md** (PQ posture & toggles).
- **Perfection Gates A–O** (Gate I: economic invariants; Gate M: appeal paths).

> **Conflict rule:** If anything here conflicts with repo-level blueprints, **the blueprints win**.

---

## 1. Invariants (MUST)

Non-negotiable rules for micronode:

- **[I-G1] No economic authority:** Micronode **cannot** mint, settle, or mutate ledger state. Economic truth lives in `ron-ledger` / `svc-rewarder`.
- **[I-G2] Capability-only control:** All privileged actions require **capability tokens (macaroons v1)** with least privilege & short TTL; no ambient admin.
- **[I-G3] Append-only audit:** Every governance-relevant action (policy change, PQ toggle, amnesia flip, freeze/unfreeze) **MUST** be logged to an **append-only, tamper-evident** log (BLAKE3 chained).
- **[I-G4] Bounded authority:** No unbounded override. Emergency controls are **time-boxed**, **scoped**, and **multi-sig where configured**.
- **[I-G5] No doubles / conservation boundary:** Any local counters/credits are **non-authoritative**; reconciliation against ledger is mandatory (no double issuance or spend represented locally as truth).
- **[I-G6] Amnesia honored:** In amnesia mode, micronode **must not persist** secrets/policy beyond RAM. Any policy state persisted when amnesia=ON is a defect.
- **[I-G7] PQ policy is governance-scoped:** Enabling `pq_hybrid` / `pq_only` is a **governance action** and must be auditable and reversible.

---

## 2. Roles & Authority

### Roles (micronode context)
- **Local Operator (Ops):** manages config, toggles features, performs freezes/drains.
- **Policy Client (`ron-policy`):** provides signed policy bundles (quotas, residency).
- **Registry (`svc-registry`):** supplies region/topology, signer sets, multi-sig rules.
- **Ledger Keeper (`ron-ledger`):** authoritative balances/settlement (read-only from micronode).
- **Rewarder (`svc-rewarder`):** distributions within ledger constraints (micronode observes only).
- **Auditor:** read-only access to logs/metrics and governance snapshots.
- **Automation (CI/CD):** can roll staged, signed config changes within scoped caps.

### Authority Boundaries
- **Micronode** may **enforce** quotas, PQ posture, admission control, and degraded modes; it may **not** alter ledger truth.
- **Policy** **proposes**; micronode **enforces** or **rejects** if invalid/incompatible (with structured reason).
- **Rewarder**/**Ledger** are **read-only** to micronode; any “preview” balances shown in UI must be labeled **non-authoritative**.
- All privileged APIs require **cap tokens**; emergency paths require **elevated caps** and (optional) **N-of-M** approval from registry policy.

---

## 3. Rules & SLAs

### Governance Rules
- **Config provenance:** Only signed configs (optionally by registry keyset) are accepted in **strict mode**; dev mode may allow unsigned but still audited.
- **PQ posture:** `pq_hybrid` default follows QUANTUM milestones; `pq_only` requires explicit governance action and a rollback window.
- **Amnesia toggle:** `MICRO_PERSIST=0→1` (or reverse) is a governance action; must declare retention boundaries & purge plan.

### SLAs (local surfaces)
- **Readiness governance:** `/readyz` reflects degrade/freeze within **≤ 1s** of action.
- **Audit visibility:** governance log entries available via API/UDS within **≤ 1s** after commit.
- **Freeze/Unfreeze latency:** shed-writes begins **≤ 2s** after freeze command; unfreeze clears within **≤ 2s**.
- **Config reload:** new policy snapshot becomes active within **≤ 3s** or fails atomically with a reason.

---

## 4. Governance Process

### Proposal Lifecycle (strict mode)
1. **Draft** (local change or upstream policy bundle).
2. **Review** (lint + policy compatibility + dry-run).
3. **Approve** (N-of-M, per registry rules) → signed envelope.
4. **Execute** (atomic apply) → append audit entry → emit metrics.

**Timeouts:** default reject if quorum not reached within **T=24h** (configurable).

### Emergency Powers (scoped)
- **Freeze writes / shed mode**, **PQ-only**, **maintenance window**.
- Must be **time-boxed** (default 60 min), **audited**, and require **elevated cap**; optionally **multi-sig**.
- Post-mortem entry appended within **24h**.

### Parameter Changes
- Quotas, PQ posture, degrade thresholds, SLA targets → go through lifecycle above (dev mode may auto-approve but still audit).

---

## 5. Audit & Observability

- **Audit log:** append-only with **BLAKE3 chaining**; entries include `who`, `cap_id`, `what`, `scope`, `ttl`, `reason`, `corr_id`, `old→new`, `timestamp`, `hash_prev`, `hash_curr`.
- **Metrics (Prometheus):**
  - `governance_actions_total{action, outcome}`  
  - `governance_freezes_total{outcome}`  
  - `pq_policy_changes_total{mode}`  
  - `amnesia_toggle_total{from,to}`  
  - `audit_write_latency_seconds` (histogram)  
  - `sla_breach_total{type}` (e.g., approval_timeout, apply_failed)  
- **Dashboards:** Governance overview (action rates, failures), SLA panel, PQ posture.
- **Red-team drills:** simulate rogue admin and cap escalation attempts; verify rejection paths, alerts, and immutable audit.

---

## 6. Config & Custody

- **Config must declare:**
  - Policy parameters (quotas, degrade rules, PQ defaults).
  - Governance mode: **dev** (auto-approve) vs **strict** (signed + quorum).
  - SLA targets & timeouts.
  - Audit sink (file/UDS/remote), retention window (note: amnesia may force RAM-only).
- **Key custody:**
  - Private keys live in **`ron-kms`** or HSM; **never** in env/files.
  - Rotation every **90 days** or on compromise.
  - Cap tokens are short-TTL, audience-scoped, and logged by **key ID only**.

---

## 7. Appeal Path

- **Local dispute:** mark **`disputed=true`** in audit stream; no silent rollbacks.
- **Escalation chain:**
  1. Publish to **governance bus topic** (kernel bus).
  2. Raise **policy override proposal** to registry signers (N-of-M).
  3. Auditor review + disclosure (public or designated channel).
- **Final authority:** registry/policy signers; micronode executes or continues shed mode per decision.

---

## 8. Acceptance Checklist (DoD)

- [ ] Invariants [I-G1…G7] enforced by code/config tests.  
- [ ] Roles & boundaries documented in `/docs/CONFIG.md` and reflected in caps.  
- [ ] Proposal lifecycle implemented (dev/strict modes) with audit entries.  
- [ ] Metrics emitted & dashboards wired; alerts on `sla_breach_total > 0`.  
- [ ] SLA thresholds tested in CI (governance jobs) and during drills.  
- [ ] Appeal path validated via chaos exercise (freeze → dispute → quorum → unfreeze).  
- [ ] QUANTUM and SECURITY cross-links updated (PQ toggles, amnesia).

---

## 9. Appendix

**Blueprints:** Economic Integrity; Hardening; Scaling; QUANTUM; Six Concerns; Microkernel.  
**References:** Macaroons v1 caps; BLAKE3 chaining for audit; TLA+ sketches for governance FSM (propose→approve→execute) and degrade/recover safety.  
**Schemas:** Governance envelope & audit entry under `/docs/spec/governance.md` (IDs, signer sets, hash chaining).  
**History:**  
- _2025-09-24_: Initial micronode governance doc; clarified **no economic authority**, added PQ posture as governance-scoped, and time-boxed emergency powers.
```

---
