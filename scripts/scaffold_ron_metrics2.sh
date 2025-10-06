#!/usr/bin/env bash
# Scaffolds the ron-metrics2 crate file tree with documented placeholders (no code).
# Usage:
#   bash scripts/scaffold_ron_metrics2.sh
# Overwrite existing files:
#   FORCE=1 bash scripts/scaffold_ron_metrics2.sh
set -euo pipefail

CRATE_DIR="crates/ron-metrics2"
CRATE_NAME="ron-metrics2"

mkdir -p "$CRATE_DIR"

# helper: write stdin to file unless it exists (unless FORCE=1)
put() {
  local path="$1"
  if [[ -e "$path" && "${FORCE:-0}" != "1" ]]; then
    echo "skip (exists): $path"
    return 0
  fi
  mkdir -p "$(dirname "$path")"
  cat > "$path"
}

echo "Scaffolding into $CRATE_DIR ..."

# -------------------------------------------------------------------
# Repo meta & policy
# -------------------------------------------------------------------
put "$CRATE_DIR/Cargo.toml" <<'EOF'
[package]
name = "ron-metrics2"
version = "0.0.0"
edition = "2021"
rust-version = "1.80"
license = "MIT OR Apache-2.0"
description = "Observability library with tiny HTTP exposer for /metrics, /healthz, /readyz"
repository = ""
homepage = ""
documentation = ""

[features]
default = []
otel = []
tls  = []
pq   = []
zk   = []
cli  = []

[dependencies]
# (scaffold) add concrete deps when implementing:
# axum = { version = "...", default-features = false, features = ["http1"] }
# prometheus = "..."
# tokio = { version = "...", features = ["rt-multi-thread", "macros", "time"] }
# tokio-rustls = "..."
# serde = { version = "...", features = ["derive"] }
# thiserror = "..."

[dev-dependencies]
# criterion = "..."
EOF

put "$CRATE_DIR/README.md" <<'EOF'
# ron-metrics2

> **Role:** library (observability library with tiny HTTP exposer)  
> **Status:** scaffold  
> **MSRV:** 1.80.0

This is the **scaffold** for ron-metrics2. It mirrors the ron-metrics documentation:
- Endpoints: GET /metrics, /healthz, /readyz
- Canon labels: service, instance, build_version, amnesia
- SLO: p95 exposition latency < 10ms (local)
- Readiness: fail-open reads / fail-closed writes

See docs/* for the living contract; tests and CI enforce acceptance gates.
EOF

put "$CRATE_DIR/CHANGELOG.md" <<'EOF'
# Changelog — ron-metrics2

All notable changes to this crate will be documented here. Follow SemVer.

## [0.0.0] - scaffold
- Initial scaffold: file tree, docs placeholders, CI stubs.
EOF

put "$CRATE_DIR/LICENSE-MIT" <<'EOF'
MIT License

Copyright (c) 2025 RustyOnions

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the “Software”), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
EOF

put "$CRATE_DIR/LICENSE-APACHE" <<'EOF'
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

Copyright (c) 2025 RustyOnions

You may obtain a copy of the License at the URL above.
Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an “AS IS” BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
EOF

put "$CRATE_DIR/CODEOWNERS" <<'EOF'
*        @your-handle-or-team
docs/    @your-handle-or-team
.github/ @your-handle-or-team
EOF

put "$CRATE_DIR/.gitignore" <<'EOF'
/target
**/*.svg
**/*.lcov
coverage/
dist/
docs/sbom/
.DS_Store
EOF

put "$CRATE_DIR/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["rustfmt", "clippy"]
EOF

put "$CRATE_DIR/deny.toml" <<'EOF'
# cargo-deny baseline (licenses, advisories, bans)
[advisories]
vulnerability = "deny"
yanked = "deny"

[licenses]
confidence-threshold = 0.8
allow = [
  "MIT", "Apache-2.0", "Unicode-DFS-2016", "BSD-3-Clause",
  "ISC", "CC0-1.0", "OpenSSL"
]
EOF

mkdir -p "$CRATE_DIR/.cargo"
put "$CRATE_DIR/.cargo/config.toml" <<'EOF'
# Minimal, portable Cargo config for CI parity
[build]
rustflags = []

[env]
RUSTFLAGS = "-Dwarnings"
EOF

# -------------------------------------------------------------------
# Source tree
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/src/exposer" "$CRATE_DIR/src/exporters"
put "$CRATE_DIR/src/lib.rs" <<'EOF'
/*!
Minimal façade for ron-metrics2.
This crate exposes a tiny HTTP exposer for GET /metrics, /healthz, /readyz.
No implementation code in scaffold — only module layout and docs.
*/
pub mod config;
pub mod metrics;
pub mod registry;
pub mod labels;
pub mod readiness;
pub mod health;
pub mod errors;
pub mod build_info;
pub mod pq;
pub mod zk;
pub mod exposer;
pub mod exporters;

