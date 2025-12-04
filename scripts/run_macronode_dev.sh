#!/usr/bin/env bash
set -euo pipefail

# Admin HTTP port (macronode /healthz, /readyz, /metrics, etc.)
ADMIN_PORT="${ADMIN_PORT:-8080}"

# Gateway HTTP port (what the SDK hits, e.g. 8090)
GATEWAY_PORT="${GATEWAY_PORT:-8090}"

echo "Starting macronode with:"
echo "  Admin HTTP   : http://127.0.0.1:${ADMIN_PORT}"
echo "  Gateway HTTP : http://127.0.0.1:${GATEWAY_PORT}"
echo

export RUST_LOG="${RUST_LOG:-info,macronode=debug}"
export RON_HTTP_ADDR="127.0.0.1:${ADMIN_PORT}"
export RON_GATEWAY_ADDR="127.0.0.1:${GATEWAY_PORT}"

cargo run -p macronode -- run --config macronode.toml
