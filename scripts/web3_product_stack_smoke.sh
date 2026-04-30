#!/usr/bin/env bash
# RO:WHAT — Starts the local WEB3 product stack and runs gateway-facing product smoke scripts.
# RO:WHY — Reproducible one-command Batch 10 smoke for WEB3_2 product proof.
# RO:INTERACTS — svc-index, svc-storage, omnigate, svc-gateway, and web3_*_smoke.sh scripts.
# RO:INVARIANTS — safe by default; isolated index DB per run; only prepare/read unless caller opts into create/upload.
# RO:METRICS — leaves per-service logs and smoke config in a timestamped artifact directory for debugging.
# RO:CONFIG — INDEX_BIND, RON_STORAGE_ADDR, OMNIGATE_BIND, SVC_GATEWAY_BIND_ADDR, RON_GATEWAY_URL.
# RO:SECURITY — disables omnigate policy only for this local smoke config; does not silently enable paid upload/site create.
# RO:TEST — manual smoke: scripts/web3_product_stack_smoke.sh.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

INDEX_BIND="${INDEX_BIND:-127.0.0.1:5304}"
RON_STORAGE_ADDR="${RON_STORAGE_ADDR:-127.0.0.1:5303}"
OMNIGATE_BIND="${OMNIGATE_BIND:-127.0.0.1:9090}"
OMNIGATE_METRICS_ADDR="${OMNIGATE_METRICS_ADDR:-127.0.0.1:9605}"
SVC_GATEWAY_BIND_ADDR="${SVC_GATEWAY_BIND_ADDR:-127.0.0.1:8090}"

OMNIGATE_INDEX_BASE_URL="${OMNIGATE_INDEX_BASE_URL:-http://${INDEX_BIND}}"
OMNIGATE_STORAGE_BASE_URL="${OMNIGATE_STORAGE_BASE_URL:-http://${RON_STORAGE_ADDR}}"
SVC_GATEWAY_OMNIGATE_BASE_URL="${SVC_GATEWAY_OMNIGATE_BASE_URL:-http://${OMNIGATE_BIND}}"
RON_GATEWAY_URL="${RON_GATEWAY_URL:-http://${SVC_GATEWAY_BIND_ADDR}}"

TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="${ROOT_DIR}/artifacts/web3-product-smoke-${TIMESTAMP}"
OMNIGATE_SMOKE_CONFIG="${LOG_DIR}/omnigate-smoke.toml"
SMOKE_INDEX_DB="${LOG_DIR}/svc-index.db"
DEFAULT_SITE_NAME="smoke-site-${TIMESTAMP}"

PIDS=()

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 127
  }
}

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

require_file() {
  local path="$1"
  [ -f "$path" ] || fail "required file not found: $path"
}

print_log_tail() {
  local logfile="$1"

  if [ -f "$logfile" ]; then
    echo
    echo "last 120 lines from ${logfile}:"
    tail -n 120 "$logfile" || true
    echo
  fi
}

start_service() {
  local name="$1"
  local logfile="$2"
  shift 2

  echo "starting ${name} ..."
  "$@" >"$logfile" 2>&1 &
  local pid=$!

  PIDS+=("$pid")

  echo "started ${name} pid=${pid} log=${logfile}"
}

log_has_startup_failure() {
  local logfile="$1"

  if [ ! -f "$logfile" ]; then
    return 1
  fi

  grep -E \
    "could not determine which binary|Address already in use|panicked at|thread '.*' panicked|error:" \
    "$logfile" >/dev/null 2>&1
}

wait_http_ok() {
  local url="$1"
  local label="$2"
  local logfile="$3"
  local attempts="${4:-180}"

  for i in $(seq 1 "$attempts"); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      echo "ready: ${label} (${url})"
      return 0
    fi

    if log_has_startup_failure "$logfile"; then
      print_log_tail "$logfile"
      fail "${label} failed during startup"
    fi

    sleep 1
  done

  print_log_tail "$logfile"
  fail "${label} did not become ready at ${url}"
}

cleanup() {
  local status=$?

  if [ "${#PIDS[@]}" -gt 0 ]; then
    echo
    echo "stopping background services ..."
    for pid in "${PIDS[@]}"; do
      if kill -0 "$pid" >/dev/null 2>&1; then
        kill "$pid" >/dev/null 2>&1 || true
      fi
    done

    for pid in "${PIDS[@]}"; do
      wait "$pid" >/dev/null 2>&1 || true
    done
  fi

  echo "logs saved in: ${LOG_DIR}"

  if [ "$status" -ne 0 ]; then
    echo "smoke failed; inspect logs above."
  fi

  exit "$status"
}

write_omnigate_smoke_config() {
  cat >"$OMNIGATE_SMOKE_CONFIG" <<EOF
[server]
bind = "${OMNIGATE_BIND}"
metrics_addr = "${OMNIGATE_METRICS_ADDR}"
amnesia = true

[oap]
max_frame_bytes = 1048576
stream_chunk_bytes = 65536

[admission.global_quota]
qps = 20000
burst = 40000

[admission.ip_quota]
enabled = false
qps = 0
burst = 0

[admission.fair_queue]
max_inflight = 2048
headroom = 256

[admission.fair_queue.weights]
anon = 1
auth = 5
admin = 10

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
max_inflight_threshold = 1800
error_rate_429_503_pct = 2.0
window_secs = 10
hold_for_secs = 30
EOF
}

trap cleanup EXIT INT TERM

need_cmd cargo
need_cmd curl
need_cmd chmod
need_cmd seq
need_cmd tail
need_cmd grep
need_cmd mkdir

