#!/usr/bin/env bash
# RO:WHAT — Live smoke for CrabLink identity/passport/wallet display routes through svc-gateway.
# RO:WHY — WEB3_2 browser-client contract proof; verifies extension-facing routes before real svc-passport wiring.
# RO:INTERACTS — svc-gateway /identity/* and /wallet/* routes, omnigate /v1/identity/* and /v1/wallet/*.
# RO:INVARIANTS — gateway-only; no direct svc-passport/svc-wallet calls; no fake positive ROC; no ledger mutation.
# RO:METRICS — emits x-correlation-id so backend logs/metrics can correlate the smoke.
# RO:CONFIG — RON_GATEWAY_URL, RON_PASSPORT, RON_WALLET_ACCOUNT, RON_AUTH_HEADER.
# RO:SECURITY — dev smoke only; checks that display routes do not grant spend authority.
# RO:TEST — run after local WEB3_2 product stack is up.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STAMP="$(date +%Y%m%d-%H%M%S)"
ARTIFACT_DIR="${RON_IDENTITY_SMOKE_ARTIFACT_DIR:-$ROOT/artifacts/web3-identity-smoke-$STAMP}"

RON_GATEWAY_URL="${RON_GATEWAY_URL:-http://127.0.0.1:8090}"
RON_GATEWAY_URL="${RON_GATEWAY_URL%/}"

RON_PASSPORT="${RON_PASSPORT:-passport:main:dev}"
RON_WALLET_ACCOUNT="${RON_WALLET_ACCOUNT:-acct_dev}"
RON_AUTH_HEADER="${RON_AUTH_HEADER:-Bearer dev}"
CORRELATION_PREFIX="${RON_CORRELATION_PREFIX:-crablink-identity-smoke-$STAMP}"
ALLOW_DEGRADED_READY="${RON_ALLOW_DEGRADED_READY:-0}"

mkdir -p "$ARTIFACT_DIR"

echo "WEB3 identity stack smoke"
echo "root:      $ROOT"
echo "logs:      $ARTIFACT_DIR"
echo "gateway:   $RON_GATEWAY_URL"
echo "passport:  $RON_PASSPORT"
echo "wallet:    $RON_WALLET_ACCOUNT"
echo

request_json() {
  local name="$1"
  local method="$2"
  local path="$3"
  local body="${4:-}"
  local expected_status="${5:-200}"

  local url="${RON_GATEWAY_URL}${path}"
  local body_file="$ARTIFACT_DIR/${name}.body.json"
  local header_file="$ARTIFACT_DIR/${name}.headers.txt"
  local request_file="$ARTIFACT_DIR/${name}.request.json"
  local status_file="$ARTIFACT_DIR/${name}.status.txt"

  local auth_arg=()
  if [[ -n "$RON_AUTH_HEADER" ]]; then
    auth_arg=(-H "Authorization: ${RON_AUTH_HEADER}")
  fi

  echo "→ $method $path"

  local status

  if [[ -n "$body" ]]; then
    printf '%s' "$body" > "$request_file"

    status="$(
      curl -sS \
        -D "$header_file" \
        -o "$body_file" \
        -w "%{http_code}" \
        -X "$method" \
        -H "Accept: application/json" \
        -H "x-correlation-id: ${CORRELATION_PREFIX}-${name}" \
        -H "x-ron-passport: ${RON_PASSPORT}" \
        -H "x-ron-wallet-account: ${RON_WALLET_ACCOUNT}" \
        -H "Content-Type: application/json" \
        -H "Idempotency-Key: ${CORRELATION_PREFIX}-${name}" \
        "${auth_arg[@]}" \
        --data-binary "@$request_file" \
        "$url" || true
    )"
  else
    printf '' > "$request_file"

    status="$(
      curl -sS \
        -D "$header_file" \
        -o "$body_file" \
        -w "%{http_code}" \
        -X "$method" \
        -H "Accept: application/json" \
        -H "x-correlation-id: ${CORRELATION_PREFIX}-${name}" \
        -H "x-ron-passport: ${RON_PASSPORT}" \
        -H "x-ron-wallet-account: ${RON_WALLET_ACCOUNT}" \
        "${auth_arg[@]}" \
        "$url" || true
    )"
  fi

  printf '%s\n' "$status" > "$status_file"

  if [[ "$status" != "$expected_status" ]]; then
    echo "error: expected HTTP $expected_status, got HTTP $status for $method $path"
    echo "body:"
    cat "$body_file" || true
    echo
    echo "headers:"
    cat "$header_file" || true
    echo
    exit 1
  fi

  echo "ok: $method $path -> HTTP $status"
}

