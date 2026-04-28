
---

# `WEB3_2_0.md`

````markdown
---
title: WEB3 2.0 — RON Passport, Crab Links, b3 Asset Pages, ROC Economics, and Paid Access
status: draft
last-updated: 2026-04-27
msrv: 1.80.0
owners: [Stevan White]
profile: WEB3-2.0
supersedes: WEB3.MD for the next product layer while preserving every WEB3.MD invariant
---

# WEB3 2.0 — RON Passport, Crab Links, b3 Asset Pages, ROC Economics, and Paid Access

## 0. Purpose

WEB3 2.0 extends the proven RustyOnions internal ROC economy into a user-facing product architecture.

WEB3 1.0 delivered the closed-loop value foundation:

- ROC as the internal unit of account
- `ron-ledger` as durable append-only economic truth
- `svc-wallet` as the sole normal mutation front-door
- `ron-accounting` as transient metering and sealed snapshots
- `svc-rewarder` as deterministic payout planner
- `svc-storage` / `svc-gateway` as the first paid use case
- wallet receipt verification for paid storage
- hold → capture → release as the safe paid-operation lifecycle

WEB3 2.0 adds the user/product layer:

- local persistent RON Passports
- main identity passports
- alternate anonymous/pseudonymous passports
- wallet-linked passport permissions
- a browser extension instead of a full custom browser
- `crab://` links for RON navigation
- b3-hash-first asset pages for every upload
- content manifests with ownership, payout address, tags, description, curator metadata, and provenance
- configurable ROC economics and payout splits through TOML
- paid site visits, paid content views, paid storage, and creator/provider payouts
- dashboard/admin visibility for identity, wallet, receipts, payouts, and economy flow

Core product goal:

```text
Let any developer launch a site, app, article, image, song, video, comment, or page with
content addressing, identity, payment, receipts, storage, indexing, and ROC economics built in.
````

RustyOnions should feel like:

```text
content-addressed web
+ local passport identity
+ optional anonymous identities
+ built-in wallet/payment rail
+ creator/provider payouts
+ deterministic receipts
+ user-controlled paid access
```

---

## 1. Non-Negotiable Invariants Preserved from WEB3.MD

All WEB3 2.0 work must preserve the original WEB3 1.0 invariants.

### 1.1 Value-Plane Invariants

```text
ROC remains internal only for this phase.
ROX/Solana/external settlement remains deferred.
No floating-point money math.
All ROC amounts use integer minor units.
All percentages use integer basis points.
ron-ledger is append-only truth.
svc-wallet is the only normal mutation front-door.
ron-accounting is transient counters/snapshots, not truth.
svc-rewarder computes deterministic issuance/reward intents, but does not mutate ledger directly.
Paid operations use hold → capture → release.
All wallet mutations require nonce + idempotency.
No negative balances by default.
Transfers conserve value.
Mint/issue and burn are explicit audited exceptions.
Wallet receipts are stable and retrievable.
```

### 1.2 Identity / Capability Invariants

```text
Capabilities and policy checks fail closed.
No ambient authority.
Passports may reference wallets, but passports are not wallets.
Wallet spend authority is caveated by account, asset, action, amount, audience, and expiry.
Short-lived capabilities are preferred for runtime actions.
Persistent passport keys stay local to the user’s device unless explicitly exported.
Alt passports must not automatically inherit main-passport wallet authority.
```

### 1.3 Content / Addressing Invariants

```text
Every uploaded content object must receive a BLAKE3-256 content ID.
Canonical content IDs use: b3:<64 lowercase hex>.
The full digest must verify before content is served as that object.
Names are optional human pointers; hashes are canonical.
A named site can update its manifest pointer; a b3 content address cannot mutate.
Asset manifests can add ownership, payout, tags, description, and curator metadata without changing raw bytes.
```

### 1.4 Service Boundary Invariants

Correct value-plane boundary:

```text
usage/metering      → ron-accounting
reward calculation  → svc-rewarder
economic mutation   → svc-wallet
durable truth       → ron-ledger
paid enforcement    → svc-storage / svc-gateway / omnigate
identity/access     → svc-passport / ron-auth
naming/indexing     → ron-naming / svc-index
content bytes       → svc-storage / svc-edge
visibility          → svc-admin / dashboard / extension UI
```

Forbidden:

```text
svc-storage → ron-ledger direct mutation
svc-gateway → ron-ledger direct mutation
omnigate → ron-ledger direct mutation
ron-accounting → ron-ledger direct mutation
svc-rewarder → ron-ledger direct mutation
browser extension → ledger direct mutation
passport token → raw wallet private state
```

---

## 2. b3 Content Addressing as a Core RustyOnions Feature

## 2.1 The Big Idea

Every piece of content uploaded to RustyOnions must be hashed with BLAKE3.

This includes:

```text
images
posts
comments
webpages
articles
songs
albums
videos
documents
manifests
site bundles
app bundles
profile metadata
curator records
```

The hash becomes the canonical address of that content.

Canonical form:

```text
b3:<64 lowercase hex>
```

Example:

```text
b3:4f2c0d7a34...64hex...91aa
```

This address is immutable. The bytes either match the hash or they do not.

## 2.2 Asset Pages

RustyOnions should automatically create or expose an asset page for each uploaded content object.

The asset page is a network page that presents:

```text
raw content identity
asset type
preview/player/reader
ownership claims
creator passport
owner payout account
storage providers
edge/cache providers
tags
description
curator notes
license
content warnings
payout policy
reward attribution
manifest history
related assets
comments/replies
```

This means a song is not merely a blob. It has a native RON page.

An image is not merely a blob. It has a native RON page.

A comment is not merely text. It has a native RON page.

A webpage is not merely HTML. It has a native RON page.

## 2.3 Canonical Raw Content ID vs User-Facing Asset Address

The canonical raw ID remains:

```text
b3:<64 lowercase hex>
```

The user-facing crab asset page should use a browser-safe shape:

```text
crab://b3/<64 lowercase hex>.image
crab://b3/<64 lowercase hex>.post
crab://b3/<64 lowercase hex>.comment
crab://b3/<64 lowercase hex>.page
crab://b3/<64 lowercase hex>.article
crab://b3/<64 lowercase hex>.song
crab://b3/<64 lowercase hex>.video
crab://b3/<64 lowercase hex>.album
crab://b3/<64 lowercase hex>.manifest
```

Human shorthand may display as:

```text
b3hashaddress.song
b3hashaddress.image
b3hashaddress.article
b3hashaddress.video
```

But the parser should normalize it into the full canonical crab form.

Recommended canonical parser result:

```text
scheme      = crab
namespace   = b3
hash        = <64 lowercase hex>
asset_kind  = song | image | article | video | ...
```

## 2.4 Why the Suffix Matters

The suffix tells the resolver how to hydrate the asset page.

Examples:

```text
crab://b3/<hash>.image
→ image viewer + image manifest + ownership + payout + tags

