#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=${RUST_LOG:-info}
export SVC_INTEROP_BIND_ADDR=${SVC_INTEROP_BIND_ADDR:-0.0.0.0:8080}
export SVC_INTEROP_METRICS_ADDR=${SVC_INTEROP_METRICS_ADDR:-127.0.0.1:9909}
echo "svc-interop2 scaffold - no service logic yet"
