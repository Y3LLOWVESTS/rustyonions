#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   testing/serve_once.sh <INDEX_DB> [ADDR]
#
# If ADDR is provided, script will curl /o/$ADDR/payload.bin and show headers.
# Otherwise, it just prints the host:port and exits.

IDX="${1:?index dir required}"
ADDR="${2:-}"

LOG="$(mktemp)"
target/debug/gateway --index-db "$IDX" >"$LOG" 2>&1 &
PID=$!

cleanup() {
  kill "$PID" 2>/dev/null || true
  wait "$PID" 2>/dev/null || true
  rm -f "$LOG"
}
trap cleanup EXIT

# Wait for "listening on http://…"
for _ in {1..40}; do
  if grep -Eo 'http://[0-9.]+:[0-9]+' "$LOG" >/dev/null; then break; fi
  sleep 0.25
done

HOSTPORT="$(grep -Eo 'http://[0-9.]+:[0-9]+' "$LOG" | head -n1 | sed 's#http://##')"
echo "[gateway] $HOSTPORT"

# If ADDR provided, curl it
if [[ -n "$ADDR" ]]; then
  echo "[curl] http://$HOSTPORT/o/$ADDR/payload.bin"
  curl -sSI "http://$HOSTPORT/o/$ADDR/payload.bin" | grep -Ei 'HTTP/|Content-Type|ETag'
fi
