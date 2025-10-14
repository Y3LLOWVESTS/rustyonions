
---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-14
audience: contributors, security auditors, ops
crate: svc-rewarder
crate-type: service|econ
pillar: 9               # Economic Integrity / Rewards
owners: [Stevan White]
review-cadence: quarterly or on PQ algo/library updates (Perf Board sign-off per GOVERNANCE §4)

# QUANTUM.md — `svc-rewarder`

## 0) Purpose

Describe how `svc-rewarder` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops. Scope covers algorithms, custody, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (Shor-vulnerable):**

  * Transport: TLS 1.3 (rustls) with **X25519** KEX (classical).
  * Signatures: **Ed25519** for macaroons/caps and any local attest/audit signing.
* **Symmetric/Hash (Grover-affected only):**

  * AEAD: **ChaCha20-Poly1305** and/or **AES-256-GCM** (256-bit security target).
  * Hash: **BLAKE3-256** (content addressing).
* **Data at rest / long-lived artifacts:** reward results digests, intent envelopes, audit lines/attestations.
  **Retention:** audit artifacts up to **7 years**; results digests **12–36 months** → **HNDL risk:** **Low→Medium** (integrity-centric, little confidentiality).
* **Transport/session lifetime:** seconds–minutes (short; lowers HNDL risk).
* **Worst-case blast radius if classical PKI breaks:** forged caps/tokens or TLS MITM could inject bogus results; **ledger idempotency + conservation** still gate settlement, but risk rises. PQ/hybrid closes this class.

---

## 2) Current Crypto Profile (Today)

* **Algorithms:** KEX: X25519; Sig: Ed25519; AEAD: ChaCha20-Poly1305/AES-256-GCM; Hash: BLAKE3-256.
* **Libraries (examples):** `tokio-rustls 0.26.x`, `ed25519-dalek 2.1.x`, `blake3 1.5.x`, `chacha20poly1305 0.10.x`, `aes-gcm 0.10.x`.
* **Key custody:** governance/signing keys in **ron-kms/HSM**, rotation ≤ **90 days**, escrowed in KMS backup with **4/5** quorum; no raw keys in env/files.
* **Interfaces using crypto:** HTTP/TLS; macaroons/caps; audit/attestation signatures; content-addressed manifests.

---

## 3) Target PQ Posture (Where we’re going)

