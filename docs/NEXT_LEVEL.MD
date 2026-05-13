
# NEXT_LEVEL.MD — RON Asset Primitives, Reference-Graph Sites, Passports, Reputation, and Creator Economy Routing

RO:WHAT — Defines the next RustyOnions WEB3_2 product layer after the paid image + site + crab-image embed proof.
RO:WHY — Turns the proven b3/crab/ROC pipeline into reusable creator primitives, identity/display primitives, reputation/moderation surfaces, and reference-graph sites without breaking crate boundaries.
RO:INTERACTS — ron-proto, ron-naming, ron-app-sdk, svc-gateway, omnigate, svc-index, svc-storage, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, ron-policy, svc-registry, svc-dht, svc-edge, svc-passport, ron-auth, CrabLink.
RO:INVARIANTS — ROC remains internal; b3 hashes are canonical; sites store references; wallet remains mutation front-door; storage stores bytes only; passports are identities/capability records, not wallets and not merely assets; public profile manifests may be b3-addressed; no arbitrary executable linked code.
RO:METRICS — Future publish/view/serve/storage/moderation/reputation events flow through service metrics and ron-accounting where appropriate.
RO:CONFIG — Future economics, feature gates, hosting placement, moderation, and reputation rules live in configs/roc-economics.toml and policy configs.
RO:SECURITY — Feature-gate risky adoption layers; explicit user confirmation for paid actions; no silent spend; no clearnet attribution claims without signed/verified ownership; no public main↔alt linkage by default; no privacy claims without a real protocol.
RO:TEST — Extend product smoke, CrabLink smoke, route-contract tests, DTO tests, paid-flow tests, passport tests, and moderation/reputation tests batch by batch.

---

# 0. Purpose

NEXT_LEVEL is the product blueprint for turning RustyOnions from:

```text
paid image upload
+ crab://site proof
+ <crab-image> reference embed proof
```

into:

```text
content-addressed creator web
+ reusable asset primitives
+ reference-graph sites
+ manifest-backed attribution
+ RON Passports with @usernames
+ reputation and moderation summaries
+ anonymous/pseudonymous alt passports
+ privacy-honest alt-wallet handling
+ ROC payout routing
+ decentralized/dedicated/self-hosted placement modes
+ SDK/CrabLink developer UX
```

This blueprint must **not** skip ahead of the current work.

The current proven checkpoint is:

```text
1. Paid image asset path works.
2. Site creation/open flow works.
3. CrabLink renders named crab:// sites.
4. CrabLink can render a site root HTML document.
5. CrabLink can render <crab-image src="crab://<hash>.image">.
6. The site root stores only an image reference; the image remains its own b3-backed asset.
7. The guard prevents accidentally using an image CID as the site root CID.
8. crab://thedustyonion6 proves the first working reference-graph embed primitive.
```

NEXT_LEVEL now begins from this concrete proof.

---

# 1. Strategic thesis

RustyOnions sites should not behave like traditional centralized databases that own all user content.

A RON-native site should own:

```text
route maps
layout
moderation policy
payout policy
reference graphs
site identity
site manifest
site revision history
```

A RON-native site should not need to permanently own every post, comment, image, article, song, video, manifest, or page in its own database.

Instead:

```text
Every user-created content object becomes a b3-addressed asset.
The site stores references to those assets.
The asset manifest stores attribution, ownership, payout, provenance, storage/provider data, and receipts.
The passport system stores identity/capability records.
A passport may publish a public b3-addressed profile/passport manifest for discovery.
The public profile manifest may expose @username, profile metadata, public asset catalogue pointers, reputation summary, moderation summary, and site-scoped public roles.
Private keys, private alt mappings, spend authority, and local permissions stay private/local or service-internal.
```

Core model:

```text
Site = manifest + routes + policies + references + revisions
Post = b3 asset + manifest
Comment = b3 asset + manifest
Article = b3 asset + manifest
Image = b3 asset + manifest
Video = b3 asset + manifest + range/streaming plan
Music = b3 asset + manifest + range/streaming plan
Thread = reference collection / manifest
Profile = passport-linked read-only page / public manifest
Passport = identity/capability record + optional public profile manifest
Alt Passport = separate pseudonymous identity + separate wallet/account + no public main linkage
```

Example site reference graph:

```text
crab://myforum/thread/123
  ├── crab://<64hex>.post
  ├── crab://<64hex>.comment
  ├── crab://<64hex>.comment
  └── crab://<64hex>.image
```

Example HTML embed primitive:

```html
<crab-image
  src="crab://2e24f045f01a1bc77c57a94d622365e6b291936fcdd3ae64b45b0578e99c2058.image"
  alt="RustyOnions b3-backed image"
></crab-image>
```

This is the “site as reference graph” architecture.

---

# 2. Current proven foundation

Do not start from theory.

The current proven foundation is:

```text
CrabLink / script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ b3 asset page / site page / embedded referenced asset
```

Already proven or in current active handoff:

```text
GET  /b3/<hash>.image
GET  /crab/resolve?url=crab://<hash>.image
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
POST /wallet/hold
CrabLink crab://site creation flow
CrabLink named site rendering
CrabLink <crab-image> embedded reference rendering
CrabLink image-root guard
```

Current successful product pattern:

```text
prepare
→ explicit ROC hold
→ paid upload/create
→ raw bytes stored by b3 CID
→ manifest stored by b3 CID
→ index pointer stored
→ crab:// URL returned
→ hydrated page resolves in CrabLink
```

Current successful site-reference pattern:

```text
HTML root stored as b3 object
→ site manifest points at HTML root CID
→ HTML root contains <crab-image src="crab://<hash>.image">
→ CrabLink detects crab-image
→ CrabLink fetches image bytes through gateway /o/b3:<hash>
→ image renders in sandboxed site page
```

NEXT_LEVEL must generalize these patterns rather than inventing parallel systems.

---

# 3. Non-negotiable invariants

These are hard rules.

```text
- ROC remains internal only.
- No ROX.
- No Solana.
- No external settlement.
- No staking/liquidity/exchange-facing logic.
- No floating-point money math.
- All ROC amounts use integer minor units.
- All percentages use integer basis points.
- Wallet mutation must go through svc-wallet.
- Durable economic truth remains ron-ledger.
- ron-accounting remains usage/metering snapshots, not balance truth.
- svc-rewarder computes deterministic payout plans but does not directly mutate ledger.
- Paid operations use hold → capture → release.
- Wallet actions require nonce + idempotency.
- No silent ROC spending.
- No fake receipts.
- No direct ledger mutation by svc-gateway, omnigate, svc-storage, svc-index, CrabLink, SDKs, passport logic, moderation logic, or reputation logic.
- b3:<64 lowercase hex> is canonical internal content ID.
- crab://<64 lowercase hex>.<kind> is canonical public typed asset URL.
- crab://<site_name> is a human pointer.
- crab://@username is a human identity/discovery pointer.
- Names are human pointers; b3 hashes are canonical.
- passport_subject remains canonical identity; @username is a human pointer.
- Storage stores bytes by CID only.
- Index stores pointers and lookup metadata.
- Omnigate hydrates views but owns no durable truth.
- Gateway exposes public routes and enforces ingress limits.
- CrabLink remains gateway-only.
- Passports are identities/capability records, not wallets.
- Public passport/profile manifests may be b3-addressed, but private passport material is not “just an asset.”
- Wallet-linked passport authority must be explicit, caveated, scoped, and revocable.
- Main passports may create sites when policy allows.
- Anonymous/pseudonymous alts may browse/surf/comment, but must not create sites by default.
- Alt passports must not automatically inherit main-passport wallet authority.
- Alt ROC balances/accounts are separate from main passport balances/accounts.
- Public passport/profile manifests must not expose private alt↔main linkage.
- Direct alt→main transfers are traceable unless a real privacy protocol exists.
- No arbitrary executable code from crab links.
- Clearnet crossover is feature-gated until attribution rules are safe.
```

