#!/usr/bin/env bash
# RO:WHAT — Repeatable live smoke for paid CrabLink named-site visits.
# RO:WHY — NEXT_LEVEL/QuickChain preflight; proves Visitor B pays Creator A through svc-wallet/ron-ledger truth.
# RO:INTERACTS — svc-gateway, omnigate, svc-storage, svc-index, svc-wallet, crab://site routes.
# RO:INVARIANTS — wallet mutation only through svc-wallet; no fake balances; asserts A +amount and B -amount.
# RO:METRICS — prints txid, receipt_hash, ledger_root, before/after balances for operator proof.
# RO:CONFIG — GATEWAY_BASE_URL, WALLET_BASE_URL, STORAGE_BASE_URL, SITE_NAME, payer/recipient env overrides.
# RO:SECURITY — uses dev bearer only for local smoke; does not echo non-dev secrets.
# RO:TEST — run after scripts/web3_crablink_dev_stack.sh or equivalent local stack boot.

set -euo pipefail
IFS=$'\n\t'

GATEWAY_BASE_URL="${GATEWAY_BASE_URL:-http://127.0.0.1:8090}"
WALLET_BASE_URL="${WALLET_BASE_URL:-http://127.0.0.1:8088}"
STORAGE_BASE_URL="${STORAGE_BASE_URL:-http://127.0.0.1:5303}"

AUTH_BEARER="${AUTH_BEARER:-dev}"
ASSET="${ASSET:-roc}"
SITE_NAME="${SITE_NAME:-ron7}"
SITE_TITLE="${SITE_TITLE:-Reference Graph Smoke Site}"

VISITOR_PASSPORT="${VISITOR_PASSPORT:-passport:main:visitor-b}"
PAYER_ACCOUNT="${PAYER_ACCOUNT:-acct_visitor_b}"

CREATOR_PASSPORT="${CREATOR_PASSPORT:-passport:main:dev}"
RECIPIENT_ACCOUNT="${RECIPIENT_ACCOUNT:-acct_dev}"

SITE_VISIT_AMOUNT_MINOR="${SITE_VISIT_AMOUNT_MINOR:-10}"
VISITOR_STARTER_GRANT_MINOR="${VISITOR_STARTER_GRANT_MINOR:-1000}"

RUN_SITE_REPAIR="${RUN_SITE_REPAIR:-1}"
RUN_VISITOR_FUNDING="${RUN_VISITOR_FUNDING:-1}"

RUN_ID="${RUN_ID:-$(date +%Y%m%d%H%M%S)}"
TMP_DIR="${TMPDIR:-/tmp}/rustyonions-site-visit-smoke-${RUN_ID}"
mkdir -p "${TMP_DIR}"

need_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "missing required tool: ${tool}" >&2
    exit 127
  fi
}

need_tool curl
need_tool jq
need_tool mktemp

log() {
  printf '[site-visit-smoke] %s\n' "$*"
}

fail() {
  printf '[site-visit-smoke] ERROR: %s\n' "$*" >&2
  exit 1
}

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
  local body_file="$3"

  case "${status}" in
    2*) return 0 ;;
    *)
      echo "---- ${label} failed: HTTP ${status} ----" >&2
      cat "${body_file}" >&2 || true
      echo >&2
      fail "${label} expected 2xx"
      ;;
  esac
}

wallet_balance_minor() {
  local account="$1"
  local out="${TMP_DIR}/wallet-balance-${account}.json"
  local status

  status="$(http_status GET "${WALLET_BASE_URL%/}/v1/balance?account=${account}&asset=${ASSET}" "${out}" \
    -H "authorization: Bearer ${AUTH_BEARER}" \
    -H "accept: application/json")" || {
      printf '0\n'
      return 0
    }

  case "${status}" in
    2*)
      jq -r '.amount_minor // .available_minor_units // "0"' "${out}"
      ;;
    *)
      printf '0\n'
      ;;
  esac
}

