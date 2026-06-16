# svc-rewarder QuickChain Phase-0 Preflight

## Purpose

`svc-rewarder` is a deterministic ROC payout planner.

For QuickChain Phase 0, this crate is not a chain runtime, not a validator, not a bridge,
not a checkpoint writer, not a root producer, and not a ledger mutation authority.

Its job is intentionally narrow:

1. consume sealed `ron-accounting`-style aggregate reward snapshots;
2. apply an explicit signed reward policy;
3. compute deterministic integer-only ROC payout plans;
4. emit wallet-shaped issue-request previews or explicit wallet egress batches;
5. preserve the boundary that svc-wallet is the mutation front-door and ron-ledger is durable economic truth.

This document exists so future patches do not accidentally turn reward planning into settlement
authority before QuickChain gates are ready.

## QuickChain doctrine for this crate

The crate must preserve the current doctrine:

- determinism before distribution;
- DTOs before roots;
- roots before validators;
- proofs before pruning;
- internal ROC before external anchors.

For `svc-rewarder`, the practical meaning is:

- no root-producing code;
- no checkpoint-producing code;
- no validator code;
- no bridge or external settlement code;
- no staking, liquidity, ROX, Solana, or exchange-facing logic;
- no direct ledger mutation;
- no fake balances;
- no fake receipts;
- no fake finality;
- no fake proof fields.

## Current allowed Phase-0 scope

Allowed in this crate now:

- strict serde DTOs with `deny_unknown_fields`;
- integer minor-unit money strings only;
- canonical lowercase `b3:<64 hex>` content identifiers and commitments;
- deterministic payout computation from bounded aggregate counters;
- explicit reward policy validation;
- explicit funding provenance on reward plans;
- replay/dedupe/idempotency tests;
- wallet issue request planning;
- explicit wallet egress through `svc-wallet`;
- documentation and preflight tests that reject authority creep.

## Current forbidden Phase-0 scope

Forbidden in this crate now:

- producing QuickChain state roots;
- producing receipt roots;
- producing checkpoints;
- validator selection, validator signatures, validator voting, or finality;
- bridge, anchor, DA, L2, Solana, ROX, staking, liquidity, or external settlement logic;
- direct ledger issue, transfer, burn, hold, capture, release, or receipt creation;
- using raw engagement counters as protocol ROC authority;
- claiming wallet balances or ledger truth;
- treating idempotency keys as authority;
- treating funding provenance as mutation permission.

## Value-plane boundary

The intended internal value loop remains:

    ron-accounting snapshot
      -> svc-rewarder deterministic payout plan
      -> svc-wallet issue/transfer/burn/hold/capture/release front-door
      -> ron-ledger durable truth

`svc-rewarder` may plan and preview wallet-shaped issue requests.

`svc-rewarder` must not directly mutate balances or claim that a plan is final ledger truth.

## Funding-source boundary

Reward policies must carry explicit funding provenance.

Current funding sources are represented by `RewardFundingSource` and serialize as snake-case labels:

- `protocol_pool`
- `advertiser_budget`
- `creator_pool`
- `sponsor_budget`
- `governance_budget`

`protocol_pool` and `governance_budget` require a signed policy.

Funding provenance is not settlement finality. It does not authorize direct mutation. It is metadata
that travels on rewarder planning objects while wallet issue requests keep the narrow `svc-wallet`
issue shape.

## Raw-engagement boundary

Current reward contribution counters are storage/network/provider style counters only:

- `bytes_stored`
- `bytes_served`
- `uptime_seconds`

Raw engagement fields must stay rejected at the DTO boundary:

- raw views;
- raw likes;
- raw comments;
- raw watch seconds;
- raw clicks;
- raw impressions;
- raw active users;
- views-to-ROC formulas;
- engagement-mint authority fields.

Raw engagement may later feed analytics or ad-budgeted systems, but it must not directly mint or
allocate protocol ROC.

## Replay and idempotency boundary

`run_key` and wallet idempotency keys are replay/dedupe tools.

They are not ledger operation identity, not account sequence authority, not finality,
not validator consensus, not receipt truth, and not bridge authority.

Future durable operation identity belongs below this crate in the wallet/ledger path.

## HTTP boundary

Current service routes intentionally expose only service health, readiness, metrics, version,
epoch compute, epoch inspection, settlement preview, and explicit wallet egress.

The service must not expose direct mutation or chain-authority routes such as:

- `/v1/issue`
- `/ledger/issue`
- `/ledger/transfer`
- `/ledger/burn`
- `/quickchain/root`
- `/quickchain/checkpoint`
- `/quickchain/validator`
- `/bridge/anchor`

Wallet-shaped issue requests are emitted to `svc-wallet`, not handled as local rewarder mutation
authority.

## Focused preflight suites

The focused QuickChain preflight gate currently includes:

    cargo test -p svc-rewarder --test quickchain_preflight_boundary
    cargo test -p svc-rewarder --test quickchain_preflight_raw_engagement
    cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
    cargo test -p svc-rewarder --test quickchain_preflight_funding_source
    cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
    cargo test -p svc-rewarder --test quickchain_preflight_docs

These tests are intentionally narrow and should remain fast.

## Full dev gate

Run this from the repository root:

    crates/svc-rewarder/scripts/dev-quickchain-preflight.sh

The script performs:

1. `cargo fmt -p svc-rewarder -- --check`
2. focused QuickChain preflight tests
3. `cargo test -p svc-rewarder --all-targets`
4. `cargo clippy -p svc-rewarder --all-targets -- -D warnings`

## Future QuickChain parking lot

Do not implement these in `svc-rewarder` until the wider QuickChain gates are explicitly opened:

- canonical bytes and locked vectors for root preimages;
- state/account Merkle roots;
- receipt roots;
- validator-set logic;
- checkpoint signing;
- pruning;
- external DA;
- public anchors;
- bridges;
- staking or liquidity;
- CrabLink chain authority;
- gateway/omnigate/rewarder ledger mutation.

When those gates open, the rewarder should still remain a planner. Chain/root/proof work should land
in the proper QuickChain/ledger/proof crates, not by smuggling authority into rewarder DTOs.

## Park-ready meaning for this crate

For Phase 0, `svc-rewarder` is considered park-ready when:

- focused QuickChain preflight tests pass;
- all-targets tests pass;
- Clippy passes with `-D warnings`;
- docs state the planning-only boundary;
- wallet/ledger ownership remains clear;
- no external settlement or root-producing code appears in the crate.
