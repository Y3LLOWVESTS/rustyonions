# Internal ROC Beta — svc-ads Feature-Gate / Parking Note

## Status

svc-ads status: FEATURE-GATED / PARKED

svc-ads is intentionally not active for Internal ROC Beta Phase 5 closeout.

This crate may exist in the workspace, but it must not become an active ads runtime, budget authority, reward authority, wallet authority, ledger authority, paid entitlement authority, bridge authority, staking authority, or external settlement surface during Phase 5.

## Current decision

Ads are deferred.

Phase 5 closes the tokenomics config and anti-farming proof without activating ads.

The current doctrine is:

- svc-ads must remain disabled by default.
- ad_budgeted semantics are preserved for future use.
- ad_budgeted means explicit payer-authorized budget material.
- ad_budgeted must not mean protocol-pool emission.
- raw impressions, clicks, views, likes, shares, comments, hovers, scrolls, or watch-time must not directly mint, allocate, or plan protocol ROC.
- analytics_only and metering remain non-payout material.
- proof_eligible requires verification, caps, and ron-policy gate.
- svc-rewarder may only consume ad_budgeted material later if explicit non-protocol budget is present.

## Required gate posture

svc-ads must eventually be controlled by both:

- compile-time feature gate
- runtime config gate

Suggested compile-time feature:

- internal-roc-ads-beta

Suggested runtime default:

- ads.enabled = false
- ads.internal_roc_ads_beta = false

Default behavior must be fail-closed.

When disabled, svc-ads must not:

- accept active ad budgets
- activate campaigns
- serve paid ads
- emit rewarder payout material
- call svc-wallet
- call ron-ledger
- create receipt truth
- create balance truth
- create finality truth
- unlock paid content
- perform settlement
- enable bridge behavior
- enable staking behavior
- enable liquidity behavior

## Hard authority boundary

svc-ads is not economic truth.

svc-ads must not become:

- wallet authority
- ledger authority
- receipt authority
- balance authority
- finality authority
- payout authority
- reward authority
- policy authority
- accounting truth
- paid entitlement authority
- protocol minting authority
- bridge authority
- staking authority
- liquidity authority
- external settlement authority

The only valid economic truth path remains:

svc-wallet -> ron-ledger

The only valid reward planning path remains:

ron-accounting -> ron-policy -> svc-rewarder -> svc-wallet -> ron-ledger

## Allowed future role

When explicitly resumed later, svc-ads may coordinate:

- draft campaign metadata
- quote requests
- explicit budget confirmation requests
- wallet/ledger-derived budget status display
- bounded campaign state
- bounded delivery counters
- classified ad events
- audit records
- disabled-mode routes
- policy-gated ad_budgeted candidate preparation

But all monetary authority must come from wallet/ledger-backed truth.

## Forbidden future role

svc-ads must never:

- mint ROC
- issue ROC
- transfer ROC
- burn ROC
- mutate ledger
- open holds directly
- capture holds directly
- release holds directly
- invent budget balance
- invent ad spend receipt
- invent campaign finality
- invent payout entitlement
- unlock paid content
- reward raw impressions
- reward raw clicks
- reward raw engagement
- use protocol reward pool for ad_budgeted payouts
- bridge ROC externally
- settle externally
- enable staking
- enable liquidity
- expose exchange-facing behavior

## ad_budgeted doctrine

Safe interpretation:

An advertiser or campaign owner explicitly funded a budget through the wallet/ledger path, and later verified/capped delivery facts may be used to distribute from that explicit budget only.

Forbidden interpretation:

A user clicked, viewed, liked, shared, commented, watched, hovered, scrolled, or otherwise engaged, therefore protocol ROC is minted, issued, allocated, or planned.

## Future safe lifecycle

The only safe future lifecycle is:

1. draft campaign
2. prepare quote
3. explicit payer confirmation
4. svc-wallet budget operation
5. ron-ledger durable operation truth
6. backend-derived campaign budget state
7. campaign activation
8. bounded delivery
9. verified/capped delivery accounting
10. ron-policy ad_budgeted gate
11. svc-rewarder non-protocol-budget plan
12. svc-wallet approved execution if applicable
13. ron-ledger receipt/balance truth

