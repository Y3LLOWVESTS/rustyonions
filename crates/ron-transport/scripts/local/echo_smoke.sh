#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="$(mktemp -t ron_transport_echo.XXXXXX.log)"
RUST_LOG=info cargo run -q -p ron-transport --example tcp_echo >"$LOG_FILE" 2>&1 &
PID=$!
trap 'kill $PID >/dev/null 2>&1 || true; rm -f "$LOG_FILE"' EXIT

# Wait for "echo listening on ..."
for _ in {1..50}; do
  if grep -q "echo listening on" "$LOG_FILE"; then break; fi
  sleep 0.1
done

line="$(grep "echo listening on" "$LOG_FILE" | tail -n1)"
PORT="$(awk -F: '{print $NF}' <<<"$line" | tr -d '[:space:]')"
HOST="$(sed -E 's/.* on ([0-9\.]+):[0-9]+/\1/' <<<"$line")"

echo "[ OK ] echo server: ${HOST}:${PORT}"

# 1) curl round-trip allowing HTTP/0.9 (raw TCP echo)
if command -v curl >/dev/null 2>&1; then
  printf 'hello RON\n' | curl --http0.9 --no-progress-meter --data-binary @- "http://${HOST}:${PORT}/" || true
else
  echo "[WARN] curl not found; skipping curl test"
fi

# 2) nc round-trip
if command -v nc >/dev/null 2>&1; then
  printf 'hello RON\n' | nc -w 1 "${HOST}" "${PORT}" || true
else
  echo "[WARN] nc not found; skipping nc test"
fi

echo "[ OK ] probes done"
