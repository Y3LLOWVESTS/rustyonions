# INTERNAL_ROC_MODEL.md — Internal ROC Tokenomics, Fairness, and Reward Doctrine

RO:WHAT — Defines the internal ROC tokenomics doctrine for CrabLink/RustyOnions: reward pools, event classes, splits, burns, anti-farming rules, and config ownership.

RO:WHY — ROC needs a fair, bounded, configurable, replayable internal economy before any external anchor, ROX, bridge, or public settlement path is considered.

RO:INTERACTS — configs/roc-economics.toml, ron-policy, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-ads, svc-storage, svc-gateway, omnigate, svc-index, CrabLink Tauri, QuickChain.

RO:INVARIANTS — ROC remains internal; wallet/ledger remain economic truth; raw engagement does not directly mint or allocate protocol ROC; all mutable rates live in TOML; no fake receipts, fake balances, fake finality, or silent spend.

RO:METRICS — Future metrics may track reward planning, category pool exhaustion, burn totals, ad-budgeted flow, node delivery proof outcomes, moderation outcomes, anti-farming rejections, and epoch settlement summaries.

RO:CONFIG — All mutable tokenomics values live in configs/roc-economics.toml or explicitly versioned local/dev overrides. No hard-coded payout amounts in business logic.

RO:SECURITY — No ROX/Solana active runtime, no public bridge, no external settlement, no exchange-facing logic, no client-side payout authority, no cache-only paid unlock, no policy/index/rewarder/accounting ledger mutation.

RO:TEST — Future tests should cover TOML validation, basis-point sum checks, integer minor-unit parsing, event-class routing, reward pool caps, anti-farming caps, rounding/remainder sinks, burn accounting, and no-authority boundaries.

---

## 0. Status

This document is the internal ROC tokenomics guide for RustyOnions / CrabLink.

It is a doctrine and design reference.

It is not a live payout promise.

It is not a public financial guarantee.

It is not a bridge implementation plan.

It is not a ROX launch plan.

It is not a Solana runtime plan.

It is not exchange-facing documentation.

This guide exists to make the internal ROC economy:

```text
bounded
fair
configurable
auditable
deterministic
anti-farming
creator-first
wallet/ledger-truthful
QuickChain-proofable later
```

The active buildplan remains authoritative for QuickChain phase order.

This guide supports the buildplan by defining the internal economics doctrine that future DTOs, reward plans, and checkpoint commitments may reference.

---

## 1. Relationship to QuickChain Phase 5

QuickChain Phase 5 remains:

```text
Phase 5 Round 1 — anchor-only design and dry-run
Phase 5 Round 2 — DA/archive/challenge fallback
Phase 5 Round 3 — chosen external integration path
```

This tokenomics guide does not change those rounds.

This guide does not authorize active ROX runtime, Solana runtime, external settlement, public bridge, public staking, liquidity, or exchange-facing logic.

For Phase 5 Round 1, this guide helps identify deterministic internal economics artifacts that may later be committed into compact checkpoint evidence, such as:

```text
classified event snapshots
epoch reward pool plans
reward point summaries
manifest split structures
ad-budgeted burn summaries
node service summaries
moderation outcome summaries
payout planning manifests
remainder sink records
```

But those artifacts remain internal and non-authoritative until wallet/ledger execution.

Correct Phase 5 relationship:

```text
tokenomics guide
→ stable economics doctrine

configs/roc-economics.toml
→ mutable economics values

ron-policy
→ validates/evaluates economics config

svc-rewarder
→ plans capped payouts

svc-wallet
→ executes approved mutations

ron-ledger
→ records durable economic truth

QuickChain
→ later commits/proves deterministic artifacts
```

Incorrect relationship:

```text
QuickChain anchor
→ directly mutates balances

reward plan
→ directly becomes ledger truth

raw engagement
→ directly mints ROC

config value
→ directly creates receipt

CrabLink UI
→ directly unlocks paid content or creates payout
```

---

## 2. Core doctrine

The core internal ROC doctrine is:

```text
raw engagement must not directly mint or allocate protocol ROC
```

Raw activity may create metering, analytics, or proof-eligible events.

Raw activity is not itself a payout.

Correct value path:

```text
event classification
→ ron-accounting snapshot/report
→ svc-rewarder capped payout planning
→ ron-policy validation/gating
→ svc-wallet approved mutation
→ ron-ledger durable economic truth
```

Only svc-wallet may request economic mutation.

Only ron-ledger records durable economic truth.

ron-accounting is not balance truth.

svc-rewarder plans payouts but does not mutate ledger.

ron-policy declares and validates rules but does not mutate ledger.

svc-index points and indexes but does not prove payment.

svc-storage stores bytes/artifacts but does not prove payment.

svc-gateway and omnigate expose and enforce backend-derived flows but do not mutate ledger truth.

CrabLink displays backend-derived truth and captures user intent only.

---

## 3. Canonical economics config

All mutable ROC economics values must live in:

```text
configs/roc-economics.toml
```

Optional local/dev override files may exist:

```text
configs/roc-economics.dev.toml
configs/roc-economics.local.toml
```

Those override files must not replace canonical policy unless explicitly intended.

Hard invariant:

```text
No hard-coded payout amounts in business logic.
No hard-coded reward rates scattered across crates.
No hard-coded burn rates scattered across crates.
No hard-coded split percentages scattered across crates.
No hard-coded tokenomics constants in UI/client code.
```

The TOML config is the single editable reference point for:

```text
site visit weights
post view weights
article read weights
image view weights
comment weights
share/referral weights
video playback weights
video minute weights
song play weights
podcast play weights
livestream viewer-minute weights
livestream participant split defaults
reuse/mashup split defaults
source clip pool defaults
curator/referrer split defaults
node delivery reward weights
node availability reward weights
rare/archival content premiums
moderation reward weights
abuse-report reward weights
appeal-review reward weights
ad campaign burn rates
ad campaign viewer/site/node/moderation splits
protocol emission caps
epoch reward pool category percentages
reward point conversion parameters
quality multipliers
reputation multipliers
scarcity multipliers
anti-sybil multipliers
rate caps
max spend limits
hold multipliers
rounding mode
remainder sink
future ROC/ROX conversion fee placeholders
future bridge burn-rate placeholders
```

All values must be deterministic and integer-safe:

```text
money amounts: integer minor-unit strings or integer minor units
percentages/splits: basis points
10000 bps = 100%
no floats
no implicit rounding
no hidden remainder
```

Every rounding remainder must go to an explicit configured sink:

```text
remainder_sink = "treasury"
remainder_sink = "burn"
remainder_sink = "stability_buffer"
remainder_sink = "configured_account:<id>"
```

The sink must be validated.

No silent inflation from rounding drift is allowed.

---

## 4. Suggested TOML shape

The exact schema may evolve, but the economics config should eventually resemble this structure.

All numeric values in this example are illustrative doctrine posture only. Actual values must come from `configs/roc-economics.toml`.

```toml
schema = "rustyonions.roc-economics.v1"
mode = "internal_roc"
currency = "ROC"

[units]
minor_unit_name = "roc_minor"
money_format = "integer_minor_units"
split_format = "basis_points"
bps_total = 10000
floats_allowed = false

[epoch]
epoch_duration_seconds = 86400
reward_settlement_mode = "epoch_close"
rounding_mode = "floor_with_remainder_sink"
remainder_sink = "stability_buffer"

[emission]
enabled = true
epoch_protocol_pool_minor = "1000000000"
max_epoch_payout_minor = "1000000000"
creator_pool_cap_bps = 3500
curation_pool_cap_bps = 1000
node_pool_cap_bps = 2000
moderation_pool_cap_bps = 1500
developer_ecosystem_pool_cap_bps = 500
stability_buffer_bps = 1500

[anti_farming]
max_passive_attention_bps = 1000
max_site_visit_points_per_passport_per_day = 100
max_post_view_points_per_passport_per_day = 200
max_comment_points_per_passport_per_day = 300
max_share_points_per_passport_per_day = 300
min_dwell_seconds_for_view = 5
min_song_seconds_for_partial_play = 30
min_video_seconds_for_partial_play = 10
repeat_decay_enabled = true
sybil_multiplier_min_bps = 0
sybil_multiplier_max_bps = 10000

[ad_campaigns]
enabled = true
default_burn_bps = 8000
viewer_reward_bps = 700
creator_site_bps = 700
node_service_bps = 400
moderation_reserve_bps = 200
remainder_sink = "burn"

[paid_content]
creator_default_bps = 9000
node_service_default_bps = 500
protocol_burn_default_bps = 300
moderation_reserve_default_bps = 200

[future_bridge]
enabled = false
roc_to_rox_burn_bps = 0
rox_to_roc_burn_bps = 0
bridge_fee_bps = 0
bridge_min_fee_minor = "0"
bridge_max_fee_minor = "0"
bridge_challenge_window_epochs = 0
bridge_daily_limit_minor = "0"
bridge_per_user_limit_minor = "0"
bridge_pause_enabled = true
bridge_emergency_halt_enabled = true
bridge_remainder_sink = "burn"
```

