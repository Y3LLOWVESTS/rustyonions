#!/usr/bin/env bash
# Metrics smoke (gateway-first, demo-optional)
# - Default: discover a running gateway and verify /metrics, /healthz, /readyz.
# - DEMO=1: attempt to run node_index/node_overlay; if they don't bind or crash, fall back to gateway mode.
# - macOS/Bash 3.2 safe. No magic sleeps (bounded 0.2s polling).
#
# How to test:
#
# BIND=127.0.0.1:9080 PACK_FIRST=1 HOLD=0 testing/run_stack.sh
# bash testing/smoke_metrics.sh
#

set -euo pipefail

# --------------------------- Config -----------------------------------------
BIN_DIR="${BIN_DIR:-./target/debug}"
RUST_LOG="${RUST_LOG:-info}"
TIMEOUT="${TIMEOUT:-25}"            # seconds for readiness waits
DEBUG="${DEBUG:-0}"
DEMO="${DEMO:-0}"                   # set DEMO=1 to try demo bins first

STACK_BIND_FILE="${STACK_BIND_FILE:-.tmp/stack/bind.txt}"
PROBE_HOST="${PROBE_HOST:-127.0.0.1}"
PROBE_PORTS="${PROBE_PORTS:-9080 9081 9082 9083 9084 9085}"

ok()   { printf "\033[32m%s\033[0m\n" "$*"; }
info() { printf "\033[36m%s\033[0m\n" "$*"; }
err()  { printf "\033[31m%s\033[0m\n" "$*" >&2; }
log()  { if [ "$DEBUG" = "1" ]; then printf "[smoke_metrics] %s\n" "$*" >&2; fi; }

# ------------------------ Ready helpers -------------------------------------
wait_http_ok() { # URL TIMEOUT_SEC
  local url="$1" timeout="${2:-15}" start end code
  start=$(date +%s)
  while true; do
    code="$(curl -fsS -o /dev/null -w '%{http_code}' "$url" || true)"
    if [ "$code" = "200" ]; then return 0; fi
    end=$(date +%s)
    if [ $((end - start)) -ge "$timeout" ]; then
      err "Timeout waiting for 200 from $url (last=$code)"
      return 1
    fi
    sleep 0.2
  done
}

wait_http_status() { # URL EXPECTED TIMEOUT_SEC
  local url="$1" want="$2" timeout="${3:-15}" start end code
  start=$(date +%s)
  while true; do
    code="$(curl -fsS -o /dev/null -w '%{http_code}' "$url" || true)"
    if [ "$code" = "$want" ]; then return 0; fi
    end=$(date +%s)
    if [ $((end - start)) -ge "$timeout" ]; then
      err "Timeout waiting for $want from $url (last=$code)"
      return 1
    fi
    sleep 0.2
  done
}

discover_admin_from_log_or_lsof() { # PID LOG_FILE
  local pid="$1" logf="$2"

  # 1) Parse host:port from logs and probe /healthz
  if [ -f "$logf" ]; then
    awk '{gsub(/\x1B\[[0-9;]*[mK]/,"");print}' "$logf" | \
      grep -Eio 'http://[0-9\.]+:[0-9]{2,5}|(127\.0\.0\.1|0\.0\.0\.0):[0-9]{2,5}' | \
      sed -E 's#^http://##' | awk '!seen[$0]++' | while read -r hp; do
        local url="http://${hp}"
        if curl -fsS -o /dev/null "${url}/healthz"; then
          echo "${url}"
          return 0
        fi
      done
  fi

  # 2) lsof LISTEN sockets for the PID, probe /healthz
  if command -v lsof >/dev/null 2>&1; then
    lsof -nP -a -p "$pid" -iTCP -sTCP:LISTEN 2>/dev/null | \
      awk 'NR>1 {print $9}' | sed -E 's#^.*->##; s#^\*:(.*)$#127.0.0.1:\1#' | \
      awk -F':' '{if ($2 ~ /^[0-9]+$/) print $1":"$2}' | \
      sed 's#^\*#127.0.0.1#' | \
      awk '!seen[$0]++' | while read -r hp; do
        local url="http://${hp}"
        if curl -fsS -o /dev/null "${url}/healthz"; then
          echo "${url}"
          return 0
        fi
      done
  fi

  echo ""
  return 1
}

discover_gateway_base() {
  if [ -f "$STACK_BIND_FILE" ]; then
    local bind; bind="$(tr -d '[:space:]' < "$STACK_BIND_FILE")"
    if [ -n "$bind" ]; then echo "http://${bind}"; return 0; fi
  fi
  for p in $PROBE_PORTS; do
    if curl -fsS -o /dev/null "http://${PROBE_HOST}:${p}/healthz"; then
      echo "http://${PROBE_HOST}:${p}"
      return 0
    fi
  done
  echo ""
  return 1
}

metrics_grep() { # URL LABEL
  local url="$1" label="$2" tmp
  tmp="$(mktemp -t ron_metrics.XXXXXX)"
  curl -fsS "$url" -o "$tmp"
  grep -Eq '^# (HELP|TYPE) ' "$tmp"
  grep -Eq '(request_latency_seconds(_count|_sum)|http_requests_total|_bytes_(in|out)_total|_overflow_dropped_total)' "$tmp" || true
  rm -f "$tmp"
  ok "$label metrics: OK ($url)"
}

