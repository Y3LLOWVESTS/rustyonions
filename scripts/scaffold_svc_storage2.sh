#!/usr/bin/env bash
set -euo pipefail

# Scaffold svc-storage2 (code-free, modular file tree)
# Usage: bash scripts/scaffold_svc_storage2.sh

TARGET="crates/svc-storage2"

if [ -d "$TARGET" ] && find "$TARGET" -mindepth 1 -print -quit 2>/dev/null | grep -q .; then
  echo "Error: $TARGET already exists and is not empty. Remove or choose another path."
  exit 1
fi

echo "Creating directory tree under $TARGET ..."

# Basic directories
mkdir -p "$TARGET"
mkdir -p "$TARGET/.github/workflows"
mkdir -p "$TARGET/configs/profiles"
mkdir -p "$TARGET/docs/specs/vectors"
mkdir -p "$TARGET/docs/openapi/history"
mkdir -p "$TARGET/docs/api-history/svc-storage"
mkdir -p "$TARGET/src/tls"
mkdir -p "$TARGET/src/uds"
mkdir -p "$TARGET/src/http/routes"
mkdir -p "$TARGET/src/storage"
mkdir -p "$TARGET/src/auth"
mkdir -p "$TARGET/src/policy"
mkdir -p "$TARGET/benches"
mkdir -p "$TARGET/testing/integration"
mkdir -p "$TARGET/testing/performance/fixtures"
mkdir -p "$TARGET/testing/performance/scripts"
mkdir -p "$TARGET/testing/fuzz/range_fuzz"
mkdir -p "$TARGET/testing/fuzz/decomp_fuzz"
mkdir -p "$TARGET/testing/fuzz/addr_fuzz"

# ---------- Root files ----------
cat > "$TARGET/Cargo.toml" <<'EOF'
[package]
name = "svc-storage2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false
description = "RustyOnions storage service - code-free scaffold (CAS over HTTP)."

[features]
amnesia = []
pq-hybrid = []
uds = []

[dependencies]
# Intentionally minimal; real pins come from workspace.
EOF

cat > "$TARGET/README.md" <<'EOF'
# svc-storage2

Role: storage service (CAS over HTTP).
Contract: `GET/HEAD /o/{b3}`, `PUT /o/{b3}`, `POST /o`.
Invariants: 1 MiB body cap, ~64 KiB chunks, safe decompression ≤10×, BLAKE3 identity -> `ETag: "b3:<hex>"`.
This is a code-free scaffold; implementation arrives later.
EOF

cat > "$TARGET/CHANGELOG.md" <<'EOF'
# Changelog — svc-storage2

## [0.1.0]
- Initial code-free scaffold: structure, docs, CI, config, tests skeleton.
EOF

cat > "$TARGET/LICENSE-APACHE" <<'EOF'
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/
EOF

cat > "$TARGET/LICENSE-MIT" <<'EOF'
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy...
EOF

