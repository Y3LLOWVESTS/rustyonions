#!/usr/bin/env bash
# Scaffolds the full svc-overlay crate structure with descriptive, tiny files (no service code).
# Usage:
#   bash scripts/scaffold_svc_overlay.sh [--force]
# Behavior:
#   - By default, will NOT overwrite existing files.
#   - Pass --force to overwrite files.
set -euo pipefail

TARGET_DIR="crates/svc-overlay"
FORCE="0"
if [[ "${1:-}" == "--force" ]]; then
  FORCE="1"
fi

say() { printf "\033[1;32m[scaffold]\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m[warn]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[error]\033[0m %s\n" "$*" 1>&2; }

mkd() {
  mkdir -p "$1"
  say "dir  : $1"
}

write() {
  local path="$1"
  local content="$2"
  if [[ -e "$path" && "$FORCE" != "1" ]]; then
    warn "skip : $path (exists; use --force to overwrite)"
    return 0
  fi
  printf "%s" "$content" > "$path"
  say "file : $path"
}

root="$TARGET_DIR"

say "Scaffolding crate into ./$root ..."
mkd "$root"
mkd "$root/.github/workflows"
mkd "$root/docs/specs"
mkd "$root/docs/api-history/svc-overlay"
mkd "$root/benches"
mkd "$root/fuzz/fuzz_targets"
mkd "$root/fuzz/corpus"
mkd "$root/examples"
mkd "$root/tests/integration"
mkd "$root/tests/loom"
mkd "$root/.devcontainer"
mkd "$root/src"
mkd "$root/src/api"
mkd "$root/src/admin"
mkd "$root/src/readiness"
mkd "$root/src/protocol"
mkd "$root/src/transport"
mkd "$root/src/conn"
mkd "$root/src/gossip"
mkd "$root/src/auth"
mkd "$root/src/pq"

