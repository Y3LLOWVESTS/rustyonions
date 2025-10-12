#!/usr/bin/env bash
set -euo pipefail

ROOT="crates/svc-dht2"

note() { printf "%s\n" "$*" >&2; }
mk() { mkdir -p "$1"; note "dir: $1"; }

writefile() { # writefile <path> <<'EOF' ... EOF
  local path="$1"; shift || true
  mk "$(dirname "$path")"
  cat >"$path"
  note "write: $path"
}

# ───────────────────────────────────────────────────────────────────────────────
# Root files
# ───────────────────────────────────────────────────────────────────────────────
mk "$ROOT"

writefile "$ROOT/README.md" <<'EOF'
<!-- svc-dht2 README placeholder. Replace with your finalized README.md. -->
# svc-dht2
Role: service (Kademlia/Discv5 discovery & provider lookups)
Status: draft • MSRV: 1.80.0
This is a placeholder; drop in the finalized README content you approved.
EOF

writefile "$ROOT/CHANGELOG.md" <<'EOF'
# Changelog
All notable changes to this crate will be documented here following SemVer.
EOF

writefile "$ROOT/LICENSE-APACHE" <<'EOF'
Apache License placeholder (dual-licensed with MIT).
EOF

writefile "$ROOT/LICENSE-MIT" <<'EOF'
MIT License placeholder (dual-licensed with Apache-2.0).
EOF

writefile "$ROOT/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["clippy", "rustfmt"]
EOF

mk "$ROOT/.cargo"
writefile "$ROOT/.cargo/config.toml" <<'EOF'
[alias]
test-all = "test --workspace --all-features"
bench-all = "bench --workspace"
fmt-all = "fmt --all"
clippy-all = "clippy --workspace -- -D warnings"
EOF

writefile "$ROOT/.rustfmt.toml" <<'EOF'
max_width = 100
use_small_heuristics = "Max"
newline_style = "Unix"
EOF

writefile "$ROOT/clippy.toml" <<'EOF'
warns = []
deny = ["clippy::dbg_macro"]
EOF

writefile "$ROOT/build.rs" <<'EOF'
// build.rs: embeds build metadata for metrics/tracing (placeholder).
fn main() {}
EOF

writefile "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "svc-dht2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "RustyOnions DHT service (scaffold)"

[features]
default = ["tokio", "serde"]
arti = []
sled-cache = []
tls = []

[dependencies]
# Fill from workspace pins (tokio, axum 0.7, tower-http 0.6, prometheus 0.14, serde, etc.)

[dev-dependencies]
# test-only deps
EOF

# ───────────────────────────────────────────────────────────────────────────────
# src/ tree (no code, just headers)
# ───────────────────────────────────────────────────────────────────────────────
mk "$ROOT/src/supervision" "$ROOT/src/peer" "$ROOT/src/provider" "$ROOT/src/pq" \
   "$ROOT/src/codec" "$ROOT/src/rpc" "$ROOT/src/pipeline" "$ROOT/src/transport" \
   "$ROOT/src/cache"

writefile "$ROOT/src/lib.rs" <<'EOF'
// lib.rs: stable surface & re-exports (placeholder).
// Re-export public API once implemented.
EOF

writefile "$ROOT/src/main.rs" <<'EOF'
// main.rs: binary entrypoint (placeholder).
fn main() {
    // parse flags/env, init tracing/metrics, spawn supervisor, serve admin HTTP
}
EOF

writefile "$ROOT/src/config.rs" <<'EOF'
// config.rs: env/flags, profiles, ASN/PQ/hedge defaults (placeholder).
EOF

writefile "$ROOT/src/errors.rs" <<'EOF'
// errors.rs: error taxonomy with user-hints (placeholder).
EOF

writefile "$ROOT/src/types.rs" <<'EOF'
// types.rs: B3 CID, PeerInfo, ProviderRecord v1 (placeholder).
EOF

writefile "$ROOT/src/invariants.rs" <<'EOF'
// invariants.rs: encoded must-holds for tests (placeholder).
EOF

writefile "$ROOT/src/readiness.rs" <<'EOF'
// readiness.rs: /readyz gate logic (placeholder).
EOF