---

# 4. Definitions

## 4.1 Asset primitive

An asset primitive is a safe declarative builder and route contract for publishing a typed b3-backed asset.

Initial asset primitives:

```text
.image      proven
.site       proven
.crab-image embed primitive proven in CrabLink
.post       next text primitive
.comment    second text primitive
.article    third text primitive
.music      later media primitive
.video      later media primitive
.stream     later live/segmented primitive
.page       later
.profile    later public profile page/manifest
.thread     later
```

An asset primitive creates:

```text
1. canonical content bytes
2. b3 asset CID
3. asset manifest
4. b3 manifest CID
5. index pointer
6. crab:// URL
7. optional wallet receipt reference
8. hydrated asset page
9. optional embed renderer
```

## 4.2 Embed primitive

An embed primitive is a trusted CrabLink/client renderer for a declarative tag.

Current proof:

```html
<crab-image src="crab://<64hex>.image"></crab-image>
```

Future possible tags:

```html
<crab-image src="crab://<64hex>.image"></crab-image>
<crab-article src="crab://<64hex>.article"></crab-article>
<crab-post src="crab://<64hex>.post"></crab-post>
<crab-comment src="crab://<64hex>.comment"></crab-comment>
<crab-video src="crab://<64hex>.video"></crab-video>
<crab-audio src="crab://<64hex>.music"></crab-audio>
```

Rules:

```text
- Embed tags are declarative.
- Embed tags do not execute remote JS.
- Embed renderers are trusted CrabLink/client code.
- Embed resolution goes through svc-gateway only.
- Embed content must retain its own asset manifest and ownership/payout metadata.
- Sites reference assets; sites do not own asset bytes.
```

## 4.3 Interaction primitive

An interaction primitive is an action around an asset.

Examples:

```text
view
reply
quote
repost
like
tip
pin
moderate
curate
subscribe
serve
store
```

Important rule:

```text
Not every interaction should become a full b3 asset.
```

MVP distinction:

```text
Posts/comments/articles/images/videos/music = b3 assets.
Views/serves/storage-days = accounting events.
Likes/reactions = feature-gated until event-vs-asset model is settled.
Moderation actions = signed/accounted events, not necessarily public b3 assets.
```

This prevents high-volume tiny actions from overwhelming storage/index before the model is mature.

---

# 5. Feature gates

NEXT_LEVEL should be explicitly feature-gated so the project does not become spaghetti.

## 5.1 Core gates

```text
next_level_assets_text
  Enables .post, .comment, .article DTOs/routes/rendering.

next_level_assets_media_lite
  Enables MVP .music/.video descriptors and small-file tests only.
  Does not imply production video streaming.

next_level_media_streaming
  Enables range/chunk/edge-aware audio/video serving.
  Default: OFF until storage/edge route contracts are ready.

next_level_reference_graph
  Enables site/thread/page reference maps.

next_level_site_revisions
  Enables owner-authorized site edits and revision history.

next_level_paid_views
  Enables paid view prepare/capture flow.

next_level_provider_rewards
  Enables provider storage/egress payout planning.

next_level_hosting_placement
  Enables automatic/dedicated/self-hosted placement policy surfaces.

next_level_passport_usernames
  Enables @username claim/validation for main passports.

next_level_passport_public_profiles
  Enables read-only public passport/profile manifests and crab://@username resolution.

next_level_passport_alts
  Enables local alternate anonymous/pseudonymous passports.

next_level_reputation
  Enables read-only reputation/moderation score display.

next_level_site_moderation_roles
  Enables site-designated moderator roles and permissions.

next_level_alt_private_withdrawal
  Enables future privacy-preserving alt→main ROC movement.
  Default: OFF.
  Research/security gate required.

next_level_clearnet_crossover
  Enables clearnet embed/import helpers.
  Default: OFF.

next_level_free_impression_rewards
  Enables free-view reward pool experiments.
  Default: OFF.

next_level_executable_apps
  Enables executable/sandboxed app/module concepts.
  Default: OFF and out of scope for this blueprint.
```

## 5.2 Clearnet crossover gate

Clearnet crossover is valuable but dangerous.

Problems:

```text
- Attribution for old clearnet content may be unverifiable.
- A site might import content it does not own.
- A user might claim authorship of scraped/copied material.
- Existing websites may have mixed authorship.
- Copyright and takedown implications are harder than native RON content.
```

Therefore:

```text
next_level_clearnet_crossover = false by default
```

When enabled, it must support only safe modes first:

```text
Mode A: reference-only embed
  Old site stores crab:// refs created by real RON users.

Mode B: owner-declared import
  Site owner imports their own content with a self-attested claim.

Mode C: signed migration
  Existing user logs in and signs a RON ownership claim.

Mode D: unverified mirror
  Content is marked as mirror/unverified and should not receive creator payouts by default.
```

Clearnet-imported content must not automatically receive creator payouts unless ownership is verified.

---

# 6. Passport, @username, reputation, and moderation model

## 6.1 Main passport

A main passport is the user’s primary RON identity/capability record.

A main passport should eventually contain or reference:

```text
passport_subject
public_key_id
@username
display_name optional
created_at
capability_summary
wallet_links
site_creation_permission
asset_catalogue_pointer
reputation_summary
moderation_summary
site_roles
profile_manifest_cid optional
public_profile_url optional
```

Main passport rule:

```text
Main passports may create sites when policy allows.
```

Main passports should eventually resolve to a read-only profile/passport page:

```text
crab://@username
```

The site creator chip on site pages should eventually show:

```text
site creator: @username  reputation:<score>
```

Current placeholder:

```text
site creator: @username
```

Target:

```text
site creator: @alice  rep 842  mod 77
```

Exact UI can evolve, but the public site page should help users vet who created the site.

## 6.2 @username claims

When a user creates their first main passport, CrabLink should let them choose a unique `@username`.

Rules:

```text
- @username belongs to a main passport.
- @username must be normalized deterministically.
- @username must be unique.
- @username must reject confusing/reserved/system names.
- @username should be human-friendly but not the canonical identity key.
- passport_subject remains canonical identity.
- @username is a human pointer/discovery handle.
```

Suggested validation:

```text
allowed = lowercase a-z, 0-9, underscore, hyphen
must start with letter or number
min length = 3
max length = 32
reserved = admin, root, ron, crab, system, wallet, passport, site, image, video, music, post, comment, article, mod, moderator
```

Crate ownership:

```text
ron-naming:
  pure @username normalization/parsing only

svc-passport:
  username claim persistence and uniqueness

omnigate/svc-gateway:
  public route/hydration/proxy

CrabLink:
  first-passport creation UX
```

## 6.3 Public passport/profile manifest

A passport may publish a public b3-addressed profile/passport manifest.

Important distinction:

```text
The public profile/passport manifest may be a b3-addressed object.
The passport itself is not merely an asset.
Private keys, local alt mappings, spend authority, and recovery data are not public manifest fields.
svc-passport and ron-auth remain responsible for identity/capability semantics.
```

Suggested public manifest shape:

```json
{
  "schema": "ron.passport-manifest.v1",
  "passport_subject": "passport:main:...",
  "username": "@alice",
  "display_name": "Alice",
  "profile": {
    "bio": "...",
    "avatar": "crab://<hash>.image",
    "links": []
  },
  "wallet_public": {
    "default_receive_account": "acct_...",
    "spend_authority": "not_public"
  },
  "reputation": {
    "score": 842,
    "version": "rep-v1",
    "updated_at_ms": 1776000000000
  },
  "moderation": {
    "global_score": 77,
    "roles_summary": [
      {
        "site": "crab://example",
        "role": "trusted_moderator"
      }
    ]
  },
  "assets": {
    "owned_assets_index": "crab://@alice/assets",
    "recent": [
      "crab://<hash>.image",
      "crab://<hash>.article"
    ]
  },
  "sites": {
    "created": [
      "crab://alice-site"
    ]
  }
}
```

