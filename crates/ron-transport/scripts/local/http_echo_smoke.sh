#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="$(mktemp -t ron_transport_http_echo.XXXXXX.log)"
RUST_LOG=info cargo run -q -p ron-transport --example http_echo >"$LOG_FILE" 2>&1 &
PID=$!
trap 'kill $PID >/dev/null 2>&1 || true; rm -f "$LOG_FILE"' EXIT

for _ in {1..50}; do
  if grep -q "http-echo listening on" "$LOG_FILE"; then break; fi
  sleep 0.1
done

line="$(grep "http-echo listening on" "$LOG_FILE" | tail -n1)"
PORT="$(awk -F: '{print $NF}' <<<"$line" | tr -d '[:space:]')"
HOST="$(sed -E 's/.* on ([0-9\.]+):[0-9]+/\1/' <<<"$line")"

echo "[ OK ] http-echo server: ${HOST}:${PORT}"
printf 'Hello via curl 🧪\n' | curl --no-progress-meter --data-binary @- "http://${HOST}:${PORT}/" || true
echo
echo "[ OK ] done"
