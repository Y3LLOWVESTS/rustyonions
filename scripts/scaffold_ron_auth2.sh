#!/usr/bin/env bash
# Scaffolds the code-free crate structure for crates/ron-auth2.
# Usage:
#   bash scripts/scaffold_ron_auth2.sh [--readme-src PATH] [--alldocs-src PATH] [--force]
# Notes:
# - Creates minimal placeholder files; no implementation code is generated.
# - If --readme-src/--alldocs-src are provided, copies those files into place.
# - Will not overwrite existing files unless --force is specified.

set -euo pipefail

CRATE_DIR="crates/ron-auth2"
FORCE=0
README_SRC=""
ALLDOCS_SRC=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force) FORCE=1; shift ;;
    --readme-src) README_SRC="${2:-}"; shift 2 ;;
    --alldocs-src) ALLDOCS_SRC="${2:-}"; shift 2 ;;
    *)
      echo "Unknown arg: $1"
      exit 2
      ;;
  esac
done

# Helpers
ensure_dir() {
  mkdir -p "$1"
}

write_file() {
  # write_file <path> <content>
  local path="$1"
  local content="$2"
  if [[ -e "$path" && $FORCE -ne 1 ]]; then
    echo "exists: $path (skip; use --force to overwrite)"
    return 0
  fi
  ensure_dir "$(dirname "$path")"
  printf "%s" "$content" > "$path"
  echo "wrote:  $path"
}

copy_if_provided() {
  # copy_if_provided <src> <dst>
  local src="$1"
  local dst="$2"
  if [[ -n "$src" ]]; then
    if [[ ! -f "$src" ]]; then
      echo "WARN: source not found: $src (writing placeholder instead)"
      return 1
    fi
    if [[ -e "$dst" && $FORCE -ne 1 ]]; then
      echo "exists: $dst (skip copy; use --force to overwrite)"
      return 0
    fi
    ensure_dir "$(dirname "$dst")"
    cp "$src" "$dst"
    echo "copied: $src -> $dst"
    return 0
  fi
  return 1
}

# Create root crate dir
ensure_dir "$CRATE_DIR"

