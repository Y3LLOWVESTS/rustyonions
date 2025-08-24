#!/usr/bin/env bash
# Payment enforcement smoke test for RustyOnions gateway
# - Builds tldctl + gateway
# - Packs a paywalled object into .objects and indexes in .data/index
# - Starts gateway WITH enforcement -> expects 402 on both endpoints
# - Starts gateway WITHOUT enforcement -> expects 200 on both endpoints (+ advisory headers)

set -euo pipefail

# -----------------------------
# Config (override via env vars)
# -----------------------------
BIN_TLDCTL="${BIN_TLDCTL:-target/debug/tldctl}"
BIN_GATEWAY="${BIN_GATEWAY:-target/debug/gateway}"
INDEX_DB="${INDEX_DB:-.data/index}"
STORE_ROOT="${STORE_ROOT:-.objects}"
BIND_ADDR="${BIND_ADDR:-127.0.0.1:0}"

# -----------------------------
# Helper: pick up printed URL
# -----------------------------
wait_for_listen() {
  # $1 = logfile path
  # prints URL to stdout, returns 0 if found, non-zero on timeout
  for _ in $(seq 1 100); do
    if grep -q 'gateway listening on http://' "$1"; then
      sed -n 's/.*listening on \(http:\/\/[^ ]*\).*/\1/p' "$1" | tail -n1
      return 0
    fi
    sleep 0.05
  done
  return 1
}

need_header() {
  # $1=header name, $2=headers file
  if ! grep -qi "^$1:" "$2"; then
    echo "[FAIL] missing header: $1"
    echo "----- response headers -----"
    cat "$2"
    echo "----------------------------"
    exit 1
  fi
}

# -----------------------------
# Step 0: build
# -----------------------------
echo "[*] Building tldctl + gateway…"
cargo build -p tldctl -p gateway >/dev/null

# -----------------------------
# Step 1: pack paywalled object
# -----------------------------
echo "[*] Packing paywalled object…"
mkdir -p "$INDEX_DB" "$STORE_ROOT"
echo "hello paywall" > /tmp/pay.txt

PACK_OUT="$("$BIN_TLDCTL" pack \
  --tld text \
  --input /tmp/pay.txt \
  --index-db "$INDEX_DB" \
  --store-root "$STORE_ROOT" \
  --required \
  --currency USD \
  --price-model per_request \
  --price 0.001 \
  --wallet lnurlp://example.com/alice)"

# Your packer prints ONLY the address line, e.g.:
#   b3:<64-hex>.text
ADDR="$(printf "%s\n" "$PACK_OUT" \
  | grep -E '^b3:[0-9a-f]{64}\.[A-Za-z0-9_-]+$' \
  | tail -n1 \
  | tr -d '\r\n')"

if [ -z "${ADDR:-}" ]; then
  echo "[FAIL] Could not parse ADDR from pack output:"
  echo "$PACK_OUT"
  exit 1
fi
echo "[*] ADDR=$ADDR"

# -----------------------------
# Step 2: start gateway (enforced)
# -----------------------------
echo "[*] Starting gateway (enforcement ON)…"
LOG1="$(mktemp)"
"$BIN_GATEWAY" --index-db "$INDEX_DB" --bind "$BIND_ADDR" --enforce-payments >"$LOG1" 2>&1 &
PID1=$!

trap 'kill '"$PID1"' '"${PID2:-}"' >/dev/null 2>&1 || true' EXIT

BASE="$(wait_for_listen "$LOG1" || true)"
if [ -z "${BASE:-}" ]; then
  echo "[FAIL] Could not detect listening URL (enforced). Log:"
  cat "$LOG1"
  exit 1
fi
echo "[*] BASE(enforced)=$BASE"

# Expect 402 on both endpoints
H_M1="$(mktemp)"; H_P1="$(mktemp)"
C_M1="$(curl -sS -D "$H_M1" -o /dev/null -w '%{http_code}' "$BASE/o/$ADDR/Manifest.toml")"
C_P1="$(curl -sS -D "$H_P1" -o /dev/null -w '%{http_code}' "$BASE/o/$ADDR/payload.bin")"

echo "[*] Enforced responses: manifest=$C_M1 payload=$C_P1"
[ "$C_M1" = "402" ] || { echo "[FAIL] expected 402 for manifest"; exit 1; }
[ "$C_P1" = "402" ] || { echo "[FAIL] expected 402 for payload"; exit 1; }

# Ensure advisory headers are present
for H in X-Payment-Currency X-Payment-Price-Model X-Payment-Price X-Payment-Wallet; do
  need_header "$H" "$H_M1"
  need_header "$H" "$H_P1"
done

# -----------------------------
# Step 3: restart gateway (no enforcement)
# -----------------------------
echo "[*] Restarting gateway (enforcement OFF)…"
kill "$PID1" >/dev/null 2>&1 || true
sleep 0.2

LOG2="$(mktemp)"
"$BIN_GATEWAY" --index-db "$INDEX_DB" --bind "$BIND_ADDR" >"$LOG2" 2>&1 &
PID2=$!

BASE2="$(wait_for_listen "$LOG2" || true)"
if [ -z "${BASE2:-}" ]; then
  echo "[FAIL] Could not detect listening URL (unenforced). Log:"
  cat "$LOG2"
  exit 1
fi
echo "[*] BASE(unenforced)=$BASE2"

# Expect 200 OK on both endpoints
H_M2="$(mktemp)"; H_P2="$(mktemp)"
C_M2="$(curl -sS -D "$H_M2" -o /dev/null -w '%{http_code}' "$BASE2/o/$ADDR/Manifest.toml")"
C_P2="$(curl -sS -D "$H_P2" -o /dev/null -w '%{http_code}' "$BASE2/o/$ADDR/payload.bin")"

echo "[*] Unenforced responses: manifest=$C_M2 payload=$C_P2"
[ "$C_M2" = "200" ] || { echo "[FAIL] expected 200 for manifest"; exit 1; }
[ "$C_P2" = "200" ] || { echo "[FAIL] expected 200 for payload"; exit 1; }

# Check for advisory headers (payload path should still include them)
for H in X-Payment-Currency X-Payment-Price-Model X-Payment-Price X-Payment-Wallet; do
  need_header "$H" "$H_P2"
done

echo "[PASS] smoke_402: enforcement gates with 402 and falls back to 200 when disabled."
