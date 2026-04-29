#!/usr/bin/env bash
# RO:WHAT — Live WEB3 edge paid-write smoke through svc-gateway → omnigate → svc-storage.
# RO:WHY — Ops/CI; Concerns: ECON/DX/RES. Proves product-facing paid storage execution path after estimate.
# RO:INTERACTS — svc-wallet, ron-accounting, svc-storage, omnigate, svc-gateway.
# RO:INVARIANTS — wallet is mutation front-door; storage owns paid-write enforcement; gateway/omnigate only proxy.
# RO:METRICS — service logs go to target/web3-edge-write-smoke/*.log.
# RO:CONFIG — SVC_WALLET_ADDR, RON_ACCOUNTING_ADDR, ADDR, OMNIGATE_BIND_ADDR, SVC_GATEWAY_BIND_ADDR.
# RO:SECURITY — local dev bearer only; loopback-only; does not echo real secrets.
# RO:TEST — run with: bash scripts/web3_paid_storage_edge_write_smoke.sh

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
need cmp

WALLET_ADDR="${SVC_WALLET_ADDR:-127.0.0.1:18088}"
ACCOUNTING_ADDR="${RON_ACCOUNTING_ADDR:-127.0.0.1:19600}"
STORAGE_ADDR="${ADDR:-127.0.0.1:15303}"
OMNIGATE_ADDR="${OMNIGATE_BIND_ADDR:-127.0.0.1:15305}"
OMNIGATE_METRICS_ADDR="${OMNIGATE_METRICS_ADDR:-127.0.0.1:19605}"
GATEWAY_ADDR="${SVC_GATEWAY_BIND_ADDR:-127.0.0.1:15304}"

WALLET_URL="http://${WALLET_ADDR}"
ACCOUNTING_URL="http://${ACCOUNTING_ADDR}"
STORAGE_URL="http://${STORAGE_ADDR}"
OMNIGATE_URL="http://${OMNIGATE_ADDR}"
GATEWAY_URL="http://${GATEWAY_ADDR}"

ECONOMICS_PATH="${RON_STORAGE_ROC_ECONOMICS_PATH:-configs/roc-economics.toml}"
ECONOMICS_ACTION="${RON_STORAGE_ROC_ECONOMICS_ACTION:-paid_storage_put}"
EXPECTED_CAPTURE_MINOR="${WEB3_EDGE_EXPECTED_CAPTURE_MINOR:-84}"
HOLD_PADDING_MINOR="${WEB3_EDGE_HOLD_PADDING_MINOR:-16}"

LOG_DIR="${ROOT_DIR}/target/web3-edge-write-smoke"
mkdir -p "$LOG_DIR"

WALLET_LOG="${LOG_DIR}/svc-wallet.log"
ACCOUNTING_LOG="${LOG_DIR}/ron-accounting.log"
STORAGE_LOG="${LOG_DIR}/svc-storage.log"
OMNIGATE_LOG="${LOG_DIR}/omnigate.log"
GATEWAY_LOG="${LOG_DIR}/svc-gateway.log"
OMNIGATE_CONFIG="${LOG_DIR}/omnigate.toml"

OBJ_FILE="${LOG_DIR}/paid-edge-object.bin"
GET_FILE="${LOG_DIR}/paid-edge-object.get.bin"
BAD_FILE="${LOG_DIR}/paid-edge-bad-object.bin"

: > "$WALLET_LOG"
: > "$ACCOUNTING_LOG"
: > "$STORAGE_LOG"
: > "$OMNIGATE_LOG"
: > "$GATEWAY_LOG"

WALLET_BIN="${ROOT_DIR}/target/debug/svc-wallet"
ACCOUNTING_BIN="${ROOT_DIR}/target/debug/ron_accounting_ingest"
STORAGE_BIN="${ROOT_DIR}/target/debug/svc-storage"
OMNIGATE_BIN="${ROOT_DIR}/target/debug/omnigate"
GATEWAY_BIN="${ROOT_DIR}/target/debug/svc-gateway"

WALLET_PID=""
ACCOUNTING_PID=""
STORAGE_PID=""
OMNIGATE_PID=""
GATEWAY_PID=""

