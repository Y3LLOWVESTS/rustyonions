
---

# 🏛 GOVERNANCE.md

---

title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-09-19
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ
-----------------------

## 0) Purpose

This document defines the **rules of engagement** for `svc-wallet`’s economic/policy logic.
It ensures:

* Transparent, auditable decision-making.
* Enforcement of **economic invariants** (no doublespends, bounded issuance, conservation).
* Clear **authority boundaries** and **appeal paths**.
* SLA-backed commitments to external consumers.

It ties into:

* **Economic Integrity Blueprint** (no doubles, bounded issuance, conservation).
* **Hardening Blueprint** (bounded authority, custody, amnesia mode).
* **Perfection Gates A–O** (esp. **Gate I:** bounded economic invariants, **Gate M:** appeal paths, **Gate K:** continuous vigilance).

---

## 1. Invariants (MUST)

Non-negotiable rules for `svc-wallet`:

* **[I-G1] No Double Spend**
  A `(account_id, sequence, idempotency_key)` tuple may lead to **at most one** debit in `ron-ledger`.

* **[I-G2] Conservation / Balanced Entries**
  Every committed transaction must balance across accounts (credits = debits) in `ron-ledger`.

* **[I-G3] Bounded Issuance**
  Wallet cannot mint. Issuance (if any) is **only** via governance-approved ledger actions, executed by `ron-ledger` under multi-sig and policy caps.

* **[I-G4] Capability-Gated Mutations**
  All mutating endpoints require **macaroon capabilities** with correct scopes; no ambient/admin bypass.

* **[I-G5] Auditability**
  All governance actions and exceptional flows (freezes, overrides, disputes) are emitted to `ron-audit` (append-only, signed), and reflected in metrics.

* **[I-G6] Bounded Authority**
  No single actor (human or service) possesses unilateral power to mint, reverse settled ledger entries, or bypass invariants.

* **[I-G7] Deterministic Replay / Idempotency**
  Retries must not change economic outcomes; replays reference the original decision via idempotency keys.

---

## 2. Roles & Authority

### 2.1 Roles

* **Policy Owner (`ron-policy`)**
  Authors declarative policies (quotas, allowed opcodes, emission schedules), proposes governance actions.

* **Ledger Keeper (`ron-ledger`)**
  Executes settlement with conservation checks; rejects non-conforming requests; stores immutable truth.

* **Wallet Service (`svc-wallet`)**
  Enforces anti-doublespend/sequence gates, verifies caps, mediates requests to ledger, and updates counters (`ron-accounting`).

* **Rewarder (`svc-rewarder`)**
  Distributes rewards per policy; cannot mint; must respect ledger constraints.

* **Auditor (External / Read-only)**
  Verifies audit streams, metrics, and conservation proofs; can raise disputes.

* **KMS Custodian (`ron-kms`/HSM)**
  Holds governance & service keys; orchestrates rotation and access control.

### 2.2 Authority Boundaries

* **Policy ≠ Execution**: Policy proposes; **Ledger** executes or rejects.
* **Wallet ≠ Mint**: `svc-wallet` validates and forwards; it **cannot** mint or force settlement.
* **Rewarder ≠ Mint**: `svc-rewarder` allocates pre-budgeted rewards only; issuance governed by ledger policy.
* **Caps Everywhere**: Every actor authenticates with scoped macaroons; **no backdoors**.
* **Emergency Power = Multi-Sig Only**: Any freeze/override requires N-of-M signers and is time-boxed.

---

## 3. Rules & SLAs

### 3.1 Economic Rules

* **Settlement Finality**: Once `ron-ledger` commits, wallet treats it as final; corrections happen via **forward** compensating entries, not rollback.
* **Disputes**: Mark entries `disputed=true` and trigger governance workflow; no silent changes.

### 3.2 Operational SLAs (reference targets; see PERFORMANCE.md)

