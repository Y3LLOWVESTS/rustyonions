

---

# 🏛 GOVERNANCE.md

---

title: Governance & Economic Integrity — svc-registry
status: draft
msrv: 1.80.0
last-updated: 2025-10-08
audience: contributors, ops, auditors, stakeholders
crate-type: policy|control-plane
--------------------------------

## 0. Purpose

This document defines the **rules of engagement** for `svc-registry`, the authoritative, **append-only** registry of **signed** descriptors (nodes, services, regions, quotas, policies). It ensures:

* Transparent, auditable decision-making (proposal → approvals → commit).
* Enforcement of **control-plane invariants** (no forks, monotonic HEAD, signed provenance).
* Clear **authority boundaries** (who can propose, who can approve, who can operate).
* SLA-backed commitments for governance actions (proposal visibility, commit latency, audit availability).

Ties into: Economic/Integrity & Hardening Blueprints (bounded authority; no out-of-band edits), and **Perfection Gates** (I: control-plane invariants, K: vigilance, L: black-swan drills, M: appeal paths).

---

## 1. Invariants (MUST)

Non-negotiable rules for the registry’s control-plane log:

* **[I-R1] Append-Only & No Forks.** History is immutable; **HEAD is strictly monotonic**. No rebase, no deletion, no mutation.
* **[I-R2] Quorum-Gated Commits.** Every commit MUST be accompanied by **M-of-N** valid approvals (ed25519 over `payload_b3`) bound to the exact proposal hash.
* **[I-R3] Deterministic Canonicalization.** All proposed state is encoded as **canonical JSON**; `payload_b3` is stable and collision-resistant for approvals and audit.
* **[I-R4] Bounded Authority.** No single operator or code path can bypass quorum, rewrite history, or mint out-of-process changes. Emergency actions are **freeze-only**, never edit.
* **[I-R5] Full Auditability.** Each proposal, approval, and commit emits a **signed, timestamped** audit record with reproducible hashes and correlation IDs.
* **[I-R6] Loss-Tolerant Distribution.** Bus/SSE delivery is **lossy by design**; correctness is preserved via `/registry/head` reconciliation.
* **[I-R7] Schema Hygiene.** DTOs enforce `deny_unknown_fields`; unknown/ambiguous fields are rejected to prevent stealth semantics drift.
* **[I-R8] Capability-Gated APIs.** Mutating endpoints require capabilities (macaroons-style) with **least privilege** and expiry.

---

## 2. Roles & Authority

### Roles

* **Proposer** — Authors a change (descriptor set delta). Needs `cap:registry:propose`.
* **Approver (Signer)** — Member of the governance signer set; supplies cryptographic approvals. Needs `cap:registry:approve`.
* **Committer (Service logic)** — `svc-registry` itself performs the commit **only** when quorum is satisfied; there is no human “committer” role.
* **Operator (SRE)** — Runs the service; can toggle **read-only/freeze**, scale, and roll versions, **but cannot bypass quorum**.
* **Auditor** — Read-only consumer of logs, metrics, and proofs; can open disputes.
* **Consumer** — Downstream services (gateway/overlay/index) that observe HEAD and reconcile.

### Authority Boundaries

* Proposers cannot commit.
* Approvers cannot change payloads; they only sign **exact** proposal hashes.
* Operators cannot inject commits; they may **freeze** the write plane (maintenance/emergency).
* Only `svc-registry` code advances HEAD, and **only** after verifying quorum + invariants.
* All mutating requests must present valid **capability tokens**; scopes are minimal and auditable.

---

## 3. Rules & SLAs (Governance Path)

* **Proposal publication SLA:** visible via `GET /proposals/{id}` within **1s** of accept.
* **Approval validation SLA:** each approval validated and recorded within **500 ms** of receipt.
* **Commit SLA:** when quorum is satisfied, **p95 < 750 ms** to commit+emit audit event; HEAD becomes observable immediately in `/registry/head`.
* **Audit availability:** audit record and Merkle/materialized proof available within **1s** after commit.
* **Signer Set Management:** changes to the signer set themselves **require quorum**, go through the same proposal→approval→commit lifecycle, and take effect only at a **clearly versioned height**.
* **Freeze semantics:** read plane stays up; write plane rejects with typed `reason="maintenance|deps"` and `Retry-After`.
* **Rate/Quota:** mutating endpoints are **rate-limited**; excessive load returns `429` with structured reasons.

---

## 4. Governance Process

### Lifecycle (happy path)

1. **Draft** — Proposer builds canonical payload; local checks pass.
2. **Submit** — `POST /propose` → returns `proposal_id` + stable `payload_b3`.
3. **Approve** — Approvers fetch the proposal & sign `payload_b3`; `POST /approve`.
4. **Commit** — When M-of-N approvals for the same `proposal_id` are present, `svc-registry` performs invariant checks and **commits** (append-only).
5. **Distribute** — Bus/SSE event emitted; consumers reconcile via `/head`.

### Quorum & Timing

* **Quorum:** **M-of-N**; defaults: `N ≥ 5`, `M ≥ 3`.
* **Expiry:** If quorum not reached within **T = 72h**, proposal auto-expires (still fully auditable).
* **Re-propose:** Superseding proposals reference prior `proposal_id` (soft link) for traceability.

### Emergency Powers (bounded)

