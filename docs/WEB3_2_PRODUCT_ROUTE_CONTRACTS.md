Perfect. Since the backend is now green, the next best batch is **Batch 10C: route contracts + reproducible green-gate runner**.

This captures what we just proved before moving into the Chrome extension. Your last run proves `omnigate` is green, `site_launch` runs 5 tests, and the live `RON_RUN_SITE_CREATE=1` product stack smoke passes through gateway → omnigate → storage/index → site resolve. 

No Cargo.toml changes.

---

## `docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md`

````markdown
# WEB3_2 Product Route Contracts

RO:WHAT — Public and internal WEB3_2 product route contract reference.
RO:WHY — Locks the backend surface before Chrome extension work begins.
RO:INTERACTS — svc-gateway, omnigate, svc-storage, svc-index, ron-naming, ron-proto.
RO:INVARIANTS — b3 hashes are canonical; gateway is proxy-only; omnigate owns product hydration; storage stores bytes only.
RO:METRICS — Route behavior is covered by existing service HTTP metrics and smoke scripts.
RO:CONFIG — Uses svc-gateway/omnigate/storage/index base URL env vars in live smoke.
RO:SECURITY — Production policy fails closed; local smoke may disable omnigate policy only via generated smoke config.
RO:TEST — cargo tests plus scripts/web3_product_stack_smoke.sh.

## Status

As of Batch 10C, the server-side WEB3_2 product proof is green for:

```text
client/script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
````

Proven flows:

```text
GET  /b3/<hash>.image
GET  /crab/resolve?url=crab://<hash>.image
POST /paid/o/prepare
POST /assets/image/prepare
POST /sites/prepare
POST /sites
GET  /sites/:name
```

Mutation-enabled site smoke is green:

```text
/sites/prepare
→ /sites create
→ JSON site manifest stored in svc-storage
→ site name pointer stored in svc-index
→ /sites/:name hydrates manifest
→ omnigate.site-page.v1 returned
```

Paid image upload is intentionally not part of the default green gate until a real wallet hold txid is supplied.

---

## Canonical identifiers

### Internal content ID

Internal content IDs use canonical BLAKE3 form:

```text
b3:<64 lowercase hex>
```

Example:

```text
b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

### Public crab asset URL

Public typed asset URLs use:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Example:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Do not use the old `crab://b3/<hash>.image` path form for new public UX.

### Public site URL

Named site URLs use:

```text
crab://<site_name>
```

Example:

```text
crab://smoke-site-20260429-173458
```

Names are human pointers. BLAKE3 hashes remain canonical for content.

---

## Gateway public route surface

`svc-gateway` exposes the public product routes.

Gateway responsibilities:

```text
- expose stable browser/client paths
- proxy selected headers and bodies to omnigate
- preserve query strings
- filter hop-by-hop headers
- return upstream product response bodies
```

Gateway non-responsibilities:

```text
- no manifest parsing
- no product hydration
- no pricing logic
- no wallet mutation
- no ledger mutation
- no raw storage semantics
```

### GET `/crab/resolve?url=<crab_url>`

Proxies to:

```text
omnigate GET /v1/crab/resolve?url=<crab_url>
```

Used by the Chrome extension when it intercepts a `crab://...` URL and needs an HTTP DTO.

Supported asset example:

```text
GET /crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Expected successful schema for image asset pages:

```json
{
  "schema": "omnigate.asset-page.v1"
}
```

### GET `/b3/:asset`

Proxies to:

```text
omnigate GET /v1/b3/:asset
```

`:asset` format:

```text
<64 lowercase hex>.<asset_kind>
```

Example:

```text
GET /b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Expected successful schema:

```json
{
  "schema": "omnigate.asset-page.v1",
  "asset_kind": "image",
  "asset_cid": "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
}
```

### POST `/paid/o/prepare`

Proxies to:

```text
omnigate POST /v1/paid/o/prepare
```

Purpose:

```text
Prepare a paid object/storage operation and return a wallet hold template.
```

Gateway does not create the hold.

Expected successful schema:

```json
{
  "schema": "omnigate.paid-object-prepare.v1"
}
```

### POST `/assets/image/prepare`

Proxies to:

```text
omnigate POST /v1/assets/image/prepare
```

Purpose:

```text
Prepare a paid image upload and return an image-specific hold template.
```

Expected successful schema:

```json
{
  "schema": "omnigate.image-asset-prepare.v1",
  "asset_kind": "image"
}
```

### POST `/assets/image`

Proxies to:

```text
omnigate POST /v1/assets/image
```

Purpose:

```text
Coordinate paid image upload after caller has a valid wallet hold txid.
```

