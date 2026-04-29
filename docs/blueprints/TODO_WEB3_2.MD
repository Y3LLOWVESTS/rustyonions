Absolutely. Below is a **comprehensive updated optimal TODO list** that incorporates the Chrome extension decision while keeping it correctly **last** in the implementation order.

The key strategy remains:

```text id="qlny9y"
Do not build the browser layer before the server-side product layer exists.
Build the RON product primitives first.
Then build the Chrome extension as a thin client on top.
```

WEB3_2 already makes the browser-extension direction explicit: the product layer includes local RON Passports, wallet-linked passport permissions, `crab://` links, b3 asset pages, manifests, payout metadata, paid access flows, and a browser extension instead of a full custom browser.  The current carryover notes also show the paid storage chain is already product-facing and green through `client → svc-gateway → omnigate → svc-storage → svc-wallet → ron-accounting`. 

---

# RustyOnions WEB3_2 Product-Proof TODO List

## Goal: `crab://image`, `crab://site`, typed b3 asset pages, and Chrome extension beta

## Strategic framing

```text id="1m2b2f"
RON beta internal economy is now proven.
Next: prove the economy is usable by creators.

Do that with:
1. typed b3 asset pages
2. crab:// links
3. paid image upload
4. paid site launch
5. Chrome extension as the browser-facing beta layer
```

This is **not scope creep**. This is the product layer WEB3_2 was waiting for.

The correct product story:

```text id="ri6r6z"
A user opens RON.
Uploads an image.
Pays ROC.
Receives a b3 hash.
Gets a crab://b3/<hash>.image page.
The asset page shows owner/passport, payout address, storage info, and receipts.

Then:

A user opens crab://site.
Chooses a RON name.
Pays ROC.
Launches a static site.
The site is manifest-backed, content-addressed, passport-owned, and RON-routable.
```

---

# 0. Non-negotiable invariants for every batch

These are hard rules across the entire TODO list.

```text id="ikpadw"
- ROC remains internal only.
- No ROX/Solana/external settlement.
- No floating-point money math.
- All ROC amounts use integer minor units.
- All percentages use basis points.
- ron-ledger remains append-only durable truth.
- svc-wallet remains the only normal economic mutation front-door.
- ron-accounting remains transient metering/snapshot input, not balance truth.
- svc-rewarder plans deterministic rewards/payouts but does not mutate ledger directly.
- Paid operations use hold → capture → release.
- Wallet mutations require nonce + idempotency.
- No negative balances by default.
- Transfers conserve value.
- b3:<64 lowercase hex> remains canonical content ID.
- Names are human pointers; b3 hashes are canonical.
- Capabilities/policy checks fail closed.
- No crate crosses its boundary just to make the demo easier.
```

WEB3_2’s service-boundary section explicitly places usage in `ron-accounting`, reward calculation in `svc-rewarder`, economic mutation in `svc-wallet`, durable truth in `ron-ledger`, paid enforcement in storage/gateway/omnigate, identity in passport/auth, naming/indexing in ron-naming/svc-index, bytes in storage/edge, and visibility in dashboard/extension UI. 

---

# 1. Architecture boundary map

## `ron-naming`

Owns:

```text id="y009rr"
crab:// parsing
b3 address normalization
asset kind parsing
named-site syntax
canonical crab string formatting
pure validation helpers
```

Does **not** own:

```text id="ttz51e"
storage
wallet mutation
database persistence
payout execution
asset page hydration
```

## `ron-proto`

Owns:

```text id="qlrzvu"
cross-service DTOs
asset manifest DTOs
ownership claim DTOs
payout role DTOs
asset page response DTOs
paid prepare response DTOs
site manifest DTOs
strict serde contracts
```

Does **not** own:

```text id="qn1qfv"
storage logic
resolver logic
policy decisions
wallet mutation
```

## `svc-storage`

Owns:

```text id="7pd5rp"
raw immutable bytes
BLAKE3 CID verification
paid storage admission
wallet hold receipt verification
wallet capture/release calls for paid writes
usage event export
storage metadata / HEAD availability
```

Does **not** own:

```text id="9yz5y6"
names
manifests
ownership claims
payout split logic
asset pages
crab:// routing
passport identity
site access policy
```

