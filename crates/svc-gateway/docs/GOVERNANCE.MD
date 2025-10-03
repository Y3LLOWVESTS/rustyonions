

---

```markdown
# 🏛 GOVERNANCE.md — `svc-gateway`

---
title: Governance & Economic Integrity — svc-gateway
status: draft
msrv: 1.80.0
last-updated: 2025-10-03
audience: contributors, ops, auditors, stakeholders
crate-type: policy
---

# GOVERNANCE.md

## 0. Purpose

This document defines the **rules of engagement** for `svc-gateway` as the ingress policy and admission-control surface of RustyOnions.  
It ensures:

- Transparent, auditable **admission decisions** (accept/reject/shape).  
- Enforcement of **economic invariants** for multi-tenant fairness (quotas, DRR), avoiding noisy-neighbor harm.  
- Clear **authority boundaries** (who can change limits, when, and how).  
- SLA-backed commitments to external consumers and internal services.  

Ties into:
- **Economic Integrity** (no doubles, bounded issuance) interpreted at ingress as **no double-count or double-use of quota**, and **bounded authority** over resource allocation.  
- **Hardening** (bounded authority, key custody, immutable caps).  
- **Perfection Gates A–O** with emphasis on **Gate I** (bounded economic invariants) and **Gate M** (appeal paths & reversibility).

---

## 1. Invariants (MUST)

**Admission & Accounting**
- **[I-G1] No double-count:** Each accepted request is counted exactly once against the correct tenant/principal bucket; retries with the same idempotency key do not double-charge.
- **[I-G2] No quota bypass:** No route, header, or encoding path may bypass rate/quota enforcement; early shed happens **before** expensive work.
- **[I-G3] Deterministic decisions:** Given `(principal, route, time_bin, config_version)`, the decision (ACCEPT/REJECT/SHAPE) is deterministic and reproducible from logs.

**Safety & Boundaries**
- **[I-G4] Hard caps immutable at runtime:** Global safety caps (e.g., body ≤ 1 MiB, decode ratio ≤ 10×, decoded abs ≤ 8 MiB) cannot be raised in prod except via signed governance override (multi-sig) and time-bounded policy.
- **[I-G5] Authority is bounded:** No single operator can widen global safety caps or tenant ceilings; overrides require quorum (see §4).
- **[I-G6] Auditability:** Every governance-relevant action (policy update, cap change, override, freeze) emits a signed, append-only audit event with diff.

**Fairness & Integrity**
- **[I-G7] DRR fairness preserved:** Weighted Deficit Round Robin cannot starve any active tenant with a non-zero weight; weights must sum to a bounded value and be validated at load.
- **[I-G8] Error taxonomy stability:** Policy outcomes map to a stable status + JSON schema (e.g., `429 quota`, `413 body-cap`, `413 decoded-cap`, `503 degrade`).

---

## 2. Roles & Authority

### Roles
- **Policy Owner (ron-policy / Governance Council):** Defines global ingress policy templates, safety caps, and change windows.
- **Gateway Operator (Ops):** Operates instances; may **tighten** limits locally (never widen beyond global bounds).
- **Tenant Admin:** Manages tenant-scoped quotas within allowed envelope; cannot exceed global/cluster caps.
- **Auditor (Read-only):** Verifies logs, metrics, and signatures; can trigger audits and freeze proposals.

### Authority Boundaries
- **Global Safety Caps:** Only Policy Owner can propose widening; requires **multi-sig quorum** to enact (see §4). Operators may tighten.
- **Tenant Quotas/Weights:** Tenant Admin can request increases up to policy ceilings; automatic approval may apply within band; above band requires policy review.
- **Runtime Flags:** `danger_ok=true` is **non-prod only** and cannot be set in prod builds/config; prod binaries refuse to start with this flag.
- **Capability Tokens:** All role actions are scoped via macaroons with caveats (time, route, tenant, CIDR). Gateway never accepts ambient admin.

---

## 3. Rules & SLAs

**Ingress SLAs (under healthy downstreams)**
- **Availability:** Control surface (`/healthz`, `/readyz`, `/metrics`) ≥ 99.95% monthly.  
- **Latency:** Intra-region p95 GET < 80 ms; inter-region p95 GET < 200 ms.  
- **Error Budget:** 5xx < 0.1%; **429/503 < 1%** sustained over 5 minutes at declared capacity.

**Quota & Fairness**
- **Ceilings:** Instance RPS cap defaults to 500 rps; inflight cap ≤ 512; DRR weights validated on deploy.  
- **Isolation:** Per-tenant token buckets ensure no tenant can exceed its ceiling; burst shaping is logged.

**Audit Timeliness**
- Governance/audit events must be visible in the audit log **≤ 1s** after commit and retained ≥ 365 days.

**Breach Handling**
- Sustained breach of SLAs triggers automatic **degrade-first** mode (writes shed), page to Ops, and opens a governance incident.

---

## 4. Governance Process

**Proposal Lifecycle (caps, quotas, weights, policy templates)**
1. **Draft:** Proposer (Policy Owner or Tenant Admin within bounds) submits change with rationale, risk, and rollback plan.
2. **Review:** Security & Ops sign-off; automatic checks validate invariants ([I-G1..I-G8]).
3. **Approve:** **Quorum N-of-M multi-sig** (configurable per environment) signs the proposal artifact.
4. **Execute (Staged):** Canary 5% of traffic → 25% → 100% with automated metrics gating; auto-rollback on red metrics.
5. **Record:** Signed audit event persisted and broadcast to `governance.*` bus topics.

**Emergency Powers**
- **Freeze:** Majority multi-sig may place gateway in **degrade-only** mode (reads only) or full **admission freeze**. Must publish signed event and post-mortem within **24h**.
- **Temporary Override:** Time-boxed widening for incident mitigation (e.g., set RPS ceiling +10% for ≤ 2h). Requires multi-sig; auto-expires; must include rollback plan.

**Change Windows**
- Prod changes occur within published windows with on-call coverage; outside windows require incident ticket + multi-sig.

---

## 5. Audit & Observability

**Audit Logs (append-only, signed)**
- `policy.update` (diff, signatures, config_version)
- `quota.change` (tenant id, old/new, scope, TTL)
- `override.issue|expire` (reason, scope, TTL, signers)
- `freeze.enable|disable` (scope, reason)

**Metrics (minimum)**
- `governance_proposals_total{status}`
- `quota_exhaustions_total{tenant}`
- `rejected_total{reason}` with stable `"reason"` taxonomy
- `drr_weight_sum` gauge and per-tenant `drr_weight`
- `gateway_degraded{bool}`

**Verifiability**
- **Proofs of accounting:** For any window, sum(accepted_by_tenant) ≤ global ceilings; per-tenant acceptance equals bucket grants − rejects within error margin.  
- **Range checks:** Safety caps present and equal configured values across replicas (config_version match).

**Red-Team Drills**
- Simulate rogue operator attempting to widen cap; expect rejection without multi-sig and an audit event.

---

## 6. Config & Custody

**Config Declares**
- Global safety caps (body cap, decode ratio/abs).  
- Instance ceilings (RPS, inflight).  
- DRR weights default set and allowed ranges.  
- Governance parameters (quorum M, N, change windows).  
- SLA targets & alert thresholds.

**Key Custody**
- TLS keys and macaroon root/verification keys reside in **ron-kms/HSM**; no raw private keys on disk/env.  
- **Rotation:** Keys rotated every **90 days** or on compromise; macaroon caveats enforce short TTLs (hours-days).  
- **Separation of Duties:** Signers for governance not co-located with operational SSH keys.

---

## 7. Appeal Path

**Disputes**
1. Tenant opens dispute referencing `X-Corr-ID` and `audit_event_id`.  
2. Gateway marks related ledger/audit entries `disputed=true` (no silent rollback).  
3. Policy Owner reviews; if change required, submit governance proposal with fast-track multi-sig.

**Escalation**
- If unresolved in **T=72h**, escalate to Governance Council; publish outcome and rationale.  
- All appeals and outcomes are appended to the audit log.

---

## 8. Acceptance Checklist (DoD)

- [ ] Invariants [I-G1..I-G8] implemented & enforced by tests/CI.  
- [ ] Roles and authority boundaries documented in CONFIG & enforced at runtime.  
- [ ] Multi-sig proposal lifecycle implemented; artifacts signed & archived.  
- [ ] Metrics & audit logs exported; dashboards & alerts wired.  
- [ ] SLA thresholds tested (perf/soak) and monitored in prod.  
- [ ] Appeal path validated in quarterly governance drill.  
- [ ] Emergency freeze & temporary override tested with auto-expiry.

---

## 9. Appendix

**Blueprint Anchors**
- *Economic Integrity:* no double-counting, bounded issuance → ingress quotas & deterministic accounting.  
- *Hardening:* bounded authority, custody/rotation, non-bypass safety caps.  
- *Perfection Gates:* **I** (bounded invariants), **M** (appeal path), **F/L** (perf/chaos gating supports governance).

**References**
- Macaroons v1 capability tokens (short-TTL, caveats).  
- Governance bus topics schema: `governance.*` (proposal, approval, execute, override, freeze).  
- TLA+ sketches: DRR fairness and token-bucket conservation (optional).  
- `/docs/spec/governance.md` (if present) for schema specifics.

**History**
- Record governance incidents (freezes, overrides), with timestamps, signers, and outcomes to build institutional memory.

```

---
