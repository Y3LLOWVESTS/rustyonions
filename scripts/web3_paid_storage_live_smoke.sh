#!/usr/bin/env bash
# RO:WHAT — Live WEB3 paid-storage smoke with wallet hold/capture/release and accounting export.
# RO:WHY — Proves paid storage uses svc-wallet + ron-ledger receipts and exports real ron-accounting usage.
# RO:INTERACTS — svc-wallet, svc-storage, ron-accounting ingest, roc_b3_tool, optional ron-policy ROC economics.
# RO:INVARIANTS — storage stores bytes only; wallet remains mutation front-door; no direct ledger mutation from storage.
# RO:METRICS — child services expose /metrics; smoke validates accounting rows and printed settlement receipts.
# RO:CONFIG — WEB3_PAID_STORAGE_USE_ECONOMICS, RON_STORAGE_ROC_ECONOMICS_PATH, RON_STORAGE_ROC_ECONOMICS_ACTION.
# RO:SECURITY — local/dev smoke only; bearer defaults to dev; no production secrets.
# RO:TEST — bash scripts/web3_paid_storage_live_smoke.sh.

set -euo pipefail
IFS=$'\n\t'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

need_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "missing required tool: ${tool}" >&2
    exit 127
  fi
}

need_tool curl
need_tool jq
need_tool python3

WORK_DIR="${WORK_DIR:-${ROOT}/target/web3-smoke}"
mkdir -p "${WORK_DIR}"

ACCOUNTING_ADDR="${RON_ACCOUNTING_ADDR:-127.0.0.1:19600}"
WALLET_ADDR="${SVC_WALLET_ADDR:-127.0.0.1:18088}"
STORAGE_ADDR="${ADDR:-127.0.0.1:15303}"

ACCOUNTING_BASE_URL="http://${ACCOUNTING_ADDR}"
WALLET_BASE_URL="http://${WALLET_ADDR}"
STORAGE_BASE_URL="http://${STORAGE_ADDR}"

BEARER="${WEB3_SMOKE_BEARER:-dev}"
ASSET="${WEB3_SMOKE_ASSET:-roc}"
PAYER="${WEB3_SMOKE_PAYER:-acct_user}"
ESCROW="${WEB3_SMOKE_ESCROW:-escrow_paid_write}"
PAYEE="${WEB3_SMOKE_PAYEE:-svc_storage}"

TENANT="${WEB3_SMOKE_TENANT:-7}"
ACCOUNTING_SUBJECT="${WEB3_SMOKE_ACCOUNTING_SUBJECT:-svc_storage_provider}"
REGION="${WEB3_SMOKE_REGION:-local}"
PIN_SECONDS="${WEB3_SMOKE_PIN_SECONDS:-60}"

WEB3_PAID_STORAGE_USE_ECONOMICS="${WEB3_PAID_STORAGE_USE_ECONOMICS:-0}"
RON_STORAGE_ROC_ECONOMICS_PATH="${RON_STORAGE_ROC_ECONOMICS_PATH:-}"
RON_STORAGE_ROC_ECONOMICS_ACTION="${RON_STORAGE_ROC_ECONOMICS_ACTION:-paid_storage_put}"

OBJECT_FILE="${WORK_DIR}/paid-object.bin"
OBJECT_GET_FILE="${WORK_DIR}/paid-object.get.bin"
BAD_OBJECT_FILE="${WORK_DIR}/bad-object.bin"
BAD_RESPONSE_FILE="${WORK_DIR}/bad-response.json"

ACCOUNTING_LOG="${WORK_DIR}/ron-accounting.log"
WALLET_LOG="${WORK_DIR}/svc-wallet.log"
STORAGE_LOG="${WORK_DIR}/svc-storage.log"

ACCOUNTING_PID=""
WALLET_PID=""
STORAGE_PID=""

cleanup() {
  local code=$?

  for pid in "${STORAGE_PID}" "${WALLET_PID}" "${ACCOUNTING_PID}"; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" >/dev/null 2>&1; then
      kill "${pid}" >/dev/null 2>&1 || true
    fi
  done

  wait "${STORAGE_PID:-}" >/dev/null 2>&1 || true
  wait "${WALLET_PID:-}" >/dev/null 2>&1 || true
  wait "${ACCOUNTING_PID:-}" >/dev/null 2>&1 || true

  exit "${code}"
}

