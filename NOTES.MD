Below are **five separate paste-ready crate notes**, tailored for:

```text
ron-ledger
ron-accounting
svc-rewarder
svc-wallet
svc-storage
```

The current green sweep proves `ron-accounting`, `svc-rewarder`, `svc-wallet`, and `svc-storage` are clean on their requested gates, including `ron-accounting` with and without `wal`, `svc-rewarder`’s wallet loop, `svc-wallet`’s escrow/paid-storage smoke, and `svc-storage`’s paid-write enforcement/verifier tests. 

---

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

# 2. NOTE — `ron-accounting`

## Current Status

`ron-accounting` is now the measurement, aggregation, projection, and reward-snapshot preparation layer for the ROC value plane.

Its role is:

```text
usage events → normalized counters → sealed accounting slices/windows → reward projection → canonical reward snapshot → rewarder input
```

The latest full sweep proved `ron-accounting` passes both normal and WAL-enabled test suites. The normal run passed the property tests, loom placeholders, unit tests, interop vector tests, reward projection tests, reward snapshot tests, rollover tests, and benches. The WAL-enabled run also passed, including WAL append/drain and corrupt-record skip behavior. 

## What We Accomplished

### 1. Accounting Usage Event Ingest Is Active and Proven

`ron-accounting` now accepts service usage events and records them into normalized accounting counters.

The tested event forms include:

```text
bytes_stored
bytes_served
request_ok
pin_seconds
custom normalized request metrics
```

The tests prove:

```text
- zero timestamp is rejected
- empty subject is rejected
- zero-value events are skipped/no-op
- bytes_stored records into the bytes dimension
- bytes_served records into GET bytes
- source_service attribution works
- batch ingest reports inspected/recorded/skipped counts
```

### 2. Reward Projection Is Working

`ron-accounting` can project raw recorded usage into reward-oriented contribution shape.

The projection tests prove:

```text
- grouping by tenant/service/region works
- PUT bytes map to bytes_stored
- GET bytes map to bytes_served
- uptime/request rows can map to uptime_seconds
- unmapped request rows are ignored
- projection CID is stable even when input slice order changes
- overflowing aggregates reject safely
```

This matters because `svc-rewarder` should not invent contribution data. It should consume a sealed accounting snapshot.

### 3. Stable Reward Snapshot Interop Vector Exists

The interop vector is one of the most important outcomes of this phase.

It gives `svc-rewarder` a known-good accounting snapshot with:

```text
canonical JSON
canonical snapshot CID
known contribution count
known total score
stable rewarder-compatible shape
```

The current vector is used directly by `svc-rewarder` tests. This gives us a hard compatibility contract between accounting and rewarder.

### 4. Sealing and Recording Are Proven

Tests now prove:

```text
- recording is monotone and saturating
- sealing drains only the requested stream
- sealing commits a digest
- rollover happens once at boundary crossing
- windows align to UTC boundary
```

This is important because reward epochs depend on deterministic accounting windows.

### 5. WAL Mode Is Proven

The WAL-enabled test run now proves:

```text
- WAL appends and drains
- WAL replay skips corrupt records
- normal no-WAL build still works
```

That gives us a path toward more durable accounting without forcing every dev run to use persistence.

## How `ron-accounting` Now Integrates With Other Crates

### With `svc-storage`

`svc-storage` now emits accounting-compatible usage event DTOs from `/paid/o`.

The paid storage response includes events such as:

```text
bytes_stored
request_ok
pin_seconds
```

The `svc-storage` integration test parses those events as `ron-accounting::UsageEvent` and feeds them into `record_usage_events`, proving the storage → accounting event bridge works. The current full sweep shows this integrated test passing. 

### With `svc-rewarder`

`svc-rewarder` consumes the `ron-accounting` reward snapshot interop vector and validates:

```text
snapshot shape
canonical CID
expected score total
wallet issue preview shape
```

This proves the accounting → rewarder bridge.

## What Remains for `ron-accounting`

### 1. Real Service Event Exporters

Right now `svc-storage` proves event compatibility in-process. The next production step is to decide how events move in real deployments:

```text
HTTP ingest endpoint
local bus event
WAL file export/import
batch sidecar
direct crate API in monolith/micronode mode
```

Remaining work:

```text
- define production event ingestion path
- add auth/capability rules for event submitters
- add rate limits and replay/idempotency behavior
- add tests for duplicate event behavior
```

### 2. Storage Revenue Attribution

`svc-storage` now emits usage. We still need to decide how accounting maps usage into provider rewards and/or service revenue:

```text
bytes_stored → provider contribution
bytes_served → provider contribution
pin_seconds → storage/pinning contribution
request_ok → reliability/availability signal
```

Remaining work:

```text
- define exact reward projection weights
- define per-service route mapping policy
- decide how paid writes translate to provider rewards vs treasury revenue
```

### 3. Snapshot Production API

The interop vector is stable, but production needs a real “seal and emit reward snapshot” workflow:

```text
seal accounting window
project contributions
create canonical snapshot
compute snapshot CID
publish snapshot reference to rewarder
```

Remaining work:

```text
- add CLI or API for producing reward snapshots
- define storage location for snapshot artifacts
- define retry/replay behavior
- add end-to-end test with generated snapshot rather than static vector
```

### 4. WAL Production Decisions

WAL mode works. Remaining decisions:

```text
- when WAL is enabled by default
- where WAL files live
- retention/compaction policy
- corruption alerting
- replay at startup
```

### 5. Dashboard/Admin Visibility

Accounting needs dashboard-friendly outputs:

```text
usage by service
usage by tenant
provider contribution score
storage bytes / served bytes / pin seconds
epoch snapshot status
projection CID
```

## Recommended Next Steps for `ron-accounting`

```text
1. Add a production event ingress/export path.
2. Add storage-paid-write event source tests beyond in-process DTO parsing.
3. Add a snapshot generation command/API.
4. Add dashboard summary DTOs.
5. Keep the interop vector stable as a contract test fixture.
```

## Completion Estimate

```text
ron-accounting for internal ROC beta: ~88–92%
ron-accounting for production-grade accounting service: ~72–80%
```

The core accounting math and interop contracts are strong. The remaining work is mostly production wiring, event transport, persistence policy, and dashboard/export surfaces.

---

# 3. NOTE — `svc-rewarder`

## Current Status

`svc-rewarder` is now the deterministic reward computation and payout-planning layer for ROC.

Its role is:

```text
sealed accounting snapshot → deterministic reward manifest → settlement intent / wallet issue batch
```

The latest sweep proves `svc-rewarder` is clippy clean and test clean. It has:

```text
1 lib test
9 integration tests
38 unit tests
reward_calc bench smoke success
```

The integration tests include the full rewarder → wallet issue loop and the sealed accounting CID enforcement. 

