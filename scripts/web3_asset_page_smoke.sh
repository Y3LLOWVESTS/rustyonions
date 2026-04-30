#!/usr/bin/env bash
# RO:WHAT — Smoke-test typed b3 asset pages through svc-gateway.
# RO:WHY — Batch 10 reproducible b3/crab asset-page contract.
# RO:INTERACTS — svc-gateway /b3 and /crab/resolve; omnigate asset-page resolver.
# RO:INVARIANTS — read-only; no wallet/ledger/storage mutation.
# RO:CONFIG — RON_GATEWAY_URL, SVC_GATEWAY_BASE_URL, RON_TEST_IMAGE_HASH.
# RO:SECURITY — validates only public resolver paths.
# RO:TEST — manual smoke: scripts/web3_asset_page_smoke.sh.

set -euo pipefail

GATEWAY_URL="${RON_GATEWAY_URL:-${SVC_GATEWAY_BASE_URL:-http://127.0.0.1:5304}}"
HASH="${RON_TEST_IMAGE_HASH:-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef}"

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

validate_hash() {
  HASH="$HASH" python3 - <<'PY'
import os
import re
import sys

h = os.environ["HASH"]
if not re.fullmatch(r"[0-9a-f]{64}", h):
    print("RON_TEST_IMAGE_HASH must be 64 lowercase hex chars", file=sys.stderr)
    sys.exit(1)
PY
}

http_get() {
  local url="$1"
  local response status payload

  response="$(curl -sS -w $'\n__RON_STATUS__:%{http_code}' "$url")"
  status="${response##*__RON_STATUS__:}"
  payload="${response%$'\n'__RON_STATUS__:*}"

  if [ "$status" != "200" ]; then
    echo "$payload" >&2
    fail "GET $url expected HTTP 200 but got HTTP $status"
  fi

  printf '%s' "$payload"
}

assert_asset_page() {
  local body="$1"
  JSON_BODY="$body" HASH="$HASH" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])
h = os.environ["HASH"]
expected_cid = f"b3:{h}"
expected_crab = f"crab://{h}.image"

checks = {
    "schema": "omnigate.asset-page.v1",
    "asset_cid": expected_cid,
    "asset_kind": "image",
}

for key, expected in checks.items():
    actual = data.get(key)
    if actual != expected:
        print(f"{key} expected {expected!r}, got {actual!r}", file=sys.stderr)
        print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)

links = data.get("links", {})
if links.get("crab") != expected_crab:
    print("links.crab mismatch", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

for key in ["manifest", "storage", "links", "warnings"]:
    if key not in data:
        print(f"missing {key}", file=sys.stderr)
        print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)
PY
}

need_cmd curl
need_cmd python3
validate_hash

echo "WEB3 asset-page smoke"
echo "gateway: ${GATEWAY_URL}"
echo "hash: ${HASH}"

b3_body="$(http_get "${GATEWAY_URL}/b3/${HASH}.image")"
assert_asset_page "$b3_body"
echo "ok: /b3/${HASH}.image"

crab_url="crab://${HASH}.image"
response="$(
  curl -sS \
    --get "${GATEWAY_URL}/crab/resolve" \
    --data-urlencode "url=${crab_url}" \
    -w $'\n__RON_STATUS__:%{http_code}'
)"
status="${response##*__RON_STATUS__:}"
payload="${response%$'\n'__RON_STATUS__:*}"

if [ "$status" != "200" ]; then
  echo "$payload" >&2
  fail "GET /crab/resolve expected HTTP 200 but got HTTP $status"
fi

assert_asset_page "$payload"
echo "ok: /crab/resolve?url=${crab_url}"

echo "WEB3 asset-page smoke passed"