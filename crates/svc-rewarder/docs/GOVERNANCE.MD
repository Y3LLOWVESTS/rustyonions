

---

# 🏛 GOVERNANCE.md — `svc-rewarder`

---

title: Governance & Economic Integrity — svc-rewarder
status: draft
msrv: 1.80.0
last-updated: 2025-10-14
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ
-----------------------

## 0. Purpose

This document defines the **rules of engagement** for `svc-rewarder`’s economic/policy logic.
It ensures:

* Transparent, reproducible, and auditable decision-making.
* Enforcement of **economic invariants** (no doubles, bounded issuance, conservation).
* Clear **authority boundaries**, proposal lifecycle, and **appeal paths**.
* SLA-backed commitments to external consumers and deterministic behavior under brownouts.

Ties into: **Economic Integrity Blueprint** (no doubles, bounded emission), **Hardening Blueprint** (bounded authority, custody), **Scaling Blueprint**, **Interop** (idempotent emits), **Perfection Gates A–O** (esp. Gate I: economic invariants; Gate M: appeal paths; Gate L: scaling/chaos; Gate O: PQ toggles).

---

## 1. Invariants (MUST)

These are non-negotiable and enforced via tests, metrics, and runtime guards:

* **[I-G1 No Double Issuance]**: For a given `(epoch_id, run_key, tenant)`, at most one **effective** credit may settle. Replays must classify as `dup` and not re-credit.
* **[I-G2 Conservation]**: For each epoch and tenant, `∑(credits)` ≤ `epoch_budget(tenant)`; never negative totals.
* **[I-G3 Bounded Emission]**: Reward emission follows the configured **emission curve** (caps per epoch/tenant/policy). No out-of-band minting.
* **[I-G4 Auditability]**: Every governance action, parameter change, and override emits an **append-only, signed** audit log record with correlation IDs.
* **[I-G5 Bounded Authority]**: No actor (human or service) has unbounded override. Emergency actions require multi-sig quorum (see §4).
* **[I-G6 Determinism]**: Identical sealed inputs produce identical outputs across runs (hash-pinned policy).
* **[I-G7 Idempotent Egress]**: Ledger emits use idempotency keys; second/subsequent emits are `dup`.
* **[I-G8 Quarantine Safety]**: During dependency brownouts, results are **quarantined** (no emit) until SLOs recover; no silent backlog drain without re-verification.
* **[I-G9 PQ Honesty]**: If `REQUIRE_PQ_ON=true`, all governance and emit signatures adhere to PQ/hybrid policy; toggles are logged and monotonic (no downgrade without quorum).

---

## 2. Roles & Authority

### Roles

* **Policy Owner** (`ron-policy`): curates emission curve, reward formulas, and guardrails. Proposes changes; cannot settle.
* **Ledger Keeper** (`ron-ledger`): settles credits/debits; enforces conservation; can reject invalid/over-cap requests.
* **Rewarder** (`svc-rewarder`): computes rewards deterministically and proposes **intents**; cannot mint/settle.
* **Auditor** (external/read-only): verifies invariants and signatures; can file disputes; no write privileges.
* **Governance Signers**: N-of-M keys authorized to approve parameter changes, freezes, and emergency actions.

### Authority Boundaries

* Policy **proposes** mutable parameters; **does not** execute ledger mutations.
* Ledger **verifies** and **rejects** out-of-policy requests (caps, malformed, replay).
* Rewarder **submits intents** within configured caps; **cannot** bypass ledger validation.
* All actors use **capability tokens (macaroons v1)** with least privilege (scoped caveats: tenant, epoch, action, ceiling, expiry).

---

## 3. Rules & SLAs

### Economic & Process SLAs

* **Settlement SLA (ledger)**: 99.9% of intents settled **< 5s** (p95), audit log available **< 1s** after commit.
* **Compute SLA (rewarder)**: p95 `< 500 ms`, p99 `< 2 s` per PERF; error budget: 5xx `< 0.1%`, 429/503 `< 1%` sustained.
* **Brownout Behavior**: If ledger p99 `> 250 ms` for `> 10 min`, rewarder enters **quarantine-only** (202 Accepted, `quarantine=true`, **no emits**). Exit after 10-min p95 `< 100 ms`.

### Emission Schedule & Caps