crab://b3/<hash>.song
→ audio player + song manifest + artist/owner + payout + tags

crab://b3/<hash>.article
→ article reader + article manifest + author + payout + tags

crab://b3/<hash>.comment
→ comment view + parent thread + author + moderation/curator metadata

crab://b3/<hash>.page
→ webpage renderer + page manifest + owner + route policy
```

If no suffix is supplied:

```text
crab://b3/<hash>
```

The resolver may:

```text
1. fetch the raw object metadata
2. infer media type if safe
3. check index/manifest records for registered asset kinds
4. show a generic asset page
```

## 2.5 Asset Manifests

Every content object can have one or more manifests.

The raw object is immutable. The manifest is metadata about that object.

### Minimal Asset Manifest

```toml
version = 1
asset_cid = "b3:<64 lowercase hex>"
asset_kind = "image"
created_at_ms = 0

[owner]
passport_subject = "ron:user:alice"
wallet_account = "t:default/u:alice/w"

[payout]
default_action = "paid_content_view"
recipient_role = "content_owner"
recipient_account = "t:default/u:alice/w"

[metadata]
title = "Example Image"
description = "An example uploaded image."
tags = ["art", "example", "ron"]
language = "en"
license = "all-rights-reserved"
```

### Rich Asset Manifest

```toml
version = 1
asset_cid = "b3:<64 lowercase hex>"
asset_kind = "song"
manifest_cid = "b3:<64 lowercase hex>"
created_at_ms = 0
updated_at_ms = 0

[owner]
passport_subject = "ron:user:artist123"
display_name = "Artist Name"
wallet_account = "t:default/u:artist123/w"
ownership_claim_signature = "sig:..."

[payout]
economics_action = "paid_song_play"
primary_recipient = "content_owner"
primary_account = "t:default/u:artist123/w"
royalty_policy = "standard-song-v1"

[metadata]
title = "Example Song"
description = "A song uploaded to RustyOnions."
tags = ["music", "indie", "demo"]
language = "en"
license = "creator-standard-v1"
content_warning = "none"

[media]
mime = "audio/mpeg"
duration_ms = 180000
byte_len = 1234567
cover_image_cid = "b3:<64 lowercase hex>"

[curation]
curator_subjects = ["ron:user:curator1"]
curator_tags = ["recommended", "new-music"]
curator_note_cid = "b3:<64 lowercase hex>"

[provenance]
uploaded_by = "ron:user:artist123"
upload_passport_kind = "main"
storage_receipt_txid = "tx_..."
storage_receipt_hash = "b3:<64 lowercase hex>"
```

## 2.6 Manifest Mutability Rules

Raw content is immutable.

Manifests are metadata records.

There are two types of manifests:

```text
1. immutable manifest
2. mutable manifest pointer
```

Immutable manifest:

```text
A manifest blob is itself content-addressed.
It receives its own b3 hash.
It never changes after being stored.
```

Mutable manifest pointer:

```text
A name/site/profile/index record may point to the latest manifest CID.
This pointer can update through signed owner actions.
```

This gives us both:

```text
immutable auditability
and
updatable metadata / ownership / tags / descriptions
```

## 2.7 Ownership Claims

A b3 hash by itself proves content identity, not legal ownership.

Ownership must be declared by a signed claim.

Ownership claim should bind:

```text
asset_cid
asset_kind
passport_subject
wallet_account
timestamp
claim_type
signature
```

Example claim:

```json
{
  "claim_type": "asset_ownership",
  "asset_cid": "b3:<64 lowercase hex>",
  "asset_kind": "song",
  "passport_subject": "ron:user:artist123",
  "wallet_account": "t:default/u:artist123/w",
  "created_at_ms": 0,
  "signature": "sig:..."
}
```

The first uploader may be the default owner in simple dev mode, but production should treat ownership as a signed claim, not an assumption.

## 2.8 Payout Addressing

Every asset manifest may include payout destinations.

Examples:

```text
content_owner
site_owner
app_owner
artist
album_owner
storage_provider
edge_provider
curator_pool
moderator_pool
treasury
reward_pool
burn
```

The economics policy decides percentages.

The manifest tells the system who the dynamic recipient is.

Example:

```toml
[payout_roles]
content_owner = "t:default/u:artist123/w"
curator_pool = "t:default/curators/music"
storage_provider = "dynamic:storage_provider"
edge_provider = "dynamic:edge_provider"
```

Then `roc-economics.toml` can say:

```toml
[[actions.paid_song_play.splits]]
to = "content_owner"
bps = 7000

