#!/usr/bin/env bash
set -euo pipefail

IDX="$(mktemp -d)"
STORE="$(mktemp -d)"
INPUT="${1:-README.md}"

ADDR="$(target/debug/tldctl pack --tld text --input "$INPUT" --index-db "$IDX" --store-root "$STORE")"
echo "[enc] addr: $ADDR"

# Start gateway
LOG="$(mktemp)"
target/debug/gateway --index-db "$IDX" >"$LOG" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true; wait $PID 2>/dev/null || true; rm -f "$LOG"' EXIT

for _ in {1..20}; do
  if grep -Eo 'http://[0-9.]+:[0-9]+' "$LOG" >/dev/null; then break; fi
  sleep 0.25
done
BASE="http://$(grep -Eo 'http://[0-9.]+:[0-9]+' "$LOG" | head -n1 | sed 's#http://##')"
echo "[enc] gateway: $BASE"

echo "[enc] Expect zstd..."
curl -sSI -H 'Accept-Encoding: zstd' "$BASE/o/$ADDR/payload.bin" | tr -d '\r' | egrep -i 'HTTP/|Content-Encoding:'
echo "[enc] Expect br..."
curl -sSI -H 'Accept-Encoding: br' "$BASE/o/$ADDR/payload.bin"   | tr -d '\r' | egrep -i 'HTTP/|Content-Encoding:'
echo "[enc] Expect identity..."
curl -sSI "$BASE/o/$ADDR/payload.bin"                            | tr -d '\r' | egrep -i 'HTTP/|Content-Encoding:|^$' | sed '/Content-Encoding:/d'
echo "[enc] ✅ done"
