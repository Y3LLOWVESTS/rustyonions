#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Live non-mutating adversarial content_view probe.
# RO:WHY — Proves malformed/tampered/unfunded paid-access attempts fail closed without balance mutation.
# RO:INTERACTS — svc-gateway /content/view/quote, /content/view/pay, svc-wallet balances.
# RO:INVARIANTS — fake paid/receipt/finality headers cannot authorize; rejected attempts do not mutate balances.
# RO:TEST — scripts/dev-internal-roc-beta-live-probes.sh.

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8090}"
CRAB_URL="${CRAB_URL:?set CRAB_URL=crab://<64hex>.image}"
ASSET_HEX="${ASSET_HEX:-}"
CREATOR_ACCOUNT="${CREATOR_ACCOUNT:?set CREATOR_ACCOUNT}"
VISITOR_ACCOUNT="${VISITOR_ACCOUNT:?set VISITOR_ACCOUNT}"
UNFUNDED_ACCOUNT="${UNFUNDED_ACCOUNT:-acct_probe_unfunded_negative}"

if [[ -z "$ASSET_HEX" ]]; then
  ASSET_HEX="$(printf '%s' "$CRAB_URL" | sed -nE 's#^crab://([0-9a-f]{64})\.[A-Za-z0-9_-]+$#\1#p')"
fi

TMP_DIR="$(mktemp -d /tmp/crablink-negative-content-view.XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

log() { printf '\n== %s ==\n' "$*"; }
fail() { printf '\nFAIL: %s\n' "$*" >&2; exit 1; }

command -v jq >/dev/null 2>&1 || fail "jq is required"

json_post() {
  local path="$1"
  local body="$2"
  local out="$3"

  curl --connect-timeout 2 --max-time 12 -sS \
    -o "$out" \
    -w '%{http_code}' \
    -X POST "$GATEWAY_URL$path" \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Bearer dev' \
    -H 'X-RON-Receipt: fake-receipt-should-not-authorize' \
    -H 'X-RON-Paid: true' \
    -H 'X-RON-Ledger-Root: fake-root-should-not-authorize' \
    -d "$body" || true
}

get_balance() {
  local account="$1"
  curl --connect-timeout 2 --max-time 8 -fsS \
    "$GATEWAY_URL/wallet/$account/balance" \
    | jq -r '.available_minor_units'
}

assert_non_2xx() {
  local code="$1"
  local label="$2"
  case "$code" in
    2*) fail "$label unexpectedly returned HTTP $code" ;;
    *) printf '%s rejected as expected with HTTP %s\n' "$label" "$code" ;;
  esac
}

log "baseline balances"
VISITOR_BEFORE="$(get_balance "$VISITOR_ACCOUNT")"
CREATOR_BEFORE="$(get_balance "$CREATOR_ACCOUNT")"
UNFUNDED_BEFORE="$(curl --connect-timeout 2 --max-time 8 -sS "$GATEWAY_URL/wallet/$UNFUNDED_ACCOUNT/balance" | jq -r '.available_minor_units // "0"' 2>/dev/null || printf '0')"

printf 'visitor before:  %s\n' "$VISITOR_BEFORE"
printf 'creator before:  %s\n' "$CREATOR_BEFORE"
printf 'unfunded before: %s\n' "$UNFUNDED_BEFORE"

log "positive quote sanity: quote must be read-only"
QUOTE_OK="$TMP_DIR/quote-ok.json"
QUOTE_CODE="$(json_post "/content/view/quote" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_wallet_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:probe-unfunded\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"max_amount_minor\":\"5\",
  \"client_idempotency_key\":\"negative-probe-quote-ok-$(date +%s)\"
}" "$QUOTE_OK")"

if [[ "$QUOTE_CODE" != 2* ]]; then
  cat "$QUOTE_OK" || true
  fail "positive quote sanity failed with HTTP $QUOTE_CODE"
fi

jq . "$QUOTE_OK"

QUOTE_ID="$(jq -r '.quote_id // .quote.quote_id' "$QUOTE_OK")"
QUOTE_HASH="$(jq -r '.quote_hash // .quote.quote_hash' "$QUOTE_OK")"

log "negative quote: old crab://b3 slash form must reject"
BAD_OLD_FORM="$TMP_DIR/bad-old-form.json"
BAD_OLD_CODE="$(json_post "/content/view/quote" "{
  \"asset_crab_url\":\"crab://b3/$ASSET_HEX.image\",
  \"payer_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_wallet_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:probe-unfunded\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"max_amount_minor\":\"5\",
  \"client_idempotency_key\":\"negative-probe-old-form-$(date +%s)\"
}" "$BAD_OLD_FORM")"
assert_non_2xx "$BAD_OLD_CODE" "old slash-form crab URL"

