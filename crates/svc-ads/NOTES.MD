Recommended drop-in path:

```text id="hkjq0q"
crates/svc-ads/docs/INTERNAL_ROC_BETA_ADS_FEATURE_GATE.md
```

# INTERNAL ROC BETA — `svc-ads` Feature-Gate / Parking Notes

## Status

```text
svc-ads Internal ROC Beta ads surface: FEATURE-GATED / PARKED
```

`svc-ads` is intentionally deferred until the core Internal ROC Beta value loop is closed out and reproducible. Ads must remain disabled by default and must not become a shortcut around the wallet, ledger, policy, accounting, or rewarder boundaries.

This note exists so `svc-ads` can be resumed later without architecture drift.

---

## Current decision

Ads are not being implemented as an active Phase 5 requirement.

Instead:

```text
svc-ads is parked behind an explicit feature gate.
ad_budgeted event semantics are preserved.
ad_budgeted reward material is allowed only as future explicit-budget material.
ad_budgeted must never become raw protocol ROC emission.
```

The current Internal ROC Beta Phase 5 anti-farming spine is:

```text
event classification
→ anti-farming caps
→ ron-policy declarative gate
→ svc-rewarder deterministic non-mutating plan
→ svc-wallet approved execution only
→ ron-ledger durable receipt/balance truth
```

`svc-ads` must fit into that spine later. It must not create a parallel one.

---

## Why this is parked

The current priority is proving the internal ROC value loop before adding a live advertising surface.

Ads introduce extra risk:

```text
budget custody risk
impression/click farming risk
raw engagement gaming risk
creator/reward manipulation risk
silent spend risk
privacy/targeting risk
policy bypass risk
fake receipt/balance risk
```

Until the beta value loop is reproducible, `svc-ads` should remain dormant.

---

## Required feature-gate posture

When resumed, `svc-ads` should have both:

```text
compile-time feature gate
runtime config gate
```

Suggested compile-time feature name:

```text
internal-roc-ads-beta
```

Suggested runtime config key:

```toml
[ads]
enabled = false
internal_roc_ads_beta = false
```

Suggested environment override:

```text
SVC_ADS_INTERNAL_ROC_BETA=0
```

Default behavior must be:

```text
disabled
fail closed
no budget acceptance
no reward planning output
no wallet calls
no ledger calls
no active campaign serving
no hidden background spend
```

If the feature is disabled, any active endpoint should return a safe disabled response such as:

```text
404 Not Found
or
403 Forbidden
or
501 Not Implemented
```

It must not silently run in degraded mode.

---

## Hard boundary

`svc-ads` is not economic truth.

`svc-ads` must not become:

```text
wallet authority
ledger authority
receipt authority
balance authority
finality authority
paid entitlement authority
reward authority
policy authority
accounting truth
protocol minting authority
staking authority
bridge authority
settlement authority
```

`svc-ads` may eventually coordinate ad campaign metadata and budgeted delivery, but only after wallet/ledger-backed budget proof exists.

---

## Allowed future role

When resumed, `svc-ads` may eventually do these things:

```text
create draft ad campaign metadata
quote campaign budget requirements
prepare budget requests
record display-safe campaign state
emit metering/analytics/ad_budgeted event candidates
enforce campaign caps
pause exhausted campaigns
surface budget status derived from backend truth
produce audit records for campaign delivery
```

But only if all monetary state comes from the wallet/ledger path.

---

## Forbidden role

`svc-ads` must never do these things:

```text
mint ROC
issue ROC
transfer ROC
burn ROC
open wallet holds directly
capture wallet holds directly
release wallet holds directly
mutate ledger
invent budget balance
invent ad spend receipt
invent campaign finality
invent payout entitlement
unlock paid content
grant paid access
reward raw impressions
reward raw clicks
reward likes/shares/comments directly
turn analytics into protocol ROC
use protocol reward pool for ad_budgeted payouts
bridge ROC externally
settle externally
enable staking
enable liquidity
```

All economic effects must go through:

```text
svc-wallet
→ ron-ledger
```

All reward planning must go through:

```text
ron-accounting / verified inputs
→ ron-policy
→ svc-rewarder
```

---

## Required future ad-budgeted doctrine

`ad_budgeted` means:

```text
explicit payer-authorized budget material
```

It does not mean:

```text
protocol reward pool material
```

It does not mean:

```text
engagement reward material
```

