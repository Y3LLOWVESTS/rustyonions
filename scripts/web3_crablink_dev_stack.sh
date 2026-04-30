#!/usr/bin/env bash
# RO:WHAT — Persistent local WEB3_2 stack runner for CrabLink extension testing.
# RO:WHY — Keeps svc-index, svc-storage, omnigate, and svc-gateway alive while using the browser extension.
# RO:INTERACTS — svc-index, svc-storage, omnigate, svc-gateway, scripts/web3_identity_stack_smoke.sh.
# RO:INVARIANTS — gateway-only browser API; no direct extension calls to internal services; no ledger mutation here.
# RO:METRICS — services expose normal /metrics; smoke uses x-correlation-id for traceability.
# RO:CONFIG — INDEX_BIND, ADDR/RON_STORAGE_ADDR, OMNIGATE_BIND, SVC_GATEWAY_BIND_ADDR.
# RO:SECURITY — local dev only; omnigate policy disabled only in generated local config.
# RO:TEST — run this script, then use CrabLink Check Node / Check Passport / Refresh Balance.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TS="$(date +%Y%m%d-%H%M%S)"
ARTIFACT_DIR="${RON_CRABLINK_ARTIFACT_DIR:-$ROOT/artifacts/crablink-dev-stack-$TS}"

INDEX_BIND="${INDEX_BIND:-127.0.0.1:5304}"
STORAGE_BIND="${RON_STORAGE_ADDR:-${ADDR:-127.0.0.1:5303}}"
OMNIGATE_BIND="${OMNIGATE_BIND:-127.0.0.1:9090}"
OMNIGATE_METRICS_ADDR="${OMNIGATE_METRICS_ADDR:-127.0.0.1:9605}"
GATEWAY_BIND="${SVC_GATEWAY_BIND_ADDR:-127.0.0.1:8090}"

INDEX_URL="http://$INDEX_BIND"
STORAGE_URL="http://$STORAGE_BIND"
OMNIGATE_URL="http://$OMNIGATE_BIND"
GATEWAY_URL="http://$GATEWAY_BIND"

INDEX_DB="${RON_INDEX_DB:-$ARTIFACT_DIR/svc-index.db}"
OMNIGATE_CONFIG="$ARTIFACT_DIR/omnigate-crablink-dev.toml"
PIDS_FILE="$ARTIFACT_DIR/service-pids.txt"

PIDS=()
NAMES=()
LOGS=()
LAST_PID=""
LAST_LOG=""

say() {
  printf "\n%s\n" "$*"
}

die() {
  printf "error: %s\n" "$*" >&2
  exit 1
}

write_pid() {
  local name="$1"
  local pid="$2"
  local log="$3"

  PIDS+=("$pid")
  NAMES+=("$name")
  LOGS+=("$log")
  LAST_PID="$pid"
  LAST_LOG="$log"

  printf "%s %s %s\n" "$name" "$pid" "$log" >> "$PIDS_FILE"
}

show_log_tail() {
  local name="$1"
  local log="$2"

  printf "\n--- %s log tail: %s ---\n" "$name" "$log" >&2
  if [ -f "$log" ]; then
    tail -n 160 "$log" >&2 || true
  else
    printf "log file does not exist yet\n" >&2
  fi
  printf -- "--- end %s log tail ---\n\n" "$name" >&2
}

cleanup() {
  local count="${#PIDS[@]}"

  if [ "$count" -gt 0 ]; then
    say "stopping CrabLink dev stack ..."

    local i
    for ((i=count-1; i>=0; i--)); do
      local pid="${PIDS[$i]}"
      local name="${NAMES[$i]}"

      if kill -0 "$pid" >/dev/null 2>&1; then
        printf "stopping %s pid=%s\n" "$name" "$pid"
        kill -TERM "$pid" >/dev/null 2>&1 || true
      fi
    done

    for ((i=count-1; i>=0; i--)); do
      local pid="${PIDS[$i]}"
      wait "$pid" >/dev/null 2>&1 || true
    done
  fi

  say "logs saved in: $ARTIFACT_DIR"
}

