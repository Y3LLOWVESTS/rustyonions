#!/usr/bin/env bash
# RO:WHAT — Smoke-test crab://site prepare, and optionally site create/resolve.
# RO:WHY — Batch 10 reproducible static site launch contract.
# RO:INTERACTS — svc-gateway /sites/prepare, /sites, /sites/:name.
# RO:INVARIANTS — create/resolve is skipped unless RON_RUN_SITE_CREATE=1; create must store manifest and pointer.
# RO:METRICS — prints route-level pass/fail status; exits non-zero on contract failure.
# RO:CONFIG — RON_GATEWAY_URL, SVC_GATEWAY_BASE_URL, RON_SITE_NAME, RON_ROOT_DOCUMENT_CID.
# RO:SECURITY — no wallet calls in gateway; paid hold txid is caller-provided when create is enabled.
# RO:TEST — manual smoke: scripts/web3_crab_site_smoke.sh.

set -euo pipefail

GATEWAY_URL="${RON_GATEWAY_URL:-${SVC_GATEWAY_BASE_URL:-http://127.0.0.1:5304}}"
AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}"
SITE_NAME="${RON_SITE_NAME:-smoke-site}"
PAYER_ACCOUNT="${RON_PAYER_ACCOUNT:-acct_smoke}"
PASSPORT="${RON_PASSPORT:-passport:main:smoke}"
ROOT_DOCUMENT_CID="${RON_ROOT_DOCUMENT_CID:-b3:1111111111111111111111111111111111111111111111111111111111111111}"
RUN_SITE_CREATE="${RON_RUN_SITE_CREATE:-0}"
HOLD_TXID="${RON_WALLET_HOLD_TXID:-hold_smoke_site_launch}"

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

http_post_json() {
  local url="$1"
  local body="$2"
  local idem="$3"
  local response status payload

  response="$(
    curl -sS \
      -X POST \
      -H "Authorization: ${AUTH_HEADER}" \
      -H "Content-Type: application/json" \
      -H "Idempotency-Key: ${idem}" \
      -H "x-ron-passport: ${PASSPORT}" \
      -H "x-ron-wallet-account: ${PAYER_ACCOUNT}" \
      -H "x-ron-wallet-hold-txid: ${HOLD_TXID}" \
      --data "$body" \
      -w $'\n__RON_STATUS__:%{http_code}' \
      "$url"
  )"

  status="${response##*__RON_STATUS__:}"
  payload="${response%$'\n'__RON_STATUS__:*}"

  if [ "$status" != "200" ]; then
    echo "$payload" >&2
    fail "POST $url expected HTTP 200 but got HTTP $status"
  fi

  printf '%s' "$payload"
}

http_get_json() {
  local url="$1"
  local response status payload

  response="$(
    curl -sS \
      -H "Authorization: ${AUTH_HEADER}" \
      -H "x-ron-passport: ${PASSPORT}" \
      -H "x-ron-wallet-account: ${PAYER_ACCOUNT}" \
      -w $'\n__RON_STATUS__:%{http_code}' \
      "$url"
  )"

  status="${response##*__RON_STATUS__:}"
  payload="${response%$'\n'__RON_STATUS__:*}"

  if [ "$status" != "200" ]; then
    echo "$payload" >&2
    fail "GET $url expected HTTP 200 but got HTTP $status"
  fi

  printf '%s' "$payload"
}

assert_schema() {
  local json_body="$1"
  local schema="$2"

  JSON_BODY="$json_body" EXPECTED_SCHEMA="$schema" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])
expected = os.environ["EXPECTED_SCHEMA"]

if data.get("schema") != expected:
    print(f"schema expected {expected!r}, got {data.get('schema')!r}", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)
PY
}

assert_site_prepare() {
  local json_body="$1"

  JSON_BODY="$json_body" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])

if data.get("schema") != "omnigate.site-prepare.v1":
    print("expected omnigate.site-prepare.v1", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

for path in [
    ("wallet_hold", "amount_minor"),
    ("wallet_hold", "minimum_hold_minor"),
    ("site_manifest_preview", "will_create_site_manifest"),
    ("next", "submit_site"),
]:
    value = data
    for part in path:
        if not isinstance(value, dict) or part not in value:
            print(f"missing {'.'.join(path)}", file=sys.stderr)
            print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
            sys.exit(1)
        value = value[part]
PY
}

assert_site_create() {
  local json_body="$1"

  JSON_BODY="$json_body" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])

