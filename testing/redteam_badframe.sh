#!/usr/bin/env bash
# testing/redteam_badframe.sh — send invalid OAP frame type to ensure rejection
# Usage: ADDR=127.0.0.1:9444 ./testing/redteam_badframe.sh
set -euo pipefail

ADDR="${ADDR:-127.0.0.1:9444}"
HOST="${ADDR%:*}"
PORT="${ADDR##*:}"

command -v nc >/dev/null 2>&1 || { echo "nc (netcat) is required"; exit 1; }

echo "[*] Sending invalid frame type 0xFF to $HOST:$PORT"
{
  # length = 1 (u32, big-endian), type = 0xFF (invalid)
  printf '\x00\x00\x00\x01'
  printf '\xFF'
} | nc -v "$HOST" "$PORT" || true
echo "[*] Done (expect an ERROR or drop without panic)"
