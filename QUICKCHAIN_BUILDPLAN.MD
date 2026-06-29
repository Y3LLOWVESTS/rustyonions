
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


---

# Appendix — Internal ROC Tokenomics, Economics Config, and Future ROC/ROX Bridge Planning

This appendix records tokenomics and future bridge planning without changing the active QuickChain phase order.

The active buildplan remains authoritative:

```text
Phase 5 Round 1 — anchor-only design and dry-run
Phase 5 Round 2 — DA/archive/challenge fallback
Phase 5 Round 3 — chosen external integration path
```

This appendix does not authorize ROX active runtime, Solana active runtime, external settlement, public bridge, liquidity, exchange-facing logic, or public staking.

Those remain future/deferred unless explicitly authorized by a later governance decision, audit gate, and dedicated implementation phase.

---

## A. Internal ROC tokenomics doctrine

ROC is internal first.

The internal ROC economy must remain bounded, deterministic, configurable, and auditable.

The core rule:

```text
raw engagement must not directly mint or allocate protocol ROC
```

Raw activity such as views, visits, watch seconds, clicks, comments, shares, playback, livestream presence, and node traffic may create metering or proof-eligible events, but those events are not themselves payouts.

Correct value path:

```text
event classification
→ ron-accounting snapshot/report
→ svc-rewarder capped payout planning
→ ron-policy validation/gating
→ svc-wallet approved mutation
→ ron-ledger durable economic truth
```

Event classes remain:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

Rules:

```text
economic_receipt:
  may represent wallet/ledger-accepted economic truth.

metering:
  records usage/attention/service facts only.

proof_eligible:
  may become eligible for reward planning only after verification, caps, and policy gates.

ad_budgeted:
  paid from explicit ad/campaign budget, not from unlimited protocol emission.

analytics_only:
  must never become protocol ROC payout material.
```

---

## B. Single editable economics config

All mutable ROC economics values must live in one canonical TOML config.

Canonical file:

```text
configs/roc-economics.toml
```

Optional local/dev override files may exist, but they must not replace the canonical config unless explicitly intended:

```text
configs/roc-economics.dev.toml
configs/roc-economics.local.toml
```

Hard invariant:

```text
No hard-coded payout amounts in business logic.
No hard-coded reward rates scattered across crates.
No hard-coded burn rates scattered across crates.
No hard-coded split percentages scattered across crates.
No hard-coded tokenomics constants in UI/client code.
```

The TOML config is the single reference point for adjustable values such as:

```text
site visit weights
post view weights
article read weights
comment weights
share/referral weights
video playback weights
video minute weights
song play weights
livestream viewer-minute weights
livestream participant split defaults
reuse/mashup split defaults
source clip pool defaults
curator/referrer split defaults
node delivery reward weights
node availability reward weights
rare/archival content premiums
moderation reward weights
abuse-report reward weights
appeal-review reward weights
ad campaign burn rates
ad campaign viewer/site/node/moderation splits
protocol emission caps
epoch reward pool categories
reward point conversion parameters
quality multipliers
reputation multipliers
scarcity multipliers
anti-sybil multipliers
rate caps
max spend limits
hold multipliers
rounding mode
remainder sink
future ROC/ROX conversion fee placeholders
future bridge burn-rate placeholders
```

All values must use deterministic integer-safe representation:

```text
money amounts: integer minor-unit strings or integer minor units
percentages/splits: basis points
10000 bps = 100%
no floats
no implicit rounding
no hidden remainder
```

Remainders must go to an explicit configured sink:

```text
remainder_sink = "treasury" | "burn" | "stability_buffer" | another validated account alias
```

---

## C. Config ownership and authority

`configs/roc-economics.toml` is policy/config input only.

It is not:

```text
ledger truth
wallet authority
receipt proof
balance proof
paid unlock proof
finality proof
settlement proof
bridge proof
root proof
```

Ownership model:

