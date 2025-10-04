

---

# QUANTUM.md — `svc-overlay`

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-03
audience: contributors, security auditors, ops
crate: svc-overlay
crate-type: service
pillar: 10              # Overlay / Transport / Discovery
owners: [Stevan White]
----------------------

## 0) Purpose

Describe how `svc-overlay` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops. Scope covers: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure. `svc-overlay` is **non-custodial** (no mint/settle/state), but it is a **transport/session fabric**; transcripts of overlay traffic are the primary HNDL concern.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (Shor-breakable):**

  * **Key exchange:** TLS 1.3 ECDHE (**X25519**) on TCP; optional QUIC ECDH on stream setup.
  * **Signatures:** **Ed25519** for service identity and capability envelopes (verification in overlay; private signing keys live outside overlay).

* **Symmetric/Hash (Grover-affected only):**

  * **AEAD:** AES-256-GCM and/or ChaCha20-Poly1305.
  * **Hash:** BLAKE3-256 (and SHA-256 when required by dependencies).
  * ⮕ Using 256-bit primitives keeps effective security ≥128-bit under Grover.

* **Data at rest / long-lived artifacts (overlay is stateless):**

  * Overlay stores **no customer payloads**. Retained artifacts: logs/metrics, optional onion host keys, transient TLS tickets (if enabled).
  * **Retention:** logs ≤ 30–90d (operator policy); TLS tickets disabled or ≤ 24h; onion keys in KMS.
  * **HNDL risk:** **Low** at rest (stateless) / **Medium** in transit (captured TLS/QUIC can be decrypted later if ECDH is broken).

* **Transport/Session lifetime:** seconds→minutes (idle timeout ~15–30s; connections reused). Short lifetimes reduce HNDL, but do not eliminate it.

* **Worst-case blast radius if classical PKI breaks:** An adversary recording overlay traffic today could decrypt it later; active attacks could **impersonate peers** if Ed25519 is broken. Economic custody remains unaffected (overlay is enforcement-only), but **policy enforcement and confidentiality** would be compromised.

---

## 2) Current Crypto Profile (Today)

* **Algorithms:**

  * **KEX:** X25519 (TLS 1.3 / QUIC ECDH).
  * **Signatures:** Ed25519 for node/service identity and signed capability envelopes (verification by overlay).
  * **AEAD/Hash:** AES-256-GCM or ChaCha20-Poly1305; BLAKE3-256 (and SHA-256 as needed).

* **Libraries:** `rustls` (TLS 1.3), QUIC stack (feature-gated), `ed25519-dalek` v2 for verification, `ring`/`orion`/`blake3` for primitives (as wired by workspace).

* **Key custody:**

  * **Verification keys only** on overlay hosts.
  * Signing keys and TLS private keys live in **ron-kms/HSM**; rotation **≤90d**.
  * Onion/Tor keys (if enabled) via KMS file-provider; rotation auditable.

* **Interfaces carrying crypto:**

  * OAP/1 frames over TLS/QUIC; admin plane (loopback HTTP) is non-TLS by default; **no secrets** on admin endpoints.

---

## 3) Target PQ Posture (Where we’re going)

* **Key exchange / encryption:** **Hybrid KEX** = X25519 **+** **ML-KEM-768 (Kyber-768)**; QUIC/TLS adapters negotiate hybrid when `pq_hybrid=true`.
* **Signatures:** Keep **Ed25519** for now; add optional **ML-DSA-2 (Dilithium-2)** for capability envelopes and service identity as upstreams adopt.
* **Transport TLS/QUIC:** classical now; hybrid by default in **M3** for app-facing planes.
* **Tokens / capabilities:** macaroon-style cap envelopes gain a **PQ signature option**; negotiation/policy per tenant.
* **Backwards compatibility:** classical supported through **M3**; then **default = hybrid**; classical can be refused via `pq_only=true`.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (svc-overlay)
[features]
pq = []               # enable PQ plumbing in this crate
pq-hybrid = ["pq"]    # negotiate Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]      # accept/verify PQ signatures for envelopes
pq-only = []          # (optional) refuse classical peers at build/runtime
quic = []             # QUIC transport (0-RTT remains disabled)
```

```ini
# overlay config (excerpt)
pq_hybrid = false          # M1 default; M3 default=true
pq_sign_algo = "ml-dsa"    # "ml-dsa" (Dilithium) | "slh-dsa" (SPHINCS+)
pq_only = false            # if true, refuse classical handshakes
key_rotation_days = 90
session_tickets = "off"    # avoid HNDL surface; if "on", ttl<=24h
```

* **Interoperability:** If a peer lacks PQ: **negotiate classical** unless `pq_only=true` (then refuse with a clear error).
* **Metrics:** Emit PQ labels **always** (value=0 when disabled) for adoption tracking.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Hooks & Assessment**

* Add `pq` features and config stubs (no behavior change).
* Document exposure; baseline perf for classical handshake and data path.
* Ensure unit/integration tests **build with PQ features** (mock where needed).

**M2 (Silver) — Hybrid Enablement**

* Enable **Hybrid KEX** in `ron-transport` (TCP/TLS) and QUIC (feature-gated).
* Optional: **PQ signatures** in `ron-auth`/`svc-passport` (overlay verifies).
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (when allowed).
* Perf budget: **≤30%** handshake overhead (track actuals; see PERFORMANCE.md).

**M3 (Gold) — Default Hybrid + Ops**

* **Default `pq_hybrid=true`** for app-facing planes.
* PQ signatures optional per-tenant policy (cap envelopes).
* Runbooks for incident handling and rollback (`pq_only=false` flip if needed).
* QUANTUM.md finalized; SECURITY.md updated; dashboards live.

**Post-M3 — De-risking & Deprecation**

* Introduce **`pq_only=true`** on regulated edges; sunset pure classical externally.
* Re-audit PQ libs annually; refresh NIST selections as upstreams stabilize.

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path runs **pure ECC/RSA** unless wrapped in **hybrid PQ** when policy requires PQ.
* **[PQ-I2]** Symmetric keys **≥256-bit**; hashes **≥256-bit**.
* **[PQ-I3]** Long-lived artifacts we touch (tickets/keys) have **rotation ≤24h/90d** as appropriate and are re-encryptable under PQ keys when available.
* **[PQ-I4]** If `pq_only=true`, classical peers **must be refused** with explicit error + audit.
* **[PQ-I5]** Key rotation procedures **upgrade algorithms** without silent fallback.
* **[PQ-I6]** PQ feature builds pass CI; **interop parity** with classical proven in tests.

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics **with algo labels**:

* `pq_handshake_total{algo="x25519|ml-kem|hybrid", role}`
* `pq_signature_total{algo}`, `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify", algo}` (histogram)
* `pq_mode{mode="off|hybrid|pq-only"}` (gauge)
* **Readiness:** `/readyz` **fails** if policy mandates PQ and negotiation fails or PQ stack is unavailable.
* **Logs:** structured fields `pq_mode`, `kex_algo`, `sig_algo`, `peer_mode`, `downgrade_detected=true|false`.

