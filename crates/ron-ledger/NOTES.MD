### RON-LEDGER NOTES - APRIL 26 2026 - 17:20 CST


## ron-ledger — current status

`ron-ledger` is now a **working library-first ledger core**, not just a scaffold. That matters because the TODO and crate layout were intentionally designed as a **library, not a service**: the crate should expose DTOs, types, config, error taxonomy, engine logic, storage seam, replay/checkpoint logic, and observer hooks, while a future service wrapper is responsible for HTTP, metrics endpoints, readiness, and ops surfaces. 

At the architecture level, the crate’s intended role is still exactly what the IDB says: RustyOnions’ **immutable, append-only economic truth**, with total order, conservation, deterministic accumulator roots, idempotent ingestion, crash-recovery monotonicity, amnesia compatibility, and strict role separation from wallet/rewarder/accounting. 

## What is finished now

Based on your latest successful run, the crate currently has a clean basic engineering floor:

* `cargo fmt` passes
* `cargo clippy -p ron-ledger --all-targets -- -D warnings` passes
* `cargo test -p ron-ledger --all-targets` passes
* the Criterion micro bench smoke runs successfully

That means the crate is currently **build-clean, lint-clean, test-clean, and bench-smoke-clean** in your local repo.

Functionally, the following parts are now real enough to count as implemented:

* The **library-first file structure** exists and is coherent: `api.rs`, `types.rs`, `config.rs`, `error.rs`, `engine/{storage,ledger,accumulator,checkpoint,replay,observer}.rs`, tests, bench, fuzz target, example, vectors, and CI script scaffolds. That matches the intended ron-ledger shape from the TODO.
* The crate now enforces the **core invariants we targeted first**: append-only behavior, conservation checks, deterministic roots, idempotent ingestion, stable reject taxonomy, and crash/replay restoration. Those are exactly the invariants the IDB treats as non-negotiable. 
* The crate is aligned with **Pillar 12** in the project map: `ron-ledger` as immutable truth, with wallet/accounting/rewarder kept separate.

## What the passing tests have actually proved

The current passing test set proves more than “it compiles.”

`idempotency_prop.rs` proves that repeated ingestion keyed the same way behaves like a **no-op** instead of mutating ledger state again. That is direct evidence for the IDB’s idempotent ingestion invariant.

`interop_vectors.rs` proves three different things:

* the happy-path ingest shape can parse and commit,
* malformed nested DTO fields are rejected,
* conservation is checked rather than assumed.
  That lines up with the documented goals for interop vectors and DTO strictness.

`reject_taxonomy.rs` proves the internal error mapping is stable and machine-countable for the current buckets: `Invalid`, `TooLarge`, and `Conflict`. That is exactly what the TODO says this suite is supposed to guard.

`replay_recovery.rs` is the biggest milestone. It proves file-backed replay restores both **root** and **balance state**, which is the first real proof that crash-recovery monotonicity and deterministic replay are working in practice rather than only on paper. The TODO explicitly says replay/checkpoint must yield the same `new_root` across runs, and that now appears satisfied by the current implementation.

`public_api_semver.rs` proves there is at least a basic guard on public surface drift, though it is still only a smoke-level protection compared with the full public-api and semver-checks workflow the docs expect.

The `micro.rs` bench smoke proves the ingest path can execute under Criterion without blowing up, but it does **not** yet prove the documented SLOs or regression gates. The docs expect much more than a smoke bench for performance signoff.

## What remains inside the crate

This is the most important section, because the crate is working now, but it is **not done**.

First, the crate still needs **deeper test coverage inside `src/`**. The docs expect unit coverage for accumulator hashing, config validation, serde strictness, and redaction helpers, while the current run still shows `src/lib.rs` itself has zero unit tests. 

Second, the crate still lacks the **broader validation envelope** promised by `TESTS.md`:

* more property tests,
* actual fuzz execution,
* loom concurrency models,
* chaos/smoke scripts,
* coverage thresholds,
* perf regression gating.
  The docs lay out Bronze, Silver, and Gold acceptance gates, and the crate is not yet at Silver or Gold by those standards.

Third, the **observer layer is still just hooks**, which is correct for a library crate, but it means the full observability contract in the docs is not yet realized inside a service wrapper. The TODO is explicit that `observer.rs` should emit typed callbacks only and leave Prometheus wiring to a service.

Fourth, the crate is still missing proof around **single-writer and bounded-queue concurrency semantics** at the test level. The docs expect loom/backpressure/shutdown validation, and that has not been demonstrated by the current passing suite. 

Fifth, the crate has not yet demonstrated stronger **storage corruption / disk pressure / safe-mode** behavior. The docs and runbook expect failure handling around WAL/fsync issues, seq gaps, and safe-mode behavior, but the current test set does not appear to exercise those cases yet.

Sixth, the **fuzz target exists by scaffold**, but existence is not the same thing as being run and enforced. The TODO and tests doc both expect replay/parse fuzzing to be part of the quality contract.

## What remains outside the crate

This is the key distinction:

The crate is now a decent **library core**, but the docs also describe a much larger **service-class surface** for `ron-ledger`, including:

* HTTP/UDS endpoints,
* `/healthz`, `/readyz`, `/metrics`, `/version`,
* OTEL + exemplars,
* readiness degradation rules,
* Grafana dashboards,
* Prometheus alerts,
* operator runbooks,
* TLS/mTLS posture.

That service-class surface is **not** what the TODO said to build inside this crate. The TODO is explicit that the crate itself should stay library-first and that service wrappers consume it. So those docs are still important, but they should be read as **future wrapper obligations**, not missing work inside `ron-ledger` proper. 

In other words:

* **inside the crate**: build and harden deterministic ledger logic
* **outside the crate**: later expose service HTTP/readiness/metrics/ops on top of it

That tension is normal, and the crate is currently on the right side of it.

## Honest completion estimate

I would record the current status like this:

**ron-ledger core library milestone:** about **85% complete**
Because:

* core structure exists,
* core invariants are implemented,
* replay and file-backed recovery now work,
* taxonomy and DTO strictness work,
* local fmt/clippy/tests are green.

**ron-ledger docs-level / production-grade completion:** about **60–65% complete**
Because the docs still expect:

* richer unit/property/fuzz/loom coverage,
* stronger failure-mode testing,
* perf baselines and regression gates,
* semver/public-api workflows enforced in CI,
* service-wrapper observability/readiness/ops surfaces.

So the right mental model is:

**The deterministic ledger core is mostly real now.**
**The full documented operational envelope is not finished yet.**

