

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-08
audience: contributors, security auditors, ops
crate: ron-audit
crate-type: lib
pillar: 4                      # Integrity & Evidence (audit plane)
owners: [Stevan White]

# QUANTUM.md

## 0) Purpose

Define how `ron-audit` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** signatures without breaking interop, reproducibility, or ops. Scope: algorithms, custody, config flags, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** authenticity exposure.

> Governance note: enabling PQ by default or enforcing `pq_only=true` requires a governance proposal per **GOVERNANCE.md §4**.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (Shor-breakable):**

  * **Signatures:** Ed25519 on each audit record and checkpoint/manifest.
  * **KEX/transport:** N/A inside this crate (hosts handle transports).
* **Symmetric / Hash (Grover-affected):**

  * **Hash:** BLAKE3-256 for `self_hash` and chain linkage (≥256-bit).
  * **AEAD:** N/A here (no encryption in this crate).
* **Data at rest / long-lived artifacts:** exported audit records, manifests, checkpoints (retention: **years**).
  **HNDL risk:** Confidentiality low (not encrypted); **Authenticity high** (future PQ adversary could forge classical signatures unless mitigated).
* **Blast radius statement:** If classical sigs become forgeable and verifiers accept them, an attacker could mint alternative histories/checkpoints. Hybrid/PQ signatures and cross-sign rotations prevent acceptance of such forgeries by PQ-aware verifiers.

---

## 2) Current Crypto Profile (Today)

* **Algorithms:** Ed25519 (sign), BLAKE3-256 (hash).
* **Libraries:** `ed25519-dalek` v2, `blake3`. (Hosts may use `rustls`/`ring` externally.)
* **Key custody:** KMS/HSM (host), rotation **90 days** or on incident; rotations are **cross-signed** in the audit stream.
* **Crypto-carrying interfaces:** record signature envelope; checkpoint/manifest signature.

---

## 3) Target PQ Posture (Where we’re going)

* **Signatures:** Hybrid **dual-sign** (`Ed25519 + ML-DSA/Dilithium-2`) per record and checkpoint; optional **SLH-DSA/SPHINCS+** conservative path.
* **PQ-only mode:** Available via policy once ecosystem supports it.
* **Transport TLS:** Out of scope here; hosts move to **hybrid KEX (X25519+ML-KEM/Kyber)** per their QUANTUM.md.
* **Back-compat:** Classical supported through **M2 (Silver)**; **M3 (Gold)** defaults to hybrid.

### 3.1 Signature Algorithms (reference)

| Algo                    | Security  | Sig size (approx) | Pubkey size | Notes                                           |
| ----------------------- | --------- | ----------------- | ----------- | ----------------------------------------------- |
| Ed25519                 | Classical | 64 B              | 32 B        | Fast; Shor-vulnerable long-term                 |
| ML-DSA (Dilithium-2)    | PQ        | ~2.4–2.7 KB       | ~1.3 KB     | NIST primary; good perf/size tradeoff           |
| SLH-DSA (SPHINCS+-128s) | PQ        | ~8–17 KB          | 32 B        | Stateless hash-based; conservative, larger sigs |

