

---

```markdown
# 🏛 GOVERNANCE.md — ron-accounting

---
title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-15
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ (supporting: metering/export)
---

## 0. Purpose

This document defines the **rules of engagement** for `ron-accounting` as it relates to **economic integrity** and **operational governance**.

`ron-accounting` is a **transient metering/exporter**: it aggregates usage (requests, bytes, time-slices) and **exports idempotent batches** to `ron-ledger`, which is the **only source of durable economic truth**.  
Therefore governance here focuses on **invariants for counting and export**, **bounded authority**, **clear roles**, and **appeal paths** around disputed usage before/after settlement.

This document ties into:
- **Economic Integrity Blueprint** (no doubles, bounded issuance; counters-vs-truth split)
- **Hardening Blueprint** (bounded authority, custody, least privilege)
- **Perfection Gates A–O**, esp. **Gate I** (economic invariants) and **Gate M** (appeal paths)

---

## 1. Invariants (MUST)

Non-negotiable rules enforced in code, config, and process:

- **[I-G1] No double issuance or double spend**
  - `ron-accounting` **cannot mint or settle**; it can only **propose usage batches** with **idempotency keys**. `ron-ledger` rejects duplicates and enforces conservation.

- **[I-G2] Counters must balance to truth**
  - Every exported usage batch must be **reconstructible** from inputs; nightly reconciliation must prove equality between **(sum of accepted batches)** and **(derived usage snapshots)** for the same window.

- **[I-G3] No out-of-band emission**
  - `ron-accounting` cannot alter **emission**, **prices**, or **rewards**. It produces **usage facts** only. Any economic schedule (e.g., reward emission) resides in `ron-ledger`/`svc-rewarder` and must not be shadowed here.

- **[I-G4] Auditability**
  - All governance-relevant actions (policy changes, parameter updates, freeze/unfreeze, dispute tagging) must be **append-only** logged via `ron-audit` with actor, capability, corr_id, and hash of the affected data.

- **[I-G5] Bounded authority**
  - No single admin can force accounting outcomes. Mutations of governance parameters require **multi-sig** approval; unsafe flags must degrade service (shedding writes) rather than silently bypass invariants.

- **[I-G6] Amnesia safety**
  - Default profile keeps no durable economic state. Any optional cache/WAL must be **non-authoritative**; export logic and reconciliation must not depend on it for truth.

- **[I-G7] Determinism & Idempotency**
  - Given `(tenant_id, window, labels, counts)`, idempotency key must be stable across retries; exports are **at-least-once** with **exactly-once settlement** in `ron-ledger`.

---

## 2. Roles & Authority

### Roles

- **Policy Owner (ron-policy)**  
  Defines quota knobs, window sizing, label allow-lists; **cannot** settle.

- **Ledger Keeper (ron-ledger)**  
  Validates and commits usage → economic events. Enforces conservation and anti-double-spend.

- **Rewarder (svc-rewarder)**  
  Computes/distributes incentives under pre-defined caps, **without mint powers**; consumes ledger outputs, not accounting drafts.

- **Accounting Operator (this crate’s ops)**  
  Runs the service, manages rollout, ensures invariants and SLOs; **no authority** to change economics.

- **Auditor (read-only)**  
  Verifies logs, metrics, and proofs; initiates/observes disputes and appeals.

### Authority Boundaries

- Policy proposes **parameters** (e.g., `window_len_s`, `max_labels`, queue caps) but **cannot** enforce ledger commits.
- Ledger **may reject** any batch that violates schema, time window, or idempotency constraints.
- Rewarder **cannot** alter usage facts; it **must** reference committed ledger entries.
- All roles authenticate via **capability tokens (macaroon-style)** with least privilege (scope: object, action, TTL).  
- **Emergency powers** (freeze/unfreeze) require **N-of-M multi-sig** and produce a visible audit event.

---

## 3. Rules & SLAs

### Settlement & Freshness

- **Ledger settlement SLA:**  
  - 99.9% of accepted accounting batches settle in **< 5s** intra-AZ.  
  - Audit log for each settlement appears **< 1s** after commit.

- **Export SLO (this crate):**  
  - Export latency p95 **< 150ms** steady; **< 500ms** during 5-minute peaks.  
  - Backlog remains **< 1000 batches** for 99.9% minutes/day.

### Dispute Handling

- Any batch may be marked **`disputed=true`** without rollback.  
- Disputed entries must retain original hashes and corr_ids.  
- Resolution outcomes are **append-only** (e.g., compensating entries) in the ledger.

### Reward Schedules (Out-of-scope here)

- Emission curves and reward policies **live outside** `ron-accounting`. If a reward result appears inconsistent with metering, file a **dispute**; do not hot-patch accounting totals.

---

## 4. Governance Process

### Proposal Lifecycle (for governance-affecting changes)

```

