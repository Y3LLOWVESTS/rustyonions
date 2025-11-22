#!/usr/bin/env bash
# RO:WHAT — Extract unique Prometheus metric names from /metrics.
# RO:WHY  — Quick sanity check on exposed metrics without digging through all samples.
# RO:INVARIANTS —
#   - Does not assume any specific metric set; just prints the current names.
#   - Uses RON_HTTP_ADDR if set, otherwise 127.0.0.1:8080.

set -euo pipefail

ADDR="${RON_HTTP_ADDR:-127.0.0.1:8080}"
BASE="http://${ADDR}"

say() { printf '[macronode] %s\n' "$*"; }

say "Fetching metrics from ${BASE}/metrics"

curl -sS "${BASE}/metrics" \
  | grep -E '^[a-zA-Z_][a-zA-Z0-9_:]*[[:space:]]' \
  | cut -d' ' -f1 \
  | sort -u

say "Done."
