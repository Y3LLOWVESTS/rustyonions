#!/usr/bin/env bash
set -euo pipefail

# Simple smoke for ron-auth:
#  - fmt, clippy, tests
#  - builds the example
#  - runs the example with a provided or default b64url token
#
# Usage:
#   crates/ron-auth/scripts/smoke_auth.sh
#   crates/ron-auth/scripts/smoke_auth.sh "<base64url_token>"
#
# Env:
#   FEATURES   - Cargo features to use for build/test (default: "")
#   RUSTFLAGS  - Optional (e.g., -C target-cpu=native)
#   RAYON_NUM_THREADS - Optional for benches or future parallel runs

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$CRATE_DIR"

FEATURES="${FEATURES:-}"

echo "[INFO] crate dir = $CRATE_DIR"
echo "[INFO] features  = '${FEATURES}'"

echo "[STEP] cargo fmt"
cargo fmt -p ron-auth

echo "[STEP] cargo clippy (deny warnings)"
if [[ -n "$FEATURES" ]]; then
  cargo clippy -p ron-auth --features "$FEATURES" --all-targets -- -D warnings
else
  cargo clippy -p ron-auth --all-targets -- -D warnings
fi

echo "[STEP] cargo test"
if [[ -n "$FEATURES" ]]; then
  cargo test -p ron-auth --features "$FEATURES"
else
  cargo test -p ron-auth
fi

# Build the example (exists in examples/verify.rs)
echo "[STEP] build example: verify"
if [[ -n "$FEATURES" ]]; then
  cargo build -p ron-auth --features "$FEATURES" --example verify
else
  cargo build -p ron-auth --example verify
fi

# Default demo token: valid base64url JSON-ish payload; verification will likely return Deny
# unless it matches your StaticKeys/kid/tid and caveats.
DEFAULT_TOKEN='eyJkZW1vIjoidG9rZW4tZm9yLXJvbi1hdXRoLXNtb2tlIn0'

TOKEN="${1:-$DEFAULT_TOKEN}"

echo "[STEP] run example: verify (token length: ${#TOKEN})"
set +e
if [[ -n "$FEATURES" ]]; then
  cargo run -p ron-auth --features "$FEATURES" --example verify -- "$TOKEN"
else
  cargo run -p ron-auth --example verify -- "$TOKEN"
fi
STATUS=$?
set -e

# We don't fail the smoke if the example returns Deny/Err (non-zero). The goal is “it runs”.
if [[ $STATUS -ne 0 ]]; then
  echo "[WARN] example exited with status ${STATUS} (often OK if token doesn't match keys/caveats)"
else
  echo "[OK] example ran successfully"
fi

echo "[DONE] smoke_auth complete."
