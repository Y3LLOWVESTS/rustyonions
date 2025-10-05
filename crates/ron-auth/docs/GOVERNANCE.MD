
````markdown
# 🏛 GOVERNANCE.md — `ron-auth`

---
title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, ops, auditors, stakeholders
crate-type: policy
---

## 0. Purpose

`ron-auth` defines the **verification policy** for capability tokens (macaroon-style), including stable **deny reason taxonomy**, **caveat semantics**, and **key-lookup contracts**.  
This document sets the **rules of engagement** for change control, auditability, and bounded authority so that upgrades cannot silently weaken authentication or break interop.

It ensures:
- Transparent, auditable decision-making for policy changes (reason strings, caveat registry).
- Enforcement of **auth-policy invariants** (deny-by-default for unknowns, bounded inputs).
- Clear **authority boundaries** between verifier, issuers (svc-passport), and custodians (ron-kms).
- SLA-backed commitments to external consumers (stable surface across v1.x).

Ties into:
- **Economic Integrity Blueprint** (no doubles, bounded issuance) — adapted here as **no “free” authority escalation** and **no unbounded attenuation bypass**.
- **Hardening Blueprint** (bounded authority, key custody, size caps).
- **Perfection Gates A–O** (esp. **Gate I**: bounded invariants; **Gate M**: appeal paths; **Gate F/L**: perf + scaling).

---

## 1. Invariants (MUST)

Non-negotiable rules for this crate’s policy surface:

- **[I-A1] Stable Reason Strings:**  
  All deny reasons are stable **string constants** (e.g., `parse.b64`, `parse.cbor`, `parse.bounds`, `schema.unknown_field`, `mac.mismatch`, `kid.unknown`, `tenant.mismatch`, `caveat.*`, `caveat.custom.*`).  
  • Adding new reasons = **minor**; renaming existing = **major** (v2).  
  • Tests MUST assert exact reason strings on golden vectors.

- **[I-A2] Deny by Default:**  
  Unknown caveat namespaces, unknown schema fields (unless explicitly permitted), invalid MACs, or out-of-bounds sizes MUST result in **Deny** (never “best effort”).

- **[I-A3] Bounded Inputs:**  
  `max_token_bytes` default 4096 (validated range [512, 16384]); `max_caveats` default 64 (validated range [1, 1024]). Tokens beyond caps MUST be denied with `parse.bounds`.

- **[I-A4] Key Custody Separation:**  
  `ron-auth` never holds raw secret keys. It receives **opaque handles** via `MacKeyProvider`. All key generation, storage, and rotation belong to **ron-kms**; minting belongs to **svc-passport**.

- **[I-A5] Rotation Window Safety:**  
  Verifier MUST accept **current + N previous** KIDs (window determined by ops). Dropping previous KID from the window MUST deterministically yield `kid.unknown`.

- **[I-A6] No Hidden Overrides:**  
  No “debug” or environment toggle may suppress **deny-by-default** or enlarge authority without explicit config + audit log in host.

- **[I-A7] Auditability:**  
  All policy-relevant outcomes are observable via **golden metrics and stable reasons** exported by hosts; changes to taxonomy/semantics require entry in CHANGELOG + runbook notes.

---

## 2. Roles & Authority

### Roles
- **Verifier Owner (this crate):** maintains reason taxonomy, caveat semantics, and config schema; ships golden vectors.  
- **Issuer (`svc-passport`):** mints capabilities; enforces mint-time constraints; publishes rotation and policy digest.  
- **Custodian (`ron-kms`):** stores/rotates keys; enforces rotation window guarantees.  
- **Host Owners (e.g., `svc-gateway`):** embed `ron-auth`, export metrics/logs, enforce size caps early, wire amnesia policy.  
- **Auditor:** independent read-only; validates changelogs, vectors, and dashboards.

### Authority Boundaries
- `ron-auth` may **define** taxonomy & semantics, but **cannot mint** tokens or control keys.  
- `svc-passport` may define mint policies but **cannot override** verifier deny-by-default rules.  
- `ron-kms` may rotate keys but **cannot alter** decision semantics.  
- Host wrappers may **label/observe** decisions but **cannot change** reason strings or bypass deny-by-default.

---

## 3. Rules & SLAs

- **Semantic Stability SLA (v1.x):**  
  - No breaking changes to reason strings, CBOR field names, or MAC domain strings.  
  - Deprecations require: announcement + 90-day transition + dual-accept w/ warnings.

- **Verification Observability SLA:**  
  - For every `verify()` call, host MUST record: `verify_total`, `deny_total{reason}`, `parse_error_total{kind}`, `unknown_kid_total`, `duration_us`, `token_size_bytes`.  
  - Audit dashboards MUST render deny taxonomy distribution and rotation health.

- **Rotation Health SLA:**  
  - `unknown_kid_total` steady-state ≈ 0.  
  - Any sustained increase (>0 for 10m) triggers rotation drift alert & runbook.

- **Appeals/Overrides SLA:**  
  - Disputed denies are marked via host “dispute” pathway (no silent rollback).  
  - Overrides (e.g., temporary allow for a namespace) require a **multi-sig governance** change with expiry and audit entry.

---

## 4. Governance Process