* **Settlement Path Availability**: ≥ 99.95% monthly.
* **Settlement Latency**: 99.9% of commits < **5s** end-to-end (wallet→ledger).
* **Audit Visibility**: audit event emitted < **1s** after commit.
* **Policy Propagation**: Hot-reload to active policy < **30s** after approval.

### 3.3 Policy/Quota Rules

* **Quotas**: Per-account and per-tenant spending/ops quotas enforced; exceeding quotas triggers **structured rejects** and metrics.
* **Emission Schedules**: If configured, reward/emission curves are predefined (e.g., epoch halving). Deviations → **alert** + **governance freeze** (see §4.3).

---

## 4. Governance Process

### 4.1 Proposal Lifecycle

State machine: **Draft → Review → Approve → Execute → Publish**.

* **Submission**: Policy Owner submits `Proposal{ id, type, params, rationale, not_before, expiry }`.
* **Review**: Auditors and stakeholders comment; static checks (schema, quotas, budget limits) must pass.
* **Approval (Quorum)**: N-of-M multi-sig using keys in KMS/HSM.
* **Execution**: Ledger Keeper executes if valid; Wallet updates policy cache via hot-reload.
* **Publish**: Append to `ron-audit` with proposal hash, signatures, and execution receipt.

**Default**: proposals auto-expire at `expiry`; non-approved = rejected.

### 4.2 Parameter Changes (Policy Surface)

* **Governance-gated**: emission curves, per-tenant quotas, SLO/SLA targets, fee schedules, allowed opcodes.
* **Operational-level (no vote)**: telemetry sampling rate, histogram buckets, non-economic logging levels.

### 4.3 Emergency Powers

* **Freeze**: Pause **mutating** wallet ops (reads may proceed) when: suspected compromise, emission anomaly, or ledger inconsistency.
* **Activation**: N-of-M signers; effect time-boxed (e.g., 2h), auto-sunset unless renewed.
* **Disclosure**: Freeze event published to `ron-audit` within **24h** with rationale and incident ID.

---

## 5. Audit & Observability

* **Audit Logs**: Append-only, signed events:

  * `governance_proposal_created|approved|executed|expired`
  * `freeze_activated|released`
  * `dispute_opened|resolved`
* **Metrics (Prometheus)**:

  * `governance_proposals_total{status}`
  * `ledger_disputes_total{reason}`
  * `rewarder_emissions_total{epoch}`
  * `policy_reload_seconds{result}`
  * `wallet_conflicts_total` (sequence/idempotency)
  * `quota_exhaustions_total` (policy pressure)
* **Verifiability**:

  * **Conservation Proofs**: periodic proofs over ledger slices (credits=debits).
  * **Range Checks**: rewards within epoch budgets.
  * **Drill Logs**: red-team/chaos exercises proving bounded authority.

---

## 6. Config & Custody

### 6.1 Config (must declare)

* **Policy Parameters**: quotas, allowed opcodes, reward curves, dispute time windows.
* **SLA Targets**: settlement latency/availability, audit emission lag.
* **Readiness Strategy**: degrade-first (writes fail readiness before collapse).
* **OAP/1 Bounds**: `body_max_bytes=1MiB`, `decompress_max_ratio=10`.
* **Amnesia Mode**: RAM-only posture; no secret persistence.

### 6.2 Custody

* **Key Storage**: `ron-kms` or HSM; no raw private keys in env/files.
* **Rotation**: governance and service keys rotated **≤ 90 days** or immediately on compromise; hybrid/PQ-ready where available.
* **Access**: least-privilege, audited retrieval; emergency break-glass requires dual control and auto-revoke.

---

## 7. Appeal Path (Disputes & Overrides)

