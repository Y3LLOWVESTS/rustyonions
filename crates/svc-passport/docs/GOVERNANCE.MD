

```markdown
# 🏛 GOVERNANCE.md — svc-passport

---
title: Governance & Economic Integrity (svc-passport)
status: draft
msrv: 1.80.0
last-updated: 2025-10-09
review-cadence: quarterly
audience: contributors, ops, auditors, stakeholders
crate-type: policy
---

# GOVERNANCE.md

## 0. Purpose

This document defines the **rules of engagement** for `svc-passport`’s policy/issuance logic.  
For a control-plane identity service, “economic integrity” maps to **issuance integrity** (no duplicate authority windows; bounded TTL/claims), **revocation monotonicity**, and **key custody**.

It ensures:
- Transparent, auditable decision-making and configuration changes.
- Enforcement of **invariants** (no duplicate authority; bounded claims; monotonic revocation).
- Clear authority boundaries, quorum rules, and appeal/escalation paths.
- SLA-backed commitments to consumers.

It ties into:
- **Economic Integrity Blueprint** (no doubles; bounded issuance/claims)
- **Hardening Blueprint** (bounded authority, caps, custody)
- **Scaling/Observability Blueprints** (SLOs, burn-rate)
- **Perfection Gates A–O** (Gate I = bounded invariants, Gate M = appeals, Gate F/L = perf & chaos)

> Scope: `svc-passport` does **not** track money. “Ledger” below refers to the **governance/audit ledger** (tamper-evident record of issuance, revocation, and parameter changes).

---

## 1. Invariants (MUST)

Violations are **SEV-1** incidents: freeze issuance, open incident, post-mortem required.

- **[I-G1] No Duplicate Authority Window**  
  For the same `{subject, audience, scope}` within the same validity window, **do not** re-issue without an intervening revocation or epoch bump.

- **[I-G2] Bounded Authority**  
  Each passport must satisfy **TTL ≤ max_ttl**, **caveats ≤ max_caveats**, **scope/audience ∈ allow-lists**, **size ≤ 1 MiB**. No out-of-band claims.

- **[I-G3] Monotonic Revocation**  
  After revocation at epoch *N*, verification at epoch ≥ *N* **must refuse**. Epoch bumps invalidate caches immediately.

- **[I-G4] Signed, Append-Only Audit**  
  All governance actions (issue/revoke/epoch/param/freeze) are appended to WORM storage with chained hashes and signer attestations.

- **[I-G5] Separation of Duties; No Unbounded Admin**  
  No entity can both define and execute policy unilaterally. Sensitive actions require **multi-sig**. No “god mode”.

- **[I-G6] Key Custody & Rotation**  
  Private keys never leave `ron-kms`/HSM. Rotation **≤ 90 days** or immediately upon suspicion. Old keys become verify-only as needed.

- **[I-G7] SLA Visibility**  
  SLO/SLA breaches must be measurable and alertable (burn-rate fast/slow). Breaches can trigger governance actions (e.g., temporary issuance freeze).

---

## 2. Roles & Authority

### Roles (principals)

| Role            | Can                                                                 | Cannot                                                     |
|-----------------|---------------------------------------------------------------------|------------------------------------------------------------|
| **Policy Owner** (ron-policy) | Propose policy caps (TTL, caveats), PQ posture, allow-lists            | Execute issuance; rotate keys; bypass caps                 |
| **Issuer** (svc-passport)     | Enforce policy; issue/refuse; emit audits                               | Alter policy; access raw keys; override multi-sig          |
| **Key Custodian** (ron-kms/HSM) | Sign/verify; rotate keys via dual-control                               | Change policy or issue directly                            |
| **Security Officer (SecOps)** | Propose revocations, epoch bumps, emergency freezes                      | Execute changes without quorum                             |
| **Auditor** (read-only)       | Verify audits/metrics; file disputes                                     | Execute policy or operational changes                      |
| **SRE/Platform**              | Rollout/rollback; scale; apply approved config                            | Change caps without approved proposal                      |

All roles must use **capability tokens (macaroons v1)** with least-privilege caveats (purpose, TTL, IP CIDR).

### Authority Boundaries

- Policy Owner **proposes**; Issuer **enforces**; KMS **signs**; SecOps **requests** revocation/epoch/freeze; Auditor **verifies**.
- Issuer must **refuse** any request violating caps—even if proposed/approved erroneously.
- KMS only signs authenticated, capability-bound requests; cannot originate issuance.

---

## 3. Rules & SLAs

- **Service SLOs** (match PERFORMANCE.md)  
  - Issuance: p95 ≤ **40 ms**, p99 ≤ **100 ms** (intra-region)  
  - Verify: p95 ≤ **10 ms**, p99 ≤ **25 ms** (intra-region); p95 ≤ **120 ms**, p99 ≤ **200 ms** (inter-region)  
  - Revocation propagation: p99 ≤ **5 s**

- **Audit Availability**  
  - Audit record appears **≤ 1 s** after commit. Append-only guarantee.

- **Parameter Change SLA**  
  - Approved proposals roll within **24 h** unless labeled emergency.

- **Burn-rate & Freezes**  
  - If error-budget burn-rate **fast (2h) ≥ 14×** for 10m, SecOps may propose **temporary issuance freeze** (verification remains up) pending review.

- **Regional Governance**  
  - Inter-region SLO breach isolated to specific region(s) may trigger **regional freeze** rather than global freeze, if policy allows.

---

## 4. Governance Process

### Review Cadence
- **Quarterly** review of invariants, SLAs, and quorum config; publish minutes and diffs in audit log.

### Proposal Lifecycle
`Draft → Review → Approve → Execute → Audit`

- **Draft:** Proposal includes rationale, risk, rollback, blast radius, validation plan.
- **Review:** ≥ **2** independent reviewers (Policy + SecOps or Auditor).
- **Approve (Quorum):** **n_of_m** multi-sig (default **2-of-3**: Policy Owner, SecOps, Platform). Time-box: **24 h**; otherwise default reject.
- **Execute:** Platform applies change (config reload/rollout). Issuer enforces. KMS rotation if applicable.
- **Audit:** Append signed record (who/what/when/hash); publish to bus topic `governance.audit`.

#### Deadlock & Escalation
- If quorum not reached in **24 h**, escalate to **m_of_m** (full board) with an additional **24 h** window.  
- If still deadlocked and risk high (e.g., unresolved compromise), allow **temporary 12 h issuance freeze** via **2-of-3** with mandatory public audit entry and scheduled re-vote.

### Emergency Powers (bounded)
- **Freeze Issuance** (verify remains up)  
  - Trigger: suspected key compromise, mass invalid issuance, uncontrolled SLO melt-down  
  - Approval: **≥ 2-of-3**; auto-expire in **6 h** unless re-approved  
  - Disclosure: audit entry **≤ 24 h** with timeline and remediation plan

- **Epoch Bump**  
  - Trigger: sweeping revocation; suspected leak  
  - Approval: **2-of-3**  
  - Effect: immediate cache invalidation; monotonic refusal ≥ new epoch

### Parameter Changes (must go through lifecycle)
- `max_ttl`, `max_caveats`, allow-lists (audience/scope), `pq.mode` (`mirror|require|enforce`), SLA targets, and quorum changes.

---

## 5. Audit & Observability

- **Audit Log (WORM)**  
  - Events: `issue`, `revoke`, `epoch_bump`, `param_change`, `freeze`, `unfreeze`, `appeal`, `override`  
  - Fields: `id`, `ts`, `actor`, `role`, `cap_id`, `action`, `payload_hash`, `prev_hash`, `sig[]`

- **Metrics of Record**
  - `governance_proposals_total{status="draft|approved|rejected|executed"}`  
  - `governance_freezes_total{reason}`  
  - `governance_epoch_bumps_total`  
  - `ledger_disputes_total` (disputed issuances)  
  - `passport_issuance_total{outcome="ok|rejected|disputed"}`

- **Verifiability**
  - **Conservation checks:** issuance vs. revocation/expiry per `{subject, scope, epoch}`  
  - **Range proofs:** TTL ≤ cap; caveats count ≤ cap; size ≤ cap  
  - **PQ posture attestation:** `pq.mode` embedded in audit payload

- **Red-team Drills (frequency)**
  - **Quarterly**, aligned with RUNBOOK chaos: simulate rogue admin, key misuse, and cap bypass; ensure multi-sig + KMS isolation block attempts; verify alerts & audit entries.

- **Burn-rate PromQL (reference)**
```

