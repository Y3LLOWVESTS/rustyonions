

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-12
audience: contributors, security auditors, ops
crate: svc-interop
crate-type: service
pillar: 12
owners: [Stevan White]

---

# QUANTUM.md

## 0) Purpose

Describe how `svc-interop` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops.
Scope: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (Shor-breakable):**

  * **TLS 1.3 KEX:** X25519 (between interop and upstream peers when not fronted by gateway).
  * **Signatures:** Ed25519 (internal receipts/tokens via `ron-auth`/`svc-passport`, optional depending on deployment). Provider webhooks are typically **HMAC** (symmetric).
* **Symmetric/Hash (Grover-affected only):**

  * **AEAD:** AES-256-GCM or ChaCha20-Poly1305 (gateway/transport); interop itself terminates TLS only in direct mode.
  * **Hash:** BLAKE3-256 (internal addressing); **SHA-256** only at the *edge* for webhook verification (never persisted).
* **Data at rest / long-lived artifacts:**

  * `svc-interop` is **stateless** by design (amnesia); durable audit/evidence lives in `ron-audit` (append-only) and ledger-side systems.
  * Retention window (if any) for interop-local temp buffers is seconds to minutes → **HNDL risk: low**.
* **Transport/Session lifetime:** short-lived (seconds); tokens have short TTLs; replay windows ≤ 5 minutes.
* **Crate-specific blast radius (worst case if classical PKI breaks):**

  * Passive adversary could decrypt *recorded* TLS sessions terminating **at interop** in direct mode or upstream if not hybrid → potential leakage of webhook payloads and headers. Internal invariants (BLAKE3 addressing, capability enforcement) remain intact; economic authority is still bounded by downstream policy/ledger.

> **HNDL risk:** Low for interop (stateless, short sessions), but **moderate** at the perimeter until hybrid TLS is standard across the edge.

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use (effective):**

  * **KEX:** X25519 (TLS 1.3 via `rustls`) when interop terminates TLS itself; often TLS terminates at `svc-gateway`.
  * **Signatures:** Ed25519 for internal receipts/capabilities (via `ed25519-dalek` through `ron-auth`/`svc-passport`) where configured. Many provider webhooks use **HMAC-SHA256** (symmetric).
  * **Symmetric/Hash:** AES-256-GCM or ChaCha20-Poly1305 in transport; **BLAKE3-256** for internal content addressing; **SHA-256** permitted **only at edge** for provider verification.
* **Libraries:** `rustls`/`ring` (transport), `ed25519-dalek` (v2), `blake3`, `hmac`, `sha2` (edge-only HMAC).
* **Key custody:** Signing/verification keys live in **`ron-kms`** (preferred) or provider-held secrets mounted as env/secret volumes; rotation every ≤ 90 days; no raw private keys in files/logs.
* **Interfaces carrying crypto:** TLS sessions, webhook signatures (HMAC or provider-specific), macaroons (HMAC-based), capability receipts, audit decision digests.

---

## 3) Target PQ Posture (Where we’re going)

* **Key exchange / transport:** **Hybrid KEX** (X25519 + **ML-KEM**/Kyber) when `pq_hybrid=true` across gateway/transport → covers HNDL at perimeter.
* **Signatures:** Enable **ML-DSA (Dilithium)** for receipts/capabilities where policy demands; consider **SLH-DSA (SPHINCS+)** for long-term non-malleable audit anchors.
* **Tokens/capabilities:** Add PQ signature option and negotiate per-tenant policy (classical, hybrid verify, or PQ-only).
* **Backwards compatibility:** Classical remains supported through **M2**; default to **hybrid** in **M3**; PQ-only available per environment/policy.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (svc-interop)
[features]
pq = []                 # enable PQ plumbing (types, telemetry)
pq-hybrid = ["pq"]      # prefer Hybrid KEX in transport clients/servers we own
pq-sign = ["pq"]        # enable PQ signatures via ron-auth/svc-passport integration
pq-only = []            # optional hard refusal of classical peers (runtime gated)
```

```ini
# svc-interop config knobs (example)
pq_hybrid = false            # M1 default; set true at M2 for staged rollouts
pq_sign_algo = "ml-dsa"      # "ml-dsa" | "slh-dsa" | "off"
pq_only = false              # if true, refuse classical-only peers
key_rotation_days = 90       # keep as-is; rotations include PQ keys when present
```

* **Interoperability switch:** If peer lacks PQ, **negotiate** down unless `pq_only=true` (then 4xx/5xx with explicit reason).
* **Metrics:** Always emit PQ-labeled metrics (even at 0) to ease adoption tracking.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Planning & Hooks**

* Add `pq` features and plumbing; no behavior change.
* Exposure documented; baseline classical perf recorded (handshake latency, CPU).
* CI compiles with `--features pq,pq-hybrid,pq-sign` (even if PQ backends mocked).

**M2 (Silver) — Hybrid Enablement**

* Turn on **Hybrid KEX** in `ron-transport` consumers (interop direct mode, internal clients).
* Wire optional **PQ signatures** behind `pq-sign` for capability receipts (via `svc-passport`).
* Interop tests: classical↔classical, hybrid↔hybrid, hybrid↔classical (when allowed).
* Perf budget: ≤ 10–20% handshake overhead; record actuals in dashboards.

**M3 (Gold) — Default & Operationalization**

* Default `pq_hybrid = true` for app-facing paths (or at gateway if terminating TLS there).
* PQ signatures for **audit anchors/receipts** where policy requires; publish verification kits.
* Runbooks: enable/rollback toggles; **pq_only** playbook for incident “classical break”.
* QUANTUM.md & SECURITY.md finalized; GOVERNANCE ties PQ knobs to proposal lifecycle.

**Post-M3 — De-risking & De-precation**

* Introduce **pq_only** in high-sensitivity environments; sunset pure-classical at edges by policy when peers are ready.
* Annual PQ review (libs, parameters, FIPS/NIST updates).

---

## 6) Invariants (MUST)

* **[PQ-I1]** No pure ECC/RSA in security-critical paths **unless** wrapped in **hybrid PQ**.
* **[PQ-I2]** Symmetric keys **≥ 256-bit**; hashes **≥ 256-bit** (BLAKE3-256).
* **[PQ-I3]** Any long-lived encrypted data must be re-encryptable under PQ keys (delegated to storage/ledger owners).
* **[PQ-I4]** If `pq_only=true`, classical peers are refused with explicit error and audit evidence.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback; dual-verify windows are bounded and audited.
* **[PQ-I6]** CI matrix includes PQ features; **interop parity** tests prove no regression in protocols or reason codes.

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with **algo labels**:

* `pq_handshake_total{algo="x25519"| "ml-kem" | "hybrid", role}`
* `pq_signature_total{algo}`, `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify", algo}` (histogram)
* Readiness: `/readyz` **fails** if policy requires PQ and negotiation can’t satisfy it.
* Structured logs: include `pq_mode={off|hybrid|pq-only}`, `kex_algo=…`, `sig_algo=…`, `peer_mode=…`.

