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

---

## Phase 1 Round 2 — downstream confirmation

Plain scanner boundary phrases:

```text
phase 1 round 2 downstream confirmation
index can point to artifacts but not prove them
artifact pointer is not proof
manifest pointer is not proof
index pointer is not quickchain root authority
index pointer is not finality authority
policy metadata on an index entry is not wallet or ledger proof
svc-index cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
future statuses remain parked: accepted, epoch_included, finalized, anchored
quickchain_phase1_round2_confirmation
```

`svc-index` may store and return mutable lookup pointers to immutable b3 artifacts, including future vector/root/proof artifacts when other crates produce them. That pointer is navigation and discovery context only. It is not proof inclusion, not settlement finality, not spend authority, not a paid unlock, and not wallet or ledger truth.

---

## Phase 2 Round 1 verifier artifact / read-only replication boundary

This crate-pair status is Phase 2 Round 1 verifier artifact / read-only replication.

`quickchain_phase2_replay_boundary` is the focused regression target for this svc-index boundary.

For `svc-index`:

- svc-index may lookup artifact pointers only
- svc-index may point to read-only replay/proof artifacts
- artifact pointers are references only
- index artifact pointer is not proof authority
- index artifact pointer is not verifier truth
- index artifact pointer is not replay truth
- index artifact pointer is not quorum truth
- index artifact pointer is not committee truth
- index artifact pointer is not fork choice
- index artifact pointer is not finality
- index artifact pointer cannot unlock paid content
- b3 proves bytes, not verifier truth
- b3 proves bytes, not finality
- b3 proves bytes, not paid entitlement

Hard boundary:

- names are pointers, not proof
- crab navigation is navigation, not authority
- manifest lookup is not paid unlock
- provider lookup is not settlement finality
- owner wallet fields are references only
- owner passport fields are references only
- svc-wallet remains the paid mutation path
- ron-ledger remains durable economic truth

Still forbidden here:

- committee signing
- quorum/fork-choice
- validator signatures
- staking
- slashing
- public bridge
- external settlement
- ROX
- Solana
- direct wallet mutation
- direct ledger mutation
- fake receipts
- fake balances
- fake finality
- cache-only paid unlock
- index-only paid unlock
- artifact-pointer-only paid unlock

---

## Phase 2 Round 2 committee readiness boundary

Phase 2 Round 2 is the small committee agreement/readiness boundary.

svc-index may lookup artifact pointers only.

svc-index may point to backend-derived verifier readiness artifacts only as references.

svc-index may point to backend-derived committee readiness artifacts only as references.

Index artifact pointer is not verifier truth.

Index artifact pointer is not committee truth.

Index artifact pointer is not quorum truth.

Index artifact pointer is not fork choice.

Index artifact pointer is not finality.

Index artifact pointer is not settlement.

Index artifact pointer cannot unlock paid content.

b3 proves bytes, not verifier truth.

b3 proves bytes, not committee truth.

b3 proves bytes, not quorum truth.

b3 proves bytes, not finality.

Names are pointers, not proof.

crab navigation is navigation, not authority.

manifest lookup is not paid unlock.

provider lookup is not settlement finality.

owner wallet fields are references only.

owner passport fields are references only.

svc-index does not produce signed verification attestations.

svc-index does not decide committee membership.

svc-index does not decide quorum.

svc-index does not decide fork choice.

svc-index does not decide finality.

svc-index does not bridge externally.

svc-index does not implement staking.

svc-index does not implement liquidity.

svc-index does not mutate wallet.

svc-index does not mutate ledger.

svc-wallet remains the paid mutation path.

ron-ledger remains durable economic truth.

quickchain_phase2_committee_boundary

## Phase 3 Round 1 validator/passport boundary

svc-index may point to backend-derived validator set/readiness artifacts if future backend routes expose them.

Index validator status pointers are references only.

svc-index is lookup and pointer infrastructure only.

svc-index is not validator identity authority.
svc-index is not passport registry authority.
svc-index is not validator capability authority.
svc-index is not validator-set authority.

svc-index cannot admit validators.
svc-index cannot revoke validators.
svc-index cannot rotate validators.

svc-index cannot unlock paid content from validator/passport material.
svc-index cannot replace wallet/ledger truth.

Name pointers, b3 pointers, manifest pointers, provider pointers, and future validator/readiness artifact pointers are not proof, quorum, finality, settlement, paid entitlement, or spend authority.

