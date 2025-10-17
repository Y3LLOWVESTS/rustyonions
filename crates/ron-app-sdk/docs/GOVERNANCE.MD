

---

# 🏛 GOVERNANCE.md

---

title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-16
audience: contributors, ops, auditors, stakeholders
crate-type: sdk|lib
-------------------

# GOVERNANCE.md — ron-app-sdk

## 0. Purpose

This document defines the **rules of engagement** for `ron-app-sdk` as it participates in RustyOnions’ economic/policy system from the **application edge**. While the SDK does not hold mint/settle authority, it **must** enforce policy-relevant client-side invariants (idempotency, quotas adherence, DTO strictness) and provide **auditable, repeatable** mechanisms for change control.

It ensures:

* Transparent, auditable change management of the SDK surface that affects **economic outcomes** downstream.
* Enforcement of **economic invariants at the edge** (no double submission, bounded retries, no out-of-band mint requests).
* Clear **authority boundaries**: SDK proposes; **nodes decide** (policy/ledger/rewarder).
* SLA-backed commitments for SDK behaviors that materially impact consumers (e.g., idempotent retries, header formation, limits).

Ties into:

* **Economic Integrity Blueprint** (no doubles, bounded issuance).
* **Hardening Blueprint** (bounded authority, cap tokens, key custody).
* **Scaling & Performance** (deadline/backoff, OAP/1 frame limits).
* **Perfection Gates** (esp. Gate I: bounded economic invariants; Gate M: appeal paths; Gate K: continuous vigilance).

---

## 1. Invariants (MUST)

Non-negotiable rules that the SDK enforces locally and/or never violates:

* **[I-G1] No double submission:** All **mutations** must carry an **idempotency key**; the SDK must never auto-retry a mutation without the same key, and must never generate two different keys for the same logical operation.
* **[I-G2] Bound retries:** The SDK will not mask failures with unbounded retries. Retries are **exponential with cap** and respect `Retry-After`/quota signals. Auth/Schema failures are **non-retriable**.
* **[I-G3] Frame & chunk limits:** OAP/1 **max frame = 1 MiB** (hard cap). Storage I/O streams in **~64 KiB** chunks. Oversize is refused **client-side**.
* **[I-G4] DTO strictness:** `deny_unknown_fields` stays enabled. Unknown/extra fields cause typed errors; the SDK never silently drops unrecognized data that could impact accounting.
* **[I-G5] Capability discipline:** Every request that requires authority includes a valid capability token (macaroon). The SDK **never** logs, persists, or transmits caps outside the intended channel.
* **[I-G6] Bounded authority:** The SDK cannot mint, settle, or override ledger entries; it only **proposes** operations to node services. There is **no admin override path** in the SDK.
* **[I-G7] Redaction & auditability:** Logs are structured and **safe by default** (no secrets/idempotency keys/macaroons). All governance-relevant events are **traceable** via correlation IDs.
* **[I-G8] Version integrity:** Public API/DTO evolution is **additive-first**. Removals/renames require a **major** version bump and documented deprecation windows.

---

## 2. Roles & Authority

### Roles

* **SDK Maintainers (this crate):** Own SDK surface, invariants enforcement, release governance, and security posture.
* **Policy Owner (`ron-policy`):** Defines system-wide invariants and policy rules.
* **Ledger Keeper (`ron-ledger`):** Executes settlement; enforces conservation; can reject invalid operations.
* **Rewarder (`svc-rewarder`):** Distributes emissions under caps; cannot mint.
* **Auditor:** Internal/external reviewer with read-only insight into logs/metrics, public API diffs, and test evidence.
* **Node Operator:** Operates gateway/overlay/index/storage; consumes SDK edge behavior.

### Authority Boundaries

* SDK **cannot** mint/settle/freeze; it **must** respect transport, quota, and policy signals from nodes.
* Policy can propose rules; Ledger can **reject** requests that violate invariants (SDK must surface these errors without mutation retries).
* Rewarder never mints; SDK may only submit requests within published reward schemas.
* All parties authenticate/authorize using **capability tokens** with scoped caveats.

---

## 3. Rules & SLAs (SDK-level)

* **Idempotency SLA:** For mutation endpoints, the SDK **always** attaches an idempotency key when the caller provides one; helper APIs must make providing it trivial. Duplicate deliveries with the same key must **not** create additional effects (server-enforced; SDK verifies semantics via integration tests).
* **Retry & Deadline SLA:** Default deadlines and retry caps are published; changes follow governance process (Section 4). The SDK never exceeds the cap or ignores `Retry-After`.
* **Limits SLA:** The SDK **refuses oversize** frames locally; chunking behavior is stable across minor releases.
* **Logging SLA:** On error, logs must include `event`, `endpoint`, `attempt`, `deadline_ms`, `latency_ms`, `error_kind`, `transport`, `corr_id`, with **redaction enforced**.
* **Compat SLA:** API/DTO changes are additive-first. Deprecations include **migration notes** and minimum one minor-version grace period.

