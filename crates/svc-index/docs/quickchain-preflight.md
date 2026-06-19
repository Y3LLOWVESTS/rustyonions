# svc-index QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary note for `svc-index`.
RO:WHY — Proves index/pointer/name/manifest lookup does not become economic authority.
RO:INTERACTS — svc-index, svc-storage, svc-gateway, omnigate, ron-policy, svc-wallet, ron-ledger, ron-proto.
RO:INVARIANTS — svc-index stores and resolves references only; it must not mint, transfer, unlock, finalize, settle, checkpoint, validate, bridge, or manufacture receipts/balances/finality.
RO:TEST — `cargo test -p svc-index --test quickchain_preflight_docs`; crate park script.

## Status

svc-index is a lookup and pointer service.

It may resolve:

- names
- b3 content identifiers
- manifest pointers
- provider metadata
- site manifest pointers
- asset manifest pointers
- reference metadata

It must not become a value-plane shortcut.

## Boundary doctrine

index truth is not economic truth.

pointer truth is not receipt truth.

name resolution is not ownership proof.

b3 byte identity is not payment proof.

manifest lookup is not paid unlock.

policy metadata is not wallet authority.

provider lookup is not settlement finality.

A `b3:<64 lowercase hex>` value identifies bytes or a manifest root. It is not a payment proof.

A `crab://` URL is navigation. It is not authority.

A name pointer is a pointer. It is not wallet ownership, paid entitlement, ledger truth, or finality.

## Paid access truth path

paid access must be proven through backend service paths.

The paid access path remains:

- svc-wallet mutation or lookup path
- ron-ledger durable receipt truth
- svc-storage / gateway paid enforcement
- backend-derived receipt or authorization result
- render/display after backend acceptance

The following are forbidden as unlock authority:

- index entry → unlock
- manifest lookup → unlock
- cache hit → unlock
- client says paid → unlock

svc-index can return metadata that helps another backend service find a manifest or provider. It cannot decide that paid content is unlocked by itself.

## QuickChain parked scope

The following remain parked and forbidden in svc-index:

- roots
- state roots
- receipt roots
- root-producing index snapshots
- checkpoints
- validators
- consensus
- settlement
- external anchors
- bridges
- ROX
- Solana
- staking
- liquidity
- fake receipts
- fake balances
- fake finality
- fake unlocks
- wallet mutation
- ledger mutation
- receipt creation
- balance creation
- finality creation
- checkpoint creation
- validator behavior
- bridge behavior
- external settlement behavior

svc-index must not expose QuickChain runtime routes, settlement routes, validator routes, bridge routes, wallet mutation routes, ledger mutation routes, receipt authority routes, balance authority routes, or paid-unlock-from-index routes.

## Metadata warning

Do not ban legitimate metadata words blindly.

The following words may appear as reference metadata:

- owner
- creator
- recipient
- policy
- manifest
- wallet as a reference string

Allowed metadata must stay descriptive.

Ban or constrain authority-shaped usage.

Forbidden authority-shaped examples include:

- unlock_from_index
- receipt_from_index
- balance_from_index
- finality_from_index
- checkpoint_from_index
- validator_from_index
- root_from_index
- settle_from_index
- mint_from_index
- transfer_from_index
- paid_from_index
- entitlement_from_index

The rule is simple:

metadata is allowed; authority is not.