quickchain_phase3_validator_boundary

---

## Phase 3 Round 2 validator lifecycle boundary

svc-index may store and return backend-derived lookup pointers and metadata only.

svc-index is not validator lifecycle authority.

svc-index is not validator rotation authority.

svc-index is not validator revocation authority.

svc-index is not validator downtime authority.

svc-index is not validator degraded-status authority.

svc-index is not validator equivocation authority.

svc-index is not replay challenge authority.

svc-index is not governance parameter-update authority.

validator rotation, revocation, downtime, degraded status, equivocation evidence, double-attestation evidence, split-brain evidence, replay challenge evidence, and governance-gated parameter updates are not index truth.

index entries, names, manifests, profile pointers, b3 hashes, provider records, cache hits, and route metadata cannot unlock paid content.

validator lifecycle metadata cannot mint, transfer, burn, hold, capture, release, issue receipts, mutate balances, prove finality, prove settlement, or replace wallet/ledger truth.

svc-index must reject validator lifecycle/evidence/governance authority smuggling through pointer DTOs, routes, and source boundaries.

quickchain_phase3_validator_lifecycle_boundary

## QuickChain Phase 4 Round 1 bond artifact pointer boundary

Phase 4 Round 1 bond artifact pointer boundary for svc-index.

svc-index may point to bond/evidence/report artifacts only as references.

Index records are lookup truth only.

Index pointer is not bond truth.

Index pointer is not slash truth.

Index pointer is not staking authority.

Index pointer is not liquidity authority.

Index pointer is not settlement finality.

Index pointer is not validator economy authority.

Bond artifact CID proves bytes only.

Slash evidence CID proves bytes only.

b3 proves bytes, not bond lifecycle truth.

b3 proves bytes, not slash truth.

Names are pointers, not bond authority.

crab navigation is navigation, not bond authority.

Manifest lookup is not slash authority.

Provider lookup is not staking authority.

Owner wallet fields are references only.

Owner passport fields are references only.

Index cache cannot unlock paid content or trigger validator consequences.

Index pointer cannot capture a bond.

Index pointer cannot release a bond.

Index pointer cannot slash a validator.

Index pointer cannot open a staking market.

Index pointer cannot create liquidity.

Index pointer cannot prove public validator economy status.

svc-wallet remains the paid mutation path.

ron-ledger remains durable economic truth.

No bond mutation route belongs in svc-index.

No slash route belongs in svc-index.

No staking route belongs in svc-index.

No liquidity route belongs in svc-index.

No bridge, ROX/Solana, or external settlement route belongs in svc-index.

quickchain_phase4_bond_boundary is the focused Phase 4 Round 1 boundary test for svc-index.

## Phase 4 Round 2 bond dispute/challenge pointer boundary

Phase 4 Round 2 is simulation only.

svc-index may point to dispute/challenge/appeal/freeze artifacts only as references.

svc-index may store or return reference metadata for:

    bond dispute artifact CIDs
    slash evidence artifact CIDs
    challenge window artifact CIDs
    appeal artifact CIDs
    freeze status display labels
    disputed bond status display labels
    slash simulation artifact CIDs

But:

    index pointer is not dispute truth
    index pointer is not challenge-window truth
    index pointer is not appeal authority
    index pointer is not freeze authority
    index pointer is not irreversible slash authority
    index pointer is not slash simulation authority
    dispute artifact cid proves bytes only
    challenge evidence cid proves bytes only
    appeal artifact cid proves bytes only
    freeze status label is display metadata only
    b3 proves bytes, not dispute resolution truth
    names are pointers, not dispute authority
    manifest lookup is not challenge evidence validation
    provider lookup is not appeal authority
    index cache cannot unlock paid content or trigger dispute consequences

Dispute/challenge/appeal/freeze metadata cannot unlock paid content.

Index lookup cannot validate slash evidence.

Index lookup cannot open a challenge window.

Index lookup cannot grant appeal authority.

Index lookup cannot freeze or release a bond.

Index lookup cannot execute or commit an irreversible slash.

svc-wallet remains the paid mutation path.

ron-ledger remains durable economic truth.

No live irreversible slash through svc-index.

No public staking market through svc-index.

No liquidity through svc-index.

No ROX, Solana, bridge, external settlement, exchange-facing logic, or public validator economy through svc-index.

Test marker:

    quickchain_phase4_bond_dispute_boundary