# --------------------------
# Top-level metadata files
# --------------------------
write "$root/.gitignore" "\
/target
**/*.svg
coverage/
fuzz/artifacts/
"

write "$root/CHANGELOG.md" "\
# CHANGELOG — svc-overlay

All notable changes to this crate will be documented here.
Follows SemVer. Record changes to CLI flags, HTTP/metrics schema, and lib public API.

## [0.1.0] - TBA
- Initial scaffold (no runtime code).
"

write "$root/LICENSE-MIT" "\
MIT License

Copyright (c) 2025

Permission is hereby granted, free of charge, to any person obtaining a copy...
"

write "$root/LICENSE-APACHE" "\
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION...
"

write "$root/README.md" "\
# svc-overlay

> Role: service (sessions & gossip; no DHT)  
> Status: scaffold  
> MSRV: 1.80.0

This is a **structure-only** scaffold (no service code) matching the RustyOnions canon.
See \`docs/ALL_DOCS.md\` and \`docs/IDB.md\` for invariants and acceptance gates.

## Layout
- Small, single-purpose files (<300–400 LOC target).
- Contracts enforced by tests, fuzz seeds, and CI workflows.
"

write "$root/Cargo.toml" "\
[package]
name = \"svc-overlay\"
version = \"0.1.0\"
edition = \"2021\"
# NOTE: Dependencies are inherited from [workspace.dependencies] at the repo root to avoid drift.
# Add only crate-local features here. Keep optional deps feature-gated.

[features]
default = []
arti = []     # Tor/Arti transport via ron-transport (optional)
quic = []     # QUIC transport (optional)
tls = []      # TLS transport (optional)
pq = []       # Post-quantum hybrid posture flags (transport-level)
libapi = []   # Minimal embedding API
cli = []      # CLI surface for the binary

[dev-dependencies]
# Add test-only tooling here as needed by the workspace.
"

# --------------------------
# GitHub workflows
# --------------------------
write "$root/.github/workflows/ci.yml" "\
name: ci
on: [push, pull_request]
jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy,rustfmt
      - name: Build
        run: cargo build -p svc-overlay
      - name: Clippy (deny warnings)
        run: cargo clippy -p svc-overlay -- -D warnings
      - name: Test
        run: cargo test -p svc-overlay --all-features
      - name: Doc tests
        run: cargo test -p svc-overlay --doc
      - name: Cargo deny
        run: cargo deny check
      - name: Cargo audit
        run: cargo install cargo-audit || true
      - name: Audit vulnerabilities
        run: cargo audit
      - name: Coverage (floor 85%)
        run: echo \"(hook up your coverage tool here and fail below 85%)\"
"

write "$root/.github/workflows/render-mermaid.yml" "\
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          mkdir -p docs
          for f in \$(git ls-files '*.mmd'); do
            out=\"\${f%.mmd}.svg\"
            mmdc -i \"\$f\" -o \"\$out\"
          done
"

write "$root/.github/workflows/concurrency-guardrails.yml" "\
name: concurrency-guardrails
on: [push, pull_request]
jobs:
  loom:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Loom interleavings
        run: RUSTFLAGS='--cfg loom' cargo test -p svc-overlay --test loom_overlay
"

write "$root/.github/workflows/contract-apis.yml" "\
name: contract-apis
on: [push, pull_request]
jobs:
  public-api-and-schemas:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Cargo public-api
        run: |
          cargo install cargo-public-api || true
          cargo public-api -p svc-overlay > public-api.txt || true
      - name: Diff api-history snapshots
        run: echo \"(compare against docs/api-history/svc-overlay/*)\" 
"

# --------------------------
# docs/
# --------------------------
write "$root/docs/ALL_DOCS.md" "\
# ALL_DOCS.md — svc-overlay (combined crate docs)
Paste your combined documentation here (SECURITY, INTEROP, QUANTUM, PERFORMANCE, RUNBOOK, etc.).
"

write "$root/docs/IDB.md" "\
# IDB — Invariant-Driven Blueprint (svc-overlay)

## Invariants (MUST)
- [I-1] Authenticated sessions; bounded inflight; one-writer per conn.
- [I-2] OAP/1 envelope limits enforced (max_frame = 1 MiB).
- [I-3] Amnesia honored: no durable peer state.

## Principles (SHOULD)
- [P-1] Delegate discovery to svc-dht; overlay is sessions + gossip only.
- [P-2] Observability contract: /healthz, /readyz, golden metrics.

## Acceptance Gates (PROOF)
- [G-1] Chaos churn w/o deadlock; /readyz degrades and recovers.
- [G-2] p95 gossip < 150 ms intra-AZ.
- [G-3] Loom + fuzz + coverage ≥ 85%; 24h soak = 0 FD leaks.
- [G-4] Interop vectors pass; OAP/1 polyglot tests green.

## Forbidden (Anti-scope)
- [F-1] No DHT logic (FIND_NODE/STORE/buckets).
- [F-2] No persistent peer state.
- [F-3] No app semantics (storage/mailbox/ledger/policy).
"

write "$root/docs/arch.mmd" "\
flowchart LR
  subgraph Node
    A[svc-index/storage/mailbox/omnigate] -->|Bus/RPC| B(svc-overlay)
  end
  B -->|Transport| C[ron-transport]
  B -->|Discovery| D[svc-dht]
  B -->|/metrics| E[[ron-metrics]]
  style B fill:#0b7285,stroke:#083344,color:#fff
"

write "$root/docs/sequence.mmd" "\
sequenceDiagram
  actor Caller
  participant Bus as RON Bus
  participant O as svc-overlay
  Caller->>Bus: Overlay::dial(peer)
  Bus->>O: Request
  O-->>Bus: SessionHandle
  O-->>Bus: Health events
"

write "$root/docs/state.mmd" "\
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: session established
  Running --> Backoff: disconnect/quota
  Backoff --> Running: retry (jitter)
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
"

write "$root/docs/specs/OAP-1.md" "\
# OAP/1 — Local Normative Notes (svc-overlay)
- Frame cap: 1 MiB (reject above).
- Storage streaming chunk: ~64 KiB (separate concern, noted for context).
- REQUIRED headers for object frames include obj:\"b3:<hex>\".
- Interop vectors for hello/ack and error paths live under tests/.
"

write "$root/docs/api-history/svc-overlay/v0.1.0-metrics.txt" "\
# Golden metrics snapshot (names/labels)
overlay_sessions_active{service}
overlay_gossip_latency_seconds{stage}
bus_queue_depth{service}
service_restarts_total{service}
errors_total{stage,kind}
rejected_total{reason}
"

write "$root/docs/api-history/svc-overlay/v0.1.0-cli.txt" "\
# Golden CLI flags (subject to SemVer)
--bind <ADDR>
--max-conns <N>
--config <PATH>
--amnesia <bool>
"

write "$root/docs/api-history/svc-overlay/v0.1.0-http.json" "\
{
  \"healthz\": {\"status\": \"ok\"},
  \"readyz\": {\"ready\": true, \"reason\": \"\"},
  \"metrics\": \"<prometheus exposition text>\"
}
"

write "$root/docs/api-history/svc-overlay/v0.1.0-libapi.txt" "\
# Public lib API surface (libapi feature)
OverlayConfig
OverlayHandle
spawn(cfg) -> OverlayHandle
"

# --------------------------
# benches/
# --------------------------
write "$root/benches/oap_codec.rs" "\
/*! oap_codec.rs — microbench placeholders
- Benchmarks for frame encode/decode at 1 KiB / 64 KiB / 1 MiB.
*/
"