* **Freeze/Thaw:** Operator may set **read-only** mode immediately; action emits an audit event with operator identity and reason.
* **No Edit/Erase:** There is **no** emergency path to rewrite history, rotate payloads in place, or inject unsigned commits.
* **Disclosure:** Any freeze must be disclosed in audit log **immediately** and in the weekly ops report.

---

## 5. Audit & Observability

* **Audit Log:** Append-only, signed entries for proposal/approval/commit/freeze; includes `payload_b3`, signer IDs, timestamps, and correlation IDs.
* **Metrics (golden set additions):**

  * `governance_proposals_total{status=accepted|expired|rejected}`
  * `governance_approvals_total{valid|invalid}`
  * `governance_commits_total` and `commit_latency_seconds{quantile}`
  * `governance_freeze_total{reason}`
  * `rejected_total{reason="auth|busy|deps|maintenance|schema"}`
* **Proofs:** HEAD includes hashes allowing clients to verify continuity (no fork) and content integrity.
* **Red-Team Drills:** Quarterly simulation of **rogue operator** and **stale signer** scenarios; expected outcome: **freeze**, dispute, and superseding proposal—**never** mutation.

---

## 6. Config & Custody

* **Config declares (and is audited into HEAD):**

  * Quorum parameters (N, M, expiry T).
  * Signer set (IDs, public keys, activation heights).
  * Write-plane toggles (maintenance/read-only).
  * API quotas for propose/approve.

* **Key Custody (signers):**

  * Private keys reside in **ron-kms** or approved **HSM**; hardware-backed if available.
  * **No raw private keys** in files/env on app hosts.
  * **Rotation:** at most every **90 days** or immediately upon suspected compromise; rotations are governed by the same **quorum** process and take effect at a declared height with overlap windows.

* **Capabilities:**

  * Mutating capabilities (propose/approve) are **scope-limited**, expiring, and logged.
  * Read capabilities (if any) default to public for `/head` and `/version`, subject to global policy.

---

## 7. Appeal Path (Disputes & Corrections)

* **Dispute (non-destructive):**

  1. Raise dispute via governance topic with references to `proposal_id`/commit height and evidence.
  2. Mark the disputed record in **auxiliary index** (does not alter history).
  3. Prepare a **superseding proposal** that corrects state going forward (e.g., deprecate entry, revert config), and pass quorum normally.
* **No Rollbacks:** There is **no** rollback of committed history; only **forward-fix** via new commits.
* **Escalation:** If a signer is compromised or quorum is stalled:

  * Freeze write plane.
  * Rotate affected signer(s) via quorum action.
  * Publish incident note in the audit log and ops report.
* **Auditor Review:** External auditors can attest that invariants (I-R1..I-R8) held across the dispute window.

---

## 8. Rules & External SLAs (Consumers)

* **HEAD Stability:** Consumers must treat HEAD as **authoritative** and immutable; reconciliation on gap or SSE loss is **mandatory**.
* **Change Windows:** For **breaking** changes (e.g., schema evolution), registry commits MUST include versioned flags and a **grace window** (min 7 days) before enforcement by gateway/overlay.
* **Backward Compatibility:** DTO fields are **additive**; removals require a deprecation cycle and explicit consumer ack in dependent services.

---

## 9. Acceptance Checklist (DoD)

* [ ] Invariants [I-R1..I-R8] encoded as tests/guards (unit/integration/property).
* [ ] Roles & boundaries documented in `CONFIG.md` and enforced via capabilities.
* [ ] Proposal→Approval→Commit flow implemented with **quorum** and **expiry**.
* [ ] Audit events & metrics exported; dashboards + alerts wired.
* [ ] Freeze path is operational and emits audit records.
* [ ] Signer custody & rotation policy documented and verified in staging.
* [ ] Appeal path executed at least once in staging (table-top or live drill).
* [ ] Consumers’ reconcilers validated under loss (chaos test passed).

---

## 10. Appendix

### 10.1 Minimal Governance Diagram (read-only; commit gated)

```mermaid
flowchart TD
  P[Proposer] -->|POST /propose| R[svc-registry]
  S1[Signer A] -->|POST /approve| R
  S2[Signer B] -->|POST /approve| R
  S_M[Signer M] -->|POST /approve| R
  R -->|Quorum M-of-N?| C{Quorum satisfied?}
  C -- Yes --> K[Commit (append-only, update HEAD)]
  C -- No  --> W[Wait / Expire T]
  K -->|Bus Event| B[(ron-bus)]
  B --> D[Consumers reconcile via /registry/head]
```

### 10.2 Example Policy Knobs (defaults)

* `quorum.n = 7`, `quorum.m = 3`, `proposal.expiry_hours = 72`
* `write_plane.maintenance = false`
* `capabilities.propose.rps = 2`, `capabilities.approve.rps = 10`

### 10.3 Perfection Gates Mapping

* **Gate I** — Invariants enforced (no forks; signed, canonical, quorum).
* **Gate K** — Alerts + dashboards for commits, freezes, errors.
* **Gate L** — Black-swan drills: signer loss, bus loss, storage latency.
* **Gate M** — Appeal path exercised and documented.

### 10.4 History (fill as events occur)

* `2025-Q4` — Initial governance activation; signer set v1 (N=7, M=3).
* (add disputes, rotations, freezes with links to audit records)

---

**Why this is right for `svc-registry`:** it encodes the registry’s true economics—**trust and authority**—as control-plane invariants: signed provenance, quorum-gated commits, immutable history, bounded ops powers, and forward-only corrections. This preserves systemic integrity while keeping operations practical under real-world faults.