## Recommended “done enough to move on” line

I would not block `svc-wallet` on making `ron-ledger` perfect. I would block it only on a few remaining high-value items:

* add a few direct unit tests in `src/` for config validation, accumulator determinism, and serialization strictness
* run at least a smoke fuzz pass for `replay_parse`
* add one or two stronger replay/corruption edge tests
* optionally wire a real `cargo public-api` / semver-checks CI gate
* freeze the current carry-over notes with the current local green run

Once that is done, the crate is solid enough to stop being the bottleneck and let `svc-wallet` become the next high-impact step.

## Carry-over note you can save

`ron-ledger` is now a working, library-first deterministic ledger core aligned to the ron-ledger TODO shape. It compiles cleanly, passes clippy under `-D warnings`, passes the current integration/property-style tests, and successfully restores file-backed state through replay. The implemented core now demonstrates append-only truth, conservation checks, idempotent ingestion, strict DTO rejection, stable reject taxonomy, and deterministic replay/root restoration. Remaining work is mostly hardening and breadth: more unit coverage inside `src/`, actual fuzz/loom/chaos/perf gate execution, stronger corruption/disk-pressure scenarios, and CI enforcement for semver/public-api/perf. Treat the crate as roughly 85% complete as a ledger core and about 60–65% complete against the full docs-level production envelope. The next crate can be `svc-wallet`, but ron-ledger should still get a final small hardening pass before we stop touching it.


### END NOTE - APRIL 26 2026 - 17:20 CST

### BEGIN NOTE - APRIL 27 2026 - 19:55 CST

# 1. NOTE — `ron-ledger`

## Current Status

`ron-ledger` remains the durable economic truth substrate for the ROC value plane. We did not heavily rewrite `ron-ledger` in this final `svc-storage` session, but every higher-level WEB3 crate now depends on the correctness of the ledger model through `svc-wallet`.

The important architectural decision remains:

```text
ron-ledger is the source of committed economic truth.
svc-wallet is the mutation front-door.
svc-rewarder plans payouts but does not mutate the ledger directly.
svc-storage charges through wallet escrow rather than writing ledger entries itself.
```

This separation is critical. It prevents the storage layer, rewarder layer, or accounting layer from accidentally becoming alternate sources of balance truth.

## What `ron-ledger` Provides to the Current WEB3 Stack

`ron-ledger` is currently used as the lower-level accounting/commit engine behind `svc-wallet`. Through `svc-wallet`, it now supports the core ROC operations needed by the internal value plane:

```text
issue      explicit supply increase
transfer   balanced debit/credit movement
burn       explicit supply decrease
hold       payer → escrow reserve movement
capture    escrow → service/provider settlement movement
release    escrow → payer refund movement
balance    ledger-backed balance reads
```

The most important new dependency on `ron-ledger` is the escrow lifecycle. `svc-wallet` now wraps ledger operations into:

```text
hold    = reserve funds from user into escrow
capture = pay the service/provider from escrow
release = return unused escrow to the user
```

This gives paid storage/pinning a safe value flow without introducing direct ledger writes inside `svc-storage`.

## What Was Proven Indirectly Through Higher-Level Tests

Although this session focused more on `svc-wallet` and `svc-storage`, the ledger path was exercised through wallet tests and integration loops. The full sweep shows `svc-wallet` passing its ledger, escrow, non-negativity, conservation, double-spend, nonce, idempotency, and paid-storage smoke tests. 

The following ledger-backed invariants are now being exercised through `svc-wallet`:

```text
1. Issue creates balance for the recipient.
2. Transfer conserves total value between participants.
3. Burn explicitly reduces supply.
4. Failed transfer/burn/capture does not create partial credits.
5. Nonce replay is rejected.
6. Idempotent replay returns the same receipt instead of duplicating state.
7. Hold/capture/release preserves total value across payer, escrow, and service accounts.
8. Capture above held amount rejects without crediting the payee.
```

## Why This Matters for RON Beta

RON’s internal ROC economy now has a ledger foundation strong enough to support:

```text
provider rewards
user balances
paid storage
paid pinning
escrow settlement
future app/service billing
dashboard accounting
```

The ledger is not just a theoretical crate anymore. It is being exercised by the live wallet mutation boundary and by the paid-storage flow.

## What Remains for `ron-ledger`

### 1. Durability / Persistence Hardening

The current tests heavily exercise the in-memory/dev path through wallet. Before beta production, we need to decide exactly how ledger persistence is configured:

```text
amnesia/dev mode       in-memory, rebuildable, test/demo safe
durable beta mode      persistent storage, crash replay, audit roots
production mode        backup/restore, corruption checks, root anchoring
```

Remaining work:

```text
- confirm durable storage backend expectations
- add crash/restart replay tests if not already complete
- document ledger persistence mode for Micronode vs Macronode
- define whether beta ledger is local-only or cluster-replicated
```

### 2. Formal Receipt/Audit Surface

`svc-wallet` currently emits receipts derived from committed ledger operations. `ron-ledger` should eventually expose or support a stronger audit model:

```text
ledger sequence ranges
ledger root after commit
receipt hash binding
append-only audit trail
optional export/snapshot format
```

Remaining work:

```text
- ensure receipts bind to ledger root/sequence in a stable way
- document receipt verification rules
- define how external auditors or dashboard panels verify ledger continuity
```

### 3. Wider Integration Tests

Current ledger proof is mostly indirect through wallet. Eventually add direct multi-crate tests that verify:

```text
rewarder issue batch → wallet → ledger
paid storage hold/capture/release → wallet → ledger
ledger balances match dashboard/API views
ledger replay preserves balances and receipt history
```

### 4. Supply Policy Documentation

The ledger supports explicit supply exceptions like issue and burn, but the system needs governance documentation for:

```text
who can issue ROC
when reward issuance is allowed
how burns are authorized
how supply is reported
how accidental issuance is prevented
```

### 5. Future ROX Boundary

ROX/Solana remains deferred. `ron-ledger` should continue to treat ROC as internal truth. Future bridge/export work should not pollute the current ledger model with external chain assumptions.

## Recommended Next Steps for `ron-ledger`

```text
1. Keep ron-ledger stable while hardening wallet/storage/accounting integration.
2. Add/confirm durable replay tests.
3. Add a ledger receipt verification note.
4. Add a beta-mode persistence decision.
5. Avoid adding ROX/Solana logic until ROC beta is proven internally.
```

## Completion Estimate

```text
ron-ledger for internal ROC beta: ~85–90%
ron-ledger for production-grade audited economy: ~70–78%
```

