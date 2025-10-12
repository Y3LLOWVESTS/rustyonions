

---

# 🏛 GOVERNANCE.md

---

title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-12
audience: contributors, ops, auditors, stakeholders
crate-type: policy|econ
-----------------------

## 0. Purpose

This document defines the **rules of engagement** for `svc-interop`—the interop bridge that ingests external events/requests (webhooks, REST/GraphQL, SDK calls) and translates them into **capability-guarded** internal actions **without expanding economic authority**.

It ensures:

* Transparent and auditable decision-making for adapter enable/disable, provider policy, and signature/secret rotation.
* Enforcement of **economic invariants**: no double-effects from the same external event, bounded issuance of rewards/credits via downstream services, and strict default-deny policy.
* Clear authority boundaries and appeal paths across **policy → interop → ledger/rewarder** lines.
* SLA-backed commitments to external consumers with **brownout** behavior rather than unsafe failure.

It ties into:

* **Economic Integrity Blueprint** (no doubles, bounded issuance).
* **Hardening Blueprint** (bounded authority, key custody, amnesia).
* **Perfection Gates A–O** (esp. Gate I: bounded economic invariants, Gate K: vigilance, Gate L: black swans, Gate M: appeal paths).

---

## 1. Invariants (MUST)

Non-negotiable rules for `svc-interop`:

* **[I-G1] No double-effects:** Each external event (by provider+event_id or idempotency key) **MUST** be applied at most once to downstream systems. Deduplicate at the edge and/or via capability caveats.
* **[I-G2] No mint/settle authority:** `svc-interop` **MUST NOT** mint value, settle ledger entries, or bypass `ron-ledger`/`svc-rewarder`. It may only propose or request via scoped capabilities.
* **[I-G3] Default-deny:** All adapters and endpoints are **deny-by-default**. Enabling requires a governance action recorded in audit with reason, owner, and expiry/review date.
* **[I-G4] Boundaries on hashing:** SHA-256 may be used **only** at the edge for provider verification. Internal addressing and persistence **MUST** be BLAKE3-256.
* **[I-G5] Bounded issuance downstream:** Any “credits/rewards” flows triggered by interop **MUST** pass through policy/ledger limits (caps, schedules). No out-of-band emissions.
* **[I-G6] Append-only governance trail:** Every adapter toggle, threshold change, secret rotation, and policy update is **append-only** logged (with KID/version, diff, quorum).
* **[I-G7] Kill-switches are scoped:** Emergency disable may only target specific adapters/providers/scopes; global off requires multi-sig super-quorum (see §4).
* **[I-G8] Amnesia compliance:** No plaintext secrets or durable writes outside approved sinks; memory/audit practices follow amnesia guidance.
* **[I-G9] Reversible bridges:** Disabling an adapter must never corrupt core state. Re-enable restores prior behavior without debt or replay holes.
* **[I-G10] Stable reasons & schemas:** Error **reason codes** and bus **topic schemas** are versioned and stable; changes require dual-publish windows and golden-file test updates (§10, §12).

---

## 2. Roles & Authority

### 2.1 Roles

* **Interop Steward (crate owners):** Maintains adapter catalog, default-deny posture, reason-code stability; authors proposals for adapter changes.
* **Policy Owner (ron-policy):** Defines economic invariants/caps; approves interop policy that could impact reward/ledger semantics.
* **Ledger Keeper (ron-ledger):** Executes settlement; rejects invalid proposals; preserves conservation laws.
* **Rewarder (svc-rewarder):** Distributes incentives under configured caps; cannot mint.
* **Security/KMS (ron-kms ops):** Manages KID lifecycles, secret/credential rotation, attestations.
* **SRE/Operators:** Runbooks, brownout thresholds, incident response, chaos/game-days.
* **Auditor (internal/external):** Read-only; validates trail, parameters, and invariants.

### 2.2 RACI & Quorum (authoritative)

| Decision              | R (Responsible) | A (Accountable) | C (Consulted)                 | I (Informed) | Quorum                            |
| --------------------- | --------------- | --------------- | ----------------------------- | ------------ | --------------------------------- |
| Enable adapter        | Interop Steward | Policy Owner    | Security/KMS, SRE             | Auditor      | **2-of-3** (Policy, Sec/KMS, SRE) |
| Parameter thresholds  | Interop Steward | Policy Owner    | SRE, Security/KMS             | Auditor      | **2-of-3**                        |
| KID/secret rotation   | Security/KMS    | Policy Owner    | Interop Steward, SRE          | Auditor      | **2-of-3**                        |
| Scoped freeze/disable | SRE             | Policy Owner    | Security/KMS                  | Auditor      | **2-of-3**                        |
| Global freeze         | SRE             | Policy Owner    | Security/KMS, Interop Steward | Auditor      | **3-of-4** super-quorum           |

### 2.3 Authority Boundaries