issue_roc() {
  local account="$1"
  local amount="$2"
  local out="${TMP_DIR}/wallet-issue-${account}.json"
  local idem="site-visit-smoke-issue-${RUN_ID}-${account}"
  local payload
  local status

  payload="$(jq -n \
    --arg to "${account}" \
    --arg asset "${ASSET}" \
    --arg amount "${amount}" \
    --arg idem "${idem}" \
    '{
      to: $to,
      asset: $asset,
      amount_minor: $amount,
      idempotency_key: $idem,
      memo: "web3 paid site_visit smoke starter grant"
    }')"

  status="$(http_status POST "${WALLET_BASE_URL%/}/v1/issue" "${out}" \
    -H "authorization: Bearer ${AUTH_BEARER}" \
    -H "accept: application/json" \
    -H "content-type: application/json" \
    -H "idempotency-key: ${idem}" \
    --data "${payload}")"

  assert_2xx "${status}" "svc-wallet issue ${account}" "${out}"

  log "funded ${account} with ${amount} ${ASSET} minor units"
}

wait_for_2xx() {
  local url="$1"
  local label="$2"
  local out="${TMP_DIR}/wait-${label}.json"
  local status

  for _ in $(seq 1 80); do
    status="$(http_status GET "${url}" "${out}" -H "accept: application/json" || true)"
    case "${status}" in
      2*) return 0 ;;
    esac
    sleep 0.25
  done

  cat "${out}" >&2 || true
  fail "${label} did not become ready at ${url}"
}

store_root_document() {
  local html_file="${TMP_DIR}/site-root.html"
  local out="${TMP_DIR}/site-root-put.json"
  local status

  cat >"${html_file}" <<HTML
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>${SITE_TITLE}</title>
  </head>
  <body>
    <main>
      <h1>${SITE_TITLE}</h1>
      <p>Paid site_visit smoke root for crab://${SITE_NAME}.</p>
      <p>Visitor account: ${PAYER_ACCOUNT}</p>
      <p>Creator account: ${RECIPIENT_ACCOUNT}</p>
    </main>
  </body>
</html>
HTML

  status="$(curl -sS \
    -X POST \
    -o "${out}" \
    -w "%{http_code}" \
    -H "content-type: text/html; charset=utf-8" \
    --data-binary @"${html_file}" \
    "${STORAGE_BASE_URL%/}/o")"

  assert_2xx "${status}" "store root document in svc-storage" "${out}"

  jq -r '.cid // empty' "${out}"
}

resolve_site_status() {
  local out="$1"
  http_status GET "${GATEWAY_BASE_URL%/}/sites/${SITE_NAME}" "${out}" \
    -H "authorization: Bearer ${AUTH_BEARER}" \
    -H "accept: application/json" \
    -H "x-ron-passport: ${VISITOR_PASSPORT}" \
    -H "x-ron-wallet-account: ${PAYER_ACCOUNT}"
}

site_has_expected_payout() {
  local file="$1"
  local recipient
  local action

  recipient="$(jq -r '.payout.recipient_account // .manifest.payout.recipient_account // empty' "${file}")"
  action="$(jq -r '.payout.default_action // .manifest.payout.default_action // empty' "${file}")"

  [[ "${recipient}" == "${RECIPIENT_ACCOUNT}" && "${action}" == "site_visit" ]]
}

