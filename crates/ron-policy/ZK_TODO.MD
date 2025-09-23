I’m going to make the ZK seam concrete with paste-ready files you can drop in. These are **minimal, compiling stubs** that establish the trait boundary, config, feature flags, and `/version` surfacing—so you can iterate without pulling in heavy ZK deps yet. (All real proving/verifying libs can be added later under the same trait.)

---

# `crates/ron-policy/Cargo.toml` (add/replace relevant parts)

```toml
[package]
name = "ron-policy"
version = "0.1.0"
edition = "2021"

[features]
default = ["zk-nop"]
# Enable the ZK interface (trait + types). No external ZK backend by default.
zk = []
# Provide a no-op verifier implementation that always returns false (for tests/CI).
zk-nop = ["zk"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "2"
```

---

# `crates/ron-policy/src/zk/mod.rs`

```rust
//! ZK Verifier seam for policy: trait + types + factory.
//! Keep this lightweight: no backend deps here; backends live behind features.

pub mod types;
pub mod verifier;

#[cfg(feature = "zk-nop")]
pub mod nop;

use crate::zk::types::{ProofBytes, VerifyingKeyBytes};
use crate::zk::verifier::{BatchItem, ZkVerifier, ZkVerifierFactory, ZkVerifierKind};

/// Public constructor used by macronode to obtain a verifier instance.
/// For now, only `ZkVerifierKind::NoOp` is wired unless a real backend is added.
pub fn make_verifier(kind: ZkVerifierKind, vk: Option<VerifyingKeyBytes>) -> Box<dyn ZkVerifier> {
    ZkVerifierFactory::make(kind, vk)
}

/// A tiny, compile-time self-test that can be called from health checks
/// to ensure the ZK seam is wired (does NOT validate math, only flow).
pub fn seam_smoke_test(verifier: &dyn ZkVerifier) -> bool {
    let bogus_vk = VerifyingKeyBytes::from(vec![0u8; 32]);
    let bogus_proof = ProofBytes::from(vec![1u8; 64]);

    let single = verifier.verify(&bogus_vk, &bogus_proof, b"domain:policy", b"context");
    let batch = verifier.batch_verify(
        b"domain:policy",
        &[
            BatchItem {
                vk: &bogus_vk,
                proof: &bogus_proof,
                public_input: b"context",
            },
        ],
    );
    // Both should be false for the no-op; real backends may differ but test shows surface is alive.
    !single && !batch
}
```

---

# `crates/ron-policy/src/zk/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// Raw bytes for a proof (opaque to policy).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofBytes(#[serde(with = "serde_bytes")] Vec<u8>);

impl ProofBytes {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}
impl From<Vec<u8>> for ProofBytes {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

/// Raw bytes for a verifying key (opaque to policy).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyingKeyBytes(#[serde(with = "serde_bytes")] Vec<u8>);

impl VerifyingKeyBytes {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}
impl From<Vec<u8>> for VerifyingKeyBytes {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

/// A tiny helper for hashing/config pinning of parameters (e.g., via KMS).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParamHash(pub [u8; 32]); // e.g., BLAKE3-256 digest of params
```

---

# `crates/ron-policy/src/zk/verifier.rs`

```rust
use crate::zk::types::{ProofBytes, VerifyingKeyBytes};

/// Enumerates supported verifier kinds.
/// Extend with "Snark(Bls12_381)", "Stark", etc.
#[derive(Clone, Copy, Debug)]
pub enum ZkVerifierKind {
    /// No-op verifier: always returns false (safe default).
    NoOp,
}

/// Batch verification item.
pub struct BatchItem<'a> {
    pub vk: &'a VerifyingKeyBytes,
    pub proof: &'a ProofBytes,
    /// Public input encoding (domain-separated). Opaque to policy.
    pub public_input: &'a [u8],
}

/// ZK verification trait: stable policy boundary.
/// Real backends implement this trait without changing policy code.
pub trait ZkVerifier: Send + Sync {
    /// Verify a single proof under the given domain separation and input.
    fn verify(
        &self,
        vk: &VerifyingKeyBytes,
        proof: &ProofBytes,
        domain_sep: &[u8],
        public_input: &[u8],
    ) -> bool;

