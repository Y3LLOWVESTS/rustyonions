

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-07
audience: contributors, security auditors, ops
crate: omnigate
crate-type: service
pillar: 6            # Ingress & Edge / BFF layer
owners: [Stevan White]

# QUANTUM.md — omnigate

## 0) Purpose

Describe how **omnigate** resists **quantum attacks** and how it migrates to **post-quantum (PQ)** cryptography without breaking interop or ops.
Scope covers: algorithms in use, where keys live, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

> Omnigate is a **read-path BFF** behind `svc-gateway`. TLS is normally terminated at the gateway. Omnigate does **not** mint/settle ledger state; its crypto footprint is primarily: (a) verifying capability tokens and audit signatures; (b) optionally using TLS/mTLS to internal downstreams (index/storage/mailbox).

---

## 1) Exposure Assessment (What’s at risk?)

**Public-key (Shor-breakable) in scope for omnigate:**

* **Transport KEX/handshake:** Typically **none at the public edge** (gateway terminates TLS). Internal calls to downstreams may use **TLS 1.3** with **X25519** (or mTLS).
* **Signatures:** **Ed25519** for capability/macaroons verification (short-lived) and for signed audit events/controls.

**Symmetric/Hash (Grover-affected only):**

* **AEAD:** **AES-256-GCM** or **ChaCha20-Poly1305** (TLS stack when used).
* **Hash:** **BLAKE3-256** for content addressing; Grover’s algorithm effectively halves brute-force cost → ~**128-bit** effective security, still acceptable for addressing, monitor for future upgrades.

**Data at rest / long-lived artifacts touching omnigate:**

* **Omnigate itself:** stateless. Keeps **logs/metrics** only (short retention per ops).
* **Referenced data:** mailbox/media objects live in `svc-storage` (long-lived); manifests/indices in `svc-index`. These are **outside omnigate’s custody** but drive overall HNDL posture.

**Session lifetimes:**

* Edge TLS (at gateway): seconds–minutes.
* Internal service TLS: minutes. (Short lifetimes → lower HNDL risk.)

**Worst-case impact (classical PKI break):**

* Forged ops tokens or audit signatures **if** Ed25519 is compromised and tokens lack PQ binding; possible **policy bypass** until detected.
* Recorded intra-cluster TLS could be decrypted later, but omnigate does not carry long-lived secrets; risk is **low HNDL for omnigate**, **systemic** for storage/mailbox.

**HNDL rating for omnigate:** **Low** (stateless, short-lived transit, short-TTL caps). End-to-end HNDL driven by downstream storage/mailbox.

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use:**

  * **KEX/Handshake:** X25519 (TLS 1.3) **only** when omnigate speaks TLS to downstreams.
  * **Signatures:** Ed25519 for macaroons/capabilities and audit entries.
  * **Symmetric/AEAD:** AES-256-GCM or ChaCha20-Poly1305 (TLS, if used).
  * **Hash/Addr:** BLAKE3-256 (`b3:<hex>`).

* **Libraries (pins indicative):**

  * **TLS:** `tokio-rustls 0.26.x` (rustls backend).
  * **Sigs:** `ed25519-dalek v2`.
  * **Hashing:** `blake3 1.x`.
  * **(PQ adapter placeholder for M2/M3):** `oqs-rust v0.8.x` or equivalent, **feature-gated** behind `pq` (subject to audit).

* **Key custody:**

  * **Ops/admin caps & audit signing keys:** **ron-kms** or HSM; rotation **≤ 90 days**.
  * No raw keys in env/files; tokens are short-TTL macaroons with caveats.

* **Interfaces carrying crypto:**

  * Capability tokens (headers), signed audit events, optional intra-cluster TLS.
  * Edge TLS owned by gateway.

---

## 3) Target PQ Posture (Where we’re going)

