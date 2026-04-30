#!/usr/bin/env bash
# RO:WHAT — Smoke-test crab://image prepare, and optionally paid image upload.
# RO:WHY — Batch 10 reproducible image creator flow before extension MVP.
# RO:INTERACTS — svc-gateway /assets/image/prepare and /assets/image.
# RO:INVARIANTS — upload is skipped unless RON_RUN_PAID_UPLOAD=1 and hold txid/file are provided.
# RO:METRICS — prints route-level pass/fail status; exits non-zero on contract failure.
# RO:CONFIG — RON_GATEWAY_URL, SVC_GATEWAY_BASE_URL, RON_IMAGE_FILE, RON_WALLET_HOLD_TXID.
# RO:SECURITY — never silently spends; paid upload requires explicit env flag.
# RO:TEST — manual smoke: scripts/web3_crab_image_smoke.sh.

set -euo pipefail

GATEWAY_URL="${RON_GATEWAY_URL:-${SVC_GATEWAY_BASE_URL:-http://127.0.0.1:5304}}"
AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}"
PAYER_ACCOUNT="${RON_PAYER_ACCOUNT:-acct_smoke}"
PASSPORT="${RON_PASSPORT:-passport:main:smoke}"
IMAGE_CONTENT_TYPE="${RON_IMAGE_CONTENT_TYPE:-image/png}"
RUN_PAID_UPLOAD="${RON_RUN_PAID_UPLOAD:-0}"
IMAGE_FILE="${RON_IMAGE_FILE:-}"
HOLD_TXID="${RON_WALLET_HOLD_TXID:-}"

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
  local response status payload

  response="$(
    curl -sS \
      -X POST \
      -H "Authorization: ${AUTH_HEADER}" \
      -H "Content-Type: application/json" \
      -H "Idempotency-Key: smoke-image-prepare-$(date +%s)" \
      -H "x-ron-passport: ${PASSPORT}" \
      -H "x-ron-wallet-account: ${PAYER_ACCOUNT}" \
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

assert_prepare() {
  JSON_BODY="$1" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])

checks = {
    "schema": "omnigate.image-asset-prepare.v1",
    "asset_kind": "image",
}

for key, expected in checks.items():
    if data.get(key) != expected:
        print(f"{key} expected {expected!r}, got {data.get(key)!r}", file=sys.stderr)
        print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)

required_paths = [
    ("wallet_hold", "amount_minor"),
    ("wallet_hold", "minimum_hold_minor"),
    ("wallet_hold", "capability"),
    ("paid_storage", "estimate"),
    ("next", "submit_upload"),
]

for path in required_paths:
    value = data
    for part in path:
        if not isinstance(value, dict) or part not in value:
            print(f"missing {'.'.join(path)}", file=sys.stderr)
            print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
            sys.exit(1)
        value = value[part]
PY
}

assert_upload() {
  JSON_BODY="$1" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_BODY"])

if data.get("schema") != "omnigate.image-asset-upload.v1":
    print("expected schema omnigate.image-asset-upload.v1", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

asset_cid = data.get("asset_cid", "")
crab_url = data.get("crab_url", "")

if not asset_cid.startswith("b3:"):
    print("asset_cid must start with b3:", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

if not crab_url.startswith("crab://") or not crab_url.endswith(".image"):
    print("crab_url must be crab://<hash>.image", file=sys.stderr)
    print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

for key in ["storage_upload", "manifest", "index_pointer", "links"]:
    if key not in data:
        print(f"missing {key}", file=sys.stderr)
        print(json.dumps(data, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)
PY
}

need_cmd curl
need_cmd python3

echo "WEB3 crab image smoke"
echo "gateway: ${GATEWAY_URL}"

prepare_body="$(
  python3 - <<PY
import json
print(json.dumps({
    "bytes": int("${RON_IMAGE_BYTES:-64}"),
    "payer_account": "${PAYER_ACCOUNT}",
    "owner_passport_subject": "${PASSPORT}",
    "content_type": "${IMAGE_CONTENT_TYPE}",
    "title": "${RON_IMAGE_TITLE:-Smoke Image}",
    "description": "${RON_IMAGE_DESCRIPTION:-WEB3 smoke image prepare}",
    "tags": ["smoke", "image"],
    "client_idempotency_key": "smoke-image-prepare"
}))
PY
)"

prepare_response="$(http_post_json "${GATEWAY_URL}/assets/image/prepare" "$prepare_body")"
assert_prepare "$prepare_response"
echo "ok: /assets/image/prepare"

if [ "$RUN_PAID_UPLOAD" != "1" ]; then
  echo "skip: paid upload; set RON_RUN_PAID_UPLOAD=1 with RON_IMAGE_FILE and RON_WALLET_HOLD_TXID to enable"
  echo "WEB3 crab image smoke passed"
  exit 0
fi

[ -n "$IMAGE_FILE" ] || fail "RON_IMAGE_FILE is required when RON_RUN_PAID_UPLOAD=1"
[ -f "$IMAGE_FILE" ] || fail "RON_IMAGE_FILE does not exist: $IMAGE_FILE"
[ -n "$HOLD_TXID" ] || fail "RON_WALLET_HOLD_TXID is required when RON_RUN_PAID_UPLOAD=1"

response="$(
  curl -sS \
    -X POST \
    -H "Authorization: ${AUTH_HEADER}" \
    -H "Content-Type: ${IMAGE_CONTENT_TYPE}" \
    -H "Idempotency-Key: smoke-image-upload-$(date +%s)" \
    -H "x-ron-wallet-hold-txid: ${HOLD_TXID}" \
    -H "x-ron-passport: ${PASSPORT}" \
    -H "x-ron-wallet-account: ${PAYER_ACCOUNT}" \
    -H "x-ron-asset-title: ${RON_IMAGE_TITLE:-Smoke Image}" \
    -H "x-ron-asset-description: ${RON_IMAGE_DESCRIPTION:-WEB3 smoke image upload}" \
    -H "x-ron-asset-tags: smoke,image" \
    --data-binary "@${IMAGE_FILE}" \
    -w $'\n__RON_STATUS__:%{http_code}' \
    "${GATEWAY_URL}/assets/image"
)"

status="${response##*__RON_STATUS__:}"
payload="${response%$'\n'__RON_STATUS__:*}"

if [ "$status" != "200" ]; then
  echo "$payload" >&2
  fail "POST /assets/image expected HTTP 200 but got HTTP $status"
fi

assert_upload "$payload"
echo "ok: /assets/image paid upload"

echo "WEB3 crab image smoke passed"