```text
ron-policy:
  parses and validates the economics TOML.
  rejects unknown fields.
  rejects invalid split totals.
  rejects unsafe authority fields.
  evaluates policy only.
  does not mutate wallet or ledger.

svc-rewarder:
  uses policy-approved values to plan capped payouts.
  does not mutate ledger.
  does not issue receipts.
  does not create balance truth.

svc-wallet:
  executes explicit approved mutations.
  remains the mutation front-door.
  requires explicit confirmation where spend/hold/burn is involved.

ron-ledger:
  records durable economic truth.
  proves conservation.
  rejects invalid mutations.

ron-accounting:
  records snapshots/reports.
  does not become balance truth.

CrabLink:
  displays backend-derived values only.
  does not own payout truth, balance truth, or spend authority.
```

---

## D. Tokenomics categories to model

The economics config and tokenomics guide should support the following broad categories.

### Content and attention

```text
site visit
post view
article read
image view
video playback
video minute
song play
podcast play
livestream viewer-minute
stream replay
```

Passive attention should be low-value and heavily capped.

### Creation

```text
post creation
article creation
image publishing
music publishing
video publishing
podcast publishing
site publishing
livestream hosting
template/theme creation
developer/facet creation
```

Creation rewards should be higher than passive viewing but still gated by quality, reputation, and anti-farming rules.

### Curation and sharing

```text
share
quote share
curated repost
playlist placement
collection inclusion
site embed
reference graph inclusion
```

Sharing should reward distribution without making repost farming more profitable than original creation.

Default posture:

```text
original creator receives majority
sharer/curator receives minority
node/service receives service share
moderation/trust may receive reserve share
```

### Reuse and mashups

If a manifest says reuse is allowed, the economics config should support source splits.

Example categories:

```text
reuse_ok video clip
commentary/mashup video
source clip pool
transformative commentary
simple aggregation
music sample
stream clip
embedded source asset
```

Default posture:

```text
transformative creator receives meaningful share
original source creators receive required pool share
manifest-defined splits override generic defaults where valid
```

Unauthorized reposting should route value back to the original creator and may trigger penalties, bond loss, moderation review, or reputation loss.

### Music and rights splits

Music and audio manifests should support basis-point splits for:

```text
artist
producer
featured artist
publisher/collective
curator/playlist
node/service
moderation/trust reserve
burn/reserve if configured
```

No full reward for instant skips.

### Livestreams

Livestream economics should support:

```text
host share
active speaker/performer pool
guest pool
moderator pool
node/relay pool
protocol burn/reserve
```

Splits should be explicit before paid livestreams begin.

Possible inputs:

```text
screen time
audio time
active speaker time
manual pre-stream split
role weight
viewer-minute
paid ticket/subscription receipt
```

### Nodes

Node rewards should split service into at least:

```text
verified delivery
availability
rare/archival availability
paid-content service share
ad-budgeted delivery share
```

Node rewards must be audited and capped.

Node traffic must not become unlimited ROC mining.

### Moderation

Moderation rewards should be tied to resolved, reviewable outcomes:

```text
confirmed abuse report
spam cluster confirmed
appeal reviewed
false positive corrected
dangerous content removed
repeat abuser identified
policy dispute resolved
community queue triaged
```

Moderation rewards should survive appeal windows before final payout where appropriate.

Bad-faith moderation should reduce reputation or trigger penalties.

### Ads and burns

Ad campaigns are the primary planned burn sink.

Default doctrine:

```text
most ad campaign ROC is burned
a smaller share may reward viewers/engagement
a smaller share may reward creator/site placement
a smaller share may reward node/service delivery
a smaller share may fund moderation/abuse reserve
```

Ad-funded rewards must be labeled:

```text
ad_budgeted
```

not normal protocol emission.

---

## E. Internal ROC supply and burn doctrine

ROC emissions should be bounded by epoch/category pools.

Prefer:

```text
capped epoch reward pools
category budgets
reward points converted at epoch close
burn-adjusted supply targets
reputation/quality/scarcity multipliers
anti-farming caps
explicit remainder sink
```