*(Sizes are ballpark; use crate docs/bench outputs for exacts.)*

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (crate)
[features]
pq = []                 # compile PQ plumbing (envelopes, traits)
pq-sign = ["pq"]        # emit PQ signatures (per record/manifest)
pq-hybrid = ["pq"]      # dual-sign (Ed25519 + PQ)
pq-only = ["pq-sign"]   # forbid classical sign/verify paths at runtime gates
sphincs = ["pq-sign"]   # optional PQ alternative (SLH-DSA)
```

```ini
# Host/Config knobs (mapped via ron-audit public API)
audit.pq_hybrid = true            # dual-sign on
audit.pq_sign_algo = "ml-dsa"     # "ml-dsa" | "slh-dsa"
audit.pq_only = false             # if true, refuse classical peers+artifacts
audit.key_rotation_days = 90
```

* **Interoperability:** If peer lacks PQ verify and `pq_only=false`, accept classical while logging downgrade; else fail.
* **Metrics:** Always emit PQ-labeled metrics (0 when off) to ease adoption tracking.

---

## 5) Migration Plan (Milestones)

### 5.1 Roadmap (scan)

| Milestone       | Goals                                                                                   | Canon tie-ins             | Exit criteria                                             |
| --------------- | --------------------------------------------------------------------------------------- | ------------------------- | --------------------------------------------------------- |
| **M1 (Bronze)** | Add flags; introduce **signature envelope** (multi-algo); baseline perf/size            | TESTS, PERFORMANCE        | CI builds with `--features pq*`; no behavior change       |
| **M2 (Silver)** | **Dual-sign opt-in**; interop matrix (classical↔/hybrid↔); telemetry for downgrades     | RUNBOOK, OBS, CONCURRENCY | Hybrid signing stable; perf deltas recorded; alerts wired |
| **M3 (Gold)**   | **Hybrid default**; archive **checkpoint co-sign backfill** tool; runbooks for rollback | GOVERNANCE §4, SECURITY   | PQ verify parity; default enabled; backfill tested        |
| **Post-M3**     | PQ-only tenants; deprecate classical acceptance                                         | GOVERNANCE                | Policies documented; annual NIST re-eval                  |

### 5.2 Flow (visual)

```mermaid
flowchart LR
  A[Classical-only] --> B[Add envelopes & PQ verify]
  B --> C[Dual-sign opt-in (hybrid)]
  C --> D[Default hybrid (M3)]
  D --> E[PQ-only per policy]
