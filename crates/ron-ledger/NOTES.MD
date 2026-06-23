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


### BEGIN NOTE - JUNE 16 2026 - 14:10 CST

Below is one shared notes document you can paste into **both** crates. It is based on the current `ron-proto` bundle, which now includes the exhaustive QuickChain preflight runner and bash vector verifiers, the current `ron-ledger` bundle, which includes the gated QuickChain preflight feature and exhaustive runner, and the latest terminal proof where `ron-proto` passed 20 discovered QuickChain tests, both bash verifiers, clippy, and the final success line.   

# QuickChain Phase-0 Shared Notes — ron-proto + ron-ledger Pair

Date: 2026-06-16
Crates covered by this shared note:

* `crates/ron-proto`
* `crates/ron-ledger`

Purpose of this note:

This is a shared milestone record for the `ron-proto + ron-ledger` QuickChain crate pair. It is intentionally written as one set of notes that can be dropped into both crates. It is not a narrow crate-specific changelog. It documents the completed pair-level strategy, the current green gates, the remaining boundaries, and the next crate-pair build order for continuing QuickChain coherently.

---

## Executive status

The `ron-proto + ron-ledger` pair is effectively complete for the current allowed QuickChain Phase-0 / pre-root scope.

Estimated status:

* `ron-proto` QuickChain Phase-0 / pre-root scope: 99% complete.
* `ron-ledger` QuickChain Phase-0 / pre-root scope: 97–98% complete.
* Combined `ron-proto + ron-ledger` crate-pair milestone: 98–99% complete.
* Overall internal QuickChain Phase-0/preflight across all crates: approximately 68–75% complete before the remaining service/client crate-pair work.
* Full long-term QuickChain blueprint: approximately 35–42% complete, because roots, validators, settlement, anchors, bridges, staking, liquidity, pruning, and external-chain behavior remain intentionally forbidden future phases.

This pair is not marked as 100% because QuickChain deliberately does not yet authorize root-producing runtime, validators, settlement, pruning, public anchors, bridges, staking, or external chain integration. For the work that is currently allowed, this pair is essentially done.

---

## Current doctrine reminder

QuickChain is future settlement infrastructure. It is not the current runtime, not a public chain, not a bridge, not staking, not liquidity, not Solana/ROX integration, and not a replacement for internal ROC wallet/ledger truth.

Current mission remains:

```text
prove internal ROC economy first
```

Internal value-plane path:

```text
ron-proto econ/QuickChain DTOs
  -> ron-ledger deterministic truth and preflight execution
  -> svc-wallet mutation front door
  -> svc-storage / svc-gateway / omnigate paid enforcement
  -> ron-accounting snapshots
  -> svc-rewarder payout planning
  -> wallet/ledger receipts
```

QuickChain doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

---

## Hard boundaries that remain in force

For this phase, do not add:

* QuickChain root-producing runtime.
* QuickChain checkpoint-producing runtime.
* Validators.
* Validator committees.
* Settlement.
* External anchors.
* Public bridges.
* Solana integration.
* ROX integration.
* Staking.
* Liquidity.
* Exchange-facing logic.
* Pruning.
* Live chain state.
* CrabLink chain authority.
* Gateway/omnigate/rewarder ledger mutation.
* Raw engagement protocol payouts.
* DB-order roots.
* Wall-clock roots.
* Placeholder/fake hashes.
* Unknown consensus fields.
* Any Python helper scripts for QuickChain preflight/tooling.

Important nuance for `ron-ledger`:

`ron-ledger` already has legacy/internal accumulator/root/checkpoint mechanics for the existing append-only ledger engine. Those are not the same thing as new QuickChain root/checkpoint/consensus production. The QuickChain-specific Phase-0 work must remain pre-root and gated under `quickchain-preflight`.

---

## Bash-only tooling rule

A major milestone in this pair was converting QuickChain helper tooling to bash-only and guarding against Python script drift.

Current rule:

```text
QuickChain helper scripts must be bash-only.
No new Python helper scripts.
No python/python3 one-liners in future QuickChain patch batches.
Use bash, Rust tests, and cargo commands.
```

This applies to future work across the project, especially new preflight runners and verifier scripts.

---

# ron-proto QuickChain status

## Role of ron-proto

`ron-proto` owns DTOs and canonical wire/data shapes. It must remain pure DTO/schema/canonicalization territory.

`ron-proto` must not:

* Mutate wallet/ledger/storage state.
* Perform service I/O.
* Own ledger truth.
* Verify signatures as authority.
* Produce live roots.
* Decide finality.
* Create settlement claims.
* Act as validator runtime.
* Store secrets.
* Represent spend authority.

It may safely define strict DTO shapes, validation helpers, canonical JSON byte experiments, domain separators, vector schemas, and test vectors.

---

## ron-proto accomplishments in this milestone

The crate now has a robust QuickChain Phase-0 DTO/vector surface covering:

* Canonical JSON v1 helpers.
* Strict DTO round-trip behavior.
* Unknown-field rejection.
* Strict `b3:<64 lowercase hex>` behavior.
* Integer minor-unit money strings.
* No floats for money.
* Operation IDs and hold IDs.
* Idempotency key shape.
* Operation-class wire names.
* Operation intent validation.
* Hold open/capture/release/expire intent matrix.
* Hold state lifecycle validation.
* Receipt DTO validation.
* Accepted receipt shapes.
* Receipt direction/lifecycle constraints.
* Receipt ordering and sort keys.
* Account and hold sort keys.
* Replay/idempotency scenario vectors.
* Event class policy helpers.
* Usage event validation.
* Empty tree DTOs and exact canonical bytes.
* Chain params with explicit disabled state for external/ROX anchor posture.
* Challenge wire names as descriptive DTOs, not active challenge runtime.
* Domain separation constants and validation.
* Hash payload DTOs for:

  * operation hash payload
  * receipt hash payload
  * account leaf payload
  * active hold leaf payload
  * unsigned checkpoint payload
* Locked hash vector review gates.
* File-backed locked bytes vectors.
* Bash vector inventory verifier.
* Bash hash payload verifier.
* Exhaustive bash preflight runner.

---

## ron-proto vector inventory status

Current checked-in QuickChain vector inventory:

```text
total vector files: 36
locked_bytes: 31
locked_hash: 5
sketch: 0
typed quickchain.test-vector.v1 vectors: 31
```

Current hash payload verifier summary:

```text
hash payload vector files: 5
payload bytes: 3841
preimage bytes: 3972
optional b3sum checks: 0
```

The optional `b3sum` checks being zero is acceptable when `b3sum` is not installed. The Rust `quickchain_hash_payloads` test remains the mandatory hash-truth gate.

---

## ron-proto exhaustive preflight runner

Current runner:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Current behavior:

* Runs from repo root or from inside the repository.
* Rejects checked-in `.py` helper scripts under `crates/ron-proto`.
* Runs `cargo fmt -p ron-proto -- --check`.
* Discovers every `crates/ron-proto/tests/quickchain_*.rs` test.
* Runs each discovered QuickChain integration test.
* Requires at least 20 discovered QuickChain tests.
* Runs the bash vector inventory verifier.
* Runs the bash hash payload verifier.
* Runs `cargo clippy -p ron-proto --all-targets -- -D warnings`.
* Prints final success line.

Current proven final success line:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=20
```

---

## ron-proto important tests now covered

The exhaustive runner currently discovers and runs 20 QuickChain integration tests, including:

* `quickchain_canonical_json`
* `quickchain_chain_params`
* `quickchain_challenge_wire`
* `quickchain_domain_separation`
* `quickchain_dto_strict`
* `quickchain_empty_tree`
* `quickchain_event_class`
* `quickchain_file_vectors`
* `quickchain_hash_payloads`
* `quickchain_hold_scenarios`
* `quickchain_ids_and_money`
* `quickchain_locked_bytes`
* `quickchain_operation_hold`
* `quickchain_receipt_dto`
* `quickchain_receipt_order`
* `quickchain_replay_idempotency`
* `quickchain_sort_keys`
* `quickchain_tooling_boundary`
* `quickchain_vector_dto`
* `quickchain_vector_inventory`

---

## ron-proto commands to prove current state

Run:

```bash
bash crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Expected final output:

```text
verified quickchain vector inventory: files=36 locked_bytes=31 locked_hash=5 sketch=0 typed_vectors=31
verified quickchain hash payload vectors with bash: files=5 payload_bytes=3841 preimage_bytes=3972 optional_b3sum_checks=0
ron-proto quickchain exhaustive preflight gate passed: tests=20
```

Optional focused checks:

```bash
cargo fmt -p ron-proto

cargo test -p ron-proto --test quickchain_tooling_boundary
cargo test -p ron-proto --test quickchain_hash_payloads
cargo test -p ron-proto --test quickchain_vector_inventory

bash crates/ron-proto/tests/tools/verify_quickchain_vector_inventory.sh
bash crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.sh

cargo clippy -p ron-proto --all-targets -- -D warnings
```

---

## ron-proto completion assessment

For the current pre-root QuickChain scope, `ron-proto` is effectively done.

Remaining future work is intentionally deferred until the blueprint authorizes the next phase:

* Add or update vectors only when the QuickChain blueprint intentionally changes.
* Do not add fake hashes or placeholder hashes.
* Do not add root-producing APIs yet.
* Do not add validator/checkpoint/finality runtime.
* Do not add settlement or bridge DTOs that imply live external integration.
* Do not add unknown consensus fields.
* Do not allow DTOs to become authority.

Future `ron-proto` work, when authorized, should continue the sequence:

```text
sketch -> locked_bytes -> locked_hash
```

No vector should jump straight to claimed hash/root truth without reviewed canonical bytes and reviewed preimage framing.

---

# ron-ledger QuickChain status

## Role of ron-ledger

`ron-ledger` owns deterministic economic truth and preflight execution/replay behavior.

For QuickChain Phase-0, `ron-ledger` must remain:

* Internal.
* Deterministic.
* Feature-gated.
* Pre-root.
* Pre-validator.
* Pre-settlement.
* Pre-bridge.
* Pre-anchor.

`ron-ledger` may safely own:

* Checked integer balance transitions.
* Non-negative balance invariants.
* Issue/transfer/burn transition behavior.
* Hold open/capture/release/expire lifecycle behavior.
* Reserved-vs-available balance rules.
* Ledger-assigned account sequence behavior.
* Ledger-assigned primitive sequence ranges.
* Operation replay identity.
* Idempotency retry behavior.
* Accepted-operation replay.
* Explicit replay boundaries.
* Pure projection into frozen `ron-proto` hash-payload DTOs.
* Deterministic snapshots and ordering.

`ron-ledger` must not yet own:

* QuickChain root production.
* QuickChain checkpoint finality.
* Validators.
* External settlement.
* Public anchors.
* Bridges.
* Staking.
* Liquidity.
* Chain authority for CrabLink.
* Wall-clock-derived root ordering.
* DB-order-derived root ordering.
* Unknown consensus fields.

---

## ron-ledger accomplishments in this milestone

The crate now has a strong gated QuickChain preflight layer under:

```toml
quickchain-preflight = ["dep:ron-proto"]
```

Major coverage includes:

* Atomic QuickChain execution state.
* Copy-on-write candidate mutation.
* All-or-nothing balance transitions.
* Issue/transfer/burn behavior.
* Supply decision handling as explicit input.
* No silent supply authority.
* Non-negative balance invariants.
* Current supply invariants.
* Hold lifecycle execution:

  * open
  * capture
  * release
  * expire
* Active hold state.
* Terminal hold state.
* Reserved balance tracking.
* Available balance enforcement.
* Holds preventing transfer/burn from spending reserved funds.
* Explicit epoch inputs for hold lifecycle decisions.
* No wall-clock hold expiry authority.
* Replay index.
* Operation identity tracking.
* Idempotency key conflict/retry behavior.
* Account sequence assignment.
* Primitive ledger sequence range assignment.
* Chain binding.
* Accepted replay.
* Accepted replay boundary validation.
* Accepted replay chain ID validation.
* Accepted replay operation count validation.
* Accepted replay next ledger sequence validation.
* Accepted replay evidence tamper rejection.
* Retry-after-accepted rebuild behavior.
* Projection context boundaries.
* Operation hash payload projection.
* Receipt hash payload projection.
* Account leaf payload projection.
* Active hold leaf payload projection.
* Projection replay equality.
* State snapshot determinism.
* Terminal hold account projection.
* Terminal hold replay evidence.
* Pre-root source boundary checks.
* Bash-only tooling boundary checks.
* Exhaustive QuickChain preflight runner.