request_status() {
  local name="$1"
  local path="$2"
  local expected_status="$3"
  local url="${RON_GATEWAY_URL}${path}"
  local body_file="$ARTIFACT_DIR/${name}.body.txt"
  local header_file="$ARTIFACT_DIR/${name}.headers.txt"
  local status_file="$ARTIFACT_DIR/${name}.status.txt"

  echo "→ GET $path"

  local status
  status="$(
    curl -sS \
      -D "$header_file" \
      -o "$body_file" \
      -w "%{http_code}" \
      -H "Accept: application/json" \
      -H "x-correlation-id: ${CORRELATION_PREFIX}-${name}" \
      "$url" || true
  )"

  printf '%s\n' "$status" > "$status_file"

  if [[ "$status" != "$expected_status" ]]; then
    echo "error: expected HTTP $expected_status, got HTTP $status for GET $path"
    echo "body:"
    cat "$body_file" || true
    echo
    exit 1
  fi

  echo "ok: GET $path -> HTTP $status"
}

validate_identity_me() {
  python3 - "$ARTIFACT_DIR/identity_me.body.json" "$RON_PASSPORT" "$RON_WALLET_ACCOUNT" <<'PY'
import json
import sys

path, expected_passport, expected_wallet = sys.argv[1:4]

with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)

def pick(obj, *paths):
    for path_parts in paths:
        cur = obj
        ok = True
        for part in path_parts:
            if not isinstance(cur, dict) or part not in cur:
                ok = False
                break
            cur = cur[part]
        if ok and cur not in (None, ""):
            return cur
    return None

passport = pick(
    data,
    ("passport", "subject"),
    ("passportSubject",),
    ("passport_subject",),
    ("subject",),
)

wallet = pick(
    data,
    ("wallet", "account"),
    ("walletAccount",),
    ("wallet_account",),
    ("account",),
)

if passport != expected_passport:
    raise SystemExit(f"identity/me passport mismatch: expected {expected_passport!r}, got {passport!r}")

if wallet != expected_wallet:
    raise SystemExit(f"identity/me wallet mismatch: expected {expected_wallet!r}, got {wallet!r}")

caps = data.get("capabilities", {})
if isinstance(caps, dict) and caps.get("can_spend") is True:
    raise SystemExit("identity/me must not grant spend authority")

print("ok: identity/me DTO validated")
PY
}

validate_bootstrap() {
  python3 - "$ARTIFACT_DIR/passport_bootstrap.body.json" "$RON_PASSPORT" "$RON_WALLET_ACCOUNT" <<'PY'
import json
import re
import sys

path, expected_passport, expected_wallet = sys.argv[1:4]

with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)

def pick(obj, *paths):
    for path_parts in paths:
        cur = obj
        ok = True
        for part in path_parts:
            if not isinstance(cur, dict) or part not in cur:
                ok = False
                break
            cur = cur[part]
        if ok and cur not in (None, ""):
            return cur
    return None

schema = data.get("schema", "")
if schema and schema != "crablink.identity.bootstrap.v1":
    raise SystemExit(f"unexpected bootstrap schema: {schema!r}")

passport = pick(
    data,
    ("passport", "subject"),
    ("passportSubject",),
    ("passport_subject",),
    ("subject",),
)

wallet = pick(
    data,
    ("wallet", "account"),
    ("walletAccount",),
    ("wallet_account",),
    ("account",),
)

if passport != expected_passport:
    raise SystemExit(f"bootstrap passport mismatch: expected {expected_passport!r}, got {passport!r}")

if wallet != expected_wallet:
    raise SystemExit(f"bootstrap wallet mismatch: expected {expected_wallet!r}, got {wallet!r}")

