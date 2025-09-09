#!/usr/bin/env bash
# testing/redteam_slowloris.sh — exercise server slow-read defenses
# Usage: ADDR=127.0.0.1:9444 ./testing/redteam_slowloris.sh
set -euo pipefail

ADDR="${ADDR:-127.0.0.1:9444}"
HOST="${ADDR%:*}"
PORT="${ADDR##*:}"

command -v nc >/dev/null 2>&1 || { echo "nc (netcat) is required"; exit 1; }

echo "[*] Connecting to $HOST:$PORT and trickling bytes (Ctrl-C to stop)"
# Trickle a HELLO payload very slowly; server should drop on idle/low-throughput.
# Note: payload length is intentionally mismatched to provoke early rejection.
{
  printf '\x00\x00\x00\x20'  # claim 32 bytes payload
  printf '\x01'                 # type = HELLO
  printf '{"proto":"OAP/1",'     # partial JSON
  sleep 2
  printf '"accept_max_frame":1}' # finish later
  sleep 5
} | nc -v "$HOST" "$PORT" || true
echo "[*] Done (server should have dropped the connection)"