---

## ron-ledger exhaustive preflight runner

Current runner:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Current behavior:

* Runs from repo root or from inside the repository.
* Rejects checked-in `.py` helper scripts under `crates/ron-ledger`.
* Runs `cargo fmt -p ron-ledger -- --check`.
* Discovers every `crates/ron-ledger/tests/quickchain_*.rs` test.
* Runs each discovered QuickChain integration test with `--features quickchain-preflight`.
* Requires at least 20 discovered QuickChain tests.
* Runs feature-gated clippy:

  ```bash
  cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
  ```
* Prints final success line.

Previously proven final success line:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=26
```

---

## ron-ledger important tests now covered

The current codebundle shows the QuickChain test surface includes:

* `quickchain_accepted_replay`
* `quickchain_accepted_replay_auxiliary_inputs`
* `quickchain_accepted_replay_boundary`
* `quickchain_accepted_replay_boundary_extremes`
* `quickchain_accepted_replay_boundary_identity_rejection`
* `quickchain_accepted_replay_boundary_rejection_stability`
* `quickchain_accepted_replay_boundary_retry_stability`
* `quickchain_accepted_replay_evidence_tamper`
* `quickchain_accepted_replay_sequence_evidence`
* `quickchain_account_leaf_projection`
* `quickchain_active_hold_leaf_projection`
* `quickchain_atomic_execution`
* `quickchain_balance_transition`
* `quickchain_chain_binding`
* `quickchain_hold_atomic_execution`
* `quickchain_operation_hash_projection`
* `quickchain_pre_root_boundary`
* `quickchain_projection_context_boundaries`
* `quickchain_projection_replay_equality`
* `quickchain_receipt_hash_projection`
* `quickchain_replay_index`
* `quickchain_retry_after_accepted_rebuild`
* `quickchain_state_snapshot`
* `quickchain_terminal_hold_account_projection`
* `quickchain_terminal_hold_replay_evidence`
* `quickchain_tooling_boundary`

---

## ron-ledger commands to prove current state

Run:

```bash
bash crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Expected final output:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=26
```

Optional focused checks:

```bash
cargo fmt -p ron-ledger

cargo test -p ron-ledger --features quickchain-preflight --test quickchain_tooling_boundary
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_pre_root_boundary
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_context_boundaries
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_replay_equality

cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings
```

Existing broader ledger check remains:

```bash
bash crates/ron-ledger/scripts/dev-quickcheck.sh
```

---

## ron-ledger completion assessment

For the current pre-root QuickChain scope, `ron-ledger` is effectively complete for this pass.

Remaining future work is intentionally deferred until the blueprint authorizes the next phase:

* No QuickChain root-producing engine yet.
* No QuickChain checkpoint finality yet.
* No validators yet.
* No settlement yet.
* No public anchors yet.
* No bridge logic yet.
* No pruning yet.
* No persistent QuickChain chain state yet.
* No gateway/omnigate/rewarder ledger mutation.
* No raw engagement payout mutation.

Future `ron-ledger` work, when authorized, should begin with formal root preparation only after `ron-proto` canonical bytes and locked hash vectors explicitly authorize the next shape.

---

# Pair-level summary

The pair now gives QuickChain a strong internal foundation:

```text
ron-proto:
  strict DTOs
  canonical JSON
  domain separation
  vector inventory
  locked bytes
  locked hashes
  bash verifiers
  exhaustive preflight runner

ron-ledger:
  deterministic execution
  hold lifecycle
  replay identity
  accepted replay
  sequence boundaries
  projection into ron-proto DTOs
  pre-root guardrails
  exhaustive feature-gated preflight runner
```

Together, these crates prove:

* DTO shapes are strict.
* Unknown fields reject.
* Money is integer minor-unit only.
* B3 hashes are lowercase and canonical.
* Idempotency is retry identity, not authority.
* Operation IDs are durable operation identity.
* Account sequences are ledger-assigned.
* Holds are lifecycle objects.
* Epoch behavior is explicit input, not wall-clock authority.
* Replay is deterministic.
* Accepted history can be rebuilt and boundary-checked.
* Projections are read-only and explicit-context-only.
* Tooling is bash-only.
* The current pair does not produce QuickChain roots, settlement, validators, bridges, or anchors.

---

# Current estimated completion

Current estimates after finishing this pair:

```text
ron-proto QuickChain pre-root scope: 99%
ron-ledger QuickChain pre-root scope: 97–98%
ron-proto + ron-ledger pair: 98–99%

overall internal QuickChain Phase-0/preflight: 68–75%
overall full long-term QuickChain blueprint: 35–42%
```

After completing the remaining crate-pair build order below, estimated status would become:

```text
internal Phase-0/preflight overall: 88–93%
internal ROC economy readiness before roots/validators: 80–88%
full long-term QuickChain blueprint: 48–58%
```

The full long-term blueprint remains below 60% even after all current crate pairs because major later phases are intentionally deferred:

* root engine
* checkpoint/finality layer
* proof system
* challenge/archive/fallback planning
* validators
* pruning
* DA/anchor strategy
* settlement
* external bridge
* staking/liquidity/exchange-facing logic

---

# Coherent next crate-pair build order

The next QuickChain work should continue in crate pairs so the value-plane proof stays coherent and does not drift into premature chain/runtime work.

## Pair 1 — `svc-wallet + ron-accounting`

This is the immediate next pair.

Why:

* `svc-wallet` is the service mutation front door.
* `ron-accounting` is snapshots/reporting only, not balance truth.
* This pair proves service-level wallet mutations and accounting snapshots do not diverge from ledger truth.

Key goals:

* Ensure `svc-wallet` is the only service front door for issue/transfer/burn/hold/capture/release/receipt flows.
* Ensure no fake balances.
* Ensure no fake receipts.
* Ensure no silent spend.
* Ensure quotes and confirmations are explicit.
* Ensure receipts are backend-derived.
* Ensure `ron-accounting` never becomes balance truth.
* Ensure accounting snapshots are derived from accepted ledger/wallet truth.
* Ensure accounting does not mutate ledger/wallet state.
* Add or harden exhaustive bash QuickChain preflight runners.
* Add or harden no-Python tooling boundaries.
* Add or harden no-root/no-settlement/no-anchor guardrails.

Expected impact:

This is the biggest remaining internal-economy gate after `ron-proto + ron-ledger`.

---

## Pair 2 — `svc-rewarder + svc-storage`

Why:

* `svc-rewarder` plans payouts but must never mutate ledger.
* `svc-storage` owns bytes by b3 and paid storage/access boundaries but must never unlock paid content from cache alone.

Key goals:

* Ensure rewarder payout plans are planning artifacts only.
* Ensure rewarder cannot directly mint or transfer ROC.
* Ensure raw engagement cannot directly allocate protocol ROC.
* Ensure storage enforces b3 identity.
* Ensure storage cannot fabricate paid access receipts.
* Ensure paid storage/content enforcement relies on backend wallet/ledger receipts.
* Ensure no fake content unlock from offline cache alone.
* Add exhaustive bash QuickChain preflight runners if missing.
* Add no-Python tooling boundaries.
* Add no-root/no-validator/no-settlement guardrails.

Expected impact:

This pair protects the creator payout path and paid byte path from authority drift.

---

## Pair 3 — `svc-gateway + omnigate`

Why:

* `svc-gateway` is the public boundary.
* `omnigate` hydrates user-facing content and paid access views.
* This pair is where public paid access could accidentally bypass wallet/ledger truth if not guarded.

Key goals:

* Ensure gateway is public boundary only.
* Ensure omnigate hydrates but does not mutate ledger.
* Ensure paid access requires backend wallet/receipt path.
* Ensure display-only receipts are not treated as balance truth.
* Ensure no fake unlocks.
* Ensure cache cannot unlock paid content alone.
* Ensure no gateway/omnigate QuickChain root/checkpoint/validator behavior.
* Ensure no external settlement or bridge references become runtime behavior.
* Add exhaustive bash QuickChain preflight runners if missing.
* Add no-Python tooling boundaries.

Expected impact:

This pair proves the public access surface cannot become economic authority.

---

## Pair 4 — `svc-index + ron-policy`

Why:

* `svc-index` is lookup/pointer service only.
* `ron-policy` is declarative policy only.
* These layers must not become authority, balance truth, settlement, or hidden QuickChain runtime.

Key goals:

* Ensure index is lookup/pointer service only.
* Ensure names/pointers are not authority.
* Ensure b3 hashes remain truth.
* Ensure policy is declarative and does not mutate balances.
* Ensure policy cannot produce receipts.
* Ensure policy cannot unlock paid content without backend wallet/receipt truth.
* Ensure no root/checkpoint/validator/bridge logic enters either crate.
* Add exhaustive bash QuickChain preflight runners if missing.
* Add no-Python tooling boundaries.

Expected impact:

This pair protects navigation, lookup, and policy from becoming hidden economic/runtime authority.

---

## Pair 5 — `CrabLink Tauri + client adapters`

Why:

* Tauri is the primary CrabLink client.
* React/TS owns display and user intent, not backend truth.
* Tauri Rust mediates native privilege but still must not mutate ledger truth directly.

Key goals:

* Ensure client requests quotes and confirmations explicitly.
* Ensure client displays backend-derived balances/receipts only.
* Ensure no private keys/seeds/raw capabilities/spend authority enter React state/localStorage/URLs/logs.
* Ensure paid flow stays:

  ```text
  prepare/quote -> explicit confirmation -> backend wallet path -> backend receipt -> unlock/render -> display-only receipt cache -> balance refresh
  ```
* Ensure offline cache verifies b3 before trusted render.
* Ensure offline cache cannot unlock paid content alone.
* Ensure media remains bounded and range/segment oriented.
* Ensure no full-file large media through command result payloads.
* Ensure sidecar/local node stays optional, explicit, supervised, bounded, and disabled by default.
* Ensure facets remain inert until sandboxed.
* Ensure CrabLink Tauri does not become QuickChain chain authority.

Expected impact:

This pair closes the loop at the product/client boundary after backend service authority is clean.

---

# Next-session recommendation

The next session should start with:

```text
svc-wallet + ron-accounting
```

Attach fresh codebundles for both crates:

```bash
bash scripts/make_crate_codex.sh -c svc-wallet
bash scripts/make_crate_codex.sh -c ron-accounting
```

Then work in the same style used for `ron-proto + ron-ledger`:

* full paste-ready files or bash scripts
* no Python helper scripts
* no git patches/unified diffs
* exact paths
* preserve RO headers
* run fmt
* run clippy with `-D warnings`
* run focused tests
* add exhaustive bash preflight runners where useful
* add no-Python tooling boundary tests
* add no-fake-balance/no-fake-receipt/no-silent-spend tests
* add no-root/no-validator/no-settlement/no-anchor guardrails
* keep QuickChain future-only and internal-ROC-first

---

# Final pair verdict

`ron-proto + ron-ledger` should be treated as complete for the current QuickChain Phase-0 / pre-root milestone.

Do not continue adding random features to this pair right now. The correct next move is to carry the same discipline into the service boundary crates, beginning with:

```text
svc-wallet + ron-accounting
```

Only return to `ron-proto + ron-ledger` when:

* a future QuickChain blueprint update requires new DTO/vector work,
* a later phase explicitly authorizes root-producing preparation,
* a downstream crate exposes a gap in the current DTO/execution boundary,
* or a regression breaks the current exhaustive preflight gates.


### END NOTE - JUNE 16 2026 - 14:10 CST



### BEGIN NOTE - JUNE 18 2026 - 20:10 CST

Below are two paste-ready notes sections. The terminal proof shows `ron-proto` passed its focused source-boundary test, full QuickChain preflight, vector verifiers, and clippy; `ron-ledger` passed its focused canonical projection boundary plus the exhaustive QuickChain preflight with 27 tests and clippy. 

# QuickChain QC-1A / Pre-Root Notes — `ron-proto`

## Status

`ron-proto` is green for the current QuickChain QC-1A / pre-root DTO, canonicalization, vector, and boundary-hardening pass.

This crate remains the QuickChain DTO and vector contract layer. It does **not** own runtime authority, ledger mutation, service IO, chain execution, validator behavior, settlement, anchors, bridges, staking, pruning, or external-chain behavior. Its job in this phase is to define strict, deterministic, reviewable shapes that downstream crates can consume without smuggling in authority.

Current estimate for this crate in the QuickChain pre-root phase: approximately **99% complete**.

## What changed in this pass

### 1. Hardened the QuickChain source-boundary test

The primary change was to replace the brittle `quickchain_source_boundary.rs` allowlist logic with a more durable boundary test.

The old test was failing because it attempted to explicitly allowlist every `#[serde(default)]` field by exact field name and file. That caused repeated failures as legitimate optional DTO metadata fields appeared, such as:

