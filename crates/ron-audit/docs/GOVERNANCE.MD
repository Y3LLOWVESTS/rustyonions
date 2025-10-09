

---

# 🏛 GOVERNANCE.md

---

title: Governance & Integrity — ron-audit
status: draft
msrv: 1.80.0
last-updated: 2025-10-08
audience: contributors, ops, auditors, stakeholders
crate-type: lib (integrity plane)
---------------------------------

## 0. Purpose

This document defines the **rules of engagement** for the `ron-audit` integrity plane:

* Transparent, reproducible decision-making for **changes that impact audit integrity**.
* Enforcement of **integrity invariants** (append-only, tamper-evident, bounded authority).
* Clear **role boundaries**, escalation and **appeal paths** when disputes arise.
* SLO-backed commitments (visibility of commits, durability posture), with **observability** to prove compliance.

Ties into:

* **Hardening Blueprint** (bounded authority, key custody, amnesia mode).
* **Scaling & Performance Blueprints** (SLOs, gates).
* **Perfection Gates A–O** (esp. Gate **I**: integrity invariants enforced; Gate **K**: continuous vigilance; Gate **M**: appeal paths; Gate **F/L**: perf & scaling under chaos).

---

## 1. Invariants (MUST)

Non-negotiable rules the audit plane **must** uphold:

* **[I-A1] Append-only**: No deletion, in-place mutation, or re-ordering of audit records. Corrections occur via **new records** that reference the original.
* **[I-A2] Tamper-evident chain**: Each record’s `self_hash` and `prev` link must verify; chain breaks surface as `audit_verify_fail_total > 0` and stop the world for writes (fail-closed or shed).
* **[I-A3] Canonicalization**: Inputs are normalized (e.g., NFC) and size-bounded; canonical bytes are deterministic across runs/hosts.
* **[I-A4] No silent redaction**: Redactions, if allowed, occur **only** via a cryptographic **tombstone** (new record) that: (a) references the redacted record’s address, (b) states a reason code, (c) is gated by **multi-sig** approval, (d) leaves the original bytes intact in durable stores (or indirection swapped) while remaining provably detectable.
* **[I-A5] Bounded authority**: No single admin can bypass invariants. Sensitive actions (freeze, unfreeze, redaction, key rotation) require **N-of-M** approval and are themselves audited.
* **[I-A6] Custody & rotation**: Signing keys live in KMS/HSM; rotations are **cross-signed** in the audit stream; private keys never reside in env/plain files.
* **[I-A7] Observability is truth**: Audit plane exposes metrics/counters that make failures and operator actions **measurable**. Metrics must be registered by hosts once and never silently disabled.

---

## 2. Roles & Authority

### Roles

* **Policy Owner** (project stewards): curates invariants, schema constraints, acceptance gates across crates.
* **Host Owner** (service team, e.g., `svc-gateway`): operates the embedding service; owns readiness, quotas, WAL/export posture.
* **Custodian** (KMS/HSM admins): manages signing keys & rotations under dual control.
* **SRE/Operator**: runs upgrades, scaling, chaos drills; owns alerts and runbook execution.
* **Security (Blue Team)**: leads tamper/bitrot incident response, redaction governance, and post-mortems.
* **Auditor** (internal/external): read-only verification of checkpoints, manifests, and chain proofs.

### Authority Boundaries

* `ron-audit` (lib) **cannot** mutate or delete durable history; it only appends and verifies.
* Hosts decide **posture** (Micronode amnesia vs Macronode durable) but **must** surface metrics and follow fail-closed policy on integrity failures.
* **Custodians** can rotate keys but **cannot** bypass verify; rotations must emit chain-verifiable events.
* **Security** can request redaction via governance proposal; execution requires multi-sig and results in a tombstone record.

---

## 3. Rules & SLAs

### Integrity & Visibility

* **Commit visibility SLO**: Newly appended audit events become visible to in-process readers **≤ 1s** (Micronode) and **≤ 3s** (Macronode with WAL/export).
* **Verification SLO**: Background verification detects a corrupted segment within **≤ 60s** (Macronode) or at read-time (Micronode).

### Durability Posture

* **Micronode** (amnesia ON): No WAL/export; integrity relies on verification and upstream replication; allowed only for dev/edge profiles as documented in CONFIG.
* **Macronode** (durable): WAL batched fsync p95 ≤ 8ms; periodic export + checkpoint; restores must re-verify to **zero** verify-fail.

### Change Control

* **Config changes** impacting audit (queue depth, WAL/export toggle, batch size, checkpoint cadence, readiness behavior) require:

  1. PR with rationale + expected metric deltas,
  2. Staging canary + Grafana snapshots,
  3. Approval from Host Owner **and** Security,
  4. Entry in CHANGELOG and a signed audit record emitted by the host at rollout.

### Redactions

* Allowed only for lawful requests (e.g., PII exposure under policy). Must use **tombstone** flow; no byte erasure in chain. Public redaction ledger is exported with reason codes and approver fingerprints.

---

## 4. Governance Process

### Proposal Lifecycle (for sensitive actions)

**Draft → Review → Approve → Execute → Post-mortem**

