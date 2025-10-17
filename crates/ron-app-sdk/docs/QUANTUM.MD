
---

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-16
audience: contributors, security auditors, ops
crate: ron-app-sdk
crate-type: sdk
pillar: 7
owners: [Stevan White]
---

# QUANTUM.md — ron-app-sdk

## 0) Purpose

Define how `ron-app-sdk` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** modes without breaking interop or DX.  
Scope covers: algorithms used via transports/tokens, where keys live (or don’t, for the SDK), runtime toggles, telemetry, tests, rollout, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

**Crate role:** Client SDK (Tier-2). It **does not** terminate TLS, mint ledger state, or persist secrets. It **forms** requests, attaches capabilities, enforces OAP/1 bounds (1 MiB), chunks storage (~64 KiB), and negotiates transport features.

- **Public-key usage (Shor-breakable) – via dependencies/peers:**
  - **Transport:** TLS 1.3 to gateway/overlay using **X25519** ECDH KEX (classical today).
  - **Signatures (SDK-adjacent):**  
    - **Server certs**: classical WebPKI; verified by rustls.  
    - **Receipts/tickets**: verified server-side; SDK may **verify** optional signatures (Ed25519) when surfaces are exposed.  
  - **SDK tokens (capabilities/macaroons):** HMAC-based (symmetric) → **not Shor-broken**; security margin governed by key length.

- **Symmetric/Hash (Grover-affected only):**
  - **AEAD:** ChaCha20-Poly1305 and/or AES-256-GCM.
  - **Hashing/content addressing:** **BLAKE3-256** (or SHA-256 where required).  
  - **Macaroons:** HMAC-SHA-256 (or equivalent) → increase key sizes for Grover cushion.

- **Data at rest / long-lived artifacts (SDK side):**
  - Default **amnesia**: SDK does **not** persist caps, keys, or payloads.  
  - Optional ephemeral cache (if enabled): bounded TTL/minimal scope; **no secrets** stored.  
  - **HNDL risk:** **Low** for the SDK. Long-lived risk lives server-side (ledger, mailbox, archives).

- **Transport/session lifetime:** seconds to minutes (connection reuse). Short sessions reduce HNDL exposure.

- **Worst-case blast radius if classical PKI breaks:**  
  Adversary could MITM TLS to nodes if the app is coerced into classical-only; SDK mitigates by offering **hybrid KEX** negotiation and a **pq_only** refuse mode. Capability abuse remains gated by HMAC (symmetric).

> **HNDL = Harvest-Now, Decrypt-Later**: recording TLS today and decrypting post-breakthrough. SDK’s mitigation is to (1) prefer short-lived sessions, (2) enable hybrid KEX when supported, (3) allow `pq_only` for high-assurance tenants.

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use (through dependencies):**  
  - **KEX:** X25519 (TLS 1.3).  
  - **Signatures:** Ed25519 (optional verification surfaces), WebPKI (server certs).  
  - **Symmetric/AEAD:** ChaCha20-Poly1305 and/or AES-256-GCM.  
  - **Hash:** BLAKE3-256; SHA-256 (for HMAC/macaroons).  
- **Libraries:** `rustls` (via `ron-transport`), `ed25519-dalek` (optional), `ring`/`sha2`/`blake3`.  
- **Key custody:** **None in SDK.** Capability tokens (macaroons) are caller-provided or minted server-side; the SDK holds them **in memory only** and never logs/persists them.  
- **Interfaces touching crypto:**  
  - Outbound HTTPS/Tor+TLS requests; headers (caps/idempotency); OAP/1 framing; optional receipt verification adapters.

---

## 3) Target PQ Posture (Where we’re going)

- **Transport (KEX):** **Hybrid** KEX by default when enabled: `X25519 + ML-KEM (Kyber)`.  
- **Signatures (optional verification surfaces):** **ML-DSA (Dilithium)** by default; **SLH-DSA (SPHINCS+)** as a fallback profile.  
- **Tokens/capabilities:** remain **HMAC-based** (symmetric). Ensure ≥256-bit keys and rotation to mitigate Grover.  
- **Backwards compatibility:** Classical remains supported through **M2**; SDK can run **hybrid** opportunistically; **pq_only** opt-in for high-assurance tenants.  
- **PQ at rest:** Out of SDK scope; documented expectations for node/storage crates.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (SDK)
[features]
pq = []                # enable PQ plumbing & labels
pq-hybrid = ["pq"]     # prefer Hybrid KEX (X25519 + ML-KEM) when peer supports
pq-sign = ["pq"]       # enable PQ signature verify adapters (Dilithium/Sphincs+)
pq-only = []           # build-time hardening (optional): refuse classical peers

# Dependency hints (illustrative; bound in transport/auth crates)
# pq uses adapters around rustls/oqs (or equivalent) via ron-transport/ron-auth.
````

```ini
# SDK Config (mapped onto SdkConfig)
pq_hybrid = false        # M1 default=false → M3 default=true (opportunistic)
pq_sign_algo = "ml-dsa"  # "ml-dsa" | "slh-dsa" | "off"
pq_only = false          # if true, refuse classical negotiation (fail closed)
key_rotation_days = 90   # guidance for HMAC/macaroons (caller policy)
```

*Interoperability switch:* If peer lacks PQ and `pq_only=false`, fall back to classical but **log downgrade** with metrics. If `pq_only=true`, **refuse** with a typed error.
*Metrics toggle:* Always emit PQ-labeled metrics (value=0 when disabled) to ease adoption.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Hooks & Baselines**