The storage notes are especially strict here: storage stores bytes by CID, must not resolve names, must not own manifests, and must not decide ownership/payout metadata; names, manifests, and asset pages belong upstream in `svc-index`, `ron-naming`, `svc-gateway`, `omnigate`, and WEB3_2 product layers. 

## `svc-index`

Owns:

```text id="uw6jl9"
name → manifest pointer
b3 CID → asset manifest pointer
asset metadata lookup
site manifest lookup
profile/app/page manifest lookup
mutable pointer records
```

Does **not** own:

```text id="10hbnu"
raw bytes
wallet mutation
ledger truth
payout execution
```

## `omnigate`

Owns:

```text id="956l2t"
product-facing BFF routes
crab resolver route
b3 asset page hydration
paid upload prepare routes
paid site prepare routes
view composition
storage/index/wallet-ready response envelopes
```

Does **not** own:

```text id="nnoqig"
durable asset truth
raw byte storage
direct ledger mutation
wallet balances
```

The latest notes already say `omnigate` now sits correctly between edge clients and backend paid-use services and is ready to become the WEB3_2 home for passports, paid app/site/content access, typed b3 asset pages, and crab link hydration. 

## `svc-gateway`

Owns:

```text id="niq2yr"
ingress
edge route exposure
proxying to omnigate/storage
rate limits
body caps
policy gate at edge
public URL shape
```

Does **not** own:

```text id="rwcfrr"
business logic
manifests
payouts
wallet mutation
ledger mutation
asset ownership
```

## `svc-wallet`

Owns:

```text id="6to6gt"
issue
transfer
burn
hold
capture
release
balances
receipts
nonce/idempotency
ledger commit path
```

Does **not** own:

```text id="9b889p"
content metadata
asset pages
storage policy
site routing
ownership manifests
```

## Chrome extension

Owns:

```text id="jgubmx"
browser-facing UI
gateway selection
local node connection settings
crab link input
crab link detection where possible
passport selector UI
wallet binding UI
paid action prompts
receipt display
copy/share buttons
asset page helper UI
```

Does **not** own:

```text id="en1y81"
ledger truth
wallet balance truth
storage truth
DHT routing tables
overlay sessions
reward computation
accounting snapshots
durable content registry
direct ledger mutation
```

WEB3_2 explicitly says the extension should not own ledger truth, wallet balance truth, storage truth, DHT routing tables, overlay sessions, reward computation, or accounting snapshots. 

---

# 2. Optimal implementation order

The new optimal order is:

```text id="f8w3ce"
Batch 0: economics lock-down check
Batch 1: ron-naming crab/b3 parser foundation
Batch 2: ron-proto asset manifest / ownership / payout DTOs
Batch 3: svc-index manifest pointer foundation
Batch 4: omnigate b3 asset page resolver
Batch 5: omnigate /v1/paid/o/prepare
Batch 6: gateway product route exposure
Batch 7: crab://image demo
Batch 8: crab://site backend foundation
Batch 9: crab://site demo
Batch 10: smoke scripts / docs / route contracts
Batch 11: Chrome extension MVP
Batch 12: Chrome extension beta hardening
```

The Chrome extension comes **after** the server-side foundations, because it should be a thin client over working RON routes.

---

# Batch 0 — Economics lock-down check

## Purpose

Do **not** rebuild economics from scratch. Verify the current economics path and only extend what is needed for product demos.

Current paid storage already uses `configs/roc-economics.toml` through the estimate path, and the live smoke proved paid storage, capture/release, accounting export, b3 CID readback, and replay rejection. 

## Tasks

```text id="eis3ub"
1. Verify configs/roc-economics.toml exists and is loaded by storage estimate path.
2. Verify current storage estimate tests still pass.
3. Add or reserve actions for:
   - paid_storage_put
   - asset_publish
   - site_publish
   - paid_content_view
   - paid_site_visit
4. Add basis-point split validation if missing.
5. Ensure all split totals either:
   - equal exactly 10_000 bps, or
   - follow explicit policy for remainder/treasury/burn.
6. Ensure no floating-point math.
7. Ensure all amounts are integer minor units.
```