write "$root/benches/handshake.rs" "\
/*! handshake.rs — microbench placeholders
- Control-plane handshake latency; PQ-hybrid delta vs classic.
*/
"

# --------------------------
# fuzz/
# --------------------------
write "$root/fuzz/fuzz_targets/oap_frame_parse.rs" "// fuzz target placeholder: OAP envelope parsing edge cases\n"
write "$root/fuzz/fuzz_targets/gossip_lane.rs"   "// fuzz target placeholder: gossip lane scheduler/backpressure\n"
write "$root/fuzz/corpus/valid_frame"            "seed: minimal valid OAP/1 frame\n"
write "$root/fuzz/corpus/oversize_frame"         "seed: frame > 1 MiB to assert hard cap behavior\n"
write "$root/fuzz/corpus/split_frame"            "seed: fragmented/partial frame for reassembly paths\n"

# --------------------------
# examples/
# --------------------------
write "$root/examples/libapi_embed.rs" "\
/*! libapi_embed.rs — example placeholder
Demonstrates embedding the service in-process via the libapi surface.
*/
"

write "$root/examples/pq_embed.rs" "\
/*! pq_embed.rs — example placeholder
Demonstrates enabling transport-level PQ hybrid posture via feature flags.
*/
"

# --------------------------
# tests/
# --------------------------
write "$root/tests/http_contract.rs" "\
// http_contract.rs — placeholder
// Validates /healthz, /readyz semantics and /metrics scrape shape.
"

write "$root/tests/metrics_schema.rs" "\
// metrics_schema.rs — placeholder
// Emits sample metrics and asserts names/labels match docs/api-history.
"

write "$root/tests/readiness_under_pressure.rs" "\
// readiness_under_pressure.rs — placeholder
// Induces saturation; expects early degrade and recovery.
"

write "$root/tests/interop_vectors.rs" "\
// interop_vectors.rs — placeholder
// Golden vectors for hello/ack, oversize/ratio rejects (OAP invariants).
"

write "$root/tests/pq_negotiation.rs" "\
// pq_negotiation.rs — placeholder
// Matrix tests: off<->off, hybrid<->hybrid, hybrid<->off refusal cases.
"

write "$root/tests/integration/overlay_admin_roundtrip.rs" "\
// overlay_admin_roundtrip.rs — placeholder
// Spins admin plane (healthz/readyz/metrics) and asserts end-to-end contracts.
"

write "$root/tests/integration/oap_session_handshake.rs" "\
// oap_session_handshake.rs — placeholder
// Drives session establishment over the real stack (hello/ack paths).
"

write "$root/tests/integration/overlay_oap_streaming.rs" "\
// overlay_oap_streaming.rs — placeholder
// Streaming happy-path with bounded backpressure and latency assertions.
"

write "$root/tests/loom/loom_overlay.rs" "\
// loom_overlay.rs — placeholder
// Interleavings for one-writer invariant, bounded queues, orderly shutdown.
"

# --------------------------
# Devcontainer
# --------------------------
write "$root/.devcontainer/devcontainer.json" "\
{
  \"name\": \"svc-overlay\",
  \"image\": \"mcr.microsoft.com/devcontainers/rust:1-1.80-bookworm\",
  \"features\": {},
  \"customizations\": {
    \"vscode\": {
      \"extensions\": [\"rust-lang.rust-analyzer\", \"serayuzgur.crates\"]
    }
  },
  \"postCreateCommand\": \"cargo fetch\"
}
"

# --------------------------
# src/ — tiny, single-purpose comment files (no code)
# --------------------------
write "$root/src/main.rs" "\
//! main.rs — wire-up entrypoint (placeholder; no runtime code).
//! CLI -> config -> tracing/metrics -> supervisor -> shutdown.
"

write "$root/src/lib.rs" "\
//! lib.rs — (feature `libapi`) public embed API re-exports (placeholder; no runtime code).
"

write "$root/src/api/mod.rs" "\
//! api/mod.rs — (feature `libapi`) OverlayConfig, OverlayHandle, spawn(cfg) (placeholder only).
"

write "$root/src/cli.rs" "\
//! cli.rs — CLI flags -> config overrides (placeholder; names match api-history).
"

write "$root/src/config.rs" "\
//! config.rs — typed config, defaults, env mapping, validation (placeholder).
"

write "$root/src/bootstrap.rs" "\
//! bootstrap.rs — tracing + Prometheus exporter bring-up (placeholder).
"

write "$root/src/observe.rs" "\
//! observe.rs — metric names/labels + registration helpers (placeholder).
"

write "$root/src/admin/mod.rs" "\
//! admin/mod.rs — admin HTTP router builder (placeholder).
"

write "$root/src/admin/health.rs" "\
//! admin/health.rs — liveness check handler (placeholder).
"