trap cleanup EXIT INT TERM

http_status() {
  local method="$1"
  local url="$2"
  local out="$3"
  shift 3

  curl -sS \
    -X "${method}" \
    -o "${out}" \
    -w "%{http_code}" \
    "$@" \
    "${url}"
}

assert_2xx() {
  local status="$1"
  local label="$2"
  local body="$3"

  case "${status}" in
    2*) return 0 ;;
    *)
      echo "---- ${label} failed with HTTP ${status} ----" >&2
      cat "${body}" >&2 || true
      echo >&2
      exit 1
      ;;
  esac
}

wait_for_health() {
  local url="$1"
  local label="$2"
  local out="${WORK_DIR}/wait-${label}.json"
  local status

  for _ in $(seq 1 120); do
    status="$(http_status GET "${url}" "${out}" -H "accept: application/json" || true)"
    case "${status}" in
      2*)
        echo "${label} is healthy"
        cat "${out}" | jq . || cat "${out}"
        return 0
        ;;
    esac
    sleep 0.25
  done

  echo "service did not become healthy: ${label} at ${url}" >&2
  cat "${out}" >&2 || true
  exit 1
}

wallet_balance() {
  local account="$1"
  local out="${WORK_DIR}/balance-${account}.json"
  local status

  status="$(http_status GET "${WALLET_BASE_URL}/v1/balance?account=${account}&asset=${ASSET}" "${out}" \
    -H "authorization: Bearer ${BEARER}" \
    -H "accept: application/json")"

  assert_2xx "${status}" "wallet balance ${account}" "${out}"
  jq -r '.amount_minor // .available_minor_units // "0"' "${out}"
}

resolve_price_plan() {
  local bytes="$1"

  if [[ "${WEB3_PAID_STORAGE_USE_ECONOMICS}" != "1" ]]; then
    local capture
    if (( bytes > 0 )); then
      capture="${bytes}"
    else
      capture="1"
    fi

    local hold="${WEB3_PAID_STORAGE_HOLD_MINOR:-70}"
    if (( hold < capture )); then
      hold="${capture}"
    fi

    cat <<PLAN
PRICE_MODE='legacy:max(bytes,1)'
CAPTURE_MINOR='${capture}'
HOLD_MINOR='${hold}'
RELEASE_MINOR='$((hold - capture))'
PLAN
    return 0
  fi

  if [[ -z "${RON_STORAGE_ROC_ECONOMICS_PATH}" ]]; then
    echo "WEB3_PAID_STORAGE_USE_ECONOMICS=1 requires RON_STORAGE_ROC_ECONOMICS_PATH" >&2
    exit 1
  fi

  python3 - "${RON_STORAGE_ROC_ECONOMICS_PATH}" "${RON_STORAGE_ROC_ECONOMICS_ACTION}" "${bytes}" <<'PY'
import math
import pathlib
import re
import shlex
import sys

path = pathlib.Path(sys.argv[1])
action_name = sys.argv[2]
bytes_stored = int(sys.argv[3])

if not path.exists():
    raise SystemExit(f"missing ROC economics config: {path}")

target = f"actions.{action_name}"
current = None
action = {}

section_re = re.compile(r"^\[([A-Za-z0-9_.-]+)\]\s*$")
kv_re = re.compile(r"^([A-Za-z0-9_.-]+)\s*=\s*(.+?)\s*$")

for raw in path.read_text().splitlines():
    line = raw.split("#", 1)[0].strip()
    if not line:
        continue

    section = section_re.match(line)
    if section:
        current = section.group(1)
        continue

    if current != target:
        continue

    kv = kv_re.match(line)
    if not kv:
        continue

    key, value = kv.group(1), kv.group(2).strip()
    if value.startswith('"') and value.endswith('"'):
        value = value[1:-1]
    action[key] = value

if not action:
    raise SystemExit(f"missing economics action: {action_name}")

enabled = str(action.get("enabled", "false")).lower()
if enabled != "true":
    raise SystemExit(f"economics action is disabled: {action_name}")

pricing_kind = action.get("pricing_kind", "").strip()
minimum = int(action.get("minimum_charge_minor", "0") or "0")
max_spend = int(action.get("max_spend_minor", "0") or "0")
hold_multiplier_bps = int(action.get("max_hold_multiplier_bps", "10000") or "10000")

if pricing_kind == "flat":
    price = int(action.get("price_minor", "0") or "0")
    capture = max(price, minimum)
elif pricing_kind == "per_byte_plus_minimum":
    per_byte = int(
        action.get("price_per_byte_minor")
        or action.get("per_byte_minor")
        or action.get("byte_price_minor")
        or action.get("price_minor")
        or "1"
    )
    capture = minimum + (bytes_stored * per_byte)
elif pricing_kind == "per_byte":
    per_byte = int(
        action.get("price_per_byte_minor")
        or action.get("per_byte_minor")
        or action.get("byte_price_minor")
        or action.get("price_minor")
        or "1"
    )
    capture = max(minimum, bytes_stored * per_byte)
else:
    raise SystemExit(
        f"this smoke supports flat, per_byte, and per_byte_plus_minimum pricing; "
        f"{action_name} has pricing_kind={pricing_kind!r}"
    )

if capture <= 0:
    raise SystemExit(f"computed non-positive capture amount: {capture}")

if max_spend > 0 and capture > max_spend:
    raise SystemExit(f"computed capture {capture} exceeds max_spend_minor {max_spend}")

hold = math.ceil(capture * hold_multiplier_bps / 10000)
hold = max(hold, capture)
release = hold - capture

mode = f"economics:{action_name}:{pricing_kind}"
print(f"PRICE_MODE={shlex.quote(mode)}")
print(f"CAPTURE_MINOR={capture}")
print(f"HOLD_MINOR={hold}")
print(f"RELEASE_MINOR={release}")
PY
}

