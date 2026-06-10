# QuickChain Preflight

RO:WHAT — Preflight documentation for QuickChain, the future ROC checkpoint/settlement spine.
RO:WHY — QuickChain must begin as safety-first, future-gated work before any chain runtime exists.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-registry, ron-kms, ron-audit.
RO:INVARIANTS — no live chain; no external settlement; no fake receipts; no ledger mutation outside svc-wallet/ron-ledger.
RO:METRICS — none yet; future metrics are checkpoint/proof/challenge/reward/carrier counters.
RO:CONFIG — QuickChain remains disabled by default.
RO:SECURITY — no ROX/Solana bridge, staking, liquidity, public validator market, or private-key custody.
RO:TEST — docs reviewed with ron-proto DTO strictness tests.

## Status

QuickChain is a future-chain blueprint.

This directory exists to make QuickChain harder to implement incorrectly.

Current scope:

Allowed:

- threat model
- preflight gates
- crate ownership map
- DTO boundaries
- canonicalization notes
- strict ron-proto DTO scaffolding
- tests that prove DTOs reject drift

Forbidden:

- live consensus
- validators
- public chain state
- ROX
- Solana
- bridges
- staking
- liquidity
- exchange-facing logic
- runtime ledger mutation from QuickChain
- chain authority in CrabLink

## Phase 0 rule

QuickChain Phase 0 is not a blockchain implementation.

Phase 0 is:

- determinism before distribution
- DTOs before roots
- roots before validators
- proofs before pruning
- internal ROC before external anchors

## First implementation slice

The first safe code slice is ron-proto::quickchain.

ron-proto::quickchain may define strict DTO shapes and validation helpers.

It must not:

- hash live ledger state
- produce roots
- verify validator signatures
- mutate balances
- call services
- write storage
- open sockets
- spawn tasks
- anchor externally

## Acceptance

After applying the first slice:

    cargo fmt -p ron-proto
    cargo clippy -p ron-proto --all-targets -- -D warnings
    cargo test -p ron-proto --all-targets

Recommended wider gate before the next QuickChain slice:

    cargo check --workspace
    cargo test -p ron-ledger
    cargo test -p svc-wallet
    cargo test -p ron-accounting
    cargo test -p svc-rewarder
    cargo test -p ron-policy
    cargo test -p svc-registry
    cargo test -p ron-kms
    cargo test -p ron-audit