/* Public re-exports go here when implemented:
   pub use metrics::Metrics;
   pub use config::Config;
*/
EOF

put "$CRATE_DIR/src/config.rs" <<'EOF'
// Typed configuration placeholder for ron-metrics2.
// Defines TCP addr vs UDS, timeouts, TLS paths, OTLP endpoint (env-first).
// Implementation intentionally omitted in scaffold.
EOF

put "$CRATE_DIR/src/metrics.rs" <<'EOF'
// Metric family registration placeholder.
// Register golden families once; provide handles used by the exposer.
// Suffix discipline: *_seconds | *_bytes | *_total.
EOF

put "$CRATE_DIR/src/registry.rs" <<'EOF'
// Registry helpers placeholder.
// Prevent duplicate family registration; namespace consistently.
EOF

put "$CRATE_DIR/src/labels.rs" <<'EOF'
// Base labels placeholder: service, instance, build_version, amnesia.
// Enforce low-cardinality and required label presence.
EOF

put "$CRATE_DIR/src/readiness.rs" <<'EOF'
// Readiness DTO placeholder.
// Policy: fail-open reads / fail-closed writes; emits Retry-After on 503.
EOF

put "$CRATE_DIR/src/health.rs" <<'EOF'
// Liveness glue placeholder (separate from readiness).
EOF

put "$CRATE_DIR/src/errors.rs" <<'EOF'
// Error taxonomy placeholder: Busy, Timeout, Canceled, Lagging (ops-focused).
EOF

put "$CRATE_DIR/src/build_info.rs" <<'EOF'
// Build metadata placeholder (version/commit/time) for build_version label.
EOF

put "$CRATE_DIR/src/pq.rs" <<'EOF'
// PQ metrics placeholder (present even if zero): pq_kex_failures_total, etc.
EOF

put "$CRATE_DIR/src/zk.rs" <<'EOF'
// ZK metrics placeholder (present even if zero): zk_verify_failures_total, zk_proof_latency_seconds.
EOF

# exposer submodules
put "$CRATE_DIR/src/exposer/mod.rs" <<'EOF'
// Router assembly placeholder for /metrics, /healthz, /readyz.
pub mod http;
pub mod middleware;
pub mod uds;
pub mod tls;
EOF

put "$CRATE_DIR/src/exposer/http.rs" <<'EOF'
// Axum handlers placeholder (GET-only); headers: no-store, nosniff.
// Shapes responses for metrics/healthz/readyz.
EOF

put "$CRATE_DIR/src/exposer/middleware.rs" <<'EOF'
// Middleware placeholder: timeouts, concurrency caps, inflight gauge hooks.
EOF

put "$CRATE_DIR/src/exposer/uds.rs" <<'EOF'
// UDS bind placeholder with secure modes (0700 dir / 0600 socket), no symlinks.
EOF

put "$CRATE_DIR/src/exposer/tls.rs" <<'EOF'
// TLS wiring placeholder (tokio_rustls::rustls::ServerConfig only).
EOF

