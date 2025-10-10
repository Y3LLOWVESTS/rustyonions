
````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-09
audience: contributors, security auditors, ops
crate: svc-passport
crate-type: service
pillar: 3               # Identity & Keys
owners: [Stevan White]
review-cadence: quarterly
---

# QUANTUM.md — svc-passport

## 0) Purpose
Describe how `svc-passport` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or operations.  
Scope covers: algorithms in use, key custody, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure. Aligned with `SECURITY.md`, `RUNBOOK.md`, `TESTS.md`, `GOVERNANCE.md`, and `PERFORMANCE.md`.

---

## 1) Exposure Assessment (What’s at risk?)
**Public-key usage (Shor-breakable):**
- TLS 1.3 KEX (edge, s2s): **X25519** today.
- Issuance signatures: **Ed25519** via `ron-kms`.

**Symmetric/Hash (Grover-affected only):**
- AEAD: **ChaCha20-Poly1305** (or **AES-256-GCM**).
- Hash: **BLAKE3-256** (ample Grover margin).

**Data at rest / long-lived artifacts:**
- Audit ledger (append-only, signed); revocation/epoch journal.
- Retention: **years** (audit), **90–180d** (revocation).
- **HNDL risk:** Transport **low** (short sessions), **signature integrity high** if we remain classical (future forgeries).

**Crate-specific blast radius:** If classical sigs are forgeable, attacker could mint **valid-looking passports** off-path. Therefore we must adopt **hybrid/PQ signatures** on issuance and verification acceptance policy.

---

## 2) Current Crypto Profile (Today)
- **KEX (TLS 1.3):** X25519
- **Signatures (issuance/audit):** Ed25519 (`ed25519-dalek v2` via `ron-kms`)
- **Capabilities:** Macaroons (HMAC-BLAKE3-256) — symmetric, PQ-safe
- **AEAD/Hash:** ChaCha20-Poly1305 or AES-256-GCM / BLAKE3-256
- **Libs (illustrative):** `rustls`, `ring`/`evercrypt`, `ed25519-dalek`, `blake3`
- **Custody:** Keys in **ron-kms/HSM**; rotation **≤90d**; no raw keys on disk/env
- **Interfaces:** HTTP API (issue/verify/revoke over TLS), signed bus events (audit/revocation)

---

## 3) Target PQ Posture (Where we’re going)
- **Transport:** **Hybrid KEX** `X25519 + ML-KEM (Kyber)` once upstream supports it.
- **Signatures:** **Hybrid issuance/audit**: `Ed25519 + ML-DSA (Dilithium)`; optional **SLH-DSA (SPHINCS+)** for high-assurance domains.
- **Capabilities:** Remain HMAC-BLAKE3 (no change).
- **Compatibility / Modes:**  
  `pq_mode = off|mirror|require|enforce`
  - **mirror**: issue dual, accept classical+hybrid; measure adoption
  - **require**: issue hybrid; accept classical inbound for a **grace** window
  - **enforce**: refuse classical
- **Default:** Hybrid becomes default (require) at **M3 (Gold)**; enforce only per-tenant/region with sign-off.

---