* Emission curve is versioned: `policy_id` + `policy_hash` pin each epoch’s rules.
* Per-tenant caps: `{ epoch_cap, actor_cap, category_cap }` enforced in compute and on ledger.
* Deviations: alert + automatic **governance freeze** (quarantine on) until reviewed.

### Appeals & Overrides

* **Disputed** entries: ledger marks `disputed=true`; **no silent rollback**.
* Overrides require **multi-sig** (see §4) and produce an explicit audit record including rationale and diff vs policy.

---

## 4. Governance Process

### Proposal Lifecycle

1. **Draft** (YAML, see §6 schema) → includes rationale, risk class, diffs, proposed go-live.
2. **Review** (Policy Owner + Auditor): validation against invariants + perf budget.
3. **Approve** (Quorum): N-of-M multi-sig over proposal digest (PQ/hybrid per config).
4. **Execute** (Ops): staged rollout (canary → fleet), with rollback plan.
5. **Attest** (Auditor): publish attestation + Merkle root of artifacts.

**Quorum Rules**

* **Param changes (non-emergency)**: `N=2 of M≥3`.
* **Emergency freeze/unfreeze**: `N=3 of M≥5`.
* **PQ downgrade** (turning OFF): **disallowed** without super-quorum `N=4 of M≥5`, and only if legal/compliance bound.

**Timeouts**

* Default reject if quorum not reached within **72h** (non-emergency) or **30m** (emergency).

### Emergency Powers

* **Freeze** (stop emits; quarantine on) under majority multi-sig + auto-page.
* Must be disclosed in audit log within **24h** with RCA plan and expected duration.
* **Auto-unfreeze** prohibited; requires explicit quorum approval.

### Parameter Changes

* Any change to emission curve, caps, SLOs, PQ toggles, or capability caveats follows lifecycle above.
* Canary: ≥ 30 min with metrics parity; otherwise rollback.

---

## 5. Audit & Observability

### Audit Trail (append-only, signed)

* Every governance action produces:

  * `action`, `actor(s)`, `quorum`, `proposal_digest`, `signatures[]`, `pq_mode`, `timestamp`, `corr_id`.
  * Stored in `ron-audit`; hash-chained; exported to cold archive daily.

### Governance Metrics

* `governance_proposals_total{status="draft|review|approved|rejected|executed"}`
* `governance_freezes_total{reason}`
* `ledger_disputes_total{tenant}`
* `rewarder_emissions_total{tenant,policy_id,outcome="accepted|dup|error|quarantine"}`
* `governance_quorum_duration_seconds{type}` (p95 target: < 10m non-emergency)

### Verifiability Artifacts

* **Conservation proofs** (ledger): per epoch/tenant, published digest.
* **Range checks** (rewarder): min/max payouts, cap checks, with seeds for replay.
* **Golden vectors**: S/M/L sealed inputs kept under `testing/performance/vectors/` with signed digests.

---

## 6. Config & Custody

### Governance Config (YAML schema)

```yaml
version: 1
policy:
  policy_id: "rev42"
  policy_hash: "b3:3aa1...f83"
  emission_curve:
    kind: "halving"
    period_epochs: 100_000
    base_per_epoch: "1000000u"
  caps:
    epoch_cap: "250000u"
    actor_cap: "1000u"
    category_cap: { video: "500u", music: "400u" }
pq:
  require_pq_on: true
  scheme: "hybrid(Ed25519, ML-DSA)"
governance:
  quorum:
    param_change: { n: 2, m: 3 }
    emergency_freeze: { n: 3, m: 5 }
    pq_downgrade: { n: 4, m: 5 }
  timeout:
    param_change_hours: 72
    emergency_minutes: 30
custody:
  signer_keys: "ron-kms://governance/*"   # HSM-backed; never exported
  rotation_days: 90
  attestation_required: true
capabilities:
  macaroon_issuer: "svc-passport"
  caveats: [tenant, epoch, action, cap, expiry]
```

### Custody Rules

* Governance signer keys live in **ron-kms/HSM**; **never** on disk/env.
* Rotation every **90 days** or on compromise; rotations produce audit entries.
* Attestations (TEE/HSM) required for signer nodes if configured.

---

## 7. Appeal Path

