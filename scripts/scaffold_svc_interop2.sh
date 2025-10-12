#!/usr/bin/env bash
# Scaffolder for crates/svc-interop2 — structure-first, tiny stubs, no business logic.
set -euo pipefail

ROOT="crates/svc-interop2"

# --- helpers ---
mkd() { mkdir -p "$1"; }
mkf() { mkdir -p "$(dirname "$1")"; cat > "$1"; }
touchf() { mkdir -p "$(dirname "$1")"; : > "$1"; }

echo "Scaffolding $ROOT ..."

# --- directories ---
while read -r d; do
  [ -z "$d" ] && continue
  mkd "$ROOT/$d"
done <<'DIRS'
src
src/config
src/audit
src/middleware
src/routes
src/routes/webhooks
src/clients
src/dto
src/pq
src/telemetry
tests
tests/webhooks
tests/vectors
benches
fuzz
fuzz/fuzz_targets
docs
docs/openapi
docs/api-history
docs/api-history/http
docs/api-history/rust
.github
.github/workflows
ops
ops/alerts
ops/dashboards
scripts
DIRS

# --- top-level files ---
mkf "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "svc-interop2"
version = "0.1.0"
edition = "2021"
publish = false
license = "MIT OR Apache-2.0"
description = "RustyOnions interop service (scaffold)"
readme = "README.md"
repository = "https://example.com/rustyonions"

[features]
libapi = []
tls = []
pq = []
blake3-simd = []
cli = []

[dependencies]
# Workspace-pinned deps recommended; add as you implement.
anyhow = "1"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bytes = "1"
EOF

mkf "$ROOT/README.md" <<'EOF'
# svc-interop2

Scaffold for the RustyOnions interop service. This directory tree is structure-first and code-light.
Replace this README with the finalized svc-interop README content when ready.

- Architecture/sequence/state diagrams go under `docs/*.mmd`
- HTTP/OpenAPI snapshot under `docs/openapi/`
- Public API snapshots under `docs/api-history/`
- CI/gates templates are placed for convenience
EOF

touchf "$ROOT/CHANGELOG.md"
mkf "$ROOT/LICENSE-MIT" <<'EOF'
Permission is hereby granted, free of charge, to any person obtaining a copy...
(Replace with your standard MIT text)
EOF
mkf "$ROOT/LICENSE-APACHE" <<'EOF'
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/
(Replace with your standard Apache-2.0 text)
EOF

mkf "$ROOT/Config.toml.example" <<'EOF'
# svc-interop2 example configuration
bind_addr = "0.0.0.0:8080"
metrics_addr = "127.0.0.1:9909"
max_conns = 512
max_rps = 500
burst_rps = 250
body_limit_bytes = 1048576         # 1 MiB
decomp_ratio_max = 10.0
brownout_errrate_5m = 0.01
upstream_p95_ms = 500
pq_hybrid_on = false

# kms KIDs (dual-verify rotation)
kms_active_kid = ""
kms_next_kid = ""

# optional
allowlist_origins = []
EOF

mkf "$ROOT/.env.example" <<'EOF'
RUST_LOG=info
SVC_INTEROP_BIND_ADDR=0.0.0.0:8080
SVC_INTEROP_METRICS_ADDR=127.0.0.1:9909
SVC_INTEROP_MAX_CONNS=512
SVC_INTEROP_MAX_RPS=500
SVC_INTEROP_BURST_RPS=250
BODY_LIMIT_BYTES=1048576
DECOMP_RATIO_MAX=10.0
BROWNOUT_ERRRATE_5M=0.01
UPSTREAM_P95_MS=500
PQ_HYBRID_ON=false
KMS_ACTIVE_KID=
KMS_NEXT_KID=
ALLOWLIST_ORIGINS=
EOF

# --- src files (tiny compiling stubs only) ---
mkf "$ROOT/src/main.rs" <<'EOF'
fn main() {
    // Minimal, no business logic. Replace when wiring the service.
    println!("svc-interop2 scaffold (no business logic)");
}
EOF

mkf "$ROOT/src/lib.rs" <<'EOF'
//! Optional lib surface for testing/public-api snapshots.
//! Keep this minimal; do not leak service internals here.

