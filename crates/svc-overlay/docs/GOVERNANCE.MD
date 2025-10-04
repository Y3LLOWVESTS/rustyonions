

---

# 🏛 GOVERNANCE.md — `svc-overlay`

---

title: Governance & Economic Integrity — svc-overlay
status: draft (facet-aware)
msrv: 1.80.0
last-updated: 2025-10-03
audience: contributors, ops, auditors, stakeholders
crate-type: service (policy-enforcement)
----------------------------------------

## 0. Purpose

`svc-overlay` provides the **session & gossip plane** and enforces ingress **policy**, but it **does not** mint, settle, or hold economic state. This document defines the **rules of engagement** for how `svc-overlay` applies policy (capabilities, quotas, deny/allow, PQ posture) in a **transparent, auditable, and bounded** manner.

It ensures:

* Clear **authority boundaries** (overlay enforces; policy/ledger decide & settle).
* Enforced **economic-adjacent invariants** (no doubles by enforcement; no silent overrides).
* **Appeal paths** with SLAs.
* Verifiable, **append-only audit** of governance-relevant actions.

**Ties into:** Economic Integrity Blueprint (no doubles, bounded issuance), Hardening Blueprint (bounded authority, custody), **Perfection Gates** (Gate **I**: bounded economic invariants, Gate **M**: appeal paths, Gate **K**: continuous vigilance).

---

## 1. Invariants (MUST)

> Non-negotiable, testable rules. Violations require immediate freeze + incident.

* **[G-OL1 Non-custody]** `svc-overlay` holds **no economic state**; it cannot mint, burn, or settle.
* **[G-OL2 Capability Required]** All **mutating or privileged** operations require a valid **capability token** (macaroon v1) chained to the current **policy root**; overlay never self-issues caps.
* **[G-OL3 Bounded Enforcement]** Rate limits, quotas, and deny/allow lists are **scope- and time-bounded** (TTL required). No **unbounded admin override** exists.

  * **[G-OL3.1 Facet-Bounded]** Facet-specific enforcement (e.g., Graph/Search via `svc-index`, Feed/Media via `svc-mailbox`) **inherits** these TTL/scope bounds; **no facet-specific overrides** without a signed policy bundle.
* **[G-OL4 Truthful Readiness]** `/readyz` reflects policy health (e.g., policy fetch/verify failure flips **not-ready**). No masking under load.
* **[G-OL5 Append-Only Audit]** Every governance-relevant action (policy apply, deny/allow change, quota change, PQ toggle, emergency freeze) produces a **signed, append-only** audit record.
* **[G-OL6 No Doubles by Enforcement]** Overlay must **not** cause economic doubles (e.g., double PUT credit) via retries: idempotency keys required; replay windows bounded.

  * **[G-OL6.1 ZK Phase]** Future ZK commitments/proofs are **ledger-side**; overlay never verifies or carries secret material beyond capability enforcement.
* **[G-OL7 Cryptographic Posture Changes]** PQ↔classical toggle requires a recorded **governance event**; cannot be silent.
* **[G-OL8 Fairness & No Starvation]** Backpressure and shedding **must be fair**; long-lived peers cannot starve others of the same class.

---

## 2. Roles & Authority

### Roles

* **Policy Owner (`ron-policy`)**: Defines policy (caps, quotas, lists, PQ posture).
* **Ledger Keeper (`ron-ledger`)**: Settles accounts & conserves value (authoritative).
* **Rewarder (`svc-rewarder`)**: Distributes rewards **within ledger caps**; no mint.
* **Audit (`ron-audit`)**: Append-only registrar of governance events.
* **Service Owner (overlay maintainers)**: Operate/enforce policy; cannot alter policy roots.
* **SRE / On-Call**: Executes approved operations; cannot bypass policy cryptography.
* **External Auditor**: Read-only access to logs/metrics/proofs.
* **ARM/Edge Note**: Amnesia mode governance follows general policy; **no persistent state overrides** on edge nodes.

### Authority Boundaries