It does not mean:

```text
automatic ROC emission
```

The safe interpretation is:

```text
An advertiser or campaign owner explicitly funded a budget through the wallet/ledger path, and later verified/capped delivery facts may be used to distribute from that explicit budget only.
```

The unsafe interpretation is forbidden:

```text
A user clicked/viewed/engaged, therefore protocol ROC is minted or allocated.
```

---

## Future budget lifecycle

If `svc-ads` is resumed, the safe campaign budget lifecycle should look like this:

```text
draft campaign
→ quote required budget
→ explicit confirmation by payer
→ svc-wallet hold/open/transfer/capture path
→ ron-ledger durable receipt
→ backend-derived budget state
→ campaign activation
→ bounded delivery
→ verified/capped delivery accounting
→ ron-policy ad_budgeted gate
→ svc-rewarder non-protocol budget plan
→ svc-wallet approved payout execution if applicable
→ ron-ledger receipt/balance truth
```

The unsafe lifecycle is forbidden:

```text
draft campaign
→ local budget counter
→ serve ads
→ local spend decrement
→ local receipt
→ direct reward
```

---

## Campaign state must be display-only unless backed by wallet/ledger

Suggested campaign states:

```text
draft
quoted
awaiting_confirmation
budget_pending
budget_funded
active
paused
exhausted
closed
rejected
```

Important rule:

```text
Only wallet/ledger-backed state may imply spendable budget.
```

`svc-ads` may cache display state, but cached state must not be entitlement, balance, or spend authority.

---

## Required budget truth source

Budget truth must come from:

```text
svc-wallet receipt / hold / capture / transfer result
ron-ledger durable operation truth
backend-derived balance or budget query
```

Budget truth must not come from:

```text
React state
localStorage
browser cache
svc-ads local counter alone
gateway cache
omnigate cache
policy decision
rewarder manifest
accounting snapshot
analytics event
```

---

## Event-class mapping

`svc-ads` must respect the Internal ROC event classes:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

Expected ad event handling:

```text
ad impression                 → analytics_only or metering
ad click                      → analytics_only or metering
ad hover                      → analytics_only
ad scroll                     → analytics_only
ad view duration              → metering or analytics_only
campaign budget authorized    → ad_budgeted only if wallet/ledger-backed
campaign spend receipt        → economic_receipt only if wallet/ledger-backed
verified delivery candidate   → proof_eligible or ad_budgeted only after verification/caps/policy
```

Forbidden handling:

```text
raw impression → protocol payout
raw click → protocol payout
raw view → protocol payout
raw engagement → protocol payout
analytics_only → rewarder contribution
metering → direct rewarder contribution
ad_budgeted → protocol pool payout
```

---

## Integration with `ron-accounting`

Future `svc-ads` may emit accounting material only as classified, bounded facts.

Allowed:

```text
campaign_id
budget_id
payer_account reference
publisher/content reference
event_class
delivery counters
timestamp/epoch
verification status
cap status
budget source label
```

Forbidden:

```text
receipt truth
balance truth
finality truth
wallet authority
ledger authority
private keys
capability secrets
raw auth tokens
uncapped spend authority
```

`ron-accounting` remains snapshot/reporting only. It does not become balance truth.

---

## Integration with `ron-policy`

`ron-policy` must gate ad-budgeted eligibility declaratively.

Policy may say:

```text
allow
deny
requires explicit budget
requires non-protocol budget
requires verified delivery
requires cap check
requires wallet execution later
```

Policy must not say:

```text
issue
transfer
burn
capture
release
mint
settle
unlock
finalize
create receipt
set balance
```

Policy decisions are gates, not execution.

---

## Integration with `svc-rewarder`

`svc-rewarder` may consume ad-budgeted material only if all are true:

```text
event_class = ad_budgeted
verified = true
explicit_budget_authorized = true
policy_gate_passed = true
funding_source != ProtocolPool
anti-farming caps passed
```

`svc-rewarder` must reject ad-budgeted material if:

```text
budget is missing
budget is local-only
budget is not wallet/ledger-backed
policy gate is missing
verification is missing
caps are missing
funding source is protocol pool
```

Rewarder output remains a deterministic plan/handoff only.

---

## Integration with `svc-wallet`

`svc-ads` must not mutate wallet state directly.

If ads need funding, the flow must be:

```text
prepare/quote
→ explicit confirmation
→ svc-wallet
→ ron-ledger
→ backend receipt
→ backend-derived campaign budget state
```

Any spend, refund, payout, or capture must be wallet-mediated.

No direct calls to ledger mutation paths are allowed.

---

## Integration with `ron-ledger`

`ron-ledger` remains durable economic truth.

`svc-ads` must not write ledger records directly.

Any ad budget receipt, budget capture, campaign payout, campaign refund, or budget closeout must be represented as wallet-mediated ledger truth.

---

## Integration with `svc-gateway` / `omnigate`

Gateway and omnigate may eventually expose ads API boundaries, but they must not become budget authority.

Allowed:

```text
authenticated routing
policy-aware forwarding
redacted errors
bounded body handling
backend-derived campaign display
```

Forbidden:

```text
gateway-issued budget
gateway-issued receipt
gateway-created unlock
omnigate-created budget
omnigate-created receipt
cache-only campaign spend
cache-only ad entitlement
```

---

## Integration with CrabLink Tauri

CrabLink may eventually display campaign UX, but must remain display/user-intent only.

Allowed:

```text
campaign draft UI
budget quote display
explicit confirmation prompt
backend-derived campaign status display
backend-derived receipt display
```

Forbidden:

```text
local budget authority
local spend authority
local receipt truth
silent spend
auto-confirmed budget
raw wallet secrets in React
capability tokens in React state/localStorage/URL/logs
cache-only budget unlock
```

---

## Privacy / targeting boundary

Ads must not introduce uncontrolled targeting or surveillance behavior.

When resumed, `svc-ads` should prefer:

```text
contextual placement
content/category targeting
campaign-level caps
coarse aggregate analytics
short retention windows
redacted logs
bounded identifiers
```

Avoid:

```text
sensitive personal targeting
precise location targeting
cross-site behavioral profiles
raw user event logs as economic material
unbounded clickstream storage
private data in campaign proofs
```

No private user attributes should be required to prove ad-budgeted spend.

---

## Anti-farming requirements

Before any ad-budgeted event can affect payouts or campaign accounting, it must pass anti-farming gates.

Required future checks:

```text
per-account caps
per-content caps
per-campaign caps
per-epoch caps
duplicate event detection
replay protection
budget exhaustion check
publisher self-click/self-view controls
bot/spam throttling
rate limits
minimum delivery validity rules
clock/epoch sanity
campaign status active check
```

Raw engagement must remain quarantined unless transformed through verified/capped, budget-authorized lanes.

---

## Idempotency and replay requirements

Future ad operations must use stable IDs.

Suggested fields:

```text
campaign_id
budget_id
delivery_event_id
operation_id
idempotency_key
account_sequence if ledger-derived
hold_id if wallet hold lifecycle is used
epoch_id
```

Rules:

```text
operation_id is backend-assigned durable ledger/wallet operation ID.
idempotency_key is retry protection only, not authority.
hold_id belongs to one wallet hold lifecycle.
expiry must be epoch/time based.
```

No event may become economic authority merely because it has an idempotency key.

---

## Integer money requirements

All money must be integer minor-unit strings.

Allowed:

```json
"budget_minor": "1000000"
"spend_cap_minor": "250000"
"remaining_budget_minor": "750000"
```

Forbidden:

```json
"budget": 1.0
"budget_minor": 1000000
"budget_minor": "1.25"
"budget_minor": "001000000"
```

No floats. No lossy decimal math. No JavaScript number money authority.

---

## Suggested future DTO posture

Future DTOs should be strict and deny unknown fields.

Safe draft campaign DTO shape may include:

```text
campaign_id
payer_account
publisher_account or content/site target
budget_quote_minor
max_spend_minor
start_epoch
end_epoch
creative_b3
placement_kind
status
```

Safe budget confirmation DTO shape may include:

```text
campaign_id
quote_id
payer_account
amount_minor
idempotency_key
explicit_confirmation = true
```

Safe delivery event DTO shape may include:

```text
campaign_id
delivery_event_id
event_class
placement_id
content_b3 or site_name
epoch_id
count
verified
cap_checked
policy_gate_passed
budget_id
```

Forbidden DTO fields:

```text
wallet_private_key
wallet_seed
raw_capability
ledger_mutation
mint_authority
issue_authority
transfer_authority
receipt_truth
balance_truth
finality
unlock_granted
staking_position
bridge_proof
external_settlement
liquidity_pool
protocol_pool_ad_budget
```

