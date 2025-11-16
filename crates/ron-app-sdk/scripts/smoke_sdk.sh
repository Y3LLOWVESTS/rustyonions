#!/usr/bin/env bash
set -euo pipefail

# ron-app-sdk smoke: keep this crate honest without touching the rest
# of the workspace. Intended to be fast enough to run often.

echo "[STEP] fmt + clippy + unit tests"
cargo fmt -p ron-app-sdk
cargo clippy -p ron-app-sdk --no-deps -- -D warnings
cargo test  -p ron-app-sdk --lib

# NOTE: When we add a mock gateway + integration tests, extend this
# script with a small spawn/kill harness here (see NOTES.MD).
# For beta, unit tests are our DoD.

echo "[OK] ron-app-sdk smoke passed"
