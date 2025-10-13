```markdown
# 🏛 GOVERNANCE.md — svc-mailbox

---
title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-12
audience: contributors, ops, auditors, stakeholders
crate-type: service (policy/econ overlay)
---

## 0. Purpose

This document defines the **rules of engagement** for `svc-mailbox`—the store-and-forward messaging plane that powers notifications and feed fan-out.  
It ensures:

- Transparent and auditable decision-making over throttles, quotas, retention, and moderation/fraud controls.  
- Enforcement of **economic invariants** (no doubles, bounded issuance of delivery capacity, fair share across tenants).  
- Clear **authority boundaries** and **appeal paths** for tenants and end users.  
- SLA-backed commitments to external consumers and operators.  

It ties into:  
- **Economic Integrity Blueprint** (no doubles, bounded issuance/fair share).  
- **Hardening Blueprint** (bounded authority, capability custody, amnesia mode).  
- **Perfection Gates A–O** (esp. Gate **I**: bounded invariants; Gate **K**: vigilance; Gate **M**: appeal paths; Gate **L**: black-swan readiness).  

---

## 1. Invariants (MUST)

Non-negotiable rules for `svc-mailbox`:

- **[I-M1] No silent drops.** Every rejected write is surfaced with a reason (`quota|degraded|auth|storage|integrity`) and counted.  
- **[I-M2] No double issuance of capacity.** Delivery capacity is a bounded resource; burst/steady tokens are conserved per tenant/topic.  
- **[I-M3] At-least-once without duplicate effects.** Duplicate deliveries are permitted by design but **must** be idempotent at API boundary (idempotency key required for mutating SEND).  
- **[I-M4] Bounded retention.** Ready, visibility, and DLQ queues have explicit TTLs/size caps; no unbounded growth.  
- **[I-M5] Governance changes are auditable.** All policy changes (quotas, retention, visibility, moderation toggles) are logged, versioned, and attributable.  
- **[I-M6] Authority is bounded.** No single actor can mint capacity, bypass quotas, or purge DLQ without multi-party controls.  
- **[I-M7] Privacy & least knowledge.** Operators see metadata (KID/cap ID, tenant, sizes), not message bodies (unless tenant grants redacted inspection under incident protocol).  
- **[I-M8] Protocol invariants.** DTOs are strict (`deny_unknown_fields`), OAP/1 frame limits enforced (max_frame=1 MiB), visibility_ms within configured bounds.  
- **[I-M9] Amnesia fidelity (Micronode).** Edge profile must not persist user content to disk; audits confirm zero on-disk artifacts.

---

## 2. Roles & Authority

### 2.1 Roles

- **Policy Owner (ron-policy maintainers):** Authors policy bundles (quotas, retention, moderation toggles), proposes governance changes.  
- **Service Owner (svc-mailbox maintainers):** Operates the service, implements governance mechanics, enforces invariants.  
- **Abuse/Trust Team:** Tunes moderation/anti-abuse levers (rate limits, blocklists), proposes content-level mitigations (hash lists), never reads content without tenant authorization.  
- **Tenant Owner (per tenant/app):** Receives fair-share allocation, may request quota changes and provide allowlists/blocklists under contract.  
- **Auditor (internal/external):** Read-only access to governance logs/metrics, verifies adherence to invariants and SLAs.  
- **SRE/On-Call:** Executes emergency levers (brownout, freeze) within pre-approved bounds; cannot change long-term policy unilaterally.  

### 2.2 Authority Boundaries

- **Policy Owner** can **propose** but not **enforce** runtime changes; proposals become effective only after quorum + signed publish.  
- **Service Owner/SRE** can **activate emergency presets** (brownout/freeze) within pre-approved envelopes and **must** file an incident + rollback plan.  
- **Abuse/Trust** can **enable pre-vetted rule sets** (e.g., anti-spam) but cannot bypass capability verification or expand storage/retention.  
- **Tenant Owners** cannot affect other tenants’ quotas or policy.  
- **All actors** must present **capability tokens (macaroons v1)** scoped to their role. No ambient/root authority.

---

## 3. Rules & SLAs

### 3.1 Service SLAs (public)

- **Delivery latency (intra-AZ):** p95 enqueue→dequeue < **50 ms**.  
- **Admission control:** shed-writes (503) must engage before internal queues exceed **80%** capacity.  
- **Audit availability:** governance change log entries emitted within **1 s** after commit.  
- **Auth correctness:** invalid/expired capabilities are rejected with **401/403**; false-accept rate = **0**.

### 3.2 Fair-Share Economics

- **Per-tenant token buckets:** `(rate, burst)` for SEND; independent limits for fan-out hop.  
- **DRR (Deficit Round Robin)** across hot topics prevents starvation.  
- **No off-books capacity.** All exceptions use named presets with expiry (e.g., `brownout-1`, `event-spike-48h`).

### 3.3 Moderation & Abuse Controls

- Rate limits, deny-lists (hashes/origins), and anomaly thresholds are **configuration**, not code.  
- Enforcement actions are **non-destructive** (reject/quarantine) and **audited**; original messages are retained per tenant retention rules or dropped in Micronode.

### 3.4 Overrides & Appeals (high-level)

- **Overrides** require **N-of-M multi-sig** approval (see §4) and expire automatically.  
- **Appeals**: tenants can dispute rejections or throttles; mailbox marks items `disputed=true` (metadata only) and maintains an appeal trail (see §7).

---

## 4. Governance Process

### 4.1 Proposal Lifecycle

1. **Draft** (PR + policy bundle diff)  
2. **Review** (automated checks + stakeholder sign-off)  
3. **Approve** (**N-of-M** multi-sig: `Policy Owner ×2`, `Service Owner ×1`, `Abuse/Trust ×1`)  
4. **Publish** (signed policy to config channel; version bump)  
5. **Execute** (hot reload)  
6. **Verify** (metrics + log assertion; rollback window)

- **Default reject** if quorum not met in **72 h**.  
- All steps emit governance events to a dedicated **bus topic: `governance.mailbox`**.

### 4.2 Emergency Powers

- **Brownout (pre-set):** raise rejection threshold, shed earlier. **Scope-limited** (duration ≤ 2 h, auto-revert). Requires SRE + Service Owner ack.  
- **Write Freeze:** pause new SEND for tenant/topic set. Requires **majority multi-sig** within **15 min** or auto-revert.  
- **Disclosure:** incident record in audit log within **24 h** with rationale, scope, and impact.

### 4.3 Parameter Changes (examples)

| Parameter            | Range/Rule                         | Requires |
|---------------------|------------------------------------|---------|
| `send.rate`         | 1–100k rps per tenant              | Proposal lifecycle |
| `send.burst`        | 1–1,000                            | Proposal lifecycle |
| `visibility_ms`     | 500–60,000                         | Proposal lifecycle |
| `retention.ready`   | 0–72 h (0 for Micronode)           | Proposal lifecycle |
| `retention.dlq`     | 0–30 d (0 for Micronode)           | Proposal lifecycle |
| `payload_max_bytes` | ≤ 1 MiB (protocol hard cap)        | Code + policy (cannot exceed protocol) |

---

## 5. Audit & Observability

### 5.1 Audit Log (append-only)

- Signed entries with: `timestamp`, `actor`, `cap_id`, `change_set`, `before/after`, `quorum_refs`, `expiry` (if preset).  
- Stored in **append-only** channel; verifiable via hash chain (daily anchor).

### 5.2 Governance Metrics

- `governance_policy_reload_total{result}`  
- `governance_proposals_total{status="draft|approved|rejected|expired"}`  
- `quota_overrides_active{tenant}`  
- `dlq_purges_total{reason="expiry|manual|policy"}`  
- `shed_writes_events_total{reason}`  
- `disputed_messages_total{tenant}`

### 5.3 Verifiability

- **Conservation proofs:** token-bucket accounting equals admitted + rejected + shed.  
- **Range checks:** parameter values remain within allowed envelopes; violations alert and auto-rollback.

---

## 6. Config & Custody

### 6.1 Config File MUST Declare

- **Quotas:** per-tenant SEND/FANOUT `(rate, burst)`; DRR weights.  
- **Retention:** `ready_ttl`, `visibility_ms`, `dlq_ttl`.  
- **Moderation:** rule-set IDs (hash lists, anomaly flags).  
- **SLA Targets:** latency, readiness, audit emission.  
- **Profiles:** `micronode|macronode` (amnesia toggle).

### 6.2 Key Custody

- Capability verification uses key material anchored in **ron-kms** or HSM; only **KID** stored/configured in mailbox.  
- **No private keys** in env/files.  
- **Rotation policy:** every **90 days** or after compromise; mailbox supports dual-KID grace to avoid downtime.

---

## 7. Appeal Path (Tenant-Facing)

1. **File Dispute** via governance API or email; include `corr_id`, timestamps, and rejection codes.  
2. **Mailbox Action:** mark `disputed=true` (metadata); preserve relevant counters; no content exposure required.  
3. **Review Board:** Policy Owner + Service Owner + Abuse/Trust triage within **2 business days**.  
4. **Outcome:**  
   - **Uphold:** rationale + pointers to policy doc; guidance provided.  
   - **Adjust:** issue a **time-boxed override** (preset) or **policy change proposal** (full lifecycle).  
5. **Disclosure:** anonymized dispute statistics published monthly.

**No silent rollbacks.** All decisions are logged and attributable.

---

## 8. Economic Integrity & Fairness

- **No tenant pre-emption.** DRR ensures hot tenants cannot starve others; audits compare allocated vs actual share.  
- **Bursty fairness.** Bursts are allowed within `burst` but charged against future fairness windows.  
- **No “pay-to-bypass.”** Higher tiers adjust **published** quotas, not enforcement rules.

---

## 9. Acceptance Checklist (Definition of Done)

- [ ] Invariants **[I-M1..I-M9]** encoded as tests/CI checks (see TESTS.md).  
- [ ] Roles & boundaries implemented (caps scoped per role, no ambient root).  
- [ ] Proposal lifecycle & quorum enforced; events on `governance.mailbox`.  
- [ ] Metrics & audit logs exported; daily anchor job green.  
- [ ] SLA thresholds monitored with alerts; brownout/freeze presets tested quarterly.  
- [ ] Appeal path validated in chaos/desk drills (Gate **M** evidence).  
- [ ] Key rotation dual-KID flow demonstrated in staging.

---

## 10. Black-Swan Playbook (Gate L)

- **Bus partition ≥ 5 min:** enter brownout; prefer enqueue admission correctness over latency; resume with backfill; publish incident.  
- **DLQ surge > 2% traffic:** auto-quarantine new poison patterns; enable sandboxed replay; cap DLQ growth; convene review board.  
- **Moderation false-positive wave:** freeze rule-set deltas; enable tenant opt-out flag for affected topics; escalate appeal handling.  
- **KMS outage:** accept only previously cached KIDs; shrink cap TTLs; rotate when KMS recovers; publish post-mortem.

---

## 11. Change Management & Windows

- **Normal window:** UTC Tue–Thu 14:00–20:00.  
- **Freeze window:** No production policy changes on Fri–Mon or before major events (tenant-declared).  
- **Rollback window:** 60 min after publish; automatic revert if golden metrics fail SLOs.

---

## 12. References

- RUNBOOK.md (ops procedures, scaling & chaos)  
- TESTS.md (governance tests, fuzz/property hooks)  
- CONFIG.md (declared parameters)  
- OBSERVABILITY.md (metrics & logs schema)  
- SECURITY.md (capabilities, KMS custody, amnesia)  
- Economic Integrity & Hardening Blueprints  
- Perfection Gates A–O (focus: I, K, L, M)

---

## 13. History (summarize major decisions)

| Date (UTC) | Change | By | Quorum | Notes |
|------------|--------|----|--------|-------|
| 2025-10-12 | Initial governance baseline | svc-mailbox owners | 3/4 | Establish invariants, roles, appeals |

```