## What We Accomplished

### 1. `ron-accounting` Interop Vector Is Consumed by `svc-rewarder`

We added the accounting interop test that consumes the live `ron-accounting` reward snapshot vector.

This proves:

```text
- accounting canonical JSON parses as rewarder AccountingSnapshot
- accounting snapshot CID matches rewarder canonical CID
- contribution count is stable
- expected total score is stable
- rewarder payout math is deterministic
- wallet issue preview shape is compatible with svc-wallet
```

This was the first major bridge from accounting into rewarder.

### 2. `inputs_cid` Enforcement Is Now Real

`svc-rewarder` now enforces:

```text
inputs_cid == canonical_snapshot_cid(snapshot)
```

This moved the snapshot integrity rule from “documented/tested” into production compute behavior.

The test suite proves:

```text
- matching snapshot CID is accepted
- mismatched inputs_cid is rejected
- canonicalization is deterministic after sorting
- duplicate accounts reject
- uppercase/malformed policy hashes reject
```

This prevents a caller from submitting one snapshot while claiming another CID.

### 3. Deterministic Manifest and Settlement Planning Are Proven

The rewarder computes:

```text
run_key
manifest commitment
payout list
residual amount
wallet issue batch
settlement preview
idempotency keys
```

The tests prove:

```text
- same input produces deterministic output
- dry-run can promote to production without consuming run key
- settlement preview returns wallet issue batch
- settlement intent egress is idempotent by run key
- metrics include planned settlement intents after compute
```

### 4. First Closed-Loop Rewarder → Wallet Proof Is Green

The `web3_roc_loop` integration test now proves:

```text
ron-accounting vector
→ svc-rewarder manifest
→ wallet issue requests
→ svc-wallet HTTP mutation path
→ ron-ledger-backed balances
→ idempotent replay without double issue
```

This is a major value-plane milestone. It proves reward issuance is not merely theoretical.

## How `svc-rewarder` Fits the Current System

Current flow:

```text
ron-accounting produces sealed reward snapshot
svc-rewarder verifies snapshot CID
svc-rewarder computes payouts
svc-rewarder emits wallet issue requests
svc-wallet commits issue operations through ledger
```

Important boundary:

```text
svc-rewarder does not mutate ledger directly.
svc-wallet remains the mutation front-door.
```

## What Remains for `svc-rewarder`

### 1. Production Wallet Client

The current wallet loop is in-process/test/dev. Production needs a real wallet client path:

```text
POST /v1/issue
Authorization/capability
Idempotency-Key
timeout/retry policy
receipt verification
partial failure handling
```

Remaining work:

```text
- implement outbound wallet issue client
- test retry and idempotent replay
- handle wallet unavailable/unready
- add metrics for wallet submit success/failure/replay
```

### 2. Accounting Snapshot Fetch Adapter

Currently snapshots are inline for deterministic dev/testing. Production should support:

```text
inputs_cid reference
fetch snapshot from ron-accounting or CAS
verify canonical CID
compute only after exact CID match
```

Remaining work:

```text
- add accounting snapshot fetch adapter
- preserve inline mode for tests/dev
- add negative tests for fetch mismatch/corrupt snapshot
```

### 3. Signed Policy / Governance

Reward policy exists in deterministic form, but production needs governance:

```text
approved policy IDs
policy hash registry
signed reward policy documents
policy activation/deactivation
```

Remaining work:

```text
- define reward policy registry
- add signed policy verification if needed
- dashboard current policy and hash
```

### 4. Artifact Persistence and Audit Trail

Rewarder should persist or emit:

```text
reward manifest
settlement batch
wallet issue receipts
run status
quarantine reason
```

Remaining work:

```text
- finalize artifact directory/schema
- ensure amnesia mode suppresses persistence correctly
- add replay/recovery tests for production artifacts
```

### 5. Dashboard/Admin Visibility

Needed dashboard panels:

```text
epoch ID
input snapshot CID
policy hash
total pool
payout total
residual
number of payouts
settlement status
wallet issue status
quarantine count
```

## Recommended Next Steps for `svc-rewarder`

```text
1. Add production wallet client behind a trait.
2. Keep current dev/in-process wallet loop as a test harness.
3. Add snapshot fetch adapter.
4. Add signed/approved policy registry.
5. Add manifest/settlement audit export.
```

## Completion Estimate

```text
svc-rewarder for internal ROC beta: ~88–92%
svc-rewarder for production-grade reward service: ~72–82%
```

The deterministic compute core and wallet compatibility are strong. The main unfinished work is production networking, artifact/audit persistence, and governance around reward policy.

---

# 4. NOTE — `svc-wallet`

## Current Status

`svc-wallet` is now the mutation boundary for the internal ROC economy.

Its role is:

```text
all ROC balance mutations must go through wallet
wallet commits through ron-ledger
wallet protects mutations with auth, idempotency, nonce rules, policy checks, and receipts
```

The latest sweep proves `svc-wallet` is clippy clean and test clean. The run includes:

```text
29 lib tests
HTTP blackbox tests
escrow HTTP tests
transport bounds tests
overflow ceiling tests
hold/capture/release invariant tests
double-spend tests
non-negativity tests
conservation tests
capability/policy tests
DTO hygiene tests
ledger primacy tests
observability tests
amnesia tests
loom invariants
readiness/shedding tests
receipt hash vectors
paid-storage smoke
bench smokes
```

All passed. 

## What We Accomplished

### 1. Ledger-Safe Escrow Core

We added wallet escrow primitives:

```text
hold
capture
release
```

These are implemented as ledger-backed balanced movements:

```text
hold:    payer  → escrow
capture: escrow → service/provider
release: escrow → payer/refund target
```

Core tests prove:

```text
- hold reserves funds into escrow
- capture pays service from escrow
- release refunds remainder to payer
- total value is conserved
- capture above held amount rejects
- failed capture does not credit payee
```

### 2. Escrow HTTP Endpoints

We exposed:

```text
POST /v1/hold
POST /v1/capture
POST /v1/release
```

These endpoints reuse the wallet’s mutation discipline:

```text
bearer auth
capability scope
idempotency key
nonce reservation
rollback on failed ledger commit
receipt storage
metrics
ledger-backed balances
```

HTTP escrow tests prove:

```text
- hold/capture/release works through public wallet API
- idempotent hold replay returns same receipt
- over-capture rejects without crediting payee
- duplicate nonce hold rejects
- balances remain correct after failure
- metrics expose hold/capture/release
```

### 3. Paid-Storage State Machine Smoke

We added the wallet-level paid-storage smoke test.

It proves:

```text
0 ROC payer cannot reserve paid storage
funded payer can reserve max estimate
hold reserves estimate into escrow
capture pays actual cost to svc_storage
release refunds unused remainder
final total value is conserved
metrics expose issue/hold/capture/release/reject
```