Expected successful schema when enabled with real hold:

```json
{
  "schema": "omnigate.image-asset-upload.v1"
}
```

Default smoke scripts do not run this path because it requires a real wallet hold txid.

### POST `/sites/prepare`

Proxies to:

```text
omnigate POST /v1/sites/prepare
```

Purpose:

```text
Prepare a paid static site launch and return a wallet hold template.
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-prepare.v1"
}
```

### POST `/sites`

Proxies to:

```text
omnigate POST /v1/sites
```

Purpose:

```text
Create a site manifest object and write the site name → manifest CID pointer.
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-create.v1"
}
```

Successful create must report:

```json
{
  "manifest": {
    "status": "stored",
    "manifest_cid": "b3:<64 lowercase hex>"
  },
  "index_pointer": {
    "status": "stored"
  }
}
```

### GET `/sites/:name`

Proxies to:

```text
omnigate GET /v1/sites/:name
```

Purpose:

```text
Resolve a site name through svc-index, fetch the manifest from svc-storage, and return a hydrated site page DTO.
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-page.v1"
}
```

Successful hydration must report:

```json
{
  "manifest": {
    "status": "present",
    "hydration_status": "hydrated"
  }
}
```

---

## Internal omnigate route surface

`omnigate` owns product semantics and hydration.

Internal routes:

```text
GET  /v1/crab/resolve
GET  /v1/b3/:asset
POST /v1/paid/o/prepare
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

Omnigate responsibilities:

```text
- parse and normalize product request DTOs
- call svc-storage for paid estimates and manifest object storage
- call svc-index for pointer writes/lookups
- hydrate asset and site page DTOs
- return strict deterministic Problem bodies on failure
```

Omnigate non-responsibilities:

```text
- no direct ledger mutation
- no wallet balance mutation
- no raw CAS ownership
- no mutable index storage of its own
```

---

## Storage and index contracts

### svc-storage

Storage owns raw immutable bytes by BLAKE3 content ID.

For these product routes, storage is used for:

```text
- paid object estimates
- storing generated JSON manifests
- retrieving JSON manifests by b3 CID
```

Storage must not own:

```text
- site names
- asset ownership
- payout splits
- crab:// parsing
- page hydration
```

### svc-index

Index owns mutable product pointers.

For these product routes, index is used for:

```text
- site name → site manifest CID
- asset CID → asset manifest CID
```

Index must not own:

```text
- raw bytes
- wallet mutation
- ledger truth
- payout execution
```

---

## Required forwarded headers

Gateway and omnigate product proxies should forward only selected headers:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

Hop-by-hop headers must not be forwarded:

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

## Error shape

Product routes should return structured problem bodies on deterministic failures:

```json
{
  "code": "some_error_code",
  "message": "human-readable message",
  "retryable": false,
  "reason": "stable_reason"
}
```

Production policy failures are expected to fail closed:

```json
{
  "code": "POLICY_DENY",
  "message": "Access denied",
  "retryable": false,
  "reason": "default"
}
```

Local product stack smoke disables omnigate policy only inside a generated temporary config to test routing and product contracts without requiring a production policy bundle.

---

## Green gates

Run the backend route contract tests:

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test site_launch
cargo test -p omnigate --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
```

Run the safe stack smoke:

```bash
scripts/web3_product_stack_smoke.sh
```

Run the mutation-enabled site smoke:

```bash
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
```

Do not run paid image upload smoke until a real wallet hold txid is available.

---

## Chrome extension handoff

The Chrome extension should treat these gateway routes as its backend API:

```text
GET  /crab/resolve?url=...
GET  /b3/:hash.kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

The extension should remain thin:

```text
- intercept/translate crab:// links
- store local passport/profile selection
- attach x-ron-* headers
- render returned DTOs
- never own wallet/ledger truth
- never invent b3 hashes client-side unless explicitly hashing local upload bytes
```

````

---

## `docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md`

```markdown
# WEB3_2 Product Smoke Runbook

RO:WHAT — One-command WEB3_2 product smoke runbook.
RO:WHY — Makes backend proof reproducible before Chrome extension work.
RO:INTERACTS — scripts/web3_product_stack_smoke.sh and route-specific smoke scripts.
RO:INVARIANTS — safe smoke is default; mutation paths require explicit env flags.
RO:METRICS — Smoke output and per-service logs are written under artifacts/web3-product-smoke-*.
RO:CONFIG — INDEX_BIND, RON_STORAGE_ADDR, OMNIGATE_BIND, SVC_GATEWAY_BIND_ADDR, RON_GATEWAY_URL.
RO:SECURITY — local smoke disables omnigate policy only through generated temporary config.
RO:TEST — Manual runbook for Batch 10C.