Avoid:

```text
unlimited fixed ROC per view
unlimited fixed ROC per click
unlimited fixed ROC per watch second
unbounded passive mining
raw engagement direct payouts
```

Ad burns, dispute penalties, spam resistance, premium namespace pressure, and future conversion fees may act as ROC sinks.

Normal creator payments should remain creator-first and should not be over-burned.

---

## F. Future ROC/ROX bridge planning

This section records the long-term bridge concept only.

It does not authorize implementation during Phase 5 Round 1.

It does not authorize active ROX runtime.

It does not authorize active Solana runtime.

It does not authorize public bridge runtime.

It does not authorize external settlement replacing wallet/ledger truth.

The future bridge concept is:

```text
ROC ↔ ROX bidirectional bridge
```

One possible future implementation path:

```text
Solana Anchor-based program for ROX
```

In this context, “Solana Anchor” means the Solana smart contract/program framework.

This is distinct from QuickChain Phase 5 “anchor-only” commitments, where “anchor” means a compact external commitment/timestamp/proof reference.

### Future ROC → ROX direction

Possible future flow:

```text
user requests ROC → ROX bridge quote
backend displays fees, burn rate, challenge window, and risks
user explicitly confirms
svc-wallet burns or locks internal ROC through normal wallet path
ron-ledger records durable ROC burn/lock receipt
QuickChain checkpoint includes the burn/lock event
proof/challenge/finality window completes
bridge verifier/relayer submits approved proof to Solana Anchor program
Solana program mints or releases ROX to the user
```

Rules:

```text
no silent burn
no silent mint
no UI-created bridge truth
no cache-created bridge truth
no external mint without internal proof
no bridge before audit and governance approval
```

### Future ROX → ROC direction

Possible future flow:

```text
user requests ROX → ROC bridge quote
backend displays fees, burn rate, challenge window, and risks
user explicitly confirms
Solana Anchor program burns or locks ROX
external proof/event is observed
proof/challenge/finality window completes
svc-wallet mints or unlocks internal ROC through normal wallet path
ron-ledger records durable ROC mint/unlock receipt
```

Rules:

```text
no ROC mint/unlock from unverified external event
no external chain event directly mutates ron-ledger
no gateway/omnigate/index/policy/client bridge authority
no fake bridge receipt
no fake external finality
no exchange-facing assumptions
```

### Future bridge burn/fee policy

All future bridge fees and burn rates must come from validated config, not code constants.

Potential TOML-controlled fields:

```text
roc_to_rox_burn_bps
rox_to_roc_burn_bps
bridge_fee_bps
bridge_min_fee_minor
bridge_max_fee_minor
bridge_challenge_window_epochs
bridge_daily_limit_minor
bridge_per_user_limit_minor
bridge_pause_enabled
bridge_emergency_halt_enabled
bridge_remainder_sink
```

These fields should remain inert/deferred until a future bridge phase explicitly authorizes implementation.

---

## G. Future bridge prerequisites

A bidirectional ROC/ROX bridge may only be considered after:

```text
Phase 5 Round 1 anchor-only dry-run is complete.
Phase 5 Round 2 DA/archive/challenge fallback is complete.
Phase 5 Round 3 chosen external posture is complete.
Internal ROC value plane is stable.
Wallet/ledger replay and conservation are proven.
Bridge threat model is written.
Bridge challenge windows are defined.
Bridge halt/pause/governance controls are defined.
Bridge accounting and audit events are defined.
Bridge smart contract/program is independently audited.
Solana/ROX risk model is accepted.
Public/legal/product language is reviewed.
```

The bridge must never bypass:

```text
svc-wallet
ron-ledger
explicit user confirmation
challenge/finality windows
audit trail
policy gates
emergency halt controls
```

---

## H. Active-scope warning

This appendix is a planning guardrail.

It must not be used as permission to add:

```text
ROX active runtime
Solana active runtime
Solana Anchor program code
bridge mint/burn code
external settlement
exchange-facing logic
liquidity
staking
public validator economy
gateway direct ledger mutation
omnigate direct ledger mutation
index settlement truth
policy settlement truth
CrabLink bridge authority
client-side bridge mint/burn
cache-only bridge proof
fake receipts
fake balances
fake finality
silent spend
```

Until a later phase explicitly authorizes bridge work, the correct status remains:

```text
Future/deferred bridge design only.
Internal ROC first.
QuickChain Phase 5 proceeds as written.
```

---

## I. Relationship to tokenomics guide

This buildplan appendix is intentionally short enough to keep the phase plan readable.

The detailed economics model should live in a separate guide:

```text
docs/tokenomics/INTERNAL_ROC_MODEL.md
```

That guide should define:

```text
reward pools
reward points
event classes
anti-farming rules
manifest split defaults
creator/reuse/mashup/music/livestream split doctrine
node rewards
moderation rewards
ad burns
burn sinks
emission schedule
ROC/ROX future bridge economics
```

The editable numerical source of truth remains:

```text
configs/roc-economics.toml
```

The buildplan defines phase order.

The tokenomics guide defines economic doctrine.

The TOML config defines mutable economics values.

The code validates and executes only through the correct authority path.

---



````markdown id="x1q48o"
---

# Appendix — Internal ROC Tokenomics Reference

The detailed internal ROC economics doctrine lives in:

```text
docs/tokenomics/INTERNAL_ROC_MODEL.md
````

This appendix mirrors the non-negotiable rules so Phase 5 work does not drift.

## Canonical economics source

All mutable ROC economics values must live in:

```text
configs/roc-economics.toml
```

This includes:

```text
payout amounts
reward weights
split defaults
burn rates
epoch pool category percentages
reward point conversion parameters
quality/reputation/scarcity multipliers
anti-farming caps
max spend limits
hold multipliers
rounding mode
remainder sink
future bridge fee/burn placeholders
```

Hard invariant:

```text
No hard-coded payout amounts in business logic.
No scattered tokenomics constants across crates.
No UI/client-owned reward, balance, receipt, or payout truth.
```

All monetary values must use integer minor units or integer minor-unit strings.

All percentages and splits must use basis points.

```text
10000 bps = 100%
```

Floating point economics are forbidden.

Remainders must go to an explicit validated remainder sink.

## Internal ROC value path

Raw engagement must not directly mint or allocate protocol ROC.

Correct value path:

```text
event classification
→ ron-accounting snapshot/report
→ svc-rewarder capped payout planning
→ ron-policy validation/gating
→ svc-wallet approved mutation
→ ron-ledger durable economic truth
```

Event classes remain:

```text
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

Rules:

```text
economic_receipt:
  wallet/ledger-accepted economic truth only

metering:
  usage/service facts only; not payout truth

proof_eligible:
  may feed reward planning only after verification, caps, and policy

ad_budgeted:
  funded from explicit ad/campaign budget, not unlimited protocol emission

analytics_only:
  never becomes protocol ROC payout material
```

## Authority boundaries

```text
ron-proto:
  DTOs only

ron-policy:
  validates/evaluates config and declarative policy only

ron-accounting:
  snapshots/reports only; not balance truth

svc-rewarder:
  payout planning only; no ledger mutation

svc-wallet:
  mutation front-door

ron-ledger:
  durable economic truth

svc-storage:
  bytes/artifacts only

svc-index:
  lookup/pointer only

svc-gateway:
  public boundary only

omnigate:
  hydration/access composition only

CrabLink:
  display/user intent only
```

Forbidden:

```text
rewarder direct ledger mutation
accounting direct ledger mutation
policy direct ledger mutation
gateway direct ledger mutation
omnigate direct ledger mutation
index as payment truth
storage as payment truth
cache-only paid unlock
client-side payout truth
fake receipts
fake balances
fake finality
silent spend
```