log "negative quote: uppercase hash must reject"
UPPER_HEX="$(printf '%s' "$ASSET_HEX" | tr '[:lower:]' '[:upper:]')"
BAD_UPPER="$TMP_DIR/bad-upper.json"
BAD_UPPER_CODE="$(json_post "/content/view/quote" "{
  \"asset_crab_url\":\"crab://$UPPER_HEX.image\",
  \"payer_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_wallet_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:probe-unfunded\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"max_amount_minor\":\"5\",
  \"client_idempotency_key\":\"negative-probe-uppercase-$(date +%s)\"
}" "$BAD_UPPER")"
assert_non_2xx "$BAD_UPPER_CODE" "uppercase hash crab URL"

log "negative quote: recipient mismatch must reject"
BAD_RECIP="$TMP_DIR/bad-recipient.json"
BAD_RECIP_CODE="$(json_post "/content/view/quote" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_wallet_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:probe-unfunded\",
  \"recipient_account\":\"acct_attacker\",
  \"max_amount_minor\":\"5\",
  \"client_idempotency_key\":\"negative-probe-recipient-$(date +%s)\"
}" "$BAD_RECIP")"
assert_non_2xx "$BAD_RECIP_CODE" "recipient mismatch quote"

log "negative pay: unfunded payer must not pay even with fake authority headers"
BAD_UNFUNDED_PAY="$TMP_DIR/bad-unfunded-pay.json"
BAD_UNFUNDED_PAY_CODE="$(json_post "/content/view/pay" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_wallet_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:probe-unfunded\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"amount_minor\":\"5\",
  \"asset\":\"roc\",
  \"quote_id\":\"$QUOTE_ID\",
  \"quote_hash\":\"$QUOTE_HASH\",
  \"client_idempotency_key\":\"negative-probe-unfunded-pay-$(date +%s)\"
}" "$BAD_UNFUNDED_PAY")"
assert_non_2xx "$BAD_UNFUNDED_PAY_CODE" "unfunded pay with fake authority headers"

log "negative pay: self-payment must reject"
BAD_SELF_PAY="$TMP_DIR/bad-self-pay.json"
BAD_SELF_PAY_CODE="$(json_post "/content/view/pay" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$CREATOR_ACCOUNT\",
  \"viewer_wallet_account\":\"$CREATOR_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:self\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"amount_minor\":\"5\",
  \"asset\":\"roc\",
  \"quote_id\":\"$QUOTE_ID\",
  \"quote_hash\":\"$QUOTE_HASH\",
  \"client_idempotency_key\":\"negative-probe-self-pay-$(date +%s)\"
}" "$BAD_SELF_PAY")"
assert_non_2xx "$BAD_SELF_PAY_CODE" "self-payment"

log "negative pay: wrong amount must reject"
BAD_AMOUNT_PAY="$TMP_DIR/bad-amount-pay.json"
BAD_AMOUNT_PAY_CODE="$(json_post "/content/view/pay" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_wallet_account\":\"$UNFUNDED_ACCOUNT\",
  \"viewer_passport_subject\":\"passport:main:probe-unfunded\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"amount_minor\":\"999999\",
  \"asset\":\"roc\",
  \"quote_id\":\"$QUOTE_ID\",
  \"quote_hash\":\"$QUOTE_HASH\",
  \"client_idempotency_key\":\"negative-probe-wrong-amount-pay-$(date +%s)\"
}" "$BAD_AMOUNT_PAY")"
assert_non_2xx "$BAD_AMOUNT_PAY_CODE" "wrong amount payment"

log "post-negative balances must be unchanged"
VISITOR_AFTER="$(get_balance "$VISITOR_ACCOUNT")"
CREATOR_AFTER="$(get_balance "$CREATOR_ACCOUNT")"
UNFUNDED_AFTER="$(curl --connect-timeout 2 --max-time 8 -sS "$GATEWAY_URL/wallet/$UNFUNDED_ACCOUNT/balance" | jq -r '.available_minor_units // "0"' 2>/dev/null || printf '0')"

printf 'visitor after:  %s\n' "$VISITOR_AFTER"
printf 'creator after:  %s\n' "$CREATOR_AFTER"
printf 'unfunded after: %s\n' "$UNFUNDED_AFTER"

[[ "$VISITOR_AFTER" == "$VISITOR_BEFORE" ]] || fail "visitor balance changed during negative probes"
[[ "$CREATOR_AFTER" == "$CREATOR_BEFORE" ]] || fail "creator balance changed during negative probes"
[[ "$UNFUNDED_AFTER" == "$UNFUNDED_BEFORE" || "$UNFUNDED_AFTER" == "0" ]] || fail "unfunded probe account unexpectedly gained balance"

log "NEGATIVE CONTENT_VIEW PROBE PASSED"