Production values must come from `configs/roc-economics.toml`.

---

## 5. Event classes

The internal ROC economy uses these event classes:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

### economic_receipt

An `economic_receipt` may represent wallet/ledger-accepted economic truth.

Examples:

```text
paid content purchase
creator payout execution
node payout execution
moderation payout execution
burn receipt
hold open
hold capture
hold release
transfer
issue
burn
```

Rules:

```text
must come from wallet/ledger path
may affect balances only through svc-wallet and ron-ledger
must not be invented by UI/cache/index/policy/rewarder/accounting
```

### metering

A `metering` event records usage, attention, or service facts.

Examples:

```text
site visit
post view
video playback second
song play second
node bytes served
range request
livestream viewer-minute
comment view
share click
```

Rules:

```text
not a payout
not balance truth
not paid unlock truth
may feed accounting snapshots
may become proof_eligible after filtering
```

### proof_eligible

A `proof_eligible` event may be considered for reward planning after verification.

Examples:

```text
qualified article read
qualified video completion
qualified song play
accepted moderation action
verified node delivery proof
availability proof
confirmed useful report
valid referral conversion
```

Rules:

```text
still not a payout
requires caps
requires policy
requires rewarder planning
requires wallet execution before money exists
```

### ad_budgeted

An `ad_budgeted` event is funded by an explicit ad/campaign budget.

Examples:

```text
ad impression with threshold
ad click with abuse checks
sponsored content view
campaign-funded creator placement
campaign-funded node delivery
```

Rules:

```text
funded from advertiser/campaign budget
not unlimited protocol emission
usually includes burn
must be labeled separately
```

### analytics_only

An `analytics_only` event is for measurement only.

Examples:

```text
page layout analytics
hover event
scroll event
impression below threshold
preview event
bot-labeled traffic
debug event
```

Rules:

```text
never becomes protocol ROC payout material
never mutates balances
never unlocks paid content
```

---

## 6. Reward points before ROC

The recommended internal model is:

```text
verified contribution
→ reward points
→ epoch pool conversion
→ planned ROC payout
→ policy gate
→ wallet execution
→ ledger receipt
```

Do not hard-code fixed ROC-per-view values into business logic.

Use reward points first.

At epoch close:

```text
category_point_value =
  category_epoch_pool_minor / total_verified_points_in_category
```

Then each eligible payout is planned from:

```text
user_points
× category_point_value
× quality_multiplier_bps
× reputation_multiplier_bps
× scarcity_multiplier_bps
× anti_sybil_multiplier_bps
```

All multipliers must be integer basis points.

All final payouts must be integer minor units.

Rounding remainders must go to the configured remainder sink.

If total verified points exceed the pool, point value falls.

If farming increases, farmers dilute themselves rather than inflating the protocol.

---

## 7. Category pools

Protocol emission should be separated into capped epoch categories.

Example doctrine categories:

```text
creator_content_rewards
curation_sharing_rewards
node_storage_delivery_rewards
moderation_trust_safety_rewards
developer_ecosystem_rewards
stability_buffer
```

Illustrative doctrine posture only:

```text
creator_content_rewards          35%
curation_sharing_rewards         10%
node_storage_delivery_rewards    20%
moderation_trust_safety_rewards  15%
developer_ecosystem_rewards       5%
stability_buffer                 15%
```

These percentages are not hard-coded.

They must live in `configs/roc-economics.toml`.

Rules:

```text
one category cannot drain the whole economy
passive attention should have a low cap
creator/node/moderation work should have stronger pools
stability buffer absorbs uncertainty
ad-budgeted activity is separate from protocol emission
```

---