## Acceptance gates

```text id="jk4qc7"
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --all-targets
cargo test -p svc-storage --test paid_write_estimate
```

## Stop condition

If current economics already passes and no product split support is needed yet, move immediately to Batch 1.

---

# Batch 1 — `ron-naming` crab/b3 parser foundation

## Purpose

Build the pure parser/normalizer foundation for `crab://` and typed b3 asset links.

WEB3_2 defines `crab://` as the native RustyOnions link scheme and includes examples like `crab://sealobsta`, `crab://site/sealobsta`, `crab://b3/<64hex>.image`, `crab://u/<passport-profile>`, and `crab://pay/<wallet-account>`. 

## Files likely needed

```text id="aeeh62"
crates/ron-naming/src/crab.rs
crates/ron-naming/src/asset.rs
crates/ron-naming/src/lib.rs
crates/ron-naming/tests/crab_links.rs
crates/ron-naming/tests/asset_kind.rs
```

## Supported beta forms

```text id="1l18ap"
crab://b3/<64hex>.image
crab://b3/<64hex>.song
crab://b3/<64hex>.article
crab://b3/<64hex>.video
crab://b3/<64hex>.comment
crab://b3/<64hex>.page
crab://b3/<64hex>.manifest
crab://site/<name>
crab://name/<name>
crab://<name>
```

## Parser should return

```text id="n7l4pl"
scheme
namespace
canonical_b3_cid
raw_hash_hex
asset_kind
name if named route
query parameters as normalized map
canonical_string()
```

## Validation rules

```text id="bmn5zm"
accept canonical crab links
reject malformed hashes
normalize or reject uppercase b3 hex by explicit policy
normalize asset kind suffixes
reject invalid suffixes
reject path traversal
reject embedded credentials
reject unsafe control characters
preserve safe query parameters
produce deterministic errors
```

WEB3_2’s crab parser requirements specifically include accepting canonical links, rejecting malformed hashes, normalizing asset kind suffixes, lowercasing b3 hex, rejecting unknown suffixes unless policy allows them, rejecting path traversal, rejecting embedded credentials, rejecting unsafe control characters, and preserving query parameters in normalized form. 

## Acceptance gates

```text id="vdcnin"
valid typed crab links parse
invalid hash length rejects
invalid hash characters reject
unknown asset kind rejects
canonical string is stable
named site links parse
no IO
no async
no storage calls
no wallet calls
no ledger/accounting/rewarder dependencies
```

## Commands

```bash
cargo fmt
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

---

# Batch 2 — `ron-proto` asset manifest / ownership / payout DTOs

## Purpose

Define shared cross-service DTOs for asset manifests, ownership claims, payout roles, and hydrated asset page responses.

`ron-naming` should parse names and asset addresses. `ron-proto` should own cross-service message shapes.

## Files likely needed

```text id="t43u14"
crates/ron-proto/src/asset/mod.rs
crates/ron-proto/src/asset/manifest.rs
crates/ron-proto/src/asset/ownership.rs
crates/ron-proto/src/asset/payout.rs
crates/ron-proto/src/asset/page.rs
crates/ron-proto/src/lib.rs
crates/ron-proto/tests/asset_manifest.rs
crates/ron-proto/tests/asset_page_wire.rs
```

## Minimum DTOs

```text id="b3jgo1"
AssetKind
AssetManifestV1
AssetOwnershipClaimV1
PayoutRole
PayoutTarget
AssetPageV1
StorageAvailabilityV1
ReceiptRefV1
```

## Minimum manifest fields

```text id="4v96t6"
version
asset_cid
asset_kind
manifest_cid optional
owner.passport_subject
owner.wallet_account
payout.default_action
payout.recipient_account
metadata.title
metadata.description
metadata.tags
metadata.license
provenance.created_at
storage.provider_ref optional
receipts optional
```

WEB3_2 says asset manifests can include ownership, payout addresses, tags, descriptions, and curator metadata without changing raw bytes.  It also states ownership should be declared by a signed claim binding asset CID, asset kind, passport subject, wallet account, timestamp, claim type, and signature. 

## Validation rules

```text id="248g3d"
asset_cid must be canonical b3:<64 lowercase hex>
asset_kind must match typed crab suffix when provided
owner passport subject required for ownership claim
wallet account required when payout is enabled
tags bounded
description bounded
license optional but bounded
unknown asset kind rejected unless explicitly experimental
serde deny_unknown_fields where appropriate
stable JSON wire shape
```

## Acceptance gates

```text id="mzk7wk"
DTOs serialize/deserialize deterministically
unknown fields reject
invalid CID rejects
invalid payout roles reject
ownership claim shape stable
asset page response stable
no IO
no service clients
no wallet/ledger mutation
```

## Commands

```bash
cargo fmt
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

