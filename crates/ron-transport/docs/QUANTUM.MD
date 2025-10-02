````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-01
audience: contributors, security auditors, ops
crate: ron-transport
crate-type: transport
pillar: 10
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose

Define how `ron-transport` resists **quantum-enabled adversaries** and how we migrate to **post-quantum (PQ)** without breaking interop or operations. Scope covers: exposure, current crypto, target posture, flags/config, rollout plan (M1→M3), invariants, observability, tests, risks, and acceptance.

> `ron-transport` is a **transport library**. It does not define application payload crypto or OAP parsing. PQ scope here focuses on **session establishment** (TLS handshakes/KEX) and **policy/telemetry** so embedding services can roll out PQ safely.

---

## 1) Exposure Assessment (What’s at risk?)

- **Public-key usage (Shor-breakable):**
  - **TLS 1.3 KEX:** Classical **X25519** (and/or P-256) today when backend=`tls`.
  - **TLS server cert signatures:** Classical (Ed25519 or ECDSA P-256) — depends on operator cert choice.
  - **Tor/Arti circuits (backend=`arti`):** Classical Tor handshake/circuit building (no PQ today).
- **Symmetric / Hash (Grover-affected only):**
  - **TLS AEAD:** **AES-256-GCM** or **ChaCha20-Poly1305** (256-bit strength acceptable vs Grover).
  - **Handshake transcript hash (TLS):** SHA-256/384 (library-internal to rustls).
- **Data at rest / long-lived artifacts:**  
  - `ron-transport` stores **no persistent data** under **amnesia=ON** (default for Micronode). TLS private keys/certs are short-lived files mounted by operators (rotation policy applies).
  - **HNDL risk** (Harvest-Now, Decrypt-Later): **Medium** for **recorded TLS sessions** if only classical KEX is used; **Low** for Arti if sessions are short but still classical (cannot be retro-decrypted if wrapped by hybrid TLS above Arti; see below).
- **Transport/session lifetime:** seconds to minutes (short, lowers HNDL impact but not zero).
- **Worst-case blast radius if classical PKI/KEX is broken:** passive adversaries could decrypt captured TLS traffic that used classical KEX/certs; Arti circuits remain classical until upstream PQ exists.

> **Note:** `ron-transport` can run **TLS over Arti** only if the embedding service chooses to dial TLS endpoints through Tor. When backend=`arti` alone, the stream is a raw onion circuit (classical).

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use (when `backend=tls`):**
  - **KEX:** X25519 (TLS 1.3)  
  - **Server cert signatures:** Ed25519 or ECDSA P-256 (operator-chosen)  
  - **AEAD:** AES-256-GCM or ChaCha20-Poly1305  
  - **Handshake transcript hash:** SHA-256/384 (per suite)
- **Algorithms (when `backend=arti`):** Tor/Arti’s classical onion circuit crypto (no PQ today).
- **Libraries:** `tokio-rustls` (re-exports `rustls`), `ring` (via rustls), `arti` (optional).
- **Key custody:** TLS keys/certs are operator-provided files (0600); recommend KMS/HSM. Rotation ≤ 90 days.
- **Interfaces carrying crypto:** TLS handshakes and cert chains; Arti onion circuits; **no** app-level signing/verification in this crate.

---

## 3) Target PQ Posture (Where we’re going)

- **KEX (TLS):** adopt **Hybrid KEM** once available in rustls (e.g., **X25519 + ML-KEM (Kyber-768)**).  
  - Policy name: `pq.mode = "hybrid"`.
  - Classical-only remains supported until **M3** default switch.
- **Signatures (TLS certificates):** stay classical short-term (Ed25519/ECDSA). Track ecosystem for **ML-DSA (Dilithium)** or **SLH-DSA (SPHINCS+)** certificate chains; plan migration once supported by ACME/PKI.
- **Arti/Tor:** remains **classical** until Arti exposes PQ circuits. PQ posture for `backend=arti` is **Degraded-PQ**; operators may choose **TLS-over-Arti** (hybrid TLS wrapped inside Tor) to mitigate HNDL.
- **Backwards compatibility:** classical KEX is accepted unless `pq_only=true`. Hybrid negotiation prioritizes PQ when peers support it.

