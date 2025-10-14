

---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing — ron-ledger
status: draft
msrv: 1.80.0
last-updated: 2025-10-13
audience: contributors, security auditors, ops
crate: ron-ledger
crate-type: service|econ
pillar: 12               # P12 Economics & Wallets
owners: [Stevan White]
links:
  - SECURITY: ./SECURITY.md
  - INTEROP: ./INTEROP.md
  - API: ./API.md
  - IDB: ./IDB.md
  - OBSERVABILITY: ./OBSERVABILITY.md
  - RUNBOOK: ./RUNBOOK.md
  - PERFORMANCE: ./PERFORMANCE.md
---

# QUANTUM.md

## 0) Purpose
Describe how `ron-ledger` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or operations.  
Scope: algorithms in use, custody, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)
- **Public-key usage (Shor-breakable):**
  - **TLS 1.3 KEX:** X25519 (today) for API/metrics planes.
  - **Signatures:** Ed25519 for capability tokens (macaroon envelopes signed by KMS) and governance artifacts.
- **Symmetric / Hash (Grover-affected only):**
  - **AEAD:** ChaCha20-Poly1305 and/or AES-256-GCM (both 256-bit strength).
  - **Hash:** BLAKE3-256 for accumulators (option), SHA-256 in legacy Merkle profiles.
- **Data at rest / long-lived artifacts:**
  - **Ledger entries & roots** (immutable, public-verifiable), **governance artifacts**, **audit logs**. Entries are integrity-critical; confidentiality is not assumed, but **token material** must never appear in logs.
  - **Retention:** years → **HNDL risk: Medium** (if API sessions were recorded today and later decrypted, bearer **cap tokens in transit** could be exposed; entries themselves are integrity-only).
- **Transport/session lifetime:** seconds (API); minutes (OTLP). Short lifetimes reduce HNDL exposure but don’t eliminate it.
- **Crate-specific blast radius:** If classical PKI is broken, an attacker who recorded traffic could recover **session keys** ex post and read historical API exchanges (including any bearer tokens mistakenly transmitted). Ledger state/integrity remains anchored by append-only proofs but **operational confidentiality** of past sessions is at risk → mandate **Hybrid KEX → PQ**.

> **HNDL:** Can adversaries record now and decrypt later? **Yes** for classical TLS; mitigate with **hybrid KEX** as soon as peers allow.

---

## 2) Current Crypto Profile (Today)
- **Algorithms in use:**
  - **KEX:** X25519 (TLS 1.3 via rustls/tokio-rustls).
  - **Signatures:** Ed25519 (macaroons/caps via ron-kms; governance artifacts; release signing).
  - **AEAD:** ChaCha20-Poly1305 (default) and/or AES-256-GCM.
  - **Hash/Accumulator:** Merkle(SHA-256) default; **BLAKE3-256** optional feature.
- **Libraries:** `rustls`, `ring`/`rust-crypto` for AEAD, `ed25519-dalek`, `blake3`.
- **Key custody:** Keys live in **ron-kms/HSM**; app sees KIDs only (no raw keys). Rotation ≤ 90 days (caps ≤ 30 days).
- **Interfaces carrying crypto:** HTTP API (TLS), metrics (TLS or loopback), macaroons in `Authorization` headers, governance JSON artifacts with signatures.

---

## 3) Target PQ Posture (Where we’re going)
- **KEX / Encryption:** **Hybrid TLS** — **X25519 + ML-KEM (Kyber)** during handshake; session AEAD unchanged (ChaCha20/AES-256).
- **Signatures:** **ML-DSA (Dilithium)** preferred; **SLH-DSA (SPHINCS+)** fallback for defense-in-depth (stateless, larger).
- **Transport TLS:** Classical today → enable **hybrid KEX** when `pq_hybrid=true`. Long-term: `pq_only=true` for privileged planes (sequencer/admin), classical allowed for public read-only until ecosystem catches up.
- **Tokens/Capabilities:** Support **PQ-signed macaroons**; negotiate per-tenant. Classical Ed25519 accepted until **M3**; after that, default **ML-DSA** for high-sensitivity caps.
- **Backwards compatibility:** Classical remains supported until **M3 (Gold)**; **M3+ defaults hybrid**; **M4** enables `pq_only` on privileged.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features for ron-ledger
[features]
pq = []                 # enable PQ plumbing
pq-hybrid = ["pq"]      # TLS Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]        # enable PQ signatures (ML-DSA/Sphincs+ via ron-kms)
pq-blake3 = []          # prefer BLAKE3-256 accumulator by default
pq-only = []            # compile-time hard-fail classical (privileged planes)
````