print_log_tail() {
  local label="$1"
  local path="$2"

  echo
  echo "----- ${label}: ${path} -----"
  if [[ -f "$path" ]]; then
    tail -n 220 "$path" || true
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

  if [[ -n "${WALLET_PID}" ]] && kill -0 "${WALLET_PID}" >/dev/null 2>&1; then
    kill "${WALLET_PID}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${ACCOUNTING_PID}" ]] && kill -0 "${ACCOUNTING_PID}" >/dev/null 2>&1; then
    kill "${ACCOUNTING_PID}" >/dev/null 2>&1 || true
  fi

  for pid in "${GATEWAY_PID}" "${OMNIGATE_PID}" "${STORAGE_PID}" "${WALLET_PID}" "${ACCOUNTING_PID}"; do
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

wait_for_route() {
  local url="$1"
  local label="$2"
  local pid="$3"
  local log_path="$4"
  local deadline=$((SECONDS + 30))

  until curl -fsS --connect-timeout 1 --max-time 3 "$url" >/dev/null 2>&1; do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      echo "${label} exited before route became ready: ${url}" >&2
      print_log_tail "$label log" "$log_path" >&2
      exit 1
    fi

    if (( SECONDS >= deadline )); then
      echo "timeout waiting for ${label} route: ${url}" >&2
      curl -v --connect-timeout 1 --max-time 5 "$url" >&2 || true
      print_log_tail "$label log" "$log_path" >&2
      exit 1
    fi

    sleep 0.25
  done
}

json_post() {
  local url="$1"
  local idem="$2"
  local body="$3"

  curl -fsS --connect-timeout 1 --max-time 5 -X POST "$url" \
    -H "Authorization: Bearer dev" \
    -H "Idempotency-Key: ${idem}" \
    -H "Content-Type: application/json" \
    -d "$body"
}

wallet_balance() {
  local account="$1"

  curl -fsS --connect-timeout 1 --max-time 5 \
    -H "Authorization: Bearer dev" \
    "${WALLET_URL}/v1/balance?account=${account}&asset=roc" \
    | jq -r '.amount_minor'
}

cid_for_file() {
  cargo run -q -p svc-storage --bin roc_b3_tool -- cid "$1"
}

context_idem() {
  cargo run -q -p svc-storage --bin roc_b3_tool -- context-idem "$1" "$2" "$3" "$4" "$5"
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
    print_log_tail "ron-accounting log" "$ACCOUNTING_LOG" >&2
    print_log_tail "svc-wallet log" "$WALLET_LOG" >&2
    print_log_tail "svc-storage log" "$STORAGE_LOG" >&2
    print_log_tail "omnigate log" "$OMNIGATE_LOG" >&2
    print_log_tail "svc-gateway log" "$GATEWAY_LOG" >&2
    exit 1
  fi
}

require_u64() {
  local name="$1"
  local value="$2"

  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    echo "expected unsigned integer for ${name}, got '${value}'" >&2
    exit 1
  fi
}

if [[ ! -f "$ECONOMICS_PATH" ]]; then
  echo "missing economics config: ${ECONOMICS_PATH}" >&2
  exit 1
fi

require_u64 "WEB3_EDGE_EXPECTED_CAPTURE_MINOR" "$EXPECTED_CAPTURE_MINOR"
require_u64 "WEB3_EDGE_HOLD_PADDING_MINOR" "$HOLD_PADDING_MINOR"

echo "building svc-wallet, ron-accounting, svc-storage, omnigate, and svc-gateway"
cargo build -p svc-wallet -p ron-accounting -p svc-storage -p omnigate -p svc-gateway

for bin in "$WALLET_BIN" "$ACCOUNTING_BIN" "$STORAGE_BIN" "$OMNIGATE_BIN" "$GATEWAY_BIN"; do
  if [[ ! -x "$bin" ]]; then
    echo "missing compiled binary: ${bin}" >&2
    exit 1
  fi
done

printf 'RustyOnions WEB3 live paid-storage smoke object\n' > "$OBJ_FILE"

CID="$(cid_for_file "$OBJ_FILE")"
OBJECT_BYTES="$(wc -c < "$OBJ_FILE" | tr -d '[:space:]')"

PAYER="acct_user"
ESCROW="escrow_paid_write"
PAYEE="svc_storage"
ASSET="roc"

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

echo "object CID: ${CID}"
echo "object bytes: ${OBJECT_BYTES}"
echo "economics path: ${ECONOMICS_PATH}"
echo "economics action: ${ECONOMICS_ACTION}"