## Purpose

This runbook proves the WEB3_2 backend product layer through the public gateway.

The tested path is:

```text
script/client
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
````

This validates the server-side foundation needed before the Chrome extension MVP.

---

## Main script

Use:

```bash
scripts/web3_product_stack_smoke.sh
```

The script:

```text
1. creates a timestamped artifact directory
2. starts svc-index with an isolated per-run DB
3. starts svc-storage
4. starts omnigate with generated local smoke config
5. starts svc-gateway
6. waits for /healthz and /readyz
7. runs product smoke scripts
8. stops all background services
9. saves logs under artifacts/web3-product-smoke-<timestamp>
```

Default safe smoke does not perform paid image upload or site create.

---

## Safe smoke

Run:

```bash
chmod +x scripts/web3_product_stack_smoke.sh scripts/web3_extension_backend_smoke.sh scripts/web3_asset_page_smoke.sh scripts/web3_crab_image_smoke.sh scripts/web3_crab_site_smoke.sh
scripts/web3_product_stack_smoke.sh
```

Expected ending:

```text
WEB3 extension backend smoke passed
WEB3 asset-page smoke passed
WEB3 crab image smoke passed
WEB3 crab site smoke passed

WEB3 product stack smoke passed
```

Expected skips in safe mode:

```text
skip: paid upload
skip: site create/resolve
```

Those skips are intentional.

---

## Mutation-enabled site smoke

Run:

```bash
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
```

Expected site output:

```text
WEB3 crab site smoke
ok: /sites/prepare
ok: /sites create
ok: /sites/<unique-smoke-site>
WEB3 crab site smoke passed
```

Expected final output:

```text
WEB3 product stack smoke passed
```

This proves:

```text
/sites/prepare
→ /sites create
→ site manifest stored as JSON in svc-storage
→ site name pointer stored in svc-index
→ /sites/:name hydrates manifest
```

---

## Paid image upload smoke

Do not run this by default.

It requires:

```text
- a real local image file
- a real wallet hold txid
```

Template:

```bash
RON_RUN_PAID_UPLOAD=1 \
RON_IMAGE_FILE=/absolute/path/to/test.png \
RON_WALLET_HOLD_TXID=hold_real_txid_here \
scripts/web3_product_stack_smoke.sh
```

Never paste placeholder angle brackets like `<hold_txid>` into zsh. Use a real value without angle brackets.

---

## Environment variables

### Stack addresses

Defaults:

```text
INDEX_BIND=127.0.0.1:5304
RON_STORAGE_ADDR=127.0.0.1:5303
OMNIGATE_BIND=127.0.0.1:9090
SVC_GATEWAY_BIND_ADDR=127.0.0.1:8090
```

Override example:

```bash
INDEX_BIND=127.0.0.1:6304 \
RON_STORAGE_ADDR=127.0.0.1:6303 \
OMNIGATE_BIND=127.0.0.1:19090 \
SVC_GATEWAY_BIND_ADDR=127.0.0.1:18090 \
scripts/web3_product_stack_smoke.sh
```

### Product smoke values

Common overrides:

```text
RON_GATEWAY_URL
RON_AUTH_HEADER
RON_PAYER_ACCOUNT
RON_PASSPORT
RON_SITE_NAME
RON_ROOT_DOCUMENT_CID
RON_TEST_IMAGE_HASH
RON_RUN_SITE_CREATE
RON_RUN_PAID_UPLOAD
RON_IMAGE_FILE
RON_WALLET_HOLD_TXID
```

---

## Logs

Each run writes logs to:

```text
artifacts/web3-product-smoke-<timestamp>/
```

Files:

```text
svc-index.log
svc-storage.log
omnigate.log
svc-gateway.log
omnigate-smoke.toml
svc-index.db/
```

Find latest run:

```bash
ls -td artifacts/web3-product-smoke-* | head -1
```

Tail latest service log:

```bash
tail -n 120 "$(ls -td artifacts/web3-product-smoke-* | head -1)/omnigate.log"
```

---

## Common failures

### `POLICY_DENY`

Symptom:

```json
{"code":"POLICY_DENY","message":"Access denied","retryable":false,"reason":"default"}
```

Meaning:

```text
production policy blocked the route
```

For local product smoke, `scripts/web3_product_stack_smoke.sh` generates an `omnigate-smoke.toml` with policy disabled. If this still appears, inspect:

```bash
tail -n 120 "$(ls -td artifacts/web3-product-smoke-* | head -1)/omnigate.log"
```

### `cargo run could not determine which binary`

Symptom:

```text
cargo run could not determine which binary to run
available binaries: roc_b3_tool, svc-storage
```

Fix:

```text
svc-storage must be started as:
cargo run -p svc-storage --bin svc-storage
```

The stack script already does this.

### `site_manifest_bad_json`

Symptom:

```json
{"code":"site_manifest_bad_json","message":"site manifest object was not valid JSON","retryable":true,"reason":"site_manifest_bad_json"}
```

Meaning:

```text
omnigate fetched a manifest CID but could not parse it as the expected site manifest DTO
```

The known Batch 10 fix was to include generated manifest fields like `provenance` and `storage` in the resolver DTO.

### Port already in use

Symptom:

```text
Address already in use
```

Fix by choosing different ports:

```bash
INDEX_BIND=127.0.0.1:6304 \
RON_STORAGE_ADDR=127.0.0.1:6303 \
OMNIGATE_BIND=127.0.0.1:19090 \
SVC_GATEWAY_BIND_ADDR=127.0.0.1:18090 \
scripts/web3_product_stack_smoke.sh
```

---

## Acceptance checklist

Safe smoke is complete when:

```text
WEB3 product stack smoke passed
```

Site mutation smoke is complete when:

```text
ok: /sites create
ok: /sites/<unique-site-name>
WEB3 product stack smoke passed
```

Before Chrome extension work begins, keep these green:

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test site_launch
cargo test -p omnigate --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
scripts/web3_product_stack_smoke.sh
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
```

