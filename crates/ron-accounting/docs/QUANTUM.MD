

---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-16
audience: contributors, security auditors, ops
crate: ron-accounting
crate-type: service|econ (supporting: metering/export)
pillar: 7        # Economics / Integrity
owners: [Stevan White]
---

# QUANTUM.md — ron-accounting

## 0) Purpose
Describe how `ron-accounting` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto **without breaking interop or ops**.  
Scope covers: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

`ron-accounting` is **transient**: it aggregates usage (requests/bytes/time-slices) and exports **idempotent batches** to `ron-ledger` over **UDS** (peer-cred) or in-proc bus. It **does not** hold durable economic truth or long-lived keys.

---

## 1) Exposure Assessment (What’s at risk?)

- **Public-key usage (breakable by Shor)**
  - **Key exchange:** Mostly **not used** directly; primary link to `ron-ledger` is **UDS**. Any TLS edges (admin/metrics if exposed beyond loopback) use **TLS 1.3** with **X25519** KEX today.
  - **Signatures:** Capability tokens (macaroon-style) are signed upstream (e.g., `ron-auth`) with **Ed25519**. This crate **verifies** but does **not issue** tokens.
- **Symmetric / Hash (Grover-affected only)**
  - **AEAD:** If TLS terminates here (rare), it’s **AES-256-GCM** or **ChaCha20-Poly1305** (rustls defaults).  
  - **Hash:** **BLAKE3-256** for content-addressing / batch hashing.
- **Data at rest / long-lived artifacts**
  - Default **amnesia**: no durable state. Optional WAL/cache (if enabled) holds **ephemeral** batches (minutes/hours) → HNDL **low**. No customer payloads.
- **Transport/session lifetime**
  - UDS (local) with peer-cred: no KEX. HTTP admin loopback: short-lived TLS sessions (seconds/minutes) if enabled.
- **Crate-specific blast radius (if classical PKI/signatures were broken)**
  - Worst case: forged capability tokens could trick `ron-accounting` into accepting bogus usage **proposals**; however `ron-ledger` still enforces **idempotency** and downstream reconciliation detects anomalies. Impact: **temporary misbilling pressure** until dispute/compensation; no loss of durable truth.

> **HNDL Risk:** **Low** for this crate (short sessions, no durable secrets, amnesia defaults). Elevated **indirect** risk exists via upstream token/signature verifiers until hybrid/PQ signatures are adopted system-wide.

---

## 2) Current Crypto Profile (Today)

- **Algorithms**
  - **KEX:** X25519 (only on any TLS edges; UDS path is default).
  - **Signatures:** Ed25519 (cap tokens; verified, not minted here).
  - **Symmetric/Hash:** AES-256-GCM or ChaCha20-Poly1305 (TLS); BLAKE3-256 (hashing).
- **Libraries**
  - **tokio-rustls** (TLS 1.3 where applicable), **ed25519-dalek**, **blake3**.
- **Key custody**
  - **No private keys here.** Any needed signing (rare) is delegated to **ron-kms/HSM**. Verification keys loaded via config/policy; rotate ≤ 90 days.
- **Interfaces carrying crypto**
  - Optional HTTPS admin surface; capability token verification; batch hash computation. Primary ledger link is **UDS + SO_PEERCRED** with filesystem perms.

---

## 3) Target PQ Posture (Where we’re going)

- **KEX / transport**
  - Prefer **UDS** (no KEX). If/when TCP/TLS is used, adopt **Hybrid KEX**: **X25519 + ML-KEM (Kyber)** when `pq_hybrid=true`.
- **Signatures**
  - Support **PQ signatures** for capability tokens / receipts: **ML-DSA (Dilithium)** preferred; **SLH-DSA (SPHINCS+)** as fallback policy. This crate verifies whichever algorithm policy specifies.
- **Tokens / capabilities**
  - Add an **algo tag** to caps; `ron-accounting` verifies **Ed25519** or **ML-DSA** per policy.
