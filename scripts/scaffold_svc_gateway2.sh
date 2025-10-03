
#!/usr/bin/env bash
# Scaffolds the svc-gateway2 crate structure with minimal placeholder files.
# - No implementation code; only short role descriptions.
# - Safe for macOS bash (avoids readarray/mapfile).
# - Idempotent: re-running will not overwrite existing files unless noted.

set -euo pipefail

ROOT="svc-gateway2"

make_dir() {
  local d="$1"
  mkdir -p "$ROOT/$d"
}

make_file() {
  # create file only if it doesn't exist
  local f="$ROOT/$1"
  if [ -e "$f" ]; then
    return 0
  fi
  mkdir -p "$(dirname "$f")"
  cat > "$f"
}

note() {
  printf "%s\n" "$1"
}

note "Creating directory tree under ./$ROOT ..."

# --- Directories ---
while IFS= read -r d; do
  [ -z "$d" ] && continue
  make_dir "$d"
done <<'DIRS'
.github/workflows
configs
docs
docs/diagrams
docs/dashboards
scripts
examples
benches
tests/vectors
tests/integration
fuzz
fuzz/targets
src
src/cli
src/config
src/observability
src/headers
src/tls
src/pq
src/readiness
src/policy
src/layers
src/admission
src/routes
src/forward
DIRS

# --- Top-level files ---
make_file "Cargo.toml" <<'EOF'
# Cargo.toml — svc-gateway2
# Role: service crate manifest (features: tls, pq, cli, econ). Keep deps minimal & pinned.
[package]
name = "svc-gateway2"
version = "0.0.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[features]
tls = []
pq = []
cli = []
econ = []

[dependencies]
# Intentionally empty in scaffold: add real deps during implementation.

[dev-dependencies]
EOF

make_file "README.md" <<'EOF'
# svc-gateway2

> Role: Ingress gateway (edge safety first: caps → admission → routes → forward).  
> Status: scaffold only — no implementation code yet.

This crate was scaffolded with small, single-purpose modules and docs aligned to RustyOnions canon.
EOF

make_file "CHANGELOG.md" <<'EOF'
# Changelog — svc-gateway2
All notable changes to this crate will be documented here (SemVer-aligned).
EOF

make_file "LICENSE-MIT" <<'EOF'
MIT License (placeholder). Replace with the project’s standard MIT text.
EOF

make_file "LICENSE-APACHE" <<'EOF'
Apache-2.0 License (placeholder). Replace with the project’s standard Apache-2.0 text.
EOF