---

# Batch 3 — `svc-index` manifest pointer foundation

## Purpose

Add the indexing layer required for asset pages and site manifests.

## Responsibilities

```text id="v9qty1"
b3 CID → asset manifest pointer
name → site manifest pointer
manifest pointer read/write/update APIs
basic dev-mode owner check or placeholder hook
lookup metadata for omnigate hydration
```

## Important model

```text id="ltwe4k"
raw content is immutable
manifest blobs are immutable
manifest pointers can update through signed owner actions
names point to latest manifest CID
b3 asset pages hydrate from manifest pointer + storage metadata
```

WEB3_2 distinguishes immutable manifest blobs from mutable manifest pointers: a manifest blob gets its own b3 hash and never changes, while a name/site/profile/index record may point to the latest manifest CID and can update through signed owner actions. 

## Likely routes or internal APIs

```text id="jmcnkg"
PUT /v1/index/assets/:cid/manifest
GET /v1/index/assets/:cid/manifest
PUT /v1/index/sites/:name/manifest
GET /v1/index/sites/:name/manifest
```

Exact route names can adapt to existing `svc-index` conventions.

## Acceptance gates

```text id="wifw0m"
can store asset manifest pointer
can fetch asset manifest pointer
can store site manifest pointer
can fetch site manifest pointer
bad b3 rejects
bad name rejects
does not store raw bytes
does not mutate wallet/ledger
amnesia/durable behavior explicit
```

## Commands

```bash
cargo fmt
cargo clippy -p svc-index --all-targets -- -D warnings
cargo test -p svc-index --all-targets
```

---

# Batch 4 — `omnigate` server-side b3 asset page resolver

## Purpose

Create the read-only product route that turns a typed b3 crab link into a hydrated asset page response.

The current notes already identify typed b3 asset pages and HTTP route equivalents as the next product-layer work, likely in `omnigate`, while preserving the boundary that storage stores bytes by CID only and omnigate/index/naming hydrate manifests and metadata. 

## Candidate routes

```text id="6ostcz"
GET /v1/crab/resolve?url=crab://b3/<hash>.<kind>
GET /v1/b3/:hash
GET /v1/b3/:hash.:kind
GET /v1/assets/b3/:hash/:kind
```

Recommended MVP:

```text id="pkvk0q"
GET /v1/crab/resolve?url=...
GET /v1/b3/:hash.:kind
```

## Flow

```text id="5xcxmv"
request comes to omnigate
→ parse crab URL with ron-naming
→ validate b3 CID and asset kind
→ ask svc-index for asset manifest pointer
→ ask svc-storage for HEAD/metadata/availability
→ compose AssetPageV1 response
→ return JSON
```

## Beta response shape

```json id="e7xp0k"
{
  "schema": "omnigate.asset-page.v1",
  "asset_cid": "b3:<64hex>",
  "asset_kind": "image",
  "manifest": {
    "status": "present",
    "manifest_cid": "b3:<64hex>"
  },
  "storage": {
    "available": true,
    "size_bytes": 123,
    "content_type": "image/png"
  },
  "owner": {
    "passport_subject": "ron:passport:dev",
    "wallet_account": "acct_creator"
  },
  "payout": {
    "default_action": "content_view",
    "recipient_account": "acct_creator"
  },
  "links": {
    "raw": "/o/b3:<64hex>",
    "crab": "crab://b3/<64hex>.image"
  }
}
```

## Acceptance gates

