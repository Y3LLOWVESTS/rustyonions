#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
PID_FILE="$ROOT/target/gateway.pid"

if [[ -f "$PID_FILE" ]]; then
  PID="$(cat "$PID_FILE" || true)"
  if [[ -n "${PID:-}" ]] && ps -p "$PID" >/dev/null 2>&1; then
    echo "Stopping svc-gateway (PID $PID)…"
    kill "$PID" || true
    sleep 0.3
    if ps -p "$PID" >/dev/null 2>&1; then
      echo "Sending SIGKILL…"
      kill -9 "$PID" || true
    fi
  else
    echo "No live process found for PID file."
  fi
  rm -f "$PID_FILE"
else
  echo "No PID file; trying pkill fallback…"
  pkill -f 'svc-gateway' 2>/dev/null || true
fi

echo "Stopped."
