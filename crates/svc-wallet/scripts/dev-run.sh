#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=${RUST_LOG:-info}
export SVC_WALLET_BIND_ADDR=${SVC_WALLET_BIND_ADDR:-0.0.0.0:8080}
export SVC_WALLET_METRICS_ADDR=${SVC_WALLET_METRICS_ADDR:-127.0.0.1:0}
cargo run -p svc-wallet2
