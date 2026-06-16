# ron-accounting QuickChain Preflight Boundary

RO:WHAT — Documents the QuickChain Phase-0 boundary for `ron-accounting`.
RO:WHY — `ron-accounting` must remain derivative metering/snapshot infrastructure, not balance truth, not wallet authority, not ledger authority, and not QuickChain root production.
RO:INTERACTS — `UsageEvent`, `Recorder`, `SealedSlice`, `RewardSnapshotExport`, `ProjectedRewardSnapshot`, `svc-storage`, `svc-gateway`, `omnigate`, `svc-rewarder`, `svc-wallet`, `ron-ledger`, future QuickChain DTOs.
RO:INVARIANTS — usage only; integer counters only; no wallet mutation; no ledger mutation; no balances; no receipts; no roots; no checkpoints; no validators; no settlement; no external anchors; raw engagement cannot allocate protocol ROC.
RO:METRICS — accounting metrics may describe usage, rows, sealing, export, and projection health only; metrics must not imply balances or finality.
RO:CONFIG — `ron-accounting` config controls recorder/export/WAL/HTTP limits only; it must not enable QuickChain chain behavior.
RO:SECURITY — unknown authority fields reject at DTO boundaries; artifact CIDs are not roots; idempotency is retry/dedupe only; reward projection is planning input only.
RO:TEST — `crates/ron-accounting/scripts/dev-quickchain-preflight.sh` and `crates/ron-accounting/scripts/dev-quickchain-park.sh`.

---

## 0. Status

`ron-accounting` is now in a strong QuickChain Phase-0 posture.

It is **not** a chain.

It is **not** a ledger.

It is **not** a wallet.

It is **not** a settlement service.

It is **not** a root producer.

It is **not** a validator.

It is **not** a payout executor.

It is derivative infrastructure that records bounded usage counters, seals deterministic accounting artifacts, and emits reward-planning inputs for downstream policy/rewarder code.

The current allowed role is:

```text
usage/metering input
→ normalized counters
→ sealed accounting slices
→ deterministic artifact hashes/CIDs
→ reward-compatible planning snapshots
→ svc-rewarder planning input
```

The forbidden role is:

```text
usage/metering input
→ direct balance mutation
→ direct wallet mutation
→ direct ledger mutation
→ direct payout execution
→ QuickChain root/finality/checkpoint authority
```

---

## 1. Role in the RustyOnions value loop

The intended internal ROC value loop is:

```text
ron-proto econ DTOs
→ ron-ledger truth
→ svc-wallet issue/transfer/burn/hold/capture/release/receipt
→ svc-storage/svc-gateway/omnigate paid enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ wallet/ledger receipts
```

`ron-accounting` sits after paid enforcement and before reward planning.

It may:

```text
record usage events
normalize labels
maintain bounded counters
seal usage windows
export sealed slices
generate deterministic reward-planning snapshots
emit b3 artifact CIDs over canonical snapshot bytes
```

It must not:

```text
invent balances
invent receipts
invent operation_id authority
invent account_sequence authority
commit wallet mutations
commit ledger mutations
mint ROC
burn ROC
transfer ROC
hold ROC
capture ROC
release ROC
mark settlement finalized
produce checkpoint roots
```

---

## 2. Accounting is not balance truth

`ron-accounting` counters are not balances.

A row like:

```text
tenant=7 service=provider-a dimension=bytes value=100
```

means only:

```text
ron-accounting observed or was told about 100 metered units under normalized labels.
```

It does not mean:

```text
provider-a owns ROC
provider-a is owed ROC
provider-a has a payable receipt
provider-a has a finalized reward
provider-a has a verified QuickChain proof
```

Only `svc-wallet` and `ron-ledger` may produce durable economic truth.

---

## 3. Current artifact types

The current crate exposes these relevant shapes:

```text
UsageEvent
EventIngestPolicy
UsageCounterInput
Recorder
CounterRow
SealedSlice
SliceId
SliceMeta
SliceRow
RewardSnapshotExport
RewardContributionExport
RewardProjectionConfig
RewardProjectionReport
ProjectedRewardSnapshot
RewardSnapshotInteropVector
```

These are metering, sealing, export, and planning artifacts.

They are not QuickChain consensus objects.

---

## 4. Artifact CID versus QuickChain root

`RewardSnapshotExport::canonical_cid()` and `canonical_snapshot_cid()` produce a canonical `b3:<64 lowercase hex>` artifact CID over canonical snapshot bytes.

That CID is allowed to be used as:

```text
snapshot artifact identifier
interop vector identifier
rewarder input artifact identifier
debug/test/reference hash
```

It must not be labeled or treated as:

```text
accounting_root
reward_root
state_root
receipt_root
checkpoint_root
checkpoint_hash
settlement finality
validator commitment
external anchor commitment
```

A deterministic artifact hash is useful, but it is not a QuickChain root unless and until future canonical root vectors and root-producing code are explicitly implemented after the proper gates.

---

## 5. Event-class doctrine

QuickChain event classes are doctrine at this stage.

The doctrine classes are:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

`ron-accounting` must not allow clients to smuggle these as authority fields into usage-event or ingest DTOs.

Current Phase-0 accounting behavior:

```text
BytesStored      → reward-projection eligible metering input
BytesServed      → reward-projection eligible metering input
UptimeSeconds    → reward-projection eligible metering input

RequestOk        → not reward-projection eligible by itself
PinSeconds       → not reward-projection eligible by itself
CpuUnits         → not reward-projection eligible by itself
Custom(...)      → not reward-projection eligible by itself
```

Raw engagement examples:

```text
views
likes
comments
impressions
watch_seconds
ad_impression
ad_click
sponsor_view
campaign_view
```

These must not directly allocate protocol ROC.

They may be used later for analytics, dashboards, fraud scoring, ad-budget accounting, or policy inputs only if a future crate explicitly gates them. They must not become an automatic protocol reward denominator.

---

## 6. Ingest boundary

The lightweight ingest adapter accepts storage-style usage event batches.

Its idempotency model is retry safety only:

```text
HTTP Idempotency-Key header = retry/dedupe key
```

It is not:

```text
operation_id
receipt id
ledger id
account sequence
settlement id
checkpoint id
```

Ingest bodies must not accept:

```text
idempotency_key as body authority
operation_id
account_sequence
state_root
receipt_root
accounting_root
reward_root
checkpoint_root
validator
finality
settlement_status
bridge
staking
liquidity
payout_authorized
ledger_mutation
wallet_mutation
event_class
```

Malformed schema, malformed b3 CIDs, oversized batches, invalid nested usage events, and authority-looking unknown fields must reject before recording.

---

## 7. Reward projection boundary

Reward projection converts sealed slices into `RewardSnapshotExport`.

It is allowed to:

```text
aggregate reward-eligible metering counters
sort accounts deterministically
use integer-only accumulation
produce a reward-planning snapshot
produce a b3 artifact CID for the snapshot
report ignored rows
```

It is not allowed to:

```text
execute payout
authorize payout
commit payout
mutate wallet
mutate ledger
mint ROC
transfer ROC
issue ROC
burn ROC
create receipts
create roots
mark finality
```

`svc-rewarder` remains the next planning layer.

`svc-wallet` remains the mutation front-door.

`ron-ledger` remains durable truth.

---

## 8. Current QuickChain preflight test inventory

The focused QuickChain preflight suite is:

```text
crates/ron-accounting/tests/quickchain_preflight_boundary.rs
crates/ron-accounting/tests/quickchain_preflight_ingest_poisoning.rs
crates/ron-accounting/tests/quickchain_preflight_snapshot_non_authority.rs
crates/ron-accounting/tests/quickchain_preflight_reward_dto_strictness.rs
crates/ron-accounting/tests/quickchain_preflight_reward_projection_boundary.rs
crates/ron-accounting/tests/quickchain_preflight_event_class_boundary.rs
```

