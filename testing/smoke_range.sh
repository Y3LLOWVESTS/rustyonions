#!/usr/bin/env bash
# Range/206 smoke test for RustyOnions gateway
# - Packs a known payload
# - Confirms 206 with proper Content-Range for two patterns:
#   1) bytes=0-3
#   2) bytes=-5 (suffix)
set -euo pipefail

BIN_TLDCTL="${BIN_TLDCTL:-target/debug/tldctl}"
BIN_GATEWAY="${BIN_GATEWAY:-target/debug/gateway}"
INDEX_DB="${INDEX_DB:-.data/index}"
STORE_ROOT="${STORE_ROOT:-.objects}"
BIND_ADDR="${BIND_ADDR:-127.0.0.1:0}"

wait_for_listen() {
  for _ in $(seq 1 100); do
    if grep -q 'gateway listening on http://' "$1"; then
      sed -n 's/.*listening on \(http:\/\/[^ ]*\).*/\1/p' "$1" | tail -n1
      return 0
    fi
    sleep 0.05
  done
  return 1
}

echo "[*] Building…"
cargo build -p tldctl -p gateway >/dev/null

mkdir -p "$INDEX_DB" "$STORE_ROOT"

# Payload with known size 26 bytes: "abcdefghijklmnopqrstuvwxyz"
printf "abcdefghijklmnopqrstuvwxyz" > /tmp/range.bin
TOTAL=$(wc -c </tmp/range.bin | tr -d ' ')
echo "[*] TOTAL=$TOTAL"

echo "[*] Packing…"
PACK_OUT="$("$BIN_TLDCTL" pack \
  --tld bin \
  --input /tmp/range.bin \
  --index-db "$INDEX_DB" \
  --store-root "$STORE_ROOT")"

ADDR="$(printf "%s\n" "$PACK_OUT" | grep -E '^b3:[0-9a-f]{64}\.[A-Za-z0-9_-]+$' | tail -n1)"
[ -n "$ADDR" ] || { echo "[FAIL] could not parse ADDR"; echo "$PACK_OUT"; exit 1; }
echo "[*] ADDR=$ADDR"

echo "[*] Starting gateway (no enforcement)…"
LOG="$(mktemp)"
"$BIN_GATEWAY" --index-db "$INDEX_DB" --bind "$BIND_ADDR" >"$LOG" 2>&1 &
PID=$!
trap 'kill '"$PID"' >/dev/null 2>&1 || true' EXIT

BASE="$(wait_for_listen "$LOG" || true)"
[ -n "$BASE" ] || { echo "[FAIL] could not detect BASE"; cat "$LOG"; exit 1; }
echo "[*] BASE=$BASE"

# Test 1: bytes=0-3
H1="$(mktemp)"; B1="$(mktemp)"
C1="$(curl -sS -D "$H1" -o "$B1" -H 'Range: bytes=0-3' -w '%{http_code}' "$BASE/o/$ADDR/payload.bin")"
echo "[*] 0-3: HTTP=$C1 len=$(wc -c <"$B1")"
[ "$C1" = "206" ] || { echo "[FAIL] expected 206 for bytes=0-3"; exit 1; }
grep -qi '^Accept-Ranges: *bytes' "$H1" || { echo "[FAIL] missing Accept-Ranges"; exit 1; }
grep -qi "^Content-Range: *bytes 0-3/$TOTAL" "$H1" || { echo "[FAIL] bad Content-Range (0-3)"; exit 1; }
[ "$(wc -c <"$B1" | tr -d ' ')" = "4" ] || { echo "[FAIL] body length != 4"; exit 1; }

# Test 2: bytes=-5 (suffix)
H2="$(mktemp)"; B2="$(mktemp)"
C2="$(curl -sS -D "$H2" -o "$B2" -H 'Range: bytes=-5' -w '%{http_code}' "$BASE/o/$ADDR/payload.bin")"
START=$((TOTAL-5)); END=$((TOTAL-1))
echo "[*] -5: HTTP=$C2 len=$(wc -c <"$B2")"
[ "$C2" = "206" ] || { echo "[FAIL] expected 206 for bytes=-5"; exit 1; }
grep -qi "^Content-Range: *bytes $START-$END/$TOTAL" "$H2" || { echo "[FAIL] bad Content-Range (-5)"; exit 1; }
[ "$(wc -c <"$B2" | tr -d ' ')" = "5" ] || { echo "[FAIL] body length != 5"; exit 1; }

echo "[PASS] smoke_range: byte ranges work."