* Overlay may **enforce** (rate limits, deny/allow, quotas, PQ posture **only if** a signed policy allows).
* Overlay may **not**: mint caps, settle ledger, or permanently ban without **multi-sig** policy instruction.
* Ledger may **reject** requests inconsistent with conservation; overlay surfaces the error transparently.
* Rewarder cannot mint; overlay cannot instruct rewarder—only carry frames.
* All sensitive operations require **capabilities** derived from the **current** policy root (checked at runtime).

---

## 3. Rules & SLAs

### Enforcement & Admission

* **Capabilities**: Verified macaroons; caveats include **scopes**, **quotas**, **expiry**, **peer class**.
* **Quotas/Rate Limits**: Applied per **cap**, **peer**, **tenant**, **IP class**; fairness enforced (WFQ/FQ).
* **Deny/Allow Lists**: Must include **reason**, **TTL**, **scope**; default **deny TTL ≤ 24h** unless extended via multi-sig governance.
* **Facet Appeals**: Graph/Feed/Search denials follow the general appeal path; facet evidence may include **ingest-lag** or **index staleness** proofs from `svc-index`/`svc-mailbox`.

### Service SLAs (governance-relevant)

* **Policy apply latency**: new signed policy bundle applied within **30s** (p95).
* **Audit latency**: governance events visible in `ron-audit` within **1s** after apply.
* **Appeal acknowledgment**: within **4h**; resolution within **72h** barring multi-sig delay.
* **Freeze scope**: **narrowest possible** (cap/peer/tenant); global freeze requires **N-of-M** signers and **explicit expiry**.

### Economic Integrity Adjacent

* **Idempotency**: Overlay must require/request idempotency keys for billable ops; duplicates with same key **must** be coalesced.
* **Retry policy**: Exponential backoff with jitter; max retries capped to avoid accidental doubles downstream.

---

## 4. Governance Process

### Policy / Parameter Changes

* **Change types**: quotas, rate classes, deny/allow TTL defaults, PQ posture, allowed cipher suites, readiness policies.
* **Lifecycle**: Draft → Review → **Approve (N-of-M)** → **Canary** → Execute → Audit.
* **N-of-M thresholds** (policy-configurable in `ron-policy`):

  * **Standard changes** (quotas, lists, posture): **3-of-5**
  * **Emergency/global freeze**: **4-of-6**
* **Time-boxed**: If quorum not reached in **T = 24h**, default **reject**.
* **Canary**: Apply to ≤10% of overlay replicas; monitor metrics for **15m** before fleet rollout.
* **Backout**: Prior version retained; backout available within **1m**.

### Emergency Powers

* **Freeze**: SRE may issue **temporary freeze (≤2h)** on scope (cap/peer/tenant) with auto-expiry; requires **immediate audit record** + paging policy owner.
* **Global Freeze**: Only via **multi-sig**; must be logged and disclosed within **24h**.

---

## 5. Audit & Observability

### Audit (append-only, signed)

* `governance_event{type,scope,issuer,reason,ttl,prev_hash,bundle_hash}`
  Types: `policy_apply`, `deny_add`, `deny_remove`, `quota_change`, `pq_toggle`, `emergency_freeze`, `freeze_expire`.

### Metrics (minimum)

* `governance_proposals_total{status}`
* `policy_apply_total{result}` and `policy_apply_latency_seconds`
* `capability_validation_failures_total{reason}`
* `quota_exhaustions_total{scope}`
* `denied_requests_total{reason}`
* `pq_posture{mode="classical|hybrid"}` (gauge)

### Verifiability

* **Proofs**: policy bundles use **detached signatures**; overlay logs the **bundle hash** it enforced.
* **Range checks**: deny/allow TTLs checked against max; quota deltas within policy bounds.
* **CI Proof**: unit/integration tests assert audit events are emitted on policy applies/toggles; fail CI if missing.

### Red-Team Drills (Quarterly)

* Simulate **rogue admin** without cap → overlay must **reject & audit**.
* Attempt **silent PQ toggle** → overlay refuses & audits attempt.
* Inject **invalid policy signature** → overlay refuses apply; `/readyz` flips.

