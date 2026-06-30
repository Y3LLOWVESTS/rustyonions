#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Live quote-binding tamper probe for content_view pay.
# RO:WHY — Proves a valid quote cannot be reused with tampered recipient, quote_hash, or quote_id.
# RO:INTERACTS — omnigate /content/view/quote, /content/view/pay, svc-wallet balances.
# RO:INVARIANTS — tampered quote context must reject before wallet mutation; balances unchanged.
# RO:TEST — scripts/dev-internal-roc-beta-live-probes.sh.

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8090}"
CRAB_URL="${CRAB_URL:?set CRAB_URL}"
PAYER_ACCOUNT="${PAYER_ACCOUNT:?set PAYER_ACCOUNT}"
CREATOR_ACCOUNT="${CREATOR_ACCOUNT:?set CREATOR_ACCOUNT}"
ATTACKER_ACCOUNT="${ATTACKER_ACCOUNT:-acct_attacker_quote_tamper}"
VIEWER_PASSPORT="${VIEWER_PASSPORT:-passport:main:probe}"
AMOUNT="${AMOUNT:-5}"

RUN_ID="$(date +%s)"
TMP_DIR="$(mktemp -d /tmp/crablink-quote-tamper.XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

log() { printf '\n== %s ==\n' "$*"; }
fail() { printf '\nFAIL: %s\n' "$*" >&2; exit 1; }

command -v jq >/dev/null 2>&1 || fail "jq is required"

get_balance() {
  local account="$1"
  curl --connect-timeout 2 --max-time 8 -fsS \
    "$GATEWAY_URL/wallet/$account/balance" \
    | jq -r '.available_minor_units'
}

post_json() {
  local path="$1"
  local body="$2"
  local out="$3"

  curl --connect-timeout 2 --max-time 12 -sS \
    -o "$out" \
    -w '%{http_code}' \
    -X POST "$GATEWAY_URL$path" \
    -H 'Authorization: Bearer dev' \
    -H 'Content-Type: application/json' \
    -d "$body"
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
PAYER_BEFORE="$(get_balance "$PAYER_ACCOUNT")"
CREATOR_BEFORE="$(get_balance "$CREATOR_ACCOUNT")"
ATTACKER_BEFORE="$(curl --connect-timeout 2 --max-time 8 -sS "$GATEWAY_URL/wallet/$ATTACKER_ACCOUNT/balance" | jq -r '.available_minor_units // "0"' 2>/dev/null || printf '0')"

printf 'payer before:    %s\n' "$PAYER_BEFORE"
printf 'creator before:  %s\n' "$CREATOR_BEFORE"
printf 'attacker before: %s\n' "$ATTACKER_BEFORE"

log "create valid quote for real creator"
QUOTE_JSON="$TMP_DIR/quote.json"
QUOTE_CODE="$(post_json "/content/view/quote" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$PAYER_ACCOUNT\",
  \"viewer_wallet_account\":\"$PAYER_ACCOUNT\",
  \"viewer_passport_subject\":\"$VIEWER_PASSPORT\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"max_amount_minor\":\"$AMOUNT\",
  \"client_idempotency_key\":\"quote-tamper-valid-quote-$RUN_ID\"
}" "$QUOTE_JSON")"

if [[ "$QUOTE_CODE" != 2* ]]; then
  cat "$QUOTE_JSON" || true
  fail "valid quote failed with HTTP $QUOTE_CODE"
fi

jq . "$QUOTE_JSON"

QUOTE_ID="$(jq -r '.quote_id // .quote.quote_id' "$QUOTE_JSON")"
QUOTE_HASH="$(jq -r '.quote_hash // .quote.quote_hash' "$QUOTE_JSON")"

[[ "$QUOTE_ID" != "null" && -n "$QUOTE_ID" ]] || fail "missing quote_id"
[[ "$QUOTE_HASH" != "null" && -n "$QUOTE_HASH" ]] || fail "missing quote_hash"