```text id="6jhh1s"
route is read-only
route does not mutate wallet
route does not mutate ledger
invalid crab URL returns deterministic problem response
missing manifest returns partial asset page if storage exists
missing storage returns unavailable status if manifest exists
storage and index failures map cleanly
headers/correlation IDs preserved
metrics added
```

## Commands

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --all-targets
```

---

# Batch 5 — `omnigate` `/v1/paid/o/prepare`

## Purpose

Bridge UX to the green paid-storage path.

This is the route that lets a browser/extension ask:

```text id="ofllnk"
How much ROC do I need?
What hold should I create?
Where do I submit the upload after the hold exists?
```

The carryover notes explicitly recommend `/v1/paid/o/prepare` because it connects estimate UX to wallet hold UX, does not mutate ledger, prepares browser extension/dashboard flow, fits naturally in omnigate, and does not disturb the green paid-write path. 

## Route

```text id="t92pim"
POST /v1/paid/o/prepare
```

## Flow

```text id="fyqg10"
client sends intended bytes/content_type/action
→ omnigate calls svc-storage /paid/o/estimate
→ omnigate builds wallet hold template
→ omnigate returns submit route and required context
```

## Response shape

```json id="efinlo"
{
  "schema": "omnigate.paid-object-prepare.v1",
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 12345,
  "minimum_hold_minor": "84",
  "recommended_hold_minor": "84",
  "escrow_account": "escrow_paid_write",
  "wallet_hold_request": {
    "from": "acct_user",
    "to": "escrow_paid_write",
    "asset": "roc",
    "amount_minor": "84",
    "memo": "paid_storage_put"
  },
  "next": {
    "create_hold": "/v1/wallet/hold",
    "submit_upload": "/v1/paid/o"
  }
}
```

## Important rule

```text id="a1z541"
This route does not spend money.
This route does not mutate ledger.
This route only prepares the client for wallet hold creation.
```

## Acceptance gates

```text id="zn2bnp"
calls storage estimate route
maps storage estimate errors
returns stable DTO
includes wallet hold template
does not call wallet directly for MVP unless explicitly designed
does not capture/release
does not store content
deterministic problem responses
```

## Commands

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test paid_storage_prepare
cargo test -p omnigate --all-targets
```

---

# Batch 6 — `svc-gateway` product route exposure

## Purpose

Expose clean edge routes for the browser extension and normal HTTP clients.

## Routes

```text id="cobxoj"
GET  /crab/resolve?url=...
GET  /b3/:hash.:kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
```

These may proxy to omnigate as:

```text id="gemwbp"
svc-gateway /crab/resolve       → omnigate /v1/crab/resolve
svc-gateway /b3/:hash.:kind     → omnigate /v1/b3/:hash.:kind
svc-gateway /paid/o/prepare     → omnigate /v1/paid/o/prepare
svc-gateway /assets/image       → omnigate /v1/assets/image
svc-gateway /sites              → omnigate /v1/sites
```

## Acceptance gates

```text id="npe7mu"
gateway remains edge/proxy layer
does not implement business logic
does not parse manifests deeply
does not mutate wallet/ledger
forwards required auth/idempotency/correlation headers
filters hop-by-hop headers
maps upstream errors consistently
body caps enforced
```

## Commands

```bash
cargo fmt
cargo clippy -p svc-gateway --all-targets -- -D warnings
cargo test -p svc-gateway --all-targets
```

---

# Batch 7 — `crab://image` backend demo

## Purpose

First visible product proof.

## User flow

```text id="9s0g7s"
Open crab://image
Upload image
Estimate ROC
Create wallet hold
Submit paid upload
Store bytes by b3 CID
Create/attach asset manifest
Create/attach ownership claim
Index b3 → manifest
Hydrate crab://b3/<hash>.image page
Show receipt
```

## Backend flow

```text id="o6fswp"
client/extension
→ gateway /assets/image/prepare
→ omnigate prepare
→ storage estimate
→ wallet hold
→ gateway /assets/image
→ omnigate upload coordinator
→ storage /paid/o
→ wallet capture/release through storage
→ accounting export
→ index asset manifest pointer
→ omnigate asset page resolver
```

## Minimal dev-mode assumptions

