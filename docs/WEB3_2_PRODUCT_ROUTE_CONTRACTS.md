
# WEB3_2 Product Route Contracts

RO:WHAT — Public and internal WEB3_2 product route contract reference.
RO:WHY — Locks the backend surface consumed by CrabLink before paid content-view and QuickChain preflight work.
RO:INTERACTS — svc-gateway, omnigate, svc-storage, svc-index, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, ron-policy, CrabLink.
RO:INVARIANTS — b3 hashes are canonical; gateway is proxy-only; omnigate coordinates product hydration; wallet is mutation front-door; ledger is durable truth.
RO:METRICS — Route behavior is covered by service HTTP metrics and smoke scripts.
RO:CONFIG — Uses svc-gateway/omnigate/storage/index/wallet base URL env vars in local smoke.
RO:SECURITY — Local dev may use Bearer dev; production policy must fail closed; no silent spend.
RO:TEST — cargo tests plus scripts/web3_product_stack_smoke.sh and scripts/web3_paid_site_visit_smoke.sh.

---

## 0. Status

Current WEB3_2 server-side product proof is green for:

```text
CrabLink / script
→ svc-gateway public route
→ omnigate product route
→ svc-storage / svc-index / svc-wallet
→ ron-ledger receipt/root
→ response back to CrabLink
````

Proven route families:

```text
GET  /healthz
GET  /readyz

GET  /b3/<hash>.<kind>
GET  /crab/resolve?url=crab://<hash>.<kind>
GET  /crab/resolve?url=crab://site
GET  /crab/resolve?url=crab://image

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /assets/post/prepare
POST /assets/post
POST /assets/comment/prepare
POST /assets/comment
POST /assets/article/prepare
POST /assets/article

POST /sites/prepare
POST /sites
GET  /sites/:name

POST /sites/:name/visit/quote
POST /sites/:name/visit/pay

GET  /wallet/:account/balance
POST /wallet/hold
```

Current major paid creator proof:

```text
Visitor B / acct_visitor_b
→ opens crab://ron7
→ pays 10 ROC for site_visit
→ Creator A / acct_dev receives 10 ROC
→ svc-wallet returns transfer receipt
→ ron-ledger returns ledger root
→ CrabLink displays receipt and unlocks after backend truth
```

This document is a route contract. It is not a license to move wallet or ledger mutation into gateway, omnigate, CrabLink, accounting, or rewarder.

---

## 1. Canonical identifiers

### 1.1 Internal content ID

Internal content IDs use canonical BLAKE3 form:

```text
b3:<64 lowercase hex>
```

Example:

```text
b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

### 1.2 Public typed asset URL

Public typed asset URLs use:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Example:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Do not use the old public form:

```text
crab://b3/<hash>.<asset_kind>
```

### 1.3 Public named site URL

Named site URLs use:

```text
crab://<site_name>
```

Example:

```text
crab://ron7
```

Names are human pointers. BLAKE3 hashes remain canonical for content.

---

## 2. Service boundary rules

### 2.1 svc-gateway

Gateway responsibilities:

```text
- expose stable public browser/client HTTP paths
- proxy selected headers and bodies to omnigate
- preserve query strings
- filter inbound hop-by-hop headers
- return upstream product response bodies
- apply admission/limits/metrics where configured
```

Gateway non-responsibilities:

```text
- no manifest parsing
- no product hydration
- no pricing logic
- no payout recipient selection
- no wallet mutation
- no ledger mutation
- no accounting truth
- no reward planning
```

### 2.2 omnigate

Omnigate responsibilities:

```text
- coordinate product routes
- hydrate asset/site/profile views
- resolve site manifest context through svc-index and svc-storage
- validate product route request shape
- validate payout context before paid access
- call svc-wallet for economic mutation where required
- wrap wallet receipts into product response DTOs
```

Omnigate non-responsibilities:

```text
- no direct ron-ledger mutation
- no durable economic truth
- no storage byte ownership
- no external chain logic
- no fake receipts
```

### 2.3 svc-wallet and ron-ledger

Wallet responsibilities:

```text
- issue
- transfer
- burn
- hold
- capture
- release
- balance
- tx receipt lookup
- nonce/idempotency gates
- receipt generation
```

Ledger responsibilities:

```text
- durable append-only economic truth
- deterministic replay
- balance state
- ledger root / sequence proof surface
```

### 2.4 ron-accounting and svc-rewarder

Accounting responsibilities:

```text
- usage events
- metering snapshots
- sealed snapshot hashes
- reward-compatible exports
```

Accounting is not balance truth.

Rewarder responsibilities:

```text
- deterministic reward/payout planning
- payout manifests/intents from accounting and policy signals
```

Rewarder must not directly mutate ron-ledger.

---

## 3. Public gateway route surface

### 3.1 GET `/crab/resolve?url=<crab_url>`

Proxies to:

```text
omnigate GET /v1/crab/resolve?url=<crab_url>
```

Supported examples:

```text
GET /crab/resolve?url=crab://site
GET /crab/resolve?url=crab://image
GET /crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Expected successful typed asset schema:

```json
{
  "schema": "omnigate.asset-page.v1"
}
```

Expected successful built-in page schema varies by built-in page.

Named site resolution may be served through `/sites/:name` even when generic `/crab/resolve` is not the preferred path.

### 3.2 GET `/b3/:asset`

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

### 3.3 POST `/paid/o/prepare`

Proxies to:

```text
omnigate POST /v1/paid/o/prepare
```

Purpose:

```text
estimate paid object storage before wallet hold
```

Gateway must not calculate the price. Omnigate/storage/policy provide the estimate.

### 3.4 POST `/assets/image/prepare`

Proxies to:

```text
omnigate POST /v1/assets/image/prepare
```

Purpose:

```text
prepare paid image publish and quote expected ROC cost
```

### 3.5 POST `/assets/image`

Proxies to:

```text
omnigate POST /v1/assets/image
```

Purpose:

```text
publish paid image asset after valid wallet hold proof
```

Important paid-proof headers:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <amount>
x-ron-wallet-txid: <txid>
x-ron-wallet-receipt-hash: <b3 hash>
x-ron-wallet-from: <payer account>
x-ron-wallet-to: <escrow account>
idempotency-key: <client idempotency key>
```

Do not use old header names such as:

```text
x-ron-wallet-hold-txid
```

for new image publish flows.

### 3.6 POST `/assets/post/prepare`

Proxies to:

```text
omnigate POST /v1/assets/post/prepare
```

Purpose:

```text
prepare paid post publish
```

### 3.7 POST `/assets/post`

Proxies to:

```text
omnigate POST /v1/assets/post
```

Purpose:

```text
publish b3-backed .post asset after wallet hold proof
```

Expected successful schema:

```json
{
  "schema": "omnigate.text-asset-publish.v1",
  "asset_kind": "post"
}
```

### 3.8 POST `/assets/comment/prepare`

Proxies to:

```text
omnigate POST /v1/assets/comment/prepare
```

Purpose:

```text
prepare paid comment publish
```

### 3.9 POST `/assets/comment`

Proxies to:

```text
omnigate POST /v1/assets/comment
```

Purpose:

```text
publish b3-backed .comment asset after wallet hold proof
```

Expected successful schema:

```json
{
  "schema": "omnigate.text-asset-publish.v1",
  "asset_kind": "comment"
}
```

### 3.10 POST `/assets/article/prepare`

Proxies to:

```text
omnigate POST /v1/assets/article/prepare
```

Purpose:

```text
prepare paid article publish
```

### 3.11 POST `/assets/article`

Proxies to:

```text
omnigate POST /v1/assets/article
```

Purpose:

```text
publish b3-backed .article asset after wallet hold proof
```

Expected successful schema:

```json
{
  "schema": "omnigate.text-asset-publish.v1",
  "asset_kind": "article"
}
```

### 3.12 POST `/sites/prepare`

Proxies to:

```text
omnigate POST /v1/sites/prepare
```

Purpose:

```text
prepare site creation/launch
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-prepare.v1"
}
```

### 3.13 POST `/sites`

Proxies to:

```text
omnigate POST /v1/sites
```

Purpose:

```text
create a named site manifest and store a site name → manifest CID pointer
```

A site create request must be strict JSON. The site root must be a stored root document CID, not an image CID.

Expected successful schema:

```json
{
  "schema": "omnigate.site-create.v1"
}
```

### 3.14 GET `/sites/:name`

Proxies to:

```text
omnigate GET /v1/sites/:name
```

Purpose:

```text
hydrate a named site page from index pointer + stored site manifest
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-page.v1"
}
```

### 3.15 POST `/sites/:name/visit/quote`

Proxies to:

```text
omnigate POST /v1/sites/:name/visit/quote
```

Purpose:

```text
quote a paid named-site visit before Visitor B pays Creator A
```

Gateway requirements:

```text
- preserve request body
- preserve authorization/dev bearer header
- preserve x-ron-passport
- preserve x-ron-wallet-account
- preserve idempotency-key
- preserve correlation/request IDs
- filter inbound hop-by-hop headers
- do not calculate price
- do not select payout recipient
- do not mutate wallet
```

Omnigate requirements:

```text
- route site name must match body site_name when body provides it
- resolve site pointer from svc-index
- load site manifest from svc-storage
- validate payout.default_action == site_visit
- derive/validate recipient_account from manifest payout
- return integer amount_minor
- no ledger mutation during quote
```

Typical request:

```json
{
  "site_name": "ron7",
  "crab_url": "crab://ron7",
  "action": "site_visit",
  "quantity": 1,
  "payer_account": "acct_visitor_b",
  "visitor_wallet_account": "acct_visitor_b",
  "visitor_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_dev",
  "max_amount_minor": "10",
  "client_idempotency_key": "crablink-site-visit-quote-ron7"
}
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-visit-quote.v1",
  "ok": true,
  "site_name": "ron7",
  "crab_url": "crab://ron7",
  "action": "site_visit",
  "asset": "roc",
  "amount_minor": "10",
  "display_amount": "10 ROC",
  "payer_account": "acct_visitor_b",
  "recipient_account": "acct_dev"
}
```

### 3.16 POST `/sites/:name/visit/pay`

Proxies to:

```text
omnigate POST /v1/sites/:name/visit/pay
```

Purpose:

```text
commit a paid named-site visit through svc-wallet /v1/transfer
```

Gateway requirements:

```text
- proxy-only
- preserve body and selected context headers
- no price/payout logic
- no direct wallet or ledger mutation
```

Omnigate requirements:

```text
- validate same site context used by quote
- reject self-payment
- validate payer_account
- validate recipient_account against site manifest payout
- validate amount_minor equals configured site_visit price
- call svc-wallet /v1/transfer
- return wallet receipt and product receipt wrapper
```

Typical request:

```json
{
  "site_name": "ron7",
  "crab_url": "crab://ron7",
  "action": "site_visit",
  "quantity": 1,
  "payer_account": "acct_visitor_b",
  "visitor_wallet_account": "acct_visitor_b",
  "visitor_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_dev",
  "amount_minor": "10",
  "asset": "roc",
  "quote_id": "site-visit-quote-id",
  "quote_hash": "site-visit-quote-hash",
  "client_idempotency_key": "crablink-site-visit-pay-ron7"
}
```

Expected successful schema:

```json
{
  "schema": "omnigate.site-visit-payment.v1",
  "ok": true,
  "site_name": "ron7",
  "crab_url": "crab://ron7",
  "action": "site_visit",
  "asset": "roc",
  "amount_minor": "10",
  "payer_account": "acct_visitor_b",
  "recipient_account": "acct_dev",
  "txid": "tx_<id>",
  "receipt_hash": "b3:<64 lowercase hex>",
  "ledger_root": "<64 lowercase hex>",
  "wallet_receipt": {
    "op": "transfer",
    "from": "acct_visitor_b",
    "to": "acct_dev",
    "asset": "roc",
    "amount_minor": "10",
    "nonce": 1,
    "idem": "crablink-site-visit-pay-ron7",
    "ledger_root": "<64 lowercase hex>",
    "receipt_hash": "b3:<64 lowercase hex>"
  }
}
```

Balance effect:

```text
acct_visitor_b decreases by 10
acct_dev increases by 10
```

Receipt display rule:

```text
CrabLink may cache/display the returned receipt for UX.
That cache is display-only.
It is not wallet truth.
It is not ledger replay truth.
It is not authorization.
```

---

## 4. Wallet public route surface used by CrabLink

### 4.1 GET `/wallet/:account/balance`

Gateway public route for wallet balance display.

Purpose:

```text
show backend-derived balance
```