* Add `pq`/`pq-hybrid`/`pq-sign` feature flags and config fields (no behavior change by default).
* Document exposure; record **classical** handshake/latency baselines.
* CI compiles with PQ features (mock adapters if needed).

**M2 (Silver) — Hybrid Enablement**

* Enable **Hybrid KEX** path via `ron-transport` when `pq_hybrid=true`.
* Optional: expose **PQ signature** verification adapter for receipts when available.
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (when allowed).
* Perf budget: **≤20%** handshake overhead; record actuals and publish.

**M3 (Gold) — Default Hybrid & Ops**

* **Default `pq_hybrid=true`** for app-facing planes (opportunistic).
* Add `pq_only` mode for high-assurance tenants; include **rollback** guidance (flip back to classical).
* Dashboards: PQ metrics/alerts; Runbook updated.

**Post-M3 — De-risking & Deprecation**

* Evaluate making hybrid the **mandatory** default where peers support it; begin sunsetting pure-classical edges.
* Periodic PQ review as libs mature.

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path remains **pure classical** once `pq_hybrid` is enabled; hybrid negotiation must not be silently skipped.
* **[PQ-I2]** Symmetric strength **≥ 256-bit**; HMAC keys **≥ 256 bits** with rotation ≤ 90 days.
* **[PQ-I3]** If `pq_only=true`, **refuse** classical peers with a clear, typed error and guidance.
* **[PQ-I4]** SDK never persists caps or private keys; PQ adoption must not introduce storage of secrets.
* **[PQ-I5]** Public API/DTO stability: introducing PQ must be **additive**; classical interop remains available until documented milestone.
* **[PQ-I6]** CI must test classical, hybrid, and (where applicable) pq-only matrices; no hidden regressions.

---

## 7) Observability (Metrics, Logs, Readiness)

Emit per-op metrics with **algo labels** to visualize rollout:

* `sdk_pq_handshake_total{mode="off|hybrid|pq-only", algo="x25519|ml-kem|hybrid"}`
* `sdk_pq_downgrade_total{reason="peer_no_pq|policy"}`
* `sdk_pq_signature_total{algo="ml-dsa|slh-dsa"}`
* `sdk_pq_signature_failures_total{reason}`
* `sdk_crypto_latency_seconds{op="kex|sign|verify", algo}` (histogram)

**Structured logs:** include `pq.mode`, `pq.algo`, `peer.mode`, `negotiated=true|false`, `downgraded=true|false`.
**Readiness mapping (host):** if tenant policy requires PQ (pq_only) and peer can’t negotiate, surface **not ready** for that dependency.

---

## 8) Testing & Verification

* **Unit/property:** DTO codecs, OAP framing, token/header builders under PQ flags; never panic on malformed inputs.
* **Interop suite:** classical↔classical, hybrid↔hybrid, hybrid↔classical (allowed), pq-only↔classical (must fail clearly).
* **Fuzz:** negotiation paths and PQ decoder/validator surfaces.
* **Load:** handshake/sec and p95 latency deltas for TLS vs Hybrid; ARM/edge where relevant.
* **Security drills:** simulate **“classical break”** by forcing `pq_only=true`; ensure safe failure and actionable errors.

---

## 9) Risks & Mitigations

* **Perf/footprint hit:** Larger keys/certs & slower handshakes.
  *Mitigate:* session reuse, connection pooling, keep-alives; measure and budget (<20% handshake hit target).
* **Library churn/ABI stability:** PQ libs evolve.
  *Mitigate:* thin **adapter traits** in transport/auth crates; pin versions workspace-wide.
* **Downgrade abuse:** Attackers push classical fallback.
  *Mitigate:* `pq_only` for strict tenants; **downgrade metrics + alerts**; log peer capabilities.
* **DX friction:** Too many knobs may confuse devs.
  *Mitigate:* sane defaults (opportunistic hybrid), one-liner enablement, clear errors.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure labeled; **HNDL risk** assessed (SDK = **Low**).
* [ ] `pq`, `pq-hybrid`, `pq-sign`, `pq-only` features compile; CI matrix includes them.
* [ ] Interop matrix passes; clear typed errors on forbidden mixes.
* [ ] PQ metrics/logs emitted (even when disabled with zeros).
* [ ] Runbook/SECURITY cross-links updated (enable/rollback, policy notes).
* [ ] Perf numbers recorded (handshake delta, CPU/RAM).
* [ ] Owners ack; release notes document defaults and migration.

---

## 11) Role Presets for This SDK

* **Primary target:** Client-side **negotiation** and **policy enforcement** (fail-closed options).
* **Defaults:**

  * M1: `pq_hybrid=false`, `pq_sign=off`, `pq_only=false`.
  * M2: tenants **may** enable `pq_hybrid=true`.
  * M3: default `pq_hybrid=true` (opportunistic), `pq_only` opt-in per tenant risk profile.

---

## 12) Appendix (fill as we adopt)

* **Algorithms chosen:** KEX=`hybrid(x25519+ml-kem)`; SIG=`ml-dsa` (fallback `slh-dsa`).
* **Libraries:** rustls + PQ adapter (via `ron-transport`), ed25519-dalek (legacy verify), oqs/oqs-sys (if selected) — versions pinned in workspace.
* **Interop notes:** peers supported = {gateway≥X, overlay≥Y}; classical fallback allowed unless `pq_only=true`.
* **Change log:**

  * 2025-Q4: Hooks added; baselines captured.
  * 2026-Q1: Hybrid optional (Silver).
  * 2026-Q2: Hybrid default (Gold); pq_only supported for strict tenants.

```
---

