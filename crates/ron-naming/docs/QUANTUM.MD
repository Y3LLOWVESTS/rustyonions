

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, security auditors, ops
crate: ron-naming
crate-type: lib
pillar: 9
owners: [Stevan White]

# QUANTUM.md

## 0) Purpose

Define how `ron-naming`—a **pure library** (schemas/normalization/canonical encodings) with an **offline CLI** `tldctl`—remains safe under a PQ future.
Because this crate **does not perform network handshakes, token verification, or at-rest encryption**, our PQ focus is:

* **Provenance** of releases and **golden test vectors** (supply-chain integrity).
* **Compatibility hooks** for downstream crates that *do* perform crypto.
* Guardrails to avoid accidentally introducing classical-only crypto here.

Scope: algorithms (if any), release signing, optional vector attestation, tests, rollout plan, and Harvest-Now-Decrypt-Later (HNDL) exposure.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (breakable by Shor):**

  * **In-crate:** **None at runtime.** No TLS, no KEX, no signatures during normalization/encoding.
  * **Around the crate:** Git tag **release signing** (maintainer keys) and optional **attestation signatures** on golden vectors. If classical PKI breaks, attackers could spoof releases/vectors.
* **Symmetric/Hash (Grover-affected only):**

  * **In-crate:** Hashing may be used **only for internal IDs/tests** (e.g., vector IDs). We standardize on **BLAKE3-256** (256-bit security margin; Grover-style requires ~2^128). No AEAD is used here.
* **Data at rest / long-lived artifacts:**

  * **Golden vectors**, **baselines**, **SBOMs**, **CHANGELOG**—all public artifacts; **no secrets**.
  * Retention window: **years** (repo history). **HNDL risk: Low**—there’s nothing private to decrypt; provenance can be attacked if signatures are classical-only (mitigated below).
* **Transport/Session lifetime:** N/A (no transport).
* **Worst-case blast radius if classical PKI breaks:**

  * An adversary could distribute a **maliciously modified crate/tag** or **altered golden vectors**, tricking downstreams. Runtime resolution/data privacy is unaffected (those live in other crates).

> **HNDL:** Low. There are no confidential payloads produced by this crate; risk concentrates on **authenticity of releases**.

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use (runtime):** **None.**
* **Provenance (release process):**

  * Git tag **signatures** by maintainers (classical, e.g., Ed25519 via git tooling).
  * **SBOM + checksums** attached to releases.
  * Optional: **hashes (BLAKE3-256)** for golden vector bundles.
* **Libraries linked for crypto:** None required by the lib itself; only dev/release tooling.
* **Key custody (release signing):** Maintainer keys in KMS/HSM or hardware tokens; rotation policy 90 days or post-incident.
* **Interfaces that carry crypto:** N/A (library); `tldctl` does not produce or verify tokens/certs.

---

## 3) Target PQ Posture (Where we’re going)

For **ron-naming**, “PQ readiness” means **provenance hardening** and **clean handoff** to PQ-capable neighbors:

* **Provenance / Attestation**

  * Dual-track release signing: maintain classical signatures **and** add an **independent transparency mechanism** (e.g., transparency log or two-maintainer co-signature).
  * Optional **PQ signature** on **golden vector archives** (e.g., ML-DSA/Dilithium) stored alongside classical sigs so auditors can verify with either.
* **Downstream Interop Hooks**

  * Keep the crate **crypto-free**. Expose **stable DTOs** so `ron-auth`, `ron-kms`, `svc-passport`, and transports can layer **hybrid KEX** and **PQ signatures** in their domains without changes here.
* **Backwards compatibility**

  * PQ attestations are **additive**; classical signatures remain until the workspace reaches **M3 (Gold)** posture elsewhere.
  * `tldctl` remains offline/crypto-free; it may print **vector hash + provenance info** to help auditors verify artifacts signed elsewhere.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (no runtime crypto here; these are NO-OP placeholders to prevent drift)
