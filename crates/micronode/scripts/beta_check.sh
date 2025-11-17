#!/usr/bin/env bash
set -euo pipefail

# Simple beta gate for `micronode`.
#
# Runs (from repo root):
#   1) fmt + clippy (warnings = errors)
#   2) unit + integration tests
#   3) HTTP+KV benchmarks (http_kv)
#   4) Smoke test via scripts/smoke_micronode.sh with dev routes enabled
#
# Usage:
#   bash crates/micronode/scripts/beta_check.sh
#
# Env toggles:
#   SKIP_BENCH=1  → skip Criterion benches (useful on battery / CI)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="micronode"

echo "[beta-check] crate=${CRATE} root=${ROOT_DIR}"

cd "${ROOT_DIR}"

###############################################################################
# 1) fmt + clippy
###############################################################################
echo "[beta-check] 1) fmt + clippy"
cargo fmt -p "${CRATE}"
cargo clippy -p "${CRATE}" --no-deps -- -D warnings

###############################################################################
# 2) Tests (unit + integration + doc tests)
###############################################################################
echo "[beta-check] 2) cargo test -p ${CRATE}"
cargo test -p "${CRATE}"

###############################################################################
# 3) HTTP+KV benchmarks (optional)
###############################################################################
if [[ "${SKIP_BENCH:-0}" != "1" ]]; then
  echo "[beta-check] 3) cargo bench -p ${CRATE} --bench http_kv"
  cargo bench -p "${CRATE}" --bench http_kv
else
  echo "[beta-check] 3) benches skipped (SKIP_BENCH=1)"
fi

###############################################################################
# 4) Smoke test (admin plane + KV roundtrip)
###############################################################################
echo "[beta-check] 4) smoke_micronode.sh (with MICRONODE_DEV_ROUTES=1)"

MICRONODE_DEV_ROUTES=1 cargo run -p "${CRATE}" &
APP_PID=$!

# Give the server a moment to bind.
sleep 2

# Run the smoke script (expects 127.0.0.1:5310 by default).
bash "crates/${CRATE}/scripts/smoke_micronode.sh"

echo "[beta-check] smoke_micronode.sh OK, shutting down micronode (pid=${APP_PID})"
kill "${APP_PID}" >/dev/null 2>&1 || true
wait "${APP_PID}" 2>/dev/null || true

echo "[beta-check] ✅ ${CRATE} beta gate PASSED"
