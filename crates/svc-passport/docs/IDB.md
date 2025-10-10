````markdown
---
title: svc-passport — Identity & Credentialing Service (IDB)
version: 0.1.0
status: draft
last-updated: 2025-10-09
audience: contributors, ops, auditors, integrators
---

# svc-passport — Invariant-Driven Blueprint

> **Role in canon:** Identity & Key Management pillar (P3) alongside `ron-kms` and `ron-auth`. Issues short-lived, capability-scoped tokens; binds keys to attestations; avoids PII accumulation. Verification lives in `ron-auth`; private keys live in `ron-kms`.

---

## 1) Invariants (MUST)

- **[I-1] Capability-only surface.** All issued artifacts are **capability tokens** (macaroon-style); no ambient trust, no session cookies. Authoritative verification lives in `ron-auth`.

- **[I-2] Short lifetimes + rotation.** Default TTLs are short (minutes–hours). Issuer/roots rotate ≤ 30 days with overlapping validity; rotation must not induce downtime.

- **[I-3] KMS custody, not here.** Private keys and seal/unseal live behind `ron-kms` traits; svc-passport never handles raw private key bytes beyond opaque KMS handles. Any transient secrets are zeroized by the KMS layer.

- **[I-4] PQ-hybrid readiness.** Issuance metadata and verification hints support PQ-hybrid (e.g., ML-DSA signatures) without breaking interop. Tokens carry an explicit `alg` family; negotiation is caller-driven.

- **[I-5] Minimal PII, amnesia-aware.** Default flows require **no durable PII**. With amnesia mode ON, the service stores only RAM-resident, expiring state (e.g., rate counters). Optional identity attributes must be hashed/salted or externalized.

- **[I-6] Deterministic DTOs.** All request/response DTOs use `#[serde(deny_unknown_fields)]` and live in a pure types module (or `ron-proto`). No logic embedded in DTOs.

- **[I-7] Hardened ingress.** Apply default limits: 5s timeout, 512 inflight, 500 rps/instance, 1 MiB body cap, structured errors, and early quota checks. `/readyz` **fail-closes writes** first under pressure.

- **[I-8] Governance hooks.** Issuer descriptors and policy toggles are versioned and signed in `svc-registry`; revocation events are bus-announced; ops can canary/rollback.

- **[I-9] Six Concerns mapping.** This crate satisfies **SEC** and **GOV** concerns and must register appropriate CI gates.

---

## 2) Design Principles (SHOULD)

- **[P-1] Grant the least, late.** Issue the narrowest capability with explicit caveats (target svc/routes, byte/method ceilings, region) and **short TTL**; prefer mint-per-purpose over omnibus tokens.

- **[P-2] Externalize identity proofs.** Defer heavyweight authN (e.g., OIDC/JWT introspection) to interop translators → capabilities; svc-passport only **mints** capabilities after translation.

- **[P-3] Clear revocation model.** Prefer time-bounded caveats + key rotation over stateful deny-lists; keep fast path **stateless** verification (`ron-auth`) plus push revocation epochs via bus.

- **[P-4] Profile parity.** Same API in Micronode and Macronode; Micronode defaults to amnesia.

  **Config Parity Flags (examples)**
  - `amnesia = true|false` (Micronode default: `true`)
  - `pq.mode = "mirror"|"require"|"enforce"` (default: `"mirror"`)
  - `limits.body_max_bytes = 1_048_576`
  - `limits.rps = 500`

- **[P-5] Observability first.** Expose issuance/verify **latency histograms**, reject reasons, and revocation counters via `ron-metrics`.

---

## 3) Implementation (HOW)

> Service class: **svc** (not a library). Ingress via `svc-gateway` HTTP; internal calls over UDS where applicable. DTOs remain logic-free.

### 3.1 Endpoints (stable surface)

- `POST /v1/passport/issue`
  - **Input:** `IssueRequest { subject_ref, audience, ttl_s, caveats[], accept_algs[], proof? }`
  - **Effects:** Validates policy; asks `ron-kms` to sign; returns `Capability { token, kid, alg, exp, caveats[] }`

- `POST /v1/passport/verify`
  - **Input:** `VerifyRequest { token }`
  - **Effects:** Delegates to `ron-auth` verification path (local call); echoes decision + parsed caveats for caller diagnostics

- `POST /v1/passport/revoke`
  - **Input:** `RevokeRequest { kid|epoch|subject_ref, reason }`
  - **Effects:** Rotates epoch or key material via `ron-kms`; emits bus revocation event; returns new `current_epoch`

