#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   crates/ron-kms/scripts/bench.sh        # dalek lane
#   FAST=1 crates/ron-kms/scripts/bench.sh # ring lane

export RUSTFLAGS="-C target-cpu=native"

# robust under set -u: use string not array
FEATURES_ARGS=""
if [[ "${FAST:-0}" == "1" ]]; then
  FEATURES_ARGS="--features fast"
fi

cargo bench -p ron-kms $FEATURES_ARGS \
  --bench sign_bench \
  --bench verify_bench \
  --bench batch_verify \
  --bench parallel_throughput \
  -- --sample-size 120 --measurement-time 10 --warm-up-time 3