The crate appears functionally strong for the current internal economy, but production durability, audit export, and governance documentation remain the main unfinished pieces.

---

### END NOTE - APRIL 27 2026 - 19:55 CST


### BEGIN NOTE - JUNE 12 2026 - 21:40 CST


# RustyOnions / QuickChain Carryover Notes — `ron-ledger`

## Status at pause

`ron-ledger` is now effectively complete for the current allowed QuickChain Phase 0 / pre-root / pre-consensus slice.

Current status:

```text
ron-ledger QuickChain preflight slice: 98–99% complete
full ron-ledger QuickChain responsibility: ~64–67%
full QuickChain vision: ~31–34%
```

The crate should now be treated as green for the deterministic pre-root execution and projection layer. Further `ron-ledger` code changes should be avoided unless a real bug, missing invariant, contradiction with the QuickChain blueprint, or cross-crate integration need is discovered.

The important milestone is not only that tests pass. The important milestone is that `ron-ledger` now has a gated, deterministic, replayable, pre-root QuickChain execution model that can:

```text
validate operation intent through ron-proto DTOs
execute checked ROC balance transitions
execute hold lifecycle transitions
bind operation identity and idempotency identity
assign ledger-owned account sequences
assign primitive ledger sequence ranges
return exact retry evidence
rebuild from accepted history
verify accepted replay boundaries
snapshot deterministic state
project pre-root payload DTOs
preserve explicit context boundaries
avoid roots, validators, settlement, persistence formats, and external-chain authority
```

This is the right stopping point for `ron-ledger` until canonical bytes, golden vectors, domain separators, ordering rules, and later QuickChain phases are explicitly unlocked.

---

# 1. Final confirmed gate state

The final command batch run was:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-ledger -- --check

crates/ron-ledger/scripts/dev-quickcheck.sh

cargo check --workspace

bash scripts/make_crate_codex.sh -c ron-ledger
```

Confirmed final state:

```text
cargo fmt/check for ron-ledger passed
ron-ledger no-feature clippy/test path passed
ron-ledger quickchain-preflight clippy/test path passed
public_api_semver passed in no-feature mode
public_api_semver passed with quickchain-preflight enabled
full QuickChain test suite passed
legacy ron-ledger tests passed
bench smoke passed
example compile smoke passed
cargo check --workspace completed
fresh ron-ledger CODEBUNDLE.md generated
```

Workspace warnings remain outside `ron-ledger`:

```text
micronode: unused import UpstreamSpec
ryker: dead field max_msg_bytes
omnigate: unused MAX_ACCOUNT_BYTES
```

These are not `ron-ledger` blockers.

---

# 2. Master QuickChain doctrine that governed this work

The master reference remains:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
```

That file must be read first before any future QuickChain patch, design, or test plan.

The core doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

The current active mission remains internal ROC first.

Allowed right now:

```text
strict DTOs
validation helpers
schema constants
readiness docs
threat-model docs
canonicalization notes
golden-vector planning
serde drift tests
unknown-field rejection tests
invalid b3 rejection tests
integer-money rejection tests
deterministic pre-root ledger execution/replay/projection scaffolding
```

Forbidden right now:

```text
roots
checkpoints as QuickChain roots
validators
consensus
fork choice
pruning
settlement
anchors
bridges
staking
liquidity
exchange-facing logic
ROX
Solana
external settlement
CrabLink chain authority
gateway ledger mutation
omnigate direct ledger mutation
accounting ledger mutation
rewarder ledger mutation
raw engagement protocol payouts
fake balances
fake receipts
fake proofs
DB-order roots
wall-clock roots
unknown consensus fields
root-producing code without canonical bytes and golden vectors
```

`ron-ledger` work stayed inside the safe boundary. It produced deterministic execution and projection surfaces, but it did not produce QuickChain roots, checkpoint hashes, validators, proofs, settlement behavior, or external-chain integration.

---

# 3. Crate-level boundary of `ron-ledger`

`ron-ledger` is the economic truth crate.

It may own:

```text
checked ROC arithmetic
deterministic balance state
deterministic hold state
accepted operation evidence
ledger-owned account sequence assignment
primitive ledger sequence assignment
operation replay identity
idempotency replay identity
accepted-history replay
deterministic snapshots
pre-root payload projection adapters
```

It must not own:

```text
wallet authorization policy
private keys
client authority
gateway mutation authority
omnigate mutation authority
rewarder mutation authority
accounting-as-balance truth
CrabLink display-generated balances
external settlement
bridges
validators
public chain state
staking
liquidity
exchange-facing flows
```

Important mental model:

```text
svc-wallet is the mutation front-door
ron-ledger is the economic truth engine
ron-accounting is snapshot/reporting, not balance truth
svc-rewarder plans payouts, but does not mutate balances
CrabLink shows user intent and backend truth, but does not create truth
QuickChain is future settlement infrastructure, not current runtime authority
```

---

# 4. Feature gating and public crate shape

QuickChain work is gated behind:

```toml
quickchain-preflight = ["dep:ron-proto"]
```

The crate root exposes QuickChain only when the feature is enabled:

```rust
#[cfg(feature = "quickchain-preflight")]
pub mod quickchain;
```

This is correct because QuickChain is not live runtime infrastructure yet. The preflight module should remain opt-in until later phases explicitly approve deeper integration.

The public QuickChain module currently exports the following categories:

```text
accepted replay types
balance state
replay errors
execution errors
atomic execution state and outcomes
operation/receipt hash-payload projection adapters
hold errors
hold state
hold transition types
leaf projection adapters
replay index
state snapshot types
balance transition types
committed operation records
submission decision types
```

The public API smoke test was updated to cover the important feature-gated preflight public surface. This protects the crate from accidentally dropping the newly added QuickChain exports.

---

# 5. QuickChain module source map

Current `ron-ledger::quickchain` modules:

```text
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/src/quickchain/accepted_replay.rs
crates/ron-ledger/src/quickchain/balance_state.rs
crates/ron-ledger/src/quickchain/error.rs
crates/ron-ledger/src/quickchain/execution_error.rs
crates/ron-ledger/src/quickchain/execution_state.rs
crates/ron-ledger/src/quickchain/hash_payload_projection.rs
crates/ron-ledger/src/quickchain/hold_error.rs
crates/ron-ledger/src/quickchain/hold_state.rs
crates/ron-ledger/src/quickchain/hold_transition.rs
crates/ron-ledger/src/quickchain/leaf_projection.rs
crates/ron-ledger/src/quickchain/replay_index.rs
crates/ron-ledger/src/quickchain/state_snapshot.rs
crates/ron-ledger/src/quickchain/transition.rs
crates/ron-ledger/src/quickchain/transition_error.rs
crates/ron-ledger/src/quickchain/types.rs
```

