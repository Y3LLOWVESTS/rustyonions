Short answer: you’re good—**they already align.**
Two tiny tune-ups will make macronode’s `QUANTUM.md` perfectly “canon-anchored” to the project blueprint you pasted:

1. **Add an `inherits-from` pointer** to the repo-level file (`/docs/PROJECT_QUANTUM_BLUEPRINT.md`).
2. **Normalize the env-var names and SLOs** to the project canon (the names you showed: `PQ_HYBRID`, `PQ_ONLY`, `PQ_SIGN_ALGO`, `KEY_ROTATION_DAYS`; handshake P99 200ms/400ms).

Everything else (hybrid X25519+ML-KEM, Dilithium default + SPHINCS+ archival, HNDL focus, milestones, metrics, downgrade guards, amnesia handling) matches 1:1.

Because you like paste-ready updates, here’s a **minimal, fully in-sync replacement** for `crates/macronode/docs/QUANTUM.md` that (a) points to the project blueprint, (b) keeps macronode-specific exposure and knobs, and (c) eliminates any drift.

---

### Paste over: `crates/macronode/docs/QUANTUM.md`

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
inherits-from: /docs/PROJECT_QUANTUM_BLUEPRINT.md
---

# QUANTUM.md (macronode delta)

> This file documents **macronode-specific** PQ exposure, knobs, and acceptance.  
> All global policy/invariants/CI gates live in **/docs/PROJECT_QUANTUM_BLUEPRINT.md** (the canon).  
> If anything here conflicts with the project blueprint, the project blueprint wins.

---

## 1) Surface & Exposure (Delta)

**Role:** macronode is the multi-service “big node” profile (gateway/omnigate + index/storage/mailbox + overlay + policy/registry + sandboxed mods).

**Shor-breakable surfaces:**
- TLS 1.3 KEX (X25519) on external APIs (gateway/omnigate) and inter-service mTLS.
- Signature verification (Ed25519) for capability tokens and receipts consumed from identity/econ planes.

**Grover-affected:**
- AEAD (AES-256-GCM / ChaCha20-Poly1305), Hash (BLAKE3-256; SHA-256/384 inside TLS).  
  Note: AES-256 preserves ~128-bit effective strength under Grover; we **do not** use AES-128 for long-lived secrets.

**HNDL vectors (long-lived, at rest):**
- Audit/service logs, policy snapshots, cached manifests, index/metadata, mailbox queues.
- **Retention:** logs (weeks→months), indexes/manifests (months→years).  
- **HNDL label:** **medium → high** (focus on re-encryptability and PQ-signed receipts for non-repudiation).

**Worst-case blast radius (if classical PK breaks):**
- Recorded TLS traffic decrypted absent hybrid KEX; forged classical signatures could counterfeit caps/receipts unless PQ signatures are enforced.

---

## 2) Current Crypto Profile (Today) — for macronode surfaces

- **KEX:** X25519 (TLS 1.3).
- **Signatures (verify):** Ed25519 for caps/receipts.
- **AEAD:** AES-256-GCM and/or ChaCha20-Poly1305.
- **Hash:** BLAKE3-256 (addressing), SHA-256/384 (TLS).
- **Libraries:** tokio-rustls (rustls), ring (via rustls), ed25519-dalek, blake3, sha2.
- **Key custody:** node identity/mTLS via `ron-kms` (preferred) with soft-HSM fallback; rotation ≤ **90 days**; zeroize on reload; **amnesia mode** = RAM-only.
- **Crypto-bearing interfaces:** HTTPS APIs (/metrics, /healthz, /readyz, control), overlay/OAP frames over TLS, capability headers, manifests/receipts persisted locally.

---

## 3) Target PQ Posture (applies here; details in project blueprint)

- **Transport/TLS:** **Hybrid** X25519 + **ML-KEM-768 (Kyber-768)** when `pq_hybrid=true`.  
  Dark-launch internally first, then app-facing edges as peers support it.
