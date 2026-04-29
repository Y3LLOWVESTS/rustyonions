#!/usr/bin/env bash
# RO:WHAT — Live WEB3 smoke for wallet + storage paid write + accounting ingest + optional ROC economics pricing.
# RO:WHY — Ops/CI; Concerns: ECON/RES/GOV. Proves hold→storage→capture→release→usage export.
# RO:INTERACTS — svc-wallet, svc-storage, ron-accounting ingest; /paid/o, /v1/usage-events, /v1/snapshot.
# RO:INVARIANTS — wallet is mutation front-door; storage never mutates ledger; accounting records usage only.
# RO:METRICS — prints request results; service logs go to target/web3-smoke/*.log.
# RO:CONFIG — SVC_WALLET_ADDR, ADDR, RON_ACCOUNTING_ADDR, RON_STORAGE_*, WEB3_PAID_STORAGE_USE_ECONOMICS.
# RO:SECURITY — uses local dev bearer only; does not echo real secrets.
# RO:TEST — run with: bash scripts/web3_paid_storage_live_smoke.sh

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
STORAGE_ADDR="${ADDR:-127.0.0.1:15303}"
ACCOUNTING_ADDR="${RON_ACCOUNTING_ADDR:-127.0.0.1:19600}"

WALLET_URL="http://${WALLET_ADDR}"
STORAGE_URL="http://${STORAGE_ADDR}"
ACCOUNTING_URL="http://${ACCOUNTING_ADDR}"

USE_ECONOMICS="${WEB3_PAID_STORAGE_USE_ECONOMICS:-1}"
ECONOMICS_PATH="${RON_STORAGE_ROC_ECONOMICS_PATH:-configs/roc-economics.toml}"
ECONOMICS_ACTION="${RON_STORAGE_ROC_ECONOMICS_ACTION:-paid_storage_put}"

LOG_DIR="${ROOT_DIR}/target/web3-smoke"
OBJ_FILE="${LOG_DIR}/paid-object.bin"
GET_FILE="${LOG_DIR}/paid-object.get.bin"

WALLET_BIN="${ROOT_DIR}/target/debug/svc-wallet"
STORAGE_BIN="${ROOT_DIR}/target/debug/svc-storage"
ACCOUNTING_BIN="${ROOT_DIR}/target/debug/ron_accounting_ingest"

mkdir -p "$LOG_DIR"
: > "${LOG_DIR}/svc-wallet.log"
: > "${LOG_DIR}/svc-storage.log"
: > "${LOG_DIR}/ron-accounting.log"

WALLET_PID=""
STORAGE_PID=""
ACCOUNTING_PID=""

print_log_tail() {
  local label="$1"
  local path="$2"

  echo
  echo "----- ${label}: ${path} -----"
  if [[ -f "$path" ]]; then
    tail -n 160 "$path" || true
  else
    echo "log file not found"
  fi
  echo "----- end ${label} -----"
  echo
}

cleanup() {
  if [[ -n "${STORAGE_PID}" ]] && kill -0 "${STORAGE_PID}" >/dev/null 2>&1; then
    kill "${STORAGE_PID}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${WALLET_PID}" ]] && kill -0 "${WALLET_PID}" >/dev/null 2>&1; then
    kill "${WALLET_PID}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${ACCOUNTING_PID}" ]] && kill -0 "${ACCOUNTING_PID}" >/dev/null 2>&1; then
    kill "${ACCOUNTING_PID}" >/dev/null 2>&1 || true
  fi
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
  local deadline=$((SECONDS + 20))

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

