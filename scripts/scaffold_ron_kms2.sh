#!/usr/bin/env bash
# Scaffolder for crates/ron-kms2 — structure only, no Rust logic.
set -euo pipefail

ROOT="crates/ron-kms2"

# Helpers
mkd() { mkdir -p "$1"; }
mkf() { mkdir -p "$(dirname "$1")"; cat > "$1"; }

echo "Scaffolding $ROOT ..."

# ------------------------------------------------------------------------------
# Directories
# ------------------------------------------------------------------------------
DIRS="
$ROOT
$ROOT/.cargo
$ROOT/src
$ROOT/src/traits
$ROOT/src/sealed
$ROOT/src/backends
$ROOT/src/ops
$ROOT/src/pq
$ROOT/src/util
$ROOT/docs
$ROOT/tests
$ROOT/tests/unit
$ROOT/tests/vectors
$ROOT/tests/vectors/ed25519
$ROOT/tests/vectors/mlkem
$ROOT/tests/vectors/mldsa
$ROOT/benches
$ROOT/fuzz
$ROOT/fuzz/fuzz_targets
$ROOT/xtask
$ROOT/xtask/src
$ROOT/testing
$ROOT/testing/kms-dev-server
$ROOT/testing/kms-dev-server/src
"

for d in $DIRS; do
  mkd "$d"
done

# ------------------------------------------------------------------------------
# Top-level files
# ------------------------------------------------------------------------------
mkf "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "ron-kms2"
version = "0.0.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "RustyOnions KMS (library-only) — scaffold"
repository = "https://example.invalid/RustyOnions"
publish = false

[lib]
name = "ron_kms2"
path = "src/lib.rs"

[features]
# Crypto algorithms (off by default in scaffold)
mlkem = []
mldsa = []
slhdsa = []
soft-seal = []
with-metrics = []

[dependencies]
# Intentionally empty for scaffold
EOF

mkf "$ROOT/README.md" <<'EOF'
# ron-kms2 (scaffold)

Library-only KMS crate scaffold for RustyOnions. No servers, no HTTP here.
This is structure only — fill modules with real implementations later.
EOF

mkf "$ROOT/CHANGELOG.md" <<'EOF'
# Changelog (scaffold)
- 0.0.0: Initial scaffold.
EOF

mkf "$ROOT/LICENSE-MIT" <<'EOF'
Permission is hereby granted, free of charge, to any person obtaining a copy
... (MIT license text placeholder) ...
EOF

mkf "$ROOT/LICENSE-APACHE" <<'EOF'
Apache License
Version 2.0, January 2004
... (Apache-2.0 license text placeholder) ...
EOF

mkf "$ROOT/CODEOWNERS" <<'EOF'
* @owner1 @owner2
EOF

mkf "$ROOT/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["clippy", "rustfmt"]
EOF

mkf "$ROOT/deny.toml" <<'EOF'
# cargo-deny scaffold (fill with project rules later)
[advisories]
yanked = "warn"

[bans]
multiple-versions = "warn"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
EOF

mkf "$ROOT/.cargo/config.toml" <<'EOF'
[build]
rustflags = []

[term]
verbose = false
EOF

# ------------------------------------------------------------------------------
# src/ (library)
# ------------------------------------------------------------------------------
mkf "$ROOT/src/lib.rs" <<'EOF'
// ron-kms2 — library-only scaffold.
// Intentionally minimal; add exports as modules are implemented.

pub mod types;
pub mod error;
pub mod metrics;
pub mod config;
pub mod traits;
pub mod sealed;
pub mod backends;
pub mod ops;
pub mod pq;
pub mod util;
pub mod prelude;
EOF

mkf "$ROOT/src/types.rs" <<'EOF'
// Common KMS types (scaffold)
#[allow(dead_code)]
pub struct KeyId(pub String);

#[allow(dead_code)]
pub enum Alg {
    Ed25519,
    X25519,
    MlKem,
    MlDsa,
    SlhDsa,
}
EOF

mkf "$ROOT/src/error.rs" <<'EOF'
// Error taxonomy (scaffold)
#[derive(Debug)]
pub enum KmError {
    NoSuchKey,
    SealedCorrupt,
    AlgUnavailable,
    Expired,
    Entropy,
    Backend,
}
pub type KmResult<T> = Result<T, KmError>;
EOF

mkf "$ROOT/src/metrics.rs" <<'EOF'
// Metrics scaffold (no-op placeholders)
pub struct KmsMetrics;
impl KmsMetrics {
    pub fn new() -> Self { Self }
}
EOF

mkf "$ROOT/src/config.rs" <<'EOF'
// Config scaffold (no IO yet)
pub struct KmsConfig;
impl KmsConfig {
    pub fn load() -> Self { Self }
}
EOF

mkf "$ROOT/src/prelude.rs" <<'EOF'
// Prelude scaffold — export common items
pub use crate::{types::*, error::*, traits::*};
EOF

