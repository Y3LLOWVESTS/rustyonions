
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: YYYY-MM-DD
audience: contributors, security auditors, ops
crate: <crate-name>          # e.g., ron-kms
crate-type: lib|service|transport|econ|policy|kms|sdk|node
pillar: <1..12>              # which architectural pillar this crate belongs to
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose
Describe how this crate resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops.  
Scope covers: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and “harvest-now-decrypt-later” exposure.

---

## 1) Exposure Assessment (What’s at risk?)
- **Public-key usage (breakable by Shor):**
  - Key exchange: <X25519?|TLS1.3?>  
  - Signatures: <Ed25519?|JWT?|macaroons?>  
- **Symmetric/Hash (Grover-affected only):**
  - Ciphers/AEAD: <AES-256?|ChaCha20-Poly1305?>  
  - Hash: <BLAKE3-256?>  
- **Data at rest / long-lived artifacts:**
  - <ledger entries / mailbox payloads / manifests / caps>  
  - Retention window: <days/weeks/months/years> → **HNDL risk:** low/med/high
- **Transport/Session lifetime:** <seconds/minutes/hours> (shorter = lower HNDL risk)
- **Crate-specific blast radius:** One sentence on worst-case impact if classical PKI is broken.

> **HNDL = Harvest-Now, Decrypt-Later** risk: Can an adversary record now and decrypt in a PQ future?

---

## 2) Current Crypto Profile (Today)
- **Algorithms in use:**  
  - KEX: <X25519>  
  - Signatures: <Ed25519>  
  - Symmetric/Hash: <AES-256 / ChaCha20-Poly1305 / BLAKE3-256>  
- **Libraries:** <rustls / ring / ed25519-dalek / …>  
- **Key custody:** <ron-kms|file|env|HSM?>; rotation <N days>; backup policy <…>  
- **Interfaces that carry crypto:** APIs, frames, tokens, manifests, certificates.

---

## 3) Target PQ Posture (Where we’re going)
- **Key exchange / encryption (PQ or Hybrid):** <ML-KEM (Kyber) | Hybrid(X25519+ML-KEM)>  
- **Signatures:** <ML-DSA (Dilithium) | SLH-DSA (SPHINCS+)>  
- **Transport TLS:** <classical now> → enable **hybrid KEX** when `pq_hybrid = true`.  
- **Tokens/capabilities:** add PQ signature option and negotiate per-tenant policy.  
- **Backwards compatibility:** classical remains supported until **M3** (Gold), then default to hybrid.

---

## 4) Feature Flags & Config (How to turn it on)
```toml
# Cargo features (example)
[features]
pq = []              # enable PQ plumbing in this crate
pq-hybrid = ["pq"]   # use Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]     # enable PQ signatures (Dilithium/Sphincs+)
pq-only = []         # (optional) disable classical fallback at runtime/build

# Example dependency toggles (illustrative)
# liboqs-rust / oqs-sys crates or equivalent PQ libs would be feature-gated
```

```ini
# Config knobs (example, map to your crate Config)
pq_hybrid = true            # default off until M2; on by default in M3
pq_sign_algo = "ml-dsa"     # "ml-dsa" | "slh-dsa"
pq_only = false             # if true, refuse classical handshakes
key_rotation_days = 90
```

* **Interoperability switch:** if peer lacks PQ, behavior = \<negotiate | refuse when pq\_only=true>.
* **Metrics toggle:** always emit PQ labels even when disabled (value=0) for easy adoption.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze)** — Planning & Hooks

  * Add `pq` feature and config stubs (no behavior change).
  * Document exposure; baseline perf for classical.
  * Unit tests compile with PQ features (even if mocked).

* **M2 (Silver)** — Hybrid Enablement

  * Turn on **Hybrid KEX** in transports (`ron-transport`, `svc-arti-transport`).
  * Optional: **PQ signatures** behind `pq-sign` in `ron-auth` / `svc-passport`.
  * Interop tests: classical↔classical, hybrid↔hybrid, hybrid↔classical (when allowed).
  * Perf budget: target <10–20% handshake overhead; log actuals.

* **M3 (Gold)** — Default & Operationalization

  * Default **pq\_hybrid = true** for app-facing planes.
  * PQ signatures for **ledger receipts** / **caps** where policy requires it.
  * Add runbooks for incident handling and rollback (flip to classical if needed).
  * Docs: QUANTUM.md finalized; SECURITY.md updated.

* **Post-M3** — De-risking & De-precation

  * Add `pq_only` environments; begin sunsetting pure-classical in external edges.
  * Monitor ecosystem updates; plan periodic PQ re-evaluation.

---

## 6) Invariants (MUST)

