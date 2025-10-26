#!/usr/bin/env bash
# RO:WHAT — local loopback perf smoke (placeholder)
set -euo pipefail
RUST_LOG=${RUST_LOG:-info} cargo run -p ron-transport --example bench_echo || true