```ini
# ron-ledger Config (excerpt)
[tls]
enable = true
pq_hybrid = false          # M2: opt-in; M3: default true on app-facing
pq_only = false            # refuse classical peers when true (M4 privileged)
min_version = "1.3"

[pq]
sign_algo = "ml-dsa"       # "ml-dsa" | "slh-dsa"
require_pq_caps = false    # require PQ-signed caps for writes on privileged planes

[rotation]
cap_days = 30
key_days = 90
```

* **Interoperability:** If peer lacks PQ → negotiate classical **unless** `pq_only=true` (then clear 4xx/alert).
* **Metrics:** always emit PQ counters with `algo` label even if value=0.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Hooks & Inventory**

* Add `pq`, `pq-hybrid`, `pq-sign` features and config stubs (no behavior change).
* Inventory HNDL exposure; verify no secrets in logs (SECURITY redaction).
* CI builds with PQ features; mock KMS returns PQ signatures.

**M2 (Silver) — Hybrid Enablement**

* Enable **Hybrid TLS KEX** in service planes when `pq_hybrid=true`.
* Add PQ-signed macaroon support in verification path (via ron-kms); dual-accept Ed25519 + ML-DSA.
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (write ops allowed only when policy permits).
* Perf budget: handshake overhead **≤ 20%** (record p50/p95), no steady-state TPS regression > **5%**.

**M3 (Gold) — Default Hybrid & Operationalization**

* Default `pq_hybrid=true` on app-facing; privileged planes **require PQ** for writes unless override.
* Governance artifacts: default **ML-DSA** signatures.
* Runbook: enable/rollback steps; feature flags; alert tuning; PQ dashboards.

**Post-M3 — De-risk & De-precate**

* Environments with `pq_only=true` for privileged planes; phase-out classical caps.
* Periodic re-eval of algorithms/libraries; track NIST/impl guidance.

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical paths (write auth, governance, privileged admin) use **Hybrid KEX** at minimum; no pure ECC/RSA once `pq_hybrid=true`.
* **[PQ-I2]** Symmetric strength **≥ 256-bit**; hashes **≥ 256-bit** (BLAKE3-256 or SHA-256 acceptable).
* **[PQ-I3]** Long-lived data is integrity-anchored by PQ-upgradable proofs; no secrets written to ledger/audit.
* **[PQ-I4]** When `pq_only=true`, classical peers **must** fail with explicit error and metric.
* **[PQ-I5]** Rotation workflows must upgrade **algorithms and keys** without silent downgrade; reject mixed/invalid chains.
* **[PQ-I6]** PQ builds pass CI; interop parity with classical proven by canonical vectors.

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics (Prometheus):**

* `pq_handshake_total{algo="x25519|ml-kem|hybrid", role="client|server"}`
* `pq_handshake_failures_total{reason}` — `no_common_algo|policy_refused|internal`
* `pq_signature_total{algo="ed25519|ml-dsa|slh-dsa", op="sign|verify"}`
* `crypto_latency_seconds{op="kex|sign|verify", algo}` (histogram)
* `pq_policy_mode{mode="off|hybrid|pq-only"}` gauge 0/1
* `pq_downgrade_detected_total` (ClientHello offered PQ, server selected classical)

**Readiness:**

* `/readyz` **fails** if policy requires PQ (`pq_only=true` or `require_pq_caps=true`) and negotiation cannot establish PQ guarantees.

**Logs (structured):**

* Include `pq.mode`, `kex.algo`, `sig.algo`, `peer.mode`, `downgrade=true/false` (no key material).

---

## 8) Testing & Verification

**Unit / Property:**