make_file ".gitignore" <<'EOF'
/target
**/*.rs.bk
.DS_Store
.env
EOF

make_file ".editorconfig" <<'EOF'
root = true
[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 2
trim_trailing_whitespace = true
EOF

# --- CI workflows ---
make_file ".github/workflows/ci.yml" <<'EOF'
name: ci
on: [push, pull_request]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: fmt
        run: cargo fmt --all -- --check
      - name: clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
      - name: test
        run: cargo test --all-features
      - name: deny (licenses/advisories)
        run: echo "placeholder — add cargo-deny later"
EOF

make_file ".github/workflows/api-stability.yml" <<'EOF'
name: api-stability
on:
  pull_request:
    paths:
      - "src/**"
      - "Cargo.toml"
jobs:
  api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: public api (placeholder)
        run: echo "wire cargo-public-api / semver-checks here"
EOF

make_file ".github/workflows/fuzz-nightly.yml" <<'EOF'
name: fuzz-nightly
on:
  schedule:
    - cron: "0 3 * * *"
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: fuzz (placeholder)
        run: echo "wire cargo-fuzz here"
EOF

make_file ".github/workflows/perf-guard.yml" <<'EOF'
name: perf-guard
on:
  pull_request:
    paths:
      - "benches/**"
jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: benches (placeholder)
        run: echo "run benches, compare benches/baseline.json"
EOF

# --- Configs ---
make_file "configs/svc-gateway.sample.toml" <<'EOF'
# Sample config — svc-gateway2
# Role: Declarative knobs (timeouts, caps, quotas, amnesia, PQ/TLS policy).
[server]
bind = "127.0.0.1:9300"
metrics_bind = "127.0.0.1:9301"

[caps]
body_max_bytes = 1048576      # 1 MiB
decoded_max_bytes = 8388608   # 8 MiB
decoded_ratio_max = 10.0

[limits]
global_rps = 500

[amnesia]
enabled = true

[pq]
hybrid_enabled = false
EOF

make_file "configs/.env.example" <<'EOF'
# Example environment file for local runs
RUST_LOG=info
SVC_GATEWAY_CONFIG=./configs/svc-gateway.sample.toml
EOF

# --- Docs ---
for doc in IDB.md INTEROP.md CONFIG.md SECURITY.md OBSERVABILITY.md PERFORMANCE.md RUNBOOK.md QUANTUM.md API.md POLICY.md; do
  make_file "docs/$doc" <<EOF
# $doc — svc-gateway2
This is a placeholder. Populate from your finalized templates. Each section should map to invariants and acceptance gates.
EOF
done

make_file "docs/diagrams/architecture.mmd" <<'EOF'
%% Mermaid — architecture (placeholder)
flowchart LR
  client -->|HTTP/TLS| gateway
  gateway -->|forward| overlay
EOF

make_file "docs/diagrams/sequence_ingress.mmd" <<'EOF'
%% Mermaid — ingress sequence (placeholder)
sequenceDiagram
  participant C as Client
  participant G as Gateway
  participant O as Overlay
  C->>G: Request
  G->>G: Layers (caps/DRR/auth)
  G->>O: Forward
  O-->>G: Response
  G-->>C: Response
EOF

make_file "docs/diagrams/readiness_state.mmd" <<'EOF'
%% Mermaid — readiness state (placeholder)
stateDiagram-v2
  [*] --> Healthy
  Healthy --> Degraded: shed writes first
  Degraded --> Healthy: recovery
EOF

make_file "docs/dashboards/gateway_golden.json" <<'EOF'
{
  "_comment": "Grafana-style dashboard placeholder with panel names only."
}
EOF

# --- Scripts ---
make_file "scripts/dev_run.sh" <<'EOF'
#!/usr/bin/env bash
# Starts svc-gateway2 with local env/config (placeholder runtime).
set -euo pipefail
echo "dev_run placeholder — wire up cargo run once implemented"
EOF
chmod +x "$ROOT/scripts/dev_run.sh"

make_file "scripts/soak_test.sh" <<'EOF'
#!/usr/bin/env bash
# Long-running load to validate SLOs (placeholder).
set -euo pipefail
echo "soak_test placeholder — integrate your load tool"
EOF
chmod +x "$ROOT/scripts/soak_test.sh"

make_file "scripts/chaos_burst.sh" <<'EOF'
#!/usr/bin/env bash
# Burst traffic and simulate downstream slowness (placeholder).
set -euo pipefail
echo "chaos_burst placeholder — add chaos injection once routes exist"
EOF
chmod +x "$ROOT/scripts/chaos_burst.sh"

make_file "scripts/export_metrics.sh" <<'EOF'
#!/usr/bin/env bash
# Quick metrics scrape (placeholder).
set -euo pipefail
curl -s http://127.0.0.1:9301/metrics || echo "metrics endpoint not yet implemented"
EOF
chmod +x "$ROOT/scripts/export_metrics.sh"

# --- Examples ---
make_file "examples/curl.http" <<'EOF'
# Paste-ready HTTP examples (placeholder). Use once routes exist.
GET http://127.0.0.1:9300/healthz

GET http://127.0.0.1:9300/readyz

GET http://127.0.0.1:9301/metrics
EOF

make_file "examples/httpie.http" <<'EOF'
# httpie examples (placeholder).
# http :9300/healthz
# http :9301/metrics
EOF

# --- Benches ---
make_file "benches/README.md" <<'EOF'
# Benches — svc-gateway2
Placeholder for micro/macro benches and baseline capture.
EOF

make_file "benches/baseline.json" <<'EOF'
{
  "_comment": "Performance baseline placeholder. Fill with real numbers after first bench run."
}
EOF

# --- Tests (vectors + integration placeholders) ---
make_file "tests/vectors/oap1_frame_roundtrip.json" <<'EOF'
{ "_comment": "OAP/1 frame roundtrip vector placeholder." }
EOF
make_file "tests/vectors/manifest_digest.json" <<'EOF'
{ "_comment": "BLAKE3 manifest digest vector placeholder." }
EOF
make_file "tests/vectors/error_taxonomy.json" <<'EOF'
{ "_comment": "Deterministic error taxonomy mapping placeholder." }
EOF

for t in readyz_degrade.rs caps_limits.rs interop_vectors.rs taxonomy_stability.rs loom_readiness.rs; do
  make_file "tests/integration/$t" <<EOF
//! $t — integration test placeholder.
//! Role: prove invariants (shed writes first, caps, interop vectors, taxonomy freeze, loom interleavings).
EOF
done

# --- Fuzz targets ---
make_file "fuzz/README.md" <<'EOF'
# Fuzz — svc-gateway2
Targets: oap_frame (bounds), taxonomy_mapper (deterministic mapping). Placeholders for now.
EOF

make_file "fuzz/targets/oap_frame.rs" <<'EOF'
//! oap_frame.rs — fuzz target placeholder.
//! Role: enforce 1 MiB frame invariants in OAP/1 envelope parsing.
EOF

make_file "fuzz/targets/taxonomy_mapper.rs" <<'EOF'
//! taxonomy_mapper.rs — fuzz target placeholder.
//! Role: guarantee deterministic, bounded error mapping.
EOF

# --- Source files (Rust placeholders only with role docs) ---
make_file "src/main.rs" <<'EOF'
//! Binary entry (placeholder). Wires CLI → config → TLS → router; mounts control plane routes.
//! No implementation here yet.
fn main() {
    // placeholder main
    println!("svc-gateway2 scaffold (no implementation yet)");
}
EOF

make_file "src/lib.rs" <<'EOF'
//! lib.rs — minimal public surface for tests/examples (placeholder).
//! Keep service wiring in main; avoid leaking internals here.
EOF

make_file "src/consts.rs" <<'EOF'
//! consts.rs — protocol & transport bounds (placeholder).
//! Example: MAX_FRAME (1 MiB), STREAM_CHUNK (~64 KiB), default timeouts.
EOF

make_file "src/errors.rs" <<'EOF'
//! errors.rs — deterministic error taxonomy → canonical HTTP mapping (placeholder).
EOF

make_file "src/result.rs" <<'EOF'
//! result.rs — local Result<T, Error> alias and helpers (placeholder).
EOF

make_file "src/state.rs" <<'EOF'
//! state.rs — shared handles: metrics, readiness registry, forward clients (placeholder). Stateless by design.
EOF

# CLI
make_file "src/cli/mod.rs" <<'EOF'
//! cli/mod.rs — CLI flags (binds, pq policy, econ enforcement) — placeholder.
EOF

# Config
make_file "src/config/mod.rs" <<'EOF'
//! config/mod.rs — Config struct and validation entry points — placeholder.
EOF
make_file "src/config/env.rs" <<'EOF'
//! config/env.rs — Env var loaders for quick dev — placeholder.
EOF
make_file "src/config/safety.rs" <<'EOF'
//! config/safety.rs — Hard safety checks (caps/timeouts) — placeholder.
EOF
make_file "src/config/amnesia.rs" <<'EOF'
//! config/amnesia.rs — RAM-only posture, zeroization cadence — placeholder.
EOF

# Observability
make_file "src/observability/mod.rs" <<'EOF'
//! observability/mod.rs — One import point — placeholder.
EOF
make_file "src/observability/metrics.rs" <<'EOF'
//! observability/metrics.rs — Golden counters/gauges/histograms — placeholder.
EOF
make_file "src/observability/tracing.rs" <<'EOF'
//! observability/tracing.rs — Trace subscriber & context propagation — placeholder.
EOF
make_file "src/observability/logging.rs" <<'EOF'
//! observability/logging.rs — JSON logs, UTC timestamps, redaction — placeholder.
EOF

# Headers
make_file "src/headers/mod.rs" <<'EOF'
//! headers/mod.rs — Normalize tracing/cache headers — placeholder.
EOF
make_file "src/headers/etag.rs" <<'EOF'
//! headers/etag.rs — ETag helpers ("b3:<hex>") — placeholder.
EOF

# TLS / PQ
make_file "src/tls/mod.rs" <<'EOF'
//! tls/mod.rs — TLS 1.3 via tokio_rustls; PQ-neutral posture — placeholder.
EOF
make_file "src/pq/mod.rs" <<'EOF'
//! pq/mod.rs — Feature-gated PQ posture (no crypto here) — placeholder.
EOF
make_file "src/pq/policy.rs" <<'EOF'
//! pq/policy.rs — Allowed TLS suites, hybrid toggle, suite drift checks — placeholder.
EOF

# Readiness
make_file "src/readiness/mod.rs" <<'EOF'
//! readiness/mod.rs — Readiness registry and degrade signals — placeholder.
EOF
make_file "src/readiness/keys.rs" <<'EOF'
//! readiness/keys.rs — Canonical readiness keys — placeholder.
EOF

# Policy
make_file "src/policy/mod.rs" <<'EOF'
//! policy/mod.rs — Policy trait & lightweight types (pure logic) — placeholder.
EOF
make_file "src/policy/residency.rs" <<'EOF'
//! policy/residency.rs — Region/geo allow/deny decisions — placeholder.
EOF
make_file "src/policy/abuse.rs" <<'EOF'
//! policy/abuse.rs — Abuse categories & scoring (for tarpit/rate-limit) — placeholder.
EOF

# Layers
make_file "src/layers/mod.rs" <<'EOF'
//! layers/mod.rs — Layer wiring hub — placeholder.
EOF
make_file "src/layers/timeouts.rs" <<'EOF'
//! layers/timeouts.rs — Read/write/connect timeouts — placeholder.
EOF
make_file "src/layers/body_caps.rs" <<'EOF'
//! layers/body_caps.rs — 1 MiB body cap (early 413) — placeholder.
EOF
make_file "src/layers/decode_guard.rs" <<'EOF'
//! layers/decode_guard.rs — Decoded cap ≤ 8 MiB & ≤ 10× ratio — placeholder.
EOF
make_file "src/layers/rate_limit.rs" <<'EOF'
//! layers/rate_limit.rs — Global RPS limit with Retry-After — placeholder.
EOF
make_file "src/layers/drr.rs" <<'EOF'
//! layers/drr.rs — Per-tenant DRR fair-queue + gauges — placeholder.
EOF
make_file "src/layers/tarpit.rs" <<'EOF'
//! layers/tarpit.rs — Exponential delays for abuse categories — placeholder.
EOF
make_file "src/layers/auth.rs" <<'EOF'
//! layers/auth.rs — Macaroon capability verification — placeholder.
EOF
make_file "src/layers/corr.rs" <<'EOF'
//! layers/corr.rs — Correlation-ID extraction/injection — placeholder.
EOF

# Admission
make_file "src/admission/mod.rs" <<'EOF'
//! admission/mod.rs — Admission orchestrator — placeholder.
EOF
make_file "src/admission/classifier.rs" <<'EOF'
//! admission/classifier.rs — Cheap request classifier (read/write/admin/media) — placeholder.
EOF
make_file "src/admission/capabilities.rs" <<'EOF'
//! admission/capabilities.rs — Capability checks & caveats — placeholder.
EOF
make_file "src/admission/quotas.rs" <<'EOF'
//! admission/quotas.rs — Tenant quotas; 429 with Retry-After — placeholder.
EOF
make_file "src/admission/taxonomy.rs" <<'EOF'
//! admission/taxonomy.rs — Deterministic reason/status mapping — placeholder.
EOF
make_file "src/admission/residency.rs" <<'EOF'
//! admission/residency.rs — Thin adapter to policy::residency — placeholder.
EOF
make_file "src/admission/payments.rs" <<'EOF'
//! admission/payments.rs — (feature = "econ") enforce paid writes/prepaid quotas — placeholder.
EOF

# Routes
make_file "src/routes/mod.rs" <<'EOF'
//! routes/mod.rs — Router assembly; mounts sub-routers — placeholder.
EOF
make_file "src/routes/health.rs" <<'EOF'
//! routes/health.rs — /healthz liveness — placeholder.
EOF
make_file "src/routes/ready.rs" <<'EOF'
//! routes/ready.rs — /readyz with degrade signals (shed writes first) — placeholder.
EOF
make_file "src/routes/metrics.rs" <<'EOF'
//! routes/metrics.rs — /metrics Prometheus exposition — placeholder.
EOF
make_file "src/routes/version.rs" <<'EOF'
//! routes/version.rs — /version build metadata — placeholder.
EOF
make_file "src/routes/objects.rs" <<'EOF'
//! routes/objects.rs — GET /o/{addr}; addressing + ETag — placeholder.
EOF
make_file "src/routes/objects_range.rs" <<'EOF'
//! routes/objects_range.rs — GET /o/{addr} with strict Range (read-only) — placeholder.
EOF

# Forward
make_file "src/forward/mod.rs" <<'EOF'
//! forward/mod.rs — Facade selectors & shared types — placeholder.
EOF
make_file "src/forward/overlay_client.rs" <<'EOF'
//! forward/overlay_client.rs — Forward to overlay; map errors deterministically — placeholder.
EOF
make_file "src/forward/index_client.rs" <<'EOF'
//! forward/index_client.rs — Optional: resolve helpers (addr→location) — placeholder.
EOF
make_file "src/forward/storage_client.rs" <<'EOF'
//! forward/storage_client.rs — Optional: read-only media proxy; range-reads — placeholder.
EOF

note "Scaffold complete: $ROOT"