Current active QuickChain test files:

```text
crates/ron-ledger/tests/quickchain_accepted_replay.rs
crates/ron-ledger/tests/quickchain_accepted_replay_auxiliary_inputs.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_extremes.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_identity_rejection.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_rejection_stability.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_retry_stability.rs
crates/ron-ledger/tests/quickchain_accepted_replay_evidence_tamper.rs
crates/ron-ledger/tests/quickchain_accepted_replay_sequence_evidence.rs
crates/ron-ledger/tests/quickchain_account_leaf_projection.rs
crates/ron-ledger/tests/quickchain_active_hold_leaf_projection.rs
crates/ron-ledger/tests/quickchain_atomic_execution.rs
crates/ron-ledger/tests/quickchain_balance_transition.rs
crates/ron-ledger/tests/quickchain_chain_binding.rs
crates/ron-ledger/tests/quickchain_hold_atomic_execution.rs
crates/ron-ledger/tests/quickchain_operation_hash_projection.rs
crates/ron-ledger/tests/quickchain_projection_context_boundaries.rs
crates/ron-ledger/tests/quickchain_projection_replay_equality.rs
crates/ron-ledger/tests/quickchain_receipt_hash_projection.rs
crates/ron-ledger/tests/quickchain_replay_index.rs
crates/ron-ledger/tests/quickchain_retry_after_accepted_rebuild.rs
crates/ron-ledger/tests/quickchain_state_snapshot.rs
crates/ron-ledger/tests/quickchain_terminal_hold_account_projection.rs
crates/ron-ledger/tests/quickchain_terminal_hold_replay_evidence.rs
```

Legacy/non-QuickChain tests still pass:

```text
crates/ron-ledger/tests/idempotency_prop.rs
crates/ron-ledger/tests/interop_vectors.rs
crates/ron-ledger/tests/public_api_semver.rs
crates/ron-ledger/tests/reject_taxonomy.rs
crates/ron-ledger/tests/replay_recovery.rs
crates/ron-ledger/benches/micro.rs
crates/ron-ledger/examples/minimal.rs
```

---

# 6. What was accomplished in `ron-ledger`

## 6.1 Gated QuickChain preflight module

We added and hardened a `quickchain` module that is feature-gated behind `quickchain-preflight`.

This module intentionally does not run as a live chain. It is a deterministic pre-root execution and projection layer that prepares the ledger crate for future QuickChain root/proof/checkpoint work without crossing into forbidden implementation.

The module docs now repeatedly establish:

```text
no hashes
no roots
no checkpoint production
no validators
no signatures
no sockets
no clocks
no IO
no persistence format
no service mutation
no settlement authority
no wallet/spend authority
```

This is critical because many type names are close to future chain concepts. The docs now make the current boundary explicit.

---

## 6.2 Deterministic ROC balance state

We added deterministic in-memory balance state:

```text
QuickChainBalanceState
```

It uses deterministic account ordering and checked integer arithmetic.

It tracks:

```text
per-account balance_minor
total_issued_minor
total_burned_minor
current_supply_minor
```

Core invariants:

```text
all money is integer minor units
no floats
no saturation
no negative balances
sum(account balances) == current_supply_minor
current_supply_minor == total_issued_minor - total_burned_minor
```

Supported balance operations:

```text
issue
transfer
burn
```

Key behavior implemented and tested:

```text
authorized issue increases account balance and supply
unauthorized issue rejects without mutation
authorized burn reduces account balance and supply
unauthorized burn rejects without mutation
burn underflow rejects without mutation
transfer conserves total supply
transfer underflow rejects without crediting destination
zero amount rejects
amount above u128 rejects through ron-proto validation
missing amount rejects through ron-proto validation
missing transfer counterparty rejects through ron-proto validation
account credit overflow rejects without mutation
total supply overflow rejects without mutation
hold operations remain out of basic balance transition scope
different application orders with equivalent final state produce identical ordered state
```

This gives QuickChain a deterministic balance-transition substrate without yet producing state roots.

---

## 6.3 Supply decisions are explicit

Balance operations use:

```text
QuickChainSupplyDecision
```

This was important because `ron-ledger` must not silently decide supply policy on behalf of wallet/policy layers.

Issue and burn decisions are explicit boundary inputs. Accepted replay also requires the original supply decision to be supplied again. If replay uses the wrong supply decision, reconstruction rejects.

This preserves the separation:

```text
policy/wallet decides authorization
ron-ledger executes deterministic checked state transition
```

---

## 6.4 Replay index for operation identity, idempotency, chain binding, and sequences

We added:

```text
QuickChainReplayIndex
QuickChainSubmissionDecision
QuickChainCommittedOperationRecord
QuickChainOperationFamily
QuickChainReplayError
```

The replay index owns:

```text
chain_id binding
operation_id uniqueness
idempotency scope binding
operation family binding
account sequence tracking
primitive ledger sequence tracking
committed evidence lookup
```

Important rules implemented:

```text
operation_id is durable backend-assigned ledger-operation identity
idempotency_key is retry identity, not authority
account_sequence is ledger-assigned
client-assigned account_sequence rejects
first committed operation binds the index to one chain_id
cross-chain submissions reject without mutation
duplicate operation_id rejects
same idempotency key in same scoped family returns original only if intent is identical
same idempotency key with different intent rejects as conflict
same idempotency key may be reused in other account/family scope where allowed
rejections do not consume account sequences
rejections do not consume ledger sequences
committed records must append contiguously by primitive ledger sequence
actor account sequence must advance exactly once
two-account operations advance both distinct participants once
self-transfer advances one account state sequence once
```

This is one of the most important QuickChain foundations because it prevents ambiguous replay identity before roots exist.

---

## 6.5 Chain binding

We added explicit chain binding behavior.

The first accepted committed operation binds the replay index and atomic state to one `chain_id`. Any later operation targeting a different `chain_id` rejects before economic mutation.

Tested behavior:

```text
empty index binds to first committed chain
replay index rejects other chain without mutation
atomic state rejects cross-chain issue before economic mutation
accepted replay boundary can include chain_id
wrong-chain accepted replay rejects
numeric boundary without chain_id remains a legacy shape-only check
```

This is important because future QuickChain roots must not mix state from multiple chain identities.

---

## 6.6 Atomic execution state

We added:

```text
QuickChainAtomicState
QuickChainBalanceExecutionOutcome
QuickChainHoldExecutionOutcome
QuickChainExecutionDisposition
QuickChainExecutionError
```

`QuickChainAtomicState` composes:

```text
QuickChainBalanceState
QuickChainHoldState
QuickChainReplayIndex
```

The key design is copy-on-write atomicity:

```text
clone current state
classify retry/conflict first
execute economic/hold transition on candidate
create committed record
record identity/sequences
validate combined invariants
commit candidate back to self only if everything succeeded
```

This ensures that any failure rolls back the whole operation.

Tested behavior:

```text
fresh issue commits balance, identity, and sequences together
exact retry returns original committed evidence
exact retry happens before reauthorization or new receipt txid validation
idempotency conflict leaves all state components unchanged
duplicate operation_id leaves all state components unchanged
arithmetic rejection does not create identity or consume sequences
invalid receipt reference rolls back an already computed transition
identical ordered commands rebuild identical atomic state
transfer advances both account sequences
self-transfer advances one account sequence once
burn receives one posting sequence and reduces supply
client-assigned account sequence rejects before mutation
```

This is the core deterministic execution boundary for future QuickChain.

---

## 6.7 Hold lifecycle state and transitions

We added deterministic hold state:

```text
QuickChainHoldState
QuickChainOpenHoldRecord
QuickChainTerminalHoldRecord
QuickChainHoldTerminalStatus
QuickChainHoldEpochInput
QuickChainHoldTransition
QuickChainHoldTransitionKind
QuickChainHoldError
```

Supported hold operations:

```text
hold_open
hold_capture
hold_release
hold_expire
```

Hold model:

```text
one hold_id identifies exactly one lifecycle
active holds reserve available ROC
reserved ROC remains part of total balance but is not spendable
terminal holds are retained as replay evidence
terminal holds do not contribute to held totals
release and expiry return the reserved amount to availability without changing total balance
capture moves value from holder to beneficiary
partial capture closes the lifecycle and releases the uncaptured remainder
expiry is deterministic and epoch-gated
capture/release after expiry epoch are blocked
terminal holds cannot be reopened or transitioned again
```

Important constraints:

```text
hold_id must exist for hold operations
amount must be positive
created_at_epoch < expires_at_epoch
terminal epoch cannot precede creation epoch
capture requires counterparty
capture counterparty must match fixed counterparty if one was specified at open
release/expire must consume the full held amount
capture may consume less than held amount
held_by_account must equal sum(active holds)
active and terminal hold IDs must be disjoint
held amount cannot exceed balance
```

Tested behavior:

```text
concurrent holds capture/release/retry correctly
partial capture closes hold and releases remainder
reserved value blocks transfer and burn
expiry is epoch-gated
late capture or release is blocked
terminal hold cannot transition or reopen
release amount mismatch rolls back
capture counterparty mismatch rolls back
invalid receipt and epoch input leave all state unchanged
```

This gives QuickChain safe pre-root reservation semantics.

---

## 6.8 Available balance enforcement

Transfer and burn now respect active holds.

Rules:

```text
total balance includes reserved value
available balance = total balance - held balance
transfer and burn may only spend available balance
hold reservations cannot be bypassed by ordinary balance operations
```

Tested behavior:

```text
reserved value blocks transfer
reserved value blocks burn
ordinary balance-transition errors remain preserved when no holds exist
```

This is important because paid access, holds, escrow-like flows, and future metering cannot allow the same ROC to be reserved and spent at the same time.

---

## 6.9 Terminal hold evidence

We added durable terminal hold evidence for:

```text
captured
released
expired
```

Terminal records retain:

```text
hold_id
holder account
opened counterparty
terminal counterparty
original amount
terminal amount
terminal status
created_at_epoch
expires_at_epoch
terminal_at_epoch
opened operation id
terminal operation id
terminal receipt txid
opened account sequence
terminal account sequence
terminal primitive ledger sequence range
```

Tested behavior:

```text
accepted replay preserves captured, released, and expired terminal holds
terminal hold history rebuilt from accepted replay blocks reopen and retransition
terminal capture can project final account leaves without active holds
terminal capture counterparty account requires explicit projection context
```

This is important because terminal hold evidence must remain replayable even though terminal holds no longer reserve active value.

---

## 6.10 Accepted replay

We added accepted-history replay:

```text
QuickChainAcceptedOperation
QuickChainAcceptedReplayBoundary
QuickChainAtomicState::rebuild_from_accepted_operations
QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary
QuickChainAtomicState::accepted_replay_boundary
```

Accepted replay reconstructs state from a list of already accepted operations. Every operation is re-executed through the same atomic execution methods used by live submissions.

Accepted replay verifies:

```text
same operation intent
same supply decision for balance operations
same hold epoch input for hold operations
same trusted receipt txid
same account sequence
same primitive ledger sequence range
same committed evidence
same chain binding when boundary includes chain_id
same operation count when boundary includes count
same next primitive ledger sequence when boundary includes sequence
```

Important doctrine:

```text
QuickChainAcceptedReplayBoundary is not a root.
It is not a checkpoint.
It is not a signature.
It is not a proof.
It is not a persistence format.
It is not consensus.
It is not settlement.
It is a pre-root adapter seam to check that restored accepted history ends at the expected operation/sequence/chain boundary.
```

Tested behavior:

```text
mixed accepted history rebuilds identical complete state
contradictory accepted history rejects through live transition rules
duplicate record in accepted history rejects
mismatched committed sequences reject reconstruction
accepted replay supply decision must match original execution
accepted replay hold epoch input must be preserved exactly
accepted replay rejects balance replay receipt txid mismatch
accepted replay rejects terminal hold replay receipt txid mismatch
accepted replay rejects hold open account sequence mismatch
accepted replay rejects hold capture primitive ledger range mismatch
accepted replay boundary accepts empty history
accepted replay boundary rejects truncated valid history
accepted replay boundary rejects overextended valid history
accepted replay boundary rejects next ledger sequence mismatch
accepted replay boundary rejects same-shape wrong-chain history
accepted replay boundary round-trips live terminal history
chain-bound empty boundary rejects empty history
numeric boundary without chain_id remains legacy shape-only check
```

This was one of the hardest and most valuable parts of the `ron-ledger` QuickChain work.

---

## 6.11 Boundary stability after retry and rejection

We added tests proving that accepted replay boundaries only advance on actual fresh accepted commits.

Tested retry stability:

```text
exact balance retry does not advance accepted replay boundary
exact hold retry does not advance accepted replay boundary
```