This was the first wallet-side proof that paid storage/pinning can be economically enforced.

### 4. Rewarder Issue Loop Is Proven

Through `svc-rewarder` integration, wallet now receives rewarder-generated issue requests and commits them through its HTTP API.

The test proves:

```text
rewarder-generated wallet issue requests are accepted
balances increase for recipients
idempotent replay does not double issue
wallet metrics reflect issue/replay behavior
```

### 5. Receipt Shape Now Matters to `svc-storage`

`svc-storage` now has a `WalletReceipt` contract that can parse wallet-style receipts and validate whether a receipt is a valid paid-write hold proof.

This creates a future production path:

```text
svc-storage /paid/o
→ WalletReceiptVerifier
→ lookup receipt from svc-wallet
→ validate hold/roc/positive/payer/escrow/b3 hash
→ allow CAS write
```

## What Remains for `svc-wallet`

### 1. Dedicated Hold Object / Hold ID Model

Current escrow endpoints use transfer-like requests directly:

```text
from
to
amount
nonce
memo
```

Production paid storage may want a richer hold object:

```text
hold_id
payer
escrow
intended_service
max_amount
expires_at
context/cid/route
status = held|captured|released|expired
```

Remaining work:

```text
- decide whether hold IDs are needed before beta
- add GET /v1/hold/:hold_id if needed
- add terminal-state rules
- prevent capture/release after terminal state
```

### 2. Receipt Lookup Contract

`svc-storage` now expects to eventually verify wallet receipts. Wallet should ensure `/v1/tx/:txid` or equivalent returns a stable receipt shape.

Remaining work:

```text
- confirm receipt lookup route shape
- ensure receipts tolerate future fields
- ensure receipt_hash is stable
- add storage-compatible receipt lookup tests
```

### 3. Production Auth/Caps

The dev tests use `Bearer dev`. Production needs real capability/macaroon handling for:

```text
issue
transfer
burn
hold
capture
release
receipt lookup
balance reads
```

Remaining work:

```text
- define scopes for hold/capture/release
- decide whether receipt lookup is public, account-scoped, or service-scoped
- add caveats for service/payment context
```

### 4. Durable Wallet State

Wallet currently has strong in-memory/dev/amnesia behavior. Production must decide:

```text
durable idempotency store
durable receipt book
durable nonce table
ledger persistence
expiration/compaction policy
```

Remaining work:

```text
- define beta durability mode
- test restart/replay behavior
- make receipt lookup survive restart if beta requires it
```

### 5. Service-to-Service Client Support

Other services need stable clients for:

```text
issue
hold
capture
release
receipt lookup
balance check
```

Remaining work:

```text
- add client DTO module or SDK
- add retry/timeout conventions
- add idempotency key helpers
```

## Recommended Next Steps for `svc-wallet`

```text
1. Stabilize receipt lookup route and DTO.
2. Add tests proving receipt lookup returns storage-compatible hold receipts.
3. Decide whether hold_id state machine is needed before beta.
4. Add production capability scopes for escrow operations.
5. Add durable receipt/idempotency/nonce policy.
```

## Completion Estimate

```text
svc-wallet for internal ROC beta: ~90–94%
svc-wallet for production-grade wallet service: ~75–85%
```

Wallet is now the strongest and most mature piece of the ROC value plane. The remaining work is mostly production durability, receipt lookup hardening, richer hold lifecycle, and real auth/caps.

---

# 5. NOTE — `svc-storage`

## Current Status

`svc-storage` is now both a working CAS service and the first storage-side WEB3 enforcement point.

Its roles are now:

```text
free/dev CAS object store
paid ROC-gated CAS write endpoint
accounting-compatible usage event emitter
paid-write metrics producer
future wallet receipt verifier host
```

The latest sweep proves `svc-storage` is clippy clean and test clean. It now has:

```text
http_blackbox.rs:          1 passed
paid_write_policy.rs:     5 passed
paid_write_verifier.rs:   11 passed
web3_paid_storage_loop.rs: 1 passed
bench targets compile
```

All passed. 

The original storage notes defined the crate as a content-addressed storage service with BLAKE3 `b3:<hex>` identifiers, free CAS HTTP routes, health/ready, metrics, range GET, and amnesia profile.  The docs also identify CAS invariants, byte-range correctness, `/metrics`, `/healthz`, `/readyz`, and capability-only writes as part of the intended storage contract. 

## What We Accomplished

### 1. Clean `svc-storage` Baseline Restored

We first removed unused direct path dependencies on `ron-kernel` and `ron-metrics` from `svc-storage` so clippy could reach the crate itself.

This was necessary because `svc-storage` currently uses its own local minimal Prometheus route and does not yet need kernel/metrics integration.