cat > "$TARGET/.gitignore" <<'EOF'
/target
**/*.swp
.DS_Store
coverage/
EOF

cat > "$TARGET/.editorconfig" <<'EOF'
root = true

[*]
charset = utf-8
end_of_line = lf
indent_style = space
indent_size = 2
insert_final_newline = true
trim_trailing_whitespace = true
EOF

cat > "$TARGET/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["rustfmt", "clippy"]
EOF

# ---------- GitHub Workflows ----------
cat > "$TARGET/.github/workflows/ci.yml" <<'EOF'
name: CI
on: [push, pull_request]
jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
          components: rustfmt, clippy
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all --locked
EOF

cat > "$TARGET/.github/workflows/contract-apis.yml" <<'EOF'
name: Contract APIs
on: [push, pull_request]
jobs:
  openapi-diff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check OpenAPI present
        run: test -f crates/svc-storage2/docs/openapi/svc-storage.yaml
EOF

cat > "$TARGET/.github/workflows/perf.yml" <<'EOF'
name: Perf
on:
  workflow_dispatch: {}
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Perf rig placeholder (range-heavy profile)."
EOF

cat > "$TARGET/.github/workflows/concurrency-guardrails.yml" <<'EOF'
name: Concurrency Guardrails
on:
  workflow_dispatch: {}
jobs:
  guardrails:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Loom/sanitizers hooks placeholder."
EOF

cat > "$TARGET/.github/workflows/redteam-fuzz.yml" <<'EOF'
name: Redteam Fuzz
on:
  workflow_dispatch: {}
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Fuzz targets placeholder (range/decomp/addr)."
EOF

cat > "$TARGET/.github/workflows/render-mermaid.yml" <<'EOF'
name: Render Mermaid
on:
  workflow_dispatch: {}
jobs:
  render:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Render arch/sequence/state diagrams placeholder."
EOF

cat > "$TARGET/.github/workflows/quantum.yml" <<'EOF'
name: Quantum Matrix
on:
  workflow_dispatch: {}
jobs:
  pq-matrix:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        feature: ["", "pq-hybrid"]
    steps:
      - uses: actions/checkout@v4
      - run: echo "PQ matrix smoke: ${{ matrix.feature }}"
EOF

cat > "$TARGET/.github/workflows/coverage.yml" <<'EOF'
name: Coverage
on:
  workflow_dispatch: {}
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Coverage gate ≥85% placeholder."
EOF

# ---------- Configs ----------
cat > "$TARGET/configs/svc-storage.example.toml" <<'EOF'
# svc-storage2 config (example)
listen_addr = "0.0.0.0:8080"
max_body_bytes = "1MiB"
chunk_bytes = "64KiB"
safe_decompress_ratio = 10
amnesia = false
data_dirs = ["./data/a", "./data/b"]
hedge_delay_ms = 15
EOF

cat > "$TARGET/configs/pq-hybrid.toml" <<'EOF'
# Enable PQ-hybrid transport/auth envelopes
pq_hybrid = true
EOF

cat > "$TARGET/configs/validate.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Validating TOML configs (lightweight placeholder)..."
for f in *.toml profiles/*.toml 2>/dev/null; do
  if [ -f "$f" ]; then
    echo "OK: $f"
  fi
done
EOF
chmod +x "$TARGET/configs/validate.sh"

cat > "$TARGET/configs/profiles/micronode.toml" <<'EOF'
# Micronode defaults: single-tenant, amnesia ON
amnesia = true
chunk_bytes = "64KiB"
EOF

cat > "$TARGET/configs/profiles/macronode.toml" <<'EOF'
# Macronode defaults: persistent, replication ready
amnesia = false
chunk_bytes = "64KiB"
EOF

# ---------- Docs ----------
cat > "$TARGET/docs/ALL_DOCS.md" <<'EOF'
# ALL_DOCS — svc-storage2
Combined single source of truth. (Scaffold placeholder)
EOF

cat > "$TARGET/docs/README.linkmap.md" <<'EOF'
# Link Map
- INTEROP.md: HTTP contract
- CONFIG.md: knobs & defaults
- TESTS.md: gates & proofs
EOF

for f in API CONFIG GOVERNANCE IDB INTEROP OBSERVABILITY PERFORMANCE QUANTUM RUNBOOK SECURITY TESTS FACETS ECONOMICS; do
  cat > "$TARGET/docs/$f.md" <<EOF
# $f — svc-storage2
Scaffold placeholder. See ALL_DOCS.md for canon.
EOF
done

cat > "$TARGET/docs/arch.mmd" <<'EOF'
flowchart LR
  Client -->|HTTP| Gateway --> Storage[(svc-storage2)]
  Storage -->|CAS| Disk[(Data Dirs)]
EOF

cat > "$TARGET/docs/sequence.mmd" <<'EOF'
sequenceDiagram
  participant C as Client
  participant S as svc-storage2
  C->>S: PUT /o/{b3}
  S-->>C: 200 {"address":"b3:<hex>"}
EOF

cat > "$TARGET/docs/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Ready
  Ready --> Degraded: Spill Risk / Policy
  Degraded --> Ready: Recovered
EOF

cat > "$TARGET/docs/specs/OAP-1.md" <<'EOF'
# OAP/1 Notes (svc-storage2)
Frame max: 1 MiB. Storage stream chunk: ~64 KiB. (Scaffold)
EOF

cat > "$TARGET/docs/specs/vectors/chunking-64k.txt" <<'EOF'
Vector: 64 KiB chunk boundaries (scaffold)
EOF

cat > "$TARGET/docs/specs/vectors/frame-1MiB.txt" <<'EOF'
Vector: 1 MiB frame cap (scaffold)
EOF

cat > "$TARGET/docs/specs/vectors/range-requests.txt" <<'EOF'
Vector: byte-range request/response examples (scaffold)
EOF

cat > "$TARGET/docs/openapi/svc-storage.yaml" <<'EOF'
openapi: "3.0.3"
info:
  title: svc-storage2
  version: 0.1.0
paths:
  /o/{b3}:
    get: { summary: "GET object (byte-range capable)" }
    head: { summary: "HEAD object" }
    put: { summary: "PUT object (idempotent by address)" }
  /o:
    post: { summary: "POST to compute address" }
  /metrics:
    get: { summary: "Prometheus metrics" }
  /healthz:
    get: { summary: "Liveness" }
  /readyz:
    get: { summary: "Readiness" }
  /version:
    get: { summary: "Build & feature booleans" }
EOF

cat > "$TARGET/docs/openapi/history/v0.1.0-http.json" <<'EOF'
{ "note": "Initial HTTP contract snapshot (scaffold)" }
EOF

cat > "$TARGET/docs/api-history/svc-storage/v0.1.0-libapi.txt" <<'EOF'
# Rust API surface snapshot
# Service exposes HTTP contract; Rust API is internal. (scaffold)
EOF

# ---------- Source skeleton ----------
cat > "$TARGET/src/main.rs" <<'EOF'
// svc-storage2 main (scaffold) — compose config->metrics->transports->routes
fn main() {
    println!("svc-storage2 scaffold (no runtime yet)");
}
EOF

cat > "$TARGET/src/prelude.rs" <<'EOF'
// Shared imports/aliases (scaffold)
EOF

cat > "$TARGET/src/version.rs" <<'EOF'
// /version responder (scaffold): expose booleans like amnesia, pq-hybrid (non-fingerprintable)
EOF

cat > "$TARGET/src/readiness.rs" <<'EOF'
// Readiness DAG (scaffold): fail-closed for writes under degradation
EOF

cat > "$TARGET/src/errors.rs" <<'EOF'
// Structured error envelope + HTTP mapping (scaffold)
EOF

cat > "$TARGET/src/types.rs" <<'EOF'
// Local DTOs/types for handlers/metrics (scaffold)
EOF

cat > "$TARGET/src/config.rs" <<'EOF'
// Env + TOML merge; validation for caps, chunks, safe decompression (scaffold)
EOF

cat > "$TARGET/src/metrics.rs" <<'EOF'
// Golden metrics + storage gauges (rf_target/observed, integrity, rejects) (scaffold)
EOF

cat > "$TARGET/src/amnesia.rs" <<'EOF'
// Enforce amnesia mode; flip readiness if spill would occur (scaffold)
EOF

cat > "$TARGET/src/bus.rs" <<'EOF'
// Kernel bus events (Health/Shutdown/ConfigUpdated) + economics signals (scaffold)
EOF

# TLS / UDS
cat > "$TARGET/src/tls/mod.rs" <<'EOF'
// TLS module (scaffold)
EOF

cat > "$TARGET/src/tls/server_config.rs" <<'EOF'
// Build tokio_rustls::rustls::ServerConfig (TLS 1.3) (scaffold)
EOF

cat > "$TARGET/src/tls/pq.rs" <<'EOF'
// Feature-gated PQ-hybrid suite selection (design stub, scaffold)
EOF

cat > "$TARGET/src/uds/mod.rs" <<'EOF'
// UDS module (scaffold)
EOF

cat > "$TARGET/src/uds/server.rs" <<'EOF'
// UDS listener wiring (scaffold)
EOF

cat > "$TARGET/src/uds/peercred.rs" <<'EOF'
// SO_PEERCRED extraction/validation (scaffold)
EOF

# HTTP
cat > "$TARGET/src/http/mod.rs" <<'EOF'
// HTTP surface wiring (scaffold)
EOF

cat > "$TARGET/src/http/server.rs" <<'EOF'
// Axum/Hyper composition; timeouts, concurrency caps, graceful shutdown (scaffold)
EOF

cat > "$TARGET/src/http/error.rs" <<'EOF'
// Map internal errors -> HTTP status + envelope (scaffold)
EOF

cat > "$TARGET/src/http/extractors.rs" <<'EOF'
// Range parsing; capability token extraction (scaffold)
EOF

cat > "$TARGET/src/http/middleware.rs" <<'EOF'
// 1 MiB body cap; safe decompression ≤10×; inflight/RPS ceilings (scaffold)
EOF

cat > "$TARGET/src/http/routes/mod.rs" <<'EOF'
// Route registry (scaffold)
EOF

cat > "$TARGET/src/http/routes/get_object.rs" <<'EOF'
// Route: GET /o/{b3} (scaffold)
EOF

cat > "$TARGET/src/http/routes/head_object.rs" <<'EOF'
// Route: HEAD /o/{b3} (scaffold)
EOF

cat > "$TARGET/src/http/routes/put_object.rs" <<'EOF'
// Route: PUT /o/{b3} (scaffold)
EOF

cat > "$TARGET/src/http/routes/post_object.rs" <<'EOF'
// Route: POST /o (scaffold)
EOF

cat > "$TARGET/src/http/routes/health.rs" <<'EOF'
// Route: /healthz (scaffold)
EOF

cat > "$TARGET/src/http/routes/ready.rs" <<'EOF'
// Route: /readyz (scaffold)
EOF

cat > "$TARGET/src/http/routes/metrics.rs" <<'EOF'
// Route: /metrics (scaffold)
EOF

cat > "$TARGET/src/http/routes/version.rs" <<'EOF'
// Route: /version (scaffold)
EOF

# Storage engine
cat > "$TARGET/src/storage/mod.rs" <<'EOF'
// Storage surface (trait hooks; scaffold)
EOF

cat > "$TARGET/src/storage/cas.rs" <<'EOF'
// Storage module: cas (scaffold)
EOF

cat > "$TARGET/src/storage/fs.rs" <<'EOF'
// Storage module: fs (scaffold)
EOF

cat > "$TARGET/src/storage/io.rs" <<'EOF'
// Storage module: io (scaffold)
EOF

cat > "$TARGET/src/storage/compression.rs" <<'EOF'
// Storage module: compression (scaffold)
EOF

cat > "$TARGET/src/storage/cache.rs" <<'EOF'
// Storage module: cache (scaffold)
EOF

cat > "$TARGET/src/storage/placement.rs" <<'EOF'
// Storage module: placement (scaffold)
EOF

cat > "$TARGET/src/storage/replication.rs" <<'EOF'
// Storage module: replication (scaffold)
EOF

cat > "$TARGET/src/storage/erasure.rs" <<'EOF'
// Storage module: erasure (scaffold)
EOF

cat > "$TARGET/src/storage/repair.rs" <<'EOF'
// Storage module: repair (scaffold)
EOF

cat > "$TARGET/src/storage/hedged.rs" <<'EOF'
// Storage module: hedged (scaffold)
EOF

cat > "$TARGET/src/storage/pq_envelope.rs" <<'EOF'
// Storage module: pq_envelope (scaffold)
EOF

# Auth / Policy
cat > "$TARGET/src/auth/mod.rs" <<'EOF'
// Auth module (scaffold)
EOF

cat > "$TARGET/src/auth/macaroon.rs" <<'EOF'
// Capability verification for writes (scaffold)
EOF

cat > "$TARGET/src/policy/mod.rs" <<'EOF'
// Policy module (scaffold)
EOF

cat > "$TARGET/src/policy/quotas.rs" <<'EOF'
// Policy module: quotas (scaffold)
EOF

cat > "$TARGET/src/policy/residency.rs" <<'EOF'
// Policy module: residency (scaffold)
EOF

cat > "$TARGET/src/policy/economics.rs" <<'EOF'
// Policy module: economics (read-only settlement signals; scaffold)
EOF

# ---------- Benches ----------
cat > "$TARGET/benches/read_path.rs" <<'EOF'
// Criterion bench: read path p95 (scaffold)
EOF

cat > "$TARGET/benches/write_path.rs" <<'EOF'
// Criterion bench: write path p95 (scaffold)
EOF

cat > "$TARGET/benches/etag_range.rs" <<'EOF'
// Criterion bench: conditional GET + Range (scaffold)
EOF

# ---------- Testing ----------
cat > "$TARGET/testing/integration/http_get_head_put.rs" <<'EOF'
// Integration: GET/HEAD/PUT/POST round-trips (scaffold)
EOF

cat > "$TARGET/testing/integration/range_tests.rs" <<'EOF'
// Integration: Range semantics, 416 taxonomy (scaffold)
EOF

cat > "$TARGET/testing/integration/error_caps.rs" <<'EOF'
// Integration: 413/429/503 + decompression guard (scaffold)
EOF

cat > "$TARGET/testing/integration/profile_matrix.rs" <<'EOF'
// Runs test suites against micronode/macronode profiles (scaffold)
EOF

cat > "$TARGET/testing/performance/fixtures/zip_bomb.bin" <<'EOF'
ZIP_BOMB_PLACEHOLDER
EOF

cat > "$TARGET/testing/performance/scripts/range.lua" <<'EOF'
-- wrk range profile (scaffold)
EOF

cat > "$TARGET/testing/performance/scripts/media_facet.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Media facet load profile placeholder"
EOF
chmod +x "$TARGET/testing/performance/scripts/media_facet.sh"

cat > "$TARGET/testing/performance/run_load.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Run perf load placeholder"
EOF
chmod +x "$TARGET/testing/performance/run_load.sh"

cat > "$TARGET/testing/performance/compare_baselines.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Compare perf baselines placeholder"
EOF
chmod +x "$TARGET/testing/performance/compare_baselines.sh"

cat > "$TARGET/testing/fuzz/range_fuzz/README.md" <<'EOF'
# range_fuzz
Fuzz target scaffold.
EOF

cat > "$TARGET/testing/fuzz/decomp_fuzz/README.md" <<'EOF'
# decomp_fuzz
Fuzz target scaffold.
EOF

cat > "$TARGET/testing/fuzz/addr_fuzz/README.md" <<'EOF'
# addr_fuzz
Fuzz target scaffold.
EOF

echo "Done. Scaffold created at $TARGET"