Tested economic rejection stability:

```text
rejected balance operation does not advance accepted replay boundary
rejected hold operation does not advance accepted replay boundary
```

Tested identity rejection stability:

```text
duplicate operation_id rejects without advancing accepted replay boundary
scoped idempotency conflict rejects without advancing accepted replay boundary
```

This means boundary movement is now tightly coupled to real accepted commits only.

---

## 6.12 Deterministic state snapshots

We added:

```text
QuickChainStateSnapshot
QuickChainAccountSnapshot
QuickChainActiveHoldSnapshot
QuickChainStateSnapshotError
```

Snapshots are deterministic, ordered, read-only projections of the current atomic state.

Snapshot includes:

```text
chain_id
ordered account rows
ordered active hold rows
current_supply_minor
next_ledger_sequence
operation_count
```

Account snapshot rows include:

```text
account_id
balance_minor
held_minor
available_minor
account_sequence
```

Active hold snapshot rows include:

```text
hold_id
account_id
counterparty_account_id
amount_minor
created_at_epoch_number
expires_at_epoch_number
opened_operation_id
opened_idempotency_key
```

Important boundaries:

```text
snapshots are not serialized persistence formats
snapshots are not roots
snapshots are not proofs
snapshots are not checkpoint artifacts
snapshots do not hash anything
snapshots exclude terminal holds from active-hold rows
snapshots preserve deterministic account/hold ordering
```

Tested behavior:

```text
empty state snapshot is explicit and read-only
snapshot sorts accounts and active holds
snapshot excludes terminal holds
equivalent final state has identical snapshot despite cross-account commit order
```

This prepares the state for future leaf/root work without producing roots yet.

---

## 6.13 Account leaf projection adapter

We added account leaf projection from internal snapshot rows into frozen `ron-proto` payload DTOs.

Main type:

```text
QuickChainAccountLeafProjectionContext
```

Projected DTO family:

```text
QuickChainAccountLeafPayloadV1
```

Context requires explicit non-ledger values:

```text
receipt_root
holds_root
permissions_root
updated_at_epoch
```

Important boundary:

```text
ron-ledger does not compute receipt_root
ron-ledger does not compute holds_root
ron-ledger does not compute permissions_root
ron-ledger does not invent updated_at_epoch
ron-ledger copies explicit reviewed context
ron-ledger validates final DTO through ron-proto
```

Tested behavior:

```text
empty snapshot projects no account payloads
account payloads are valid
account payloads follow frozen account sort order
projection context set must match account set exactly
repeated projection is deterministic and read-only
ron-proto rejects invalid updated epoch context
terminal capture replay projects final account leaves without active holds
terminal capture counterparty account requires explicit projection context
```

This is the correct pre-root boundary: produce payload DTOs, not canonical bytes or hashes.

---

## 6.14 Active hold leaf projection adapter

We added active hold leaf projection from snapshot rows into frozen `ron-proto` payload DTOs.

Main types:

```text
QuickChainActiveHoldLeafProjectionContext
QuickChainEpochBinding
```

Projected DTO family:

```text
QuickChainActiveHoldLeafPayloadV1
```

Context requires explicit non-ledger values:

```text
purpose
policy_hash
created_at_epoch canonical binding
expires_at_epoch canonical binding
```

The epoch binding is important because:

```text
ledger hold execution uses numeric epoch input
frozen payloads use canonical epoch ID strings
adapter requires explicit binding between the two
```

No silent `u64.to_string()` conversion is allowed.

Tested behavior:

```text
active hold payloads are valid
active hold payloads follow snapshot hold order
projection context set must match active hold set exactly
numeric epoch binding must match ledger-owned hold state
one numeric epoch cannot map to multiple canonical epoch IDs
one canonical epoch ID cannot map to multiple numeric epochs
ron-proto rejects invalid explicit projection context
```

This prevents accidental epoch ambiguity before roots exist.

---

## 6.15 Operation hash-payload projection adapter

We added pure operation hash-payload projection.

Main type:

```text
QuickChainOperationHashProjectionContext
```

Projected DTO family:

```text
QuickChainOperationHashPayloadV1
```

The projection context supplies explicit reviewed values not owned by the operation intent:

```text
purpose
session_budget_id
policy_hash
chain_params_hash
```

The projection copies values from the validated operation intent and explicit context into the DTO. It validates the final DTO through ron-proto.

Important boundary:

```text
this does not serialize canonical JSON
this does not construct preimage bytes
this does not calculate BLAKE3
this does not assert an operation hash
this does not construct a receipt
this does not read a clock
this does not perform IO
this does not mutate ledger state
```

Tested behavior:

```text
all current operation classes project exact canonical family tokens
projection context cannot be rebound to another operation
client-assigned account_sequence is rejected and not silently dropped
produced timestamp is excluded and projection is read-only
ron-proto rejects invalid explicit operation context
operation hash projection copies explicit policy context without computing hashes
```

This gives future canonical hash work a clean input payload without pretending the hash exists yet.

---

## 6.16 Receipt hash-payload projection adapter

We added pure receipt hash-payload projection.

Main type:

```text
QuickChainReceiptHashProjectionContext
```

Projected DTO family:

```text
QuickChainReceiptHashPayloadV1
```

The committed operation record supplies:

```text
operation intent
receipt txid
account sequence
primitive ledger sequence start
primitive ledger sequence end
```

The explicit context supplies:

```text
operation_hash
op/action token
session_budget_id
previous_ledger_root
new_ledger_root
produced_at_ms
```

Important boundary:

```text
previous_ledger_root is opaque reviewed input
new_ledger_root is opaque reviewed input
operation_hash is opaque reviewed input
ron-ledger does not compute these values
legacy accumulator roots must not be treated as QuickChain roots
produced_at_ms is explicit and not derived from client intent timestamp
```

Receipt direction is derived from operation class:

```text
issue: None -> actor
transfer: actor -> counterparty
burn: actor -> None
hold_open: actor -> optional fixed counterparty
hold_capture: actor -> counterparty
hold_release: actor -> None
hold_expire: actor -> None
```

Tested behavior:

```text
all current operation classes project exact receipt direction
receipt context cannot be rebound to another operation
receipt production timestamp is explicit and not client-derived
projection is read-only
zero receipt timestamp rejects
ron-proto rejects invalid explicit receipt action
receipt hash projection copies explicit operation and continuity context
```

This prepares future receipt hash vectors while keeping actual hashing deferred.

---

## 6.17 Projection context boundary tests