# exporters under src/exporters (feature seam)
put "$CRATE_DIR/src/exporters/mod.rs" <<'EOF'
// Exporters feature seam placeholder (keeps core API unchanged).
// Example: OTLP exporter behind `otel` feature.
#[allow(dead_code)]
mod _placeholder {}
EOF

put "$CRATE_DIR/src/exporters/otel.rs" <<'EOF'
// OTLP exporter placeholder: 1:1 mapping with Prometheus families/labels.
EOF

# -------------------------------------------------------------------
# Examples
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/examples"
put "$CRATE_DIR/examples/exposer.rs" <<'EOF'
// Minimal example placeholder: Metrics::new().serve(addr, health).println(bound);
// Intentionally code-free in scaffold.
fn main() {}
EOF

# -------------------------------------------------------------------
# Tests & vectors (acceptance gates)
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/tests/vectors/interop/ron-metrics"
put "$CRATE_DIR/tests/integration_http_endpoints.rs" <<'EOF'
// Ensures /metrics, /healthz, /readyz status/headers/body shapes (scaffold placeholder).
#[test] fn placeholder() {}
EOF

put "$CRATE_DIR/tests/taxonomy_labels.rs" <<'EOF'
// Verifies suffix discipline and base labels presence (scaffold placeholder).
#[test] fn placeholder() {}
EOF

put "$CRATE_DIR/tests/readiness_semantics.rs" <<'EOF'
// Asserts fail-open reads / fail-closed writes and Retry-After semantics (scaffold placeholder).
#[test] fn placeholder() {}
EOF

put "$CRATE_DIR/tests/public_api.rs" <<'EOF'
// Guards public symbol drift via cargo-public-api snapshots (scaffold placeholder).
#[test] fn placeholder() {}
EOF

put "$CRATE_DIR/tests/loom_shutdown.rs" <<'EOF'
// Loom-gated shutdown sequencing (no locks across .await) — scaffold placeholder.
#[test] fn placeholder() {}
EOF

put "$CRATE_DIR/tests/vectors/interop/ron-metrics/readyz-ready.json" <<'EOF'
{ "degraded": false, "missing": [], "retry_after": 0 }
EOF

put "$CRATE_DIR/tests/vectors/interop/ron-metrics/readyz-degraded.json" <<'EOF'
{ "degraded": true, "missing": ["config_loaded","kernel_bus_attached"], "retry_after": 5 }
EOF

put "$CRATE_DIR/tests/vectors/interop/ron-metrics/metrics-small.prom" <<'EOF'
# HELP request_latency_seconds Latency histogram
# TYPE request_latency_seconds histogram
request_latency_seconds_bucket{route="/metrics",method="GET",le="0.01"} 1
request_latency_seconds_sum{route="/metrics",method="GET"} 0.002
request_latency_seconds_count{route="/metrics",method="GET"} 1
EOF

put "$CRATE_DIR/tests/vectors/interop/ron-metrics/metrics-large.prom" <<'EOF'
# Simulated large registry for perf smoke (scaffold placeholder)
EOF

# -------------------------------------------------------------------
# Benches (SLO proof)
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/benches"
put "$CRATE_DIR/benches/exposer_bench.rs" <<'EOF'
// Criterion bench placeholder for exposition latency across registry sizes.
fn main() {}
EOF

put "$CRATE_DIR/benches/hotpath_bench.rs" <<'EOF'
// Measures counter/histogram hot path (scaffold placeholder).
fn main() {}
EOF

# -------------------------------------------------------------------
# Docs (living design)
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/docs/mmd" "$CRATE_DIR/docs/api-history/ron-metrics" "$CRATE_DIR/docs/sbom"
for f in API CONCURRENCY CONFIG GOVERNANCE IDB INTEROP OBSERVABILITY PERFORMANCE QUANTUM RUNBOOK SECURITY; do
  put "$CRATE_DIR/docs/${f}.md" <<EOF
# ${f} — ron-metrics2 (scaffold)

