

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-16
audience: contributors, security auditors, ops
crate: svc-wallet
crate-type: service|econ
pillar: 12  # Economics & Wallets
owners: [Stevan White]
review-cadence: quarterly (Gate K – continuous vigilance)

# QUANTUM.md

## 0) Purpose

Define how `svc-wallet` resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops.
Scope: algorithms in use, custody & rotation, runtime knobs, telemetry, tests, rollout plan, formal hooks, and **harvest-now-decrypt-later (HNDL)** exposure.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (breakable by Shor):**

  * **Transport KEX:** TLS 1.3 (rustls) with **X25519**.
  * **TLS Server Cert Signatures:** commonly ECDSA P-256 (issuer-dependent).
  * **Tokens/Capabilities:** macaroon-style capabilities signed **Ed25519** (via `ron-auth`/`svc-passport`).
  * **Receipts/Attestations:** Ed25519 today; moving to PQ-capable receipts.

* **Symmetric / Hash (Grover-affected only):**

  * **AEAD:** ChaCha20-Poly1305 and/or AES-256-GCM (profiles vary).
  * **Hash:** **BLAKE3-256** (≥ 256-bit effective strength).

* **Data at rest / long-lived artifacts (retention & HNDL):**

  * **ron-ledger entries** (years): **HNDL HIGH** (integrity must remain unforgeable; if confidentiality is later required, re-envelop under PQ).
  * **ron-audit logs** (years): **HNDL HIGH** (integrity and non-repudiation).
  * **Policy snapshots/manifests** (months–years): **HNDL MED**.
  * **Capabilities (macaroons)** (minutes–hours): **HNDL LOW** (short TTL dominates).

* **Session lifetime:** seconds–minutes (short life reduces HNDL for session privacy).

* **Worst-case impact if classical PKI breaks:**
  Forged capabilities or TLS MITM could authorize **unauthorized debits** at the wallet edge. Final settlement is still checked by `ron-ledger` (conservation), but economic loss/DoS risk rises. **Priority hardening:** transport auth + capability signatures + receipt signatures.

---

## 2) Current Crypto Profile (Today)

* **Algorithms:**

  * **KEX:** X25519 (TLS 1.3).
  * **Signatures:** Ed25519 (caps/receipts); ECDSA P-256 (TLS server certs typical).
  * **Symmetric/Hash:** ChaCha20-Poly1305 or AES-256-GCM; BLAKE3-256.

* **Libraries:** `rustls` / `tokio-rustls`, `ed25519-dalek`, `ring`/`evercrypt` (AEAD), `blake3`.

* **Key custody:** **ron-kms / HSM** only (no raw keys in env/files). **Rotation:** ≤ 90 days; immediate on suspicion.

* **Crypto-carrying interfaces:** HTTP/TLS control plane; capability tokens; ledger receipts; audit attestations; policy signatures.

---

## 3) Target PQ Posture (Where we’re going)

* **Transport KEX:** **Hybrid** = X25519 + **ML-KEM (Kyber-768)** when `pq_hybrid = true`.
* **Signatures:** Prefer **ML-DSA (Dilithium-II)** for receipts/caps/governance; allow **SLH-DSA (SPHINCS+)** where hash-based determinism is desired.
* **TLS posture:** classical today → **hybrid KEX** as peers permit; policy-gated.
* **Tokens/Capabilities:** dual-sig (Ed25519 + ML-DSA) or PQ-only per tenant policy; negotiated by `svc-passport`/`ron-auth`.
* **Back-compat:** classical supported through **M2**; hybrid defaults in **M3**; start sunsetting pure-classical at external edges post-M3.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (scaffold; wired via transport/auth crates)
[features]
pq = []                 # enable PQ plumbing/types
pq-hybrid = ["pq"]      # Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]        # PQ signatures (Dilithium/Sphincs+)
pq-only = []            # opt-in: refuse classical at build/runtime
```

```ini
# svc-wallet config knobs (map into Config)
pq_hybrid = false          # M1=false, M2 opt-in=true, M3 default=true
pq_sign_algo = "ml-dsa"    # "ml-dsa" | "slh-dsa" | "off"
pq_only = false            # if true, refuse classical handshakes/tokens
key_rotation_days = 90
```

* **Interoperability:** if peer lacks PQ, **negotiate** to classical unless `pq_only=true`.
* **Metrics:** PQ labels emitted even when disabled (value=0) to pre-warm dashboards.

---

## 5) Migration Plan (Milestones)

```mermaid
flowchart LR
  M1(Bronze: Hooks & Baselines) --> M2(Silver: Hybrid Enablement)
  M2 --> M3(Gold: Hybrid Default)
  M3 --> PostM3(De-risk & Deprecate classical at edges)
