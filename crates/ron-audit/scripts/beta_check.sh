#!/usr/bin/env bash
set -euo pipefail

# Simple beta gate for ron-audit.
# Runs:
#   1) fmt + clippy + unit tests
#   2) benches for hash_b3, verify_chain, wal_batching with saved baselines
#
# Usage (from repo root):
#   bash crates/ron-audit/scripts/beta_check.sh
#   BASE=local-dev bash crates/ron-audit/scripts/beta_check.sh
#
# Notes:
#   - Baseline suffix defaults to YYYYMMDD if BASE is not set.
#   - This script does NOT enable any cargo features by default
#     (run simd benches separately if needed).

CRATE="ron-audit"

# Resolve repo root (two levels up from scripts/)
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

BASE_SUFFIX="${BASE:-$(date +%Y%m%d)}"

echo "[beta-check] crate=${CRATE} root=${ROOT_DIR}"
echo "[beta-check] 1) fmt + clippy + unit tests"

cargo fmt -p "${CRATE}"
cargo clippy -p "${CRATE}" --no-deps -- -D warnings
cargo test -p "${CRATE}"

echo "[beta-check] 2) benches (hash_b3, verify_chain, wal_batching)"

cargo bench -p "${CRATE}" --bench hash_b3 \
  -- --save-baseline "audit-hash_b3-${BASE_SUFFIX}"

cargo bench -p "${CRATE}" --bench verify_chain \
  -- --save-baseline "audit-verify_chain-${BASE_SUFFIX}"

cargo bench -p "${CRATE}" --bench wal_batching \
  -- --save-baseline "audit-wal_batching-${BASE_SUFFIX}"

echo "[beta-check] done."
