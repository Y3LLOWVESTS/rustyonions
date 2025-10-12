**File:** `crates/svc-dht/QUANTUM.md`

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-11
audience: contributors, security auditors, ops
crate: svc-dht
crate-type: service
pillar: 10
owners: [Stevan White]
review-cadence: quarterly
---

# QUANTUM.md — svc-dht

## 0) Purpose
Describe how `svc-dht` (Kademlia/Discv5 discovery service) resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or operations. Scope includes: current algorithms, key custody, runtime knobs, telemetry, tests, rollout plan, and **harvest-now-decrypt-later (HNDL)** exposure.

> **MSRV review:** repo-wide MSRV is `1.80.0`. Re-evaluate **quarterly** and bump only when the full workspace CI matrix is green (helps unlock PQ library fixes).

---

## 1) Exposure Assessment (What’s at risk?)
- **Public-key (Shor-breakable):**
  - **Transport KEX:** X25519 in TLS 1.3 via `ron-transport`.
  - **Signatures:** Ed25519 for control-plane/auth (capability macaroons, admin ops, signed audit events), node identities, and bootstrap fingerprints.
- **Symmetric/Hash (Grover-affected):**
  - **AEAD:** ChaCha20-Poly1305 and/or AES-256-GCM.
  - **Hash:** BLAKE3-256 (and/or SHA-256 in third-party stacks).
  - **Guidance:** Prefer **AES-256-GCM** where available to keep ≥256-bit Grover margin; ChaCha20-Poly1305 is acceptable with 256-bit keys.
- **Data at rest / long-lived artifacts:**
  - `svc-dht` is **stateless**; optional warm caches (macronode) are discardable. Long-lived: **governance/audit logs** (signed).
  - **Retention:** governance/audit ≥ **90 days**; caches ≤ hours.
  - **HNDL risk:** **Low** for DHT payloads; **Medium** for control-plane artifacts (forgery risk if signatures remain classical).
- **Session lifetime:** seconds–minutes → low HNDL for transport payloads; identity/auth still at risk.
- **Worst-case classical break:** spoofed bootstraps (eclipse), forged admin ops → traffic steering, lookup corruption. PQ priorities: **control-plane & bootstrap trust**, then transport.

---

## 2) Current Crypto Profile (Today)
- **Algorithms:**
  - **KEX:** X25519 (TLS 1.3).
  - **Signatures:** Ed25519 (caps/audit/node IDs).
  - **Symmetric/Hash:** ChaCha20-Poly1305 or AES-256-GCM; BLAKE3-256.
- **Libraries (pin & audit quarterly):**
  - `ron-transport` (rustls-based), `ed25519-dalek` 2.x, `blake3` 1.x, platform AEADs.
- **Key custody:** Admin/capability signing keys in **ron-kms/HSM**; transport keys ephemeral (Micronode) or KMS-backed (Macronode). Rotation **≤90 days**; **no raw keys** in env/files.
- **Crypto-bearing interfaces:** TLS sessions, macaroon capabilities, signed audit entries, bootstrap fingerprints, optional Tor (`arti`) link.

---

## 3) Target PQ Posture (Where we’re going)
- **KEX/Encryption:** **Hybrid KEX** = X25519 + **ML-KEM-768** (Kyber-768) where supported; pure-classical allowed M1–M2; **pq_only** mode for regulated envs.
- **Signatures:** **ML-DSA-87** (Dilithium-3) primary for control-plane; **SLH-DSA-128s** (SPHINCS+) optional for special deployments (e.g., ultra-conservative).
- **Transport TLS:** remains classical until **M3**; when `pq_hybrid = true`, advertise/choose hybrid if peer supports.
- **Tokens/capabilities:** add PQ signature option; per-tenant policy decides required families.
- **Back-compat:** classical interop through **M2**; default hybrid in **M3**; **Post-M3** offers **pq_only** toggles per plane.

---

## 4) Feature Flags & Config (How to turn it on)
```toml
# Cargo (svc-dht)
[features]
pq = []                # enable PQ plumbing
pq-hybrid = ["pq"]     # hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]       # PQ signatures for control-plane (Dilithium/SPHINCS+)
pq-only = []           # hard-disable classical at build (rare; prefer runtime knobs)
````

```ini
# svc-dht config (runtime)
pq_hybrid = false           # M1 default -> true in M3
pq_sign_algo = "ml-dsa"     # "ml-dsa" | "slh-dsa"
pq_only = false             # refuse classical peers for all planes when true
pq_only_control = false     # require PQ for control-plane/admin only
key_rotation_days = 90

