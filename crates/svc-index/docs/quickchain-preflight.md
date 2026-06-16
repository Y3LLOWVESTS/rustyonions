# svc-index QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0/preflight boundary notes for `svc-index`.
RO:WHY — Prevent lookup, pointer, cache, name, manifest, or provider records from becoming accidental economic authority or QuickChain authority.
RO:INTERACTS — svc-index routes, store, cache, provider/resolve pipelines, svc-gateway, omnigate, svc-storage, svc-wallet, ron-ledger, ron-proto, ron-policy, CrabLink.
RO:INVARIANTS — lookup truth is not economic truth; pointer truth is not receipt truth; names are pointers; b3 identifies bytes/content; manifests do not unlock paid content alone; no roots/checkpoints/validators/settlement here.
RO:METRICS — existing service HTTP/cache/provider metrics only; no QuickChain checkpoint/root/finality metrics yet.
RO:CONFIG — no QuickChain runtime config; existing index config remains service-local.
RO:SECURITY — no fake receipts, fake balances, fake finality, fake unlocks, direct ledger mutation, wallet mutation, bridge, staking, liquidity, ROX, Solana, or external settlement.
RO:TEST — `quickchain_preflight_docs.rs`, `quickchain_preflight_boundary.rs`, `quickchain_preflight_pointer_authority.rs`, `quickchain_preflight_routes.rs`.

## Status

`svc-index` is a lookup and pointer service.

Plain scanner phrase: svc-index is a lookup and pointer service.

It is not:

- economic authority
- wallet authority
- ledger truth
- paid-access authority
- receipt authority
- balance authority
- finality authority
- a QuickChain runtime
- a root producer
- a checkpoint producer
- a validator
- a bridge
- staking infrastructure
- liquidity infrastructure
- external settlement infrastructure

QuickChain remains future settlement infrastructure. This document is Phase-0 hardening only.

## svc-index authority boundary

`svc-index` may be authoritative for pointer records it owns.

Examples of allowed pointer/index truth:

- this name points to this manifest
- this site name points to this site manifest CID
- this asset CID points to this asset manifest CID
- this provider list is a lookup result
- this cache entry is a lookup acceleration
- this record was stored in the index service

Examples of forbidden economic or settlement truth:

- this user has paid
- this wallet balance is correct
- this receipt is final
- this object is unlocked
- this entitlement is valid
- this checkpoint is canonical
- this state root is canonical
- this validator set is active
- this bridge settlement is complete
- this external anchor mutates ROC truth

## Core distinction

```text
Index truth is not economic truth.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Policy metadata is not wallet authority.
Provider lookup is not settlement finality.
```

## Names, b3 hashes, and manifests

Names are pointers.

b3 hashes identify bytes or content-addressed records.

Manifests describe content, metadata, policy, ownership references, payout references, provenance references, or navigation references.

None of these unlock paid content without backend wallet, ledger, and storage receipt truth.

A manifest pointer may include owner/passport/wallet reference metadata. Those fields are references only. They are not spend authority, capture authority, balance authority, receipt authority, or entitlement authority.

## Paid access rule

Paid access must be proven through backend service paths that include wallet/ledger/storage truth.

Allowed paid-access proof path:

```text
client intent
→ svc-gateway / omnigate boundary
→ svc-wallet mutation or lookup path when payment is required
→ ron-ledger durable receipt truth
→ svc-storage / gateway paid enforcement
→ backend-derived receipt/unlock response
```

Forbidden shortcut paths:

```text
index entry → unlock
manifest lookup → unlock
name exists → unlock
b3 exists → unlock
provider exists → unlock
cache hit → unlock
owner metadata exists → unlock
policy metadata exists → unlock
header says paid → unlock
client says paid → unlock
```

## QuickChain Phase-0 forbidden scope in svc-index

`svc-index` must not implement:

- roots
- state roots
- receipt roots
- accounting roots
- reward roots
- checkpoint roots
- checkpoints
- validators
- validator sets
- committee logic
- consensus
- fork choice
- settlement
- external anchors
- bridges
- ROX
- Solana
- staking
- liquidity
- exchange-facing logic
- DA/archive/challenge/pruning logic
- root-producing index snapshots
- root-producing pointer maps
- fake hashes
- placeholder hashes
- fake receipts
- fake balances
- fake finality
- fake unlocks

Future QuickChain phases may need deterministic pointer snapshots or proof-eligible lookup records. Phase 0 may discuss ordering hygiene, but it must not produce canonical roots or checkpoint artifacts.

## Scanner posture

Do not ban legitimate metadata words blindly.

Allowed as metadata when they stay non-authoritative:

- `owner`
- `creator`
- `recipient`
- `policy`
- `manifest`
- wallet as a reference string
- `paid` in documentation about forbidden shortcuts

Ban or constrain authority-shaped usage:

- `paid_from_index`
- `unlock_from_index`
- `receipt_from_index`
- `balance_from_index`
- `finality_from_index`
- `checkpoint_from_index`
- `validator_from_index`
- `root_from_index`
- `paid_from_manifest`
- `unlock_from_manifest`
- `receipt_from_pointer`
- `balance_from_pointer`
- `unlock_from_cache`
- `entitlement_from_cache`

## Acceptance gate

Before parking `svc-index` for QuickChain Phase 0:

- docs must preserve the pointer-only boundary
- Cargo dependencies must not include wallet or ledger mutation crates
- production source must not import `ron_ledger` or `svc_wallet`
- production source must not call wallet mutation verbs
- public routes must not expose QuickChain roots, checkpoints, validators, settlement, bridges, staking, liquidity, ROX, or Solana
- DTOs must reject unknown authority-shaped fields
- owner/passport/wallet metadata must remain references only
- tests must pass under the crate-local preflight script
