# svc-gateway QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary doctrine for `svc-gateway`.
RO:WHY — Keeps the public route surface from drifting into wallet, ledger, receipt, balance, root, finality, validator, bridge, cache, or external settlement authority.
RO:INTERACTS — svc-gateway routes, omnigate, svc-storage, svc-wallet, ron-ledger, ron-policy, ron-accounting, svc-rewarder, CrabLink/Tauri.
RO:INVARIANTS — svc-gateway is the public boundary; it may route, proxy, and fail closed, but it must not become wallet, ledger, receipt, balance, root, finality, validator, bridge, cache, or external settlement authority.
RO:METRICS — Existing gateway HTTP/correlation metrics only; metrics are not balance truth, receipt truth, or settlement finality.
RO:CONFIG — Uses configured upstreams for omnigate and svc-storage; config must not enable QuickChain runtime behavior.
RO:SECURITY — Caller-supplied receipt, root, finality, validator, bridge, settlement, balance, or cache-unlock claims are not authority.
RO:TEST — `quickchain_preflight_*` tests plus `scripts/dev-quickchain-preflight.sh` and `scripts/dev-quickchain-park.sh`.

## Status

`svc-gateway` is the public boundary.

`svc-gateway` exposes a public route surface and proxies to omnigate or svc-storage.

`svc-gateway` is not a chain runtime.
`svc-gateway` is not a validator.
`svc-gateway` is not a bridge.
`svc-gateway` is not an anchor writer.
`svc-gateway` is not a checkpoint writer.
`svc-gateway` is not a root producer.
`svc-gateway` is not a wallet mutation authority.
`svc-gateway` is not a ledger mutation authority.
`svc-gateway` is not finality authority.

Plain scanner phrase: svc-gateway is the public boundary.
Plain scanner phrase: svc-gateway does not mutate ledger.
Plain scanner phrase: svc-gateway does not mint ROC.
Plain scanner phrase: svc-gateway does not issue wallet receipts.
Plain scanner phrase: svc-gateway does not invent balances.
Plain scanner phrase: svc-gateway does not unlock paid content from cache alone.

## Boundary doctrine

Allowed Phase-0 gateway work:

- route exposure
- header filtering
- proxying to omnigate
- proxying raw object reads to svc-storage
- wallet hold route proxying to omnigate only
- public ingress fail-closed behavior
- route contract tests
- request/response DTO shape tests
- cache/header non-authority tests
- Bash/cargo-only dynamic preflight scripts

Forbidden gateway work:

- no direct svc-wallet mutation implementation
- no direct ron-ledger mutation implementation
- no fake balances
- no fake receipts
- no local entitlement truth
- no root-producing code
- no checkpoint-producing code
- no validator code
- no bridge or external settlement code
- no finality claims
- no ROX
- no Solana
- no staking
- no liquidity
- no exchange-facing logic

## Paid access doctrine

Paid access must fail closed.

Gateway may forward explicit user intent and backend-derived metadata. It cannot decide paid access from caller-supplied claims alone.

Safe gateway behavior:

- forward selected headers such as Authorization, x-ron-passport, x-ron-wallet-account, Idempotency-Key, and backend receipt metadata where explicitly allowed
- proxy paid/product routes to omnigate
- proxy raw object reads to svc-storage
- return upstream backend decisions honestly

Unsafe gateway behavior:

- trusting `paid=true`
- trusting `unlock=true`
- trusting `unlocked=true`
- trusting `x-ron-entitlement`
- trusting `x-ron-unlock-authorized`
- trusting local cache state as entitlement
- turning ETag or Cache-Control into payment proof
- treating b3 as payment proof
- treating crab:// navigation as economic authority

## Cache/proxy doctrine

Cache-Control, ETag, If-None-Match, and related transport headers are transport metadata only.

Cache cannot unlock paid content.
Cache is convenience only.
A b3 address proves byte identity, not economic entitlement.
b3 hashes are content truth, not economic truth.
b3 proves bytes.
b3 does not prove payment.
crab:// navigation is navigation, not spend authority.

## Transport/header doctrine

Transport headers are not economic authority.

Gateway request/response surfaces must strip or avoid authority fields such as:

- x-ron-operation-id
- x-ron-account-sequence
- x-ron-state-root
- x-ron-receipt-root
- x-ron-accounting-root
- x-ron-reward-root
- x-ron-checkpoint-hash
- x-ron-validator-*
- x-ron-bridge-*
- x-ron-anchor-*
- x-ron-quickchain-*
- x-ron-finality
- x-ron-finalized
- x-ron-balance
- x-ron-entitlement
- x-ron-unlock-authorized
- x-ron-ledger-*

operation_id is backend-assigned durable ledger operation identity.
idempotency-key is retry identity only.
Idempotency-Key is retry identity only.
account_sequence is ledger-assigned.
hold_id identifies one hold lifecycle.

Gateway cannot decide paid access from caller-supplied claims alone.

## Receipt display-only doctrine

A backend receipt may be display/validation context.

A gateway-relayed backend receipt is not gateway-issued truth.

Backend receipt is display/validation context.
Receipt display cache is display-only.
Receipt metadata may be upstream-derived or backend-derived.
Receipt metadata is not finality.
Receipt metadata is not balance truth.
Receipt metadata is not receipt authority when supplied by a client.

## Future QuickChain work parked outside gateway

