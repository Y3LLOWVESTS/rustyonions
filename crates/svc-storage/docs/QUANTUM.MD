
---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, security auditors, ops
crate: svc-storage
crate-type: service
pillar: 9                # Primary pillar: Integrity / Content Addressing (touches 10, 12)
owners: [Stevan White]
---

# QUANTUM.md — svc-storage

## 0) Purpose
Describe how `svc-storage` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops.  
Scope: algorithms in use, custody, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

### Public-key usage (breakable by Shor)
- **Transport/TLS 1.3:** ECDHE **X25519** key exchange; server certs (Ed25519 or ECDSA) depending on deployment.
- **Service identity/signing:** none required by storage itself beyond TLS; governance/audit systems may sign records (see cross-crate notes).
- **Tokens/capabilities:** **macaroons (HMAC-based)** (cap + caveats). No public-key dependency for verification.

**Impact if classical PKI breaks:**  
An adversary who records traffic today could later **decrypt TLS sessions** (HNDL) once ECC is broken—exposing request/response bodies and headers for **private** objects. Public objects see no confidentiality delta.

### Symmetric/Hash (Grover-affected only)
- **AEAD:** AES-256-GCM or ChaCha20-Poly1305 (256-bit security margin).  
- **Hashing / addressing:** **BLAKE3-256** (content addressing, ETag). Grover reduces the brute-force exponent, but **256-bit** outputs remain conservatively strong.

### Data at rest / long-lived artifacts
- **Objects:** by default immutable blobs on disk; **not** encrypted at rest unless enabled at deployment.
- **Manifests/indices/metrics/audit:** small metadata; may be retained **months→years**.
- **HNDL risk rating:**  
  - **Transit:** **Medium** (until hybrid PQ is enabled).  
  - **At rest (unencrypted):** **N/A** to “decryption later” (no ciphertext). Confidentiality is a *physical/OS custody* risk, not a PQ cryptanalytic risk.  
  - **At rest (encrypted deployments):** **Medium** if envelopes rely on classical KEX; mitigated by PQ-ready envelope (see §3).

### Transport/session lifetime
- HTTP over TLS; connections typically **seconds**; PFS helps but is still vulnerable to HNDL if ECDH falls to Shor later.

### Crate-specific blast radius
If classical PKI breaks and hybrid PQ is **not** enabled, an attacker with recorded traffic could later decrypt **private** PUT/GET bodies and metadata for this service. They **cannot** forge macaroons (HMAC) nor alter content addresses (BLAKE3).

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use**
  - **KEX / TLS:** TLS 1.3 with **X25519** ECDHE.
  - **Signatures:** Server certificate (Ed25519 or ECDSA, deployment-dependent).
  - **Symmetric/AEAD:** AES-256-GCM or ChaCha20-Poly1305.
  - **Hash:** **BLAKE3-256** for content addressing/ETag.
  - **Tokens:** **Macaroon v1** (HMAC-based, capability + caveats).
- **Libraries:** `tokio-rustls` (rustls), BLAKE3 crate, AEAD from RustCrypto or ring/AWS-LC via rustls; macaroons implementation (HMAC).
- **Key custody:** TLS private keys via file/KMS/HSM per deployment; macaroon root keys via KMS/HSM; rotation target **≤ 90 days**.
- **Interfaces carrying crypto:** TLS handshakes; capability tokens in HTTP headers; integrity digests in paths/headers.

---

## 3) Target PQ Posture (Where we’re going)

- **Key exchange / encryption (Transport):** adopt **Hybrid KEX** (e.g., **X25519 + ML-KEM (Kyber)**) as soon as our TLS stack supports it. Policy knob gates requirement per interface plane.
- **Signatures:** remain classical for TLS certs until vendor stacks ship PQ sigs; optional **ML-DSA (Dilithium)** or **SLH-DSA (SPHINCS+)** for **out-of-band artifacts** (e.g., signed purge receipts) via governance/audit crates.
- **At-rest encryption (optional deployments):** move to **PQ-ready envelope encryption**:
  - Data Encryption Key (DEK): AES-256-GCM.
  - Key Encapsulation/Wrap: **ML-KEM** for the KEK or **hybrid** (X25519+ML-KEM).
