
---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, security auditors, ops
crate: ron-policy
crate-type: policy|lib
pillar: 3 (Security & Governance)
owners: [Stevan White]

---

# QUANTUM.md — ron-policy

## 0) Purpose

Define how `ron-policy` remains **safe under quantum threats** and how we migrate toward **post-quantum (PQ)** without breaking interop or operator workflows.
Scope includes: exposure assessment, current crypto profile, target posture, feature/config toggles, migration milestones, invariants, observability, tests, and acceptance gates. Because `ron-policy` is a **pure library** (no I/O), most PQ mechanics (KEX, session, certs) live in **transports/identity**; this doc pins how **bundle authenticity** and **policy acceptance** interact with PQ.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (breakable by Shor):**

  * **Bundle signatures (governance approval):** Signed by K-of-N approvers. ron-policy itself does not verify I/O; *consumers* (e.g., gateway/index) verify signatures before passing a bundle to `ron-policy::validate`. If classical signatures become forgeable, an attacker could push **malicious policy** (worst-case: unauthorized allow/deny).
  * **Tokens/capabilities:** Verified in **ron-auth/svc-passport**, not here; ron-policy consumes already-validated capability claims/caveats in `ctx`.

* **Symmetric/Hash (Grover-affected only):**

  * **Hash/Fingerprint:** **BLAKE3-256** for bundle fingerprints in audit/metrics. Grover gives quadratic speedup → **effective ~128-bit** preimage security; still strong for fingerprints.

* **Data at rest / long-lived artifacts:**

  * **Policy bundles** (public, versioned, stored for years): *authenticity* matters, not confidentiality → **HNDL risk: low**.
  * **Decision logs/metrics** in consumers: should not contain secrets; **HNDL risk: low**.

* **Transport/Session lifetime:** N/A at library level (handled by transports). Typical sessions are short-lived → low HNDL.

* **Crate-specific blast radius:** If classical PKI is broken and consumers accept forged signatures, **malicious bundles** could be adopted fleet-wide. This is mitigated by **multi-sig (K-of-N)**, **trust-anchor rotation**, and **PQ signature acceptance**.

> **HNDL = Harvest-Now, Decrypt-Later** — not a primary concern for ron-policy (no secrets, focuses on integrity).

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use (indirect, via consumers):**

  * **KEX:** X25519 (TLS 1.3) in transports (outside this crate).
  * **Signatures:** Ed25519 for **bundle approvals**, capability tokens (macaroons) in auth.
  * **Symmetric/Hash:** AES-256 or ChaCha20-Poly1305 in transport; **BLAKE3-256** for fingerprints.

* **Libraries (typical in consumers):** tokio-rustls/rustls, ed25519-dalek, ring, blake3.

* **Key custody:** Governance signer keys live in **ron-kms/HSM** (no raw keys on disk). Rotation policy ≈ **90 days** with dual control.

* **Interfaces that carry crypto:**

  * **Bundle headers** (fingerprint, semver, nbf/exp, signer set, signatures).
  * **Capability tokens** verified by **ron-auth**; ron-policy consumes the resulting claims in `ctx`.

---

## 3) Target PQ Posture (Where we’re going)

