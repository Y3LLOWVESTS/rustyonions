#!/usr/bin/env bash
set -euo pipefail

# Where to create the crate
CRATE_PATH="crates/ron-kernel2"

# Guardrails
if [[ -e "$CRATE_PATH/src/lib.rs" ]]; then
  echo "Refusing to overwrite existing crate at $CRATE_PATH (src/lib.rs exists)."
  echo "Remove it or change CRATE_PATH in this script."
  exit 1
fi

echo "Scaffolding ron-kernel2 into: $CRATE_PATH"

# Directories
mkdir -p "$CRATE_PATH"/{.cargo,.github/workflows,scripts,docs/specs,specs,fuzz,testing/performance,examples,benches,tests,src/{bus,metrics,config,supervisor,internal}}

# Top-level files
cat > "$CRATE_PATH/Cargo.toml" <<'EOF'
[package]
name = "ron-kernel2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "RustyOnions microkernel (scaffolded) — frozen, tiny public API."
repository = "https://example.invalid/RustyOnions"
keywords = ["microkernel","supervision","metrics"]
categories = ["asynchronous","concurrency"]

[lib]
path = "src/lib.rs"

[features]
default = ["tokio", "serde"]
kameo = []

[dependencies]
# Keep deps minimal; do not leak surface.
tokio = { version = "1", features = ["rt-multi-thread","macros","signal"], optional = false }
serde = { version = "1", features = ["derive"], optional = false }
# Prometheus/HTTP live in consumer services, not in the kernel surface.

[dev-dependencies]
anyhow = "1"
criterion = { version = "0.5", default-features = false, features = ["html_reports"] }
proptest = "1"

[package.metadata.docs.rs]
all-features = true
EOF

cat > "$CRATE_PATH/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["rustfmt","clippy"]
EOF

cat > "$CRATE_PATH/deny.toml" <<'EOF'
[advisories]
yanked = "deny"
unmaintained = "deny"
vulnerability = "deny"
[licenses]
allow = ["MIT", "Apache-2.0"]
EOF

cat > "$CRATE_PATH/LICENSE-MIT" <<'EOF'
MIT License (placeholder)
EOF

cat > "$CRATE_PATH/LICENSE-APACHE" <<'EOF'
Apache License 2.0 (placeholder)
EOF

cat > "$CRATE_PATH/CHANGELOG.md" <<'EOF'
# Changelog — ron-kernel2

## 0.1.0
- Scaffolding created; public API stubs; no behavior yet.
EOF

# README / API docs (short placeholders; you already have the full docs elsewhere)
cat > "$CRATE_PATH/README.md" <<'EOF'
# ron-kernel2

Scaffold for the RustyOnions microkernel crate. See main project docs for the full README and diagrams.
EOF

cat > "$CRATE_PATH/API.md" <<'EOF'
# API.md — ron-kernel2

Frozen, tiny public API (stubs). Use `cargo public-api` to enforce surface.
EOF

# .cargo and CI
cat > "$CRATE_PATH/.cargo/config.toml" <<'EOF'
[build]
rustflags = []

[term]
verbose = false
EOF

cat > "$CRATE_PATH/.github/workflows/kernel-ci.yml" <<'EOF'
name: kernel-ci
on: [push, pull_request]
jobs:
  public-api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-public-api || true
      - run: cargo public-api -p ron-kernel2 || true
  mermaid:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files 'crates/ron-kernel2/docs/*.mmd' 2>/dev/null); do
            mmdc -i "$f" -o "${f%.mmd}.svg"
          done
EOF

cat > "$CRATE_PATH/.github/workflows/rust.yml" <<'EOF'
name: rust
on: [push, pull_request]
jobs:
  test:
    strategy:
      matrix:
        amnesia: [off, on]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build
        run: cargo build -p ron-kernel2
      - name: Test
        run: AMNESIA=${{ matrix.amnesia }} cargo test -p ron-kernel2 --all-features
  loom:
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run loom tests (ignored)
        run: RUSTFLAGS="--cfg loom" cargo test -p ron-kernel2 -- --ignored
EOF

# Scripts
cat > "$CRATE_PATH/scripts/ci_public_api.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cargo install cargo-public-api >/dev/null 2>&1 || true
cargo public-api -p ron-kernel2 --simplified
EOF
chmod +x "$CRATE_PATH/scripts/ci_public_api.sh"