### Proposal Lifecycle (policy-affecting changes)
1. **Draft** (issue with rationale, examples, vectors).  
2. **Review** (CODEOWNERS: verifier + security + host owners + issuer).  
3. **Approve** (N-of-M maintainers; quorum defined below).  
4. **Execute** (merge; publish new vectors; update RUNBOOK & OBSERVABILITY dashboards).

**Quorum / Signers:**  
- Minimum quorum **3 of 5**: {Verifier maintainer, Security lead, Host rep, KMS rep, Issuer rep}.  
- **Default reject** if quorum not reached in **72h**.

### Emergency Powers (bounded)
- **Freeze Custom Namespaces:** temporary deny for `caveat.custom.*` not on allowlist if abused at scale.  
- **Rotation Window Extend:** temporarily accept **N+1** previous KID to avoid accidental outages.  
- Both require **majority multi-sig**, expiry ≤ 7 days, and audit entry within **24h**.

### Parameter Changes
- Bounds (`max_token_bytes`, `max_caveats`), namespace allowlist, amnesia semantics, or PQ feature toggles must follow the proposal lifecycle (no “drive-by” merges).

---

## 5. Audit & Observability

- **Audit Log (host-exported):**  
  - JSON lines with `event=auth.verify`, `result`, `reason`, `tenant`, `kid`, `corr_id`, `token_digest8` (redacted).  
  - No secrets; no raw tokens; retention matches org policy.

- **Metrics (host-exported; stable series):**  
  - `ron_auth_verify_total`  
  - `ron_auth_deny_total{reason}`  
  - `ron_auth_parse_error_total{kind}`  
  - `ron_auth_unknown_kid_total`  
  - `ron_auth_duration_us_bucket`  
  - `ron_auth_token_size_bytes`

- **Verifiability:**  
  - Golden vectors (allow/deny with exact reasons) published per release; CI enforces parity.  
  - Rotation drills: alert on `rate(unknown_kid_total)` > 0 for 10m; attach RUNBOOK link.

- **Red-team Drills:**  
  - Simulate rogue admin attempting to introduce permissive custom caveat; ensure deny-by-default holds and governance prevents merge without quorum.

---

## 6. Config & Custody

- **Config (host-provided to verifier):**  
  - Bounds: `max_token_bytes`, `max_caveats` (validated).  
  - Registrar: custom caveat allowlist (minimal by default).  
  - Amnesia binding: whether host operates in **amnesia mode** (RAM-only/no persistent logs).  
  - PQ toggles: `pq-hybrid` envelope verification (optional).

- **Custody (keys):**  
  - Private keys live in **ron-kms**/HSM; verifier receives opaque `MacHandle`.  
  - No raw keys in env/files.  
  - **Rotation Policy:** at least **every 90 days** or immediately after suspected compromise; window accepts current + N previous until fleet convergence.

---

## 7. Appeal Path

- **When a deny is disputed:**  
  1) Host attaches `disputed=true` metadata (no rollback).  
  2) Open governance issue with token digest/time, reason, and minimal reproduction.  
  3) If change is warranted, file a **proposal** (Section 4) to adjust registrar or policy.

- **Escalation:**  
  - Step 1: governance bus topic or Git issue labelled `auth-policy`.  
  - Step 2: multi-sig override proposal with expiry.  
  - Step 3: auditor review & public disclosure note in release.

---

## 8. Acceptance Checklist (DoD)

- [ ] Invariants (I-A1…A7) enforced by tests (unit/integration/property).  
- [ ] Reason taxonomy & caveat semantics documented; **no rename** without major bump.  
- [ ] Proposal lifecycle + CODEOWNERS/quorum in place.  
- [ ] Metrics & audit formats stable; dashboards wired.  
- [ ] Rotation drills + alert thresholds defined and tested.  
- [ ] Appeal path validated via a dry-run (dispute → proposal → resolution).  

---

## 9. Appendix

### 9.1 Blueprints
- **Economic Integrity:** no silent authority inflation; no “free” attenuation.  
- **Hardening:** bounded authority; key custody separation; size caps.  
- **Perfection Gates:**  
  - **Gate I** — bounded invariants (deny-by-default, stable reasons).  
  - **Gate M** — appeal paths operational.  
  - **Gate F/L** — perf/scaling guardrails intact (cross-ref PERF/RUNBOOK).

### 9.2 Mermaid — Policy Change Flow
```mermaid
flowchart LR
  D[Draft] --> R[Review]
  R -->|Quorum met (N-of-M)| A[Approve]
  R -->|No quorum in 72h| X[Reject]
  A --> E[Execute: merge + vectors + runbook]
  E --> O[Observe: dashboards + alerts]
  style A fill:#166534,stroke:#052e16,color:#fff
  style X fill:#7f1d1d,stroke:#450a0a,color:#fff
````

### 9.3 References

* `API.md` — traits (`MacKeyProvider`, `SigAdapter`), `Decision`, deny reasons.
* `CONFIG.md` — bounds, registrar, amnesia binding, env mapping.
* `OBSERVABILITY.md` — metrics taxonomy, dashboards, alerts.
* `SECURITY.md` — zeroization/log redaction; trust boundaries.
* `IDB.md` — invariants & canonical vectors; interop guarantees.

### 9.4 History

* Keep a short ledger of notable governance events (reason additions, namespace freezes, rotation incidents, emergency powers usage).

```