````

---

## `scripts/web3_product_green_gate.sh`

```bash
#!/usr/bin/env bash
# RO:WHAT — Runs the WEB3_2 product proof green gate.
# RO:WHY — Batch 10C reproducible backend acceptance before Chrome extension work.
# RO:INTERACTS — omnigate tests, svc-gateway tests, product stack smoke scripts.
# RO:INVARIANTS — does not run paid image upload; mutation-enabled site create is explicit and deterministic.
# RO:METRICS — prints pass/fail gate sections; service logs are emitted by web3_product_stack_smoke.sh.
# RO:CONFIG — RUN_FULL_GATE, RUN_STACK_SMOKE, RUN_SITE_CREATE_SMOKE.
# RO:SECURITY — local product stack smoke disables omnigate policy only in generated temporary smoke config.
# RO:TEST — manual: scripts/web3_product_green_gate.sh.

set -euo pipefail

RUN_FULL_GATE="${RUN_FULL_GATE:-1}"
RUN_STACK_SMOKE="${RUN_STACK_SMOKE:-1}"
RUN_SITE_CREATE_SMOKE="${RUN_SITE_CREATE_SMOKE:-1}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 127
  }
}

run_step() {
  local label="$1"
  shift

  echo
  echo "==> ${label}"
  "$@"
}

need_cmd cargo
need_cmd chmod

chmod +x scripts/web3_product_stack_smoke.sh

if [ "$RUN_FULL_GATE" = "1" ]; then
  run_step "cargo fmt" cargo fmt

  run_step "omnigate clippy" \
    cargo clippy -p omnigate --all-targets --no-deps -- -D warnings

  run_step "omnigate site_launch test" \
    cargo test -p omnigate --test site_launch

  run_step "omnigate all-targets tests" \
    cargo test -p omnigate --all-targets

  run_step "svc-gateway clippy" \
    cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings

  run_step "svc-gateway product route proxy test" \
    cargo test -p svc-gateway --test product_routes_proxy

  run_step "svc-gateway all-targets tests" \
    cargo test -p svc-gateway --all-targets
else
  echo "skip: cargo/test gate because RUN_FULL_GATE=${RUN_FULL_GATE}"
fi

if [ "$RUN_STACK_SMOKE" = "1" ]; then
  run_step "safe WEB3 product stack smoke" \
    scripts/web3_product_stack_smoke.sh
else
  echo "skip: safe stack smoke because RUN_STACK_SMOKE=${RUN_STACK_SMOKE}"
fi

if [ "$RUN_SITE_CREATE_SMOKE" = "1" ]; then
  run_step "mutation-enabled site create/resolve stack smoke" \
    env RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
else
  echo "skip: site create smoke because RUN_SITE_CREATE_SMOKE=${RUN_SITE_CREATE_SMOKE}"
fi

echo
echo "WEB3_2 product green gate passed"
````

---

Run this:

```bash
chmod +x scripts/web3_product_green_gate.sh
scripts/web3_product_green_gate.sh
```

For a quicker smoke-only check later:

```bash
RUN_FULL_GATE=0 scripts/web3_product_green_gate.sh
```
