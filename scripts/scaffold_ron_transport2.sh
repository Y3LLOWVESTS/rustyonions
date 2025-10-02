#!/usr/bin/env bash
set -euo pipefail

ROOT="crates/ron-transport2"

# Directories
mkdir -p "$ROOT"/{.cargo,.github/workflows,scripts/ci,scripts/local,specs,benches,examples,tests/vectors,tests/integration,tests/amnesia,tests/loom,tests/soak,fuzz/fuzz_targets,src/{util,conn,tcp,tls,arti,quic}}

# -----------------------------
# Top-level files
# -----------------------------
cat > "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "ron-transport2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "RustyOnions transport library (TCP/TLS + optional Arti + optional QUIC) — scaffold only"
repository = "https://example.com/RustyOnions"

[lib]
name = "ron_transport2"
path = "src/lib.rs"

[features]
default = []
arti = []
quic = []

[dependencies]
# Intentionally minimal; real pins come from workspace once code lands.

[dev-dependencies]
# criterion = "0.5"
# loom = "0.7"
EOF

cat > "$ROOT/README.md" <<'EOF'
# ron-transport2 (scaffold)

Pure **library** scaffold for RustyOnions transport:
- TCP/TLS via caller-supplied tokio_rustls configs
- Optional Tor v3 (Arti) behind `feature = "arti"`
- Optional QUIC behind `feature = "quic"`

This crate currently contains *structure only* (no implementation).
EOF

cat > "$ROOT/ALL_DOCS.md" <<'EOF'
# Combined Documentation (placeholder)
Insert your finalized combined docs here (IDB, SECURITY, QUANTUM, INTEROP, PERFORMANCE, TESTS, RUNBOOK, etc.).
EOF

# -----------------------------
# .cargo
# -----------------------------
cat > "$ROOT/.cargo/config.toml" <<'EOF'
[build]
# Local build/test knobs can live here (e.g., enabling loom via cfg)
EOF

# -----------------------------
# GitHub workflows
# -----------------------------
cat > "$ROOT/.github/workflows/ci.yml" <<'EOF'
name: ron-transport2 CI
on:
  push:
  pull_request:
jobs:
  build-test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features: ["", "arti", "quic", "arti,quic"]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build
        run: cargo build -p ron-transport2 ${MATRIX:+--features ${{ matrix.features }}}
        env: { MATRIX: ${{ matrix.features }} }
      - name: Test
        run: cargo test -p ron-transport2 ${MATRIX:+--features ${{ matrix.features }}}
        env: { MATRIX: ${{ matrix.features }} }
      - name: Clippy (deny warnings)
        run: cargo clippy -p ron-transport2 ${MATRIX:+--features ${{ matrix.features }}} -- -D warnings
        env: { MATRIX: ${{ matrix.features }} }
EOF

cat > "$ROOT/.github/workflows/perf.yml" <<'EOF'
name: ron-transport2 Perf
on: { workflow_dispatch: {} }
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run benches (placeholder)
        run: cargo bench -p ron-transport2 || true
      - name: Archive Criterion artifacts
        uses: actions/upload-artifact@v4
        with:
          name: bench-artifacts
          path: target/criterion
EOF

cat > "$ROOT/.github/workflows/tla.yml" <<'EOF'
name: ron-transport2 TLA
on:
  push:
  pull_request:
jobs:
  tla:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run TLC (placeholder)
        run: bash crates/ron-transport2/scripts/ci/run_tlc.sh
EOF

# -----------------------------
# scripts
# -----------------------------
cat > "$ROOT/scripts/ci/run_tlc.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "[tla] placeholder TLC run for specs; integrate TLA2Tools/Apalache later."
EOF
chmod +x "$ROOT/scripts/ci/run_tlc.sh"

cat > "$ROOT/scripts/ci/env_sanitize.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export AMNESIA=1
export TMPDIR="${TMPDIR:-/tmp}"
echo "[ci] AMNESIA=1 and TMPDIR set for no-disk tests."
EOF
chmod +x "$ROOT/scripts/ci/env_sanitize.sh"

cat > "$ROOT/scripts/local/perf_repro.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Local perf repro placeholder (pin warmups/iters when benches land)."
EOF
chmod +x "$ROOT/scripts/local/perf_repro.sh"

# -----------------------------
# specs
# -----------------------------
cat > "$ROOT/specs/transport_handshake.tla" <<'EOF'
---- MODULE transport_handshake ----
EXTENDS Naturals, Sequences
(* Minimal sketch; wire real invariants later. *)
====
EOF

