Batch 4 adds a **live CrabLink identity smoke** plus a dedicated **identity route contract doc**. This matches the current backend direction: CrabLink treats `svc-gateway` as its backend API, while gateway stays proxy-only and forwards selected identity/account headers like `Authorization`, `Idempotency-Key`, `x-correlation-id`, and `x-ron-*`.  

No Cargo.toml changes.

---

## `scripts/web3_identity_stack_smoke.sh`

```bash
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

  local -a headers
  headers=(
    -H "Accept: application/json"
    -H "x-correlation-id: ${CORRELATION_PREFIX}-${name}"
    -H "x-ron-passport: ${RON_PASSPORT}"
    -H "x-ron-wallet-account: ${RON_WALLET_ACCOUNT}"
  )

  if [[ -n "$RON_AUTH_HEADER" ]]; then
    headers+=(-H "Authorization: ${RON_AUTH_HEADER}")
  fi

  local -a body_args
  body_args=()

  if [[ -n "$body" ]]; then
    printf '%s' "$body" > "$request_file"
    headers+=(
      -H "Content-Type: application/json"
      -H "Idempotency-Key: ${CORRELATION_PREFIX}-${name}"
    )
    body_args=(--data-binary "@$request_file")
  else
    printf '' > "$request_file"
  fi

  echo "→ $method $path"

  local status
  status="$(
    curl -sS \
      -D "$header_file" \
      -o "$body_file" \
      -w "%{http_code}" \
      -X "$method" \
      "${headers[@]}" \
      "${body_args[@]}" \
      "$url" || true
  )"

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
```

---

## `docs/WEB3_2_IDENTITY_ROUTE_CONTRACTS.md`

````markdown
# WEB3_2 Identity Route Contracts

RO:WHAT — Extension-facing identity/passport/wallet display route contract.
RO:WHY — Locks the CrabLink backend contract before real svc-passport and svc-wallet wiring.
RO:INTERACTS — CrabLink Extension, svc-gateway, omnigate, future svc-passport, future svc-wallet.
RO:INVARIANTS — passports are identities, not wallets; no private keys in browser; no fake ROC truth.
RO:METRICS — Routes inherit normal gateway/omnigate HTTP metrics and use x-correlation-id.
RO:CONFIG — RON_GATEWAY_URL, RON_PASSPORT, RON_WALLET_ACCOUNT, RON_AUTH_HEADER in smoke scripts.
RO:SECURITY — No ambient authority; no response grants uncaveated spend authority.
RO:TEST — scripts/web3_identity_stack_smoke.sh and svc-gateway identity_routes_proxy.rs.

## Purpose

CrabLink needs a small backend surface to determine whether the browser client has a usable RON Passport and linked wallet label.

The extension must call only public gateway routes:

```text
CrabLink Extension
→ svc-gateway
→ omnigate
→ future svc-passport / svc-wallet
````

The extension must not call `svc-passport`, `svc-wallet`, or `ron-ledger` directly.

---

## Current status

The current backend route state is:

```text
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/:account/balance
```

Gateway routes proxy to:

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
GET  /v1/wallet/:account/balance
```

The current omnigate implementation is a dev-safe façade.

It may return safe labels such as:

```text
passport:main:dev
acct_dev
```

It must not return private keys, seed phrases, PIN material, or uncaveated wallet spend authority.

---

## Gateway responsibilities

`svc-gateway` owns public browser/client route exposure.

Gateway responsibilities:

```text
- expose stable public browser paths
- forward selected request headers
- preserve request bodies for POST routes
- preserve query strings where applicable
- return upstream response bodies
- return structured 502 Problem responses when omnigate is unavailable
```

Gateway non-responsibilities:

```text
- no passport key creation
- no private key custody
- no wallet mutation
- no ledger mutation
- no starter ROC issuance
- no local balance calculation
```

---

## Omnigate responsibilities

`omnigate` currently owns the dev façade response shape and later becomes the BFF/coordinator for real downstream identity services.

Current omnigate responsibilities:

```text
- return safe passport display labels
- return safe wallet account display labels
- return capability summary for UI decisions
- mark dev placeholder wallet balances as non-ledger-backed
- fail clearly when request DTOs are malformed
```

Future omnigate responsibilities:

```text
- call svc-passport for passport create/load/status
- call svc-wallet for balance display
- coordinate starter grant request through svc-wallet only
- return committed receipt metadata from svc-wallet/ron-ledger
```

Omnigate must not:

```text
- directly mutate ron-ledger
- directly mint/issue ROC
- fake positive balances
- grant uncaveated spend authority
- store browser private keys
```

---

## Route: GET `/identity/me`

### Public route

```text
GET /identity/me
```

### Internal route

```text
GET /v1/identity/me
```

### Request headers

Recommended headers from CrabLink:

```text
Authorization: Bearer <dev-token-or-capability>
x-correlation-id: <client-correlation-id>
x-ron-passport: <passport-subject>
x-ron-wallet-account: <wallet-account>
```

### Purpose

Return the current browser identity context, if supplied or resolved.

### Example response

```json
{
  "schema": "crablink.identity.me.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "Local Dev Passport",
    "created_at": null,
    "source": "request_header"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "request_header"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "identity view is header-derived until svc-passport is wired"
  ]
}
```

### Contract rules