**Transport invariants:** same hardening defaults as other ingress services (time/rps/body caps), structured error envelope, `/readyz` semantics.

**DTO Hygiene**
- All requests use `#[serde(deny_unknown_fields)]`.
- `ttl_s` is clamped to `policy.max_ttl`; exceeding requests return `400 {reason:"ttl_too_long"}`.
- `caveats` validated against a registry; unknown caveats return `400 {reason:"unknown_caveat"}`.

#### Sequence (GitHub-friendly)

```mermaid
sequenceDiagram
  autonumber
  participant App as App/SDK
  participant PP as svc-passport
  participant KMS as ron-kms
  participant AUTH as ron-auth
  App->>PP: POST /v1/passport/issue {accept_algs, audience, caveats, ttl_s}
  PP->>KMS: sign(cap_bytes, kid)
  KMS-->>PP: sig(s), kid
  PP-->>App: Capability {token, alg, kid, exp, caveats}
  App->>Service: Call with Capability (Authorization: Bearer)
  Service->>AUTH: Verify(token)
  AUTH-->>Service: {ok, parsed_caveats}
  Service-->>App: 2xx/4xx (enforced by caveats & quotas)
````

---

### 3.2 Token format (capability, macaroon-style)

* **Envelope fields (sketch):** `ver, alg, kid, iss, aud, sub, iat, exp, caveats[]`
* **Caveat families:**

  * `svc=svc-mailbox|svc-storage|svc-index|svc-edge|svc-gateway`
  * `route=/mailbox/send|/o/*|/edge/*`
  * `region=us-east-1`
  * `budget.bytes=1048576`, `budget.reqs=100`, `rate.rps=5`
  * `proof.bind=pubkey-hash` (bind to client pubkey when applicable)

**PQ-Hybrid Negotiation (concrete example)**

* Clients include an `accept_algs` list when requesting a token, e.g., `["ed25519+ml-dsa", "ed25519"]`.
* If issuer policy supports ML-DSA, svc-passport mints with `alg="ed25519+ml-dsa"` and `kid` pointing to a hybrid key bundle in `ron-kms`.
* If hybrid is unavailable, it **falls back** to the first acceptable classical alg (e.g., `ed25519`) and sets `caveats += ["pq.fallback=true"]`.

**Verification Hinting**

* Tokens carry `alg`, `kid`, and `epoch`. `ron-auth` selects the verify path by `alg`.
* For `ed25519+ml-dsa`, verification passes only if **both** signatures validate and `epoch` is current (or grace).

---

### 3.3 Key material workflow

* **Mint path:** svc-passport → `ron-kms::sign(cap_bytes, key_id)` → returns detached sig(s) & `kid`.
* **Rotate path:** schedule overlapping key epochs; publish to `svc-registry`; `ron-auth` consumes descriptors; `/readyz` degrades if verification set desyncs.
* **Amnesia mode:** on Micronode, issuer cache (public keys/epochs) is RAM-only and expiring; no long-term logs.

**Hybrid Rollout Phases**

1. **Mirror**: mint both classical and hybrid; verification accepts either (grace window).
2. **Require**: mint hybrid; verification requires hybrid unless `pq.fallback=true` and still within policy grace.
3. **Enforce**: mint hybrid only; verification of classical-only tokens fails post-grace.

**Safe Fallback**

* If `accept_algs` excludes all issuer-supported algs, respond `400 {reason:"no_acceptable_alg"}` (do not silently choose).

---

### 3.4 Data & storage stance

* **No durable identity DB.** Store only issuance logs (structured, append-only) routed to `ron-audit` if configured; otherwise keep rolling in-RAM counters for rate/abuse.
* **PII minimization:** `subject_ref` is caller-provided opaque handle (e.g., salted hash); svc-passport never stores usernames/emails.

---

### 3.5 Metrics (golden set + SLOs)

* `passport_issue_latency_seconds{result}` (histogram)
* `passport_verify_latency_seconds{result}` (histogram)
* `passport_revocations_total{reason}`
* `passport_tokens_issued_total{aud,svc}`
* `passport_rejects_total{reason}` (e.g., `policy_violation`, `ttl_too_long`, `revoked`, `sig_mismatch`)

**SLO Targets (per instance)**

* p95 `passport_issue_latency_seconds` ≤ 40ms; p99 ≤ 100ms
* p95 `passport_verify_latency_seconds` ≤ 10ms; p99 ≤ 25ms
* `passport_rejects_total{reason="policy_violation"}` / `passport_tokens_issued_total` ≤ 1% rolling 5m
* Revocation propagation (bus to downstream refusal) ≤ 5s (p99)

**Alerting (recommend)**

* Page if p99 issue latency > 150ms for 5m
* Page if revocation propagation SLO missed for 2 consecutive windows

---

### 3.6 Bus events

* `PassportRevoked { kid|epoch, reason, ts }`
* `PassportPolicyUpdated { version }`

Subscribers include `svc-gateway`, `svc-index`, `svc-mailbox` to proactively drop stale caps.

---

### 3.7 Interop & app integration

* **Apps** obtain caps from `svc-passport` and attach them via SDK on each call; ingress enforces quotas; downstreams verify via `ron-auth`.

---

## 4) Acceptance Gates (PROOF)

**Unit/Property**

* **[G-1] TTL discipline.** Reject issuance where `ttl_s > policy.max_ttl`; property tests ensure `exp = iat + ttl_s` monotonic and non-wrap.
* **[G-2] Caveat correctness.** Round-trip encode/decode; unknown caveats rejected; budget/rate caveats never increase effective rights.
* **[G-3] PQ toggles.** Build matrix proves tokens carry `alg` hints; verification path (via `ron-auth` mock) honors classical and hybrid.

**Integration**

* **[G-4] KMS boundary.** All signing goes through `ron-kms`; tests prove no code path loads raw private key bytes into svc memory; zeroization verified by KMS tests.
* **[G-5] Rotation without downtime.** Overlapping key epochs validated: old tokens verify until `exp`; new tokens mint immediately; `/readyz` remains 200 throughout.
* **[G-6] Revocation propagation.** Bus event causes downstream caches to drop affected `kid/epoch` within SLO (≤ 5s p99).

**Hardening/Resilience**

* **[G-7] Ingress caps.** 2 MiB body → **413**; >500 rps load → early shedding; `/readyz` fail-closed for issuance, read-open for verification (can still verify while mints shed).
* **[G-8] Amnesia hygiene.** With amnesia=ON, fs probes show **zero** persistent artifacts after steady-state; logs routed to RAM or disabled.

**Observability**

* **[G-9] Golden metrics present** and increment in smoke: one issue, one verify, one revoke → counters/histograms reflect.

**Governance**

* **[G-10] Registry descriptors signed**; policy changes emit versioned events; CI denies unsigned issuer updates.

**SLO/Perf**

* **[G-11] SLO conformance.** Load-test suite proves p95/p99 targets; CI fails if regressions >10% vs baseline.
* **[G-12] Propagation bound.** Integration test measures revoke→refusal ≤ 5s (p99) across 3 downstream services.

**Six Concerns CI routing**

* Label `concern:SEC,GOV` + `pillar:3` to run hardening, schema, and policy/registry checks.

---

## 5) Anti-Scope (Forbidden)

* ❌ No user account database, passwords, or profile CRUD (externalize via interop; this crate only **mints** capabilities).
* ❌ No durable PII stores in svc-passport (hashes/opaque refs only).
* ❌ No token verification logic forked here (authoritative verification is in `ron-auth`).
* ❌ No key storage or custom crypto; all keys via `ron-kms`.
* ❌ No economic semantics (fees, balances)—that’s `ron-ledger`/`svc-wallet`.
* ❌ No ambient trust or blanket “admin” tokens; every token is caveat-scoped.

---

## 6) References

* Canon crate atlas: Identity & Keys pillar (P3), Micronode amnesia posture, rotation ≤ 30d, capability-first design.
* App Integration Blueprint: capability flows; SDK attachment; ingress enforcement.
* Hardening Blueprint: global limits; DTO hygiene; amnesia mode.
* Six Concerns: `SEC`, `GOV` gates and CI labeling.
* svc-registry: signed issuer descriptors/policies.

---

## 7) Versioning Policy (SemVer + Policy)

* **Crate SemVer**

  * **Minor** bump when the default `alg` set changes (e.g., enabling hybrid by default).
  * **Patch** for non-breaking policy toggles or SLO tweaks.
  * **Major** if DTO fields change in a non-backward-compatible way.

* **Issuer Policy Version**

  * `issuer_policy.version` monotonically increases and is signed in `svc-registry`.
  * Releases MUST reference the minimum required policy version in `CHANGELOG.md`.

---

### Definition of Done (for this IDB)

* Only references **canonical crates** and P3 boundaries.
* Maps each invariant to at least one acceptance gate.
* Aligns with PQ-hybrid posture, amnesia, hardening limits, and governance wiring.
* Keeps verification in `ron-auth` and keys in `ron-kms`, preventing scope creep.

```
```
