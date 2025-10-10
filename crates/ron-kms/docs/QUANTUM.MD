
---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-09
audience: contributors, security auditors, ops
crate: ron-kms
crate-type: kms|service
pillar: P3 (Identity & Keys)
owners: [Stevan White]

# QUANTUM.md

## 0) Purpose

Describe how `ron-kms` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops. Scope covers: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and Harvest-Now-Decrypt-Later (HNDL) exposure.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (Shor-breakable):**

  * **KEX:** X25519 for KEM/wrap flows when hybrid is off (transport/library layers).
  * **Signatures:** Ed25519 for sign/verify bundles and attest/checkpoints today.
* **Symmetric/Hash (Grover-affected only):**

  * **AEAD:** ChaCha20-Poly1305 / AES-GCM.
  * **Hash:** BLAKE3-256 (content addressing and input hashing).
* **Data at rest / long-lived artifacts:**

  * **Tamper-evident audit chain** (append-only Merkle + signed checkpoints). Retention: years → **HNDL risk = medium** (signatures can be verified later; contents already hashed).
* **Transport/session lifetime:** short (HTTP/TLS requests), low HNDL exposure on transit.
* **Crate-specific blast radius (worst case if classical PKI is broken):** adversary could forge classical-only signatures or wrap/unwrap sessions if hybrid isn’t enforced; audit integrity detection still prevents silent tampering on restore.

> **HNDL**: We mitigate by adopting **hybrid** now and planning PQ-only later; long-lived artifacts are content-addressed and checkpoint-signed.

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use (allow-list):**

  * **Sign:** Ed25519 (classical) → PQ target ML-DSA (Dilithium).
  * **KEX:** X25519 (classical) → PQ target ML-KEM (Kyber).
  * **AEAD:** ChaCha20-Poly1305 / AES-GCM.
  * Modes support **OR/AND/PQ-only** for verify (policy-driven).
* **Libraries:** ed25519-dalek v2 (noted in ops docs), rustls/transport adapters via `ron-transport`.
* **Key custody:** sealed/file, PKCS#11/HSM, TPM, in-mem dev; non-exportable secrets, anti-rollback & audit checkpoints.
* **Interfaces that carry crypto:** HTTP `/v1/kms/sign|verify|wrap|unwrap|rotate` (canonical), optional gRPC mirror; strict DTOs.

---

## 3) Target PQ Posture (Where we’re going)