We added tests that explicitly prove projection adapters copy reviewed external context without computing roots/hashes.

Covered adapters:

```text
operation hash projection
receipt hash projection
account leaf projection
active hold projection
```

Core proof:

```text
projection copies explicit supplied context
projection does not invent missing context
projection rejects context mismatch
projection validates final ron-proto DTO
projection remains read-only
```

This is very important because the projection adapters are close to future root/hash work. The tests keep them honest.

---

## 6.18 Live/replay projection equality

We added projection equality tests proving that live execution and accepted replay produce identical projected state/payload behavior.

Important test:

```text
mixed_live_history_and_accepted_replay_project_identically
```

This matters because future QuickChain root/proof work depends on deterministic replay equality. The current work proves replay equality at the pre-root payload level.

---

## 6.19 Public API smoke coverage

We updated:

```text
crates/ron-ledger/tests/public_api_semver.rs
```

The smoke test now covers:

```text
legacy public API without quickchain-preflight
feature-gated QuickChain public surface with quickchain-preflight
accepted replay boundary
accepted operation construction
atomic state accepted replay boundary method
atomic state rebuild with boundary method
execution disposition
replay index
snapshot types
projection context types
hash-payload projection functions
leaf projection context types
replay errors
```

The first attempted patch failed because `QuickChainSubmissionDecision` is `#[non_exhaustive]` and the external test tried to exhaustively match it. This was fixed by avoiding exhaustive external matching.

Final result:

```text
public_surface_smoke passed
quickchain_public_api::quickchain_preflight_public_surface_smoke passed
```

---

# 7. Legacy `ron-ledger` remained intact

The old append-only ledger engine still exists and still passes its tests.

Legacy engine responsibilities:

```text
IngestRequest / IngestResponse DTOs
Entry / EntryRecord
Root / Seq
LedgerConfig / Limits
FileStorage / MemoryStorage
append-only WAL-style storage
checkpoint helper
replay_records
rolling accumulator root
observer events
reject taxonomy
idempotency behavior
interop vectors
file-backed replay
micro bench
minimal example
```

Important clarification:

```text
legacy accumulator roots are not QuickChain roots
legacy checkpoints are not QuickChain checkpoints
legacy root publication is not QuickChain settlement
```

Do not confuse the older append-only root mechanism with future QuickChain canonical roots.

---

# 8. What remains for `ron-ledger` when we return

`ron-ledger` is done for the current allowed pre-root slice. Future work should only happen when upstream preconditions are met.

## 8.1 Do not add more `ron-ledger` code immediately

No further code patches are recommended right now.

The next work should move to the next crate in the internal ROC value loop rather than continuing to add `ron-ledger` surface area.

Current next likely crate:

```text
svc-wallet
```

Reason:

```text
ron-proto DTOs are established for current Phase 0
ron-ledger deterministic pre-root execution/projection is green
svc-wallet is the mutation front-door and must eventually call ledger truth safely
```

The wallet/service boundary is now the natural next place to inspect.

---

## 8.2 Future gated `ron-ledger` work: canonical bytes consumption

Do not implement until canonicalization is locked.

Future requirement:

```text
canonical JSON v1 experiment
exact byte vectors
BLAKE3 vector tests
field order tests
unknown-field rejection
float rejection
map ordering tests
domain separator framing
```

Candidate preimage rule from blueprint:

```text
domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Do not implement the framing until exact vectors exist.

When this becomes allowed, `ron-ledger` may consume already-locked canonical payload bytes or hashes, but should not invent its own canonicalization rules independently from `ron-proto`/QC-0A.

---

## 8.3 Future gated `ron-ledger` work: operation and receipt hashes

Current state:

```text
ron-ledger projects operation hash payload DTOs
ron-ledger projects receipt hash payload DTOs
ron-ledger does not canonicalize them
ron-ledger does not hash them
ron-ledger does not produce b3 IDs for them
```

Future work may add actual hash verification/production only after:

```text
payload DTOs are frozen
canonical JSON bytes are locked
domain separators are locked
golden vectors exist
independent reproduction passes
```

Do not use placeholder hashes. Do not use fake b3 values. Do not treat payload projection as hash proof.

---

## 8.4 Future gated `ron-ledger` work: account and hold roots

Current state:

```text
ron-ledger captures ordered account snapshots
ron-ledger captures ordered active hold snapshots
ron-ledger projects account leaf payload DTOs
ron-ledger projects active hold leaf payload DTOs
ron-ledger does not compute leaf hashes
ron-ledger does not compute account roots
ron-ledger does not compute hold roots
```

Before implementing roots, freeze and reproduce vectors for:

```text
empty state root
single-account root
two-account root
three-account odd-leaf root
permutation invariance
duplicate account-key rejection