trap cleanup EXIT INT TERM

http_code() {
  local url="$1"
  curl -s -o /dev/null -w "%{http_code}" "$url" || true
}

tcp_port_available() {
  local bind_addr="$1"

  if ! command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  python3 - "$bind_addr" <<'PY'
import socket
import sys

addr = sys.argv[1]
if ":" not in addr:
    sys.exit(0)

host, port_text = addr.rsplit(":", 1)
try:
    port = int(port_text)
except ValueError:
    sys.exit(0)

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
try:
    s.bind((host, port))
except OSError as exc:
    print(exc)
    sys.exit(1)
finally:
    try:
        s.close()
    except Exception:
        pass

sys.exit(0)
PY
}

ensure_port_free() {
  local name="$1"
  local bind_addr="$2"
  local health_url="$3"

  local code
  code="$(http_code "$health_url")"

  if [ "$code" = "200" ]; then
    die "$name already appears to be running at $health_url. Stop that process or run this script with alternate ports."
  fi

  if ! tcp_port_available "$bind_addr"; then
    die "$name bind address $bind_addr is already in use by another process."
  fi
}

wait_for_http() {
  local name="$1"
  local url="$2"
  local expected="$3"
  local pid="$4"
  local log="$5"
  local attempts="${RON_CRABLINK_WAIT_ATTEMPTS:-360}"
  local delay="${RON_CRABLINK_WAIT_DELAY:-0.5}"

  printf "waiting for %s (%s) ...\n" "$name" "$url"

  local code
  local i
  for ((i=1; i<=attempts; i++)); do
    code="$(http_code "$url")"

    if [ "$code" = "$expected" ]; then
      printf "ready: %s (%s)\n" "$name" "$url"
      return 0
    fi

    if ! kill -0 "$pid" >/dev/null 2>&1; then
      printf "error: %s process exited before readiness; last HTTP code=%s\n" "$name" "$code" >&2
      show_log_tail "$name" "$log"
      return 1
    fi

    if (( i % 20 == 0 )); then
      printf "still waiting for %s; last HTTP code=%s; log=%s\n" "$name" "$code" "$log"
    fi

    sleep "$delay"
  done

  printf "error: %s did not become ready at %s; last HTTP code=%s\n" "$name" "$url" "$code" >&2
  show_log_tail "$name" "$log"
  return 1
}

start_service() {
  local name="$1"
  local log="$2"
  shift 2

  say "starting $name ..."
  "$@" > "$log" 2>&1 &
  local pid="$!"

  write_pid "$name" "$pid" "$log"
  printf "started %s pid=%s log=%s\n" "$name" "$pid" "$log"
}

mkdir -p "$ARTIFACT_DIR"
: > "$PIDS_FILE"

say "CrabLink WEB3_2 dev stack"
printf "root:       %s\n" "$ROOT"
printf "logs:       %s\n" "$ARTIFACT_DIR"
printf "index:      %s\n" "$INDEX_URL"
printf "index_db:   %s\n" "$INDEX_DB"
printf "storage:    %s\n" "$STORAGE_URL"
printf "omnigate:   %s\n" "$OMNIGATE_URL"
printf "gateway:    %s\n" "$GATEWAY_URL"
printf "policy:     disabled for local CrabLink dev config only\n"

ensure_port_free "svc-index" "$INDEX_BIND" "$INDEX_URL/healthz"
ensure_port_free "svc-storage" "$STORAGE_BIND" "$STORAGE_URL/healthz"
ensure_port_free "omnigate" "$OMNIGATE_BIND" "$OMNIGATE_URL/healthz"
ensure_port_free "svc-gateway" "$GATEWAY_BIND" "$GATEWAY_URL/healthz"

say "building dev stack binaries ..."
cargo build -p svc-index -p svc-storage -p omnigate -p svc-gateway

cat > "$OMNIGATE_CONFIG" <<EOF
[server]
bind = "$OMNIGATE_BIND"
metrics_addr = "$OMNIGATE_METRICS_ADDR"
amnesia = true