* **Transport (internal):** **Hybrid KEX** (X25519 + **ML-KEM/Kyber**) when `pq_hybrid = true`. Classical allowed during M1/M2; hybrid default by **M3** where peers support it.
* **Signatures (tokens/audit):** Add **ML-DSA (Dilithium)** as an optional verification/issuance path. **SLH-DSA (SPHINCS+)** is permissible for ultra-conservative deployments.
* **Gateway edge TLS:** migrate via **svc-gateway** plan; omnigate uses a **policy bit** and refuses plain-classical only if `pq_only=true` **and** all peers are ready.
* **Back-compat:** classical supported until **M3** (Gold); then **hybrid** default. Pure-PQ (`pq_only`) gated by config + interop.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (omnigate)
[features]
pq = []              # compile PQ plumbing (types/metrics/negotiation)
pq-hybrid = ["pq"]   # enable Hybrid KEX (X25519 + ML-KEM) for internal TLS
pq-sign = ["pq"]     # enable PQ signature verification for caps/audits
pq-only = []         # (advanced) refuse classical peers at runtime
```

```ini
# omnigate Config (snippets)
pq_hybrid = false            # M1 default -> true in M3 where peers support
pq_sign_algo = "off"         # "off" | "ml-dsa" | "slh-dsa"
pq_only = false              # refuse classical handshakes when true
key_rotation_days = 90
```

**Interop behavior:**

* Peer lacks PQ & `pq_only=false` → negotiate **best available** (classical/hybrid).
* `pq_only=true` & peer lacks PQ → **fail fast** with explicit error; `/readyz` turns red if policy requires PQ but peers can’t comply.

**Metrics:** always emit PQ labels (zero when disabled) to ease progressive rollouts.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Planning & Hooks**

* Add `pq` features, config stubs, and metrics labels; **no behavior change**.
* Document exposure; baseline classical perf.
* Unit tests compile with `--features pq,*` (mock adapters OK).

**M2 (Silver) — Hybrid Enablement**

* Turn on **Hybrid KEX** for **internal** TLS via `ron-transport` adapter; keep edge TLS in `svc-gateway`.
* Optional **PQ signature verification** for caps/audits under `pq-sign`.
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (when allowed).
* Perf budget: **≤ 10–20%** handshake overhead; capture and publish actuals.
* Toil objective: end-to-end **enablement < 1 week** per cluster or file ADR.

**M3 (Gold) — Default Hybrid**

* `pq_hybrid = true` by default where peers support; fallback allowed unless `pq_only=true`.
* PQ signatures on **audit events** and **caps** per tenant policy.
* Runbook entries for enable/rollback; QUANTUM/SECURITY finalized.

**Post-M3 — De-risking & De-precation**

* Introduce **`pq_only`** profiles (canary); sunset pure-classical on internal links.
* Annual PQ review (libs/algos), ADR for changes; black-swan HNDL drill (see §9).

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical paths rely on **pure ECC/RSA** **unless** wrapped in **hybrid PQ** (where TLS is used).
* **[PQ-I2]** Symmetric strength **≥ 256-bit**; hashes **≥ 256-bit** (BLAKE3-256).
* **[PQ-I3]** Omnigate stores no long-lived data; downstream services MUST support PQ re-encryption at rest (tracked in their QUANTUM docs).
* **[PQ-I4]** If `pq_only=true`, classical peers are refused; `/readyz` reflects policy non-compliance.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback (fail-closed on misconfig).
* **[PQ-I6]** CI runs interop suites for classical/hybrid; feature builds must pass (`pq`, `pq-hybrid`, `pq-sign`, combos).
* **[PQ-I7]** Grover-resistant hashes only: **no** reduction below 256-bit without proposal & approval (ADR).

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics (with labels):**

* `pq_handshake_total{algo="x25519|ml-kem|hybrid", role="client|server"}`
* `crypto_latency_seconds{op="kex|sign|verify", algo}` (Histogram)
* `pq_signature_total{algo}` & `pq_signature_failures_total{reason}`
* `pq_sessions_current{mode="off|hybrid|pq-only"}`
* `pq_policy_violation` (gauge: 0/1)

**Logs (structured):** include `pq_mode={off|hybrid|pq-only}`, `kex_algo`, `sign_algo`, `peer_mode`, `downgrade_detected`.
**Readiness:** `/readyz` turns **red** if policy requires PQ and peers cannot negotiate.

---

## 8) Testing & Verification

* **Unit/Property:** token/parsers and negotiation FSM under `pq` features; round-trip encode/verify; no panics.
* **Interop:** classical↔classical, hybrid↔hybrid, hybrid↔classical; **downgrade attempts are detected and logged**.
* **Formal:** TLA+ sketch for negotiation FSM (“no silent downgrade” property) under `docs/tla/pq_negotiation.tla` (required for **Gold**).
* **Fuzz:** PQ decode/negotiation surfaces; malformed tokens (caps/audits) with PQ signatures. Run with **ASAN/LSAN** on nightly in CI.
* **Load/Perf:** handshake/sec (classical vs hybrid) on **AMD64** and **ARM**; record latency deltas and RSS.
* **Security drills:** simulate “classical break” → set `pq_only=true` and confirm safe failure & clear errors.

---

## 9) Risks & Mitigations

* **Perf/Footprint:** Larger KEM/SIG artifacts, slower handshakes. → **Session resumption**, connection pooling; cache verification within TTLs; baseline and gate overhead (≤20%).
* **Library churn:** PQ libs evolve. → Isolate behind a **thin adapter trait** in `ron-transport`/auth; pin versions; CI canaries; security review for upgrades.
* **Downgrade abuse:** Attackers force classical. → Enforce `pq_only` where mandated; **log+alert** on downgraded sessions; formal FSM check.
* **Ecosystem gaps:** Not all peers ready. → Keep hybrid optional until coverage proven; document per-peer requirements.
* **Black-swan HNDL:** Quarterly drill: replay recorded internal TLS (synthetic) and ensure `pq_only` policy fails closed with clear signals; publish findings.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL = Low** for omnigate documented.
* [ ] `pq` features compile; CI matrix includes `--features pq,pq-hybrid,pq-sign`.
* [ ] Hybrid KEX interop passes; downgrade detection and `/readyz` policy checks wired.
* [ ] PQ metrics/log fields emitted; dashboards updated; alerts for `pq_policy_violation` in place.
* [ ] Runbook updated with enable/rollback and failure modes.
* [ ] Perf numbers recorded (handshake/sign/verify, CPU/RAM) for **AMD64**/**ARM**; ARM overhead **≤ 20%** vs x86 baseline.
* [ ] SECURITY.md cross-links updated; owners ACK.
* [ ] TLA+ FSM checked in and passing model checks.

---

## 11) Role Preset — omnigate (service/BFF)

* **Primary targets:** Policy-driven negotiation; PQ **telemetry first**, **hybrid next**, **pq-only** last (operator-gated).
* **Defaults:**

  * M1: `pq_hybrid=false`, `pq_sign_algo="off"`, `pq_only=false`
  * M2: hybrid available (off by default), optional PQ signature verify
  * M3: `pq_hybrid=true` where peers support; classical fallback allowed unless `pq_only=true`
* **Non-scope:** No econ writes, no reward issuance, no long-term custody.

---

## 12) Appendix (fill as adopted)

* **Algorithms chosen (initial):**

  * KEX: **Hybrid(X25519 + ML-KEM)** for internal TLS; edge via gateway roadmap.
  * Signatures: **Ed25519** (default) + **ML-DSA** optional verify.
* **Libraries (pins/notes):**

  * `tokio-rustls 0.26.x`, `ed25519-dalek v2`, `blake3 1.x`.
  * PQ adapter: `oqs-rust v0.8.x` or equivalent (subject to audit); wrapped in adapter trait.
* **Interop notes:** peers advertise PQ support via transport/auth adapters; fallbacks logged with `downgrade_detected=1`.
* **ZK/Econ read ties:** Omnigate may **read** ZK receipts from `ron-ledger` but does not verify proofs; PQ impact is **downstream** (their QUANTUM docs).
* **Change log:**

  * 2025-10-07 — Initial QUANTUM.md; hooks/features defined; HNDL classified.
  * YYYY-MM-DD — Enable hybrid in staging; record perf deltas; update baselines.
  * YYYY-MM-DD — Default to hybrid in prod where peers ready; publish ADR.

---

**How to use:**

* Keep M1 minimal (hooks + metrics).
* Prove hybrid in staging (M2), record deltas, and wire alerts for downgrade.
* Flip defaults carefully (M3) with clear rollback.
