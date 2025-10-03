

---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-03
audience: contributors, security auditors, ops
crate: svc-gateway
crate-type: service
pillar: 3
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose
Define how `svc-gateway` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** without breaking interop or ops.  
Scope: algorithms, custody, runtime knobs, telemetry, tests, rollout plan, and **Harvest-Now-Decrypt-Later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

**Public-key usage (Shor-breakable today)**
- **TLS 1.3 KEX:** X25519 (ephemeral ECDH).
- **Signatures:** Ed25519 used indirectly for verifying upstream tokens/receipts (via policy/auth libs), not minted by gateway.
- **Capabilities/Macaroons:** bearer-style with Ed25519 verification (short TTLs); gateway **must not** log tokens.

**Symmetric/Hash (Grover-affected only)**
- **AEAD:** ChaCha20-Poly1305 (or AES-256-GCM if selected by TLS).
- **Hashing:** BLAKE3-256 for addressing/IDs; SHA-256 may appear in third-party TLS cert chains but is not the addressing canon.

**Data at rest / long-lived artifacts (inside the gateway)**
- Gateway is **stateless** by design: no user payloads stored; only ephemeral buffers and short-retention logs/metrics.
- **Retention window:** logs ≤ 30–90 days (policy-driven), **with secrets redacted**. Payload bodies not persisted.  
  → **HNDL risk: Low** within the gateway. (Downstream services—index/storage/mailbox—carry their own HNDL plans.)

**Transport / Session lifetime**
- TLS sessions are **short-lived (seconds to minutes)**; session resumption keys rotate frequently. Short lifetimes reduce HNDL payoff.

**Crate-specific blast radius (if classical PKI is broken)**
- Worst case: recorded TLS traffic to the gateway could be decrypted in a PQ future if only classical X25519 was used.  
  Practical impact is limited to **transit confidentiality** of requests/responses at the edge; no long-term artifacts are held by the gateway.

> **HNDL note:** For ingress, the dominant mitigation is **hybrid PQ KEX** in TLS and minimizing logged material (no tokens, no payloads).

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use**
  - **KEX:** X25519 (TLS 1.3).
  - **Signatures:** Ed25519 (verify-only for macaroons/receipts via upstream libs).
  - **Symmetric:** ChaCha20-Poly1305 (primary), AES-256-GCM (optional).
  - **Hash:** BLAKE3-256 (content addressing / IDs), SHA-256 possibly in external PKI chains.

- **Libraries**
  - `tokio-rustls` / `rustls` (TLS 1.3).
  - `ed25519-dalek` (verify only, when capability verification is local).
  - `blake3`, `ring`/`hpke` (where applicable via transport/auth stacks).

- **Key custody**
  - **TLS server keys:** in `ron-kms`/HSM or OS keystore; never raw in env/files in prod.
  - **Gateway does not mint tokens**; verification keys fetched from KMS/policy and cached with short TTL.
  - **Rotation:** TLS keys ≥ every 90 days; verification keys per policy TTL.

- **Interfaces carrying crypto**
  - TLS endpoints, capability tokens (authorization headers), OAP/1 frames (transit only), audit event signatures (verify path).

---

## 3) Target PQ Posture (Where we’re going)

- **KEX / encryption (Transport)**
  - **Hybrid TLS KEX:** **X25519 + ML-KEM (Kyber)** when `pq_hybrid = true`.  
    *Goal:* neutralize HNDL for edge ingress without breaking classical peers.

- **Signatures**
  - Support verification for **ML-DSA (Dilithium)** and/or **SLH-DSA (SPHINCS+)** for capability/receipt verification as upstreams adopt PQ.  
  - Gateway remains **verify-only**; minting stays in `ron-auth` / `svc-passport` / `ron-kms`.

- **Transport TLS**
  - Classical default now; enable **hybrid** per env.  
  - **M3 (Gold):** default `pq_hybrid = true` on public edges where peer stacks are ready.

- **Tokens/Capabilities**
  - **Dual-rail verification:** accept PQ-signed tokens when present; fall back to Ed25519 unless `pq_only=true`.

- **Backwards compatibility**
  - Classical remains supported until **M3** default shift; enforcement controlled via config and feature flags.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (crate-local)
[features]
pq = []                 # enable PQ plumbing
pq-hybrid = ["pq"]      # hybrid TLS KEX adapters
pq-sign = ["pq"]        # enable PQ signature verification adapters
pq-only = []            # (optional) compile-time hard refuse classical
````

```ini
# svc-gateway Config (snippet)
pq_hybrid = false            # M1 default; set true per env starting M2/M3
pq_sign_algo = "ml-dsa"      # "ml-dsa" | "slh-dsa" | "off"
pq_only = false              # if true, refuse classical handshakes/tokens
key_rotation_days = 90
```

* **Interoperability:** if peer lacks PQ → **negotiate classical** (log downgrade) unless `pq_only=true` (then refuse).
* **Metrics:** always emit PQ-labeled counters/histograms even when disabled (value=0) to ease adoption.

---

## 5) Migration Plan (Milestones)

**M1 — Bronze (Hooks & Hygiene)**

* Add feature flags (`pq`, `pq-hybrid`, `pq-sign`) and config knobs; no default behavior change.
* Ensure **no secrets** in logs; scrub tokens/keys; shorten TLS session ticket lifetimes.
* Baseline perf for classical TLS; compile tests with PQ features (mock adapters acceptable initially).