    /// Verify a batch of proofs. Implementations SHOULD be faster than N singles.
    fn batch_verify(&self, domain_sep: &[u8], batch: &[BatchItem<'_>]) -> bool;
}

/// Factory for ZK verifiers. Constructs implementations behind trait objects.
pub struct ZkVerifierFactory;

impl ZkVerifierFactory {
    pub fn make(kind: ZkVerifierKind, _vk: Option<VerifyingKeyBytes>) -> Box<dyn ZkVerifier> {
        match kind {
            ZkVerifierKind::NoOp => Box::new(crate::zk::nop::NoOpVerifier::default()),
        }
    }
}
```

---

# `crates/ron-policy/src/zk/nop.rs` (feature `zk-nop`)

```rust
use super::verifier::{BatchItem, ZkVerifier};

#[derive(Default)]
pub struct NoOpVerifier;

impl ZkVerifier for NoOpVerifier {
    fn verify(
        &self,
        _vk: &crate::zk::types::VerifyingKeyBytes,
        _proof: &crate::zk::types::ProofBytes,
        _domain_sep: &[u8],
        _public_input: &[u8],
    ) -> bool {
        // Safe default: never accept. Real backends override with true on valid proof.
        false
    }

    fn batch_verify(&self, _domain_sep: &[u8], _batch: &[BatchItem<'_>]) -> bool {
        false
    }
}
```

---

# `crates/macronode/Cargo.toml` (add feature flags surfaced via /version)

```toml
[package]
name = "macronode"
version = "0.1.0"
edition = "2021"

[features]
default = []
# Plumb ZK presence into /version (the verifier lives in ron-policy).
zk = ["ron-policy/zk"]

[dependencies]
axum = { version = "0.7", features = ["tokio", "http1", "http2", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.47", features = ["rt-multi-thread", "macros", "io-util"] }
tracing = "0.1"
ron-policy = { path = "../ron-policy" }
```

---

# `crates/macronode/src/version.rs`

```rust
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct VersionInfo {
    name: &'static str,
    semver: &'static str,
    git_sha: &'static str,
    features: Features,
}

#[derive(Serialize)]
struct Features {
    zk: bool,
    amnesia: bool,
    arti: bool,
}

pub fn router() -> Router {
    Router::new().route("/version", get(version))
}

async fn version() -> Json<VersionInfo> {
    // These env vars can be set in build.rs or CI to embed metadata.
    let semver = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    let git_sha = option_env!("GIT_SHA").unwrap_or("unknown");

    Json(VersionInfo {
        name,
        semver,
        git_sha,
        features: Features {
            zk: cfg!(feature = "zk"),
            amnesia: cfg!(feature = "amnesia"),
            arti: cfg!(feature = "arti"),
        },
    })
}
```

---

# `crates/macronode/src/config.rs` (ZK config seam; keep it simple)

```rust
use serde::{Deserialize, Serialize};

/// Macronode configuration fragment for ZK verifier wiring.
/// Real deployments pin keys/params by hash (e.g., BLAKE3) and load bytes from KMS.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkConfig {
    /// Select verifier kind; start with "NoOp" until a backend is integrated.
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Hex/base64 of verifying key bytes (optional).
    pub verifying_key_b64: Option<String>,
    /// BLAKE3-256 hash of the verifying key or parameter bundle.
    pub param_hash_hex: Option<String>,
    /// Domain separation tag for policy proofs.
    #[serde(default = "default_domain")]
    pub domain_sep: String,
}

fn default_kind() -> String { "NoOp".to_string() }
fn default_domain() -> String { "domain:policy".to_string() }

impl ZkConfig {
    pub fn enabled(&self) -> bool {
        self.kind.to_lowercase() != "noop"
    }
}
```

---

# `crates/macronode/src/main.rs` (show wiring snippets)

```rust
use axum::{routing::get, Router};
use ron_policy::zk::{make_verifier};
use ron_policy::zk::types::{VerifyingKeyBytes};
use ron_policy::zk::verifier::ZkVerifierKind;

mod version;
mod config; // your loader merges ZkConfig into global node config

#[tokio::main]
async fn main() {
    // … load your signed, versioned config bundle …
    // let cfg = load_config(); // (pseudo)

    // Example: construct a verifier (NoOp unless real backend added)
    let zk_kind = ZkVerifierKind::NoOp;
    let vk = None::<VerifyingKeyBytes>;
    let verifier = make_verifier(zk_kind, vk);

    // Optional seam smoke: proves the boundary is live (not the math)
    let _ok = ron_policy::zk::seam_smoke_test(verifier.as_ref());

    // Router with /version
    let app = Router::new()
        .merge(version::router())
        .route("/healthz", get(|| async { "ok" }))
        .route("/readyz", get(|| async { "ready" }));

    // … serve with axum::serve + your TLS/transport wiring …
    // axum::serve(listener, app).await.unwrap();
}
```

---

# `crates/ron-policy/tests/zk_seam.rs` (optional quick test)

```rust
#[cfg(feature = "zk-nop")]
#[test]
fn zk_nop_seam_is_wired() {
    use ron_policy::zk::{make_verifier, seam_smoke_test};
    use ron_policy::zk::verifier::ZkVerifierKind;

    let v = make_verifier(ZkVerifierKind::NoOp, None);
    assert!(seam_smoke_test(v.as_ref()), "seam smoke should pass");
}
```

---

# `docs/blueprints/ZK_CIRCUITS_CATALOG.MD` (drop-in doc stub)

```markdown
---
title: ZK Circuits Catalog
version: 0.1.0
status: draft
last-updated: 2025-09-22
audience: contributors, security auditors, ops
---

## Purpose
Catalog the zero-knowledge circuits that macronode MAY verify at capability and policy boundaries without revealing witnesses/PII.

## Circuits

1) Capability Attributes (Selective Disclosure)
- Goal: Prove capability has attributes {role ∈ GroupX, not expired} without revealing identity.
- Public Input: capability commitment, expiry, group root.
- Domain: `domain:capability`.
- Verifier: `ZkVerifierKind::<backend>`; VK pinned via KMS; param hash recorded.

2) Quota Compliance
- Goal: Prove “requests_used < quota_limit” using a committed counter (private).
- Public Input: commitment root, limit, current epoch.
- Domain: `domain:quota`.

3) Audit Inclusion (Merkle)
- Goal: Prove event is included in transparency log without revealing neighbors.
- Public Input: Merkle root + event hash.
- Domain: `domain:audit`.

### Operational Notes
- Verifier keys are authenticated via KMS; param hashes (BLAKE3-256) pinned in config.
- Witness material is never logged or persisted; amnesia mode zeroizes all buffers.
- Batch verification path required for throughput (target: p95 ≤ 15 ms/proof for small preds).
```

---

## What you get now

* A **stable trait boundary** (`ZkVerifier`) inside `ron-policy`, with a safe **no-op backend** that compiles today.
* Macronode **/version** shows whether `zk` feature is enabled.
* A **config seam** (`ZkConfig`) to pin VK/param hashes when you’re ready.
* A tiny **health smoke** to assert the seam is alive (without math).
* A **catalog** doc to keep circuits and parameters auditable.

If you want, I can follow up by adding:

* A **real backend feature** (e.g., `zk-snark-bls12_381`) wired to a well-maintained crate.
* `/policy/verify` handler examples in `svc-policy` with batch paths and metrics.
* KMS loaders that pin param hashes and pull VK bytes securely.
