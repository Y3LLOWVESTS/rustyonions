
#!/usr/bin/env bash
set -euo pipefail

ROOT="crates/ron-ledger2"

say() { printf "%s\n" "$*"; }
mk()  { mkdir -p "$1"; say "dir: $1"; }
wr()  {
  local path="$1"; shift
  mk "$(dirname "$path")"
  say "write: $path"
  cat > "$path" <<'EOF'
'"$@"'
EOF
}

# --- dirs ---
mk "$ROOT"
mk "$ROOT/src/engine"
mk "$ROOT/tests"
mk "$ROOT/benches"
mk "$ROOT/fuzz/fuzz_targets"
mk "$ROOT/examples"
mk "$ROOT/docs/diagrams"
mk "$ROOT/testing/vectors"
mk "$ROOT/testing/performance/baselines"
mk "$ROOT/testing/performance/payloads"
mk "$ROOT/.github/workflows"
mk "scripts"

# --- Cargo.toml ---
wr "$ROOT/Cargo.toml" '
[package]
name = "ron-ledger2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "RustyOnions ledger engine (library-only) — immutable, append-only, deterministic roots."
repository = "https://github.com/RustyOnions/RustyOnions"
readme = "README.md"
categories = ["data-structures","algorithms","finance"]
keywords = ["ledger","append-only","deterministic","rustyonions"]

[lib]
path = "src/lib.rs"
crate-type = ["rlib"]

[features]
default = ["serde"]
pq-hybrid = []
arbitrary = []

[dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "1"
bytes = "1"

[dev-dependencies]
anyhow = "1"
serde_json = "1"
'

# --- README.md (library-focused; trimmed scaffold version) ---
wr "$ROOT/README.md" '# ron-ledger2

> **Role:** library (immutable, append-only economic engine)  
> **Owner:** Stevan White  
> **Status:** beta  
> **MSRV:** 1.80.0  
> **Last reviewed:** 2025-10-13

[![Build](https://github.com/RustyOnions/RustyOnions/actions/workflows/ci.yml/badge.svg)](https://github.com/RustyOnions/RustyOnions/actions/workflows/ci.yml)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](./)

`ron-ledger2` is the core **library** implementation scaffold for the RustyOnions ledger engine:
append-only, single-writer, deterministic roots. This crate is **transport-free**: embed it in a
service (e.g., `svc-ledger`) to expose HTTP/metrics.

See `docs/` for invariants (IDB), concurrency rules, and test/bench plans.
'

# --- CHANGELOG & LICENSE placeholders ---
wr "$ROOT/CHANGELOG.md" '## 0.1.0
- Initial scaffold for ron-ledger2 (library-only).'
wr "$ROOT/LICENSE-MIT" 'Permission is hereby granted, free of charge, to any person obtaining a copy...'
wr "$ROOT/LICENSE-APACHE" 'Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/'

# --- src/lib.rs ---
wr "$ROOT/src/lib.rs" '//! ron-ledger2 — library-only ledger engine scaffold.

pub mod api;
pub mod types;
pub mod config;
pub mod error;
pub mod engine;

// Re-exports to keep consumer paths short.
pub use crate::engine::ledger::Ledger;
pub use crate::engine::storage::Storage;
pub use crate::config::LedgerConfig;
pub use crate::api::{IngestRequest, IngestResponse};
pub use crate::error::{LedgerError, RejectReason};
'

# --- src/api.rs ---
wr "$ROOT/src/api.rs" '//! DTOs for ingestion/results (transport-agnostic).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    #[serde(default)]
    pub entries: Vec<Vec<u8>>, // placeholder payload; real schema lives in docs/API.md
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub committed: u64,
    pub root: [u8; 32],
    #[serde(default)]
    pub rejects: Vec<Reject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reject {
    pub index: u64,
    pub reason: super::error::RejectReason,
}
'

# --- src/types.rs ---
wr "$ROOT/src/types.rs" '//! Core primitive types (versioned).

use serde::{Deserialize, Serialize};

pub type Seq = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Root(pub [u8; 32]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kid(pub String); // key identifier only; no secrets here

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryHeader {
    pub seq: Seq,
    pub kind: u16,
    pub ver: u16,
    pub kid: Kid,
}
'

# --- src/config.rs ---
wr "$ROOT/src/config.rs" '//! Library configuration (embedder maps env/CLI → this struct).

#[derive(Debug, Clone)]
pub enum EngineMode {
    Persistent,
    Amnesia,
}

#[derive(Debug, Clone)]
pub enum AccumulatorKind {
    Merkle,
    Verkle, // experimental
}

#[derive(Debug, Clone)]
pub enum PqMode {
    Off,
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct LedgerConfig {
    pub engine_mode: EngineMode,
    pub accumulator: AccumulatorKind,
    pub batch_max_entries: usize,
    pub queue_capacity: usize,
    pub limits_max_entry_bytes: usize,
    pub pq_mode: PqMode,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            engine_mode: EngineMode::Persistent,
            accumulator: AccumulatorKind::Merkle,
            batch_max_entries: 1024,
            queue_capacity: 65_536,
            limits_max_entry_bytes: 1 << 20, // 1 MiB
            pq_mode: PqMode::Off,
        }
    }
}
'

# --- src/error.rs ---
wr "$ROOT/src/error.rs" '//! Error taxonomy (machine-countable).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("invalid entry: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", content = "detail")]
pub enum RejectReason {
    Invalid,
    TooLarge,
    Timeout,
    Conflict,
}
'

# --- engine modules ---
wr "$ROOT/src/engine/mod.rs" '//! Engine: orchestrates ingest → validate → append → root → checkpoint.
pub mod storage;
pub mod ledger;
pub mod accumulator;
pub mod checkpoint;
pub mod replay;
pub mod observer;
'
wr "$ROOT/src/engine/storage.rs" '//! Storage trait — implement in your own crate (e.g., ron-ledger-sled).

pub trait Storage {
    type Error: std::error::Error + Send + Sync + "static;

    fn append_entry(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
    fn persist_checkpoint(&mut self, seq: u64, root: [u8; 32], ts: u64) -> Result<(), Self::Error>;
    fn stream_from(&mut self, seq: u64) -> Result<Box<dyn Iterator<Item = Result<Vec<u8>, Self::Error>> + "_>, Self::Error>;
    fn latest_checkpoint(&mut self) -> Result<Option<(u64, [u8; 32], u64)>, Self::Error>;
}
'
wr "$ROOT/src/engine/ledger.rs" '//! Single-writer orchestrator (scaffold).

use crate::{api::{IngestRequest, IngestResponse}, config::LedgerConfig, engine::storage::Storage};

pub struct Ledger<S: Storage> {
    storage: S,
    _cfg: LedgerConfig,
}

impl<S: Storage> Ledger<S> {
    pub fn new(storage: S, cfg: LedgerConfig) -> Result<Self, crate::error::LedgerError> {
        Ok(Self { storage, _cfg: cfg })
    }

    pub fn ingest(&mut self, _req: IngestRequest) -> Result<IngestResponse, crate::error::LedgerError> {
        // Placeholder only. Real logic lives in future commits.
        Ok(IngestResponse { committed: 0, root: [0u8; 32], rejects: vec![] })
    }
}
'
wr "$ROOT/src/engine/accumulator.rs" '//! Deterministic state root accumulator (scaffold).
pub fn compute_root(_bytes: &[u8]) -> [u8; 32] { [0u8; 32] }
'
wr "$ROOT/src/engine/checkpoint.rs" '//! Checkpoint helpers (scaffold).
#[derive(Debug, Clone, Copy)]
pub struct CheckpointMeta {
    pub seq: u64,
    pub root: [u8; 32],
    pub ts: u64,
}
'
wr "$ROOT/src/engine/replay.rs" '//! Replay helpers (scaffold).
pub fn replay_noop() {}
'
wr "$ROOT/src/engine/observer.rs" '//! Observer hooks (service wires metrics/logging).
#[derive(Debug, Clone, Copy)]
pub struct CommitEvent {
    pub committed: u64,
    pub new_root: [u8; 32],
}
'

# --- tests (compile-fast stubs) ---
wr "$ROOT/tests/idempotency_prop.rs" '#[test]
fn idempotency_prop_scaffold() {
    assert!(true);
}'
wr "$ROOT/tests/replay_recovery.rs" '#[test]
fn replay_recovery_scaffold() {
    assert!(true);
}'
wr "$ROOT/tests/reject_taxonomy.rs" 'use ron_ledger2::error::RejectReason;
#[test]
fn reject_taxonomy_scaffold() {
    let r = RejectReason::Invalid;
    match r { RejectReason::Invalid => (), _ => unreachable!() }
}'
wr "$ROOT/tests/interop_vectors.rs" '#[test]
fn interop_vectors_scaffold() {
    // vectors live under testing/vectors/*.json
    assert!(true);
}'
wr "$ROOT/tests/public_api_semver.rs" '#[test]
fn public_api_semver_scaffold() {
    // gate reserved for cargo-public-api / semver-checks in CI
    assert!(true);
}'

# --- benches (simple main; no criterion needed) ---
wr "$ROOT/benches/micro.rs" 'fn main() {
    // Placeholder bench. Use Criterion in future iterations.
    println!("ron-ledger2 micro bench scaffold");
}'

# --- fuzz target placeholder ---
wr "$ROOT/fuzz/fuzz_targets/replay_parse.rs" '// libFuzzer entry goes here (scaffold)'

# --- example ---
wr "$ROOT/examples/minimal.rs" 'use ron_ledger2::{Ledger, LedgerConfig};
fn main() -> anyhow::Result<()> {
    struct Noop; impl ron_ledger2::Storage for Noop {
        type Error = std::io::Error;
        fn append_entry(&mut self, _b:&[u8])->Result<(),Self::Error>{Ok(())}
        fn persist_checkpoint(&mut self,_:u64,_:[u8;32],_:u64)->Result<(),Self::Error>{Ok(())}
        fn stream_from(&mut self,_:u64)->Result<Box<dyn Iterator<Item=Result<Vec<u8>,Self::Error>>>,Self::Error>{Ok(Box::new(std::iter::empty()))}
        fn latest_checkpoint(&mut self)->Result<Option<(u64,[u8;32],u64)>,Self::Error>{Ok(None)}
    }
    let storage = Noop;
    let mut ledger = Ledger::new(storage, LedgerConfig::default())?;
    let _ = ledger.ingest(ron_ledger2::IngestRequest{ entries: vec![] })?;
    Ok(())
}'

# --- docs placeholders ---
wr "$ROOT/docs/IDB.md" '# IDB — Invariant-Driven Blueprint (scaffold)'
wr "$ROOT/docs/CONCURRENCY.md" '# CONCURRENCY — single-writer, bounded queues (scaffold)'
wr "$ROOT/docs/CONFIG.md" '# CONFIG — LedgerConfig schema (scaffold)'
wr "$ROOT/docs/SECURITY.md" '# SECURITY — KID-only, no secrets, bounded inputs (scaffold)'
wr "$ROOT/docs/OBSERVABILITY.md" '# OBSERVABILITY — observer hooks → service metrics (scaffold)'
wr "$ROOT/docs/PERFORMANCE.md" '# PERFORMANCE — baselines & harness (scaffold)'
wr "$ROOT/docs/QUANTUM.md" '# QUANTUM — PQ-hybrid seam (scaffold)'
wr "$ROOT/docs/TESTS.md" '# TESTS — property/fuzz/soak gates (scaffold)'
wr "$ROOT/docs/RUNBOOK.md" '# RUNBOOK — applies to service wrapper (scaffold)'
wr "$ROOT/docs/INTEROP.md" '# INTEROP — DTO mapping notes (scaffold)'
wr "$ROOT/docs/API.md" '# API — DTOs (transport-agnostic) (scaffold)'
wr "$ROOT/docs/diagrams/arch.mmd" 'flowchart LR; A[svc-ledger]-->B(ron-ledger2); B-->C[(Storage)];'
wr "$ROOT/docs/diagrams/ingest-seq.mmd" 'sequenceDiagram; participant S as Service; participant L as ron-ledger2; S->>L: ingest(); L-->>S: result;'
wr "$ROOT/docs/diagrams/engine-states.mmd" 'stateDiagram-v2; [*]-->Idle; Idle-->Running; Running-->Shutdown; Shutdown-->[*];'

# --- testing vectors & perf payloads (placeholders) ---
wr "$ROOT/testing/vectors/happy.json" '{ "case": "happy" }'
wr "$ROOT/testing/vectors/conflict.json" '{ "case": "conflict" }'
wr "$ROOT/testing/vectors/malformed.json" '{ "case": "malformed" }'
wr "$ROOT/testing/vectors/reversible.json" '{ "case": "reversible" }'
wr "$ROOT/testing/performance/baselines/micronode.json" '{ "profile": "micronode" }'
wr "$ROOT/testing/performance/baselines/macronode.json" '{ "profile": "macronode" }'
wr "$ROOT/testing/performance/payloads/ingest_batch_1k.json" '{ "entries": 1000 }'

# --- CI workflows (scaffolds) ---
wr "$ROOT/.github/workflows/ci.yml" 'name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy -p ron-ledger2 -- -D warnings
      - run: cargo build -p ron-ledger2
      - run: cargo test -p ron-ledger2
'
wr "$ROOT/.github/workflows/semver.yml" 'name: semver
on: [pull_request]
jobs:
  semver:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: echo "placeholder for public-api/semver-checks"
'
wr "$ROOT/.github/workflows/render-mermaid.yml" 'name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files "docs/diagrams/*.mmd"); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
'
wr "$ROOT/.github/workflows/fuzz.yml" 'name: fuzz
on: [workflow_dispatch]
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: echo "placeholder fuzz smoke-run"
'

# --- dev quickcheck script (helper) ---
wr "scripts/dev-quickcheck.sh" '#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all
cargo clippy -p ron-ledger2 -- -D warnings
cargo test -p ron-ledger2
cargo deny check || true
say "ok"
'
chmod +x "scripts/dev-quickcheck.sh"

say "Scaffold complete for $ROOT"
