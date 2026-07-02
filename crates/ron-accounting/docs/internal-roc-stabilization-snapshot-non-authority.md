# Internal ROC Stabilization — ron-accounting Snapshot Non-Authority Boundary

RO:WHAT — Product beta readiness boundary for ron-accounting as derivative metering, snapshot, report, and reward-planning input infrastructure.

RO:WHY — Product beta needs accounting snapshots to be deterministic and useful without becoming wallet, ledger, receipt, balance, entitlement, payout, finality, bridge, staking, liquidity, or external-settlement authority.

RO:INTERACTS — accounting/events.rs, accounting/reward_snapshot.rs, accounting/reward_projection.rs, accounting/slice.rs, http_ingest.rs, existing Internal ROC beta accounting tests.

RO:INVARIANTS — accounting may observe accepted wallet/ledger-derived receipts; accounting may produce deterministic artifacts and b3 CIDs; accounting must never mutate wallet/ledger state or create economic truth.

RO:SECURITY — no wallet mutation, no ledger mutation, no issue, no transfer, no burn, no hold, no capture, no release, no fake receipt, no fake balance, no fake finality, no paid unlock, no payout execution, no bridge, no ROX/Solana, no staking, no liquidity, no external settlement.

RO:TEST — cargo test -p ron-accounting --test internal_roc_stabilization_snapshot_non_authority_boundary.

## Allowed

usage event ingest
strict metering/event validation
authority-poison rejection
normalized derivative counters
sealed accounting slices
deterministic snapshot artifacts
canonical reward snapshot material
b3 artifact CIDs
downstream reward-planning input

## Forbidden

accounting as balance truth
accounting as receipt truth
accounting as payout truth
accounting as paid entitlement truth
accounting as finality truth
accounting as wallet mutation authority
accounting as ledger mutation authority
accounting as raw-engagement direct mint authority
accounting as bridge or external settlement authority

## Product beta lock

The correct path is:

wallet/ledger accepted receipt
→ accounting observation
→ deterministic accounting artifact
→ reward-planning input

The incorrect path remains forbidden:

accounting snapshot
→ balance truth
→ receipt truth
→ paid unlock
→ payout execution