Private/local passport state may include:

```text
private keys
wallet spend caveats
alt passport mapping
local labels
recovery hints
device-local permissions
```

Private state must not leak into public manifests.

## 6.4 Alternate passports / anonymous alts

A user may create alternate anonymous/pseudonymous passports.

Alt rules:

```text
- Alt passports are separate identities.
- Alt passports may browse/surf/comment.
- Alt passports should not create sites by default.
- Alt passports have separate ROC balances/accounts.
- Alt passports do not automatically inherit main wallet authority.
- Alt passports should not be publicly linked to the main passport.
- Public main passport manifests should not list private alts.
- Public alt passport manifests should not reveal the main passport.
```

Local device may maintain a private mapping:

```text
main passport
→ local alt passport list
```

This local mapping is private local state.

It should not be published to public passport manifests.

## 6.5 Alt ROC balance and private withdrawal problem

Alt ROC balances should be separate from main passport balances.

User goal:

```text
A user may want to move ROC earned under an alt back to their main passport without public doxxing.
```

Important caveat:

```text
Do not promise absolute untraceability casually.
Privacy-preserving internal ROC movement is a hard cryptographic/accounting problem.
```

Safe blueprint position:

```text
next_level_alt_private_withdrawal = false by default
```

Future research-gated approaches may include:

```text
blind internal withdrawal notes
shielded internal ROC pool
epoch-based withdrawal batches
delayed claim windows
fixed denomination notes
zero-knowledge membership proofs later
wallet receipts that prove conservation without public alt→main linkage
```

MVP rule:

```text
Direct alt→main transfer is allowed only as an explicit public/traceable action with user warning.
Private alt→main withdrawal waits for a dedicated privacy protocol.
```

This protects user safety and avoids creating a false privacy promise.

## 6.6 Reputation score

Every passport should have a reputation score.

Initial reputation score should be read-only/display-only.

Inputs may eventually include:

```text
account age
successful paid publishes
assets not flagged
sites not flagged
positive moderation history
verified ownership claims
provider reliability if passport also operates provider role
community signals
site-specific trust signals
```

Anti-abuse rule:

```text
Do not make reputation directly farmable from raw impressions or simple likes in MVP.
```

Reputation should be:

```text
integer
deterministic
explainable
bounded
versioned
policy-driven
not wallet balance
not transferable
not directly purchased
```

## 6.7 Moderator score

Every passport should have a moderation score/profile.

Interpretation:

```text
All passports are eligible to participate in basic moderation signals.
Not all passports have enforcement privileges everywhere.
```

Suggested model:

```text
global_moderator_score
  Measures historical moderation reliability across RON.

site_moderator_roles
  Site-scoped roles granted by a site owner or site policy.

site_moderator_score
  Reputation for moderation inside a specific site.
```

Site-scoped moderator roles may include:

```text
viewer
commenter
signal_moderator
queue_moderator
content_moderator
trusted_moderator
admin_moderator
site_owner
```

Important rule:

```text
Being a passport holder does not automatically grant delete/ban/edit power on every site.
Every passport can submit moderation signals.
Sites designate special moderators with greater site-specific privileges.
```

---

# 7. Crate boundary map

## 7.1 ron-naming

Owns:

```text
crab:// parsing
typed asset suffix parsing
canonical crab string formatting
b3 normalization
site/name syntax validation
@username syntax normalization
reserved built-in names
deterministic parse errors
```

Does not own:

```text
storage
wallet mutation
database persistence
manifest hydration
payout execution
provider discovery
clearnet import policy
passport persistence
username uniqueness
```

NEXT_LEVEL work here should be tiny and isolated:

```text
Add missing asset kinds only when matching DTO/routes are ready.
Add @username parser/normalizer when passport username work begins.
Add tests for .post, .comment, .article, .video, .music only as they become active.
Do not add IO.
Do not add async.
```

## 7.2 ron-proto

Owns:

```text
cross-service DTOs
AssetKind
AssetManifestV1
AssetPageV1
ownership claim DTOs
payout role DTOs
receipt refs
site manifest DTOs
post/comment/article DTOs
reference graph DTOs
public passport/profile manifest DTOs
reputation summary DTOs
moderation summary DTOs
strict serde wire contracts
```

Does not own:

```text
storage logic
resolver logic
wallet mutation
ledger mutation
policy decisions
route handlers
username uniqueness
reputation calculation
moderation enforcement
passport key custody
```

NEXT_LEVEL work here should come before service logic.

Initial new DTOs:

```text
TextAssetBodyV1
PostAssetV1
CommentAssetV1
ArticleAssetV1
AssetReferenceV1
ReferenceGraphV1
ThreadManifestV1
SiteReferenceMapV1
PassportPublicManifestV1
PassportPublicProfileV1
ReputationSummaryV1
ModeratorSummaryV1
SiteModeratorRoleV1
AttributionStatus
ClearnetOriginRefV1 feature-gated
```

## 7.3 svc-passport

Owns:

```text
passport creation
main passport record
alt passport record
username claims
passport public manifest/profile publication metadata
capability issuance/association
wallet-link permission summaries
passport public profile hydration source
```

Does not own:

```text
wallet balances
ledger truth
storage bytes
asset ownership indexes
site manifests
moderation scoring algorithm unless explicitly assigned
```

NEXT_LEVEL svc-passport work should be staged:

```text
1. main passport creation with @username
2. read-only passport/profile route
3. public profile manifest publication metadata
4. alt passport creation as local/private-first feature
5. wallet-link permission display
6. moderation/reputation summaries as read-only fields
```

## 7.4 ron-auth

Owns:

```text
capability verification
macaroon/caveat primitives
spend authority caveats
site edit authority caveats
moderation authority caveats
short-lived runtime permissions
```

Does not own:

```text
passport persistence
wallet balances
site edits
moderation scoring
```

## 7.5 ron-app-sdk / future TS SDK

Owns:

```text
developer helpers
canonical request builders
idempotency helpers
typed publish flows
embed helpers
resolve helpers
passport helpers
error mapping
```

Does not own:

```text
wallet truth
ledger truth
storage truth
policy truth
provider records
passport truth
reputation truth
```

Target API shape:

```ts
ron.publishPost(...)
ron.publishComment(...)
ron.publishArticle(...)
ron.publishImage(...)
ron.publishVideoLite(...)
ron.resolveAsset(...)
ron.embedAsset(...)
ron.preparePaidView(...)
ron.createPassport(...)
ron.claimUsername(...)
ron.resolvePassport("@alice")
```

The SDK should reduce DX friction but not hide paid actions.

## 7.6 CrabLink

Owns:

```text
browser UX
full-window site rendering
gateway URL setting
passport creation UX
@username selection UX
passport/wallet display
alt passport selector
explicit paid action prompts
asset page rendering
built-in pages
embed renderers
copy/share controls
developer JSON panels
```

Does not own:

```text
wallet engine
ledger truth
storage truth
index truth
reward computation
passport truth
direct internal service calls
arbitrary code execution
```

NEXT_LEVEL CrabLink work should remain gateway-only.

Immediate CrabLink responsibilities:

```text
- Make site content span the usable window width.
- Keep site root rendering full-window instead of narrow debug cards.
- Keep Site Manifest proof drawer available.
- Show site creator @username from backend when available.
- Show reputation/moderation summary beside creator when backend provides it.
- Provide safe embed renderer family.
- Do not execute site-provided JS.
```

## 7.7 svc-gateway

Owns:

```text
public route exposure
header/body proxying
rate limits
body caps
policy edge checks
structured upstream errors
gateway-only API surface for CrabLink/SDKs
```

Does not own:

```text
business logic
asset manifests
payout splits
wallet mutation
ledger mutation
storage semantics
view hydration
passport truth
username uniqueness
```

NEXT_LEVEL gateway work should be mostly route proxy tests.

## 7.8 omnigate

Owns:

```text
product-facing BFF routes
prepare routes
crab resolver
b3 asset-page hydration
site hydration
reference graph hydration
passport/profile hydration
storage/index/wallet-ready response envelopes
```

Does not own:

```text
durable asset truth
raw byte storage
wallet balances
direct ledger mutation
provider truth
passport key custody
username uniqueness
```

NEXT_LEVEL omnigate work should be sequential:

```text
1. passport/profile read hydration after svc-passport has fields
2. post prepare/create
3. post resolve/hydrate
4. comment prepare/create
5. comment resolve/hydrate
6. article prepare/create
7. reference graph hydration
8. media-lite hydration
```

## 7.9 svc-storage

Owns:

```text
raw immutable bytes
manifest bytes as ordinary stored bytes
BLAKE3 verification
paid storage admission
wallet receipt verification for paid writes
storage availability metadata
usage event export
```

Does not own:

```text
names
manifests as semantic records
ownership claims
payout split logic
asset pages
site access policy
provider placement policy
passport profiles
```

MVP should reuse existing storage path.

Do not modify heavily until media chunking/range support requires it.

## 7.10 svc-index

Owns:

```text
asset CID → manifest CID pointer
site/name → manifest CID pointer
passport username → passport subject/profile pointer
reference graph lookup
thread/post/comment refs
asset catalogue lookup
metadata summaries
search facets later
```

Does not own:

```text
raw bytes
wallet mutation
ledger truth
payout execution
full content rendering
reputation computation
```

NEXT_LEVEL index work should be staged:

```text
1. existing asset pointer reuse
2. @username lookup only after svc-passport claim route exists
3. asset catalogue pointer/list only after manifest model exists
4. add reference-list records only if required
5. add thread/reference graph APIs
6. add search/list APIs later
```

## 7.11 svc-wallet / ron-ledger

Own:

```text
hold
capture
release
transfer
issue
burn
receipt
nonce
idempotency
ledger commit
balance truth
```

Do not change for text/media/passport primitives unless existing hold/capture/receipt shape is insufficient.

Future alt private withdrawals must not be bolted directly into svc-wallet without a dedicated privacy design.

## 7.12 ron-accounting

Owns:

```text
usage events
view events
serve events
storage byte-days
egress events
moderation event summaries later
qualified metering snapshots
```

Does not own:

```text
balance truth
ledger mutation
final payout execution
raw reputation truth unless explicitly specified
```

NEXT_LEVEL accounting comes after publish/resolve flows are stable.

## 7.13 svc-rewarder

Owns:

```text
deterministic payout plans
epoch calculations
split application
provider reward planning
creator/site-owner reward planning
event-qualified reward planning
```

Does not own:

```text
raw view collection
direct ledger mutation
asset ownership declarations
passport reputation calculation unless explicitly added later
```

Rewarder should come after accounting emits stable events.

## 7.14 ron-policy

Owns:

```text
pricing rules
split basis points
placement policy
provider class gates
feature flags
paid/free action policy
clearnet import policy
moderation role policy
reputation scoring policy later
anti-abuse thresholds later
```

Does not own:

```text
durable wallet truth
storage bytes
asset rendering
passport key custody
```

## 7.15 svc-registry

Owns:

```text
provider descriptors
dedicated provider class
self-hosted provider class
signed node metadata
uptime/SLA declarations later
```

Use later. Do not touch during first passport/embeds/text-primitive batches.

## 7.16 svc-dht

Owns:

```text
provider discovery
CID availability records
republish/TTL
lookup paths
```

Use later. Do not touch during first passport/embeds/text-primitive batches.

## 7.17 svc-edge

Owns:

```text
edge serving
cache/range delivery
egress accounting
edge provider role later
audio/video range serving later
```

Use later. Do not touch during first passport/embeds/text-primitive batches.

---

# 8. Data model

## 8.1 Canonical asset object

Every asset has:

```text
asset_kind
canonical bytes
asset_cid = b3:<BLAKE3(bytes)>
public_url = crab://<64hex>.<asset_kind>
```

## 8.2 Manifest object

Every published asset should have a manifest:

```text
manifest_cid = b3:<BLAKE3(manifest_bytes)>
```

Minimum fields:

```text
version
asset_cid
asset_kind
owner.passport_subject
owner.username optional
owner.wallet_account
payout.default_action
payout.recipient_account
metadata.title
metadata.description
metadata.tags
provenance.created_at
relations optional
storage optional
receipts optional
```

## 8.3 Public profile/passport manifest object

A public profile/passport manifest may also be stored as a b3 object.

It is not the private passport itself.

Minimum public fields:

```text
schema
passport_subject
username
display_name optional
profile avatar/bio/links optional
public asset catalogue pointer optional
public site list optional
reputation summary optional
moderation summary optional
public site roles optional
updated_at_ms
```

Forbidden public fields:

```text
private keys
seed material
local alt mapping
private spend caveats
private recovery hints
private wallet authority secrets
private alt→main linkage
```

## 8.4 Text asset body

For `.post`, `.comment`, and `.article`, the content bytes should be canonical JSON or canonical UTF-8 envelope.

MVP recommendation:

```json
{
  "schema": "ron.text-asset.v1",
  "kind": "post",
  "format": "text/markdown",
  "body": "Hello RON",
  "created_at_ms": 1776000000000
}
```

Do not hash arbitrary UI state.

Do not include wallet receipt fields inside the raw body.

Receipt and ownership live in the manifest.

## 8.5 Post asset

```json
{
  "schema": "ron.post.v1",
  "format": "text/markdown",
  "body": "My post",
  "relations": {
    "site": "crab://myforum",
    "thread": "crab://<64hex>.thread",
    "reply_to": null
  }
}
```

## 8.6 Comment asset

```json
{
  "schema": "ron.comment.v1",
  "format": "text/markdown",
  "body": "My comment",
  "relations": {
    "target": "crab://<64hex>.post",
    "site": "crab://myforum",
    "thread": "crab://<64hex>.thread"
  }
}
```

## 8.7 Article asset

```json
{
  "schema": "ron.article.v1",
  "format": "text/markdown",
  "title": "My Article",
  "body": "# Article body",
  "relations": {
    "site": "crab://myblog"
  }
}
```

## 8.8 Media asset

A `.video` or `.music` asset should not be treated as a simple text path forever.

MVP media-lite may use:

```json
{
  "schema": "ron.media-asset.v1",
  "kind": "video",
  "content_type": "video/mp4",
  "asset_cid": "b3:<hash>",
  "byte_length": 123456,
  "duration_ms": 12345,
  "poster_image": "crab://<hash>.image"
}
```

Production media eventually needs:

```text
range requests
chunked upload
edge caching
egress accounting
content-type validation
optional transcoding through svc-mod/svc-sandbox later
```

Do not pretend full video streaming is done when only small-file upload works.

## 8.9 Reference graph

A site page or thread may store references like:

```json
{
  "schema": "ron.reference-graph.v1",
  "owner_site": "crab://myforum",
  "route": "/thread/123",
  "refs": [
    {
      "kind": "post",
      "src": "crab://<64hex>.post"
    },
    {
      "kind": "comment",
      "src": "crab://<64hex>.comment"
    },
    {
      "kind": "image",
      "src": "crab://<64hex>.image"
    }
  ]
}
```

MVP can avoid building this as a new storage/index structure until post/comment/article creation works.

---

# 9. Payout model

## 9.1 Payout roles

NEXT_LEVEL payout roles:

```text
content_owner
site_owner
storage_provider
edge_provider
curator_pool
moderator_pool
treasury
reward_pool
burn
```