grant = data.get("starter_grant") or data.get("starterGrant") or {}
if isinstance(grant, dict):
    issued = grant.get("issued")
    amount = str(
        grant.get("amount_minor_units")
        if grant.get("amount_minor_units") is not None
        else grant.get("amountMinorUnits", "0")
    )
    receipt = grant.get("receipt_id") or grant.get("receiptId")

    if issued is False and amount != "0":
        raise SystemExit("non-issued starter grant must report 0 minor units")

    if issued is True:
        if not re.fullmatch(r"[0-9]+", amount):
            raise SystemExit("issued starter grant amount must be an integer string")
        if not receipt:
            raise SystemExit("issued starter grant must include a receipt id")

caps = data.get("capabilities", {})
if isinstance(caps, dict) and caps.get("can_spend") is True:
    raise SystemExit("bootstrap response must not grant spend authority")

print("ok: passport bootstrap DTO validated")
PY
}

validate_balance() {
  python3 - "$ARTIFACT_DIR/wallet_balance.body.json" "$RON_WALLET_ACCOUNT" <<'PY'
import json
import re
import sys

path, expected_wallet = sys.argv[1:3]

with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)

account = data.get("account") or data.get("wallet_account") or data.get("walletAccount")
if account != expected_wallet:
    raise SystemExit(f"wallet balance account mismatch: expected {expected_wallet!r}, got {account!r}")

available = str(
    data.get("available_minor_units")
    if data.get("available_minor_units") is not None
    else data.get("availableMinorUnits", "")
)

held = str(
    data.get("held_minor_units")
    if data.get("held_minor_units") is not None
    else data.get("heldMinorUnits", "0")
)

if not re.fullmatch(r"-?[0-9]+", available):
    raise SystemExit(f"available_minor_units must be an integer string, got {available!r}")

if not re.fullmatch(r"-?[0-9]+", held):
    raise SystemExit(f"held_minor_units must be an integer string, got {held!r}")

ledger_backed = data.get("ledger_backed")
if ledger_backed is None:
    ledger_backed = data.get("ledgerBacked")

if ledger_backed is False and available != "0":
    raise SystemExit("non-ledger-backed balance must not fake a positive available balance")

if data.get("unit") not in (None, "ROC"):
    raise SystemExit(f"unexpected balance unit: {data.get('unit')!r}")

print("ok: wallet balance DTO validated")
PY
}

request_status "healthz" "/healthz" "200"

if [[ "$ALLOW_DEGRADED_READY" == "1" ]]; then
  echo "→ GET /readyz"
  READY_STATUS="$(
    curl -sS \
      -D "$ARTIFACT_DIR/readyz.headers.txt" \
      -o "$ARTIFACT_DIR/readyz.body.txt" \
      -w "%{http_code}" \
      -H "Accept: application/json" \
      -H "x-correlation-id: ${CORRELATION_PREFIX}-readyz" \
      "${RON_GATEWAY_URL}/readyz" || true
  )"
  printf '%s\n' "$READY_STATUS" > "$ARTIFACT_DIR/readyz.status.txt"
  if [[ "$READY_STATUS" != "200" && "$READY_STATUS" != "503" ]]; then
    echo "error: expected /readyz HTTP 200 or 503, got HTTP $READY_STATUS"
    cat "$ARTIFACT_DIR/readyz.body.txt" || true
    echo
    exit 1
  fi
  echo "ok: GET /readyz -> HTTP $READY_STATUS"
else
  request_status "readyz" "/readyz" "200"
fi

request_json "identity_me" "GET" "/identity/me" "" "200"
validate_identity_me

BOOTSTRAP_BODY='{"kind":"main","label":"CrabLink main passport","client":"crablink-chrome","request_starter_grant":true,"create_wallet":true}'
request_json "passport_bootstrap" "POST" "/identity/passport/bootstrap" "$BOOTSTRAP_BODY" "200"
validate_bootstrap

request_json "wallet_balance" "GET" "/wallet/${RON_WALLET_ACCOUNT}/balance" "" "200"
validate_balance

echo
echo "WEB3 identity stack smoke passed"
echo "artifacts: $ARTIFACT_DIR"