

---

# 🏛 GOVERNANCE.md — ron-kms

---

title: Governance & Economic Integrity — ron-kms
status: draft
msrv: 1.80.0
last-updated: 2025-10-09
audience: contributors, ops, auditors, stakeholders
crate-type: policy
------------------

## 0. Purpose

`ron-kms` controls **key custody and authorization** for signing/verification and wrap/unwrap. This document defines the **rules of engagement** that bind its policy surface, authority boundaries, quorum, and audit posture.

It ensures:

* Transparent, immutable, and externally **auditable** control decisions.
* Enforcement of **economic-style invariants** adapted to KMS (bounded capability issuance, no out-of-band key material, no unbounded admin).
* **Clear roles & quorum** for sensitive actions (e.g., root rotation, PQ mode changes).
* SLA-backed commitments (revocation latency, audit durability, capability issuance fairness).

Ties into: Hardening (bounded authority/custody), Scaling/Observability (SLOs & alerts), QUANTUM (PQ rollout), Performance (drift gates), and Perfection Gates I/K/L/M.

---

## 1. Invariants (MUST)

* **[K-G1] No out-of-band keys.** All private keys originate from approved backends (HSM/TPM/sealed-file) under declared custody. No ad-hoc/imported secrets outside approved migration.
* **[K-G2] No sign without capability.** Every signing op requires a **capability token** (macaroon v1) with passing caveats (tenant/scope/alg/expiry/usage).
* **[K-G3] Bounded capability issuance.** Capability minting is scoped, time-boxed, and rate-limited per tenant; no unbounded “god tokens.”
* **[K-G4] Sensitive actions are **quorum-gated**.** Root rotation, backend change, PQ escalation (`pq_only`), global rollback require N-of-M multi-sig approval.
* **[K-G5] Immutable audit.** Every authority change and every sign/verify/wrap/unwrap result emits an append-only, signed audit record (local + `ron-audit`).
* **[K-G6] Attestation precedence.** If required by policy, **sign** is blocked while attestation readiness is failing; **verify** may continue per policy.
* **[K-G7] PQ governance safety.** PQ rollout follows Phase 0→3; default production mode is **hybrid-AND** unless a tenant-scoped exception is quorum-approved and audited.
* **[K-G8] Deny on ambiguity.** On conflicting policy/caveats, the service MUST refuse to sign and produce an auditable reject.

---

## 2. Roles & Authority

### Roles

* **Policy Owner (ron-policy):** Maintains global/tenant policy (caps, PQ phases, attestation). Proposes changes; cannot mint caps.
* **KMS Operator (ron-kms ops):** Operates service; executes policy-approved changes; cannot bypass quorum.
* **Custodian (HSM/TPM admin):** Manages backend modules/slots; cannot alter policy or mint caps.
* **Auditor (read-only):** Verifies logs/metrics/proofs; can mark disputes; cannot execute changes.
* **Incident Commander (IC):** Coordinates emergencies; still bound by quorum.
* **Release Manager:** Approves deploys; SBOM/signature compliance.

### Authority Boundaries

* Policy **proposes**; quorum-approved execution only for sensitive actions.
* Operators may tune runtime limits **within policy bounds** (e.g., ±20%); beyond requires proposal.
* Custodians cannot mint caps or change policy; backend swaps are quorum-gated and audited.
* No “break-glass” user without multi-sig; emergency freezes are quorum-gated and auto-expire.

---

## 3. Rules & SLAs

* **Revocation SLA:** Capability or key revocation enforced **≤ 5s** after acceptance (readyz true); corresponding metric/log entry **≤ 1s** after enforcement.
* **Audit availability:** Audit record for a successful sign is durable locally and exported to `ron-audit` **≤ 1s** after completion.
* **Capability ceilings:** Per-tenant active capabilities and mint rate are enforced; excess attempts rejected with explicit taxonomy.
* **Attestation SLO:** When required, `attestation_ready=true` **≥ 99.99%** of minutes; dips alarm and **freeze sign** until recovery.
* **PQ transition SLA:** Phase changes (OR→AND→PQ-only) occur inside a declared change window; **tenant rollback to OR in ≤ 60s** if rejects spike. Each proposal must attach CI links to **phase-transition property tests** and a **15-minute perf snapshot**.
* **Governance proposal SLA:** **Decision within 24h**; default reject if quorum not met.

