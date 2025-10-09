#!/usr/bin/env bash
# Scaffolder for crates/svc-registry2 — structure only, no Rust logic.
set -euo pipefail

ROOT="crates/svc-registry2"

# --- helpers ---------------------------------------------------------------
mkd() { mkdir -p "$1"; }
mkf() { mkdir -p "$(dirname "$1")"; cat > "$1"; }
say() { printf '%s\n' "$*"; }

# --- start -----------------------------------------------------------------
say "Scaffolding $ROOT ..."

# Top-level dirs
mkd "$ROOT"
mkd "$ROOT/src"
mkd "$ROOT/src/config"
mkd "$ROOT/src/observability"
mkd "$ROOT/src/http/middleware"
mkd "$ROOT/src/auth"
mkd "$ROOT/src/interop"
mkd "$ROOT/src/governance"
mkd "$ROOT/src/pipeline"
mkd "$ROOT/src/storage"
mkd "$ROOT/src/bus"
mkd "$ROOT/src/readiness"
mkd "$ROOT/src/pq"
mkd "$ROOT/benches"
mkd "$ROOT/tests/vectors"
mkd "$ROOT/fuzz/fuzz_targets"
mkd "$ROOT/fuzz/corpus"
mkd "$ROOT/docs/openapi"
mkd "$ROOT/docs/api-history"
mkd "$ROOT/scripts"
mkd "$ROOT/.github/workflows"

# --- top-level files -------------------------------------------------------

mkf "$ROOT/Cargo.toml" <<'EOF'
[package]
name = "svc-registry2"
version = "0.0.0"
edition = "2021"
publish = false
license = "MIT OR Apache-2.0"
description = "RustyOnions service & node registry (scaffold only)."
repository = "https://example.invalid/RustyOnions"

[lib]
path = "src/lib.rs"

[[bin]]
name = "svc-registry2"
path = "src/main.rs"

[features]
default = ["tokio", "serde"]
tls = []
kameo = []
pq = []
pq-verify = []

[dependencies]
# Workspace pins expected; keep empty to avoid drift. Add when implementing.

[dev-dependencies]
# Intentionally empty for scaffold.

[build-dependencies]
# Intentionally empty for scaffold.
EOF

mkf "$ROOT/README.md" <<'EOF'
# svc-registry2 (scaffold)

This is a structure-only scaffold mirroring the `svc-registry` crate layout:
- Small, modular files
- Clear seams per concern (config, governance, pipeline, storage, http, obs)
- No Rust logic in this scaffold — all files are stubs with module docs

See the primary README in `svc-registry` for the full spec and invariants.
EOF

mkf "$ROOT/LICENSE-MIT" <<'EOF'
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files to deal in the Software
without restriction... (scaffold placeholder)
EOF

mkf "$ROOT/LICENSE-APACHE" <<'EOF'
Apache License, Version 2.0 (scaffold placeholder)
EOF

mkf "$ROOT/.gitignore" <<'EOF'
/target
**/*.rs.bk
.DS_Store
.env
_data
_state
EOF

mkf "$ROOT/.env.example" <<'EOF'
BIND=127.0.0.1:9402
METRICS_ADDR=127.0.0.1:0
REGISTRY_DB=./_data
SIG_THRESHOLD=2
SIGNER_ALLOWLIST=key1,key2
AMNESIA=false
LOG_LEVEL=info
EOF

mkf "$ROOT/Config.toml.sample" <<'EOF'
# svc-registry2 sample config (scaffold)
[binding]
bind = "127.0.0.1:9402"

[metrics]
addr = "127.0.0.1:0"

[storage]
path = "./_data"
backend = "sled" # or "sqlite"

[governance]
sig_threshold = 2
signer_allowlist = ["key1","key2"]
EOF

mkf "$ROOT/build.rs" <<'EOF'
// Scaffold build script placeholder (embeds build info in future).
fn main() {}
EOF

# --- src root --------------------------------------------------------------

mkf "$ROOT/src/main.rs" <<'EOF'
/*!
Binary entrypoint — scaffold only.

Flow (when implemented):
1) Load & validate config
2) Initialize observability (metrics/tracing/logging)
3) Wire HTTP routes, pipeline, storage, bus, readiness
4) Block on graceful shutdown
*/
fn main() {
    // Intentionally empty; no logic in scaffold.
}
EOF

