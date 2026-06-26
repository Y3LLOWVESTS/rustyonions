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

---

## Phase 1 Round 2 downstream confirmation

`svc-rewarder` remains downstream-light in Phase 1 Round 2.

Required confirmation:

```text
reward plans can be referenced as artifacts
reward manifest commitments are artifact references, not QuickChain roots
storage artifact CIDs are artifact references, not QuickChain roots
svc-rewarder cannot mutate ledger truth
svc-wallet commits approved payout intents
ron-ledger remains durable economic truth
```

The focused confirmation suite is:

```text
quickchain_phase1_round2_confirmation
```

Still forbidden:

```text
no direct ledger mutation
no root-producing rewarder runtime
no checkpoint production
no validator behavior
no bridge or external settlement
no staking or liquidity
no raw engagement protocol ROC minting
no fake balances
no fake receipts
no fake finality
```

---

## Phase 2 Round 1 verifier artifact / read-only replication boundary

This crate-pair status is Phase 2 Round 1 verifier artifact / read-only replication.

`quickchain_phase2_replay_boundary` is the focused regression target for this boundary.

For `svc-rewarder`:

- reward manifests may become read-only verifier artifact inputs
- reward manifests may be replayed by independent verifiers as evidence material
- reward manifests are not committee votes
- reward manifests are not quorum decisions
- reward manifests are not fork choice
- reward manifests are not finality
- reward manifests are not validator signatures
- reward manifests are not balance truth
- reward manifests are not direct payout execution

Hard boundary:

- svc-rewarder does not sign committee votes
- svc-rewarder does not decide quorum
- svc-rewarder does not claim fork choice
- svc-rewarder does not claim finality
- svc-rewarder still cannot mutate ledger truth
- svc-wallet commits approved payout intents
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
- direct ledger mutation
- fake receipts
- fake balances
- fake finality

<!-- BEGIN QUICKCHAIN PHASE 2 ROUND 2 COMMITTEE BOUNDARY -->

## QuickChain Phase 2 Round 2 committee readiness boundary

This section locks the Phase 2 Round 2 committee readiness boundary for `svc-rewarder`.

Required markers for `quickchain_phase2_committee_boundary`:

```text
phase 2 round 2 committee readiness boundary
reward manifests remain payout planning artifacts
wallet issue requests remain explicit svc-wallet handoff previews
svc-rewarder is not a committee member
svc-rewarder does not produce signed verification attestations
svc-rewarder does not decide quorum
svc-rewarder cannot claim fork choice
svc-rewarder cannot claim finality
svc-rewarder cannot create validator rewards from raw engagement
svc-rewarder cannot mutate ledger truth
svc-wallet commits approved payout intents
ron-ledger remains durable economic truth
quickchain_phase2_committee_boundary
```

Allowed in this crate:

```text
- deterministic payout planning
- reward manifests
- manifest commitments as artifact references
- explicit wallet issue request previews
- idempotent wallet handoff planning
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
- direct ledger mutation
- raw engagement protocol payout authority
```

<!-- END QUICKCHAIN PHASE 2 ROUND 2 COMMITTEE BOUNDARY -->

## Phase 3 Round 1 validator/passport boundary

svc-rewarder remains deterministic payout planning only.

Phase 3 validator/passport material is not payout execution authority.
Phase 3 validator/passport material is not wallet authority.
Phase 3 validator/passport material is not ledger authority.
Phase 3 validator/passport material is not settlement finality.
Phase 3 validator/passport material is not staking or slashing authority.

svc-rewarder must reject or ignore client-supplied validator/passport authority fields such as validator_set_hash, validator_capability, passport_registry_proof, bond_required, bonded_economics, staking_power, and slash_evidence.

No validator admission.
No validator revocation.
No validator rotation.
No passport registry authority.
No capability authority.
No bonded economics.
No staking.
No slashing.
No direct wallet mutation.
No direct ledger mutation.

## Phase 4 Round 1 bond planning boundary

Phase 4 Round 1 may introduce bond-related reports or future planning references, but `svc-rewarder` remains deterministic payout planning only.

`svc-rewarder` is not bond truth, slash truth, balance truth, wallet mutation authority, ledger mutation authority, validator reward authority, public staking authority, liquidity authority, bridge authority, or external settlement authority.

Any future economic action must still follow the approved path:

svc-rewarder deterministic plan
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
-> backend-derived receipt

The current Phase 4 Round 1 guard is test-only boundary hardening. It does not add live slashing, public staking, liquidity, ROX, Solana, bridge, or external settlement behavior.

## QuickChain Phase 4 Round 2 — Disputed-bond reward boundary

svc-rewarder may see downstream reports or operator context about disputed-bond simulation in later phases, but in Phase 4 Round 2 it remains deterministic payout planning only.

Required boundary:

- challenge/freeze/appeal/slash simulation must not create reward payouts
- disputed-bond state must not become validator reward authority
- slash evidence must not become payout authority
- rejected slash simulation must not become protocol reward input
- rewarder must not mutate wallet or ledger state
- rewarder must not produce bond/slash/penalty receipts
- rewarder must not create staking, liquidity, bridge, ROX, Solana, or external settlement behavior
- svc-wallet remains mutation front-door
- ron-ledger remains economic truth