empty active-hold root
single-hold root
multi-hold root
terminal-hold exclusion
duplicate hold-key rejection
```

The algorithm must explicitly define:

```text
leaf domain
internal-node domain or framing
left/right framing
empty-root derivation
odd-leaf rule
duplicate-key rule
sort-key rule
canonical byte input
hash output format
```

“Standard Merkle tree” is not precise enough.

---

## 8.5 Future gated `ron-ledger` work: receipt roots

Current state:

```text
ron-ledger projects receipt hash payload DTOs
ron-ledger does not store a receipt tree
ron-ledger does not compute receipt roots
```

Before implementing receipt roots, freeze vectors for:

```text
empty receipt root
single-receipt root
sequence-ordered multi-receipt root
txid tie-break behavior
duplicate receipt-key rejection
odd-leaf behavior
receipt ordering rule
receipt inclusion proof shape
```

This should not be started until canonical hash payload vectors are locked.

---

## 8.6 Future gated `ron-ledger` work: durable QuickChain recovery

Current state:

```text
accepted replay can rebuild complete in-memory state from accepted operation history
accepted replay boundary can detect count/sequence/chain mismatch
accepted replay rejects tampered split receipt/evidence fields
accepted replay rejects sequence and epoch mismatches
```

Future durable recovery still needs:

```text
approved durable storage schema
append-only accepted-operation persistence
clear split-field evidence model
versioned replay records
migration plan
crash recovery tests
partial-write tests
tampered-history tests
truncated-history tests
overextended-history tests
wrong-chain-history tests
replay boundary persistence
```

Important: `QuickChainAcceptedOperation` is not currently a persistence format. It is an internal replay input type. Do not serialize it as a durable format without a reviewed storage schema.

---

## 8.7 Future gated `ron-ledger` work: epoch artifacts

Current state:

```text
holds use explicit numeric epoch input
leaf projection requires explicit numeric-to-canonical epoch bindings
ron-ledger does not define epoch sealing
ron-ledger does not define epoch root artifacts
```

Future work needs:

```text
epoch ID canonical form
epoch duration policy
epoch boundary source
epoch sealing input set
receipt/account/hold root inclusion per epoch
replayable epoch artifact DTOs
epoch vector tests
clock/wall-time avoidance rules
```

No wall-clock roots. Epoch inputs must remain explicit and replayable.

---

## 8.8 Future gated `ron-ledger` work: proof-adjacent structures

Current state:

```text
no proofs
no inclusion proofs
no pruning
no DA/challenge/archive
```

Future work must wait for:

```text
root algorithms
inclusion proof format
exclusion proof rules if needed
challenge windows
archive fallback
DA fallback
carrier/archive responsibilities
proof verification tests
missing-data challenge tests
pruning restore tests
```

No pruning before DA/challenge/archive fallback is proven.

---

## 8.9 Future gated `ron-ledger` work: service integration

Future integration should preserve:

```text
svc-wallet calls ledger mutation
ron-ledger stores/executes truth
svc-gateway does not mutate balances
omnigate does not mutate balances
ron-accounting reads/snapshots/reporting only
svc-rewarder plans payouts only
CrabLink displays backend truth only
```

Any service integration must reject:

```text
fake client balances
fake receipts
silent spend
gateway-side mutation
omnigate-side mutation
rewarder direct mutation
accounting direct mutation
raw engagement protocol payouts
```

This likely means the next crate after `ron-ledger` should be `svc-wallet`.

---

# 9. Recommended next crate

Recommended next crate:

```text
svc-wallet
```

Reason:

```text
ron-proto DTOs define the shape
ron-ledger now has deterministic execution/replay/projection
svc-wallet is the mutation front-door
the internal ROC loop needs issue/transfer/burn/hold/capture/release/receipt discipline
```

The next session should inspect whether `svc-wallet` currently:

```text
calls ron-ledger or still uses placeholder/local state
uses integer minor units only
prevents fake balances
prevents fake receipts
requires explicit spend confirmation where appropriate
routes issue/transfer/burn/hold/capture/release through backend truth
returns backend-derived receipts only
has idempotency behavior
does not let gateway/omnigate/rewarder mutate ledger directly
does not expose secrets or spend authority to clients
does not use ROX/Solana/external settlement
```

Potential `svc-wallet` first-session goals:

```text
inventory wallet API surface
identify any fake balance/receipt paths
identify mutation entry points
identify DTO mismatch with ron-proto/ron-ledger
add or harden tests around issue/transfer/burn idempotency
add or harden tests around hold/capture/release if present
keep all QuickChain future-facing work feature-gated or preflight-only
```

Do not assume `svc-wallet` is ready to wire every QuickChain concept. First inspect the codebundle.

---

# 10. Commands to run if returning to `ron-ledger`

Before any future `ron-ledger` work, establish baseline:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-ledger -- --check

crates/ron-ledger/scripts/dev-quickcheck.sh

cargo check --workspace
```

If touching a focused QuickChain test:

```bash
cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test <focused_test_name>
```

If touching public API:

```bash
cargo test -p ron-ledger \
  --test public_api_semver

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test public_api_semver
```

If touching projections:

```bash
cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_operation_hash_projection

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_receipt_hash_projection

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_account_leaf_projection

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_active_hold_leaf_projection

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_projection_replay_equality
```

If touching accepted replay:

```bash
cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_accepted_replay

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_accepted_replay_boundary

cargo test -p ron-ledger \
  --features quickchain-preflight \
  --test quickchain_retry_after_accepted_rebuild
```

Always finish with:

```bash
crates/ron-ledger/scripts/dev-quickcheck.sh

cargo check --workspace

bash scripts/make_crate_codex.sh -c ron-ledger
```

---

# 11. Things not to do when returning to `ron-ledger`

Do not add:

```text
QuickChain root production
QuickChain checkpoint hashes
QuickChain validators
QuickChain consensus
fork choice
signing committee runtime
pruning
carrier rewards
archive rewards
ROX
Solana
external settlement
public bridge
staking
liquidity
exchange-facing logic
gateway-side ledger mutation
omnigate-side ledger mutation
rewarder-side ledger mutation
accounting-side ledger mutation
CrabLink chain authority
fake balances
fake receipts
fake proofs
placeholder hashes
placeholder roots
DB-order roots
wall-clock roots
raw engagement payouts
```

Do not convert current preflight adapter types into persistence formats without a reviewed durable schema.

Do not treat:

```text
idempotency_key as authority
receipt_txid as spend authority
ContentId input as proof
projection payload as hash
snapshot as root
accepted replay boundary as checkpoint
legacy ledger root as QuickChain root
```

---

# 12. Important invariants to preserve

Preserve all of the following:

```text
money uses integer minor units only
no floats
operation_id is durable backend-assigned operation identity
idempotency_key is scoped retry identity only
account_sequence is ledger-assigned
hold_id identifies one lifecycle
expiry is epoch-based and explicit
chain_id binds accepted state
accepted replay must reproduce exact committed evidence
exact retry returns original evidence without mutation
rejections do not advance accepted boundary
reserved ROC is not spendable
terminal holds cannot reopen
snapshots are deterministic and read-only
projection contexts are explicit
ron-ledger does not invent roots, hashes, policy, epochs, or timestamps
ron-ledger does not perform service mutation outside its truth role
```

---

# 13. Summary of what we achieved

In this `ron-ledger` QuickChain stretch, we transformed `ron-ledger` from a legacy append-only ROC ledger crate into a crate that also contains a serious, gated QuickChain preflight execution core.

We added deterministic state machinery for:

```text
balances
supply counters
holds
terminal hold evidence
operation identity
idempotency identity
chain binding
account sequences
primitive ledger sequences
accepted replay
state snapshots
pre-root payload projection
public API smoke protection
```

We also proved the important negative space:

```text
no root production
no checkpoint production
no canonical hashing
no validators
no consensus
no settlement
no persistence format
no external chain logic
no fake balances
no fake receipts
```

This is exactly the kind of boring deterministic substrate QuickChain needed before any future root or proof work.

The next productive move is not more `ron-ledger` code. The next productive move is inspecting `svc-wallet`, because the internal ROC loop needs the wallet mutation front-door to route issue/transfer/burn/hold/capture/release/receipt behavior through backend truth without fake balances or fake receipts.


### END NOTE - JUNE 12 2026 - 21:40 CST