## 9.2 Safe first payout actions

Start with actions that are hard to fake:

```text
paid_asset_publish
paid_post_publish
paid_comment_publish
paid_article_publish
paid_site_launch
paid_storage
paid_view
storage_byte_day
egress_bytes
```

Avoid raw impression payouts in MVP.

## 9.3 Impression payouts are feature-gated

```text
next_level_free_impression_rewards = false
```

Reason:

```text
raw impressions are bot-prone
view farming can drain reward pools
anti-abuse is not trivial
```

Safer order:

```text
1. paid publishing/storage
2. paid views
3. provider storage/egress rewards
4. qualified free views later
```

## 9.4 Example split

```toml
[actions.paid_post_view]
base_price_minor = 1

[[actions.paid_post_view.splits]]
to = "content_owner"
bps = 5000

[[actions.paid_post_view.splits]]
to = "site_owner"
bps = 2500

[[actions.paid_post_view.splits]]
to = "storage_provider"
bps = 1500

[[actions.paid_post_view.splits]]
to = "edge_provider"
bps = 750

[[actions.paid_post_view.splits]]
to = "treasury"
bps = 250
```

Rules:

```text
bps total must equal 10_000
all amounts are integer minor units
rounding must be deterministic
dust handling must be deterministic
```

---

# 10. Hosting model

NEXT_LEVEL recognizes three hosting/placement modes.

## 10.1 Mode A — Automatic decentralized placement

Default mode.

User does not manually choose storage nodes.

The system chooses placement based on policy:

```text
replication factor
price
provider availability
capacity
region
latency
provider reputation later
content policy
```

Do not implement full automatic provider selection in the first passport/embed/text primitive batches.

MVP can use current local storage provider.

## 10.2 Mode B — Dedicated provider placement

Feature-gated.

```text
next_level_hosting_placement = true
dedicated_provider = true
```

Dedicated providers may advertise:

```text
price
region
uptime target
retention class
storage capacity
egress pricing
content policy
SLA metadata
```

Dedicated providers may charge custom ROC rates, but only when:

```text
user/site explicitly opts in
policy allows it
provider descriptor is signed/registered
pricing is visible before hold/capture
```

## 10.3 Mode C — Self-hosted placement

For large sites.

A site may run its own node and act as:

```text
site owner
storage provider
edge provider
publisher
```

Self-hosting should reuse the same asset/manifest/index/accounting model.

It must not become a separate architecture.

---

# 11. Clearnet crossover

Clearnet crossover is deferred and feature-gated.

```text
next_level_clearnet_crossover = false
```

## 11.1 Why deferred

Attribution is hard.

Potential problems:

```text
old sites may have many authors
site owner may not own user-generated content
scraped content may be misattributed
copyright claims are complex
legacy account identity may not map to RON passports
```

## 11.2 Safe future modes

### Mode A — Embed-only

Old website keeps its current backend and embeds RON assets.

```html
<ron-asset src="crab://<64hex>.post"></ron-asset>
```

Attribution comes from the RON asset itself.

This is safest.

### Mode B — User-signed migration

A user logs into the old site and signs a RON claim tying old account → RON passport.

Only then can migrated content receive creator payout.

### Mode C — Site-owner import

Site owner imports content they own.

Payout is limited to site owner unless individual creators later claim assets.

### Mode D — Unverified mirror

Imported content is marked:

```text
attribution_status = unverified_mirror
```

Default rule:

```text
no creator payout
no ownership claim
display provenance warning
```

---

# 12. Browser and executable-code policy

Do not execute arbitrary code from crab links.

Allowed:

```text
declarative manifests
typed asset renderers
safe markdown rendering
safe image/audio/video rendering
CrabLink-owned UI components
SDK-owned request builders
sandboxed HTML display with scripts disabled
```

Forbidden in this phase:

```text
remote JS execution from asset manifests
browser extension executing code blocks from crab links
untrusted app logic without sandbox
silent wallet actions
manifest-provided JS injection
```

Future executable app logic must go through:

```text
svc-mod
svc-sandbox
ron-policy
capability gating
```

That is outside NEXT_LEVEL MVP.

---

# 13. Implementation plan

The plan is divided so we do not touch too many crates at once.

---

## Phase 0 — Finish current CrabLink/image/site/reference-embed work

### Goal

Do not skip the current active flow.

### Status

```text
- CrabLink can create/open sites.
- CrabLink can render named crab:// sites.
- CrabLink can render <crab-image>.
- crab://thedustyonion6 proves reference-image embedding.
- Site root image-CID guard exists.
```

### Remaining tasks

```text
- Make rendered site content span the usable browser window cleanly.
- Remove accidental narrow-column layout where the site should be full-window.
- Keep Site Manifest proof drawer available but not dominant.
- Make image embed helper polished.
- Verify scripts/check-chrome.sh passes.
- Verify scripts/package-chrome.sh passes.
- Verify scripts/make_codebundle.sh passes.
- Verify new site creation after guard patch.
- Add optional smoke notes for crab-image embed.
```

### Repos touched

```text
CrabLink repo only
```

### Acceptance gates

```text
scripts/check-chrome.sh
scripts/package-chrome.sh
manual crab://image upload succeeds
manual crab://site create/open succeeds
manual crab://site with <crab-image> succeeds
gateway remains only backend target
no silent ROC spend
```

---

## Phase 1 — CrabLink embed resolver cleanup/generalization

### Goal

Avoid spaghetti before adding video/article/music embeds.

### Repos touched

```text
CrabLink repo only
```

### Tasks

```text
- Extract internal embed resolver shape from crab-image.
- Keep <crab-image> behavior unchanged.
- Create an embed registry table:
  image -> fetch bytes -> blob URL -> img
  article/post/comment -> future manifest/text renderer
  music/video -> future media renderer
- Keep all actual fetches gateway-only.
- Keep scripts disabled in sandbox.
```

### Acceptance

```text
<crab-image> still works
no backend crate touched
check/package/codebundle green
```

---

## Phase 2 — Thin passport/profile DTO + username parsing foundation

### Goal

Document and type the identity/display layer without jumping into full passport mutation yet.

This is intentionally narrow because the site creator chip already exists in CrabLink and currently uses an `@username` placeholder.

### Crates touched

```text
ron-naming
ron-proto
```

### Add

```text
Username / RonUsername parser
PassportPublicManifestV1 DTO
PassportPublicProfileV1 DTO
ReputationSummaryV1 DTO placeholder
ModeratorSummaryV1 DTO placeholder
SiteModeratorRoleV1 DTO placeholder
```

### Do not add yet

```text
svc-passport mutation
alt private withdrawal
full reputation algorithm
moderation enforcement
site edit authority
```

### Commands