# Traits
mkf "$ROOT/src/traits/mod.rs" <<'EOF'
// Trait module scaffold
pub mod keystore;
pub mod signer;
pub mod verifier;
pub mod kem;
pub mod hybrid;
EOF

mkf "$ROOT/src/traits/keystore.rs" <<'EOF'
// Keystore trait scaffold
use crate::{types::KeyId, error::KmResult};

pub trait KeyStore {
    fn create(&self) -> KmResult<KeyId>;
}
EOF

mkf "$ROOT/src/traits/signer.rs" <<'EOF'
// Signer trait scaffold
use crate::error::KmResult;

pub trait Signer {
    fn sign(&self, _msg: &[u8]) -> KmResult<Vec<u8>>;
}
EOF

mkf "$ROOT/src/traits/verifier.rs" <<'EOF'
// Verifier trait scaffold
use crate::error::KmResult;

pub trait Verifier {
    fn verify(&self, _msg: &[u8], _sig: &[u8]) -> KmResult<bool>;
}
EOF

mkf "$ROOT/src/traits/kem.rs" <<'EOF'
// KEM trait scaffold
use crate::error::KmResult;

pub trait Kem {
    fn encap(&self, _peer_pub: &[u8]) -> KmResult<(Vec<u8>, Vec<u8>)>;
    fn decap(&self, _ct: &[u8]) -> KmResult<Vec<u8>>;
}
EOF

mkf "$ROOT/src/traits/hybrid.rs" <<'EOF'
// Hybrid trait scaffold
use crate::error::KmResult;

pub trait Hybrid {
    fn wrap(&self, _pt: &[u8]) -> KmResult<Vec<u8>>;
    fn unwrap_(&self, _ct: &[u8]) -> KmResult<Vec<u8>>;
}
EOF

# Sealed primitives
mkf "$ROOT/src/sealed/mod.rs" <<'EOF'
// Sealed primitives scaffold
pub mod header;
pub mod aead;
pub mod anti_rollback;
pub mod store;
EOF

mkf "$ROOT/src/sealed/header.rs" <<'EOF'
// Sealed header scaffold
#[allow(dead_code)]
pub struct SealedHeader {
    pub version: u8,
}
EOF

mkf "$ROOT/src/sealed/aead.rs" <<'EOF'
// AEAD scaffold (no crypto)
EOF

mkf "$ROOT/src/sealed/anti_rollback.rs" <<'EOF'
// Anti-rollback scaffold
EOF

mkf "$ROOT/src/sealed/store.rs" <<'EOF'
// Sealed store trait scaffold
use crate::error::KmResult;

pub trait SealedStore {
    fn put(&self, _blob: &[u8]) -> KmResult<()>;
}
EOF

# Backends
mkf "$ROOT/src/backends/mod.rs" <<'EOF'
// Backends scaffold
pub mod memory;
pub mod file;
pub mod pkcs11;
EOF

mkf "$ROOT/src/backends/memory.rs" <<'EOF'
// Memory backend scaffold
EOF

mkf "$ROOT/src/backends/file.rs" <<'EOF'
// File backend scaffold
EOF

mkf "$ROOT/src/backends/pkcs11.rs" <<'EOF'
// PKCS#11 backend scaffold (feature-gated in real impl)
EOF

# Ops
mkf "$ROOT/src/ops/create.rs" <<'EOF'
// create key op scaffold
EOF
mkf "$ROOT/src/ops/rotate.rs" <<'EOF'
// rotate key op scaffold
EOF
mkf "$ROOT/src/ops/attest.rs" <<'EOF'
// attest op scaffold
EOF
mkf "$ROOT/src/ops/sign.rs" <<'EOF'
// sign op scaffold
EOF
mkf "$ROOT/src/ops/verify.rs" <<'EOF'
// verify op scaffold
EOF
mkf "$ROOT/src/ops/wrap.rs" <<'EOF'
// wrap op scaffold
EOF
mkf "$ROOT/src/ops/unwrap.rs" <<'EOF'
// unwrap op scaffold
EOF

mkf "$ROOT/src/ops/mod.rs" <<'EOF'
// ops module scaffold
pub mod create;
pub mod rotate;
pub mod attest;
pub mod sign;
pub mod verify;
pub mod wrap;
pub mod unwrap;
EOF

# PQ
mkf "$ROOT/src/pq/mod.rs" <<'EOF'
// PQ adapters scaffold
pub mod mlkem;
pub mod mldsa;
pub mod slhdsa;
EOF

mkf "$ROOT/src/pq/mlkem.rs" <<'EOF'
// ML-KEM scaffold (feature: mlkem)
EOF
mkf "$ROOT/src/pq/mldsa.rs" <<'EOF'
// ML-DSA scaffold (feature: mldsa)
EOF
mkf "$ROOT/src/pq/slhdsa.rs" <<'EOF'
// SLH-DSA scaffold (feature: slhdsa)
EOF

# Util
mkf "$ROOT/src/util/zeroize.rs" <<'EOF'
// Zeroize helpers scaffold
EOF
mkf "$ROOT/src/util/ct.rs" <<'EOF'
// Constant-time helpers scaffold
EOF
mkf "$ROOT/src/util/time.rs" <<'EOF'
// Time helpers scaffold
EOF