writefile "$ROOT/src/health.rs" <<'EOF'
// health.rs: /healthz liveness (placeholder).
EOF

writefile "$ROOT/src/metrics.rs" <<'EOF'
// metrics.rs: Prometheus registry & metric defs (placeholder).
EOF

writefile "$ROOT/src/tracing.rs" <<'EOF'
// tracing.rs: tracing setup & span fields (placeholder).
EOF

writefile "$ROOT/src/bootstrap.rs" <<'EOF'
// bootstrap.rs: seed dialing + min-table-fill (placeholder).
EOF

# supervision/
writefile "$ROOT/src/supervision/mod.rs" <<'EOF'
// supervision::mod (placeholder).
EOF

writefile "$ROOT/src/supervision/supervisor.rs" <<'EOF'
// supervisor.rs (placeholder).
EOF

writefile "$ROOT/src/supervision/backoff.rs" <<'EOF'
// backoff.rs (placeholder).
EOF

writefile "$ROOT/src/supervision/signals.rs" <<'EOF'
// signals.rs (placeholder).
EOF

# peer/
writefile "$ROOT/src/peer/mod.rs" <<'EOF'
// peer::mod - routing facade (placeholder).
EOF

writefile "$ROOT/src/peer/id.rs" <<'EOF'
// peer::id - NodeId & XOR distance (placeholder).
EOF

writefile "$ROOT/src/peer/bucket.rs" <<'EOF'
// peer::bucket - Kademlia k-bucket (placeholder).
EOF

writefile "$ROOT/src/peer/table.rs" <<'EOF'
// peer::table - routing table (placeholder).
EOF

writefile "$ROOT/src/peer/selector.rs" <<'EOF'
// peer::selector - α/β-aware selection (placeholder).
EOF

# provider/
writefile "$ROOT/src/provider/mod.rs" <<'EOF'
// provider::mod - provider store facade (placeholder).
EOF

writefile "$ROOT/src/provider/record.rs" <<'EOF'
// provider::record - schema v1 (placeholder).
EOF

writefile "$ROOT/src/provider/store.rs" <<'EOF'
// provider::store - RAM/sled admit & verify (placeholder).
EOF

writefile "$ROOT/src/provider/ttl.rs" <<'EOF'
// provider::ttl - expiry pruning (placeholder).
EOF

writefile "$ROOT/src/provider/republish.rs" <<'EOF'
// provider::republish - republish workflow (placeholder).
EOF

# pq/
writefile "$ROOT/src/pq/mod.rs" <<'EOF'
// pq::mod - PQ posture surface (placeholder).
EOF

writefile "$ROOT/src/pq/algo.rs" <<'EOF'
// pq::algo - ML-DSA/SPHINCS+ selection (placeholder).
EOF

writefile "$ROOT/src/pq/verify.rs" <<'EOF'
// pq::verify - dual-sign verify (placeholder).
EOF

writefile "$ROOT/src/pq/gating.rs" <<'EOF'
// pq::gating - REQUIRE/REQUIRE_ON policy (placeholder).
EOF

# codec/
writefile "$ROOT/src/codec/mod.rs" <<'EOF'
// codec::mod - wire codec index (placeholder).
EOF

writefile "$ROOT/src/codec/frame.rs" <<'EOF'
// codec::frame - OAP/1 frame constants (placeholder).
EOF

writefile "$ROOT/src/codec/decode.rs" <<'EOF'
// codec::decode - parsers (placeholder).
EOF

writefile "$ROOT/src/codec/encode.rs" <<'EOF'
// codec::encode - serializers (placeholder).
EOF

writefile "$ROOT/src/codec/limits.rs" <<'EOF'
// codec::limits - size/time guards (placeholder).
EOF

# rpc/
writefile "$ROOT/src/rpc/mod.rs" <<'EOF'
// rpc::mod - unified surface index (placeholder).
EOF

writefile "$ROOT/src/rpc/kad.rs" <<'EOF'
// rpc::kad - FIND_/PROVIDE handlers (placeholder).
EOF

writefile "$ROOT/src/rpc/discv5.rs" <<'EOF'
// rpc::discv5 - peer discovery (placeholder).
EOF