* `signatures`
* `account_id`
* optional receipt/hash-payload context fields
* optional usage-event metadata
* optional vector-growth fields

The updated test now enforces the actual invariant instead of chasing individual fields.

The new source-boundary test verifies:

* QuickChain source files remain DTO/vector-prep code only.
* QuickChain source files do not gain runtime authority.
* QuickChain source files do not gain implicit default constructors.
* `#[serde(default)]` remains limited to structural optional/growth fields only.
* Authority-sensitive defaults remain blocked.

### 2. Runtime-authority tokens remain forbidden

The source-boundary test continues to reject runtime or authority creep inside `src/quickchain`.

The boundary scan rejects tokens associated with:

* Hash execution in DTO source, such as direct `blake3::` use.
* Alternate hash algorithms such as `sha2::` and `sha3::`.
* Signature/validator crypto primitives such as `ring::`, `k256::`, and `secp256k1`.
* Async runtime behavior such as `tokio::`, `.await`, and `.spawn(`.
* HTTP/server/client behavior such as `reqwest::`, `axum::`, and `hyper::`.
* Filesystem IO such as `std::fs`.
* Network IO such as `std::net`.
* Threads such as `std::thread`.
* Wall-clock dependence such as `SystemTime` / `UNIX_EPOCH`.
* Randomness such as `rand::` and `thread_rng`.
* Database dependencies such as `sled::`, `rusqlite::`, `sqlx::`, `rocksdb::`, and `redb::`.
* `unsafe`.

This preserves the rule that `ron-proto` may define canonical payloads and vector contracts, but must not become a runtime, hasher, verifier, database reader, network client, or chain authority.

### 3. Implicit default constructors remain forbidden

The source-boundary test continues to reject implicit default constructors in QuickChain DTO/validation source.

The boundary scan rejects:

* `derive(Default)`
* `impl Default`
* `::default(`
* `Default::default(`

This protects against accidentally creating complete QuickChain economic, root, receipt, checkpoint, validator, or chain objects without explicit field construction.

The purpose is not to ban `#[serde(default)]` on optional/growth fields. The purpose is to ban broad Rust-side constructors that could silently invent authority-sensitive values.

### 4. `#[serde(default)]` policy was corrected

The new policy is:

`#[serde(default)]` is allowed only when the field is structurally optional or additive:

* `Option<T>`
* `Vec<T>`
* `BTreeMap<K, V>`

This is the right boundary for DTO evolution because absent optional metadata should deserialize as absent, absent lists should deserialize as empty, and future additive map-like metadata can remain inert.

The test now rejects `#[serde(default)]` on non-structural required fields.

Examples of legitimate defaults include optional or additive DTO fields such as:

* optional usage-event metadata
* optional account/counterparty/object/site references
* optional amount metadata for analytics or metering events
* optional hash-payload planning context
* optional receipt evidence fields
* optional terminal hold lifecycle context
* optional vector display/audit fields
* optional signature lists where an absent list means “no signatures present,” not “verified”

### 5. The denylist was adjusted to avoid false positives

The initial denylist was too aggressive. It treated fields like `account_id` as authority-sensitive everywhere.

That was incorrect for `UsageEvent` and similar DTOs because those fields are optional metadata, not balance truth and not ledger mutation authority.

The surgical patch removed legitimate optional metadata/evidence field names from the hard denylist while leaving truly dangerous defaults blocked.

Fields such as these are allowed when they are structural `Option`, `Vec`, or `BTreeMap` growth fields:

* `account_id`
* `from_account_id`
* `to_account_id`
* `counterparty_account_id`
* `amount_minor`
* `account_sequence`
* `account_sequence_terminal`
* `ledger_seq_start`
* `ledger_seq_end`
* `operation_hash`
* `receipt_hash`
* `receipt_root`
* `checkpoint_hash`
* `previous_ledger_root`
* `new_ledger_root`
* `data_availability_root`
* `validator_set_hash`

These fields remain controlled by downstream validation tests and canonical-vector tests. They are not made authoritative merely because they are optional in DTO payloads.

The denylist still blocks dangerous defaulting for identity and authority fields such as:

* `version`
* `schema`
* `domain`
* `domain_separator`
* `canonicalization`
* `encoding`
* `chain_id`
* `checkpoint_id`
* `checkpoint_height`
* `checkpoint_seq`
* `checkpoint_sequence`
* `epoch`
* `epoch_id`
* `epoch_start_ms`
* `epoch_end_ms`
* `created_at_ms`
* `updated_at_ms`
* `expires_at_ms`
* required root identities
* `operation_id`
* `idempotency_key`
* `balance_minor`
* `held_minor`
* `current_supply_minor`
* `asset`
* `receipt_id`
* `receipt_txid`
* `receipt_status`
* `receipt_direction`
* `receipt_action`
* `validator_id`
* `validator_public_key`
* single `signature`
* `signature_algorithm`
* `signed_payload_hash`

This creates a better long-term scanner: it allows inert DTO growth while still failing loudly on fields that must never be silently invented.

## Why this matters

This pass tightened the line between DTO evolution and authority creep.

`ron-proto` now has a source-level guardrail that says:

* DTOs can evolve additively.
* Optional metadata can remain optional.
* Vector scaffolding can exist.
* Canonical bytes can be locked.
* Hash payload shapes can be described.
* But runtime hashing, validator logic, service IO, roots, settlement, bridges, anchors, and ledger mutation cannot enter this crate.

This is aligned with the QuickChain doctrine:

* Determinism before distribution.
* DTOs before roots.
* Roots before validators.
* Proofs before pruning.
* Internal ROC before external anchors.

## Test proof from this pass

The following passed:

```text
cargo test -p ron-proto --test quickchain_source_boundary
```

Result:

```text
running 3 tests
test quickchain_serde_defaults_stay_structural_not_authority_sensitive ... ok
test quickchain_sources_do_not_gain_implicit_default_constructors ... ok
test quickchain_sources_remain_pure_dto_and_vector_prep ... ok

test result: ok. 3 passed
```

The exhaustive preflight runner also passed:

```text
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

The runner completed:

* all discovered `quickchain_*.rs` tests
* QuickChain canonical JSON tests
* chain params tests
* challenge wire tests
* domain separation tests
* strict DTO tests
* empty-tree tests
* event-class tests
* file-backed vector tests
* hash-payload tests
* hold-scenario tests
* IDs and money tests
* locked-bytes tests
* operation/hold tests
* receipt DTO tests
* receipt-order tests
* replay/idempotency tests
* sort-key tests
* source-boundary tests
* tooling-boundary tests
* vector DTO tests
* vector inventory tests
* bash vector inventory verifier
* bash hash payload verifier
* clippy with warnings denied

Final runner result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=21
```

## Current crate boundaries

`ron-proto` may contain:

* QuickChain DTOs.
* Strict serde shapes.
* `deny_unknown_fields`.
* Canonical JSON v1 helpers.
* Domain separator constants and validation.
* Integer minor-unit string validation.
* b3 lowercase validation.
* Operation/hold/receipt DTO validation.
* Event-class vocabulary and conservative helper policy.
* File-backed locked-byte vectors.
* Locked-hash vector descriptors where the hash/preimage is test-vector data.
* Bash-only verification tools for vectors.
* Source-boundary tests.
* Tooling-boundary tests.

`ron-proto` must not contain:

* ledger mutation
* wallet mutation
* service calls
* async runtime behavior
* sockets
* HTTP clients or servers
* database reads/writes
* randomness
* wall-clock authority
* root production authority
* checkpoint production authority
* validator execution
* cryptographic verification authority
* bridge/anchor/settlement logic
* staking/liquidity/exchange-facing logic
* external-chain behavior
* fake receipts
* fake balances
* silent spend authority

## Remaining work for `ron-proto`

`ron-proto` is effectively green for the current pre-root phase, but later phases will need deliberate additions.

Deferred future work includes:

### 1. Locked root vector expansion

Before root-producing code exists, root vector contracts must be expanded and reviewed.

Future vector flow remains:

```text
sketch -> locked_bytes -> locked_hash
```

No fake hashes or placeholders should be introduced.

### 2. Canonical root payload DTO review

When Phase 1 root work begins, any new root-payload DTOs must go through:

* strict field review
* canonical JSON v1 byte review
* unknown-field rejection
* exact locked bytes
* explicit domain separator
* explicit preimage construction contract
* no runtime authority in `ron-proto`

### 3. Validator/signature DTO separation

Validator and signature DTOs may exist as descriptive wire shapes, but verification and committee behavior must remain outside `ron-proto`.

`ron-proto` may describe:

* validator ID shape
* public key reference shape
* signature reference shape
* quorum/committee metadata shape

`ron-proto` must not perform:

* validator set execution
* signature verification
* committee voting
* finality logic
* slashing
* staking
* public bridge behavior

### 4. Receipt/root/checkpoint schema hardening

Future receipt/root/checkpoint DTOs should keep the same rule:

* required identity fields must be explicit
* optional evidence fields must be structurally optional
* no default constructors
* no unknown fields
* no floats
* no client-assigned ledger authority

### 5. Keep tooling Bash-only

The preflight runner and vector verification tooling are Bash-only. Keep this invariant.

Do not add Python helper scripts to `ron-proto` QuickChain tooling unless the project explicitly reverses the current tooling policy.

## Files touched or relevant in this pass

Primary file touched:

```text
crates/ron-proto/tests/quickchain_source_boundary.rs
```

Important test/tooling files exercised:

```text
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-proto/tests/quickchain_canonical_json.rs
crates/ron-proto/tests/quickchain_chain_params.rs
crates/ron-proto/tests/quickchain_challenge_wire.rs
crates/ron-proto/tests/quickchain_domain_separation.rs
crates/ron-proto/tests/quickchain_dto_strict.rs
crates/ron-proto/tests/quickchain_empty_tree.rs
crates/ron-proto/tests/quickchain_event_class.rs
crates/ron-proto/tests/quickchain_file_vectors.rs
crates/ron-proto/tests/quickchain_hash_payloads.rs
crates/ron-proto/tests/quickchain_hold_scenarios.rs
crates/ron-proto/tests/quickchain_ids_and_money.rs
crates/ron-proto/tests/quickchain_locked_bytes.rs
crates/ron-proto/tests/quickchain_operation_hold.rs
crates/ron-proto/tests/quickchain_receipt_dto.rs
crates/ron-proto/tests/quickchain_receipt_order.rs
crates/ron-proto/tests/quickchain_replay_idempotency.rs
crates/ron-proto/tests/quickchain_sort_keys.rs
crates/ron-proto/tests/quickchain_source_boundary.rs
crates/ron-proto/tests/quickchain_tooling_boundary.rs
crates/ron-proto/tests/quickchain_vector_dto.rs
crates/ron-proto/tests/quickchain_vector_inventory.rs
crates/ron-proto/tests/tools/verify_quickchain_vector_inventory.sh
crates/ron-proto/tests/tools/verify_quickchain_hash_payloads.sh
```

## Summary

This pass made `ron-proto` stronger and less brittle.

The crate now has a better QuickChain source-boundary test that:

* prevents runtime authority from entering DTO code
* prevents implicit default constructors
* allows only structural optional/growth serde defaults
* avoids false positives on legitimate optional metadata
* keeps root/checkpoint/validator/settlement authority out of `ron-proto`

`ron-proto` is green for this QuickChain pre-root phase and ready to support the next crate pair, `svc-wallet + ron-accounting`.

# QuickChain QC-1A / Pre-Root Notes — `ron-ledger`

## Status

`ron-ledger` is green for the current QuickChain QC-1A / pre-root deterministic execution, replay, snapshot, and projection-hardening pass.