---

## 8) Testing & Verification

* **Unit / property tests:** token/verifier under PQ+classical; header/signature parsers; negotiation state machines.
* **Interop matrix:** classical↔classical, hybrid↔hybrid, hybrid↔classical (when allowed by policy).
* **Fuzzing:** PQ decoders/negotiation paths; error taxonomy stability (no panic, stable reasons).
* **Load:** handshake/sec + p95 latency with/without PQ; ARM/edge profiles (blake3-simd unaffected).
* **Security drills:** simulate “classical break” → force `pq_only` and verify safe failure (clear errors, alerts, audit trail).

---

## 9) Risks & Mitigations

* **Performance/footprint:** Larger keys/certs, slower handshakes → enable **session resumption**, connection pooling, and short-lived TLS at edge only.
* **Library churn:** PQ libs move fast → isolate behind transport/auth **traits**; pin versions; add SBOM.
* **Downgrade abuse:** Attackers try to force classical → enforce `pq_only` where mandated; emit **downgrade_detected_total** and alert.
* **E2E gaps:** Partners may lag → keep PQ optional (hybrid) until interop proven; maintain clear **peer capability matrix**.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL risk** labeled per environment.
* [ ] `pq` features compile; CI includes `--features pq,pq-hybrid,pq-sign`.
* [ ] Hybrid KEX interop passes; clear downgrade errors; `/readyz` policy honored.
* [ ] PQ metrics emitted; dashboards updated (algo labels visible).
* [ ] Runbook updated (enable/rollback & `pq_only` drills).
* [ ] Perf numbers recorded (handshake, sign/verify, CPU/RAM).
* [ ] SECURITY.md and GOVERNANCE.md cross-links updated; owners ACK.

---

## 11) Role Presets (svc-interop-specific)

* **Primary Targets:**

  * **Transport** (if terminating TLS): enable **hybrid KEX** when peers support it; otherwise front by `svc-gateway` with hybrid enabled there.
  * **Identity/policy path:** offer **PQ signatures** for receipts/capabilities via `svc-passport` when tenant policy toggles on.
  * **Observability:** emit PQ metrics/labels even when disabled to baseline adoption.

* **Defaults:**

  * `pq_hybrid=false (M1)` → `true (M3)`
  * `pq_sign_algo="off"` until policy/tenants request it
  * `pq_only=false` except high-sensitivity envs during drills

---

## 12) Appendix

**Algorithms chosen (initial posture):**

* **KEX:** **Hybrid(X25519 + ML-KEM-768)** for balance of security/perf; classical-only allowed until M3.
* **Signatures:** **ML-DSA (Dilithium 2/3)** for receipts/caps; **SLH-DSA (SPHINCS+)** optional for long-term audit anchors.

**Libraries (subject to ecosystem maturity):**

* `rustls` with hybrid KEX via upstream support or pluggable KEM; `oqs`/`liboqs-rust` bindings where appropriate; `ed25519-dalek` (current), PQ sig crates as they stabilize.

**Interop notes:**

* Peer stacks: `svc-gateway` first to adopt hybrid at the edge; internal services follow.
* Fallback policy: negotiate if `pq_only=false`; refuse with explicit reason if `pq_only=true`.

**Change log:**

* **2025-10-12:** Initial QUANTUM.md; M1 hooks defined; metrics labels reserved.

---

