

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, security auditors, ops
crate: svc-index
crate-type: service
pillar: 9            # Pillar 9 — Naming & Index / Resolver Plane
owners: [Stevan White]

# QUANTUM.md — `svc-index`

## 0) Purpose

Describe how `svc-index` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop, ops, or auditability.
Scope: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and **Harvest-Now-Decrypt-Later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

**Where public-key crypto touches `svc-index`:**

* **Transport (HTTP/TLS 1.3)**: client→service; optionally service→internal peers if not fronted by gateway.
* **Snapshot / audit signatures**: signed snapshot anchors; governance/audit events.
* **Capability tokens**: macaroons (HMAC-auth, PQ-neutral) and optional Ed25519 attenuation wrappers.
* **DHT client**: peer auth/handshake (via `ron-transport`/overlay where applicable).

**Public-key usage (Shor-breakable today):**

* **Key exchange:** X25519 (TLS 1.3 ECDHE) when `svc-index` terminates TLS.
* **Signatures:** Ed25519 (snapshot anchors, optional token wrappers, governance signers).

**Symmetric / hash (Grover-affected only):**

* **AEAD:** AES-256-GCM and/or ChaCha20-Poly1305.
* **Hash:** BLAKE3-256 (content addressing, snapshot hashing).

**Data at rest / long-lived artifacts:**

* **Index DB (sled) metadata**, **snapshot journals**, **audit stream** (append-only).
  Retention: months→years (for audit/snapshots) → **HNDL risk = medium** (authenticity risk if signatures later forged; confidentiality typically low because index outputs are public facts).

**Transport/session lifetime:** seconds (HTTP); **short**, hence **low HNDL** on confidentiality of sessions recorded today. Authenticity over time is the main concern.

**Worst-case blast radius if classical PKI is broken:**

* Forged **snapshot signatures** or governance approvals could mislead downstream consumers/rewarders unless **dual-signed** (classical+PQ) and independently verified. No value transfer happens in `svc-index`, but integrity signals could be subverted—hence we adopt **dual-signing** for anchors.

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use:**

  * **KEX:** X25519 (TLS 1.3 ECDHE) if TLS is terminated in-process.
  * **Signatures:** Ed25519 (snapshot anchors, optional token wrappers).
  * **Symmetric/Hash:** AES-256-GCM / ChaCha20-Poly1305; BLAKE3-256.
* **Libraries:** tokio-rustls (rustls), ed25519-dalek, blake3, ring/aead (via deps).
* **Key custody:** snapshot/governance keys in **ron-kms/HSM**; rotation 90 days; no raw keys in env/files.
* **Crypto-carrying interfaces:** HTTP/TLS, audit/snapshot signatures, optional capability wrappers.

---

## 3) Target PQ Posture (Where we’re going)

* **KEX / transport encryption:** **Hybrid** KEX = X25519 + **ML-KEM (Kyber)** when `pq_hybrid = true` (fronted by gateway or in-proc TLS).
* **Signatures for anchors/tokens:** **ML-DSA (Dilithium)** preferred; **SLH-DSA (SPHINCS+)** optional for stateless fallback.
* **Snapshot anchors:** **Dual-signed** (Ed25519 + ML-DSA) during migration; PQ-only later for internal validation while keeping classical for legacy verifiers until deprecation.
* **Tokens/capabilities:** macaroons remain HMAC-based (PQ-neutral). Optional PQ wrapper signatures available per tenant/policy.
* **Backwards compatibility:** classical supported until **M3**; then default to **hybrid**; legacy classical gated behind policy.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (scoped to this crate, adapters live in ron-transport/ron-kms)
[features]
pq = []                  # enable PQ plumbing in this crate (adapters/labels)
pq-hybrid = ["pq"]       # allow Hybrid KEX (X25519 + ML-KEM) at TLS layer (if we terminate TLS)
pq-sign = ["pq"]         # allow PQ signatures (Dilithium/SPHINCS+) for snapshots/audit
pq-only = []             # (optional) reject classical peers/anchors (strict)
```

```ini
# svc-index Config (subset)
[pq]
pq_hybrid = false        # M1 default; true by default at M3
pq_sign_algo = "ml-dsa"  # "ml-dsa" | "slh-dsa"
pq_dual_sign = true      # dual-sign snapshots during migration
pq_only = false          # if true, refuse classical negotiation & classical-only anchors

