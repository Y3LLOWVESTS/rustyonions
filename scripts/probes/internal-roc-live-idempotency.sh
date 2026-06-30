#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Live mutating idempotency probe for content_view pay.
# RO:WHY — Proves a duplicate retry cannot double-spend after exactly one accepted transfer.
# RO:INTERACTS — omnigate content_view pay and svc-wallet balances.
# RO:INVARIANTS — requires RUN_MUTATING=1; first pay mutates exact delta; duplicate retry does not mutate.
# RO:TEST — manual hardening probe only.

[[ "${RUN_MUTATING:-0}" = "1" ]] || {
  printf 'Refusing mutating probe. Set RUN_MUTATING=1 to spend exactly AMOUNT once.\n' >&2
  exit 2
}

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8090}"
CRAB_URL="${CRAB_URL:?set CRAB_URL}"
PAYER_ACCOUNT="${PAYER_ACCOUNT:?set PAYER_ACCOUNT}"
CREATOR_ACCOUNT="${CREATOR_ACCOUNT:?set CREATOR_ACCOUNT}"
VIEWER_PASSPORT="${VIEWER_PASSPORT:-passport:main:probe}"
AMOUNT="${AMOUNT:-5}"

RUN_ID="$(date +%s)"
QUOTE_KEY="idempotency-probe-quote-$RUN_ID"
PAY_KEY="idempotency-probe-pay-$RUN_ID"

TMP_DIR="$(mktemp -d /tmp/crablink-idempotency-probe.XXXXXX)"
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

log "baseline balances"
PAYER_BEFORE="$(get_balance "$PAYER_ACCOUNT")"
CREATOR_BEFORE="$(get_balance "$CREATOR_ACCOUNT")"

printf 'payer before:   %s\n' "$PAYER_BEFORE"
printf 'creator before: %s\n' "$CREATOR_BEFORE"

if [[ "$PAYER_BEFORE" -lt "$AMOUNT" ]]; then
  fail "$PAYER_ACCOUNT does not have enough ROC for idempotency probe"
fi

log "prepare quote"
QUOTE_JSON="$TMP_DIR/quote.json"
QUOTE_CODE="$(post_json "/content/view/quote" "{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$PAYER_ACCOUNT\",
  \"viewer_wallet_account\":\"$PAYER_ACCOUNT\",
  \"viewer_passport_subject\":\"$VIEWER_PASSPORT\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"max_amount_minor\":\"$AMOUNT\",
  \"client_idempotency_key\":\"$QUOTE_KEY\"
}" "$QUOTE_JSON")"

if [[ "$QUOTE_CODE" != 2* ]]; then
  cat "$QUOTE_JSON" || true
  fail "quote failed with HTTP $QUOTE_CODE"
fi

jq . "$QUOTE_JSON"

QUOTE_ID="$(jq -r '.quote_id // .quote.quote_id' "$QUOTE_JSON")"
QUOTE_HASH="$(jq -r '.quote_hash // .quote.quote_hash' "$QUOTE_JSON")"
QUOTE_AMOUNT="$(jq -r '.amount_minor // .quote.amount_minor' "$QUOTE_JSON")"

[[ "$QUOTE_ID" != "null" && -n "$QUOTE_ID" ]] || fail "missing quote_id"
[[ "$QUOTE_HASH" != "null" && -n "$QUOTE_HASH" ]] || fail "missing quote_hash"
[[ "$QUOTE_AMOUNT" == "$AMOUNT" ]] || fail "quote amount mismatch"

PAY_BODY="{
  \"asset_crab_url\":\"$CRAB_URL\",
  \"payer_account\":\"$PAYER_ACCOUNT\",
  \"viewer_wallet_account\":\"$PAYER_ACCOUNT\",
  \"viewer_passport_subject\":\"$VIEWER_PASSPORT\",
  \"recipient_account\":\"$CREATOR_ACCOUNT\",
  \"amount_minor\":\"$AMOUNT\",
  \"asset\":\"roc\",
  \"quote_id\":\"$QUOTE_ID\",
  \"quote_hash\":\"$QUOTE_HASH\",
  \"client_idempotency_key\":\"$PAY_KEY\"
}"

log "first pay should mutate exactly once"
PAY1_JSON="$TMP_DIR/pay1.json"
PAY1_CODE="$(post_json "/content/view/pay" "$PAY_BODY" "$PAY1_JSON")"

if [[ "$PAY1_CODE" != 2* ]]; then
  cat "$PAY1_JSON" || true
  fail "first pay failed with HTTP $PAY1_CODE"
fi

jq . "$PAY1_JSON"

PAYER_AFTER_FIRST="$(get_balance "$PAYER_ACCOUNT")"
CREATOR_AFTER_FIRST="$(get_balance "$CREATOR_ACCOUNT")"

EXPECTED_PAYER_AFTER_FIRST="$((PAYER_BEFORE - AMOUNT))"
EXPECTED_CREATOR_AFTER_FIRST="$((CREATOR_BEFORE + AMOUNT))"

printf 'payer after first:   expected=%s actual=%s\n' "$EXPECTED_PAYER_AFTER_FIRST" "$PAYER_AFTER_FIRST"
printf 'creator after first: expected=%s actual=%s\n' "$EXPECTED_CREATOR_AFTER_FIRST" "$CREATOR_AFTER_FIRST"

[[ "$PAYER_AFTER_FIRST" == "$EXPECTED_PAYER_AFTER_FIRST" ]] || fail "payer did not decrease by exactly $AMOUNT"
[[ "$CREATOR_AFTER_FIRST" == "$EXPECTED_CREATOR_AFTER_FIRST" ]] || fail "creator did not increase by exactly $AMOUNT"

log "second identical pay retry must not mutate again"
PAY2_JSON="$TMP_DIR/pay2.json"
PAY2_CODE="$(post_json "/content/view/pay" "$PAY_BODY" "$PAY2_JSON")"

printf 'second pay HTTP code: %s\n' "$PAY2_CODE"
cat "$PAY2_JSON" | jq . 2>/dev/null || cat "$PAY2_JSON"

PAYER_AFTER_SECOND="$(get_balance "$PAYER_ACCOUNT")"
CREATOR_AFTER_SECOND="$(get_balance "$CREATOR_ACCOUNT")"

printf 'payer after second:   expected=%s actual=%s\n' "$PAYER_AFTER_FIRST" "$PAYER_AFTER_SECOND"
printf 'creator after second: expected=%s actual=%s\n' "$CREATOR_AFTER_FIRST" "$CREATOR_AFTER_SECOND"

[[ "$PAYER_AFTER_SECOND" == "$PAYER_AFTER_FIRST" ]] || fail "payer changed on duplicate idempotency retry"
[[ "$CREATOR_AFTER_SECOND" == "$CREATOR_AFTER_FIRST" ]] || fail "creator changed on duplicate idempotency retry"

log "IDEMPOTENCY PROBE PASSED"
