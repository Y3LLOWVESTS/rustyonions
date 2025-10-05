

---

```markdown
---
title: Governance & Economic Integrity — svc-storage
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ
---

# GOVERNANCE.md — svc-storage

## 0. Purpose

Defines **rules of engagement** for `svc-storage` as a content-addressed blob store:

- Transparent and auditable decision-making for **quotas, placement, retention, takedowns**.
- Enforcement of **economic invariants** (no double accounting, bounded quotas, no unbounded admin powers).
- Clear **authority boundaries** (policy vs. execution) and **appeal paths**.
- SLA-backed commitments to external consumers (tenants, other services).

**Ties into:**
- Economic Integrity (no doubles; bounded issuance/credits).
- Hardening (bounded authority; key custody; cap tokens).
- Perfection Gates **I** (economic invariants) & **M** (appeal paths), plus **K/L** (operational vigilance & black swan).

---

## 1. Invariants (MUST)

Non-negotiable storage-economics & policy rules:

- **[I-S1] Immutability:** Stored objects are **immutable**; address = `b3:<hex>` derived from full bytes. No in-place mutation or admin rewrite.
- **[I-S2] No double accounting:** Billable **logical bytes** for a tenant are computed once per unique address (per the billing mode; see §6). Physical replication (RF) is accounted **separately** and transparently.
- **[I-S3] Bounded quotas:** Write/egress are gated by explicit **quota caps** (hard/soft). No unlimited admin bypass; break-glass is time-boxed & logged (see §4).
- **[I-S4] Region & residency:** Placement respects **region/zone allowlists**; cross-region replication requires policy approval trail.
- **[I-S5] Retention & purge:** Deletes mark objects **logically unreachable** immediately; **physical purge** must occur within SLA (see §3) unless placed under legal hold.
- **[I-S6] Capability-first access:** All governance actions and API calls require **macaroon capabilities** with caveats (aud, path, method, nbf/exp).
- **[I-S7] Full auditability:** Every governance decision (quota change, takedown, break-glass, policy update) is **append-only**, signed, and queryable.
- **[I-S8] Bounded authority:** No single role can mint credits, override quotas indefinitely, or read bytes without explicit capability & purpose binding.
- **[I-S9] Correctness proofs:** On PUT, the **BLAKE3 digest must match** before success; serving bytes must reproduce the same digest (integrity proof on read).

---

## 2. Roles & Authority

### Roles
- **Policy Owner (Governance Council):** Owns policy defaults (quotas, residency, retention), approves parameter changes via multi-sig.
- **Storage Operator (Svc Owner):** Runs the service, enforces policies, executes purges/repairs, proposes emergency actions.
- **Tenant Admin:** Manages per-tenant quotas/retention under global caps; initiates erasure and legal holds within tenancy boundary.
- **Auditor (Read-only):** Verifies audit trails, metrics, proofs of conservation & purge.
- **Trust & Safety (T&S):** Validates takedown requests (abuse/DMCA/illegal content), can request freeze under policy.

### Authority Boundaries
- Policy can **propose** but not execute storage changes.
- Operators **execute** policy under caps; cannot raise quotas beyond policy limits without multi-sig break-glass (time-boxed).
- Tenant Admins can **lower** retention/quotas and request deletes/erasure for their tenancy; cannot exceed global caps.
- T&S can **freeze** content (no serve/no place) pending review; permanent removal requires recorded decision.
- All actions require **cap tokens**; logs include actor, capability, purpose, and expiry.

---

## 3. Rules & SLAs

### Availability & Settlement
- **Read SLO:** p95 GET < 80 ms (intra-region); p95 Range-start < 100 ms.  
- **Write SLO:** p95 PUT (1 MiB) < 250 ms.
- **Audit log SLO:** Governance/audit entry visible **< 1 s** after commit.

### Quotas & Accounting
- **Hard quotas:** Writes/egress hard-fail with 429/413 once exceeded; no silent debt.
- **Soft quotas:** Early warning at configurable thresholds (e.g., 80/90/95%); auto-notify Tenant Admin.
- **Accounting model:**  
  - **Logical bytes** (unique addresses per tenant) vs. **physical bytes** (RF × logical).  
  - Egress billed by **actual bytes served** (range accounted precisely).

### Residency & Replication
- **Placement policy:** Objects only in **allowed regions**; replication factor (RF) tracked; drift triggers repair pacing.
- **Cross-border movement:** Requires recorded approval; default deny.

### Retention, Deletion & Purge
- **Tenant delete:** Logical unlink **immediate**; physical purge within **24 h** (standard) or **4 h** (high-priority GDPR/abuse).
- **Legal hold:** Overrides purge timers; requires case ID and expiry; auto-review reminder.

### Takedowns & Abuse
- **Freeze SLA:** On valid takedown, freeze serving within **15 min** globally.
- **Final purge:** Within **4 h** unless contested; all steps logged.

### Break-glass (Emergency)
- **Time-boxed override:** 30–60 min max; N-of-M multi-sig; non-extendable without new approval.
- **Scope:** Minimal: e.g., raise egress cap by +10% temporarily; cannot grant raw byte-read without case & purpose.

---

## 4. Governance Process

### Proposal Lifecycle
1. **Draft** (policy owner/operator/tenant admin).
2. **Review** (council + T&S + operator).
3. **Approve** (N-of-M multi-sig; list of signers recorded).
4. **Execute** (operator).
5. **Disclose** (audit log entry with diffs, rationale, expiry if temporary).

**Default outcome:** If quorum not met within **72 h**, proposal **fails**.

### Emergency Powers
- Used only to restore SLOs or mitigate abuse/legal risk.
- Requires explicit **incident ID** and **post-mortem** within **72 h**.
- Immediate disclosure in audit log.

### Parameter Changes (Require Proposal)
- Quotas/egress caps, retention defaults, residency allowlists, RF targets, takedown SLAs, audit SLOs.

---

## 5. Audit & Observability

### Audit Trail (Append-only)
- Each action includes: `ts, actor, role, cap_id, action, scope, before→after, reason, incident_id?, expires?`.
- Entries are **signed** by service identity; optionally anchored to external notarization.

### Metrics (examples)
- `governance_proposals_total{status}`  
- `governance_overrides_total{kind}`  # break-glass, quota_tweak, residency_change…  
- `quota_events_total{tenant,kind}`   # soft_warn, hard_enforce  
- `takedown_requests_total{status}`   # received, frozen, purged, denied  
- `purge_latency_seconds` (histogram)  
- `residency_violations_total` (should be 0)  
- `rf_target` / `rf_observed` (gauges)  
- `accounting_bytes{type}`            # logical, physical, egress

### Verifiability
- **Conservation proofs:** For a sampling window, `physical ≈ RF × logical` (within drift bounds).
- **Purge proofs:** Sampled **proof-of-erasure** (no replica reachable; tombstone present; read returns 404/410).
- **Region proofs:** Sampled placement manifests adhere to allowlists.

### Red-Team Drills (Quarterly)
- Rogue admin attempts override beyond caps → denied & alerted.
- Fake takedown → process requires evidence; no purge without approval.
- Residency drift → repair pacing starts; alert fires.

---

## 6. Config & Custody

### Policy Configuration
- Declarative policy in `svc-storage` config (or central policy service):  
  - **quotas:** per-tenant logical/egress limits  
  - **residency:** region allowlists/denylists  
  - **retention:** default, min/max, legal hold flags  
  - **replication:** RF targets by class/region  
  - **takedown:** evidence requirements, SLAs  
  - **break-glass:** quorum, max duration, allowed scope

Changes go through §4 lifecycle; on success, the **effective policy** is versioned and announced.

### Accounting Modes (declare per deployment)
- **Mode A — Per-Tenant Unique:** Bill only unique addresses within a tenant; global dedupe allowed physically but does **not** reduce tenant’s logical accounting across tenants.
- **Mode B — Global Unique Credit:** (Optional) Global dedupe credits shared—requires cross-tenant policy approval & privacy review.

The **active mode** MUST be published in `/metrics` and audit headers to prevent surprise bills.

### Key Custody
- Governance keys (multi-sig), service identity keys, and cap roots live in **KMS/HSM**.  
- **No raw private keys** in env/files.  
- **Rotation:** every **90 days** or on incident.  
- **Access** requires MFA and is auditable.

---

## 7. Appeal Path

- **Disputes** (quota, purge, takedown) follow the chain:
  1. Open **governance topic** case with evidence (tenant admin or T&S).
  2. Council review; optionally freeze the content/accounting.
  3. If upheld: proposal to adjust/restore; if denied: rationale logged.
- **No silent rollbacks.** All outcomes are recorded with diffs & timestamps.
- **External review:** Auditor can request redaction of sensitive audit fields while preserving integrity.

---

## 8. Rules & SLAs Table (Quick Reference)

| Domain        | Rule / SLA                                   | Bound / Target                         |
|---------------|-----------------------------------------------|----------------------------------------|
| Quotas        | Hard caps, soft warns                         | soft: 80/90/95%; hard: 100%            |
| Residency     | Allowed regions only                          | 0 violations (alert on any)            |
| Retention     | Purge after delete                            | Standard: < 24h; Priority: < 4h        |
| Takedown      | Freeze on valid request                       | < 15 min                               |
| Audit         | Entry availability                            | < 1s after action                      |
| Break-glass   | Duration / quorum                             | ≤ 60 min; N-of-M multi-sig             |
| Accounting    | No double accounting                          | Mode-declared; audit-verifiable        |
| Authority     | No unlimited admin powers                     | Time-boxed, scope-limited overrides    |

---

## 9. Acceptance Checklist (DoD)

- [ ] Invariants [I-S1..I-S9] enforced in code & covered by tests.  
- [ ] Roles & boundaries wired to capabilities (macaroons) with caveats.  
- [ ] Proposal lifecycle & break-glass implemented (multi-sig, time-boxed).  
- [ ] Metrics & audit logs exported; dashboards include governance panels.  
- [ ] Residency/retention/takedown SLAs tested (chaos drills).  
- [ ] Accounting mode published & verified; conservation & purge proofs pass.  
- [ ] Appeal path executed in quarterly tabletop exercise.

---

## 10. Appendix

**Blueprint hooks:**  
- Economic Integrity: *no doubles*, bounded issuance/credits, transparent billing.  
- Hardening: bounded authority; custody in KMS/HSM; capability-first ops.  
- Perfection Gates: **I** (economic invariants), **M** (appeals), **K**/**L** (ops vigilance/black swan).

**References:**  
- Capability tokens (macaroon v1) with caveats: `aud`, `path_prefix`, `method`, `nbf`, `exp`.  
- Readiness/repair governance in RUNBOOK & PERFORMANCE.  
- Optional TLA+ sketches for quota & purge state machines (`docs/spec/governance/`).

**History:**  
- Use this section to record notable disputes, takedowns, residency exceptions, and emergency overrides with links to incidents/post-mortems.

```

---