[[actions.paid_song_play.splits]]
to = "edge_provider"
bps = 1500

[[actions.paid_song_play.splits]]
to = "treasury"
bps = 1000

[[actions.paid_song_play.splits]]
to = "reward_pool"
bps = 500
```

---

## 3. Crab Links

## 3.1 Purpose

`crab://` is the native RustyOnions link scheme.

It should let users navigate RON sites, apps, profiles, content, and payments from a browser extension without requiring a full custom browser.

Examples:

```text
crab://sealobsta
crab://site/sealobsta
crab://app/music-player
crab://name/sealobsta
crab://b3/<64hex>.image
crab://b3/<64hex>.song
crab://b3/<64hex>.article
crab://u/<passport-profile>
crab://pay/<wallet-account>
```

## 3.2 Named Sites Without TLDs

Users should be able to type:

```text
crab://sealobsta
```

No `.com` required.

Named site resolution should treat the first path/authority component as a RON name.

Examples:

```text
crab://sealobsta
crab://myblog
crab://musiczone
```

Dot notation should be reserved for typed asset pages and explicit namespaces.

Recommended distinction:

```text
crab://sealobsta
→ named site/app

crab://b3/<hash>.song
→ typed asset page

crab://name/sealobsta
→ explicit named site/app namespace

crab://site/sealobsta
→ explicit site namespace
```

## 3.3 Dot Notation

Dot notation is allowed for typed b3 asset pages.

Examples:

```text
<hash>.image
<hash>.post
<hash>.comment
<hash>.article
<hash>.song
<hash>.video
<hash>.page
```

Do not use `.com`, `.net`, `.org` semantics inside RON names.

RON names should be namespace records, not DNS clones.

## 3.4 Crab Resolution Flow

```text
User enters crab://b3/<hash>.song
→ extension parses link
→ extension selects active passport
→ extension requests/uses network capability
→ gateway/omnigate verifies capability
→ svc-index looks up asset manifest records for b3:<hash>
→ svc-storage/svc-edge fetches content and manifest bytes
→ omnigate hydrates asset page
→ browser displays song page
→ if paid action is required, wallet hold/capture/release flow runs
```

For named sites:

```text
User enters crab://sealobsta
→ extension parses as named site
→ gateway/omnigate resolves through svc-index
→ site manifest is fetched
→ site owner account and route policies are loaded
→ page/app is hydrated
→ paid access policy may run
```

## 3.5 Browser Fallbacks

Because browser custom protocol support varies, the extension should support:

```text
crab://sealobsta
crab://b3/<hash>.image
https://ron.link/resolve?crab=crab://sealobsta
https://ron.link/resolve?crab=crab://b3/<hash>.image
http://127.0.0.1:<micronode-port>/resolve?crab=...
```

The extension should support copy-paste even when the browser does not natively dispatch `crab://`.

## 3.6 Crab Link Parser Requirements

The parser must:

```text
accept canonical crab links
reject malformed hashes
normalize asset kind suffixes
lowercase b3 hex
reject uppercase b3 content IDs unless explicitly normalized before validation
reject unknown asset suffix unless policy allows experimental kinds
reject path traversal
reject embedded credentials
reject unsafe control characters
preserve query parameters in a normalized map
```

Supported initial asset kinds:

```text
image
post
comment
page
article
song
video
manifest
profile
app
site
```

Future asset kinds:

```text
album
playlist
podcast
book
document
dataset
model
game
mod
feed
thread
collection
```

---

## 4. RON Passport System

## 4.1 What a RON Passport Is

A RON Passport is the user’s persistent local RustyOnions identity credential.

It is similar in product feel to Phantom, Solflare, or MetaMask wallet identity, but architecturally it is not the same thing as the wallet.

```text
RON Passport = local identity / access credential
RON Wallet   = ROC value account
RON Cap      = short-lived scoped permission derived from passport authority
```

The passport lets the user prove:

```text
I am allowed to access the RustyOnions network.
I control this identity.
I may request short-lived capabilities.
I may access certain apps/sites/routes.
I may be linked to wallet account X.
I may use account X within caveated limits.
```

The passport must not directly contain raw wallet funds or ledger truth.

## 4.2 Passport Storage Model

Passports are stored locally on the user’s device.

Storage targets:

```text
browser extension storage
native app secure storage
OS keychain where available
encrypted local file vault for desktop
future mobile secure enclave / biometric unlock
```

A local passport vault may contain:

```text
main.ronpass
alt-anon-1.ronpass
alt-anon-2.ronpass
site-admin.ronpass
creator.ronpass
```

Each passport must be encrypted at rest.

Minimum vault properties:

```text
encrypted with user passphrase and/or device key
export/import support
backup phrase or recovery package eventually
lock timeout
manual lock
biometric unlock optional
never logs private key material
never syncs without explicit user action
```

## 4.3 Main Passport

The main passport is the user’s identity passport.

Use cases:

```text
network access
creator identity
site ownership
creator payouts
reputation
admin access
wallet binding
dashboard access
publishing content
managing site/app manifests
owning b3 asset manifests
claiming payout addresses
```