log "tampered pay: recipient changed after valid quote"
BAD_RECIP_PAY="$TMP_DIR/bad-recipient-pay.json"
BAD_RECIP_CODE="$(post_json "/content/view/pay" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$PAYER_ACCOUNT\",
  \"viewer_wallet_account\":\"$PAYER_ACCOUNT\",
  \"viewer_passport_subject\":\"$VIEWER_PASSPORT\",
  \"recipient_account\":\"$ATTACKER_ACCOUNT\",
  \"amount_minor\":\"$AMOUNT\",
  \"asset\":\"roc\",
  \"quote_id\":\"$QUOTE_ID\",
  \"quote_hash\":\"$QUOTE_HASH\",
  \"client_idempotency_key\":\"quote-tamper-bad-recipient-pay-$RUN_ID\"
}" "$BAD_RECIP_PAY")"
cat "$BAD_RECIP_PAY" | jq . 2>/dev/null || cat "$BAD_RECIP_PAY"
assert_non_2xx "$BAD_RECIP_CODE" "tampered recipient pay"

log "tampered pay: quote hash changed"
BAD_HASH_PAY="$TMP_DIR/bad-hash-pay.json"
BAD_HASH_CODE="$(post_json "/content/view/pay" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$PAYER_ACCOUNT\",
  \"viewer_wallet_account\":\"$PAYER_ACCOUNT\",
  \"viewer_passport_subject\":\"$VIEWER_PASSPORT\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"amount_minor\":\"$AMOUNT\",
  \"asset\":\"roc\",
  \"quote_id\":\"$QUOTE_ID\",
  \"quote_hash\":\"tampered-$QUOTE_HASH\",
  \"client_idempotency_key\":\"quote-tamper-bad-hash-pay-$RUN_ID\"
}" "$BAD_HASH_PAY")"
cat "$BAD_HASH_PAY" | jq . 2>/dev/null || cat "$BAD_HASH_PAY"
assert_non_2xx "$BAD_HASH_CODE" "tampered quote hash pay"

log "tampered pay: quote id changed"
BAD_ID_PAY="$TMP_DIR/bad-id-pay.json"
BAD_ID_CODE="$(post_json "/content/view/pay" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$PAYER_ACCOUNT\",
  \"viewer_wallet_account\":\"$PAYER_ACCOUNT\",
  \"viewer_passport_subject\":\"$VIEWER_PASSPORT\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"amount_minor\":\"$AMOUNT\",
  \"asset\":\"roc\",
  \"quote_id\":\"tampered-$QUOTE_ID\",
  \"quote_hash\":\"$QUOTE_HASH\",
  \"client_idempotency_key\":\"quote-tamper-bad-id-pay-$RUN_ID\"
}" "$BAD_ID_PAY")"
cat "$BAD_ID_PAY" | jq . 2>/dev/null || cat "$BAD_ID_PAY"
assert_non_2xx "$BAD_ID_CODE" "tampered quote id pay"

log "post-tamper balances must be unchanged"
PAYER_AFTER="$(get_balance "$PAYER_ACCOUNT")"
CREATOR_AFTER="$(get_balance "$CREATOR_ACCOUNT")"
ATTACKER_AFTER="$(curl --connect-timeout 2 --max-time 8 -sS "$GATEWAY_URL/wallet/$ATTACKER_ACCOUNT/balance" | jq -r '.available_minor_units // "0"' 2>/dev/null || printf '0')"

printf 'payer after:    %s\n' "$PAYER_AFTER"
printf 'creator after:  %s\n' "$CREATOR_AFTER"
printf 'attacker after: %s\n' "$ATTACKER_AFTER"

[[ "$PAYER_AFTER" == "$PAYER_BEFORE" ]] || fail "payer balance changed during quote tamper probes"
[[ "$CREATOR_AFTER" == "$CREATOR_BEFORE" ]] || fail "creator balance changed during quote tamper probes"
[[ "$ATTACKER_AFTER" == "$ATTACKER_BEFORE" || "$ATTACKER_AFTER" == "0" ]] || fail "attacker balance changed during quote tamper probes"

log "QUOTE TAMPER PROBE PASSED"
