#!/usr/bin/env bash
set -euo pipefail

# RO:WHAT — Live read-only integrity probe for a CrabLink paid image route.
# RO:WHY — Proves backend-derived wallet/display/access state remains coherent without mutating ROC.
# RO:INTERACTS — svc-gateway, omnigate, svc-wallet, svc-storage, svc-index.
# RO:INVARIANTS — no wallet mutation; balances must be ledger-backed; b3 asset and manifest must fetch from backend truth.
# RO:TEST — scripts/dev-internal-roc-beta-live-probes.sh.

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8090}"
OMNIGATE_URL="${OMNIGATE_URL:-http://127.0.0.1:9090}"
WALLET_URL="${WALLET_URL:-http://127.0.0.1:8088}"

CRAB_URL="${CRAB_URL:?set CRAB_URL=crab://<64hex>.image}"
ASSET_HEX="${ASSET_HEX:-}"
VISITOR_ACCOUNT="${VISITOR_ACCOUNT:?set VISITOR_ACCOUNT}"
CREATOR_ACCOUNT="${CREATOR_ACCOUNT:?set CREATOR_ACCOUNT}"

EXPECT_VISITOR="${EXPECT_VISITOR:-}"
EXPECT_CREATOR="${EXPECT_CREATOR:-}"
REQUIRE_PRIVACY_PROOF="${REQUIRE_PRIVACY_PROOF:-1}"

if [[ -z "$ASSET_HEX" ]]; then
  ASSET_HEX="$(printf '%s' "$CRAB_URL" | sed -nE 's#^crab://([0-9a-f]{64})\.[A-Za-z0-9_-]+$#\1#p')"
fi

[[ "$ASSET_HEX" =~ ^[0-9a-f]{64}$ ]] || {
  printf 'FAIL: could not derive ASSET_HEX from CRAB_URL=%s\n' "$CRAB_URL" >&2
  exit 1
}

TMP_DIR="$(mktemp -d /tmp/crablink-live-readonly.XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

log() { printf '\n== %s ==\n' "$*"; }
fail() { printf '\nFAIL: %s\n' "$*" >&2; exit 1; }

command -v jq >/dev/null 2>&1 || fail "jq is required"

log "gateway health"
curl --connect-timeout 2 --max-time 8 -fsS "$GATEWAY_URL/healthz"
printf '\n'

log "gateway readyz"
READYZ_BODY="$(curl --connect-timeout 2 --max-time 8 -fsS "$GATEWAY_URL/readyz" || true)"
if printf '%s' "$READYZ_BODY" | jq . >/dev/null 2>&1; then
  printf '%s' "$READYZ_BODY" | jq .
else
  printf '%s\n' "$READYZ_BODY"
fi

log "omnigate health"
curl --connect-timeout 2 --max-time 8 -fsS "$OMNIGATE_URL/healthz" | jq .

log "wallet health"
curl --connect-timeout 2 --max-time 8 -fsS "$WALLET_URL/healthz" | jq .

log "visitor/payer balance must be ledger-backed"
VISITOR_JSON="$TMP_DIR/visitor-balance.json"
curl --connect-timeout 2 --max-time 8 -fsS \
  "$GATEWAY_URL/wallet/$VISITOR_ACCOUNT/balance" \
  > "$VISITOR_JSON"
jq . "$VISITOR_JSON"

jq -e --arg account "$VISITOR_ACCOUNT" '
  .schema == "crablink.wallet.balance.v1"
  and .account == $account
  and .unit == "ROC"
  and .ledger_backed == true
  and .source == "svc_wallet.v1"
  and (.available_minor_units | test("^[0-9]+$"))
  and (.held_minor_units | test("^[0-9]+$"))
' "$VISITOR_JSON" >/dev/null || fail "visitor/payer balance is not clean ledger-backed wallet truth"

if [[ -n "$EXPECT_VISITOR" ]]; then
  jq -e --arg expected "$EXPECT_VISITOR" '.available_minor_units == $expected' "$VISITOR_JSON" >/dev/null \
    || fail "visitor/payer balance did not match expected $EXPECT_VISITOR"
fi

log "creator/recipient balance must be ledger-backed"
CREATOR_JSON="$TMP_DIR/creator-balance.json"
curl --connect-timeout 2 --max-time 8 -fsS \
  "$GATEWAY_URL/wallet/$CREATOR_ACCOUNT/balance" \
  > "$CREATOR_JSON"
jq . "$CREATOR_JSON"

jq -e --arg account "$CREATOR_ACCOUNT" '
  .schema == "crablink.wallet.balance.v1"
  and .account == $account
  and .unit == "ROC"
  and .ledger_backed == true
  and .source == "svc_wallet.v1"
  and (.available_minor_units | test("^[0-9]+$"))
  and (.held_minor_units | test("^[0-9]+$"))
' "$CREATOR_JSON" >/dev/null || fail "creator/recipient balance is not clean ledger-backed wallet truth"

