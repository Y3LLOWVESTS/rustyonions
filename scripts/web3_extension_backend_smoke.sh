#!/usr/bin/env bash
# RO:WHAT — Smoke-test the gateway backend contract expected by the Chrome extension.
# RO:WHY — Batch 10 WEB3_2 reproducibility before extension MVP.
# RO:INTERACTS — svc-gateway public routes, omnigate product routes, svc-storage estimates.
# RO:INVARIANTS — no silent paid upload; no wallet mutation; only prepare/resolve by default.
# RO:CONFIG — RON_GATEWAY_URL, SVC_GATEWAY_BASE_URL, RON_AUTH_HEADER, RON_TEST_SITE_NAME.
# RO:SECURITY — forwards dev auth only when explicitly configured; never logs secrets beyond header presence.
# RO:TEST — manual smoke: scripts/web3_extension_backend_smoke.sh.

set -euo pipefail

GATEWAY_URL="${RON_GATEWAY_URL:-${SVC_GATEWAY_BASE_URL:-http://127.0.0.1:5304}}"
AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}"
TEST_HASH="${RON_TEST_IMAGE_HASH:-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef}"
TEST_SITE_NAME="${RON_TEST_SITE_NAME:-}"

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

http_request() {
  local method="$1"
  local url="$2"
  local expected_status="$3"
  local body="${4:-}"
  local response status payload

  if [ -n "$body" ]; then
    response="$(
      curl -sS \
        -X "$method" \
        -H "Authorization: ${AUTH_HEADER}" \
        -H "Content-Type: application/json" \
        -H "Idempotency-Key: smoke-${method}-$(date +%s)" \
        -H "x-ron-passport: passport:main:smoke" \
        -H "x-ron-wallet-account: acct_smoke" \
        -H "x-correlation-id: smoke-web3-extension-backend" \
        --data "$body" \
        -w $'\n__RON_STATUS__:%{http_code}' \
        "$url"
    )"
  else
    response="$(
      curl -sS \
        -X "$method" \
        -H "Authorization: ${AUTH_HEADER}" \
        -H "x-ron-passport: passport:main:smoke" \
        -H "x-ron-wallet-account: acct_smoke" \
        -H "x-correlation-id: smoke-web3-extension-backend" \
        -w $'\n__RON_STATUS__:%{http_code}' \
        "$url"
    )"
  fi

  status="${response##*__RON_STATUS__:}"
  payload="${response%$'\n'__RON_STATUS__:*}"

  if [ "$status" != "$expected_status" ]; then
    echo "$payload" >&2
    fail "$method $url expected HTTP $expected_status but got HTTP $status"
  fi

  printf '%s' "$payload"
}

assert_json_field() {
  local json_body="$1"
  local field="$2"
  local expected="$3"

  JSON_BODY="$json_body" FIELD="$field" EXPECTED="$expected" python3 - <<'PY'
import json
import os
import sys

body = os.environ["JSON_BODY"]
field = os.environ["FIELD"]
expected = os.environ["EXPECTED"]

try:
    data = json.loads(body)
except Exception as exc:
    print(f"invalid JSON: {exc}", file=sys.stderr)
    print(body, file=sys.stderr)
    sys.exit(1)

value = data
for part in field.split("."):
    if not isinstance(value, dict) or part not in value:
        print(f"missing field {field}", file=sys.stderr)
        print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)
    value = value[part]

if str(value) != expected:
    print(f"field {field} expected {expected!r}, got {value!r}", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)
PY
}

assert_json_has_field() {
  local json_body="$1"
  local field="$2"

  JSON_BODY="$json_body" FIELD="$field" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])
value = data
for part in os.environ["FIELD"].split("."):
    if not isinstance(value, dict) or part not in value:
        print(f"missing field {os.environ['FIELD']}", file=sys.stderr)
        print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)
    value = value[part]
PY
}

need_cmd curl
need_cmd python3

echo "WEB3 extension backend smoke"
echo "gateway: ${GATEWAY_URL}"

paid_prepare_body='{"bytes":64,"payer_account":"acct_smoke","owner_passport_subject":"passport:main:smoke","client_idempotency_key":"smoke-paid-prepare"}'
paid_prepare="$(http_request POST "${GATEWAY_URL}/paid/o/prepare" 200 "$paid_prepare_body")"
assert_json_field "$paid_prepare" "schema" "omnigate.paid-object-prepare.v1"
assert_json_has_field "$paid_prepare" "wallet_hold.amount_minor"
echo "ok: /paid/o/prepare"

image_prepare_body='{"bytes":64,"payer_account":"acct_smoke","owner_passport_subject":"passport:main:smoke","content_type":"image/png","title":"Smoke Image","client_idempotency_key":"smoke-image-prepare"}'
image_prepare="$(http_request POST "${GATEWAY_URL}/assets/image/prepare" 200 "$image_prepare_body")"
assert_json_field "$image_prepare" "schema" "omnigate.image-asset-prepare.v1"
assert_json_field "$image_prepare" "asset_kind" "image"
assert_json_has_field "$image_prepare" "wallet_hold.amount_minor"
echo "ok: /assets/image/prepare"

site_prepare_body='{"site_name":"smoke-site","files":[{"path":"index.html","bytes":64}],"payer_account":"acct_smoke","owner_passport_subject":"passport:main:smoke","owner_wallet_account":"acct_smoke","title":"Smoke Site","client_idempotency_key":"smoke-site-prepare"}'
site_prepare="$(http_request POST "${GATEWAY_URL}/sites/prepare" 200 "$site_prepare_body")"
assert_json_field "$site_prepare" "schema" "omnigate.site-prepare.v1"
assert_json_field "$site_prepare" "site_name" "smoke-site"
assert_json_has_field "$site_prepare" "wallet_hold.amount_minor"
echo "ok: /sites/prepare"

asset_page="$(http_request GET "${GATEWAY_URL}/b3/${TEST_HASH}.image" 200)"
assert_json_field "$asset_page" "schema" "omnigate.asset-page.v1"
assert_json_field "$asset_page" "asset_kind" "image"
echo "ok: /b3/${TEST_HASH}.image"

if [ -n "$TEST_SITE_NAME" ]; then
  site_page="$(http_request GET "${GATEWAY_URL}/sites/${TEST_SITE_NAME}" 200)"
  assert_json_field "$site_page" "schema" "omnigate.site-page.v1"
  assert_json_field "$site_page" "site_name" "$(printf '%s' "$TEST_SITE_NAME" | tr '[:upper:]' '[:lower:]')"
  echo "ok: /sites/${TEST_SITE_NAME}"
else
  echo "skip: /sites/:name resolver smoke; set RON_TEST_SITE_NAME to require it"
fi

echo "WEB3 extension backend smoke passed"