```

**M1 (Bronze) — Planning & Hooks**

* Add `pq` features and config (no behavior change).
* Baseline perf for classical (record handshake latency/CPU/RAM).
* CI compiles `--features pq,pq-hybrid,pq-sign` (mocks acceptable).

**M2 (Silver) — Hybrid Enablement**

* Enable **Hybrid KEX** in `ron-transport`/edge ingress (gateway/omnigate).
* Optional **PQ signatures** (dual-sig) for capabilities (behind `pq-sign`).
* Interop matrix: classical↔classical, hybrid↔hybrid, hybrid↔classical (policy-controlled).
* **Perf target:** handshake overhead ≤ **+20%** CPU and ≤ **+1.0 ms p95** vs. classical; record actuals into PERFORMANCE.md.

**M3 (Gold) — Default & Operationalization**

* Default `pq_hybrid = true` where upstreams support it.
* PQ signatures for **ledger receipts** and **caps** per tenant policy; roll via KMS bundles.
* Runbook: enable/rollback procedures; alerts for downgrade spikes.
* QUANTUM.md & SECURITY.md finalized.

**Post-M3 — De-risk & Deprecate**

* Offer **pq_only** per env; begin sunsetting pure-classical at edges.
* Periodic PQ audits; pin/update libs; reassess parameters annually.

---

## 6) Invariants (MUST)

* **[PQ-I1]** Security-critical paths use **hybrid** (ECC+PQ) or PQ-only when policy mandates.
* **[PQ-I2]** Symmetric ≥ 256-bit; hashes ≥ 256-bit (BLAKE3-256).
* **[PQ-I3]** Long-lived data (ledger/audit) is **re-signable / re-envelopable** under PQ keys without data loss.
* **[PQ-I4]** If `pq_only=true`, reject classical peers/tokens with structured errors; **no silent downgrade**.
* **[PQ-I5]** KMS **bundle rotation** upgrades algorithms atomically; forbids fallback after cutover.
* **[PQ-I6]** CI matrix exercises `pq`, `pq-hybrid`, `pq-sign`; interop parity proven.

---

## 7) Observability (Metrics, Logs, Readiness, Alerts)

**Metrics (with algo labels):**

* `pq_handshake_total{algo="x25519|ml-kem|hybrid",role="server|client"}`
* `pq_signature_total{algo="ed25519|ml-dsa|slh-dsa",kind="cap|receipt"}`
* `pq_signature_failures_total{reason}`
* `crypto_latency_seconds{op="kex|sign|verify",algo}` (histogram)
* `pq_negotiation_downgrades_total{why}`

**Readiness:** `/readyz` fails if policy requires PQ and peer/stack cannot negotiate the required mode.

**Structured logs:** include `pq_mode={off|hybrid|pq-only}`, `kex=…`, `sig=…`, `peer_mode=…`, and explicit **downgrade** reasons.

**Alert examples (Prometheus rules):**

```yaml
groups:
- name: pq-wallet
  rules:
  - alert: PQDowngradeSpike
    expr: sum(rate(pq_negotiation_downgrades_total[5m])) / sum(rate(pq_handshake_total[5m])) > 0.01
    for: 10m
    labels: {severity: warn}
    annotations: {summary: "PQ downgrade ratio >1% for 10m (investigate peers/gateway)"}

  - alert: PQHandshakeFailures
    expr: sum(rate(pq_signature_failures_total[5m])) > 0
    for: 5m
    labels: {severity: page}
    annotations: {summary: "PQ signature/handshake failures detected"}