create_or_repair_site() {
  local resolve_out="${TMP_DIR}/site-resolve-before.json"
  local status

  status="$(resolve_site_status "${resolve_out}" || true)"
  if [[ "${status}" =~ ^2 ]] && site_has_expected_payout "${resolve_out}"; then
    log "site crab://${SITE_NAME} already resolves with payout recipient ${RECIPIENT_ACCOUNT}"
    return 0
  fi

  if [[ "${RUN_SITE_REPAIR}" != "1" ]]; then
    cat "${resolve_out}" >&2 || true
    fail "site crab://${SITE_NAME} is missing or payout is wrong; set RUN_SITE_REPAIR=1 to recreate"
  fi

  log "creating/repairing crab://${SITE_NAME}"

  local root_cid
  root_cid="$(store_root_document)"
  if [[ -z "${root_cid}" || "${root_cid}" != b3:* ]]; then
    fail "storage did not return canonical root CID"
  fi

  local payload="${TMP_DIR}/site-create-body.json"
  local out="${TMP_DIR}/site-create.json"

  jq -n \
    --arg site_name "${SITE_NAME}" \
    --arg root_document_cid "${root_cid}" \
    --arg owner_passport_subject "${CREATOR_PASSPORT}" \
    --arg owner_wallet_account "${RECIPIENT_ACCOUNT}" \
    --arg title "${SITE_TITLE}" \
    --arg description "Paid site_visit smoke site generated by scripts/web3_paid_site_visit_smoke.sh" \
    '{
      site_name: $site_name,
      root_document_cid: $root_document_cid,
      owner_passport_subject: $owner_passport_subject,
      owner_wallet_account: $owner_wallet_account,
      title: $title,
      description: $description,
      route_map: {},
      asset_map: {},
      receipt_refs: []
    }' >"${payload}"

  status="$(curl -sS \
    -X POST \
    -o "${out}" \
    -w "%{http_code}" \
    -H "authorization: Bearer ${AUTH_BEARER}" \
    -H "accept: application/json" \
    -H "content-type: application/json" \
    -H "x-ron-passport: ${CREATOR_PASSPORT}" \
    -H "x-ron-wallet-account: ${RECIPIENT_ACCOUNT}" \
    -H "x-ron-wallet-hold-txid: dev-site-visit-smoke-root" \
    -H "idempotency-key: site-create-${RUN_ID}-${SITE_NAME}" \
    --data @"${payload}" \
    "${GATEWAY_BASE_URL%/}/sites")"

  assert_2xx "${status}" "gateway site create" "${out}"

  local after="${TMP_DIR}/site-resolve-after.json"
  status="$(resolve_site_status "${after}" || true)"
  assert_2xx "${status}" "gateway site resolve after repair" "${after}"

  if ! site_has_expected_payout "${after}"; then
    cat "${after}" >&2
    fail "repaired site did not resolve with expected site_visit payout"
  fi

  log "created crab://${SITE_NAME} root=${root_cid}"
}

quote_site_visit() {
  local out="$1"
  local payload="${TMP_DIR}/site-visit-quote-body.json"
  local status

  jq -n \
    --arg site_name "${SITE_NAME}" \
    --arg crab_url "crab://${SITE_NAME}" \
    --arg payer_account "${PAYER_ACCOUNT}" \
    --arg visitor_wallet_account "${PAYER_ACCOUNT}" \
    --arg visitor_passport_subject "${VISITOR_PASSPORT}" \
    --arg recipient_account "${RECIPIENT_ACCOUNT}" \
    --arg max_amount_minor "${SITE_VISIT_AMOUNT_MINOR}" \
    --arg idem "site-visit-smoke-quote-${RUN_ID}-${SITE_NAME}" \
    '{
      site_name: $site_name,
      crab_url: $crab_url,
      action: "site_visit",
      quantity: 1,
      payer_account: $payer_account,
      visitor_wallet_account: $visitor_wallet_account,
      visitor_passport_subject: $visitor_passport_subject,
      recipient_account: $recipient_account,
      max_amount_minor: $max_amount_minor,
      client_idempotency_key: $idem
    }' >"${payload}"

  status="$(curl -sS \
    -X POST \
    -o "${out}" \
    -w "%{http_code}" \
    -H "authorization: Bearer ${AUTH_BEARER}" \
    -H "accept: application/json" \
    -H "content-type: application/json" \
    -H "x-ron-passport: ${VISITOR_PASSPORT}" \
    -H "x-ron-wallet-account: ${PAYER_ACCOUNT}" \
    -H "idempotency-key: site-visit-smoke-quote-${RUN_ID}-${SITE_NAME}" \
    --data @"${payload}" \
    "${GATEWAY_BASE_URL%/}/sites/${SITE_NAME}/visit/quote")"

  assert_2xx "${status}" "site_visit quote" "${out}"

  local schema amount payer recipient
  schema="$(jq -r '.schema // empty' "${out}")"
  amount="$(jq -r '.amount_minor // empty' "${out}")"
  payer="$(jq -r '.payer_account // empty' "${out}")"
  recipient="$(jq -r '.recipient_account // empty' "${out}")"

  [[ "${schema}" == "omnigate.site-visit-quote.v1" ]] || fail "unexpected quote schema: ${schema}"
  [[ "${amount}" == "${SITE_VISIT_AMOUNT_MINOR}" ]] || fail "unexpected quote amount: ${amount}"
  [[ "${payer}" == "${PAYER_ACCOUNT}" ]] || fail "unexpected quote payer: ${payer}"
  [[ "${recipient}" == "${RECIPIENT_ACCOUNT}" ]] || fail "unexpected quote recipient: ${recipient}"
}