This crate remains the economic truth and deterministic transition boundary for QuickChain preflight work. It may execute internal deterministic ROC accounting rules under feature-gated QuickChain preflight tests, but it must not become a public chain runtime, validator, anchor, bridge, staking system, settlement layer, pruning system, or external-chain integration.

Current estimate for this crate in the QuickChain pre-root phase: approximately **98–99% complete**.

## What this pass confirmed

The latest pass confirmed that `ron-ledger` is green after the paired `ron-proto` boundary fix.

The focused canonical projection boundary passed:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Result:

```text
running 2 tests
test quickchain_pre_root_module_tree_does_not_claim_runtime_phase_authority ... ok
test quickchain_pre_root_ledger_sources_do_not_serialize_hash_or_build_preimages ... ok

test result: ok. 2 passed
```

The exhaustive preflight runner also passed:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final runner result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=27
```

This means `ron-ledger` is currently aligned with the pre-root boundary: deterministic execution and projection are allowed; root production, canonical hashing, validator authority, checkpoint authority, settlement, bridge, and anchor behavior remain forbidden.

## Main QuickChain role of `ron-ledger`

`ron-ledger` is the internal economic truth engine for ROC.

Its QuickChain preflight role is to prove:

* deterministic accepted operation execution
* atomic balance transitions
* atomic hold lifecycle transitions
* idempotency behavior
* duplicate operation rejection
* replay equivalence
* state snapshot determinism
* account sequence ownership by the ledger
* ledger sequence consistency
* hold capture/release/expiry semantics
* read-only projection into `ron-proto` hash-payload DTOs
* no root-producing behavior before the root phase is explicitly opened

It must not delegate balance truth to accounting, rewarder, gateway, omnigate, CrabLink, cache, or client code.

## What has been accomplished so far

### 1. Deterministic balance execution

The QuickChain balance transition tests prove deterministic handling of:

* issue
* transfer
* burn
* zero amount rejection
* missing amount rejection
* noncanonical amount rejection
* underflow rejection
* overflow rejection
* unauthorized issue rejection
* unauthorized burn rejection
* conservation checks
* rejection without partial mutation

Important invariant: failed transitions must not partially mutate state.

### 2. Atomic operation execution

The atomic execution tests prove that accepted operations update all related state together.

The crate now verifies:

* fresh issue commits balance identity and sequence together
* exact retry returns the original record
* idempotency conflict leaves state unchanged
* duplicate operation ID leaves state unchanged
* arithmetic rejection does not create identities or consume sequences
* invalid receipt references roll back computed transitions
* transfer advances participant sequences correctly
* self-transfer advances the actor sequence once
* identical ordered commands rebuild identical atomic state

Important invariant: operation identity, account sequence, primitive ledger sequence, receipt evidence, and balance mutation must stay atomic.

### 3. Hold lifecycle execution

The hold atomic execution tests prove deterministic handling of:

* hold open
* hold capture
* hold release
* partial capture
* expiry
* terminal holds
* reserved value blocking spend
* concurrent holds
* capture/release retry behavior
* invalid receipt and epoch rollback
* amount mismatch rollback
* counterparty mismatch rollback
* no reopen after terminal transition

Important invariant: hold state must not be resurrected, double-spent, retransitioned, or terminally mutated after close.

### 4. Accepted replay reconstruction

The accepted replay tests prove that previously accepted operations can rebuild deterministic state.

Covered behavior includes:

* mixed accepted history rebuilds identical complete state
* duplicate accepted records reject
* contradictory accepted history rejects through live transition rules
* mismatched committed sequences reject reconstruction
* supply decisions are preserved exactly
* epoch input is preserved exactly
* receipt transaction references are preserved exactly
* tampered receipt references reject
* sequence evidence mismatch rejects
* duplicate operation ID rejects without advancing replay boundary
* idempotency conflict rejects without advancing replay boundary
* retry after accepted rebuild returns the original record

Important invariant: accepted replay must use the same transition code as live acceptance and must reproduce committed evidence exactly.

### 5. Replay boundary hardening

The replay boundary tests prove that durable-history boundaries are checked before roots/checkpoints exist.

Covered behavior includes:

* empty accepted replay boundary accepts empty history
* chain-bound empty boundary rejects non-empty history
* truncated valid history rejects
* overextended valid history rejects
* next ledger sequence mismatch rejects
* wrong-chain accepted history rejects
* numeric boundary without chain ID remains a legacy shape-only check
* live terminal history round-trips through boundary reconstruction

Important invariant: before root production exists, replay boundaries still catch truncation, overextension, wrong-chain history, and sequence mismatch.

### 6. Chain binding

The chain-binding tests prove that the first committed QuickChain operation establishes the chain identity for the replay index.

Covered behavior includes:

* empty index binds to the first committed chain
* replay index rejects other chain without mutation
* atomic state rejects cross-chain issue before economic mutation

Important invariant: a replay index cannot silently mix operations from multiple chain identities.

### 7. Snapshot determinism

The state snapshot tests prove deterministic read-only snapshots.

Covered behavior includes:

* empty state snapshot is explicit and read-only
* snapshots sort accounts deterministically
* snapshots sort active holds deterministically
* terminal holds are excluded from active hold snapshots
* equivalent final states produce identical snapshots despite cross-account commit order

Important invariant: snapshots are deterministic views, not balance mutation paths and not roots.

### 8. Leaf/hash-payload projection boundaries

The projection tests prove that `ron-ledger` can project explicit state into `ron-proto` DTOs without computing hashes or roots.

Covered projection areas include:

* account leaf projection
* active hold leaf projection
* operation hash payload projection
* receipt hash payload projection
* terminal hold account projection
* projection replay equality
* projection context boundaries

The projections copy explicit context into DTOs and rely on `ron-proto` validation. They do not compute BLAKE3, do not construct hash preimages internally, do not canonicalize JSON internally, and do not produce roots.

Important invariant: projection is a read-only adapter seam. It is not root production.

### 9. Canonical projection boundary

The canonical projection boundary test confirms that pre-root ledger source does not cross into canonicalization or hash execution authority.

It prevents `ron-ledger` QuickChain pre-root modules from:

* serializing canonical JSON as a root-production step
* computing hashes
* building preimages as authority
* claiming checkpoint authority
* claiming validator authority
* claiming settlement authority
* claiming bridge/anchor authority
* depending on DB order, wall-clock order, service IO, or runtime IO for roots

This is one of the most important boundaries in the current pass.

### 10. Tooling boundary

The tooling boundary tests confirm that QuickChain preflight remains Bash-only and exhaustive.

Covered behavior includes:

* preflight runner is Bash-only
* test discovery count matches runner contract
* every QuickChain integration test is feature-gated
* no helper Python scripts exist in `ron-ledger`

This matches the current project policy for QuickChain patch delivery and local preflight execution.

## Current crate boundaries

`ron-ledger` may contain:

* deterministic internal ROC state transition logic
* checked integer arithmetic
* balance state
* hold state
* hold lifecycle transition logic
* replay index
* accepted replay reconstruction
* operation identity tracking
* idempotency tracking
* account sequence assignment
* primitive ledger sequence tracking
* deterministic read-only snapshots
* explicit projection into `ron-proto` hash-payload DTOs
* feature-gated QuickChain preflight tests
* Bash-only QuickChain preflight tooling

`ron-ledger` must not contain:

* public chain runtime behavior
* validator execution
* consensus voting
* public bridge logic
* public settlement logic
* staking
* liquidity
* exchange-facing logic
* external L1/L2/DA behavior
* root production without explicit root phase approval
* checkpoint production for QuickChain finality
* pruning
* anchor mutation
* rewarder payout mutation
* gateway/omnigate mutation shortcuts
* accounting-as-balance-truth
* fake balances
* fake receipts
* client-assigned account sequences
* DB-order roots
* wall-clock roots
* async network/service authority

## Important design decisions captured by this crate

### Operation ID vs idempotency key

`operation_id` is durable backend operation identity.

`idempotency_key` is a retry key.

They are not interchangeable.

The replay/index tests preserve this distinction and reject conflicting reuse.

### Account sequence is ledger-assigned

Clients must not assign `account_sequence`.

The crate rejects client-assigned account sequence before lookup or mutation.

### Hold ID identifies one lifecycle

A `hold_id` identifies one lifecycle.

A terminal hold cannot be reopened or retransitioned.

### Expiry is epoch-gated

Hold expiry depends on explicit epoch input.

The crate does not use wall-clock time as authority.

### Accounting is not balance truth

`ron-ledger` remains balance truth.

Accounting snapshots may summarize later, but cannot replace ledger truth.

### Rewarder plans but does not mutate

Rewarder payout planning remains external to ledger mutation.

`ron-ledger` must remain the mutation truth path, reached through the wallet/service boundary when appropriate.

## Test proof from this pass

The focused boundary passed:

```text
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Result:

```text
running 2 tests
test quickchain_pre_root_module_tree_does_not_claim_runtime_phase_authority ... ok
test quickchain_pre_root_ledger_sources_do_not_serialize_hash_or_build_preimages ... ok

test result: ok. 2 passed
```

The exhaustive runner passed:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

The runner completed all discovered QuickChain preflight tests and clippy.