cat > "$CRATE_PATH/scripts/render_mermaid.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
which mmdc >/dev/null || npm i -g @mermaid-js/mermaid-cli
for f in $(git ls-files 'crates/ron-kernel2/docs/*.mmd' 2>/dev/null); do
  mmdc -i "$f" -o "${f%.mmd}.svg"
done
EOF
chmod +x "$CRATE_PATH/scripts/render_mermaid.sh"

# Docs & diagrams
cat > "$CRATE_PATH/docs/arch.mmd" <<'EOF'
flowchart LR
  A[svc-*] -->|Bus| B(ron-kernel2)
  B --> C[ron-bus]
  B --> D[ryker]
  B --> E[[ron-metrics]]
  style B fill:#0b7285,stroke:#083344,color:#fff
EOF

cat > "$CRATE_PATH/docs/seq.mmd" <<'EOF'
sequenceDiagram
  actor Caller
  participant Bus as RON Bus
  participant K as ron-kernel2
  Caller->>Bus: Publish ConfigUpdate
  Bus->>K: Deliver event
  K-->>Bus: KernelEvent::ConfigUpdated
  Bus-->>Caller: Ack
EOF

cat > "$CRATE_PATH/docs/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: start()
  Running --> Backoff: child_panic
  Backoff --> Running: restart_after_jitter
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
EOF

for f in CONCURRENCY.md PERFORMANCE.md SECURITY.md QUANTUM.md OBSERVABILITY.md IDB.md INTEROP.md; do
  cat > "$CRATE_PATH/docs/$f" <<EOF
# $f (ron-kernel2)
Stub per blueprint. Keep this doc tiny and point to canonical project blueprints.
EOF
done

cat > "$CRATE_PATH/docs/specs/OAP-1.md" <<'EOF'
# OAP-1 (pointer)
This is a pointer to the external normative OAP/1 spec. Kernel does not enforce OAP; services do.
EOF

# Formal specs
cat > "$CRATE_PATH/specs/supervisor.tla" <<'EOF'
---- MODULE Supervisor ----
(* TLA+ sketch placeholder for crash-only restart/backoff invariants *)
==== 
EOF

cat > "$CRATE_PATH/specs/bus.tla" <<'EOF'
---- MODULE Bus ----
(* TLA+ sketch placeholder for bounded broadcast invariants *)
==== 
EOF

# Fuzz skeleton
cat > "$CRATE_PATH/fuzz/cfg_parser.rs" <<'EOF'
// fuzz target placeholder for config validation/precedence
fn main() {}
EOF

# Perf harness files
cat > "$CRATE_PATH/testing/performance/publish_matrix.toml" <<'EOF'
[runs.default]
publish_rps = [100, 500, 1000]
fanout = [1, 4, 8]
duration_secs = 30
EOF

cat > "$CRATE_PATH/testing/performance/README.md" <<'EOF'
# Performance harness (ron-kernel2)
How to run and interpret publish/lag experiments. Populate with real commands when benches land.
EOF

# Examples & benches
cat > "$CRATE_PATH/examples/minimal_supervision.rs" <<'EOF'
fn main() {
    println!("ron-kernel2 example placeholder");
}
EOF

cat > "$CRATE_PATH/benches/bus_lag_vs_publish.rs" <<'EOF'
use criterion::{criterion_group, criterion_main, Criterion};
fn bench_stub(_c: &mut Criterion) {}
criterion_group!(benches, bench_stub);
criterion_main!(benches);
EOF

# Tests (placeholders; no heavy logic yet)
cat > "$CRATE_PATH/tests/public_api.rs" <<'EOF'
#[test]
fn public_api_reexports_exist() {
    // Stubs ensure these items exist; behavior comes later.
    use ron_kernel2::{Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c};
    let _ = (std::any::TypeId::of::<Bus>(),
             std::any::TypeId::of::<KernelEvent>(),
             std::any::TypeId::of::<Metrics>(),
             std::any::TypeId::of::<HealthState>(),
             std::any::TypeId::of::<Config>());
    let _ = wait_for_ctrl_c; // name check
}
EOF

for tf in bus_bounded.rs readiness_degrades.rs amnesia_label.rs supervisor_backoff.rs tls_type_invariance.rs loom_bus.rs property_config.rs; do
  cat > "$CRATE_PATH/tests/$tf" <<'EOF'
// placeholder test file — to be implemented
#[test] fn todo() { assert!(true); }
EOF
done

# Source stubs (tiny, compile-safe, no behavior)
cat > "$CRATE_PATH/src/lib.rs" <<'EOF'
#![forbid(unsafe_code)]
//! ron-kernel2: microkernel scaffold. Public API is intentionally tiny and frozen.

