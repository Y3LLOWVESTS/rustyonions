
# QUICKCHAIN_BUILDPLAN.MD — Minimum-Round Phase Plan for QuickChain

RO:WHAT — Defines the efficient round-based build plan for QuickChain phases 0 through 5.
RO:WHY — Prevents wasteful crate-pair churn by making every phase, round, crate-pair pass, and exit gate explicit.
RO:INTERACTS — QUICKCHAIN_REVIEW_BUNDLE.MD, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-storage, svc-gateway, omnigate, svc-index, ron-policy, CrabLink Tauri/client adapters, future validator/registry/passport components.
RO:INVARIANTS — ROC remains internal until proven; svc-wallet remains mutation front-door; ron-ledger remains economic truth; ron-accounting is not balance truth; svc-rewarder plans payouts but does not mutate ledger; no fake receipts; no silent spend; no root-producing code without canonical bytes and golden vectors; no public bridge/staking/liquidity/external settlement until explicitly authorized by a later phase.
RO:METRICS — Future phase/round completion metrics may track crate-pair park status, vector coverage, replay determinism, proof verification, verifier agreement, validator readiness, archive/DA challenge readiness, and anchor status.
RO:CONFIG — Phase gates and feature flags must keep QuickChain future-only until the required prior phase exits are green.
RO:SECURITY — No ROX/Solana/external L2/DA/public bridge/external settlement in active runtime; no gateway/omnigate/rewarder/index/policy/client authority over balances, roots, finality, or settlement truth.
RO:TEST — Each round requires focused tests first, then crate-local park/preflight once, then notes; final phase audits may use broader cross-crate checks.

---

## 0. Purpose

This file exists to make QuickChain work efficient.

Before this plan, it was easy to think in terms of:

```text
one crate pair
then notes
then codebundle
then cargo clean
then rebuild
then next crate pair
repeat forever
````

That is too expensive.

The correct model is:

```text
One phase = as few full crate-pair rounds as possible.
One round = one intentional pass through the crate-pair sequence.
Each round has:
  - a purpose
  - exact crate-pair work
  - an exit gate
  - no duplicate exhaustive runs
```

The goal is to get more done with fewer crate-pair cycles, fewer codebundle uploads, fewer note saves, fewer cargo cleans, and fewer rebuilds.

---

## 1. Non-negotiable doctrine

QuickChain is future ROC settlement infrastructure.

It is not current public blockchain runtime.

The active doctrine is:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Current hard boundaries:

```text
- ROC remains internal until proven.
- svc-wallet remains the mutation front-door.
- ron-ledger remains balance/economic truth.
- ron-accounting is not balance truth.
- svc-rewarder plans payouts but never mutates ledger.
- svc-storage stores bytes/artifacts; it is not payment truth.
- svc-gateway and omnigate enforce/publicly expose flows but do not mutate ledger truth.
- svc-index points to names/artifacts; it does not prove settlement.
- ron-policy decides declarative policy; it does not create settlement truth.
- CrabLink Tauri displays/user-confirms; it does not own backend truth.
```

Forbidden until explicitly authorized by the correct later phase:

```text
- ROX active runtime
- Solana active runtime
- external settlement
- public bridge
- staking
- liquidity
- exchange-facing logic
- public validator economy
- gateway/omnigate direct ledger mutation
- rewarder direct ledger mutation
- index/policy/cache/client as payment truth
- raw engagement metrics directly minting or allocating protocol ROC
- pruning before DA/challenge/archive fallback
- fake hashes/placeholders
- root-producing code without canonical bytes and locked golden vectors
```

---

## 2. Fixed crate-pair order

Use this same crate-pair order for every full round unless a phase explicitly narrows the scope.

```text
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + svc-storage
4. svc-gateway + omnigate
5. svc-index + ron-policy
6. CrabLink Tauri + client adapters
```

Why this order exists:

```text
DTOs
→ ledger truth
→ wallet mutation
→ accounting snapshots
→ rewarder payout planning
→ storage/paid artifact handling
→ gateway/omnigate paid enforcement
→ index/policy lookup and gating
→ CrabLink display/client boundary
```

This follows the internal ROC value loop:

```text
ron-proto economic DTOs
→ ron-ledger truth
→ svc-wallet issue/transfer/burn/hold/capture/release/receipt
→ svc-storage/svc-gateway/omnigate paid enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ wallet/ledger receipts
→ CrabLink display-only receipt/cache/user intent
```

---

## 3. Meaning of “round”

A round is not just “touching crates.”

A round is a complete, named pass through the crate-pair sequence with one specific goal.

Each round must answer:

```text
What are we proving?
Which crate pairs are touched?
Which crate pairs are only audited?
What is the exit gate?
What is forbidden in this round?
What notes/codebundles are required before the next pair?
```

A full round usually means all six crate pairs.

A narrow round may be allowed only if the phase objective is mostly core-local and the downstream crates only need confirmation.

---

## 4. Minimum-round master plan

Optimized full plan:

```text
Phase 0 — QC-0A safety/preflight
  Ideal full rounds: 1