* Token verifier accepts **Ed25519 & ML-DSA** per policy; rejects mixed/invalid chains.
* TLS adapter selects **hybrid** when enabled; property test “offered ∧ enabled ⇒ selected”.

**Interop (tests/vectors):**

* classical↔classical (baseline)
* hybrid↔hybrid (writes allowed)
* hybrid↔classical (reads OK; writes gated unless policy allows)
* pq_only↔classical (expect clear 4xx/metrics/alert)

**Fuzzing:**

* PQ ClientHello/ServerHello negotiation paths; malformed PQ params → clean reject.
* PQ signature decoders (ASN.1/CBOR envelope if used) — no panics.

**Load / Perf:**

* Handshake/sec and p95 handshake latency with/without PQ; TPS vs steady-state (no >5% regression target).

**Security Drills:**

* Simulate “classical break”: set `pq_only=true`; confirm safe failure paths and alerting.

---

## 9) Risks & Mitigations

* **Perf & footprint:** Larger keys/certs, slower handshakes → enable **session resumption**, **connection pooling**, tune TLS ticket lifetimes; cache macaroons’ verification keys.
* **Library churn:** PQ crates evolve → wrap behind **adapter traits**, pin workspace versions, add SBOM notes, run **cargo-deny** advisories.
* **Downgrade attacks:** Enforce policy (`pq_only`) on privileged; emit `pq_downgrade_detected_total` and alert on spikes.
* **Ecosystem gaps:** Keep **dual-accept** on signatures until partners upgrade; publish **SDK guidance**.

---

## 10) Acceptance Checklist (DoD)

* [ ] HNDL risk labeled (**Medium**) and documented.
* [ ] CI matrix includes `--features pq,pq-hybrid,pq-sign,pq-blake3`.
* [ ] Hybrid KEX interop green; pq_only errors are explicit and observable.
* [ ] PQ metrics visible in dashboards; alerts for downgrade & policy violations.
* [ ] RUNBOOK updated with enable/rollback.
* [ ] PERF numbers captured (handshake p50/p95, CPU/RAM deltas).
* [ ] SECURITY cross-links updated; owners ACK.

---

## 11) Role Presets (ron-ledger specifics)

* **Service plane (API/ingest/roots):**

  * **Target:** Hybrid TLS KEX; PQ-signed caps optional M2, recommended M3; reads may allow classical until M4.
  * **Default:** `pq_hybrid=false` (M1) → `true` (M3). `pq_only=false` except privileged.

* **Privileged plane (sequencer/admin/UDS):**

  * **Target:** `pq_only=true` by M4; reject classical; require PQ-signed caps.
  * **Default:** `pq_hybrid=true` (M3), `require_pq_caps=true` for writes.

* **Artifacts (governance/audit):**

  * **Target:** ML-DSA signatures; include `sig_algo` in artifact headers.
  * **Default:** dual-sign (Ed25519 + ML-DSA) during M2→M3 cutover.

* **Accumulator/roots:**

  * **Target:** Prefer **BLAKE3-256** (pq-agnostic, 256-bit).
  * **Default:** `pq-blake3` on for new deployments; legacy Merkle(SHA-256) allowed with migration plan.

---

## 12) Appendix

**Chosen algorithms (initial):**

* **KEX:** Hybrid(X25519 + ML-KEM)
* **SIG:** ML-DSA (Dilithium-2 or -3 tier based on payload size); SLH-DSA(SPHINCS+) fallback on request
* **AEAD:** ChaCha20-Poly1305 (default), AES-256-GCM (alt)
* **HASH:** BLAKE3-256 (accumulator), SHA-256 (legacy)

**Libraries (subject to workspace vetting):**

* `rustls` (hybrid KEX via provider integration), `oqs`/`liboqs-rust` or equivalent (behind adapter), `ed25519-dalek`, `blake3`.

**Interop notes:**

* Peers advertise `pq_mode={off|hybrid|pq-only}` via readiness and OpenAPI extension; SDKs default `hybrid` when available.

**Change log:**

* 2025-10-13 — Initial PQ charter; M1 hooks defined; hybrid roadmap adopted.

```
```