These tests prove:

```text
usage DTOs reject authority drift
ingest DTOs reject authority drift
malformed b3 CIDs reject
schema drift rejects
oversized batches reject
body idempotency/operation identity rejects
snapshot DTOs reject roots/finality/mutation fields
reward DTOs reject unknown authority fields
pool_minor_units remains integer string only
artifact CIDs are not roots
raw engagement does not become protocol ROC
event-class fields cannot be smuggled
ad-budgeted style events do not directly allocate protocol ROC
route wording cannot smuggle reward authority
reward projection is deterministic planning input only
```

---

## 9. Runbook

From repo root:

```bash
cargo fmt -p ron-accounting
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
cargo check --workspace
```

Final parking gate:

```bash
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

Expected result:

```text
== ron-accounting QuickChain preflight gate passed ==
== ron-accounting QuickChain parking gate passed ==
```

Known unrelated workspace warnings may still appear from other crates. They should not be treated as `ron-accounting` failures unless the project later promotes the entire workspace to a zero-warning policy.

---

## 10. No-go checklist for parking

`ron-accounting` can be parked for the current QuickChain Phase-0 pass when:

```text
[x] normal ron-accounting tests pass
[x] WAL feature tests pass
[x] clippy passes with -D warnings
[x] docs/quickchain-preflight.md exists
[x] local dev preflight script exists
[x] accounting snapshots documented as derivative only
[x] reward snapshots documented as planning input only
[x] snapshot CIDs documented as artifact CIDs, not roots
[x] no root-producing code added
[x] no settlement/finality labels accepted as authority
[x] no validator/checkpoint/anchor fields accepted as authority
[x] no wallet mutation dependency added
[x] no ledger mutation dependency added
[x] ingest idempotency remains retry/dedupe only
[x] malformed schema rejects
[x] malformed b3 CIDs reject where applicable
[x] unknown authority fields reject at DTO boundaries
[x] event classes are documented and tested
[x] analytics/raw engagement cannot become protocol ROC directly
[x] reward projection cannot execute payout
[x] next handoff to svc-rewarder is documented
```

Still future/not in this crate:

```text
[ ] QuickChain canonical roots
[ ] QuickChain accounting_root
[ ] QuickChain reward_root
[ ] QuickChain checkpoint_hash
[ ] validator replay
[ ] external anchors
[ ] pruning
[ ] DA/challenge/archive fallback
```

Those remain blocked by the broader QuickChain vector gates.

---

## 11. Handoff to svc-rewarder

After parking `ron-accounting`, the next crate should be:

```text
crates/svc-rewarder
```

`svc-rewarder` must prove:

```text
it consumes accounting snapshots as planning input only
it does not mutate ledger directly
it does not mutate wallet directly
it emits deterministic payout plans/intents only
it requires explicit funding source for reward plans
it rejects raw engagement as primary protocol ROC denominator
it cannot double-plan or double-issue across replay
it routes all economic mutation through svc-wallet
it never fabricates receipts or balances
```

Recommended first `svc-rewarder` QuickChain preflight families:

```text
quickchain_preflight_boundary.rs
quickchain_preflight_raw_engagement_rejection.rs
quickchain_preflight_funding_source_required.rs
quickchain_preflight_no_wallet_ledger_mutation.rs
quickchain_preflight_replay_no_double_issue.rs
```

---

## 12. Parking decision

If the final parking gate passes, `ron-accounting` is safe to park for the current QuickChain Phase-0 pass.

Suggested status after parking:

```text
ron-accounting QuickChain Phase-0 slice: 94–96%
future QuickChain root/Phase-1 accounting role: not started by design
next active crate: svc-rewarder
```
