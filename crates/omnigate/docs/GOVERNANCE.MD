

---

# 🏛 GOVERNANCE.md — omnigate

---

title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-07
audience: contributors, ops, auditors, stakeholders
crate-type: service (policy-boundary, non-econ)
-----------------------------------------------

## 0. Purpose

This document defines **rules of engagement** for **omnigate**—the BFF hydration service that orchestrates reads across `svc-index`, `svc-storage`, and `svc-mailbox`.
Omnigate is **not** an economic actor: it MUST NOT mint, settle, or mutate ledger state. Governance here focuses on:

* Transparent, auditable **operational decisions** (quotas, feature toggles, deploys).
* Enforced **authority boundaries** (who can change what; with what proofs).
* **Economic integrity fences**: zero authority to issue value; no hidden cross-subsidies or priority bypass.
* SLA-backed commitments for read-path behavior and **appeal/rollback** paths when changes regress SLOs.

**Ties into:** Economic Integrity Blueprint (no doubles, bounded issuance), Hardening Blueprint (bounded authority, custody), Six Concerns (SEC/RES/PERF/GOV/DX), and **Perfection Gates** (esp. Gate I: bounded econ invariants; Gate K: continuous vigilance; Gate M: appeal paths; Gate L: black-swan readiness).

---

## 1. Invariants (MUST)

**Service & policy invariants enforced by omnigate:**

* [I-OG1] **No economic authority.** Omnigate SHALL NOT mint, settle, or write to `ron-ledger` or any balance state.
* [I-OG2] **Statelessness (mutations).** No persistent state beyond caches/counters and ephemeral telemetry (Micronode amnesia = RAM-only).
* [I-OG3] **Capability-gated actions only.** All privileged actions (admin endpoints, config reload, feature toggles) require valid **macaroons v1** with appropriate caveats.
* [I-OG4] **Fairness and quotas.** Ingress quotas and DRR fairness are **enforced and observable**; no bypass paths.
* [I-OG5] **Auditability.** Every governance-relevant action (deploy, config change, quota change, feature enable, waiver) is **logged and attestable**.
* [I-OG6] **No unbounded admin override.** Emergency overrides must be multi-party (N-of-M) and time-boxed; all actions are auto-reverted or re-ratified.
* [I-OG7] **Shed-before-collapse.** Under duress, omnigate degrades by shedding requests (429/503) rather than violating SLOs or starving in-flight streams.
* [I-OG8] **No scope creep.** DHT/discovery, payments, rewards, and moderation policy are **hosted downstream**; omnigate only orchestrates reads and composes responses.

---

## 2. Roles & Authority

### 2.1 Roles

* **Policy Owner (ron-policy):** Curates global policies (caps, quotas). Cannot deploy/operate omnigate directly.
* **Service Owner (Omnigate):** Runs service, manages SLOs, quotas within policy bounds, maintains runbook.
* **Ledger Keeper (ron-ledger):** Executes settlement and conservation proofs (non-overlapping with omnigate).
* **Rewarder (svc-rewarder):** Distributes incentives within ledger constraints (not callable by omnigate).
* **Auditor:** Read-only verification of logs, metrics, and config footprints; can request freeze via governance process.
* **On-Call SRE:** May execute scoped runbook actions; cannot exceed policy bounds without multi-sig.

### 2.2 Authority Boundaries

* **Omnigate** may: adjust **operational** quotas (within policy caps), roll forward/back **deploys**, toggle **feature flags** marked “ops-safe,” and reload config.
* **Omnigate** may not: write to ledger, alter reward schedules, bypass fairness, or accept unsigned governance actions.
* **Ledger/Rewarder** operate independently; omnigate can **read** econ state for hydration but never mutate it.
* **All privileged API** calls must present macaroons with **capability caveats**: `svc=omnigate`, `scope=ops|read`, `ttl≤T`, `aud=cluster`.

---

## 3. Rules & SLAs

* **Read-path SLOs (reference):** p95 GET intra-region < **80 ms**; range-start < **100 ms**; hydration p95 ≤ **150 ms** (see `PERFORMANCE.md`).
* **Fairness/Quotas:** Rate limits and inflight caps are **enforced per tenant**; any change above policy caps requires **proposal** and **approval**.
* **Audit log latency:** governance events (deploys, quota changes, feature flips) become visible in audit stream within **≤ 1 s** of commit.
* **Change windows:** risky changes (e.g., toggling hedged reads) require canary + rollback plan and **must not** occur inside declared freeze windows.
* **Emergency freeze:** only by multi-sig; time-boxed; public audit entry within **24 h**.

---

## 4. Governance Process

### 4.1 Proposal Lifecycle

1. **Draft** (ADR/RFC): rationale, blast radius, rollback, alert changes, dashboards.
2. **Review** (policy + service owner + SRE): check invariants [I-OG*], perf gates, chaos impact.
3. **Approve** (N-of-M multi-sig where required): sign capability token granting **temporary** authority.
4. **Execute** (canary → fleet): record corr_id, upload artifact digests, attach Prometheus queries.
5. **Verify**: SLOs hold; alerts stable; no regression; publish audit entry.
6. **Roll back** if verification fails; file post-incident retro.

