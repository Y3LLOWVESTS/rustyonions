
---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-09-23
audience: contributors, security auditors, ops
crate: micronode
crate-type: node
pillar: 8
owners: [Stevan White]
inherits-from: /docs/QUANTUM_BLUEPRINT.MD
---

# QUANTUM.md — Micronode

> **Conflict rule:** If anything here conflicts with the repo-level QUANTUM blueprint, **the project blueprint wins**. This file documents **micronode deltas** only (profile-specific posture, knobs, and SLOs).

## 0) Purpose

Micronode is the **developer-friendly, single-binary** node profile (Pillar 8) with **amnesia ON by default** (RAM-first, ephemeral logs, no disk spill unless explicitly enabled).  
This document defines micronode’s **quantum-resilience posture** today and the staged migration to **post-quantum (PQ)** crypto with **zero interop breakage** against macronode or upstream services, while minimizing **harvest-now, decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

- **Public-key usage (Shor-breakable):**
  - **KEX:** X25519 (TLS 1.3 via `ron-transport`)
  - **Signatures:** Ed25519 (caps/macaroons, manifests, dev receipts)
- **Symmetric / Hash (Grover only):**
  - **AEAD:** AES-256, ChaCha20-Poly1305  
  - **Hash/Addressing:** BLAKE3-256 (canonical content addressing)
- **Data at rest / long-lived artifacts:**  
  - **Default amnesia:** in-RAM state, ephemeral logs, no disk by default.  
  - **Retention window:** minutes → hours (dev loops); if `MICRO_PERSIST=1`, small local caches may exist (operator choice).  
  - **HNDL label:** **Low** by default; **Medium** if persistence is enabled.
- **Transport/session lifetime:** typically seconds → minutes (short lifetimes reduce HNDL risk).
- **Blast radius (worst case if classical PKI breaks):**  
  Live session **impersonation or tampering** during runtime; **no durable secrets or payloads** persist under default amnesia.

> **HNDL = Harvest-Now, Decrypt-Later:** risk that an adversary records ciphertext today and decrypts it post-quantum.

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use:**
  - **KEX:** X25519
  - **Signatures:** Ed25519
  - **Symmetric/Hash:** AES-256, ChaCha20-Poly1305, BLAKE3-256
- **Libraries:** `tokio-rustls` (TLS), `ring` / `ed25519-dalek`, `blake3`
- **Key custody:** Provided by `ron-kms`; **RAM-only** under amnesia; zeroized on drop. Rotation N/A for ephemeral dev keys.
- **Crypto-bearing interfaces:** TLS sessions, OAP/1 frames, capability tokens (macaroons), signed manifests/receipts.

---

## 3) Target PQ Posture (Where we’re going)

- **KEX / encryption:** **Hybrid** X25519 + **ML-KEM (Kyber)**  
- **Signatures:** **ML-DSA (Dilithium)** preferred; **SLH-DSA (SPHINCS+)** optional (stateless fallback)
- **Transport TLS:** classical today → enable **hybrid KEX** when `pq_hybrid = true`
- **Tokens / capabilities:** add PQ signature option; negotiate per-tenant policy
- **Back-compat:** classical allowed until **M3 (Gold)**; hybrid becomes default at M3

---

## 4) Feature Flags, Config & Env Overlays (How to turn it on)

```toml
# Cargo (micronode) — opt-in PQ plumbing
[features]
pq = []               # enable PQ plumbing
pq-hybrid = ["pq"]    # use Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]      # enable PQ signatures (Dilithium/Sphincs+)
pq-only = []          # optional: refuse classical entirely
````

```ini
# micronode.toml (excerpt)
[security]
pq_hybrid = false         # M1 default; becomes true at M3
pq_sign_algo = "ml-dsa"   # "ml-dsa" | "slh-dsa"
pq_only = false           # if true, refuse classical peers
key_rotation_days = 90    # kept for persistence opt-in paths
```

**Env overlays (operator ergonomics):**

```
PQ_HYBRID=true|false
PQ_SIGN_ALGO=ml-dsa|slh-dsa
PQ_ONLY=true|false
MICRO_PERSIST=0|1          # amnesia: 0 (on, default) / 1 (allow disk)
```

**Interoperability switch:** If the peer lacks PQ, behavior = **negotiate** (fallback to classical) unless `PQ_ONLY=true`, in which case **refuse** with a clear error.

**Metrics toggle:** Always emit PQ metrics/labels even when disabled (value=0) to simplify adoption.

---

## 5) Migration Plan (Milestones & SLOs)

**M1 (Bronze) — Planning & Hooks**

* Add `pq` features + config stubs (no behavior change).
* Emit PQ metrics (zero-valued) for visibility.
* **Baseline perf SLOs:** record classical handshake **p95 ≤ 120 ms (local)**, **p99 ≤ 200 ms**.

**M2 (Silver) — Hybrid Enablement**

* Enable **Hybrid KEX** in `ron-transport` behind `pq_hybrid`.
* Optional **PQ signatures** via `pq-sign` (caps/manifests).
* **Interop matrix:** classical↔classical, hybrid↔hybrid, hybrid↔classical (where policy allows).
* **Perf budget:** handshake overhead target **≤ 20%** vs classical; track **p95/p99**.

**M3 (Gold) — Default & Ops**

* Default `pq_hybrid = true` for micronode builds.
* PQ signatures default for caps unless policy overrides.
* Runbook: enable/rollback steps; **flip back** to classical cleanly if needed.
* **SLO gate to exit M3:** sustained **p95 handshake ≤ 150 ms**, **downgrades < 1%** under normal ops.

**Post-M3 — De-risking**

* Offer `pq_only` for auditors/regulatory runs.
* Track ecosystem/library updates; revisit parameters quarterly.

---

## 6) Invariants (MUST)

* **\[PQ-I1]** No pure ECC/RSA in security-critical paths **post-M3** unless wrapped by **hybrid PQ**.
* **\[PQ-I2]** Symmetric keys **≥ 256-bit**; hash = **BLAKE3-256**.
* **\[PQ-I3]** Any persisted long-lived data (if `MICRO_PERSIST=1`) MUST be **re-encryptable** under PQ keys without data loss or silent downgrade.
* **\[PQ-I4]** If `pq_only = true`, **refuse** classical peers with a specific, user-actionable error.
* **\[PQ-I5]** **Interop** with macronode: classical + hybrid permutations MUST be proven in CI.
* **\[PQ-I6]** CI matrix includes `--features pq,pq-hybrid,pq-sign` and exercises env overlays.
* **\[PQ-I7]** **Blast-radius statement** present and kept current (see §1).

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with **algo labels**:

* `pq_handshake_total{algo="x25519"|"ml-kem"|"hybrid", role}`
* `pq_signature_total{algo="ed25519"|"ml-dsa"|"slh-dsa"}`
* `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify",algo}` (histogram)
* **Downgrade tracking:** `pq_downgrade_events_total{from="hybrid", to="classical", reason}`