writefile "$ROOT/src/rpc/bus.rs" <<'EOF'
// rpc::bus - bus topic adapters (placeholder).
EOF

writefile "$ROOT/src/rpc/http.rs" <<'EOF'
// rpc::http - /healthz /readyz /metrics (placeholder).
EOF

# pipeline/
writefile "$ROOT/src/pipeline/mod.rs" <<'EOF'
// pipeline::mod - orchestration (placeholder).
EOF

writefile "$ROOT/src/pipeline/lookup.rs" <<'EOF'
// pipeline::lookup - lookup FSM (placeholder).
EOF

writefile "$ROOT/src/pipeline/provide.rs" <<'EOF'
// pipeline::provide - provide flow (placeholder).
EOF

writefile "$ROOT/src/pipeline/hedging.rs" <<'EOF'
// pipeline::hedging - β hedges (placeholder).
EOF

writefile "$ROOT/src/pipeline/rate_limit.rs" <<'EOF'
// pipeline::rate_limit - inflight caps (placeholder).
EOF

writefile "$ROOT/src/pipeline/asn_guard.rs" <<'EOF'
// pipeline::asn_guard - ASN diversity (placeholder).
EOF

writefile "$ROOT/src/pipeline/deadlines.rs" <<'EOF'
// pipeline::deadlines - budgets/timeouts (placeholder).
EOF

# transport/
writefile "$ROOT/src/transport/mod.rs" <<'EOF'
// transport::mod - abstraction over ron-transport (placeholder).
EOF

writefile "$ROOT/src/transport/clients.rs" <<'EOF'
// transport::clients - pools & timeouts (placeholder).
EOF

writefile "$ROOT/src/transport/tor.rs" <<'EOF'
// transport::tor - arti support (placeholder).
EOF

# cache/
writefile "$ROOT/src/cache/mod.rs" <<'EOF'
// cache::mod - cache facade (placeholder).
EOF

writefile "$ROOT/src/cache/memory.rs" <<'EOF'
// cache::memory - RAM cache (placeholder).
EOF

writefile "$ROOT/src/cache/sled_cache.rs" <<'EOF'
// cache::sled_cache - sled-backed cache (placeholder).
EOF

# ───────────────────────────────────────────────────────────────────────────────
# tests / benches / fuzz / loom / examples
# ───────────────────────────────────────────────────────────────────────────────
mk "$ROOT/tests/chaos" "$ROOT/benches" "$ROOT/fuzz/fuzz_targets" "$ROOT/loom" "$ROOT/examples"

writefile "$ROOT/tests/api_smoke.rs" <<'EOF'
// api_smoke.rs: E2E happy path (placeholder).
EOF

writefile "$ROOT/tests/readiness_bootstrap.rs" <<'EOF'
// readiness_bootstrap.rs: readiness gates (placeholder).
EOF

writefile "$ROOT/tests/provider_roundtrip.rs" <<'EOF'
// provider_roundtrip.rs: record admit/store/serve (placeholder).
EOF

writefile "$ROOT/tests/kbucket_props.rs" <<'EOF'
// kbucket_props.rs: property tests (placeholder).
EOF

writefile "$ROOT/tests/asn_diversity.rs" <<'EOF'
// asn_diversity.rs: ASN caps/entropy tests (placeholder).
EOF

writefile "$ROOT/tests/deadline_hedge.rs" <<'EOF'
// deadline_hedge.rs: deadline & hedge tests (placeholder).
EOF

writefile "$ROOT/tests/chaos/netem.rs" <<'EOF'
// chaos/netem.rs: latency/loss model tests (placeholder).
EOF

writefile "$ROOT/tests/chaos/partition.rs" <<'EOF'
// chaos/partition.rs: split-brain healing (placeholder).
EOF

writefile "$ROOT/tests/chaos/soak_churn.rs" <<'EOF'
// chaos/soak_churn.rs: long soak & churn (placeholder).
EOF

writefile "$ROOT/benches/lookup_bench.rs" <<'EOF'
// lookup_bench.rs: SLO bench harness (placeholder).
EOF

writefile "$ROOT/benches/README.md" <<'EOF'
# Benches
How to run and interpret histogram outputs (placeholder).
EOF