### 4.2 Emergency Powers

* **Freeze** ingress (tighten quotas) or **disable** a pilot feature (e.g., hedged reads).
* Requires **majority multi-sig**; **expires** unless re-ratified in ≤ 24h; audit entry mandatory.

### 4.3 Parameter Changes

* **Quotas, inflight caps, hedging budgets, chunk sizes**: ops-safe within policy guardrails.
* **Policy caps, fairness weights, Econ/ZK pilots**: **proposal + approval** required.

---

## 5. Audit & Observability

### 5.1 Audit Events (append-only, signed)

Emitted on the **governance bus** and persisted by `ron-audit`:

```json
{
  "ts": "2025-10-07T21:03:15.412Z",
  "service": "omnigate",
  "event": "quota.update",
  "actor": "cap:msig:3of5",
  "prev": {"tenant":"acme","rps":500,"inflight":512},
  "next": {"tenant":"acme","rps":650,"inflight":640},
  "corr_id": "b7b2e2f0",
  "sig": "ed25519:…",
  "justification": "Black Friday surge; canary ok; rollback T+60m"
}
```

### 5.2 Metrics (examples)

* `governance_proposals_total{status}`
* `config_reloads_total{result}`
* `quota_updates_total{tenant}`
* `rejected_total{reason="quota|policy|auth"}`
* `pilot_feature_toggles_total{feature,action}`

### 5.3 Verifiability

* **Proof of bounded authority**: every privileged change correlates with a signed capability (macaroon) and multi-sig metadata where applicable.
* **Red-team drills**: simulate rogue admin; verify overrides are blocked or fully audited.

---

## 6. Config & Custody

* **Config MUST declare**: quotas (default & max), allowed feature flags, SLO thresholds, alert routes, and **custody model**.
* **Key custody**: all private keys in **ron-kms** or HSM; **no plaintext** keys in files/env.
* **Rotation**: ops/admin caps & signing keys rotate **≤ 90 days** or immediately on compromise.
* **Native-TLS escape** (if ever needed): only by ADR with explicit risk assessment; default is **rustls**.

---

## 7. Appeal Path

1. **Dispute raise** on governance bus with corr_id and evidence (dashboards, logs).
2. **Temporary mitigation** (canary rollback, quota restore) by On-Call within policy bounds.
3. **Multi-sig override** proposal if policy change is required (time-boxed).
4. **Auditor review**: cross-checks metrics & logs; attaches verdict to audit stream.
5. **Public disclosure** (internal/external as applicable) within **24h** for emergency actions.

> No **silent rollbacks**: all reversions produce audit events. No mutation of past audit events; corrections are **append-only**.

---

## 8. Non-Scope (Explicit)

* Omnigate SHALL NOT:

  * Mint, settle, or mutate balances in `ron-ledger`.
  * Trigger reward issuance or modify reward schedules in `svc-rewarder`.
  * Implement moderation policy decisions (scoring/weighting live in `svc-mod` or policy hosts).
  * Bypass gateway fairness (DRR) or tenant quotas.
  * Store long-lived PII or secrets beyond minimal config necessary for operation.

---

## 9. Acceptance Checklist (DoD)

* [ ] Invariants [I-OG1..I-OG8] are enforced in code (caps, routes, guards).
* [ ] Roles and boundaries documented and reflected in capabilities (macaroons).
* [ ] Proposal lifecycle implemented (ADR template, signer verification, canary playbook).
* [ ] Audit logs emitted, signed, and visible ≤ 1s after action.
* [ ] Governance metrics exported and alerted.
* [ ] Appeal path validated in quarterly chaos drill (rogue-admin simulation).
* [ ] Key custody & rotation policy active; last rotation < 90 days.
* [ ] Non-scope protections (no econ writes, no bypass) covered by tests.

---

## 10. Governance & Deploy Playbooks (Quick Reference)

* **Quota raise within policy cap**: create change ticket → signed cap → canary 30m → fleet rollout → audit entry → verify dashboards.
* **Feature pilot enable (e.g., hedged reads)**: ADR + risk table → enable on 5% → monitor SLOs/alerts → widen or roll back; all toggles audited.
* **Emergency freeze**: multi-sig cap (3/5) → reduce RPS/inflight → announce on bus → re-ratify or auto-expire in 24h.

---

## 11. Appendix

**Blueprints:** Economic Integrity (no doubles, bounded emission), Hardening (bounded authority, custody), Concurrency & Aliasing (backpressure/zero-copy), Scaling & Performance, Security.
**References:** Macaroons v1 capability tokens; Governance schema (`/docs/spec/governance.md`); optional TLA+ sketches (planner fairness).
**History:** Maintain a chronological list of governance incidents (disputes, freezes, emergency overrides) with links to ADRs, dashboards, and audit entries.

---