if [[ -n "$EXPECT_CREATOR" ]]; then
  jq -e --arg expected "$EXPECT_CREATOR" '.available_minor_units == $expected' "$CREATOR_JSON" >/dev/null \
    || fail "creator/recipient balance did not match expected $EXPECT_CREATOR"
fi

log "resolve crab image route through gateway"
RESOLVE_JSON="$TMP_DIR/resolve.json"
curl --connect-timeout 2 --max-time 12 -fsS \
  --get "$GATEWAY_URL/crab/resolve" \
  --data-urlencode "url=$CRAB_URL" \
  > "$RESOLVE_JSON"

jq '{
  schema,
  asset_cid,
  asset_kind,
  manifest,
  storage,
  owner,
  payout,
  metadata,
  links,
  warnings
}' "$RESOLVE_JSON"

jq -e \
  --arg asset "b3:$ASSET_HEX" \
  --arg creator "$CREATOR_ACCOUNT" \
  '
    .schema == "omnigate.asset-page.v1"
    and .asset_cid == $asset
    and .asset_kind == "image"
    and .manifest.status == "present"
    and .manifest.hydration_status == "hydrated"
    and (.manifest.manifest_cid | startswith("b3:"))
    and .storage.available == true
    and (.storage.size_bytes | type == "number")
    and .owner.wallet_account == $creator
    and .payout.recipient_account == $creator
    and .payout.default_action == "content_view"
    and (.links.raw | startswith("/o/b3:"))
    and (.links.manifest | startswith("/o/b3:"))
    and ((.warnings // []) | length == 0)
  ' "$RESOLVE_JSON" >/dev/null || fail "resolved asset page failed integrity assertions"

if [[ "$REQUIRE_PRIVACY_PROOF" = "1" ]]; then
  log "privacy cleanup proof must be present"
  jq -e '
    .rendition_group.privacy.status == "clean"
    and .rendition_group.privacy.metadataRemoved == true
    and .rendition_group.privacy.verification.status == "passed"
    and .rendition_group.relationship_truth == "privacy_cleaned_client_b3_prediction_then_backend_verified_before_display"
  ' "$RESOLVE_JSON" >/dev/null || fail "privacy cleanup proof missing or not passed"
fi

RAW_PATH="$(jq -r '.links.raw' "$RESOLVE_JSON")"
MANIFEST_PATH="$(jq -r '.links.manifest' "$RESOLVE_JSON")"
MANIFEST_CID="$(jq -r '.manifest.manifest_cid' "$RESOLVE_JSON")"
EXPECTED_BYTES="$(jq -r '.storage.size_bytes' "$RESOLVE_JSON")"

log "fetch raw asset bytes"
ASSET_FILE="$TMP_DIR/asset.bin"
curl --connect-timeout 2 --max-time 20 -fsS "$GATEWAY_URL$RAW_PATH" -o "$ASSET_FILE"
ACTUAL_BYTES="$(wc -c < "$ASSET_FILE" | tr -d ' ')"
printf 'asset bytes: expected=%s actual=%s\n' "$EXPECTED_BYTES" "$ACTUAL_BYTES"
[[ "$ACTUAL_BYTES" == "$EXPECTED_BYTES" ]] || fail "raw asset byte count mismatch"

if command -v b3sum >/dev/null 2>&1; then
  log "verify raw asset b3 digest"
  ACTUAL_B3="$(b3sum "$ASSET_FILE" | awk '{print $1}')"
  printf 'asset b3: expected=%s actual=%s\n' "$ASSET_HEX" "$ACTUAL_B3"
  [[ "$ACTUAL_B3" == "$ASSET_HEX" ]] || fail "raw asset b3 mismatch"
else
  log "b3sum not installed; skipped local raw-byte b3 verification"
fi

log "fetch manifest bytes"
MANIFEST_FILE="$TMP_DIR/manifest.json"
curl --connect-timeout 2 --max-time 20 -fsS "$GATEWAY_URL$MANIFEST_PATH" -o "$MANIFEST_FILE"
jq . "$MANIFEST_FILE" >/dev/null || fail "manifest is not valid JSON"

if command -v b3sum >/dev/null 2>&1; then
  log "verify manifest b3 digest"
  MANIFEST_HEX="${MANIFEST_CID#b3:}"
  ACTUAL_MANIFEST_B3="$(b3sum "$MANIFEST_FILE" | awk '{print $1}')"
  printf 'manifest b3: expected=%s actual=%s\n' "$MANIFEST_HEX" "$ACTUAL_MANIFEST_B3"
  [[ "$ACTUAL_MANIFEST_B3" == "$MANIFEST_HEX" ]] || fail "manifest b3 mismatch"
else
  log "b3sum not installed; skipped local manifest b3 verification"
fi

log "LIVE READ-ONLY INTEGRITY CHECK PASSED"
