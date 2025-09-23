

# 🏛 GOVERNANCE.md

```markdown
---
title: Governance & Operational Integrity — macronode
status: draft
msrv: 1.80.0
last-updated: 2025-09-23
audience: contributors, ops, auditors, stakeholders
crate-type: host
scope: operational-governance (composition, rollout, custody, audit)
non-scope: economic-policy (delegated to ron-ledger / svc-rewarder / accounting)
---

# GOVERNANCE.md — macronode (Host-Level)

## 0. Purpose

`macronode` is a **composing host** that wires and supervises canonical services
(gateway, omnigate, index, storage, mailbox, overlay, DHT, etc.) **by configuration**.
This document defines the **rules of engagement for operational governance**:
who may change what, how changes are audited, and which invariants must hold for safe operation.

**Explicit non-scope:** Economic/policy logic (issuance, rewards, conservation) is
**delegated** to the econ crates (`ron-ledger`, `ron-accounting`, `svc-rewarder`) and their
own GOVERNANCE files.

It ties into:
- **Concurrency & Aliasing Blueprint** (drain deadlines, bounded channels).
- **Hardening Blueprint** (key custody, TLS, amnesia).
- **Economic Integrity Blueprint** (referenced, but enforced in econ crates).
- **Perfection Gates** (esp. Gate A: invariants; Gate G: observability; Gate M: appeal).

---

## 1. Invariants (MUST)

Non-negotiable **operational** rules for macronode:

- [O-G1] **No economic state** is created or mutated here. macronode must not mint, settle, or bypass econ governance.
- [O-G2] **Config changes are auditable & reversible**: every change has a proposer, approver(s), diff, and rollback plan.
- [O-G3] **Authority is bounded**: operators hold least-privilege macaroons; no unilateral “god mode”.
- [O-G4] **Graceful control**: enable/disable services, scale factors, and deadlines must respect shutdown **drain ≤ 5s p99**.
- [O-G5] **Fail-closed**: invalid configs or missing custody material (keys/certs) **refuse to boot**.
- [O-G6] **Observability non-optional**: `/readyz`, `/healthz`, and metrics must be green before declaring Ready.
- [O-G7] **Amnesia honored**: when `amnesia=ON`, no persistent secrets/caches are written; all sensitive buffers zeroize on shutdown.
- [O-G8] **No lock across .await in control paths** (supervision, hot paths) — concurrency doc enforced by CI.

> Economic invariants like “no double spend/issuance” are **[D] Delegated** to the econ crates.

---

## 2. Roles & Authority

### Roles
- **Ops Admin** — may approve production config changes and service (de)activation.
- **Release Manager** — controls version/rollout plans (canary, staged, rollback).
- **Security Officer** — custodian of macaroons, TLS certs, rotation policy.
- **SRE On-call** — may trigger emergency freeze/rollback within bounded playbook.
- **Auditor** — read-only access to configs, logs, metrics, and change history.

### Authority Boundaries
- Ops/Release may **enable/disable** services and adjust **non-disruptive** limits at runtime.
- Changing **disruptive** params (bind addresses, TLS, UDS, service set) requires a **maintenance window** and approval from Ops + Release.
- **No one** in macronode may alter ledger/reward rules; those require econ-crate governance.
- All privileged actions require **capability tokens** (macaroons) with explicit scopes and expiry.

---

## 3. Rules & SLAs (Operational)

- **Readiness SLA**: a config/apply cycle must return the node to `Ready` (via `/readyz`) within **30s** p95.
- **Shutdown SLA**: cooperative drain must complete within **3s p95 / 5s p99**; otherwise abort stragglers and log.
- **Change Windows**: disruptive changes limited to approved windows; canary first, then staged rollout.
- **Rollback**: every change includes a one-step rollback command (documented alongside the PR).
- **Audit Latency**: config change must be visible in audit log within **1s** after apply.

---

## 4. Governance Process (Operational Changes)

- **Proposal lifecycle**: Draft → Review (2 reviewers: Ops + Release) → Approve → Execute → Verify.
- **Quorum**: 2-of-N maintainers for production changes; 1-of-N for staging.
- **Defaults**: Changes auto-expire if not approved within **72h**.
- **Emergency powers**:
  - **Freeze** a service (disable intake) with SRE + Ops approval; record reason and incident ID.
  - Must be disclosed in audit log immediately and in the weekly ops report.

**What counts as disruptive?**
- Changing `bind_addr`, `tls.*`, `uds.*`, or the set of `services.enable`.
- Increasing hard caps (e.g., `total_http_concurrency`) beyond policy’d maxima.

---

## 5. Audit & Observability

- **Audit logs**: append-only, signed (host time, actor, scopes, diff summary, outcome).
- **Metrics** (must exist and be green before Ready):
  - `readyz_state{service}` (0/1/2), `service_restarts_total{service}`
  - `busy_rejections_total{endpoint}`, `queue_dropped_total{queue}`
  - `config_apply_total{status}`, `config_rollback_total{status}`
- **Traces**: change-apply spans include corr_id linking PR → deploy → runtime swap.
- **Red-team drills**: quarterly exercise to simulate a rogue operator token; confirm bounded authority & alerting.

---

## 6. Config & Custody

- **Config**: stored as code; changes via PR only; secrets are referenced from secret stores; never inline in git.
- **Custody**:
  - Macaroons: short-lived; rotated ≤ 30 days; scopes limited to host-level ops.
  - TLS: stored in KMS/HSM or root-owned files (`0600`); rotated via runbook.
  - **Amnesia**: when set, disable persistence of secrets/caches; zeroize buffers on shutdown.
- **Validation**: boot/reload **fails closed** on missing/invalid custody materials.

---

## 7. Appeal Path

- **Operational disputes**:
  1) Raise in ops channel with corr_id; 2) escalate to maintainers quorum; 3) final tie-break by Security Officer.
- **Economic disputes**: **out of scope** here — escalate to the relevant econ crate governance (ledger/rewarder/accounting).
- **No silent rollbacks**: all reversions logged with cause and impact analysis.

---

## 8. Acceptance Checklist (Definition of Done)

- [ ] Invariants in §1 enforced by code/CI (lint gates, fail-closed validation).
- [ ] Roles and scopes mapped to macaroon capabilities; least privilege verified.
- [ ] Governance process (proposal → approval → execute → verify) documented in the repo’s CONTRIBUTING.
- [ ] Metrics and audit logs exported; dashboards linked.
- [ ] Shutdown and readiness SLAs covered by tests/chaos drills.
- [ ] Emergency freeze/rollback playbooks tested in staging.

---

## 9. Appendix

**Delegations (Econ):**
- `ron-ledger` — settlement, conservation proofs, double-spend prevention.  
- `ron-accounting` — balances, accrual, reporting integrity.  
- `svc-rewarder` — emission schedules, distribution caps.

**Blueprints & Policies:**
- Concurrency & Aliasing, Hardening, Economic Integrity, Perfection Gates.

**History:**  
- Record major freezes, emergency actions, and post-mortems (corr_ids, dates, outcomes).

```

---

### Why this split matters

* It **keeps macronode honest**: no economic authority, no policy drift.
* It gives ops **teeth**: auditable change control, bounded emergency powers, and clear SLAs.
* It aligns with your **Concurrency** and **Hardening** docs: drain deadlines, fail-closed validation, custody rotation, amnesia.
