# svc-rewarder QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary notes for `svc-rewarder`.
RO:WHY — Locks `svc-rewarder` as deterministic ROC payout planning infrastructure, not wallet, ledger, chain, bridge, validator, root, checkpoint, settlement, staking, or liquidity authority.
RO:INTERACTS — ron-accounting reward snapshots, svc-rewarder planning, svc-wallet issue request planning, ron-ledger truth, ron-policy funding policy, future QuickChain DTOs.
RO:INVARIANTS — deterministic ROC payout planner only; svc-wallet is the mutation front-door; ron-ledger is durable economic truth; no root/checkpoint/validator/finality/bridge/external settlement authority.
RO:METRICS — planning and egress-result metrics only; metrics are not balance truth or receipt truth.
RO:CONFIG — reward policy, funding provenance, wallet handoff path, idempotency salt, concurrency limits, amnesia posture.
RO:SECURITY — no fake balances, no fake receipts, no fake finality, no raw engagement direct protocol payouts, no silent spend, no direct ledger mutation.
RO:TEST — quickchain_preflight_boundary, quickchain_preflight_raw_engagement, quickchain_preflight_replay_no_double_issue, quickchain_preflight_funding_source, quickchain_preflight_no_direct_mutation, quickchain_preflight_docs, quickchain_preflight_value_loop_boundary, quickchain_tooling_boundary.

## 0. Status

`svc-rewarder` is a deterministic ROC payout planner.

It is not a chain runtime.

It is not a validator.

It is not a bridge.

It is not a checkpoint writer.

It is not a root producer.

It is not a ledger mutation authority.

It is not wallet authority.

It does not create balance truth.

It does not create receipt truth.

It does not create settlement finality.

`svc-wallet is the mutation front-door`.

`ron-ledger is durable economic truth`.

`ron-accounting` snapshots are derivative planning input only.

## 1. Correct value-loop position

The pair-level value loop is:

```text
svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

`svc-rewarder` may produce deterministic planning artifacts and wallet issue request planning payloads.

`svc-rewarder` must not bypass the wallet path.

`svc-rewarder` must not treat any planning artifact as finality.

`svc-rewarder` must not treat funding provenance as settlement finality.

Plain scanner phrase: funding provenance is not settlement finality.

`svc-rewarder` must not treat an idempotency key as ledger operation identity.

## 2. Allowed Phase-0 scope

Allowed now:

```text
strict serde DTOs
integer minor-unit money strings only
canonical lowercase b3 identifiers
explicit funding provenance
wallet issue request planning
deterministic plan ordering
idempotency/replay boundary checks
raw engagement rejection checks
docs and preflight tooling
```

Allowed funding provenance examples:

```text
protocol_pool
governance_budget
creator_budget
operator_budget
```

Protocol/governance-style funding must require a signed policy before any real handoff is considered.

## 3. Forbidden Phase-0 scope

Forbidden now:

```text
no root-producing code
no checkpoint-producing code
no validator code
no bridge or external settlement code
no direct ledger mutation
no direct wallet mutation outside explicit svc-wallet handoff
no fake balances
no fake receipts
no fake finality
no Solana
no ROX
no public bridge
no external anchors
no staking or liquidity
no exchange-facing logic
```

Future QuickChain work remains parked outside `svc-rewarder` until canonical bytes and locked vectors exist:

```text
canonical bytes and locked vectors
state/account merkle roots
receipt roots
validator-set logic
checkpoint signing
external da
public anchors
bridges
staking or liquidity
crablink chain authority
gateway/omnigate/rewarder ledger mutation
```

## 4. Raw engagement boundary

Raw engagement fields must stay rejected as direct protocol ROC authority.

Examples that must not become payout authority by themselves:

```text
raw views
raw watch seconds
raw clicks
raw impressions
likes
shares
follows
views-to-roc formulas
watch-seconds-to-roc formulas
```

Raw usage may feed accounting, fraud analysis, or policy inputs only after classification and validation.

## 5. Replay and identity boundary

Idempotency keys are replay/dedupe tools.

They are not ledger operation identity.

They are not validator consensus.

They are not settlement authority.

`operation_id` remains backend-assigned durable ledger operation identity in the wallet/ledger path, not rewarder authority.

## 6. Focused suites

This crate’s QuickChain preflight sweep is protected by:

```text
quickchain_preflight_boundary
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_docs
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The dynamic preflight script must discover `quickchain*.rs` tests and run them through Cargo.