# ------------------------------------------------------------------------------
# docs/
# ------------------------------------------------------------------------------
mkf "$ROOT/docs/IDB.md" <<'EOF'
# IDB (scaffold)
EOF
mkf "$ROOT/docs/INTEROP.md" <<'EOF'
# INTEROP (scaffold)
EOF
mkf "$ROOT/docs/QUANTUM.md" <<'EOF'
# QUANTUM (scaffold)
EOF
mkf "$ROOT/docs/SECURITY.md" <<'EOF'
# SECURITY (scaffold)
EOF
mkf "$ROOT/docs/PERFORMANCE.md" <<'EOF'
# PERFORMANCE (scaffold)
EOF
mkf "$ROOT/docs/CONFIG.md" <<'EOF'
# CONFIG (scaffold)
EOF
mkf "$ROOT/docs/OBSERVABILITY.md" <<'EOF'
# OBSERVABILITY (scaffold)
EOF
mkf "$ROOT/docs/CONCURRENCY.md" <<'EOF'
# CONCURRENCY (scaffold)
EOF
mkf "$ROOT/docs/API.md" <<'EOF'
# API (scaffold)
EOF
mkf "$ROOT/docs/RUNBOOK.md" <<'EOF'
# RUNBOOK (scaffold)
EOF
mkf "$ROOT/docs/GOVERNANCE.md" <<'EOF'
# GOVERNANCE (scaffold)
EOF
mkf "$ROOT/docs/PQ_MIGRATION.md" <<'EOF'
# PQ Migration (scaffold)
EOF
mkf "$ROOT/docs/arch.mmd" <<'EOF'
flowchart TD
  A[Caller] -->|traits| B[ron-kms2]
  B --> C[backends]
  B --> D[sealed]
  B --> E[pq]
EOF
mkf "$ROOT/docs/arch.svg" <<'EOF'
<!-- placeholder: render from arch.mmd in CI -->
EOF

# ------------------------------------------------------------------------------
# tests/ benches/ fuzz/
# ------------------------------------------------------------------------------
mkf "$ROOT/tests/unit/sealed_header_test.rs" <<'EOF'
// scaffold test placeholder
EOF
mkf "$ROOT/tests/unit/rotate_test.rs" <<'EOF'
// scaffold test placeholder
EOF
mkf "$ROOT/tests/unit/attest_test.rs" <<'EOF'
// scaffold test placeholder
EOF
mkf "$ROOT/tests/unit/zeroize_test.rs" <<'EOF'
// scaffold test placeholder
EOF

mkf "$ROOT/tests/interop_kats.rs" <<'EOF'
// interop KATs scaffold
EOF

mkf "$ROOT/benches/sign_bench.rs" <<'EOF'
// bench scaffold
fn main() {}
EOF
mkf "$ROOT/benches/verify_bench.rs" <<'EOF'
// bench scaffold
fn main() {}
EOF
mkf "$ROOT/benches/encap_bench.rs" <<'EOF'
// bench scaffold
fn main() {}
EOF
mkf "$ROOT/benches/decap_bench.rs" <<'EOF'
// bench scaffold
fn main() {}
EOF

mkf "$ROOT/fuzz/Cargo.toml" <<'EOF'
[package]
name = "ron-kms2-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[dependencies]
libfuzzer-sys = { version = "0.4", features = ["arbitrary-derive"] }

[package.metadata]
cargo-fuzz = true

[[bin]]
name = "sealed_header"
path = "fuzz_targets/sealed_header.rs"
test = false
doc = false

[[bin]]
name = "dto_sign"
path = "fuzz_targets/dto_sign.rs"
test = false
doc = false
EOF

mkf "$ROOT/fuzz/fuzz_targets/sealed_header.rs" <<'EOF'
// fuzz target scaffold
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data.len();
});
EOF

mkf "$ROOT/fuzz/fuzz_targets/dto_sign.rs" <<'EOF'
// fuzz target scaffold
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data.iter().fold(0u8, |acc, b| acc ^ b);
});
EOF

# ------------------------------------------------------------------------------
# xtask (helper CLI stub)
# ------------------------------------------------------------------------------
mkf "$ROOT/xtask/src/main.rs" <<'EOF'
// xtask scaffold (placeholder CLI)
fn main() {
    println!("xtask: ron-kms2 scaffold");
}
EOF

# ------------------------------------------------------------------------------
# testing/ dev harness (out-of-tree)
# ------------------------------------------------------------------------------
mkf "$ROOT/testing/kms-dev-server/Cargo.toml" <<'EOF'
[package]
name = "kms-dev-server"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
# Intentionally empty; fill when building the dev server harness.
EOF

mkf "$ROOT/testing/kms-dev-server/src/main.rs" <<'EOF'
// Dev server harness scaffold (keep separate from library)
fn main() {
    println!("kms-dev-server scaffold (no HTTP yet)");
}
EOF

echo "Done."