write "$root/src/admin/ready.rs" "\
//! admin/ready.rs — readiness aggregator (placeholder).
"

write "$root/src/admin/metrics.rs" "\
//! admin/metrics.rs — /metrics exposition (placeholder).
"

write "$root/src/admin/version.rs" "\
//! admin/version.rs — build/version endpoint (placeholder).
"

write "$root/src/supervisor.rs" "\
//! supervisor.rs — supervises listener, gossip pool, admin, config watcher (placeholder).
"

write "$root/src/listener.rs" "\
//! listener.rs — accept loop, hands streams to per-connection supervisor (placeholder).
"

write "$root/src/shutdown.rs" "\
//! shutdown.rs — cancellation tokens & graceful stop patterns (placeholder).
"

write "$root/src/readiness/mod.rs" "\
//! readiness/mod.rs — readiness state machine, gauges/counters (placeholder).
"

write "$root/src/readiness/sampler.rs" "\
//! readiness/sampler.rs — queue-depth sampling windows & thresholds (placeholder).
"

write "$root/src/protocol/mod.rs" "\
//! protocol/mod.rs — wire building blocks; isolates framing from transports (placeholder).
"

write "$root/src/protocol/oap.rs" "\
//! protocol/oap.rs — OAP/1 codec (1 MiB hard cap, ratio guard) (placeholder).
"

write "$root/src/protocol/flags.rs" "\
//! protocol/flags.rs — REQ/RESP/EVENT/COMP/ACKREQ bits and helpers (placeholder).
"

write "$root/src/protocol/handshake.rs" "\
//! protocol/handshake.rs — HELLO/ACK negotiation table & errors (placeholder).
"

write "$root/src/protocol/cbor.rs" "\
//! protocol/cbor.rs — deterministic DAG-CBOR helpers (placeholder).
"

write "$root/src/protocol/error.rs" "\
//! protocol/error.rs — protocol-local error taxonomy (placeholder).
"

write "$root/src/transport/mod.rs" "\
//! transport/mod.rs — abstraction over TCP/TLS/QUIC/Tor (placeholder).
"

write "$root/src/transport/tls.rs" "\
//! transport/tls.rs — TLS 1.3 via rustls; ALPN; timeouts (placeholder).
"

write "$root/src/transport/quic.rs" "\
//! transport/quic.rs — QUIC adapter; single bidi; 0-RTT off (placeholder).
"

write "$root/src/transport/tor.rs" "\
//! transport/tor.rs — Tor/Arti adapter; bootstrap readiness (placeholder).
"

write "$root/src/conn/mod.rs" "\
//! conn/mod.rs — connection types/builders; wires reader/writer tasks (placeholder).
"

write "$root/src/conn/supervisor.rs" "\
//! conn/supervisor.rs — per-connection state, backpressure, shutdown (placeholder).
"

write "$root/src/conn/reader.rs" "\
//! conn/reader.rs — reads bytes -> OAP frames -> bounded ingress (placeholder).
"

write "$root/src/conn/writer.rs" "\
//! conn/writer.rs — single owner of writes; bounded egress; timeouts (placeholder).
"

write "$root/src/conn/error.rs" "\
//! conn/error.rs — connection error kinds (Busy/Timeout/Canceled/...) (placeholder).
"

write "$root/src/gossip/mod.rs" "\
//! gossip/mod.rs — facade used by service (broadcast, subscriptions) (placeholder).
"

write "$root/src/gossip/engine.rs" "\
//! gossip/engine.rs — worker pool; fair-shed first; bounded queues (placeholder).
"

write "$root/src/gossip/types.rs" "\
//! gossip/types.rs — tiny DTOs for gossip envelopes/topics (placeholder).
"

write "$root/src/auth/mod.rs" "\
//! auth/mod.rs — capability-token hooks (placeholder).
"

write "$root/src/auth/macaroon.rs" "\
//! auth/macaroon.rs — macaroon verification adapter; safe logging (placeholder).
"

write "$root/src/pq/mod.rs" "\
//! pq/mod.rs — PQ posture types/metrics; readiness visibility (placeholder).
"

write "$root/src/pq/negotiate.rs" "\
//! pq/negotiate.rs — handshake helpers to detect downgrades; posture metrics (placeholder).
"

write "$root/src/limits.rs" "\
//! limits.rs — centralized caps (1 MiB frame, ratio guard) + reject reasons (placeholder).
"

write "$root/src/errors.rs" "\
//! errors.rs — crate-level error unification + mapping to protocol/HTTP (placeholder).
"

write "$root/src/types.rs" "\
//! types.rs — shared newtypes/aliases (peer ids, counters) (placeholder).
"

say "Done."
say "Scaffold created in: $root"
say "Tip: run 'git add $root' to stage the scaffold."