# Fast (2h window) — 14× monthly budget (budget B=0.001)

(
sum(rate(http_requests_total{job="svc-passport",status=~"5..|429|503"}[2h])) /
sum(rate(http_requests_total{job="svc-passport"}[2h]))
) / 0.001 > 14

# Slow (24h window) — 6× monthly budget

(
sum(rate(http_requests_total{job="svc-passport",status=~"5..|429|503"}[24h])) /
sum(rate(http_requests_total{job="svc-passport"}[24h]))
) / 0.001 > 6

````

---

## 6. Config & Custody

**Config MUST declare**
- Policy parameters: `max_ttl`, `max_caveats`, `allowed_aud[]`, `allowed_scopes[]`
- PQ posture: `pq.mode = "mirror|require|enforce"`
- SLA targets: reflect PERFORMANCE.md (issuance/verify/propagation)
- Governance keys & quorum: `quorum.n_of_m` (default `2-of-3`), and `quorum.m_of_m` (fallback full board)
- Audit sinks: WORM URI + `governance.audit` bus topic

**Custody**
- Signing keys: `ron-kms`/HSM only; **never** in files/env
- Rotation: **≤ 90 days**; dual-control (two-person rule) to activate new keys
- Secrets in logs: **forbidden**; log key IDs, not secrets