- **Signatures:** **ML-DSA (Dilithium-2)** default for caps/receipts; **SLH-DSA (SPHINCS+)** for archival lanes.
- **ZK/Econ hooks:** Use ≥256-bit hash domains and leave an interface for PQ-friendly ZK commitments in econ lanes.
- **Back-compat:** Classical remains until **M3**, then hybrid is the default; explicit opt-out is audit-logged.

---

## 4) Feature Flags, Config & Env (Normalized to project canon)

```toml
# Cargo features (macronode)
[features]
pq = []                # enable PQ plumbing
pq-hybrid = ["pq"]     # Hybrid KEX (X25519 + ML-KEM)
pq-sign = ["pq"]       # PQ signature verification path
pq-only = []           # refuse classical peers (policy-driven)
````

```ini
# Config keys (mapped to macronode Config)
pq_hybrid = false            # M1=false → M3=true (default)
pq_sign_algo = "ml-dsa"      # "ml-dsa" | "slh-dsa"
pq_only = false
key_rotation_days = 90
```

**Env overlays (exact names per canon):**

* `PQ_HYBRID=true|false`
* `PQ_SIGN_ALGO=ml-dsa|slh-dsa`
* `PQ_ONLY=true|false`
* `KEY_ROTATION_DAYS=90`

**Negotiation rule:** If peer lacks PQ, fall back to classical **unless** `pq_only=true`, in which case return explicit 4xx/5xx with actionable reason.

---

## 5) Migration (Milestones) — macronode application

* **M1 (Bronze):** Land flags/env, no behavior change; label HNDL surfaces; baseline classical perf.
* **M2 (Silver):** Enable **hybrid KEX internally** (`pq_hybrid=true` on node↔service); optional PQ verification (Dilithium-2) for caps/receipts; exercise interop matrix.
* **M3 (Gold):** **Default hybrid** on app-facing edges; PQ signatures where policy requires; canary then widen; `pq_only` available per tenant/edge; runbooks/alerts live.

Perf/SLO targets (match canon): **P99 handshake ≤ 200 ms internal / ≤ 400 ms external** under hybrid. Dilithium-2 verify **P99 ≤ 2 ms/op** on x86\_64 ref.

---

## 6) Invariants (MUST) — macronode enforcement

* No security-critical path remains pure classical where policy demands hybrid.
* Symmetric **≥256-bit**, hash **≥256-bit**.
* Long-lived artifacts are **re-encryptable** under PQ keys (no format traps).
* `pq_only=true` refuses classical with explicit, actionable errors.
* Rotations upgrade algorithms without silent downgrade; rotations auditable.
* PQ feature builds pass CI; interop parity proven before defaults change.
* **Amnesia:** never persist classical-only secrets; PQ prekeys zeroized on rotation.
* Downgrade resistance: no silent PQ→classical when policy requires PQ.

(See project blueprint §6 for workspace-wide invariant text.)

---

## 7) Observability (Metrics, Logs, Tracing, Readiness)

Emit the **canonical** PQ metrics with macronode labels:

* `pq_handshake_total{algo="x25519"|"ml-kem"|"hybrid",role}`
* `pq_signature_total{algo}`, `pq_signature_failures_total{reason}`
* `pq_downgrade_events_total{edge}`
* `crypto_latency_seconds{op="kex|sign|verify",algo}` (histograms)

Readiness: `/readyz` fails if PQ is required by policy and negotiation can’t meet it.
Tracing: `pq.kex`, `pq.sign`, `pq.verify` spans with `algo` + `peer_mode`.

---

## 8) Tests & CI (Delta)

* Interop matrix: classical↔classical; hybrid↔hybrid; hybrid↔classical (downgrade only when policy allows).
* Fuzz: parsers/negotiator error paths; downgrade resistance.
* Concurrency: Loom on negotiator FSM; Sanitizers (asan/ubsan) lane; Miri on parsers where practical.
* Load: handshake/sec & P95/P99 latency with/without hybrid on x86\_64 + ARM edge.
* Drill: simulate “classical break” → set `PQ_ONLY=true`; verify safe refusal + stable health.

(See project blueprint §8 for the CI matrix and supply-chain checks.)

---

## 9) Acceptance Checklist (macronode)

* [ ] HNDL exposure labeled; owners ACK.
* [ ] Flags/env wired; CI builds `--features pq,pq-hybrid,pq-sign` on x86\_64 + ARM.
* [ ] Interop matrix passes; downgrade behavior matches policy; explicit errors on mismatch.
* [ ] PQ metrics shipped; dashboards + alerts (downgrade, handshake latency) live.
* [ ] Runbooks (enable/canary/rollback; `pq_only` playbook) verified in staging.
* [ ] Perf numbers recorded; meet SLOs or variance waivers filed.
* [ ] SECURITY.md cross-links updated (this doc + project blueprint).

---

## 10) Notes

* **Dependencies to coordinate:** `ron-transport` (hybrid KEX), `ron-auth`/`svc-passport` (PQ sigs), `ron-kms` (custody/rotation), `svc-gateway`/`svc-overlay`/`svc-index`/`svc-mailbox`/`svc-storage` for PQ-ready envelopes at rest; econ crates for PQ receipts/ZK commitments.
* **Change log (seed):**

  * 2025-09-22 — Converted macronode QUANTUM.md to **delta-only**, anchored to project blueprint; normalized env names & SLOs.

# TODO QUANTUM AND DESIGN CONSIDERATIONS FOR EACH CRATE -- VERY IMPORTANT DO NOT SKIP!

Yes—let’s lock in the **hybrid plan** exactly as you summarized:

* **One project blueprint** = governance + invariants (canonical source of truth).
* **Thin adapter crates** = `ron-pq-*` stable interfaces (no god-crate).
* **Tiny crate-local QUANTUM.md** = delta-only (crate-specific exposure/knobs/tests), pointing back to the canon.

Below is a **small, paste-ready TODO pack** so we don’t drift. It includes a succinct task list plus minimal scaffolds you can drop in when you’re ready.

---

# TODO — PQ Adapters & Per-Crate Hooks (keep in `/docs/todo/PQ_ADAPTERS.md`)

## A) Create adapter crates (workspace libs)

1. Add crates to the workspace:

   * `crates/ron-pq-kex`
   * `crates/ron-pq-sign`
   * `crates/ron-pq-metrics`
   * (optional) `crates/ron-pq-policy`

2. Wire features (uniform across adapters):

   * `pq` (root switch)
   * backend toggles (example for KEX): `kyber768_oqs`, and later `kyber768_pure` when a pure-Rust backend is viable.

3. Expose **traits only** (no global state):

   * `HybridKex` + `KexPolicy` (negotiate classical|hybrid|pq-only)
   * `Signer`/`Verifier` with labeled algos (`"ml-dsa-2"`, `"slh-dsa-s5"`)
   * Metric helpers that standardize labels (`algo`, `role`, `peer_mode`) and counters/histograms
   * Policy schema (`pq_hybrid`, `pq_only`, `pq_sign_algo`) + env overlay utilities

4. Add **docs** and **readme** snippets pointing to `/docs/PROJECT_QUANTUM_BLUEPRINT.md`.

5. CI:

   * Build matrix: `{none, pq}`, `{pq+kyber768_oqs}`, `{pq+pq-sign}`
   * Sanitizers (`asan`, `ubsan`) on negotiator code
   * Loom tests for negotiation FSM (kex downgrade resistance)

---

## B) Minimal crate scaffolds (paste-ready)

### 1) `crates/ron-pq-kex/Cargo.toml`

```toml
[package]
name = "ron-pq-kex"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[features]
default = []
pq = []
kyber768_oqs = ["pq"]      # OQS-backed Kyber-768
kyber768_pure = ["pq"]     # placeholder for future pure-Rust backend