---

## 8) Testing & Verification

* **Unit / property:** codec boundaries, token/envelope verification under classical + PQ; negotiation state machine (no panic on unknown algs).
* **Interop:** classical↔classical, hybrid↔hybrid, hybrid↔classical (policy-driven); **SDK parity** (Rust vs TS) with PQ on/off.
* **Fuzz:** PQ decoders & negotiation paths; malformed KEM ciphertexts; downgrade vectors.
* **Load:** handshake/sec with and without PQ; measure **CPU/RAM** delta; ARM/edge profiles included.
* **Security drills:** simulate “classical break” → set `pq_only=true`; verify safe refusal and clear errors; attempt silent PQ toggle → must **audit**.

---

## 9) Risks & Mitigations

* **Perf/footprint:** Larger keys/ciphertexts; handshake slower. ⮕ Mitigate via **connection reuse**, keep-alive, and **hybrid only on session setup**.
* **Library churn:** PQ libraries evolve. ⮕ Hide behind **transport/adaptor traits**; pin versions workspace-wide; vendor patches if needed.
* **Downgrade abuse:** Attackers force classical. ⮕ Enforce policy (`pq_only`), **log+alert** on downgrade; expose `downgrade_detected` counter.
* **Ticket resumption risk:** Recorded tickets could aid decryption. ⮕ **Disable session tickets** by default; if enabled, **ttl≤24h** and bind to client properties.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL labeled (**Medium** transit / **Low** at rest).
* [ ] `pq` feature matrix compiles: `pq`, `pq-hybrid`, `pq-sign`, `pq-only`.
* [ ] Hybrid KEX interop passes; downgrade behavior clear and audited.
* [ ] PQ metrics emitted; dashboards/alerts live.
* [ ] Runbook updated (enable/rollback, `pq_only` safety).
* [ ] Perf numbers recorded (handshake overhead, CPU/RAM).
* [ ] SECURITY.md cross-links updated; owners ack.

---

## 11) Role Presets (svc-overlay focus)

* **transport / overlay (`svc-overlay`, `ron-transport`, `svc-arti-transport`):**

  * **Primary:** **Hybrid KEX** first; PQ posture metrics; policy-driven negotiation.
  * **Defaults:** `pq_hybrid=false (M1) → true (M3)`; `pq_only=false` unless mandated.
  * **Notes:** QUIC **0-RTT stays disabled**; resumption tickets **off** by default.

* **identity/policy/kms (`ron-kms`, `ron-auth`, `svc-passport`, `ron-policy`, `ron-audit`):**

  * PQ signatures optional; custody/rotation; append-only audit of posture changes.

* **gateway/nodes (`svc-gateway`, `macronode`, `micronode`):**

  * Policy-driven PQ negotiation; `/readyz` reflects PQ requirements; rollback path documented.

* **econ (`ron-ledger`, `svc-rewarder`) & storage/index/mailbox:**

  * Overlay **does not** verify ZK/ledger proofs; defer to owners. Ensure HNDL mitigations for long-lived data at rest.

---

## 12) Appendix

**Algorithms (target):**

* **KEX:** **Hybrid** (X25519 + **ML-KEM-768**).
* **Signatures:** Ed25519 today; **ML-DSA-2** (Dilithium-2) as optional envelope signature.
* **Symmetric/Hash:** AES-256-GCM / ChaCha20-Poly1305; BLAKE3-256.

**Libraries (indicative):**

* `rustls` (TLS 1.3 adapters for hybrid KEX), QUIC stack with KEM extension once upstreamed, `ed25519-dalek` v2, PQ libs via feature-gated adapters.

**Interop notes:**

* Peers without PQ → negotiate classical unless `pq_only=true`.
* Metrics record mode/algos even when PQ is off to ease fleet rollout.

**Change log (to update as we roll):**

* 2025-10-03: QUANTUM.md introduced; features & knobs stubbed; perf budget set (≤30% handshake overhead).
* M2: Enable hybrid in staging; add PQ metrics panels.
* M3: Default hybrid on app-facing planes; consider `pq_only=true` on regulated edges.

---

**TL;DR:** `svc-overlay` goes **Hybrid KEX** (X25519+ML-KEM), optional **PQ signatures** for envelopes, with knobs, metrics, tests, and clear rollback. It minimizes **HNDL** exposure for transit, keeps **admin/local** plane simple, and remains **non-custodial** with auditable posture changes.