echo "starting ron-accounting ingest on ${ACCOUNTING_ADDR}"
RON_ACCOUNTING_ADDR="$ACCOUNTING_ADDR" \
RON_ACC_BEARER=dev \
"$ACCOUNTING_BIN" >"$ACCOUNTING_LOG" 2>&1 &
ACCOUNTING_PID="$!"

wait_for_http "${ACCOUNTING_URL}/healthz" "ron-accounting" "$ACCOUNTING_PID" "$ACCOUNTING_LOG"

echo "ron-accounting is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${ACCOUNTING_URL}/healthz" | jq .

echo "starting svc-wallet on ${WALLET_ADDR}"
SVC_WALLET_ADDR="$WALLET_ADDR" \
RUST_LOG="${RUST_LOG:-info,svc_wallet=debug,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$WALLET_BIN" >"$WALLET_LOG" 2>&1 &
WALLET_PID="$!"

wait_for_http "${WALLET_URL}/healthz" "svc-wallet" "$WALLET_PID" "$WALLET_LOG"

echo "svc-wallet is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${WALLET_URL}/healthz" | jq .

echo "starting svc-storage on ${STORAGE_ADDR} with wallet settlement, accounting export, and ROC economics"
ADDR="$STORAGE_ADDR" \
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt \
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture \
RON_STORAGE_PAID_SETTLEMENT_PAYEE="$PAYEE" \
RON_STORAGE_WALLET_BASE_URL="$WALLET_URL" \
RON_STORAGE_WALLET_BEARER=dev \
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS=2000 \
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http \
RON_STORAGE_ACCOUNTING_BASE_URL="$ACCOUNTING_URL" \
RON_STORAGE_ACCOUNTING_BEARER=dev \
RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000 \
RON_STORAGE_ROC_ECONOMICS_PATH="$ECONOMICS_PATH" \
RON_STORAGE_ROC_ECONOMICS_ACTION="$ECONOMICS_ACTION" \
RUST_LOG="${RUST_LOG:-info,svc_wallet=debug,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$STORAGE_BIN" >"$STORAGE_LOG" 2>&1 &
STORAGE_PID="$!"

wait_for_http "${STORAGE_URL}/healthz" "svc-storage" "$STORAGE_PID" "$STORAGE_LOG"

echo "svc-storage is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${STORAGE_URL}/healthz" | jq .

echo "starting omnigate on ${OMNIGATE_ADDR}; storage upstream ${STORAGE_URL}"
OMNIGATE_STORAGE_BASE_URL="$STORAGE_URL" \
OMNIGATE_DEV_READY=1 \
RUST_LOG="${RUST_LOG:-info,svc_wallet=debug,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$OMNIGATE_BIN" --config "$OMNIGATE_CONFIG" >"$OMNIGATE_LOG" 2>&1 &
OMNIGATE_PID="$!"

wait_for_route "${OMNIGATE_URL}/v1/paid/o/estimate?bytes=${OBJECT_BYTES}" "omnigate" "$OMNIGATE_PID" "$OMNIGATE_LOG"

echo "starting svc-gateway on ${GATEWAY_ADDR}; omnigate upstream ${OMNIGATE_URL}"
SVC_GATEWAY_BIND_ADDR="$GATEWAY_ADDR" \
SVC_GATEWAY_OMNIGATE_BASE_URL="$OMNIGATE_URL" \
RUST_LOG="${RUST_LOG:-info,svc_wallet=debug,svc_storage=debug,omnigate=debug,svc_gateway=debug}" \
"$GATEWAY_BIN" >"$GATEWAY_LOG" 2>&1 &
GATEWAY_PID="$!"

wait_for_http "${GATEWAY_URL}/healthz" "svc-gateway" "$GATEWAY_PID" "$GATEWAY_LOG"
wait_for_route "${GATEWAY_URL}/paid/o/estimate?bytes=${OBJECT_BYTES}" "svc-gateway" "$GATEWAY_PID" "$GATEWAY_LOG"

echo "checking product-facing gateway estimate"
GATEWAY_ESTIMATE_JSON="$(
  curl -fsS --connect-timeout 1 --max-time 5 \
    -H 'Authorization: Bearer dev' \
    -H 'x-ron-token: smoke-token' \
    -H 'x-ron-passport: smoke-passport' \
    -H 'x-correlation-id: web3-edge-write-smoke' \
    "${GATEWAY_URL}/paid/o/estimate?bytes=${OBJECT_BYTES}"
)"
echo "$GATEWAY_ESTIMATE_JSON" | jq .