```text id="td2h4p"
first uploader can be default owner in dev mode
passport can be placeholder/dev subject
wallet account can be configured test account
ownership claim signature can be stubbed only if clearly dev-mode
```

WEB3_2 allows first uploader as default owner in simple dev mode, but production should treat ownership as a signed claim rather than an assumption. 

## Acceptance gates

```text id="ql1vmj"
image bytes stored by b3 CID
paid upload succeeds with wallet hold
capture/release happens
accounting export happens
asset manifest pointer exists
asset page resolves
receipt visible
replay with different bytes rejects
storage does not own manifest/ownership logic
```

## Demo output should show

```text id="8qtdsc"
b3:<64 lowercase hex>
crab://b3/<hash>.image
owner/passport subject
wallet/payout account
storage availability
receipt hash/txid
copy buttons
```

---

# Batch 8 — `crab://site` backend foundation

## Purpose

Prepare the static-site launcher without trying to perfect browser UX yet.

## Site manifest fields

```text id="39i6rt"
site_name
site_manifest_cid
owner_passport_subject
owner_wallet_account
root_document_cid
asset_map
route_map
created_at
updated_at
payout_roles
storage_refs
receipt_refs
```

## Required backend behavior

```text id="tl2nlw"
validate site name
estimate total storage cost
prepare wallet hold template
store static files by b3 CID
create site manifest
store site manifest by b3 CID
index name → site manifest CID
resolve crab://<name>
hydrate basic static page
```

## Acceptance gates

```text id="ji4jv3"
site name validates
bad names reject
site bundle files stored by CID
site manifest created
name pointer created
crab://<name> resolves
site page loads through gateway/omnigate
storage does not own site routing
wallet remains mutation front-door
```

---

# Batch 9 — `crab://site` demo

## User flow

```text id="c0zfvb"
Open crab://site
Choose name
Choose template or upload static folder
Estimate ROC
Approve hold
Launch site
Open crab://<name>
Show manifest and receipts
```

## MVP version

```text id="ycmbs1"
one built-in template
one local node
one dev passport
one payer wallet
one storage provider identity
one named site
```

## Stretch version

```text id="e84t2y"
upload static folder
multiple files
site manifest route map
provider payout role
site owner payout role
basic edit/update manifest pointer
```

## Acceptance gates

```text id="ruxvjx"
one-click demo works locally
site launched without CLI/curl
site has manifest CID
site route resolves by crab name
receipts visible
accounting rows visible
balances prove conservation
```

---

# Batch 10 — smoke scripts, docs, and route contracts

## Purpose

Make the product proof reproducible.

## Scripts

```text id="o34cb5"
scripts/web3_crab_image_smoke.sh
scripts/web3_crab_site_smoke.sh
scripts/web3_asset_page_smoke.sh
scripts/web3_extension_backend_smoke.sh
```

## Docs

```text id="3lnfc2"
docs/web3/CRAB_LINKS.md
docs/web3/B3_ASSET_PAGES.md
docs/web3/PAID_UPLOAD_FLOW.md
docs/web3/SITE_LAUNCH_FLOW.md
docs/web3/CHROME_EXTENSION_BACKEND_CONTRACT.md
```

## Contract examples

```text id="3nyizq"
GET /crab/resolve?url=...
GET /b3/:hash.:kind
POST /paid/o/prepare
POST /assets/image
POST /sites
```

## Acceptance gates

```text id="b2ko9r"
fresh checkout can run smoke
scripts avoid magic sleeps where possible
health/ready polling used
clear env var list
clear expected JSON fields
clear failure modes
```

---

# Batch 11 — Chrome extension MVP

## Purpose

Build the browser-facing RON beta layer **after** the server-side product routes are working.

WEB3_2 explicitly says not to build a full RustyOnions browser for MVP because a custom browser would be expensive and slow; the MVP should be a browser extension with passport vault, wallet binding, `crab://` resolver, gateway selector, local Micronode connector, paid action prompts, receipt viewer, identity switcher, and asset page helper UI. 

## Why this comes last

```text id="jfr3pk"
The extension should not invent business logic.
The extension should not compensate for missing backend routes.
The extension should call stable gateway/omnigate routes.
The extension should be a thin UI/client layer.
```

## Extension MVP scope

