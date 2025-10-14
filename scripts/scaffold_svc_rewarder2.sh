#!/usr/bin/env bash
# Scaffolds crates/svc-rewarder2 with modular, maintainable stubs.
# Re-runnable: skips existing files unless --force is passed.

set -euo pipefail

CRATE="crates/svc-rewarder2"
FORCE="${1:-}"

log_dir() {
  echo "dir: $1"
  mkdir -p "$1"
}

should_write() {
  local path="$1"
  if [[ -f "$path" && "$FORCE" != "--force" ]]; then
    echo "skip: $path (exists)"
    return 1
  fi
  mkdir -p "$(dirname "$path")"
  echo "write: $path"
  return 0
}

# Write file from STDIN using a quoted heredoc to avoid shell expansion.
writef() {
  local path="$1"
  if should_write "$path"; then
    cat > "$path"
  else
    # drain heredoc input if caller still provides it
    cat >/dev/null
  fi
}

# ----------------------------
# Root structure
# ----------------------------
log_dir "$CRATE"
log_dir "$CRATE/configs"
log_dir "$CRATE/docs/openapi"
log_dir "$CRATE/docs/schemas"
log_dir "$CRATE/docs/mermaid"
log_dir "$CRATE/src/http"
log_dir "$CRATE/src/config"
log_dir "$CRATE/src/core"
log_dir "$CRATE/src/inputs"
log_dir "$CRATE/src/outputs"
log_dir "$CRATE/src/metrics"
log_dir "$CRATE/src/readiness"
log_dir "$CRATE/src/bus"
log_dir "$CRATE/src/security"
log_dir "$CRATE/src/util"
log_dir "$CRATE/tests/unit"
log_dir "$CRATE/tests/integration"
log_dir "$CRATE/tests/vectors/v1"
log_dir "$CRATE/benches"
log_dir "$CRATE/xtask"

# ----------------------------
# Cargo + top-level docs
# ----------------------------
writef "$CRATE/Cargo.toml" <<'EOF'
[package]
name = "svc-rewarder2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[features]
default = ["tokio", "serde"]
tls = []
pq = []
pq-hybrid = []
pq-sign = []

[dependencies]
anyhow = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
axum = { version = "0.7", features = ["tokio","http1","json"] }
tower-http = { version = "0.6", features = ["trace","cors","compression-gzip"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
prometheus = "0.14"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "json"] }
blake3 = "1"
bytes = "1"
humantime = "2"

[dev-dependencies]
criterion = "0.5"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

[[bench]]
name = "reward_calc"
harness = false
EOF

writef "$CRATE/README.md" <<'EOF'
# svc-rewarder2

> **Role:** service  
> **Status:** draft  
> **MSRV:** 1.80.0

This crate mirrors the svc-rewarder design: deterministic reward compute from sealed inputs, idempotent ledger intents, and strict invariants. This README is a placeholder—copy the finalized svc-rewarder README here if you want both crates identical in docs.
EOF

writef "$CRATE/CHANGELOG.md" <<'EOF'
# Changelog
All notable changes to this project will be documented in this file.
EOF

writef "$CRATE/LICENSE-MIT" <<'EOF'
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction...
EOF

writef "$CRATE/LICENSE-APACHE" <<'EOF'
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/
EOF

# ----------------------------
# Example config
# ----------------------------
writef "$CRATE/configs/svc-rewarder.toml" <<'EOF'
# Example configuration (stub)
http_addr = "0.0.0.0:8080"
metrics_addr = "0.0.0.0:9909"
max_inflight = 512
work_queue_max = 4096
tenant_rps_max = 100
ledger_deadline_ms = 1000
ledger_max_retries = 2
req_body_max_bytes = 1048576
decomp_ratio_max = 10
require_pq_on = false
amnesia_mode = true
EOF

# ----------------------------
# Docs placeholders (12 docs)
# ----------------------------
for d in API CONCURRENCY CONFIG GOVERNANCE IDB INTEROP OBSERVABILITY PERFORMANCE QUANTUM RUNBOOK SECURITY TESTS; do
  writef "$CRATE/docs/$d.md" <<'EOF'
# (placeholder) See consolidated ALL_DOCS in the main project; this file exists for local edits.
EOF
done

writef "$CRATE/docs/openapi/svc-rewarder.json" <<'EOF'
{ "openapi": "3.0.0", "info": {"title": "svc-rewarder2", "version": "0.0.1"}, "paths": {} }
EOF

# NOTE: use single-quoted heredocs so "$schema" is not expanded by bash
writef "$CRATE/docs/schemas/compute.request.v1.json" <<'EOF'
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ComputeRequest",
  "type": "object",
  "properties": {
    "inputs_cid": { "type": "string" },
    "policy_id":  { "type": "string" },
    "policy_hash":{ "type": "string" }
  },
  "required": ["inputs_cid","policy_id","policy_hash"]
}
EOF