Final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=27
```

Major test groups exercised include:

```text
quickchain_accepted_replay_auxiliary_inputs
quickchain_accepted_replay_boundary_extremes
quickchain_accepted_replay_boundary_identity_rejection
quickchain_accepted_replay_boundary_rejection_stability
quickchain_accepted_replay_boundary_retry_stability
quickchain_accepted_replay_boundary
quickchain_accepted_replay_evidence_tamper
quickchain_accepted_replay_sequence_evidence
quickchain_accepted_replay
quickchain_account_leaf_projection
quickchain_active_hold_leaf_projection
quickchain_atomic_execution
quickchain_balance_transition
quickchain_chain_binding
quickchain_hold_atomic_execution
quickchain_operation_hash_projection
quickchain_pre_root_boundary
quickchain_projection_canonical_boundary
quickchain_projection_context_boundaries
quickchain_projection_replay_equality
quickchain_receipt_hash_projection
quickchain_replay_index
quickchain_retry_after_accepted_rebuild
quickchain_state_snapshot
quickchain_terminal_hold_account_projection
quickchain_terminal_hold_replay_evidence
quickchain_tooling_boundary
```

Clippy also passed under the QuickChain preflight feature.

## Files relevant to this pass

Important source modules:

```text
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
crates/ron-ledger/src/quickchain/mod.rs
crates/ron-ledger/src/quickchain/replay_index.rs
crates/ron-ledger/src/quickchain/state_snapshot.rs
crates/ron-ledger/src/quickchain/transition.rs
crates/ron-ledger/src/quickchain/transition_error.rs
crates/ron-ledger/src/quickchain/types.rs
```

Important test/tooling files:

```text
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/tests/quickchain_accepted_replay_auxiliary_inputs.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_extremes.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_identity_rejection.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_rejection_stability.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary_retry_stability.rs
crates/ron-ledger/tests/quickchain_accepted_replay_boundary.rs
crates/ron-ledger/tests/quickchain_accepted_replay_evidence_tamper.rs
crates/ron-ledger/tests/quickchain_accepted_replay_sequence_evidence.rs
crates/ron-ledger/tests/quickchain_accepted_replay.rs
crates/ron-ledger/tests/quickchain_account_leaf_projection.rs
crates/ron-ledger/tests/quickchain_active_hold_leaf_projection.rs
crates/ron-ledger/tests/quickchain_atomic_execution.rs
crates/ron-ledger/tests/quickchain_balance_transition.rs
crates/ron-ledger/tests/quickchain_chain_binding.rs
crates/ron-ledger/tests/quickchain_hold_atomic_execution.rs
crates/ron-ledger/tests/quickchain_operation_hash_projection.rs
crates/ron-ledger/tests/quickchain_pre_root_boundary.rs
crates/ron-ledger/tests/quickchain_projection_canonical_boundary.rs
crates/ron-ledger/tests/quickchain_projection_context_boundaries.rs
crates/ron-ledger/tests/quickchain_projection_replay_equality.rs
crates/ron-ledger/tests/quickchain_receipt_hash_projection.rs
crates/ron-ledger/tests/quickchain_replay_index.rs
crates/ron-ledger/tests/quickchain_retry_after_accepted_rebuild.rs
crates/ron-ledger/tests/quickchain_state_snapshot.rs
crates/ron-ledger/tests/quickchain_terminal_hold_account_projection.rs
crates/ron-ledger/tests/quickchain_terminal_hold_replay_evidence.rs
crates/ron-ledger/tests/quickchain_tooling_boundary.rs
```

## Remaining work for `ron-ledger`

`ron-ledger` is effectively green for this pre-root phase, but several future phases remain deliberately deferred.

### 1. Root engine remains deferred

Do not add root-producing logic until canonical bytes and golden vectors are fully locked.

Root work must come after DTO/vector readiness.

Future state-tree work should follow the selected candidate:

```text
sorted binary Merkle map over canonical account-state leaves
```

No DB-order roots and no wall-clock roots.

### 2. Receipt root and operation root remain deferred

The current projection layer can build DTO payloads for future hashing, but it must not produce roots yet.

Future work must introduce:

* exact canonical bytes
* exact preimage bytes
* locked hashes
* replay equality
* golden vectors
* no placeholder/fake hashes

### 3. Checkpoint/finality remains deferred

No finality checkpoint behavior should be added yet.

Future checkpoint work must remain downstream of:

* canonical root vectors
* explicit root production module
* challenge/evidence model
* archive/DA fallback planning

### 4. Validator behavior remains deferred

No validators, committees, voting, quorum execution, slashing, staking, or public consensus behavior should enter `ron-ledger` yet.

Validator DTOs may be described in `ron-proto`, but execution authority remains deferred.

### 5. Pruning remains forbidden

No pruning until data availability, challenge, and archive fallback are designed and tested.

Current doctrine remains:

```text
proofs before pruning
```

### 6. External settlement remains forbidden

No ROX, Solana, external L2, public bridge, liquidity, staking, exchange-facing logic, or public settlement behavior should be added to `ron-ledger`.

Internal ROC must be proven first.

## Summary

This pass confirmed `ron-ledger` is green as the QuickChain deterministic economic truth layer for the current pre-root phase.

The crate proves:

* deterministic balance transitions
* atomic operation execution
* hold lifecycle correctness
* replay reconstruction
* replay boundary validation
* chain binding
* snapshot determinism
* read-only projection into `ron-proto`
* no canonical hash/root production in pre-root modules
* no runtime phase authority
* Bash-only exhaustive preflight tooling

`ron-ledger` is ready to support the next crate pair, `svc-wallet + ron-accounting`, where wallet mutation boundaries and accounting snapshot non-authority need to be carried forward without weakening ledger truth.


### END NOTE - JUNE 18 2026 - 20:10 CST


### BEGIN NOTE - JUNE 19 2026 - 21:55 CST

Use this as the crate note for both `ron-proto` and `ron-ledger`. The latest terminal output shows the focused tree-material tests passed, the canonical-boundary scanner passed, and the full `ron-ledger` QuickChain preflight finished green with `tests=28`. 

# QuickChain QC-1A Crate Notes — `ron-proto` + `ron-ledger`

## Session Scope

This session advanced QuickChain from QC-0A safety/preflight into the first QC-1A Phase 1 preparation slice for the core backend crates:

1. `ron-proto`
2. `ron-ledger`

The goal was not to produce roots, checkpoints, validators, anchors, public chain state, settlement artifacts, or any live chain authority. The goal was to establish a safe Phase 1 input-material boundary:

* `ron-proto` owns strict DTO shape, canonical vocabulary, schema names, validation rules, and vector-facing material structures.
* `ron-ledger` owns deterministic ledger truth and can project/assemble sorted input material, but must not become the canonicalization, hashing, root, proof, checkpoint, validator, or settlement layer.
* Tree-material preparation is allowed because it is pre-root, inert, deterministic, and DTO/projection-only.
* Root-producing work remains blocked until locked canonical bytes, locked hashes, and independent vectors exist.

This batch was intentionally narrow and architecture-preserving.

---

## High-Level Result

This QC-1A batch is green for `ron-proto + ron-ledger`.

Validated outcomes:

* `ron-proto` QuickChain preflight passed with `tests=22`.
* `ron-ledger` QuickChain preflight passed with `tests=28`.
* `cargo clippy` passed under the ledger QuickChain preflight gate.
* New `ron-proto` tree-material DTO tests passed.
* New `ron-ledger` tree-material projection tests passed.
* The `quickchain_projection_canonical_boundary` scanner passed after removing direct canonical-vocabulary ownership from ledger source.
* No root production was introduced.
* No live checkpoint/root authority was introduced.
* No validator/settlement/anchor/bridge/staking/liquidity code was introduced.
* No public blockchain runtime was introduced.
* No wallet/service mutation authority was introduced into `ron-ledger` projection code.
* No fake balances, fake receipts, or silent spend paths were introduced.

This is a clean Phase 1 preparation checkpoint, not a full Phase 1 completion.

---

## `ron-proto` Changes

### New Purpose Added

`ron-proto` now has the strict DTO surface for QuickChain tree-material preparation.

This is the first safe bridge from “we have validated receipts/accounts/holds/hash payloads” toward “we can later build deterministic tree/root vectors,” without actually producing roots yet.

### New File

Added:

```text
crates/ron-proto/src/quickchain/root_material.rs
```

This file defines DTO-only tree-material structures and validation helpers.

### Module Wiring

Updated:

```text
crates/ron-proto/src/quickchain/mod.rs
```

Expected module/export additions include:

```rust
pub mod root_material;
pub use root_material::*;
```

The purpose is to expose the tree-material DTOs through the existing `ron_proto::quickchain::*` surface.

---

## `ron-proto` Tree-Material DTOs

The new `root_material.rs` module provides strict Phase 1 material DTOs.

Core constants introduced:

```text
QUICKCHAIN_TREE_MATERIAL_ITEM_SCHEMA
QUICKCHAIN_TREE_MATERIAL_BATCH_SCHEMA
QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTEWISE_ASCENDING_V1
MAX_QUICKCHAIN_TREE_MATERIAL_SORT_KEY_HEX_BYTES
MAX_QUICKCHAIN_TREE_MATERIAL_PAYLOAD_SCHEMA_BYTES
MAX_QUICKCHAIN_TREE_MATERIAL_SORT_RULE_BYTES
MAX_QUICKCHAIN_TREE_MATERIAL_ITEMS
```

Core enum introduced:

```rust
QuickChainTreeMaterialKindV1
```

Current tree-material kinds:

```text
State
Holds
Receipts
Accounting
Rewards
```

Core DTO introduced:

```rust
QuickChainTreeMaterialItemV1
```

The item DTO binds:

```text
schema
version
tree
sort_key_hex
payload_schema
payload_hash
```

Core batch DTO introduced:

```rust
QuickChainTreeMaterialBatchV1
```

The batch DTO binds:

```text
schema
version
chain_id
epoch_id
tree
canonical_encoding
item_sort_rule
items
```

The batch explicitly requires canonical JSON v1 through the DTO boundary, but this remains owned by `ron-proto`, not by `ron-ledger`.

---

## `ron-proto` Validation Rules Added

The new DTO validation enforces:

* Correct schema token.
* Correct DTO version.
* Valid `chain_id`.
* Valid `epoch_id`.
* Valid canonical encoding.
* Valid item sort rule.
* Bounded item count.
* Bounded sort key length.
* Lowercase even-length hex sort keys.
* Strictly ascending item sort keys.
* No duplicate sort keys.
* Valid tree/material schema pairing.
* Valid `b3:<64 lowercase hex>` payload commitments.
* Unknown-field rejection through serde tests.
* Payload schemas must be known/allowed for their target tree.
* Tree kind must match batch tree.
* Batch-level tree and item-level tree cannot drift.

This is important because it prevents Phase 1 from later building roots over ambiguous, unordered, unknown, or schema-mismatched material.

---

## `ron-proto` Helper Added

Added a helper in `root_material.rs`:

```rust
quickchain_tree_material_json_v1_encoding()
```

Purpose:

* Keeps the canonical encoding vocabulary in `ron-proto`.
* Allows downstream crates to request the currently allowed tree-material encoding without directly owning or naming canonicalization policy in their own source.
* Prevents `ron-ledger` from tripping source-boundary checks that ban ledger-side canonicalization/root-preimage ownership.

This helper became necessary after the `ron-ledger` canonical-boundary scanner correctly rejected direct ledger usage of `QuickChainCanonicalEncodingV1`.

---

## `ron-proto` Tests Added

Added:

```text
crates/ron-proto/tests/quickchain_root_material.rs
```

This test suite covers:

1. A sorted state-material batch validates.
2. Unsorted or duplicate sort keys reject.
3. Uppercase or odd-length hex sort keys reject.
4. Payload schema / tree mismatch rejects.
5. Tree-material DTOs reject unknown fields.

Focused test result:

```text
cargo test -p ron-proto --test quickchain_root_material
```

Result:

```text
5 passed
```

The full `ron-proto` QuickChain preflight also passed:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
```

---

## `ron-ledger` Changes

### New Purpose Added

`ron-ledger` now has a narrow, pre-root tree-material projection adapter.

The adapter takes caller-supplied material commitments, sorts them deterministically, and assembles a strict `ron-proto` tree-material batch.

It does not compute roots.

It does not compute hashes.

It does not serialize canonical bytes.

It does not build preimages.

It does not mutate ledger state.

It does not create receipts.

It does not become service authority.

It does not decide settlement/finality.

This keeps `ron-ledger` aligned with its role as deterministic internal truth/projection logic, not as a QuickChain chain runtime.

### New File

Added:

```text
crates/ron-ledger/src/quickchain/tree_material_projection.rs
```

### Module Wiring

Updated:

```text
crates/ron-ledger/src/quickchain/mod.rs
```

Expected additions:

```rust
mod tree_material_projection;

pub use tree_material_projection::{
    build_tree_material_batch,
    QuickChainTreeMaterialProjectionError,
    QuickChainTreeMaterialProjectionItem,
};
```

The re-export is required so integration tests and later safe preflight consumers can access the projection adapter.

---

## `ron-ledger` Tree-Material Projection API

New struct:

```rust
QuickChainTreeMaterialProjectionItem
```

Fields:

```text
sort_key_hex
payload_schema
payload_hash
```

Constructor:

```rust
QuickChainTreeMaterialProjectionItem::new(...)
```

Getters:

```rust
sort_key_hex()
payload_schema()
payload_hash()
```

New error enum:

```rust
QuickChainTreeMaterialProjectionError
```

Current variants:

```text
DuplicateSortKey
InvalidBatch
```

New builder function:

```rust
build_tree_material_batch(...)
```

Current intent:

* Accept unordered caller-supplied material items.
* Sort by bytewise ascending lowercase hex sort key.
* Reject duplicate sort keys before returning a batch.
* Build a strict `QuickChainTreeMaterialBatchV1`.
* Delegate final validation to the `ron-proto` DTO boundary.
* Return an inert, validated DTO batch ready for later vector work.

---

## Important Boundary Correction

An early version of the ledger projection file directly imported and used:

```rust
QuickChainCanonicalEncodingV1
```

That caused the broader ledger source-boundary test to fail because `ron-ledger` source is not allowed to own canonicalization vocabulary or root/preimage policy.

The fix was architectural, not cosmetic:

* Moved the canonical encoding choice behind a `ron-proto` helper:

  ```rust
  quickchain_tree_material_json_v1_encoding()
  ```
* Removed direct `QuickChainCanonicalEncodingV1` naming from `ron-ledger` projection source.
* Kept canonical DTO vocabulary in `ron-proto`.
* Kept `ron-ledger` as a deterministic sorting/projection adapter only.
* Repaired the ledger integration test imports so the helper is imported from `ron_proto::quickchain`, not from `ron_ledger::quickchain`.

This was the correct fix because it reinforced the project boundary instead of weakening the scanner.

---

## `ron-ledger` Tests Added

Added:

```text
crates/ron-ledger/tests/quickchain_phase1_tree_material.rs
```

This test suite covers:

1. Unordered account material is sorted into deterministic Phase 1 input order.
2. Duplicate material sort keys reject before a batch is returned.
3. Schema/tree mismatch rejects at the `ron-proto` DTO boundary.