[keys]
rotation_days = 90       # enforced by ron-kms; rotate signer set by policy/quorum
```

**Interoperability:** if peer lacks PQ, behavior = negotiate classical unless `pq_only=true`.
**Metrics:** always emit PQ-related metrics/labels (value=0 when disabled) for easy onboarding.

---

## 5) Migration Plan (Milestones)

**M1 — Bronze (Hooks & Baselines)**

* Add `pq` feature flags; no behavior change by default.
* Document exposure + HNDL; capture classical perf baselines (handshake, sign/verify).
* CI builds PQ variants; snapshot pipeline accepts **dual-sign** config but keeps classical default.

**M2 — Silver (Hybrid Enablement + Dual-Sign)**

* Enable **Hybrid KEX** where we terminate TLS or via gateway that supports it.
* **Dual-sign** snapshots (Ed25519 + ML-DSA).
* Interop tests: classical↔classical, hybrid↔hybrid, hybrid↔classical (when policy allows).
* Perf budget: **< 10–20%** handshake overhead; record and publish actuals.

**M3 — Gold (Default Hybrid + Policy Rollout)**

* Default `pq_hybrid=true` for app-facing planes; maintain classical fallback only where peers lag.
* Require PQ signature verification for internal consumers (e.g., rewarder), keep dual-sign for external legacy until sunset.
* Runbook entries for quick rollback to classical (flip `pq_hybrid=false`) with audit reason.

**Post-M3 — Deprecation & Hardening**

* Introduce `pq_only=true` environments (internal, high-assurance).
* Rotate signer sets to PQ-first; remove classical signatures where all dependents attest PQ support.
* Annual PQ posture review (libs, parameters, perf).

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical paths (snapshot anchoring, governance approvals, transport to clients when we terminate TLS) MUST be **hybrid or PQ-signed** by M3.
* **[PQ-I2]** Symmetric crypto ≥ 256-bit; hash ≥ 256-bit; content addressing remains **BLAKE3-256**.
* **[PQ-I3]** Long-lived authenticity artifacts (snapshots/audit) MUST be **dual-signed** during migration; verifiers reject unsigned or single-classical if policy requires.
* **[PQ-I4]** If `pq_only=true`, classical negotiation or classical-only anchors are rejected with clear, observable errors.
* **[PQ-I5]** Key rotation NEVER silently downgrades algo strength; rotation plan upgrades signatures without trust gaps (overlap windows).
* **[PQ-I6]** PQ builds pass CI; interop parity with classical proven for all supported client SDKs.

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics (examples):**

* `pq_handshake_total{algo="x25519|ml-kem|hybrid",role="server|client"}`
* `pq_handshake_errors_total{reason}`
* `pq_signature_total{algo="ed25519|ml-dsa|slh-dsa",op="sign|verify"}`
* `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify",algo}` (histogram)
* `pq_enabled{mode="off|hybrid|pq-only"}` gauge

**Readiness:** `/readyz` **fails** when policy requires PQ (e.g., `pq_only=true`) and peer/stack cannot negotiate; reasons are enumerated.

**Logs:** structured fields: `pq_mode`, `kex_algo`, `sig_algos=["ed25519","ml-dsa"]`, `peer_mode`, `downgrade_detected=true|false`.

---

## 8) Testing & Verification

* **Unit/Property:**

  * Snapshot dual-sign/verify (order-independent, stable).
  * Token validators under classical+PQ wrappers; no panics on arbitrary inputs.
  * Negotiation matrix state machine (classical/hybrid/pq-only).

* **Interop:**

  * classical↔classical (control), hybrid↔hybrid, hybrid↔classical (allowed), pq-only↔classical (must fail clearly).
  * Client SDKs: Rust/TS/Python parity.

* **Fuzz:**

  * Signature decoders, snapshot record parser, negotiation TLVs.
  * Error taxonomy must remain stable.

