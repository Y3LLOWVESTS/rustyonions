# QUANTUM.md — svc-edge

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-17
audience: contributors, security auditors, ops
crate: svc-edge
crate-type: service
pillar: 3  # Network/Edge Plane (serving & ingress)
owners: [Stevan White]
----------------------

## 0) Purpose

Define how `svc-edge` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops.
Scope: transport algorithms, token/signature usage, where keys live, runtime knobs, metrics, tests, rollout, and **harvest-now-decrypt-later (HNDL)** exposure.

`svc-edge` is an **edge asset server**: it serves static/ranged bytes with strong ETag semantics and can **live-fill** from allow-listed HTTPS origins. It is typically **stateless** and often deployed behind a gateway/CDN. PQ risk here is dominated by **TLS key exchange/signatures** and any **control-plane tokens** used for admin surfaces.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (Shor-breakable):**

  * **Transport KEX:** X25519 via TLS 1.3 (inbound if we terminate; outbound for live-fill).
  * **Signatures:** Ed25519 (possible for admin/capability tokens or config packages, depending on deployment). Many admin flows use **macaroons (HMAC)** (symmetric; not Shor-broken).
* **Symmetric/Hash (Grover-affected only):**

  * **AEAD:** TLS uses AES-256-GCM or ChaCha20-Poly1305; our control tokens use HMAC-SHA-256 or BLAKE3-based MACs.
  * **Hashing:** BLAKE3-256 for content addressing / strong ETags (effective ≥256-bit output).
* **Data at rest / long-lived artifacts:**

  * Served content is typically **public** artifacts (packs/objects). If origin responses are cached, they are unhashed+ETagged and **not secret**.
  * **Log & artifact retention windows:** ops audit logs **180 days** (default); access logs **30 days**; crash dumps **disabled in prod**.
  * **HNDL risk labeling:** Payloads served by `svc-edge` are public ⇒ **Low** confidentiality risk. Control-plane/admin sessions and origin TLS captures ⇒ **Medium** if recorded and later decrypted.
* **Transport/session lifetime:** HTTP request/response; sessions are short (seconds). Short lifetimes reduce HNDL exposure.
* **Crate-specific blast radius (worst case):**

  * If classical PKI is broken, an adversary **impersonates allowed origins or svc-edge** during TLS handshakes → risk of **serving tainted content**, origin spoofing for live-fill, or **MITM of admin traffic**. No direct exfil of private user data (svc-edge doesn’t hold secrets/data at rest by design).

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use**

  * **KEX:** X25519 (TLS 1.3)
  * **Signatures:** Ed25519 (code signing/config bundles, optional admin auth), or **HMAC-based macaroons** for capability tokens
  * **Symmetric/Hash:** AES-256-GCM or ChaCha20-Poly1305 in TLS; HMAC-SHA-256 and BLAKE3-256 for tokens/ETags.
    *(Grover caveat: AES-256’s effective security ~128-bit under ideal quadratic speedup; we conservatively keep 256-bit keys everywhere.)*
* **Libraries:** `rustls` (tokio-rustls), `ed25519-dalek` (if used), `ring`/`sha2`/`blake3` per crate conventions
* **Key custody:**

  * TLS private keys: **ron-kms/HSM** preferred; file-based dev keys only in non-prod.
  * Macaroon roots: sealed in **ron-kms**, rotated ≤ **90 days**.
  * No raw private keys in env files; no secrets in logs.
* **Interfaces carrying crypto:**

  * Inbound/outbound **TLS**; **macaroon** headers for admin; signed **manifests/config** (optional).

---

## 3) Target PQ Posture (Where we’re going)

* **Key exchange / encryption:**

  * **Hybrid KEX** = X25519 + **ML-KEM (Kyber)** for TLS when `pq_hybrid = true`.
  * Outbound (live-fill) and inbound (if we terminate TLS).
* **Signatures:**

  * **ML-DSA (Dilithium)** for code/config signatures and optional admin tokens; **HMAC macaroons** remain valid (symmetric).
  * Optionally **SLH-DSA (SPHINCS+)** for ultra-conservative/verifiability-only paths (slower, stateless).
* **Transport TLS posture:**

  * Default **classical** now; enable **hybrid** behind a feature/config flag.
  * Gradual migration to **hybrid-by-default** once upstreams (CDN, origins, gateways) support it.
* **Tokens/capabilities:**

  * Continue **HMAC macaroon** (PQ-friendly if using ≥256-bit keys).
  * Add PQ-sig option for non-repudiable admin artifacts (Dilithium).
* **Backwards compatibility:**

  * Classical supported until **M3 (Gold)**; then **hybrid default**. **`pq_only=true`** available for high-security deployments.
