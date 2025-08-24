#!/usr/bin/env bash
set -euo pipefail

echo "[integrity] building…"
cargo build -q -p tldctl -p gateway

IDX="$(mktemp -d)"
STORE="$(mktemp -d)"
ADDR="$(target/debug/tldctl pack --tld text --input README.md --index-db "$IDX" --store-root "$STORE")"

# Find bundle & encoded file paths
HEX="${ADDR#b3:}"; HEX="${HEX%%.*}"; SHARD2="${HEX:0:2}"
BUNDLE="$STORE/objects/text/$SHARD2/$HEX.text"
ZST="$BUNDLE/payload.bin.zst"

# Start gateway and capture host:port
LOG="$(mktemp)"
target/debug/gateway --index-db "$IDX" >"$LOG" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true; wait $PID 2>/dev/null || true; rm -f "$LOG"' EXIT

for _ in {1..40}; do
  grep -Eo 'http://[0-9.]+:[0-9]+' "$LOG" >/dev/null && break
  sleep 0.25
done
HOSTPORT="$(grep -Eo 'http://[0-9.]+:[0-9]+' "$LOG" | head -n1 | sed 's#http://##')"
echo "[integrity] gateway: http://$HOSTPORT"
URL="http://$HOSTPORT/o/$ADDR/payload.bin"

echo "[integrity] before tamper (expect zstd)…"
curl -sSI -H 'Accept-Encoding: zstd' "$URL" | tr -d '\r' | egrep -i 'HTTP/|Content-Encoding:' || true

# Tamper: append one null byte → size+hash mismatch
printf '\x00' >> "$ZST"

echo "[integrity] after tamper (should fall back to identity — no Content-Encoding)…"
curl -sSI -H 'Accept-Encoding: zstd' "$URL" | tr -d '\r' | egrep -i 'HTTP/|Content-Encoding:' || true