## 4) Feature Flags & Config (How to turn it on)
```toml
# Cargo features
[features]
pq = []                 # enable PQ plumbing
pq-hybrid = ["pq"]      # multi-alg envelopes and verification
pq-sign = ["pq"]        # ML-DSA / SLH-DSA signing via KMS
pq-only = []            # build-time forbid classical (rare)
````

```ini
# svc-passport.toml (excerpt)
pq_mode = "mirror"          # off|mirror|require|enforce
pq_hybrid_kex = false       # transport capability (informational here)
pq_sign_algo = "ml-dsa"     # ml-dsa | slh-dsa (fallbacks below)
pq_sign_fallback = "slh-dsa" # used if preferred algo unavailable
classical_grace_days = 30   # acceptance window when pq_mode=require
pq_only = false             # hard refuse classical
key_rotation_days = 90
rollback_sla_mins = 60      # commit to roll back to mirror within 60m on PQ incident
```

* **Interop behavior:**
  `mirror` → allow classical, emit **pq_downgrade_total**;
  `require` → accept classical **only** within `classical_grace_days`; alert on downgrades;
  `enforce`/`pq_only` → refuse classical with explicit error.

* **SBOM/pinning:** PQ libs must be pinned; SBOM published (see `SECURITY.md` SBOM section).

---

## 5) Migration Plan (Milestones)

| Milestone       | Deliverables                                                                                                          | Gates / SLAs                                                                                       |
| --------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **M1 (Bronze)** | Features/config stubs; hybrid envelope type behind flags; exposure documented; baseline perf                          | CI builds `--features pq,pq-hybrid,pq-sign`; classical p95 baselined                               |
| **M2 (Silver)** | Issue dual sigs in `mirror/require`; verify per policy; interop matrix classical/hybrid; KMS PQ keys provisioned      | Perf delta p95: sign ≤ **+20ms** (x86), **+30ms** (ARM64); verify ≤ **+10ms**; TESTS PQ fuzz green |
| **M3 (Gold)**   | Default `pq_mode=require`; dashboards show adoption/downgrades; runbooks for flip/rollback; audits include PQ posture | Burn-rate stable; roll-forward/back in ≤ **rollback_sla_mins**; no perf gate violations            |
| **Post-M3**     | Start `enforce` in selected envs; deprecate classical                                                                 | No classical accepted in enforced envs; quarterly PQ review                                        |

Rollback policy: on sustained `pq_downgrade_total` or perf gate breach, **flip to mirror within `rollback_sla_mins`**, open incident, and proceed with remediation.

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical signatures (issuance/audit) are **hybrid/PQ** per policy; no pure ECC beyond grace.
* **[PQ-I2]** Symmetric ≥ **256-bit**, hashes ≥ **256-bit** (BLAKE3-256).
* **[PQ-I3]** Long-lived records remain verifiable under PQ—include hybrid now to avoid re-signing later.
* **[PQ-I4]** `pq_only=true` / `enforce` **refuses** classical cleanly (observable).
* **[PQ-I5]** Rotation upgrades algorithms without silent fallback; dual-control required to activate PQ keys.
* **[PQ-I6]** CI runs PQ builds + interop; classical parity proven; failure triggers rollback per SLA.

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics** (labels: `algo`, `mode`, `tenant`, `region`):

* `pq_mode_info{mode="off|mirror|require|enforce"} 1`
* `pq_signature_total{algo="ed25519|ml-dsa|slh-dsa", outcome="ok|fail"}`
* `pq_signature_latency_seconds_bucket{op="sign|verify",algo}`
* `pq_downgrade_total{reason="peer_no_pq|policy_grace|config_off"}`
* `pq_adoption_ratio = pq_signature_total{algo!="ed25519"} / pq_signature_total{}`

**Readiness:** `/readyz` fails when `enforce` is set and PQ path is not healthy (missing PQ key/adapter), or when `require` is set and inbound classical exceeds grace policy.

**Logs:** Always include `pq.mode`, `pq.sign_algo`, `pq.sign_fallback`, `peer_alg_set`, `decision=accept|reject|grace`.

---

## 8) Testing & Verification

* **Unit/property:** multi-alg envelope parsing/verification; idempotent hybrid verify; malformed PQ artifacts never panic.
* **Interop matrix (TESTS.md):** classical↔classical ✔, hybrid↔hybrid ✔, hybrid↔classical (require+grace) ✔, classical↔hybrid (reject under enforce) ✔ — run on x86_64 **and** ARM64.
* **Fuzz:** `hybrid_sig_verify.rs` (ML-DSA/SLH-DSA parts + negotiation); ensure correct rejects.
* **Load/Perf:** record sign/verify deltas: x86_64 sign ≤ **+20ms** p95, ARM64 sign ≤ **+30ms** p95; verify ≤ **+10ms** both; issuance/verify SLOs remain within gates; include WAN profiles.
* **Drills (cadence):** **Quarterly** simulate “classical break”: set `pq_only=true` → controlled failures, clear operator guidance (RUNBOOK §9).

---

## 9) Risks & Mitigations

| Risk                | Impact                | Mitigation                                                                        |
| ------------------- | --------------------- | --------------------------------------------------------------------------------- |
| Perf overhead       | Latency/cost ↑        | Hybrid only where needed; cache verifies; monitor p95/p99; scale; ARM64 profiling |
| Library churn       | Instability           | Adapter trait for PQ; pin versions; SBOM; advisories in CI                        |
| Downgrade abuse     | Peer forces classical | `require/enforce` + `pq_downgrade_total` alert; refuse under enforce              |
| Rollback complexity | Blast radius          | One-flag rollback to `mirror`; **rollback_sla_mins** ≤ 60; runbook automation     |
| Envelope bloat      | Token size ↑          | Compact multi-alg encoding; prefer ML-DSA; SLH-DSA only when mandated             |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL** labeled (transport low; signature integrity high if classical only).
* [ ] PQ features compile; CI matrix includes `--features pq,pq-hybrid,pq-sign`.
* [ ] Hybrid signature envelope implemented; interop matrix **passes** on x86_64 and ARM64.
* [ ] PQ metrics emitted; dashboards show **adoption ratio** and **downgrades**.
* [ ] RUNBOOK updated with enable/rollback steps and `/readyz` gating.
* [ ] Perf deltas captured and within budgets (x86_64/ARM64; WAN).
* [ ] SECURITY/OBSERVABILITY/GOVERNANCE cross-links updated; SBOM pinned; owners ACK.

---

## 11) Role Presets (svc-passport focus)

**Identity/policy/kms:** Hybrid signatures for issuance/audit; KMS custody of ML-DSA keys; `pq_mode: mirror→require` by M3; rotation ≤90d; dual-control on PQ key activation.
**Transport:** Enable hybrid KEX upstream; svc-passport treats transport readiness as part of `/readyz` policy checks.
**SDK/clients:** Verify hybrid envelopes; clear error when server `enforce` is active; keep classical verify until M3.

---

## 12) Appendix

**Algorithms (initial):**

* KEX: Hybrid `X25519 + ML-KEM-768`
* Signatures: **ML-DSA-65 (Dilithium3)** primary; **SLH-DSA-128s** fallback

**Libraries:** PQ via `oqs`/`liboqs` (wrapped behind `pq::Sig`/`pq::Kem` traits); classical via `ed25519-dalek`, `ring`/`evercrypt`. All pinned; SBOM published.

**Fallback logic:** If `pq_sign_algo` unavailable at runtime, attempt `pq_sign_fallback`; if neither available and `pq_mode∈{require,enforce}`, **fail `/readyz`** and **refuse** issuance (fail-closed).

**Interop notes:** Envelope supports `algs=[ed25519, ml-dsa, slh-dsa?]`; policy determines acceptance; `classical_grace_days` bounds acceptance window.

**Change log:**

* 2025-10-09 — Initial QUANTUM plan; knobs and budgets added.
* 2025-11-xx — Hybrid envelope merged (behind `pq-sign`).
* 2026-xx-xx — Default `pq_mode=require` in production control-plane.

**Operator quick steps:**

* Enable mirror → `pq_mode=mirror` and watch `pq_signature_total{algo="ml-dsa"}`.
* Require hybrid → `pq_mode=require`; ensure peers upgraded; watch **downgrades** = 0 before grace ends.
* Enforce → `pq_mode=enforce` only after interop green; `/readyz` enforces PQ path health.
* Rollback → flip to `mirror`; aim < **rollback_sla_mins**; attach incident.

```
```