- **Tokens/capabilities:** keep **HMAC-macaroons** (Shor-immune); offer **PQ-signed tokens** only if tenant policy demands public-key tokens for federated verification.
- **Backwards compatibility:** classical remains supported through **M2 (Silver)**; hybrid becomes default at **M3 (Gold)** for external planes.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (svc-storage)
[features]
pq = []               # enable PQ plumbing (types/metrics)
pq-hybrid = ["pq"]    # prefer Hybrid KEX for TLS (when supported by transport)
pq-envelope = ["pq"]  # PQ-ready envelope for at-rest (DEK wrap via ML-KEM)
pq-only = []          # refuse classical peers (strict mode)
````

```ini
# Service config (svc-storage)
pq_hybrid = false          # M1 default; set true in M2 pilots; default true in M3
pq_only = false            # when true, classical-only peers are refused
pq_envelope = false        # enable PQ-ready at-rest envelope encryption
pq_sign_artifacts = "off"  # "off" | "ml-dsa" | "slh-dsa" for receipts/metadata
key_rotation_days = 90
```

**Interoperability:**

* If `pq_only=false`: negotiate hybrid when peer supports; otherwise use classical (emit downgrade metric).
* If `pq_only=true`: refuse classical; `/readyz` fails if policy requires PQ but stack/peer cannot.

---

## 5) Migration Plan (Milestones)

**M1 — Bronze (Hooks & Baselines)**

* Introduce feature flags, config knobs, and **metrics** (even if PQ is off).
* Baseline perf for classical handshakes & throughput.
* Unit tests compile under `--features pq,*` (mock adapters acceptable).

**M2 — Silver (Hybrid in the Edges)**

* Enable **hybrid KEX** on user-facing ingress once transport stack supports it.
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (allowed when `pq_only=false`).
* Optional: **pq_envelope=true** in a pilot cluster; measure overhead on PUT/GET.
* Record handshake overhead targets: **< 20%** vs classical; log actuals.

**M3 — Gold (Default Hybrid & Ops Playbooks)**

* Default **pq_hybrid=true** for external planes; publish runbook for rollback (flip to classical if incident).
* Offer **pq_only=true** for regulated tenants.
* If at-rest encryption is used, switch envelope KEK to **ML-KEM/hybrid** by default.
* QUANTUM/SECURITY docs finalized; chaos drill “classical break” ready.

**Post-M3 — De-risk & Deprecate**

* Periodically re-evaluate algorithm choices; pin library versions; plan path toward **PQ signatures** in TLS when stable.

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path remains **pure classical** when policy demands PQ: require **hybrid KEX** or **refuse** (`pq_only`).
* **[PQ-I2]** Symmetric security **≥ 256-bit**; hashing **≥ 256-bit** (BLAKE3-256).
* **[PQ-I3]** If at-rest encryption is enabled, DEKs **must** be re-encryptable under a **PQ-ready** envelope without data rewrite.
* **[PQ-I4]** Negotiation must be **auditable**; downgrades are measured and alertable.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback; rotations logged & testable.
* **[PQ-I6]** CI matrix includes PQ builds; interop parity proven for all supported modes.

---

## 7) Observability (Metrics, Logs, Readiness)

Export with algorithm labels:

* `pq_mode{plane="ingress"} = {off|hybrid|pq-only}`
* `pq_handshake_total{algo="x25519|ml-kem|hybrid", role="server|client"}`
* `pq_downgrade_total{reason}`              # peer lacked PQ / policy off / error
* `crypto_latency_seconds{op="kex|sign|verify", algo}`  # histogram
* `envelope_ops_total{op="wrap|unwrap", algo}`          # when pq_envelope=true
* **Readiness tie-in:** `/readyz` fails if `pq_only=true` and peer/stack cannot satisfy PQ.
* **Logs:** structured fields `pq=off|hybrid|pq-only`, `tls_kex=...`, `peer_mode=...`.