cat > "$ROOT/specs/transport_handshake.cfg" <<'EOF'
# Placeholder TLC configuration
EOF

cat > "$ROOT/specs/README.md" <<'EOF'
# Specs
Tiny TLA+ sketch for dial→HELLO→{Ready,NotReady,Timeout} and reason invariants. CI runs a bounded check.
EOF

# -----------------------------
# benches
# -----------------------------
cat > "$ROOT/benches/bench_throughput.rs" <<'EOF'
#![allow(unused)]
fn main() {}
// Placeholder Criterion bench: 1 MiB frames, ~64 KiB streaming.
EOF

cat > "$ROOT/benches/bench_latency.rs" <<'EOF'
#![allow(unused)]
fn main() {}
// Placeholder Criterion bench: connect→first-byte latency per backend.
EOF

# -----------------------------
# examples
# -----------------------------
cat > "$ROOT/examples/bench_echo.rs" <<'EOF'
fn main() { println!("bench_echo placeholder"); }
EOF

cat > "$ROOT/examples/tls_echo.rs" <<'EOF'
fn main() { println!("tls_echo placeholder"); }
EOF

cat > "$ROOT/examples/onion_echo.rs" <<'EOF'
fn main() { println!("onion_echo (Arti) placeholder"); }
EOF

cat > "$ROOT/examples/quic_echo.rs" <<'EOF'
fn main() { println!("quic_echo placeholder"); }
EOF

# -----------------------------
# tests: vectors + suites
# -----------------------------
cat > "$ROOT/tests/vectors/oap_hello.json" <<'EOF'
{ "name": "oap_hello", "version": 1, "notes": "placeholder vector" }
EOF

cat > "$ROOT/tests/vectors/comp_bounds.json" <<'EOF'
{ "max_frame_bytes": 1048576, "inflate_cap": "8x", "notes": "placeholder" }
EOF

cat > "$ROOT/tests/vectors/tor_parity.json" <<'EOF'
{ "backend": "arti", "parity": true, "notes": "placeholder" }
EOF

cat > "$ROOT/tests/vectors/pq_hybrid_hello.json" <<'EOF'
{ "kex": "hybrid_x25519_mlkem768", "sig": "ed25519", "notes": "placeholder" }
EOF

cat > "$ROOT/tests/integration/tls_handshake_limits.rs" <<'EOF'
#[test]
fn tls_handshake_limits_placeholder() { assert!(true); }
EOF

cat > "$ROOT/tests/integration/arti_bootstrap_ready.rs" <<'EOF'
#[test]
fn arti_bootstrap_ready_placeholder() { assert!(true); }
EOF

cat > "$ROOT/tests/integration/quic_parity.rs" <<'EOF'
#[test]
fn quic_parity_placeholder() { assert!(true); }
EOF

cat > "$ROOT/tests/amnesia/no_disk_touches.rs" <<'EOF'
#[test]
fn no_disk_touches_placeholder() { assert!(true); }
EOF

cat > "$ROOT/tests/loom/single_writer.rs" <<'EOF'
#[test]
fn single_writer_placeholder() { assert!(true); }
EOF

cat > "$ROOT/tests/soak/loopback_1MiB.rs" <<'EOF'
#[test]
fn soak_loopback_placeholder() { assert!(true); }
EOF

# -----------------------------
# fuzz target
# -----------------------------
cat > "$ROOT/fuzz/fuzz_targets/frame_boundaries.rs" <<'EOF'
#![no_main]
// Placeholder fuzz target for frame boundaries.
EOF

# -----------------------------
# src files
# -----------------------------
cat > "$ROOT/src/lib.rs" <<'EOF'
//! ron-transport2 scaffold: lib-only entrypoint (no implementation yet).
//! Re-exports will live here once modules are implemented.

pub mod config;
pub mod limits;
pub mod error;
pub mod reason;
pub mod readiness;
pub mod metrics;
pub mod types;
pub mod util;
pub mod conn;
pub mod tcp;
pub mod tls;
#[cfg(feature = "arti")]
pub mod arti;
#[cfg(feature = "quic")]
pub mod quic;
EOF

cat > "$ROOT/src/config.rs" <<'EOF'
//! TransportConfig DTO (placeholder; no implementation yet).
pub struct TransportConfig;
EOF