Suggested fields:

```json
{
  "passport_version": 1,
  "kind": "main",
  "subject": "ron:user:...",
  "display_name": "optional",
  "public_key": "key:...",
  "wallet_account": "t:default/u:<user>/w",
  "created_at": 0,
  "scopes_hint": [
    "network:access",
    "site:visit",
    "wallet:read",
    "wallet:hold",
    "profile:publish",
    "asset:claim",
    "asset:publish"
  ]
}
```

## 4.4 Alternate Anonymous Passports

Users should be allowed to create alternate local passports for anonymous or pseudonymous activity.

Alt passports are persistent local pseudonyms controlled by the user.

Use cases:

```text
anonymous browsing
pseudonymous comments
low-trust site visits
separate public persona
burner interactions
privacy-preserving exploration
```

Suggested fields:

```json
{
  "passport_version": 1,
  "kind": "alt-anon",
  "subject": "ron:anon:...",
  "public_key": "key:...",
  "wallet_account": "optional burner wallet",
  "created_at": 0,
  "scopes_hint": [
    "network:access",
    "site:visit",
    "comment:create"
  ]
}
```

Alt passports may have:

```text
lower rate limits
lower spending caps
no creator payout by default
no admin scopes by default
no profile metadata by default
optional burner wallet binding
```

## 4.5 Passport Does Not Equal Wallet

Critical rule:

```text
A passport may reference or bind to a wallet account.
A passport must not be the wallet itself.
```

The wallet remains the value boundary.

The passport proves identity/authorization.
The wallet enforces spend authority.
The ledger records truth.

Correct relationship:

```text
passport subject      = identity
wallet account        = ROC balance container
capability token      = short-lived action permission
ledger transaction    = committed economic truth
```

---

## 5. Passport + Wallet Binding

## 5.1 Binding Model

A passport can bind to one or more wallet accounts.

Example:

```toml
[passport]
kind = "main"
subject = "ron:user:alice"
public_key = "key:alice-main"

[wallet_binding]
account = "t:default/u:alice/w"
asset = "roc"
permissions = ["read", "hold", "receive"]
spend_cap_minor_per_day = 100000
```

Binding does not automatically allow unlimited spending.

Every spend/hold/capture path still requires:

```text
short-lived capability
scope check
account caveat check
asset caveat check
amount cap check
route/action policy check
wallet nonce/idempotency
ledger commit
receipt
```

## 5.2 Capabilities Derived from Passport

The extension should request short-lived capabilities from local or remote passport services.

Capability examples:

```text
network:access
site:visit
site:publish
asset:claim
asset:publish
asset:curate
comment:create
wallet:read
wallet:hold
wallet:capture
wallet:release
storage:put
content:view
admin:site
```

Wallet capability caveats:

```text
subject = ron:user:alice
account = t:default/u:alice/w
asset = roc
max_amount_minor = 1000
audience = svc-wallet
expires_at = short TTL
route = /v1/hold
site = optional site id
asset_cid = optional content id
action = paid_content_view
```

Asset capability caveats:

```text
subject = ron:user:alice
asset_cid = b3:<64 lowercase hex>
asset_kind = song
action = asset:claim
expires_at = short TTL
signature_required = true
```

---

## 6. Browser Extension Instead of Custom Browser

## 6.1 Product Decision

Do not build a full RustyOnions browser for MVP.

A custom browser would be expensive and slow. The MVP should be a browser extension that provides:

```text
passport vault
wallet binding
crab:// link resolver
gateway selector
local Micronode connector
paid action prompts
receipt viewer
identity/alt-passport switcher
asset page viewer helpers
```

This lets users surf RON through existing browsers:

```text
Chrome
Brave
Firefox
Safari eventually
```

## 6.2 Extension Responsibilities

The extension should own:

```text
local passport vault
main/alt passport selector
wallet connection/binding UI
crab link input bar
crab link detection where possible
capability request/signing
payment approval prompts
spending caps per site
spending caps per asset kind
receipt display
gateway/Micronode selection
basic phishing/safety warnings
```

The extension should not own:

```text
ledger truth
wallet balance truth
storage truth
DHT routing tables
overlay sessions
reward computation
accounting snapshots
```

It talks to local or remote RON services:

```text
http://127.0.0.1:<micronode-port>
https://<trusted-ron-gateway>
```

## 6.3 Extension MVP

Minimum version:

```text
1. Create/import local main passport.
2. Create/import alt anonymous passport.
3. Select active passport.
4. Configure gateway:
   - local Micronode
   - remote gateway
5. Enter crab:// link.
6. Resolve crab link through gateway/omnigate.
7. Load resolved RON content in browser.
8. Show b3 asset page for typed asset links.
9. Prompt before paid action.
10. Call wallet hold.
11. Display receipt after capture/release.
```

## 6.4 Extension Security

Rules:

```text
private passport keys never leave device
extension never logs bearer tokens
extension never silently spends ROC
paid actions require prompt unless user explicitly sets cap
caps are short-lived
per-site spend caps are local and visible
per-asset-kind spend caps are local and visible
alt passport mode must be obvious in UI
main identity mode must be obvious in UI
content hash and owner/payout address should be visible on asset pages
```

---

## 7. ROC Economics TOML

## 7.1 Purpose

WEB3 2.0 needs a central TOML policy file for pricing and payout percentages.

Recommended path:

```text
configs/roc-economics.toml
```

This file defines:

```text
action pricing
hold estimation
payout splits
recipient roles
treasury share
reward pool share
burn share, if any
rounding behavior
remainder handling
per-action caps
per-asset-kind policies
site visit policy
content view policy
storage policy
creator payouts
curator payouts
```

## 7.2 Use Basis Points, Not Floats

Never use floats for payout percentages.

Use basis points:

```text
10000 bps = 100%
2500 bps  = 25%
1 bps     = 0.01%
```

## 7.3 Example TOML

```toml
version = 1
unit = "roc_minor"
default_asset = "roc"
rounding = "floor"
remainder_sink = "treasury"

[accounts]
treasury = "t:default/treasury"
reward_pool = "t:default/reward_pool"
burn = "t:default/burn"

[roles]
site_owner = "dynamic:site_owner"
content_owner = "dynamic:content_owner"
storage_provider = "dynamic:storage_provider"
edge_provider = "dynamic:edge_provider"
app_owner = "dynamic:app_owner"
curator_pool = "dynamic:curator_pool"

[limits]
default_daily_spend_cap_minor = 100000
default_single_action_cap_minor = 10000
anonymous_daily_spend_cap_minor = 1000

[actions.paid_storage_put]
enabled = true
pricing_kind = "per_byte_plus_minimum"
price_per_byte_minor = 1
minimum_charge_minor = 70
max_hold_multiplier_bps = 12000

[[actions.paid_storage_put.splits]]
to = "storage_provider"
bps = 8000

[[actions.paid_storage_put.splits]]
to = "treasury"
bps = 1500

[[actions.paid_storage_put.splits]]
to = "reward_pool"
bps = 500

[actions.paid_content_view]
enabled = true
pricing_kind = "flat_plus_egress"
flat_charge_minor = 25
price_per_kib_served_minor = 1
minimum_charge_minor = 25
max_hold_multiplier_bps = 12000

[[actions.paid_content_view.splits]]
to = "content_owner"
bps = 7000

[[actions.paid_content_view.splits]]
to = "edge_provider"
bps = 1500

[[actions.paid_content_view.splits]]
to = "treasury"
bps = 1000

[[actions.paid_content_view.splits]]
to = "reward_pool"
bps = 500

[actions.paid_song_play]
enabled = true
pricing_kind = "flat_plus_stream_bytes"
flat_charge_minor = 10
price_per_kib_served_minor = 1
minimum_charge_minor = 10
max_hold_multiplier_bps = 12000

[[actions.paid_song_play.splits]]
to = "content_owner"
bps = 7000

[[actions.paid_song_play.splits]]
to = "edge_provider"
bps = 1500

[[actions.paid_song_play.splits]]
to = "curator_pool"
bps = 500

[[actions.paid_song_play.splits]]
to = "treasury"
bps = 500

[[actions.paid_song_play.splits]]
to = "reward_pool"
bps = 500

[actions.site_visit]
enabled = false
pricing_kind = "flat"
flat_charge_minor = 1
minimum_charge_minor = 1
max_hold_multiplier_bps = 10000

[[actions.site_visit.splits]]
to = "site_owner"
bps = 8000

[[actions.site_visit.splits]]
to = "treasury"
bps = 2000
```

## 7.4 Validation Rules

The validator must reject:

```text
unknown action kind
unknown split role
split bps sum != 10000
negative amount
float amount
float percentage
zero minimum where action requires charge
missing remainder sink
invalid account string
disabled action with active enforcement
max_hold_multiplier_bps below 10000
max_hold_multiplier_bps above configured ceiling
```

Validator must enforce:

```text
integer-only amounts
basis-point percentages
deterministic rounding
stable action names
stable role names
explicit remainder sink
explicit enabled/disabled
known asset kinds
known payout role semantics
```

## 7.5 First Actions

Priority order:

```text
paid_storage_put
paid_storage_pin
paid_content_view
paid_song_play
paid_video_view
paid_article_read
paid_app_route
site_visit
subscription_pass
creator_tip
reward_pool_funding
```

`site_visit` should remain disabled by default at first.

---

## 8. Paid Access Flows

## 8.1 General Rule

All paid operations follow:

```text
estimate → hold → serve/perform → capture → release → receipt
```

No silent draining.

No “charge after the fact” without prior reservation.

## 8.2 Paid Content View Flow

```text
User opens crab://b3/<hash>.article
→ extension resolves active passport
→ gateway/omnigate checks passport capability
→ asset manifest declares owner and payout address
→ route/economics policy says paid_content_view required
→ economics config estimates max charge
→ extension prompts user:
   “This action may cost up to X ROC.”
→ wallet hold reserves max estimate
→ content is served
→ actual bytes/time/access cost is calculated
→ wallet capture pays split recipients
→ wallet release refunds remainder
→ user gets receipt
→ accounting records usage
```

## 8.3 Paid Song Play Flow

```text
User opens crab://b3/<hash>.song
→ asset page loads song manifest
→ manifest identifies artist/content owner payout account
→ policy says paid_song_play required
→ extension prompts or applies user’s pre-approved cap
→ wallet hold reserves ROC
→ song stream begins
→ bytes/time listened are metered
→ wallet capture pays content_owner / edge_provider / curator_pool / treasury / reward_pool
→ wallet release refunds unused estimate
→ receipt links to song asset CID
```

## 8.4 Site Visit Flow

Site visits should be opt-in and capped.

```text
User opens crab://sealobsta
→ gateway checks site manifest and policy
→ if site_visit enabled:
   estimate tiny max charge
   prompt or apply user’s pre-approved site cap
   hold ROC
   serve site
   capture small fee
   release remainder
```