* **Key exchange / transport encryption:** Done in transports; target **Hybrid KEX** (X25519 + ML-KEM/Kyber) when enabled by env/policy.
* **Signatures for bundle approval:** Accept **PQ signatures** (ML-DSA/Dilithium or SLH-DSA/SPHINCS+) in addition to Ed25519.
* **Tokens/capabilities:** PQ signature option exposed by **ron-auth**; ron-policy remains agnostic but can **require** “PQ-signed” claims by policy rule if configured.
* **Backcompat:** Classical remains supported through **M2 (Silver)**; **M3 (Gold)** defaults to **hybrid** acceptance for bundles. Pure-classical may be disabled via policy/config (“pq_only”).

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (library surface; verification occurs in consumers)
[features]
pq = []                # enable PQ type enums, algo labels, validation hooks
pq-sign = ["pq"]       # expose PQ signature algorithm IDs in bundle header schema
pq-require = ["pq"]    # compile-time guardrails to require PQ-signed bundles in tests
```

```ini
# Consumer-facing config knobs (validated before calling ron-policy)
pq_hybrid = true                 # transport layer (outside ron-policy); default off until M3
pq_sign_accept = ["ed25519","ml-dsa","slh-dsa"]  # accepted signer algos for bundles
pq_only = false                  # if true, refuse classical-signed bundles
key_rotation_days = 90
```

* **Interoperability switch:** If peer/fleet lacks PQ, **negotiate** (accept classical when `pq_only=false`) or **refuse** (when `pq_only=true`).
* **Metrics:** Always emit **algo labels** (value 0 when unused) for easy dashboard adoption.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Hooks & Assessment**

* Add `pq`/`pq-sign` feature gates and enum variants for signer algos in bundle headers (no behavior change).
* Document exposure; baseline perf for classical signature checks in consumers.
* Ensure tests compile with PQ features enabled (mock adapters OK).

**M2 (Silver) — Hybrid Enablement**

* Consumers: enable **Hybrid KEX** in transport stacks.
* Governance: allow **PQ signatures** for bundles (`ml-dsa` or `slh-dsa`) alongside Ed25519.
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (allowed when `pq_only=false`).
* Metrics: record signature algo per bundle; alert on downgraded acceptance when `pq_only=true`.

**M3 (Gold) — Default & Ops**

* Default **pq_hybrid=true** in app-facing planes.
* Require **PQ-signed bundles** for **prod** (flip `pq_only=true` via staged rollout).
* Runbooks include **rapid rollback** to hybrid-accept (keep PQ preferred).

**Post-M3 — De-risk & Deprecate**

* Sunsetting pure-classical bundle acceptance at edges; keep hybrid as default.
* Periodic PQ audit: lib updates, cert profiles, signer rotation, perf retests.

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical *approval* of policy bundles must not rely on **pure classical** when `pq_only=true`; allow **hybrid/PQ** signatures.
* **[PQ-I2]** Symmetric strength ≥ **256-bit**; hash ≥ **256-bit** (BLAKE3-256).
* **[PQ-I3]** All bundle headers include **fingerprint (BLAKE3)**, **semver**, **nbf/exp**, **signer set**; PQ algo is part of the signed header.
* **[PQ-I4]** Negotiation policy: if `pq_only=true`, classical-signed bundles are **rejected** with explicit error + audit.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback; **trust anchors** are rotated first fleet-wide.
* **[PQ-I6]** PQ builds pass CI; **interop parity** with classical (same Decisions for identical rules/ctx).

---

## 7) Observability (Metrics, Logs, Readiness)

Expose via **consumers** (gateway/index/etc.) with labels visible to auditors:

* `policy_bundle_loaded_total{algo="ed25519|ml-dsa|slh-dsa",fingerprint,quorum}`
* `policy_bundle_rejected_total{reason="bad-sig|expired|algo-not-allowed|not-pq-only|…"}`
* `crypto_latency_seconds{op="verify",algo}` (histogram; verify only at load/reload)
* Readiness rule: `/readyz` **fails** if policy mandates PQ (`pq_only=true`) and current bundle isn’t PQ-signed.
* Structured logs: include `pq={off|hybrid|pq-only}`, `bundle_algo`, `signers`, `fingerprint`.

---

## 8) Testing & Verification

* **Unit/property (this crate):**

  * Header schema round-trip (algo enum stable); determinism/lint unchanged with PQ flags.
  * Property: enabling PQ acceptance **does not change** evaluation semantics given identical bundle rules.

* **Interop (consumers + test harness):**

  * classical↔classical, hybrid↔hybrid, hybrid↔classical (allowed only when configured).
  * Negative tests: `pq_only=true` → classical-signed bundle **refused** with precise error.

* **Fuzz:**

  * Fuzz bundle header decoders with mixed/invalid algo IDs; ensure robust error taxonomy (no panics).

* **Load/Perf (consumers):**

  * Measure verify latency deltas (Ed25519 vs Dilithium/SPHINCS+). Expect **higher** verify costs; amortized by infrequent reloads.
  * Ensure **eval p95 < 1 ms** remains true (verification is not in the hot path).

* **Security drills:**

  * “Classical break” exercise: flip `pq_only=true` fleet-wide; verify clean failures + rollback guidance.

---

## 9) Risks & Mitigations

* **Library churn / supply chain:** PQ libs move fast. → Use **adapter traits** at the consumer verification layer; pin versions; audit crates.
* **Perf/size overhead:** PQ signatures are larger/slower. → Verification happens *on reload only*; cache acceptance; keep hot path unaffected.
* **Downgrade attacks:** Misconfig accepts classical when PQ mandated. → Central **policy knob** (`pq_only`) + alert on violation.
* **Interop gaps:** Some peers lack PQ. → Keep **hybrid** acceptance; staged rollout; clear operator UX and error messages.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL = low** for ron-policy.
* [ ] `pq` / `pq-sign` features compile; test matrix includes PQ flags.
* [ ] Consumers accept **PQ-signed bundles**; interop matrix green.
* [ ] Readiness fails (by config) when PQ is required but absent.
* [ ] PQ metrics/logs emitted with algo labels; dashboards updated.
* [ ] Runbook updated with **enable/rollback** steps; signer rotation rehearsed.
* [ ] Perf notes recorded (verify time deltas; eval p95 unchanged).
* [ ] SECURITY.md cross-links updated.

---

## 11) Role Presets (how ron-policy fits)

* **Identity/Policy/KMS (ron-kms, ron-auth, svc-passport, ron-policy, ron-audit):**

  * ron-auth handles **token PQ signatures**; ron-policy consumes verified claims.
  * ron-policy requires/accepts **PQ bundle signatures** based on config and exposes algo labels.
  * ron-kms provides custody/rotation; **multi-sig** uses PQ-capable signers.

* **Transport (ron-transport, svc-…):**

  * Enable **hybrid KEX**; unrelated to ron-policy runtime but part of end-to-end PQ posture.

* **Gateway/Index/Storage:**

  * Enforce `pq_only` where mandated; refuse classical bundles; surface metrics/logs.

---

## 12) Appendix

* **Algorithms chosen (initial):**

  * **KEX (transport):** X25519 + **ML-KEM (Kyber)** hybrid (when enabled).
  * **Signatures (bundles):** **ML-DSA (Dilithium)** preferred; **SLH-DSA (SPHINCS+)** optional fallback.
  * **Hash:** **BLAKE3-256** (fingerprints).

* **Libraries (illustrative, in consumers):**

  * tokio-rustls/rustls (TLS), blake3, ed25519-dalek, PQ adapter (e.g., liboqs/oqs-sys or vendor equivalents).

* **Interop notes:**

  * Mixed fleets allowed during M2; `pq_only=true` for prod in M3. Canary first; rollback safe.

* **Change log (PQ posture):**

  * 2025-Q4: Added PQ feature flags and header algo enums.
  * 2026-Q1: Consumers accept PQ-signed bundles (canary).
  * 2026-Q2: `pq_only=true` default in prod; hybrid transport enabled fleet-wide.

---

### Operator quick ref (consumer side)

* **Require PQ for bundles now:** set `pq_only=true`; reload; verify `/readyz` + `policy_bundle_loaded_total{algo!="ed25519"}` increases.
* **Rollback:** set `pq_only=false`; reload; accept classical while investigating signer/interop issues.

---