- **Backwards compatibility**
  - Classical remains until **M3** (Gold). After M3, **hybrid by default** on external edges; UDS remains default for intra-node.
- **At-rest envelopes (if WAL enabled)**
  - Optional: encrypt transient WAL with **AES-256-GCM** under keys **wrappable** by PQ-safe KMS (future re-encrypt support).

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (crate)
[features]
pq = []               # PQ plumbing (types, metrics, cfg gates)
pq-hybrid = ["pq"]    # enable hybrid KEX when TLS is used
pq-sign = ["pq"]      # enable PQ signature verification (Dilithium/Sphincs+)
pq-only = []          # refuse classical handshakes/tokens (policy-enforced)
````

```ini
# Config knobs (ron-accounting.toml)
pq_hybrid = false          # M1: off; M2: opt-in; M3: default true on TCP/TLS edges
pq_sign_algo = "auto"      # "auto" | "ml-dsa" | "slh-dsa" | "ed25519"
pq_only = false            # if true, refuse classical peers/tokens
key_rotation_days = 90     # verification key refresh horizon
```

* **Interoperability switch:** If peer lacks PQ, **negotiate** when `pq_only=false`; **refuse** with clear error when `pq_only=true`.
* **Metrics:** Always emit PQ metrics **even when disabled** (value=0) to ease adoption.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Planning & Hooks**

* Add `pq`, `pq-hybrid`, `pq-sign`, `pq-only` features and config gates; **no behavior change** by default.
* Baseline perf for classical (TLS off/loopback default).
* CI compiles with `--features pq` and runs basic tests (including mocks).

**M2 (Silver) — Hybrid Enablement**

* If TLS edge exists, wire **Hybrid KEX** (X25519 + ML-KEM) via transport adapter.
* Implement **PQ signature verification** for caps when `pq_sign_algo` is `ml-dsa`/`slh-dsa`.
* Interop tests:

  * classical↔classical
  * hybrid↔hybrid
  * hybrid↔classical (allowed only when `pq_only=false`)
* Perf budget: **≤ 20%** handshake overhead vs classical; record actuals.

**M3 (Gold) — Default & Operationalization**

* Default `pq_hybrid = true` on external TCP/TLS edges (if any); UDS intra-node remains primary path.
* Prefer PQ verification for caps (`pq_sign_algo="auto"` picks PQ when presented).
* Runbook updated with enable/rollback toggles; alert on downgrades.

**Post-M3 — De-risking & De-precation**

* Environments may set `pq_only=true`.
* Periodic PQ re-evaluation; rotate verification keys; keep adapter trait thin to swap libs.

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path relies **solely** on classical ECC/RSA when a PQ/hybrid option exists.
* **[PQ-I2]** Symmetric strength **≥ 256-bit**; hashes **≥ 256-bit** (BLAKE3-256).
* **[PQ-I3]** If WAL at rest is enabled, it must be **re-encryptable** under PQ-safe KMS envelopes.
* **[PQ-I4]** When `pq_only=true`, classical tokens/handshakes are **rejected with explicit error**; `/readyz` reflects policy mismatch.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback (emit audit + metrics).
* **[PQ-I6]** CI matrix includes PQ features; **interop parity** with classical proven before flipping defaults.

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with **algorithm labels**:

* `pq_handshake_total{algo="x25519"|"ml-kem"|"hybrid", role="server|client"}`
* `pq_signature_total{algo="ed25519"|"ml-dsa"|"slh-dsa", result="ok|fail"}`
* `pq_signature_failures_total{reason}`
* `crypto_latency_seconds_bucket{op="kex|sign|verify", algo=...}` (histogram)
* Config surface gauges:

  * `pq_mode{mode="off|hybrid|pq-only"} 1`
  * `pq_sign_algo{algo="auto|ed25519|ml-dsa|slh-dsa"} 1`
