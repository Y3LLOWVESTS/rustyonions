````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-12
audience: contributors, security auditors, ops
crate: svc-mailbox
crate-type: service
pillar: 11              # Feed & Messaging
owners: [Stevan White]
---

# QUANTUM.md — svc-mailbox

## 0) Purpose
Describe how `svc-mailbox` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop, SLOs, or ops.  
Scope: transport handshakes, capability verification, where keys live, amnesia vs persistent profiles, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

**Role:** store-and-forward messaging (SEND/RECV/ACK with visibility + DLQ). Mailbox is **crypto-neutral** by design, but it sits on crypto-bearing edges (transport, capability auth).

### Public-key usage (Shor-breakable)
- **Transport KEX:** TLS 1.3 (rustls) with **X25519** today (ingress behind gateway or direct in dev).  
- **Signatures (tokens):** capabilities issued/verified via **Ed25519** (svc-passport/ron-auth). Mailbox consumes KID & verifies via trusted path; it should not hold private keys.

### Symmetric/Hash (Grover-affected only)
- **AEAD:** AES-256-GCM and/or ChaCha20-Poly1305 (symmetric safe at 256-bit level).  
- **Hashing:** BLAKE3-256 (collision/PRF usage; fine under Grover with 256-bit security).

### Data at rest / long-lived artifacts
- **Micronode (amnesia):** RAM-only queues → **HNDL risk: Low** (no long-term ciphertext to harvest).  
- **Macronode (persistent queues/DLQ, optional):** message bodies/metadata may persist per retention/TTL → **HNDL risk: Medium** (if captured now over classical KEX, future decryption of sessions could reveal transit; at rest is symmetric—OK if DEKs remain secret).  
- **Caps/Manifests:** short-lived JWT/macaroons-style tokens (minutes to hours) → **Low HNDL** if not logged.

### Transport/session lifetime
- Typical sessions: seconds–minutes; short lifetime reduces HNDL value (but still upgrade).

### Worst-case blast radius if classical PKI breaks
- An on-path adversary could **retro-decrypt recorded TLS traffic** (ingress payloads) and **forge capabilities** if they obtain issuer SKs (outside mailbox). Idempotency + audit trails bound economic damage, but **confidentiality** of message bodies in transit is at risk.

> **HNDL = Harvest-Now, Decrypt-Later**: record ciphertext today, decrypt after PQ computers arrive.

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use**
  - **KEX/Transport:** TLS 1.3 with **X25519**.
  - **Signatures (caps):** **Ed25519** (issued by svc-passport/ron-auth; mailbox verifies via KID path).
  - **Symmetric/Hash:** **AES-256-GCM** / **ChaCha20-Poly1305**, **BLAKE3-256**.

- **Libraries**
  - **tokio-rustls/rustls**, **ed25519-dalek** (via auth path), standard Rust crypto crates.
  - Mailbox avoids bespoke crypto; it depends on upstream identity/transport crates.

- **Key custody**
  - **ron-kms/HSM** custody for signing and KEK material; mailbox holds **no private keys**. It caches **KID → verifier** and policy. Rotation ≤ **90 days** (target).

- **Interfaces carrying crypto**
  - TLS endpoints (ingress), capability headers on API calls, governance signatures (indirect).

---

## 3) Target PQ Posture (Where we’re going)

- **Transport (KEX/Enc):** **Hybrid** TLS handshakes: **X25519 + ML-KEM (Kyber)** when `pq_hybrid = true`.  
- **Signatures:** Capability and governance signatures support **ML-DSA (Dilithium)** (primary) with **SLH-DSA/SPHINCS+** as conservative fallback (latency/size trade).  
- **Tokens/Capabilities:** Dual-sig support (Ed25519 || ML-DSA) during transition; mailbox accepts either per policy.  
- **Back-compat:** Classical remains supported through **M2 (Silver)**; **M3 (Gold)** defaults to **hybrid** at edges under policy.  
- **At Rest (Macronode):** Optional PQ-ready envelope for persistent payloads: DEK = XChaCha20-Poly1305/AES-256; **DEK wrapped** with **ML-KEM** (plus legacy KEK) for re-encryption without re-ingest.