Focused command:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase1_tree_material
```

Result:

```text
3 passed
```

The canonical-boundary scanner was also rerun:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Result:

```text
2 passed
```

Full ledger gate:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

## Why This Batch Matters

This batch is important because QuickChain Phase 1 cannot safely begin with root-producing code.

Before any root can exist, the system needs:

1. Strict input material DTOs.
2. Deterministic sort rules.
3. Explicit tree kinds.
4. Explicit schema/tree pairing.
5. Bounded input sizes.
6. Unknown-field rejection.
7. Cross-crate boundary enforcement.
8. Tests proving ledger does not take over canonicalization/root authority.

This session established those Phase 1 prerequisites without violating the blueprint.

The result is a safe “tree material staging layer”:

```text
ron-ledger deterministic state/projection
        ↓
explicit material commitments + sort keys
        ↓
ron-proto strict tree-material DTO batch
        ↓
future locked canonical byte vectors
        ↓
future locked hash vectors
        ↓
future root work
```

No root-producing step was added in this batch.

---

## Current Crate Status

### `ron-proto`

Status:

```text
Green for QC-1A entry batch.
```

What is complete in this slice:

* Tree-material DTOs exist.
* Tree-material validation exists.
* Tree-material unknown-field rejection exists.
* Tree-material schema/tree matching exists.
* Tree-material sort rule validation exists.
* Tree-material helper keeps canonical vocabulary in `ron-proto`.
* QuickChain preflight passed with `tests=22`.

What remains later:

* Locked canonical byte vectors for tree-material batches.
* Vector inventory updates for tree-material files.
* Independent verifier coverage for tree-material vectors.
* Locked hash vectors after locked bytes are reviewed.
* No fake/placeholder hashes.
* No root-producing code until vector gates are ready.

### `ron-ledger`

Status:

```text
Green for QC-1A entry batch.
```

What is complete in this slice:

* Deterministic tree-material projection adapter exists.
* Unordered input sorting is tested.
* Duplicate sort-key rejection is tested.
* DTO-boundary validation handoff is tested.
* Canonicalization/root source-boundary scanner passes.
* Full ledger QuickChain preflight passed with `tests=28`.

What remains later:

* Additional projection equality tests from live/replay state into material batches.
* Material-batch tests for holds, receipts, accounting, and rewards when those are ready.
* No root-producing code yet.
* No direct canonical byte serialization in ledger source.
* No hash/preimage construction in ledger source.
* No service or wallet mutation authority.

---

## Current QuickChain Completion Estimate After This Batch

Approximate status:

```text
QC-0A / Phase 0 safety-preflight:        ~98–100% complete
QC-1A / Phase 1 prep:                    ~12–18% complete
Phase 1 root-producing prototype:         ~5–10% complete
Full long-term QuickChain blueprint:      ~33–37% complete
```

This session moved QuickChain across the line from “Phase 0 safety/preflight only” into “Phase 1 preparation,” but it did not make Phase 1 root production ready yet.

---

## What Not To Do Next In These Crates

Do not add any of the following yet:

```text
- root-producing code
- checkpoint-producing code
- validator code
- committee code
- staking code
- liquidity code
- bridge code
- external settlement code
- public blockchain runtime code
- gateway/omnigate/rewarder ledger mutation
- raw engagement payout mutation
- DB-order roots
- wall-clock roots
- fake hashes
- placeholder hashes
- unreviewed canonical bytes
- unreviewed preimage formats
```

Do not move canonicalization ownership into `ron-ledger`.

Do not let `ron-ledger` serialize canonical bytes for root material.

Do not let `ron-ledger` build hash preimages.

Do not weaken the boundary scanners to make tests pass.

The correct pattern is:

```text
ron-proto owns DTO/canonical/vector vocabulary.
ron-ledger owns deterministic economic state/projection.
Future vector gates lock exact bytes and hashes before roots.
```

---

## Commands That Passed

Focused `ron-proto` tree-material test:

```bash
cargo test -p ron-proto --test quickchain_root_material
```

Focused `ron-ledger` tree-material projection test:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase1_tree_material
```

Focused `ron-ledger` canonical-boundary test:

```bash
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_projection_canonical_boundary
```

Full `ron-proto` preflight gate:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
```

Expected/observed final result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
```

Full `ron-ledger` preflight gate:

```bash
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Observed final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

## Codebundle Regeneration

After the green gates, regenerate focused codebundles:

```bash
bash scripts/make_crate_codex.sh --force -c ron-proto
bash scripts/make_crate_codex.sh --force -c ron-ledger
```

These regenerated bundles should include:

For `ron-proto`:

```text
crates/ron-proto/src/quickchain/root_material.rs
crates/ron-proto/tests/quickchain_root_material.rs
```

For `ron-ledger`:

```text
crates/ron-ledger/src/quickchain/tree_material_projection.rs
crates/ron-ledger/tests/quickchain_phase1_tree_material.rs
```

---

## Recommended Next Crate Pair

The next crate pair should be:

```text
svc-wallet + ron-accounting
```

Reason:

```text
ron-proto now defines strict DTO material.
ron-ledger now provides deterministic internal truth/projection.
svc-wallet is the mutation front-door.
ron-accounting provides reporting/snapshot views but must never become balance truth.
```

The next work should prove that wallet/accounting can participate in QC-1A preparation without becoming QuickChain authority.

Expected next goals:

```text
- svc-wallet remains the explicit mutation front-door.
- wallet receipts remain backend-derived.
- no fake balances.
- no fake receipts.
- no silent spend.
- no direct QuickChain settlement authority.
- ron-accounting remains snapshot/reporting only.
- accounting never becomes balance truth.
- accounting never mutates ledger state.
- source-boundary tests block roots/validators/settlement creep.
- any material preparation remains inert and vector-facing only.
```

---

## Carry-Forward Warning

This was a successful QC-1A preparation slice, but it is still early Phase 1.

The next major safe milestone inside Phase 1 is not “build roots immediately.” The next safe milestone is:

```text
tree-material locked canonical byte vectors
        ↓
bash/vector inventory verification
        ↓
independent expected bytes review
        ↓
locked hash vectors
        ↓
only then root-prep discussion
```

The doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

This session honored that doctrine.


### END NOTE - JUNE 19 2026 - 21:55 CST




### BEGIN NOTE - JUNE 21 2026 - 19:35 CST

Your terminal output confirms both crate preflights are green: `ron-proto` passed its exhaustive QuickChain preflight with **22 tests**, and `ron-ledger` passed with **28 tests**. 

Here are the paste-ready crate notes.

# QuickChain Phase 1 Crate Notes — `ron-proto` + `ron-ledger`

## Session Summary

This session completed a safe QuickChain Phase 1 pre-root preparation slice for the core protocol and ledger crates:

* `crates/ron-proto`
* `crates/ron-ledger`

The work added and validated deterministic tree-material and first-layer reduction-plan preparation without crossing into forbidden Phase 1 authority. The patch keeps QuickChain inside the approved QC-1A boundary: DTOs, canonical material ordering, validation, replayable projections, and reduction planning only.

This session did **not** introduce root-producing code, checkpoint authority, validators, pruning, settlement, external anchors, public bridge logic, staking, liquidity, live chain state, CrabLink authority, service mutation authority, or public blockchain behavior.

The core outcome is that `ron-proto` can now describe validated tree material and adjacent-pair reduction plans, while `ron-ledger` can deterministically project ledger state into that material and prepare the first reduction layer. This gives the project a clean, audited pre-root staging surface before any actual deterministic root implementation is allowed.

---

## Phase / Scope

Current QuickChain phase context:

* Phase 0 — QC-0A safety/preflight: already green.
* Phase 1 — single-authority deterministic roots/proofs: in progress.
* Current session scope: QC-1A pre-root preparation.
* Current session result: `ron-proto` + `ron-ledger` are green for this pre-root/root-material preparation slice.

This work advances Phase 1, but it does not complete full Phase 1 by itself. Full Phase 1 still requires the later root/proof implementation once canonical bytes, locked vectors, and review gates explicitly authorize root-producing code.

---

## Safety Doctrine Preserved

The patch stayed aligned with the QuickChain doctrine:

* Determinism before distribution.
* DTOs before roots.
* Roots before validators.
* Proofs before pruning.
* Internal ROC before external anchors.
* Accounting is not balance truth.
* Rewarder plans payouts but never mutates ledger.
* Wallet/ledger remain the economic mutation authority.
* CrabLink remains a client/display layer, not chain authority.

The patch also preserved the current prohibition against:

* Root production.
* Checkpoint production.
* Validator implementation.
* Committee logic.
* Pruning.
* Settlement.
* External anchors.
* Public bridges.
* Staking or liquidity.
* Solana/ROX integration.
* Gateway/omnigate/rewarder ledger mutation.
* Raw engagement protocol payouts.
* DB-order or wall-clock-derived roots.
* Unknown consensus fields.
* Fake hashes or placeholder hashes.

---

# `ron-proto` Notes

## Purpose of the `ron-proto` Changes

`ron-proto` owns the strict DTO layer. In this session, it was extended so QuickChain can represent deterministic tree material and first-layer reduction plans as pure data.

The key idea is that `ron-proto` can describe the shape of material that will later feed root construction, but it still does not compute roots, hashes, signatures, proofs, settlement artifacts, or authority decisions.

The new DTOs make future root work safer because the input material and reduction plan are now explicit, versioned, validated, and test-gated before any root-producing code exists.

---

## Primary `ron-proto` File Changed

### `crates/ron-proto/src/quickchain/root_material.rs`

This file now contains the validated protocol surface for:

* Tree material items.
* Tree material batches.
* Tree material sort rules.
* Tree material tree kinds.
* Tree-material canonical JSON encoding.
* First-layer tree reduction pairs.
* First-layer tree reduction plans.
* Adjacent-pair reduction rule validation.
* Source item count validation.
* Odd final carry validation.

---

## Existing Tree Material DTOs Preserved

The existing tree-material DTOs remain the base of the pre-root pipeline:

### `QuickChainTreeMaterialItemV1`

Represents one already-prepared leaf/hash-payload material item.

Important fields:

* `schema`
* `version`
* `tree`
* `sort_key_hex`
* `payload_schema`
* `payload_hash`

Important validation behavior:

* Schema must match `quickchain.tree-material-item.v1`.
* Version must be `1`.
* Sort key must be lowercase even-length hex.
* Payload hash must be canonical `b3:<64 lowercase hex>`.
* Payload schema must match the declared tree type.

This DTO is still material only. It does not compute a payload hash and does not compute a tree root.

### `QuickChainTreeMaterialBatchV1`

Represents a sorted batch of material items for one tree.

Important fields:

* `schema`
* `version`
* `chain_id`
* `epoch_id`
* `tree`
* `canonical_encoding`
* `sort_rule`
* `items`

Important validation behavior:

* Schema must match `quickchain.tree-material-batch.v1`.
* Version must be `1`.
* `chain_id` and `epoch_id` must be bounded, non-empty QuickChain IDs.
* Canonical encoding must be `json_v1`.
* Sort rule must be `sort_key_hex_ascending_v1`.
* All items must match the batch tree.
* All items must be sorted by `sort_key_hex`.
* Duplicate sort keys are rejected.
* Batch size is bounded.

This is the validated pre-root input list.

---

## New `ron-proto` Constants Added

The following constants were added for the reduction-plan DTO layer:

```rust
pub const QUICKCHAIN_TREE_REDUCTION_PAIR_SCHEMA: &str = "quickchain.tree-reduction-pair.v1";
pub const QUICKCHAIN_TREE_REDUCTION_PLAN_SCHEMA: &str = "quickchain.tree-reduction-plan.v1";
pub const QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_ADJACENT_WITH_ODD_CARRY_V1: &str =
    "adjacent_pairs_with_odd_carry_v1";