cat > "$ROOT/src/limits.rs" <<'EOF'
//! Centralized ceilings (placeholder).
pub const MAX_FRAME_BYTES: usize = 1_048_576;
pub const STREAM_CHUNK_BYTES: usize = 65_536;
EOF

cat > "$ROOT/src/error.rs" <<'EOF'
//! Error taxonomy (placeholder).
pub enum Error {}
EOF

cat > "$ROOT/src/reason.rs" <<'EOF'
//! Append-only reason canon (placeholder).
pub const EXAMPLE_REASON: &str = "placeholder_reason";
EOF

cat > "$ROOT/src/readiness.rs" <<'EOF'
//! Tri-state readiness (placeholder).
pub enum Readiness { NotReady, Degraded, Ready }
EOF

cat > "$ROOT/src/metrics.rs" <<'EOF'
//! Prometheus metrics declarations (placeholder).
pub struct Metrics;
EOF

cat > "$ROOT/src/types.rs" <<'EOF'
//! Public DTOs/handles (placeholder).
pub struct Listener;
pub struct Transport;
pub struct ConnStats;
EOF

# util/
cat > "$ROOT/src/util/mod.rs" <<'EOF'
pub mod cancel;
pub mod timeouts;
pub mod bytes;
EOF

cat > "$ROOT/src/util/cancel.rs" <<'EOF'
//! Cooperative cancellation helpers (placeholder).
pub struct CancelToken;
EOF

cat > "$ROOT/src/util/timeouts.rs" <<'EOF'
//! Deadline helpers (placeholder).
pub struct Timeouts;
EOF

cat > "$ROOT/src/util/bytes.rs" <<'EOF'
//! Owned byte helpers (placeholder).
pub struct OwnedBytes;
EOF

# conn/
cat > "$ROOT/src/conn/mod.rs" <<'EOF'
pub mod backpressure;
pub mod reader;
pub mod writer;
pub mod rate_limit;
EOF

cat > "$ROOT/src/conn/backpressure.rs" <<'EOF'
//! Bounded writer queue and counters (placeholder).
pub struct Backpressure;
EOF

cat > "$ROOT/src/conn/reader.rs" <<'EOF'
//! Single reader task (placeholder).
pub struct Reader;
EOF

cat > "$ROOT/src/conn/writer.rs" <<'EOF'
//! Single writer task (placeholder).
pub struct Writer;
EOF

cat > "$ROOT/src/conn/rate_limit.rs" <<'EOF'
//! Optional token bucket hooks (placeholder).
pub struct RateLimit;
EOF

# tcp/
cat > "$ROOT/src/tcp/mod.rs" <<'EOF'
pub mod listener;
pub mod dialer;
EOF

cat > "$ROOT/src/tcp/listener.rs" <<'EOF'
//! TCP accept loop (placeholder).
pub struct TcpListener;
EOF

cat > "$ROOT/src/tcp/dialer.rs" <<'EOF'
//! TCP dialer (placeholder).
pub struct TcpDialer;
EOF

# tls/
cat > "$ROOT/src/tls/mod.rs" <<'EOF'
pub mod client;
pub mod server;
EOF

cat > "$ROOT/src/tls/client.rs" <<'EOF'
//! TLS client (placeholder).
pub struct TlsClient;
EOF

cat > "$ROOT/src/tls/server.rs" <<'EOF'
//! TLS server (placeholder).
pub struct TlsServer;
EOF

# arti/ (feature)
cat > "$ROOT/src/arti/mod.rs" <<'EOF'
pub mod client;
pub mod service;
pub mod readiness;
EOF

cat > "$ROOT/src/arti/client.rs" <<'EOF'
//! Arti outbound (placeholder).
pub struct ArtiClient;
EOF

cat > "$ROOT/src/arti/service.rs" <<'EOF'
//! Arti onion service (placeholder).
pub struct ArtiService;
EOF

cat > "$ROOT/src/arti/readiness.rs" <<'EOF'
//! Arti readiness glue (placeholder).
pub struct ArtiReadiness;
EOF

# quic/ (feature)
cat > "$ROOT/src/quic/mod.rs" <<'EOF'
pub mod client;
pub mod server;
EOF

cat > "$ROOT/src/quic/client.rs" <<'EOF'
//! QUIC client (placeholder).
pub struct QuicClient;
EOF

cat > "$ROOT/src/quic/server.rs" <<'EOF'
//! QUIC server (placeholder).
pub struct QuicServer;
EOF

echo "✅ ron-transport2 scaffold created (all files written)."
