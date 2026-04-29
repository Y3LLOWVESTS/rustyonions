#!/usr/bin/env bash
# RO:WHAT — Live WEB3 paid-storage estimate smoke through svc-gateway → omnigate → svc-storage.
# RO:WHY — Ops/CI; Concerns: ECON/DX/RES. Proves product-facing preflight price path before wallet holds.
# RO:INTERACTS — svc-gateway /paid/o/estimate, omnigate /v1/paid/o/estimate, svc-storage /paid/o/estimate.
# RO:INVARIANTS — read-only; no wallet mutation; no ledger mutation; no object write; economics config fails closed.
# RO:METRICS — service logs go to target/web3-estimate-edge-smoke/*.log.
# RO:CONFIG — ADDR, OMNIGATE_STORAGE_BASE_URL, SVC_GATEWAY_OMNIGATE_BASE_URL, RON_STORAGE_ROC_ECONOMICS_*.
# RO:SECURITY — local dev loopback only; forwards harmless test headers; does not echo real secrets.
# RO:TEST — run with: bash scripts/web3_paid_estimate_edge_smoke.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required tool: $1" >&2
    exit 1
  }
}

need cargo
need curl
need jq

STORAGE_ADDR="${ADDR:-127.0.0.1:15303}"
OMNIGATE_ADDR="${OMNIGATE_BIND_ADDR:-127.0.0.1:15305}"
OMNIGATE_METRICS_ADDR="${OMNIGATE_METRICS_ADDR:-127.0.0.1:19605}"
GATEWAY_ADDR="${SVC_GATEWAY_BIND_ADDR:-127.0.0.1:15304}"

STORAGE_URL="http://${STORAGE_ADDR}"
OMNIGATE_URL="http://${OMNIGATE_ADDR}"
OMNIGATE_METRICS_URL="http://${OMNIGATE_METRICS_ADDR}"
GATEWAY_URL="http://${GATEWAY_ADDR}"

ECONOMICS_PATH="${RON_STORAGE_ROC_ECONOMICS_PATH:-configs/roc-economics.toml}"
ECONOMICS_ACTION="${RON_STORAGE_ROC_ECONOMICS_ACTION:-paid_storage_put}"

LOG_DIR="${ROOT_DIR}/target/web3-estimate-edge-smoke"
mkdir -p "$LOG_DIR"

STORAGE_LOG="${LOG_DIR}/svc-storage.log"
OMNIGATE_LOG="${LOG_DIR}/omnigate.log"
GATEWAY_LOG="${LOG_DIR}/svc-gateway.log"
OMNIGATE_CONFIG="${LOG_DIR}/omnigate.toml"

: > "$STORAGE_LOG"
: > "$OMNIGATE_LOG"
: > "$GATEWAY_LOG"

STORAGE_BIN="${ROOT_DIR}/target/debug/svc-storage"
OMNIGATE_BIN="${ROOT_DIR}/target/debug/omnigate"
GATEWAY_BIN="${ROOT_DIR}/target/debug/svc-gateway"

STORAGE_PID=""
OMNIGATE_PID=""
GATEWAY_PID=""

print_log_tail() {
  local label="$1"
  local path="$2"

  echo
  echo "----- ${label}: ${path} -----"
  if [[ -f "$path" ]]; then
    tail -n 180 "$path" || true
  else
    echo "log file not found"
  fi
  echo "----- end ${label} -----"
  echo
}

cleanup() {
  if [[ -n "${GATEWAY_PID}" ]] && kill -0 "${GATEWAY_PID}" >/dev/null 2>&1; then
    kill "${GATEWAY_PID}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${OMNIGATE_PID}" ]] && kill -0 "${OMNIGATE_PID}" >/dev/null 2>&1; then
    kill "${OMNIGATE_PID}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${STORAGE_PID}" ]] && kill -0 "${STORAGE_PID}" >/dev/null 2>&1; then
    kill "${STORAGE_PID}" >/dev/null 2>&1 || true
  fi

  for pid in "${GATEWAY_PID}" "${OMNIGATE_PID}" "${STORAGE_PID}"; do
    if [[ -n "$pid" ]]; then
      wait "$pid" >/dev/null 2>&1 || true
    fi
  done
}
trap cleanup EXIT

probe_http() {
  local url="$1"
  curl -fsS --connect-timeout 0.5 --max-time 1 "$url" >/dev/null 2>&1
}

wait_for_http() {
  local url="$1"
  local label="$2"
  local pid="$3"
  local log_path="$4"
  local deadline=$((SECONDS + 30))

  until probe_http "$url"; do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      echo "${label} process exited before ${url} became ready" >&2
      print_log_tail "$label log" "$log_path" >&2
      exit 1
    fi

    if (( SECONDS >= deadline )); then
      echo "timeout waiting for ${label}: ${url}" >&2
      echo "direct probe output:" >&2
      curl -v --connect-timeout 1 --max-time 2 "$url" >&2 || true
      print_log_tail "$label log" "$log_path" >&2
      exit 1
    fi

    sleep 0.25
  done
}