## 8. Passive attention

Passive attention includes:

```text
site visits
post views
article views
image views
video starts
song starts
livestream presence
```

Passive attention should be low-value and heavily capped.

Reason:

```text
passive attention is easy to bot
passive attention can be farmed by refresh loops
passive attention is weaker evidence of value than creation, service, moderation, or verified curation
```

Examples of required gates:

```text
minimum dwell time
unique-enough viewer
rate cap per passport
rate cap per site/content/day
hidden-tab rejection where possible
repeat decay
bot/sybil multiplier
quality/reputation multiplier
```

A site visit should never be a large unconditional payout.

A post view should never be a large unconditional payout.

A video start should not equal a full video play.

A song start should not equal a full song play.

---

## 9. Creator rewards

Creator rewards may apply to:

```text
post creation
article creation
image publishing
video publishing
music publishing
podcast publishing
site publishing
livestream hosting
template/theme creation
developer/facet creation
```

Creation should be rewarded more strongly than passive attention, but still not blindly.

Recommended gates:

```text
content must be original or properly attributed
content must survive moderation
content must not be duplicate spam
content must have valid manifest/provenance
content must not be self-farmed through fake engagement
content may be weighted by quality/reputation/retention
```

Creator rewards should generally come from:

```text
paid content revenue
ad-budgeted placement
creator/content epoch pool
tips/supporter payments
manifest-defined splits
```

Creator rewards should not come directly from raw views without caps and verification.

---

## 10. Manifest-defined splits

Manifest-defined splits are the long-term fairness primitive.

Every asset or content object should eventually support a payout split.

Examples:

```text
post manifest payout split
comment thread payout split
image manifest payout split
video manifest payout split
music manifest payout split
podcast manifest payout split
livestream session split
site payout policy
reuse/mashup source split
```

Splits must use basis points:

```text
10000 bps = 100%
```

The split total must equal exactly `10000`.

Unknown split fields must be rejected.

Invalid totals must be rejected.

Negative values are impossible.

Floating point values are forbidden.

A generic split may include:

```text
creator_bps
collaborator_bps
source_reuse_pool_bps
curator_bps
node_service_bps
moderation_reserve_bps
burn_bps
remainder_sink
```

Illustrative doctrine posture only:

```text
creator_bps              7000
collaborator_bps         1000
source_reuse_pool_bps    1000
curator_bps               500
node_service_bps          400
moderation_reserve_bps    100
burn_bps                    0
total                   10000
```

The exact fields may differ by asset kind, but the invariant is stable:

```text
manifest splits are integer, explicit, validated, and deterministic
```

---

## 11. Sharing and anti-theft splits

Sharing should reward distribution without making repost farming more profitable than creation.

Default doctrine:

```text
original creator receives majority
sharer/curator receives minority
node/service receives service share
moderation/trust may receive reserve share
```

For normal sharing, illustrative doctrine posture only:

```text
original_creator_bps      7500
sharer_curator_bps        1500
node_service_bps           700
moderation_reserve_bps     300
```

For quote-post or commentary sharing, illustrative ranges only:

```text
original_creator_bps      6000-7000
commentator_bps           2000-3000
node_service_bps           500-1000
moderation_reserve_bps       0-500
```

For unauthorized reposts, illustrative posture only:

```text
original_creator_bps      9000-10000
reposter_bps                 0
possible_penalty             yes
possible_bond_loss           later, if bonded rules apply
possible_moderation_review   yes
```

All actual defaults must live in `configs/roc-economics.toml`.

Manifest/provenance should route value back to the original source where possible.

---

## 12. Reuse, remix, and mashup doctrine

Reuse is allowed only if the source manifest permits it or policy allows it.

A reusable asset may declare:

```text
reuse_ok
attribution_required
commercial_reuse_ok
derivative_ok
source_pool_min_bps
creator_min_bps
burn_bps
node_service_bps
```

For video commentary/mashup content, the fair split depends on transformation level.

Illustrative doctrine posture only:

```text
new_commentary_creator_bps   5000-7000
source_clip_pool_bps         2000-4000
node_service_bps              500-1000
moderation_reserve_bps          0-500
```

The source clip pool may be divided by:

```text
clip seconds used
viewer-visible time
audio prominence
source importance
manifest-defined minimums
manual source weighting
```