pub use bus::Bus;
pub use events::KernelEvent;
pub use metrics::{Metrics, HealthState};
pub use config::Config;
pub use shutdown::wait_for_ctrl_c;

pub mod bus;
mod events;
pub mod metrics;
pub mod config;
pub mod supervisor;
pub mod amnesia;
pub mod shutdown;
mod internal;
EOF

# Bus module
cat > "$CRATE_PATH/src/bus/mod.rs" <<'EOF'
//! In-process broadcast bus (scaffold). Bounded + single-receiver-per-task (to be implemented).

#[derive(Debug, Default)]
pub struct Bus;
EOF

cat > "$CRATE_PATH/src/bus/bounded.rs" <<'EOF'
//! Overflow policy, counters (placeholder).
EOF

cat > "$CRATE_PATH/src/bus/topic.rs" <<'EOF'
//! Topic/predicate helpers (placeholder).
EOF

# Events
cat > "$CRATE_PATH/src/events.rs" <<'EOF'
//! KernelEvent (shape frozen; fields may not change without a major bump).
#[derive(Debug)]
pub enum KernelEvent {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}
EOF

# Metrics
cat > "$CRATE_PATH/src/metrics/mod.rs" <<'EOF'
//! Golden metrics & health/readiness scaffolding.

#[derive(Debug, Default)]
pub struct Metrics;

#[derive(Debug, Default)]
pub struct HealthState;

impl Metrics {
    pub fn new() -> Self { Self::default() }
    pub fn health(&self) -> &HealthState { &HealthState::default() }
}

EOF

cat > "$CRATE_PATH/src/metrics/exporter.rs" <<'EOF'
//! Prometheus exporter task (placeholder).
EOF

cat > "$CRATE_PATH/src/metrics/health.rs" <<'EOF'
//! Liveness (/healthz) plumbing (placeholder).
EOF

cat > "$CRATE_PATH/src/metrics/readiness.rs" <<'EOF'
//! Readiness (/readyz) plumbing (placeholder).
EOF

# Config
cat > "$CRATE_PATH/src/config/mod.rs" <<'EOF'
//! Validated, hot-reloadable config (placeholder).
#[derive(Debug, Default)]
pub struct Config;
EOF

cat > "$CRATE_PATH/src/config/watcher.rs" <<'EOF'
//! Hot-reload watcher emitting KernelEvent::ConfigUpdated (placeholder).
EOF

cat > "$CRATE_PATH/src/config/validation.rs" <<'EOF'
//! Config validation rules (placeholder).
EOF

# Supervisor
cat > "$CRATE_PATH/src/supervisor/mod.rs" <<'EOF'
//! Crash-only supervision entry (placeholder). No locks across .await in hot paths.
EOF

cat > "$CRATE_PATH/src/supervisor/backoff.rs" <<'EOF'
//! Jittered backoff + intensity caps (placeholder).
EOF

cat > "$CRATE_PATH/src/supervisor/child.rs" <<'EOF'
//! Child task runner; on panic emit ServiceCrashed (placeholder).
EOF

cat > "$CRATE_PATH/src/supervisor/lifecycle.rs" <<'EOF'
//! Lifecycle state machine (placeholder).
EOF

# Amnesia flag
cat > "$CRATE_PATH/src/amnesia.rs" <<'EOF'
//! Global Amnesia Mode flag + metrics label helper (placeholder).
EOF

# Shutdown
cat > "$CRATE_PATH/src/shutdown.rs" <<'EOF'
//! wait_for_ctrl_c(): cooperative shutdown (stub).
pub async fn wait_for_ctrl_c() -> std::io::Result<()> {
    Ok(())
}
EOF

# Internal
cat > "$CRATE_PATH/src/internal/mod.rs" <<'EOF'
pub mod constants;
pub mod types;
EOF

cat > "$CRATE_PATH/src/internal/constants.rs" <<'EOF'
//! Internal tunables. OAP constants here are comments only; kernel does not enforce OAP.
//! See docs/specs/OAP-1.md for the external normative spec.
EOF

cat > "$CRATE_PATH/src/internal/types.rs" <<'EOF'
//! Small internal types shared across modules (placeholder).
EOF

echo "✅ ron-kernel2 scaffold created at $CRATE_PATH"
echo "Next steps:"
echo " - Open $CRATE_PATH/README.md and API.md and paste the finalized docs."
echo " - Start filling tests in $CRATE_PATH/tests to enforce invariants."
echo " - cargo build -p ron-kernel2"