writefile "$ROOT/fuzz/fuzz_targets/msg_frame_decode.rs" <<'EOF'
// fuzz: msg_frame_decode (placeholder).
EOF

writefile "$ROOT/fuzz/fuzz_targets/kad_packet_decode.rs" <<'EOF'
// fuzz: kad_packet_decode (placeholder).
EOF

writefile "$ROOT/loom/loom_kbucket.rs" <<'EOF'
// loom: k-bucket invariants (placeholder).
EOF

writefile "$ROOT/loom/loom_hedge.rs" <<'EOF'
// loom: hedged fan-out invariants (placeholder).
EOF

writefile "$ROOT/examples/find_providers.rs" <<'EOF'
// example: find_providers (placeholder).
EOF

writefile "$ROOT/examples/provide.rs" <<'EOF'
// example: provide (placeholder).
EOF

# ───────────────────────────────────────────────────────────────────────────────
# docs / scripts / workflows
# ───────────────────────────────────────────────────────────────────────────────
mk "$ROOT/docs" "$ROOT/scripts/chaos" "$ROOT/.github/workflows"

writefile "$ROOT/docs/IDB.md" <<'EOF'
# IDB (Invariant-Driven Blueprint)
List invariants and how tests enforce them (placeholder).
EOF

writefile "$ROOT/docs/DHT_CONFIG.md" <<'EOF'
# DHT Config
Env/flags table, profiles, examples (placeholder).
EOF

writefile "$ROOT/docs/PERFORMANCE.md" <<'EOF'
# Performance & SLO Repro
Machine profile, seeds, churn, commands (placeholder).
EOF

writefile "$ROOT/docs/arch.mmd" <<'EOF'
flowchart LR
  A[svc-index] -->|FIND_PROVIDERS| DHT(svc-dht2)
  B[svc-storage] -->|PROVIDE| DHT
  DHT --> Peers{Mesh}
  DHT --> PM[[Prometheus]]
  DHT --> BUS[[Kernel Bus]]
EOF

writefile "$ROOT/docs/sequence.mmd" <<'EOF'
sequenceDiagram
  actor Caller as svc-index
  participant Bus as RON Bus
  participant D as svc-dht2
  Caller->>Bus: FindProviders{cid}
  Bus->>D: Deliver
  D-->>Bus: Providers{addrs, rf}
  Bus-->>Caller: Response
EOF

writefile "$ROOT/docs/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Boot
  Boot --> Bootstrap: seeds reachable
  Bootstrap --> Running: min_table_fill
  Running --> Backoff: churn/rate
  Backoff --> Running: jittered restart
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
EOF

writefile "$ROOT/scripts/run-local.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Run svc-dht2 locally with sane defaults (placeholder).
EOF
chmod +x "$ROOT/scripts/run-local.sh"

writefile "$ROOT/scripts/render-mermaid.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Render docs/*.mmd to SVG (placeholder).
EOF
chmod +x "$ROOT/scripts/render-mermaid.sh"

writefile "$ROOT/scripts/chaos/netem.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Apply/remove tc netem profiles (placeholder).
EOF
chmod +x "$ROOT/scripts/chaos/netem.sh"

writefile "$ROOT/scripts/chaos/partition.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Simulate network partition topology locally (placeholder).
EOF
chmod +x "$ROOT/scripts/chaos/partition.sh"

writefile "$ROOT/.github/workflows/ci.yml" <<'EOF'
name: ci
on: [push, pull_request]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo 'placeholder CI (fmt/clippy/tests/deny/coverage/mutation)'
EOF

writefile "$ROOT/.github/workflows/mermaid.yml" <<'EOF'
name: mermaid
on: [push, pull_request]
jobs:
  render:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo 'placeholder mermaid render'
EOF

writefile "$ROOT/.github/workflows/fuzz.yml" <<'EOF'
name: fuzz
on:
  schedule: [{cron: "0 3 * * *"}]
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo 'placeholder fuzz workflow'
EOF

writefile "$ROOT/.github/workflows/perf.yml" <<'EOF'
name: perf
on:
  workflow_dispatch:
  schedule: [{cron: "0 6 * * *"}]
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo 'placeholder perf SLO regression check'
EOF

note "Scaffold complete at $ROOT"