Result:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings ✅
cargo test -p svc-storage --all-targets ✅
```

### 2. Replaced Flaky TCP Spawn Test With In-Process Router Test

The old `http_blackbox.rs` spawned `cargo run -p svc-storage`, which caused:

```text
clippy::zombie_processes
flaky startup timing
port/process issues
```

We replaced it with an in-process Axum router test.

Now `http_blackbox.rs` proves:

```text
POST /o stores bytes and returns cid
HEAD /o/:cid returns metadata headers
GET /o/:cid returns original bytes
Range GET returns exact slice and Content-Range
unknown CID returns 404
bad CID path returns 404
/metrics is mounted when enabled
```

This gives a stable CAS baseline without spawning a child process.

### 3. Added Dedicated Paid Write Route

We added:

```text
POST /paid/o
PUT  /paid/o
```

The existing free/dev route remains unchanged:

```text
POST /o
PUT  /o
HEAD /o/:cid
GET  /o/:cid
```

This means:

```text
/o       = free/dev CAS path
/paid/o  = ROC-paid write path
```

This avoids breaking existing developer workflows while giving the value plane an enforcement seam.

### 4. Paid Write Proof Enforcement

`/paid/o` now requires wallet hold proof metadata.

It rejects before writing when:

```text
proof is missing
operation is not hold
asset is not roc
estimate is zero
estimate is non-integer
receipt hash is not b3:<64 lowercase hex>
payer is missing
escrow is missing
```

It also rejects malformed accounting context:

```text
tenant is zero
tenant is non-integer
pin_seconds is non-integer
```

The `paid_write_policy.rs` tests prove these failures do not store the object.

### 5. Paid Write Usage Events

Successful `/paid/o` responses now include accounting-compatible usage events:

```text
bytes_stored
request_ok
pin_seconds, when x-ron-pin-seconds > 0
```

The cross-crate paid storage loop parses these as `ron-accounting::UsageEvent` and ingests them through `record_usage_events`.

This proves:

```text
svc-storage paid write
→ usage_events
→ ron-accounting ingest
→ recorded accounting rows
```

### 6. Paid Write Metrics

We added local Prometheus metrics:

```text
storage_paid_write_total{status="accepted"}
storage_paid_write_total{status="payment_required"}
storage_paid_write_total{status="bad_accounting_context"}
storage_paid_write_total{status="storage_error"}
storage_paid_write_bytes_total
```

The paid write metrics test proves these series are exposed through `/metrics`.

### 7. Cross-Crate Paid Storage Loop

`web3_paid_storage_loop.rs` now proves the full dev loop:

```text
unfunded wallet hold rejects
unfunded storage write does not create object
funded payer receives ROC
wallet hold reserves ROC into escrow
/paid/o accepts hold proof and stores object
GET /o/:cid returns exact object bytes
storage emits accounting-compatible usage events
ron-accounting ingests those events
wallet capture pays svc_storage
wallet release refunds payer
final wallet value is conserved
```

This is the strongest proof so far that the value plane is not just crate-local.

### 8. Paid Write Verifier Seam

We refactored trusted proof parsing out of `paid_object.rs` into:

```text
policy::paid_write
```

New components:

```text
PaidWriteVerifier trait
DevHeaderVerifier
WalletReceiptVerifier
WalletReceipt DTO
PaidWriteProof
VerifiedPaidWrite
PaidWriteVerificationError
```

Current behavior:

```text
/paid/o → DevHeaderVerifier → CAS write
```

Future behavior:

```text
/paid/o → WalletReceiptVerifier → svc-wallet receipt lookup → CAS write
```

`WalletReceiptVerifier` currently fails closed until the real lookup adapter is wired.

### 9. Wallet Receipt Contract

`WalletReceipt` now validates the receipt shape storage will require from wallet.

It accepts only:

```text
op = hold
asset = roc
amount_minor = positive integer
from = payer present
to = escrow present
receipt_hash = b3:<64 lowercase hex>
```

Tests prove it rejects:

```text
wrong operation
wrong asset
zero amount
non-integer amount
missing payer
missing escrow
bad receipt hash
```

It also tolerates future wallet fields during JSON deserialization, which protects storage against additive wallet receipt changes.

## What Remains for `svc-storage`

### 1. Real Wallet Receipt Verification

Current `/paid/o` uses `DevHeaderVerifier`. This is correct for dev/test, but production must not trust headers blindly.

Remaining work:

```text
- add wallet receipt lookup client trait
- add mock client tests
- implement real GET /v1/tx/:txid lookup when ready
- verify returned receipt matches paid write proof
- ensure mismatched txid/receipt/account/amount rejects
```

Recommended progression:

```text
Step A: WalletReceiptLookup trait + MockWalletReceiptLookup
Step B: WalletReceiptVerifier validates looked-up receipt
Step C: HTTP client implementation
Step D: route config chooses DevHeaderVerifier or WalletReceiptVerifier
```

### 2. Paid Route Configuration

We need a mode switch:

```text
dev mode       /paid/o uses DevHeaderVerifier
production     /paid/o uses WalletReceiptVerifier
disabled       /paid/o maybe 404/403
```

Remaining work:

```text
- add config enum for paid_write_verifier_mode
- add tests for fail-closed production default if desired
- decide beta default
```

### 3. Stronger Proof Binding

Eventually, the paid proof should bind to more than “some wallet hold happened.”

Possible future binding fields:

```text
expected CID
payer
escrow
service = svc-storage
route = /paid/o
max bytes or price estimate
expiry timestamp
receipt hash
idempotency key
```

Remaining work:

```text
- define hold context/memo schema
- ensure wallet hold receipt includes context
- verify context in storage before write
```

### 4. Actual Capture/Release Automation

Current loop does:

```text
wallet hold
storage write
manual/test capture
manual/test release
```

Production may need storage or a billing coordinator to trigger:

```text
capture actual cost
release remainder
emit final settlement event
```

Remaining work:

```text
- decide whether svc-storage captures directly or emits event for coordinator
- add capture/release adapter if storage owns final settlement
- add failure handling for write succeeded but capture failed
```

### 5. Storage Usage Event Export

Currently usage events are returned in the `/paid/o` response and ingested in-process by tests.

Production needs a real export path:

```text
POST to ron-accounting
bus event
local WAL/export file
batch forwarder
```

Remaining work:

```text
- define event delivery mechanism
- add retry/backoff
- add idempotency key for usage events
- add dead-letter/quarantine behavior
```

### 6. Broader CAS Hardening

Current CAS baseline is good, but remaining storage hardening includes:

```text
PUT /o/:cid with hash verification, if desired
strict CID-only APIs
body cap enforcement tests
concurrency/race tests
range edge cases
conditional GET/If-None-Match
FS backend, if beta needs persistence
amnesia-vs-durable mode decision
```

The older notes emphasized BLAKE3 `b3:<hex>`, no names inside storage, and content-address-only semantics.  Those invariants should continue to guide all storage changes.

### 7. Metrics Expansion

Current paid metrics exist. Broader storage metrics remain:

```text
storage_put_total
storage_get_total
storage_bytes_written_total
storage_bytes_read_total
storage_put_latency_seconds
storage_get_latency_seconds
storage_objects_total
storage_fs_errors_total
```

These are part of the intended storage observability contract. 

## Recommended Next Steps for `svc-storage`

```text
1. Add WalletReceiptLookup trait and mock verifier tests.
2. Keep DevHeaderVerifier as dev/test mode.
3. Add real WalletReceiptVerifier only after wallet receipt lookup contract is locked.
4. Add config switch for paid verifier mode.
5. Add production accounting event exporter.
6. Add capture/release automation strategy.
7. Expand storage metrics to full golden set.
```

## Completion Estimate

```text
svc-storage CAS baseline: ~80–85%
svc-storage paid-write beta path: ~75–82%
svc-storage production paid storage: ~60–70%
```

The paid-storage path is now real and tested, but production verification, accounting export, and capture/release automation remain the major hardening steps.

---

## Overall Session Checkpoint

The internal ROC value plane is now in a very strong place:

```text
ron-accounting → svc-rewarder → svc-wallet → svc-storage → ron-accounting
```

The current green sweep proves the core loop is alive across the crates. 

Current rough completion:

```text
Internal ROC WEB3 value plane: ~90–92%
Production-hardened WEB3 stack: ~70–78%
```

The next best session should focus on:

```text
svc-storage WalletReceiptLookup trait + mock tests
then svc-wallet receipt lookup compatibility
then real WalletReceiptVerifier wiring
```
# WEB3 Carryover Notes — RustyOnions ROC Value Plane

Date: 2026-04-27
Project: RustyOnions / RON-CORE / WEB3 value plane
North star: `WEB3.MD`
Strategic phase: **internal ROC closed-loop economy hardening**
Current status: **WEB3 internal ROC value plane is approximately 90–92% for beta-grade internal flow; production hardening remains approximately 70–78%.**

---

## 0. One-Sentence Carryover Summary

We advanced RustyOnions WEB3 from isolated crate foundations into a working internal ROC value-plane loop: accounting events can feed reward snapshots, rewarder can issue deterministic wallet requests, wallet can mutate ledger-backed balances with escrow, storage can enforce paid writes through `/paid/o`, storage emits accounting-compatible usage events, and the four-crate sweep is green across `ron-accounting`, `svc-rewarder`, `svc-wallet`, and `svc-storage`. The WEB3 blueprint remains clear that ROC is the internal economy target now, while ROX/Solana/external settlement stays deferred.  

---

## 1. WEB3 North Star to Preserve

`WEB3.MD` defines the current mission as shipping **ROC** as an internal deterministic economy with ledger, wallet, accounting, rewarder, enforcement, visibility, and at least one real in-network use case, recommended as paid storage/pinning. It explicitly defers **ROX, Solana, bridge contracts, on-chain settlement, staking, public governance, exchanges, and advanced ZK/privacy** until later. 

The WEB3 v1 done condition remains:

```text
1. ron-ledger is durable/replayable and crash-safe.
2. svc-wallet is the sole mutation front-door with nonce + idempotency.
3. At least one real in-network use case charges ROC.
4. svc-rewarder issues ROC deterministically from measurable contributions.
5. Metrics/dashboard/admin endpoints show the economy moving.
```

We have now substantially proven items 2, 3, and 4 in dev/in-process form. Item 1 still needs production durability hardening. Item 5 still needs dashboard/admin visibility.

---

## 2. Non-Negotiable Invariants

Preserve these across all future WEB3 work:

```text
ROC only for now.
ROX/Solana remains deferred.
No floating-point money or accounting math.
All economic amounts use integer minor units.
ron-ledger is append-only truth.
ron-accounting is transient counters/snapshots, not truth.
svc-wallet is the only normal mutation front-door.
svc-rewarder computes issuance intents, but does not mutate ron-ledger directly.
svc-storage/gateway must not mutate ron-ledger directly.
All wallet mutations use nonce + idempotency.
No negative balances by default.
Transfers conserve value.
Issue/mint and burn are explicit audited exceptions.
Paid operations use hold → capture → release.
Amnesia mode never silently persists to disk.
Observability must be truthful.
Capability/policy checks must fail closed.
```

Correct value boundary:

```text
usage/metering → ron-accounting
reward calculation → svc-rewarder
economic mutation → svc-wallet
durable truth → ron-ledger
paid-use enforcement → svc-storage / svc-gateway
visibility → svc-admin / dashboard
```

Wrong pattern to reject:

```text
svc-rewarder → ron-ledger direct mutation
ron-accounting → ron-ledger direct mutation
svc-storage/gateway → ron-ledger direct mutation
```

---

## 3. Current Verified Green Gates

The full four-crate WEB3 sweep passed. The uploaded terminal output shows `ron-accounting` passing normal and WAL-feature suites, `svc-rewarder` passing its unit/integration suite including the ROC loop, `svc-wallet` passing its broad wallet/escrow/invariant suite, and `svc-storage` passing its paid-storage tests. 

Standard commands used:

```bash
cargo fmt

cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal

cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets

cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets

cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

Most recent `svc-storage` gate after the verifier-contract batch:

```text
http_blackbox.rs:          1 passed
paid_write_policy.rs:     5 passed
paid_write_verifier.rs:   11 passed
web3_paid_storage_loop.rs: 1 passed
clippy:                   clean
bench targets:            compile
```

---

## 4. What We Accomplished This Session

### 4.1 `ron-accounting`

Current role:

```text
usage events
→ normalized counters
→ deterministic slices/windows
→ reward projection
→ canonical reward snapshot
→ rewarder input
```

Important proven capabilities:

```text
UsageEvent ingest is active.
MetricKind supports bytes_stored, bytes_served, pin_seconds, request_ok, uptime_seconds, custom.
record_usage_events reports inspected/recorded/skipped counts.
Reward projection maps PUT bytes to stored bytes and GET bytes to served bytes.
Uptime/request rows can map to uptime_seconds.
Reward snapshot canonicalization is stable.
Reward snapshot CID is stable.
Interop vector validates and matches rewarder shape.
WAL mode appends/drains and skips corrupt records.
Normal and WAL-feature suites are green.
```

Current test highlights:

```text
normal mode:
  2 loom/model tests
  3 property tests
  37 unit tests
  record bench smoke
  seal bench smoke

WAL mode:
  2 loom/model tests
  3 property tests
  38 unit tests
  record bench smoke
  seal bench smoke
```

Major integration achieved:

```text
svc-storage /paid/o now emits ron-accounting-compatible usage_events.
web3_paid_storage_loop parses those as ron_accounting::UsageEvent.
record_usage_events ingests bytes_stored + request_ok + pin_seconds.
```

Remaining work:

```text
1. Real production event transport:
   - HTTP ingest
   - bus event
   - WAL/export file
   - batch forwarder
   - direct library path for micronode mode

2. Snapshot production workflow:
   - seal window
   - project contributions
   - generate canonical snapshot
   - compute b3 snapshot CID
   - publish snapshot reference to svc-rewarder

3. Service adapter hardening:
   - idempotency for usage events
   - duplicate handling
   - failed export recovery
   - dead-letter/quarantine behavior

4. Dashboard/admin DTOs:
   - usage by tenant/service/region
   - provider contribution score
   - epoch snapshot status
   - projection CID
   - WAL/replay health

5. Production WAL policy:
   - default on/off decision
   - WAL path
   - compaction/retention
   - corruption alerting
   - startup replay behavior
```

Completion estimate:

```text
ron-accounting for internal ROC beta: ~88–92%
ron-accounting for production service: ~72–80%
```

---

### 4.2 `svc-rewarder`

Current role:

```text
sealed accounting snapshot
→ deterministic reward manifest
→ settlement batch
→ wallet issue batch
```

What we added/proved:

```text
svc-rewarder consumes ron-accounting reward_snapshot_interop_vector_v1.
AccountingSnapshot parses the accounting canonical JSON.
canonical_snapshot_cid agrees with ron-accounting snapshot_cid.
Expected contribution count and score are stable.
Expected total score = 342.
Reward manifest computation is deterministic.
Wallet issue preview shape is compatible with svc-wallet.
```

Critical invariant now enforced:

```text
inputs_cid == canonical_snapshot_cid(snapshot)
```

Tested behavior:

```text
matching inputs_cid accepts
mismatched inputs_cid rejects
duplicate accounts reject
canonical sorting is stable
policy hash mismatch rejects
uppercase hash rejects
settlement preview returns wallet issue batch
dry-run can promote to production without consuming run key
settlement egress is idempotent by run key
```