For simple aggregation with little commentary, source creators should receive more.

For transformative commentary, the new creator may receive more.

Unauthorized reuse should route value back to original sources and may trigger moderation.

All actual split defaults must live in `configs/roc-economics.toml`.

---

## 13. Music and song play doctrine

Music should be creator- and rights-holder-favorable.

Music manifests should support splits for:

```text
artist
producer
featured_artist
publisher_or_collective
curator_or_playlist
node_service
moderation_reserve
burn_or_protocol_reserve
```

Illustrative doctrine posture for paid or qualified play only:

```text
artist_rights_pool_bps       7500-8500
curator_playlist_bps          500-1000
node_service_bps              500-1000
moderation_reserve_bps          0-200
burn_or_reserve_bps             0-500
```

No full reward for instant skips.

Suggested play thresholds, also illustrative only:

```text
under 10 seconds:
  no reward or analytics_only

30 seconds:
  partial play eligibility

80%+ completion:
  full play eligibility

repeat loop:
  decay/cap

playlist farming:
  cap and audit
```

All thresholds and splits must be TOML-controlled.

---

## 14. Video playback doctrine

Video rewards should distinguish:

```text
video start
qualified watch
verified minute
completion
replay
paid unlock
ad-budgeted view
```

A video start should be low-value.

A verified video minute may be eligible for more, but still capped.

Completion can add a quality signal.

Paid video playback should use actual paid receipts, not protocol emission alone.

Video reward inputs may include:

```text
seconds watched
percentage completed
unique viewer cap
device/session trust
replay decay
creator manifest split
source reuse split
node delivery cost
ad-budgeted campaign terms
```

Video should not become the easiest farming surface.

All actual thresholds, weights, and caps must live in `configs/roc-economics.toml`.

---

## 15. Livestream doctrine

Livestreams need explicit splits because multiple people may contribute at once.

A paid livestream should define splits before money is collected.

Possible livestream split categories:

```text
host
active speakers/performers
guest pool
moderator pool
node/relay pool
protocol burn/reserve
```

Illustrative doctrine posture only:

```text
host_bps                    2500-4000
active_participant_pool_bps 3500-5500
guest_pool_bps               500-1500
moderator_pool_bps           500-1000
node_relay_pool_bps          500-1500
burn_or_reserve_bps            0-500
```

Participant pool may be divided by:

```text
screen time
audio time
active speaker time
manual pre-stream split
role weight
viewer-minute
```

Rules:

```text
no surprise splits after payment
stream host cannot silently take guest shares
viewer-minute rewards must be capped
node/relay compensation must account for actual service
moderator compensation should be outcome/review based
```

All actual splits, thresholds, and viewer-minute weights must live in `configs/roc-economics.toml`.

---

## 16. Comments and conversation doctrine

Comments are easy to spam.

Do not reward every comment automatically.

Recommended lifecycle:

```text
comment posted
→ no immediate payout or tiny pending eligibility
→ survives moderation
→ receives organic engagement
→ may become proof_eligible
→ rewarder plans within comment/thread cap
→ policy gate
→ wallet/ledger execution
```

A valuable comment thread may split rewards among:

```text
commenter
original post creator
thread curator/referrer
moderation/trust pool
node/service pool
```

Bad comments should receive no reward.

Removed comments should receive no reward.

Spam comments may reduce reputation or trigger penalties.

---

## 17. Node reward doctrine

Nodes should be paid for verified useful service, not for claims.

Node rewards should be split into:

```text
verified delivery
availability
rare/archival availability
paid-content service share
ad-budgeted delivery share
future DA/archive support
```

Inputs may include:

```text
bytes served
successful range requests
verified b3 integrity
availability proofs
rarity/scarcity
latency/reliability
geographic diversity
requester diversity
challenge/audit result
```

Rules:

```text
popular content pays lower per-byte rewards
rare/archival content may pay availability premium
paid content may share transaction service fee
free/ad-funded content may pay from ad or infrastructure pool
self-traffic must not become profitable
node rewards require audits and caps
```

Node rewards must never mutate ledger directly.

Node rewards must go through rewarder planning, policy validation, wallet execution, and ledger receipt.

---

## 18. Moderation doctrine

Moderation should be high-value but reviewable.

Moderators should not be paid merely for raw action volume.