Populate from finalized ron-metrics docs; keep invariants and acceptance gates aligned.
EOF
done

put "$CRATE_DIR/docs/mmd/arch.mmd" <<'EOF'
flowchart LR
  subgraph Host Service
    M[ron-metrics2 exposer]
  end
  P[Prometheus] -->|GET /metrics| M
  P -->|GET /healthz| M
  P -->|GET /readyz| M
  M --> E[[OTLP Collector (optional)]]
EOF

put "$CRATE_DIR/docs/mmd/sequence.mmd" <<'EOF'
sequenceDiagram
  participant P as Prometheus
  participant E as ron-metrics2 exposer
  P->>E: GET /metrics (deadline=2s)
  E-->>P: 200 text/plain
EOF

put "$CRATE_DIR/docs/mmd/state.mmd" <<'EOF'
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: serve()
  Running --> Degraded: deps missing
  Degraded --> Running: deps restored
  Running --> Shutdown: ctrl_c
  Shutdown --> [*]
EOF

put "$CRATE_DIR/docs/api-history/ron-metrics/0001.initial.txt" <<'EOF'
# cargo public-api snapshot placeholder (scaffold)
EOF

put "$CRATE_DIR/docs/sbom/.gitkeep" <<'EOF'
# SBOMs (CycloneDX) go here on releases.
EOF

# -------------------------------------------------------------------
# Scripts
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/scripts"
put "$CRATE_DIR/scripts/check-taxonomy.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Placeholder: fetch /metrics and assert suffix and base labels (scaffold).
echo "check-taxonomy: placeholder"
EOF
put "$CRATE_DIR/scripts/render-mermaid.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Renders all docs/mmd/*.mmd to .svg (requires mmdc).
echo "render-mermaid: placeholder"
EOF
chmod +x "$CRATE_DIR/scripts/check-taxonomy.sh" "$CRATE_DIR/scripts/render-mermaid.sh"

# -------------------------------------------------------------------
# GitHub Actions workflows
# -------------------------------------------------------------------
mkdir -p "$CRATE_DIR/.github/workflows"
put "$CRATE_DIR/.github/workflows/ci.yml" <<'EOF'
name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup toolchain install 1.80.0 --profile minimal --component rustfmt clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy -p ron-metrics2 -- -D warnings
      - run: cargo test -p ron-metrics2
      - run: cargo deny check
EOF

put "$CRATE_DIR/.github/workflows/coverage.yml" <<'EOF'
name: coverage
on: [push, pull_request]
jobs:
  cov:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --workspace --lcov --output-path lcov.info
      - run: cargo llvm-cov report --json | jq -e '.data[0].totals.branches.percent >= 80'
EOF

put "$CRATE_DIR/.github/workflows/public-api.yml" <<'EOF'
name: public-api
on: [push, pull_request]
jobs:
  api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-public-api
      # Allow first run to pass before snapshots exist; tighten later.
      - run: cargo public-api --crate ron-metrics2 --deny changed || true
EOF

put "$CRATE_DIR/.github/workflows/mermaid.yml" <<'EOF'
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
          for f in $(git ls-files 'docs/mmd/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
EOF

put "$CRATE_DIR/.github/workflows/sanitizer.yml" <<'EOF'
name: sanitizer
on:
  schedule:
    - cron: "0 4 * * *"
jobs:
  tsan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup toolchain install nightly --profile minimal
      - run: rustup default nightly
      - run: RUSTFLAGS="-Z sanitizer=thread" RUSTDOCFLAGS="-Z sanitizer=thread" cargo test -p ron-metrics2 --target x86_64-unknown-linux-gnu || true
EOF

put "$CRATE_DIR/.github/workflows/perf-smoke.yml" <<'EOF'
name: perf-smoke
on:
  workflow_dispatch:
jobs:
  bombardier:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "perf smoke placeholder (would hammer /metrics and record p95)"
EOF

echo "Done. Scaffolding complete in $CRATE_DIR."