**M2 — Silver (Enablement & Interop)**

* Enable **Hybrid TLS KEX** behind `pq_hybrid=true` in environments where upstream LB/clients support it.
* Introduce **PQ verify adapters** for tokens/receipts; interop matrix:

  * classical↔classical, hybrid↔hybrid, hybrid↔classical (downgrade logged).
* Perf target: ≤ 10–20% handshake overhead; measure & record.

**M3 — Gold (Default & Ops)**

* Default `pq_hybrid=true` for public ingress planes; provide `pq_only` for high-assurance tenants.
* Accept **PQ-signed caps/receipts** (verify path) by default; classical accepted unless policy forbids.
* Update RUNBOOK (enable/rollback playbooks); SECURITY (threat model) finalized.

**Post-M3 — De-Risk & Sunsets**

* Gradually enable `pq_only=true` for regulated tenants; sunset pure classical at edges where safe.
* Periodic PQ algorithm re-evaluation; prepare for new NIST levels/params.

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path remains **pure classical** when policy demands PQ/hybrid.
* **[PQ-I2]** Symmetric strengths at **256-bit**; hashes at **≥ 256-bit** (BLAKE3-256).
* **[PQ-I3]** Gateway stores **no long-lived plaintext**; any cached verification keys come from KMS and have short TTLs.
* **[PQ-I4]** If `pq_only=true`, classical handshakes/tokens are **refused with clear error taxonomy**.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback (audit event must include old/new algos).
* **[PQ-I6]** CI matrix runs `--features pq,pq-hybrid,pq-sign` and exercises classical↔hybrid interop.

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics**

* `pq_handshake_total{mode="off|classical|hybrid|pq-only", algo="x25519|ml-kem|hybrid"}`
* `pq_signature_verify_total{algo="ed25519|ml-dsa|slh-dsa", result="ok|fail"}`
* `crypto_latency_seconds{op="kex|sign|verify", algo}` (histogram)
* `pq_downgrade_total{reason="peer-no-pq|policy|error"}`

**Logs (structured)**

* Include `pq_mode`, negotiated `kex_algo`, `sig_algos_supported`, `peer_mode`, and downgrade reasons.
* **Never** log raw tokens or keys.

**Readiness**

* `/readyz` **fails** if policy mandates PQ (`pq_only=true`) and negotiation cannot establish PQ/hybrid.

---

## 8) Testing & Verification

* **Unit/property:** negotiation state machine, header/token acceptance rules, error taxonomy for `pq_only` refusals.
* **Interop:** matrix across classical/hybrid peers; verify downgrade logging and metrics.
* **Fuzz:** PQ negotiation decoders; token parser under mixed algos; TLS extension parser (if locally handled).
* **Load:** handshake/sec with hybrid vs classical on x86_64 and ARM; measure CPU/RAM deltas.
* **Security drill:** simulate “classical break” day—flip `pq_only=true` in staging; ensure clean refusals and stable error taxonomy.

---

## 9) Risks & Mitigations

* **Perf/footprint:** hybrid increases handshake cost → enable keep-alive, session resumption, and tune accept backlog.
* **Library churn:** PQ stacks evolve → wrap behind **thin adapter traits**; pin and audit versions at workspace root.
* **Downgrade abuse:** attackers force classical → alert on `pq_downgrade_total`; tenants can enforce `pq_only=true`.
* **Ecosystem gaps:** older clients lack PQ → default hybrid is opt-in per env until coverage is sufficient.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL risk = Low** for gateway; notes added.
* [ ] Features compile: `pq`, `pq-hybrid`, `pq-sign`; CI matrix green.
* [ ] Interop tests pass (classical↔classical, hybrid↔hybrid, hybrid↔classical).
* [ ] PQ metrics/log fields emitted; dashboards updated.
* [ ] RUNBOOK updated with enable/rollback steps; SECURITY updated.
* [ ] Perf numbers recorded (handshake, verify, CPU/RAM).
* [ ] KMS-backed custody confirmed; rotation policy enforced.

---

## 11) Role Presets (svc-gateway)

* **Primary Targets:** policy-driven negotiation at the edge; `/readyz` reflects PQ policy; verify-only for PQ signatures; no token minting.
* **Defaults:** `pq_hybrid=false` (M1) → env-opt-in (M2) → **true by default (M3)** where peers support it; `pq_only=false` except for high-assurance tenants.

---

## 12) Appendix

**Algorithms Chosen (initial)**

* **KEX:** X25519 (now); **Hybrid X25519+ML-KEM** (target).
* **SIG:** Ed25519 (verify) now; **ML-DSA** / **SLH-DSA** verify when available.
* **AEAD:** ChaCha20-Poly1305 (pref) / AES-256-GCM (alt).
* **HASH:** BLAKE3-256.

**Libraries (subject to workspace pinning)**

* `tokio-rustls`/`rustls` (TLS), `ed25519-dalek`, `blake3`, optional PQ adapters (`oqs-sys`/equiv) behind features.

**Interop Notes**

* LB/clients must advertise hybrid KEX ciphersuites; otherwise classical is negotiated (downgrade logged).
* For regulated tenants, enable `pq_only=true` and distribute client guidance.

**Change Log**

* 2025-10-03: Draft QUANTUM.md; added flags/knobs; set M1→M3 plan.

```

---
```
