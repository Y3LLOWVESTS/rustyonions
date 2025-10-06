---

title: Security Notes — ron-policy
crate: ron-policy
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
-------------

# Security Documentation — ron-policy

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-policy`.
It complements the repo-wide Hardening and Interop blueprints, and the crate’s `IDB.md`, `CONFIG.md`, and `CONCURRENCY.md`.

> `ron-policy` is a **pure library**: no sockets, no background tasks, no persistent storage, no cryptography.
> It validates and evaluates **declarative policy bundles** and returns **decisions + obligations**. All I/O, auth, and crypto live in consuming services.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats (library-relevant)                                          | Relevant? | Mitigation in `ron-policy` (and consumer notes)                                                                                                                                                                                               |
| -------------------------- | ------------------------------------------------------------------- | :-------: | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Untrusted or forged policy bundle masquerading as signed            |   **Y**   | Library enforces **schema strictness** (`deny_unknown_fields`), **content-address check** (`id_b3` recomputation helper), **version monotonicity**; consumers must only accept bundles via **multi-sig registry** and authenticated channels. |
| **T**ampering              | On-path mutation of bundles; drift between `id_b3` and body         |   **Y**   | Deterministic parse/serialize; **recompute & compare `id_b3`** helper; **tighten-only** validator forbids relaxing platform caps; **break_change** gating for widening.                                                                       |
| **R**epudiation            | Missing audit trail for allow/deny decisions                        |   **Y**   | Decisions carry **reason codes** and **AuditTag** obligations; consumers must emit **structured JSON logs** with `corr_id`/policy `version` and expose metrics.                                                                               |
| **I**nformation Disclosure | Accidental logging of sensitive policy metadata or capability hints |   **Y**   | Library holds **no secrets** and avoids logging bundle contents; recommends **redaction** helpers; consumers must avoid logging tokens/headers and treat obligations as **non-secret** hints.                                                 |
| **D**enial of Service      | Oversized/complex bundles; pathological inputs; rule explosion      |   **Y**   | Upper bounds: **rule count cap** (perf.max_rules_hot_path), **tighten-only caps** (1 MiB body, 10× decompress), **deterministic O(m)** evaluator; strict parsing; recommend **bundle byte-size cap** in consumers.                            |
| **E**levation of Privilege | Ambient allow decisions; bypass of capability checks                |   **Y**   | **Capability-first** invariant (no ambient authority); routes may set `require_cap=true`; evaluation is **pure** and only returns decisions—**enforcement** remains in services with real auth.                                               |

---

## 2) Security Boundaries

* **Inbound (to the library):**

  * `PolicyBundle` (JSON/TOML, usually fetched from `svc-registry` by a service).
  * `RequestCtx` structs from services (metadata only; **no opaque secrets**).
  * Configuration knobs (`ron_policy::Config`) provided by services.

* **Outbound (from the library):**

  * `Decision { allow, reason, obligations[] }` and **typed errors** (e.g., `ValidationError::TightenOnly`).
  * Optional helpers: `validate_bundle_diff(old,new)`; **no** network effects.

* **Trust Zone:**

  * Runs **in-process** with the consumer service; **not** network-facing; **unprivileged**.
  * Trusts only the **caller’s identity checks** (macaroons/KMS in other crates).

* **Assumptions:**

  * Multi-sig `svc-registry` integrity is enforced outside this crate.
  * Consumers provide **TLS 1.3** and **capability auth** for any network‐exposed surface.
  * Metrics/health/config bus are trustworthy within the tenant.

---

## 3) Key & Credential Handling

* **Keys handled by `ron-policy`:** **None.** The crate must never accept, parse, or store secret key material.
* **References only:** Policies may contain **key IDs** or **proof kinds** (ECON/ZK hints) in obligations; **no secret bytes**.
* **Storage:** Not applicable (no secrets).
* **Rotation:** Not applicable; rotation happens in `ron-kms`/`svc-passport`/`svc-registry`.
* **Zeroization:** Not required for this crate; do **not** introduce secret-like fields.

> If a future helper accidentally admits secrets (e.g., reading a macaroon), **remove it or move it** to the appropriate service crate.

---

## 4) Hardening Checklist (library-scoped)

* [ ] **Schema strictness:** `serde(deny_unknown_fields)` on all DTOs.
* [ ] **Tighten-only validation:** caps cannot exceed platform bounds (1 MiB body, 10× decompress).
* [ ] **Churn protection:** widening residency/limits demands `metadata.break_change=true` + runbook URL.
* [ ] **Deterministic evaluation:** pure function; no I/O, no global state.
* [ ] **Panic hygiene:** no `unwrap`/`expect` on untrusted data paths; return typed errors.
* [ ] **Performance bound:** evaluation p95 < 1 ms for ≤1k rules (bench-gated in CI).
* [ ] **No logging of bundle contents by default;** decision reason strings are safe.
* [ ] **No cryptography:** content addressing/check helpers allowed; signature verification is **out of scope** here.
* [ ] **Docs as code:** IDB/CONFIG/CONCURRENCY kept in sync; CI gate.

> **Service-level** items (timeouts, RPS caps, TLS, UDS perms, `/readyz`) remain in consumer services and are referenced in their security docs.

---

## 5) Observability for Security

* **Decision/Validation metrics (names only; emitted by services):**

  * `policy_rejects_total{reason="tighten_only|churn|schema|perf|other"}`
  * `policy_changes_total{result="applied|rejected"}`
  * `policy_eval_latency_seconds` (histogram)

* **Log guidance (consumers):**

  * **Structured JSON**; include `corr_id`, `policy_version`, `policy_id_b3`, `reason`.
  * Redact any user tokens/headers; do not log raw bundles in prod.
  * On rejection, include **diff summary** (counts/keys), not entire documents.

* **Health gates (consumers):**

  * Fail `/readyz` if **no valid snapshot** is loaded or last reload **rejected** and no previous good exists.

---

## 6) Dependencies & Supply Chain

* **Core:** `serde`, `serde_json`, `toml`, `blake3`, `thiserror`/`anyhow` (typed errors), optional `schemars` (schema).
* **Dev-only:** `proptest`/`arbitrary` (property tests), `cargo-fuzz` (parsers), `criterion` (bench), `loom` (atomic swap model).
* **Pinning:** Versions pinned in workspace; **MSRV 1.80**.
* **Controls:** `cargo-deny` (license/advisory), `cargo-audit` (RUSTSEC), **SBOM** at release (`/docs/sbom/ron-policy.spdx.json`).
* **No crypto libs:** `ron-policy` must not depend on `ring`, `openssl`, or `rustls` directly.

---

## 7) Formal & Destructive Validation

* **Property tests:**

  * Determinism: `(bundle,ctx,clock)` ⇒ identical `Decision`.
  * Schema strictness: unknown fields rejected (when strict enabled).
  * Tighten-only/churn rules: negative cases covered.

* **Fuzzing:**

  * JSON/TOML policy bundle parsers (max depth, string lengths, numeric bounds).

* **Loom (dev):**

  * Model atomic **`Arc<Policy>` snapshot swap** used by consumers; assert no torn state and no deadlocks.

* **Chaos (in consumers):**

  * Kill loader between validate/swap; service continues on last good; next good applies.

* **TLA+ (optional):**

  * Spec: **Safety** (never accept relaxed cap), **Liveness** (eventual application of valid bundle under repeated events).

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Security contact:** [security@rustyonions.example](mailto:security@rustyonions.example) (or repo `SECURITY.md`)
* **Disclosure:** Coordinated disclosure per repo root policy.

---

## 9) Migration & Upgrades

* **Breaking changes** to DTOs/validation require a **major** version and a migration note in `CHANGELOG.md`.
* **Deprecations**: keep aliases ≥1 minor; mark warnings; provide convert tool if needed.
* **Schema**: evolve via semver’d JSON Schema; CI enforces N→N+1 compatibility unless `break_change=true`.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  R[svc-registry (multi-sig)] -->|bundle JSON + id_b3| V{ron-policy<br/>validate}
  V -- reject --> E[Error + Metrics/logs]
  V -- ok --> S[Arc<PolicyBundle> Snapshot]
  C[Service (e.g., gateway)] -->|ctx| P[ron-policy evaluate]
  P -->|Decision + Obligations| C
  C -.->|enforce allow/deny, auth, quotas| Down[Downstream services]
  style V fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text:** Registry provides a bundle; `ron-policy` validates it (tighten-only, schema, churn). On success the service swaps in a new snapshot. Requests pass a context to `evaluate`; the service enforces the returned decision and obligations.

---

### Appendix A — Negative Patterns (Forbidden)

* Logging raw bundles or secrets.
* Accepting bundles from unauthenticated/unverified sources.
* Performing any network I/O or signature verification inside `ron-policy`.
* Introducing ambient allow paths (capability-less allows where `require_cap=true`).

### Appendix B — Reviewer Checklist

* [ ] DTOs still `deny_unknown_fields`.
* [ ] Tighten-only and churn gates covered by tests.
* [ ] No new dependencies introduce crypto/I/O.
* [ ] Bench still meets p95 < 1 ms @ ≤1k rules.
* [ ] Docs (IDB/CONFIG/CONCURRENCY/SECURITY) updated together.
