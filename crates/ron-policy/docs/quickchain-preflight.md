# ron-policy QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary note for `ron-policy`.
RO:WHY — Proves declarative policy/evaluation does not become economic authority.
RO:INTERACTS — ron-policy, svc-gateway, omnigate, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, svc-storage, svc-index.
RO:INVARIANTS — policy may classify, allow, deny, explain, and declare obligations; it must not mutate, mint, settle, finalize, checkpoint, validate, bridge, or manufacture receipts/balances/finality.
RO:TEST — `cargo test -p ron-policy --test quickchain_preflight_docs`; crate park script.

## Status

ron-policy is declarative policy infrastructure.

It may provide:

- allow decisions
- deny decisions
- reason codes
- obligations
- classification
- request constraints
- quota/config interpretation
- economics policy configuration
- explanations/traces for policy decisions

It must not become wallet truth, ledger truth, receipt truth, balance truth, paid-access authority, finality authority, payout authority, root authority, checkpoint authority, validator authority, bridge authority, or external settlement authority.

## Required scanner phrases

policy decision is not economic truth.

policy allow is not paid proof.

policy obligation is not receipt proof.

policy explanation is not finality proof.

economics policy config is not ledger mutation.

feature flag is not settlement authority.

policy config is not wallet authority.

policy classification is not payout allocation.

reason code is not finality.

policy metadata is not paid entitlement.

Policy must not manufacture paid proof.

Policy must not manufacture receipt proof.

Policy must not manufacture finality proof.

Policy must not manufacture balance proof.

## Value-loop boundary

The value loop remains backend-owned:

- svc-wallet prepares, issues, transfers, burns, holds, captures, releases, and returns backend receipts.
- ron-ledger remains durable economic truth.
- ron-accounting records snapshots and usage views, not balance truth.
- svc-rewarder plans payouts, not direct ledger mutation.
- svc-storage, svc-gateway, and omnigate enforce paid access using backend-derived truth.
- ron-policy only declares policy and evaluates policy.

ron-policy must not:

- mutate wallet state
- mutate ledger state
- create wallet receipts
- create ledger receipts
- fabricate receipt IDs
- fabricate balances
- allocate protocol ROC
- allocate rewards
- execute payout
- unlock paid content by itself
- capture holds
- release holds
- settle operations
- claim accepted receipt status
- claim epoch_included status
- claim finalized status
- claim anchored status

## QuickChain parked scope

The following remain parked and forbidden in ron-policy:

- root-producing code
- checkpoint-producing code
- validator code
- settlement code
- wallet mutation
- ledger mutation
- paid unlock finality
- external anchors
- bridge logic
- staking
- liquidity
- ROX
- Solana
- external DA
- external L2
- public bridge
- fake receipts
- fake balances
- fake finality
- fake unlocks

Policy can say a request is allowed or denied. It cannot prove payment, balance, receipt existence, settlement, epoch inclusion, anchoring, or finality.

## Transport/header boundary

If a caller supplies transport/header context, ron-policy must treat it as context only.

These must never become authority inside policy:

- x-ron-paid: true as authority
- x-ron-receipt: fake as authority
- x-ron-balance: fake as authority
- x-ron-finalized: true as authority
- x-quickchain-root as authority
- x-quickchain-checkpoint as authority
- x-quickchain-validator as authority

Policy is transport-agnostic. Transport authority must be validated by callers through backend wallet/ledger/storage truth paths.

---

## Phase 1 Round 2 — downstream confirmation

Plain scanner boundary phrases:

```text
phase 1 round 2 downstream confirmation
policy can gate access but not create proof/finality
policy cannot turn a proof into spend authority
policy decision is not quickchain proof
policy allow is not epoch_included
policy allow is not finalized
policy allow is not anchored
policy obligation is not receipt inclusion proof
policy obligation is not account proof
policy obligation is not spend authority
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
quickchain_phase1_round2_confirmation
```

`ron-policy` may classify requests, require backend wallet/ledger proof, and declare obligations for gateway or omnigate enforcement. It must not manufacture receipt inclusion proof, account proof, root proof, finality, settlement status, spend authority, wallet truth, or ledger truth.