writef "$CRATE/docs/schemas/manifest.v1.json" <<'EOF'
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Manifest",
  "type": "object",
  "properties": {
    "version":    { "type": "integer" },
    "epoch_id":   { "type": "string" },
    "run_key":    { "type": "string" },
    "inputs_cid": { "type": "string" },
    "policy_id":  { "type": "string" },
    "policy_hash":{ "type": "string" }
  },
  "required": ["version","epoch_id","run_key","inputs_cid","policy_id","policy_hash"]
}
EOF

writef "$CRATE/docs/mermaid/arch.mmd" <<'EOF'
flowchart LR
  A[Caller] -->|POST /rewarder| B(svc-rewarder2)
  B --> C[ron-ledger]
  B --> D[[Prometheus]]
  B --> E[(tmpfs: amnesia)]
EOF

writef "$CRATE/docs/mermaid/sequence.mmd" <<'EOF'
sequenceDiagram
  actor Caller
  participant S as svc-rewarder2
  Caller->>S: POST /rewarder/epochs/:id/compute
  S-->>Caller: 200 accepted | 409 quarantined
EOF

writef "$CRATE/docs/mermaid/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: admission
  Running --> Quarantine: invariant breach
  Running --> Emitting: ledger intent
  Emitting --> Running: dup|accepted
  Running --> Shutdown: signal
  Shutdown --> [*]
EOF

# ----------------------------
# Source stubs (no logic)
# ----------------------------
writef "$CRATE/src/main.rs" <<'EOF'
// svc-rewarder2 bootstrap (stub). Keep thin; all logic lives in library modules.
fn main() {
    // TODO: parse config, init telemetry, start server (implementation later).
    println!("svc-rewarder2 stub");
}
EOF

writef "$CRATE/src/lib.rs" <<'EOF'
// Public surface (stub). Re-export types as they are added.
pub mod prelude;
pub mod config;
pub mod http;
pub mod core;
pub mod inputs;
pub mod outputs;
pub mod metrics;
pub mod readiness;
pub mod bus;
pub mod security;
pub mod util;

// Re-exports (will be real types later)
pub struct Metrics;
pub struct HealthState;
pub struct Bus;
pub enum KernelEvent { Health { service: String, ok: bool } }

pub fn wait_for_ctrl_c() {}
EOF

writef "$CRATE/src/prelude.rs" <<'EOF'
// Common imports (stub)
pub use anyhow::Result;
EOF

# --- config ---
writef "$CRATE/src/config/mod.rs" <<'EOF'
// Config module (stub)
pub mod types;
pub mod load;
pub mod validate;
EOF

writef "$CRATE/src/config/types.rs" <<'EOF'
// Strongly-typed config (stub)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub http_addr: String,
    pub metrics_addr: String,
    pub max_inflight: u32,
}
EOF

writef "$CRATE/src/config/load.rs" <<'EOF'
// Config loading (stub)
use super::types::Config;

pub fn load() -> Config {
    Config {
        http_addr: "0.0.0.0:8080".into(),
        metrics_addr: "0.0.0.0:9909".into(),
        max_inflight: 512,
    }
}
EOF

writef "$CRATE/src/config/validate.rs" <<'EOF'
// Config validation (stub)
use super::types::Config;

pub fn validate(_cfg: &Config) -> bool { true }
EOF

# --- http ---
writef "$CRATE/src/http/mod.rs" <<'EOF'
// HTTP server wiring (stub)
pub mod routes;
pub mod handlers;
pub mod dto;
pub mod error;
EOF

writef "$CRATE/src/http/routes.rs" <<'EOF'
// Route table (stub)
pub fn router() {}
EOF

writef "$CRATE/src/http/handlers.rs" <<'EOF'
// Handlers (stub)
pub fn compute() {}
EOF

writef "$CRATE/src/http/dto.rs" <<'EOF'
// DTOs (stub)
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ComputeRequest {
    pub inputs_cid: String,
    pub policy_id: String,
    pub policy_hash: String,
}
EOF

writef "$CRATE/src/http/error.rs" <<'EOF'
// Uniform error envelope (stub)
EOF

# --- core ---
writef "$CRATE/src/core/mod.rs" <<'EOF'
// Core calculus & invariants (stub)
pub mod compute;
pub mod invariants;
pub mod algebra;
EOF

writef "$CRATE/src/core/compute.rs" <<'EOF'
// Deterministic compute (stub)
EOF

writef "$CRATE/src/core/invariants.rs" <<'EOF'
// Invariant checks (stub)
EOF

writef "$CRATE/src/core/algebra.rs" <<'EOF'
// Money-safe arithmetic helpers (stub)
EOF

# --- inputs ---
writef "$CRATE/src/inputs/mod.rs" <<'EOF'
// Inputs facade (stub)
pub mod accounting;
pub mod policy;
pub mod ledger_snapshot;
pub mod cid;
pub mod cache;
EOF