Draft → Review → Multi-sig Approve (N-of-M) → Execute → Postmortem/Audit

````

- **Draft**: Open a Governance Proposal (GP) with YAML (see Appendix A) specifying: change type, rationale, invariants touched, rollback plan.
- **Review**: At least **2 reviewers** from separate roles (e.g., policy + ops).
- **Approve**: Multi-sig, threshold configurable per environment (e.g., 2/3 for staging, 3/5 for production).
- **Execute**: Change enacted via controlled rollout (canary, then fleet); `/readyz` may **degrade writes** during transition to preserve safety.
- **Postmortem**: Within **72h**, publish results including metric deltas and verification steps.

### Parameter Changes (examples)

- `window_len_s`, `max_labels`, `queue_caps`, `degrade_thresholds`, `idempotency_salt` (if used), observability sampling rates.  
- Must include **impact analysis** on invariants and SLOs.

### Emergency Powers

- **Freeze** (stop accepting new writes; reads OK) → **multi-sig required**; auto-expire after T hours unless renewed.  
- Must emit `audit_emergency_freeze{by, reason, until}` and be disclosed within **24h**.

---

## 5. Audit & Observability

### Audit Trails

- All governance actions → `ron-audit` (append-only), signed by actor’s capability.  
- Minimal event fields: `{ts, actor, cap_id, action, target, hash, corr_id}`.

### Economic Integrity Metrics

- `governance_proposals_total{status in ["draft","approved","rejected","executed"]}`  
- `ledger_disputes_total{state in ["open","resolved","rejected"]}`  
- `ron_accounting_export_backlog_gauge`  
- `ron_accounting_rejected_total{reason}`  
- `ron_accounting_export_latency_seconds{le}` histogram

### Verifiability

- **Conservation proofs**: nightly job compares **expected usage** vs **committed ledger deltas** per tenant/window; publishes Merkle root to audit log.  
- **Range checks**: label cardinality and per-tenant limits enforced; violations alert and are quarantined (no silent drop).

### Red-Team Drills (Quarterly)

- **Rogue admin** tries to bypass multi-sig → must fail with audit event.  
- **Tamper attempt** on idempotency keys → ledger rejects duplicates; alert fires.  
- **Config shadowing** (out-of-band param flip) → service degrades writes, logs mismatch, and refuses unsafe changes.

---

## 6. Config & Custody

### Config (must declare)

- **Policy parameters**: `window_len_s`, `max_labels`, backpressure/queue caps, degradation thresholds.  
- **SLA targets** used for alerting and CI perf gates.  
- **Auth policy**: capability scopes for exporter → ledger.  
- **Freeze/Unfreeze** toggles guarded by multi-sig.

### Custody

- `ron-accounting` holds **no durable signing keys**.  
- Any needed signing (e.g., batch attestations) uses **ron-kms/HSM** via short-lived session keys or detached signatures.  
- **No raw private keys in env or files**.  
- **Rotation**: capability tokens (and any ephemeral keys) rotate at ≤ 90-day intervals or immediately after suspected compromise.

---

## 7. Appeal Path

1. **Open Dispute** (`disputed=true`) referencing batch id, corr_id, and reason (user-reported mismatch, suspected over-count, etc.).  
2. **Governance Topic**: publish dispute on bus; assign reviewers from policy + ledger.  
3. **Investigate**: replay usage inputs; verify reconciliation and label policies.  
4. **Resolve**:  
   - If accounting error: commit **compensating ledger entry** (append-only), keep original intact.  
   - If policy misconfiguration: raise GP to fix parameters; do not mutate history.  
5. **Escalation**: If not resolved in **T=72h**, require **multi-sig override proposal** and public disclosure in `ron-audit`.

---

## 8. Rules → SLAs → Alerts (operational mapping)