**Readiness:** `/readyz` **fails** if policy requires PQ (`pq_only=true` or tenant policy) and the peer/stack can’t negotiate it.

**Structured logs:** include `pq={off|hybrid|pq-only}`, `algo=...`, `peer_mode=...`, and `downgrade_reason` when applicable.

---

## 8) Testing & Verification

* **Unit / property:** token validators, envelope sign/verify in classical + PQ; header negotiation sanity.
* **Interop suite:** classical↔classical; hybrid↔hybrid; hybrid↔classical (when allowed).
* **Fuzzing:** PQ decoders & negotiation paths; error taxonomy (downgrade, refuse, unsupported).
* **Load tests:** handshake/sec with and without PQ on developer hardware profiles.
* **Concurrency model (dev-friendly):** add **Loom** tests for the handshake/negotiator FSM and config hot-swap edges.
* **Security drill:** simulate “classical break” day: force `pq_only=true` and verify safe failure modes (clear errors, /readyz behavior, metrics/alerts).

---

## 9) Risks & Mitigations

* **Perf & footprint:** PQ keys/certs are larger; handshakes slower → reuse sessions, cache creds (within amnesia limits), keep sessions short.
* **Library churn:** Pin PQ deps via workspace; isolate behind adapter traits to allow swaps without API churn.
* **Downgrade abuse:** Enforce `pq_only` where mandated; **alert** on `pq_downgrade_events_total > 0`.
* **Interop gaps:** Keep classical available until M3; CI interop matrix as a release gate.

---

## 10) Acceptance Checklist (Definition of Done)

* [ ] **HNDL risk labeled:** **Low** (default amnesia) / **Medium** (if persistence enabled).
* [ ] `pq`, `pq-hybrid`, `pq-sign` **compile & run in CI**; env overlays honored.
* [ ] **Interop tests pass** across classical/hybrid permutations.
* [ ] **PQ metrics emitted**, including **downgrade counter**; dashboards updated.
* [ ] **Runbook updated** with enable/rollback and pq\_only incident playbook.
* [ ] **Perf recorded**: handshake p95/p99 (classical vs hybrid) and overhead ≤ target.
* [ ] **SECURITY.md updated** to reflect PQ roadmap, amnesia tie-ins, and rollback plan.

---

## 11) Role Presets (Micronode)

* **Primary target:** policy-driven **hybrid PQ negotiation** with amnesia-first defaults.
* **Defaults:** `amnesia=ON`, `pq_hybrid=false (M1) → true (M3)`, `pq_only=false`.
* **Operator toggles:** `PQ_HYBRID`, `PQ_ONLY`, `PQ_SIGN_ALGO`, `MICRO_PERSIST`.

---

## 12) Appendix (adoption log)

* **Algorithms chosen:** KEX = **Hybrid (X25519 + ML-KEM/Kyber)**; SIG = **ML-DSA (Dilithium)** (primary) / **SLH-DSA (SPHINCS+)** (optional).
* **Libraries (candidates):** `liboqs-rust` / `oqs-sys` feature-gated; pinned via workspace.
* **Interop notes:** Must interop with macronode in both classical and hybrid modes.
* **Change log:**

  * **2025-09-23:** Initial micronode QUANTUM.md (canon-aligned, inherits-from added, SLOs, downgrade metric, Loom tests, env overlays).

```

---
