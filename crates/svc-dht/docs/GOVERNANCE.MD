**File:** `crates/svc-dht/GOVERNANCE.md`

````markdown
# 🏛 GOVERNANCE.md — svc-dht

---
title: Governance & Operational Integrity (svc-dht)
status: draft
msrv: 1.80.0
last-updated: 2025-10-11
audience: contributors, ops, auditors, stakeholders
crate-type: service|ops
owner: Stevan White
review-cadence: quarterly
---

## 0. Purpose

This document defines the **rules of engagement** for `svc-dht`—the Kademlia/Discv5 discovery service of RustyOnions.

It ensures:
- Transparent, auditable operational decision-making.
- Enforcement of **integrity invariants** (no duplicates, bounded TTL/authority).
- Clear **authority boundaries** and **appeal paths** for allow/deny lists, bootstrap sets, rate/geo policy, and PQ enforcement.
- SLA-backed commitments to external consumers (overlay, registry, gateway).

It ties into:
- **Economic Integrity Blueprint** (bounded authority/quotas; no unbounded issuance of “providers”).
- **Hardening Blueprint** (capabilities, custody, amnesia).
- **Perfection Gates A–O** (esp. **I** invariants, **K** vigilance, **M** appeals, **F** perf gates).

> MSRV policy: repo-wide MSRV is `1.80.0`. Review and bump **quarterly** only after the workspace CI matrix is green.

---

## 1. Invariants (MUST)

- **[I-D1] Idempotent PROVIDE:** No duplicate provider records for the same `(key, peer_id)`; re-PROVIDE extends TTL only.
- **[I-D2] Bounded TTL:** Provider TTL is finite and **≤ 48h** unless a governance-approved override is active.
- **[I-D3] Bounded authority:** No ambient admin. All write actions require **capability tokens** (macaroons v1) with explicit caveats (scope, verbs, expiry).
- **[I-D4] Policy-before-perf:** Rejections (429/403/413) fire before overload; backpressure beats collapse.
- **[I-D5] Signed, append-only governance:** All governance actions emit a **tamper-evident** audit event (hash-chained).
- **[I-D6] Safe defaults:** Micronode runs with **amnesia ON**; write paths fail closed in degraded mode.
- **[I-D7] PQ monotonicity:** When `REQUIRE_PQ_ON=true`, turning it **off** requires Security quorum **and a 24h delay window** with public audit; post-toggle, **ML-DSA-87** is mandatory for control ops while ON.
- **[I-D8] Ready means routable:** `/readyz=200` only after ≥1 **live** bootstrap is reachable and the routing table is warmed.
- **[I-D9] No hidden state:** Any config affecting network behavior (α/β, ceilings, bootstraps, PQ) is versioned and observable via metrics.

**Enforcement:** Verified by `TESTS.md` (unit/property/integration), perf/soak CI gates, and governance-lint in CI.

---

## 2. Roles & Authority

### Roles
- **Service Owner (Owner):** SLOs, releases, arbitration. _(Current: **Stevan White**.)_
- **Ops (Pillar 9):** rollouts, incidents, chaos drills, config deployment.
- **Security (Pillar 10):** policy profiles, PQ enforcement, allow/deny edits, red-team drills.
- **Auditor (Read-only):** queries logs/metrics/audit; verifies proofs.
- **Contributor:** code/config changes via PR; no production authority.

### Authority Boundaries
- **Ops** can deploy, canary, and set **degraded mode**; cannot change allow/deny, bootstraps, or PQ without Security quorum.
- **Security** approves policy profiles, PQ toggles, and bootstrap rotations via **quorum**.
- **Owner** approves SemVer-impacting changes and resolves contention.
- **Auditor** reads only; may file disputes (see §7).
- **All write actions** use macaroons with scoped caveats:
  - Example: `service=svc-dht`, `action=pq.toggle`, `expires<=2025-10-12T00:00Z`, `ip=10.0.0.0/8`.

---

## 3. Rules & SLAs

- **Service SLOs (steady state, intra-AZ):**
  - Lookup latency **p95 < 500ms**, hop **p99 ≤ 5**.
  - Restart-to-ready **p99 ≤ 10s**.
  - Monthly availability **≥ 99.9%** for `/readyz` and lookup RPCs.

- **Governance propagation SLO:**
  - Approved changes **visible in metrics/config digest within ≤ 60s**.
  - Alert if `governance_propagate_ms > 60000` for **2m**.

- **Provider lifecycle:**
  - Default TTL **24h**; max TTL **48h** without explicit override.
  - Per-peer ceilings enforced; exceeders receive 429 with `Retry-After`.

- **Overrides & appeals:**
  - Temporary allow/deny exceptions require Security quorum; **auto-expire ≤ 7 days**.
  - Appeals in §7; **no silent rollbacks**.

---

## 4. Governance Process

### Quorum & Defaults
- **Security quorum (standard): `2-of-3`** signers.
- **Emergency majority:** `ceil(M/2)+1` of registered Security signers.
- **Auto-reject:** proposals lapse if quorum not met in **24h** (business-hours clock).

### Proposal Lifecycle
1. **Draft**: PR + governance issue (change spec, risk, rollback plan, metrics to watch).
2. **Review**: Owner + Security; CI/lints green; threat note added.
3. **Approve**: Security quorum signs off (macaroon-scoped).
4. **Execute**: Ops canary **10%/10m** → promote on green.
5. **Record**: Signed audit event with proposal ID, signers, config digest, and resulting metrics snapshot.

