

# 🏛 GOVERNANCE.md — God-Tier Template

```markdown
---
title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: YYYY-MM-DD
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ
---

# GOVERNANCE.md

## 0. Purpose

This document defines the **rules of engagement** for this crate’s economic/policy logic.  
It ensures:

- Transparent and auditable decision-making.  
- Enforcement of **economic invariants** (no doubles, bounded issuance).  
- Clear authority boundaries and appeal paths.  
- SLA-backed commitments to external consumers.  

It ties into:  
- **Economic Integrity Blueprint** (no doubles, bounded issuance).  
- **Hardening Blueprint** (bounded authority, key custody).  
- **Perfection Gates A–O** (esp. Gate I: bounded economic invariants, Gate M: appeal paths).  

---

## 1. Invariants (MUST)

Non-negotiable rules:

- [I-G1] No double issuance or double spend.  
- [I-G2] Every ledger entry MUST balance (credits = debits).  
- [I-G3] Reward emission MUST follow defined schedule (no out-of-band minting).  
- [I-G4] All governance actions MUST be logged and auditable.  
- [I-G5] Authority is bounded → no unbounded admin override.  

---

## 2. Roles & Authority

### Roles
- **Policy owner:** defines invariants (ron-policy).  
- **Ledger keeper:** executes settlement, ensures conservation (ron-ledger).  
- **Rewarder:** distributes incentives under caps (svc-rewarder).  
- **Auditor:** external/verifier role, read-only.  

### Authority Boundaries
- Policy can propose, but not execute, ledger changes.  
- Ledger can reject invalid policy requests.  
- Rewarder cannot mint; only distribute within ledger constraints.  
- All roles MUST use capability tokens (macaroons v1).  

---

## 3. Rules & SLAs

- **Ledger settlement SLA:**  
  - 99.9% of settlements within < 5s.  
  - Audit log available within 1s after commit.  

- **Reward schedule:**  
  - Emissions follow pre-defined curve (e.g., halving every N epochs).  
  - Deviations trigger alert + governance freeze.  

- **Appeals & overrides:**  
  - Invalid transaction? → mark disputed, no rollback.  
  - Override possible only via multi-sig governance action.  

---

## 4. Governance Process

- **Proposal lifecycle:**  
  - Draft → Review → Approve → Execute.  
  - Quorum rules: N-of-M signers.  
  - Default reject if quorum not reached in T hours.  

- **Emergency powers:**  
  - Freeze ledger only under majority multi-sig.  
  - Must be disclosed in audit log within 24h.  

- **Parameter changes:**  
  - Emission curve, quotas, SLA changes must go through proposal lifecycle.  

---

## 5. Audit & Observability

- **Audit logs:** append-only, signed.  
- **Metrics:**  
  - `governance_proposals_total{status}`  
  - `ledger_disputes_total`  
  - `rewarder_emissions_total`  
- **Verifiability:** proofs of conservation (ledger) and range checks (rewarder).  
- **Red-team drills:** simulate rogue admin, ensure bounded authority.  

---

## 6. Config & Custody

- Config file MUST declare:  
  - Policy parameters (emission, quotas).  
  - Key custody model (where private keys live).  
  - SLA targets.  

- Custody:  
  - Keys stored in ron-kms or HSM.  
  - No raw private keys in env vars or files.  
  - Rotation policy: every 90 days or after compromise.  

---

## 7. Appeal Path

- Disputes resolved via:  
  1. Multi-sig governance action.  
  2. Transparent entry in ledger (`disputed=true`).  
  3. No silent rollbacks.  

- Escalation:  
  - Step 1: raise via governance bus topic.  
  - Step 2: propose override with quorum.  
  - Step 3: auditor review + public disclosure.  

---

## 8. Acceptance Checklist (DoD)

- [ ] Invariants defined & enforced in code.  
- [ ] Roles and authority boundaries documented.  
- [ ] Governance process implemented (proposal lifecycle).  
- [ ] Metrics and audit logs exported.  
- [ ] SLA thresholds tested and monitored.  
- [ ] Appeal path validated in chaos drill.  

---

## 9. Appendix

- **Blueprints:**  
  - Economic Integrity (no doubles, bounded emission).  
  - Hardening (bounded authority, custody).  
  - Perfection Gates I & M.  

- **References:**  
  - Macaroons v1 capability tokens.  
  - TLA+ sketches (ledger, rewarder flows).  
  - Governance schema in `/docs/spec/governance.md`.  

- **History:**  
  - Record past governance disputes, overrides, freezes.  

```

---

✅ This template is **God-tier** because it:

* Anchors to **invariants** (no doubles, bounded authority).
* Separates **roles** (policy vs. ledger vs. rewarder).
* Defines **rules, SLAs, and appeal paths**.
* Locks in **auditability + observability**.
* Forces **custody & rotation** policy.
* Includes a **checklist for acceptance gates**.