---

## 6. Config & Custody

* **Config declares**: policy endpoint(s), **policy root key ID**, quota classes, deny/allow TTL defaults, PQ posture policy, SLA targets.

* **Custody**:

  * Keys live in **ron-kms** or HSM; overlay uses **verification** keys only.
  * No raw private keys on disk/env.
  * **Rotation**: policy roots **90d**; capability roots **30d**; TLS keys per org policy.
  * **Tor/Onion** keys (if used) stored via KMS file provider; rotations auditable.

* **Startup checks** (must pass): reachable policy endpoint, valid **signature chain**, policy freshness within **max_age**.

---

## 7. Appeal Path

1. **File dispute**: client/tenant submits **corr_id**, capability ID, timestamps, and evidence.
2. **Overlay response**: acknowledge within **4h**; lock conflicting deny/quota until review; produce audit trail.
3. **Governance review**: policy owner investigates; may propose **override** (N-of-M).
4. **Outcome**:

   * **Override accepted** → policy bundle updates; audit entries for change & rationale.
   * **Override rejected** → dispute remains logged (`disputed=true`); no silent rollback.
5. **Disclosure**: material incidents disclosed in release notes / public audit log as policy dictates.

---

## 8. Acceptance Checklist (DoD)

* [ ] **[G-OL1..G-OL8]** invariants encoded as tests/CI checks.
* [ ] Roles & **authority boundaries** documented in code + ops runbooks.
* [ ] **Policy apply** pipeline enforces signature + freshness; canary + backout in place.
* [ ] **Audit events** emitted for all governance-relevant actions; retained & queryable.
* [ ] **Metrics** above exported; alerts tied to thresholds (e.g., validation failures spike).
* [ ] **Appeal path** tested in chaos drill (freeze/appeal/override).
* [ ] **Custody model** validated; no private keys on overlay hosts.
* [ ] **ZK/facet governance hooks** validated (phased hand-off; no facet overrides on overlay).
* [ ] Perfection Gates **I** (bounded economic invariants) and **M** (appeals) marked **green**.

---

## 9. Change-Impact Matrix (who can change what)

| Parameter / Action              | Level | Control                              | Requires              | TTL / Scope                    |
| ------------------------------- | ----- | ------------------------------------ | --------------------- | ------------------------------ |
| Quota class weights             | Std   | Policy bundle                        | **3-of-5**            | Persistent until next change   |
| Denylist add/remove             | Std   | Policy bundle / Emergency (SRE temp) | **3-of-5** / SRE temp | Temp ≤ 24h (SRE) / policy sets |
| PQ posture (classical ↔ hybrid) | Std   | Policy bundle                        | **3-of-5**            | Persistent; audited            |
| Global freeze                   | Emer  | Governance proposal                  | **4-of-6**            | Explicit expiry required       |
| Policy root rotation            | Std   | KMS/Policy                           | **3-of-5** + rollout  | Per rotation schedule          |

---

## 10. References

* **Blueprints**: Economic Integrity (no doubles, bounded emission); Hardening (bounded authority, custody); Scaling/Omnigate; Perfection Gates **I**, **M**, **K**.
* **Docs**: `SECURITY.md`, `CONFIG.md`, `OBSERVABILITY.md`, `PERFORMANCE.md`, `INTEROP.md`, `RUNBOOK.md`.
* **Standards**: Macaroons v1 (capabilities), Signed policy bundles, PQ posture policy.
* **Spec**: governance schema (see `/docs/spec/governance.md`).

---

## 11. History

Record governance incidents (disputes, overrides, freezes), decisions, rationale, links to audit records and PRs.

* `2025-10-03` — Initial governance charter for `svc-overlay` adopted (facet-aware revision).

---

### Why this works

* **Non-custodial clarity**: overlay enforces without owning value.
* **Bounded power**: TTLs, scopes, explicit N-of-M, canary/backout.
* **Provability**: signed policy bundles + append-only audits + CI checks.
* **Resilience**: truthful readiness; appeal SLA; facet hooks without scope creep.

---