# Governance mapping
REQUIRE_PQ_ON = false       # operational guard for PQ on control-plane (maps to pq_only_control)
```

**Ops toggles (copy-paste):**

```bash
# Turn on hybrid KEX
ronctl set svc-dht pq_hybrid=true

# Enforce PQ on control-plane only
ronctl set svc-dht pq_only_control=true

# Full PQ-only (refuse classical everywhere)
ronctl set svc-dht pq_only=true
```

*Interoperability:* if peer lacks PQ → **negotiate** hybrid when available, else **refuse** only when `pq_only*`/`REQUIRE_PQ_ON` is set.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Planning & Hooks**

* Land `pq`, `pq-hybrid`, `pq-sign` features and config; no behavior change by default.
* Document exposure; baseline classical perf on x86_64 & ARM.
* Ensure unit tests compile with PQ features (mock adapters allowed).

**M2 (Silver) — Hybrid Enablement**

* Enable **hybrid KEX** behind `pq_hybrid=true` in `ron-transport`.
* Add **PQ signatures** option for macaroon/audit events.
* Interop: classic↔classic, hybrid↔classic (allowed), hybrid↔hybrid (preferred).
* Perf budget: handshake overhead **≤20%** (p95); record actuals & tail.

**M3 (Gold) — Default & Operationalization**

* Default `pq_hybrid=true` for external links; keep classical fallback unless `pq_only*` set.
* Control-plane end-to-end **ML-DSA-87** support; policy can **require** it.
* Runbooks for enable/rollback; chaos drill simulates “classical break” (flip pq_only* on).

**Post-M3 — De-risking & Deprecation**

* Offer **pq_only** per plane; sunset pure-classical when **≥90%** of peers advertise hybrid for **30 consecutive days** (measured by metrics).
* Re-evaluate algorithms annually; re-sign long-lived artifacts under PQ.

**Mermaid — Rollout**

```mermaid
flowchart LR
M1[Hooks & Baseline] --> M2[Hybrid Enabled (opt-in)]
M2 --> M3[Hybrid Default + PQ Sign Control-Plane]
M3 --> M4[Deprecate Classical (>=90% peers hybrid for 30d)]
```

---

## 6) Invariants (MUST)

* **[PQ-I1]** No security-critical path remains **pure classical** once a PQ/hybrid option exists (transport=hybrid; control-plane=PQ sign).
* **[PQ-I2]** Symmetric strength **≥256-bit**; **hash outputs ≥256-bit** (AES-256-GCM / ChaCha20-Poly1305; BLAKE3-256).
* **[PQ-I3]** Long-lived artifacts (audit/governance receipts) are **signable** under PQ; **re-signing** procedure exists.
* **[PQ-I4]** If `pq_only* = true`, classical handshakes/tokens are **refused** with structured errors and `Retry-After` guidance.
* **[PQ-I5]** Key rotation upgrades algorithms without silent fallback; rotations are **audited**.
* **[PQ-I6]** CI covers **PQ feature builds**; interop parity with classical is proven (tests + soak).
* **[PQ-I7]** **No downgrade without quorum + 24h delay** (see GOVERNANCE.md); attempted downgrades are logged/alerted.

---

## 7) Observability (Metrics, Logs, Readiness)

Expose per-op metrics with algorithm labels:

* `pq_handshake_total{algo="x25519"|"ml-kem"|"hybrid",role="client|server"}`
* `pq_signature_total{algo="ed25519"|"ml-dsa"|"slh-dsa",kind="cap|audit|id"}`
* `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify",algo}` (histogram)
* `pq_enforced` (0/1), `pq_downgrade_attempt_total`
* **Readiness:** `/readyz` **fails** if policy requires PQ (`REQUIRE_PQ_ON`/`pq_only*`) and negotiation can’t satisfy it.
* **Structured logs:** include `pq_mode={off|hybrid|pq-only}`, `algo=*`, `peer_mode=*`, `downgrade_reason=*`.

**PromQL (alerts):**

```promql
ALERT PqDowngradeAttempts
  IF increase(pq_downgrade_attempt_total[10m]) > 0
  LABELS { severity="ticket" }
  ANNOTATIONS { summary="PQ downgrade attempts detected" }

ALERT PqEnforcementBroken
  IF pq_enforced == 1 AND (probe_success{job="svc-dht-readyz"} == 0)
  FOR 2m
  LABELS { severity="page" }
  ANNOTATIONS { summary="PQ enforcement active but service not ready" }
```