* **KEX / Encryption:** **Hybrid KEM** by default (**X25519+ML-KEM**), then selective **PQ-only** by policy/env.
* **Signatures:** **Hybrid Sign** (Ed25519 + ML-DSA) for bundles; policy can require **AND** or allow **OR** during migration; eventual **PQ-only** for new KIDs.
* **Transport TLS:** enable hybrid KEX when `pq_hybrid=true`; fall back only if policy allows.
* **Tokens/Capabilities:** support PQ signature option; negotiate per-tenant via `ron-policy`.
* **Back-compat:** classical remains until **Phase 3**; default becomes **Hybrid-AND** at **Phase 2** (Gold).

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (illustrative)
[features]
pq = []            # enable PQ plumbing in this crate
pq-hybrid = ["pq"] # use Hybrid KEM (X25519 + ML-KEM) where applicable
pq-sign = ["pq"]   # enable PQ signatures (ML-DSA); hybrid bundles in sign path
pq-only = []       # (rare) build-time harden to PQ-only (ops prefers runtime)
```

```ini
# Runtime (maps to CONFIG.md schema)
KMS_PQ_PRODUCE_HYBRID=1             # sign emits classical+PQ (when policy allows)
KMS_PQ_VERIFY_MODE=or|and|pq_only   # interop policy (Phase 1→3)
KMS_PQ_ROLLOUT_PERCENT=0..100       # per-tenant staged cutover
```

Knobs and phases are canonical in the repo and surfaced in metrics and docs.

**Interoperability:** if a peer lacks PQ, behavior = negotiate (OR) unless `pq_only` is required → refuse with stable error. **Emit PQ-labeled metrics even when disabled** to ease adoption.

---

## 5) Migration Plan (Milestones)

* **Phase 0 — Classical-only (Bronze)**
  Policy hybrid off. Measure baselines (handshake/sign/verify CPU/latency).

* **Phase 1 — Hybrid-OR (Silver)**
  Verify accepts **classical OR PQ**; sign produces hybrid bundles when `produce_hybrid=true`. Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (when policy allows).

* **Phase 2 — Hybrid-AND (Gold default)**
  Verify **requires both** sigs; sign emits both; rollout gated by tenant `%`. Strict observability: rejections tagged `reason="pq_policy"`.

* **Phase 3 — PQ-only (post-Gold)**
  New KIDs PQ-only; classical verify allowed only during grace for legacy IDs, then sunset.

Rollback path: flip `KMS_PQ_VERIFY_MODE` one notch looser, drop rollout %, and alert on downgrade.

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical paths never run **pure classical** once policy requires hybrid/PQ.
* **[PQ-I2]** Symmetric ≥256-bit; hashes ≥256-bit; constant-time checks.
* **[PQ-I3]** Long-lived data (audit, key blobs) stay re-encryptable; anti-rollback enforced.
* **[PQ-I4]** `pq_only=true` must refuse downgraded sessions with a clear, canonical error.
* **[PQ-I5]** Rotation upgrades algorithms atomically; no silent fallback.
* **[PQ-I6]** CI matrix builds/tests `pq`, `pq-hybrid`, `pq-sign`; interop parity proven.

---

## 7) Observability (Metrics, Logs, Readiness)

Emit per-op metrics with **algo** labels:

* `kms_ops_total{op,alg,result}` (includes `alg="ed25519|ml-dsa|x25519|ml-kem"`)
* `kms_latency_seconds{op,alg}` (histogram)
* `kms_reject_total{reason="pq_policy|attest_missing|…"}`
* `kms_audit_integrity_failed_total` (tamper detection)
* PQ rollout watchpoints: `kms_ops_total{alg="ml-dsa|ml-kem"}`, `kms_reject_total{reason="pq_policy"}`.

**Readiness:** `/readyz=false` if policy requires hybrid/PQ and peer/stack can’t satisfy it (or audit integrity fails).

**Logs (structured):** include `pq={off|hybrid|pq-only}`, `pq_mode={or|and|pq_only}`, `alg`, `tenant`, `kid_hash`, never raw secrets.

---

## 8) Testing & Verification

* **Unit/Property:** DTO parsers, bundle assembly, policy state-machine under classical/PQ/hybrid.
* **Interop suite:** classical↔classical, hybrid↔hybrid, hybrid↔classical (policy OR).
* **Fuzz:** PQ decoders & negotiation paths; robust error taxonomy.
* **Load:** handshake/ops/sec with and without PQ; include ARM/edge profiles (Micronode).
* **Security drills:** force `pq_only` and verify safe/refusals; simulate classical break.

---

## 9) Risks & Mitigations

* **Perf & footprint (bigger keys, slower handshakes):** cache handshakes; reuse sessions; scale signer pools; monitor latency deltas by alg.
* **Library churn:** isolate via thin adapter traits; pin versions per workspace.
* **Downgrade abuse:** enforce `pq_only` where mandated; alert on downgraded sessions.
* **E2E gaps:** keep PQ optional until peers are ready; document peer requirements and tenant rollout %.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL risk labeled.
* [ ] CI builds with `--features pq,pq-hybrid,pq-sign` and runs interop tests.
* [ ] Hybrid KEM/sign interop passes; clear downgrade errors.
* [ ] PQ metrics emitted; dashboards show alg split & rejections.
* [ ] Runbook updated with enable/rollback steps (flip verify_mode / rollout %).
* [ ] Perf numbers recorded (handshake/sign/verify CPU & latency).
* [ ] SECURITY.md cross-links updated; owners ack.

---

## 11) Role Presets (crate-specific)

**Identity/policy/kms (`ron-kms`, `ron-auth`, `svc-passport`, `ron-policy`, `ron-audit`)**
Primary targets: **PQ signatures** option; custody & rotation; audit coverage.
Defaults: `pq_sign=false (Phase 0/1) → optional (Phase 2/3)`; `key_rotation_days=90`.

**Transport (`ron-transport`)**
Primary: **Hybrid KEX** adapter + negotiation policy.
Default: `pq_hybrid=false (Phase 0/1) → true (Phase 2)`; `pq_only=false`.

**Gatekeepers (omnigate/gateway/nodes)**
Policy-driven negotiation; `/readyz` reflects PQ requirements; easy rollback.

---

## 12) Appendix

* **Algorithms chosen:**

  * **KEX:** Hybrid (X25519+ML-KEM) → PQ-only ML-KEM for new KIDs post-sunset.
  * **SIG:** Hybrid (Ed25519+ML-DSA) → PQ-only ML-DSA for new KIDs when policy moves to Phase 3.
* **Libraries (tracking):** ed25519-dalek v2; transport via rustls/`ron-transport`; PQ adapters behind feature gates (workspace-pinned).
* **Interop notes:** canonical HTTP/JSON; optional gRPC mirrors 1:1; DTOs strict; peers must advertise hybrid/PQ readiness.
* **Config quick-ref:** `KMS_PQ_PRODUCE_HYBRID`, `KMS_PQ_VERIFY_MODE=or|and|pq_only`, `KMS_PQ_ROLLOUT_PERCENT=0..100`.
* **Change log (seed):**

  * 2025-10-09: Initial QUANTUM.md; wired phases, knobs, metrics; defaults = Phase 1 (Hybrid-OR) for dev, Phase 2 (Hybrid-AND) targeted for Gold.

---

### Notes for auditors & ops

* This document inherits **Hardening v2** mandates: **capabilities only**, **PQ-hybrid readiness**, **amnesia mode** on Micronode.
* The **tamper-evident audit chain** is the lifeline under migration—if integrity check fails at startup, the service refuses writes and surfaces degraded readiness.

---