write_fixture_objects() {
  cat >"${OBJECT_FILE}" <<'EOF'
RustyOnions paid storage live smoke object.
EOF

  cat >"${BAD_OBJECT_FILE}" <<'EOF'
RustyOnions paid storage replay attack object.
EOF
}

build_binaries() {
  echo "building svc-wallet, svc-storage, ron-accounting ingest, and local BLAKE3 helper"

  cargo build \
    -p svc-wallet \
    -p svc-storage \
    -p ron-accounting \
    --bins >/dev/null
}

start_accounting() {
  echo "starting ron-accounting ingest on ${ACCOUNTING_ADDR}"

  RON_ACCOUNTING_ADDR="${ACCOUNTING_ADDR}" \
  RON_ACC_BEARER="${BEARER}" \
    "${ROOT}/target/debug/ron_accounting_ingest" \
      >"${ACCOUNTING_LOG}" 2>&1 &

  ACCOUNTING_PID="$!"
  wait_for_health "${ACCOUNTING_BASE_URL}/healthz" "ron-accounting"
}

start_wallet() {
  echo "starting svc-wallet on ${WALLET_ADDR}"

  SVC_WALLET_ADDR="${WALLET_ADDR}" \
  RUST_LOG="${RUST_LOG:-info}" \
    "${ROOT}/target/debug/svc-wallet" \
      >"${WALLET_LOG}" 2>&1 &

  WALLET_PID="$!"
  wait_for_health "${WALLET_BASE_URL}/healthz" "svc-wallet"
}

start_storage() {
  local storage_mode="wallet-receipt + wallet-capture + accounting-http + ${PRICE_MODE}"

  echo "starting svc-storage on ${STORAGE_ADDR} in ${storage_mode} mode"

  local -a economics_env=()
  if [[ "${WEB3_PAID_STORAGE_USE_ECONOMICS}" == "1" ]]; then
    economics_env=(
      "RON_STORAGE_ROC_ECONOMICS_PATH=${RON_STORAGE_ROC_ECONOMICS_PATH}"
      "RON_STORAGE_ROC_ECONOMICS_ACTION=${RON_STORAGE_ROC_ECONOMICS_ACTION}"
    )
  fi

  env \
    ADDR="${STORAGE_ADDR}" \
    RUST_LOG="${RUST_LOG:-info}" \
    RON_STORAGE_PAID_WRITE_VERIFIER_MODE="wallet-receipt" \
    RON_STORAGE_WALLET_BASE_URL="${WALLET_BASE_URL}" \
    RON_STORAGE_WALLET_BEARER="${BEARER}" \
    RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS="2000" \
    RON_STORAGE_PAID_SETTLEMENT_MODE="wallet-capture" \
    RON_STORAGE_PAID_SETTLEMENT_PAYEE="${PAYEE}" \
    RON_STORAGE_ACCOUNTING_EXPORT_MODE="http" \
    RON_STORAGE_ACCOUNTING_BASE_URL="${ACCOUNTING_BASE_URL}" \
    RON_STORAGE_ACCOUNTING_BEARER="${BEARER}" \
    RON_STORAGE_ACCOUNTING_TIMEOUT_MS="2000" \
    "${economics_env[@]}" \
    "${ROOT}/target/debug/svc-storage" \
      >"${STORAGE_LOG}" 2>&1 &

  STORAGE_PID="$!"
  wait_for_health "${STORAGE_BASE_URL}/healthz" "svc-storage"
}