```text id="jcnoui"
Manifest V3
popup UI
options page
gateway URL setting
local node health check
crab link input box
open crab link button
image upload button
paid action prompt
wallet hold prompt/display
receipt display
asset page preview/open button
copy b3 hash
copy crab:// link
basic error states
```

## Extension folder proposal

```text id="xyi26e"
apps/ron-chrome-extension/
  manifest.json
  package.json
  tsconfig.json
  vite.config.ts

  src/
    background/
      serviceWorker.ts

    popup/
      Popup.tsx
      popup.html
      popup.css

    options/
      Options.tsx
      options.html
      options.css

    content/
      detectCrabLinks.ts

    lib/
      ronClient.ts
      crab.ts
      gateway.ts
      receipts.ts
      errors.ts
      storage.ts

    components/
      NodeStatusBadge.tsx
      CrabInput.tsx
      ImageUploadCard.tsx
      ReceiptCard.tsx
      AssetLinkCard.tsx
      SettingsForm.tsx

    types/
      ron.ts
      extension.ts
```

## MVP screens

```text id="90lwed"
Popup:
  node status
  active gateway
  crab link input
  quick buttons:
    Open crab link
    Upload image
    View receipts
    Settings

Options:
  local gateway URL
  remote gateway URL
  dev bearer/cap token if needed
  default payer account
  default passport subject
  spending prompt settings

Image upload:
  choose file
  estimate cost
  approve hold
  upload
  show result

Result:
  b3 CID
  crab://b3/<hash>.image
  receipt txid/hash
  copy buttons
```

## Extension must call backend routes only

```text id="mgqs7o"
GET  /healthz or /readyz
GET  /crab/resolve?url=...
GET  /b3/:hash.:kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
```

## Extension must not

```text id="dh270v"
mutate ledger directly
own wallet balance truth
own storage truth
store raw private keys in plaintext
silently spend ROC
log bearer tokens
bypass wallet hold/capture/release
hard-code production dev tokens
pretend a payment happened without receipt
```

WEB3_2’s extension security rules include: private passport keys never leave the device, the extension never logs bearer tokens, it never silently spends ROC, paid actions require prompts unless explicit caps are set, caps are short-lived, and identity mode must be obvious in UI. 

## Install/testing path

```text id="2vyu5b"
Phase 1:
  load unpacked extension for internal/devnet testing

Phase 2:
  signed zip release for beta testers

Phase 3:
  unlisted Chrome Web Store beta

Phase 4:
  public Chrome Web Store listing
```

## Estimated LOC

```text id="js05yg"
Basic MVP:        3,000–7,000 LOC
Strong beta:      6,000–12,000 LOC
Polished release: 12,000–25,000 LOC
```

## Difficulty

```text id="874ph1"
Basic MVP: easy-medium
Strong beta: medium
Security-polished release: medium-hard
```

## Acceptance gates

```text id="dq918o"
loads unpacked in Chrome
gateway URL configurable
node health visible
can resolve crab link
can upload image through backend
can show b3 CID
can show crab://b3/<hash>.image
can show receipt
copy buttons work
no scary unnecessary permissions
tokens not logged
paid actions require user approval
works on Windows/macOS/Linux desktop Chrome/Chromium-compatible browsers
```

---

# Batch 12 — Chrome extension beta hardening

## Purpose

Prepare the extension for broader beta testers.

## Tasks

```text id="dxe40p"
1. Minimize permissions.
2. Add privacy policy text.
3. Add local-only token handling rules.
4. Add better error pages:
   - node offline
   - gateway unreachable
   - insufficient ROC
   - hold failed
   - capture failed
   - asset manifest missing
   - storage object missing
5. Add receipt history view.
6. Add content script to detect crab:// links on normal pages.
7. Add right-click “Open with RON” if useful.
8. Add spending cap UI.
9. Add active passport indicator.
10. Add dev/prod mode warning.
11. Add extension backend smoke.
12. Prepare unlisted Chrome Web Store listing.
```

## Acceptance gates

```text id="90fykc"
permissions are minimal
privacy disclosure accurate
unpacked install guide written
beta tester install docs written
extension can survive node restart
extension handles failed payments safely
extension handles missing manifests gracefully
extension never silently spends
```

