
---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing — ron-auth
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, security auditors, ops
crate: ron-auth
crate-type: lib | policy
pillar: 3  # Identity & Keys
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose
`ron-auth` verifies **capability tokens** (macaroon-style). Its correctness underpins the entire identity plane: if verification can be forged, upper-layer economics collapse.  
This document explains how `ron-auth` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto while keeping interop and policy stability.

Scope: algorithms in use, custody boundaries, runtime knobs, telemetry, tests, rollout plan, and “Harvest-Now, Decrypt-Later” (HNDL) exposure.

---

## 1) Exposure Assessment (What’s at risk?)

| Class | Algorithm / Usage | Quantum Risk | Notes |
|---|---|---:|---|
| Public-key usage (Shor) | Ed25519 (signatures via optional `SigAdapter` for cross-org envelopes) | High **if used alone** | Optional only; disabled unless feature-gated. |
| Key exchange | N/A in this crate | – | Transport handled by hosts (ron-transport). |
| Symmetric/Hash (Grover) | Keyed **BLAKE3-256** (MAC chain) | Low | 256-bit → ~128-bit effective post-Grover; keep 256-bit outputs. |
| Data at rest | None (no persistence) | – | Tokens are transient in RAM only. |
| Token TTL / HNDL | Default TTL **≤ 5 minutes** (policy) | **Low HNDL** | Short lifetime makes harvested bytes useless later. |
| Blast radius | Forged envelope sigs could spoof cross-tenant tokens for affected tenants if classical-only signatures were broken. |

> **HNDL risk:** Low. `ron-auth` stores no ciphertext or long-lived secrets; only transient token bytes.

---

## 2) Current Crypto Profile (Today)

| Function | Algorithm | Library | Notes |
|---|---|---|---|
| MAC chain | **Keyed BLAKE3-256** | `blake3` | Deterministic O(n_caveats) chain. |
| Hashing | BLAKE3-256 | `blake3` | Keep 256-bit outputs. |
| Envelope signatures (optional) | **Ed25519** | `ed25519-dalek` v2 | Only via `SigAdapter` when feature-enabled. |
| Key custody | **ron-kms / HSM** | – | Verifier sees opaque handles only. |
| Rotation | ≥ every **90 days** | – | Accept current + N previous. |

---

## 3) Target PQ Posture (Where we’re going)

| Plane | Goal Algorithm | Adoption | Comment |
|---|---|---|---|
| MAC / Hash | Keep **BLAKE3-256** | Stable | Grover-aware sizing already adequate. |
| Envelope Signatures | **Hybrid** Ed25519 + **ML-DSA (Dilithium2)** | M2 optional → M3 recommended default | Maintain classical interop; add PQ assurance. |
| KMS / KEX (outside) | **Hybrid** X25519 + **ML-KEM (Kyber-768)** | In `ron-kms`/transport | Out of scope for this crate’s code; tracked for completeness. |

**Back-compat:** Classical Ed25519 remains supported through M3; hybrid becomes the default thereafter (policy-controlled per tenant).

---

## 4) Feature Flags & Config (How to turn it on)