issue_to_payer() {
  local out="${WORK_DIR}/issue.json"
  local payload
  local status

  local issue_amount="${WEB3_PAID_STORAGE_ISSUE_MINOR:-100}"
  if (( issue_amount < HOLD_MINOR )); then
    issue_amount=$((HOLD_MINOR + 25))
  fi

  payload="$(jq -n \
    --arg to "${PAYER}" \
    --arg asset "${ASSET}" \
    --arg amount "${issue_amount}" \
    '{
      to: $to,
      asset: $asset,
      amount_minor: $amount,
      memo: "web3 paid-storage live smoke issue"
    }')"

  echo "issuing ROC to payer"

  status="$(http_status POST "${WALLET_BASE_URL}/v1/issue" "${out}" \
    -H "authorization: Bearer ${BEARER}" \
    -H "accept: application/json" \
    -H "content-type: application/json" \
    -H "idempotency-key: idem_live_issue_paid_storage" \
    --data "${payload}")"

  assert_2xx "${status}" "wallet issue" "${out}"
  cat "${out}" | jq .
}

create_hold() {
  local out="${WORK_DIR}/hold.json"
  local payload
  local status

  CONTEXT_IDEM="$("${ROOT}/target/debug/roc_b3_tool" context-idem \
    "${OBJECT_CID}" \
    "${PAYER}" \
    "${ESCROW}" \
    "${ASSET}" \
    "${HOLD_MINOR}")"

  payload="$(jq -n \
    --arg from "${PAYER}" \
    --arg to "${ESCROW}" \
    --arg asset "${ASSET}" \
    --arg amount "${HOLD_MINOR}" \
    '{
      from: $from,
      to: $to,
      asset: $asset,
      amount_minor: $amount,
      nonce: 1,
      memo: "web3 paid-storage live smoke hold"
    }')"

  echo "creating wallet hold bound to paid-storage context"

  status="$(http_status POST "${WALLET_BASE_URL}/v1/hold" "${out}" \
    -H "authorization: Bearer ${BEARER}" \
    -H "accept: application/json" \
    -H "content-type: application/json" \
    -H "idempotency-key: ${CONTEXT_IDEM}" \
    --data "${payload}")"

  assert_2xx "${status}" "wallet hold" "${out}"
  cat "${out}" | jq .

  HOLD_TXID="$(jq -r '.txid' "${out}")"
  HOLD_RECEIPT_HASH="$(jq -r '.receipt_hash' "${out}")"
}