---

## 4. Governance Process

* **Lifecycle:** *Draft → Review (policy+security) → Quorum sign → Execute → Post-audit*.

  * Proposals are signed objects: author, scope (global/tenant), rationale, changeset, risk, rollback, expiry, signer set, attachments (CI evidence).
  * Minimum quorum: **2-of-3** (default; environment-configurable).

* **Sensitive actions (quorum required):**
  Root key rotation; backend change (module/path/slot); production PQ escalation to `pq_only`; attestation policy from optional→required; disabling hybrid-AND in prod; enabling “amnesia off” in a profile that mandates amnesia.

* **Parameter changes (non-quorum within bounds):**
  Inflight caps, cache TTL/size, per-tenant cap mint rate **within ±20%** of policy baseline. Outside bounds → proposal.

### 4.x Quorum Cryptography (How)

* **Scheme:** Ed25519 threshold (Shamir-protected signing keys) aggregating an Ed25519 multi-signature envelope over a **canonical JSON** digest (SHA-256). Compatible with `ed25519-dalek`.
* **Threshold:** default **2-of-3** (policy owner, security officer, custodian).
* **Signer set rotation:** **Quarterly** or upon role change; rotations are quorum-approved proposals.
* **Artifacts:** Proposals carry `signatures[]` (role, key id, sig).
* **Verification:** `ron-kms` verifies threshold **before** executing any sensitive action.

### 4.y Operator Workflow (ronctl)

* Draft:
  `ronctl governance propose --scope tenant:ACME --change 'pq_mode=and' --rationale 'hybrid AND cutover' --expires 24h`
* Review: `ronctl governance show PROPOSAL_ID`
* Approve: `ronctl governance approve PROPOSAL_ID --role security-officer`
* Execute: `ronctl governance execute PROPOSAL_ID`
* Audit log: `ronctl governance log --since 7d`

---

## 5. Audit & Observability

* **Audit records** (append-only, signed): capability mint/revoke; policy changes; backends/rotations; sign/verify/wrap/unwrap outcomes (metadata only); emergency freeze/unfreeze; PQ phase changes.
* **Metrics (examples):**

  * `kms_governance_proposals_total{status}`
  * `kms_capabilities_active{tenant}` / `kms_capabilities_minted_total{tenant}`
  * `kms_capabilities_minted_fairness_skew{window}` *(gauge; top-QPS vs median)*
  * `kms_emergency_freezes_total`
  * `kms_attestation_ready_seconds_total{state}`
  * `kms_policy_rejects_total{reason}`
  * `kms_pq_phase{tenant,phase}` (gauge)
* **Verifiability:**

  * Custody proofs: attestation quotes or sealed-file attest artifacts linked to key IDs.
  * Capability proofs: macaroon verifiers + caveat transcripts.
  * Policy diffs: signed; hash-linked chain of custody.
* **Red-team drills (quarterly):**
  Rogue admin attempt (single-party override) must fail; capability replay must fail (nonce/expiry); attestation bypass must block sign.

---

## 6. Config & Custody

* **Config MUST declare:** active backend & slot, attestation policy, cap mint ceilings, PQ defaults & per-tenant exception policy, revocation webhooks/bus topics.
* **Custody rules:** keys reside in approved backends only; **no raw keys** in env/files; rotation **≥ every 90 days** or upon compromise; rotations emit KAT-verifiable continuity proofs (old→new linkage).

**Micronode / Edge Profiles**
For single-tenant, air-gapped micronodes, quorum may be temporarily reduced to **1-of-2** (owner, security officer) with stricter audit: freeze auto-expiry **30 min**, mandatory **weekly** rotation of the secondary signer device, and continuous export of audit logs off-box.