[features]
pq = []               # Reserved: enables PQ attestation *metadata* emission (no crypto)
pq-attest = []        # Reserved: emits vector bundle hash & external-signature URIs
pq-hard-fail = []     # Reserved: CI-only, fail if classical-only provenance detected
```

```ini
# (No runtime config) Example CI/env toggles
RON_NAMING_ATTEST_HASH=blake3-256     # hash algo for vector bundles (default)
RON_NAMING_ATTEST_URI=https://…/sig   # pointer to external PQ signature produced by release infra
```

> All actual **signing** is performed by release infrastructure (e.g., `ron-kms` or CI HSM), **not** by this crate.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze) — Documentation & Guardrails**

  * Declare **no in-crate crypto**; add **anti-drift CI check** that fails if crypto crates are linked.
  * Hash and publish **golden vector bundles** (BLAKE3-256).
  * Release signing must be **mandatory** (classical), with keys documented and rotated.

* **M2 (Silver) — Attestation & Transparency**

  * Add **PQ attestation** (e.g., Dilithium) for **vector bundles** via external signer; publish sidecar `.sig-pq`.
  * Add **transparency log** entry or multi-maintainer co-signature requirement for releases.
  * CI verifies: (a) classical sig valid, (b) PQ attestation available & well-formed, (c) vector hash matches.

* **M3 (Gold) — Default & Policy**

  * Documentation and RUNBOOK require auditors to **verify PQ attestation** for vectors/releases.
  * Downstream crates (auth/transport/ledger) default to **hybrid**; `ron-naming` remains crypto-free but **references** required PQ verification steps in its release notes.
  * Quarterly **PQ drill**: simulate classical break (treat classical sigs as untrusted); auditors must be able to complete verification via PQ attestation/transparency only.

---

## 6) Invariants (MUST)

* **[PQ-N1] Crypto-free core:** No KEX, signatures, AEAD in `ron-naming` runtime.
* **[PQ-N2] Provenance required:** Every release and golden vector bundle must have **verifiable provenance** (at minimum classical signing + checksums).
* **[PQ-N3] PQ path available:** Provide a **PQ attestation** (external signer) or transparency proof for vector bundles by **M2**.
* **[PQ-N4] Anti-downgrade:** CI must fail if PQ attestation is marked “required” in workspace policy and is missing.
* **[PQ-N5] Stable DTOs:** Changes to DTOs/normalization are SemVer-gated; no hidden crypto material ever embedded.
* **[PQ-N6] No secrets:** CLI/library must never read secrets or private keys.

---

## 7) Observability (Metrics, Logs, Readiness)

* **CLI logs (`tldctl`):** on `--about`/`--vectors-info`, print: `hash_algo`, `vectors_hash`, `provenance_uri`, `pq_attestation_present={true|false}`.
* **CI artifacts:** attach `vectors.tar.zst`, `vectors.tar.zst.b3`, `vectors.tar.zst.sig` (classical), `vectors.tar.zst.sig-pq` (PQ).
* **Dashboards (repo level):** counts of releases with PQ attestation; last rotation date for maintainer keys.

---

## 8) Testing & Verification

* **Unit/property:** unaffected (no crypto).
* **Provenance checks (CI):**

  * Validate **BLAKE3 hash** of vector bundle; confirm presence of classical sig; optionally verify external **PQ signature** (out-of-band tool).
  * Negative tests: corrupt vectors → hash mismatch → CI fails.
* **Security drill (“classical break”):**

  * Treat classical sig as untrusted; verify **only** via PQ attestation/transparency; ensure auditors can reproduce verification with published tools.
* **No fuzz targets here** for crypto; fuzz stays focused on normalization/parse/encode paths.

---

## 9) Risks & Mitigations

* **Release spoofing if classical breaks:** Mitigate with **PQ attestation** and/or a **transparency log** requiring multiple identities.
* **Supply-chain library creep:** Prevent by **CI denylist** of crypto crates in `ron-naming`; crypto belongs to other crates.
* **Operational confusion:** Document clearly that `ron-naming` is **crypto-free**; PQ controls live in **release process** and **downstream services**.
* **Verifier tooling fragmentation:** Publish a **single verifier script** (repo `tools/verify_vectors.sh`) that checks hash + classical sig + optional PQ attestation.

---

## 10) Acceptance Checklist (DoD)

* [ ] README/SECURITY/RUNBOOK explicitly state **no runtime crypto** in `ron-naming`.
* [ ] Release process signs tags and publishes **SBOM + checksums**.
* [ ] Golden vector bundles produced with **BLAKE3-256**; hashes published.
* [ ] CI includes **anti-drift** (no crypto deps), vector hash check, and presence of signatures.
* [ ] **M2:** PQ attestation available for vectors; verifier script documented.
* [ ] Quarterly **PQ drill** executed; results logged.

---

## 11) Role Presets (how this crate aligns)

* **kernel/lib:** Crypto-agnostic; expose no keys, no handshakes.
* **transport/identity/econ (neighbors):** Implement **hybrid KEX**, **PQ signatures**, **pq_only** policies **outside** this crate; `ron-naming` DTOs remain stable to avoid re-signing churn.

---

## 12) Appendix

* **Algorithms:**

  * Hash for vector bundles: **BLAKE3-256** (deterministic, fast; 256-bit security level).
  * PQ attestation (external): **ML-DSA (Dilithium)** or **SLH-DSA (SPHINCS+)** as provided by release infra.
* **Libraries:** none linked in this crate; external signing handled by CI/KMS.
* **Interop notes:** Downstreams (e.g., `ron-auth`, `ron-kms`, transports) will negotiate **hybrid**; this crate stays unchanged.
* **Change log (provenance):**

  * 2025-Q4: Added vector hashing and signed bundles (classical).
  * 2026-Q1: Added optional PQ attestation; published verifier script.

---

**Bottom line:** `ron-naming` remains **crypto-free at runtime** and therefore has **negligible HNDL risk**. Our PQ readiness centers on **provenance**—making sure auditors can **trust what they build and test**, even if classical PKI weakens—while leaving heavy cryptographic negotiation and enforcement to the crates designed for it.