* **Quorum**: **N-of-M** (e.g., 2/3) among Security + Host Owner + Custodian (if keys involved).
* **Timeout**: Default reject if quorum not met in **24h** (emergency: 1h).
* **Auditability**: Proposal, votes, and execution emit audit records with addresses and prior hashes.

### Emergency Powers

* **Freeze writes** (fail-closed) when `audit_verify_fail_total > 0` or chain mismatch suspected. Requires immediate Security ack; must log within **15m** and produce an incident record in the audit stream within **24h**.
* **Unfreeze** only after **full re-verification** and post-mortem approval.

### Parameter Changes

* WAL batch, export cadence, queue depth, readiness gating → go through the lifecycle above. Canary first, then fleet.

---

## 5. Audit & Observability

### Mandatory Metrics (host-registered)

* `audit_appended_total`, `audit_bytes_total`
* `audit_queue_depth`, `audit_drop_total{reason}`
* `busy_rejections_total{endpoint="emit"}`
* `audit_verify_fail_total`
* Macronode: `wal_fsync_seconds` (histogram), `export_batch_seconds` (histogram), `audit_export_checkpoints_total`

### Governance Metrics

* `audit_gov_proposals_total{status}` (`draft|approved|rejected|executed|expired`)
* `audit_gov_freezes_total`
* `audit_redactions_total{reason}`
* `audit_key_rotations_total`

### Verifiability

* Every export includes a **manifest** with Merkle roots; checkpoints are signed by the current key and cross-signed on rotation.
* Field tools (`ronctl audit verify`) must reproduce **zero** verify-fail for healthy sets.

### Drills (Red-Team)

* **Rogue admin** simulation: attempt unauthorized redaction; assert denial (multi-sig enforced) and **no metrics suppression**.
* **Bitrot** simulation: single-chunk corruption triggers verify-fail and freeze flow.

---

## 6. Config & Custody

### Config (host)

* Declare posture (`AUDIT_WAL_ENABLED`, `AUDIT_EXPORT_ENABLED`), batch sizes, queue depths, checkpoint cadence, readiness behavior on failure (fail-closed vs shed).
* Declare `AUDIT_KEY_ID` (KMS alias) and rotation policy window.

### Custody (keys)

* Keys in **ron-kms**/HSM; **never** in env or plaintext files.
* **Rotation**: every **90 days** or on incident. Rotations must:

  1. Emit a rotation-start audit record,
  2. Cross-sign first checkpoint under the new key,
  3. Retain prior key until **two** checkpoints confirmed,
  4. Emit rotation-complete record.

---

## 7. Appeal Path (Disputes & Corrections)

* **Dispute**: Append a **dispute record** referencing the contested event (`disputed=true`, reason). No rollback.
* **Appeal**: File a governance proposal to attach a **tombstone** (if policy permits). Requires quorum and produces its own audit record.
* **Disclosure**: Public export of the dispute/tombstone is mandatory in the next checkpoint cycle.
* **Escalation**: If disagreement persists, Security convenes an appeals review; decision and rationale are appended to the audit stream.

---

## 8. Rules & SLAs (Ops-Facing)

* **Freeze SLA**: If `audit_verify_fail_total > 0`, host must block writes or shed within **≤ 60s** while emitting a freeze record.
* **Recovery SLA**: From freeze to validated restore **≤ 15m** on staged runbooks (see RUNBOOK §6.2).
* **Chaos cadence**: Quarterly drills (latency, disk-full, storm, bitrot) with attached dashboards.
* **Perf gates**: No release may regress perf >10% vs last green baselines (PERFORMANCE.md).

---

## 9. Acceptance Checklist (DoD)

* [ ] Invariants [I-A1…A7] enforced in code paths and tests (unit/property/loom/fuzz).
* [ ] Role boundaries and sensitive action **quorum** implemented; proposal/approval/execute records emitted.
* [ ] Metrics: governance counters live and scraped; alerts wired (verify-fail, shedding budget, WAL p95).
* [ ] Custody: KMS integration in hosts; rotation cross-sign flow validated in staging.
* [ ] Redaction/tombstone flow tested (staging) with multi-sig and public export.
* [ ] Chaos drills executed and documented (latency, disk-full, storm, bitrot).
* [ ] RUNBOOK, PERFORMANCE, TESTS cross-refs current; PR template requires governance impact section.

---

## 10. Appendix

### A. Canon & Blueprints

* **Hardening Blueprint** — bounded authority, custody, amnesia.
* **Scaling Blueprint** — SLO grammar, perf gates, chaos.
* **Concurrency & Aliasing Blueprint** — bounded queues; shed-not-block.
* **Perfection Gates** — A (metrics), F (perf), I (integrity), K (vigilance), L (black swan), M (appeals).

### B. References

* Capability tokens/macaroons (for host governance endpoints).
* TLA+/Alloy sketches (chain invariants, governance quorum).
* Governance schema: `/docs/spec/governance.md` (proposal records, tombstone format).

### C. History (fill as events occur)

* 2025-Q4: Initial governance standing up (multi-sig, rotation cross-sign).
* … add disputes, freezes, overrides with links to incident reports and audit addresses.

---

**Summary**: This governance charter converts `ron-audit` from “a hashing library” into a **verifiable institution**: bounded powers, cryptographic accountability, repeatable procedures, and public evidence that integrity is not optional—it’s enforced.