Phase 1 — single-authority deterministic roots/proofs
  Ideal full rounds: 2

Phase 2 — small committee / replicated verification
  Ideal full rounds: 2

Phase 3 — passport-gated validator set
  Ideal full rounds: 2

Phase 4 — bonded / higher-stakes validator model
  Ideal full rounds: 3

Phase 5 — broader decentralization / external anchoring options
  Ideal full rounds: 3
```

Total ideal full rounds:

```text
13 full rounds
```

Current practical status at the time this file is created:

```text
Phase 0:
  essentially done / audit-only unless regressions appear

Phase 1:
  Round 1 is in progress
  ron-proto + ron-ledger have already been through the current pre-root slice
  svc-wallet + ron-accounting focused QC-1A pair interlock tests are green
  remaining Phase 1 Round 1 pairs:
    - svc-rewarder + svc-storage
    - svc-gateway + omnigate
    - svc-index + ron-policy
    - CrabLink Tauri + client adapters
```

---

# Phase 0 — QC-0A safety/preflight

## Goal

Prove the safety model, forbidden scope, DTO hygiene, canonicalization posture, and no-authority boundaries before roots.

## Ideal full rounds

```text
1
```

## Phase 0 Round 1 — Safety/preflight sweep

### Purpose

Freeze boundaries and prevent premature chain behavior.

### Crate-pair work

```text
ron-proto + ron-ledger:
  - strict DTOs
  - canonical JSON experiments
  - IDs/money validation
  - operation/idempotency/hold semantics
  - no root-producing runtime

svc-wallet + ron-accounting:
  - wallet is mutation front-door
  - accounting is snapshots only
  - no fake receipts/balances
  - no accounting mutation authority

svc-rewarder + svc-storage:
  - rewarder plans only
  - storage stores bytes/manifests only
  - no rewarder ledger mutation
  - no storage paid-unlock authority by cache alone

svc-gateway + omnigate:
  - gateway/omnigate enforce routes/policy boundaries
  - no direct ledger mutation
  - no fake receipts
  - no QuickChain authority

svc-index + ron-policy:
  - names/pointers/policy are not payment truth
  - policy is declarative only
  - index is lookup only

CrabLink Tauri + client adapters:
  - display/user intent only
  - no chain authority
  - no fake balances/receipts
  - no cache-only paid unlock
```

### Exit gate

```text
Every crate pair has:
  - focused QuickChain tests green
  - crate-local park/preflight green
  - docs/runbook updated
  - no forbidden scope markers
  - no duplicate exhaustive reruns
```

### Completion label

When green, call this:

```text
QC-0A safety/preflight complete
```

Do not call it:

```text
QuickChain complete
Phase 1 complete
chain ready
validator ready
settlement ready
```

---

# Phase 1 — single-authority deterministic roots/proofs

## Goal

One local authority can produce deterministic, replayable, independently reproducible roots/proofs from internal ROC truth.

## Ideal full rounds

```text
2
```

Phase 1 should not be six or seven full sweeps.

It should be:

```text
Round 1:
  QC-1A foundation/boundary sweep

Round 2:
  actual deterministic root/proof implementation + downstream confirmation
```

---

## Phase 1 Round 1 — QC-1A pre-root/root-material foundation

### Purpose

Prepare every crate for roots/proofs without letting any service become root authority.

### Current status

```text
In progress.
```

### Crate-pair work

```text
ron-proto + ron-ledger:
  - root material DTOs
  - canonical bytes/locked bytes groundwork
  - sorted material validation
  - duplicate rejection planning
  - replay projection
  - no live roots unless explicitly vector-authorized