[dependencies]
# Keep deps thin; real backends live behind features
hkdf = "0.12"
sha2 = { version = "0.10", default-features = false, features = ["sha384"] }
thiserror = "1"

# Only when kyber is enabled:
oqs = { package = "liboqs-rust", version = "0.9", optional = true }
```

### 2) `crates/ron-pq-kex/src/lib.rs`

```rust
//! Post-Quantum KEX adapter (hybrid X25519 + ML-KEM-768) — traits only by default.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KexMode {
    Classical,   // X25519 only
    Hybrid,      // X25519 + ML-KEM-768 (Kyber-768)
    PqOnly,      // PQ only (if a pure-PQ transport exists)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerMode {
    Classical,
    Hybrid,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct KexPolicy {
    pub require_pq: bool,    // maps from pq_only
    pub prefer_hybrid: bool, // maps from pq_hybrid
}

impl Default for KexPolicy {
    fn default() -> Self {
        Self { require_pq: false, prefer_hybrid: false }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KexError {
    #[error("peer does not support PQ but policy requires it")]
    PolicyRefusal,
    #[error("backend unavailable: {0}")]
    Backend(String),
    #[error("internal error")]
    Internal,
}

/// Output of a negotiated KEX: traffic secrets derived from classical + PQ shares as applicable.
pub struct TrafficSecrets {
    pub client_write_key: Vec<u8>,
    pub server_write_key: Vec<u8>,
    pub hkdf_label: &'static str,
}

/// Trait implemented by transport layers (e.g., TLS wrapper) to drive negotiation.
pub trait HybridKex {
    /// Negotiate mode with peer capabilities + local policy, then derive secrets.
    fn negotiate_and_derive(
        &self,
        peer: PeerMode,
        policy: KexPolicy,
    ) -> Result<(KexMode, TrafficSecrets), KexError>;
}

/// Reference HKDF combiner for hybrid secrets (classical ⊕ PQ).
pub fn hkdf_combine(classical_ss: &[u8], pq_ss: Option<&[u8]>) -> TrafficSecrets {
    use hkdf::Hkdf;
    use sha2::Sha384;

    let mut ikm = Vec::with_capacity(classical_ss.len() + pq_ss.map(|s| s.len()).unwrap_or(0));
    ikm.extend_from_slice(classical_ss);
    if let Some(pq) = pq_ss { ikm.extend_from_slice(pq); }

    let hk = Hkdf::<Sha384>::new(None, &ikm);
    let mut c = vec![0u8; 48];
    let mut s = vec![0u8; 48];
    hk.expand(b"client_write", &mut c).expect("hkdf ok");
    hk.expand(b"server_write", &mut s).expect("hkdf ok");

    TrafficSecrets { client_write_key: c, server_write_key: s, hkdf_label: "HKDF-SHA384" }
}
```

---

### 3) `crates/ron-pq-sign/Cargo.toml`

```toml
[package]
name = "ron-pq-sign"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[features]
default = []
pq = []
ml_dsa_2 = ["pq"]        # Dilithium-2
slh_dsa_s5 = ["pq"]      # SPHINCS+ (slow, archival)

[dependencies]
thiserror = "1"
```

### 4) `crates/ron-pq-sign/src/lib.rs`

```rust
//! PQ signatures (traits) — Dilithium-2 default, SPHINCS+ for archival lanes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigAlgo { MlDsa2, SlhDsaS5 }

#[derive(Debug, thiserror::Error)]
pub enum SigError {
    #[error("unsupported algorithm")]
    Unsupported,
    #[error("verification failed")]
    VerifyFailed,
    #[error("internal error")]
    Internal,
}

pub trait Verifier {
    fn verify(&self, algo: SigAlgo, msg: &[u8], sig: &[u8], pubkey: &[u8]) -> Result<(), SigError>;
}

pub trait Signer {
    fn sign(&self, algo: SigAlgo, msg: &[u8], sk: &[u8]) -> Result<Vec<u8>, SigError>;
}
```

---

### 5) `crates/ron-pq-metrics/Cargo.toml`

```toml
[package]
name = "ron-pq-metrics"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
prometheus = "0.14"
once_cell = "1"
```

### 6) `crates/ron-pq-metrics/src/lib.rs`

```rust
//! Canonical PQ metric helpers (counters + histograms with stable labels).

use once_cell::sync::Lazy;
use prometheus::{register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec};

pub static PQ_HANDSHAKE_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "pq_handshake_total",
        "PQ handshakes by algo and role",
        &["algo", "role"]
    ).unwrap()
});