```bash
cargo fmt
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets

cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

### Stop condition

Do not touch svc-passport until DTO/naming contracts are green.

---

## Phase 3 — svc-passport main username claim MVP

### Goal

When a user creates their first main passport, they choose their @username.

### Crates touched

```text
svc-passport
```

### Add

```text
main passport creation with username claim
reserved name rejection
duplicate name rejection
passport public profile/manifest payload
basic read-only profile response
```

### Do not add yet

```text
alt private withdrawal
reputation scoring engine
moderation enforcement
site edit authority
```

---

## Phase 4 — Passport/profile exposure through omnigate and gateway

### Goal

Expose read-only passport/profile lookup without making CrabLink call svc-passport directly.

### Crates touched one at a time

```text
omnigate
svc-gateway
```

### Candidate routes

```text
POST /identity/passports/bootstrap
GET  /identity/passports/@:username
GET  /crab/resolve?url=crab://@username
```

Exact route naming should follow existing project conventions.

### Acceptance

```text
CrabLink still only talks to svc-gateway.
crab://@username resolves to a read-only profile/passport payload.
Missing profile returns structured problem.
No wallet mutation occurs.
```

---

## Phase 5 — CrabLink first-passport @username UX and creator hydration

### Goal

Let the user choose an @username during first passport setup and hydrate site creator display when backend fields exist.

### Repo touched

```text
CrabLink
```

### Add

```text
first passport creation form
@username field
username validation hints
profile/passport drawer display
site creator field uses backend @username when present
site creator reputation/mod score displays when present
```

### Target UI

```text
site creator: @alice  rep 842  mod 77
```

### Acceptance

```text
Placeholder still works when backend has no username.
Real backend fields show when present.
Missing reputation/moderation fields do not break rendering.
No direct svc-passport calls.
No wallet truth in CrabLink.
```

---

## Phase 6 — Alt passport MVP

### Goal

Let users create/select alt passports locally and keep alt ROC account separate.

### Rules

```text
- Alt passports can browse/surf/comment.
- Alt passports cannot create sites by default.
- Alt ROC balance/account is separate.
- Public main manifest does not list alts.
- Public alt manifest does not reveal main.
- Local device may keep private main→alt mapping.
```

### Batch order

```text
CrabLink local selector/prototype
svc-passport alt creation route
omnigate exposure
svc-gateway exposure
wallet account link display
```

### Do not add yet

```text
private alt→main withdrawal
advanced anonymity claims
site creation by alts
```

---

## Phase 7 — Site edit/revision route contract

### Goal

Define, but do not yet overbuild, owner-authorized site editing.

### Why later

Site editing is privilege-bearing mutation.

It requires:

```text
owner passport check
site creation authority
site update authority
optional wallet hold/capture
new root document storage
new manifest revision
index pointer update
revision history
rollback/read history semantics
```

### Candidate routes

```text
POST /sites/:name/update/prepare
POST /sites/:name/update
GET  /sites/:name/revisions
GET  /sites/:name/revisions/:revision
```

### Invariants

```text
- Edits create new root CIDs.
- Edits do not mutate old root bytes.
- Name pointer advances only after authorization succeeds.
- Only main/site-owner passport or delegated site role can update.
- Alt passports cannot update sites by default.
```

### Start condition

Only after:

```text
passport @username MVP
site creator identity display
site creation flow is stable
```

---

## Phase 8 — Text asset DTO foundation

### Goal

Add DTO support for `.post`, `.comment`, `.article` without route logic.

### Crates touched

```text
ron-proto
possibly ron-naming if missing asset kinds
```

### Files likely touched

```text
crates/ron-proto/src/asset/mod.rs
crates/ron-proto/src/asset/manifest.rs
crates/ron-proto/src/asset/page.rs
crates/ron-proto/src/asset/ownership.rs
crates/ron-proto/src/asset/payout.rs
crates/ron-proto/src/lib.rs
crates/ron-proto/tests/asset_manifest.rs
crates/ron-proto/tests/asset_page_wire.rs
new: crates/ron-proto/tests/text_asset_wire.rs

crates/ron-naming/src/asset.rs
crates/ron-naming/tests/asset_kind.rs
crates/ron-naming/tests/crab_links.rs
```

### Add

```text
PostAssetV1
CommentAssetV1
ArticleAssetV1
TextFormat
AssetRelationV1
AttributionStatus
```

### Validation

```text
body bounded
title bounded
description bounded
tags bounded
asset kind must match suffix
unknown fields reject
invalid b3 rejects
invalid relation URL rejects
no IO
no async
no service clients
```

### Commands

```bash
cargo fmt
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets

cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

### Stop condition

Do not touch omnigate until DTOs are green.

---

## Phase 9 — Video/music exploratory plan

### Goal

The user wants to try video next, but video is more complex than images.

### Important constraint

Real video needs:

```text
larger files
range requests
chunking
content-type validation
possibly transcoding later
edge serving
egress accounting
```

### Safe next approach

Start with:

```text
video-lite
```

Meaning:

```text
- route contracts and DTOs first
- small test video only if current body limits allow
- no production streaming claim
- no edge/range rewrite in same batch
```

### Phase 9A — Media DTO only

Crates touched:

```text
ron-proto
ron-naming
```

Add:

```text
VideoAssetV1
MusicAssetV1
MediaMetadataV1
MediaPosterRefV1
```

### Phase 9B — CrabLink renderer skeleton only

Repo touched:

```text
CrabLink
```

Add future-safe renderer registry:

```html
<crab-video src="crab://<hash>.video"></crab-video>
<crab-audio src="crab://<hash>.music"></crab-audio>
```

But do not claim full production video streaming until backend support exists.

### Phase 9C — Backend video-lite only

Crates touched one at a time:

```text
omnigate
svc-gateway
```

Routes:

```text
POST /assets/video/prepare
POST /assets/video
GET  /b3/<hash>.video
GET  /crab/resolve?url=crab://<hash>.video
```

### Phase 9D — Full media streaming later

Crates likely touched later:

```text
svc-storage
svc-edge
svc-gateway
omnigate
ron-accounting
```

Only start when video-lite is proven.

---

## Phase 10 — Omnigate post primitive only

### Goal

Implement `.post` as the first text primitive using the image path as template.

### Crates touched

```text
omnigate
```

### Routes

```text
POST /v1/assets/post/prepare
POST /v1/assets/post
GET  /v1/b3/<hash>.post
GET  /v1/crab/resolve?url=crab://<hash>.post
```

### Behavior

```text
prepare returns hold template
create accepts paid proof
create stores post body as bytes through storage
create stores manifest as bytes through storage
create writes asset CID → manifest CID pointer through index
resolve hydrates omnigate.asset-page.v1
```

### Do not add

```text
comments
articles
reference graphs
paid views
provider placement
clearnet crossover
```

### Tests

```text
omnigate post_asset_prepare
omnigate post_asset_create
omnigate post_asset_resolve
invalid paid proof fails
invalid body too large fails
invalid asset kind mismatch fails
```