Rewardable moderation outcomes may include:

```text
confirmed abuse report
spam cluster confirmed
appeal reviewed
false positive corrected
dangerous content removed
repeat abuser identified
policy dispute resolved
community queue triaged
```

Moderation payout should depend on:

```text
case difficulty
accuracy
appeal survival
reviewer reputation
queue priority
harm prevented
policy confidence
```

Bad-faith moderation should trigger:

```text
no reward
reputation loss
appeal review
possible bond loss in future bonded systems
moderator privilege reduction
```

Moderation rewards should generally settle after appeal windows where appropriate.

---

## 19. Curation and referral doctrine

Curation is valuable when it helps users discover content honestly.

Rewardable curation may include:

```text
share
quote share
playlist placement
collection inclusion
site embed
reference graph inclusion
creator onboarding
community onboarding
approved referral program
```

Curation must not become repost farming.

Useful curation should be measured by:

```text
qualified downstream engagement
source attribution correctness
non-spam behavior
viewer diversity
conversion quality
repeat decay
original creator protection
```

The original creator should usually receive the majority of downstream value.

---

## 20. Ads and burn doctrine

Ad campaigns are the primary planned burn sink.

Ad campaigns should not become pure inflation.

Illustrative doctrine posture only:

```text
burn_bps                    7000-8500
viewer_engagement_bps         500-1000
creator_site_placement_bps    500-1000
node_service_bps              300-700
moderation_reserve_bps        200-500
```

Rules:

```text
most ad ROC is burned
ad rewards are ad_budgeted
ad rewards do not drain protocol emission pools
ad campaigns must be explicit and paid
campaign terms must be visible before spend
abusive campaigns can be rejected or halted
```

Ad spending path:

```text
campaign prepare/quote
→ explicit advertiser confirmation
→ wallet hold/burn/capture path
→ ledger receipt
→ campaign delivery budget
→ ad-budgeted event accounting
→ burn/service/reward allocation
```

No silent ad spend.

No fake ad campaign receipts.

No ad reward from analytics-only events.

All actual ad burn rates and allocation splits must live in `configs/roc-economics.toml`.

---

## 21. Burn sinks

Possible ROC burn sinks:

```text
ad campaign launch
spammy high-volume publishing if configured
premium namespace registration
failed dispute/challenge bond
abusive moderation bond loss
optional boost/promotion
storage reservation expiration
future ROC/ROX conversion fee
```

Normal creator payments should not be over-burned.

Paid content should remain creator-first.

Illustrative doctrine posture only:

```text
creator_rights_holders_bps   8500-9500
node_service_bps              300-1000
protocol_burn_reserve_bps     100-500
moderation_trust_bps            0-200
```

Actual values must come from `configs/roc-economics.toml`.

---

## 22. User “mining” / earning doctrine

A normal user should be able to earn enough ROC to participate comfortably, but passive farming must not become the dominant strategy.

Recommended earning posture:

```text
casual honest user:
  earns enough for basic CrabLink participation

active good user:
  earns enough for regular participation and some paid unlocks

creator/moderator/node:
  earns meaningfully more because they provide scarce value
```

Passive browsing should be low.

Quality commenting should be medium only after gates.

Curation/sharing should be medium when it drives real qualified engagement.

Creation should be high.

Moderation should be high but reviewable.

Node service should be high but audited.

The system should not promise that passive viewing alone can sustain heavy paid consumption.

---

## 23. Emission and halving doctrine

A rigid Bitcoin-style halving is not recommended for internal ROC rewards.

ROC is an internal utility/accounting/reward token for a living creator network.

Prefer:

```text
capped epoch reward pools
gradual emission decay
burn-adjusted supply targets
category budgets
stability buffer
anti-farming limits
seasonal tuning
```

Illustrative season model only:

```text
Season 1:
  bootstrap creator/node/moderation rewards

Season 2:
  reduce passive attention rewards

Season 3:
  tune creator/reuse/node pools

Season 4:
  lower emissions as ad burns and paid content grow
```

All season/emission values must be config-controlled.

No hard-coded halving schedule unless governance later decides it.

---

## 24. Future ROC/ROX bridge economics

The ROC/ROX bridge remains future/deferred.

This guide may document future economics placeholders, but it does not authorize bridge code.

Possible future bridge directions:

```text
ROC → ROX
ROX → ROC
```

Possible future implementation path:

```text
Solana Anchor-based program for ROX
```

In this context, “Solana Anchor” means the Solana smart contract/program framework.

This is distinct from QuickChain Phase 5 “anchor-only” commitments, where “anchor” means compact external commitment/timestamp/proof evidence.

Future bridge fee/burn config fields may include:

```text
roc_to_rox_burn_bps
rox_to_roc_burn_bps
bridge_fee_bps
bridge_min_fee_minor
bridge_max_fee_minor
bridge_challenge_window_epochs
bridge_daily_limit_minor
bridge_per_user_limit_minor
bridge_pause_enabled
bridge_emergency_halt_enabled
bridge_remainder_sink
```

These fields must remain inert/deferred until a future bridge phase explicitly authorizes implementation.

---

## 25. Future bridge prerequisites and active-scope boundary

A bidirectional ROC/ROX bridge may only be considered after:

```text
Phase 5 Round 1 anchor-only dry-run is complete.
Phase 5 Round 2 DA/archive/challenge fallback is complete.
Phase 5 Round 3 chosen external posture is complete.
Internal ROC value plane is stable.
Wallet/ledger replay and conservation are proven.
Bridge threat model is written.
Bridge challenge windows are defined.
Bridge halt/pause/governance controls are defined.
Bridge accounting and audit events are defined.
Bridge smart contract/program is independently audited.
Solana/ROX risk model is accepted.
Public/legal/product language is reviewed.
```

The bridge must never bypass:

```text
svc-wallet
ron-ledger
explicit user confirmation
challenge/finality windows
audit trail
policy gates
emergency halt controls
```

Until a later phase explicitly authorizes bridge work, the correct status remains:

```text
Future/deferred bridge design only.
Internal ROC first.
QuickChain Phase 5 proceeds as written.
```

This guide must not be used as permission to add:

```text
ROX active runtime
Solana active runtime
Solana Anchor program code
bridge mint/burn code
external settlement
exchange-facing logic
liquidity
staking
public validator economy
gateway direct ledger mutation
omnigate direct ledger mutation
index settlement truth
policy settlement truth
CrabLink bridge authority
client-side bridge mint/burn
cache-only bridge proof
fake receipts
fake balances
fake finality
silent spend
```

---

## 26. Anti-farming doctrine

Anti-farming must be explicit and auditable.

Core defenses:

```text
minimum dwell thresholds
completion thresholds
rate caps per passport/site/content/day
repeat decay
reputation multipliers
quality multipliers
scarcity multipliers
sybil multipliers
random audit
challenge windows
node delivery verification
moderation appeal windows
bot-labeled traffic rejection
analytics-only quarantine
```

Do not rely on one signal.

Do not use raw IP as the only anti-sybil tool.

Do not let the frontend decide farming status.

Do not let local cache decide payout eligibility.

Do not let index pointers become payout proof.

---

## 27. Privacy and passport doctrine

Reward systems can create privacy risk.

The system must preserve:

```text
no public main-alt passport linkage by default
no private keys in UI state
no spend authority in React/localStorage/URLs/logs
no public correlation of private browsing/payment history
no unnecessary high-cardinality labels
capability-scoped proof views
redacted logs/errors
```

Alt passports must be separate by default.

Passports are identity/capability records, not wallets.

Wallets are economic authority.

A passport may reference a wallet account, but it is not itself spend authority.

---

## 28. Determinism and QuickChain-proofable artifacts

Future QuickChain commitments may include deterministic economics artifacts.

Examples:

```text
classified_event_snapshot_hash
reward_plan_hash
reward_pool_config_hash
manifest_split_hash
ad_burn_summary_hash
node_delivery_summary_hash
moderation_outcome_summary_hash
epoch_payout_plan_hash
ledger_receipt_root
burn_receipt_root
```

Rules:

```text
canonical bytes before hashes
no placeholder hashes
b3 lowercase hashes only
unknown fields rejected
integer minor units only
basis points only
replayable from published artifacts
no root-producing code without locked vectors
```

QuickChain may prove that the planned and executed economy was deterministic.

QuickChain must not replace wallet/ledger truth.

---

## 29. Rounding and remainder doctrine

All payout math must define:

```text
input units
multipliers
division order
rounding mode
remainder sink
```

Recommended default:

```text
floor division for individual payouts
accumulate remainder
send remainder to configured sink
record remainder sink in epoch settlement summary
```

Forbidden:

```text
floating point payout math
hidden rounding
silent dust creation
implicit treasury capture
implicit burn
unrecorded remainder
```

Remainder behavior must be testable.

---

## 30. Suggested DTO families for later

This guide does not require immediate DTO implementation.

Later, after doctrine and TOML shape are stable, DTOs may include:

```text
ManifestSplitV1
PayoutSplitV1 extension/generalization
EpochRewardPoolConfigV1
EpochRewardPlanV1
RewardPointSummaryV1
RewardPointConversionV1
AdBudgetSplitV1
BurnPolicyV1
NodeRewardSummaryV1
ModerationRewardSummaryV1
AntiFarmingPolicySummaryV1
RemainderSinkV1
```

Preferred module ownership:

```text
ron-proto/src/asset/payout.rs:
  asset/manifest split primitives

ron-proto/src/econ/:
  epoch reward plans, pool configs, burn policies, ad budget splits, reward point summaries

ron-proto/src/quickchain/:
  only later: commitment/root/anchor DTOs proving deterministic epoch artifacts
  only after Phase 5 Round 1 canonical bytes and vectors are locked
```

QuickChain should commit/prove the economy later.

QuickChain should not become the economy itself.

Commitment-related QuickChain DTOs should not be added until the relevant Phase 5 Round 1 canonical payloads, domain separators, locked bytes, and locked hashes are stable.

---

## 31. Testing requirements

Future tokenomics tests should include:

```text
TOML parses valid config
TOML rejects unknown fields
TOML rejects floats
TOML rejects invalid basis-point totals
TOML rejects missing remainder sink
TOML rejects negative or malformed money strings
TOML rejects bridge-enabled config unless feature gate allows it
category budgets total correctly
manifest splits total exactly 10000 bps
ad campaign splits total exactly 10000 bps
paid content splits total exactly 10000 bps
reward point conversion is deterministic
rounding remainder is deterministic
anti-farming caps are enforced
analytics_only never becomes reward material
metering never directly becomes payout
proof_eligible requires verification
ad_budgeted uses ad budget not protocol pool
svc-rewarder cannot mutate ledger
ron-accounting cannot become balance truth
ron-policy cannot create receipt/balance/finality
CrabLink cannot create payout/balance/receipt truth
```

---

## 32. No-authority boundaries

This guide does not grant authority.

Boundaries:

```text
ron-proto:
  DTOs only

ron-policy:
  config validation and declarative rules only

ron-accounting:
  snapshots and reports only

svc-rewarder:
  payout planning only

svc-wallet:
  mutation front-door

ron-ledger:
  durable economic truth

svc-storage:
  bytes/artifacts only

svc-index:
  lookup/pointer only

svc-gateway:
  public boundary only

omnigate:
  hydration/access composition only

CrabLink:
  display/user intent only
```

Forbidden:

```text
rewarder direct ledger mutation
accounting direct ledger mutation
policy direct ledger mutation
gateway direct ledger mutation
omnigate direct ledger mutation
index as payment truth
storage as payment truth
cache-only paid unlock
client-side payout truth
fake receipts
fake balances
fake finality
silent spend
```

---

## 33. Implementation order

Recommended order:

```text
1. Keep this guide as doctrine.
2. Keep configs/roc-economics.toml as mutable economics source.
3. Validate current ron-policy economics config behavior.
4. Continue QuickChain Phase 5 Round 1 as anchor-only dry-run.
5. Add tokenomics DTO refinements only after this guide and TOML shape are stable.
6. Add rewarder planning changes only after DTO/config validation is stable.
7. Add wallet/ledger execution paths only through explicit approved mutations.
8. Add QuickChain commitment/proof links only after deterministic vectors are locked.
9. Keep bridge fields inert until a future bridge phase is authorized.
```

---

## 34. One-line summary

The internal ROC economy is a capped, configurable, manifest-aware, anti-farming reward system where raw engagement becomes classified evidence, rewarder plans from bounded pools, policy validates, wallet executes, ledger records truth, and QuickChain may later prove deterministic artifacts without replacing wallet/ledger authority.

---