- **Rule**: Backlog must remain bounded.  
  **SLA**: `< 1000` 99.9% minutes/day.  
  **Alert**: `RonAccountingBacklogHigh` if `export_backlog_gauge > 1000 for 10m` (warning) or `30m` (critical).

- **Rule**: Writes degrade rather than break invariants.  
  **SLA**: In outage, `/readyz=503` for writes; reads OK.  
  **Alert**: `RonAccountingDegradedWritesSustained` when `increase(rejected_total{reason="degraded"}[30m]) > 1000`.

- **Rule**: No restart storms.  
  **SLA**: ≤ 3 restarts / 5m.  
  **Alert**: `RonAccountingRestartStorm` when `increase(service_restarts_total{service="ron-accounting"}[5m]) > 3`.

All alert definitions must be **checked into repo** under `deploy/prometheus/alerts/`.

---

## 9. Acceptance Checklist (Definition of Done)

- [ ] **Invariants implemented & enforced** (I-G1…I-G7), with unit/integration tests.  
- [ ] **Roles & boundaries** documented; capabilities scoped and tested (deny-by-default).  
- [ ] **Governance process** (proposal lifecycle + multi-sig) documented and runnable (scripts/).  
- [ ] **Audit trails** wired (ron-audit), including emergency freeze/unfreeze.  
- [ ] **Metrics & alerts** shipped; dashboards include integrity panels (backlog, disputes, export latency).  
- [ ] **SLA validation** covered in CI perf gates (p95 export latency, backlog bound).  
- [ ] **Appeal path** exercised in chaos drill (open→resolve with compensating entry).  
- [ ] **Custody** verified: no private keys on disk; KMS paths green; rotation policy active.

---

## 10. Visuals

### 10.1 Governance Flow (high level)

```mermaid
flowchart LR
  P[Policy: ron-policy] -- propose params --> A[Accounting: ron-accounting]
  A -- idempotent batch --> L[Ledger: ron-ledger]
  L -- commit events --> R[Rewarder: svc-rewarder]
  A -- audit events --> AU[ron-audit]
  G[Governance Multi-sig] -- approve/freeze --> A & L
  AUD[Auditor] -- read-only --> AU & L
````

### 10.2 Dispute/Appeal

```mermaid
sequenceDiagram
  participant U as User/Stakeholder
  participant A as ron-accounting
  participant L as ron-ledger
  participant G as Governance(Multi-sig)
  participant AU as ron-audit

  U->>A: File dispute (batch_id, reason)
  A->>AU: Log dispute(open)
  A->>G: Publish governance ticket
  G-->>A: Assign reviewers
  A->>L: Reconcile usage vs committed
  L-->>A: Evidence (hashes, corr_ids)
  alt Error confirmed
    G->>L: Approve compensating entry (multi-sig)
    L->>AU: Audit commit
  else No error
    G->>AU: Close dispute with rationale
  end
```

---

## 11. Appendix

### Appendix A — Governance Proposal (GP) YAML

```yaml
id: GP-YYYYMMDD-### 
title: "Change window_len_s from 60 to 120"
author: "alice@company"
targets:
  crate: ron-accounting
  env: production
rationale:
  - "Reduce export overhead; align with downstream billing batch size."
invariants_touched:
  - I-G2
  - I-G7
risk_assessment:
  - "Temporary spike in backlog during rollout; mitigated by degrade mode."
rollout_plan:
  - canary: 5% traffic for 30m
  - fleet: 100% after success
  - rollback: revert param; drain backlog to 0
validation:
  - "PromQL p95 export latency < 500ms during canary"
  - "Backlog gauge < 1000 for 99.9% of minutes"
approvals:
  requires: "3-of-5"
  signed_by: []
```

### Appendix B — Terminology

* **Dispute**: A marker on an accepted usage batch indicating contention; does not delete history.
* **Compensating Entry**: A ledger operation that corrects prior effects without mutation.
* **Degrade Mode**: Writes refused with 503 to preserve invariants; reads still available.

### Appendix C — References

* Economic Integrity Blueprint
* Hardening Blueprint
* Six Concerns / Perfection Gates (Gate I, Gate M)
* `ron-ledger` contract (append-only, conservation)
* `ron-audit` event schema
* Capability tokens (macaroon-style)

---

## 12. Change Log (Governance)

* 2025-10-15: Initial draft aligned with counters-vs-truth, bounded authority, and multi-sig governance.

```
---
