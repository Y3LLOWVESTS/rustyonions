#!/usr/bin/env bash
# ttl-demo.sh — local provide → find_providers → TTL expiry demo for svc-dht
# Supports auto-spawn of the service.
#
# Usage:
#   ./ttl-demo.sh [--spawn] [--cid b3:short] [--node local://tmp] [--ttl 2] [--addr 127.0.0.1:5301] [--timeout 30]
#
# Notes:
# - If --spawn is provided, this script will run `cargo run -p svc-dht` in the background,
#   wait for /readyz, perform the demo, and then terminate the service.
# - Requires: curl; jq (optional for pretty JSON)
#
#
# EXAMPLE RUN: 
# TERMINAL A: cargo run -p svc-dht
# TERMINAL B: crates/svc-dht/scripts/ttl-demo.sh --spawn --ttl 2
#

set -euo pipefail

CID="b3:short"
NODE="local://tmp"
TTL=2
ADDR="127.0.0.1:5301"
TIMEOUT=30
SPAWN=0
CARGO_CMD="cargo run -p svc-dht"
LOGFILE="/tmp/svc-dht.demo.log"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cid)      CID="$2"; shift 2 ;;
    --node)     NODE="$2"; shift 2 ;;
    --ttl)      TTL="$2"; shift 2 ;;
    --addr)     ADDR="$2"; shift 2 ;;
    --timeout)  TIMEOUT="$2"; shift 2 ;;
    --spawn)    SPAWN=1; shift ;;
    -h|--help)
      echo "Usage: $0 [--spawn] [--cid b3:short] [--node local://tmp] [--ttl 2] [--addr 127.0.0.1:5301] [--timeout 30]"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1"
      exit 1
      ;;
  esac
done

has_jq() { command -v jq >/dev/null 2>&1; }
json_pretty() { if has_jq; then jq; else python3 -m json.tool 2>/dev/null || cat; fi; }
get() { curl -sS "http://$ADDR$1"; }
post_json() { curl -sS -H "content-type: application/json" -d "$2" "http://$ADDR$1"; }

PROC_PGID=""
cleanup() {
  if [[ "$SPAWN" -eq 1 && -n "${PROC_PGID:-}" ]]; then
    echo
    echo ">>> Stopping spawned svc-dht (pgid=$PROC_PGID)"
    kill -TERM "-$PROC_PGID" >/dev/null 2>&1 || true
    sleep 1
    kill -KILL "-$PROC_PGID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [[ "$SPAWN" -eq 1 ]]; then
  echo ">>> Spawning svc-dht and logging to $LOGFILE"
  # Start in its own process group so we can kill the whole tree cleanly later.
  bash -c "set -m; $CARGO_CMD &> '$LOGFILE' & echo \$! > '$LOGFILE.pid'; disown" &
  # Wait for pid file
  for _ in $(seq 1 50); do
    [[ -f "$LOGFILE.pid" ]] && break
    sleep 0.1
  done
  if [[ ! -f "$LOGFILE.pid" ]]; then
    echo "Failed to obtain svc-dht PID (check $LOGFILE)."
    exit 1
  fi
  PID="$(cat "$LOGFILE.pid")"
  # Get the process group id (pgid == pid for group leader)
  PROC_PGID="$(ps -o pgid= -p "$PID" 2>/dev/null | tr -d ' ')"
  if [[ -z "$PROC_PGID" ]]; then
    echo "Could not determine process group; PID=$PID. Proceeding without kill group."
  fi
fi

echo ">>> Waiting for readiness at http://$ADDR/readyz (timeout ${TIMEOUT}s)..."
deadline=$(( $(date +%s) + TIMEOUT ))
while :; do
  # Capture both curl status and http code
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://$ADDR/readyz" || echo 000)
  if [[ "$HTTP_CODE" == "200" ]]; then
    echo "Ready."
    break
  fi
  if (( $(date +%s) >= deadline )); then
    echo "Service not ready (HTTP $HTTP_CODE) before timeout."
    if [[ "$SPAWN" -eq 1 ]]; then
      echo "Last 50 lines from $LOGFILE:"
      tail -n 50 "$LOGFILE" || true
    fi
    exit 1
  fi
  sleep 0.2
done

echo
echo ">>> Version:"
get "/version" | json_pretty

echo
echo ">>> Posting provide (cid=$CID, node=$NODE, ttl=$TTL s)"
RESP=$(post_json "/dht/provide" "$(printf '{"cid":"%s","node":"%s","ttl_secs":%s}' "$CID" "$NODE" "$TTL")")
echo "$RESP" | json_pretty

echo
echo ">>> Immediate find_providers:"
get "/dht/find_providers/$CID" | json_pretty

echo
echo ">>> Debug snapshot (with seconds remaining):"
get "/dht/_debug/list" | json_pretty

echo
echo ">>> Metrics (before sleep):"
get "/metrics" | grep -E "dht_provides_total|dht_lookups_total" || true

echo
echo ">>> Sleeping ${TTL}s + 1 to allow TTL to expire..."
sleep $((TTL + 1))

echo
echo ">>> find_providers after expiry (should be empty):"
get "/dht/find_providers/$CID" | json_pretty

echo
echo ">>> Metrics (after):"
get "/metrics" | grep -E "dht_provides_total|dht_lookups_total" || true

if [[ "$SPAWN" -eq 1 ]]; then
  echo
  echo ">>> svc-dht logs (tail):"
  tail -n 50 "$LOGFILE" || true
fi

echo
echo "Done."