* \[PQ-I1] No pure ECC/RSA in security-critical paths **unless** wrapped in **hybrid PQ**.
* \[PQ-I2] Symmetric keys **≥ 256-bit**; hashes **≥ 256-bit** (e.g., BLAKE3-256).
* \[PQ-I3] Long-lived data (ledger/mailbox/archives) MUST be re-encryptable under PQ keys.
* \[PQ-I4] Negotiation: if `pq_only = true`, refuse classical peers with a clear error.
* \[PQ-I5] Key rotation procedure upgrades algorithms without silent fallback.
* \[PQ-I6] PQ feature builds pass tests/CI; interop parity with classical is proven.

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with **algo labels**:

* `pq_handshake_total{algo="x25519",role}`, `{algo="ml-kem"}`, `{algo="hybrid"}`
* `pq_signature_total{algo}`, `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify",algo}` (histogram)
* **Readiness:** `/readyz` fails if policy requires PQ and peer/stack can’t negotiate it.
* **Structured logs:** include `pq={off|hybrid|pq-only}`, `algo=…`, `peer_mode=…`.

---

## 8) Testing & Verification

* **Unit / property tests:** frame parsers, token validators under PQ + classical.
* **Interop suite:** classical↔classical, hybrid↔hybrid, hybrid↔classical.
* **Fuzzing:** PQ decoders, negotiation paths, error taxonomy.
* **Load tests:** handshake/sec with and without PQ; ARM/edge profiles if applicable.
* **Security drills:** simulate “classical break” (force pq\_only) and verify safe failure.

---

## 9) Risks & Mitigations

* **Perf & footprint:** larger keys/certs and slower handshakes → cache handshakes; reuse sessions.
* **Library churn:** PQ libs evolve → isolate via a thin adapter trait; pin versions per workspace.
* **Classical fallback abuse:** enforce `pq_only` where mandated; log and alert on downgraded sessions.
* **E2E ecosystem gaps:** keep PQ optional until interop coverage is proven; document peer requirements.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL risk labeled (low/med/high).
* [ ] `pq` features compile; CI matrix includes `--features pq,pq-hybrid,pq-sign`.
* [ ] Hybrid KEX interop passes; clear errors on mismatch.
* [ ] PQ metrics emitted; dashboards updated.
* [ ] Runbook updated with enable/rollback steps.
* [ ] Perf numbers recorded (handshake, sign/verify, RAM/CPU).
* [ ] SECURITY.md cross-links updated; owners ack.

---

## 11) Role Presets (fill quickly per crate)

> Use these as drop-ins for **Primary Targets** and **Defaults**.

### kernel/lib (`ron-kernel`, `ron-bus`, `ron-metrics`, `ron-proto`)

* **Primary Targets:** PQ-agnostic core; expose flags & types but avoid crypto binding.
* **Defaults:** `pq=false`; provide traits for transports/tokens to plug PQ.

### transport (`ron-transport`, `svc-arti-transport`)

* **Primary Targets:** **Hybrid KEX** first; TLS/KEX adapters; negotiation policy.
* **Defaults:** `pq_hybrid=false (M1) → true (M3)`; `pq_only=false`.

### identity/policy/kms (`ron-kms`, `ron-auth`, `svc-passport`, `ron-policy`, `ron-audit`)

* **Primary Targets:** **PQ signatures** option; custody & rotation; audit coverage.
* **Defaults:** `pq_sign=false (M1) → optional (M2/M3)`; `key_rotation_days=90`.

### storage/index/mailbox (`svc-storage`, `svc-index`, `svc-overlay`, `svc-mailbox`)

* **Primary Targets:** HNDL mitigation (encrypt long-lived at rest with PQ-ready envelopes).
* **Defaults:** classical transit OK in M1; add PQ at rest hooks; document retention.

### gateway/omnigate/nodes (`svc-gateway`, `omnigate`, `macronode`, `micronode`)

* **Primary Targets:** policy-driven negotiation; `/readyz` + metrics; easy rollback.
* **Defaults:** `pq_hybrid=true` **only** when upstreams support it; otherwise log+allow.

### econ (`ron-ledger`, `ron-accounting`, `svc-wallet`, `svc-rewarder`, `svc-ads`)

* **Primary Targets:** PQ sign for receipts; optional PQ-aware ZK hooks.
* **Defaults:** classical OK until partners support PQ; add adapter traits now.

### sdk (`ron-app-sdk`, `oap`)

* **Primary Targets:** client-side negotiation; clear errors; sample code for hybrid.
* **Defaults:** keep classical examples; add PQ example behind feature flag.

---

## 12) Appendix (fill as you adopt)

* **Algorithms chosen:** KEX=<…>, SIG=<…> (and why).
* **Libraries:** \<lib names & versions>; audit notes.
* **Interop notes:** peer stacks supported, fallback policy.
* **Change log:** date → change (enable hybrid by default, add pq\_only for env X, etc.)


---

### How to use
- Drop this file into `crates/<name>/docs/QUANTUM.md`.
- Fill just **sections 1, 2, 11, and 12** first (10–15 minutes per crate).
- For transport/identity/econ crates, also fill **sections 4–7** now; others can inherit presets.