writef "$CRATE/src/inputs/accounting.rs" <<'EOF'
// Accounting snapshot ingestion (stub)
EOF

writef "$CRATE/src/inputs/policy.rs" <<'EOF'
// Policy retrieval/validation (stub)
EOF

writef "$CRATE/src/inputs/ledger_snapshot.rs" <<'EOF'
// Optional ledger reads (stub)
EOF

writef "$CRATE/src/inputs/cid.rs" <<'EOF'
// Content address helpers (stub)
EOF

writef "$CRATE/src/inputs/cache.rs" <<'EOF'
// Bounded in-memory cache (stub)
EOF

# --- outputs ---
writef "$CRATE/src/outputs/mod.rs" <<'EOF'
// Outputs facade (stub)
pub mod manifest;
pub mod artifacts;
pub mod intents;
pub mod attestation;
EOF

writef "$CRATE/src/outputs/manifest.rs" <<'EOF'
// Canonical manifest (stub)
EOF

writef "$CRATE/src/outputs/artifacts.rs" <<'EOF'
// Artifact writing (stub)
EOF

writef "$CRATE/src/outputs/intents.rs" <<'EOF'
// Idempotent ledger intents (stub)
EOF

writef "$CRATE/src/outputs/attestation.rs" <<'EOF'
// Attestations (Ed25519/PQ) (stub)
EOF

# --- metrics/readiness/bus/security/util ---
writef "$CRATE/src/metrics/mod.rs" <<'EOF'
// Prometheus metrics registry (stub)
EOF

writef "$CRATE/src/readiness/mod.rs" <<'EOF'
// Readiness gate (stub)
pub mod health;
EOF

writef "$CRATE/src/readiness/health.rs" <<'EOF'
// /healthz and /readyz (stub)
EOF

writef "$CRATE/src/bus/mod.rs" <<'EOF'
// Bus wiring (stub)
pub mod events;
EOF

writef "$CRATE/src/bus/events.rs" <<'EOF'
// Event shapes (stub)
EOF

writef "$CRATE/src/security/mod.rs" <<'EOF'
// Security (caps/TLS/PQ) (stub)
pub mod caps;
pub mod tls;
pub mod pq;
EOF

writef "$CRATE/src/security/caps.rs" <<'EOF'
// Capability verification (stub)
EOF

writef "$CRATE/src/security/tls.rs" <<'EOF'
// rustls config (stub)
EOF

writef "$CRATE/src/security/pq.rs" <<'EOF'
// PQ toggles/policy (stub)
EOF

writef "$CRATE/src/util/bytes.rs" <<'EOF'
// Byte helpers (stub)
EOF

writef "$CRATE/src/util/timeouts.rs" <<'EOF'
// Deadline helpers (stub)
EOF

# ----------------------------
# Tests & benches
# ----------------------------
writef "$CRATE/tests/unit/invariants.rs" <<'EOF'
// Property tests for invariants (stub)
#[test] fn invariants_hold() { assert!(true); }
EOF

writef "$CRATE/tests/unit/idempotency.rs" <<'EOF'
// Idempotency tests (stub)
#[test] fn idempotent_keys() { assert!(true); }
EOF

writef "$CRATE/tests/unit/config.rs" <<'EOF'
// Config validation tests (stub)
#[test] fn config_ok() { assert!(true); }
EOF

writef "$CRATE/tests/integration/http_compute.rs" <<'EOF'
// HTTP compute E2E (stub)
#[test] fn compute_works() { assert!(true); }
EOF

writef "$CRATE/tests/integration/readiness.rs" <<'EOF'
// Readiness behavior (stub)
#[test] fn readiness_drops_under_load() { assert!(true); }
EOF

writef "$CRATE/tests/integration/egress_dedupe.rs" <<'EOF'
// Egress dedupe behavior (stub)
#[test] fn dedupe_ok() { assert!(true); }
EOF

writef "$CRATE/tests/vectors/v1/happy-001.json" <<'EOF'
{ "note": "happy path stub" }
EOF

writef "$CRATE/tests/vectors/v1/dup-epoch.json" <<'EOF'
{ "note": "duplicate epoch stub" }
EOF

writef "$CRATE/tests/vectors/v1/quarantine-overflow.json" <<'EOF'
{ "note": "overflow quarantine stub" }
EOF

writef "$CRATE/benches/reward_calc.rs" <<'EOF'
use criterion::{criterion_group, criterion_main, Criterion};
fn bench_reward(c: &mut Criterion) { c.bench_function("reward_calc_stub", |b| b.iter(|| 1+1)); }
criterion_group!(benches, bench_reward);
criterion_main!(benches);
EOF

# ----------------------------
# xtask helper
# ----------------------------
writef "$CRATE/xtask/mod.rs" <<'EOF'
// xtask automation hooks (stub)
pub fn main_task() { println!("xtask stub"); }
EOF

echo "DONE: scaffolded $CRATE (use --force to overwrite existing files)"
