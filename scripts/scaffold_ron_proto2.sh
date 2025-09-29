#!/usr/bin/env bash
# Scaffolds crates/ron-proto2 with the agreed modular file tree (empty/stub files).
# Usage:
#   chmod +x scripts/scaffold_ron_proto2.sh
#   ./scripts/scaffold_ron_proto2.sh [repo_root]
#
# Notes:
# - No Rust code is added; only skeletal file headers/placeholders.
# - macOS-safe (no mapfile). Idempotent: re-running won’t clobber existing files.

set -euo pipefail

ROOT="${1:-.}"
CRATE_DIR="$ROOT/crates/ron-proto2"

# --- Directories to create ---
DIRS=$(cat <<'EOF'
src
src/id
src/oap
src/manifest
src/mailbox
src/cap
src/error
src/econ
src/gov
src/quantum
src/config
tests
tests/fuzz/corpora/decode_oap_header
tests/fuzz/corpora/decode_manifest
tests/fuzz/corpora/parse_contentid
tests/fuzz/corpora/decode_capability
tests/vectors
benches
fuzz
fuzz/fuzz_targets
docs
docs/api-history/ron-proto
docs/schema/json
testing/performance/baselines
.github/workflows
EOF
)

# --- Files to create (empty by default unless we inject a tiny header) ---
FILES=$(cat <<'EOF'
README.md
CHANGELOG.md
LICENSE-APACHE
LICENSE-MIT
src/lib.rs
src/version.rs
src/trace.rs
src/naming.rs
src/id/mod.rs
src/id/content_id.rs
src/id/parse.rs
src/oap/mod.rs
src/oap/hello.rs
src/oap/start.rs
src/oap/data.rs
src/oap/end.rs
src/oap/error.rs
src/manifest/mod.rs
src/manifest/v1.rs
src/manifest/common.rs
src/mailbox/mod.rs
src/mailbox/send.rs
src/mailbox/recv.rs
src/mailbox/ack.rs
src/cap/mod.rs
src/cap/header.rs
src/cap/caveats.rs
src/error/mod.rs
src/error/kind.rs
src/error/reason.rs
src/econ/mod.rs
src/econ/move_entry.rs
src/gov/mod.rs
src/gov/signed_descriptor.rs
src/quantum/mod.rs
src/quantum/pq_tags.rs
src/config/mod.rs
src/config/validate.rs
tests/interop_parity.rs
tests/cross_version.rs
tests/hash_truth.rs
tests/econ_conservation.rs
tests/vectors/oap_hello_v1.json
tests/vectors/oap_error_envelope.json
tests/vectors/manifest_v1.json
tests/vectors/content_id.json
benches/encode_decode_small.rs
benches/encode_decode_large.rs
fuzz/Cargo.toml
fuzz/fuzz_targets/decode_oap_header.rs
fuzz/fuzz_targets/decode_manifest.rs
fuzz/fuzz_targets/parse_contentid.rs
fuzz/fuzz_targets/decode_capability.rs
docs/ALL_DOCS.md
docs/API.md
docs/CONFIG.md
docs/OBSERVABILITY.md
docs/PERFORMANCE.md
docs/QUANTUM.md
docs/RUNBOOK.md
docs/SECURITY.md
docs/TESTS.md
docs/arch.mmd
docs/sequence.mmd
docs/state.mmd
docs/api-history/ron-proto/.keep
docs/schema/README.md
docs/schema/json/oap_hello.schema.json
docs/schema/json/manifest_v1.schema.json
testing/performance/baselines/ron-proto.json
.github/workflows/ci.yml
.github/workflows/fuzz.yml
.github/workflows/render-mermaid.yml
EOF
)

# --- Minimal Cargo.toml template (no code, DTO-only deps placeholder) ---
cargo_toml() {
cat <<'EOF'
[package]
name = "ron-proto2"
version = "0.0.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false
description = "DTO-only schemas for RustyOnions (scaffold)"

[lib]
path = "src/lib.rs"

[features]
default = ["serde"]
cbor = []
schemars = []

[dependencies]
serde = { version = "1", features = ["derive"] }
bytes = "1"
thiserror = "1"

[dev-dependencies]
serde_json = "1"

[package.metadata.ci]
# Non-functional markers to remind us this is a scaffold only.
EOF
}

# --- Helper: write a file only if it doesn't already exist ---
safe_write() {
  local path="$1"
  shift
  if [ -e "$path" ]; then
    return 0
  fi
  mkdir -p "$(dirname "$path")"
  printf "%s" "$*" > "$path"
}

# --- Helper: create empty file if missing ---
touch_empty() {
  local path="$1"
  if [ ! -e "$path" ]; then
    mkdir -p "$(dirname "$path")"
    : > "$path"
  fi
}

# --- Create crate directory and Cargo.toml ---
mkdir -p "$CRATE_DIR"
if [ ! -e "$CRATE_DIR/Cargo.toml" ]; then
  cargo_toml > "$CRATE_DIR/Cargo.toml"
fi

# --- Create directories ---
# shellcheck disable=SC2034
while IFS= read -r d; do
  [ -z "$d" ] && continue
  mkdir -p "$CRATE_DIR/$d"
done <<< "$DIRS"

# --- Create files (empty by default) ---
# shellcheck disable=SC2034
while IFS= read -r f; do
  [ -z "$f" ] && continue
  touch_empty "$CRATE_DIR/$f"
done <<< "$FILES"

# --- Add tiny headers to a few key files (no code, just context) ---
safe_write "$CRATE_DIR/src/lib.rs" "// ron-proto2: DTO-only public API (scaffold)\n"
safe_write "$CRATE_DIR/README.md" "# ron-proto2 (scaffold)\n\n> DTO-only crate scaffold. No runtime, no I/O, no crypto.\n"
safe_write "$CRATE_DIR/CHANGELOG.md" "## 0.0.0 (scaffold)\n- Initial scaffold, no API.\n"
safe_write "$CRATE_DIR/docs/schema/README.md" "Schemas are emitted here (optional). Downstream SDKs consume these for codegen and parity tests.\n"

# ci.yml (no var expansion)
safe_write "$CRATE_DIR/.github/workflows/ci.yml" "$(cat <<'YAML'
name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build -p ron-proto2
      - run: cargo fmt --all -- --check
      - run: cargo clippy -p ron-proto2 -- -D warnings
      - run: cargo test -p ron-proto2
  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: rustup component add miri
      - run: cargo +nightly miri test -p ron-proto2
YAML
)"

# fuzz.yml (no var expansion)
safe_write "$CRATE_DIR/.github/workflows/fuzz.yml" "$(cat <<'YAML'
name: fuzz
on:
  schedule:
    - cron: '0 3 * * *'
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: sudo apt-get update && sudo apt-get install -y clang llvm
      - run: cargo install cargo-fuzz || true
      - run: cd fuzz && cargo fuzz run decode_manifest -- -max_total_time=14400
YAML
)"

# render-mermaid.yml (no var expansion; keep ${...} literal)
safe_write "$CRATE_DIR/.github/workflows/render-mermaid.yml" "$(cat <<'YAML'
name: render-mermaid
on: [push, pull_request]
jobs:
  render:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @mermaid-js/mermaid-cli
      - run: |
          for f in $(git ls-files 'docs/*.mmd'); do
            out="${f%.mmd}.svg"
            mmdc -i "$f" -o "$out"
          done
YAML
)"

echo "Scaffold created at: $CRATE_DIR"
echo "Tip: add real content incrementally; keep files tiny and DTO-only."
