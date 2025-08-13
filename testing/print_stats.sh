#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-.tcp_server.log}"
echo "[stats] tailing $LOG — Ctrl+C to stop"
tail -f "$LOG" | grep --line-buffered 'stats/'