* **Readiness:** `/readyz` = 503 if `pq_only=true` and peer/stack cannot negotiate PQ; log `pq_downgrade_attempt=1`.

Structured logs: include `pq_mode`, `kex_algo`, `sign_algo`, `peer_mode`, and corr_id.

---

## 8) Testing & Verification

* **Unit / property**

  * Batch/frame parsers under PQ builds; idempotency keys stable across builds.
  * Capability verifier: vectors for Ed25519, ML-DSA, SLH-DSA (accept/reject cases).

* **Interop suite**

  * classical↔classical, hybrid↔hybrid, hybrid↔classical (policy-dependent).
  * UDS path (primary) remains unaffected; ensure feature toggle **does not** alter UDS readiness.

* **Fuzzing**

  * PQ token/codec decoders and negotiation state machine (no panics; correct errors).

* **Load / perf**

  * Handshake/sec with/without hybrid (if TLS edges are enabled).
  * ARM/edge profile when relevant; target ≤ 20% overhead vs classical.

* **Security drills**

  * **Classical break** simulation: set `pq_only=true` → classical caps/peers rejected; system degrades writes rather than violating invariants.
  * **Downgrade attack** simulation: attempt classical-only negotiation when policy demands PQ → alert + refusal.

---

## 9) Risks & Mitigations

* **Perf/footprint**: PQ handshakes and signatures are heavier.
  *Mitigation*: Prefer **UDS**, enable **session resumption**, cache verifiers, keep PQ confined to edges.

* **Library churn**: PQ libs evolve quickly.
  *Mitigation*: Wrap in thin adapter traits; pin versions workspace-wide; add smoke tests per release.

* **Downgrade abuse**: Attackers try to force classical.
  *Mitigation*: `pq_only` policy, downgrade detection metrics/alerts, `/readyz` fails on mismatch.

* **Ecosystem gaps**: Partners not PQ-ready.
  *Mitigation*: Keep **hybrid**; negotiate gracefully; document peer requirements.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL = Low** documented.
* [ ] `--features pq,pq-hybrid,pq-sign` compile in CI; interop tests pass.
* [ ] PQ metrics emitted; dashboards include `pq_*` panels.
* [ ] Runbook updated with enable/rollback and `/readyz` behavior.
* [ ] Perf numbers recorded (handshake/sign/verify; CPU/RAM deltas).
* [ ] SECURITY.md cross-links updated; owners ACK.

---

## 11) Role Presets (applied to this crate)

* **Primary target for `ron-accounting`**:

  * **Transport**: **UDS first** (no KEX). TLS only if required by deployment—then use **hybrid** when enabled.
  * **Tokens**: verify **Ed25519** today; allow **ML-DSA / SLH-DSA** when policy demands (`pq_sign_algo`).
  * **Defaults**: `pq_hybrid=false` (M1) → opt-in (M2) → **true by default** on external edges (M3). `pq_only=false` except in controlled environments.

---

## 12) Appendix

### A) Algorithms (initial stance)

* **KEX**: ML-KEM (Kyber) for hybrid; X25519 remains in hybrid pair.
* **Signatures**: ML-DSA (Dilithium) preferred; SLH-DSA (SPHINCS+) allowed.
* **Hash**: BLAKE3-256 (unchanged).
* **Symmetric**: AES-256-GCM / ChaCha20-Poly1305 (unchanged).

### B) Libraries (subject to adapter)

* `tokio-rustls`, `ed25519-dalek`, `blake3`; PQ via workspace-approved crates (behind `pq*` features).

### C) Interop Notes

* UDS path unaffected by PQ.
* If TLS is front-of-house, set `pq_hybrid=true` when peers support; otherwise allow classical **only** when `pq_only=false`.

### D) Change Log

* **2025-10-16**: First draft; hooks + milestones; HNDL assessed low; UDS emphasized; hybrid KEX plan defined.

```

---
```