wait_for_omnigate_route() {
  local deadline=$((SECONDS + 30))

  until curl -fsS --connect-timeout 1 --max-time 3 "${OMNIGATE_URL}/v1/paid/o/estimate?bytes=48" >/dev/null 2>&1; do
    if ! kill -0 "$OMNIGATE_PID" >/dev/null 2>&1; then
      echo "omnigate exited before paid estimate route became ready" >&2
      print_log_tail "omnigate log" "$OMNIGATE_LOG" >&2
      exit 1
    fi

    if (( SECONDS >= deadline )); then
      echo "timeout waiting for omnigate paid estimate route" >&2
      curl -v --connect-timeout 1 --max-time 5 "${OMNIGATE_URL}/v1/paid/o/estimate?bytes=48" >&2 || true
      print_log_tail "omnigate log" "$OMNIGATE_LOG" >&2
      exit 1
    fi

    sleep 0.25
  done
}

assert_json_field() {
  local json="$1"
  local filter="$2"
  local expected="$3"
  local actual

  actual="$(echo "$json" | jq -r "$filter")"

  if [[ "$actual" != "$expected" ]]; then
    echo "assertion failed: ${filter}" >&2
    echo "expected: ${expected}" >&2
    echo "actual:   ${actual}" >&2
    echo "full JSON:" >&2
    echo "$json" | jq . >&2
    print_log_tail "svc-storage log" "$STORAGE_LOG" >&2
    print_log_tail "omnigate log" "$OMNIGATE_LOG" >&2
    print_log_tail "svc-gateway log" "$GATEWAY_LOG" >&2
    exit 1
  fi
}

if [[ ! -f "$ECONOMICS_PATH" ]]; then
  echo "missing economics config: ${ECONOMICS_PATH}" >&2
  exit 1
fi

echo "building svc-storage, omnigate, and svc-gateway"
cargo build -p svc-storage -p omnigate -p svc-gateway

if [[ ! -x "$STORAGE_BIN" ]]; then
  echo "missing compiled storage binary: ${STORAGE_BIN}" >&2
  exit 1
fi

if [[ ! -x "$OMNIGATE_BIN" ]]; then
  echo "missing compiled omnigate binary: ${OMNIGATE_BIN}" >&2
  exit 1
fi

if [[ ! -x "$GATEWAY_BIN" ]]; then
  echo "missing compiled gateway binary: ${GATEWAY_BIN}" >&2
  exit 1
fi

cat > "$OMNIGATE_CONFIG" <<EOF
[server]
bind = "${OMNIGATE_ADDR}"
metrics_addr = "${OMNIGATE_METRICS_ADDR}"
amnesia = true

[oap]
max_frame_bytes = 1048576
stream_chunk_bytes = 65536

[admission.global_quota]
qps = 20000
burst = 40000

[admission.ip_quota]
enabled = true
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
max_inflight_threshold = 64
error_rate_429_503_pct = 1.0
window_secs = 10
hold_for_secs = 20
EOF

echo "starting svc-storage on ${STORAGE_ADDR} with ROC economics config"
ADDR="$STORAGE_ADDR" \
RON_STORAGE_ROC_ECONOMICS_PATH="$ECONOMICS_PATH" \
RON_STORAGE_ROC_ECONOMICS_ACTION="$ECONOMICS_ACTION" \
RUST_LOG="${RUST_LOG:-info,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$STORAGE_BIN" >"$STORAGE_LOG" 2>&1 &
STORAGE_PID="$!"

wait_for_http "${STORAGE_URL}/healthz" "svc-storage" "$STORAGE_PID" "$STORAGE_LOG"

echo "svc-storage is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${STORAGE_URL}/healthz" | jq .

echo "checking direct svc-storage estimate"
STORAGE_ESTIMATE_JSON="$(
  curl -fsS --connect-timeout 1 --max-time 5 "${STORAGE_URL}/paid/o/estimate?bytes=48"
)"
echo "$STORAGE_ESTIMATE_JSON" | jq .

assert_json_field "$STORAGE_ESTIMATE_JSON" '.route' "/paid/o"
assert_json_field "$STORAGE_ESTIMATE_JSON" '.action' "$ECONOMICS_ACTION"
assert_json_field "$STORAGE_ESTIMATE_JSON" '.asset' "roc"
assert_json_field "$STORAGE_ESTIMATE_JSON" '.bytes | tostring' "48"
assert_json_field "$STORAGE_ESTIMATE_JSON" '.amount_minor' "84"
assert_json_field "$STORAGE_ESTIMATE_JSON" '.minimum_hold_minor' "84"
assert_json_field "$STORAGE_ESTIMATE_JSON" '.pricing_mode' "roc-economics"