### Commands

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --all-targets
```

### Stop condition

`.post` must work before `.comment`.

---

## Phase 11 — Gateway post route exposure

### Goal

Expose public gateway routes for `.post`.

### Crates touched

```text
svc-gateway
```

### Routes

```text
POST /assets/post/prepare
POST /assets/post
GET  /b3/<hash>.post
GET  /crab/resolve?url=crab://<hash>.post
```

### Tests

```text
product_routes_proxy.rs
post_prepare_proxy_preserves_body_headers
post_create_proxy_preserves_paid_headers
post_resolve_proxy_targets_omnigate
upstream failure returns structured problem
```

### Commands

```bash
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --all-targets
```

---

## Phase 12 — Product smoke for paid post

### Goal

Add terminal proof before UI.

### Scripts

```text
scripts/web3_post_asset_smoke.sh
scripts/web3_product_stack_smoke.sh optional flag
```

### Feature flag

```text
RON_RUN_POST_CREATE=1
```

### Smoke flow

```text
start stack
prepare post
create wallet hold
publish post
resolve returned crab://<hash>.post
assert omnigate.asset-page.v1
assert manifest hydrated
assert owner/payout/receipt present
```

### Default

Safe default should skip mutation unless explicitly enabled.

---

## Phase 13 — CrabLink post UI

### Goal

Add built-in page:

```text
crab://post
```

### Repo touched

```text
CrabLink
```

### Behavior

```text
open crab://post
enter body/title/tags/site/thread
build preview
send prepare
confirm ROC hold
publish post
show crab://<hash>.post
auto-open asset page
refresh balance
```

### Invariants

```text
gateway-only
explicit hold confirmation
no local wallet truth
no arbitrary code execution
no direct omnigate/storage/index/wallet calls
```

---

## Phase 14 — Repeat for comment

### Goal

Add `.comment` after `.post` is proven.

### Crates touched

```text
ron-proto only if DTO gaps
omnigate
svc-gateway
CrabLink
scripts
```

### Difference from post

Comment requires target relation:

```text
target = crab://<hash>.post
or
target = crab://<hash>.article
or
target = crab://<site>/route
```

### Stop condition

Do not add article until comment is green.

---

## Phase 15 — Repeat for article

### Goal

Add `.article` as longer-form content.

### Difference from post

Article supports:

```text
title
subtitle optional
body
cover image optional
license optional
canonical source optional
```

Clearnet origin fields remain feature-gated and unverified by default.

---

## Phase 16 — Reference graph MVP

### Goal

Allow sites/pages/threads to store refs to assets.

### Crates touched

```text
ron-proto
svc-index
omnigate
svc-gateway
```

### Model

```text
site route or thread ID
→ ordered list of asset refs
```

### MVP route ideas

```text
POST /sites/:name/refs/prepare
POST /sites/:name/refs
GET  /sites/:name/refs
GET  /sites/:name/thread/:id
```

Exact route names should follow existing conventions.

### Rule

Reference graph work only begins after:

```text
.post create/resolve green
.comment create/resolve green
.article create/resolve green
```

---

## Phase 17 — Paid views

### Goal

Add explicit paid view flow.

### Crates touched

```text
ron-proto
omnigate
svc-gateway
svc-wallet only if existing hold/capture path needs no changes
ron-accounting later
svc-rewarder later
```

### Routes

```text
POST /assets/:kind/view/prepare
POST /assets/:kind/view
```

### MVP

```text
viewer pays
wallet hold/capture
asset page returns paid_view receipt
```

### Do not add

```text
free impression rewards
ad pool rewards
complex anti-bot
```

---

## Phase 18 — Accounting events for storage/serve/view/moderation

### Goal

Record qualified events.

### Crates touched

```text
ron-accounting
svc-storage
svc-edge later
svc-gateway/omnigate as needed
```

### Events

```text
asset_published
asset_view_paid
asset_served
storage_byte_day
egress_bytes
moderation_signal_submitted
moderation_action_taken
```

### Rule

Accounting remains transient counters/snapshots.

No ledger mutation.

---

## Phase 19 — Rewarder split planning

### Goal

Turn accounting/policy into deterministic payout plans.

### Crates touched

```text
svc-rewarder
ron-policy
svc-wallet client path if already exists
```

### Payout targets

```text
content_owner
site_owner
storage_provider
edge_provider
moderator_pool
curator_pool
treasury
```

### Acceptance

```text
same inputs produce same payout plan
bps total validates
integer rounding deterministic
no direct ledger mutation
wallet mutation only through svc-wallet
```

---

## Phase 20 — Hosting placement foundation

### Goal

Define provider classes and placement policy, not full global hosting yet.

### Crates touched

```text
ron-policy
svc-registry
svc-index
svc-storage
possibly svc-dht later
```

### Provider classes

```text
automatic
dedicated
self_hosted
```

### Rule

This is not first.

Do not touch before passport basics, embeddables, text primitives, reference graph, and basic accounting are stable.

---

## Phase 21 — Clearnet crossover experimental gate

### Goal

Allow old sites to adopt RON safely.

### Feature flag

```text
next_level_clearnet_crossover
```

### MVP safe crossover

```html
<ron-asset src="crab://<64hex>.post"></ron-asset>
```

No creator payout attribution is inferred from clearnet content.

### Later

```text
signed migration
legacy account binding
owner-declared import
unverified mirror
```

### Rule

No unverified clearnet import should receive creator payout by default.

---

# 14. Acceptance ladder

## Bronze

```text
CrabLink site rendering spans usable window width.
CrabLink <crab-image> remains green.
CrabLink embed resolver is clean enough to extend.
Public passport/profile DTOs exist.
@username normalization exists.
No service mutation for passports is attempted until DTO/naming foundation is green.
```

## Silver

```text
Main passport creation with @username works.
crab://@username resolves to a read-only profile/passport payload.
Site creator @username hydrates from backend when available.
Reputation and moderator score summaries display beside site creator when available.
Alt passport creation/selection exists in safe MVP form.
Alt wallet/account is separate from main wallet/account.
Direct alt→main transfer is explicitly labeled traceable until privacy protocol exists.
```

## Gold

```text
.post DTOs exist.
.post prepare/create/resolve works through omnigate.
gateway exposes .post routes.
terminal smoke publishes and resolves a post.
CrabLink can render crab://<hash>.post.
.comment and .article work.
site pages can reference post/comment/article assets.
CrabLink built-in pages exist for post/comment/article.
asset pages show owner/payout/receipt metadata.
passport profile pages show asset catalogue pointers.
```

## Platinum

```text
site edit/revision contract is implemented with owner capability checks.
reference graph MVP works.
paid views work.
accounting records qualified views/serves/storage/moderation events.
rewarder computes deterministic payout plans.
wallet commits payout receipts.
automatic decentralized placement exists.
dedicated provider class exists.
self-hosted provider class exists.
provider payout routing exists.
clearnet crossover embed-only safe mode exists.
```

## Diamond

```text
video/music streaming is range/chunk/edge-aware.
private alt→main ROC withdrawal has a real privacy protocol.
reputation/moderation scoring is deterministic, abuse-resistant, and explainable.
site-scoped moderation roles are enforceable by policy/capabilities.
```

---

# 15. Anti-spaghetti rules

## 15.1 One crate cluster at a time

Allowed batch shapes:

```text
CrabLink batch:
  CrabLink only

DTO/naming batch:
  ron-proto + maybe ron-naming only

Passport service batch:
  svc-passport only

Passport exposure batch:
  omnigate only, then svc-gateway only

BFF batch:
  omnigate only

Gateway batch:
  svc-gateway only

Smoke batch:
  scripts/docs only

Economics batch:
  ron-policy + svc-rewarder only after accounting shape is stable

Accounting batch:
  ron-accounting + one emitting service only
```

Avoid touching:

```text
ron-proto + ron-naming + svc-passport + omnigate + svc-gateway + svc-index + svc-storage + svc-wallet + CrabLink
```

in one batch unless it is a tiny final integration patch.

## 15.2 No new crates

Do not add a new “primitives service.”

The primitive layer is:

```text
ron-proto DTOs
ron-naming parsing
ron-app-sdk helpers
svc-passport identity records
omnigate BFF routes
svc-gateway exposure
CrabLink UX
```

## 15.3 Never make storage semantic

Storage must not learn what a forum post means.

It stores bytes.

## 15.4 Never make index economic

Index must not calculate payouts.

It stores pointers/refs.

## 15.5 Never make CrabLink truth

CrabLink displays backend truth.

## 15.6 Never execute manifest code

Manifests are declarative.

Rendering code is trusted client code.

## 15.7 Never promise privacy you have not built

Alt privacy must be real.

Until a real protocol exists:

```text
direct transfers are traceable
private withdrawal is deferred
public manifests do not link alts
local mapping stays local
```

---

# 16. Recommended immediate next action

The immediate next action is **not** to jump into five Rust crates at once.

Recommended order from current checkpoint:

```text
1. CrabLink-only layout polish:
   Make site content span the usable browser window.
   Keep Site Manifest proof available but not dominant.

2. CrabLink-only embed resolver cleanup:
   Keep <crab-image> green.
   Prepare clean registry for future <crab-video>, <crab-audio>, <crab-article>, <crab-post>, <crab-comment>.

3. Thin passport/profile DTO + naming batch:
   Add public profile/passport manifest DTOs to ron-proto.
   Add @username parser/normalizer to ron-naming.
   Do not add svc-passport mutation yet.

4. svc-passport username batch:
   First-passport creation with @username.

5. Omnigate/gateway read-only profile exposure:
   Resolve crab://@username through gateway.

6. CrabLink passport creation UX:
   User chooses @username during first passport setup.

7. Site creator hydration:
   Site page top bar uses real @username and reputation/mod score when backend provides it.