**Mermaid — Lifecycle**
```mermaid
flowchart TD
A[Draft PR + Governance Issue] --> B[Security/Owner Review + CI Green]
B --> C{Quorum 2-of-3?}
C -- No --> X[Auto-Reject @24h]
C -- Yes --> D[Execute Canary 10%/10m]
D --> E{Green Metrics?}
E -- No --> R[Rollback to LKG]
E -- Yes --> F[Promote to 100%]
F --> G[Record Signed Audit Event]
````

### Emergency Powers

* Freeze PROVIDE (writes) with emergency majority; reads continue when safe.
* Replace/disable malicious bootstrap endpoints with emergency majority.
* Must publish **audit event ≤ 24h** and a **post-mortem ≤ 72h**.

### Parameter Changes (require lifecycle)

* α/β, inflight ceilings, timeouts.
* Provider TTL/caps.
* Allow/deny policies; geo/ASN rules.
* Bootstrap set (add/rotate/remove).
* PQ enforcement (`REQUIRE_PQ_ON`).

### Example CLI (normative)

```bash
# Propose a policy profile change
ronctl governance propose --service svc-dht --change policy.profile=strict --reason "Eclipse risk; tighten for 48h"

# View and verify config digest
ronctl config hash --file /etc/ron/svc-dht.toml

# Approve (Security signer; macaroon scoped to action and expiry)
ronctl governance approve --proposal-id 42 --cap macaroon.txt
```

---

## 5. Audit & Observability

* **Audit log:** append-only, hash-chained; each entry includes
  `ts, actor, capability_id, action, before_digest, after_digest, reason, proposal_id, signers[]`.
* **Retention:** audit logs stored **≥ 90 days** in tamper-evident storage.
* **Metrics (required):**

  * `governance_actions_total{action,status}`
  * `governance_propagate_ms` (histogram)
  * `policy_profile_active{profile}` (gauge)
  * `pq_enforced` (0/1), `pq_downgrade_attempt_total`
  * `bootstrap_set_hash{hash="..."}`
  * `rejected_total{reason}` (403/413/429/503 breakdown)
* **Proofs/validations:**

  * Provider conservation: unique `(key,peer_id)` count matches active entries.
  * TTL range checks: all entries `<= max_ttl`.
* **Red-team drills (quarterly):**

  * “Rogue admin” simulation; **fail drill** if any capability scope is bypassed or quorum is circumvented.

---

## 6. Config & Custody

* **Config MUST declare:**

  * α/β/ceilings/timeouts; provider TTL/caps; active policy profile.
  * Bootstrap seeds by **fingerprint + endpoint**.
  * PQ enforcement flag and the 24h downgrade delay window.
  * SLO targets (echoed for dependents).

* **Custody:**

  * Governance/macaroons signing keys in **ron-kms** or HSM.
  * No raw private keys in env/files.
  * Onion/transport keys:

    * **Micronode:** ephemeral (amnesia); rotate on restart.
    * **Macronode:** KMS-backed; rotation every **90 days** with **≤ 7 days** overlap.

---

## 7. Appeal Path

1. **File dispute** on governance bus topic with `{proposal_id|audit_id}`, evidence (metrics/logs/pcaps), and requested remedy.
2. **Security review** drafts an override (limited scope/time).
3. **Quorum vote** (2-of-3). If approved, Ops executes; overrides **auto-expire ≤ 7 days**.
4. **Record**: ledger entry `disputed=true` + link to incident/post-mortem.
5. **Escalate** if unresolved **> 72h**: Owner convenes Ops/Sec panel; may freeze the implicated control (policy enforcement, bootstrap change) pending outcome.

**Template (suggested):**

```
Dispute: <proposal_id|audit_id>
Summary: <one-liner>
Evidence: <metrics, logs, pcaps>
Requested Override: <scope, duration>
Impact: <SLO/SLA effects>
Contacts: <on-call rota link>
```

---

## 8. Acceptance Checklist (DoD)

* [ ] Invariants (§1) enforced by tests (unit/property/integration) and CI gates.
* [ ] Role bindings and capability scopes validated in staging (macaroons with caveats).
* [ ] Proposal lifecycle implemented; quorum config set; audit emission verified.
* [ ] Metrics in §5 exported; dashboards/alerts wired (incl. `governance_propagate_ms`).
* [ ] SLA thresholds exercised in perf/soak; alerts for breaches present.
* [ ] Appeal path (§7) validated in a governance drill.
* [ ] Custody & rotation policy operational (staging + prod dry-run).
* [ ] PQ toggle workflow exercised (canary → enforce → 24h downgrade window → rollback path).
* [ ] **Governance-lint CI job** checks schema/quorum fields and required metrics are present.

---

## 9. Appendix

### Blueprints

* Economic Integrity (bounded authority/quotas; no unbounded issuance).
* Hardening (capabilities, custody, amnesia; redaction).
* Perfection Gates: I (Invariants), M (Appeals), K (Vigilance), F (Perf).

### References

* Macaroons v1 capability tokens (attenuation, caveats, expiry).
* Kademlia/Discv5 operational notes (provider TTLs, churn, bootstraps).
* `RUNBOOK.md`, `SECURITY.md`, `CONFIG.md`, `TESTS.md`, `OBSERVABILITY.md`.

### History

* **2025-10-11** — Governance genesis (quorum=2-of-3; TTL max=48h; PQ 24h downgrade delay) — approver(s): Stevan White — `audit_id=genesis`
* *Add subsequent events here…*

---

## On-call & CI hooks

* **On-call rota:** see `docs/ops/oncall.md` (weekly rotation; contact escalation levels).
* **CI governance-lint:** validates presence of quorum config, required metrics, and denies PRs that alter governance without an audit plan.

```
```
