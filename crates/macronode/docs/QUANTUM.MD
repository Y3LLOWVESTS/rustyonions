
`crates/macronode/docs/QUANTUM.md`

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-09-22
audience: contributors, security auditors, ops
crate: macronode
crate-type: node
pillar: 8
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose

Define how **macronode** resists **quantum attacks** and how we roll out **post-quantum (PQ)** cryptography without breaking interop or operations across the node’s surfaces (gateway, overlay/transport, auth/policy, storage/index, metrics/health). We:
- inventory current crypto,
- identify HNDL (Harvest-Now, Decrypt-Later) exposure,
- add runtime flags + policy controls,
- emit PQ observability,
- test for downgrade resistance,
- stage migration from classical → **hybrid PQ**.

**Amnesia mode:** when enabled, keys and PQ prekeys are held in RAM-only, aggressively zeroized on rotation or shutdown.

---

## 1) Exposure Assessment (What’s at risk?)

- **Public-key usage (Shor-breakable):**
  - **KEX (TLS 1.3):** X25519 for external APIs and inter-service mTLS.
  - **Overlay/Transport:** ECC via `ron-transport` TLS; Arti/Tor paths (Curve25519) when enabled.
  - **Signatures:** Ed25519 on capability tokens/receipts (from `ron-auth`/econ plane).
- **Symmetric/Hash (Grover-affected):**
  - **AEAD:** AES-256-GCM and/or ChaCha20-Poly1305.
  - **Hash:** BLAKE3-256; SHA-256/384 inside TLS.
  - **Grover quantification:** Grover reduces *effective* brute-force security roughly by √N. With **AES-256**, effective security ≈ 128-bit—still acceptable. We avoid AES-128 for long-lived secrets.
- **Data at rest / long-lived artifacts (node side):**
  - Node config, audit/service logs, indexes/metadata, cached manifests, mailbox queues, policy snapshots.
  - **Retention:** logs (weeks→months), indexes/manifests (months→years).
  - **HNDL risk:** **medium→high** (focus: re-encryptability + PQ-signed receipts for non-repudiation).
- **Transport/session lifetimes:** overlay frames (seconds), HTTP/TLS sessions (minutes). Short lifetimes mitigate HNDL in transit.
- **Crate-specific blast radius:** If classical PKI breaks, recorded TLS traffic could be decrypted later absent **hybrid KEX**; forged classical signatures could counterfeit caps/receipts unless PQ signatures are enforced.

> **HNDL policy:** prioritize PQ for **external edges** and **long-retention artifacts**.

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use:**
  - **KEX:** X25519 (TLS 1.3).
  - **Signatures:** Ed25519 (caps/receipts verification).
  - **AEAD:** AES-256-GCM, ChaCha20-Poly1305.
  - **Hashing:** BLAKE3-256 (content addressing, fast hashing); SHA-256/384 (TLS).
- **Libraries:** `tokio-rustls (rustls)`, `ring` (via rustls), `ed25519-dalek`, `blake3`, `sha2`.
- **Key custody:**
  - Node identity/mTLS: via **ron-kms** (preferred) or soft-HSM fallback; rotation ≤ **90 days**; zeroize on reload; **amnesia** = RAM-only.
  - Trust anchors (token/receipt verification): fetched from `ron-auth`/policy; hot-reload on rotation.
- **Crypto-bearing interfaces:** external HTTPS APIs (/metrics, /healthz, /readyz, control), overlay/OAP frames over TLS, capability tokens (headers), manifests/receipts persisted locally.

---

## 3) Target PQ Posture (Where we’re going)

- **KEX / Transport:** **Hybrid** X25519 + **ML-KEM (Kyber-768)** for TLS edges when `pq_hybrid=true`. Canary on internal links first; then external edges as peers support it.
- **Signatures:** **ML-DSA (Dilithium-2)** default for caps/receipts; **SLH-DSA (SPHINCS+)** option for archival/long-retention receipts (hedge vs. lattice concerns).
- **TLS posture:** Classical default in **M1**; **hybrid** behind flag in **M2**; **hybrid default** in **M3** for app-facing planes.
- **Tokens/capabilities:** Negotiate policy: classical or PQ signatures per tenant/edge.
- **ZK/Econ hooks:** Ensure PQ-friendly commitments where used (e.g., swap hash domains to ≥256-bit; leave interface for PQ-friendly ZK proof systems in econ lanes).
- **Back-compat:** Classical supported until **M3**; after M3, default hybrid with explicit opt-out (audit-logged).

---

## 4) Feature Flags, Config & Env Overrides