write_paid_object() {
  local out="${WORK_DIR}/paid-write.json"
  local status

  echo "writing paid object through svc-storage; storage should capture/release and export usage"

  status="$(curl -sS \
    -X POST \
    -o "${out}" \
    -w "%{http_code}" \
    -H "authorization: Bearer ${BEARER}" \
    -H "content-type: application/octet-stream" \
    -H "accept: application/json" \
    -H "idempotency-key: ${CONTEXT_IDEM}" \
    -H "x-ron-paid-op: hold" \
    -H "x-ron-paid-asset: ${ASSET}" \
    -H "x-ron-paid-estimate-minor: ${HOLD_MINOR}" \
    -H "x-ron-wallet-txid: ${HOLD_TXID}" \
    -H "x-ron-wallet-receipt-hash: ${HOLD_RECEIPT_HASH}" \
    -H "x-ron-wallet-from: ${PAYER}" \
    -H "x-ron-wallet-to: ${ESCROW}" \
    -H "x-ron-tenant: ${TENANT}" \
    -H "x-ron-accounting-subject: ${ACCOUNTING_SUBJECT}" \
    -H "x-ron-region: ${REGION}" \
    -H "x-ron-pin-seconds: ${PIN_SECONDS}" \
    --data-binary @"${OBJECT_FILE}" \
    "${STORAGE_BASE_URL}/paid/o")"

  assert_2xx "${status}" "paid storage write" "${out}"
  cat "${out}" | jq .

  local returned_cid
  returned_cid="$(jq -r '.cid' "${out}")"
  if [[ "${returned_cid}" != "${OBJECT_CID}" ]]; then
    echo "storage returned unexpected cid: expected ${OBJECT_CID}, got ${returned_cid}" >&2
    exit 1
  fi

  local paid
  paid="$(jq -r '.paid' "${out}")"
  if [[ "${paid}" != "true" ]]; then
    echo "storage response did not mark paid=true" >&2
    exit 1
  fi

  local capture
  capture="$(jq -r '.settlement.capture_amount_minor // empty' "${out}")"
  if [[ "${capture}" != "${CAPTURE_MINOR}" ]]; then
    echo "unexpected capture amount: expected ${CAPTURE_MINOR}, got ${capture}" >&2
    exit 1
  fi

  local release
  release="$(jq -r '.settlement.release_amount_minor // "0"' "${out}")"
  if [[ "${release}" != "${RELEASE_MINOR}" ]]; then
    echo "unexpected release amount: expected ${RELEASE_MINOR}, got ${release}" >&2
    exit 1
  fi

  local exported
  exported="$(jq -r '.accounting_export.status // empty' "${out}")"
  if [[ "${exported}" != "exported" ]]; then
    echo "expected accounting_export.status=exported, got ${exported}" >&2
    exit 1
  fi
}

read_object_back() {
  local status

  echo "reading object back"

  status="$(curl -sS \
    -X GET \
    -o "${OBJECT_GET_FILE}" \
    -w "%{http_code}" \
    -H "authorization: Bearer ${BEARER}" \
    "${STORAGE_BASE_URL}/o/${OBJECT_CID}")"

  case "${status}" in
    2*) ;;
    *)
      echo "object readback failed with HTTP ${status}" >&2
      exit 1
      ;;
  esac

  if ! cmp -s "${OBJECT_FILE}" "${OBJECT_GET_FILE}"; then
    echo "object readback bytes did not match original" >&2
    exit 1
  fi
}

check_final_balances() {
  echo "checking final wallet balances"

  local payer_balance escrow_balance payee_balance
  payer_balance="$(wallet_balance "${PAYER}")"
  escrow_balance="$(wallet_balance "${ESCROW}")"
  payee_balance="$(wallet_balance "${PAYEE}")"

  local issue_amount="${WEB3_PAID_STORAGE_ISSUE_MINOR:-100}"
  if (( issue_amount < HOLD_MINOR )); then
    issue_amount=$((HOLD_MINOR + 25))
  fi

  local expected_payer=$((issue_amount - CAPTURE_MINOR))
  local expected_escrow=0
  local expected_payee="${CAPTURE_MINOR}"

  echo "payer balance:  ${payer_balance}"
  echo "escrow balance: ${escrow_balance}"
  echo "payee balance:  ${payee_balance}"

  if (( payer_balance != expected_payer )); then
    echo "unexpected payer balance: expected ${expected_payer}, got ${payer_balance}" >&2
    exit 1
  fi

  if (( escrow_balance != expected_escrow )); then
    echo "unexpected escrow balance: expected ${expected_escrow}, got ${escrow_balance}" >&2
    exit 1
  fi

  if (( payee_balance != expected_payee )); then
    echo "unexpected payee balance: expected ${expected_payee}, got ${payee_balance}" >&2
    exit 1
  fi
}