1. **Raise dispute** (any role) on governance bus topic with `corr_id`, evidence, and affected entries.
2. **Mark disputed** in ledger; freeze related emits (quarantine) for the scope of the dispute.
3. **Assemble proposal** for override/remediation; run lifecycle §4.
4. **Auditor review**: publish public disclosure (redacted as necessary) and close with verifiable artifacts.

Escalation timeline targets:

* Acknowledge within **1h**, temporary containment within **4h**, decision within **72h** (non-emergency).

---

## 8. Rules & SLA Table (Quick Reference)

| Domain              | SLO / Rule | Enforced By                              | Alert                            |
| ------------------- | ---------- | ---------------------------------------- | -------------------------------- |
| No double issuance  | [I-G1]     | Ledger idempotency key, rewarder run_key | Any violation = page             |
| Conservation        | [I-G2]     | Ledger conservation proofs               | Drift >0 = page                  |
| Emission caps       | [I-G3]     | Rewarder compute + ledger reject         | >1% cap miss = page              |
| Auditability        | [I-G4]     | ron-audit append-only                    | Missing entry = ticket           |
| Bounded authority   | [I-G5]     | Quorum checks in svc-rewarder + KMS      | Override w/o quorum = page       |
| Brownout quarantine | [I-G8]     | Ready-degrade + quarantine code path     | p99>250ms 10m = page             |
| PQ mode             | [I-G9]     | Build flags + signer policy              | Downgrade attempt = super-quorum |

---

## 9. Implementation Hooks (enforce in code/CI)

* **Pre-flight guard**: compute path refuses to run if `policy_hash` mismatches or caps missing.
* **Emit gate**: deny emit when `readyz_degraded{cause="ledger_degraded"}` or `quarantine=true`.
* **Quorum verifier**: on governance endpoint, validate N-of-M KMS signatures (PQ/hybrid) against proposal digest.
* **CI gates**:

  * Block merge if `GOVERNANCE.md` changes lack matching **test vectors**/**audit schema** updates.
  * `cargo deny` + `cargo public-api` + perf gates to ensure non-drift.
  * Unit tests for conservation and idempotency invariants; property tests for determinism.

---

## 10. Decision Records (ADR-lite template)

When proposals land, record:

```
ADR: GOV-YYYYMMDD-<slug>
Context: what/why (policy, risk class, baselines)
Decision: precise change (YAML diff)
Quorum: N-of-M, signer key IDs, PQ mode
Rollout Plan: canary scope, KPIs, rollback conditions
Verification: metrics to watch, golden vector diffs
Links: PRs, audit entries, incident tickets
```

---

## 11. Risk Classes & Controls

| Class         | Example                          | Control                                     |
| ------------- | -------------------------------- | ------------------------------------------- |
| R1 (Low)      | Non-economic telemetry tweaks    | Single maintainer + reviewer                |
| R2 (Medium)   | Minor cap adjustments within ±5% | 2/3 quorum, canary 30m                      |
| R3 (High)     | Emission curve change; PQ toggle | 3/5 quorum, canary 2h + rollback plan       |
| R4 (Critical) | Emergency freeze/unfreeze        | 3/5 quorum, post-mortem & public disclosure |

---

## 12. Acceptance Checklist (DoD)

* [ ] Invariants [I-G1…G9] enforced in code and tests (unit, property, integration).
* [ ] Roles and authority boundaries documented and **reflected in macaroon caveats**.
* [ ] Proposal lifecycle endpoints + quorum verifier implemented and tested.
* [ ] Governance metrics exported; dashboards and alerts wired.
* [ ] Custody model documented; keys in KMS/HSM; rotation policy active.
* [ ] Appeal path exercised in quarterly drill (dispute → quorum override).
* [ ] PQ/hybrid policy tested ON/OFF; downgrade path requires super-quorum and is logged.

---

## 13. Appendix

**Blueprints**: Economic Integrity (no doubles, bounded emission), Hardening (bounded authority, custody), Scaling, Omni-Gate.
**References**: Macaroons v1 capability tokens; KMS signer policy; PQ schemes (hybrid Ed25519 + ML-DSA).
**Governance Bus Topics**: `governance.proposals`, `governance.actions`, `governance.disputes`.
**History**: Maintain `docs/governance/HISTORY.md` with disputes, freezes, overrides, and ADRs.

---