toml_action_value() {
  local action="$1"
  local key="$2"
  local file="$3"

  awk -v action="$action" -v key="$key" '
    BEGIN {
      section = "[actions." action "]";
      in_section = 0;
    }

    /^[[:space:]]*\[/ {
      line = $0;
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", line);
      in_section = (line == section);
    }

    in_section && $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
      val = $0;
      sub(/^[^=]*=/, "", val);
      sub(/[[:space:]]*#.*/, "", val);
      gsub(/^[[:space:]"]+/, "", val);
      gsub(/[[:space:]"]+$/, "", val);
      print val;
      exit;
    }
  ' "$file"
}

require_u64() {
  local name="$1"
  local value="$2"

  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    echo "expected unsigned integer for ${name}, got '${value}'" >&2
    exit 1
  fi
}

calculate_capture_amount() {
  local bytes_stored="$1"

  if [[ "$USE_ECONOMICS" != "1" ]]; then
    if (( bytes_stored < 1 )); then
      echo "1"
    else
      echo "$bytes_stored"
    fi
    return 0
  fi

  if [[ ! -f "$ECONOMICS_PATH" ]]; then
    echo "missing ROC economics config: ${ECONOMICS_PATH}" >&2
    echo "Either copy roc-economics.toml to configs/roc-economics.toml or run with WEB3_PAID_STORAGE_USE_ECONOMICS=0." >&2
    exit 1
  fi

  local pricing_kind
  local price_per_byte
  local minimum_charge
  local multiplier_bps

  pricing_kind="$(toml_action_value "$ECONOMICS_ACTION" "pricing_kind" "$ECONOMICS_PATH")"
  price_per_byte="$(toml_action_value "$ECONOMICS_ACTION" "price_per_byte_minor" "$ECONOMICS_PATH")"
  minimum_charge="$(toml_action_value "$ECONOMICS_ACTION" "minimum_charge_minor" "$ECONOMICS_PATH")"
  multiplier_bps="$(toml_action_value "$ECONOMICS_ACTION" "max_hold_multiplier_bps" "$ECONOMICS_PATH")"

  if [[ "$pricing_kind" != "per_byte_plus_minimum" ]]; then
    echo "this smoke currently supports per_byte_plus_minimum pricing only; ${ECONOMICS_ACTION} has pricing_kind='${pricing_kind}'" >&2
    exit 1
  fi

  require_u64 "price_per_byte_minor" "$price_per_byte"
  require_u64 "minimum_charge_minor" "$minimum_charge"
  require_u64 "max_hold_multiplier_bps" "$multiplier_bps"

  local base_amount
  base_amount=$((bytes_stored * price_per_byte))

  if (( base_amount < minimum_charge )); then
    base_amount="$minimum_charge"
  fi

  local capture_amount
  capture_amount=$(((base_amount * multiplier_bps) / 10000))

  if (( capture_amount < 1 )); then
    capture_amount=1
  fi

  echo "$capture_amount"
}

echo "building svc-wallet, svc-storage, ron-accounting ingest, and local BLAKE3 helper"
cargo build -p svc-wallet -p svc-storage -p ron-accounting

if [[ ! -x "$WALLET_BIN" ]]; then
  echo "missing compiled wallet binary: ${WALLET_BIN}" >&2
  exit 1
fi

if [[ ! -x "$STORAGE_BIN" ]]; then
  echo "missing compiled storage binary: ${STORAGE_BIN}" >&2
  exit 1
fi

if [[ ! -x "$ACCOUNTING_BIN" ]]; then
  echo "missing compiled accounting binary: ${ACCOUNTING_BIN}" >&2
  exit 1
fi

printf 'RustyOnions WEB3 live paid-storage smoke object\n' > "$OBJ_FILE"

CID="$(cid_for_file "$OBJ_FILE")"
OBJECT_BYTES="$(wc -c < "$OBJ_FILE" | tr -d '[:space:]')"

PAYER="acct_user"
ESCROW="escrow_paid_write"
PAYEE="svc_storage"
ASSET="roc"

CAPTURE_EXPECTED="$(calculate_capture_amount "$OBJECT_BYTES")"

if [[ "$USE_ECONOMICS" == "1" ]]; then
  PRICING_LABEL="roc-economics:${ECONOMICS_ACTION}"
  AMOUNT_DEFAULT="$((CAPTURE_EXPECTED + 16))"
else
  PRICING_LABEL="legacy:max(bytes,1)"
  AMOUNT_DEFAULT="70"
fi

AMOUNT="${WEB3_PAID_STORAGE_HOLD_AMOUNT_MINOR:-$AMOUNT_DEFAULT}"

require_u64 "WEB3_PAID_STORAGE_HOLD_AMOUNT_MINOR" "$AMOUNT"

if (( CAPTURE_EXPECTED > AMOUNT )); then
  echo "test object costs ${CAPTURE_EXPECTED}, which exceeds hold amount ${AMOUNT}" >&2
  echo "Set WEB3_PAID_STORAGE_HOLD_AMOUNT_MINOR to at least ${CAPTURE_EXPECTED}." >&2
  exit 1
fi

ISSUE_DEFAULT="100"
if (( AMOUNT > ISSUE_DEFAULT )); then
  ISSUE_DEFAULT="$((AMOUNT + 30))"
fi

ISSUE_AMOUNT="${WEB3_PAID_STORAGE_ISSUE_AMOUNT_MINOR:-$ISSUE_DEFAULT}"

require_u64 "WEB3_PAID_STORAGE_ISSUE_AMOUNT_MINOR" "$ISSUE_AMOUNT"

if (( AMOUNT > ISSUE_AMOUNT )); then
  echo "hold amount ${AMOUNT} exceeds issue amount ${ISSUE_AMOUNT}" >&2
  echo "Set WEB3_PAID_STORAGE_ISSUE_AMOUNT_MINOR to at least ${AMOUNT}." >&2
  exit 1
fi

RELEASE_EXPECTED="$((AMOUNT - CAPTURE_EXPECTED))"
CTX_IDEM="$(context_idem "$CID" "$PAYER" "$ESCROW" "$ASSET" "$AMOUNT")"

echo "object CID: ${CID}"
echo "object bytes: ${OBJECT_BYTES}"
echo "pricing mode: ${PRICING_LABEL}"
if [[ "$USE_ECONOMICS" == "1" ]]; then
  echo "economics path: ${ECONOMICS_PATH}"
  echo "economics action: ${ECONOMICS_ACTION}"
fi
echo "context idem: ${CTX_IDEM}"
echo "issue amount: ${ISSUE_AMOUNT}"
echo "hold amount: ${AMOUNT}"
echo "expected capture: ${CAPTURE_EXPECTED}"
echo "expected release: ${RELEASE_EXPECTED}"

echo "starting ron-accounting ingest on ${ACCOUNTING_ADDR}"
RON_ACCOUNTING_ADDR="$ACCOUNTING_ADDR" \
RON_ACC_BEARER=dev \
"$ACCOUNTING_BIN" >"${LOG_DIR}/ron-accounting.log" 2>&1 &
ACCOUNTING_PID="$!"

wait_for_http "${ACCOUNTING_URL}/healthz" "ron-accounting" "$ACCOUNTING_PID" "${LOG_DIR}/ron-accounting.log"

echo "ron-accounting is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${ACCOUNTING_URL}/healthz" | jq .

echo "starting svc-wallet on ${WALLET_ADDR}"
SVC_WALLET_ADDR="$WALLET_ADDR" \
RUST_LOG="${RUST_LOG:-svc_wallet=debug,svc_storage=debug,tower_http=debug}" \
"$WALLET_BIN" >"${LOG_DIR}/svc-wallet.log" 2>&1 &
WALLET_PID="$!"

wait_for_http "${WALLET_URL}/healthz" "svc-wallet" "$WALLET_PID" "${LOG_DIR}/svc-wallet.log"

echo "svc-wallet is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${WALLET_URL}/healthz" | jq .

echo "starting svc-storage on ${STORAGE_ADDR} in wallet-receipt + wallet-capture + accounting-http + ${PRICING_LABEL} mode"
(
  export ADDR="$STORAGE_ADDR"
  export RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
  export RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
  export RON_STORAGE_PAID_SETTLEMENT_PAYEE=svc_storage
  export RON_STORAGE_WALLET_BASE_URL="$WALLET_URL"
  export RON_STORAGE_WALLET_BEARER=dev
  export RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS=2000
  export RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
  export RON_STORAGE_ACCOUNTING_BASE_URL="$ACCOUNTING_URL"
  export RON_STORAGE_ACCOUNTING_BEARER=dev
  export RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000
  export RUST_LOG="${RUST_LOG:-svc_wallet=debug,svc_storage=debug,tower_http=debug}"

  if [[ "$USE_ECONOMICS" == "1" ]]; then
    export RON_STORAGE_ROC_ECONOMICS_PATH="$ECONOMICS_PATH"
    export RON_STORAGE_ROC_ECONOMICS_ACTION="$ECONOMICS_ACTION"
  else
    unset RON_STORAGE_ROC_ECONOMICS_PATH
    unset RON_STORAGE_ROC_ECONOMICS_ACTION
  fi

  exec "$STORAGE_BIN"
) >"${LOG_DIR}/svc-storage.log" 2>&1 &
STORAGE_PID="$!"

wait_for_http "${STORAGE_URL}/healthz" "svc-storage" "$STORAGE_PID" "${LOG_DIR}/svc-storage.log"

echo "svc-storage is healthy"
curl -fsS --connect-timeout 1 --max-time 3 "${STORAGE_URL}/healthz" | jq .

echo "issuing ROC to payer"
ISSUE_JSON="$(
  json_post "${WALLET_URL}/v1/issue" "idem_live_issue_paid_storage" \
    "$(jq -nc \
      --arg to "$PAYER" \
      --arg asset "$ASSET" \
      --arg amount "$ISSUE_AMOUNT" \
      '{to:$to,asset:$asset,amount_minor:$amount,memo:"web3 paid storage live smoke"}')"
)"
echo "$ISSUE_JSON" | jq .

echo "creating wallet hold bound to paid-storage context"
HOLD_JSON="$(
  json_post "${WALLET_URL}/v1/hold" "$CTX_IDEM" \
    "$(jq -nc \
      --arg from "$PAYER" \
      --arg to "$ESCROW" \
      --arg asset "$ASSET" \
      --arg memo "paid_storage_put cid=${CID}" \
      --arg amount "$AMOUNT" \
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

echo "writing paid object through svc-storage; storage should capture/release and export usage"
PAID_JSON="$(
  curl -fsS --connect-timeout 1 --max-time 10 -X POST "${STORAGE_URL}/paid/o" \
    -H "Content-Type: application/octet-stream" \
    -H "x-ron-paid-op: hold" \
    -H "x-ron-paid-asset: ${ASSET}" \
    -H "x-ron-paid-estimate-minor: ${AMOUNT}" \
    -H "x-ron-wallet-txid: ${HOLD_TXID}" \
    -H "x-ron-wallet-receipt-hash: ${HOLD_HASH}" \
    -H "x-ron-wallet-from: ${PAYER}" \
    -H "x-ron-wallet-to: ${ESCROW}" \
    -H "x-ron-wallet-idem: ${CTX_IDEM}" \
    -H "x-ron-tenant: 7" \
    -H "x-ron-accounting-subject: svc_storage_provider" \
    -H "x-ron-region: local" \
    -H "x-ron-pin-seconds: 60" \
    --data-binary @"$OBJ_FILE"
)"
echo "$PAID_JSON" | jq .

PAID_CID="$(echo "$PAID_JSON" | jq -r '.cid')"
PAID_CONTEXT="$(echo "$PAID_JSON" | jq -r '.paid_context_idem')"
SETTLEMENT_MODE="$(echo "$PAID_JSON" | jq -r '.settlement.mode')"
CAPTURE_AMOUNT="$(echo "$PAID_JSON" | jq -r '.settlement.capture_amount_minor')"
RELEASE_AMOUNT="$(echo "$PAID_JSON" | jq -r '.settlement.release_amount_minor')"
CAPTURE_OP="$(echo "$PAID_JSON" | jq -r '.settlement.capture_receipt.op')"
RELEASE_OP="$(echo "$PAID_JSON" | jq -r '.settlement.release_receipt.op')"
ACCOUNTING_STATUS="$(echo "$PAID_JSON" | jq -r '.accounting_export.status')"
ACCOUNTING_HTTP_STATUS="$(echo "$PAID_JSON" | jq -r '.accounting_export.http_status')"
ACCOUNTING_EVENTS="$(echo "$PAID_JSON" | jq -r '.accounting_export.event_count')"
ACCOUNTING_IDEM="$(echo "$PAID_JSON" | jq -r '.accounting_export.idempotency_key')"

if [[ "$PAID_CID" != "$CID" ]]; then
  echo "paid write CID mismatch: expected ${CID}, got ${PAID_CID}" >&2
  exit 1
fi

if [[ "$PAID_CONTEXT" != "$CTX_IDEM" ]]; then
  echo "paid context mismatch: expected ${CTX_IDEM}, got ${PAID_CONTEXT}" >&2
  exit 1
fi

if [[ "$SETTLEMENT_MODE" != "wallet-capture" ]]; then
  echo "expected settlement mode wallet-capture, got ${SETTLEMENT_MODE}" >&2
  exit 1
fi

if [[ "$CAPTURE_AMOUNT" != "$CAPTURE_EXPECTED" ]]; then
  echo "expected capture amount ${CAPTURE_EXPECTED}, got ${CAPTURE_AMOUNT}" >&2
  exit 1
fi

if [[ "$RELEASE_AMOUNT" != "$RELEASE_EXPECTED" ]]; then
  echo "expected release amount ${RELEASE_EXPECTED}, got ${RELEASE_AMOUNT}" >&2
  exit 1
fi

if [[ "$CAPTURE_OP" != "capture" ]]; then
  echo "expected capture receipt op=capture, got ${CAPTURE_OP}" >&2
  exit 1
fi

if [[ "$RELEASE_OP" != "release" ]]; then
  echo "expected release receipt op=release, got ${RELEASE_OP}" >&2
  exit 1
fi

if [[ "$ACCOUNTING_STATUS" != "exported" ]]; then
  echo "expected accounting export status exported, got ${ACCOUNTING_STATUS}" >&2
  exit 1
fi

if [[ "$ACCOUNTING_HTTP_STATUS" != "202" ]]; then
  echo "expected accounting export http_status 202, got ${ACCOUNTING_HTTP_STATUS}" >&2
  exit 1
fi

if [[ "$ACCOUNTING_EVENTS" != "3" ]]; then
  echo "expected accounting export event_count 3, got ${ACCOUNTING_EVENTS}" >&2
  exit 1
fi

if [[ "$ACCOUNTING_IDEM" != storage_acct:* ]]; then
  echo "expected accounting idempotency key storage_acct:*, got ${ACCOUNTING_IDEM}" >&2
  exit 1
fi

echo "reading object back"
curl -fsS --connect-timeout 1 --max-time 10 "${STORAGE_URL}/o/${CID}" -o "$GET_FILE"
cmp "$OBJ_FILE" "$GET_FILE"

echo "checking final wallet balances"
PAYER_BALANCE="$(wallet_balance "$PAYER")"
ESCROW_BALANCE="$(wallet_balance "$ESCROW")"
PAYEE_BALANCE="$(wallet_balance "$PAYEE")"

PAYER_EXPECTED="$((ISSUE_AMOUNT - AMOUNT + RELEASE_EXPECTED))"
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

echo "proving same hold cannot be replayed for different bytes"
BAD_FILE="${LOG_DIR}/bad-object.bin"
printf 'different body must not reuse the same wallet hold\n' > "$BAD_FILE"

BAD_STATUS="$(
  curl -sS --connect-timeout 1 --max-time 10 -o "${LOG_DIR}/bad-response.json" -w '%{http_code}' -X POST "${STORAGE_URL}/paid/o" \
    -H "Content-Type: application/octet-stream" \
    -H "x-ron-paid-op: hold" \
    -H "x-ron-paid-asset: ${ASSET}" \
    -H "x-ron-paid-estimate-minor: ${AMOUNT}" \
    -H "x-ron-wallet-txid: ${HOLD_TXID}" \
    -H "x-ron-wallet-receipt-hash: ${HOLD_HASH}" \
    -H "x-ron-wallet-from: ${PAYER}" \
    -H "x-ron-wallet-to: ${ESCROW}" \
    -H "x-ron-wallet-idem: ${CTX_IDEM}" \
    --data-binary @"$BAD_FILE"
)"

if [[ "$BAD_STATUS" != "402" ]]; then
  echo "expected replay with different body to fail with 402, got ${BAD_STATUS}" >&2
  cat "${LOG_DIR}/bad-response.json" >&2 || true
  print_log_tail "ron-accounting log" "${LOG_DIR}/ron-accounting.log" >&2
  print_log_tail "svc-wallet log" "${LOG_DIR}/svc-wallet.log" >&2
  print_log_tail "svc-storage log" "${LOG_DIR}/svc-storage.log" >&2
  exit 1
fi

jq . "${LOG_DIR}/bad-response.json"

echo "WEB3 paid-storage live smoke with settlement + real accounting export green"
echo "pricing mode:   ${PRICING_LABEL}"
echo "capture amount: ${CAPTURE_EXPECTED}"
echo "release amount: ${RELEASE_EXPECTED}"
echo "accounting log: ${LOG_DIR}/ron-accounting.log"
echo "wallet log:     ${LOG_DIR}/svc-wallet.log"
echo "storage log:    ${LOG_DIR}/svc-storage.log"