```

---

## 6) Invariants (MUST)

| ID        | Invariant                                                                                 | Enforcement                                                      |
| --------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| **PQ-I1** | When `pq_hybrid=true`, every record & checkpoint is **dual-signed**                       | Sign path asserts PQ present; tests check envelope contains both |
| **PQ-I2** | When `pq_only=true`, classical **not emitted/accepted**; readiness fails if peers lack PQ | Gate in sign/verify; `/readyz` policy check                      |
| **PQ-I3** | Hash stays **BLAKE3-256** (≥256-bit)                                                      | API/impl constant; tests                                         |
| **PQ-I4** | **Key rotations are cross-signed** in stream and over checkpoints                         | Rotation procedure + tests                                       |
| **PQ-I5** | Verifiers record **which algo** validated                                                 | Metrics/log labels + tests                                       |
| **PQ-I6** | CI passes classical, hybrid, and PQ-only builds & test suites                             | CI matrix                                                        |
| **PQ-I7** | Historical **checkpoints** can be **co-signed** without altering record payloads          | Backfill tool + verification tests                               |

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics (with `{algo}` labels)**

* `audit_sign_total{algo="ed25519|ml-dsa|slh-dsa"}`
* `audit_verify_total{algo}`
* `audit_verify_fail_total{algo,reason}`
* `audit_pq_missing_verify_total` (verification done but only classical path used)
* `crypto_latency_seconds{op="sign|verify",algo}` (histogram)

**Alerts (PromQL examples)**

* PQ missing too often (adoption lag):
  `sum(rate(audit_pq_missing_verify_total[15m])) / clamp_min(sum(rate(audit_verify_total[15m])),1) > 0.10`
* PQ verify failures:
  `sum(rate(audit_verify_fail_total{algo!="ed25519"}[5m])) > 0`

**Readiness**

* Fail when policy requires PQ (e.g., `pq_only=true`) and PQ stacks/peers aren’t available.

**Logs (sample)**

```
level=INFO pq_mode=hybrid sign_algos=[ed25519,ml-dsa] verify_algos=[ml-dsa,ed25519] pq_only=false
```

---

## 8) Testing & Verification

* **Unit/property**

  * Signature envelope is deterministic; verification succeeds if **any** valid signature present; order-independent.
  * Canonical bytes invariant unchanged under hybrid/PQ signing.
* **Interop suite**

  * classical↔classical; hybrid↔hybrid; hybrid emitter ↔ classical verifier (accept classical unless `pq_only`).
* **Fuzz**

  * Envelope decoders (malformed/mixed algos); stripped-sig detection; duplicate/forged combos.
* **Loom (concurrency)**

  * Dual-sign path quiesces on shutdown; no torn states between classical and PQ signer tasks.
* **Perf / load**

  * Append p50/p95; verify throughput across Ed25519, ML-DSA, and dual-sign; update **PERFORMANCE.md** baselines.
* **Archive upgrader**

  * Backfill PQ co-signatures for **checkpoint manifests** (not individual records) preserving Merkle roots; PQ-only verification then passes.
* **Security drills**

  * Simulate “classical break”: set `pq_only=true` in staging; ensure classical verifiers fail cleanly; hybrid clusters continue.

---

## 9) Risks & Mitigations

| Risk                               | Impact                              | Mitigation                                                                                |
| ---------------------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------- |
| Larger PQ sigs, slower sign/verify | Higher tail latency; bigger exports | Batch signing; parallel workers; keep hash-chain unchanged; store PQ on checkpoints first |
| PQ library churn                   | Breakage, CVEs                      | Adapter traits (`Signer`/`Verifier`), pinned versions, `cargo deny` in CI                 |
| Downgrade attacks to classical     | Loss of PQ benefit                  | `pq_only=true` where mandated; alert on `audit_pq_missing_verify_total`                   |
| Key sprawl / ops complexity        | Rotation errors                     | Dual-control KMS; cross-sign checkpoints; documented runbooks                             |
| Interop with classical verifiers   | Adoption friction                   | Hybrid by default (M3); envelope format deterministic; clear error taxonomy               |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure documented; **HNDL authenticity risk = high**.
* [ ] Features compile: `--features pq,pq-sign,pq-hybrid,pq-only` (plus `sphincs` optional).
* [ ] Interop matrix green (classical, hybrid, PQ-only); CI includes all modes.
* [ ] PQ metrics and alerts present (even when disabled).
* [ ] Archive checkpoint **co-sign** tool verifies under PQ-only.
* [ ] PERFORMANCE baselines updated with PQ deltas; gates adjusted if needed.
* [ ] RUNBOOK/SECURITY updated (custody, rotation, rollback).
* [ ] GOVERNANCE proposal template ready for default flip.

**Sign-off:** Date ______  ·  Owner ______  ·  Result ☐ Approved ☐ Revisions needed

---

## 11) Role Presets (this crate)

* **Signer/Verifier abstraction:** Provide `Signer`/`Verifier` traits with **Ed25519** and **PQ** backends (ML-DSA primary; optional SLH-DSA) behind features.
* **Defaults:** `pq_hybrid=false` (M1–M2) → **true** (M3); `pq_only=false`. Rotation every 90 days; **cross-sign** manifests at rotate.
* **Peers:** Host verifiers and off-box readers must accept the **envelope** and record which algo succeeded.

---

## 12) Appendix

* **Algorithms chosen (initial):** SIG=**ML-DSA (Dilithium-2)**; HASH=BLAKE3-256. SLH-DSA optional.
* **Libraries (candidates):** `ml-dsa`/`pqcrypto`/`liboqs-rust` (hidden behind adapter traits; do not leak into public API).
* **Envelope format:** Deterministic CBOR/flat struct: `{algo_id, sig, pubkey_id?}`; canonicalized; stable across versions.
* **Change log (to fill):**

  * 2025-Q4: Flags & envelope landed; hybrid verify path.
  * 2026-Q1: Hybrid sign in staging; checkpoint co-sign backfill tool.
  * 2026-Q2: Hybrid default; PQ-only tenants begin.

---