assert_json_field "$GATEWAY_ESTIMATE_JSON" '.route' "/paid/o"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.action' "$ECONOMICS_ACTION"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.asset' "$ASSET"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.bytes | tostring' "$OBJECT_BYTES"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.amount_minor' "$EXPECTED_CAPTURE_MINOR"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.minimum_hold_minor' "$EXPECTED_CAPTURE_MINOR"
assert_json_field "$GATEWAY_ESTIMATE_JSON" '.pricing_mode' "roc-economics"

CAPTURE_EXPECTED="$(echo "$GATEWAY_ESTIMATE_JSON" | jq -r '.minimum_hold_minor')"
HOLD_AMOUNT="$((CAPTURE_EXPECTED + HOLD_PADDING_MINOR))"
RELEASE_EXPECTED="$((HOLD_AMOUNT - CAPTURE_EXPECTED))"

ISSUE_AMOUNT="${WEB3_EDGE_ISSUE_AMOUNT_MINOR:-100}"
require_u64 "WEB3_EDGE_ISSUE_AMOUNT_MINOR" "$ISSUE_AMOUNT"

if (( ISSUE_AMOUNT < HOLD_AMOUNT )); then
  ISSUE_AMOUNT="$((HOLD_AMOUNT + 30))"
fi

CTX_IDEM="$(context_idem "$CID" "$PAYER" "$ESCROW" "$ASSET" "$HOLD_AMOUNT")"

echo "capture expected: ${CAPTURE_EXPECTED}"
echo "hold amount:      ${HOLD_AMOUNT}"
echo "release expected: ${RELEASE_EXPECTED}"
echo "issue amount:     ${ISSUE_AMOUNT}"
echo "context idem:     ${CTX_IDEM}"

echo "issuing ROC to payer"
ISSUE_JSON="$(
  json_post "${WALLET_URL}/v1/issue" "idem_edge_issue_paid_storage" \
    "$(jq -nc \
      --arg to "$PAYER" \
      --arg asset "$ASSET" \
      --arg amount "$ISSUE_AMOUNT" \
      '{to:$to,asset:$asset,amount_minor:$amount,memo:"web3 edge paid storage live smoke"}')"
)"
echo "$ISSUE_JSON" | jq .

echo "creating wallet hold bound to paid-storage CID context"
HOLD_JSON="$(
  json_post "${WALLET_URL}/v1/hold" "$CTX_IDEM" \
    "$(jq -nc \
      --arg from "$PAYER" \
      --arg to "$ESCROW" \
      --arg asset "$ASSET" \
      --arg memo "edge paid_storage_put cid=${CID}" \
      --arg amount "$HOLD_AMOUNT" \
      '{from:$from,to:$to,asset:$asset,amount_minor:$amount,nonce:1,memo:$memo}')"
)"
echo "$HOLD_JSON" | jq .

HOLD_TXID="$(echo "$HOLD_JSON" | jq -r '.txid')"
HOLD_HASH="$(echo "$HOLD_JSON" | jq -r '.receipt_hash')"
HOLD_IDEM="$(echo "$HOLD_JSON" | jq -r '.idem')"

if [[ "$HOLD_IDEM" != "$CTX_IDEM" ]]; then
  echo "hold receipt idem mismatch: expected ${CTX_IDEM}, got ${HOLD_IDEM}" >&2
  exit 1
fi

echo "writing paid object through svc-gateway → omnigate → svc-storage"
PAID_JSON="$(
  curl -fsS --connect-timeout 1 --max-time 15 -X POST "${GATEWAY_URL}/paid/o" \
    -H "Content-Type: application/octet-stream" \
    -H "Authorization: Bearer dev" \
    -H "Idempotency-Key: idem_edge_paid_write" \
    -H "x-ron-paid-op: hold" \
    -H "x-ron-paid-asset: ${ASSET}" \
    -H "x-ron-paid-estimate-minor: ${HOLD_AMOUNT}" \
    -H "x-ron-wallet-txid: ${HOLD_TXID}" \
    -H "x-ron-wallet-receipt-hash: ${HOLD_HASH}" \
    -H "x-ron-wallet-from: ${PAYER}" \
    -H "x-ron-wallet-to: ${ESCROW}" \
    -H "x-ron-wallet-idem: ${CTX_IDEM}" \
    -H "x-ron-tenant: 7" \
    -H "x-ron-accounting-subject: svc_storage_provider" \
    -H "x-ron-region: local" \
    -H "x-ron-pin-seconds: 60" \
    -H "x-correlation-id: web3-edge-write-smoke" \
    --data-binary @"$OBJ_FILE"
)"
echo "$PAID_JSON" | jq .