* **Load/Perf:**

  * Handshake/sec with/without hybrid; sign/verify throughput for Ed25519 vs ML-DSA.
  * Edge ARM profile (Micronode) vs x86_64 (Macronode).

* **Security Drills:**

  * “Classical break” simulation → force `pq_only=true`; ensure clean refusal paths and truthful `/readyz`.
  * Key rotation with overlapping signer sets; verify no verification gaps.

---

## 9) Risks & Mitigations

| Risk                               | Impact                      | Mitigation                                                                                         |
| ---------------------------------- | --------------------------- | -------------------------------------------------------------------------------------------------- |
| PQ libs evolve / API churn         | Build breaks; interop drift | **Adapter trait** around KEX/sign; pin versions; CI matrix with `pq*` features                     |
| Larger keys/certs → handshake cost | Latency/CPU ↑               | Session resumption; keep-alive; tune timeouts; document overhead; scale guidance in PERF           |
| Downgrade attacks to classical     | Lose PQ benefit             | Mode flag + metrics; **reject** when `pq_only`; alert on downgraded sessions                       |
| Dual-sign complexity               | Operational mistakes        | Single codepath for signing that always includes classical+PQ during M2/M3; centralized KMS policy |
| Ecosystem gaps (some peers lag)    | Interop breaks              | Default hybrid with fallback; per-tenant policy; runway before deprecating classical               |

---

## 10) Acceptance Checklist (DoD)

* [ ] HNDL exposure labeled **(medium authenticity, low confidentiality)**; documented.
* [ ] `pq`, `pq-hybrid`, `pq-sign` features compile; CI includes these matrices.
* [ ] Hybrid KEX interop tests pass; pq-only refusal path observable.
* [ ] Snapshots **dual-signed**; verifiers accept PQ; metrics/labels emitted.
* [ ] Runbook updated (enable/rollback steps, failure modes).
* [ ] Perf diffs recorded for handshake/sign/verify across archs.
* [ ] SECURITY.md cross-links; KMS rotation play validated.

---

## 11) Role Presets (quick guidance)

* **This crate (svc-index | service | Pillar 9):**
  Primary targets: **dual-signed snapshots**, **hybrid KEX** where TLS is terminated, **pq_only** policy support, and **readiness truth** tied to PQ posture. Defaults: `pq=false`, `pq_hybrid=false (M1) → true (M3)`, `pq_dual_sign=true (M2+)`.

* **Transports (ron-transport/edge gateways):**
  Own hybrid KEX; expose negotiation to upstreams; emit `pq_*` metrics.

* **KMS / Identity (ron-kms, ron-auth):**
  Manage signer sets; dual-sign service; rotation with overlap; PQ audit anchors.

* **Rewarder / Ledger (consumers):**
  Require **snapshot hash + PQ verification** before attribution; keep classical acceptance until sunset date.

---

## 12) Appendix (to fill as adopted)

**Chosen algorithms (initial):**

* **KEX:** Hybrid(X25519 + ML-KEM).
* **Signatures:** Dual(Ed25519 + ML-DSA).
* **Hash:** BLAKE3-256.
* **AEAD:** AES-256-GCM or ChaCha20-Poly1305.

**Libraries (pin & audit):**

* `tokio-rustls` (TLS), `ed25519-dalek` (classical sig), **PQ adapter** (via liboqs-style bindings or provider-native hybrid), `blake3`.
* Notes: keep PQ behind adapter traits; prefer provider-maintained hybrids when available.

**Interop notes:**

* Gateways may terminate TLS; enable hybrid there first; `svc-index` still dual-signs anchors.
* Policy per tenant controls pq_only.

**Change log:**

* 2025-Q4: Add `pq*` features + dual-sign plumbing.
* 2026-Q1: Hybrid on by default (M3) for app-facing surfaces; pq_only pilots in staging.
* 2026-Q2: Begin classical sunset for internal verifiers.

---

**Operator note:** Treat PQ posture as **gradual hardening of authenticity** (dual-sign) plus **negotiated confidentiality** (hybrid KEX). Keep observability first: if PQ is required but missing, `/readyz` must be false with a crisp reason, and dashboards should show **pq_mode**, **downgrade events**, and **signature coverage**.