mkf "$ROOT/src/lib.rs" <<'EOF'
/*! Library surface (scaffold)
Re-exports will be minimal when implemented.
*/
EOF

mkf "$ROOT/src/build_info.rs" <<'EOF'
/*! Build info surface (scaffold) */
EOF

mkf "$ROOT/src/error.rs" <<'EOF'
/*! Error taxonomy holder (scaffold).
Wire-layer envelope will map HTTP->JSON with {code,message,hint,retryable,...}.
*/
EOF

mkf "$ROOT/src/result.rs" <<'EOF'
/*! Local Result alias (scaffold) */
EOF

mkf "$ROOT/src/shutdown.rs" <<'EOF'
/*! Cooperative shutdown helpers (scaffold) */
EOF

# --- src/config ------------------------------------------------------------

mkf "$ROOT/src/config/mod.rs" <<'EOF'
/*! Config facade (scaffold) */
EOF

mkf "$ROOT/src/config/model.rs" <<'EOF'
/*! Strongly-typed config model (scaffold) */
EOF

mkf "$ROOT/src/config/load.rs" <<'EOF'
/*! Config loader/merger (scaffold) */
EOF

mkf "$ROOT/src/config/validate.rs" <<'EOF'
/*! Config validation (scaffold) */
EOF

mkf "$ROOT/src/config/reload.rs" <<'EOF'
/*! Hot reload plumbing (scaffold) */
EOF

# --- src/observability -----------------------------------------------------

mkf "$ROOT/src/observability/mod.rs" <<'EOF'
/*! Observability facade (scaffold) */
EOF

mkf "$ROOT/src/observability/metrics.rs" <<'EOF'
/*! Prometheus metrics registration (scaffold) */
EOF

mkf "$ROOT/src/observability/tracing.rs" <<'EOF'
/*! Tracing/OpenTelemetry setup (scaffold) */
EOF

mkf "$ROOT/src/observability/logging.rs" <<'EOF'
/*! Structured logging config (scaffold) */
EOF

mkf "$ROOT/src/observability/endpoints.rs" <<'EOF'
/*! /healthz, /readyz, /version (scaffold) */
EOF

# --- src/http --------------------------------------------------------------

mkf "$ROOT/src/http/mod.rs" <<'EOF'
/*! HTTP facade (scaffold) */
EOF

mkf "$ROOT/src/http/routes.rs" <<'EOF'
/*! Route table for documented endpoints (scaffold) */
EOF

mkf "$ROOT/src/http/sse.rs" <<'EOF'
/*! SSE stream support (scaffold) */
EOF

mkf "$ROOT/src/http/responses.rs" <<'EOF'
/*! JSON responders incl. error envelope (scaffold) */
EOF

mkf "$ROOT/src/http/middleware/limits.rs" <<'EOF'
/*! Body/ratio caps middleware (scaffold) */
EOF

mkf "$ROOT/src/http/middleware/timeouts.rs" <<'EOF'
/*! IO timeouts middleware (scaffold) */
EOF

mkf "$ROOT/src/http/middleware/corr_id.rs" <<'EOF'
/*! Correlation ID middleware (scaffold) */
EOF

mkf "$ROOT/src/http/middleware/auth.rs" <<'EOF'
/*! Capability token middleware (scaffold) */
EOF

# --- src/auth --------------------------------------------------------------

mkf "$ROOT/src/auth/macaroon.rs" <<'EOF'
/*! Macaroon verifier (scaffold) */
EOF

mkf "$ROOT/src/auth/uds.rs" <<'EOF'
/*! UDS peer credential checks (scaffold) */
EOF

# --- src/interop -----------------------------------------------------------

mkf "$ROOT/src/interop/dto.rs" <<'EOF'
/*! ron-proto DTO glue (scaffold) */
EOF

mkf "$ROOT/src/interop/event_shapes.rs" <<'EOF'
/*! Bus event JSON shapes (scaffold) */
EOF

mkf "$ROOT/src/interop/openapi_stub.rs" <<'EOF'
/*! OpenAPI sync check (scaffold) */
EOF

# --- src/governance --------------------------------------------------------

mkf "$ROOT/src/governance/mod.rs" <<'EOF'
/*! Governance facade (scaffold) */
EOF

mkf "$ROOT/src/governance/quorum.rs" <<'EOF'
/*! M-of-N quorum evaluation (scaffold) */
EOF