CrabLink must not invent balances or treat fallback display as spend authority.

### 4.2 POST `/wallet/hold`

Gateway public route for explicit paid publish/site-create holds.

Purpose:

```text
reserve ROC before paid storage/publish/site create operation
```

All paid actions must be explicit user-confirmed.

---

## 5. Current local smoke gates

### 5.1 Paid creator loop

```bash
bash scripts/web3_nextlevel_creator_loop_green_gate.sh
```

Optional live proof:

```bash
RON_RUN_LIVE_SITE_VISIT=1 bash scripts/web3_nextlevel_creator_loop_green_gate.sh
```

### 5.2 Direct paid site visit smoke

```bash
SITE_NAME=ron7 \
PAYER_ACCOUNT=acct_visitor_b \
RECIPIENT_ACCOUNT=acct_dev \
bash scripts/web3_paid_site_visit_smoke.sh
```

Expected live result:

```text
payer before - 10 == payer after
creator before + 10 == creator after
txid present
receipt_hash starts with b3:
ledger_root present
```

### 5.3 QuickChain preflight

```bash
bash scripts/web3_quickchain_preflight_gate.sh
```

Optional heavier proof:

```bash
RON_PREFLIGHT_RUN_HEAVY=1 \
RON_PREFLIGHT_RUN_LIVE=1 \
bash scripts/web3_quickchain_preflight_gate.sh
```

This does not implement QuickChain. It only proves prerequisites.

---

## 6. Future paid content-view route shape

Do not implement this until site_visit remains green across local stack restarts.

Recommended first paid content view target:

```text
article_view
```

Candidate route family:

```text
POST /assets/:kind/:hash/view/quote
POST /assets/:kind/:hash/view/pay
```

Alternative generic family:

```text
POST /content/view/quote
POST /content/view/pay
```

Required invariants:

```text
- integer minor units only
- no silent spend
- quote before pay
- payer explicit
- recipient explicit or manifest-derived
- wallet mutation through svc-wallet only
- gateway/omnigate do not mutate ledger directly
- receipt returned
- receipt stored/displayed only after success
```

---

## 7. QuickChain lock

QuickChain is still a blueprint only.

Forbidden in active WEB3/NEXT_LEVEL MVP:

```text
- ROX
- Solana
- external settlement
- staking
- liquidity
- exchange-facing logic
- public bridge
- chain logic inside CrabLink
- gateway-side economic mutation
- omnigate direct ledger mutation
```

Current active order:

```text
1. Finish internal ROC closed loop.
2. Prove paid storage/pinning.
3. Prove paid image/site/post/comment/article flows.
4. Prove Visitor B pays Creator A for site visit.
5. Prove wallet receipts, ledger replay, and balance truth.
6. Harden ron-accounting → svc-rewarder → svc-wallet reward loop.
7. Only then begin QuickChain research/prototype.
```

---

## 8. Do-not-regress checklist

Do not regress:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
quote amount 10 ROC in dev config
payer acct_visitor_b
recipient acct_dev
wallet receipt op=transfer
CrabLink unlock after backend proof
gateway proxy-only boundary
wallet as mutation front-door
ledger as durable truth
receipt_hash canonical b3:<64 lowercase hex>
ledger_root present
balance movement verified
```

````

---

## Run this batch

```bash
cd /Users/mymac/Desktop/RustyOnions

chmod +x scripts/web3_nextlevel_creator_loop_green_gate.sh
chmod +x scripts/web3_quickchain_preflight_gate.sh

cargo fmt -p svc-wallet -p omnigate -p svc-gateway
cargo test -p svc-wallet --test i_14_site_visit_receipt_replay
bash scripts/web3_nextlevel_creator_loop_green_gate.sh
````

With the local stack running:

```bash
cd /Users/mymac/Desktop/RustyOnions

RON_RUN_LIVE_SITE_VISIT=1 \
SITE_NAME=ron7 \
PAYER_ACCOUNT=acct_visitor_b \
RECIPIENT_ACCOUNT=acct_dev \
bash scripts/web3_nextlevel_creator_loop_green_gate.sh
```

For the broader preflight:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/web3_quickchain_preflight_gate.sh
```

If the wallet test compiles green, this gives us a durable regression shield for the first real visitor→creator ROC payment loop.