[oap]
max_frame_bytes = 1048576
stream_chunk_bytes = 65536

[admission.global_quota]
qps = 20000
burst = 40000

[admission.ip_quota]
enabled = false
qps = 2000
burst = 4000

[admission.fair_queue]
max_inflight = 2048
weights = { anon = 1, auth = 5, admin = 10 }

[admission.body]
max_content_length = 10485760
reject_on_missing_length = false

[admission.decompression]
allow = ["identity", "gzip"]
deny_stacked = true

[policy]
enabled = false
bundle_path = "crates/omnigate/configs/policy.bundle.json"
fail_mode = "deny"

[readiness]
max_inflight_threshold = 2000
error_rate_429_503_pct = 95.0
window_secs = 5
hold_for_secs = 2
EOF

start_service "svc-index" "$ARTIFACT_DIR/svc-index.log" \
  env \
  RUST_LOG="${RUST_LOG:-info}" \
  INDEX_BIND="$INDEX_BIND" \
  RON_INDEX_DB="$INDEX_DB" \
  "$ROOT/target/debug/svc-index"

wait_for_http "svc-index" "$INDEX_URL/healthz" "200" "$LAST_PID" "$LAST_LOG"

start_service "svc-storage" "$ARTIFACT_DIR/svc-storage.log" \
  env \
  RUST_LOG="${RUST_LOG:-info}" \
  ADDR="$STORAGE_BIND" \
  RON_STORAGE_ADDR="$STORAGE_BIND" \
  "$ROOT/target/debug/svc-storage"

wait_for_http "svc-storage" "$STORAGE_URL/healthz" "200" "$LAST_PID" "$LAST_LOG"

start_service "omnigate" "$ARTIFACT_DIR/omnigate.log" \
  env \
  RUST_LOG="${RUST_LOG:-info}" \
  OMNIGATE_STORAGE_BASE_URL="$STORAGE_URL" \
  OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL="$STORAGE_URL" \
  OMNIGATE_INDEX_BASE_URL="$INDEX_URL" \
  OMNIGATE_DOWNSTREAM_INDEX_BASE_URL="$INDEX_URL" \
  OMNIGATE_BIND="$OMNIGATE_BIND" \
  OMNIGATE_METRICS_ADDR="$OMNIGATE_METRICS_ADDR" \
  "$ROOT/target/debug/omnigate" \
  --config "$OMNIGATE_CONFIG"

wait_for_http "omnigate" "$OMNIGATE_URL/healthz" "200" "$LAST_PID" "$LAST_LOG"

start_service "svc-gateway" "$ARTIFACT_DIR/svc-gateway.log" \
  env \
  RUST_LOG="${RUST_LOG:-info}" \
  SVC_GATEWAY_BIND_ADDR="$GATEWAY_BIND" \
  SVC_GATEWAY_OMNIGATE_BASE_URL="$OMNIGATE_URL" \
  RON_GATEWAY_URL="$GATEWAY_URL" \
  "$ROOT/target/debug/svc-gateway"

wait_for_http "svc-gateway" "$GATEWAY_URL/healthz" "200" "$LAST_PID" "$LAST_LOG"
wait_for_http "svc-gateway readyz" "$GATEWAY_URL/readyz" "200" "$LAST_PID" "$LAST_LOG"

say "running identity smoke ..."
if [ "${RON_SKIP_IDENTITY_SMOKE:-0}" = "1" ]; then
  printf "skip: identity smoke disabled by RON_SKIP_IDENTITY_SMOKE=1\n"
else
  RON_GATEWAY_URL="$GATEWAY_URL" scripts/web3_identity_stack_smoke.sh
fi

say "CrabLink dev stack is online"
printf "gateway: %s\n" "$GATEWAY_URL"
printf "logs:    %s\n" "$ARTIFACT_DIR"
printf "\nLeave this terminal open while using the CrabLink extension.\n"
printf "Press Ctrl-C in this terminal to stop the stack.\n"

while true; do
  sleep 3600
done