echo "starting omnigate on ${OMNIGATE_ADDR}; storage upstream ${STORAGE_URL}"
OMNIGATE_STORAGE_BASE_URL="$STORAGE_URL" \
OMNIGATE_DEV_READY=1 \
RUST_LOG="${RUST_LOG:-info,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$OMNIGATE_BIN" --config "$OMNIGATE_CONFIG" >"$OMNIGATE_LOG" 2>&1 &
OMNIGATE_PID="$!"

wait_for_omnigate_route

echo "checking omnigate estimate"
OMNIGATE_ESTIMATE_JSON="$(
  curl -fsS --connect-timeout 1 --max-time 5 \
    -H 'Authorization: Bearer dev' \
    -H 'x-ron-token: smoke-token' \
    -H 'x-ron-passport: smoke-passport' \
    -H 'x-correlation-id: web3-estimate-edge-smoke' \
    "${OMNIGATE_URL}/v1/paid/o/estimate?bytes=48"
)"
echo "$OMNIGATE_ESTIMATE_JSON" | jq .

assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.route' "/paid/o"
assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.action' "$ECONOMICS_ACTION"
assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.asset' "roc"
assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.bytes | tostring' "48"
assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.amount_minor' "84"
assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.minimum_hold_minor' "84"
assert_json_field "$OMNIGATE_ESTIMATE_JSON" '.pricing_mode' "roc-economics"

echo "starting svc-gateway on ${GATEWAY_ADDR}; omnigate upstream ${OMNIGATE_URL}"
SVC_GATEWAY_BIND_ADDR="$GATEWAY_ADDR" \
SVC_GATEWAY_OMNIGATE_BASE_URL="$OMNIGATE_URL" \
RUST_LOG="${RUST_LOG:-info,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$GATEWAY_BIN" >"$GATEWAY_LOG" 2>&1 &
GATEWAY_PID="$!"

wait_for_http "${GATEWAY_URL}/healthz" "svc-gateway" "$GATEWAY_PID" "$GATEWAY_LOG"

echo "svc-gateway is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${GATEWAY_URL}/healthz"

echo
echo "checking product-facing gateway estimate"
GATEWAY_ESTIMATE_JSON="$(
  curl -fsS --connect-timeout 1 --max-time 5 \
    -H 'Authorization: Bearer dev' \
    -H 'x-ron-token: smoke-token' \
    -H 'x-ron-passport: smoke-passport' \
    -H 'x-correlation-id: web3-estimate-edge-smoke' \
    "${GATEWAY_URL}/paid/o/estimate?bytes=48"
)"
echo "$GATEWAY_ESTIMATE_JSON" | jq .

assert_json_field "$GATEWAY_ESTIMATE_JSON" '.route' "/paid/o"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.action' "$ECONOMICS_ACTION"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.asset' "roc"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.bytes | tostring' "48"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.amount_minor' "84"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.minimum_hold_minor' "84"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.pricing_mode' "roc-economics"

echo "checking malformed request passes back a stable bad_request through the full chain"
BAD_STATUS="$(
  curl -sS --connect-timeout 1 --max-time 5 \
    -o "${LOG_DIR}/bad-estimate.json" \
    -w '%{http_code}' \
    "${GATEWAY_URL}/paid/o/estimate?bytes=not-a-number"
)"

if [[ "$BAD_STATUS" != "400" ]]; then
  echo "expected malformed estimate to return 400, got ${BAD_STATUS}" >&2
  cat "${LOG_DIR}/bad-estimate.json" >&2 || true
  print_log_tail "svc-storage log" "$STORAGE_LOG" >&2
  print_log_tail "omnigate log" "$OMNIGATE_LOG" >&2
  print_log_tail "svc-gateway log" "$GATEWAY_LOG" >&2
  exit 1
fi

BAD_JSON="$(cat "${LOG_DIR}/bad-estimate.json")"
echo "$BAD_JSON" | jq .

assert_json_field "$BAD_JSON" '.error' "bad_request"
assert_json_field "$BAD_JSON" '.reason' "bytes must be an unsigned integer"

echo
echo "WEB3 paid estimate edge smoke green"
echo "path:           svc-gateway → omnigate → svc-storage"
echo "gateway route:  ${GATEWAY_URL}/paid/o/estimate?bytes=48"
echo "omnigate route: ${OMNIGATE_URL}/v1/paid/o/estimate?bytes=48"
echo "storage route:  ${STORAGE_URL}/paid/o/estimate?bytes=48"
echo "amount_minor:   84"
echo "pricing_mode:   roc-economics"
echo "logs:"
echo "  storage:  ${STORAGE_LOG}"
echo "  omnigate: ${OMNIGATE_LOG}"
echo "  gateway:  ${GATEWAY_LOG}"