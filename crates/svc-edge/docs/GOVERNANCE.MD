# 🏛 GOVERNANCE.md — svc-edge

---

title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-17
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ
-----------------------

## 0. Purpose

This document defines the **rules of engagement** and **operational governance** for `svc-edge`, a stateless edge asset server that serves static and ranged bytes, optionally **live-filling** from allow-listed HTTPS origins. It ensures:

* Transparent, auditable decisions over **who/what** may be served.
* Enforcement of **safety and economic invariants** (bounded resource use, fair throttling, no unauthorized origin egress).
* Clear **authority boundaries**, change controls, and **appeal paths** (for takedowns, allow-list changes, and service-level parameter shifts).
* SLA-backed commitments for **latency**, **availability**, and **observability**.

It ties into:

* **Economic Integrity Blueprint** (no doubles; bounded issuance → here: bounded resource consumption & rate governance).
* **Hardening Blueprint** (bounded authority, least privilege, key custody).
* **Perfection Gates A–O** (notably Gate I: bounded economic invariants; Gate K: continuous vigilance; Gate L: black-swan prep; Gate M: appeal paths).

---

## 1. Invariants (MUST)

Non-negotiable rules for `svc-edge`:

* **[I-E1] Authorization/Origin:** No **live fill** from an origin outside the configured **allow-list**. No wildcard bypass.
* **[I-E2] Resource Bounds:** Requests are served under **strict backpressure**:

  * `max_inflight`, **bounded queues**, and **RPS caps** enforced; excess is rejected (429) rather than degrading into meltdown.
* **[I-E3] Protocol Correctness:**

  * Strong **ETag** semantics (deterministic for identical bytes).
  * **Range/Conditional** handling is correct and deterministic (206/304/416).
* **[I-E4] Amnesia Mode:** When enabled, **no operation may persist state**; any write-like path must be rejected.
* **[I-E5] Secrets & Custody:** No plaintext secrets in logs; TLS keys, macaroon roots, and admin caps are custody-bound (KMS/HSM).
* **[I-E6] Observability:** Governance actions (policy changes, freezes, allow-list edits) generate **append-only audit events** with actor, reason, diff, and corr_id.
* **[I-E7] SLA Disclosure:** SLAs and limits are public, testable, and cannot be silently downgraded.

---

## 2. Roles & Authority

### Roles

* **Policy Owner (ron-policy):** Authors policy/guardrails (allow-list rules, CORS posture, RPS tiers); cannot push code or flip runtime flags directly.
* **Edge Operator (svc-edge ops):** Runs deployments, applies approved config, handles incident response and freeze/unfreeze operations.
* **Security Officer (sec-ops):** Owns secrets/capability roots, reviews high-risk changes (origin allow-list, amnesia off), approves emergency freezes.
* **Auditor (read-only):** Verifies compliance, reviews audit logs/metrics, attests to SLA conformance.
* **SRE (platform):** Capacity planning, autoscaling policy, performance SLO stewardship.

### Authority Boundaries

* **Separation of Duties:**

  * Policy **proposes**; Ops **applies**; Security **approves** high-risk items; Auditor **verifies**.
* **Capability Tokens:** All admin/governance surfaces require **macaroon-style caps** with least privilege and expiry.
* **No God Mode:** No single actor can both propose **and** apply risky changes (e.g., adding a broad wildcard origin). Multi-party approval required.

---

## 3. Rules & SLAs

### Service SLAs (public)

* **Availability:** ≥ 99.95% monthly for `/healthz`, `/readyz`, and asset GETs (excluding scheduled maintenance).
* **Latency (intra-AZ, warm-hit):** p95 < **40 ms**, p99 < **80 ms**.
* **Error Budget:** 5xx rate < **0.1%** per rolling 10m; shedding (429) < **1%** per rolling 15m.
* **Metrics & Logs:** `/metrics` scrape and structured logs available in near-real-time; **audit events** published ≤ **1 s** after commit.

### Governance SLAs (internal)

* **Policy Change Application:** approved low-risk changes (e.g., RPS tier) applied ≤ **1 h**; high-risk (allow-list) ≤ **24 h** after approvals.
* **Emergency Freeze:** execute within **5 min** of Security approval; disclose via audit topic immediately.
* **Appeal Review:** initial response within **24 h**; resolution target **5 business days**.

