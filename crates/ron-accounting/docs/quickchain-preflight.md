# ron-accounting QuickChain Preflight Boundary

RO:WHAT — Documents the QuickChain Phase-0 boundary for `ron-accounting`.
RO:WHY — Keeps accounting as derivative metering, sealed snapshot, and reward-planning infrastructure without becoming balance truth, wallet authority, ledger authority, or QuickChain root authority.
RO:INTERACTS — ron-accounting usage events, sealed slices, reward snapshots, svc-rewarder planning, svc-wallet mutation front-door, ron-ledger economic truth, ron-proto DTOs.
RO:INVARIANTS — Accounting is not balance truth; snapshots are derivative artifacts; raw engagement is not protocol ROC authority; no roots/checkpoints/validators/settlement/anchors/bridges.
RO:METRICS — Accounting metrics may count ingest, windows, rows, projection reports, and exporter status; they do not represent balances.
RO:CONFIG — Uses accounting/reward projection config only; no chain params, validator sets, anchor cadence, bridge config, staking config, or liquidity config.
RO:SECURITY — No fake balances, no fake receipts, no silent spend, no wallet mutation, no ledger mutation, no payout execution, no external settlement.
RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_docs; crates/ron-accounting/scripts/dev-quickchain-preflight.sh; crates/ron-accounting/scripts/dev-quickchain-park.sh.

## 0. Status

This crate is in QuickChain Phase 0 / preflight.

`ron-accounting` is not a chain.

`ron-accounting` is not a ledger.

`ron-accounting` is not a wallet.

`ron-accounting` is not a root producer.

`ron-accounting` is not a settlement service.

`ron-accounting` is not validator infrastructure.

`ron-accounting` is not bridge infrastructure.

`ron-accounting` is not staking or liquidity infrastructure.

Accounting is not balance truth.

The only acceptable Phase-0 role is:

```text
usage / metering input
→ normalization
→ bounded counters
→ deterministic sealed slices/windows
→ derivative reward snapshot artifacts
→ svc-rewarder planning input
```

The mutation path remains:

```text
svc-wallet
→ ron-ledger
→ backend-derived receipt
```

Accounting must not bypass that path.

## 1. Role in the internal ROC value loop

The current authority model is:

```text
ron-proto:
  DTOs and validation shapes only

svc-wallet:
  ROC wallet mutation front-door

ron-ledger:
  durable economic truth

ron-accounting:
  derivative usage, metering, sealed snapshots, and reward-planning artifacts

svc-rewarder:
  payout planning only

svc-wallet again:
  approved payout intent commits through wallet/ledger truth path

ron-ledger again:
  final balance truth and receipt truth
```

`ron-accounting` may help answer questions like:

```text
how many bytes were stored
how many bytes were served
which account contributed storage work
which window was sealed
what deterministic planning artifact was exported
```

`ron-accounting` must not answer as truth:

```text
what is the user's spendable balance
whether a payment finalized
whether a receipt is included in a root
whether a payout has executed
whether a checkpoint is final
whether an external anchor exists
```

## 2. Non-authority boundary

`ron-accounting` must not perform or claim any of the following:

```text
issue ROC
mint ROC
burn ROC
transfer ROC
hold ROC
capture a hold
release a hold
expire a hold
settle a payment
finalize a payment
anchor a payment
bridge a payment
stake ROC
provide liquidity
mutate wallet state
mutate ledger state
issue wallet receipts
invent fake receipts
invent fake balances
unlock paid content
```

Valid accounting artifacts are derivative reports only.

Invalid authority fields include:

```text
balance
available_balance
spendable_balance
balance_minor
wallet_balance
ledger_balance
wallet_mutation
ledger_mutation
operation_id as client authority
account_sequence as client authority
settlement
settlement_status as client finality
finality
finalized
checkpoint
checkpoint_root
state_root
accounting_root
reward_root
receipt_root
validator
validator_signature
anchor
external_anchor
bridge
staking
liquidity
rox
solana
```

These words may appear in this document only to deny them.

They must not become `ron-accounting` runtime authority.