have_demo_bins() {
  [ -x "${BIN_DIR}/node_index" ] && [ -x "${BIN_DIR}/node_overlay" ]
}

# ------------------------ Gateway mode runner -------------------------------
run_gateway_mode() {
  info "Gateway mode"
  local base
  base="$(discover_gateway_base || true)"
  if [ -z "$base" ]; then
    err "No running gateway discovered."
    err "Start one, then re-run this script. Example:"
    err "BIND=127.0.0.1:9080 PACK_FIRST=1 HOLD=0 testing/run_stack.sh"
    return 2
  fi
  log "Gateway discovered at: ${base}"
  wait_http_ok "${base}/healthz" "$TIMEOUT"
  wait_http_status "${base}/readyz" "200" "$TIMEOUT" || \
  wait_http_status "${base}/readyz" "204" "$TIMEOUT"
  wait_http_ok "${base}/metrics" "$TIMEOUT"
  metrics_grep "${base}/metrics" "gateway"
  ok "Health/Ready (gateway):"
  printf "/healthz: "; curl -s "${base}/healthz"; echo
  printf "/readyz : "; curl -s -o /dev/null -w "HTTP %{http_code}\n" "${base}/readyz"
  ok "✅ smoke_metrics (gateway mode) completed"
}

# ------------------------ Demo mode runner ----------------------------------
run_demo_mode() {
  info "Demo mode (node_index + node_overlay)"
  cargo build -q || true

  local TMP_DIR=".tmp/smoke_metrics"
  mkdir -p "$TMP_DIR"
  local INDEX_LOG="$TMP_DIR/node_index.log"
  local OVERLAY_LOG="$TMP_DIR/node_overlay.log"

  local PIDS=()
  cleanup() {
    local rc=$?
    if [ "$rc" -ne 0 ]; then
      err "=== node_index last 50 lines ==="; tail -n 50 "$INDEX_LOG" 2>/dev/null || true
      err "=== node_overlay last 50 lines ==="; tail -n 50 "$OVERLAY_LOG" 2>/dev/null || true
    fi
    if [ "${#PIDS[@]}" -gt 0 ]; then
      info "Stopping demos…"
      kill "${PIDS[@]}" 2>/dev/null || true
      wait "${PIDS[@]}" 2>/dev/null || true
    fi
    return "$rc"
  }
  trap 'cleanup' RETURN

  # Start node_index
  info "Starting node_index…"
  RUST_LOG="$RUST_LOG" "${BIN_DIR}/node_index" >"$INDEX_LOG" 2>&1 &
  local PID_INDEX=$!; PIDS+=("$PID_INDEX")
  local INDEX_ADMIN
  INDEX_ADMIN="$(discover_admin_from_log_or_lsof "$PID_INDEX" "$INDEX_LOG" || true)"
  if [ -z "$INDEX_ADMIN" ]; then
    err "node_index admin bind not discovered — falling back to gateway mode."
    return 3
  fi
  log "node_index admin at $INDEX_ADMIN"
  wait_http_ok     "${INDEX_ADMIN}/healthz" "$TIMEOUT"
  wait_http_status "${INDEX_ADMIN}/readyz" "200" "$TIMEOUT" || \
  wait_http_status "${INDEX_ADMIN}/readyz" "204" "$TIMEOUT"
  metrics_grep "${INDEX_ADMIN}/metrics" "node_index"

  # Start node_overlay
  info "Starting node_overlay…"
  RUST_LOG="$RUST_LOG" "${BIN_DIR}/node_overlay" >"$OVERLAY_LOG" 2>&1 &
  local PID_OVERLAY=$!; PIDS+=("$PID_OVERLAY")
  local OVERLAY_ADMIN
  OVERLAY_ADMIN="$(discover_admin_from_log_or_lsof "$PID_OVERLAY" "$OVERLAY_LOG" || true)"
  if [ -z "$OVERLAY_ADMIN" ]; then
    err "node_overlay admin bind not discovered — falling back to gateway mode."
    return 3
  fi
  log "node_overlay admin at $OVERLAY_ADMIN"
  wait_http_ok     "${OVERLAY_ADMIN}/healthz" "$TIMEOUT"
  wait_http_status "${OVERLAY_ADMIN}/readyz" "200" "$TIMEOUT" || \
  wait_http_status "${OVERLAY_ADMIN}/readyz" "204" "$TIMEOUT"
  metrics_grep "${OVERLAY_ADMIN}/metrics" "node_overlay"

  ok "Health/Ready (demo):"
  printf "index   /healthz: ";  curl -s "${INDEX_ADMIN}/healthz"; echo
  printf "index   /readyz : ";  curl -s -o /dev/null -w "HTTP %{http_code}\n" "${INDEX_ADMIN}/readyz"
  printf "overlay /healthz: ";  curl -s "${OVERLAY_ADMIN}/healthz"; echo
  printf "overlay /readyz : ";  curl -s -o /dev/null -w "HTTP %{http_code}\n" "${OVERLAY_ADMIN}/readyz"

  ok "✅ smoke_metrics (demo mode) completed"
  return 0
}

# ------------------------ Mode selection ------------------------------------
if [ "$DEMO" = "1" ] && have_demo_bins; then
  if run_demo_mode; then
    exit 0
  else
    info "Falling back to gateway mode…"
    run_gateway_mode
    exit $?
  fi
else
  run_gateway_mode
  exit $?
fi