---

## 4) Feature Flags & Config (How to turn it on)

> `ron-transport` is **crypto-neutral**. It exposes policy/config and consumes PQ from its TLS stack when available. No algorithms are hard-coded here.

### Cargo features (crate)
```toml
[features]
tls = []        # TLS paths (tokio-rustls/rustls)
arti = []       # Arti backend
pq = []         # Enable PQ plumbing (labels, config, test vectors)
# (No direct dependency on liboqs in this crate; rustls provides KEM groups when ready)
````

### Config knobs (see CONFIG.md)

```ini
[transport.pq]
mode   = "off"      # "off" | "hybrid"
# optional future policy:
# only  = false     # when true, refuse classical peers (NotReady/clear error)
```

**Interoperability:**

* `mode="off"` → classical TLS only.
* `mode="hybrid"` → negotiate hybrid when peer supports; otherwise classical unless `pq_only=true`.
* `pq_only=true` (future knob) → refuse classical; readiness = NotReady with reason `pq_policy`.

**Metrics:** Always emit PQ labels, even if disabled (`pq="off"`), for adoption tracking.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Hooks & Safety**

* Add `pq` feature, config, metrics labels (`pq={off|hybrid}`); no behavioral change.
* Establish **HNDL posture** and document operator guidance (this file).
* CI builds with `--features pq` and runs **hybrid KEX test vector** when upstream rustls provides it (gated).

**M2 (Silver) — Hybrid Enablement**

* Enable **hybrid KEX** in TLS backend when supported by rustls KEM groups.
* Interop tests: classical↔classical, hybrid↔hybrid, hybrid↔classical (allowed unless `pq_only`).
* Perf budget: ≤ **+20%** handshake latency at p95; record measured deltas.
* Optional: allow **TLS-over-Arti** to benefit from hybrid where policy demands.

**M3 (Gold) — Default & Ops**

* Default `pq.mode="hybrid"` for app-facing planes where peers support it; keep **rollback** path (`off`) in config.
* Dashboards & alerts include PQ labels; runbooks updated (enable/rollback).
* Consider `pq_only` for **regulated environments** after ecosystem confirms support.

**Post-M3 — De-risking**

* Periodic re-evaluation of KEM/SIG parameter sets; add migration plan for PQ certificate chains when PKI catches up.
* Begin sunsetting classical KEX on internal links first (behind `pq_only`).

---

## 6) PQ Invariants (MUST)

* **[PQ-I1]** No **security-critical** TLS paths remain pure classical **when policy requires hybrid** (enforced via config).
* **[PQ-I2]** Symmetric strength **≥ 256-bit**; transcript/hash functions per TLS suite remain ≥ 256-bit security.
* **[PQ-I3]** Negotiation policy is explicit:

  * `pq.mode="hybrid"` attempts hybrid first;
  * `pq_only=true` (future) **refuses** classical peers with a **stable error** (`pq_policy`).
* **[PQ-I4]** PQ enablement is **config-only**; public API does not change.
* **[PQ-I5]** Metrics/logs must label sessions with `pq={off|hybrid|only}` and `algo` when known.
* **[PQ-I6]** CI proves **interop parity**: enabling PQ does not regress non-PQ behavior.

---

## 7) Observability (Metrics, Logs, Readiness)

Emit or ensure the embedding service emits:

* **Counters:**

  * `pq_handshake_total{backend="tls", mode="off|hybrid", result="ok|fail"}`
  * `transport_dials_total{backend,result}` (existing), now with `pq` label when backend="tls"
* **Latency (histogram):**

  * `crypto_latency_seconds{op="kex", backend="tls", mode="off|hybrid"}`
* **Readiness:**

  * If `pq_only=true` and peer/stack can’t negotiate → **NotReady** with reason `pq_policy`.
* **Logs (structured):**

  * Include `pq` and `algo` (e.g., `algo="x25519+ml-kem768"` when exposed by TLS stack).

---

## 8) Testing & Verification

* **Unit/property:** negotiation policy, label propagation, config validation.
* **Integration:** handshake success paths with classical; hybrid when upstream provides KEM groups; clear error on `pq_only` mismatch.
* **Fuzz:** negotiation state machine + error taxonomy (no panics).
* **Loom/Chaos/Soak:** no PQ-specific changes; ensure enabling PQ does not affect cancellation/backpressure behavior.
* **Perf:** compare handshake p95 **off vs hybrid**; assert overhead ≤ budget (alert if exceeded).

> **CI Matrix addenda:** `--features pq`, and when TLS KEM support is present in toolchain, run **Hybrid KEX vector** and set a `pq="hybrid"` label expectation in metrics tests.

---

## 9) Risks & Mitigations

| Risk                            | Impact                          | Mitigation                                                                                  |
| ------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------- |
| PQ handshake overhead           | Higher p95; CPU spikes          | Session reuse; tune `max_conns`/rates; canary rollout; perf gates with regression budget    |
| Ecosystem lag (PKI for PQ sigs) | Can’t deploy PQ cert chains yet | Use **hybrid KEX** first; keep classical sigs for certs; monitor ACME/PKI timelines         |
| Downgrade/interop confusion     | Unexpected classical fallback   | Explicit `pq_only` policy; structured logs; metrics label `pq` on every handshake           |
| Arti remains classical          | PQ gap for onion circuits       | Optionally run **TLS-over-Arti (hybrid)**; mark `backend=arti` as **Degraded-PQ** in audits |
| Library API churn upstream      | Build fragility                 | Thin adapter layer; vendor exact rustls version range; feature-gate PQ paths                |

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure documented; **HNDL risk** labeled per backend.
* [ ] `pq` feature compiles; config validated; metrics carry `pq` labels.
* [ ] Interop suite: classical↔classical and (when available) hybrid↔hybrid pass.
* [ ] `pq_only` policy (future) returns stable error and readiness status.
* [ ] Perf deltas recorded (off vs hybrid) with SLOs; alerts wired.
* [ ] Runbook & Security docs updated; operators have enable/rollback steps.

---

## 11) Role Presets (focused for transport)

### Transport (`ron-transport`)

* **Primary target:** **Hybrid KEX** on TLS when available; **policy-driven negotiation**; consistent labels & readiness.
* **Defaults:** `pq.mode="off"` (M1) → enable via config in M2; **default to `"hybrid"` at M3** for app-facing planes with peer support; `pq_only=false`.

### Identity/KMS (context)

* TLS cert issuance remains classical until PQ PKI is practical; track KMS/HSM readiness for ML-DSA/SLH-DSA.

### Gateways/Nodes (consumers)

* Expose `/readyz` + metrics including `pq` labels; provide **one-click rollback** to `pq.mode="off"`.

---

## 12) Appendix

* **Algorithms (today):** KEX=X25519; SIG=Ed25519/ECDSA-P256; AEAD=AES-256-GCM/ChaCha20-Poly1305; Hash=SHA-256/384 (TLS internal).
* **Algorithms (target):** KEX=**Hybrid X25519+ML-KEM-768**; SIG=TBD (ML-DSA or SLH-DSA) when PKI support lands.
* **Libraries:** `tokio-rustls/rustls` (TLS), `arti` (Tor). No direct PQ lib dependency in this crate.
* **Interop notes:** Classical remains accepted unless `pq_only=true`. Arti is classical; use TLS-over-Arti for PQ KEX benefit.
* **Change log (PQ):**

  * 2025-10-01 — Added PQ policy knobs and metrics labels; planned Hybrid KEX vector in CI (gated).

---

### Operator Quickstart (once upstream KEM is available)

1. Set in service config:

   ```toml
   [transport.pq]
   mode = "hybrid"
   ```
2. Roll out to **1 canary**; verify dashboards: `pq="hybrid"`, handshake p95 within budget.
3. Gradually expand; keep **rollback** ready: `mode="off"`.
4. For `backend=arti`, consider **TLS-over-Arti** to gain PQ KEX on top of onion circuits.

```
```