The following remains parked until the proper QuickChain phase gates are green:

- canonical bytes and locked vectors
- state/account merkle roots
- receipt roots
- checkpoint signing
- validator-set logic
- external DA
- public anchors
- bridges
- staking or liquidity
- CrabLink chain authority
- gateway/omnigate ledger mutation

No roots.
No validators.
No bridges.
No external settlement.
No fake receipts.
No fake balances.

svc-wallet = economic mutation front-door.
ron-ledger = durable replayable truth.

## Focused preflight suites

The crate-local preflight gate discovers every `quickchain*.rs` test dynamically.

Known current focused suites include:

- quickchain_preflight_boundary
- quickchain_preflight_docs
- quickchain_preflight_no_fake_receipts
- quickchain_preflight_cache_boundary
- quickchain_preflight_paid_access
- quickchain_preflight_transport_authority
- quickchain_tooling_boundary

Related proxy regressions include:

- app_proxy
- paid_storage_estimate_proxy
- paid_storage_write_proxy
- product_routes_proxy
- site_visit_routes_proxy

Script contract:

- `scripts/dev-quickchain-preflight.sh`
- `scripts/dev-quickchain-park.sh`

## Parking condition

Park `svc-gateway` when:

- this document exists
- focused QuickChain tests exist
- the preflight script discovers `quickchain*.rs` tests dynamically
- the parking script delegates to the preflight script
- all-targets tests pass
- clippy `-D warnings` passes
- no Python quickchain helper drift exists
- no fake receipts
- no fake balances
- no roots
- no validators
- no bridges
- no external settlement

## Exact scanner phrases retained by tests

Plain scanner phrase: proxy to omnigate.
Plain scanner phrase: proxying to omnigate.
Plain scanner phrase: gateway is proxy-only for public product routes.
Plain scanner phrase: svc-gateway remains a public route surface.
Plain scanner phrase: svc-gateway is not a chain runtime.

---

## Pair-level QuickChain value-loop boundary - svc-gateway

This section locks the shared public/product value-loop boundary between `svc-gateway` and `omnigate` for QuickChain Phase 0.

Scanner phrase: svc-gateway public route boundary -> omnigate product hydration/access coordination -> svc-wallet mutation front-door -> ron-ledger durable economic truth.

Scanner phrase: client intent -> svc-gateway public boundary -> omnigate quote/access/hydration coordinator -> svc-wallet hold/transfer/capture/release/receipt path -> ron-ledger accepted receipt -> paid unlock/render using backend-derived truth.

Scanner phrase: gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority.

Scanner phrase: accepted backend receipt can unlock local paid content.

Scanner phrase: accepted is not finalized.

Scanner phrase: accepted is not epoch_included.

Scanner phrase: accepted is not anchored.

Scanner phrase: gateway is not receipt truth.

Scanner phrase: gateway is not balance truth.

Scanner phrase: gateway is not settlement finality.

Scanner phrase: gateway never converts accepted wallet/ledger receipt into QuickChain finality.

Scanner phrase: current paid unlock is backend-derived local access, not future QuickChain epoch inclusion.

Scanner phrase: future statuses remain parked: accepted, epoch_included, finalized, anchored.

Scanner phrase: no root-producing code, no checkpoint-producing code, no validator code, no bridge code, no external settlement code.

Phase-0 meaning:

- `svc-gateway` may expose public routes and proxy product/payment/access requests.
- `svc-gateway` may preserve safe request context and idempotency metadata.
- `svc-gateway` may return backend-derived responses.
- `svc-gateway` must not create or transform economic truth.
- `svc-gateway` must not treat a backend accepted receipt as QuickChain finality.
- Future QuickChain statuses such as `epoch_included`, `finalized`, and `anchored` remain parked until root, checkpoint, and proof phases are explicitly authorized.

## Phase 1 Round 2 downstream confirmation

quickchain_phase1_round2_confirmation confirms svc-gateway remains downstream-light for QuickChain Phase 1 Round 2.

Round 2 root/proof implementation belongs to the authorized QuickChain core path, primarily ron-proto and ron-ledger. The gateway role is public route/admission/proxy enforcement only.

Required gateway boundary markers:

- gateway paid enforcement remains backend-derived
- gateway is not wallet truth
- gateway is not ledger truth
- gateway is not QuickChain root authority
- gateway is not finality authority
- gateway cannot unlock paid content from cache alone
- svc-wallet remains the paid mutation path
- ron-ledger remains durable economic truth

Current paid access language:

- accepted backend receipt can unlock local paid content
- accepted is not finalized
- accepted is not epoch_included
- accepted is not anchored
- future statuses remain parked: accepted, epoch_included, finalized, anchored
- current paid unlock is backend-derived local access, not future QuickChain epoch inclusion

Gateway status display rule:

- gateway never converts accepted wallet/ledger receipt into QuickChain finality
- gateway must label status honestly and must not fabricate status
- cache is convenience, not entitlement
- headers are transport metadata, not payment proof
- b3 is byte identity, not payment proof
- index pointers are lookup metadata, not payment proof
- policy decisions are declarative gating, not settlement truth

Forbidden scope remains:

- no root-producing code, no checkpoint-producing code, no validator code, no bridge code, no external settlement code
- no ROX runtime, no Solana runtime, no staking, no liquidity, no exchange-facing logic
- no fake balances, no fake receipts, no fake finality, no silent spend

