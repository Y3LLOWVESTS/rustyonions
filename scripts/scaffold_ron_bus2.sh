#!/usr/bin/env bash
# Scaffolds crates/ron-bus2 with empty files + tiny stubs (no code logic).
# Works on macOS's default bash (3.2) and Linux.
# Usage:
#   chmod +x scripts/scaffold_ron_bus2.sh
#   ./scripts/scaffold_ron_bus2.sh [repo_root]
# Set RON_FORCE=1 to overwrite an existing crates/ron-bus2 directory.

set -euo pipefail

ROOT="${1:-.}"
CRATE_DIR="$ROOT/crates/ron-bus2"

# Safety: refuse to overwrite unless RON_FORCE=1
if [ -e "$CRATE_DIR" ] && [ "${RON_FORCE:-0}" != "1" ]; then
  echo "Error: $CRATE_DIR already exists. Set RON_FORCE=1 to overwrite." >&2
  exit 1
fi

# Clean existing dir if force
if [ -e "$CRATE_DIR" ]; then
  rm -rf "$CRATE_DIR"
fi

# --- Directories to create ---
DIRS="
.cargo
.github/workflows
docs
docs/diagrams
docs/api-history/ron-bus
scripts
src
src/internal
tests
benches
"

# --- Files to create (relative to CRATE_DIR) ---
FILES="
Cargo.toml
README.md
CHANGELOG.md
LICENSE-APACHE
LICENSE-MIT
CODEOWNERS
rust-toolchain.toml
deny.toml
.clippy.toml
.gitignore
.cargo/config.toml
.github/workflows/ci.yml
.github/workflows/coverage.yml
.github/workflows/nightly-chaos.yml
.github/workflows/render-mermaid.yml
docs/README.md
docs/API.md
docs/CONCURRENCY.md
docs/CONFIG.md
docs/OBSERVABILITY.md
docs/IDB.md
docs/INTEROP.md
docs/diagrams/arch.mmd
docs/diagrams/sequence.mmd
docs/diagrams/state.mmd
docs/api-history/ron-bus/v1.0.0.txt
scripts/update_api_snapshot.sh
src/lib.rs
src/bus.rs
src/event.rs
src/config.rs
src/metrics.rs
src/errors.rs
src/prelude.rs
src/internal/channel.rs
src/internal/depth_estimator.rs
src/internal/seals.rs
tests/fanout_ok.rs
tests/lagged_overflow_smoke.rs
tests/receiver_ownership.rs
tests/capacity_cutover.rs
tests/api_surface.rs
tests/property_bus.rs
tests/pq_labels_feature.rs
tests/chaos_amnesia.rs
tests/loom_model.rs
benches/throughput.rs
benches/latency.rs
benches/overflow.rs
"

# Create directories
echo "$DIRS" | while IFS= read -r d; do
  [ -z "$d" ] && continue
  mkdir -p "$CRATE_DIR/$d"
done

# Helper: write file with heredoc content (called inline)
# usage: write_file "path" <<'EOF'
# <content>
# EOF
write_file() {
  local path="$1"
  cat > "$CRATE_DIR/$path"
}

# Helper: touch files that aren't explicitly written below
touch_missing_files() {
  echo "$FILES" | while IFS= read -r f; do
    [ -z "$f" ] && continue
    if [ ! -f "$CRATE_DIR/$f" ]; then
      : > "$CRATE_DIR/$f"
    fi
  done
}

# --- Minimal helpful stubs (tiny content; you’ll fill in later) ---

write_file "README.md" <<'EOF'
# ron-bus

This crate provides the bounded, lossy, observable in-proc broadcast bus for RustyOnions.

See docs in `./docs` and CI in `.github/workflows`.
EOF

write_file "Cargo.toml" <<'EOF'
[package]
name = "ron-bus2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[lib]
path = "src/lib.rs"

[features]
tracing = []
pq-labels = []
loom = []
EOF

write_file "CHANGELOG.md" <<'EOF'
# Changelog

All notable changes to this project will be documented here.
EOF

write_file "LICENSE-APACHE" <<'EOF'
Apache-2.0 (placeholder)
EOF

write_file "LICENSE-MIT" <<'EOF'
MIT (placeholder)
EOF

write_file "CODEOWNERS" <<'EOF'
* @stevanwhite
EOF

write_file "rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.80.0"
components = ["rustfmt", "clippy"]
EOF

write_file "deny.toml" <<'EOF'
# cargo-deny config (placeholder). See workspace root for canonical policy.
EOF

write_file ".clippy.toml" <<'EOF'
# Keep lock-across-await and pedantic checks tight (placeholder)
warn-on-all-wildcard-imports = true
EOF

write_file ".gitignore" <<'EOF'
target/
**/*.svg
**/coverage/
*.profraw
EOF

write_file ".cargo/config.toml" <<'EOF'
[alias]
lint = "clippy -p ron-bus2 -- -D warnings"
test-all = "test -p ron-bus2 --all-features"
ci-check = "fmt --all && clippy -D warnings && test -p ron-bus2 && deny check"
bench-all = "bench -p ron-bus2"
EOF

write_file ".github/workflows/ci.yml" <<'EOF'
name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
          components: rustfmt, clippy
      - run: cargo fmt --all --check
      - run: cargo clippy -p ron-bus2 -- -D warnings
      - run: cargo test -p ron-bus2 --all-features
      - run: cargo test -p ron-bus2 --doc
EOF

write_file ".github/workflows/coverage.yml" <<'EOF'
name: coverage
on:
  push:
    branches: [ main ]
  pull_request:
jobs:
  cover:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
      - name: Run coverage (placeholder)
        run: echo "Implement tarpaulin/grcov coverage here with Bronze/Silver/Gold thresholds"
EOF

write_file ".github/workflows/nightly-chaos.yml" <<'EOF'
name: nightly-chaos
on:
  schedule:
    - cron: "0 3 * * *"
jobs:
  chaos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
      - name: Run ignored chaos tests (placeholder)
        run: cargo test -p ron-bus2 -- --ignored --nocapture
EOF

write_file ".github/workflows/render-mermaid.yml" <<'EOF'
name: render-mermaid
on: [push, pull_request]
jobs:
  mmdc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          mkdir -p docs/diagrams
          for f in $(git ls-files 'docs/diagrams/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
EOF

write_file "docs/README.md" <<'EOF'
# ron-bus docs

Index for crate documentation. See individual files for details.
EOF

write_file "docs/API.md" <<'EOF'
# API

Frozen monomorphic surface: Bus::{new,sender,subscribe,capacity}, Event::{...}.
EOF

write_file "docs/CONCURRENCY.md" <<'EOF'
# Concurrency

One receiver per task. Never hold a lock across .await. No I/O or background tasks.
EOF

write_file "docs/CONFIG.md" <<'EOF'
# Config

Capacity fixed at creation; cutover by constructing a new bus; amnesia label guidance.
EOF

write_file "docs/OBSERVABILITY.md" <<'EOF'
# Observability

Host-owned metrics: bus_overflow_dropped_total, bus_queue_depth. Example alert & dashboard.
EOF

write_file "docs/IDB.md" <<'EOF'
# IDB

Invariants, non-goals, proofs, and acceptance gates tied to tests and CI.
EOF

write_file "docs/INTEROP.md" <<'EOF'
# Interop

In-proc only; HTTP/RPC live in host crates; no secrets/PII on the bus.
EOF

write_file "docs/diagrams/arch.mmd" <<'EOF'
%% Mermaid arch diagram placeholder
flowchart LR
  A[Publishers] --> B((ron-bus))
  B --> C[Subscribers]
EOF

write_file "docs/diagrams/sequence.mmd" <<'EOF'
%% Mermaid sequence diagram placeholder
sequenceDiagram
  actor P as Publisher
  participant B as ron-bus
  participant S as Subscriber
  P->>B: send(Event)
  B-->>S: recv()
EOF

write_file "docs/diagrams/state.mmd" <<'EOF'
%% Mermaid state diagram placeholder
stateDiagram-v2
  [*] --> Idle
  Idle --> Receiving
  Receiving --> Lagged
  Lagged --> Receiving
  Receiving --> Shutdown
  Shutdown --> [*]
EOF

write_file "docs/api-history/ron-bus/v1.0.0.txt" <<'EOF'
public API snapshot (placeholder)
EOF

write_file "scripts/update_api_snapshot.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Placeholder to regenerate public API snapshot into docs/api-history/ron-bus/
echo "Implement public API snapshot generation here."
EOF
chmod +x "$CRATE_DIR/scripts/update_api_snapshot.sh"

# Source placeholders (intentionally minimal)
write_file "src/lib.rs" <<'EOF'
// re-exports will live here (placeholder)
EOF

write_file "src/bus.rs" <<'EOF'
// bus core (bounded broadcast) — placeholder
EOF

write_file "src/event.rs" <<'EOF'
// Event enum placeholder
EOF

write_file "src/config.rs" <<'EOF'
// BusConfig placeholder
EOF

write_file "src/metrics.rs" <<'EOF'
// host-owned metrics facade placeholder
EOF

write_file "src/errors.rs" <<'EOF'
// error taxonomy placeholder
EOF

write_file "src/prelude.rs" <<'EOF'
// minimal prelude placeholder
EOF

write_file "src/internal/channel.rs" <<'EOF'
// internal channel wrapper placeholder
EOF

write_file "src/internal/depth_estimator.rs" <<'EOF'
// queue depth heuristic placeholder
EOF

write_file "src/internal/seals.rs" <<'EOF'
// sealed traits placeholder
EOF

# Tests placeholders (named so they're easy to fill)
write_file "tests/fanout_ok.rs" <<'EOF'
// fanout_ok test placeholder
EOF

write_file "tests/lagged_overflow_smoke.rs" <<'EOF'
// lagged_overflow_smoke test placeholder
EOF

write_file "tests/receiver_ownership.rs" <<'EOF'
// receiver_ownership test placeholder
EOF

write_file "tests/capacity_cutover.rs" <<'EOF'
// capacity_cutover test placeholder
EOF

write_file "tests/api_surface.rs" <<'EOF'
// api_surface snapshot test placeholder
EOF

write_file "tests/property_bus.rs" <<'EOF'
// property-based tests placeholder
EOF

write_file "tests/pq_labels_feature.rs" <<'EOF'
// pq-labels feature test placeholder
EOF

write_file "tests/chaos_amnesia.rs" <<'EOF'
// ignored chaos amnesia test placeholder
EOF

write_file "tests/loom_model.rs" <<'EOF'
// loom model test placeholder
EOF

# Benches placeholders
write_file "benches/throughput.rs" <<'EOF'
// throughput bench placeholder
EOF

write_file "benches/latency.rs" <<'EOF'
// latency bench placeholder
EOF

write_file "benches/overflow.rs" <<'EOF'
// overflow bench placeholder
EOF

# Touch any remaining listed files not explicitly written
touch_missing_files

echo "Scaffold created at: $CRATE_DIR"