---

## 7. Appeal Path

* **Dispute flow:**

  1. Tenant/auditor files dispute on governance topic (cap ID, request IDs, policy refs).
  2. KMS marks disputed items (`disputed=true`) in audit; **no silent rollback** of completed signatures.
  3. Quorum-gated override proposal if remediation requires policy exception.
  4. Auditor review + published resolution.

* **Time goals:** acknowledge ≤ **1h**, triage ≤ **24h**, final decision ≤ **24h** (unless extended with rationale).

**Escalation & Deadlock Handling**
If quorum is **deadlocked > 24h** on a safety-neutral appeal, escalate to an **external auditor** signer (temporary 4th key) with a **48h time-lock**. Auditor may cast a **tie-break** signature limited to that proposal. All escalations are reported in governance reports and expire after the proposal closes.

---

## 8. Acceptance Checklist (DoD)

* [ ] Invariants [K-G1..K-G8] enforced in code & covered by tests.
* [ ] Roles & boundaries documented; no backdoors.
* [ ] Quorum flow implemented; signer set maintained & rotated.
* [ ] Audit/metrics exported; dashboards & alerts wired (attestation, freeze, rejects, fairness skew).
* [ ] SLAs (revocation, audit export) tested in CI/chaos; evidence attached to proposals.
* [ ] Appeal & escalation path validated (tabletop + red-team drill).

---

## 9. Appendix

### A. Blueprint Links

Hardening (bounded authority/custody), Scaling (readiness/shedding), QUANTUM (PQ phases), Perfection Gates: I (bounded issuance/custody), K (vigilance), L (black swan), M (appeals).

### B. Governance Objects (Schema Hints)

* **Proposal**
  `{ id, author, scope, rationale, changeset, risk, rollback, quorum, signer_set{members[], threshold, rotated_at}, signatures[], created_at, expires_at, attachments[] }`
* **Capability**
  `{ id, tenant, scope, caveats{alg, exp, rps, paths}, minted_at, expires_at, minted_by }`
* **Rotation**
  `{ key_id_old, key_id_new, backend, attestation, approved_by[], executed_at }`

### C. Thresholds (Defaults)

* Quorum: **2-of-3** signers (policy owner, security officer, custodian).
* Freeze auto-expiry: **60 min** (micronode: **30 min**).
* Revocation SLA: **≤ 5s**; Audit export: **≤ 1s**.

### D. History

* **2025-10-09**: Initial charter with quorum enforcement, PQ safeguards, attestation precedence, fairness metrics, and escalation/alerts.

---

## 10. Alerts (PromQL)

```yaml
groups:
- name: ron-kms.governance
  rules:
  - alert: KMS_Governance_Deadlock
    expr: increase(kms_governance_proposals_total{status="pending"}[24h]) > 0
    for: 24h
    labels: {severity: page}
    annotations:
      summary: "ron-kms: governance proposals pending >24h (potential quorum deadlock)"

  - alert: KMS_PQ_Policy_Rejects
    expr: increase(kms_policy_rejects_total{reason="pq_policy"}[10m]) > 0
    for: 10m
    labels: {severity: page}
    annotations:
      summary: "ron-kms: PQ policy rejects observed (Phase 0–2 tenants)"

  - alert: KMS_Capability_Fairness_Skew
    expr: kms_capabilities_minted_fairness_skew{window="1h"} > 3
    for: 30m
    labels: {severity: ticket}
    annotations:
      summary: "ron-kms: tenants skew >3× in capability minting (1h)"
```

---

**Operator note:** This charter complements `RUNBOOK.md`, `PERFORMANCE.md`, and `SECURITY.md`. Any action touching **quorum-gated** surfaces requires a signed proposal—even under emergency; use the **expedited freeze** (quorum-gated, auto-expiring) rather than ad-hoc overrides.