# Root files
write_file "$CRATE_DIR/.gitignore" "$(cat <<'EOF'
/target
**/*.rs.bk
Cargo.lock
.idea
.vscode
*.profraw
EOF
)"

write_file "$CRATE_DIR/.editorconfig" "$(cat <<'EOF'
root = false

[*]
end_of_line = lf
insert_final_newline = true
charset = utf-8
indent_style = space
indent_size = 2

[*.rs]
indent_size = 4
EOF
)"

write_file "$CRATE_DIR/rustfmt.toml" "$(cat <<'EOF'
max_width = 100
edition = "2021"
use_field_init_shorthand = true
newline_style = "Unix"
EOF
)"

write_file "$CRATE_DIR/LICENSE-APACHE" "Apache License 2.0 (placeholder)"
write_file "$CRATE_DIR/LICENSE-MIT" "MIT License (placeholder)"

# Cargo.toml (placeholder; no deps yet—pure lib + features skeleton)
write_file "$CRATE_DIR/Cargo.toml" "$(cat <<'EOF'
[package]
name = "ron-auth2"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Pure verification & attenuation library—capability-first, deterministic, fail-closed."
repository = ""
rust-version = "1.80"

[lib]
path = "src/lib.rs"

[features]
default = []
pq = []
pq-hybrid = []
pq-only = []
config-env = []
zk = []

[dependencies]
# intentionally minimal; add real deps during implementation

[dev-dependencies]
# test/bench/fuzz deps go here later
EOF
)"

# README + CHANGELOG
if ! copy_if_provided "$README_SRC" "$CRATE_DIR/README.md"; then
  write_file "$CRATE_DIR/README.md" "$(cat <<'EOF'
# ron-auth2

Pure verification & attenuation library (no I/O). This is a scaffold placeholder; see docs in `docs/`.
EOF
)"
fi

write_file "$CRATE_DIR/CHANGELOG.md" "$(cat <<'EOF'
# Changelog — ron-auth2

## [0.1.0] - Unreleased
- Initial scaffold (no implementation code).
EOF
)"

# CODEOWNERS / CONTRIBUTING
write_file "$CRATE_DIR/CODEOWNERS" "$(cat <<'EOF'
# Adjust GitHub handles as appropriate
* @stevanwhite
EOF
)"

write_file "$CRATE_DIR/CONTRIBUTING.md" "$(cat <<'EOF'
# Contributing — ron-auth2

- No I/O, no `unsafe`, no network or storage dependencies.
- Public API (types, reason strings, metric names) is SemVer-stable.
- Changes to API require vector updates and SemVer checks in CI.
EOF
)"

# deny.toml (tightened skeleton)
write_file "$CRATE_DIR/deny.toml" "$(cat <<'EOF'
[advisories]
yanked = "deny"
ignore = []

[bans]
multiple-versions = "warn"
wildcards = "deny"
deny = [
  { name = "tokio" },
  { name = "hyper" },
  { name = "reqwest" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]

[licenses]
unlicensed = "deny"
allow = [
  "MIT",
  "Apache-2.0",
  "Unicode-3.0",
  "Unicode-DFS-2016",
  "CC0-1.0",
  "CDLA-Permissive-2.0",
  "OpenSSL"
]
EOF
)"

# GitHub workflows
write_file "$CRATE_DIR/.github/workflows/ci.yml" "$(cat <<'EOF'
name: CI (ron-auth2)

on:
  push:
    paths:
      - "crates/ron-auth2/**"
  pull_request:
    paths:
      - "crates/ron-auth2/**"

jobs:
  build-test:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: crates/ron-auth2
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo build --features ""
      - run: cargo test --features ""
      - run: echo "placeholder: cargo-deny/public-api checks wired later"
EOF
)"

write_file "$CRATE_DIR/.github/workflows/pq-matrix.yml" "$(cat <<'EOF'
name: PQ Matrix (ron-auth2)

on:
  push:
    paths:
      - "crates/ron-auth2/**"
  pull_request:
    paths:
      - "crates/ron-auth2/**"

jobs:
  matrix:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features: ["", "pq", "pq-hybrid", "pq-only"]
    defaults:
      run:
        working-directory: crates/ron-auth2
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.80.0
      - run: cargo build --features "${{ matrix.features }}"
      - run: cargo test --features "${{ matrix.features }}"
EOF
)"

# Docs
write_file "$CRATE_DIR/docs/README.md" "# Documentation — ron-auth2\n\nThis folder contains the per-topic docs split from ALL_DOCS.md."
if ! copy_if_provided "$ALLDOCS_SRC" "$CRATE_DIR/docs/ALL_DOCS.md"; then
  write_file "$CRATE_DIR/docs/ALL_DOCS.md" "ALL_DOCS placeholder — provide your combined docs here."
fi
for f in IDB.md API.md CONFIG.md INTEROP.md OBSERVABILITY.md PERFORMANCE.md QUANTUM.md SECURITY.md RUNBOOK.md; do
  write_file "$CRATE_DIR/docs/$f" "# $f — placeholder\n\nSplit this from ALL_DOCS.md when ready."
done
for d in arch.mmd sequence.mmd state.mmd; do
  write_file "$CRATE_DIR/docs/diagrams/$d" "% mermaid $d placeholder"
done

# src tree (code-free placeholders)
write_file "$CRATE_DIR/src/lib.rs" "$(cat <<'EOF'
//! ron-auth2 — pure verification & attenuation (no I/O)
//! This is a scaffold placeholder. Public surface will be re-exported here.

pub mod prelude;
pub mod capability;
pub mod verify;
pub mod caveats;
pub mod config;
pub mod keys;
pub mod pq;
pub mod zk;
pub mod ctx;
pub mod metrics;
pub mod redact;
EOF
)"

write_file "$CRATE_DIR/src/prelude.rs" "// prelude placeholder — re-export common types later"

# capability/
write_file "$CRATE_DIR/src/capability/mod.rs" "// capability::mod placeholder"
write_file "$CRATE_DIR/src/capability/scope.rs" "// capability::scope placeholder"
write_file "$CRATE_DIR/src/capability/caveat.rs" "// capability::caveat placeholder"
write_file "$CRATE_DIR/src/capability/encode.rs" "// capability::encode placeholder"

# verify/
write_file "$CRATE_DIR/src/verify/mod.rs" "// verify::mod placeholder — verify_token signature lives here later"
write_file "$CRATE_DIR/src/verify/decision.rs" "// verify::decision placeholder"
write_file "$CRATE_DIR/src/verify/error.rs" "// verify::error placeholder"
write_file "$CRATE_DIR/src/verify/pipeline.rs" "// verify::pipeline placeholder"

# caveats/
write_file "$CRATE_DIR/src/caveats/mod.rs" "// caveats::mod placeholder"
write_file "$CRATE_DIR/src/caveats/builtin.rs" "// caveats::builtin placeholder"
write_file "$CRATE_DIR/src/caveats/registry.rs" "// caveats::registry placeholder"
write_file "$CRATE_DIR/src/caveats/custom.rs" "// caveats::custom placeholder"

# config/
write_file "$CRATE_DIR/src/config/mod.rs" "// config::mod placeholder"
write_file "$CRATE_DIR/src/config/verifier_config.rs" "// config::verifier_config placeholder"
write_file "$CRATE_DIR/src/config/env.rs" "// config::env placeholder (feature: config-env)"

# keys/
write_file "$CRATE_DIR/src/keys/mod.rs" "// keys::mod placeholder"
write_file "$CRATE_DIR/src/keys/traits.rs" "// keys::traits placeholder"
write_file "$CRATE_DIR/src/keys/mac_handle.rs" "// keys::mac_handle placeholder"

# pq/
write_file "$CRATE_DIR/src/pq/mod.rs" "// pq::mod placeholder (feature-gated)"
write_file "$CRATE_DIR/src/pq/sig_adapter.rs" "// pq::sig_adapter placeholder (feature-gated)"

# zk/
write_file "$CRATE_DIR/src/zk/mod.rs" "// zk::mod placeholder (feature-gated)"

# misc core
write_file "$CRATE_DIR/src/ctx.rs" "// ctx placeholder"
write_file "$CRATE_DIR/src/metrics.rs" "// metrics hook trait placeholder"
write_file "$CRATE_DIR/src/redact.rs" "// redact helpers placeholder"

# tests
write_file "$CRATE_DIR/tests/allow_deny_vectors.rs" "// test: allow/deny vectors placeholder"
write_file "$CRATE_DIR/tests/attenuation_monotonicity.rs" "// test: attenuation monotonicity placeholder"
write_file "$CRATE_DIR/tests/parser_fixtures.rs" "// test: parser fixtures placeholder"
write_file "$CRATE_DIR/tests/compat_public_api.rs" "// test: public API stability placeholder"
write_file "$CRATE_DIR/tests/amnesia_mode.rs" "// test: amnesia mode placeholder"
write_file "$CRATE_DIR/tests/loom_verify.rs" "// test: loom concurrency model placeholder"

# benches
write_file "$CRATE_DIR/benches/verify_bench.rs" "// bench: verify latency/alloc placeholder"

# fuzz
write_file "$CRATE_DIR/fuzz/fuzz_targets/token_parser_fuzz.rs" "// fuzz target: token parser placeholder"

# vectors & replay
write_file "$CRATE_DIR/testing/vectors/ron-auth/v1/allow_example.json" "{ \"placeholder\": \"allow_example\" }"
write_file "$CRATE_DIR/testing/vectors/ron-auth/v1/deny_expired.json" "{ \"placeholder\": \"deny_expired\" }"
write_file "$CRATE_DIR/testing/vectors/ron-auth/v1/deny_unknown_kid.json" "{ \"placeholder\": \"deny_unknown_kid\" }"
write_file "$CRATE_DIR/testing/vectors/ron-auth/v1/custom_ns_examples.json" "{ \"placeholder\": \"custom_ns_examples\" }"
write_file "$CRATE_DIR/testing/vectors/ron-auth/v1/pq_allow.json" "{ \"placeholder\": \"pq_allow\" }"
write_file "$CRATE_DIR/testing/vectors/ron-auth/v1/pq_deny_mismatch.json" "{ \"placeholder\": \"pq_deny_mismatch\" }"

write_file "$CRATE_DIR/testing/replay/README.md" "$(cat <<'EOF'
# Vector Replay — ron-auth2

This folder hosts polyglot vector replay runners (Python/TS) to ensure SDK parity.
EOF
)"
write_file "$CRATE_DIR/testing/replay/python/placeholder.txt" "python runner placeholder"
write_file "$CRATE_DIR/testing/replay/typescript/placeholder.txt" "typescript runner placeholder"

echo "Done. Scaffold created under $CRATE_DIR"
