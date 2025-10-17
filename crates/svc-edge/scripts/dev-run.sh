#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
RUST_LOG=${RUST_LOG:-info} \
SVC_EDGE_BIND_ADDR=${SVC_EDGE_BIND_ADDR:-0.0.0.0:8080} \
SVC_EDGE_METRICS_ADDR=${SVC_EDGE_METRICS_ADDR:-127.0.0.1:9909} \
SVC_EDGE_SECURITY__AMNESIA=${SVC_EDGE_SECURITY__AMNESIA:-true} \
cargo run -p svc-edge2 -- \
  --config ./configs/svc-edge.toml