assert_json_field "$PAID_JSON" '.cid' "$CID"
assert_json_field "$PAID_JSON" '.paid | tostring' "true"
assert_json_field "$PAID_JSON" '.payer' "$PAYER"
assert_json_field "$PAID_JSON" '.escrow' "$ESCROW"
assert_json_field "$PAID_JSON" '.wallet_txid' "$HOLD_TXID"
assert_json_field "$PAID_JSON" '.wallet_receipt_hash' "$HOLD_HASH"
assert_json_field "$PAID_JSON" '.wallet_idem' "$CTX_IDEM"
assert_json_field "$PAID_JSON" '.paid_context_idem' "$CTX_IDEM"
assert_json_field "$PAID_JSON" '.settlement.mode' "wallet-capture"
assert_json_field "$PAID_JSON" '.settlement.capture_amount_minor' "$CAPTURE_EXPECTED"
assert_json_field "$PAID_JSON" '.settlement.release_amount_minor' "$RELEASE_EXPECTED"
assert_json_field "$PAID_JSON" '.settlement.capture_receipt.op' "capture"
assert_json_field "$PAID_JSON" '.settlement.release_receipt.op' "release"
assert_json_field "$PAID_JSON" '.accounting_export.status' "exported"
assert_json_field "$PAID_JSON" '.accounting_export.http_status | tostring' "202"
assert_json_field "$PAID_JSON" '.accounting_export.event_count | tostring' "3"

echo "reading object back from svc-storage by CID"
curl -fsS --connect-timeout 1 --max-time 10 "${STORAGE_URL}/o/${CID}" -o "$GET_FILE"
cmp "$OBJ_FILE" "$GET_FILE"

echo "checking final wallet balances"
PAYER_BALANCE="$(wallet_balance "$PAYER")"
ESCROW_BALANCE="$(wallet_balance "$ESCROW")"
PAYEE_BALANCE="$(wallet_balance "$PAYEE")"

PAYER_EXPECTED="$((ISSUE_AMOUNT - CAPTURE_EXPECTED))"
ESCROW_EXPECTED="0"
PAYEE_EXPECTED="$CAPTURE_EXPECTED"

echo "payer balance:  ${PAYER_BALANCE}"
echo "escrow balance: ${ESCROW_BALANCE}"
echo "payee balance:  ${PAYEE_BALANCE}"

if [[ "$PAYER_BALANCE" != "$PAYER_EXPECTED" ]]; then
  echo "expected payer balance ${PAYER_EXPECTED}, got ${PAYER_BALANCE}" >&2
  exit 1
fi

if [[ "$ESCROW_BALANCE" != "$ESCROW_EXPECTED" ]]; then
  echo "expected escrow balance ${ESCROW_EXPECTED}, got ${ESCROW_BALANCE}" >&2
  exit 1
fi

if [[ "$PAYEE_BALANCE" != "$PAYEE_EXPECTED" ]]; then
  echo "expected payee balance ${PAYEE_EXPECTED}, got ${PAYEE_BALANCE}" >&2
  exit 1
fi

echo "checking real ron-accounting snapshot"
ACCOUNTING_SNAPSHOT="$(
  curl -fsS --connect-timeout 1 --max-time 5 "${ACCOUNTING_URL}/v1/snapshot"
)"
echo "$ACCOUNTING_SNAPSHOT" | jq .

ROW_COUNT="$(echo "$ACCOUNTING_SNAPSHOT" | jq -r '.row_count')"
BYTES_ROW="$(echo "$ACCOUNTING_SNAPSHOT" | jq -r --argjson object_bytes "$OBJECT_BYTES" '[.rows[] | select(.key.dimension=="bytes" and .value==$object_bytes)] | length')"
REQUEST_ROWS="$(echo "$ACCOUNTING_SNAPSHOT" | jq -r '[.rows[] | select(.key.dimension=="requests")] | length')"

if [[ "$ROW_COUNT" != "3" ]]; then
  echo "expected accounting row_count 3, got ${ROW_COUNT}" >&2
  exit 1
fi