## 3. Current accounting artifacts

Current safe artifacts include:

```text
UsageEvent
UsageEventsIngestRequest
EventIngestPolicy
LabelSet
Dimension
Recorder
CounterRow
SliceRow
SealedSlice
Window
RewardProjectionConfig
RewardProjectionReport
ProjectedRewardSnapshot
RewardSnapshotExport
RewardContributionExport
canonical_snapshot_bytes
canonical_snapshot_cid
reward_snapshot_interop_vector_v1
```

These artifacts may be serialized, exported, and checked for deterministic byte stability.

They are not QuickChain roots.

They are not balances.

They are not receipts.

They are not settlement proofs.

They are not validator proofs.

They are not paid-content unlock authority.

## 4. Artifact CID vs QuickChain root

A reward snapshot CID is an artifact hash.

A sealed slice digest is an accounting artifact digest.

A canonical snapshot CID is an artifact CID.

These are allowed:

```text
snapshot_cid
canonical_snapshot_cid
artifact_cid
sealed_slice_digest
interop_vector_digest
```

These are not allowed in Phase 0 runtime output:

```text
accounting_root
reward_root
state_root
receipt_root
checkpoint_root
epoch_root
validator_root
settlement_root
anchor_root
```

Artifact CIDs help prove exact bytes for reports and future vector work.

They do not make accounting a QuickChain root producer.

They do not replace the future root/vector gate.

## 5. Event-class doctrine

QuickChain event-class doctrine uses these meanings:

```text
economic_receipt:
  backend wallet/ledger economic receipt evidence

metering:
  usage measurement, not direct balance effect

proof_eligible:
  future proof/challenge candidate after explicit validation and policy

ad_budgeted:
  advertiser/sponsor budget lane, not protocol inflation

analytics_only:
  reporting/fraud/product signal only; never direct protocol ROC payout
```

For `ron-accounting`, the safe default is conservative:

```text
metering:
  normal accounting lane

analytics_only:
  ignored for economic reward projection

proof_eligible:
  may be preserved only as candidate evidence, not payout authority

ad_budgeted:
  may be counted only under explicit budget policy, not as mint authority

economic_receipt:
  must originate from wallet/ledger evidence, not raw usage events
```

Raw engagement must never directly mint, allocate, transfer, or mutate protocol ROC.

Examples of raw engagement that must not directly become protocol ROC:

```text
views
likes
impressions
clicks
watch time
dwell time
followers
shares
comments
raw site visits
passive engagement
```

Future rewards must pass through:

```text
accounting summary
→ svc-rewarder deterministic payout planning
→ explicit wallet path
→ ron-ledger durable truth
→ backend-derived receipt
```

## 6. Ingest boundary

HTTP ingest and library ingest are input boundaries.

They must reject or ignore authority-smuggling fields.

The body must not supply wallet or ledger authority.

The body must not supply QuickChain finality.

The body must not supply roots.

The body must not supply validator claims.

The body must not supply bridge or anchor claims.

`Idempotency-Key` is retry safety only.

It is not:

```text
operation_id
account_sequence
wallet authority
ledger authority
receipt authority
root authority
settlement authority
```

Malformed or noncanonical IDs must reject before recording.

Poisoned finality/root fields must reject or remain inert.

Duplicate ingest retries must not create duplicate economic authority.

## 7. Reward projection boundary

Reward projection is planning input.

Reward projection may:

```text
read sealed slices
filter eligible rows
aggregate contribution data
produce deterministic planning artifacts
produce canonical bytes
produce artifact CIDs
feed svc-rewarder
```

Reward projection must not:

```text
execute payout
issue payout receipt
mutate wallet
mutate ledger
claim balance truth
claim settlement
claim finality
claim checkpoint inclusion
produce QuickChain roots
create validator evidence
anchor externally
```

Handoff to svc-rewarder is a planning handoff.

Handoff to `svc-rewarder` is not wallet execution.

## 8. Window and determinism boundary

Accounting windows must be explicit data artifacts.

Window boundaries may be based on event timestamps, slice IDs, and configured durations.