mkf "$ROOT/src/governance/approvals.rs" <<'EOF'
/*! Approval verification (scaffold) */
EOF

mkf "$ROOT/src/governance/supersede.rs" <<'EOF'
/*! Supersede mechanics (scaffold) */
EOF

mkf "$ROOT/src/governance/signer_set.rs" <<'EOF'
/*! Signer set lifecycle (scaffold) */
EOF

# --- src/pipeline ----------------------------------------------------------

mkf "$ROOT/src/pipeline/mod.rs" <<'EOF'
/*! Pipeline wiring (scaffold) */
EOF

mkf "$ROOT/src/pipeline/propose.rs" <<'EOF'
/*! Accept proposal, enqueue (scaffold) */
EOF

mkf "$ROOT/src/pipeline/approve.rs" <<'EOF'
/*! Add approval (scaffold) */
EOF

mkf "$ROOT/src/pipeline/commit.rs" <<'EOF'
/*! Single-writer commit path (scaffold) */
EOF

mkf "$ROOT/src/pipeline/bus_publish.rs" <<'EOF'
/*! Publish events non-blocking (scaffold) */
EOF

mkf "$ROOT/src/pipeline/checkpoint.rs" <<'EOF'
/*! Periodic checkpoints (scaffold) */
EOF

mkf "$ROOT/src/pipeline/retention.rs" <<'EOF'
/*! Retention/pruning (scaffold) */
EOF

mkf "$ROOT/src/pipeline/deep_verify.rs" <<'EOF'
/*! Background integrity verification (scaffold) */
EOF

# --- src/storage -----------------------------------------------------------

mkf "$ROOT/src/storage/mod.rs" <<'EOF'
/*! Storage backend facade (scaffold) */
EOF

mkf "$ROOT/src/storage/types.rs" <<'EOF'
/*! RegistryStore traits (scaffold) */
EOF

mkf "$ROOT/src/storage/head.rs" <<'EOF'
/*! HEAD snapshot & CAS (scaffold) */
EOF

mkf "$ROOT/src/storage/log.rs" <<'EOF'
/*! Append-only log (scaffold) */
EOF

mkf "$ROOT/src/storage/checkpoint.rs" <<'EOF'
/*! Durable checkpoints (scaffold) */
EOF

mkf "$ROOT/src/storage/sled_store.rs" <<'EOF'
/*! Sled backend adapter (scaffold) */
EOF

mkf "$ROOT/src/storage/sqlite_store.rs" <<'EOF'
/*! SQLite backend adapter (scaffold) */
EOF

# --- src/bus ---------------------------------------------------------------

mkf "$ROOT/src/bus/mod.rs" <<'EOF'
/*! Bus facade (scaffold) */
EOF

mkf "$ROOT/src/bus/events.rs" <<'EOF'
/*! Event constructors (scaffold) */
EOF

# --- src/readiness ---------------------------------------------------------

mkf "$ROOT/src/readiness/gate.rs" <<'EOF'
/*! Readiness aggregator (scaffold) */
EOF

# --- src/pq ---------------------------------------------------------------

mkf "$ROOT/src/pq/mod.rs" <<'EOF'
/*! PQ posture facade (scaffold) */
EOF

mkf "$ROOT/src/pq/verify_dilithium.rs" <<'EOF'
/*! Dilithium verify adapter (scaffold) */
EOF

mkf "$ROOT/src/pq/verify_falcon.rs" <<'EOF'
/*! Falcon verify adapter (scaffold) */
EOF

mkf "$ROOT/src/pq/policy.rs" <<'EOF'
/*! Mixed-quorum policy (scaffold) */
EOF

# --- benches ---------------------------------------------------------------

mkf "$ROOT/benches/blake3_payload.rs" <<'EOF'
// Criterion bench placeholder (scaffold)
fn main() {}
EOF

mkf "$ROOT/benches/verify_approvals.rs" <<'EOF'
// Criterion bench placeholder (scaffold)
fn main() {}
EOF

# --- tests -----------------------------------------------------------------

mkf "$ROOT/tests/http_contract.rs" <<'EOF'
// Contract test placeholder (scaffold)
#[test]
fn http_contract_scaffold() {
    assert!(true);
}
EOF

mkf "$ROOT/tests/invariants.rs" <<'EOF'
// Property test placeholder (scaffold)
#[test]
fn invariants_scaffold() {
    assert!(true);
}
EOF