if [[ "$BYTES_ROW" != "1" ]]; then
  echo "expected one bytes accounting row with value ${OBJECT_BYTES}, got ${BYTES_ROW}" >&2
  exit 1
fi

if [[ "$REQUEST_ROWS" != "2" ]]; then
  echo "expected two request accounting rows, got ${REQUEST_ROWS}" >&2
  exit 1
fi

echo "proving same hold cannot be replayed for different bytes through the full edge chain"
printf 'different edge body must not reuse the same wallet hold\n' > "$BAD_FILE"

BAD_STATUS="$(
  curl -sS --connect-timeout 1 --max-time 15 \
    -o "${LOG_DIR}/bad-edge-paid-write.json" \
    -w '%{http_code}' \
    -X POST "${GATEWAY_URL}/paid/o" \
    -H "Content-Type: application/octet-stream" \
    -H "Authorization: Bearer dev" \
    -H "Idempotency-Key: idem_edge_paid_write_replay_bad" \
    -H "x-ron-paid-op: hold" \
    -H "x-ron-paid-asset: ${ASSET}" \
    -H "x-ron-paid-estimate-minor: ${HOLD_AMOUNT}" \
    -H "x-ron-wallet-txid: ${HOLD_TXID}" \
    -H "x-ron-wallet-receipt-hash: ${HOLD_HASH}" \
    -H "x-ron-wallet-from: ${PAYER}" \
    -H "x-ron-wallet-to: ${ESCROW}" \
    -H "x-ron-wallet-idem: ${CTX_IDEM}" \
    -H "x-correlation-id: web3-edge-write-smoke-replay" \
    --data-binary @"$BAD_FILE"
)"

if [[ "$BAD_STATUS" != "402" ]]; then
  echo "expected replay with different body to fail with 402, got ${BAD_STATUS}" >&2
  cat "${LOG_DIR}/bad-edge-paid-write.json" >&2 || true
  print_log_tail "ron-accounting log" "$ACCOUNTING_LOG" >&2
  print_log_tail "svc-wallet log" "$WALLET_LOG" >&2
  print_log_tail "svc-storage log" "$STORAGE_LOG" >&2
  print_log_tail "omnigate log" "$OMNIGATE_LOG" >&2
  print_log_tail "svc-gateway log" "$GATEWAY_LOG" >&2
  exit 1
fi

BAD_JSON="$(cat "${LOG_DIR}/bad-edge-paid-write.json")"
echo "$BAD_JSON" | jq .
assert_json_field "$BAD_JSON" '.error' "payment_required"

echo "checking accounting row count stayed stable after rejected replay"
ACCOUNTING_AFTER_REPLAY="$(
  curl -fsS --connect-timeout 1 --max-time 5 "${ACCOUNTING_URL}/v1/snapshot"
)"
ROW_COUNT_AFTER_REPLAY="$(echo "$ACCOUNTING_AFTER_REPLAY" | jq -r '.row_count')"

if [[ "$ROW_COUNT_AFTER_REPLAY" != "3" ]]; then
  echo "expected accounting row_count to remain 3 after rejected replay, got ${ROW_COUNT_AFTER_REPLAY}" >&2
  echo "$ACCOUNTING_AFTER_REPLAY" | jq . >&2
  exit 1
fi

echo
echo "WEB3 edge paid storage write smoke green"
echo "path:             svc-gateway → omnigate → svc-storage → wallet/accounting"
echo "gateway estimate: ${GATEWAY_URL}/paid/o/estimate?bytes=${OBJECT_BYTES}"
echo "gateway write:    ${GATEWAY_URL}/paid/o"
echo "storage readback: ${STORAGE_URL}/o/${CID}"
echo "cid:              ${CID}"
echo "object_bytes:     ${OBJECT_BYTES}"
echo "capture_minor:    ${CAPTURE_EXPECTED}"
echo "release_minor:    ${RELEASE_EXPECTED}"
echo "payer_balance:    ${PAYER_BALANCE}"
echo "payee_balance:    ${PAYEE_BALANCE}"
echo "accounting_rows:  3"
echo "replay_rejection: 402 payment_required"
echo "logs:"
echo "  accounting: ${ACCOUNTING_LOG}"
echo "  wallet:     ${WALLET_LOG}"
echo "  storage:    ${STORAGE_LOG}"
echo "  omnigate:   ${OMNIGATE_LOG}"
echo "  gateway:    ${GATEWAY_LOG}"