Accounting must not create wall-clock roots.

Accounting must not depend on database iteration order for future vector material.

When deterministic bytes are produced, they must be sorted and reproducible.

No DB-order roots.

No wall-clock roots.

No placeholder hashes.

No fake hashes.

No root-producing code before canonical bytes and golden vectors authorize it.

## 9. Test inventory

Current QuickChain preflight tests should cover these classes:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_event_class_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_snapshot_non_authority
quickchain_tooling_boundary
```

The exhaustive local preflight must discover every `quickchain*.rs` integration test dynamically.

The test matrix must not be silently hardcoded and forgotten.

## 10. Runbook

Focused docs test:

```bash
cargo test -p ron-accounting --test quickchain_preflight_docs
```

Focused QuickChain tooling test:

```bash
cargo test -p ron-accounting --test quickchain_tooling_boundary
```

Full crate-local preflight:

```bash
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Final parking gate:

```bash
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

Expected final marker:

```text
== ron-accounting QuickChain parking gate passed ==
```

## 11. No-go checklist

Before `ron-accounting` is parked for this Phase-0 pass:

```text
[ ] normal ron-accounting tests pass
[ ] WAL feature tests pass, or any limitation is explicitly documented
[ ] strict clippy passes
[ ] docs/quickchain-preflight.md exists
[ ] quickchain_preflight_docs passes
[ ] quickchain_tooling_boundary passes
[ ] dev-quickchain-preflight.sh discovers quickchain*.rs tests dynamically
[ ] dev-quickchain-park.sh runs the preflight and docs regression test
[ ] accounting snapshots are documented as derivative only
[ ] reward snapshots are documented as planning input only
[ ] snapshot CIDs are documented as artifact CIDs, not QuickChain roots
[ ] raw engagement is documented as non-authoritative
[ ] event-class doctrine is documented
[ ] no roots
[ ] no receipt roots
[ ] no account state roots
[ ] no accounting_root
[ ] no reward_root
[ ] no checkpoints
[ ] no validators
[ ] no settlement
[ ] no anchors
[ ] no external anchors
[ ] no bridges
[ ] no staking
[ ] no liquidity
[ ] no Solana/ROX/external settlement path
[ ] no fake balances
[ ] no fake receipts
[ ] no silent spend
[ ] no wallet mutation
[ ] no ledger mutation
[ ] no payout execution
```

## 12. Next crate handoff

The next coherent QuickChain crate after parking `svc-wallet + ron-accounting` is:

```text
svc-rewarder + svc-storage
```

The reason:

```text
svc-rewarder:
  deterministic payout planning only; no ledger mutation

svc-storage:
  bytes, b3 integrity, paid storage/access metering; no balance truth
```

`ron-accounting` hands planning artifacts to `svc-rewarder`.

`ron-accounting` does not execute payouts.


## Phase 2 Round 2 committee boundary

ron-accounting may produce deterministic metering and reward-planning artifacts that later verifier tooling can read, but it is not a committee member, not an attestation signer, not quorum authority, not fork-choice authority, not finality authority, not settlement authority, and not balance truth.

For this phase the safe accounting rule remains:

    accounting snapshot = derivative metering artifact
    reward snapshot CID = artifact hash, not committee attestation
    committee attestation = not produced by ron-accounting
    quorum certificate = not produced by ron-accounting
    fork choice = not produced by ron-accounting
    finality = not produced by ron-accounting
    staking/slashing/bonding = not ron-accounting active runtime scope
    external settlement / bridge = forbidden

The Phase 2 Round 2 gate must preserve no wallet mutation, no ledger mutation, no fake balances, no fake receipts, no external settlement, no bridge, no staking, no slashing, no validator-economy behavior, and no raw engagement direct protocol payout.

## Phase 4 Round 1 bond report boundary

Phase 4 Round 1 may add read-only bond report DTOs. These reports are derivative summaries only.

Accounting is not bond truth, slash truth, balance truth, wallet mutation authority, ledger mutation authority, payout execution authority, staking authority, liquidity authority, bridge authority, or external settlement authority.
