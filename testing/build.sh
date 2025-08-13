#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo build --workspace