#[cfg(feature = "libapi")]
pub struct ServiceOptions;

#[cfg(feature = "libapi")]
impl ServiceOptions {
    pub fn new() -> Self { Self }
}
EOF

mkf "$ROOT/src/config/mod.rs" <<'EOF'
// Typed config placeholder (no logic yet)
#[derive(Debug, Clone)]
pub struct Config;
EOF

mkf "$ROOT/src/error.rs" <<'EOF'
// Central error taxonomy (placeholder)
#[derive(Debug)]
pub enum WireError {
    BadOrigin,
    Unauth,
    BodyLimit,
    DecompLimit,
    RateLimit,
    Backpressure,
    DownstreamUnavailable,
    PolicyBlocked,
    Malformed,
    ClockSkew,
}
EOF

mkf "$ROOT/src/hashing.rs" <<'EOF'
// Hashing policy placeholder: internal BLAKE3; edge-only SHA256 verify (adapters)
pub const INTERNAL_HASH: &str = "blake3-256";
EOF

mkf "$ROOT/src/metrics.rs" <<'EOF'
// Metrics registry placeholder (no exporters yet)
pub struct Metrics;
EOF

mkf "$ROOT/src/readiness.rs" <<'EOF'
// Readiness/brownout placeholders
#[derive(Default)]
pub struct Readiness { pub brownout: bool }
EOF

mkf "$ROOT/src/supervisor.rs" <<'EOF'
// Supervisor placeholder
pub struct Supervisor;
EOF

mkf "$ROOT/src/shutdown.rs" <<'EOF'
// Cooperative shutdown placeholder
pub fn install_shutdown_hook() {}
EOF

mkf "$ROOT/src/queues.rs" <<'EOF'
// Bounded queues placeholder
pub struct Queues;
EOF

# audit
mkf "$ROOT/src/audit/mod.rs" <<'EOF'
// Audit event types (placeholder)
pub struct AuditEvent;
EOF
mkf "$ROOT/src/audit/sink.rs" <<'EOF'
// Audit sinks (placeholder)
pub struct AuditSink;
EOF

# middleware
mkf "$ROOT/src/middleware/mod.rs" <<'EOF'
// tower layers wiring placeholder
EOF
mkf "$ROOT/src/middleware/corr_id.rs" <<'EOF'
// correlation id middleware placeholder
EOF
mkf "$ROOT/src/middleware/capability.rs" <<'EOF'
// capability translation middleware placeholder
EOF
mkf "$ROOT/src/middleware/origin_pin.rs" <<'EOF'
// origin allow-list & ts/skew checks placeholder
EOF
mkf "$ROOT/src/middleware/rate_limit.rs" <<'EOF'
// token bucket limiter placeholder
EOF
mkf "$ROOT/src/middleware/decompress_guard.rs" <<'EOF'
// decompression guard placeholder
EOF

# routes
mkf "$ROOT/src/routes/mod.rs" <<'EOF'
// axum router wiring placeholder
EOF
mkf "$ROOT/src/routes/health.rs" <<'EOF'
// /healthz handler placeholder
EOF
mkf "$ROOT/src/routes/ready.rs" <<'EOF'
// /readyz handler placeholder
EOF
mkf "$ROOT/src/routes/metrics.rs" <<'EOF'
// /metrics handler placeholder
EOF
mkf "$ROOT/src/routes/version.rs" <<'EOF'
// /version handler placeholder
EOF
mkf "$ROOT/src/routes/put.rs" <<'EOF'
// POST /put placeholder
EOF
mkf "$ROOT/src/routes/get_object.rs" <<'EOF'
// GET /o/:addr placeholder
EOF
mkf "$ROOT/src/routes/webhooks/mod.rs" <<'EOF'
// /webhooks/:provider dispatcher placeholder
EOF
mkf "$ROOT/src/routes/webhooks/github.rs" <<'EOF'
// GitHub webhook adapter placeholder
EOF
mkf "$ROOT/src/routes/webhooks/stripe.rs" <<'EOF'
// Stripe webhook adapter placeholder
EOF
mkf "$ROOT/src/routes/webhooks/slack_webhook.rs" <<'EOF'
// Slack webhook adapter placeholder
EOF