* **KEX/Encryption:** **Hybrid** = X25519 + **ML-KEM (Kyber/ML-KEM)** when enabled; classical fallback allowed until M3.
* **Signatures:** **ML-DSA (Dilithium)** primary; **SLH-DSA (SPHINCS+)** as fallback (useful for low-memory Micronodes).
* **Transport TLS:** classical now → **hybrid** when `pq_hybrid=true`.
* **Tokens/Capabilities:** PQ-signed macaroons supported; per-tenant policy negotiates acceptance.
* **Back-compat:** classical supported until **M3 (Gold)**; then **hybrid default** on external edges; `pq_only=true` available for high-assurance tenants.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (illustrative)
[features]
pq = []                 # enable PQ plumbing
pq-hybrid = ["pq"]      # hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]        # PQ signatures (ML-DSA / SLH-DSA)
pq-only = []            # compile-time hard refusal of classical peers (optional)
```

```ini
# svc-rewarder config (maps to CONFIG.md)
pq_hybrid        = false      # M1 off → M2 optional → M3 default true (external edges)
pq_sign_algo     = "ml-dsa"   # "ml-dsa" | "slh-dsa"
pq_only          = false      # if true, refuse classical handshakes/tokens
key_rotation_days = 90
```

* **Hot-reload:** `pq_hybrid` and `pq_sign_algo` reloaded via **SIGHUP/SIGUSR1** (see RUNBOOK/CONFIG).
* **Interop switch:** if peer lacks PQ, we **negotiate** to classical unless `pq_only=true`.
* **Metrics:** always emit PQ-labeled metrics (zero when disabled) to ease adoption dashboards.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze) — Hooks & Baselines**

  * Add `pq`, `pq-hybrid`, `pq-sign` gates; no behavior change.
  * Baseline classical perf (handshake, sign/verify, CPU/RAM).
  * CI compiles/tests with PQ features (mock adapters acceptable).

* **M2 (Silver) — Hybrid Enablement**

  * Enable **Hybrid KEX** (`pq_hybrid=true`) where upstream supports it.
  * Add **PQ signatures** for caps/receipts behind `pq-sign`; accept classical + PQ.
  * Interop matrix: classical↔classical; hybrid↔hybrid; hybrid↔classical (fallback policy).
  * Perf budgets: handshake overhead **≤ +20%**; sign/verify p95 **≤ +5 ms**; service p95 **≤ +5%**.

* **M3 (Gold) — Defaults & Ops**

  * Default **hybrid** on external edges; internal planes transition as partner coverage ≥ 95%.
  * PQ signatures **mandatory** for governance/receipts; caps PQ by tenant opt-in.
  * One-click rollback; alerts on any downgrade in `pq_only` zones.
  * Target **timeline:** **Q4 2026** (subject to partner readiness via governance proposals).

* **Post-M3 — De-risk & Deprecate**

  * Offer `pq_only=true` for high-assurance tenants; sunset pure classical at edges.
  * Annual PQ review (12-month cadence) and dependency refresh.

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical edges SHALL NOT remain **pure classical** where policy mandates hybrid; enforce via readiness.
* **[PQ-I2]** Symmetric keys **≥ 256-bit**; hashes **≥ 256-bit** (BLAKE3-256 OK).
* **[PQ-I3]** Long-lived archives (audit/receipts) become verifiable under PQ signatures; verified via **ML-DSA** entries in `ron-audit`.
* **[PQ-I4]** If `pq_only=true`, refuse classical peers with explicit error + metric.
* **[PQ-I5]** Key rotation is PQ-aware; no silent downgrade during rotation.
* **[PQ-I6]** CI matrix covers PQ ON/OFF; interop parity proven (no functional drift).
* **[PQ-I7]** **Downgrade detection**: any classical fallback in PQ-mandated zones must raise `pq_downgrade` metrics and page SRE.

---

## 7) Observability (Metrics, Logs, Readiness)

* **Metrics** (labeled by `algo`, `mode`):
  `pq_handshake_total{mode="classical|hybrid|pq-only",role="server"}`
  `pq_handshake_latency_seconds_bucket{algo="x25519|ml-kem|hybrid"}`
  `pq_signature_total{algo="ed25519|ml-dsa|slh-dsa",op="sign|verify"}`
  `pq_signature_failures_total{reason}`
  `pq_downgrade_total{zone="external|internal"}`
  `crypto_latency_seconds{op="kex|sign|verify",algo}`

* **Readiness:** `/readyz` **fails** if policy mandates PQ and we/peer can’t meet it; otherwise **degrade** with `readyz_degraded{cause="pq_downgrade"}`.

* **Alerts:**

  * **Page:** `pq_downgrade_total > 0` sustained **5 min** in a `pq_only` zone; or readiness fails due to PQ policy.
  * **Ticket:** any PQ signature failure burst (`pq_signature_failures_total > 3/min` for 10 min).

* **Logs:** structured fields include `pq_mode={off|hybrid|pq-only}`, `kex=…`, `sig=…`, `peer_mode=…`, `tenant_policy=…`.

* **Retention:** audit logs retained **7 years** (see GOVERNANCE §5).

---

## 8) Testing & Verification

* **Unit/Property:** token verifiers and parsers under PQ & classical; determinism/idempotency invariant unchanged across PQ toggle.
* **Interop suite:** classical↔classical; hybrid↔hybrid; hybrid↔classical (policy-gated).
* **Fuzz:** negotiation/state-machine TLVs (`fuzz_targets/pq_negotiation.rs`), PQ decoders, error taxonomy.
* **Perf/Load:** handshake/sec and p95 latencies with PQ ON/OFF; ensure service p95 delta **≤ +5%** (PERFORMANCE).
* **Mobile/Micronode:** PQ ON under 2 vCPU/2 GiB; validate cold-start and first compute p95; track RAM delta (**<10%** target).
* **Security drill:** simulate “classical break” → set `pq_only=true`; verify safe refusal + alerts + clean dashboards.

---

## 9) Risks & Mitigations

| Risk                          | Impact              | Mitigation                                                                 |
| ----------------------------- | ------------------- | -------------------------------------------------------------------------- |
| Larger keys/slower handshakes | CPU spikes, latency | Session reuse, pooling; hybrid at edges first; perf budgets in CI          |
| PQ library churn              | Build fragility     | Adapter traits; pin versions; CI interop; periodic review                  |
| Silent downgrade              | Security gap        | `pq_only` where mandated; `pq_downgrade_*` metrics + paging                |
| Partner lag                   | Interop friction    | Tenant opt-in via governance proposals; staged rollout                     |
| Edge/mobile footprint         | Startup/CPU/RAM hit | Pre-warm pools; reduce workers by 1 when PQ ON; enforce RAM **<10%** delta |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL risk Low→Medium** justified.
* [ ] `pq`, `pq-hybrid`, `pq-sign` compile; CI matrix includes PQ ON/OFF.
* [ ] Hybrid KEX interop passes; clear refusal on `pq_only=true`.
* [ ] PQ metrics live; alerts wired (`pq_downgrade`, `pq_only_refusal`).
* [ ] Runbook updated (enable/rollback; downgrade & brownout playbooks).
* [ ] Perf deltas recorded (handshake, sign/verify, CPU/RAM); service p95 delta **≤ +5%** with PQ ON.
* [ ] Mobile PQ validated (Micronode 2 vCPU/2 GiB): cold-start, first compute p95, RAM delta **<10%**.
* [ ] SECURITY & GOVERNANCE cross-links updated; Perf Board sign-off recorded.

---

## 11) Role Presets (svc-rewarder focus)

* **Transport:** adopt **hybrid KEX** once gateway supports; internal mesh can lag until partners ready.
* **Tokens/Cap:** support **PQ-signed macaroons**; **tenant opt-in via GOVERNANCE §4** proposals.
* **Receipts/Audit:** **ML-DSA** signatures for receipts; algo labels in audit lines.
* **Custody:** all signing in **ron-kms/HSM**; rotation ≤ 90 days; PQ mode pinned in attestation.
* **Perf knob:** when PQ ON, consider **reducing compute workers by 1** to keep mean CPU < 70% (PERFORMANCE).

---

## 12) Appendix (fill as adopted)

* **Algorithms chosen:** KEX = `hybrid(X25519 + ML-KEM)`; SIG = `ML-DSA` primary, `SLH-DSA` fallback.
* **PQ libraries (illustrative):** `pqclean-rust 0.9.x` or `liboqs-rust` adapter; `tokio-rustls 0.26.x`; `ed25519-dalek 2.1.x`.
* **Interop notes:** peer stacks supporting PQ; fallback policy by tenant.
* **Change log:** date → change (e.g., “2026-Q4: default pq_hybrid=true on external edges”).

---

