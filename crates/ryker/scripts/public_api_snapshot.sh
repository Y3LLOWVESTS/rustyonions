#!/usr/bin/env bash
set -euo pipefail
cd "./crates/.."
if ! command -v cargo-public-api >/dev/null 2>&1; then
  echo "cargo-public-api not found (cargo install cargo-public-api)"
  exit 1
fi
cargo public-api --manifest-path crates/ryker/Cargo.toml --simplified   > crates/ryker/docs/api-history/ryker/vX.Y.Z.txt
echo "updated public API snapshot -> docs/api-history/ryker/vX.Y.Z.txt"