---

## 4. Governance Process

### Proposal Lifecycle

1. **Draft** (Policy) → change doc with rationale, risk, blast radius, backout.
2. **Review** (SRE + Security) → test plan, thresholds, rollback, observability checks.
3. **Approve** (quorum: **2-of-3** among Policy Owner, Security Officer, SRE lead).
4. **Execute** (Edge Operator) → deploy/apply, verify with runbook checks; attach metrics snapshots.
5. **Audit** (Auditor) → verify diffs, metrics, SLA impact; sign off.

**Default:** proposals **expire** if quorum not reached in **72 h**.

### Emergency Powers

* **Freeze live-fill** and/or **force offline mode** when abuse or upstream compromise is suspected.
* Requires Security Officer approval + immediate audit disclosure; **auto-review within 24 h**.
* Unfreeze follows standard proposal path, with post-mortem attached.

### Parameter Changes

* **High-risk** (allow-list, amnesia off, CORS posture): full lifecycle + Security sign-off.
* **Medium** (RPS tiers, inflight caps): Policy + SRE approval.
* **Low** (logging verbosity, cosmetic headers): SRE approval; batchable.

---

## 5. Audit & Observability

* **Audit Log (append-only):** for every governance action, record:

  * `ts, actor, role, action, subject, old→new diff, reason, corr_id, approvals`.
* **Metrics (exposed in Prometheus):**

  * `governance_proposals_total{status="draft|approved|rejected|expired"}`
  * `governance_actions_total{action="freeze|unfreeze|allowlist_add|allowlist_remove|param_change"}`
  * `ledger_disputes_total` *(if billing/settlement metering integrated)*
  * `policy_violations_total{type="amnesia|origin|cors"}`
* **Verifiability:**

  * **Proofs of conservation** (if request metering is tied to billing): request counts = served + rejected; **no negative drift**.
  * Range/ETag conformance sampled continuously; failures trigger **policy_violations_total**.
* **Red-Team Drills:** Quarterly “rogue admin” simulation:

  * Attempt to add wildcard origin; ensure multi-sig blocks or queues it; alerts fire; no silent change.

---

## 6. Config & Custody

* **Config Declaration** (in `configs/svc-edge.toml` or equivalent):

  * **Policy params:** allow-list, CORS, amnesia, RPS tiers, `max_inflight`, timeouts.
  * **SLA targets** (for dashboard gating).
  * **Audit sinks** (bus topic, storage retention).

* **Custody & Secrets:**

  * Admin caps and cryptographic material kept in **ron-kms** or HSM; never in env files.
  * Use **short-lived** macaroon roots; delegate minimal-scope caveats per operation.
  * **Rotation:** every **90 days**, on role change, or after suspected compromise.
  * **Access Logging:** every secret access emits an audit event with actor and purpose.

---

## 7. Appeal Path (Takedowns / Policy Disputes)

1. **File Dispute**: open a governance ticket referencing asset(s), policy clause, and evidence.
2. **Freeze (optional)**: Security may freeze specific paths/origins during review.
3. **Deliberation**: Policy + Security + SRE review evidence; Auditor observes.
4. **Outcome**:

   * **Sustain**: deny change; record rationale.
   * **Modify**: adjust allow-list/CORS/tiers; apply via lifecycle above.
   * **Remove**: takedown or block origin; publish audit event and notify stakeholders.
5. **No Silent Rollbacks:** all state changes leave **disputed=true** markers in audit trail; no history rewrite.

**Escalation:** If unresolved in **5 business days**, escalate to executive governance board (3-of-5 quorum) for binding determination.

---

## 8. Economic Guardrails (Edge Context)

While `svc-edge` is not a minting or settlement crate, it **consumes infrastructure resources** that map to economics (cost of egress, CPU, I/O):

* **Fair Use Tiers:** RPS and bandwidth tiers mapped to tenants/projects; default anonymous tier conservative.
* **Abuse Containment:** per-tenant caps and **burst buckets**; overages rejected with 429/`Retry-After`.
* **Cost Attribution:** optional **meter exports** for downstream billing (ron-accounting/ron-ledger).
* **No Free-Rider Escalation:** operators may **throttle** or **block** tenants exceeding tiers; all actions audited.