Closed-loop proof added:

```text
ron-accounting vector
→ svc-rewarder manifest
→ wallet issue requests
→ svc-wallet HTTP mutation path
→ ron-ledger-backed balances
→ idempotent replay without double issue
```

Current test status:

```text
lib tests:          1 passed
integration tests:  9 passed
unit tests:         38 passed
bench smoke:        reward_calc_100 success
clippy:             clean
```

Remaining work:

```text
1. Production wallet client:
   - POST /v1/issue
   - auth/capability
   - idempotency key
   - timeout/retry policy
   - receipt verification
   - partial failure handling

2. Accounting snapshot fetch adapter:
   - fetch by inputs_cid
   - verify canonical CID
   - preserve inline mode for dev/tests
   - reject corrupt/mismatched snapshots

3. Policy registry:
   - fetch signed policy by policy_id
   - verify policy_hash
   - verify signatures
   - reject stale/revoked policy
   - metrics for policy fetch/verify

4. Anti-gaming rules:
   - per-account caps
   - per-node caps
   - per-tenant caps
   - minimum uptime thresholds
   - Sybil-resistant identity hooks
   - suspicious concentration quarantine
   - signed accounting windows

5. Manifest/audit persistence:
   - reward manifest artifact
   - settlement batch artifact
   - wallet issue receipts
   - run status
   - quarantine reasons
   - amnesia-safe behavior
```

Completion estimate:

```text
svc-rewarder for internal ROC beta: ~88–92%
svc-rewarder for production reward service: ~72–82%
```

---

### 4.3 `svc-wallet`

Current role:

```text
ROC mutation front-door
auth/policy boundary
nonce/idempotency protection
ledger-backed balance/receipt service
escrow engine for paid operations
```

Major accomplishments:

```text
Issue/transfer/burn/balance/receipt routes are working.
Strict DTO validation exists.
Amounts are string-encoded integer minor units at JSON boundary.
Idempotency replay returns same receipt.
Same idempotency key with different body rejects.
Strict per-account nonce table exists.
Rejected commit paths roll nonce reservations back.
Local ron-ledger adapter handles issue, transfer, burn, hold, capture, release.
Receipt hashing and txid/request fingerprints are deterministic.
Metrics expose success/replay/reject/op counts.
Health/ready/metrics endpoints are live.
```

Escrow added:

```text
POST /v1/hold
POST /v1/capture
POST /v1/release
```

Escrow semantics:

```text
hold:    payer  → escrow
capture: escrow → service/provider
release: escrow → payer/refund target
```

Paid-storage wallet smoke proves:

```text
0 ROC payer cannot reserve paid storage.
funded payer can reserve max estimate.
hold reserves max estimate.
capture pays actual cost to svc_storage.
release refunds unused remainder.
total value is conserved.
metrics expose issue/hold/capture/release/reject.
```

Rewarder loop proof:

```text
rewarder-generated wallet issue requests are accepted.
balances increase.
same epoch replay does not double issue.
```

Current test highlights:

```text
29 lib tests
api_contract tests
http_blackbox tests
http_escrow tests
transport bounds tests
overflow ceiling tests
hold/capture/release tests
double-spend tests
non-negativity tests
conservation tests
caps/policy tests
DTO hygiene tests
ledger primacy tests
observability tests
amnesia tests
loom invariants
readiness/shedding tests
receipt hash vector tests
web3 paid storage smoke
bench smokes
```

Remaining work:

```text
1. Receipt lookup stabilization:
   - confirm /v1/tx/:txid shape
   - ensure it returns storage-compatible WalletReceipt fields
   - ensure receipt_hash stability
   - support future fields safely

2. Dedicated hold object / hold_id model:
   - hold_id
   - payer
   - escrow
   - intended_service
   - max_amount
   - expires_at
   - context/cid/route
   - status = held|captured|released|expired
   - terminal-state enforcement

3. Production auth/caps:
   - real ron-auth / svc-passport / ron-policy integration
   - scopes for issue, burn, transfer, hold, capture, release, receipt lookup
   - service-scoped receipt lookup caveats

4. Durable wallet state:
   - durable idempotency store
   - durable receipt book
   - durable nonce table
   - durable ledger mode
   - expiration/compaction
   - restart/replay behavior

5. Service client support:
   - issue client
   - hold/capture/release client
   - receipt lookup client
   - timeout/retry/idempotency helpers
```

Completion estimate:

```text
svc-wallet for internal ROC beta: ~90–94%
svc-wallet for production wallet service: ~75–85%
```

---

### 4.4 `svc-storage`

Current role:

```text
CAS object service
free/dev object route
paid ROC-gated object route
accounting usage event emitter
paid-write metrics producer
future wallet receipt verifier host
```

Major accomplishments:

```text
Removed unused ron-kernel / ron-metrics direct dependencies from svc-storage.
Restored clean svc-storage clippy/test gate.
Replaced flaky TCP-spawn blackbox test with in-process Axum router test.
Added dedicated /paid/o route.
Kept existing /o route untouched.
Added paid-write proof enforcement.
Added accounting context validation.
Added accounting-compatible usage_events in /paid/o response.
Added paid-write Prometheus metrics.
Added cross-crate paid storage loop with svc-wallet and ron-accounting.
Added paid-write policy hardening tests.
Added PaidWriteVerifier seam.
Added DevHeaderVerifier.
Added WalletReceiptVerifier fail-closed stub.
Added WalletReceipt DTO and validation contract.
```

Current routes:

```text
POST /o
PUT  /o
HEAD /o/:cid
GET  /o/:cid

POST /paid/o
PUT  /paid/o
```

Free/dev CAS route behavior:

```text
POST /o stores bytes and returns cid.
HEAD /o/:cid returns headers.
GET /o/:cid returns bytes.
Range GET works.
unknown CID returns 404.
```

Paid route behavior:

```text
/paid/o without proof              → 402, no write
/paid/o with malformed proof        → 402, no write
/paid/o with bad accounting context → 400, no write
/paid/o with valid dev proof        → 200, writes object
```

Current paid proof requirements:

```text
x-ron-paid-op = hold
x-ron-paid-asset = roc
x-ron-paid-estimate-minor = positive integer
x-ron-wallet-txid = non-empty
x-ron-wallet-receipt-hash = b3:<64 lowercase hex>
x-ron-wallet-from = payer
x-ron-wallet-to = escrow
```

Optional accounting headers:

```text
x-ron-tenant
x-ron-accounting-subject
x-ron-region
x-ron-pin-seconds
```

Usage events emitted on paid write:

```text
bytes_stored
request_ok
pin_seconds, when requested
```

Paid-write metrics added:

```text
storage_paid_write_total{status="accepted"}
storage_paid_write_total{status="payment_required"}
storage_paid_write_total{status="bad_accounting_context"}
storage_paid_write_total{status="storage_error"}
storage_paid_write_bytes_total
```

Cross-crate paid storage loop proves:

```text
unfunded wallet hold rejects
unfunded storage write does not create object
funded payer receives ROC
wallet hold reserves ROC into escrow
/paid/o accepts hold proof and stores object
GET /o/:cid returns exact object bytes
storage emits accounting-compatible usage events
ron-accounting ingests those events
wallet capture pays svc_storage
wallet release refunds payer
final wallet value is conserved
```

Verifier seam now exists:

```text
PaidWriteVerifier trait
DevHeaderVerifier
WalletReceiptVerifier
WalletReceipt DTO
PaidWriteProof
VerifiedPaidWrite
PaidWriteVerificationError
```

Current verifier behavior:

```text
/paid/o → DevHeaderVerifier → CAS write
WalletReceiptVerifier exists but fails closed until wired
```

WalletReceipt contract validates only:

```text
op = hold
asset = roc
amount_minor = positive integer string
from = payer present
to = escrow present
receipt_hash = b3:<64 lowercase hex>
```

Current `svc-storage` tests:

```text
http_blackbox.rs:          1 passed
paid_write_policy.rs:     5 passed
paid_write_verifier.rs:   11 passed
web3_paid_storage_loop.rs: 1 passed
bench targets:            compile
clippy:                   clean
```

Remaining work:

```text
1. WalletReceiptLookup trait:
   - pure trait
   - mock lookup tests
   - no outbound HTTP yet
   - proves lookup receipt → WalletReceipt → PaidWriteProof

2. Real WalletReceiptVerifier:
   - use WalletReceiptLookup
   - verify looked-up receipt
   - reject mismatched txid/payer/escrow/amount/hash
   - keep DevHeaderVerifier for dev/test mode

3. HTTP wallet receipt client:
   - GET /v1/tx/:txid
   - timeout/retry
   - auth/capability
   - stable error handling

4. Paid write config:
   - dev-header mode
   - wallet-receipt mode
   - disabled mode
   - fail-closed production default if desired

5. Stronger proof binding:
   - expected CID
   - payer
   - escrow
   - service = svc-storage
   - route = /paid/o
   - max bytes/cost
   - expiry
   - context/memo schema

6. Capture/release automation:
   - decide whether svc-storage captures directly
   - or emits event for billing coordinator
   - handle write succeeded but capture failed
   - handle capture succeeded but release failed

7. Production accounting event export:
   - POST to ron-accounting
   - bus event
   - WAL/export file
   - batch forwarder
   - retry/dead-letter

8. Broader CAS hardening:
   - PUT /o/:cid with hash verification if desired
   - strict CID-only APIs
   - body cap tests
   - concurrency/race tests
   - conditional GET/If-None-Match
   - FS backend if beta needs persistence

9. Full storage metrics:
   - storage_put_total
   - storage_get_total
   - storage_bytes_written_total
   - storage_bytes_read_total
   - storage_put_latency_seconds
   - storage_get_latency_seconds
   - storage_objects_total
   - storage_fs_errors_total
```

Completion estimate:

```text
svc-storage CAS baseline: ~80–85%
svc-storage paid-write beta path: ~78–84%
svc-storage production paid storage: ~62–72%
```

---

### 4.5 `ron-ledger`

Current role:

```text
append-only durable truth core
balance reconstruction
ledger-backed wallet commits
receipt/audit root source
```

No heavy `ron-ledger` changes were made in the latest storage-focused sequence, but `ron-ledger` was exercised indirectly through `svc-wallet` and the WEB3 loops.

Current proven ledger-backed behavior through wallet:

```text
issue creates balance
transfer conserves value
burn explicitly reduces supply
hold reserves funds into escrow
capture moves escrow funds to service
release returns remainder to payer
failed capture above held amount does not credit payee
nonce replay rejects
idempotent replay does not double issue
```

Remaining work:

```text
1. Durable backend hardening:
   - crash/restart replay tests
   - corruption handling
   - disk pressure behavior
   - checkpoint/snapshot strategy

2. Receipt audit:
   - stable receipt binding to ledger root/sequence
   - receipt verification docs
   - audit export

3. Production mode:
   - define amnesia vs durable ledger mode
   - choose default for beta
   - document persistence paths and cleanup

4. Wider integration:
   - rewarder issue batch → wallet → ledger → replay
   - paid storage escrow → wallet → ledger → replay
   - dashboard balance/receipt checks against ledger

5. Governance:
   - who can issue ROC
   - how burns are authorized
   - how supply is reported
   - how accidental issuance is prevented
```

Completion estimate:

```text
ron-ledger for internal ROC beta: ~85–90%
ron-ledger for production audited economy: ~70–78%
```

---

## 5. Current System Completion Estimate

Current truthful status:

```text
ron-ledger      beta internal: ~85–90%
ron-accounting  beta internal: ~88–92%
svc-rewarder    beta internal: ~88–92%
svc-wallet      beta internal: ~90–94%
svc-storage     paid beta path: ~78–84%

WEB3 internal ROC value plane: ~90–92%
Production-hardened WEB3 stack: ~70–78%
```

Reasoning:

```text
The dev/internal closed loop is now real:
usage → accounting → rewarder → wallet → ledger
wallet → paid storage → accounting
```

But production still needs:

```text
real service-to-service adapters
real auth/capability scopes
durability decisions
dashboard/admin visibility
wallet receipt verification
paid-write capture/release automation
production accounting event export
```

---

## 6. Recommended Next Session Focus

The next session should continue with **svc-storage WalletReceiptLookup trait + mock tests**.

Do **not** jump to ROX/Solana.
Do **not** start another isolated crate unless something breaks.
Do **not** make `/paid/o` perform outbound HTTP yet until the lookup trait contract is proven.

### Recommended Work Package 1 — `WalletReceiptLookup` Trait

Goal:

```text
Introduce a pure lookup boundary that lets WalletReceiptVerifier validate looked-up wallet receipts without hardcoding HTTP yet.
```

Likely file:

```text
crates/svc-storage/src/policy/paid_write.rs
```

Add conceptually:

```rust
pub trait WalletReceiptLookup {
    fn lookup_receipt(&self, txid: &str) -> Result<Option<WalletReceipt>, PaidWriteVerificationError>;
}
```

Or, if async is preferred:

```rust
#[async_trait]
pub trait WalletReceiptLookup {
    async fn lookup_receipt(&self, txid: &str) -> Result<Option<WalletReceipt>, PaidWriteVerificationError>;
}
```

Recommended first step: keep it sync/pure if possible, because current route is not yet wired for injection.

Add test-only mock:

```text
MockWalletReceiptLookup
  has receipt
  missing receipt
  wrong receipt
  receipt with wrong op
  receipt with wrong asset
  receipt with wrong payer/escrow
```