Dashboards must include: **downgrade rate**, handshake latency, envelope wrap/unwrap counts, and error taxonomy.

---

## 8) Testing & Verification

* **Unit/property:** token validators and frame/header parsers must pass under PQ feature builds; ensure no panics on negotiation edge cases.
* **Interop matrix (automated):**

  * classical↔classical (baseline),
  * hybrid↔hybrid (target),
  * hybrid↔classical (allowed iff `pq_only=false`; assert downgrade metric).
* **Fuzzing:** negotiation state machine (offers/accepts), PQ decoder inputs, envelope metadata parsing.
* **Load:** handshake/sec with/without hybrid; PUT/GET throughput with `pq_envelope=true`.
* **Security drills:**

  * **Classical break simulation:** force `pq_only=true`; legacy peers must fail closed with clear errors.
  * **Downgrade attack simulation:** ensure policy prevents unexpected downgrade when `pq_only=true`.

---

## 9) Risks & Mitigations

| Risk               | Description                           | Mitigation                                                         |
| ------------------ | ------------------------------------- | ------------------------------------------------------------------ |
| Perf overhead      | Hybrid KEX increases CPU/latency      | TLS session resumption; connection pooling; budget <20%            |
| Library churn      | PQ libs evolving                      | Thin **adapter traits**; version pinning; vendor gating            |
| Downgrade exposure | Peers fall back to classical silently | `pq_only` in regulated envs; `pq_downgrade_total` alerts           |
| At-rest complexity | Envelope adds I/O/CPU                 | Make optional; measure; roll back via config                       |
| Operational drift  | Mixed modes across clusters           | Metrics labels + `/readyz` policy; CI checks for policy compliance |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL risk labeled for transit/at-rest.
* [ ] `pq`, `pq-hybrid`, `pq-envelope` features compile; CI covers them.
* [ ] Hybrid KEX interop passes across all allowed combinations.
* [ ] PQ metrics emitted; downgrade alerts wired.
* [ ] Runbook updated (enable/rollback steps, incident drill).
* [ ] Perf overhead measured (handshake and PUT/GET deltas).
* [ ] SECURITY.md cross-links updated; owners ack.

---

## 11) Role Presets (svc-storage)

* **Primary targets:**

  * **Ingress TLS:** move to **hybrid KEX** → default at M3.
  * **At-rest envelope (optional):** if enabled, use **ML-KEM** (or hybrid) wrapping for DEKs.
  * **Tokens:** keep **HMAC macaroon** (no Shor exposure); PQ-signed tokens only if tenant policy needs public-key verifiability.

* **Defaults (per environment):**

  * **Macronode:** `pq_hybrid=true (M3)`, `pq_only=false` (except regulated tenants), `pq_envelope=false` unless policy requires.
  * **Micronode (amnesia mode):** typically no at-rest encryption (RAM-only); **focus on transport PQ** for HNDL mitigation.

---

## 12) Appendix (fill as adopted)

* **Algorithms chosen:**

  * **KEX:** `hybrid(X25519 + ML-KEM)` when available; fallback `X25519`.
  * **SIG (artifacts):** optional `ML-DSA` or `SLH-DSA` for receipts.
  * **AEAD/Hash:** AES-256-GCM or ChaCha20-Poly1305; **BLAKE3-256**.

* **Libraries:**

  * `tokio-rustls` (TLS), PQ adapters (to be selected when upstream stable), `blake3`, macaroon (HMAC).

* **Interop notes:**

  * Peers without PQ: allowed only if `pq_only=false`; emit downgrade metrics.
  * Gateway/overlay/index: mirror **pq_hybrid** defaults to avoid asymmetric policy.

* **Change log:**

  * 2025-10-04 — Initial QUANTUM.md; defined M1–M3 posture and metrics.
  * TBA — Enable hybrid by default (M3), add `pq_envelope` pilots.

```

---