Forbidden shortcut:

draft campaign -> local budget counter -> serve ads -> local spend decrement -> local receipt -> direct reward

## Event-class mapping

Ad impression:

- analytics_only by default
- metering only if used for bounded measurement
- never direct payout material

Ad click:

- analytics_only by default
- metering only if used for bounded measurement
- never direct payout material

Ad hover / scroll / passive engagement:

- analytics_only only
- never direct payout material

Campaign budget authorized:

- ad_budgeted only if wallet/ledger-backed explicit budget exists

Campaign spend receipt:

- economic_receipt only if backend-derived from wallet/ledger truth

Verified delivery candidate:

- proof_eligible or ad_budgeted only after verification, caps, policy gate, and explicit budget rules

## Required anti-farming rules for later

Before any future ad_budgeted material can affect reward planning, it must pass:

- per-account caps
- per-content caps
- per-campaign caps
- per-epoch caps
- duplicate detection
- replay protection
- campaign active-state check
- budget exhaustion check
- publisher self-click/self-view controls
- bot/spam throttling
- clock/epoch sanity
- ron-policy gate
- explicit non-protocol budget gate

## Required wallet/ledger boundary

svc-ads must not call ledger mutation paths.

If funding is needed later, the flow must be:

prepare/quote -> explicit confirmation -> svc-wallet -> ron-ledger -> backend receipt -> backend-derived campaign budget state

No direct ron-ledger mutation from svc-ads is allowed.

## Required policy/rewarder boundary

ron-policy may gate ad eligibility declaratively.

ron-policy may not execute:

- issue
- transfer
- burn
- capture
- release
- mint
- settle
- unlock
- finalize
- create receipt
- set balance

svc-rewarder may consume ad_budgeted material only if:

- event_class = ad_budgeted
- verified = true
- explicit_budget_authorized = true
- policy_gate_passed = true
- funding_source != ProtocolPool
- anti-farming caps passed

svc-rewarder must reject ad_budgeted material if:

- budget is missing
- budget is local-only
- budget is not wallet/ledger-backed
- policy gate is missing
- verification is missing
- caps are missing
- funding source is protocol pool

## Required disabled-mode tests for later

When svc-ads implementation resumes, add tests proving:

- ads disabled by default
- disabled routes fail closed
- disabled mode makes no wallet calls
- disabled mode makes no ledger calls
- disabled mode emits no rewarder candidates
- disabled mode does not claim budget truth
- disabled mode does not claim receipt truth
- disabled mode does not claim balance truth
- disabled mode does not claim finality truth

## Required enabled-mode tests for later

Only after explicit authorization to resume ads, add tests proving:

- budget prepare is quote-only
- budget confirmation requires explicit user intent
- budget confirmation uses svc-wallet only
- budget receipt is backend-derived only
- budget state is wallet/ledger-derived only
- raw engagement never enters rewarder
- ad_budgeted requires explicit budget
- ad_budgeted requires non-protocol funding
- ad_budgeted requires policy gate
- campaign exhaustion stops delivery
- replay does not double-spend
- idempotency key is not authority
- DTOs reject authority-smuggling fields
- no bridge, staking, liquidity, or external settlement behavior exists

## Privacy and targeting boundary

svc-ads must not introduce uncontrolled targeting or surveillance.

Prefer later:

- contextual placement
- coarse categories
- campaign-level caps
- aggregate analytics
- redacted logs
- short retention windows
- bounded identifiers

Avoid later:

- sensitive personal targeting
- precise location targeting
- cross-site behavioral profiles
- raw user event logs as economic material
- unbounded clickstream storage
- private data in campaign proofs

## Final parking statement

Until explicitly resumed, svc-ads remains:

- FEATURE-GATED
- DISABLED BY DEFAULT
- NON-AUTHORITATIVE
- NO WALLET MUTATION
- NO LEDGER MUTATION
- NO RAW ENGAGEMENT REWARDS
- NO PROTOCOL-POOL AD PAYOUTS
- NO BRIDGE
- NO STAKING
- NO LIQUIDITY
- NO EXTERNAL SETTLEMENT

This parking note exists only to preserve future ads doctrine while allowing Internal ROC Beta Phase 5 closeout to remain honest and reproducible.