8. Media-lite exploratory DTOs:
   Prepare for .video and .music carefully without claiming production streaming.

9. Text primitive foundation:
   .post, .comment, .article.
```

This sequence keeps the project cohesive, avoids spaghetti, and builds directly on the proof we just achieved.

---

# 17. Success statement

NEXT_LEVEL succeeds when a developer can build a forum/blog/media/social app where:

```text
users create main passports with @usernames
users can use anonymous/pseudonymous alts safely
site creators are visible by @username and reputation/moderation summaries
users publish images/posts/comments/articles/music/videos as b3-addressed assets
sites store references instead of owning all content bytes
CrabLink renders the assets through crab:// links and embed primitives
ownership and payout metadata are manifest-backed
paid actions require explicit ROC confirmation
storage/serve/view/moderation events are measurable
ROC splits can route to creator + site owner + provider + moderator/curator pools
site edits are revisioned and owner-authorized
private alt→main ROC withdrawal is only claimed after a real privacy protocol exists
```

This makes RustyOnions a creator economy substrate, not just a storage/payment demo.

The implementation must be gradual, feature-gated, privacy-honest, and crate-boundary-safe.

---
### NEW: UPDATE REGARDING ALTS AND PASSPORTS: 
---

# NEXT_LEVEL Addendum — Privacy-Aware `.profile`, `.passport`, and `.alt` Identity Roots

RO:WHAT — Defines the intended identity/address model for `.profile`, `.passport`, and `.alt` before implementation.
RO:WHY — Prevents accidental privacy leaks while preserving the b3-hash-first RustyOnions identity model.
RO:INTERACTS — ron-proto, ron-naming, svc-passport, svc-index, omnigate, svc-gateway, CrabLink.
RO:INVARIANTS — b3 hashes remain canonical; passports are identity/capability records, not wallets; alts must not publicly link to main passports; wallet authority remains private/caveated.
RO:SECURITY — Alt identity roots must not contain or derive from main-passport material, main wallet material, local device IDs, funding source, or parent linkage.
RO:TEST — Future implementation requires DTO tests, crab URL parsing tests, passport issue tests, profile/alt route-contract tests, privacy negative tests, and CrabLink UI smoke tests.

## Status

This section is **doctrine/design only** for now.

Do not implement this as a quick frontend-only illusion. The address model touches identity, profile manifests, passport roots, alt roots, naming, indexing, gateway hydration, and CrabLink display. Implementation should happen only after the DTO and route contracts are explicit.

## Product model

RustyOnions identity should distinguish:

```text
.profile  = public SSO/profile page
.passport = public or semi-public passport identity/capability proof page
.alt      = pseudonymous alt identity page with no public main linkage
.wallet   = not directly public by default
````

Recommended public examples:

```text
crab://skinnycrabby.profile
crab://<64hex>.passport
crab://<64hex>.alt
```

Where:

```text
<64hex>.passport = b3 hash of a canonical main passport public root manifest
<64hex>.alt      = b3 hash of a canonical alt public root manifest
```

## `.profile`

A `.profile` is the user-facing SSO identity page.

Example:

```text
crab://skinnycrabby.profile
```

A `.profile` may show:

```text
- username / handle
- display name
- avatar
- bio
- public profile manifest CID
- public sites
- public assets
- public reputation summary
- public moderator summary
- public site roles
- optional public payout pointer
```

A `.profile` must not show:

```text
- private keys
- wallet spend authority
- private alt mappings
- private permissions
- private receipts
- private balances unless explicitly published
- private device or recovery metadata
```

Sites should be able to support SSO-style membership using `.profile` / passport identity:

```text
open membership: any valid profile/passport may join
invite-only: site grants membership to selected identities
alt-allowed: site permits pseudonymous .alt identities
main-only: site requires a main .profile/.passport
```

## `.passport`

A `.passport` is a public or semi-public passport proof/capability information page.

Example:

```text
crab://<64hex>.passport
```

The `<64hex>` should be:

```text
b3(canonical_main_passport_public_root_manifest)
```

A main passport public root may include:

```text
- schema/version
- kind = "main"
- passport public key or verification key
- public random nonce
- optional profile pointer
- optional public capability summary
- optional public site-creation eligibility flag
```

A `.passport` may be wallet-linked, but it is not the wallet itself. It must not expose wallet secrets or spend authority.

A `.passport` must not be treated as:

```text
- a raw wallet address
- a private key container
- a spend-capable token
- a complete list of user alts
```

## `.alt`

A `.alt` is a separate pseudonymous passport identity page.

Example:

```text
crab://<64hex>.alt
```

The `<64hex>` should be:

```text
b3(canonical_alt_public_root_manifest)
```

The alt public root must be generated independently from the main passport.

A valid alt public root may include:

```text
- schema/version
- kind = "alt"
- independently generated alt public key
- public random nonce
- optional alt profile pointer
- optional alt public capability summary
```

A valid alt public root must not include:

```text
- main passport ID
- main username
- main wallet account
- main public key
- main profile pointer
- parent_passport
- created_by
- funding source
- recovery controlled by main
- same deterministic derivation path
- local device identifier
- private main↔alt linkage
```

## Privacy invariant for alts

Main passports may locally create and manage alts, but public network data must not reveal the relationship.

Hard invariant:

```text
Alt public IDs MUST be generated from independently generated alt public identity roots.
An alt root may include an alt public key, random public nonce, schema/version, and optional alt profile pointer.
An alt root MUST NOT include or derive from the main passport ID, main username, main wallet, main public key, funding source, local device ID, or parent relationship.
Main↔alt linkage is local/private unless the user explicitly publishes it.
```

## Local management model

A main passport client may privately know:

```text
main passport:
  - crab://skinnycrabby.profile
  - crab://<main-root-hash>.passport
  - crab://<alt-root-hash-1>.alt
  - crab://<alt-root-hash-2>.alt
```

But the public network should only see each alt as its own identity:

```text
crab://<alt-root-hash-1>.alt
```

No public page, index record, profile manifest, site membership record, or wallet-facing DTO may reveal:

```text
this alt belongs to skinnycrabby
```

unless the user explicitly publishes that linkage.

## Privacy honesty

Do not claim absolute anonymity until stronger privacy routing and operational protections exist.

Alt passports are designed to avoid public identity-schema linkage, but users may still leak correlation through:

```text
- IP / gateway timing
- device or browser fingerprinting
- funding an alt wallet directly from the main wallet
- writing style
- reused avatar/name/tags
- posting patterns
- non-amnesia backend logs
```

Preferred product wording:

```text
Alt passports are pseudonymous and do not publicly link to the main passport by default.
They are not guaranteed anonymous against network, timing, funding, or device-correlation attacks unless stronger privacy routing is enabled.
```

## Implementation gate

Do not implement the full `.passport` / `.alt` model until these are ready:

```text
1. ron-proto DTOs for main passport public root and alt public root.
2. ron-naming parser support for:
   - crab://<username>.profile
   - crab://<64hex>.passport
   - crab://<64hex>.alt
3. svc-passport issue/verify path for public root manifests.
4. svc-index registration/lookup rules for .profile, .passport, and .alt.
5. omnigate hydration for profile/passport/alt pages.
6. svc-gateway public proxy routes.
7. CrabLink read-only rendering for profile/passport/alt pages.
8. Privacy negative tests proving no main→alt public linkage appears in DTOs, manifests, route responses, or CrabLink UI.
```

## Near-term CrabLink rule

CrabLink may continue to show:

```text
crab://profile
```

as a **local SSO/profile scaffold**, but it must label it as local/pending until backend profile/passport manifests exist.

CrabLink must not pretend that:

```text
crab://skinnycrabby.profile
crab://<hash>.passport
crab://<hash>.alt
```

are backend-confirmed until RustyOnions actually supports those route contracts.

---

```
```