* `svc-interop` can **verify**, **gate**, and **propose**; it cannot **mint**, **settle**, or **override** ledger.
* `ron-policy` sets rules; `svc-interop` enforces them at the edge; `ron-ledger` is the final arbiter for economic state.
* All actions require **capability tokens (macaroons)** with TTLs, caveats (scope, rate, quota), and monotonic revocation.
* Emergency actions (freeze/kill) are **time-boxed** and require reconfirmation (§3.4).

---

## 3. Rules, SLAs & Governance Health

### 3.1 Service SLAs (edge posture)

* **Verification latency (intra-AZ):** p95 ≤ 120 ms, p99 ≤ 300 ms for webhook checks (signature, clock window, replay guard).
* **Audit emission:** decision → audit append within ≤ 1 s.
* **Brownout behavior:** on upstream or policy threshold breach, WRITE paths degrade first (429/503 with stable `reason`), READ/health stay green when safe.

### 3.2 Economic Rules

* **No doubles:** For each `(provider, event_id|idempotency_key)`, exactly-once delivery to downstream consumers; duplicates are denied with a stable reason.
* **Bounded issuance:** Any reward/credit triggers must respect **caps/quota** specified by policy; exceeding attempts are denied and alert.
* **Schedule discipline:** Emission curves/quotas are immutably versioned; changes require proposal lifecycle (§4).

### 3.3 Governance Health SLOs (measured & alerted)

* **Proposal resolution:** 95% of non-emergency proposals decided in ≤ 72h; 99% in ≤ 7d.
* **Emergency sunset:** 100% of emergency freezes auto-sunset or reconfirmed within 24h.
* **Rotation timeliness:** median KID rotation overlap ≤ 24h; max ≤ 72h.
* **Deadlock watchdog:** if quorum not reachable for 24h, **fail-safe** defaults apply (deny/disable) and open an incident.

### 3.4 Deadlock & Tie-Breaker rules

* If voting **ties** or quorum **stalls**, maintain deny-by-default; any previously enabled risky adapter becomes scoped-disabled until a decision lands.

---

## 4. Governance Process

### 4.1 Proposal Lifecycle

1. **Draft** (Interop Steward): purpose, diff (config/policy), safety analysis, blast radius, rollback plan, test artifacts (TESTS/RUNBOOK links).
2. **Review** (Policy + Security/KMS + SRE): check invariants (I-G1..I-G10), custody, perf impact.
3. **Approve (Quorum)**: per §2.2.
4. **Execute**: controlled rollout (feature flag), observability checks, brownout readiness.
5. **Publish**: append to audit trail & CHANGELOG; schedule re-review if temporary.

**Default reject** if quorum not reached in `T=72h` (or `T=4h` emergency).

### 4.2 How Proposals Are Stored & Signed

* **Canonical storage:** proposals & decisions live in a Git-tracked repo (or tracker) with **append-only** PR history.
* **Decision envelope (signed):** `{id, quorum, votes[], signed_by[], kid, decision_digest}`; digest is published to the audit bus topic.
* **KMS attestation:** rotations/decisions include KMS attestation (origin, policy, time), stored by digest in the governance log.

### 4.3 Emergency Powers

* **Scoped Freeze/Disable:** Majority **2-of-3** can disable specific adapters/providers instantly.
* **Global Freeze:** **3-of-4** super-quorum; must revert to scoped within **24h** or sunset automatically.
* **Disclosure SLA:** emergency actions logged internally **immediately** and publicly disclosed within **24h** (§7, §11).

### 4.4 Parameter Changes

* **Thresholds:** brownout limits (error-rate, p95), rate/quotas, decompression ratio, body limits.
* **Security:** KID rotations, hash policies, replay windows.
* **Economic:** reward caps, schedules, policy caveats.
  All require proposal lifecycle with test evidence.

---

## 5. Audit & Observability

* **Append-only Audit:** Every decision includes `{ts, provider, action, decision, reason, corr_id, capability_id, KID, old→new}`; decision envelopes are **signed** and referenced by digest.
* **Governance Metrics (minimum):**

  * `interop_governance_proposals_total{status}`
  * `interop_governance_resolution_seconds_bucket` (SLOs in §3.3)
  * `interop_adapter_state{provider,adapter,state}` (0/1)
  * `interop_dedup_rejects_total{provider}`
  * `interop_policy_denies_total{reason}`
  * `interop_kid_rotations_total{provider}`
* **Dashboards:** adapter enablement timeline, proposal state funnels, dedup/deny rates, brownout durations, governance health SLOs.
* **Red-team Drills:** Quarterly rogue-admin and secret-leak simulations; demonstrate bounded authority and rotation timeliness.

---

## 6. Config & Custody