---

## 8) Testing & Verification

* **Unit / property:** frame parsers, token validators under PQ+classical; adapters property-tested (encode/verify round-trip, refusal semantics).
* **Mutation tests (adapters):** run `cargo mutants` (or equivalent) over KEX/sign adapters; Gold gate ≥ **70%** mutation score.
* **Interop suite:** classic↔classic, hybrid↔classic, hybrid↔hybrid; include Tor/`arti` where supported.
* **Fuzzing:** PQ decoders & negotiation paths (bounds checks; no panics; structured rejects).
* **Load:** handshake/s throughput; p95/p99 latency, CPU/RAM deltas with PQ on x86_64 & ARM.
* **Security drills:** simulate “classical break” (flip `REQUIRE_PQ_ON=true`); verify safe failure + operator guidance.

---

## 9) Risks & Mitigations

* **Performance/footprint:** Larger keys/slower handshakes. → **Mitigate** with session resumption, handshake caching, tuned α/β hedging to hide tail.
* **Library churn/supply chain:** PQ libs evolve. → **Mitigate** with thin adapter trait, pinned versions, **`cargo deny` advisories** on PQ deps; quarterly audit.
* **Downgrade abuse:** Forced classical. → **Mitigate** with `pq_only*`, downgrade alerts, and **quorum + 24h delay** to disable enforcement.
* **Interop gaps:** Peers not PQ-ready. → **Mitigate** with hybrid first; per-tenant policy; sunset only after **≥90%** hybrid adoption for **30d**.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL labeled (**Control-plane: Medium**, **Data-plane: Low**).
* [ ] `pq` features compile; CI matrix includes `--features pq,pq-hybrid,pq-sign`.
* [ ] Hybrid KEX interop passes; refusal semantics correct when `pq_only*` true.
* [ ] PQ metrics/alerts wired (`pq_enforced`, `pq_downgrade_attempt_total`).
* [ ] RUNBOOK updated (enable/rollback); GOVERNANCE enforces quorum + 24h delay for disabling PQ.
* [ ] Perf numbers captured (handshake/sec, p95/p99, CPU/RAM) on x86_64 & ARM; overhead ≤ **20%** p95.
* [ ] SECURITY.md cross-links updated; owners ack.
* [ ] `cargo deny` passes on PQ dependency set; versions pinned.

---

## 11) Role Presets (svc-dht-relevant)

### transport (`ron-transport`, `svc-arti-transport`)

* **Primary:** **Hybrid KEX**; negotiation policy; `pq_only_control`.
* **Defaults:** `pq_hybrid=false (M1) → true (M3)`; `pq_only=false` except regulated envs.

### identity/policy/kms (`ron-kms`, `ron-auth`, `svc-passport`, `ron-policy`, `ron-audit`)

* **Primary:** **PQ signatures** for capabilities/audit; custody & rotation; downgrade alerts.
* **Defaults:** `pq_sign=false (M1) → optional (M2/M3)`; rotation ≤ 90 days.

### gateway/overlay/node (`svc-gateway`, `svc-overlay`, `micronode`, `macronode`)

* **Primary:** policy-driven PQ negotiation; `/readyz` gating when required; clear errors.
* **Defaults:** enable hybrid where upstream supports; otherwise log+allow until M3.

---

## 12) Appendix (fill as adopted)

**Algorithms (initial posture):**

* **KEX:** **Hybrid** (X25519 + ML-KEM-768).
* **Signatures:** **ML-DSA-87** (primary), **SLH-DSA-128s** (optional).
* **Symmetric/Hash:** AES-256-GCM / ChaCha20-Poly1305; BLAKE3-256.

**Libraries (pin & audit):**

* `ron-transport` (rustls + PQ adapter), `ed25519-dalek` 2.x, PQ adapter crate (e.g., `oqs`/equiv) behind features.
* Audit via `cargo deny` (quarterly); update notes recorded below.

**Interop sunset note:**

* Pure-classical allowed until **M3**. Deprecation when **≥90%** peers advertise hybrid for **30d**.

**Change log:**

* 2025-10-11 — Draft created; features/knobs defined; alerts/mermaid/ops toggles added; supply-chain audit policy noted.

---

```
```