```toml
[features]
pq = []                 # base PQ plumbing/types
pq-hybrid = ["pq"]      # enable hybrid envelope verify (classical + PQ)
pq-sign = ["pq"]        # enable PQ signature verification paths
pq-only = []            # compile-time guard to refuse classical signatures
````

```ini
# VerifierConfig (host-provided)
pq_hybrid = true              # default off until M3
pq_sign_algo = "ml-dsa"       # "ml-dsa" | "slh-dsa"
pq_only = false               # if true, deny classical-signature envelopes
key_rotation_days = 90
```

* **Interoperability:** If peer lacks PQ, `pq_hybrid=true` still verifies classical; with `pq_only=true`, verification **denies** with a clear reason.
* **Metrics:** always emit PQ labels (even if 0) for easy adoption.

---

## 5) Migration Plan (Milestones)

| Milestone       | Deliverables                                                                                                                                                    |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **M1 (Bronze)** | Add `pq` features; compile stubs; document exposure; baseline classical perf.                                                                                   |
| **M2 (Silver)** | Implement **hybrid** envelope verify (Ed25519 + Dilithium2); interop tests (classical↔classical, hybrid↔hybrid, hybrid↔classical); record perf deltas per arch. |
| **M3 (Gold)**   | Default `pq_hybrid=true` for identity plane; publish dual vectors; update RUNBOOK/SECURITY; dashboards show PQ adoption.                                        |
| **Post-M3**     | Optional `pq_only` environments; begin sunsetting pure-classical on external edges when partners ready.                                                         |

---

## 6) Invariants (MUST)

* **[PQ-I1]** Any signature path MUST provide ≥ **128-bit** PQ security (e.g., ML-DSA level 2+).
* **[PQ-I2]** `pq_only=true` ⇒ classical-only envelopes **deny** with `deny.reason="pq.unsupported"`.
* **[PQ-I3]** MAC keys & hashes remain **256-bit**.
* **[PQ-I4]** Key rotations must migrate handles atomically; no silent downgrade to weaker algos.
* **[PQ-I5]** Golden vectors cover classical **and** hybrid cases; CI enforces parity.
* **[PQ-I6]** PQ feature builds pass the matrix; decisions are identical to classical except for algo labels.
* **[PQ-E1] (ECON tie)** PQ upgrades MUST **not** introduce any “free escalation” of authority or broaden allow-sets. Tests assert that hybrid mode is **at most** as permissive as classical (i.e., Allow_hybrid ⊆ Allow_classical unless policy explicitly widens).

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with algo labels:

* `ron_auth_verify_total{pq="off|hybrid|pq-only"}`
* `ron_auth_signature_total{algo="ed25519|ml-dsa|hybrid"}`
* `ron_auth_signature_fail_total{algo,reason}`
* `crypto_latency_seconds{op="verify",algo}` (histogram)

**Readiness (host):** if policy mandates PQ and peer/stack can’t negotiate it, `/readyz` fails.
**Logs:** structured JSON includes `pq_state`, `algo`, `peer_mode`, and stable `reason`.

---

## 8) Testing & Verification

* **Unit / property:**

  * Hybrid verify is **monotonic** (never accepts a token that classical would deny unless policy explicitly allows).
  * Corrupt PQ signatures ⇒ `pq.sig.invalid`; never panic.

* **Interop suite:**

  * classical↔classical, hybrid↔hybrid, hybrid↔classical (if `pq_only=false`).
  * Deny with `pq.unsupported` when `pq_only=true` and peer is classical.

* **Fuzz:** PQ envelope decoders, negotiation paths, error taxonomy.

* **Perf:**

  * Record verify costs per algo & arch (x86/ARM).
  * **Expectation:** Dilithium verify can be **5–50×** Ed25519 depending on libs/arch; our target is **“within SLO budget”** (see PERFORMANCE.md) rather than a fixed % cap.

* **Security drill:**

  * Flip `pq_only=true`; confirm classical envelopes deny cleanly with auditable reasons.
  * Rotation drills unchanged.

---

## 9) Risks & Mitigations

| Risk             | Impact               | Mitigation                                                                             |
| ---------------- | -------------------- | -------------------------------------------------------------------------------------- |
| PQ perf overhead | Higher CPU/latency   | Parallelize verify; cache per-request results; keep MAC chain fast; track perf labels. |
| Library churn    | Interop breaks       | Wrap via `SigAdapter`; pin versions; golden vector parity in CI.                       |
| Downgrade abuse  | Loss of PQ guarantee | Enforce `pq_only` when mandated; alert on downgraded sessions.                         |
| Key size bloat   | Memory/IPC overhead  | Opaque handles; registrar partitioning by tenant.                                      |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL risk **Low** with TTLs documented.
* [ ] PQ feature set compiles (`--features pq,pq-hybrid,pq-sign,pq-only`).
* [ ] Interop matrix passes; `pq_only` denies classical with stable reason.
* [ ] PQ metrics labeled & dashboards updated.
* [ ] Perf numbers recorded per arch; SLO budget respected.
* [ ] RUNBOOK / SECURITY / GOVERNANCE cross-links updated.

---

## 11) Role Preset (Identity/Policy)

| Aspect          | ron-auth Default             | Upgrade Path                                            |
| --------------- | ---------------------------- | ------------------------------------------------------- |
| Feature default | `pq_hybrid=false` (M1–M2)    | → `true` (M3)                                           |
| Signature mode  | Classical Ed25519            | Hybrid Ed25519 + ML-DSA                                 |
| Custody         | `ron-kms` (HSM/KMS)          | Hybrid KEM window (X25519 + Kyber-768) in KMS/transport |
| Perf target     | Fit within verify SLO budget | Monitor per-algo labels; optimize wrappers              |
| Audit scope     | Vectors + metrics + alerts   | Add PQ adoption KPIs                                    |

---

## 12) PQ Ciphersuites Policy (Normative)

* **Preferred signatures (M3):** `hybrid(ed25519, ml-dsa-65)` (Dilithium2 class).
* **Acceptable alternatives:** `hybrid(ed25519, slh-dsa-s128)` if ML-DSA unavailable.
* **KEM (outside):** `hybrid(x25519, ml-kem-768)` for KMS/transport layers.
* **Hash/MAC:** BLAKE3-256 retained; no change required.
* **Deprecation path:** pure classical signatures to be discouraged post-M3 and eventually refused when tenant policy marks `pq_only=true`.

---

## 13) CI Matrix (Minimal, copy-paste)

```yaml
name: pq-matrix
on:
  pull_request: {}
  push:
    branches: [main]
jobs:
  test-pq:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        features:
          - ""
          - "pq"
          - "pq,pq-hybrid"
          - "pq,pq-sign"
          - "pq,pq-hybrid,pq-sign"
          - "pq,pq-hybrid,pq-sign,pq-only"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build & test
        run: |
          FEAT="${{ matrix.features }}"
          if [ -n "$FEAT" ]; then
            cargo test -p ron-auth --features "$FEAT" --all-targets -- --nocapture
          else
            cargo test -p ron-auth --all-targets -- --nocapture
          fi
      - name: Golden vector parity
        run: cargo test -p ron-auth --test allow_deny_vectors -- --nocapture
```

---

## 14) Appendix

**Chosen Algorithms (target M3):**

* Sign: **hybrid** Ed25519 + ML-DSA (Dilithium2).
* KEM (external): **hybrid** X25519 + ML-KEM (Kyber-768).
* MAC/Hash: **Keyed BLAKE3-256** (unchanged).

**Libraries (anticipated):** `ed25519-dalek` v2 + `pqcrypto`/`liboqs-rust` adapters behind `SigAdapter`.

**Interop notes:** Hybrid signatures decode via `SigAdapter`; classical fallback allowed unless `pq_only=true`.

**Change log:**

| Date       | Change                                                                        | Owner |
| ---------- | ----------------------------------------------------------------------------- | ----- |
| 2025-10-04 | v2: perf wording clarified; ECON invariant added; PQ policy + CI matrix added | SW    |

```
```