* **Config MUST declare:**

  * Per-provider **enablement** (default false), scopes, rate/quotas, body/decompression caps, replay windows, error/latency thresholds for brownout.
  * **Identity & custody**: KID references, key provenance (ron-kms/HSM), rotation cadence (≤90 days) and overlap policy (dual-secret window).
  * **SLA targets** and alert thresholds (Prometheus rules).

* **Custody Rules:**

  * No raw private keys in env/files; only handles to KMS/OS keystore.
  * **Rotation** scheduled or on-demand after incident; **dual acceptance** (old+new) with bounded window; all rotations audited with **KMS attestation** digest.
  * Provider secrets are scoped, least-privilege, and redacted in logs.

---

## 7. Appeal Path & Transparency

* **Appeals:**

  1. Raise dispute on governance topic with corr_id and evidence.
  2. Optional scoped disable of affected adapter(s).
  3. Quorum override proposal with remediation (refund/replay prevention), time-boxed.
  4. Auditor review & disclosure; **public post** within **24h** and full post-mortem within **7 days**.
  5. **No silent rollbacks**—all adjustments visible in audit/ledger facets.

---

## 8. Adapter Governance (Per-Provider)

* **Onboarding:**

  * Provide verification spec (headers, base string, signing alg), replay window, event identity (`event_id`/idempotency key).
  * Conformance tests (fixtures), fuzz corpus seeds, chaos drills (latency & 5xx).
  * Must pass **TESTS.md** Bronze+ before enablement; reach Silver+ within **30 days**.

* **Operation:**

  * Enablement entries: owner, reason, scope, expiry/review date.
  * Changes to headers/replay windows go through lifecycle; hotfixes allowed only with scoped emergency powers.

* **Decommission:**

  * Archive fixtures/corpora; mark adapter “retired”; enforce deny-by-default.

---

## 9. Acceptance Checklist (DoD)

* [ ] Invariants **I-G1..I-G10** enforced in code/config and covered by tests.
* [ ] Roles, RACI, quorum documented and implemented (capability checks).
* [ ] Proposal lifecycle implemented (storage, signatures, digests).
* [ ] Metrics & audit logs exported; dashboards and **Prometheus rules** validated.
* [ ] SLA thresholds validated in perf tests and monitored in prod.
* [ ] Appeal path exercised in a chaos/game-day drill.
* [ ] KID/secret rotation playbook tested (dual-secret overlap, **KMS attestation** captured).
* [ ] Brownout behavior verified: write degrade first, read/health green where safe.
* [ ] **Golden reason codes** updated when needed, with `api-break` label and reviewer ACK.
* [ ] Governance health SLOs (resolution/sunset/rotation) monitored with alerts.

---

## 10. Bus Topics & Schemas (Canonical, Versioned)

* **`governance.proposal.v1`** — `{id, author, diff, rationale, risk, attachments, ttl}`
* **`governance.decision.v1`** — `{id, quorum, votes, result, signed_by[], kid, decision_digest}`
* **`adapter.state.v1`** — `{provider, adapter, state, reason, owner, ttl}`
* **`security.rotation.v1`** — `{provider, kid_old, kid_new, window, completed_ts, attestation_digest}`
* **`interop.dedup.v1`** — `{provider, event_id, decision, corr_id}`

**Versioning policy:** semver on topics (`*.vN`). Any change requires: (1) migration note, (2) **dual-publish window**, (3) golden-file test update in `TESTS.md` and PR labeled `api-break`.

---

## 11. Risk Controls (Black Swan)

* **Kill-switch tiers:** adapter → provider → class → global (last resort).
* **Replay storm breaker:** if dedup rejections exceed **X/min** or unsigned events exceed **Y/min**, **auto scoped-disable** the adapter and page SRE (X/Y are config-driven).
* **Signature drift:** if verification error-rate > threshold, enter brownout and rotate secrets; disclose incident per §7.
* **Supply-chain:** pin dependencies; deny unknown sources; SBOM per release.
* **Clock skew:** continuous NTP checks; deny beyond window; expose metric & alert.
* **Deadlock watchdog:** auto-fail-safe and incident creation if quorum unavailable ≥ 24h.

---

## 12. References

* Economic Integrity Blueprint (no doubles, bounded issuance).
* Hardening Blueprint (bounded authority, custody, amnesia).
* RUNBOOK.md (brownout, recovery, rotations).
* TESTS.md (gates, fuzz/chaos/loom, **golden reason codes**).
* SECURITY.md (hashing policy, macaroons, audit redaction).
* OBSERVABILITY.md (metrics taxonomy, alerts).
* CONFIG.md / INTEROP.md (adapter matrix, thresholds, headers/signing).

---

## 13. History

Maintain a summarized, append-only timeline of major governance actions:

* **2025-10-12** — Initial `svc-interop` governance spec adopted (default-deny posture; per-provider onboarding; signed decisions; governance health SLOs).
* (append entries here with PR/tag references)

---