Default:

```text
site_visit.enabled = false
```

Reason:

```text
Charging for every visit is powerful but sensitive.
Users must see and control it.
```

## 8.5 Payout Recipients

Possible dynamic recipients:

```text
site_owner
app_owner
content_owner
content_creator
artist
album_owner
storage_provider
edge_provider
gateway_operator
reward_pool
treasury
burn
referrer
curator_pool
moderator_pool
```

## 8.6 Payout Settlement

The MVP can use a simple capture recipient.

WEB3 2.0 target should support split settlement:

```text
capture 100 ROC from escrow
→ 70 ROC content_owner
→ 15 ROC edge_provider
→ 10 ROC treasury
→ 5 ROC reward_pool
```

Implementation options:

```text
Option A: wallet capture supports multiple recipients directly.
Option B: capture to settlement account, then wallet split-transfer.
Option C: capture to service account, rewarder later distributes.
```

Recommended phases:

```text
Phase 1: capture to service/provider account.
Phase 2: capture supports deterministic split recipients.
Phase 3: rewarder reconciles broader contribution payouts.
```

---

## 9. Service Responsibilities in WEB3 2.0

## 9.1 Browser Extension

Owns:

```text
local passport vault
passport switching
crab link UI
b3 asset page UX helpers
payment prompts
capability request/signing
receipt viewing
gateway selection
spending caps
```

Does not own:

```text
ledger truth
wallet mutation rules
accounting truth
reward computation
storage truth
```

## 9.2 svc-passport

Owns:

```text
passport issuance/registration
passport verification support
capability minting
capability attenuation
revocation
short TTL tokens
network access proof
identity/passport public metadata
asset claim identity binding
```

Future endpoints:

```text
POST /v1/passport/create
POST /v1/passport/import/register
POST /v1/cap/mint
POST /v1/cap/attenuate
POST /v1/cap/revoke
GET  /v1/passport/{subject}
GET  /v1/passport/{subject}/public
```

## 9.3 ron-auth

Owns:

```text
capability verification
caveat evaluation
scope matching
TTL checks
audience checks
account caveat checks
asset caveat checks
policy adapter hooks
```

## 9.4 svc-wallet

Owns:

```text
balance reads
receipt lookup
issue
transfer
burn
hold
capture
release
split settlement
spend caps
idempotency
nonce checking
account/asset caveats
```

## 9.5 svc-gateway / omnigate

Owns:

```text
passport/capability enforcement at ingress
crab link resolution entrypoint
paid route enforcement
site/app route policy
asset page hydration
pricing estimate
hold orchestration
capture/release orchestration
request metering
```

## 9.6 svc-index

Owns:

```text
crab name resolution
site/app manifest lookup
b3 asset manifest lookup
owner account lookup
route policy pointer lookup
content id lookup
provider lookup
curator metadata lookup
latest manifest pointer lookup
```

## 9.7 ron-naming

Owns:

```text
content ID schemas
crab link DTOs if added there
asset kind schemas
name hygiene
address hygiene
wire-safe naming/address types
```

## 9.8 svc-storage / svc-edge

Owns:

```text
CAS bytes
paid storage writes
paid or metered reads eventually
byte-range serving
edge/cache metering
provider attribution
raw object verification
```

## 9.9 ron-accounting

Owns:

```text
usage events
bytes stored
bytes served
request_ok
pin_seconds
site visits
content views
song plays
video views
article reads
session seconds
provider contribution snapshots
reward snapshots
```

## 9.10 svc-rewarder

Owns:

```text
reward epoch compute
provider reward payouts
curator reward payouts
anti-gaming caps
reward manifest
wallet issue intents
reward pool distribution
```

## 9.11 ron-ledger

Owns:

```text
append-only truth
replay
balances
receipts
audit roots
idempotent committed transaction identity
```

---

## 10. New Config Files

WEB3 2.0 should add these config artifacts:

```text
configs/roc-economics.toml
configs/passport.policy.toml
configs/crab-resolver.toml
configs/extension-defaults.toml
configs/asset-kinds.toml
```

## 10.1 roc-economics.toml

Defines pricing, split percentages, caps, sinks, and action policies.

## 10.2 passport.policy.toml

Defines what main and alt passports can do.

Example:

```toml
[main]
network_access = true
can_publish_sites = true
can_claim_assets = true
can_bind_wallet = true
default_daily_spend_cap_minor = 100000

[alt_anonymous]
network_access = true
can_publish_sites = false
can_claim_assets = false
can_bind_wallet = true
default_daily_spend_cap_minor = 1000
max_request_rate_per_minute = 60
```

## 10.3 crab-resolver.toml

Defines local and remote resolver behavior.

```toml
default_gateway = "http://127.0.0.1:8080"
allow_remote_gateways = true
allow_custom_protocol = true
fallback_web_resolver = "https://ron.link/resolve"
```

## 10.4 extension-defaults.toml

Defines UX safety defaults.

```toml
prompt_for_every_paid_action = true
allow_preapproved_site_caps = true
default_site_cap_minor_per_day = 100
default_alt_site_cap_minor_per_day = 10
lock_after_seconds = 900
show_full_b3_hash_by_default = false
show_payout_address_on_asset_page = true
```

## 10.5 asset-kinds.toml

Defines known asset page suffixes.