svc-wallet + ron-accounting:
  - wallet receipts remain backend-derived
  - accounting observes after wallet/ledger truth
  - accounting snapshots are not balances/roots/finality
  - no payout/root/finality poison fields
  - no accounting mutation authority

svc-rewarder + svc-storage:
  - rewarder plans payout manifests only
  - storage stores artifacts/bytes only
  - snapshot CIDs/artifact CIDs are not QuickChain roots
  - no direct wallet/ledger mutation
  - no paid unlock from storage/cache alone

svc-gateway + omnigate:
  - paid enforcement stays backend-derived
  - no gateway/omnigate ledger mutation
  - no fake paid unlock
  - no QuickChain state authority
  - no route-level finality claims

svc-index + ron-policy:
  - index names/pointers are not proof
  - policy decisions are not settlement
  - no cache/name/policy as paid-unlock authority
  - no policy-created finality

CrabLink Tauri + client adapters:
  - display readiness only
  - no local proof authority
  - no client-side root/finality claims
  - no cache-only unlock
  - no fake balances/receipts
```

### Exit gate

```text
Every pair parked.
No root-producing authority outside ron-ledger/ron-proto planned core.
No fake hashes/placeholders.
No DB-order root path.
No wall-clock root path.
No validators.
No committee.
No anchors.
No settlement.
No staking.
No bridge.
No liquidity.
```

### Completion label

When green, call this:

```text
Phase 1 Round 1 complete
QC-1A foundation complete
Phase 1 foundation complete
```

Do not call it:

```text
Full Phase 1 complete
root engine complete
proof engine complete
validator ready
settlement ready
```

---

## Phase 1 Round 2 — deterministic root/proof implementation round

### Purpose

Actually complete Phase 1.

This round is core-heavy and downstream-light.

### Crate-pair work

```text
ron-proto + ron-ledger:
  - canonical payload bytes locked
  - locked_hash vectors added
  - real BLAKE3-256 hashes
  - preimage rule enforced:
      domain_separator_bytes || 0x00 || canonical_payload_bytes
  - hash format enforced:
      b3:<64 lowercase hex>
  - deterministic account/state roots
  - deterministic receipt roots
  - deterministic hold roots
  - deterministic epoch/checkpoint material
  - duplicate sort-key rejection
  - unordered input = same root
  - no DB iteration order
  - no wall-clock root material
  - no fake hashes/placeholders
  - proof DTOs and proof verification tests if in Phase 1 scope

svc-wallet + ron-accounting:
  - confirm wallet receipts can project into root material
  - confirm accounting artifacts cannot become balance/root truth
  - confirm accounting cannot change root material
  - no mutation changes unless integration requires it

svc-rewarder + svc-storage:
  - confirm reward plans can be referenced as artifacts
  - confirm storage can store/retrieve vector/root/proof artifacts by b3
  - confirm storage cannot mutate balances/unlocks
  - confirm rewarder cannot mutate ledger truth

svc-gateway + omnigate:
  - read/display proof artifacts only if routed
  - no mutation/finality claims
  - paid unlock still wallet/ledger-derived
  - route contract labels root/proof status honestly

svc-index + ron-policy:
  - index can point to artifacts but not prove them
  - policy can gate access but not create proof/finality
  - policy cannot turn a proof into spend authority

CrabLink Tauri + client adapters:
  - display proof/root readiness/status only
  - label statuses honestly:
      accepted
      epoch_included
      finalized
      anchored
  - no client root construction as authority
  - no client-side spend/finality claims
```

### Phase 1 completion gate

```text
Phase 1 is complete only when:
  - canonical bytes are locked
  - locked hashes are real and reproducible
  - receipt roots are deterministic
  - account/state roots are deterministic
  - hold roots are deterministic
  - epoch/checkpoint roots are deterministic
  - proof format is tested or explicitly deferred
  - independent verifier reproduces vectors
  - all downstream crates remain non-authority
  - final cross-crate audit passes
