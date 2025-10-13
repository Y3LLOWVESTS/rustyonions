#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=${RUST_LOG:-info}
export METRICS_ADDR=${METRICS_ADDR:-127.0.0.1:9600}
echo "(placeholder) run svc-mailbox2 with config ./configs/svc-mailbox.toml"