---

## 4. Governance Process (SDK Changes)

* **Proposal lifecycle (changes affecting economics/security):**
  **Draft → Design Review → Security Review → Approval → Implement → Ship → Postmortem (if hotfix).**

  * **Design Review quorum:** ≥2 SDK maintainers + 1 policy/ledger reviewer for economics-relevant changes.
  * **Security Review:** Required for auth header logic, capability handling, logging/redaction, or PQ adapters.
  * **Approval:** Tracked via PR label and signed-off checklist (Section 8).
* **Deprecations:** Clearly documented, with code warnings where feasible; removal only in a **major** release.
* **Hotfix (Emergency):**

  * Trigger: critical security or economics-impacting bug (e.g., duplicate submission tolerance, leaked headers).
  * Path: short-circuit review with **post-hoc** security sign-off within 48h.
  * Disclosure: changelog entry and release notes detailing impact/scope/migration.

---

## 5. Audit & Observability

* **Auditability:**

  * Public API diffs recorded per release (`cargo public-api`).
  * DTO schema diffs stored alongside releases.
  * Test artifacts retained (coverage, fuzz corpora, integration logs).
* **Metrics (host-exported, SDK-prefixed):**

  * `sdk_requests_total{endpoint}`
  * `sdk_request_latency_seconds{endpoint}`
  * `sdk_backoff_retries_total`
  * `sdk_rate_limited_total`
  * `sdk_auth_failures_total`
  * `sdk_oap_violations_total`
  * Optional cache metrics: `sdk_cache_hits_total|_misses_total|_evictions_total`
* **Red-team drills:**

  * Simulate rogue caller attempts (oversize frame, forged caps, replay); verify SDK refuses/flags before transport.

---

## 6. Config & Custody

* **Config declaration (SDK side):**

  * Deadlines, retry caps, backoff, transport mode (TLS/Tor), OAP/1 limits (compile-time constants), idempotency behavior, optional cache bounds/TTL, PQ posture (if feature-gated).
* **Custody:**

  * The SDK **never** stores raw private keys.
  * Capability tokens (macaroons) are **in-memory only**; not logged; not persisted.
  * If a caller integrates signing keys, they must live in a **system keychain/HSM** (outside this crate); SDK only uses abstracted sign interfaces.
* **Rotation:**

  * Capability rotation is caller-driven; SDK must **fail closed** on expired/denied and surface the error without retries.

---

## 7. Appeal Path (when economic outcomes are disputed)

1. **Mark disputed at the node** (server-side ledger flagging; SDK includes correlation IDs that link client attempts).
2. **Open a governance proposal** (node governance channel) with supporting SDK request/response evidence (redacted logs).
3. **Multi-sig override** (if permitted by policy) to remedy effects; **no silent rollback** performed by the SDK.
4. **Auditor review** and public disclosure (release notes or governance log).

---

## 8. Acceptance Checklist (Definition of Done)

* [ ] Invariants enforced in code: idempotency, bounded retries, OAP cap, DTO strictness, redaction.
* [ ] Roles & authority boundaries documented in crate docs.
* [ ] Governance process followed (Design + Security Review sign-offs).
* [ ] Metrics and structured logs verified in integration/chaos tests.
* [ ] Perf/limits honored (bench + limits tests green).
* [ ] Appeal path documented, with sample evidence bundle (redacted) from tests.
* [ ] SemVer/DTO diffs attached to the release; deprecation windows communicated.

---

## 9. Appendix

* **Blueprints:** Economic Integrity (no doubles, bounded emission); Hardening (bounded authority, custody); Scaling/Performance (OAP/1 limits, latency budgets); Six Concerns alignment.
* **References:** Macaroons v1 capability tokens; DTO schemas; API/CONFIG/SECURITY/OBSERVABILITY/PERFORMANCE docs; public-api diffs; fuzz/prop test specs.
* **History:** Keep a short ledger of notable governance incidents (e.g., hotfixes, deprecation removals, red-team findings) with dates, scope, outcome.

---

### Notes for Implementers

* Any change to **idempotency semantics**, **capability handling**, **logging/redaction**, **OAP/1 limits**, or **DTO acceptance** is **governance-sensitive** and must follow Section 4.
* For economics-relevant bugfixes, always produce: (a) failing test first, (b) fix, (c) retroactive risk assessment, (d) release notes with guidance.

---