---

## Required disabled-mode tests

When `svc-ads` is implemented or wired, add tests proving disabled behavior.

Required tests:

```text
ads_feature_disabled_by_default
ads_routes_fail_closed_when_disabled
ads_disabled_mode_makes_no_wallet_calls
ads_disabled_mode_makes_no_ledger_calls
ads_disabled_mode_emits_no_rewarder_candidates
ads_disabled_mode_does_not_claim_budget_truth
```

Expected proof:

```text
feature disabled → no budget accepted
feature disabled → no active campaign
feature disabled → no spend
feature disabled → no wallet mutation
feature disabled → no ledger mutation
```

---

## Required enabled-mode tests for later

When explicitly resumed, add tests proving enabled behavior is still bounded.

Required tests:

```text
ads_budget_prepare_is_quote_only
ads_budget_confirmation_requires_explicit_user_intent
ads_budget_confirmation_uses_svc_wallet_only
ads_budget_receipt_is_backend_derived_only
ads_budget_state_is_ledger_wallet_derived_only
ads_delivery_events_are_analytics_or_metering_by_default
ads_raw_engagement_never_enters_rewarder
ads_ad_budgeted_requires_explicit_budget
ads_ad_budgeted_requires_non_protocol_funding
ads_ad_budgeted_requires_policy_gate
ads_campaign_exhaustion_stops_delivery
ads_replay_does_not_double_spend
ads_idempotency_key_is_not_authority
ads_dto_rejects_authority_smuggling_fields
ads_no_bridge_staking_liquidity_or_external_settlement
```

---

## Required preflight script for later

Suggested future script:

```text
crates/svc-ads/scripts/dev-internal-roc-beta-ads-feature-gate-preflight.sh
```

It should run:

```text
cargo fmt -p svc-ads -- --check
cargo test -p svc-ads --test internal_roc_beta_ads_feature_gate
cargo test -p svc-ads --test internal_roc_beta_ads_budget_non_authority
cargo test -p svc-ads --test internal_roc_beta_ads_event_class_boundary
cargo test -p svc-ads --test internal_roc_beta_ads_no_direct_wallet_ledger_mutation
cargo clippy -p svc-ads --all-targets -- -D warnings
```

It should also scan for forbidden Python helpers if we preserve the current bash/cargo-only posture.

---

## Source-scan forbidden phrases for future tests

Future `svc-ads` source boundary tests should reject suspicious direct authority names unless only present in tests/docs/deny lists.

Forbidden examples:

```text
direct_ledger_mutation
ledger_mutate
mutate_ledger
issue_roc
mint_roc
transfer_roc
burn_roc
create_receipt
fake_receipt
set_balance
wallet_balance_truth
ledger_balance_truth
finality_truth
unlock_paid_content
staking_position
enable_staking
bridge_settlement
external_settlement
liquidity_pool
protocol_pool_ad_budget
raw_click_reward
raw_view_reward
raw_engagement_reward
```

---

## Safe logging posture

Logs must be redacted.

Allowed:

```text
campaign_id
budget_id
epoch_id
status
low-cardinality reason
bounded count
correlation id
```

Forbidden:

```text
private keys
wallet seeds
raw capabilities
authorization tokens
macaroons
uncapped spend authority
full user profiles
precise targeting data
raw clickstream payloads
```

---

## Metrics posture

Metrics must be low-cardinality.

Allowed metric labels:

```text
status
reason
event_class
route
campaign_state
```

Avoid metric labels:

```text
account
campaign_id
wallet
receipt_id
content_b3
user_id
ip_address
raw URL
```

High-cardinality identifiers belong in audit records, not Prometheus labels.

---

## Audit posture

If `svc-ads` emits audit records later, they should be append-only and display-safe.

Suggested audit events:

```text
campaign_drafted
budget_quoted
budget_confirmation_requested
budget_funded_wallet_derived
campaign_activated
campaign_paused
campaign_exhausted
delivery_event_rejected
delivery_event_capped
ad_budgeted_candidate_emitted
campaign_closed
```

Audit records must not be receipt truth unless tied to wallet/ledger receipt IDs as references.

---

## Config doctrine

Suggested future config shape:

```toml
[ads]
enabled = false
internal_roc_ads_beta = false
max_campaigns_per_account = 10
max_active_campaigns = 100
max_events_per_campaign_per_epoch = 10000
max_budget_minor_per_campaign = "100000000"
min_budget_minor_per_campaign = "1"
default_epoch_ttl = 24

[ads.feature_gates]
ad_budgeted_events = false
wallet_budget_funding = false
rewarder_ad_budgeted_output = false
campaign_serving = false
```

Rules:

```text
all money is string minor units
all gates default false
missing config fails closed
enabled bridge/staking/liquidity config rejects
```

---

## Resume prerequisites

Before resuming `svc-ads`, confirm:

```text
Internal ROC Beta Phase 5 closeout is green.
Phase 6 reproducible smoke suite is planned or green.
svc-wallet paid mutation path is stable.
ron-ledger replay/conservation remains green.
ron-accounting snapshots remain read-only.
svc-rewarder anti-farming policy-gated path remains green.
ron-policy rejects authority-shaped tags/obligations.
CrabLink Tauri remains display/user-intent only.
```

Do not resume ads if any of these regress.

---

## Codebundle needed when resumed

When returning to `svc-ads`, bring:

```text
svc-ads CODEBUNDLE.md
svc-wallet CODEBUNDLE.md
ron-ledger CODEBUNDLE.md
ron-accounting CODEBUNDLE.md
svc-rewarder CODEBUNDLE.md
ron-policy CODEBUNDLE.md
svc-gateway CODEBUNDLE.md
omnigate CODEBUNDLE.md
CrabLink Tauri CODEBUNDLE.md if UI is involved
INTERNAL_ROC_BETA_BUILDPLAN.md
INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
INTERNAL_ROC_MODEL.md
POST_QUICKCHAIN_DECISION_GATE.md
```

---

## First safe future implementation slice

The first future slice should not serve real ads.

Recommended first slice:

```text
disabled-by-default feature gate
config validator
route disabled behavior
DTO authority-smuggling rejects
ad_budgeted event classification stub
no wallet calls
no ledger calls
no rewarder calls
preflight script
```

Expected result:

```text
svc-ads exists as a parked, test-proven, non-authoritative ads boundary.
```

Do not start with:

```text
campaign serving
budget capture
reward payout
ad marketplace
auction logic
targeting engine
external settlement
```

---

## Second safe future implementation slice

Only after the first slice is green:

```text
prepare budget quote
explicit confirmation DTO
svc-wallet-only budget funding seam
backend-derived budget status display DTO
idempotency/replay tests
no active delivery yet
```

Expected result:

```text
Budget can be prepared and funded only through wallet/ledger truth, without serving ads or issuing rewards.
```

---

## Third safe future implementation slice

Only after budget funding is green:

```text
campaign active/paused/exhausted state machine
bounded delivery counters
analytics_only and metering event output
anti-farming cap enforcement
no rewarder output yet
```

Expected result:

```text
Ads can record bounded delivery facts, but cannot payout.
```

---

## Fourth safe future implementation slice

Only after delivery caps are green:

```text
verified ad_budgeted candidate output
ron-policy gate required
svc-rewarder non-protocol budget path required
protocol pool rejection
dry-run manifest only
```

Expected result:

```text
Ad-budgeted delivery can become a deterministic non-mutating payout plan funded only by explicit non-protocol budget.
```

---

## Final reminder

`svc-ads` must be treated as a risky edge service.

Safe mantra:

```text
Ads may coordinate campaigns.
Ads may classify delivery facts.
Ads may reference explicit wallet-funded budgets.
Ads may never invent money, receipts, balances, finality, entitlement, or rewards.
```

The only acceptable economic truth path remains:

```text
svc-wallet
→ ron-ledger
```

The only acceptable reward planning path remains:

```text
ron-accounting
→ ron-policy
→ svc-rewarder
→ svc-wallet
→ ron-ledger
```

Until explicitly resumed, `svc-ads` remains:

```text
FEATURE-GATED
DISABLED BY DEFAULT
NON-AUTHORITATIVE
NO WALLET MUTATION
NO LEDGER MUTATION
NO RAW ENGAGEMENT REWARDS
NO PROTOCOL-POOL AD PAYOUTS
NO BRIDGE
NO STAKING
NO LIQUIDITY
NO EXTERNAL SETTLEMENT
```

This gives us a clean parking marker: `svc-ads` can wait without losing the ad-budgeted doctrine we just locked into `ron-policy + svc-rewarder`.