# clients
mkf "$ROOT/src/clients/mod.rs" <<'EOF'
// RPC clients placeholder
EOF
mkf "$ROOT/src/clients/passport.rs" <<'EOF'
// svc-passport client placeholder
EOF
mkf "$ROOT/src/clients/storage.rs" <<'EOF'
// svc-storage client placeholder
EOF
mkf "$ROOT/src/clients/index.rs" <<'EOF'
// svc-index client placeholder
EOF
mkf "$ROOT/src/clients/mailbox.rs" <<'EOF'
// svc-mailbox client placeholder
EOF

# dto
mkf "$ROOT/src/dto/mod.rs" <<'EOF'
// DTO module placeholder
EOF
mkf "$ROOT/src/dto/types.rs" <<'EOF'
// DTO types placeholder
EOF

# pq
mkf "$ROOT/src/pq/mod.rs" <<'EOF'
// PQ-hybrid counters/toggles placeholder (no crypto here)
EOF

# telemetry
mkf "$ROOT/src/telemetry/mod.rs" <<'EOF'
// telemetry bring-up placeholder
EOF
mkf "$ROOT/src/telemetry/tracing.rs" <<'EOF'
// tracing helpers placeholder
EOF
mkf "$ROOT/src/telemetry/logging.rs" <<'EOF'
// logging helpers placeholder
EOF

# --- tests ---
mkf "$ROOT/tests/smoke_http.rs" <<'EOF'
// smoke test placeholder (no runtime yet)
#[test]
fn boots_scaffold() { assert!(true); }
EOF
mkf "$ROOT/tests/readiness.rs" <<'EOF'
#[test]
fn readiness_placeholder() { assert!(true); }
EOF
mkf "$ROOT/tests/reason_codes.rs" <<'EOF'
#[test]
fn reason_codes_placeholder() { assert!(true); }
EOF
mkf "$ROOT/tests/hashing_policy.rs" <<'EOF'
#[test]
fn hashing_policy_placeholder() { assert!(true); }
EOF
mkf "$ROOT/tests/vectors.rs" <<'EOF'
#[test]
fn vectors_placeholder() { assert!(true); }
EOF
mkf "$ROOT/tests/webhooks/github_ping.rs" <<'EOF'
#[test]
fn github_ping_placeholder() { assert!(true); }
EOF
mkf "$ROOT/tests/webhooks/stripe_v1.rs" <<'EOF'
#[test]
fn stripe_v1_placeholder() { assert!(true); }
EOF
mkf "$ROOT/tests/webhooks/slack_v0.rs" <<'EOF'
#[test]
fn slack_v0_placeholder() { assert!(true); }
EOF
touchf "$ROOT/tests/vectors/vectors.json"
mkf "$ROOT/tests/vectors/generator.rs" <<'EOF'
// generator placeholder
#[allow(dead_code)]
fn regenerate() {}
EOF

# --- benches ---
mkf "$ROOT/benches/hashing_bench.rs" <<'EOF'
#![allow(unused)]
fn main() {}
EOF

# --- fuzz ---
mkf "$ROOT/fuzz/Cargo.toml" <<'EOF'
[package]
name = "svc-interop2-fuzz"
version = "0.1.0"
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[workspace]
members = []

[[bin]]
name = "dto_parse"
path = "fuzz_targets/dto_parse.rs"
test = false
doc = false

[[bin]]
name = "webhook_sig"
path = "fuzz_targets/webhook_sig.rs"
test = false
doc = false

[[bin]]
name = "decompression"
path = "fuzz_targets/decompression.rs"
test = false
doc = false
EOF

mkf "$ROOT/fuzz/fuzz_targets/dto_parse.rs" <<'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|_data: &[u8]| {});
EOF
mkf "$ROOT/fuzz/fuzz_targets/webhook_sig.rs" <<'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|_data: &[u8]| {});
EOF
mkf "$ROOT/fuzz/fuzz_targets/decompression.rs" <<'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|_data: &[u8]| {});
EOF