---

## 4) Feature Flags & Config (How to turn it on)

### Cargo features (mailbox surface; transport/auth crates implement heavy lifting)
```toml
[features]
pq = []                # enable PQ labels/types plumbing
pq-hybrid = ["pq"]     # prefer Hybrid KEX (X25519 + ML-KEM) when peer supports
pq-sign = ["pq"]       # accept PQ-signed capabilities (ML-DSA/Sphincs+)
pq-only = []           # refuse classical peers/tokens (ops gate, M3+ selective)
````

### Config knobs (svc-mailbox Config)

```ini
# Post-Quantum posture
pq_hybrid = false          # M2: opt-in; M3: default true (edges only if supported)
pq_sign_algo = "ml-dsa"    # "ml-dsa" | "slh-dsa" | "off" (accept classical only)
pq_only = false            # if true, reject classical transport/tokens
pq_metrics = true          # always emit algorithm-labelled metrics

# Custody/rotation expectations (enforced via ron-kms)
key_rotation_days = 90
cap_dual_kid_grace_days = 14
```

**Interoperability:**

* If peer lacks PQ: **negotiate down** unless `pq_only=true` (then clear error + metrics).
* PQ labels are emitted even when disabled (value 0) to ease rollout visibility.

---

## 5) Migration Plan (Milestones)

**M1 — Bronze (Hooks & Inventory)**

* Add `pq` features + config stubs; no behavior change.
* Inventory HNDL exposure by profile (amnesia vs persistent).
* Baseline perf: TLS handshake & cap verify cost on target SKUs.
* CI matrix builds with `--features pq,pq-hybrid,pq-sign`.

**M2 — Silver (Hybrid Enablement)**

* Enable **Hybrid KEX** in gateway/transport; mailbox honors policy & metrics.
* Accept **PQ-signed** capabilities (dual-format), with verifier lookup via **KID**.
* Interop tests: classical↔classical, hybrid↔hybrid, hybrid↔classical.
* Record perf deltas; aim < **10–20%** handshake overhead.

**M3 — Gold (Default Hybrid & Ops)**

* Default `pq_hybrid=true` where upstream supports; keep per-tenant overrides.
* Enforce `pq_only` selectively for sensitive tenants/paths.
* Optional at-rest DEK-wrap (Macronode) for long-lived queues/DLQ; validate re-encrypt-in-place tooling.
* Runbook updated with enable/rollback; dashboards live.

**Post-M3 — Sunsetting Classical**

* Begin deprecating pure-classical edges; publish schedule.
* Annual PQ re-evaluation; bump library versions; refresh test vectors.

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path remains **pure classical** once `pq_hybrid=true`.
* **[PQ-I2]** Symmetric strength ≥ **256-bit**; hashes ≥ **256-bit**.
* **[PQ-I3]** If `pq_only=true`, classical handshakes/tokens are **refused with explicit error** and metrics.
* **[PQ-I4]** Dual-KID rotation: mailbox must verify **either** of (legacy, PQ) during grace; no silent downgrade.
* **[PQ-I5]** Persistent data (Macronode) is **re-encryptable** with PQ-wrapped DEKs without body re-hydration.
* **[PQ-I6]** PQ builds pass unit/integration/property/fuzz; interop parity proven.

---

## 7) Observability (Metrics, Logs, Readiness)

Expose algo-labelled counters/histograms:

* `pq_handshake_total{algo="x25519|ml-kem|hybrid", result}`
* `pq_signature_verify_total{algo="ed25519|ml-dsa|slh-dsa", result}`
* `crypto_latency_seconds_bucket{op="kex|sign|verify", algo}`
* `pq_sessions_downgraded_total{peer="name", reason}`
* `pq_only_refusals_total{surface="transport|token"}`

**Readiness policy:**

* `/readyz` fails if policy mandates `pq_only` and peers cannot negotiate PQ.
* Structured logs always include: `pq_mode={off|hybrid|pq-only}`, `algos={...}`, `peer_mode={...}`, `kid={...}` (no secrets).

---

## 8) Testing & Verification

* **Unit/Property:** DTO strictness unaffected by PQ; negotiation state machine (classical↔hybrid↔pq-only) is total & idempotent.
* **Interop:** matrix across client/server with {classical, hybrid, pq-only}; negative tests for mismatch & downgrade attempts.
* **Fuzz:** token/parsers & negotiation branches; malformed PQ caps must count to `integrity_fail_total`.
* **Load:** handshake/sec and verify/sec at realistic concurrencies; ARM/edge profiles for Micronode.
* **Security drills:** simulate “classical break” → flip `pq_only=true`; ensure safe refusal, correct alerting, and documented rollback.

---

## 9) Risks & Mitigations

| Risk                              | Impact                        | Mitigation                                                                                |
| --------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------------- |
| Handshake perf & larger certs     | Elevated p95 on cold paths    | Session resumption; keep-alive; pre-warm pools; quantify deltas; autoscale thresholds     |
| Library churn & CVEs in PQ stacks | Stability & supply chain      | Adapter traits; version pinning at workspace; SBOM + cargo-deny; staged rollouts          |
| Downgrade attacks                 | Loss of PQ guarantees         | Strict policy gates; `pq_only` where mandated; emit `pq_sessions_downgraded_total` alerts |
| Ecosystem partial support         | Interop failures              | Tenant/edge allowlists; gradual enablement; detailed errors & fallbacks                   |
| Operator error (misconfig)        | Accidental classical fallback | Config lint + `/readyz` policy check; CI policy tests; canary environments                |

---

## 10) Acceptance Checklist (DoD)

* [ ] HNDL exposure labeled **(Micronode: Low / Macronode: Medium)**.
* [ ] `pq`, `pq-hybrid`, `pq-sign` compile & are exercised in CI.
* [ ] Interop suite passes for classical/hybrid; clear refusal on `pq_only=true`.
* [ ] PQ metrics live; dashboards & alerts wired.
* [ ] Runbook updated with enable/rollback, failure modes.
* [ ] Perf deltas recorded (handshake, verify, memory/CPU) on reference nodes.
* [ ] SECURITY.md cross-links updated; owners ACK.

---

## 11) Role Presets (cross-crate alignment)

* **Transport (`ron-transport`, edge/gateway):** implement **Hybrid KEX**, negotiation, and `pq_only` enforcement.
* **Identity/Policy/KMS (`ron-kms`, `svc-passport`, `ron-auth`, `ron-policy`):** issue/verify **ML-DSA** caps; dual-KID rotation; audit trails.
* **svc-mailbox (this crate):** enforce posture via config; verify tokens via KID with PQ support; expose PQ telemetry; at-rest DEK-wrap hooks for Macronode.
* **Micronode/Macronode:** amnesia ON (no at rest); Macronode optional PQ DEK-wrap + re-encrypt-in-place tools.

---

## 12) Appendix (fill as we adopt)

**Algorithms (initial picks):**

* **KEX:** Hybrid X25519 + **ML-KEM (Kyber)**
* **SIG:** **ML-DSA (Dilithium)** primary; **SLH-DSA (SPHINCS+)** optional conservative profile
* **AEAD/Hash:** AES-256-GCM / ChaCha20-Poly1305; BLAKE3-256

**Libraries (subject to pinning):**

* `tokio-rustls` (hybrid ciphersuites via upstream once stabilized)
* PQ adapters (e.g., liboqs-backed or equivalent) hidden behind traits in transport/auth crates

**Interop Notes:**

* Peers advertise PQ via ALPN/extension; mailbox logs peer posture and decisions.
* Tenants may opt-in to `pq_only` once their SDKs are upgraded.

**Change Log (QUANTUM posture):**

* 2025-10-12: Baseline QUANTUM.md; M1 hooks defined.

```
```