mkf "$ROOT/tests/concurrency_loom.rs" <<'EOF'
// Loom model placeholder (scaffold)
#[test]
fn loom_scaffold() {
    assert!(true);
}
EOF

mkf "$ROOT/tests/chaos_ready.rs" <<'EOF'
// Chaos readiness test placeholder (scaffold)
#[test]
fn chaos_ready_scaffold() {
    assert!(true);
}
EOF

mkf "$ROOT/tests/vectors/proposal.json" <<'EOF'
{ "scaffold": "proposal vector placeholder" }
EOF

mkf "$ROOT/tests/vectors/approvals_ed25519.json" <<'EOF'
{ "scaffold": "ed25519 approvals vector placeholder" }
EOF

mkf "$ROOT/tests/vectors/approvals_pq_mixed.json" <<'EOF'
{ "scaffold": "pq mixed approvals vector placeholder" }
EOF

mkf "$ROOT/tests/vectors/descriptor_set_v1.json" <<'EOF'
{ "scaffold": "descriptor set v1 placeholder" }
EOF

# --- fuzz ------------------------------------------------------------------

mkf "$ROOT/fuzz/fuzz_targets/dto_decode.rs" <<'EOF'
// cargo-fuzz target placeholder (scaffold)
fn main() {}
EOF

mkf "$ROOT/fuzz/fuzz_targets/approval_payload.rs" <<'EOF'
// cargo-fuzz target placeholder (scaffold)
fn main() {}
EOF

# --- docs ------------------------------------------------------------------

mkf "$ROOT/docs/arch.mmd" <<'EOF'
%% Minimal, valid mermaid (scaffold)
flowchart LR
  A[Ops] -->|Propose+Sign| B(svc-registry2)
  C[Services] -->|GET /v1/registry/*| B
  B -->|/metrics| D[[Prometheus]]
  style B fill:#0b7285,stroke:#083344,color:#fff
EOF

mkf "$ROOT/docs/sequence.mmd" <<'EOF'
sequenceDiagram
  participant Ops
  participant R as svc-registry2
  participant Bus
  Ops->>R: POST /proposals
  R-->>Ops: 202 {proposal_id}
  R->>Bus: RegistryEvent::Published
  Bus-->>Ops: ack
EOF

mkf "$ROOT/docs/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Idle
  Idle --> Verify: proposal
  Verify --> Apply: quorum_ok
  Verify --> Idle: invalid
  Apply --> Publish: append_ok
  Publish --> Idle
EOF

mkf "$ROOT/docs/openapi/registry.yaml" <<'EOF'
openapi: "3.0.0"
info:
  title: svc-registry2 (scaffold)
  version: 0.0.0
paths:
  /v1/registry/version:
    get:
      responses:
        "200":
          description: ok
EOF

mkf "$ROOT/docs/api-history/svc-registry-1.0.0.txt" <<'EOF'
Public Rust surface history placeholder (scaffold)
EOF

# --- scripts ---------------------------------------------------------------

mkf "$ROOT/scripts/run_local.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Run-local scaffold for svc-registry2. (No binary logic yet.)"
EOF
chmod +x "$ROOT/scripts/run_local.sh"

mkf "$ROOT/scripts/render_mermaid.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Rendering mermaid (requires mmdc) ..."
for f in "$(dirname "$0")"/../docs/*.mmd; do
  [ -f "$f" ] || continue
  out="${f%.mmd}.svg"
  mmdc -i "$f" -o "$out"
  echo "Rendered: $out"
done
EOF
chmod +x "$ROOT/scripts/render_mermaid.sh"

mkf "$ROOT/scripts/perf_sweep.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Perf sweep scaffold — no benches wired yet."
EOF
chmod +x "$ROOT/scripts/perf_sweep.sh"

# --- workflows -------------------------------------------------------------

mkf "$ROOT/.github/workflows/ci.yml" <<'EOF'
name: ci (scaffold)
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "CI scaffold (no build steps yet)."
EOF

mkf "$ROOT/.github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid (scaffold)
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files 'crates/svc-registry2/docs/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
EOF

mkf "$ROOT/.github/workflows/perf.yml" <<'EOF'
name: perf (scaffold)
on:
  schedule:
    - cron: "0 3 * * *"
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Nightly perf scaffold."
EOF

say "Scaffold complete."