check_accounting_snapshot() {
  local out="${WORK_DIR}/accounting-snapshot.json"
  local status

  echo "checking real ron-accounting snapshot"

  status="$(http_status GET "${ACCOUNTING_BASE_URL}/v1/snapshot" "${out}" \
    -H "authorization: Bearer ${BEARER}" \
    -H "accept: application/json")"

  assert_2xx "${status}" "ron-accounting snapshot" "${out}"
  cat "${out}" | jq .

  local row_count
  row_count="$(jq -r '.row_count // 0' "${out}")"
  if (( row_count < 3 )); then
    echo "expected at least 3 accounting rows, got ${row_count}" >&2
    exit 1
  fi

  jq -e --argjson bytes "${OBJECT_BYTES}" '
    .rows[]
    | select(.key.dimension == "bytes")
    | select(.key.labels.method == "PUT")
    | select(.key.labels.route == "/paid/o")
    | select(.value == $bytes)
  ' "${out}" >/dev/null

  jq -e '
    .rows[]
    | select(.key.dimension == "requests")
    | select(.key.labels.method == "REQ_OK")
    | select(.key.labels.route == "/paid/o")
    | select(.value == 1)
  ' "${out}" >/dev/null

  jq -e --argjson pin_seconds "${PIN_SECONDS}" '
    .rows[]
    | select(.key.dimension == "requests")
    | select(.key.labels.method == "PIN_SECONDS")
    | select(.key.labels.route == "/paid/o")
    | select(.value == $pin_seconds)
  ' "${out}" >/dev/null
}

prove_hold_replay_rejects_for_different_bytes() {
  local bad_cid
  local status

  bad_cid="$("${ROOT}/target/debug/roc_b3_tool" cid "${BAD_OBJECT_FILE}")"

  echo "proving same hold cannot be replayed for different bytes"

  status="$(curl -sS \
    -X POST \
    -o "${BAD_RESPONSE_FILE}" \
    -w "%{http_code}" \
    -H "authorization: Bearer ${BEARER}" \
    -H "content-type: application/octet-stream" \
    -H "accept: application/json" \
    -H "idempotency-key: ${CONTEXT_IDEM}-bad-replay" \
    -H "x-ron-paid-op: hold" \
    -H "x-ron-paid-asset: ${ASSET}" \
    -H "x-ron-paid-estimate-minor: ${HOLD_MINOR}" \
    -H "x-ron-wallet-txid: ${HOLD_TXID}" \
    -H "x-ron-wallet-receipt-hash: ${HOLD_RECEIPT_HASH}" \
    -H "x-ron-wallet-from: ${PAYER}" \
    -H "x-ron-wallet-to: ${ESCROW}" \
    -H "x-ron-tenant: ${TENANT}" \
    -H "x-ron-accounting-subject: ${ACCOUNTING_SUBJECT}" \
    -H "x-ron-region: ${REGION}" \
    -H "x-ron-pin-seconds: ${PIN_SECONDS}" \
    --data-binary @"${BAD_OBJECT_FILE}" \
    "${STORAGE_BASE_URL}/paid/o")"

  cat "${BAD_RESPONSE_FILE}" | jq . || cat "${BAD_RESPONSE_FILE}"

  case "${status}" in
    2*)
      echo "replay with different bytes unexpectedly succeeded; bad cid was ${bad_cid}" >&2
      exit 1
      ;;
  esac

  if ! grep -q "context idem mismatch" "${BAD_RESPONSE_FILE}"; then
    echo "expected paid storage context idem mismatch in replay rejection" >&2
    exit 1
  fi
}

write_fixture_objects
build_binaries

OBJECT_CID="$("${ROOT}/target/debug/roc_b3_tool" cid "${OBJECT_FILE}")"
OBJECT_BYTES="$(wc -c < "${OBJECT_FILE}" | tr -d '[:space:]')"

eval "$(resolve_price_plan "${OBJECT_BYTES}")"

echo "object CID: ${OBJECT_CID}"
echo "object bytes: ${OBJECT_BYTES}"
echo "pricing mode: ${PRICE_MODE}"
echo "context idem: will be computed after pricing"
echo "issue amount: ${WEB3_PAID_STORAGE_ISSUE_MINOR:-100}"
echo "hold amount: ${HOLD_MINOR}"
echo "expected capture: ${CAPTURE_MINOR}"
echo "expected release: ${RELEASE_MINOR}"

start_accounting
start_wallet
start_storage

issue_to_payer
create_hold
write_paid_object
read_object_back
check_final_balances
check_accounting_snapshot
prove_hold_replay_rejects_for_different_bytes

echo "WEB3 paid-storage live smoke with settlement + real accounting export green"
echo "pricing mode:   ${PRICE_MODE}"
echo "capture amount: ${CAPTURE_MINOR}"
echo "release amount: ${RELEASE_MINOR}"
echo "accounting log: ${ACCOUNTING_LOG}"
echo "wallet log:     ${WALLET_LOG}"
echo "storage log:    ${STORAGE_LOG}"