if data.get("schema") != "omnigate.site-create.v1":
    print("expected omnigate.site-create.v1", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

manifest = data.get("manifest", {})
index_pointer = data.get("index_pointer", {})

if manifest.get("status") != "stored":
    print("site create did not store manifest", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

manifest_cid = manifest.get("manifest_cid")
if not isinstance(manifest_cid, str) or not manifest_cid.startswith("b3:") or len(manifest_cid) != 67:
    print("site create returned invalid manifest_cid", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

if index_pointer.get("status") != "stored":
    print("site create did not store index pointer", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

if data.get("warnings") not in ([], None):
    print("site create returned warnings", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)
PY
}

assert_site_page() {
  local json_body="$1"

  JSON_BODY="$json_body" EXPECTED_SITE_NAME="$SITE_NAME" ROOT_DOCUMENT_CID="$ROOT_DOCUMENT_CID" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])
expected_site_name = os.environ["EXPECTED_SITE_NAME"].lower()
root_document_cid = os.environ["ROOT_DOCUMENT_CID"]

if data.get("schema") != "omnigate.site-page.v1":
    print("expected omnigate.site-page.v1", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

if data.get("site_name") != expected_site_name:
    print(f"site_name mismatch: expected {expected_site_name!r}, got {data.get('site_name')!r}", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

if data.get("root_document_cid") != root_document_cid:
    print("root_document_cid mismatch", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

manifest = data.get("manifest", {})
if manifest.get("hydration_status") != "hydrated":
    print("site page was not hydrated", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)
PY
}

validate_cid() {
  ROOT_DOCUMENT_CID="$ROOT_DOCUMENT_CID" python3 - <<'PY'
import os
import re
import sys

cid = os.environ["ROOT_DOCUMENT_CID"]
if not re.fullmatch(r"b3:[0-9a-f]{64}", cid):
    print("RON_ROOT_DOCUMENT_CID must be canonical b3:<64 lowercase hex>", file=sys.stderr)
    sys.exit(1)
PY
}

need_cmd curl
need_cmd python3
validate_cid

echo "WEB3 crab site smoke"
echo "gateway: ${GATEWAY_URL}"
echo "site: ${SITE_NAME}"

prepare_body="$(
  SITE_NAME="$SITE_NAME" PAYER_ACCOUNT="$PAYER_ACCOUNT" PASSPORT="$PASSPORT" python3 - <<'PY'
import json
import os

print(json.dumps({
    "site_name": os.environ["SITE_NAME"],
    "files": [
        {"path": "index.html", "bytes": 64}
    ],
    "payer_account": os.environ["PAYER_ACCOUNT"],
    "owner_passport_subject": os.environ["PASSPORT"],
    "owner_wallet_account": os.environ["PAYER_ACCOUNT"],
    "title": "Smoke Site",
    "description": "WEB3 smoke site prepare",
    "client_idempotency_key": "smoke-site-prepare"
}))
PY
)"

prepare_response="$(http_post_json "${GATEWAY_URL}/sites/prepare" "$prepare_body" "smoke-site-prepare-$(date +%s)")"
assert_site_prepare "$prepare_response"
echo "ok: /sites/prepare"

if [ "$RUN_SITE_CREATE" != "1" ]; then
  echo "skip: site create/resolve; set RON_RUN_SITE_CREATE=1 to enable manifest+pointer write smoke"
  echo "WEB3 crab site smoke passed"
  exit 0
fi

create_body="$(
  SITE_NAME="$SITE_NAME" ROOT_DOCUMENT_CID="$ROOT_DOCUMENT_CID" PAYER_ACCOUNT="$PAYER_ACCOUNT" PASSPORT="$PASSPORT" python3 - <<'PY'
import json
import os

root = os.environ["ROOT_DOCUMENT_CID"]
print(json.dumps({
    "site_name": os.environ["SITE_NAME"],
    "root_document_cid": root,
    "owner_passport_subject": os.environ["PASSPORT"],
    "owner_wallet_account": os.environ["PAYER_ACCOUNT"],
    "title": "Smoke Site",
    "description": "WEB3 smoke site launch",
    "route_map": {
        "/": root
    },
    "asset_map": {
        "index.html": root
    }
}))
PY
)"

create_response="$(http_post_json "${GATEWAY_URL}/sites" "$create_body" "smoke-site-create-$(date +%s)")"
assert_site_create "$create_response"
echo "ok: /sites create"

resolve_response="$(http_get_json "${GATEWAY_URL}/sites/${SITE_NAME}")"
assert_site_page "$resolve_response"
echo "ok: /sites/${SITE_NAME}"

echo "WEB3 crab site smoke passed"