```text
- passport.subject is a display/identity label, not a private key
- wallet.account is a wallet account label, not balance truth
- capabilities.can_spend must not be true for the dev façade
- no private key, seed phrase, password, or PIN may appear in the response
```

---

## Route: POST `/identity/passport/bootstrap`

### Public route

```text
POST /identity/passport/bootstrap
```

### Internal route

```text
POST /v1/identity/passport/bootstrap
```

### Request headers

Recommended headers:

```text
Authorization: Bearer <dev-token-or-capability>
Idempotency-Key: <unique-client-key>
x-correlation-id: <client-correlation-id>
x-ron-passport: <existing-or-requested-passport-subject>
x-ron-wallet-account: <existing-or-requested-wallet-account>
Content-Type: application/json
```

### Example request

```json
{
  "kind": "main",
  "label": "CrabLink main passport",
  "client": "crablink-chrome",
  "request_starter_grant": true,
  "create_wallet": true
}
```

### Current example response

```json
{
  "schema": "crablink.identity.bootstrap.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "CrabLink main passport",
    "created_at": null,
    "source": "omnigate_dev_bootstrap"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "omnigate_dev_bootstrap"
  },
  "starter_grant": {
    "issued": false,
    "amount_minor_units": "0",
    "receipt_id": null,
    "reason": "svc_wallet_integration_pending"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "dev bootstrap returns labels only",
    "starter ROC is not issued until svc-wallet integration is wired",
    "wallet spend authority is not granted by this response"
  ]
}
```

### Contract rules

```text
- request must be idempotent from the client perspective
- current dev route returns labels only
- current dev route must not issue fake starter ROC
- if starter_grant.issued=false, amount_minor_units must be "0"
- if starter_grant.issued=true in a future implementation, receipt_id must be present
- can_spend must not become true without explicit short-lived caveated authority
```

---

## Route: GET `/wallet/:account/balance`

### Public route

```text
GET /wallet/:account/balance
```

### Internal route

```text
GET /v1/wallet/:account/balance
```

### Purpose

Return a displayable wallet balance DTO for CrabLink.

### Current dev response

```json
{
  "schema": "crablink.wallet.balance.v1",
  "account": "acct_dev",
  "unit": "ROC",
  "available_minor_units": "0",
  "held_minor_units": "0",
  "display": "0 ROC",
  "as_of": null,
  "ledger_backed": false,
  "source": "omnigate_dev_wallet_view.v1",
  "reason": "svc_wallet_integration_pending",
  "warnings": [
    "display-only dev placeholder",
    "real balance must come from svc-wallet and ron-ledger",
    "do not treat this route as spend authority"
  ]
}
```

### Contract rules

```text
- all amounts are integer minor units encoded as strings
- no floating-point money math
- ledger_backed=false means the route is not authoritative wallet truth
- ledger_backed=false must not report a fake positive available balance
- future ledger_backed=true responses must come from svc-wallet/ron-ledger
- balance display does not imply spend authority
```

---

## Starter ROC grant rule

CrabLink may ask for a starter grant during bootstrap:

```json
{
  "request_starter_grant": true
}
```

But CrabLink must not create the balance locally.

Correct future flow:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-passport create/load passport
→ svc-wallet issue starter ROC
→ ron-ledger commit
→ wallet receipt returned
→ CrabLink displays receipt + balance
```

Allowed during current dev façade:

```text
starter_grant.issued=false
amount_minor_units="0"
reason="svc_wallet_integration_pending"
```

Not allowed:

```text
starter_grant.issued=true with no receipt
positive balance with ledger_backed=false
extension-invented ROC balance
direct omnigate ledger mutation
direct gateway ledger mutation
```

---

## Header forwarding contract

Gateway should forward only selected request headers:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

Gateway should not forward hop-by-hop transport headers:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

---

## Smoke script

Run after the local product stack is up:

```bash
scripts/web3_identity_stack_smoke.sh
```

Default environment:

```text
RON_GATEWAY_URL=http://127.0.0.1:8090
RON_PASSPORT=passport:main:dev
RON_WALLET_ACCOUNT=acct_dev
RON_AUTH_HEADER="Bearer dev"
```

Override example:

```bash
RON_GATEWAY_URL=http://127.0.0.1:8090 \
RON_PASSPORT=passport:main:dev \
RON_WALLET_ACCOUNT=acct_dev \
scripts/web3_identity_stack_smoke.sh
```

Expected ending:

```text
WEB3 identity stack smoke passed
```

Artifacts are written under:

```text
artifacts/web3-identity-smoke-<timestamp>
```

---

## Acceptance gate

Before moving from dev façade to real `svc-passport` / `svc-wallet` wiring, these must stay true:

```text
- cargo test -p svc-gateway --test identity_routes_proxy passes
- cargo test -p svc-gateway --test product_routes_proxy passes
- cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings passes
- cargo clippy -p omnigate --no-deps -- -D warnings passes
- scripts/web3_identity_stack_smoke.sh passes against a running local stack
- CrabLink popup can Check Passport and Refresh Balance without crashing
```

````

---

Run this after pasting:

```bash
chmod +x scripts/web3_identity_stack_smoke.sh
scripts/web3_identity_stack_smoke.sh
````

If your product stack is not currently running, start it first, then run the smoke.