### Recommended Work Package 2 — WalletReceiptVerifier Contract Tests

Add tests proving:

```text
valid looked-up hold receipt → accepted
missing receipt → payment_required
receipt txid mismatch → payment_required
wrong op → payment_required
wrong asset → payment_required
zero amount → payment_required
bad receipt_hash → payment_required
missing payer → payment_required
missing escrow → payment_required
```

### Recommended Work Package 3 — Keep `/paid/o` on DevHeaderVerifier

Do not switch `/paid/o` to the new WalletReceiptVerifier yet.

Current route should remain:

```text
/paid/o → DevHeaderVerifier
```

until:

```text
WalletReceiptLookup trait is green
mock verifier tests are green
svc-wallet receipt lookup shape is confirmed
config mode exists
```

### Recommended Work Package 4 — Confirm `svc-wallet` Receipt Lookup Shape

After the lookup trait is green, check `svc-wallet`:

```text
GET /v1/tx/:txid
receipt JSON contains:
  txid
  op
  from
  to
  asset
  amount_minor
  nonce
  idem
  ts
  ledger_seq_start
  ledger_seq_end
  ledger_root
  receipt_hash
```

Then add a cross-crate compatibility test:

```text
svc-wallet hold receipt JSON
→ deserialize as svc_storage::policy::paid_write::WalletReceipt
→ validate_as_paid_write_hold()
```

### Recommended Work Package 5 — Real HTTP Wallet Receipt Client

Only after Work Packages 1–4:

```text
WalletReceiptHttpClient
  base_url
  timeout
  auth/capability token source
  GET /v1/tx/:txid
  parse WalletReceipt
  validate receipt
  fail closed on network/parse/validation errors
```

### Recommended Work Package 6 — Configurable Verifier Mode

Eventually add:

```text
PaidWriteVerifierMode::DevHeader
PaidWriteVerifierMode::WalletReceipt
PaidWriteVerifierMode::Disabled
```

Recommended default discussion:

```text
dev/micronode demo: DevHeader may be acceptable
beta local integration: WalletReceipt preferred
production: WalletReceipt or Disabled; never DevHeader by accident
```

---

## 7. Fresh Baseline Commands for Next Session

Start next session with:

```bash
cargo fmt

cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

Then, after any storage changes:

```bash
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets

cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal

cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
```

Run the full sweep before closing the next session:

```bash
cargo fmt

cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal

cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets

cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets

cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

---

## 8. Files Most Likely Relevant Next Session

Upload or provide fresh versions if possible:

```text
WEB3.MD

crates/svc-storage/Cargo.toml
crates/svc-storage/src/lib.rs
crates/svc-storage/src/policy/mod.rs
crates/svc-storage/src/policy/paid_write.rs
crates/svc-storage/src/http/routes/paid_object.rs
crates/svc-storage/tests/paid_write_verifier.rs
crates/svc-storage/tests/paid_write_policy.rs
crates/svc-storage/tests/web3_paid_storage_loop.rs

crates/svc-wallet/src/routes/v1/receipt.rs
crates/svc-wallet/src/dto/responses.rs
crates/svc-wallet/src/routes/mod.rs
crates/svc-wallet/tests/http_blackbox.rs

crates/ron-accounting/src/accounting/events.rs
crates/ron-accounting/src/accounting/reward_projection.rs
crates/ron-accounting/tests/unit.rs

crates/svc-rewarder/tests/integration/web3_roc_loop.rs
```

Fresh codebundles that would help:

```text
svc-storage CODEBUNDLE + NOTES
svc-wallet CODEBUNDLE + NOTES
ron-accounting CODEBUNDLE + NOTES
svc-rewarder CODEBUNDLE + NOTES
ron-ledger CODEBUNDLE + NOTES
WEB3.MD
```

---

## 9. Suggested First Prompt for Next Session

```text
We are continuing RustyOnions WEB3 / ROC internal economy work. Use WEB3.MD as the north star. ROX/Solana remains deferred. The current four-crate ROC loop is green: ron-accounting, svc-rewarder, svc-wallet, and svc-storage all passed clippy/tests. svc-storage now has /paid/o with DevHeaderVerifier, WalletReceipt DTO validation, WalletReceiptVerifier fail-closed stub, paid-write metrics, accounting-compatible usage events, and a cross-crate wallet → storage → accounting paid-storage loop.

Next task: add the WalletReceiptLookup trait and mock lookup tests in svc-storage, without switching /paid/o away from DevHeaderVerifier yet. The goal is to prove the production verifier contract before adding outbound HTTP. Preserve all invariants: wallet is mutation front-door, ledger is truth, accounting is transient counters, rewarder does not mutate ledger directly, paid operations use hold → capture → release, and ROX/Solana remains deferred.
```

---

## 10. Suggested Commit Summary for This Session

```text
web3: prove ROC paid storage loop and add storage paid-write verifier seam
```

Possible commit body:

```text
- Add svc-rewarder accounting interop proof using ron-accounting reward snapshot vector
- Enforce svc-rewarder inputs_cid == canonical_snapshot_cid(snapshot)
- Add rewarder → wallet issue loop proving idempotent ROC issuance
- Add svc-wallet hold/capture/release ledger primitives
- Add /v1/hold, /v1/capture, /v1/release HTTP endpoints
- Add wallet paid-storage state machine smoke
- Restore svc-storage clippy/test baseline
- Replace flaky svc-storage TCP blackbox test with in-process router contract test
- Add /paid/o route for ROC-gated paid storage writes
- Add paid-write proof admission and fail-closed tests
- Emit accounting-compatible usage_events from paid storage writes
- Add ron-accounting ingest proof for storage usage events
- Add storage_paid_write_total and storage_paid_write_bytes_total metrics
- Add PaidWriteVerifier seam, DevHeaderVerifier, WalletReceiptVerifier fail-closed stub
- Add WalletReceipt DTO and contract validation tests
- Confirm four-crate WEB3 sweep is green
```

---

## 11. Final State to Remember

The major milestone is:

```text
RON’s internal ROC economy now has a working dev closed loop.
```

Current verified loops:

```text
ron-accounting snapshot
→ svc-rewarder compute
→ svc-wallet issue
→ ledger-backed balances
→ replay-safe issuance

svc-wallet hold
→ svc-storage paid write
→ ron-accounting usage event ingest
→ svc-wallet capture/release
→ conserved final balances
```

The next real production step is not “more features.” It is **verification hardening**:

```text
trusted headers → wallet receipt lookup → validated hold receipt → paid write
```

After that comes:

```text
real accounting export
capture/release automation
dashboard/admin visibility
durable mode decisions
production caps/auth/policy
```

### END WEB3 CARRYOVER NOTES — APRIL 27 2026