require_file "scripts/web3_extension_backend_smoke.sh"
require_file "scripts/web3_asset_page_smoke.sh"
require_file "scripts/web3_crab_image_smoke.sh"
require_file "scripts/web3_crab_site_smoke.sh"

chmod +x \
  scripts/web3_extension_backend_smoke.sh \
  scripts/web3_asset_page_smoke.sh \
  scripts/web3_crab_image_smoke.sh \
  scripts/web3_crab_site_smoke.sh

mkdir -p "$LOG_DIR"
write_omnigate_smoke_config

echo "WEB3 product stack smoke"
echo "root:      $ROOT_DIR"
echo "logs:      $LOG_DIR"
echo "index:     $INDEX_BIND"
echo "index_db:  $SMOKE_INDEX_DB"
echo "storage:   $RON_STORAGE_ADDR"
echo "omnigate:  $OMNIGATE_BIND"
echo "gateway:   $SVC_GATEWAY_BIND_ADDR"
echo "site_name: ${RON_SITE_NAME:-$DEFAULT_SITE_NAME}"
echo "policy:    disabled for local smoke config"
echo

SVC_INDEX_LOG="${LOG_DIR}/svc-index.log"
SVC_STORAGE_LOG="${LOG_DIR}/svc-storage.log"
OMNIGATE_LOG="${LOG_DIR}/omnigate.log"
SVC_GATEWAY_LOG="${LOG_DIR}/svc-gateway.log"

start_service \
  "svc-index" \
  "$SVC_INDEX_LOG" \
  env \
    INDEX_BIND="$INDEX_BIND" \
    RON_INDEX_DB="$SMOKE_INDEX_DB" \
    cargo run -p svc-index

wait_http_ok "http://${INDEX_BIND}/healthz" "svc-index" "$SVC_INDEX_LOG"

start_service \
  "svc-storage" \
  "$SVC_STORAGE_LOG" \
  env \
    ADDR="$RON_STORAGE_ADDR" \
    RON_STORAGE_ADDR="$RON_STORAGE_ADDR" \
    cargo run -p svc-storage --bin svc-storage

wait_http_ok "http://${RON_STORAGE_ADDR}/healthz" "svc-storage" "$SVC_STORAGE_LOG"

start_service \
  "omnigate" \
  "$OMNIGATE_LOG" \
  env \
    OMNIGATE_INDEX_BASE_URL="$OMNIGATE_INDEX_BASE_URL" \
    OMNIGATE_STORAGE_BASE_URL="$OMNIGATE_STORAGE_BASE_URL" \
    cargo run -p omnigate -- --config "$OMNIGATE_SMOKE_CONFIG"

wait_http_ok "http://${OMNIGATE_BIND}/healthz" "omnigate" "$OMNIGATE_LOG"

start_service \
  "svc-gateway" \
  "$SVC_GATEWAY_LOG" \
  env \
    SVC_GATEWAY_BIND_ADDR="$SVC_GATEWAY_BIND_ADDR" \
    SVC_GATEWAY_OMNIGATE_BASE_URL="$SVC_GATEWAY_OMNIGATE_BASE_URL" \
    cargo run -p svc-gateway

wait_http_ok "http://${SVC_GATEWAY_BIND_ADDR}/healthz" "svc-gateway" "$SVC_GATEWAY_LOG"
wait_http_ok "http://${SVC_GATEWAY_BIND_ADDR}/readyz" "svc-gateway" "$SVC_GATEWAY_LOG"

echo
echo "running product smoke scripts ..."
echo

env RON_GATEWAY_URL="$RON_GATEWAY_URL" \
  RON_AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}" \
  RON_TEST_SITE_NAME="${RON_TEST_SITE_NAME:-}" \
  scripts/web3_extension_backend_smoke.sh

env RON_GATEWAY_URL="$RON_GATEWAY_URL" \
  RON_TEST_IMAGE_HASH="${RON_TEST_IMAGE_HASH:-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef}" \
  scripts/web3_asset_page_smoke.sh

env RON_GATEWAY_URL="$RON_GATEWAY_URL" \
  RON_AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}" \
  RON_PAYER_ACCOUNT="${RON_PAYER_ACCOUNT:-acct_smoke}" \
  RON_PASSPORT="${RON_PASSPORT:-passport:main:smoke}" \
  RON_RUN_PAID_UPLOAD="${RON_RUN_PAID_UPLOAD:-0}" \
  RON_IMAGE_FILE="${RON_IMAGE_FILE:-}" \
  RON_WALLET_HOLD_TXID="${RON_WALLET_HOLD_TXID:-}" \
  scripts/web3_crab_image_smoke.sh

env RON_GATEWAY_URL="$RON_GATEWAY_URL" \
  RON_AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}" \
  RON_PAYER_ACCOUNT="${RON_PAYER_ACCOUNT:-acct_smoke}" \
  RON_PASSPORT="${RON_PASSPORT:-passport:main:smoke}" \
  RON_SITE_NAME="${RON_SITE_NAME:-$DEFAULT_SITE_NAME}" \
  RON_ROOT_DOCUMENT_CID="${RON_ROOT_DOCUMENT_CID:-b3:1111111111111111111111111111111111111111111111111111111111111111}" \
  RON_RUN_SITE_CREATE="${RON_RUN_SITE_CREATE:-0}" \
  RON_WALLET_HOLD_TXID="${RON_WALLET_HOLD_TXID:-hold_smoke_site_launch}" \
  scripts/web3_crab_site_smoke.sh

echo
echo "WEB3 product stack smoke passed"
echo "logs saved in: ${LOG_DIR}"