```

---

## 8) Testing & Verification

* **Unit / property:** token/receipt verify across classical, PQ, dual-sig; DTO/frame limits unchanged by larger keys/certs.
* **Interop suite:** classical↔classical, hybrid↔hybrid, hybrid↔classical with policy gates and **clear error taxonomy**.
* **Fuzz:** PQ decoder and negotiation paths; ensure **no panics**, bounded allocations.
* **Load / perf:** measure handshakes/sec and p95 latency **with/without PQ** on x86_64 and **ARM/edge**; capture CPU/RAM deltas.
* **Security drills:** simulate “classical break” → flip `pq_only=true` cluster-wide; verify safe failure and operability for PQ-capable clients.
* **Formal (optional, tracked):** TLA+ sketch `tla/pq_negot.tla` proving:

  * Safety: no silent downgrade when `pq_only=true`.
  * Liveness: with at least one PQ-capable peer, negotiation eventually succeeds.

---

## 9) Risks & Mitigations

* **Perf/footprint:** bigger keys/certs and slower handshakes.
  *Mitigate:* TLS resumption, connection pooling, verification caching; performance budgets in §5.

* **Library churn/maturity:** PQ libs evolve rapidly.
  *Mitigate:* thin **adapter traits** around crypto calls; pinned versions; scheduled audits; fallback implementations tested.

* **Downgrade abuse:** attackers force classical.
  *Mitigate:* `pq_only` in sensitive envs; emit/alert on `pq_negotiation_downgrades_total`.

* **Ecosystem gaps:** peers may be non-PQ.
  *Mitigate:* staged rollout (M1→M3); dual-sig tokens; policy-gated transport negotiation.

* **Micronode/Amnesia specifics:** ephemeral nodes must **not persist PQ key material**.
  *Mitigate:* derive ephemeral PQ keys at boot; zeroize on crash/shutdown; ensure no swap/paging of secrets.

---

## 10) Acceptance Checklist (DoD)

* [ ] HNDL exposure labeled per artifact (ledger, audit, policy, caps).
* [ ] Features compile in CI: `pq`, `pq-hybrid`, `pq-sign`.
* [ ] Interop matrix green; explicit errors on mismatch/downgrade.
* [ ] PQ metrics exported; dashboards & alerts live.
* [ ] Runbook updated with **enable/rollback** (hybrid / pq_only).
* [ ] Perf records (handshake p95, CPU/RAM deltas, x86_64 + ARM/edge) saved to PERFORMANCE.md.
* [ ] SECURITY.md cross-links; KMS rotation steps documented.
* [ ] (Optional) TLA+ model checked for safety/liveness.

---

## 11) Role Presets (svc-wallet context)

* **Transport (gateway/omnigate/ron-transport):**
  Primary: **Hybrid KEX**; negotiation policy. Defaults: `pq_hybrid=false (M1) → true (M3)`, `pq_only=false`.

* **Identity/Policy/KMS (`ron-auth`, `svc-passport`, `ron-kms`):**
  Primary: dual-sig caps (Ed25519 + ML-DSA) optional; custody & rotation in KMS; audit every rotation. Defaults: `pq_sign=optional (M2/M3)`; rotation ≤90d.

* **Econ plane (`ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`):**
  Primary: PQ-signed **receipts** and governance actions; ledger retains conservation truth. Defaults: classical allowed until partners support PQ; add adapters now.

---

## 12) Appendix

### 12.1 Algorithms (initial selection)

* **KEX:** Hybrid (X25519 + **ML-KEM-768**) — balance of security/perf, wide support.
* **Signatures:** **ML-DSA-2 (Dilithium-II)** for tokens/receipts; **SLH-DSA-S (SPHINCS+-SHA2-128s)** for special hash-based needs.

### 12.2 Libraries & Evaluation Notes

* **TLS/Transport:** `rustls` / `tokio-rustls` (pinned to workspace versions).
* **PQ:** `liboqs-rust` or equivalent adapter (TBD post-audit); maintain an internal trait layer to decouple choices.
* **Signatures:** `ed25519-dalek` (legacy), PQ signer adapters behind feature flags.
* **Audit plan:** vendor audit or community review prior to promoting to M3.

### 12.3 Interop Notes

* Dual-sig tokens accepted (Ed25519 + ML-DSA).
* Transport negotiation prefers hybrid; tenant policy can require PQ-only.
* Non-PQ peers are supported until deprecation window closes (Post-M3).

### 12.4 Change Log

* **2025-Q3:** Added `pq` features; HNDL documented.
* **2025-Q4:** Staging: hybrid KEX canary; dual-sig caps behind flag; downgrade alerting added.
* **2026-Q1:** Target: hybrid default in supported edges; evaluate pq_only in restricted envs.

---

## Operator Quick Reference

**Enable hybrid on a canary:**

```
./svc-wallet --config /etc/ron/svc-wallet.toml --set pq_hybrid=true
```

**Force PQ-only (break-glass if classical deemed unsafe):**

```
./svc-wallet --set pq_only=true
# /readyz fails for non-PQ peers; monitor pq_negotiation_downgrades_total
```

**Rollback to classical (incident mitigation):**

```
./svc-wallet --set pq_hybrid=false --set pq_only=false
```

---