pay_site_visit() {
  local quote_file="$1"
  local out="$2"
  local quote_id quote_hash payload status

  quote_id="$(jq -r '.quote_id // empty' "${quote_file}")"
  quote_hash="$(jq -r '.quote_hash // empty' "${quote_file}")"
  payload="${TMP_DIR}/site-visit-pay-body.json"

  jq -n \
    --arg site_name "${SITE_NAME}" \
    --arg crab_url "crab://${SITE_NAME}" \
    --arg payer_account "${PAYER_ACCOUNT}" \
    --arg visitor_wallet_account "${PAYER_ACCOUNT}" \
    --arg visitor_passport_subject "${VISITOR_PASSPORT}" \
    --arg recipient_account "${RECIPIENT_ACCOUNT}" \
    --arg amount_minor "${SITE_VISIT_AMOUNT_MINOR}" \
    --arg asset "${ASSET}" \
    --arg quote_id "${quote_id}" \
    --arg quote_hash "${quote_hash}" \
    --arg idem "site-visit-smoke-pay-${RUN_ID}-${SITE_NAME}" \
    '{
      site_name: $site_name,
      crab_url: $crab_url,
      action: "site_visit",
      quantity: 1,
      payer_account: $payer_account,
      visitor_wallet_account: $visitor_wallet_account,
      visitor_passport_subject: $visitor_passport_subject,
      recipient_account: $recipient_account,
      amount_minor: $amount_minor,
      asset: $asset,
      quote_id: $quote_id,
      quote_hash: $quote_hash,
      client_idempotency_key: $idem
    }' >"${payload}"

  status="$(curl -sS \
    -X POST \
    -o "${out}" \
    -w "%{http_code}" \
    -H "authorization: Bearer ${AUTH_BEARER}" \
    -H "accept: application/json" \
    -H "content-type: application/json" \
    -H "x-ron-passport: ${VISITOR_PASSPORT}" \
    -H "x-ron-wallet-account: ${PAYER_ACCOUNT}" \
    -H "idempotency-key: site-visit-smoke-pay-${RUN_ID}-${SITE_NAME}" \
    --data @"${payload}" \
    "${GATEWAY_BASE_URL%/}/sites/${SITE_NAME}/visit/pay")"

  assert_2xx "${status}" "site_visit pay" "${out}"

  local schema txid receipt_hash ledger_root wallet_from wallet_to wallet_amount
  schema="$(jq -r '.schema // empty' "${out}")"
  txid="$(jq -r '.txid // .wallet_receipt.txid // empty' "${out}")"
  receipt_hash="$(jq -r '.receipt_hash // .wallet_receipt.receipt_hash // empty' "${out}")"
  ledger_root="$(jq -r '.ledger_root // .wallet_receipt.ledger_root // empty' "${out}")"
  wallet_from="$(jq -r '.wallet_receipt.from // empty' "${out}")"
  wallet_to="$(jq -r '.wallet_receipt.to // empty' "${out}")"
  wallet_amount="$(jq -r '.wallet_receipt.amount_minor // .amount_minor // empty' "${out}")"

  [[ "${schema}" == "omnigate.site-visit-payment.v1" ]] || fail "unexpected pay schema: ${schema}"
  [[ -n "${txid}" ]] || fail "payment response missing txid"
  [[ -n "${receipt_hash}" ]] || fail "payment response missing receipt_hash"
  [[ -n "${ledger_root}" ]] || fail "payment response missing ledger_root"
  [[ "${wallet_from}" == "${PAYER_ACCOUNT}" ]] || fail "wallet receipt from mismatch: ${wallet_from}"
  [[ "${wallet_to}" == "${RECIPIENT_ACCOUNT}" ]] || fail "wallet receipt to mismatch: ${wallet_to}"
  [[ "${wallet_amount}" == "${SITE_VISIT_AMOUNT_MINOR}" ]] || fail "wallet receipt amount mismatch: ${wallet_amount}"
}