# --- docs ---
mkf "$ROOT/docs/arch.mmd" <<'EOF'
flowchart LR
  A[Providers] -->|HTTP/Webhooks| G[svc-gateway]
  G --> B(svc-interop2)
  B --> P[svc-passport]
  B --> K[ron-kms]
  B --> I[svc-index]
  B --> S[svc-storage]
  B --> E[[Prometheus+Bus]]
  style B fill:#0b7285,stroke:#083344,color:#fff
EOF
mkf "$ROOT/docs/sequence.mmd" <<'EOF'
sequenceDiagram
  actor Provider
  participant G as svc-gateway
  participant S as svc-interop2
  Provider->>G: webhook (signed)
  G->>S: forward (limits/stream)
  S-->>Provider: JSON { reason, corr_id }
EOF
mkf "$ROOT/docs/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Idle
  Idle --> Running
  Running --> Brownout: err>1% or p95>500ms
  Brownout --> Running: recovered
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
EOF
mkf "$ROOT/docs/openapi/svc-interop.openapi.yaml" <<'EOF'
openapi: 3.0.3
info: { title: svc-interop2, version: 0.1.0 }
paths:
  /healthz:
    get: { responses: { "200": { description: OK } } }
EOF
mkf "$ROOT/docs/api-history/http/v1.yaml" <<'EOF'
# HTTP API snapshot v1 (placeholder)
EOF
mkf "$ROOT/docs/api-history/rust/v1.txt" <<'EOF'
# cargo public-api snapshot (placeholder)
EOF
for doc in IDB CONCURRENCY CONFIG INTEROP OBSERVABILITY PERFORMANCE QUANTUM RUNBOOK SECURITY; do
  mkf "$ROOT/docs/$doc.md" <<EOF
# $doc — svc-interop2
(placeholder; copy finalized content here)
EOF
done

# --- .github workflows (kept near crate for reference; repo-level CI lives at repo root) ---
mkf "$ROOT/.github/workflows/ci.yml" <<'EOF'
name: ci-svc-interop2
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy -p svc-interop2 -- -D warnings
      - run: cargo test -p svc-interop2
EOF
mkf "$ROOT/.github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files 'crates/svc-interop2/docs/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
EOF
mkf "$ROOT/.github/workflows/concurrency-guardrails.yml" <<'EOF'
name: concurrency-guardrails
on: [pull_request]
jobs:
  guardrails:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "loom/fuzz placeholders - enable when logic lands"
EOF

# --- ops ---
mkf "$ROOT/ops/alerts/prometheus-rules.yaml" <<'EOF'
groups:
- name: svc-interop2
  rules:
  - alert: HighP95Latency
    expr: histogram_quantile(0.95, sum by(le,route)(rate(request_latency_seconds_bucket[5m]))) > 0.12
    for: 10m
    labels: { severity: page }
    annotations: { summary: "p95 latency >120ms" }
EOF
mkf "$ROOT/ops/dashboards/svc-interop.json" <<'EOF'
{ "title": "svc-interop2", "panels": [] }
EOF

# --- scripts ---
mkf "$ROOT/scripts/dev-run.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=${RUST_LOG:-info}
export SVC_INTEROP_BIND_ADDR=${SVC_INTEROP_BIND_ADDR:-0.0.0.0:8080}
export SVC_INTEROP_METRICS_ADDR=${SVC_INTEROP_METRICS_ADDR:-127.0.0.1:9909}
echo "svc-interop2 scaffold - no service logic yet"
EOF
mkf "$ROOT/scripts/regen_vectors.rs" <<'EOF'
// regen vectors placeholder
fn main() { println!("regen vectors"); }
EOF
mkf "$ROOT/scripts/openapi_diff.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "openapi diff placeholder"
EOF
mkf "$ROOT/scripts/perf_mixed.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "perf mix placeholder"
EOF

# --- Makefile.toml ---
mkf "$ROOT/Makefile.toml" <<'EOF'
[tasks.lint]
command = "cargo"
args = ["clippy","-p","svc-interop2","--","-D","warnings"]

[tasks.test]
command = "cargo"
args = ["test","-p","svc-interop2"]
EOF

echo "Done. Scaffold created at $ROOT"
