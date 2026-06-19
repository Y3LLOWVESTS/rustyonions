# omnigate QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary doctrine for `omnigate`.
RO:WHY — Keeps omnigate as hydration boundary and product coordination boundary without becoming ROC economic truth.
RO:INTERACTS — omnigate routes, svc-gateway, svc-storage, svc-index, ron-policy, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, CrabLink/Tauri.
RO:INVARIANTS — omnigate hydrates content/views and coordinates product/access views; it does not mutate ledger, mint ROC, issue wallet receipts, invent balances, or unlock paid content from cache alone.
RO:METRICS — Existing route/admission/policy metrics only; metrics are not payment proof, balance truth, or finality.
RO:CONFIG — Uses configured downstream services; config must not enable QuickChain runtime, validator, bridge, staking, liquidity, or external settlement behavior.
RO:SECURITY — Client/cache/header/local claims cannot prove receipt, paid access, balance, root, checkpoint, validator approval, bridge settlement, or finality.
RO:TEST — `quickchain_preflight_*` tests plus `scripts/dev-quickchain-preflight.sh` and `scripts/dev-quickchain-park.sh`.

## Status

`omnigate` is the product hydration and coordination boundary.

Required scanner phrases:

- hydration boundary
- product coordination boundary
- product hydration and coordination boundary
- not become ROC economic truth
- no direct ledger mutation

Plain scanner phrase: omnigate hydrates content/views.
Plain scanner phrase: omnigate does not mutate ledger.
Plain scanner phrase: omnigate does not mint ROC.
Plain scanner phrase: omnigate does not issue wallet receipts.
Plain scanner phrase: omnigate does not invent balances.
Plain scanner phrase: omnigate does not unlock paid content from cache alone.

## Boundary doctrine

`omnigate` may:

- hydrate content/views
- compose product/access views
- coordinate explicit backend service calls
- proxy paid storage, content view, site visit, stream, chat, and wallet front-door flows
- display backend-derived receipt/access state
- fail closed when backend paid truth is unavailable

`omnigate` must not be or become:

- not ledger truth
- not wallet truth
- not receipt truth
- not balance truth
- not QuickChain runtime
- not become a public Layer 1
- no roots/checkpoints/validators/bridges
- root authority
- checkpoint authority
- finality authority
- validator authority
- bridge authority
- staking authority
- liquidity authority
- external settlement authority

## Hydration doctrine

Hydration composes views. Hydration is not settlement.

Manifest hydration does not prove payment.
Profile hydration does not prove payment.
Index/provider metadata does not prove payment.
b3 hashes are content truth, not economic truth.

## Paid access doctrine

Paid render/access must be based on backend wallet/ledger truth.

Content-view, site-visit, stream, chat, and paid-storage paths must fail closed when backend receipt/access truth is missing.

Safe behavior:

- quote is read-only
- pay uses svc-wallet only
- wallet receipt lookup
- backend wallet receipt truth
- backend receipt is display/validation context
- wallet receipt metadata can be displayed as backend-derived context

Unsafe behavior:

- fake receipts
- fake balances
- fake finality
- fake proofs
- cache-only paid unlock
- client-supplied paid unlock
- local receipt cache as payment proof

## Cache doctrine

Cache cannot unlock paid content.
Cache is convenience only.
Content-addressing is integrity, not payment proof.
b3 proves bytes.
b3 does not prove payment.
manifest hydration does not prove payment.
local receipt cache does not prove payment without backend validation.
receipt display cache is display-only.

Cache-Control, ETag, If-None-Match, and related transport metadata are not entitlement.

## Transport/header doctrine

Transport headers are not economic authority.

Client-supplied receipt, paid, unlocked, entitlement, finality, balance, or cache headers must never prove paid access inside omnigate.

`x-ron-wallet-account` and `x-ron-passport` may identify payer intent or passport context.

They:

- cannot prove payment
- cannot unlock paid content
- cannot fabricate a receipt
- cannot replace backend wallet/ledger truth

Forbidden authority header families include:

- `x-ron-receipt`
- `x-ron-paid`
- `x-ron-unlocked`
- `x-ron-finalized`
- `x-ron-finality`
- `x-ron-balance`
- `x-ron-state-root`
- `x-ron-receipt-root`
- `x-ron-checkpoint`
- `x-ron-validator`
- `x-ron-bridge`
- `x-ron-anchor`
- `x-ron-quickchain`

## Operation identity rules

operation_id is backend-assigned durable ledger operation identity.
idempotency_key is retry identity only.
Idempotency-Key is retry identity only.
account_sequence is ledger-assigned.
hold_id identifies one hold lifecycle.
backend receipt is display/validation context.
receipt display cache is display-only.

## Forbidden authority fields

Omnigate request/response surfaces should reject or avoid authority fields such as:

- wallet_balance
- balance
- balance_minor
- available_minor
- held_minor
- receipt
- wallet_receipt
- payment_receipt
- ledger_receipt
- receipt_hash
- quickchain_root
- state_root
- receipt_root
- accounting_root
- reward_root
- checkpoint
- checkpoint_id
- checkpoint_hash
- validator_signature
- validator_set
- settlement_status
- finality
- finalized
- anchor
- anchored
- external_anchor
- bridge_txid
- bridge_settled
- staking
- liquidity
- ledger_commit
- mint
- transfer
- burn
- hold
- capture
- release
- unlock_from_cache

Context caveat: omnigate may legitimately display backend-derived receipt/access data when it is display-only, not balance truth, not receipt authority, not finality, not cache unlock authority, and not root/checkpoint/validator/bridge proof.

## Forbidden QuickChain scope

Do not add:

- roots
- state roots
- receipt roots
- accounting roots
- reward roots
- checkpoints
- validators
- validator signatures
- validator sets
- committee consensus
- settlement anchors
- external anchors
- bridge code
- ROX
- Solana
- external L2
- external DA
- staking
- liquidity
- public chain state
- exchange-facing logic
- omnigate ledger mutation
- omnigate wallet mutation
- fake balances
- fake receipts
- fake finality
- fake proofs
- cache-only paid unlock
- raw engagement protocol payouts
- DB-order roots
- wall-clock roots
- placeholder hashes
- fake golden vectors
- root-producing code without canonical bytes and locked vectors

No roots.
No validators.
No bridges.
No external settlement.

svc-wallet = economic mutation front-door.
ron-ledger = durable replayable truth.

## Focused preflight suites

Known current focused suites include:

- quickchain_preflight_boundary
- quickchain_preflight_docs
- quickchain_preflight_no_fake_receipts
- quickchain_preflight_paid_access
- quickchain_preflight_cache_boundary
- quickchain_preflight_transport_authority
- quickchain_tooling_boundary

Related route regressions include:

- content_view
- site_visit
- streams
- chat_routes
- paid_storage_estimate_proxy
- paid_storage_prepare
- paid_storage_write_proxy

## Script contract

- `scripts/dev-quickchain-preflight.sh`
- `scripts/dev-quickchain-park.sh`

The preflight script discovers every `quickchain*.rs` test dynamically, runs all-targets, and runs clippy with `-D warnings`.

## Parking condition

Park `omnigate` when:

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

cache cannot unlock paid content
Cache cannot unlock paid content.
Content-addressing is integrity, not payment proof.
b3 proves bytes
b3 does not prove payment
manifest hydration does not prove payment
local receipt cache does not prove payment without backend validation
Paid render/access must be based on backend wallet/ledger truth.

## Cache/content exact scanner phrases

cache is convenience only
cache cannot unlock paid content
b3 hashes are content truth, not economic truth
b3 proves bytes
b3 does not prove payment
manifest hydration does not prove payment
local receipt cache does not prove payment without backend validation
Cache is convenience only; backend wallet/ledger receipt validation remains required for paid unlock.

---

## Pair-level QuickChain value-loop boundary - omnigate

This section locks the shared public/product value-loop boundary between `svc-gateway` and `omnigate` for QuickChain Phase 0.

Scanner phrase: svc-gateway public route boundary -> omnigate product hydration/access coordination -> svc-wallet mutation front-door -> ron-ledger durable economic truth.

Scanner phrase: omnigate product hydration/access coordination -> svc-wallet mutation front-door -> ron-ledger durable economic truth.

Scanner phrase: client intent -> svc-gateway public boundary -> omnigate quote/access/hydration coordinator -> svc-wallet hold/transfer/capture/release/receipt path -> ron-ledger accepted receipt -> paid unlock/render using backend-derived truth.

Scanner phrase: gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority.

Scanner phrase: accepted backend receipt can unlock local paid content.

Scanner phrase: accepted is not finalized.

Scanner phrase: accepted is not epoch_included.

Scanner phrase: accepted is not anchored.

Scanner phrase: omnigate is not receipt truth.

Scanner phrase: omnigate is not balance truth.

Scanner phrase: omnigate is not settlement finality.

Scanner phrase: omnigate cannot promote accepted receipt to epoch_included, finalized, or anchored.

Scanner phrase: current paid unlock is backend-derived local access, not future QuickChain epoch inclusion.

Scanner phrase: future statuses remain parked: accepted, epoch_included, finalized, anchored.

Scanner phrase: no root-producing code, no checkpoint-producing code, no validator code, no bridge code, no external settlement code.

Phase-0 meaning:

- `omnigate` may coordinate product hydration, quote/access preparation, and backend-derived paid-access responses.
- `omnigate` may compose display DTOs from backend truth.
- `omnigate` may pass through accepted backend wallet/ledger receipt context for local paid unlock.
- `omnigate` must not create receipt truth, balance truth, settlement finality, roots, checkpoints, validator approval, external anchors, or bridge state.
- Future QuickChain statuses such as `epoch_included`, `finalized`, and `anchored` remain parked until root, checkpoint, and proof phases are explicitly authorized.
