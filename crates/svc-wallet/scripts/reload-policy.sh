#!/usr/bin/env bash
set -euo pipefail
PID=$(pgrep -f svc-wallet2 || true)
if [[ -n "$PID" ]]; then
  kill -HUP "$PID"
  echo "Sent SIGHUP to svc-wallet2 ($PID)"
else
  echo "svc-wallet2 not running"
fi