```

### Completion label

When green, call this:

```text
Phase 1 complete
single-authority deterministic roots/proofs complete
```

Still do not call it:

```text
public chain complete
committee complete
validator economy complete
staking complete
external settlement complete
```

---

# Phase 2 — small committee / replicated verification

## Goal

Multiple verifier nodes replay the same Phase 1 artifacts and agree on the same roots.

This is still not a public validator economy.

## Ideal full rounds

```text
2
```

---

## Phase 2 Round 1 — verifier artifact/read-only replication

### Purpose

Make Phase 1 artifacts independently replayable by small replicated verifiers.

### Primary work

```text
ron-proto + ron-ledger:
  - verifier input/output DTOs
  - replay bundle format
  - root/proof verification API
  - deterministic error taxonomy
  - replay result equivalence tests

svc-storage:
  - archive/retrieve replay bundles
  - verify b3 before serving
  - no mutation authority over replay outcome

svc-gateway + omnigate:
  - expose read-only proof/replay artifact routes if needed
  - no settlement/finality mutation

svc-index:
  - lookup artifact pointers only
  - no proof authority

ron-policy:
  - declare verifier access policy only
  - no policy-created verifier truth

CrabLink:
  - display verifier status
  - no client authority
```

### Exit gate

```text
Two independent verifier runs reproduce the same roots from the same artifacts.
No committee signing unless explicitly scoped.
No quorum/fork-choice unless explicitly scoped.
No staking.
No slashing.
No public bridge.
No external settlement.
```

---

## Phase 2 Round 2 — small committee agreement/readiness

### Purpose

Introduce minimal replicated verification semantics without validator economy.

### Primary work

```text
- validator-executable replay package
- committee member identity shape
- signed verification attestation DTOs
- quorum-readiness tests
- anti-double-attestation tests
- deterministic disagreement/error taxonomy
- no staking
- no slashing
- no public bridge
```

### Exit gate

```text
Small committee can independently verify roots and produce attestations.
Attestations are reproducible and bounded.
No bonded economics.
No external settlement.
No pruning.
```

### Completion label

When green, call this:

```text
Phase 2 complete
small committee replicated verification complete
```

Do not call it:

```text
public validator network
staking network
external settlement
bridge-ready
```

---

# Phase 3 — passport-gated validator set

## Goal

Validators are known/gated by RON Passport / registry / policy.

This is still not bonded high-stakes validation.

## Ideal full rounds

```text
2
```

---

## Phase 3 Round 1 — validator identity + registry gating

### Purpose

Turn small committee verification into passport-gated validator membership.

### Primary work

```text
svc-passport / ron-auth / svc-registry if included:
  - validator identity/capability shape
  - validator admission rules
  - revocation rules
  - capability expiry and rotation

ron-policy:
  - validator eligibility policy
  - deny-by-default admission
  - governance-gated validator set changes

ron-proto:
  - validator-set DTOs
  - attestation identity DTOs
  - validator capability DTOs

ron-ledger:
  - no validator balance authority
  - no staking/slashing
  - no validator mutation of user balances

gateway/omnigate/CrabLink:
  - display validator set/readiness only
  - no client-side validator authority
```

### Exit gate

```text
Only passport/registry-authorized validators can sign verification attestations.
Unauthorized identities reject.
Revoked validators reject.
Expired validator capabilities reject.
No bonded economics yet.
```

---

## Phase 3 Round 2 — validator operation hardening

### Purpose

Harden validator lifecycle.

### Primary work

```text
- validator rotation
- validator revocation
- equivocation evidence DTOs
- replay challenge evidence
- validator downtime/degraded status
- governance-gated parameter updates
- split-brain and double-attestation tests
```

### Exit gate

```text
Validator set can rotate safely.
Validator set can revoke safely.
Evidence can be generated for invalid/double attestations.
Still no staking.
Still no slashing.
Still no liquidity.
Still no public bridge.
```

### Completion label

When green, call this:

```text
Phase 3 complete
passport-gated validator set complete
```

---

# Phase 4 — bonded / higher-stakes validator model

## Goal

Add economic consequences to validator behavior.

This is much later and higher risk.

## Ideal full rounds

```text
3
```

This phase needs more rounds because bonding touches economics, governance, wallet flows, ledger conservation, dispute handling, and user-facing disclosure.

---

## Phase 4 Round 1 — bond DTOs and no-op accounting model

### Purpose

Model bonds without enabling live slashing.

### Primary work

```text
ron-proto:
  - bond intent DTOs
  - validator bond account DTOs
  - slash evidence DTOs
  - bond lifecycle DTOs

ron-ledger:
  - ledger-side bond accounting model
  - conservation tests
  - no silent bond mutation

