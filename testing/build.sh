#!/usr/bin/env bash
set -euo pipefail
echo "[build] fmt, clippy -D warnings, and full workspace build…"
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets --all-features
echo "[build] OK ✅"
