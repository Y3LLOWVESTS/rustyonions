# Retention and Data Availability Preconditions

RO:WHAT — QC-0A retention, pruning, archive, carrier, and DA safety rules.
RO:WHY — Prevent unsafe pruning and fake DA before proof/challenge/archive fallback is proven.
RO:INTERACTS — ron-ledger, ron-accounting, svc-storage, future carrier/archive layer, future QuickChain checkpoints.
RO:INVARIANTS — no pruning before DA/challenge/archive fallback; economic receipts outrank raw engagement telemetry.
RO:METRICS — future pruning blocked and missing DA challenge counters.
RO:CONFIG — future hot/warm/cold/archive retention windows.
RO:SECURITY — checkpoint roots alone do not justify deletion of raw proof data.
RO:TEST — future da_missing_blocks_pruning and restore-from-proof tests.

## Pre-DA rule

Before DA is proven, retention must be boring.

```text
No pruning in local-root mode.
No pruning in epoch-sealing mode.
No pruning merely because a checkpoint root exists.
No pruning merely because a carrier assignment exists.
No pruning until missing-data challenges work.
No pruning until archive fallback is tested.
No pruning until restore-from-proof is tested.
```

## Pre-DA scale rule

Before DA is proven, QuickChain must not promise web-scale raw event retention.

High-volume raw events must be:

```text
analytics-only local retention
aggregated sealed summaries
sampled metering
bounded experiment data
explicitly funded campaign data
proof-relevant data with retention cap
```

Economic receipts take priority over engagement telemetry.

## Pruning preconditions

```text
[ ] checkpoint roots exist
[ ] challenge window is defined
[ ] receipt inclusion proofs exist
[ ] account proofs exist
[ ] missing data challenge exists
[ ] archive fallback exists
[ ] restore-from-proof test passes
[ ] DA root/proof format exists if pruning depends on DA
```

## No-go

If missing DA cannot block pruning, pruning must remain disabled.