pub static PQ_SIGNATURE_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "pq_signature_total",
        "PQ signature ops by algo",
        &["algo", "op"] // op = sign|verify
    ).unwrap()
});

pub static PQ_SIGNATURE_FAILURES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "pq_signature_failures_total",
        "PQ signature failures by reason",
        &["reason"]
    ).unwrap()
});

pub static CRYPTO_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "crypto_latency_seconds",
        "Crypto op latency by op and algo",
        &["op", "algo"]
    ).unwrap()
});

pub static PQ_DOWNGRADE_EVENTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "pq_downgrade_events_total",
        "Events where PQ was expected but classical was negotiated",
        &["edge"]
    ).unwrap()
});
```

---

### 7) (Optional) `crates/ron-pq-policy/Cargo.toml`

```toml
[package]
name = "ron-pq-policy"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[features]
default = []

[dependencies]
serde = { version = "1", features = ["derive"] }
```

### 8) `crates/ron-pq-policy/src/lib.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqPolicy {
    pub pq_hybrid: bool,         // prefer hybrid if possible
    pub pq_only: bool,           // refuse classical
    pub pq_sign_algo: String,    // "ml-dsa" | "slh-dsa"
}
```

---

## C) Workspace wiring

1. **Add to root `Cargo.toml`**

```toml
[workspace]
members = [
  "crates/ron-pq-kex",
  "crates/ron-pq-sign",
  "crates/ron-pq-metrics",
  "crates/ron-pq-policy",
  # … existing crates …
]
```

2. **Per-crate adoption checklist (drop this block into each PQ-touching crate’s README or QUANTUM.md delta)**

* [ ] Depend on `ron-pq-kex` (transport/gateway/overlay) and/or `ron-pq-sign` (auth/econ).
* [ ] Expose config keys (`pq_hybrid`, `pq_only`, `pq_sign_algo`) and env overlays.
* [ ] Emit canonical metrics via `ron-pq-metrics`.
* [ ] Implement negotiator FSM using `KexPolicy` (downgrade rules enforced).
* [ ] Add `/readyz` gate if PQ is policy-required.
* [ ] CI: build `{none,pq}`, `{pq+backend}`, run interop tests, and a Loom job for the FSM.

3. **Repo docs**

   * Ensure each crate’s `docs/QUANTUM.md` has `inherits-from: /docs/PROJECT_QUANTUM_BLUEPRINT.md` and only lists **deltas**.

---

## D) Tiny bash helpers (handy while scaffolding)

Create crates:

```
cargo new --lib crates/ron-pq-kex
cargo new --lib crates/ron-pq-sign
cargo new --lib crates/ron-pq-metrics
cargo new --lib crates/ron-pq-policy
```

Run the feature matrix (example for a transport crate using the adapters):

```
cargo build -p svc-gateway
cargo build -p svc-gateway --features pq
cargo build -p svc-gateway --features pq,kyber768_oqs
```

---

## E) Drift guardrails

* **Docs:** Project blueprint is the single source; crate deltas must not restate global policy.
* **Code:** No runtime logic in adapters beyond trait helpers—backends live behind features to keep the build graph clean.
* **CI:** PRs touching crypto must run the **PQ matrix** and the **downgrade FSM Loom test**.
* **Ops:** Metrics must always emit with `algo` labels even when PQ is off (value=0), so dashboards never go dark.

---