---

# Updated demo order

## Demo 1 — Backend b3 asset page

```text id="cf763e"
GET /v1/b3/<hash>.image
→ returns hydrated asset page JSON
```

Proves:

```text id="mqb6t1"
crab parser
asset DTOs
index manifest pointer
storage metadata
omnigate hydration
```

## Demo 2 — `crab://image` via HTTP/browser page

```text id="5ct9jz"
Open local gateway page
Upload image
Pay ROC
Get b3 hash
Open b3 asset page
```

Proves:

```text id="rsr9rd"
creator upload flow without extension dependency
```

## Demo 3 — `crab://site` via HTTP/browser page

```text id="h3kwi2"
Open local gateway page
Choose name
Launch static site
Open crab://<name> equivalent
```

Proves:

```text id="ii0o3r"
site launcher backend
```

## Demo 4 — Chrome extension `crab://image`

```text id="yivswz"
Install unpacked extension
Click RON icon
Upload image
Pay ROC
Get crab://b3/<hash>.image
Open asset page
Show receipt
```

Proves:

```text id="0x3f0d"
RON browser-facing beta
```

## Demo 5 — Chrome extension `crab://site`

```text id="34p85i"
Click extension
Open crab://site
Choose name
Launch static site
Open crab://<name>
Show receipts
```

Proves:

```text id="3u3j36"
creator-owned RON publishing loop
```

---

# Updated next-session opening prompt

Use this for the next implementation session:

```text id="mx8oya"
We are pivoting to the WEB3_2 product-proof layer while preserving all WEB3 invariants.

Important strategic decision:
We will build a Chrome extension for beta, but we will implement it last, after the backend product routes are stable. The extension must be a thin client over gateway/omnigate and must not own ledger truth, wallet truth, storage truth, asset registry truth, payout logic, or accounting snapshots.

First batch target:
ron-naming crab/b3 asset foundation.

Implement:
- crates/ron-naming/src/crab.rs
- crates/ron-naming/src/asset.rs if needed
- exports from crates/ron-naming/src/lib.rs
- tests:
  crates/ron-naming/tests/crab_links.rs
  crates/ron-naming/tests/asset_kind.rs

Supported beta crab forms:
- crab://b3/<64hex>.image
- crab://b3/<64hex>.song
- crab://b3/<64hex>.article
- crab://b3/<64hex>.video
- crab://b3/<64hex>.comment
- crab://b3/<64hex>.page
- crab://b3/<64hex>.manifest
- crab://site/<name>
- crab://name/<name>
- crab://<name>

Requirements:
- pure parsing/validation only
- no IO
- no async
- no storage calls
- no wallet calls
- no ledger/accounting/rewarder dependencies
- canonical b3:<64 lowercase hex>
- deterministic errors
- reject malformed hashes
- reject unsafe control characters
- reject embedded credentials
- reject path traversal
- stable canonical_string()
- full paste-ready files
- preserve RO header conventions
- run cargo fmt, clippy for ron-naming, and ron-naming tests
```

---

# Updated “what to attach next session” list

```text id="eare5w"
Required:
- CODEBUNDLE_RS.md
- ALLNOTES.MD
- ALL_DOCS_COMBINED.MD
- WEB3.MD
- WEB3_2.MD
- TODO_WEB3.MD

Helpful:
- configs/roc-economics.toml
- current ron-naming files if codebundle is stale
- current ron-proto manifest/id modules if codebundle is stale
- current omnigate routes/v1 modules if starting Batch 4 or 5
- current svc-gateway route modules if starting Batch 6
```

---

# Current optimal path in one paragraph

The optimal path is to **start with typed b3/crab foundations**, then define shared asset/ownership/payout DTOs, then give `svc-index` manifest pointers, then make `omnigate` hydrate read-only asset pages, then add `/v1/paid/o/prepare`, then expose product routes through `svc-gateway`, then build the `crab://image` demo, then build the `crab://site` demo, and only then build the Chrome extension as a thin beta client. This order keeps RustyOnions modular, protects the already-green paid-storage loop, and gives the extension stable backend routes to call instead of forcing extension code to invent business logic.