pub const MAX_QUICKCHAIN_TREE_REDUCTION_PAIR_RULE_BYTES: usize = 128;
```

These make the reduction-plan layer explicit and versioned.

---

## New DTO: `QuickChainTreeReductionPairV1`

A reduction pair describes one first-layer adjacent pair from sorted tree material.

Important fields:

* `schema`
* `version`
* `tree`
* `layer_index`
* `pair_index`
* `left_sort_key_hex`
* `left_payload_hash`
* `right_sort_key_hex`
* `right_payload_hash`

Important validation behavior:

* Schema must match `quickchain.tree-reduction-pair.v1`.
* Version must be `1`.
* `layer_index` must be `0`.
* Left sort key must be valid lowercase even-length hex.
* Left payload hash must be canonical `b3:<64 lowercase hex>`.
* Right member fields must be both present or both absent.
* If right member exists, right sort key must be greater than left sort key.
* A pair may omit the right member only to represent the final odd carry.

This DTO intentionally does not hash the pair. It only describes which material items will later be reduced together.

---

## New DTO: `QuickChainTreeReductionPlanV1`

A reduction plan describes the complete first reduction layer for one material batch.

Important fields:

* `schema`
* `version`
* `chain_id`
* `epoch_id`
* `tree`
* `canonical_encoding`
* `item_sort_rule`
* `pair_rule`
* `source_items_count`
* `pairs`

Important validation behavior:

* Schema must match `quickchain.tree-reduction-plan.v1`.
* Version must be `1`.
* `chain_id` and `epoch_id` must be valid bounded QuickChain IDs.
* Canonical encoding must be `json_v1`.
* Item sort rule must be `sort_key_hex_ascending_v1`.
* Pair rule must be `adjacent_pairs_with_odd_carry_v1`.
* Source item count must fit within the material-item cap.
* Pair count must equal `source_items_count.div_ceil(2)`.
* Pair indexes must be contiguous.
* Pair tree must match the plan tree.
* Pair order must be deterministic.
* Non-final odd carries are rejected.
* Counted source items must equal `source_items_count`.

This gives future root code an explicit, validated reduction schedule instead of allowing implicit runtime ordering.

---

## Clippy Fix Applied in `ron-proto`

Clippy rejected manual division-ceiling logic:

```rust
((source_items_count + 1) / 2) as usize
```

This was replaced with:

```rust
source_items_count.div_ceil(2) as usize
```

This is behavior-preserving and only satisfies the `manual_div_ceil` lint under `#![deny(warnings)]`.

---

## `ron-proto` Tests Updated

### `crates/ron-proto/tests/quickchain_root_material.rs`

This test now covers both the original material DTOs and the new reduction-plan DTOs.

Validated behavior includes:

* Sorted state material batch validation.
* Rejection of uppercase or odd-length hex sort keys.
* Rejection of unsorted sort keys.
* Rejection of duplicate sort keys.
* Rejection of payload-schema/tree mismatch.
* Valid adjacent-pair reduction plan with final odd carry.
* Rejection of partial right-member fields.
* Rejection of reversed right-member ordering.
* Rejection of non-final carry.
* Rejection of pair-count mismatch.
* Unknown-field rejection for both material and reduction DTOs.

The test count for this file is now 8 tests.

---

## `ron-proto` Exhaustive Preflight Result

The exhaustive QuickChain preflight for `ron-proto` passed.

Confirmed categories included:

* Canonical JSON.
* Chain params.
* Challenge wire DTOs.
* Domain separation.
* Strict DTO validation.
* Empty tree DTOs.
* Event classes.
* File-backed locked vectors.
* Hash payload vectors.
* Hold scenarios.
* IDs and money.
* Locked bytes.
* Hold operation DTOs.
* Receipt DTOs.
* Receipt ordering.
* Replay/idempotency.
* Root material and reduction DTOs.
* Sort keys.
* Source boundary.
* Tooling boundary.
* Vector DTOs.
* Vector inventory.
* Bash vector inventory verifier.
* Bash hash payload verifier.
* Clippy preflight.

Final result:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
```

---

# `ron-ledger` Notes

## Purpose of the `ron-ledger` Changes

`ron-ledger` owns deterministic economic truth and replayable state reconstruction. In this session, it was extended to project deterministic state material into the new `ron-proto` tree-material and reduction-plan DTOs.

The ledger still does not compute roots in this QuickChain path. It only prepares validated, sorted material and a first-layer reduction schedule from already-produced payload hashes.

This keeps ledger work inside the approved pre-root boundary.

---

## Primary `ron-ledger` File Changed

### `crates/ron-ledger/src/quickchain/tree_material_projection.rs`

This file now provides deterministic projection from ledger-side material into `ron-proto` tree-material and reduction-plan DTOs.

---

## Existing Ledger Projection Preserved

### `QuickChainTreeMaterialProjectionItem`

This remains the ledger-side input item for building a protocol material batch.

Important fields:

* `sort_key_hex`
* `payload_schema`
* `payload_hash`

This is not a root or proof item. It is the ledger-side adapter item used to create a validated protocol material batch.

### `build_tree_material_batch`

This function takes:

* `chain_id`
* `epoch_id`
* `tree`
* unordered projection items

and returns a validated `QuickChainTreeMaterialBatchV1`.

Important behavior:

* Sorts items deterministically by sort key.
* Converts ledger projection items into `ron-proto` material items.
* Validates the final batch through `ron-proto`.
* Rejects duplicate sort keys.
* Rejects invalid payload schema/tree combinations.
* Rejects invalid hashes or invalid sort keys.
* Produces no root and no proof.

---

## New Ledger Function Added

### `build_tree_reduction_plan_from_batch`

This function takes a validated `QuickChainTreeMaterialBatchV1` and returns a validated `QuickChainTreeReductionPlanV1`.

Important behavior:

* Validates the source material batch first.
* Preserves the batch chain ID.
* Preserves the batch epoch ID.
* Preserves the batch tree kind.
* Preserves canonical encoding.
* Preserves item sort rule.
* Uses the explicit pair rule `adjacent_pairs_with_odd_carry_v1`.
* Chunks sorted material items into adjacent pairs.
* Creates `QuickChainTreeReductionPairV1` values.
* Allows only the final odd item to carry without a right member.
* Validates the final plan through `ron-proto`.
* Returns an error if validation fails.
* Does not compute roots.
* Does not compute hashes.
* Does not verify proofs.
* Does not read or write storage.
* Does not use runtime authority.
* Does not use wall-clock time.
* Does not mutate ledger state.

This is a pre-root adapter only.

---

## `ron-ledger` Public QuickChain Export Updated

### `crates/ron-ledger/src/quickchain/mod.rs`

The new `build_tree_reduction_plan_from_batch` function is exported through the gated QuickChain module.

This keeps the API available only under the existing `quickchain-preflight` feature boundary.

---

## Clippy Fix Applied in `ron-ledger`

The ledger projection used manual capacity calculation for pair count:

```rust
Vec::with_capacity((batch.items.len() + 1) / 2)
```

This was changed to:

```rust
Vec::with_capacity(batch.items.len().div_ceil(2))
```

This is behavior-preserving and keeps the crate clean under `-D warnings`.

---

## `ron-ledger` Tests Updated

### `crates/ron-ledger/tests/quickchain_phase1_tree_material.rs`

This test now covers both material-batch projection and reduction-plan construction.

Validated behavior includes:

* Unordered account material is sorted into deterministic Phase 1 input order.
* Duplicate material sort keys reject before batch return.
* Schema/tree mismatch rejects at the `ron-proto` boundary.
* Valid material batch builds first adjacent reduction plan with odd final carry.
* Empty material batch builds an empty reduction plan.
* Invalid material batch rejects before a reduction plan is returned.

The test count for this file is now 6 tests.

---

## Boundary Tests Preserved

The `ron-ledger` QuickChain source boundary remains green.

Most importantly:

### `quickchain_pre_root_boundary`

This test continues to prove the gated QuickChain ledger preflight sources do not gain runtime authority or root-production behavior.

The boundary continues to block forbidden concepts such as:

* Root-producing implementations in this pre-root path.
* Runtime authority.
* Network behavior.
* Storage authority.
* Clocks or randomness.
* Validator logic.
* Unauthorized consensus behavior.
* Hidden service behavior.

### `quickchain_projection_canonical_boundary`

The projection boundary also remains green and continues to ensure pre-root ledger sources do not serialize hash material or build preimages in the wrong place.

---

## `ron-ledger` Exhaustive Preflight Result

The exhaustive QuickChain preflight for `ron-ledger` passed.

Confirmed categories included:

* Accepted replay.
* Accepted replay auxiliary inputs.
* Accepted replay boundaries.
* Replay boundary extremes.
* Replay boundary identity rejection.
* Replay boundary rejection stability.
* Replay boundary retry stability.
* Replay evidence tamper rejection.
* Replay sequence evidence.
* Account leaf projection.
* Active hold leaf projection.
* Atomic execution.
* Balance transitions.
* Chain binding.
* Hold atomic execution.
* Operation hash projection.
* Phase 1 tree material projection.
* Pre-root source boundary.
* Projection canonical boundary.
* Projection context boundaries.
* Projection replay equality.
* Receipt hash projection.
* Replay index.
* Retry after accepted rebuild.
* State snapshot.
* Terminal hold account projection.
* Terminal hold replay evidence.
* Tooling boundary.
* Clippy preflight.

Final result:

```text
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

# What This Session Proves

This session proves that the core QuickChain Phase 1 material pipeline is now deterministic and pre-root safe across `ron-proto` and `ron-ledger`.

Specifically, the system can now:

1. Represent tree material as strict DTOs.
2. Validate tree material schemas and versions.
3. Validate canonical encoding.
4. Validate tree kind and payload-schema consistency.
5. Validate lowercase hex sort keys.
6. Validate canonical `b3:<64 lowercase hex>` payload hashes.
7. Validate sorted unique material batches.
8. Deterministically sort ledger-projected material.
9. Reject duplicate sort keys.
10. Reject schema/tree mismatches.
11. Build a first-layer adjacent reduction plan.
12. Carry a final odd item honestly.
13. Reject non-final odd carries.
14. Reject malformed right-member pairs.
15. Reject pair-count mismatches.
16. Preserve chain and epoch binding.
17. Preserve canonical encoding and sort rule binding.
18. Keep all projection work read-only.
19. Keep all reduction planning non-authoritative.
20. Keep root production out of scope.

This creates a safer launchpad for the eventual deterministic root implementation because root code will not get to invent ordering, pairing, or material semantics ad hoc. Those are now explicit and test-gated.

---

# What This Session Does Not Claim

This session does not claim:

* A state root exists.
* A receipt root exists.
* A holds root exists.
* A rewards root exists.
* A checkpoint is produced.
* A proof is generated.
* A proof is verified.
* Validators exist.
* A committee exists.
* QuickChain is live.
* CrabLink has chain authority.
* Gateway or omnigate can mutate ledger state.
* Rewarder can allocate ROC.
* Any external settlement path exists.
* Any ROX/Solana/public-chain bridge exists.
* Any staking or liquidity path exists.
* Any pruning path exists.

This distinction matters because the current implementation is intentionally pre-root and pre-authority.

---

# Architectural Impact

## Before This Session

The project already had:

* Strict QuickChain DTOs.
* Canonical JSON v1.
* Locked bytes vectors.
* Locked hash payload vectors.
* Sort-key validation.
* Operation/replay/idempotency tests.
* Atomic ledger execution.
* Hold lifecycle logic.
* Account and active-hold projections.
* Accepted replay boundaries.
* Source/tooling boundaries.

But the project still needed a more explicit pre-root bridge from sorted leaf material into a future tree-reduction process.

## After This Session

The project now has:

* Strict tree-material batch DTOs.
* Strict tree-reduction pair DTOs.
* Strict tree-reduction plan DTOs.
* Deterministic first-layer adjacent pairing.
* Explicit odd-carry handling.
* Ledger adapter from projected material to reduction plan.
* Tests proving both empty and non-empty reduction plans.
* Tests proving invalid material is rejected before reduction plan return.
* Clippy-clean implementation.
* Exhaustive preflight green for both crates.

This is a meaningful Phase 1 step because future root code can consume a validated plan instead of relying on hidden runtime ordering.

---

# Commands That Passed

Focused commands that passed:

```bash
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo clippy -p ron-ledger --all-targets --features quickchain-preflight -- -D warnings

cargo test -p ron-proto --test quickchain_root_material
cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase1_tree_material
```

Exhaustive preflight commands that passed:

```bash
crates/ron-proto/scripts/dev-quickchain-preflight.sh
crates/ron-ledger/scripts/dev-quickchain-preflight.sh
```

Final exhaustive results:

```text
ron-proto quickchain exhaustive preflight gate passed: tests=22
ron-ledger quickchain exhaustive preflight gate passed: tests=28
```

---

# Files Touched

## `ron-proto`

Primary file:

```text
crates/ron-proto/src/quickchain/root_material.rs
```

Primary test:

```text
crates/ron-proto/tests/quickchain_root_material.rs
```

## `ron-ledger`

Primary file:

```text
crates/ron-ledger/src/quickchain/tree_material_projection.rs
```

Public gated module export:

```text
crates/ron-ledger/src/quickchain/mod.rs
```

Primary test:

```text
crates/ron-ledger/tests/quickchain_phase1_tree_material.rs
```

---

# Current Completion Status for This Crate Pair

`ron-proto` and `ron-ledger` are green for this Phase 1 pre-root/root-material preparation slice.

Status:

```text
ron-proto:  green
ron-ledger: green
focused tests: green
exhaustive preflight: green
clippy: green
```

This crate pair should not need more work right now unless the next session intentionally adds the next authorized Phase 1 root-vector/root-material step.

---

# Recommended Next Crate Pair

The next Phase 1 crate pair should be:

```text
svc-wallet + ron-accounting
```

Reason:

* `ron-proto` and `ron-ledger` now define and prove the deterministic pre-root material surface.
* `svc-wallet` is the mutation front-door and must remain the only service-facing path for issue/transfer/burn/hold/capture/release/receipt behavior.
* `ron-accounting` must remain snapshot/read-model only and must not become balance truth.
* The next pass should verify these crates do not accidentally treat QuickChain material, receipts, snapshots, or future root references as authority.

The next session should continue preserving the same boundary:

* No root production in service crates.
* No ledger mutation outside wallet/ledger.
* No fake receipts.
* No fake balances.
* No silent spend.
* No rewarder/gateway/omnigate mutation authority.
* No external settlement creep.
* No ROX/Solana/bridge/staking/liquidity code.

---

# Carry-Forward Notes for Future Phase 1 Work

The next root-related step, when authorized, should still proceed carefully.

Before actual root production is introduced, the project should require:

1. Locked canonical bytes for any new root input DTO.
2. Locked vectors for reduction-pair canonical bytes.
3. Locked vectors for reduction-pair hash preimages.
4. Locked vectors for odd-carry behavior.
5. Locked vectors for empty tree behavior.
6. Locked vectors for one-item tree behavior.
7. Locked vectors for two-item tree behavior.
8. Locked vectors for three-item tree behavior.
9. Locked vectors for multi-layer reduction behavior.
10. Explicit domain separator for each future hash context.
11. No placeholder hashes.
12. No DB-order dependence.
13. No wall-clock dependence.
14. No service-runtime dependence.
15. No hidden consensus fields.
16. No validator or checkpoint code until root vectors are locked.

The safe next evolution from this session would be vector planning for reduction-pair canonical bytes and reduction-pair hash payloads, still without producing a live root.

---

# One-Sentence Summary

This session completed and greened the `ron-proto` + `ron-ledger` QuickChain Phase 1 pre-root material/reduction-plan slice: the project can now deterministically validate, sort, batch, and first-layer pair future root material without producing roots or crossing into chain authority.

### END NOTE - JUNE 21 2026 - 19:35 CST



### BEGIN NOTE - JUNE 22 2026 - 16:05 CST

The latest exhaustive preflight confirms both crates are green: `ron-proto` passed `tests=23`, and `ron-ledger` passed `tests=29`, including the new root/proof tests and clippy preflights. 

Paste this into your crate notes:

# QuickChain Phase 1 Round 2 Crate Notes — `ron-proto` + `ron-ledger`

## Status

`ron-proto` and `ron-ledger` are now **Phase 1 complete** for QuickChain.

This completes the two intended Phase 1 rounds for the core pair:

* **Round 1:** deterministic DTOs, replay/idempotency shape, hash payloads, leaf projection contracts, canonical boundary hardening, pre-root safety.
* **Round 2:** deterministic tree root material, tree root DTOs, inclusion proof DTOs, deterministic root construction, inclusion proof construction/verification, and exhaustive boundary validation.

These crates should now be treated as **locked for Phase 1** unless a downstream crate pair exposes a real integration mismatch.

Overall QuickChain Phase 1 is **not complete yet** until the remaining Round 2 crate pairs pass, but the `ron-proto + ron-ledger` crate pair is complete for Phase 1.

---

# `ron-proto` Phase 1 Completion Notes

## What changed

`ron-proto` now owns the strict QuickChain wire/DTO contract for deterministic Phase 1 tree material, tree roots, and inclusion proofs.

The crate remains a DTO/schema/vector crate only. It does **not** compute roots, verify roots, mutate ledger state, claim consensus, produce validators, anchor externally, or define settlement authority.

## New or completed Phase 1 DTO surface

The QuickChain DTO layer now covers:

* Tree material batches.
* Material sort keys.
* Reduction plans.
* Tree leaf nodes.
* Tree branch nodes.
* Tree root payloads.
* Tree root DTOs.
* Inclusion proof steps.
* Inclusion proof DTOs.
* Tree algorithm constants.
* Domain separator helpers for tree-specific root/proof material.
* Strict schema/version validation.
* Strict tree/domain/encoding/algorithm validation.
* Strict unknown-field rejection.

The inclusion proof DTOs validate proof shape without assigning authority to the proof. A proof is treated as deterministic evidence tied to a root DTO, not as finality, consensus, or settlement.

## Important Phase 1 invariants now represented in `ron-proto`

`ron-proto` now freezes these core rules:

* Canonical JSON v1 remains the locked encoding.
* Domain-separated preimage rules remain explicit.
* BLAKE3 hashes are lowercase `b3:<64 hex>`.
* Tree identities are explicit and must match across root, leaf, proof, and domain fields.
* Root/proof fields reject unknown fields.
* Empty tree material is explicit and not represented with fake hashes.
* Source items must be sorted by canonical bytewise sort keys before root construction.
* Duplicate sort keys reject.
* Inclusion proof steps are bounded.
* Inclusion proof step layers must be unique and ordered.
* Single-item tree proofs have a strict no-step shape.
* Multi-item tree proofs must bind leaf index, source item count, root metadata, and leaf node metadata.

## Tests passed

The exhaustive `ron-proto` QuickChain preflight passed with `tests=23`.

Important Round 2 tests included:

* `quickchain_root_material`
* `quickchain_tree_proof`
* `quickchain_vector_inventory`
* `quickchain_hash_payloads`
* `quickchain_source_boundary`
* `quickchain_tooling_boundary`

The preflight also verified:

* QuickChain vector inventory.
* Locked byte vectors.
* Locked hash vectors.
* Bash-only tooling.
* Hash payload vectors.
* Clippy preflight.

## Boundary status

`ron-proto` remains correctly scoped:

* DTOs only.
* Validation only.
* Canonical/vector preparation only.
* No runtime ledger authority.
* No wallet authority.
* No validator authority.
* No settlement authority.
* No pruning.
* No external anchors.
* No public bridge.
* No ROX/Solana integration.
* No fake hashes/placeholders.

## Result

`ron-proto` is Phase 1 complete.

It now provides the strict DTO contract required by `ron-ledger` and downstream crates for deterministic Phase 1 root/proof work.

---

# `ron-ledger` Phase 1 Completion Notes

## What changed

`ron-ledger` now implements the deterministic Phase 1 root/proof slice over validated `ron-proto` material.

The crate remains the economic truth boundary for deterministic internal ROC state, but the new QuickChain tree work is still scoped to the gated `quickchain-preflight` feature and remains internal deterministic infrastructure. It does not create validators, public chain state, settlement, external anchors, pruning, or bridge logic.

## New or completed Phase 1 root/proof behavior

`ron-ledger` now supports:

* Deterministic tree material batch construction.
* Deterministic material sorting.
* Duplicate sort-key rejection.
* Root construction from validated material batches.
* Reduction plan construction.
* Odd-carry tree layer behavior.
* Empty tree root behavior using explicit empty payload hash.
* Inclusion proof construction for a requested material sort key.
* Inclusion proof verification against a root DTO.
* Tamper rejection for leaf commitments.
* Tamper rejection for sibling hashes.
* Metadata mismatch rejection between root and proof.
* Missing sort-key rejection before proof return.
* Strict conversion of `ron-proto` validation failures into bounded ledger errors.

## Important implementation boundaries

The root/proof work is intentionally narrow:

* Roots are deterministic local artifacts, not validator outputs.
* Proofs are deterministic local evidence, not finality.
* No chain authority is granted to clients, services, or proofs.
* No gateway or omnigate mutation authority is introduced.
* No accounting truth is introduced.
* No reward payout mutation is introduced.
* No external settlement hooks exist.
* No anchors exist.
* No pruning exists.
* No consensus committee exists.
* No public blockchain runtime exists.
* No ROX/Solana/public bridge logic exists.

The ledger computes deterministic tree material only from explicit validated inputs. It does not silently derive policy, receipt, clock, validator, settlement, or external-chain context.

## New error handling

`QuickChainTreeMaterialProjectionError` now includes proof/root-specific failures for:

* Missing material sort keys.
* Invalid proof DTOs.
* Failed proof verification.
* Invalid material/root DTOs.
* Duplicate sort keys.
* Invalid canonical bytes.
* Invalid computed hash.

These errors keep failure modes bounded and deterministic while avoiding raw stack traces, secrets, or authority-bearing messages.

## Tests passed

The exhaustive `ron-ledger` QuickChain preflight passed with `tests=29`.

Important Round 2 tests included:

* `quickchain_phase1_tree_material`
* `quickchain_phase1_tree_proof`
* `quickchain_pre_root_boundary`
* `quickchain_projection_canonical_boundary`
* `quickchain_tooling_boundary`

Focused Round 2 proof/root coverage passed:

* 7/7 inclusion proof tests.
* 11/11 tree material/root tests.
* 1/1 pre-root boundary test.
* 2/2 projection canonical boundary tests.

The full preflight also passed clippy with `-D warnings`.

## Specific proof/root behaviors now proven

The test suite proves:

* Two-leaf state root vector matches a real locked BLAKE3 hash.
* Three-leaf odd-carry tree root verifies correctly.
* Caller input order does not affect deterministic root output.
* Leaf commitment changes alter the root.
* Invalid material batches reject before root return.
* Invalid proof DTOs reject before success.
* Missing material sort key rejects before proof return.
* Tampered sibling hash rejects verification.
* Tampered leaf commitment rejects verification.
* Mismatched root/proof metadata rejects verification.
* Middle-leaf proof paths verify left/right sibling ordering correctly.
* Boundary scanners still prevent accidental runtime authority claims.

## Boundary status

`ron-ledger` remains correctly scoped:

* Ledger owns deterministic ROC truth.
* QuickChain root/proof logic is gated and deterministic.
* No validators.
* No checkpoints as consensus/finality.
* No external anchors.
* No pruning.
* No bridge.
* No settlement.
* No staking.
* No liquidity.
* No public chain authority.
* No wall-clock roots.
* No DB-order roots.
* No fake receipts or fake balances.

## Result

`ron-ledger` is Phase 1 complete.

It now consumes the `ron-proto` DTO contract and implements the deterministic root/proof construction and verification slice required for Phase 1.

---

# Pair-Level Result

The `ron-proto + ron-ledger` core pair is now complete for QuickChain Phase 1.

Completed pair responsibilities:

* DTO contract.
* Canonical JSON v1 boundary.
* Root material DTOs.
* Inclusion proof DTOs.
* Deterministic tree root construction.
* Deterministic inclusion proof build/verify.
* Real BLAKE3 hashes.
* Strict validation.
* Unknown-field rejection.
* Deterministic ordering.
* Empty tree handling.
* Boundary scanners.
* Bash-only tooling.
* Exhaustive crate-local preflights.
* Clippy preflight.

This pair should now be considered locked for Phase 1 unless downstream Round 2 work reveals a real integration mismatch.

---

# Next Phase 1 Round 2 Pair

The next crate pair should be:

`svc-wallet + ron-accounting`

Goal for the next pair:

* Ensure `svc-wallet` remains the mutation front door.
* Ensure `ron-accounting` remains snapshot/reporting, not balance truth.
* Confirm downstream crates do not reinterpret roots/proofs as spend authority.
* Confirm no fake receipt, fake balance, fake unlock, settlement, bridge, or external-chain authority is introduced.
* Keep QuickChain roots/proofs as deterministic evidence only.


### END NOTE - JUNE 22 2026 - 16:05 CST