```toml
[asset_kinds.image]
mime_prefixes = ["image/"]
viewer = "image"
paid_action = "paid_content_view"

[asset_kinds.song]
mime_prefixes = ["audio/"]
viewer = "audio"
paid_action = "paid_song_play"

[asset_kinds.video]
mime_prefixes = ["video/"]
viewer = "video"
paid_action = "paid_video_view"

[asset_kinds.article]
mime_prefixes = ["text/markdown", "text/html", "application/json"]
viewer = "article"
paid_action = "paid_article_read"

[asset_kinds.comment]
mime_prefixes = ["text/plain", "application/json"]
viewer = "comment"
paid_action = "none"
```

---

## 11. New Data Models

## 11.1 RonPassport

```rust
pub struct RonPassport {
    pub version: u32,
    pub kind: PassportKind,
    pub subject: String,
    pub public_key_id: String,
    pub wallet_bindings: Vec<WalletBinding>,
    pub created_at_ms: u64,
    pub label: Option<String>,
}
```

## 11.2 PassportKind

```rust
pub enum PassportKind {
    Main,
    AltAnonymous,
    SiteAdmin,
    Creator,
}
```

## 11.3 WalletBinding

```rust
pub struct WalletBinding {
    pub account: String,
    pub asset: String,
    pub permissions: Vec<WalletPermission>,
    pub spend_caps: SpendCaps,
}
```

## 11.4 CrabLink

```rust
pub struct CrabLink {
    pub scheme: String,
    pub kind: CrabLinkKind,
    pub target: String,
    pub asset_kind: Option<String>,
    pub query: BTreeMap<String, String>,
}
```

## 11.5 CrabLinkKind

```rust
pub enum CrabLinkKind {
    NamedSite,
    ExplicitSite,
    ExplicitApp,
    ExplicitName,
    B3Asset,
    B3Raw,
    PassportProfile,
    PaymentTarget,
}
```

## 11.6 AssetKind

```rust
pub enum AssetKind {
    Image,
    Post,
    Comment,
    Page,
    Article,
    Song,
    Video,
    Manifest,
    Profile,
    App,
    Site,
}
```

## 11.7 AssetManifest

```rust
pub struct AssetManifest {
    pub version: u32,
    pub asset_cid: String,
    pub asset_kind: AssetKind,
    pub manifest_cid: Option<String>,
    pub owner: AssetOwner,
    pub payout_roles: BTreeMap<String, String>,
    pub metadata: AssetMetadata,
    pub provenance: AssetProvenance,
}
```

## 11.8 AssetOwner

```rust
pub struct AssetOwner {
    pub passport_subject: String,
    pub wallet_account: String,
    pub ownership_claim_signature: String,
}
```

## 11.9 AssetMetadata

```rust
pub struct AssetMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub language: Option<String>,
    pub license: Option<String>,
    pub content_warning: Option<String>,
}
```

## 11.10 EconomicsPolicy

```rust
pub struct EconomicsPolicy {
    pub version: u32,
    pub unit: String,
    pub default_asset: String,
    pub rounding: RoundingMode,
    pub remainder_sink: String,
    pub actions: BTreeMap<String, ActionEconomics>,
}
```

## 11.11 ActionEconomics

```rust
pub struct ActionEconomics {
    pub enabled: bool,
    pub pricing_kind: PricingKind,
    pub minimum_charge_minor: u128,
    pub max_hold_multiplier_bps: u32,
    pub splits: Vec<PayoutSplit>,
}
```

## 11.12 PayoutSplit

```rust
pub struct PayoutSplit {
    pub to: PayoutDestination,
    pub bps: u16,
}
```

---

## 12. New Metrics

## 12.1 Passport Metrics

```text
passport_created_total{kind}
passport_cap_minted_total{kind,scope}
passport_cap_rejected_total{reason}
passport_alt_selected_total
passport_vault_unlock_total{result}
```

## 12.2 Crab Link Metrics

```text
crab_resolve_total{kind,result}
crab_resolve_latency_seconds{kind}
crab_unknown_scheme_total
crab_gateway_failover_total
```

## 12.3 b3 Asset Page Metrics

```text
b3_asset_page_resolve_total{asset_kind,result}
b3_asset_manifest_lookup_total{asset_kind,result}
b3_asset_manifest_validation_reject_total{reason}
b3_asset_owner_claim_total{asset_kind,result}
b3_asset_page_render_seconds{asset_kind}
```

## 12.4 Paid Access Metrics

```text
paid_access_estimate_total{action,result}
paid_access_hold_total{action,result}
paid_access_capture_total{action,result}
paid_access_release_total{action,result}
paid_access_user_cancel_total{action}
paid_access_amount_minor_total{action,destination}
```

## 12.5 Economics Policy Metrics

```text
roc_economics_reload_total{result}
roc_economics_action_enabled{action}
roc_economics_validation_reject_total{reason}
roc_economics_split_bps{action,destination}
```

---

## 13. WEB3 2.0 Milestones

## M1 — Economics Policy Foundation

Deliverables:

```text
configs/roc-economics.toml
economics parser
economics validator
basis-point split checker
unit tests for valid/invalid splits
paid_storage_put policy wired to existing storage pricing path
```

Exit gate:

```text
cargo test proves splits sum to 10000 bps and invalid configs reject.
```

## M2 — b3 Asset Page Foundation

Deliverables:

```text
asset kind schema
asset manifest DTO
asset ownership claim DTO
asset payout role mapping
b3 asset page link parser
tests for crab://b3/<hash>.image/song/article/video
manifest validation tests
ownership claim validation tests
```

Exit gate:

```text
Every uploaded test asset can produce:
1. b3 content ID
2. asset manifest
3. typed crab asset page address
4. owner payout account mapping
```

## M3 — Passport Model Foundation

Deliverables:

```text
RonPassport DTO
PassportKind
WalletBinding
local vault abstraction trait
main passport tests
alt passport tests
wallet binding tests
capability claims extended with passport subject/account/asset caveats
```

Exit gate:

```text
main and alt passports can be created locally and validated without network.
```

## M4 — Crab Link Parser and Resolver

Deliverables:

```text
crab:// parser
supported link kinds
b3 asset suffix parser
fallback web resolver shape
tests for valid/invalid links
gateway resolution request DTO
```

Exit gate:

```text
crab://sealobsta, crab://site/sealobsta, and crab://b3/<hash>.song parse deterministically.
```

## M5 — Browser Extension MVP Spec

Deliverables:

```text
extension architecture doc
passport vault UI spec
crab bar UI spec
b3 asset page UI spec
gateway selector UI spec
payment prompt UI spec
receipt viewer UI spec
```

Exit gate:

```text
we can begin extension implementation without unresolved product ambiguity.
```

## M6 — Paid Content View Prototype

Deliverables:

```text
paid_content_view action in roc-economics.toml
gateway/omnigate estimate path
wallet hold request
content serve path
capture/release plan
accounting events:
  content_view
  bytes_served
  request_ok
```

Exit gate:

```text
paid content route rejects without hold, succeeds with wallet hold, records usage, and pays configured recipient.
```

## M7 — Site Owner / Asset Owner Payout Path

Deliverables:

```text
site manifest includes owner account
asset manifest includes owner account
svc-index exposes owner account to gateway/omnigate
capture pays site_owner/content_owner based on economics split
receipt shows payout destinations
```

Exit gate:

```text
visiting paid content can transfer ROC to content owner/site owner using configured split policy.
```

## M8 — Passport + Wallet Integration

Deliverables:

```text
passport-bound wallet account caveats
wallet rejects wrong passport/account
wallet rejects wrong asset
extension requests wallet:hold cap
svc-wallet validates real ron-auth/svc-passport token
```

Exit gate:

```text
main passport can pay from its wallet; alt passport cannot spend from main wallet unless explicitly authorized.
```

## M9 — Dashboard Visibility

Deliverables:

```text
passport summary
wallet summary
recent receipts
b3 asset pages
asset owner earnings
paid access totals
site owner earnings
storage provider earnings
curator earnings
reward pool totals
treasury totals
```

Exit gate:

```text
admin/dashboard can show economy movement from paid content, b3 asset pages, and paid storage.
```

---

## 14. Implementation Priority for Next Session

Start here:

```text
1. Add configs/roc-economics.toml.
2. Add economics parser/validator module.
3. Validate bps splits sum to 10000.
4. Add paid_storage_put, paid_content_view, paid_song_play, and site_visit actions.
5. Add unit tests for invalid split sums, unknown destinations, float rejection, and disabled action behavior.
6. Add asset-kind/addressing design as DTOs/tests after economics foundation is green.
```

Recommended first implementation location:

```text
ron-policy under a dedicated economics module
```

Suggested paths:

```text
crates/ron-policy/src/economics/mod.rs
crates/ron-policy/src/economics/types.rs
crates/ron-policy/src/economics/load.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/tests/economics_policy.rs
configs/roc-economics.toml
```

Then begin b3 asset page foundation:

```text
crates/ron-naming/src/crab.rs
crates/ron-naming/src/asset.rs
crates/ron-naming/tests/crab_links.rs
crates/ron-naming/tests/asset_manifest.rs
```

Do not add a new crate unless absolutely necessary.

---

## 15. Definition of Done for WEB3 2.0

WEB3 2.0 is done when:

```text
1. User can create/import a local main RON Passport.
2. User can create/select an alt anonymous passport.
3. Passport can bind to wallet account permissions.
4. User can enter crab:// links in extension or dev tool.
5. crab:// named sites resolve through gateway/omnigate.
6. crab://b3/<hash>.<kind> asset pages resolve and render manifest data.
7. Every uploaded image/post/comment/page/song/video receives a BLAKE3 b3 content ID.
8. Every uploaded asset can have an asset manifest with ownership and payout address.
9. Paid content routes require wallet hold.
10. Capture pays configured recipient splits from roc-economics.toml.
11. Site owners can earn ROC from visits/content.
12. Asset owners can earn ROC from images/posts/songs/videos/articles.
13. Storage providers can earn ROC from storage/pinning.
14. Users see receipts for paid actions.
15. Dashboard/admin shows paid access, asset ownership, and payout activity.
16. All core WEB3 invariants remain green.
```

---

## 16. Strategic Product Summary

WEB3 2.0 turns RON from an internal economy proof into a user-facing network experience.

The product story becomes:

```text
Install RON extension.
Create your RON Passport.
Use your main identity or an anonymous alt.
Paste crab:// links.
Browse RON sites.
Open b3 asset pages.
See who owns each image/song/article/video/post/comment.
Approve paid access with ROC.
Site owners, creators, curators, and providers get paid.
Everything is hashed, metered, receipted, and auditable.
```

This is the right path because it avoids building a full browser while still creating a native RON identity, content-addressing, access, and payment experience.

The killer RustyOnions feature is:

```text
Every digital object becomes a b3-addressed page with ownership, provenance, metadata, and payout routing.
```

That makes RustyOnions feel less like a normal website host and more like a content-addressed web economy.

---

# End of WEB3_2_0.md

```