svc-wallet:
  - explicit bond/unstake-style confirmation path if authorized
  - no silent spend
  - no one-click hidden bond lock

ron-accounting:
  - reports only
  - not bond truth
  - not slash truth
```

### Exit gate

```text
Bond math is deterministic and conservative.
No automatic slashing live yet.
No public staking market.
No liquidity.
```

---

## Phase 4 Round 2 — slashing/challenge simulation

### Purpose

Simulate consequences before live enforcement.

### Primary work

```text
- challenge windows
- slash evidence validation
- appeal/freeze states
- hold-like lifecycle for disputed bonds
- replayable dispute state
- no one-step irreversible slash without tested governance rules
```

### Exit gate

```text
Slashing simulation is deterministic.
Slashing simulation is replayable.
Challenge path is safe.
Dispute windows are explicit.
No live irreversible slash yet.
```

---

## Phase 4 Round 3 — controlled live bond enforcement

### Purpose

Turn simulated bond enforcement into controlled internal-only runtime behavior.

### Primary work

```text
svc-wallet:
  - explicit bond mutation route
  - explicit user/operator confirmation
  - no silent spend

ron-ledger:
  - conservation proof
  - deterministic bond state
  - slash/capture/release lifecycle

ron-policy:
  - policy-gated slash/capture/release

ron-audit if included:
  - audit events for bond lifecycle

CrabLink:
  - display-only bond status
  - no spend authority
```

### Exit gate

```text
Bond enforcement works internally.
No public staking market.
No liquidity.
No exchange-facing logic.
No public bridge.
```

### Completion label

When green, call this:

```text
Phase 4 complete
internal bonded validator model complete
```

Do not call it:

```text
public staking live
exchange-ready
liquidity-ready
```

---

# Phase 5 — broader decentralization / external anchoring options

## Goal

Choose an external posture only after internal proofs, replay, validators, bonds, and safety are strong.

## Ideal full rounds

```text
3
```

Only begin Phase 5 after Phase 4 is stable.

---

## Phase 5 Round 1 — anchor-only design and dry-run

### Purpose

Test compact external commitments without external settlement.

### Primary work

```text
- anchor payload DTOs
- checkpoint commitment export
- external anchor dry-run file/artifact
- anchor verification tests
- no balance mutation from anchor
- no external chain as ROC truth
```

### Exit gate

```text
External anchor can prove timestamp/commitment only.
External anchor cannot mutate ROC balances.
External anchor cannot replace wallet/ledger truth.
```

---

## Phase 5 Round 2 — DA/archive/challenge fallback

### Purpose

Prove data availability and challenge fallback before pruning.

### Primary work

```text
- DA bundle rules
- archive carrier proof
- missing-data challenge
- retention windows
- pruning blockers
- archive restore tests
```

### Exit gate

```text
A verifier can challenge missing data.
A verifier can restore needed artifacts from archive/DA.
No pruning unless archive/DA/challenge path is green.
```

---

## Phase 5 Round 3 — chosen external integration path

### Purpose

Choose one external path, not all of them.

Allowed choices later:

```text
- anchor-only
- external DA
- external L2/rollup
- hybrid
```

Still forbidden unless explicitly authorized:

```text
- public bridge
- exchange-facing logic
- liquidity
- ROX/Solana active runtime
- external settlement replacing wallet/ledger truth
```

### Exit gate

```text
External integration is evidence/anchoring only unless a later governance decision changes it.
Internal wallet/ledger truth remains canonical for ROC.
External path cannot silently alter balances, receipts, or unlocks.
```

### Completion label

When green, call this:

```text
Phase 5 complete
external anchoring/decentralization option complete
```

Only use stronger wording if a later governance decision explicitly authorizes it.

---

# Low-disk workflow for every crate-pair pass

Use this every time.

## Do

```text
1. Generate only the focused codebundles for the active pair.
2. Attach only the active focused codebundles plus required notes.
3. Patch only that pair.
4. Run focused tests first.
5. Run the crate pair's park script once.
6. Save crate notes.
7. cargo clean or remove the relevant target artifacts to recover disk.
8. Move to the next pair.
```

## Do not

```text
- Do not run both preflight and park unless debugging requires it.
- Do not rerun already-green earlier pairs unless they were patched.
- Do not run workspace-wide clippy unless it is the final phase audit.
- Do not upload full repo bundles unless cross-crate context is actually needed.
- Do not cargo clean before saving notes/codebundle if the next session needs exact context.
- Do not introduce Python helper scripts in crates unless explicitly approved.
```

## Preferred per-pair command pattern

Use this shape:

```bash
cargo fmt -p <crate-a> -p <crate-b>

