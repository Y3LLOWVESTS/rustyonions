#!/usr/bin/env bash
# Run the gateway's free_vs_paid test in "online" mode against a running gateway.
# Requires env pointing at a live gateway and two object addresses (one free, one paid).
# Example:
#   BIND=127.0.0.1:9080 \
#   GW_BASE_URL=http://127.0.0.1:9080 \
#   GW_FREE_ADDR=b3:...text \
#   GW_PAID_ADDR=b3:...post \
#   testing/run_free_vs_paid.sh

set -euo pipefail

GW_BASE_URL="${GW_BASE_URL:-http://127.0.0.1:9080}"
: "${GW_BASE_URL:?Set GW_BASE_URL (e.g., http://127.0.0.1:9080)}"

if [[ -z "${GW_FREE_ADDR:-}" || -z "${GW_PAID_ADDR:-}" ]]; then
  cat <<EOF
[run_free_vs_paid] Missing addresses.
Export env like:

  export GW_BASE_URL=${GW_BASE_URL}
  export GW_FREE_ADDR="b3:...your_free_addr...text"   # Often printed as OBJ_ADDR or FREE_ADDR
  export GW_PAID_ADDR="b3:...your_paid_addr...post"   # Sometimes printed as POST_ADDR

Then rerun:
  testing/run_free_vs_paid.sh
EOF
  exit 2
fi

echo "[run_free_vs_paid] Using:"
echo "  GW_BASE_URL=${GW_BASE_URL}"
echo "  GW_FREE_ADDR=${GW_FREE_ADDR}"
echo "  GW_PAID_ADDR=${GW_PAID_ADDR}"

cargo test -p gateway --test free_vs_paid -- --nocapture