```toml
# Cargo features (macronode)
[features]
pq = []                # enable PQ plumbing
pq-hybrid = ["pq"]     # Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]       # PQ signature verification path
pq-only = []           # refuse classical peers (policy-driven)
````

```ini
# macronode config (example)
pq_hybrid = false            # M1=false → M3=true (default)
pq_sign_algo = "ml-dsa"      # "ml-dsa" | "slh-dsa"
pq_only = false              # if true, reject classical peers
key_rotation_days = 90
```

**Env var overrides (ops-friendly):**

* `PQ_HYBRID=true|false`
* `PQ_SIGN_ALGO=ml-dsa|slh-dsa`
* `PQ_ONLY=true|false`
* `KEY_ROTATION_DAYS=90`

**Interop switch:** if peer lacks PQ, negotiate classical **unless** `pq_only=true` → explicit 4xx/5xx with actionable reason.

**Metrics policy:** always emit PQ-labeled metrics (value=0 when off) for dashboard pre-wiring.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Hooks & Inventory**

* Add features/config/env overrides; no behavior change.
* Label HNDL exposure by surface; baseline classical perf (handshake P50/P95/P99, CPU/RAM).
* CI compiles `pq`, `pq-hybrid`, `pq-sign` (mock if needed); add readiness gates.

**M2 (Silver) — Dark Launch Hybrid**

* Enable **Hybrid KEX** internally (node↔service) behind `pq_hybrid=true`.
* Wire PQ signature verification (**Dilithium-2**) optional for caps/receipts.
* Interop matrix: classical↔classical; hybrid↔hybrid; hybrid↔classical (downgrade allowed only per policy).
* Perf budget: target **<20%** handshake overhead vs classical; publish actuals.

**M3 (Gold) — Hybrid Default on Edges**

* Default `pq_hybrid=true` for app-facing/gateway edges; staged/canary rollout.
* Enable PQ signatures where policy requires; **SPHINCS+** for archival lanes.
* Runbooks: single-switch enable/rollback; `pq_only` available for high-assurance tenants.
* Docs finalized; dashboards/alerts live.

**Post-M3 — De-risk & Deprecate**

* Offer `pq_only` environments; begin classical deprecation where ecosystem coverage is sufficient.
* Periodic PQ review; track upstream PQ-TLS standardization progress.

---

## 6) Invariants (MUST)

* **\[PQ-I1]** No security-critical path remains **pure classical** where policy requires hybrid.
* **\[PQ-I2]** Symmetric keys **≥256-bit**; hash outputs **≥256-bit** (BLAKE3-256 or SHA-256/384).
* **\[PQ-I3]** Long-lived artifacts must be **re-encryptable** under PQ keys; no format traps.
* **\[PQ-I4]** If `pq_only=true`, classical peers are rejected with explicit, actionable errors.
* **\[PQ-I5]** Rotation upgrades algorithms without silent downgrade; rotations are auditable.
* **\[PQ-I6]** PQ feature builds pass CI; interop parity proven before changing defaults.
* **\[PQ-I7]** **Amnesia:** never persist classical-only secrets; PQ prekeys are zeroized on rotation.
* **\[PQ-I8]** Downgrade resistance: negotiation state machine forbids silent PQ→classical fallback when policy demands PQ.

---

## 7) Observability (Metrics, Logs, Tracing, Readiness)

* **Counters:**

  * `pq_handshake_total{algo="x25519"|"ml-kem"|"hybrid",role}`
  * `pq_signature_total{algo}`, `pq_signature_failures_total{reason}`
  * `pq_downgrade_events_total{edge}` (hybrid expected but negotiated classical)
* **Latency (histograms):**

  * `crypto_latency_seconds{op="kex|sign|verify",algo}`
* **Tracing spans (examples):**

  * `pq.kex` (attrs: algo, peer\_mode), `pq.sign`, `pq.verify`
* **Readiness:**

  * `/readyz` fails if policy demands PQ but negotiation is impossible.
* **SLO alignment (perf concern):**

  * Handshake **P99** targets: internal ≤ **200ms**, external ≤ **400ms** under hybrid;
  * Verify **P99** for Dilithium-2 ≤ **2ms** per op on x86\_64 reference; SPHINCS+ lanes exempt.

---

## 8) Testing & Verification

* **Unit/property:** token/receipt verifiers (classical + PQ), frame parsers with PQ envelopes.
* **Interop suite:** classical↔classical; hybrid↔hybrid; hybrid↔classical (downgrade only if allowed).
* **Fuzzing:** PQ decoders; negotiator state machines; structured error taxonomy.
* **Concurrency:** **Loom** tests for negotiator FSM (no deadlocks/ordering bugs).
* **UB checks:** **Miri** on negotiator/parsers where practical.
* **Sanitizers:** CI jobs for `asan`/`ubsan` on negotiator build (nightly/coverage lane).
* **Load:** handshake/sec and P95/P99 latency with/without PQ on x86\_64 + ARM edge.
* **Security drills:** simulate “classical break” → set `pq_only=true`; verify safe refusal, health steady.

---

## 9) Risks & Mitigations

* **Perf/Footprint:** larger keys/certs; slower handshakes → enable session resumption; tune timeouts; prefer Kyber-768 + Dilithium-2; reserve SPHINCS+ for archival.
* **Library churn/interop:** PQ libs evolve; TLS PQ not universal → isolate via adapter traits; pin versions; keep classical fallback until coverage; canary rollouts.
* **Downgrade attacks:** enforce negotiation policy; log + **alert** on `pq_downgrade_events_total` spikes; allow `pq_only` for high-assurance tenants.
* **Supply chain:** PQ crates may pull C bindings (e.g., oqs-sys) → require cargo-deny gates, reproducible builds, checksum pinning, and license allow-lists; prefer pure-Rust where viable.
* **Ops complexity:** flags drift → centralize config; dashboards show PQ posture; runbooks are single-switch enable/rollback.

---

## 10) Acceptance Checklist (Definition of Done)

* [ ] HNDL exposure labeled per surface with owners’ ACK.
* [ ] CI builds `--features pq,pq-hybrid,pq-sign` on x86\_64 + ARM; tests green.
* [ ] Interop matrix passes; downgrade behavior matches policy; explicit errors on mismatch.
* [ ] PQ metrics emitted & scraped; dashboards & alerts live (downgrades, handshake latency).
* [ ] Runbook updated (enable, canary, rollback; `pq_only` playbook).
* [ ] Perf numbers recorded (handshake/sign/verify CPU/RAM); meet SLO targets or have variance waivers.
* [ ] SECURITY.md cross-links updated; supply-chain checks (cargo-deny) pass with PQ libs.

---

## 11) Role Presets (macronode-specific)

**Role:** node profile orchestrating gateway/overlay, identity/policy, storage/index, ops.

* **Primary Targets:** policy-driven negotiation at node edge; `/readyz` PQ gates; downgrade detection/alerting; single-switch rollback.
* **Defaults by milestone:**

  * **M1:** `pq_hybrid=false`, `pq_only=false`, `pq_sign=false` (verify classical).
  * **M2:** `pq_hybrid=true` **internally** (dark launch); `pq_sign=true` optional.
  * **M3:** `pq_hybrid=true` **default external**; `pq_sign=true` where required; `pq_only=false` globally, opt-in per tenant/edge.
* **Dependencies to coordinate:**
  `ron-transport` (hybrid KEX adapters), `ron-auth`/`svc-passport` (PQ sigs), `ron-kms` (custody/rotation), `svc-gateway`/`svc-overlay`/`svc-index`/`svc-mailbox`/`svc-storage` (PQ-ready envelopes at rest), econ crates for PQ-aware receipts/ZK commitments.

---

## 12) Appendix (living)

### 12.1 Algorithms (initial posture)

* **KEX:** **Hybrid X25519 + ML-KEM-768** (TLS edges). Classical X25519 persists until peers support hybrid.
* **Signatures:** **ML-DSA (Dilithium-2)** default; **SLH-DSA (SPHINCS+)** for archival/non-repudiation lanes.
* **Symmetric/Hash:** AES-256-GCM and/or ChaCha20-Poly1305; BLAKE3-256; SHA-256/384 in TLS.

### 12.2 Candidate libraries

* **Kyber/Dilithium/SPHINCS+:** `liboqs-rust`/`oqs-sys` or pure-Rust `pqcrypto` families (abstract via adapter traits).
* **TLS:** `tokio-rustls` (prefer PQ-KEX via negotiated extension when supported; otherwise app-layer hybrid wrapper).
* **Audit stance:** pin versions; license allow-list; reproducible builds; vendor checksums.

### 12.3 Interop notes

* Start with internal service mesh; canary external edges; classical fallback until partner coverage verified; emit downgrade metrics.

### 12.4 Change log

* **2025-09-22** — Upgraded QUANTUM.md: Grover quant, env overrides, ZK hooks, supply-chain gates, Loom/Miri/sanitizers, tracing spans, SLO budgets, Mermaid.

---

## 13) Hybrid KEX (sequence sketch)

```mermaid
sequenceDiagram
  participant C as Client (peer)
  participant N as Macronode (TLS terminator)

  C->>N: ClientHello (X25519 + ML-KEM-768 offers)
  N-->>C: ServerHello (selects Hybrid)
  Note over C,N: Classical ECDH (X25519) + PQ KEM (Kyber-768) negotiated
  C->>N: KEX params (X25519 share, KEM ct)
  N-->>C: KEX response (X25519 share, KEM ct/ss)
  Note over C,N: Derive traffic secrets from BOTH shares (HKDF expand)
  C->>N: Finished (MAC using hybrid-derived key)
  N-->>C: Finished (MAC using hybrid-derived key)
  Note over C,N: Application data protected; downgrade detection logged if classical chosen under PQ-required policy
```

---

### How to use

* Keep this file at `crates/macronode/docs/QUANTUM.md`.
* Treat the **Acceptance Checklist** as the gate for each milestone (M1→M3).
* Update **Appendix 12** as `ron-transport`, `ron-auth`, and `ron-kms` land PQ features and as interop partners mature.