**Capabilities (macaroons)**
- Short TTL (minutes), IP CIDR restrictions, purpose caveats (`policy.write`, `issue.execute`, `revocation.write`, `epoch.bump`), and rate caveats where applicable

---

## 7. Appeal Path

**Dispute Intake**
1. Reporter files dispute referencing `audit.id`/`corr_id`
2. Issuer marks the audit entry `disputed=true` (immutable flag)

**Resolution**
1. Post `governance.appeal` with evidence to the bus  
2. Quorum (Policy + SecOps + Auditor) reviews and signs outcome: **uphold** / **override** / **remediate**  
3. If override: create multi-sig proposal describing corrective action (e.g., compensating revocation, epoch bump)  
4. Publish decision + post-mortem link in audit log

**SLA**
- Acknowledge within **4 h**; decision within **72 h**. If unresolved, maintain or impose temporary issuance freeze with scheduled re-vote.

---

## 8. Acceptance Checklist (DoD)

- [ ] **Invariants enforced in code/tests** (no duplicate windows; bounded caps; monotonic revocation)  
- [ ] **Roles & boundaries** mapped to capabilities; least-privilege caveats applied  
- [ ] **Proposal lifecycle** implemented with n-of-m quorum and time-boxes; emergency paths bounded  
- [ ] **Audit & metrics** exported; dashboards & burn-rate alerts wired  
- [ ] **SLA thresholds** monitored; regional/global freeze procedures tested  
- [ ] **Appeal path** exercised in quarterly drill; audit entries verified  
- [ ] **Key rotation** dual-control dry-run completed; attestation recorded

---

## 9. Appendix

### Mermaid — Proposal Lifecycle
```mermaid
flowchart LR
A[Draft] --> B[Review (>=2 signers)]
B -->|Approve n_of_m| C[Execute]
B -->|Reject/Timeout| X[Close]
C --> D[Audit Append + Bus Publish]
B -.->|Deadlock 24h| E[Escalate m_of_m 24h]
E -->|Approve| C
E -->|Still Deadlocked| F[Temp Freeze 12h + Revote]
````

### Reference PromQL

```
governance_proposal_approval_rate =
  sum(rate(governance_proposals_total{status="approved"}[24h])) /
  sum(rate(governance_proposals_total[24h]))

appeals_open =
  sum(increase(ledger_disputes_total[30d])) - sum(increase(ledger_disputes_total{status="closed"}[30d]))
```

### Capability Sketch (Macaroon Caveats)

* `role=policy-owner, scope=policy.write, exp=5m, ip_cidr=10.0.0.0/8`
* `role=secops, scope=revocation.write|epoch.bump, exp=10m`
* `role=issuer, scope=issue.execute, exp=2m, rate<=R`

### History (seed)

* **2025-10-01** — TTL cap change 24h → 12h (quorum: Policy+SecOps; executed by Platform; audit id `a1b2…`)
* **2025-09-14** — Epoch bump (suspected token leak); p99 propagation 3.2s; incident PM `INC-2317`
* **2025-08-22** — Quarterly review: quorum stays 2-of-3; added regional freeze policy

```
```