1. **Raise Dispute**: mark transaction `disputed=true`; emit `ledger_dispute_opened` with corr_id.
2. **Propose Remedy**: governance proposal for forward correction (never rollback), including rationale and conservation proof.
3. **Quorum & Execute**: N-of-M approval; ledger applies compensating entries.
4. **Audit & Disclosure**: publish resolution in `ron-audit`; notify stakeholders.
5. **Escalation**: if quorum fails or safety violated → **freeze** and call external auditor review.

**No Silent Rollbacks.** All remedies are additive and publicly auditable.

---

## 8. Capability Scopes (Reference)

* `wallet:spend` — debit operations (requires per-account sequence + idempotency key).
* `wallet:refund` — compensating credit (governance-gated policy path, not unilateral).
* `wallet:read` — non-mutating balance/history reads.
* `policy:propose` — submit governance proposals.
* `policy:approve` — sign approval with KMS-managed key (counts toward quorum).
* `policy:freeze` — activate/deactivate emergency freeze (multi-sig).

All scopes are **least-privilege** and time/tenant constrained (macaroon caveats).

---

## 9. Governance Bus & Schemas

* **Topics**:

  * `gov.proposals` (Draft/Review/Approve/Execute/Publish)
  * `gov.freeze` (activate/release)
  * `gov.disputes` (open/resolve)
* **Proposal Schema (summary)**:

  ```json
  {
    "id":"uuid",
    "type":"quota|emission|fee|freeze|unfreeze|param_update",
    "params":{ "...": "domain-specific" },
    "rationale":"string",
    "not_before":"rfc3339",
    "expiry":"rfc3339",
    "signatures":[{"kid":"kms-key-id","sig":"base64"}]
  }
  ```
* **Determinism**: proposals are hashed (`blake3`) and referenced in audit events.

---

## 10. Rules & SLAs — Alert Hooks (Ops)

* **Settlement Latency Breach**: p99 > 5s for 15m ⇒ page SRE + Policy Owner.
* **Proposal Lag**: `policy_reload_seconds` p95 > 30s for 10m ⇒ warn.
* **Quota Storm**: `quota_exhaustions_total / requests_total > 1%` for 10m ⇒ warn; > 5% ⇒ page.
* **Conflict Storm**: `wallet_conflicts_total > 10/min` for 10m ⇒ investigate client sequencing or abuse.
* **Freeze Active > 2h**: page Governance Lead; require renewal or release.

---

## 11. Security & Hardening Tie-ins

* **Amnesia Mode**: when enabled, ensure no secret at rest; keys ephemeral; crash-safe zeroization.
* **No Admin Bypass**: all ops require caps; emergency actions require **multi-sig**; all signed and audited.
* **PQ-Ready Posture**: governance/KMS can rotate to hybrid/PQ algs without wallet code changes; wallet must accept new KMS bundles.

---

## 12. Acceptance Checklist (Definition of Done)

* [ ] **Invariants** ([I-G1]…[I-G7]) enforced in code & tests (see TESTS.md).
* [ ] **Roles & Boundaries** documented; no mint path in wallet; rewarder bounded.
* [ ] **Governance Process** implemented: proposal lifecycle, quorum checks, expiries.
* [ ] **Metrics & Audit** exported: proposals, disputes, freezes, policy reloads.
* [ ] **SLAs** wired to alerts (settlement latency, audit lag, quota/conflict storms).
* [ ] **Custody**: keys in KMS/HSM; rotation policy enacted ≤ 90 days.
* [ ] **Appeal Path** validated in chaos drill (freeze→resolve without econ violation).
* [ ] **Interop Bounds** enforced (OAP/1 size/ratio) with rejects observable.
* [ ] **Docs**: schemas/versioning for proposals published; bus topics stable.

---

## 13. Appendix

**Blueprints**: Economic Integrity; Hardening; Scaling; Microkernel; Developer Suite.
**References**: Macaroons (v1) capability model; DTO schemas; proposal schemas; conservation proof notes.
**History**: Maintain a changelog of disputes, overrides, freezes with links to audit entries.

---