---

## 9. Compliance & Legal

* **Content Controls:** operator can block hashes/paths per lawful request; appeal path applies.
* **Jurisdictional Rules:** CORS/allow-list may encode regional constraints; changes require Security sign-off.
* **Data Minimization:** Logs exclude PII by default; IPs truncated or salted unless legally required.

---

## 10. Acceptance Checklist (Definition of Done)

* [ ] **Invariants** ([I-E1]…[I-E7]) enforced in code + tests (see TESTS.md).
* [ ] **Roles & boundaries** documented in ops handbooks; macaroon caveats scoped and expiring.
* [ ] **Proposal lifecycle** tooling in place (templates, approver groups, expirations).
* [ ] **Audit events** emitted for all governance actions; dashboards wired (counters, rates).
* [ ] **SLAs** published; probes/alerts attached (latency, error budget, shedding).
* [ ] **Emergency freeze** runbook tested (quarterly drill).
* [ ] **Secrets custody & rotation** policy active; last rotation < 90 days.
* [ ] **Appeal path** validated via tabletop exercise; escalation contacts current.
* [ ] **Red-team drill** completed this quarter; findings tracked to closure.

---

## 11. Change Risk Classification

| Change Type   | Examples                                                    | Risk | Approvals                      | Rollback                         |
| ------------- | ----------------------------------------------------------- | ---: | ------------------------------ | -------------------------------- |
| **A: High**   | Add/remove allow-list origin; disable amnesia; broaden CORS | High | 2-of-3 (Policy, Security, SRE) | Immediate freeze + revert config |
| **B: Medium** | Adjust RPS tiers, `max_inflight`, timeouts                  |  Med | Policy + SRE                   | Config revert, no data migration |
| **C: Low**    | Log verbosity, cosmetics, headers                           |  Low | SRE                            | Reapply previous config          |

---

## 12. Incident Classes & First Actions

* **P1 (Abuse/Leakage):** suspected data exfil via origin misuse → **Freeze live-fill**, rotate admin caps, audit blast to channel.
* **P1 (Widespread 5xx):** error budget breach → rollback last change, raise replicas, verify storage/upstream health.
* **P2 (SLA Drift):** p95 approaching 40 ms → scale out; review tenant tiers; tune backpressure.
* **P3 (Audit Gap):** missing governance events → pause risky changes until sink restored; backfill from ops journal.

---

## 13. References

* **RUNBOOK.md** — ops procedures, scaling, alerts.
* **SECURITY.md** — STRIDE, hardening, macaroon/custody details.
* **CONFIG.md** — all knobs & defaults (allow-list, caps, amnesia, timeouts).
* **OBSERVABILITY.md** — metrics, logs, tracing, audit topics.
* **TESTS.md** — governance-relevant tests (amnesia, allow-list, policy violations).
* **Economic Integrity Blueprint**, **Hardening Blueprint**, **Perfection Gates** (I, K, L, M).

---

## 14. History (fill as events occur)

* *2025-10-17:* Initial governance charter drafted; roles & SLAs defined.
* *YYYY-MM-DD:* <event/decision> — <summary>, approvers, audit link.

---

### Appendix A — Governance Metrics (PromQL Hints)

* Proposals by status:
  `sum by (status) (rate(governance_proposals_total[5m]))`
* Actions by type:
  `sum by (action) (rate(governance_actions_total[5m]))`
* Policy violations:
  `sum by (type) (rate(policy_violations_total[5m]))`
* Error budget guardrail:
  `sum(rate(http_requests_total{status=~"5.."}[10m])) / sum(rate(http_requests_total[10m]))`

---

✅ **Why this is God-tier:** It codifies **non-negotiable invariants**, separates **powers and duties**, anchors **SLAs** to observable metrics, mandates **auditability**, provides **emergency levers** with oversight, and defines **appeals** and **risk-based approvals**—all tied to your runbook, tests, and security hardening so governance is **provable** and **operational**, not just prose.
