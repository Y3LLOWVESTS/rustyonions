#!/usr/bin/env bash
set -euo pipefail

# 60‑second smoke test for index → gateway path
# - Packs a file with tldctl (unique temp index/store to avoid Sled locks)
# - Starts gateway, parses its bind URL from stdout
# - Fetches Manifest.toml and payload.bin
# - Cleans up the gateway process

INPUT="${1:-README.md}"
if [[ ! -f "$INPUT" ]]; then
  echo "[smoke] ERROR: Input file '$INPUT' not found" >&2
  exit 1
fi

IDX="$(mktemp -d)"
STORE="$(mktemp -d)"
LOG="$(mktemp)"

cleanup() {
  if [[ -n "${GATEWAY_PID:-}" ]]; then
    kill "${GATEWAY_PID}" 2>/dev/null || true
    wait "${GATEWAY_PID}" 2>/dev/null || true
  fi
  rm -f "$LOG"
  # Leave IDX/STORE for post‑mortem; uncomment to auto‑remove:
  # rm -rf "$IDX" "$STORE"
}
trap cleanup EXIT

echo "[smoke] Index dir:  $IDX"
echo "[smoke] Store dir:  $STORE"

ADDR="$(target/debug/tldctl pack \
  --tld text \
  --input "$INPUT" \
  --index-db "$IDX" \
  --store-root "$STORE")"
echo "[smoke] Packed address: $ADDR"

echo "[smoke] Starting gateway..."
# Capture stdout/stderr to a log so we can parse the bind URL
target/debug/gateway --index-db "$IDX" >"$LOG" 2>&1 &
GATEWAY_PID=$!

# Wait up to ~10s for a "listening on http://HOST:PORT" line
HOSTPORT=""
for _ in {1..20}; do
  if grep -Eo 'listening on http://[0-9.]+:[0-9]+' "$LOG" >/dev/null; then
    HOSTPORT="$(grep -Eo 'listening on http://[0-9.]+:[0-9]+' "$LOG" | head -n1 | sed -E 's/.*http:\/\/([0-9.]+:[0-9]+).*/\1/')"
    break
  fi
  sleep 0.5
done

if [[ -z "$HOSTPORT" ]]; then
  echo "[smoke] ERROR: Could not detect gateway bind address from logs:"
  sed -n '1,120p' "$LOG" >&2
  exit 1
fi

BASE_URL="http://$HOSTPORT"
echo "[smoke] Gateway: $BASE_URL"

# Verify Manifest.toml
echo "[smoke] Fetching manifest..."
curl -sSf "$BASE_URL/o/$ADDR/Manifest.toml" | head -n 10

# Verify payload.bin bytes flow
echo "[smoke] Fetching payload (first 64 bytes)…"
curl -sSf "$BASE_URL/o/$ADDR/payload.bin" | head -c 64 | hexdump -C

echo "[smoke] ✅ smoke test completed"
