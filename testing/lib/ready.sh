#!/usr/bin/env bash
# Portable readiness helpers (macOS-friendly). Source, don't exec.
# Usage:
#   source testing/lib/ready.sh
#   wait_readyz "http://127.0.0.1:9080" 25
#   wait_obj "http://127.0.0.1:9080" "$OBJ_ADDR" 25
set -euo pipefail

_wait_http_200() {
  local url="$1" timeout="${2:-30}"
  local start now code
  start=$(date +%s)
  while true; do
    code=$(curl -fsS -o /dev/null -w "%{http_code}" "$url" || true)
    if [ "$code" = "200" ]; then return 0; fi
    now=$(date +%s)
    if (( now - start >= timeout )); then
      echo "timeout: expected 200 from $url (last=$code)" >&2
      return 1
    fi
    sleep 0.25
  done
}

wait_readyz() { _wait_http_200 "$1/readyz" "${2:-30}"; }
wait_obj()    { _wait_http_200 "$1/o/$2"   "${3:-30}"; }

require_env() {
  local name="$1"
  if [ -z "${!name:-}" ]; then
    echo "missing required env: $name" >&2
    return 1
  fi
}