log "checking stack readiness"
wait_for_2xx "${GATEWAY_BASE_URL%/}/healthz" "gateway-healthz"
wait_for_2xx "${GATEWAY_BASE_URL%/}/readyz" "gateway-readyz"
wait_for_2xx "${WALLET_BASE_URL%/}/healthz" "wallet-healthz"
wait_for_2xx "${STORAGE_BASE_URL%/}/healthz" "storage-healthz"

create_or_repair_site

before_b="$(wallet_balance_minor "${PAYER_ACCOUNT}")"
before_a="$(wallet_balance_minor "${RECIPIENT_ACCOUNT}")"

log "before balances: ${PAYER_ACCOUNT}=${before_b}, ${RECIPIENT_ACCOUNT}=${before_a}"

if [[ "${RUN_VISITOR_FUNDING}" == "1" ]] && (( before_b < SITE_VISIT_AMOUNT_MINOR )); then
  issue_roc "${PAYER_ACCOUNT}" "${VISITOR_STARTER_GRANT_MINOR}"
  before_b="$(wallet_balance_minor "${PAYER_ACCOUNT}")"
  log "after funding: ${PAYER_ACCOUNT}=${before_b}"
fi

if (( before_b < SITE_VISIT_AMOUNT_MINOR )); then
  fail "${PAYER_ACCOUNT} has ${before_b}; needs at least ${SITE_VISIT_AMOUNT_MINOR}"
fi

quote_file="${TMP_DIR}/site-visit-quote.json"
pay_file="${TMP_DIR}/site-visit-pay.json"

quote_site_visit "${quote_file}"
pay_site_visit "${quote_file}" "${pay_file}"

after_b="$(wallet_balance_minor "${PAYER_ACCOUNT}")"
after_a="$(wallet_balance_minor "${RECIPIENT_ACCOUNT}")"

expected_b=$((before_b - SITE_VISIT_AMOUNT_MINOR))
expected_a=$((before_a + SITE_VISIT_AMOUNT_MINOR))

log "after balances: ${PAYER_ACCOUNT}=${after_b}, ${RECIPIENT_ACCOUNT}=${after_a}"

if (( after_b != expected_b )); then
  cat "${pay_file}" >&2
  fail "payer balance assertion failed: expected ${expected_b}, got ${after_b}"
fi

if (( after_a != expected_a )); then
  cat "${pay_file}" >&2
  fail "recipient balance assertion failed: expected ${expected_a}, got ${after_a}"
fi

txid="$(jq -r '.txid // .wallet_receipt.txid // empty' "${pay_file}")"
receipt_hash="$(jq -r '.receipt_hash // .wallet_receipt.receipt_hash // empty' "${pay_file}")"
ledger_root="$(jq -r '.ledger_root // .wallet_receipt.ledger_root // empty' "${pay_file}")"
nonce="$(jq -r '.nonce // .wallet_receipt.nonce // empty' "${pay_file}")"

cat <<SUMMARY
[site-visit-smoke] PASS
  crab_url:        crab://${SITE_NAME}
  action:          site_visit
  asset:           ${ASSET}
  amount_minor:    ${SITE_VISIT_AMOUNT_MINOR}
  payer:           ${PAYER_ACCOUNT}
  recipient:       ${RECIPIENT_ACCOUNT}
  before_payer:    ${before_b}
  after_payer:     ${after_b}
  before_creator:  ${before_a}
  after_creator:   ${after_a}
  nonce:           ${nonce}
  txid:            ${txid}
  receipt_hash:    ${receipt_hash}
  ledger_root:     ${ledger_root}
  artifacts:       ${TMP_DIR}
SUMMARY