cargo test -p <crate-a> <focused-test-or-flags>
cargo test -p <crate-b> <focused-test-or-flags>

<crate-a-or-pair>/scripts/dev-quickchain-park.sh
<crate-b-or-pair>/scripts/dev-quickchain-park.sh
```

If a park script already delegates to preflight, do not run preflight separately.

---

# Codebundle and notes policy

## Before a crate-pair session

Attach:

```text
- QUICKCHAIN_REVIEW_BUNDLE.MD
- QUICKCHAIN_BUILDPLAN.MD
- latest carryover notes
- focused codebundle for crate A
- focused codebundle for crate B
- CODEBUNDLE_RS.md only if cross-crate inspection is required
- CODEBUNDLE_TAURI_APP.md only when working CrabLink Tauri/client adapters
```

## After a crate-pair session

Save crate notes with:

```text
- what changed
- why it changed
- exact files touched
- tests added
- tests run
- terminal result summary
- remaining risks
- next pair
- forbidden-scope checks
- low-disk warnings
```

## After a full round

Save round notes with:

```text
- phase number
- round number
- crate pairs completed
- crate pairs skipped and why
- exact green gates
- failures and fixes
- final status label
- next round objective
```

---

# Status labels

Use precise labels.

## Safe labels

```text
QC-0A safety/preflight complete
Phase 1 Round 1 complete
QC-1A foundation complete
Phase 1 foundation complete
Phase 1 deterministic roots/proofs complete
Phase 2 replicated verification complete
Phase 3 passport-gated validator set complete
Phase 4 internal bonded validator model complete
Phase 5 external anchoring option complete
```

## Avoid unless truly complete

```text
QuickChain complete
chain live
public chain live
staking live
bridge live
external settlement live
exchange ready
validator economy live
```

---

# Current near-term checklist

At the time this build plan is created, the near-term path is:

```text
Finish Phase 1 Round 1:
  1. ron-proto + ron-ledger
       Status: complete for current pre-root slice

  2. svc-wallet + ron-accounting
       Status: focused QC-1A pair interlock tests green
       Next: park scripts and notes

  3. svc-rewarder + svc-storage
       Status: next pair

  4. svc-gateway + omnigate
       Status: after svc-rewarder + svc-storage

  5. svc-index + ron-policy
       Status: after svc-gateway + omnigate

  6. CrabLink Tauri + client adapters
       Status: after svc-index + ron-policy
```

Then:

```text
Start Phase 1 Round 2:
  ron-proto + ron-ledger heavy deterministic root/proof implementation
  downstream crate pairs confirmation-only unless integration requires patching
```

---

# One-page quick reference

```text
PHASE 0 — QC-0A safety/preflight
Rounds: 1
Result: boundaries safe before roots

PHASE 1 — single-authority deterministic roots/proofs
Rounds: 2
R1: QC-1A foundation/boundary sweep
R2: deterministic roots/proofs + downstream confirmation

PHASE 2 — small committee / replicated verification
Rounds: 2
R1: verifier artifact replication
R2: committee attestation semantics

PHASE 3 — passport-gated validator set
Rounds: 2
R1: validator identity/registry gating
R2: validator lifecycle hardening

PHASE 4 — bonded / higher-stakes validator model
Rounds: 3
R1: bond DTO/math model
R2: slashing/challenge simulation
R3: controlled internal bond enforcement

PHASE 5 — broader decentralization / external anchoring
Rounds: 3
R1: anchor-only dry-run
R2: DA/archive/challenge fallback
R3: chosen external path

TOTAL IDEAL FULL ROUNDS: 13
CURRENT PRACTICAL TARGET: finish Phase 1 Round 1, then do Phase 1 Round 2
```

---

# Final rule

When unsure whether to add a feature in a phase, ask:

```text
Does this phase require it?
Does this round require it?
Does this crate own it?
Can it be tested deterministically?
Can it be done without creating fake authority?
Can it be done without increasing future rewrite risk?
```

If the answer is no, defer it.


