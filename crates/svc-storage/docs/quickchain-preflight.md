# svc-storage QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary notes for `svc-storage`.
RO:WHY — Locks storage as bytes/b3/content infrastructure and prevents paid storage from becoming wallet, ledger, receipt, root, finality, bridge, validator, staking, liquidity, or settlement authority.
RO:INTERACTS — svc-storage CAS, b3 verification, paid storage admission, svc-wallet hold/capture/release path, ron-accounting usage snapshots, svc-rewarder payout planning, svc-gateway, omnigate.
RO:INVARIANTS — b3 hashes identify bytes only; cache is convenience only; paid access requires backend-derived authorization; storage metering is derivative accounting input only.
RO:METRICS — storage metrics and accounting export status only; metrics are not balance truth, receipt truth, payment truth, root truth, or finality.
RO:CONFIG — paid write verifier mode, wallet receipt lookup, wallet settlement mode, accounting export mode, economics policy path, bounded body/range settings.
RO:SECURITY — no fake balances, no fake receipts, no silent spend, no cache-only unlock, no roots, no validators, no bridges, no external settlement, no Solana/ROX path.
RO:TEST — quickchain_preflight_b3_integrity, quickchain_preflight_boundary, quickchain_preflight_docs, quickchain_preflight_economics_quote, quickchain_preflight_no_direct_mutation, quickchain_preflight_observability, quickchain_preflight_paid_cache, quickchain_preflight_range_media, quickchain_preflight_settlement_boundary, quickchain_preflight_value_loop_boundary, quickchain_tooling_boundary.

## 0. Status

`svc-storage remains content-addressed byte/object infrastructure`.

`b3 hashes identify bytes only`.

b3 hashes are content truth only.

b3 hashes are not payment proof.

b3 hashes are not receipt roots.

b3 hashes are not account state roots.

b3 hashes are not checkpoint roots.

b3 hashes are not settlement finality.

`svc-wallet = economic mutation front-door`.

`ron-ledger = durable replayable truth`.

`ron-accounting` measures storage usage.

`svc-rewarder` plans payouts.

## 1. Correct value-loop position

The pair-level value loop is:

```text
svc-storage paid admission and b3 byte integrity
-> storage/access metering
-> ron-accounting derivative snapshots
-> svc-rewarder deterministic payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

Storage may:

```text
store bytes
serve bytes
serve bounded ranges
verify b3
price/quote paid writes
verify backend wallet hold evidence
capture/release through configured wallet settlement path
emit usage events
export usage events to accounting
```

Storage must not:

```text
mutate ledger directly
invent balances
invent receipts
claim finality
produce QuickChain roots
act as a validator
act as a bridge
act as settlement truth
turn raw metering into protocol payout authority
```

## 2. Paid access and cache boundary

`cache must not decide paid access by itself`.

Cache can verify b3 before trusted render.

Cache cannot unlock paid content alone.

Offline cache verifies b3 before trusted render.

Paid content requires backend-derived authorization.

Paid content requires backend-derived receipt/authorization.

Receipt cache is display-only.

A storage CID, manifest CID, or b3 hash is not payment proof.

## 3. Metering boundary

Storage metering is derivative accounting input only.

Usage events are not balance updates.

Usage events are not wallet receipts.

Usage events are not payout authority.

Storage/access metering does not mutate wallet or ledger.

Accounting export failure does not turn storage into balance truth.

## 4. Bounded media boundary

Large media must stay bounded and honest.

Range/segment serving is preferred for large media.

Full-file unbounded command/result paths are not allowed.

Each rendition owns its own b3.

No DRM or anti-rip guarantee is made.

## 5. Forbidden Phase-0 scope

Forbidden now:

```text
no fake balances
no fake receipts
no silent spend
no roots
no validators
no bridges
no external settlement
no checkpoints
no anchors
no external anchors
no bridge or external settlement authority
no staking
no liquidity
no Solana
no ROX
no exchange-facing logic
no root-producing code
no checkpoint-producing code
no validator code
```

## 6. Focused suites

This crate’s QuickChain preflight sweep is protected by:

```text
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The dynamic preflight script must discover `quickchain*.rs` tests and run them through Cargo.

---

## Phase 1 Round 2 downstream confirmation

`svc-storage` remains downstream-light in Phase 1 Round 2.

Required confirmation:

```text
storage can store and retrieve vector/root/proof artifacts by b3
artifact CIDs are byte references, not QuickChain roots
storage cannot mutate balances
storage cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

The focused confirmation suite is:

```text
quickchain_phase1_round2_confirmation
```

Storage may retain bytes for future vector/root/proof artifacts, but this does not make storage a verifier, validator, root producer, finality oracle, wallet, ledger, or paid-unlock authority.

Still forbidden:

```text
no cache-only paid unlock
no balance mutation
no direct ledger mutation
no root-producing storage runtime
no checkpoint production
no validator behavior
no bridge or external settlement
no staking or liquidity
no fake balances
no fake receipts
no fake finality
```

---

## Phase 2 Round 1 verifier artifact / read-only replication boundary

This crate-pair status is Phase 2 Round 1 verifier artifact / read-only replication.

`quickchain_phase2_replay_boundary` is the focused regression target for this boundary.

For `svc-storage`:

- storage may retain read-only verifier artifact bytes by canonical b3
- storage may retrieve read-only verifier artifact bytes by canonical b3
- artifact cids are byte references, not verifier authority
- b3 proves bytes, not balance truth
- b3 proves bytes, not paid access
- b3 proves bytes, not finality

Hard boundary:

- storage cannot decide quorum
- storage cannot sign committee votes
- storage cannot claim fork choice
- storage cannot claim finality
- storage cannot mutate replay outcomes
- storage cannot unlock paid content from cache alone
- wallet/ledger receipts remain backend truth

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

<!-- BEGIN QUICKCHAIN PHASE 2 ROUND 2 COMMITTEE BOUNDARY -->

## QuickChain Phase 2 Round 2 committee readiness boundary

This section locks the Phase 2 Round 2 committee readiness boundary for `svc-storage`.

Required markers for `quickchain_phase2_committee_boundary`:

```text
phase 2 round 2 committee readiness boundary
svc-storage stores committee/replay artifacts as bytes only
storage is not a committee member
storage does not produce signed verification attestations
storage does not decide quorum
storage does not claim fork choice
storage does not claim finality
b3 proves byte integrity, not committee agreement
artifact cids are byte references, not verifier authority
cache cannot unlock paid content alone
wallet/ledger receipts remain backend truth
quickchain_phase2_committee_boundary
```

Allowed in this crate:

```text
- storing artifact bytes by canonical b3
- retrieving artifact bytes by canonical b3
- range reads over artifact bytes
- verifying byte integrity before trusted serve
- paid storage admission and metering support
```

Forbidden in this crate:

```text
- committee membership authority
- signed verification attestation production
- quorum certificate production
- fork-choice authority
- finality authority
- validator runtime
- bonded stake
- slashing
- bridge finality
- external settlement
- direct wallet mutation
- direct ledger mutation
- cache-only paid unlock
```

<!-- END QUICKCHAIN PHASE 2 ROUND 2 COMMITTEE BOUNDARY -->