* **Parameter sets (initial):** **ML-KEM-768** for hybrid KEX; **ML-DSA-65** for signatures (config/code).
* **Algorithm diversity hook:** optional **HQC** pilot via feature/config when ecosystem support appears.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (svc-edge)
[features]
pq = []                # enable PQ plumbing (types/metrics/adapter)
pq-hybrid = ["pq"]     # enable Hybrid KEX (X25519 + ML-KEM) at the TLS adapter
pq-sign = ["pq"]       # enable PQ signatures for config/code/admin artifacts
pq-only = []           # OPTIONAL: build-time hard-disable classical fallback
pq-hqc = ["pq"]        # OPTIONAL: alternate KEM pilot (when ecosystem allows)
```

```ini
# Config (svc-edge.toml)
[pq]
pq_hybrid = false            # M1 default=false → M3 default=true
pq_only = false              # if true, refuse classical handshakes/tokens
pq_sign_algo = "ml-dsa"      # "ml-dsa" | "slh-dsa"
pq_kem = "ml-kem-768"        # "ml-kem-768" | "ml-kem-1024" | "hqc-*" (pilot)
pq_sig = "ml-dsa-65"         # "ml-dsa-44|65|87" | "slh-dsa-s128|s192|s256"
key_rotation_days = 90

# Policy for peers
require_pq_for_admin = true      # admin/control-plane must be PQ or hybrid when available
allow_pq_downgrade_data = true   # data-plane can fall back (until M3) if peer lacks PQ
```

* **Interop switch:** If peer lacks PQ → **negotiate** hybrid→classical unless `pq_only=true` or `require_pq_for_admin=true`.
* **Metrics:** Always label selected modes even when disabled (0-valued counters) to ease adoption.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze, ≤ 4 weeks)** — Hooks & Baseline

  * Introduce `pq` features and config; **no behavior change** by default.
  * Inventory peers (CDN/gateway/origins) for PQ readiness; **baseline perf** for classical on **x86_64 + ARM64**.
  * CI builds with `--features pq,pq-hybrid,pq-sign` to ensure compiles everywhere.

* **M2 (Silver, ≤ 8 weeks after M1)** — Hybrid Enablement

  * Enable **Hybrid KEX** for **outbound** live-fill first (lowest blast radius).
  * Optional: PQ signatures for **config/code artifacts** validation on boot.
  * Interop tests: classical↔classical, **hybrid↔hybrid**, hybrid↔classical (**downgrade-aware**).
  * Perf target: **≤ 20%** handshake CPU/latency overhead (x86_64) and **≤ 25%** (ARM64); record actuals.

* **M3 (Gold, ≤ 12 weeks after M2)** — Default Hybrid & Ops

  * Default `pq_hybrid=true` inbound (if terminating) and outbound.
  * Enforce `require_pq_for_admin=true` in prod; document exceptions.
  * Runbooks for **rollback** (flip to classical), dashboards include PQ panels.
  * Nightly **interop** jobs cover PQ and non-PQ peers.

* **Post-M3 — De-risk & Deprecate**

  * Offer `pq_only=true` tiers; begin **sunsetting pure-classical** for external edges when ecosystem is ready.
  * **Quarterly PQ review** (algs/params/libraries).

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical handshakes **must be Hybrid PQ** when policy requires it (admin/control plane).
* **[PQ-I2]** Symmetric keys **≥ 256-bit**; hash **≥ 256-bit** (BLAKE3-256 acceptable).
* **[PQ-I3]** No silent **downgrade**: if `pq_only=true` or `require_pq_for_admin=true`, classical peers **fail closed** with clear error and audit.
* **[PQ-I4]** Key rotation upgrades algorithms **without** silently reverting to classical.
* **[PQ-I5]** PQ builds must pass full test/CI; classical and hybrid parity is proven with **interop matrices**.
* **[PQ-I6]** If any long-lived confidential artifact appears in future designs, it **must** be encryptable under PQ envelopes (not applicable to current public assets).
* **[PQ-I7]** Ultra-sensitive control surfaces (root-of-trust admin endpoints) **prefer `pq_only=true`** in regulated environments; exceptions require audit sign-off.

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics (Prometheus):**

* `pq_handshake_total{algo="classical|ml-kem|hybrid", plane="data|admin", direction="in|out", outcome="ok|fail"}`
* `pq_signature_total{algo="ed25519|ml-dsa|slh-dsa", op="sign|verify", outcome}`
* `crypto_latency_seconds_bucket{op="kex|sign|verify", algo}` (histogram)
* `pq_downgrade_events_total{plane="admin|data", reason}`
* **Gauges:**

  * `pq_adoption_ratio{plane="data|admin"} = pq_handshake_total{algo!="classical"} / clamp_min(pq_handshake_total,1)`

**PromQL examples:**

* Adoption (admin plane):
  `sum(rate(pq_handshake_total{plane="admin",algo!="classical"}[5m])) / sum(rate(pq_handshake_total{plane="admin"}[5m]))`
* Downgrade alert (near-zero post-M3):
  `sum(rate(pq_downgrade_events_total[5m])) > 0`

**Readiness:**

* `/readyz` **fails** if policy mandates PQ and current peer/stack cannot negotiate it.

**Structured logs:**

* Include `pq.mode={off|hybrid|pq-only}`, `tls.algo=…`, `peer.mode=…`, and explicit reasons on downgrade/refusal.

---

## 8) Testing & Verification

* **Unit/Property**

  * Token/macaron validators across classical/PQ (where applicable).
  * Negotiation state machine: no panic; correct refusal paths for `pq_only`.
* **Interop matrix (must be green in CI):**

| Direction | Peer Mode | svc-edge Mode   | Expectation                        |
| --------- | --------- | --------------- | ---------------------------------- |
| inbound   | classical | classical       | success                            |
| inbound   | hybrid    | hybrid          | success (hybrid suites negotiated) |
| inbound   | classical | hybrid          | success (downgrade) unless pq_only |
| inbound   | classical | pq_only         | **fail closed** + clear audit      |
| outbound  | classical | hybrid          | success (downgrade)                |
| outbound  | hybrid    | hybrid          | success                            |
| admin     | classical | require_pq=true | **fail** + alert                   |

* **Fuzz**

  * PQ handshake transcript/negotiation parsers (if exposed), token/manifest parsers.
  * Error taxonomy fuzzing for negotiation paths.
* **Load/Perf**

  * Handshake/sec with/without hybrid on x86_64 & ARM64; CPU/latency deltas recorded.
  * Target overhead: **≤ 20%** (x86_64) / **≤ 25%** (ARM64) p95; enable **session resumption**.
* **Security Drills**

  * “Classical break” game day: flip `pq_only=true` cluster-wide; confirm safe failure, alerting, rollback.

Artifacts: PromQL dashboards for PQ counters; perf tables per arch; interop reports archived per release.

---

## 9) Risks & Mitigations

* **Perf/Footprint**: Kyber/Dilithium enlarge handshakes/certs → **session resumption**, **TLS keep-alive**, **edge-side connection pooling** to origins.
* **Library maturity**: PQ TLS integrations may be experimental.

  * **Mitigation:** wrap KEM/signature behind a **`CryptoProvider` trait** + feature-gated backends (e.g., rustls PQ adapter).
  * Pin versions; run **`cargo deny`** on every release; maintain a **known-good matrix** in CI.
* **CDN/Origin compatibility**: some peers lack PQ → staged rollout (outbound first), **allow downgrade** on data plane until M3, but **never** on admin plane.
* **Key management**: larger keys/artifacts → ensure **ron-kms/HSM** sizing, rotation, and backup plans updated.
* **Alert fatigue**: PQ downgrade noise → rate-limit alerts; prefer counters with SLO-based alerting.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL** labeled (**Low** payload, **Medium** control plane).
* [ ] `pq` features compile; CI matrix includes `--features pq,pq-hybrid,pq-sign`.
* [ ] Hybrid KEX interop (inbound/outbound) passes; clear downgrade errors.
* [ ] PQ metrics/logs present; dashboards wired (pq_* and crypto_* panels).
* [ ] Runbook updated with enable/rollback steps and SLOs.
* [ ] Perf deltas recorded for x86_64/ARM64; resumption tested.
* [ ] SECURITY.md cross-links & custody/rotation updated; owners ACK.
* [ ] **Admin plane downgrade rate == 0** for **7 consecutive days** post-M3.
* [ ] **Adoption ratio (data plane) ≥ 70% hybrid** within **30 days**, or exception log signed by owners.
* [ ] Perf artifacts uploaded: `{arch, algo, p50/p95, cpu%, rss}` JSON per build.

---

## 11) svc-edge Role Preset (concrete)

* **Primary targets:** Transport TLS adapters (inbound if terminating; outbound live-fill), admin/control-plane auth, config/code sig verification.
* **Defaults:**

  * M1: `pq_hybrid=false`, `require_pq_for_admin=false`, `allow_pq_downgrade_data=true`
  * M3: `pq_hybrid=true`, `require_pq_for_admin=true`, `allow_pq_downgrade_data=true` (tighten later)
  * `pq_only=false` except for restricted environments.

---

## 12) Appendix (initial fill)

* **Algorithms chosen (initial):**

  * KEX = **Hybrid(X25519 + ML-KEM-768)**
  * SIG = **Ed25519** (compat) + **ML-DSA-65** (policy artifacts)
* **Libraries:**

  * `tokio-rustls` + PQ adapter (feature-gated), `ed25519-dalek`; PQ sig crate **TBD**—gated behind `pq-sign`.
* **Interop peers (initial):**

  * CDN-A (classical), CDN-B (pilot hybrid Q1), Origin-X (pilot hybrid Q1), Origin-Y (classical).
* **Change log:**

  * 2025-Q4: add `pq` features & metrics
  * 2026-Q1: enable outbound hybrid to Origin-X in staging
  * 2026-Q2: **M3**—hybrid default; admin PQ required in prod

---

### Quick Operator Cheatsheet

**Enable hybrid, require PQ on admin, allow downgrade on data plane:**

```ini
[pq]
pq_hybrid = true
require_pq_for_admin = true
allow_pq_downgrade_data = true
pq_only = false
```

**Emergency “classical break” posture (cluster-wide):**

```ini
[pq]
pq_hybrid = true
require_pq_for_admin = true
allow_pq_downgrade_data = false
pq_only = true
```