## Phase 5 relationship

This tokenomics reference does not change QuickChain phase order.

Phase 5 remains:

```text
Phase 5 Round 1 — anchor-only design and dry-run
Phase 5 Round 2 — DA/archive/challenge fallback
Phase 5 Round 3 — chosen external integration path
```

For Phase 5 Round 1, economics artifacts may become deterministic commitment inputs later:

```text
classified event snapshots
epoch reward pool plans
reward point summaries
manifest split structures
ad-budgeted burn summaries
node service summaries
moderation outcome summaries
payout planning manifests
remainder sink records
```

But those artifacts remain internal and non-authoritative until wallet/ledger execution.

QuickChain may later prove deterministic economics artifacts.

QuickChain must not replace wallet/ledger truth.

## Future ROC/ROX bridge boundary

The future ROC/ROX bridge remains deferred.

Possible future direction:

```text
ROC → ROX
ROX → ROC
```

Possible future implementation path:

```text
Solana Anchor-based program for ROX
```

In this context, “Solana Anchor” means the Solana smart contract/program framework.

This is distinct from QuickChain Phase 5 “anchor-only” commitments, where “anchor” means compact external commitment/timestamp/proof evidence.

A bidirectional ROC/ROX bridge may only be considered after:

```text
Phase 5 Round 1 anchor-only dry-run is complete.
Phase 5 Round 2 DA/archive/challenge fallback is complete.
Phase 5 Round 3 chosen external posture is complete.
Internal ROC value plane is stable.
Wallet/ledger replay and conservation are proven.
Bridge threat model is written.
Bridge challenge windows are defined.
Bridge halt/pause/governance controls are defined.
Bridge accounting and audit events are defined.
Bridge smart contract/program is independently audited.
Solana/ROX risk model is accepted.
Public/legal/product language is reviewed.
```

Until explicitly authorized by a later bridge phase, this remains the active status:

```text
Future/deferred bridge design only.
Internal ROC first.
QuickChain Phase 5 proceeds as written.
No ROX active runtime.
No Solana active runtime.
No Solana Anchor program code.
No bridge mint/burn code.
No external settlement.
No exchange-facing logic.
No liquidity.
No staking.
No public validator economy.
```

---

````

For the terminal workflow, I would do this:

```bash id="s8m5xg"
cd /Users/mymac/Desktop/RustyOnions

mkdir -p docs/tokenomics

# 1) Paste the full INTERNAL_ROC_MODEL.md guide into:
# docs/tokenomics/INTERNAL_ROC_MODEL.md

# 2) Append the compact appendix above to:
# QUICKCHAIN_BUILDPLAN.MD

# 3) Append the same compact appendix to the QuickChain blueprint source.
# If QUICKCHAIN.MD exists, append there.
# If QUICKCHAIN_REVIEW_BUNDLE.MD is the working blueprint file in your repo, append there too.
````

Then run the lightweight checks:

```bash id="xh9cte"
grep -n "docs/tokenomics/INTERNAL_ROC_MODEL.md" QUICKCHAIN_BUILDPLAN.MD
grep -n "configs/roc-economics.toml" QUICKCHAIN_BUILDPLAN.MD
grep -n "raw engagement must not directly mint" QUICKCHAIN_BUILDPLAN.MD
grep -n "Future/deferred bridge design only" QUICKCHAIN_BUILDPLAN.MD

grep -n "docs/tokenomics/INTERNAL_ROC_MODEL.md" QUICKCHAIN_REVIEW_BUNDLE.MD
grep -n "configs/roc-economics.toml" QUICKCHAIN_REVIEW_BUNDLE.MD
grep -n "raw engagement must not directly mint" QUICKCHAIN_REVIEW_BUNDLE.MD
grep -n "Future/deferred bridge design only" QUICKCHAIN_REVIEW_BUNDLE.MD
```

No Cargo changes are needed. This is docs-only and keeps the active Phase 5 path clean.
