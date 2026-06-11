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


### BEGIN NOTE - APRIL 28 2026 - 23:00 CST


````md
# RustyOnions WEB3 / ROC Carryover Notes — April 29, 2026

Project: RustyOnions / RON / WEB3 / ROC internal value plane  
Current north-star docs: WEB3.MD, WEB3_2.MD, ALLNOTES.MD, CODEBUNDLE_RS.md, ALL_DOCS_COMBINED.MD, CODECHECK.md, CODECOMMENTS.MD  
Current status: WEB3 v1 internal ROC economy is now live-proven across paid storage, accounting, rewarder, wallet, and economics policy.  
Estimated current completion:  
- Core RustyOnions beta excluding browser extension: ~94–95%
- WEB3 v1 internal ROC value plane: ~96–97%
- Paid storage/pinning beta path: ~98–99%
- Reward issuance path: ~96–97%
- ROC economics config foundation: ~92–95%
- Full WEB3_2 product layer including passports, crab links, b3 asset pages, paid content, dashboard, and browser extension: still much earlier, roughly ~45–55% depending on scope

## 0. Executive Summary

This session completed several of the highest-impact WEB3/ROC integration gaps.

The WEB3 north star defines “done” for WEB3 v1 as: durable/replayable ledger, wallet as sole mutation front-door with nonce and idempotency, at least one real paid in-network use case, deterministic reward issuance from measurable contributions, and metrics/admin visibility showing the economy moving. It also explicitly keeps ROC internal and defers ROX/Solana/external settlement, staking, public exchange, and liquidity work. :contentReference[oaicite:0]{index=0}

By the end of this session, RustyOnions has live proof of both sides of the internal economy:

```text
Spend/use side:
svc-wallet issue
→ svc-wallet hold
→ svc-storage verifies wallet receipt
→ b3 CID-bound paid storage
→ svc-wallet capture/release
→ ron-accounting usage export
→ accounting snapshot rows
→ object readback
→ replay rejection for different bytes

Earn/reward side:
ron-accounting reward snapshot vector
→ svc-rewarder deterministic compute
→ svc-rewarder settlement preview
→ svc-rewarder HTTP emit
→ svc-wallet issue
→ ledger-backed balances
→ replay does not double issue
````

The rewarder live smoke passed with:

```text
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
acct_a=356 ROC minor units
acct_b=643 ROC minor units
payout_total=999 ROC minor units
replay=no double issue
```



The paid-storage live smoke passed with:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
payer balance: 52
escrow balance: 0
payee balance: 48
accounting snapshot row_count = 3
```



This means the internal ROC economy is no longer theoretical. It has a real paid use case, a real accounting output, a real reward loop, wallet mutation boundaries, and replay protection.

---

## 1. Crates Worked On This Session

Primary crates touched or validated:

```text
ron-accounting
svc-rewarder
svc-wallet
ron-policy
svc-storage
```

Root/project files touched or materially involved:

```text
configs/roc-economics.toml
scripts/web3_accounting_rewarder_wallet_smoke.sh
scripts/web3_paid_storage_live_smoke.sh
```

Indirectly validated but not heavily changed:

```text
ron-ledger
ron-proto
ron-naming
```

`ron-ledger` remains the durable economic truth underneath wallet receipts/balances, but this session did not focus on changing ledger internals.

---

## 2. Highest-Level Architecture Now Proven

The current working WEB3 v1 architecture is:

```text
ron-accounting
  usage events
  normalized counters
  reward snapshot vector
  accounting snapshot rows

svc-rewarder
  deterministic reward computation
  reward manifests
  settlement preview
  wallet issue batch
  HTTP emit to wallet

svc-wallet
  issue
  hold
  capture
  release
  receipt lookup
  balance reads
  idempotency / replay protection
  ledger-backed receipts

svc-storage
  b3 CID storage
  paid write verification
  wallet receipt verification
  hold context binding
  capture/release settlement
  accounting export

ron-policy
  ROC economics TOML parsing
  integer-only pricing
  basis-point split validation
  paid action validation
  first live consumer through svc-storage

ron-ledger
  append-only durable economic truth
  ledger roots/sequence fields surfaced through wallet receipts
```

The critical invariant remains:

```text
Accounting measures.
Rewarder plans.
Wallet mutates.
Ledger records truth.
Storage enforces paid use.
Policy validates economics.
```

Do not let any crate drift outside that boundary.

---

## 3. Session Accomplishments by Crate

## 3.1 ron-accounting

### What we accomplished

`ron-accounting` was already a strong/gold foundation crate from previous work. In this session, it became live-proven in two major ways.

First, we added/used a reward snapshot vector emitter:

```text
crates/ron-accounting/src/bin/ron_accounting_reward_snapshot_vector.rs
```

This emits a deterministic rewarder-compatible accounting vector. It was consumed by `svc-rewarder` in the live rewarder smoke.

Observed vector:

```text
epoch_id = interop-epoch-1
snapshot_cid = b3:81d428e1df7c29a8443467db8ed59ee2628ac43acd47c692fbc3c23ee495606d
```

Second, `ron-accounting` was started as a real live ingest service in the paid-storage smoke. `svc-storage` exported real usage events to it, and `/v1/snapshot` showed normalized rows:

```text
PIN_SECONDS / requests = 60
PUT / bytes = 48
REQ_OK / requests = 1
```

The live paid-storage output confirms the snapshot had `row_count = 3` and schema `ron-accounting.snapshot.v1`. 

### Current state

```text
ron-accounting role:
  usage/metering input
  normalized counters
  snapshot rows
  rewarder-compatible accounting vectors

Not its role:
  not wallet
  not ledger
  not settlement engine
  not durable economic truth
```

Current readiness estimate:

```text
ron-accounting WEB3 beta readiness: ~93–96%
production hardening readiness: ~75–82%
```

### What remains

```text
1. Add stronger HTTP ingest contract tests:
   /healthz
   /readyz
   /v1/usage-events
   /v1/snapshot
   auth rejection
   bad payload rejection
   duplicate idempotency handling
   snapshot determinism

2. Add production sealed-window lifecycle:
   open window
   sealed window
   exported window
   failed export
   replayed window
   compacted/expired window

3. Add reward snapshot endpoint:
   GET /v1/reward-snapshot/:window_id
   or equivalent stable rewarder pull API.

4. Decide WAL/default persistence posture:
   durable in macronode?
   disabled in amnesia?
   retention?
   replay?
   compaction?

5. Add dashboard/admin summary surfaces:
   usage by tenant
   usage by service
   usage by route
   latest snapshot CID
   export backlog
   ingest rejects

6. Add production capabilities:
   accounting.ingest
   accounting.snapshot.read
   accounting.admin
```

---

## 3.2 svc-rewarder

### What we accomplished

`svc-rewarder` moved from deterministic reward planning and wallet preview into actual wallet emission.

Main additions:

```text
crates/svc-rewarder/src/outputs/wallet.rs
crates/svc-rewarder/src/http/handlers.rs
crates/svc-rewarder/src/http/routes.rs
crates/svc-rewarder/src/config/load.rs
crates/svc-rewarder/tests/unit/wallet_client.rs
scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Major features completed:

```text
POST /rewarder/epochs/:epoch_id/emit
HttpWalletIssueClient
WalletHttpIssueOutcome
real POSTs to svc-wallet /v1/issue
rewarder config env overlays for wallet base URL/path
chunked/normal HTTP response parsing
idempotent replay proof
```

Final green gate:

```text
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Final test and smoke output included:

```text
9 integration tests passed
41 unit tests passed
reward_calc bench success
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```



### Current state

`svc-rewarder` now supports:

```text
deterministic reward manifest computation
snapshot CID validation
policy hash validation
settlement preview
wallet issue batch generation
real wallet HTTP emit
replay-safe reward issuance via wallet idempotency
```

Current readiness estimate:

```text
svc-rewarder WEB3 beta readiness: ~94–97%
production hardening readiness: ~82–88%
```

### What remains

```text
1. Replace raw Tokio TCP HTTP client with production client:
   reqwest-based internal client
   shared ron-app-sdk client
   UDS client
   mTLS/OAP client
   capability-aware wallet client

2. Replace hardcoded/dev bearer with real auth:
   rewarder.run
   rewarder.inspect
   rewarder.emit
   wallet.issue.rewarder

3. Pull real sealed snapshots from ron-accounting:
   currently the live proof uses the accounting vector emitter.
   production should support rewarder fetching sealed snapshot by ID/CID.

4. Decide manifest persistence:
   memory only?
   artifact files?
   durable manifest index?
   receipt index?

5. Add emit-after-restart proof:
   recompute same epoch
   emit again
   wallet returns same receipts
   balances unchanged

6. Add wallet dependency readiness:
   readiness should degrade if wallet unreachable.

7. Add richer metrics:
   rewarder_wallet_emit_total
   rewarder_wallet_emit_latency_seconds
   rewarder_wallet_emit_error_total
   rewarder_snapshot_fetch_total
   rewarder_manifest_write_total

8. Add anti-gaming reward policy:
   caps
   thresholds
   provider limits
   Sybil-resistant passport hooks
   signed accounting windows
```

---

## 3.3 svc-wallet

### What we accomplished

`svc-wallet` was not heavily edited in this session, but it was heavily proven. It is now live-proven as the sole normal economic mutation boundary for both the spend side and earn side of ROC.

It was used in the paid-storage smoke for:

```text
issue 100 ROC to acct_user
hold 70 ROC from acct_user to escrow_paid_write
capture 48 ROC from escrow_paid_write to svc_storage
release 22 ROC from escrow_paid_write to acct_user
balance reads
receipt lookup
```

The final paid-storage smoke confirmed:

```text
payer balance = 52
escrow balance = 0
payee balance = 48
```



It was used in the rewarder smoke for:

```text
issue 356 ROC to acct_a
issue 643 ROC to acct_b
return wallet receipts
prevent replay/double issue
```

The rewarder smoke proved replay did not double issue. 

### Current state

`svc-wallet` currently supports and has live proof for:

```text
issue
hold
capture
release
balance
receipt lookup
idempotency/replay protection
ledger-root/sequence receipt fields
```

Existing foundation also includes:

```text
transfer
burn
nonce discipline
healthz
readyz
metrics
strict DTOs
receipt hashes
```

Current readiness estimate:

```text
svc-wallet WEB3 beta readiness: ~94–97%
production hardening readiness: ~82–88%
```

### What remains

```text
1. Real auth/capability integration:
   wallet.balance.read
   wallet.receipt.read
   wallet.issue
   wallet.transfer
   wallet.burn
   wallet.hold
   wallet.capture
   wallet.release
   wallet.admin

2. Durable idempotency:
   preserve idempotency results across restart
   same key + same request returns same receipt
   same key + different request conflicts after restart

3. Durable receipt book:
   lookup by txid after restart
   possibly lookup by idempotency key
   receipt retention policy
   dashboard visibility

4. Wallet config hardening:
   bind addr
   metrics addr
   ledger backend
   idempotency backend
   receipt backend
   auth mode
   allowed assets
   issue/burn policy

5. OpenAPI/schema generation:
   issue
   transfer
   burn
   hold
   capture
   release
   balance
   tx lookup
   error schema

6. Escrow lifecycle visibility:
   open holds
   captured holds
   released holds
   orphaned escrow balances

7. Reconciliation/admin tools:
   verify receipt hash
   replay ledger and verify balances
   list recent receipts
   find stuck holds
   manual admin release under strict capability

8. Supply policy:
   rewarder should not have broad unrestricted issue power in production.
   issue caps by epoch/policy/run key should be enforced.
```

---

## 3.4 ron-policy

### What we accomplished

`ron-policy` got the new ROC economics foundation.

New project config:

```text
configs/roc-economics.toml
```

New module:

```text
crates/ron-policy/src/economics/
  mod.rs
  types.rs
  load.rs
  validate.rs
```

New tests:

```text
crates/ron-policy/tests/economics_policy.rs
```

Public API added:

```text
load_economics_toml
load_economics_toml_str
validate_economics_policy
EconomicsPolicy
ActionEconomics
EconomicsLimits
PricingKind
RoundingMode
PayoutSplit
```

Validated rules include:

```text
integer-only ROC minor units
no floats
no negative values
overflow rejection
known beta actions only
required beta actions present
disabled actions fail closed at lookup
basis-point split totals exactly 10_000
unknown split destinations rejected
duplicate split destinations rejected
dynamic recipient validation
capture cap enforcement
paid_storage_put price estimation
```

Final gate passed:

```text
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --all-targets
```

Final economics suite:

```text
15 passed
0 failed
```

The full `ron-policy` suite also passed, including golden reasons, unit tests, bench target, and examples. 

### Important fixes made

```text
u128 TOML parse issue fixed by using u64 in config DTOs
LazyLock MSRV issue fixed with OnceLock
SystemClock::default() clippy issue fixed
eval.rs restored to existing model/default_action behavior
missing_required_action test corrected to mutate loaded policy map
strict clippy doc lints resolved
```

### Current state

`ron-policy` can now validate and price:

```text
paid_storage_put
paid_storage_pin
paid_content_view
paid_song_play
site_visit
```

Current readiness estimate:

```text
ron-policy WEB3 beta economics readiness: ~88–93%
full WEB3_2 product policy readiness: ~70–78%
```

### What remains

```text
1. Split allocation output:
   policy.split_plan(action_id, recipients, amount_minor)

   Must produce:
   recipient account
   bps
   amount_minor
   residual/remainder sink

2. Canonical policy hash:
   economics_policy_cid = b3:<hash(canonical economics JSON/TOML)>
   include in receipts/manifests later.

3. Paid content/gateway consumers:
   paid_content_view
   paid_song_play
   site_visit
   paid_storage_pin

4. Recipient role binding:
   content_owner
   site_owner
   storage_provider
   edge_provider
   curator_pool
   app_owner

   ron-policy validates roles, but upstream services must resolve them.

5. Passport/capability caveats:
   per-action caps
   anonymous caps
   wallet-linked passport permissions
   per-site spend limits
   TTL/audience/service caveats

6. Structured economics error taxonomy:
   UnknownAction
   DisabledAction
   InvalidSplitTotal
   UnknownSplitDestination
   MissingRecipient
   CapExceeded
   Overflow
   InvalidMoneyAmount

7. CLI/admin tooling:
   validate-economics
   explain-price
   split-plan
   diff policies

8. Property tests:
   randomized valid splits
   randomized invalid splits
   price never exceeds caps
   allocations conserve total
```

---

## 3.5 svc-storage

### What we accomplished

`svc-storage` became the first real live consumer of the new ROC economics config, while preserving its paid-storage live proof.

New/updated pieces:

```text
crates/svc-storage/Cargo.toml
crates/svc-storage/src/policy/economics.rs
crates/svc-storage/src/policy/settlement.rs
crates/svc-storage/src/http/routes/paid_object.rs
crates/svc-storage/tests/paid_write_economics.rs
```

Added dependency:

```toml
ron-policy = { path = "../ron-policy" }
```

New env vars:

```text
RON_STORAGE_ROC_ECONOMICS_PATH
RON_STORAGE_ROC_ECONOMICS_ACTION
```

Behavior:

```text
If RON_STORAGE_ROC_ECONOMICS_PATH is unset:
  keep legacy beta pricing max(bytes_stored, 1)

If RON_STORAGE_ROC_ECONOMICS_PATH is set:
  load configs/roc-economics.toml through ron-policy
  price paid_storage_put or configured action
  fail closed on invalid/missing/unknown config
```

New economics tests passed:

```text
6 passed
0 failed
```

The full `svc-storage` test suite passed and the paid-storage live smoke passed. 

Live smoke proved:

```text
wallet issue
wallet hold
wallet receipt verification
CID-bound context idem
storage write
wallet capture
wallet release
accounting export
object readback
balance conservation
replay rejection for different bytes
```

The live smoke output showed:

```text
capture_amount_minor = 48
release_amount_minor = 22
accounting_export.status = exported
event_count = 3
```



### Current state

`svc-storage` is essentially beta-complete for paid storage.

Current readiness estimate:

```text
paid storage WEB3 beta readiness: ~98–99%
svc-storage production-hardening readiness: ~90–94%
```

### What remains

```text
1. Economics-enabled live smoke:
   run paid storage smoke with:
   RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml

   Script must raise wallet hold amount because current 48-byte object prices to 84 minor units under economics config.

2. Add paid storage estimate endpoint:
   GET /paid/o/estimate?bytes=48
   returns action, asset, estimated hold, pricing mode.

3. Settlement recovery:
   write succeeds but capture fails
   capture succeeds but release fails
   failed write release retry
   quarantine/dead-letter queue
   admin reconcile path

4. Accounting export outbox:
   retry
   backoff
   idempotent replay
   dead-letter
   metrics

5. Capability/passport integration:
   payer account caveats
   max bytes
   allowed action
   CID caveat
   expiry
   tenant caveat

6. Gateway/omnigate exposure:
   expose paid storage through gateway
   preflight estimate
   wallet hold UX handoff
   paid object response pass-through

7. Pinning semantics:
   paid_storage_pin
   pin duration
   pin extension
   pin expiry
   pin status
   pin cleanup

8. Storage durability/amnesia posture:
   memory vs disk
   paid object persistence policy
   provider reward implications
   GC policy

9. Metrics expansion:
   settlement_capture_total
   settlement_release_total
   economics_price_total
   wallet_receipt_lookup_latency
   paid_context_mismatch_total
```

---

## 4. Root Files / Scripts Completed or Changed

## 4.1 configs/roc-economics.toml

This new root config is now the canonical beta economics config.

It includes:

```text
version
unit
default_asset
rounding
remainder_sink
accounts
roles
limits
actions
splits
basis-point economics
```

Important current action examples:

```text
paid_storage_put:
  per_byte_plus_minimum
  minimum charge = 70
  max hold multiplier = 12000 bps
  48-byte object prices to 84

paid_content_view:
  flat price = 5

paid_song_play:
  flat price = 2

site_visit:
  flat price = 1
```

## 4.2 scripts/web3_accounting_rewarder_wallet_smoke.sh

Now proves:

```text
ron-accounting vector
→ svc-rewarder compute
→ svc-rewarder settlement preview
→ svc-rewarder HTTP emit
→ svc-wallet issue
→ ledger-backed balances
→ replay no double issue
```

Final proof line:

```text
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```



## 4.3 scripts/web3_paid_storage_live_smoke.sh

Now proves:

```text
svc-wallet issue/hold
→ svc-storage paid write
→ wallet capture/release
→ accounting export
→ accounting snapshot rows
→ object readback
→ replay rejection
```

Final proof line:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```



---

## 5. Final Green Gates From This Session

## svc-rewarder

```bash
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Result:

```text
1 lib unit test passed
9 integration tests passed
41 unit tests passed
reward_calc bench success
live rewarder → wallet smoke green
```



## ron-policy

```bash
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --all-targets
```

Result:

```text
economics_policy: 15 passed
full ron-policy test suite passed
bench target success
examples built
```



## svc-storage

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
```

Result:

```text
paid_write_economics: 6 passed
full svc-storage suite passed
live paid-storage smoke green
```



---

## 6. Current Completion Estimate

After this session:

```text
Core RON substrate / RustyOnions beta excluding extension:
  ~94–95%

WEB3 v1 internal ROC value plane:
  ~96–97%

Paid storage loop:
  ~98–99%

Reward issuance loop:
  ~96–97%

Wallet mutation boundary:
  ~94–97%

Accounting usage/reward snapshot substrate:
  ~93–96%

ROC economics config foundation:
  ~92–95%

Production hardening across value plane:
  ~80–88%

WEB3_2 product layer:
  ~45–55%

Browser extension:
  still not the main completed area; likely early/low relative to backend
```

Interpretation:

```text
The hard backend economy loop is now mostly done for beta.
The remaining work is mostly:
  productization
  dashboard/admin visibility
  production auth/capabilities
  recovery/outbox/idempotency persistence
  gateway/omnigate integration
  WEB3_2 product layer
  browser extension UX
```

---

## 7. What Remains To Complete RustyOnions

## 7.1 Next Highest-Impact Batch: Economics-Enabled Paid Storage Live Smoke

This is the immediate next batch.

Goal:

```text
Run the paid-storage smoke with:
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml
```

Need to update the script so:

```text
hold amount is high enough for configured economics price
48-byte object captures 84 instead of legacy 48
release amount is correct
balances conserve
accounting export still succeeds
context replay rejection still succeeds
fallback legacy smoke still remains green when env is unset
```

Expected economics-enabled proof:

```text
legacy mode:
  48-byte object captures 48

economics mode:
  48-byte object captures 84
```

This would prove the new economics config is not only unit-tested but live-used in the actual paid-storage smoke.

## 7.2 Add Paid Storage Estimate Endpoint

Needed endpoint:

```text
GET /paid/o/estimate?bytes=48
```

or:

```text
POST /paid/o/estimate
```

Response should include:

```json
{
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 48,
  "estimate_minor": "84",
  "pricing_mode": "roc-economics",
  "economics_policy_hash": "b3:<optional later>"
}
```

This is required for gateway, omnigate, dashboard, and browser extension UX.

## 7.3 ron-policy Split Allocation

Current `ron-policy` validates splits but does not yet compute split allocations.

Needed API:

```text
split_plan(action_id, recipients, amount_minor)
```

Output should include:

```text
destination role/account
bps
amount_minor
residual/remainder sink
```

Required invariant:

```text
sum(split amounts) + residual == captured amount
```

This is needed before paid content, site visits, creator payouts, curator pools, treasury splits, and provider splits become live.

## 7.4 Gateway / Omnigate Paid Access

Paid storage works directly on `svc-storage`. Next product layer should expose paid flows through:

```text
svc-gateway
omnigate
```

Needed:

```text
paid storage route via gateway
preflight price estimate
wallet hold flow
pass wallet receipt headers to storage
return b3 CID
surface receipt/accounting info
```

Then extend to:

```text
paid_content_view
site_visit
paid_song_play
paid_video_view
app access
```

## 7.5 B3 Asset Pages and crab:// Links

WEB3_2 product layer still needs:

```text
crab://b3/<hash>.image
crab://b3/<hash>.song
crab://b3/<hash>.article
crab://b3/<hash>.video
crab://b3/<hash>.comment
```

Important boundary:

```text
Storage stores bytes by b3 CID only.
Asset pages belong upstream.
```

Needed crates/services:

```text
ron-naming
svc-index
svc-gateway
omnigate
ron-policy
svc-passport / ron-auth
```

Asset page should eventually show:

```text
ownership
payout address
tags
description
curator metadata
provenance
storage/provider data
receipts where applicable
paid access options
```

## 7.6 RON Passports

WEB3_2 needs:

```text
local persistent RON Passports
main identity passports
alternate anonymous passports
wallet-linked passport permissions
capability caveats
paid action permissions
```

Needed work:

```text
passport local vault
passport account binding
wallet permission grant flow
anonymous/alt identity spend caps
service capability issuing
browser extension wallet/passport UX
```

## 7.7 Dashboard / Admin Visibility

WEB3 v1 “done” includes visibility. Need dashboard/admin surfaces showing the economy moving.

Minimum dashboard/admin data:

```text
total supply
issued total
burned total
wallet balances
recent receipts
paid storage captures
paid storage releases
reward epochs
reward payouts
accounting usage rows
storage bytes
pin seconds
economics config loaded
service health/readiness
```

Possible endpoints:

```text
svc-wallet:
  /v1/balance
  /v1/tx/:txid
  future /admin/wallet/summary

svc-rewarder:
  /rewarder/epochs
  /rewarder/epochs/:id
  /rewarder/epochs/:id/settlement
  /rewarder/epochs/:id/emit
  future /admin/rewarder/summary

ron-accounting:
  /v1/snapshot
  future /v1/reward-snapshot/:window_id
  future /admin/accounting/summary

svc-storage:
  future /admin/storage/paid-summary
  future /paid/o/estimate
```

## 7.8 Production Auth / Capabilities

Current smokes use dev bearer patterns. Production needs real capability enforcement.

Needed scopes:

```text
wallet.balance.read
wallet.receipt.read
wallet.issue
wallet.transfer
wallet.burn
wallet.hold
wallet.capture
wallet.release

rewarder.run
rewarder.inspect
rewarder.emit

accounting.ingest
accounting.snapshot.read
accounting.admin

storage.paid.write
storage.paid.estimate
storage.admin

gateway.paid.access
passport.wallet.grant
```

Important:

```text
Issue and burn must be the most restricted.
Rewarder issue permissions must be epoch/policy/amount scoped.
Storage paid writes must be CID/action/amount scoped.
```

## 7.9 Durable Idempotency / Restart Proofs

Need restart tests for:

```text
svc-wallet idempotency
wallet receipt lookup
wallet balances
rewarder emit replay
paid storage settlement recovery
accounting ingest dedupe
```

Critical tests:

```text
wallet:
  issue succeeds
  service restarts
  same idempotency key/body returns same receipt
  same key/different body conflicts
  balance unchanged

rewarder:
  compute epoch
  emit
  restart rewarder
  recompute same epoch
  emit again
  wallet no double issue

storage:
  hold exists
  storage write succeeds
  capture/release partially fails
  restart
  recovery/reconcile works
```

## 7.10 Settlement Recovery and Accounting Outbox

Current live path is green, but production needs failure recovery.

Storage needs:

```text
settlement retry queue
dead-letter queue
capture failure visibility
release failure visibility
failed write release retry
admin reconcile
```

Accounting export needs:

```text
outbox
retry/backoff
idempotent replay
dead-letter
metrics
```

## 7.11 Metrics Expansion

Golden metrics still need polishing.

Needed metrics by service:

```text
svc-wallet:
  wallet_ops_total{op}
  wallet_idempotency_replays_total
  wallet_rejected_total{reason}
  wallet_ledger_commit_seconds
  wallet_receipts_total{op}

svc-storage:
  storage_paid_write_total{status}
  storage_paid_write_bytes_total
  storage_settlement_total{status}
  storage_accounting_export_total{status}
  storage_economics_price_total{status,action}

ron-accounting:
  accounting_ingest_total{status}
  accounting_snapshot_rows
  accounting_export_total{status}
  accounting_window_status

svc-rewarder:
  rewarder_epochs_total{status}
  rewarder_wallet_emit_total{status}
  rewarder_payout_minor_total
  rewarder_residual_minor_total
  rewarder_snapshot_fetch_total{status}

ron-policy:
  economics_policy_load_total{status}
  economics_action_price_total{action,status}
  economics_validation_error_total{reason}
```

Avoid high-cardinality labels:

```text
CID
account ID
wallet txid
receipt hash
passport ID
raw user ID
raw route variants
```

---

## 8. Suggested Next Session Order

Recommended next session order:

```text
1. Add economics-enabled paid-storage live smoke.
2. Add /paid/o/estimate or equivalent pricing endpoint.
3. Add ron-policy split allocation API.
4. Wire split allocation into a mock/preview path, not full capture yet.
5. Add dashboard/admin economy summary endpoints.
6. Begin gateway/omnigate paid route integration.
7. Start passport/wallet permission design or implementation.
8. Begin b3 asset manifest/page/crab:// resolver work.
```

Fastest route to a “WOW beta”:

```text
1. Keep paid storage smoke green.
2. Add economics-enabled smoke.
3. Add dashboard/admin economy summary.
4. Add rewarder epoch summary.
5. Add b3 asset page preview.
6. Add gateway route so a user sees the whole flow through one interface.
```

---

## 9. Suggested Next Session Starting Prompt

Paste this into the next session:

```text
We are continuing RustyOnions WEB3/ROC work. The repo files are updated after the April 29 session. Use WEB3.MD and WEB3_2.MD as active north stars. CODEBUNDLE_RS.md is the complete current Rust code reference. ALLNOTES.MD contains current crate notes. CODECHECK.md and CODECOMMENTS.MD must be reviewed before code generation.

Current state:
- Paid storage live smoke is green:
  svc-wallet issue/hold → svc-storage wallet receipt verification → b3 CID-bound paid write → wallet capture/release → real ron-accounting export → accounting snapshot rows → balance conservation → replay rejection.
- Rewarder live smoke is green:
  ron-accounting reward vector → svc-rewarder compute → settlement preview → svc-rewarder HTTP emit → svc-wallet issue → balances → replay no double issue.
- ron-policy economics foundation is green:
  configs/roc-economics.toml validates, economics_policy tests 15/15 pass, and svc-storage can optionally price paid_storage_put from it.
- svc-storage economics tests 6/6 pass and full svc-storage tests pass.

Next highest-impact batch:
Add an economics-enabled paid-storage live smoke using RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml. Adjust the wallet hold amount so the current 48-byte object can capture 84 minor units under paid_storage_put pricing instead of legacy 48. Assert final balances, capture/release amounts, accounting export, object readback, and replay rejection. Keep the legacy fallback smoke green with env unset.

Preserve invariants:
ROC internal only. No ROX/Solana/external settlement. Wallet remains sole mutation front-door. Ledger remains truth. Accounting is transient usage/snapshot data. Rewarder plans/emits wallet issue requests but never mutates ledger directly. Storage stores bytes by b3 CID only. No floating-point money math. Full paste-ready files only.
```

---

## 10. Files To Provide / Upload Next Session

Minimum:

```text
CODEBUNDLE_RS.md
ALLNOTES.MD
WEB3.MD
WEB3_2.MD
TODO_WEB3.MD
```

Helpful:

```text
ALL_DOCS_COMBINED.MD
CODECHECK.MD
CODECOMMENTS.MD
```

Because some work touches non-Rust files, also provide or confirm:

```text
configs/roc-economics.toml
scripts/web3_paid_storage_live_smoke.sh
scripts/web3_accounting_rewarder_wallet_smoke.sh
crates/svc-storage/Cargo.toml
crates/ron-policy/Cargo.toml
root Cargo.toml
```

If CODEBUNDLE_RS does not include Cargo.toml or scripts, paste those explicitly.

---

## 11. Do Not Forget These Current Ports

Defaults used by live smokes:

```text
svc-wallet:
  127.0.0.1:18088

svc-rewarder:
  127.0.0.1:18090

ron-accounting:
  127.0.0.1:19600

svc-storage:
  127.0.0.1:15303
```

Logs:

```text
target/web3-smoke/ron-accounting.log
target/web3-smoke/svc-wallet.log
target/web3-smoke/svc-storage.log

target/web3-rewarder-smoke/svc-wallet.log
target/web3-rewarder-smoke/svc-rewarder.log
```

---

## 12. Current Must-Run Gates

Before ending any future WEB3 session, run the relevant subset.

Full reward path:

```bash
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Full economics policy:

```bash
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --all-targets
```

Full paid storage:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
```

Wallet direct gate:

```bash
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
```

Accounting direct gate:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal
```

---

## 13. Final Current Project Assessment

The RustyOnions WEB3 backend is now in an exciting place.

What is no longer theoretical:

```text
paid storage
wallet holds
wallet capture/release
receipt verification
context binding
accounting export
accounting snapshot rows
rewarder accounting input
rewarder deterministic payout
rewarder wallet emit
wallet issue receipts
idempotent replay protection
configurable ROC economics foundation
```

What remains is mostly turning the proven engine into product:

```text
economics-enabled live smoke
price estimate endpoints
split allocation
dashboard/admin visibility
gateway/omnigate UX
passports
b3 asset pages
crab links
paid content/site/song flows
browser extension
production auth/capabilities
recovery/outbox/idempotency persistence
```

RustyOnions is now past the hardest “can this economy actually work?” question. The answer is yes: the internal ROC loop works. The next push is making it configurable, visible, recoverable, and user-facing.

```

That’s the handoff I’d use to restart cleanly next session. The next most surgical win is the **economics-enabled paid-storage live smoke**, because it proves `configs/roc-economics.toml` is not just validated, but actually controlling live ROC settlement.
```


### END NOTE - APRIL 28 2026 - 23:00 CST



### BEGIN NOTE - APRIL 29 2026 - 01:10 CST

# RustyOnions WEB3 Carryover Notes — Session Handoff

## Session headline

This session pushed RustyOnions from “paid storage works at the internal service level” to a **real product-facing WEB3 paid storage flow**:

```text
client
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-wallet
→ ron-accounting
```

The biggest proof is now green:

```text
GET  svc-gateway /paid/o/estimate?bytes=48
→ 84 ROC minor units from configs/roc-economics.toml

POST svc-gateway /paid/o
→ omnigate
→ svc-storage
→ wallet capture/release
→ accounting export
→ b3 CID readback
→ replay rejection
```

The final live smoke proved the full chain with `capture_minor = 84`, `release_minor = 16`, `payer_balance = 16`, `payee_balance = 84`, `accounting_rows = 3`, and replay rejection as `402 payment_required`. 

---

# 1. Current project status after this session

## Updated completion estimate

```text
RustyOnions core substrate:        90–92%
WEB3 / ROC internal economy:       91–93%
Paid storage beta path:            95–97%
Gateway + omnigate WEB3 routing:   92–94%
WEB3_2 product layer overall:      70–75%
Browser extension:                 future / not counted
Overall RON beta readiness:        89–91%
```

## Practical headline

```text
RON beta is now approximately 90% complete.
```

This does **not** mean every WEB3_2 feature is done. It means the core beta foundation is now strong:

```text
ROC accounting loop: strong
wallet issue/hold/capture/release: strong
storage paid-use enforcement: strong
gateway/omnigate paid route exposure: strong
live paid storage demo: green
```

The remaining work is mostly productization, hardening, and WEB3_2 UX layers: passports, crab links, typed b3 asset pages, product response envelopes, metrics, route docs, and eventual browser extension flow.

---

# 2. Crates directly worked on this session

We directly changed and tested:

```text
svc-storage
omnigate
svc-gateway
```

We integrated live with:

```text
svc-wallet
ron-accounting
```

No direct code changes were made to `svc-wallet` or `ron-accounting` in this session, but both were exercised and proven in the final smoke.

---

# 3. Major accomplishment: `svc-storage` estimate endpoint

## Added

```text
GET /paid/o/estimate?bytes=N
```

This route returns a read-only paid storage estimate.

Example proven response:

```json
{
  "schema": "svc-storage.paid-storage-estimate.v1",
  "route": "/paid/o",
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 48,
  "amount_minor": "84",
  "minimum_hold_minor": "84",
  "pricing_mode": "roc-economics",
  "economics_policy_path": "configs/roc-economics.toml"
}
```

## Important behavior

```text
If RON_STORAGE_ROC_ECONOMICS_PATH is unset:
  legacy pricing remains max(bytes_stored, 1)

If RON_STORAGE_ROC_ECONOMICS_PATH is set:
  load config through ron-policy
  price action through configs/roc-economics.toml
  fail closed on explicit config problems
```

## Important fix

Initial implementation used:

```rust
axum::extract::Query
```

That failed because the `query` feature was not enabled for Axum in `svc-storage`. We fixed it by parsing the query from `Uri` instead. This avoided a `Cargo.toml` feature change and kept the crate lightweight.

Focused validation passed:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_estimate
```

The storage estimate test suite passed all 6 tests after the fix.

---

# 4. Major accomplishment: `omnigate` WEB3 paid routes

## Added

```text
GET  /v1/paid/o/estimate
POST /v1/paid/o
PUT  /v1/paid/o
```

## Role

`omnigate` is now the BFF/product layer for paid storage. It does **not** implement storage economics or mutate ledger state. It proxies product-facing routes to `svc-storage`.

Correct boundary:

```text
omnigate:
  BFF/proxy/product layer

svc-storage:
  paid-use enforcement, b3 CID, wallet receipt verification,
  capture/release delegation, accounting export

svc-wallet:
  actual economic mutation

ron-accounting:
  usage ingest/snapshot
```

## What omnigate now forwards

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
Idempotency-Key
body bytes
POST/PUT method
estimate query string
```

## What omnigate intentionally filters

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

## Tests proven green

The session validated:

```bash
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test paid_storage_write_proxy
cargo test -p omnigate --test paid_storage_estimate_proxy
cargo test -p omnigate --all-targets
```

Key new tests green:

```text
paid_storage_write_proxy_post_happy_path
paid_storage_write_proxy_put_happy_path
paid_storage_write_proxy_passes_storage_errors_through
paid_storage_write_upstream_connect_failure_yields_problem_502

paid_storage_estimate_proxy_happy_path
paid_storage_estimate_proxy_passes_storage_errors_through
paid_storage_estimate_upstream_connect_failure_yields_problem_502
```

The full omnigate suite also stayed green across admission, DTOs, metrics, middleware, policy gate, readiness, and receipts. 

---

# 5. Major accomplishment: `svc-gateway` WEB3 edge routes

## Added

```text
GET  /paid/o/estimate
POST /paid/o
PUT  /paid/o
```

## Corrected architecture

Early in the session, gateway estimate routing directly targeted `svc-storage`. We corrected this so product paths now go through `omnigate`:

```text
client
→ svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate

client
→ svc-gateway /paid/o
→ omnigate /v1/paid/o
→ svc-storage /paid/o
```

This is the correct WEB3_2 shape because future passport, paid access, b3 asset page, and crab-link product logic belongs at the BFF/product layer, not as direct gateway-to-storage bypasses.

## What gateway now forwards

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
Idempotency-Key
body bytes
query string
POST/PUT method
```

## What gateway filters

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

## Tests proven green

The session validated:

```bash
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test paid_storage_write_proxy
cargo test -p svc-gateway --test paid_storage_estimate_proxy
cargo test -p svc-gateway --test app_proxy
cargo test -p svc-gateway --all-targets
```

Key new tests green:

```text
paid_storage_write_proxy_post_happy_path
paid_storage_write_proxy_put_happy_path
paid_storage_write_proxy_passes_omnigate_errors_through
paid_storage_write_upstream_connect_failure_yields_problem_502

paid_storage_estimate_proxy_happy_path
paid_storage_estimate_proxy_passes_omnigate_errors_through
paid_storage_estimate_upstream_connect_failure_yields_problem_502
```

Existing `app_proxy` tests and `smoke` also stayed green, proving the paid routes did not regress the existing app proxy path. 

---

# 6. Scripts added or updated

## `scripts/web3_paid_storage_live_smoke.sh`

Purpose:

```text
Direct svc-storage paid-write smoke with:
  svc-wallet
  svc-storage
  ron-accounting
```

Proves the internal paid storage path without gateway/omnigate.

Key proof:

```text
48-byte object
capture 84
release 16
accounting export
readback
replay rejection
```

## `scripts/web3_paid_estimate_edge_smoke.sh`

Purpose:

```text
Read-only estimate smoke through:
  svc-gateway
  omnigate
  svc-storage
```

Proves product-facing estimate route:

```text
GET gateway /paid/o/estimate?bytes=48
→ omnigate
→ storage
→ roc-economics.toml
→ 84 ROC minor units
```

## `scripts/web3_paid_storage_edge_write_smoke.sh`

Purpose:

```text
Full live edge paid-write smoke through:
  svc-gateway
  omnigate
  svc-storage
  svc-wallet
  ron-accounting
```

This is now the most important beta proof script.

Final result:

```text
WEB3 edge paid storage write smoke green
path:             svc-gateway → omnigate → svc-storage → wallet/accounting
cid:              b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
object_bytes:     48
capture_minor:    84
release_minor:    16
payer_balance:    16
payee_balance:    84
accounting_rows:  3
replay_rejection: 402 payment_required
```

The final smoke output proves the complete live chain, including wallet issue, hold, capture, release, accounting export, object readback, and replay rejection. 

---

# 7. Exact final proven flow

The full final smoke proved this sequence:

```text
1. Build:
   svc-wallet
   ron-accounting
   svc-storage
   omnigate
   svc-gateway

2. Create 48-byte object:
   CID = b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57

3. Start ron-accounting:
   /healthz ok

4. Start svc-wallet:
   /healthz ok

5. Start svc-storage:
   wallet receipt verifier mode
   wallet-capture settlement mode
   HTTP accounting export mode
   ROC economics config enabled

6. Start omnigate:
   storage upstream = http://127.0.0.1:15303

7. Start svc-gateway:
   omnigate upstream = http://127.0.0.1:15305

8. Gateway estimate:
   GET /paid/o/estimate?bytes=48
   amount_minor = 84
   minimum_hold_minor = 84
   pricing_mode = roc-economics

9. Wallet issue:
   issue 100 ROC minor units to acct_user

10. Wallet hold:
   hold 100 ROC minor units
   from acct_user
   to escrow_paid_write
   context idem = storage_put:3519a6eed7dfb69a39e437b92a16e881

11. Gateway paid write:
   POST /paid/o
   body forwarded through gateway → omnigate → storage
   storage returns same b3 CID

12. Settlement:
   capture 84 from escrow_paid_write to svc_storage
   release 16 from escrow_paid_write to acct_user

13. Accounting export:
   status = exported
   event_count = 3
   HTTP status = 202

14. Usage events:
   bytes_stored = 48
   request_ok = 1
   pin_seconds = 60

15. Object readback:
   GET storage /o/<cid>
   cmp original bytes == fetched bytes

16. Final balances:
   payer = 16
   escrow = 0
   payee = 84

17. Replay rejection:
   same hold with different bytes
   rejected with 402 payment_required
   accounting row count stayed stable
```

This is the first full product-facing ROC paid storage demo chain.

---

# 8. Current known green commands

## `svc-storage`

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_estimate
```

Expected:

```text
paid_write_estimate: 6 passed
```

Recommended broader storage gate:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_estimate
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --all-targets
```

## `omnigate`

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test paid_storage_write_proxy
cargo test -p omnigate --test paid_storage_estimate_proxy
cargo test -p omnigate --all-targets
```

Expected:

```text
paid_storage_write_proxy: 4 passed
paid_storage_estimate_proxy: 3 passed
full omnigate tests green
```

Confirmed green in final session output. 

## `svc-gateway`

```bash
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test paid_storage_write_proxy
cargo test -p svc-gateway --test paid_storage_estimate_proxy
cargo test -p svc-gateway --test app_proxy
cargo test -p svc-gateway --all-targets
```

Expected:

```text
paid_storage_write_proxy: 4 passed
paid_storage_estimate_proxy: 3 passed
app_proxy: 5 passed
smoke: 1 passed
full svc-gateway tests green
```

Confirmed green in final session output. 

## Full live smoke

```bash
bash scripts/web3_paid_storage_edge_write_smoke.sh
```

Expected final proof:

```text
WEB3 edge paid storage write smoke green
capture_minor:    84
release_minor:    16
accounting_rows:  3
replay_rejection: 402 payment_required
```

Confirmed green. 

---

# 9. Important ports and service URLs from this session

The final smoke used:

```text
svc-wallet:     http://127.0.0.1:18088
ron-accounting: http://127.0.0.1:19600
svc-storage:    http://127.0.0.1:15303
omnigate:       http://127.0.0.1:15305
svc-gateway:    http://127.0.0.1:15304
```

Important routes:

```text
svc-gateway:
  GET  /paid/o/estimate?bytes=48
  POST /paid/o
  PUT  /paid/o

omnigate:
  GET  /v1/paid/o/estimate?bytes=48
  POST /v1/paid/o
  PUT  /v1/paid/o

svc-storage:
  GET  /paid/o/estimate?bytes=48
  POST /paid/o
  PUT  /paid/o
  GET  /o/<cid>

svc-wallet:
  POST /v1/issue
  POST /v1/hold
  balance/capture/release paths used by storage

ron-accounting:
  POST /v1/usage-events
  GET  /v1/snapshot
```

---

# 10. Important environment variables

## `svc-storage`

```text
ADDR
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
RON_STORAGE_PAID_SETTLEMENT_PAYEE=svc_storage
RON_STORAGE_WALLET_BASE_URL
RON_STORAGE_WALLET_BEARER=dev
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS=2000
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
RON_STORAGE_ACCOUNTING_BASE_URL
RON_STORAGE_ACCOUNTING_BEARER=dev
RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml
RON_STORAGE_ROC_ECONOMICS_ACTION=paid_storage_put
```

## `omnigate`

```text
OMNIGATE_STORAGE_BASE_URL
OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL
OMNIGATE_DEV_READY=1
OMNIGATE_BIND_ADDR
OMNIGATE_METRICS_ADDR
```

## `svc-gateway`

```text
SVC_GATEWAY_BIND_ADDR
SVC_GATEWAY_OMNIGATE_BASE_URL
SVC_GATEWAY_STORAGE_BASE_URL
SVC_GATEWAY_MAX_BODY_BYTES
SVC_GATEWAY_DECODE_ABS_CAP_BYTES
SVC_GATEWAY_DEV_ROUTES
SVC_GATEWAY_DEV_METRICS
```

## `svc-wallet`

```text
SVC_WALLET_ADDR
```

## `ron-accounting`

```text
RON_ACCOUNTING_ADDR
RON_ACC_BEARER=dev
```

---

# 11. Important fixed values from final proof

```text
Object bytes:       48
CID:                b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
Economics action:   paid_storage_put
Economics config:   configs/roc-economics.toml
Estimate amount:    84
Hold amount:        100
Capture amount:     84
Release amount:     16
Payer:              acct_user
Escrow:             escrow_paid_write
Payee:              svc_storage
Payer final:        16
Escrow final:       0
Payee final:        84
Accounting rows:    3
Replay rejection:   402 payment_required
```

---

# 12. Important invariants to preserve

## Global WEB3 invariants

```text
No ROX.
No Solana.
No public bridge.
No staking.
No liquidity.
No exchange-facing logic.
No floating-point money math.
Integer minor units only.
Basis points for percentages.
Fail closed on policy/capability errors.
Deterministic receipts.
Deterministic error taxonomy.
```

## Service boundary invariants

```text
ron-ledger:
  durable economic truth

svc-wallet:
  normal mutation front door for balances/holds/captures/releases

svc-storage:
  paid-use enforcement for storage
  b3 CID storage
  wallet receipt verification
  accounting event export
  no direct ledger mutation

ron-accounting:
  usage counters/snapshots
  not economic truth
  no direct ledger mutation

svc-rewarder:
  deterministic planning
  no direct ledger mutation

omnigate:
  BFF/product layer
  proxy/prepare/hydrate
  no direct ledger mutation

svc-gateway:
  edge ingress
  should route product paid paths through omnigate
  no wallet/ledger/storage business logic
```

## Paid storage invariants

```text
Estimate is read-only.
Write requires wallet receipt/proof.
Context-idem binds CID/body/account/asset/amount.
Same hold cannot be reused for different bytes.
Capture amount comes from storage pricing/economics.
Release amount equals hold - capture.
Storage exports usage events only after successful paid write.
Rejected replay must not add accounting rows.
```

---

# 13. Known clippy situation

For `omnigate` and `svc-gateway`, use `--no-deps` for focused clippy gates:

```bash
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Reason:

```text
ron-kernel / ron-metrics still have unrelated pre-existing clippy/pedantic lint debt when path dependencies are checked.
```

Do **not** get sidetracked cleaning all dependency clippy debt unless the explicit goal is core lint cleanup.

For crate-focused WEB3 work, `--no-deps` is the correct gate for now.

---

# 14. Best next work items

## Priority 1 — Product response envelope / UX polish

Now that the full execution path is green, the next strongest batch is to make the product-facing response easier for dashboard/browser/extension clients.

Current response is rich but internal. It contains storage/wallet/accounting fields directly. That is useful for debugging, but the product layer probably needs a stable envelope.

Recommended approach:

```text
Keep internal response fields for now.
Add a stable product-friendly envelope at omnigate or gateway.
Do not break existing pass-through tests.
Prefer omnigate for product shaping.
Keep gateway mostly edge pass-through.
```

Possible response shape:

```json
{
  "schema": "ron.paid_storage.write.v1",
  "cid": "b3:<hash>",
  "asset": "roc",
  "charged_minor": "84",
  "released_minor": "16",
  "payment_status": "settled",
  "accounting_status": "exported",
  "receipts": {
    "hold": "b3:<hash>",
    "capture": "b3:<hash>",
    "release": "b3:<hash>"
  },
  "links": {
    "read": "/o/b3:<hash>",
    "asset_page": "crab://b3/<hash>.asset"
  }
}
```

This would make the beta demo feel much more polished.

## Priority 2 — Add `/v1/paid/o/prepare`

A prepare route can bridge estimate + wallet UX without mutating wallet.

Possible route:

```text
POST /v1/paid/o/prepare
```

Possible responsibility:

```text
Given:
  bytes
  payer
  desired hold padding or exact hold
  tenant/region/pin_seconds

Return:
  estimate
  recommended hold request
  expected context fields
  next submit route
```

It should not create the hold itself. It should help the client/wallet create the correct hold.

This would be very useful for browser extension/dashboard UX.

## Priority 3 — Route documentation and beta demo docs

Document the new working flow clearly.

Suggested doc content:

```text
1. Start services.
2. Estimate cost through gateway.
3. Issue ROC.
4. Create wallet hold.
5. Upload object through gateway.
6. Read object by b3 CID.
7. Check accounting.
8. Replay rejection example.
```

This could go into a WEB3 beta demo doc, README section, or `docs/web3-paid-storage-demo.md`.

## Priority 4 — Paid route metrics

Add bounded metrics for paid routes.

Candidate metrics:

```text
svc_storage_paid_estimate_requests_total
svc_storage_paid_write_requests_total
svc_storage_paid_write_rejections_total
svc_storage_wallet_capture_total
svc_storage_wallet_release_total
svc_storage_accounting_export_total

omnigate_paid_estimate_requests_total
omnigate_paid_write_requests_total
omnigate_paid_storage_upstream_errors_total
omnigate_paid_storage_proxy_latency_seconds

gateway_paid_estimate_requests_total
gateway_paid_write_requests_total
gateway_paid_storage_upstream_errors_total
gateway_paid_storage_proxy_latency_seconds
```

Avoid high-cardinality labels like account, txid, CID, or receipt hash.

## Priority 5 — Settlement/accounting outbox hardening

The current happy path is strong, but production needs recovery handling.

Remaining hardening:

```text
capture succeeds but release fails
write succeeds but capture fails
accounting export fails after settlement
process crash between capture/release/export
retry/dead-letter/reconcile flow
admin repair tool
durable settlement/export outbox
```

This is not necessary for demo, but important before real production economics.

## Priority 6 — Passport header preservation / permission caveats

Before fully implementing RON Passports, add a test layer to ensure future passport headers pass through gateway and omnigate.

Candidate headers:

```text
x-ron-passport
x-ron-wallet-account
x-ron-permission
x-ron-spend-limit
x-ron-passport-sig
```

Eventually, passport enforcement should support:

```text
main identity passport
anonymous alternate passport
wallet-linked passport permissions
max spend caveats
action caveats
tenant/site/app caveats
expiry
```

## Priority 7 — Typed b3 asset pages / crab links

WEB3_2 requires typed asset pages:

```text
crab://b3/<hash>.image
crab://b3/<hash>.song
crab://b3/<hash>.article
crab://b3/<hash>.video
crab://b3/<hash>.comment
```

The next product-layer work should begin designing HTTP route equivalents, likely in omnigate:

```text
GET /v1/b3/:hash
GET /v1/b3/:hash.:kind
GET /v1/crab/b3/:hash
```

Important boundary:

```text
svc-storage stores bytes by CID only.
omnigate/svc-index/ron-naming hydrate manifests and metadata.
gateway routes edge requests to omnigate.
```

---

# 15. Recommended next session starting point

Best next session opening task:

```text
Start with product response envelope or /v1/paid/o/prepare.
```

My recommendation:

## Start with `/v1/paid/o/prepare`

Why:

```text
1. It directly connects estimate UX to wallet hold UX.
2. It does not mutate ledger.
3. It prepares the browser extension/dashboard flow.
4. It fits naturally in omnigate.
5. It can be tested with dummy storage/economics responses.
6. It does not disturb the green paid-write path.
```

Proposed flow:

```text
POST omnigate /v1/paid/o/prepare
→ calls storage /paid/o/estimate
→ returns:
    action
    asset
    bytes
    minimum_hold_minor
    recommended_hold_minor
    escrow account
    wallet hold request template
    submit route /v1/paid/o
```

Then add gateway proxy:

```text
POST gateway /paid/o/prepare
→ omnigate /v1/paid/o/prepare
```

Then add tests and a live smoke extension.

---

# 16. Files likely needed next session

Current uploaded files were enough this session because `CODEBUNDLE_RS.md`, `ALLNOTES.MD`, `ALL_DOCS_COMBINED.MD`, `WEB3.MD`, and `WEB3_2.MD` were available.

For the next session, useful materials are:

```text
CODEBUNDLE_RS.md
ALLNOTES.MD
ALL_DOCS_COMBINED.MD
WEB3.MD
WEB3_2.MD
configs/roc-economics.toml
current scripts directory if not included in codebundle
```

If we work on `/v1/paid/o/prepare`, likely files needed from current code:

```text
crates/omnigate/src/routes/v1/paid.rs
crates/omnigate/src/routes/v1/mod.rs
crates/svc-gateway/src/routes/paid_storage.rs
crates/svc-gateway/src/config/mod.rs
crates/svc-gateway/src/config/env.rs
crates/svc-gateway/src/state.rs
```

But if `CODEBUNDLE_RS.md` is fresh, the next session can work from that.

---

# 17. Session accomplishments by crate

## `svc-storage`

Completed:

```text
GET /paid/o/estimate
economics-backed pricing
legacy fallback pricing
Uri query parsing fix
estimate tests green
direct economics live smoke green
full edge paid-write smoke green
```

Current status:

```text
Paid storage enforcement point is beta-strong.
```

## `omnigate`

Completed:

```text
GET /v1/paid/o/estimate
POST /v1/paid/o
PUT /v1/paid/o
estimate proxy tests green
write proxy tests green
full omnigate suite green
live edge smoke green
```

Current status:

```text
BFF layer for paid storage is beta-strong.
```

## `svc-gateway`

Completed:

```text
GET /paid/o/estimate
POST /paid/o
PUT /paid/o
routes go through omnigate
estimate proxy tests green
write proxy tests green
app proxy regression green
gateway suite green
live edge smoke green
```

Current status:

```text
Edge ingress for paid storage is beta-strong.
```

## `svc-wallet`

Exercised live:

```text
issue
hold
capture
release
receipt hashes
balance checks
```

Current status in this session:

```text
Worked correctly as economic mutation front door.
```

## `ron-accounting`

Exercised live:

```text
HTTP usage ingest
snapshot
row_count 3
bytes/request/pin_seconds rows
replay rejection did not add rows
```

Current status in this session:

```text
Worked correctly as usage/accounting snapshot service.
```

---

# 18. Things to avoid next session

```text
Do not reintroduce direct gateway → storage product paid routing.
Do not put storage business logic into gateway.
Do not put ledger mutation into gateway, omnigate, or storage.
Do not add Solana/ROX/external bridge/staking/liquidity logic.
Do not use floating-point money.
Do not retry non-idempotent paid writes blindly.
Do not add high-cardinality metrics labels.
Do not break the final green smoke scripts.
Do not chase ron-kernel/ron-metrics clippy debt unless that is the explicit goal.
```

---

# 19. Final carryover summary

The project now has a real, working internal ROC paid storage demo:

```text
estimate through the edge
→ wallet hold
→ paid write through the edge
→ b3 CID object storage
→ wallet capture/release
→ accounting export
→ object readback
→ replay rejection
```

The strongest proof command is:

```bash
bash scripts/web3_paid_storage_edge_write_smoke.sh
```

The strongest proof result is:

```text
WEB3 edge paid storage write smoke green
path:             svc-gateway → omnigate → svc-storage → wallet/accounting
capture_minor:    84
release_minor:    16
accounting_rows:  3
replay_rejection: 402 payment_required
```

This session likely pushed RON beta readiness to roughly **90%**. The next session should focus on **product UX closure** rather than core architecture: `/v1/paid/o/prepare`, response envelopes, route docs, paid-route metrics, passport headers, and eventually typed b3/crab asset pages.


### END NOTE - APRIL 29 2026 - 01:10 CST



### BEGIN NOTE - APRIL 29 2026 - 13:00 CST

# RustyOnions WEB3_2 Carry-Over Notes — Product-Proof Asset Flow

**Session date:** 2026-04-29
**Current focus:** `TODO_WEB3_2.MD` product-proof layer
**Current status:** Server-side `crab://<64hex>.<asset_kind>` foundation is now mostly in place and green across the crates touched this session.

---

## 0. Big-picture session summary

This session built the first real WEB3_2 product-proof path for typed b3 asset pages.

The core product idea is now implemented across multiple crates:

```text
Public asset URL:
  crab://<64-lowercase-hex>.<asset_kind>

Internal content ID:
  b3:<64-lowercase-hex>
```

Example public URL:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Important decision locked in:

```text
Public crab asset URLs do NOT use crab://b3/<hash>.<kind>.
The canonical public format is crab://<hash>.<kind>.
The internal CID remains b3:<hash>.
```

The backend chain now exists in working form:

```text
ron-naming
  parses and canonicalizes crab://<64hex>.<asset_kind>

ron-proto
  defines shared WEB3_2 asset/manifest/page/payout/ownership DTOs

svc-index
  stores mutable pointers:
    asset CID → asset manifest CID
    site/name → site manifest CID

omnigate
  resolves crab/b3 asset routes into read-only hydrated asset-page responses

svc-gateway
  exposes public routes and proxies them to omnigate
```

This means RustyOnions now has the backend skeleton for a creator-facing typed asset page system.

---

## 1. Crates worked on this session

We worked on exactly five crates:

```text
ron-naming
ron-proto
svc-index
omnigate
svc-gateway
```

All five reached green targeted gates by the end of their batch.

---

# 2. Current completion estimate for `TODO_WEB3_2.MD`

## Overall TODO_WEB3_2 completion

```text
Official TODO_WEB3_2 completion: approximately 50–54%
```

This estimate includes server-side foundations, demo flows, docs/smokes, site flows, and Chrome extension work.

## Server-side product foundation completion

```text
Server-side WEB3_2 product foundation: approximately 78–82%
```

Reason: the core backend chain now exists:

```text
crab grammar
→ asset DTOs
→ index manifest pointers
→ omnigate asset-page resolver
→ gateway product proxy routes
```

What remains for the server-side foundation is mostly demo plumbing, site flow expansion, stronger route contracts, smokes, docs, and deeper end-to-end integration.

## Chrome extension completion

```text
Chrome extension completion: 0%
```

This is intentional. The strategy was to build server-side product primitives first, then build the extension last as a thin client over stable backend routes.

---

# 3. Completed batches / work areas

## 3.1 `ron-naming` — complete for current scope

Status:

```text
complete / green
```

What was added:

```text
AssetKind parser foundation
CrabLink parser
CrabRoute
CrabNamespace
CrabParseError
canonical crab://<64hex>.<kind> formatting
old crab://b3/<hash>.<kind> rejection
safe query normalization
named route parsing:
  crab://site/<name>
  crab://name/<name>
  crab://<name>
real DTO wire-vector tests replacing placeholder assert!(true)
```

Confirmed canonical format:

```text
crab://<64-lowercase-hex>.<asset_kind>
```

Internal CID preserved:

```text
b3:<64-lowercase-hex>
```

Final gate passed:

```text
cargo fmt
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

---

## 3.2 `ron-proto` — complete for current scope

Status:

```text
complete / green
```

What was added:

```text
asset module
AssetKind
AssetManifestV1
AssetOwnerV1
AssetMetadataV1
AssetProvenanceV1
CuratorMetadataV1
AssetOwnershipClaimV1
OwnershipClaimType
SignatureRefV1
PayoutTarget
PayoutSplitV1
PayoutRole
AssetPageV1
StorageAvailabilityV1
ReceiptRefV1
ReceiptKind
AssetPageLinksV1
AssetValidationError
canonical_crab_asset_url()
validation helpers
strict serde tests
wire-shape tests
```

Key helper:

```text
canonical_crab_asset_url(b3:<hash>, AssetKind::Image)
→ crab://<hash>.image
```

Final gate passed:

```text
cargo fmt
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

Important boundary preserved:

```text
ron-proto remains DTO-only.
No IO.
No async.
No storage client.
No wallet client.
No ledger mutation.
No floating-point money math.
All payout percentages use integer basis points.
```

---

## 3.3 `svc-index` — complete for current scope

Status:

```text
complete / green
```

What was added:

```text
asset CID → asset manifest CID pointer storage
site/name → site manifest CID pointer storage
pointer DTOs
pointer store keyspace
asset pointer GET/PUT routes
site pointer GET/PUT routes
normalization helpers
strict JSON pointer tests
store integration tests
validation/chaos tests
concurrency smoke test
```

Routes added:

```text
PUT /v1/index/assets/:asset_cid/manifest
GET /v1/index/assets/:asset_cid/manifest

PUT /v1/index/sites/:name/manifest
GET /v1/index/sites/:name/manifest
```

Final gate passed:

```text
cargo fmt
cargo clippy -p svc-index --all-targets -- -D warnings
cargo test -p svc-index --all-targets
```

Important boundary preserved:

```text
svc-index stores pointers only.
svc-index does not store raw bytes.
svc-index does not hydrate asset pages.
svc-index does not call storage directly for this flow.
svc-index does not call wallet.
svc-index does not mutate ledger.
owner_passport_subject and owner_wallet_account are references only.
```

---

## 3.4 `omnigate` — complete for current scope

Status:

```text
complete / green
```

What was added:

```text
read-only crab asset resolver
read-only b3 asset HTTP resolver
asset-page response composition
manifest pointer lookup via svc-index
storage availability HEAD probe via svc-storage
partial page support
warning support
deterministic parser errors
local resolver counters
asset-page resolver tests
```

Routes added:

```text
GET /v1/crab/resolve?url=crab://<64hex>.<kind>
GET /v1/b3/<64hex>.<kind>
```

Final gate passed:

```text
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test asset_page_resolver
cargo test -p omnigate --all-targets
```

Important boundary preserved:

```text
omnigate composes read-only product responses.
omnigate does not store raw bytes.
omnigate does not write index pointers.
omnigate does not call wallet for this route.
omnigate does not mutate ledger.
omnigate does not execute payouts.
```

Current asset-page read chain:

```text
GET /v1/crab/resolve?url=crab://<hash>.image
  → parse crab URL
  → normalize to b3:<hash>
  → GET svc-index /v1/index/assets/<hash>/manifest
  → HEAD svc-storage /o/b3:<hash>
  → compose asset-page JSON
```

---

## 3.5 `svc-gateway` — complete for current scope

Status:

```text
complete / green
```

What was added:

```text
public product route exposure
gateway → omnigate proxying
header allowlist forwarding
body forwarding
query preservation
hop-by-hop header filtering
upstream unavailable → 502 Problem
product route proxy tests
clippy cleanup in object stubs
```

Routes added/exposed:

```text
GET  /crab/resolve?url=crab://<64hex>.<kind>
GET  /b3/<64hex>.<kind>
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
```

These proxy to:

```text
GET  /v1/crab/resolve
GET  /v1/b3/<64hex>.<kind>
POST /v1/paid/o/prepare
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /v1/sites/prepare
POST /v1/sites
```

Final gate passed:

```text
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
```

Important boundary preserved:

```text
svc-gateway is public ingress/proxy only.
svc-gateway does not parse asset manifests.
svc-gateway does not hydrate pages.
svc-gateway does not call svc-index directly.
svc-gateway does not call svc-storage directly.
svc-gateway does not call svc-wallet.
svc-gateway does not mutate ron-ledger.
```

---

# 4. Current working product chain

The system now has a credible server-side read path for typed asset pages:

```text
Client / future browser extension
  → svc-gateway
    GET /crab/resolve?url=crab://<hash>.image
    or GET /b3/<hash>.image

  → omnigate
    GET /v1/crab/resolve?url=...
    or GET /v1/b3/<hash>.image

  → svc-index
    GET /v1/index/assets/<hash>/manifest

  → svc-storage
    HEAD /o/b3:<hash>

  → omnigate composes response

  → svc-gateway returns response to client
```

The response shape currently includes:

```text
schema
asset_cid
asset_kind
manifest summary
storage summary
owner summary
payout summary
links
warnings
```

This is enough to power a basic asset page UI later.

---

# 5. Confirmed canonical URL rules

Going forward, all public typed asset URLs should use:

```text
crab://<64-lowercase-hex>.<asset_kind>
```

Examples:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.video
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.music
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.article
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.post
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.comment
```

Do not use:

```text
crab://b3/<hash>.image
```

Internal content IDs still use:

```text
b3:<64-lowercase-hex>
```

---

# 6. Current asset kind vocabulary

Across `ron-proto`, `svc-index`, and `omnigate`, the broader vocabulary now supports:

```text
image
video
music
song
article
post
comment
page
site
app
manifest
```

`ron-naming` currently has a smaller initial beta set:

```text
image
song
article
video
comment
page
manifest
```

This is not a blocker yet.

Future cleanup option:

```text
Align ron-naming AssetKind with ron-proto AssetKind exactly.
```

Recommended time to do that:

```text
Before Chrome extension MVP or before publishing public route docs.
```

Not urgent for the next batch unless the next batch needs `.music` or `.post` parsing directly through `ron-naming`.

---

# 7. Major invariants preserved this session

The session respected the active WEB3 / WEB3_2 architecture rules:

```text
ROC remains internal only.
No ROX / Solana / external settlement.
No floating-point money math.
All payout percentages use integer basis points.
ron-ledger remains append-only durable truth.
svc-wallet remains the normal mutation front-door.
ron-accounting remains transient metering/snapshot input.
svc-rewarder remains deterministic planner, not ledger mutator.
Paid operations remain hold → capture → release.
No direct ledger mutation outside wallet/ledger path.
No raw bytes in svc-index.
No app-specific logic in core/kernel plumbing.
No crate crossed its ownership lane for demo convenience.
```

Product-layer invariants preserved:

```text
Public crab asset URL:
  crab://<64hex>.<asset_kind>

Internal content ID:
  b3:<64hex>

Names remain optional human pointers.
b3 hashes remain canonical object addresses.
Storage owns bytes.
Index owns pointers.
Omnigate owns hydration/composition.
Gateway owns ingress/proxying.
Wallet owns economic mutation.
Ledger owns durable economic truth.
```

---

# 8. Tests / gates passed this session

## `ron-naming`

```text
cargo fmt
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

Result:

```text
green
```

## `ron-proto`

```text
cargo fmt
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

Result:

```text
green
```

## `svc-index`

```text
cargo fmt
cargo clippy -p svc-index --all-targets -- -D warnings
cargo test -p svc-index --all-targets
```

Result:

```text
green
```

## `omnigate`

```text
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test asset_page_resolver
cargo test -p omnigate --all-targets
```

Result:

```text
green
```

## `svc-gateway`

```text
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
```

Result:

```text
green
```

---

# 9. Important bugs or issues fixed during this session

## 9.1 `ron-naming` placeholder test fixed

Problem:

```rust
assert!(true);
```

This failed under strict clippy.

Fix:

```text
Replaced placeholder dto_wire_vectors.rs with real JSON/CBOR round-trip tests.
```

---

## 9.2 `ron-naming` URL shape corrected

Initial draft used:

```text
crab://b3/<hash>.image
```

User corrected the desired public format:

```text
crab://<hash>.image
```

Fix:

```text
ron-naming parser and tests updated.
old b3/ path prefix now rejected as noncanonical.
```

---

## 9.3 `svc-index` newline/control-char bug fixed

Problem:

```text
normalize_site_name("sealobsta\n") passed because trim happened before control-char detection.
```

Fix:

```text
control characters are now checked before trim/lowercase.
```

---

## 9.4 `omnigate` clippy bool assertion fixed

Problem:

```rust
assert_eq!(page.storage.available, true);
```

Fix:

```rust
assert!(page.storage.available);
```

---

## 9.5 `svc-gateway` clippy cleanup fixed

Problems:

```text
doc_markdown lint
must_use_candidate lint
unused_async lint
uninlined_format_args lint
double_must_use lint
```

Fixes:

```text
added docs backticks
added #[must_use] where needed
removed unused async from object stubs
used inline format args
removed unnecessary #[must_use] from Router-returning functions
```

---

# 10. Disk-space / terminal guidance note

During this session, a test build temporarily failed because the machine ran out of disk space while writing Rust build artifacts.

Important user preference going forward:

```text
Do not recommend rm cleanup commands.
Use cargo clean or safer non-rm alternatives.
```

Acceptable cleanup commands:

```bash
cargo clean -p <crate-name>
cargo clean
```

For crate-specific cleanup:

```bash
cargo clean -p svc-index
cargo clean -p omnigate
cargo clean -p svc-gateway
```

Do not suggest manual `rm` cleanup commands.

---

# 11. Current TODO_WEB3_2 state

## Completed server-side foundations

```text
Batch 1 — ron-naming crab/b3 parser foundation       complete
Batch 2 — ron-proto asset/manifest/page DTOs         complete
Batch 3 — svc-index manifest pointer foundation      complete
Batch 4 — omnigate read-only asset-page resolver     complete
Batch 5 — svc-gateway product route exposure         complete
```

## Still remaining

Likely remaining TODO_WEB3_2 sections:

```text
crab://image demo plumbing
crab://site backend foundation
crab://site demo
route contracts / smoke scripts / docs
Chrome extension MVP
Chrome extension beta hardening
```

The Chrome extension should remain last.

---

# 12. Recommended next session starting point

The next logical step is:

```text
crab://image demo plumbing
```

Goal:

```text
Prove one end-to-end image asset page flow using the backend pieces we just created.
```

Recommended next target:

```text
Build a minimal demo flow where an image/object already in storage can be linked to an asset manifest pointer and resolved through gateway/omnigate as a typed asset page.
```

The next batch should avoid overbuilding. It should be a thin proof path.

---

# 13. Recommended next batch scope — `crab://image` demo

## 13.1 Goal

Create a simple end-to-end product proof:

```text
image bytes exist in svc-storage
→ image asset CID exists
→ asset manifest exists or manifest pointer is created
→ svc-index maps asset CID to manifest CID
→ omnigate resolves asset page
→ svc-gateway exposes it publicly
→ client can GET /b3/<hash>.image or /crab/resolve?url=crab://<hash>.image
```

## 13.2 What this should prove

```text
A user/developer can point to an image with:
  crab://<hash>.image

The system can return a typed asset page response showing:
  asset CID
  asset kind
  manifest pointer status
  storage availability
  owner reference if available
  payout reference if available
  canonical links
  warnings
```

## 13.3 What this should not do yet

Do not add too much yet.

Avoid:

```text
full Chrome extension
full browser UI
full paid content-view settlement
full site launch flow
full manifest editor
full passport key management
full dashboard
external chain logic
ROX / Solana / bridge logic
staking / liquidity / exchange logic
```

---

# 14. Candidate crate for next batch

The next batch likely touches one or more of:

```text
omnigate
svc-gateway
svc-storage
svc-index
tests/scripts
```

The cleanest next batch may be mostly tests/smoke route contracts, not large production code.

Potential options:

## Option A — Add a demo smoke test at `svc-gateway` or integration level

Purpose:

```text
Spin up dummy or real service surfaces and prove:
GET /b3/<hash>.image
returns asset-page JSON through gateway.
```

Pros:

```text
Validates the path we just built.
Low risk.
No new architecture.
Good proof for TODO progress.
```

Cons:

```text
May still use dummy upstreams unless full multi-service test harness exists.
```

## Option B — Add omnigate manifest hydration from actual manifest bytes

Purpose:

```text
If index pointer exists, omnigate fetches manifest bytes from storage and parses AssetManifestV1.
```

Pros:

```text
Makes asset page richer.
Moves closer to real typed asset pages.
```

Cons:

```text
Touches storage fetch behavior.
Needs careful timeout/error handling.
Could be more involved.
```

## Option C — Add image upload prepare endpoint in omnigate

Purpose:

```text
Begin POST /v1/assets/image/prepare.
```

Pros:

```text
Moves toward paid image upload demo.
```

Cons:

```text
Starts mixing upload/payment flow.
Needs wallet/storage alignment.
Riskier than a read-only demo.
```

Recommended next step:

```text
Option A first: create a narrow crab://image demo/smoke proof.
Then Option B: actual manifest hydration.
Then Option C: paid image upload prepare/write path.
```

---

# 15. Recommended next session files/codebundles to provide

Since the next likely work is the image demo path, provide these current files/codebundles in the next session:

```text
TODO_WEB3_2.MD
WEB3.MD
WEB3_2.MD
ALLNOTES.MD
ALL_DOCS_COMBINED.MD
CODEBUNDLE_RS.md
CODECHECK.md
CODECOMMENTS.MD
```

And if focusing on a smaller code subset, include/currently inspect these crates:

```text
crates/ron-naming
crates/ron-proto
crates/svc-index
crates/omnigate
crates/svc-gateway
crates/svc-storage
```

Most important code areas for next batch:

```text
svc-gateway/src/routes/product.rs
svc-gateway/tests/product_routes_proxy.rs

omnigate/src/routes/v1/crab.rs
omnigate/tests/asset_page_resolver.rs

svc-index/src/http/routes/index_manifests.rs
svc-index/src/store/mod.rs
svc-index/src/types.rs

svc-storage object GET/HEAD routes
svc-storage paid write routes
```

---

# 16. Suggested next commands before starting new code

At the beginning of next session, run a sanity sweep for the crates touched this session:

```bash
cargo fmt
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets

cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets

cargo clippy -p svc-index --all-targets -- -D warnings
cargo test -p svc-index --all-targets

cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test asset_page_resolver

cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
```

If disk space becomes an issue:

```bash
cargo clean -p svc-gateway
cargo clean -p omnigate
cargo clean -p svc-index
```

No manual file deletion commands.

---

# 17. Route map after this session

## Public gateway routes

```text
GET  /crab/resolve?url=crab://<64hex>.<kind>
GET  /b3/<64hex>.<kind>
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
```

## Omnigate routes

```text
GET /v1/crab/resolve?url=crab://<64hex>.<kind>
GET /v1/b3/<64hex>.<kind>
```

Existing paid routes remain:

```text
POST /v1/paid/o/prepare
```

Product upload/site routes are exposed through gateway but not fully implemented in omnigate yet unless existing stubs exist.

## svc-index routes

```text
PUT /v1/index/assets/:asset_cid/manifest
GET /v1/index/assets/:asset_cid/manifest

PUT /v1/index/sites/:name/manifest
GET /v1/index/sites/:name/manifest
```

---

# 18. Current route/data flow examples

## 18.1 Resolve by public crab URL through gateway

```text
GET /crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Gateway proxies to:

```text
GET /v1/crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Omnigate parses:

```text
raw_hash_hex:
  0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

asset_cid:
  b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

asset_kind:
  image
```

Omnigate calls index:

```text
GET /v1/index/assets/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/manifest
```

Omnigate calls storage:

```text
HEAD /o/b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

Then returns asset-page JSON.

---

## 18.2 Resolve by HTTP b3 route through gateway

```text
GET /b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Gateway proxies to:

```text
GET /v1/b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Omnigate follows the same hydration path.

---

# 19. What remains for a true `crab://image` demo

A complete demo should prove:

```text
1. Store/upload image bytes to svc-storage.
2. Receive or compute asset CID:
     b3:<hash>
3. Create/store asset manifest bytes.
4. Receive or compute manifest CID:
     b3:<manifest_hash>
5. Write pointer to svc-index:
     asset_cid → manifest_cid
6. Resolve through gateway:
     GET /b3/<hash>.image
7. Response shows:
     asset_cid
     asset_kind=image
     manifest.status=present
     manifest_cid
     storage.available=true
     links.crab=crab://<hash>.image
```

Current gap:

```text
We have pointer storage and resolver/proxy route foundation.
We still need a narrow demo path that wires real or test image bytes + manifest pointer together end-to-end.
```

---

# 20. What remains for `crab://site`

The site foundation has begun in `svc-index` with:

```text
site/name → manifest CID pointer
```

But not yet complete:

```text
site manifest DTO
site create/prepare logic
site asset bundle handling
site resolver route
site page hydration
gateway/omnigate route behavior beyond proxy exposure
site demo
```

Likely future chain:

```text
crab://site/<name>
or another canonical site URL form
→ gateway
→ omnigate
→ svc-index site pointer
→ svc-storage manifest/assets
→ site page response
```

Need to decide exact canonical public site URL shape later.

---

# 21. Chrome extension remains intentionally deferred

Chrome extension work should still wait until after:

```text
crab://image demo works
crab://site demo works
route contracts are documented
smoke scripts exist
gateway routes are stable
```

Reason:

```text
The extension should be a thin client over stable backend routes.
It should not force backend architecture changes.
```

Likely extension MVP later:

```text
Intercept crab:// URLs or provide omnibox/context flow.
Translate crab://<hash>.image into local gateway route.
Render or open gateway asset page response.
Show passport/wallet permission prompt later.
Support copy/open crab URL.
Support basic settings for gateway URL.
```

---

# 22. Important design risks to keep watching

## 22.1 Crate boundary drift

Do not let demo pressure cause:

```text
svc-gateway to hydrate pages
svc-index to store bytes
omnigate to mutate wallet/ledger
svc-storage to own manifest semantics
ron-proto to contain runtime logic
ron-naming to call services
```

Each crate must stay in its lane.

## 22.2 Duplicate asset-kind vocabularies

Currently `ron-naming`, `ron-proto`, `svc-index`, and `omnigate` each have some asset-kind handling.

This is acceptable for now but should be cleaned up before public beta docs.

Future improvement:

```text
Centralize canonical public asset-kind vocabulary in ron-proto or ron-naming.
Have other crates use shared DTO/parser or generated constants where practical.
```

Avoid dependency cycles.

## 22.3 Partial asset pages

Omnigate currently returns partial asset pages when manifest pointer or storage availability is missing.

This is good for UX, but future frontend should clearly display:

```text
manifest missing
storage unavailable
asset not pinned
receipt missing
owner unknown
```

Do not silently imply ownership/storage/payment status.

## 22.4 Public URL stability

The confirmed public URL shape is:

```text
crab://<hash>.<kind>
```

Do not reintroduce:

```text
crab://b3/<hash>.<kind>
```

## 22.5 Paid flows

Do not skip hold/capture/release.

Paid upload/site/content-view flows must follow:

```text
prepare/estimate
→ wallet hold
→ operation
→ capture on success
→ release on failure
→ receipt returned/stored
```

Gateway should only proxy. Omnigate/storage should enforce via wallet/receipts according to their lane.

---

# 23. Recommended next-session plan

A clean next session could proceed like this:

## Step 1 — Reconfirm green state

Run focused gates for touched crates.

## Step 2 — Build `crab://image` demo/smoke

Start with a narrow demo test or route contract proving the full read path:

```text
gateway /b3/<hash>.image
→ omnigate resolver
→ index pointer present
→ storage availability present
→ asset-page JSON present
```

Prefer a controlled integration test with dummy upstreams if full multi-service startup is too heavy.

## Step 3 — Decide whether to hydrate actual manifest bytes

After the read path is proven, decide whether omnigate should fetch the manifest bytes from storage and parse `AssetManifestV1`.

This would make asset pages richer:

```text
title
description
tags
owner
payout
provenance
curator metadata
receipts
```

## Step 4 — Add docs/smoke script

Once the demo flow is stable, add a simple route contract doc or smoke test showing:

```text
put/upload bytes
write index pointer
resolve asset page
```

## Step 5 — Then move to `crab://site`

Do site foundation after image flow is demonstrable.

---

# 24. Current “jaws drop” product story after this session

The product story is becoming concrete:

```text
A creator uploads or references an image.
The image gets a BLAKE3 content ID.
The public URL becomes:
  crab://<hash>.image

The index can point that asset hash to a manifest.
The manifest can describe ownership, payout, tags, provenance, and receipts.
Omnigate can hydrate a typed asset page.
Gateway can expose it to clients.
A future browser extension can make crab:// links feel native.
```

That is the core WEB3_2 product proof.

---

# 25. Bottom-line carry-over state

The current backend product-proof stack is in a strong state:

```text
ron-naming     green
ron-proto      green
svc-index      green
omnigate       green
svc-gateway    green
```

Current TODO_WEB3_2 progress estimate:

```text
Overall TODO_WEB3_2:              ~50–54%
Server-side product foundation:   ~78–82%
Chrome extension:                 0%, intentionally deferred
```

Next best target:

```text
crab://image demo / smoke proof
```

Recommended next-session one-liner:

```text
Continue TODO_WEB3_2 from the completed ron-naming/ron-proto/svc-index/omnigate/svc-gateway backend foundation. Next build the narrow crab://image demo path that proves gateway → omnigate → svc-index + svc-storage returns a typed asset-page response for crab://<64hex>.image, without adding Chrome extension work yet.
```


### END NOTE - APRIL 29 2026 - 13:00 CST



### BEGIN NOTE - APRIL 29 2026 - 18:00 CST

# RustyOnions WEB3_2 carry-over notes for next session — Chrome extension pivot

## 0. Current project state

We ended this session with a **full WEB3_2 product green gate pass**. The backend is now ready for Chrome extension MVP work.

The final gate proved:

```text
✅ cargo fmt
✅ omnigate clippy
✅ omnigate site_launch test: 5/5
✅ omnigate all-targets tests
✅ svc-gateway clippy
✅ svc-gateway product route proxy test: 6/6
✅ svc-gateway all-targets tests
✅ safe WEB3 product stack smoke
✅ mutation-enabled site create/resolve stack smoke
✅ WEB3_2 product green gate passed
```

The final live smoke proved the real product path:

```text
client/script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
```

The mutation-enabled site flow passed:

```text
/sites/prepare
→ /sites create
→ /sites/<unique-smoke-site>
→ hydrated site page returned
```

The final uploaded terminal output showed `WEB3_2 product green gate passed`, with both safe stack smoke and mutation-enabled site create/resolve smoke passing. 

---

## 1. Strategic state after this session

We have now completed the major server-side product proof needed before browser work.

The active TODO strategy says the browser layer should come **after** the server-side primitives exist, and the extension should be a thin client on top of stable gateway/omnigate routes rather than inventing backend logic itself. 

That means the next session can safely start the Chrome extension MVP because these backend routes are now green and stable enough to call:

```text
GET  /healthz
GET  /readyz
GET  /crab/resolve?url=...
GET  /b3/:hash.kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

The extension should call `svc-gateway`, not direct internal service routes.

---

## 2. Crates / areas completed in this session

### `omnigate`

Status:

```text
WEB3_2 product backend: green
```

Completed/proven:

```text
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

This means `omnigate` now supports:

```text
site launch prepare
site manifest creation
site manifest storage through svc-storage
site name pointer write through svc-index
site resolver/hydrator
site-page DTO response
```

Important bug fixed:

```text
site_manifest_bad_json
```

Cause:

```text
The generated manifest contained provenance/storage fields, but the resolver DTO denied unknown fields.
```

Fix:

```text
SiteManifestDocument now accepts optional provenance and storage fields.
```

Current invariant:

```text
omnigate hydrates product views and coordinates storage/index calls.
omnigate does not mutate wallet or ledger directly.
```

### `svc-gateway`

Status:

```text
WEB3_2 public product route exposure: green
```

Completed/proven public routes:

```text
GET  /crab/resolve
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

The product proxy test now passes 6/6.

Current invariant:

```text
svc-gateway is proxy-only.
It does not parse manifests.
It does not price anything.
It does not mutate wallet or ledger.
It forwards selected headers and filters hop-by-hop headers.
```

### Scripts/docs

Added or finalized:

```text
scripts/web3_product_stack_smoke.sh
scripts/web3_crab_image_smoke.sh
scripts/web3_crab_site_smoke.sh
scripts/web3_product_green_gate.sh
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md
```

The important one-command gate is:

```bash
scripts/web3_product_green_gate.sh
```

For quicker future checks:

```bash
RUN_FULL_GATE=0 scripts/web3_product_green_gate.sh
```

---

## 3. Current completion estimate

```text
Server-side WEB3_2 product foundation: ~96–97%
Overall TODO_WEB3_2 including extension: ~76–79%
Full RON beta readiness: ~89–91%
Chrome extension: 0%, ready to start
```

The extension is 0% only because we intentionally deferred it until the backend routes were stable. That was the correct call.

---

## 4. Remaining major project work

### Still remaining before full RON beta

```text
1. Chrome extension MVP
2. Chrome extension beta hardening
3. Paid image upload live smoke with real wallet hold txid
4. Production policy bundle for product routes
5. Optional richer product docs / user-facing demo guide
6. Optional dashboard integration
7. Final beta packaging and install docs
```

### Not currently in scope

Do **not** add:

```text
ROX
Solana
external settlement
public bridge
staking
liquidity
exchange logic
```

ROC remains internal-only until the closed-loop economy is proven.

---

## 5. Best next-session target

The next session should start:

```text
Batch 11 — Chrome extension MVP scaffold
```

Recommended first implementation target:

```text
apps/ron-chrome-extension/
```

or, if we want it clearly separated from Rust crates:

```text
extensions/ron-passport/
```

I recommend:

```text
apps/ron-chrome-extension/
```

because TODO_WEB3_2 already proposed that shape, and it keeps this as an app/client rather than a Rust crate.

---

# 6. Safest and most efficient Chrome extension strategy

## Recommendation

The safest and most efficient path is:

```text
Build a very small Manifest V3 extension first, using plain JavaScript/HTML/CSS.
No React.
No Vite.
No dependency chain.
No bundler.
No crypto key management in v1.
No paid spending automation.
```

Then, after it works, decide whether to upgrade to TypeScript/Vite/React.

Why this is safest:

```text
fewer dependencies
fewer build problems
less supply-chain risk
easier Chrome unpacked install
easier debugging
easier paste-ready files
no npm complexity at first
fastest path to visible demo
```

Why this is efficient:

```text
Chrome extensions are naturally small.
The first MVP only needs popup/options/background/content scripts.
The backend already owns the hard logic.
The extension can be thin.
```

The TODO document proposes a fuller TypeScript/Vite/React layout for the stronger MVP/beta, but for the **first working beta proof**, a dependency-free Manifest V3 scaffold is the better starting move. After that, if the UI grows, we can migrate to TypeScript/Vite.

---

## 7. Extension architecture recommendation

Use this shape first:

```text
apps/ron-chrome-extension/
  manifest.json
  README.md

  src/
    background.js
    content.js
    popup.html
    popup.js
    options.html
    options.js
    styles.css
    ronClient.js
    storage.js
    crab.js
```

### File responsibilities

```text
manifest.json
  Chrome Manifest V3 metadata, permissions, action popup, background service worker.

background.js
  Handles extension-level messages.
  Optionally intercepts navigation attempts or context menu events later.
  Should stay thin.

content.js
  Detects crab:// text/links on normal web pages.
  Can add small "Open with RON" affordances later.
  MVP can be minimal.

popup.html / popup.js
  Main quick UI:
    node status
    gateway URL status
    crab input
    open crab link
    resolve b3/crab asset
    quick site resolver

options.html / options.js
  Settings:
    gateway URL
    passport subject
    wallet account
    auth token/dev bearer token
    spending prompt preferences

ronClient.js
  All HTTP calls to svc-gateway:
    healthz
    readyz
    crab resolve
    b3 asset page
    image prepare
    site prepare
    site create
    site resolve

storage.js
  Safe chrome.storage.local wrapper.

crab.js
  Local UI helpers:
    validate simple crab URL shape
    convert b3/hash input to URL
    do not own canonical backend truth
```

---

## 8. MVP features, in order

### Phase 1 — installable shell

Goal:

```text
Extension loads unpacked in Chrome and can talk to gateway.
```

Implement:

```text
manifest.json
popup UI
options page
gateway URL setting
healthz/readyz check
```

Acceptance:

```text
extension loads unpacked
popup opens
gateway URL can be saved
node status shows online/offline
no console errors
```

### Phase 2 — crab resolver

Goal:

```text
Paste crab:// URL and resolve through gateway.
```

Implement:

```text
input box for crab URL
button: Resolve
GET /crab/resolve?url=...
render JSON summary
copy canonical link
open raw route if provided
```

Acceptance:

```text
crab://<hash>.image resolves
asset kind shown
asset CID shown
gateway errors shown cleanly
```

### Phase 3 — b3 asset page helper

Goal:

```text
Open typed b3 asset pages through gateway.
```

Implement:

```text
input hash/kind
GET /b3/:hash.kind
render asset page summary
copy b3 CID
copy crab://<hash>.image
```

Acceptance:

```text
GET /b3/<hash>.image works from popup
copy buttons work
bad hash shows friendly error
```

### Phase 4 — site resolver

Goal:

```text
Open crab://site-like names through gateway.
```

Implement:

```text
site name input
GET /sites/:name
render site title/root CID/manifest CID
```

Acceptance:

```text
resolves a site created by smoke script
shows manifest hydration status
shows owner/payout if present
```

### Phase 5 — image prepare

Goal:

```text
Choose image and estimate cost.
```

Implement:

```text
file picker
read file size/type
POST /assets/image/prepare
show wallet hold template
```

Acceptance:

```text
image prepare returns amount_minor
extension displays "This action needs ROC hold"
no upload happens yet
```

### Phase 6 — site prepare/create dev flow

Goal:

```text
Launch tiny dev site from known root document CID.
```

Implement:

```text
site name input
root_document_cid input
POST /sites/prepare
POST /sites
GET /sites/:name
```

Acceptance:

```text
site create works with existing root CID
site resolve works
extension shows crab://<site_name>
```

### Phase 7 — paid upload later

Do not start here.

Paid image upload requires:

```text
real wallet hold txid
clear hold prompt
capture/release proof
receipt display
```

This should come after the extension can already resolve and prepare.

---

## 9. Extension security rules

The safest version follows these rules:

```text
No silent spending.
No automatic wallet hold.
No direct ledger calls.
No direct storage truth assumptions.
No direct index mutation except through gateway/omnigate routes.
No raw private passport keys in v1.
No bearer token logging.
No secrets in console.log.
No hardcoded production dev tokens.
No broad host permissions unless needed.
No remote code execution.
No eval.
No external scripts.
```

Use `chrome.storage.local` for dev settings, but treat it as local convenience storage, not a secure hardware wallet.

### Token handling

For MVP:

```text
Dev bearer token can be optional in options.
Never log it.
Only attach it in Authorization header when set.
Show a warning that this is dev/beta mode.
```

### Spending prompts

For MVP:

```text
Prepare routes are safe.
Create/upload routes need explicit button click.
Actual paid upload needs a visible confirmation screen.
```

Do not auto-run paid upload after prepare.

---

## 10. Chrome permissions

Start with minimal permissions:

```json
{
  "permissions": [
    "storage",
    "activeTab"
  ],
  "host_permissions": [
    "http://127.0.0.1/*",
    "http://localhost/*"
  ]
}
```

Avoid broad permissions at first:

```text
Do not request <all_urls> in v1 unless content-script detection truly requires it.
Do not request webRequest/blocking in v1.
Do not request tabs unless needed.
```

For `crab://` links, Chrome will not treat custom schemes like normal HTTP pages. The safest MVP is:

```text
popup input box
content script detects visible crab:// text/links on allowed pages
right-click/open helper later
```

Do not try to replace Chrome’s protocol handling in the first batch.

---

## 11. Important product decision: URL shape

Use current canonical public format:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Example:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Internal CID remains:

```text
b3:<64 lowercase hex>
```

For site names:

```text
crab://<site_name>
```

Do not reintroduce the old public path form:

```text
crab://b3/<hash>.image
```

Some older notes still contain that earlier format. The current project decision is `crab://<hash>.<kind>`.

---

## 12. Backend API the extension should call

Base URL:

```text
http://127.0.0.1:8090
```

Configurable in options.

### Health

```http
GET /healthz
GET /readyz
```

### Resolve crab URL

```http
GET /crab/resolve?url=crab://<hash>.image
```

### Resolve typed b3 asset page

```http
GET /b3/<hash>.image
```

### Image prepare

```http
POST /assets/image/prepare
Content-Type: application/json
```

### Site prepare

```http
POST /sites/prepare
Content-Type: application/json
```

### Site create

```http
POST /sites
Content-Type: application/json
```

### Site resolve

```http
GET /sites/:name
```

### Headers to attach when configured

```text
Authorization: Bearer <token>
x-ron-passport: <passport_subject>
x-ron-wallet-account: <wallet_account>
x-correlation-id: <generated_id>
Idempotency-Key: <generated_id_for_mutations>
```

---

## 13. Suggested extension data model

Store this in `chrome.storage.local`:

```json
{
  "gatewayUrl": "http://127.0.0.1:8090",
  "passportSubject": "passport:main:dev",
  "walletAccount": "acct_dev",
  "authToken": "",
  "requireSpendConfirm": true,
  "lastCrabUrl": "",
  "lastAssetPage": null,
  "lastSitePage": null
}
```

Keep it simple for v1.

Later, for real RON Passports:

```text
main identity passport
alternate anonymous passports
wallet-linked passport permissions
spending caps
receipt history
```

But do not implement real key custody in the first extension scaffold.

---

## 14. Suggested extension UX

### Popup first screen

```text
RON Passport

Node: Online / Offline
Gateway: http://127.0.0.1:8090

[ crab:// URL input                    ]
[ Resolve ]

Quick actions:
[ Open b3 Asset ]
[ Prepare Image Upload ]
[ Resolve Site ]
[ Settings ]
```

### Asset result card

```text
Asset Page
Kind: image
CID: b3:<hash>
crab://<hash>.image

Manifest: present / missing
Storage: available / unknown
Owner: ...
Payout: ...

[ Copy CID ]
[ Copy crab:// ]
[ Open raw ]
```

### Site result card

```text
Site
Name: smoke-site-...
Root CID: b3:<hash>
Manifest CID: b3:<hash>
Hydration: hydrated

[ Copy crab://site ]
[ Copy manifest CID ]
```

### Prepare image card

```text
Image selected: test.png
Size: 12345 bytes
Type: image/png

Estimated ROC hold:
amount_minor: ...
minimum_hold_minor: ...

[ Confirm / create hold later ]
```

For now, button can say:

```text
Wallet hold creation not wired in extension yet.
Use backend wallet/dev flow.
```

---

## 15. Next session exact implementation plan

### Batch 11A — dependency-free extension scaffold

Create:

```text
apps/ron-chrome-extension/
  manifest.json
  README.md
  src/background.js
  src/content.js
  src/popup.html
  src/popup.js
  src/options.html
  src/options.js
  src/styles.css
  src/ronClient.js
  src/storage.js
  src/crab.js
```

Initial features:

```text
load unpacked
save gateway URL
check /healthz and /readyz
store passport subject
store wallet account
resolve crab URL
resolve b3 asset URL
render response summary
copy CID/link
```

No npm yet.

### Batch 11B — site tools

Add:

```text
site name input
GET /sites/:name
site prepare form
site create form using existing root_document_cid
```

### Batch 11C — image prepare

Add:

```text
file picker
detect MIME and byte length
POST /assets/image/prepare
show wallet hold template
```

### Batch 11D — content script helper

Add:

```text
detect crab:// text/anchors on pages
add tiny "Open with RON" button
send URL to background/popup resolver
```

Keep permissions narrow.

### Batch 11E — beta hardening

Add:

```text
better error states
receipt display
spending prompt UI
dev/prod mode badge
policy denial handling
node offline handling
unpacked install docs
```

---

## 16. Suggested acceptance gates for extension MVP

Because Chrome extension tests are less straightforward than Rust tests, start with manual acceptance gates:

```text
Extension loads unpacked.
Options page saves gateway URL.
Popup reads gateway URL.
Popup shows node online when stack is running.
Popup shows node offline when stack is stopped.
Popup resolves crab://<hash>.image.
Popup resolves /b3/<hash>.image equivalent.
Popup handles invalid hash cleanly.
Popup resolves a site created by web3_product_green_gate smoke.
Popup never logs auth token.
Popup never auto-spends.
```

Backend gate before extension testing:

```bash
RUN_FULL_GATE=0 scripts/web3_product_green_gate.sh
```

Full gate before committing extension milestone:

```bash
scripts/web3_product_green_gate.sh
```

---

## 17. What to attach / provide next session

For the next session, attach or keep available:

```text
CODEBUNDLE_RS.md
ALLNOTES.MD
ALL_DOCS_COMBINED.MD
WEB3.MD
WEB3_2.MD
TODO_WEB3_2.MD
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md
```

For extension work, also provide any current files after the first scaffold is created:

```text
apps/ron-chrome-extension/manifest.json
apps/ron-chrome-extension/src/*.js
apps/ron-chrome-extension/src/*.html
apps/ron-chrome-extension/src/*.css
```

---

## 18. Major caution for next session

Do not let the extension become the architecture.

The extension should be a **thin browser client**.

It should not:

```text
own wallet truth
own ledger truth
own storage truth
invent payment state
invent ownership state
invent b3 IDs without hashing actual bytes
bypass hold/capture/release
store private keys casually
silently spend ROC
```

The backend is now finally strong enough that the extension can be simple. That is the win.

---

## 19. Recommended opening prompt for next session

```text
We are ready to start Batch 11A: the Chrome extension MVP scaffold for RustyOnions WEB3_2.

Use WEB3.MD, WEB3_2.MD, TODO_WEB3_2.MD, CODEBUNDLE_RS.md, ALLNOTES.MD, and ALL_DOCS_COMBINED.MD as references.

Important current state:
- scripts/web3_product_green_gate.sh passed.
- omnigate site_launch passed 5/5.
- svc-gateway product_routes_proxy passed 6/6.
- safe WEB3 product stack smoke passed.
- mutation-enabled site create/resolve smoke passed.
- Backend route contracts are stable enough for extension MVP.

Build the extension as a thin client over svc-gateway.
Do not mutate ledger or wallet directly.
Do not implement private key custody yet.
Do not silently spend ROC.
Use Manifest V3.
For the first scaffold, prefer dependency-free plain JS/HTML/CSS unless there is a strong reason to add npm/Vite/React.
Target folder:
apps/ron-chrome-extension/

First files:
manifest.json
README.md
src/background.js
src/content.js
src/popup.html
src/popup.js
src/options.html
src/options.js
src/styles.css
src/ronClient.js
src/storage.js
src/crab.js

First MVP:
- save gateway URL
- health/ready check
- save passport subject and wallet account
- resolve crab:// URL through GET /crab/resolve?url=...
- resolve b3 asset through GET /b3/:hash.kind
- render asset/site JSON summary
- copy b3 CID and crab:// link
```

That is the cleanest, safest path forward.


### END NOTE - APRIL 29 2026 - 18:00 CST



### BEGIN NOTE - MAY 6 2026 - 17:45 CST

Below is **Set 1: Master Carryover Notes for the next instance**.

This is the broad session-level note. I would drop this into your main carryover notes / ALLNOTES-style area. The later crate-specific notes should be shorter and targeted per crate.

---

# RustyOnions / CrabLink NEXT_LEVEL Carryover Notes

## Session Master Notes — Backend Identity/Profile Chain Green Through Gateway

## 1. Active blueprint / project direction

Current active blueprint priority remains:

```text
NEXT_LEVEL.MD
TODO_WEB3_2.MD
WEB3.MD
WEB3_2.MD
```

`NEXT_LEVEL.MD` is now the main project continuation blueprint for turning the current RustyOnions + CrabLink Web3 product proof into a more complete creator-web system.

The current guiding architecture remains:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-passport / svc-wallet / svc-storage / svc-index
→ ron-ledger / ron-accounting / svc-rewarder as appropriate
```

Important invariants that were preserved in this session:

```text
No CrabLink direct calls to svc-passport.
No CrabLink direct calls to svc-wallet.
No CrabLink direct calls to ron-ledger.
No direct ledger mutation from gateway, Omnigate, storage, profile, or extension code.
svc-wallet remains the wallet mutation front door.
ron-ledger remains the durable accounting/economic truth.
Gateway remains proxy-only.
Omnigate remains facade/coordinator/proxy layer.
svc-passport owns identity/profile claim semantics.
CrabLink consumes backend truth only through svc-gateway.
No fake ROC.
No fake confirmed usernames.
No fake reputation/moderation scores.
No private key, seed phrase, spend authority, or private alt linkage exposed in public profile DTOs.
```

The main architectural achievement of this session was moving `@username` / public profile work from “frontend placeholder / design concept” into a tested backend route chain:

```text
ron-naming
→ ron-proto
→ svc-passport
→ omnigate
→ svc-gateway
```

The next major step is to make CrabLink consume the new gateway-only profile routes.

---

## 2. Crates worked on in this session

We worked on these RustyOnions crates:

```text
ron-naming
ron-proto
svc-passport
omnigate
svc-gateway
```

We also performed earlier CrabLink Chrome extension cleanup in this same instance, but the crate work above is the main RustyOnions backend progress.

---

## 3. High-level progress summary

Before this session, the project already had strong product proof around:

```text
Paid image publishing
Paid storage/pinning
Site creation
crab://site
crab://image
b3 typed image asset pages
Gateway/Omnigate paid route proxying
CrabLink full-tab browser shell
Creator workspace pages for site/image/article/video/stream/podcast scaffolds
```

This session added the missing backend foundation for NEXT_LEVEL identity/profile work:

```text
Canonical @username parser in ron-naming
crab://@username parser in ron-naming
crab://profile/@username parser in ron-naming
Expanded asset kinds in ron-naming and ron-proto
Public profile/passport DTOs in ron-proto
Username claim core in svc-passport
Public profile read response in svc-passport
svc-passport HTTP routes for profile claim/read
Omnigate facade routes for profile claim/read
svc-gateway public proxy routes for profile claim/read
Gateway /identity/me query forwarding fix
```

The currently proven backend chain is:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

at svc-gateway

↓ proxy to

POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username

at omnigate

↓ proxy to

POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username

at svc-passport
```

This is the correct boundary direction for CrabLink:

```text
CrabLink → svc-gateway → omnigate → svc-passport
```

Not:

```text
CrabLink → svc-passport
```

---

## 4. Current proof / green gates

The session ended with the relevant gates green.

### `ron-naming`

Passed:

```text
cargo fmt -p ron-naming
cargo clippy -p ron-naming --all-targets -- -D warnings
cargo test -p ron-naming --all-targets
```

Important green test coverage included:

```text
asset_kind.rs
username.rs
crab_links.rs
dto_wire_vectors.rs
normalize_idempotence.rs
```

Key `crab_links.rs` tests included:

```text
parses_profile_username_links
parses_profile_path_username_links
rejects_bad_profile_usernames
parses_typed_b3_asset_links_without_b3_path_prefix
rejects_old_b3_slash_prefix_as_noncanonical
normalizes_percent_encoding_in_query
```

### `ron-proto`

Passed:

```text
cargo fmt -p ron-proto
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo test -p ron-proto --all-targets
```

Important green test coverage:

```text
identity_profile.rs: 8 passed
```

This validated public profile/passport DTO shape and that public identity DTOs do not serialize private key, recovery, spend authority, or private alt linkage fields.

### `svc-passport`

Passed:

```text
cargo fmt -p svc-passport
cargo test -p svc-passport --test profile_claims
cargo test -p svc-passport --test profile_routes
cargo test -p svc-passport --test handlers
cargo test -p svc-passport --all-targets
cargo clippy -p svc-passport --all-targets --no-deps -- -D warnings
```

Important green test coverage:

```text
profile_claims.rs: 8 passed
profile_routes.rs: 5 passed
handlers.rs: 2 passed
all-targets: passed
focused clippy: passed
```

### `omnigate`

Passed:

```text
cargo fmt -p omnigate
cargo test -p omnigate --test passport_profile_routes
cargo test -p omnigate --all-targets
```

Important green test coverage:

```text
passport_profile_routes.rs: 5 passed
omnigate all-targets: passed
```

The Omnigate tests verified:

```text
Profile claim proxies to svc-passport.
Profile lookup proxies to svc-passport.
svc-passport 404 is preserved.
svc-passport 409 conflict is preserved.
svc-passport connect failure maps to structured 502.
Selected headers are forwarded.
Hop-by-hop headers are not forwarded.
No private identity fields are leaked.
```

### `svc-gateway`

Passed:

```text
cargo fmt -p svc-gateway
cargo test -p svc-gateway --test identity_routes_proxy
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Important green test coverage:

```text
identity_routes_proxy.rs: 7 passed
svc-gateway all-targets: passed
focused clippy: passed
```

The final Clippy issue was a `collapsible_if` lint in `identity_routes_proxy.rs`. It was fixed by collapsing the nested profile-claim/taken-username conditional.

The user also ran:

```text
cargo clean
```

This safely removed:

```text
52873 files, 11.5 GiB total
```

This was done because a prior Clippy rebuild had failed due to lack of disk space.

---

## 5. Files changed / added by crate

### `ron-naming`

Primary work:

```text
crates/ron-naming/src/lib.rs
crates/ron-naming/src/username.rs
crates/ron-naming/src/asset.rs
crates/ron-naming/src/crab.rs
crates/ron-naming/tests/username.rs
crates/ron-naming/tests/asset_kind.rs
crates/ron-naming/tests/crab_links.rs
```

Major additions:

```text
RonUsername
UsernameParseError
normalize_username()
normalize_handle()
AssetKind expanded for NEXT_LEVEL asset vocabulary
CrabLink profile route parsing
CrabNamespace::Profile
CrabRoute::Profile
crab://@username
crab://profile/@username
```

### `ron-proto`

Primary work:

```text
crates/ron-proto/src/lib.rs
crates/ron-proto/src/identity/mod.rs
crates/ron-proto/src/asset/mod.rs
crates/ron-proto/tests/identity_profile.rs
```

Major additions:

```text
PassportPublicProfileV1
PassportPublicManifestV1
PassportKindV1
UsernameStatusV1
AttributionStatus
ReputationSummaryV1
ModeratorSummaryV1
SiteModeratorRoleV1
SiteModeratorRoleAssignmentV1
PublicAssetReferenceV1
IdentityValidationError
Expanded AssetKind identity/media suffixes
```

### `svc-passport`

Primary work:

```text
crates/svc-passport/src/lib.rs
crates/svc-passport/src/profile.rs
crates/svc-passport/src/http/handlers/mod.rs
crates/svc-passport/src/http/handlers/profile.rs
crates/svc-passport/src/http/router.rs
crates/svc-passport/src/test_support.rs
crates/svc-passport/tests/profile_claims.rs
crates/svc-passport/tests/profile_routes.rs
crates/svc-passport/tests/handlers.rs
crates/svc-passport/tests/api_issue.rs
crates/svc-passport/tests/api_verify.rs
crates/svc-passport/tests/api_revoke.rs
crates/svc-passport/tests/invariants.rs
crates/svc-passport/tests/readiness.rs
```

Major additions:

```text
UsernameClaimStore
UsernameClaimRequest
UsernameClaimRecord
PublicProfileResponse
UsernameClaimStatus
PassportKind
ProfileClaimError
normalize_username()
normalize_handle()
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
GET  /v1/passport/profile/_debug
```

Also cleaned old test-contract drift:

```text
handlers.rs updated to current flat envelope issue/verify contract.
verify now includes required aud when policy requires audience.
standard Base64 decoding used for sig_b64.
assert!(true) scaffold tests replaced with real stable-contract assertions.
test_support.rs allowed dead_code because integration tests include it selectively.
```

### `omnigate`

Primary work:

```text
crates/omnigate/src/routes/v1/mod.rs
crates/omnigate/src/routes/v1/profile.rs
crates/omnigate/tests/passport_profile_routes.rs
```

Major additions:

```text
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
OMNIGATE_PASSPORT_BASE_URL
OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL alias
default svc-passport upstream: http://127.0.0.1:5307
selected header forwarding
hop-by-hop header filtering
structured 502 upstream failure mapping
```

Compile fixes during Omnigate work:

```text
header::IDEMPOTENCY_KEY does not exist, so idempotency-key is matched by string.
HeaderMap iteration fixed by returning Vec<(HeaderName, HeaderValue)> instead of consuming HeaderMap into Option<HeaderName>.
```

### `svc-gateway`

Primary work:

```text
crates/svc-gateway/src/routes/product.rs
crates/svc-gateway/tests/identity_routes_proxy.rs
```

Major additions:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

Also fixed:

```text
GET /identity/me now preserves query strings.
identity_routes_proxy.rs now tests profile claim/read edge routes.
Gateway preserves idempotency-key, x-correlation-id, x-request-id, x-ron-* headers.
Gateway filters hop-by-hop headers.
Gateway preserves upstream 404/409 profile errors.
Gateway maps upstream connect failure to structured 502 upstream_unavailable.
```

Final Clippy fix:

```text
Collapsed nested if in identity_routes_proxy.rs dummy Omnigate handler.
```

---

## 6. Route contracts now established

### Public gateway routes

These are the routes CrabLink should call next:

```text
GET  /identity/me
POST /identity/passport/bootstrap
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
GET  /wallet/:account/balance
POST /wallet/hold
GET  /crab/resolve?url=...
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

### Omnigate routes

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
GET  /v1/wallet/:account/balance
POST /v1/wallet/hold
GET  /v1/crab/resolve?url=...
GET  /v1/b3/:asset
POST /v1/paid/o/prepare
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

### svc-passport profile routes

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
GET  /v1/passport/profile/_debug
```

---

## 7. Profile claim request shape

The newly established profile claim request shape:

```json
{
  "passport_subject": "passport:main:skinnycrabby",
  "requested_username": "@SkinnyCrabby",
  "display_name": "Skinny Crabby",
  "bio": "Building the content-addressed creator web.",
  "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
}
```

Important behavior:

```text
@ prefix is accepted.
Username is normalized to lowercase.
Handle is returned with @.
Reserved names reject.
Duplicate username rejects.
A passport subject cannot claim a second different username.
Repeat claim for same passport + same username is idempotent.
```

Example successful public profile response shape:

```json
{
  "schema": "svc-passport.public-profile.v1",
  "passport_subject": "passport:main:skinnycrabby",
  "passport_kind": "main",
  "username": "skinnycrabby",
  "handle": "@skinnycrabby",
  "username_status": "confirmed",
  "display_name": "Skinny Crabby",
  "bio": "Building the content-addressed creator web.",
  "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image",
  "profile_crab_url": "crab://@skinnycrabby",
  "public_profile_cid": null,
  "reputation_score": null,
  "moderator_score": null,
  "warnings": [
    "public profile is read-only",
    "reputation and moderation scores are not computed yet"
  ]
}
```

Important truth rule:

```text
username_status = "confirmed" only when returned by backend.
CrabLink must not locally invent confirmed status.
```

---

## 8. Error contracts now established

### Reserved username

```text
HTTP 400
code: reserved_username
retryable: false
```

### Duplicate / unavailable username

```text
HTTP 409
code: username_unavailable
retryable: false
```

### Profile not found

```text
HTTP 404
code: profile_not_found
retryable: false
```

### Upstream failure from gateway to Omnigate

```text
HTTP 502
code: upstream_unavailable
retryable: true
```

### Upstream failure from Omnigate to svc-passport

```text
HTTP 502
code: passport_upstream
retryable: true
```

---

## 9. Header forwarding behavior

Both Omnigate and svc-gateway profile proxy routes intentionally forward selected headers only.

Important headers preserved:

```text
Authorization
Accept
Content-Type
Idempotency-Key / idempotency-key
x-correlation-id
x-request-id
x-ron-*
```

Important headers filtered:

```text
Host
Connection
Content-Length
Transfer-Encoding
Upgrade
Proxy-Authorization
TE
Trailer
```

Gateway and Omnigate both remain proxy-only and do not interpret or mutate identity/profile truth beyond safe routing and error mapping.

---

## 10. CrabLink work earlier in this instance

Earlier in this instance, CrabLink Chrome extension checks were also fixed.

Main issue:

```text
page.html still imported ./page-local-route-mode.js
```

That stale local route controller caused creator route regression where:

```text
crab://article
crab://video
crab://stream
crab://podcast
```

could remain stuck on a previous page despite the address changing.

Fixes applied:

```text
Removed duplicate page-local-route-mode.js imports from page.html.
Ensured creator draft pages own their own route detection.
Added readCurrentCrabUrl to page-article-draft.js.
Added exact checker safety tokens:
No b3 CID
No ROC
No wallet mutation
Removed forbidden internal-service wording from local creator draft files.
```

CrabLink checks passed after that:

```text
scripts/check-chrome.sh
scripts/package-chrome.sh
scripts/make_codebundle.sh
```

Current CrabLink state:

```text
Extension static checks/package/codebundle were green earlier in the session.
Backend profile routes are now ready for CrabLink consumption through svc-gateway.
CrabLink still needs new client/UI wiring for profile claim and read.
```

---

## 11. Current NEXT_LEVEL completion estimate

Updated estimate after this session:

```text
CrabLink Chrome MVP:                              88–91%
CrabLink product-proof / NEXT_LEVEL foundation:   93–96%
RustyOnions + CrabLink internal WEB3 beta:        84–88%
Backend identity/profile track:                   72–78%
Full NEXT_LEVEL blueprint overall:                65–70%
```

Working single-number estimate:

```text
NEXT_LEVEL overall: ~68%
```

Reason for increase:

```text
We now have the backend profile/username stack green through svc-gateway.
```

Reason it is not higher yet:

```text
CrabLink has not consumed the new profile claim/read gateway routes.
crab://@username is parsed but not fully resolved through /crab/resolve.
Site creator hydration does not yet use confirmed backend profile truth.
Profile DTOs exist but profile manifests are not yet b3-published.
public_profile_cid remains null.
Reputation/moderation scores remain null placeholders.
Alt passport backend semantics are not implemented.
.post / .comment / .article backend asset flows are not implemented.
Paid content views and view receipts are not implemented.
Site revision history is not implemented.
Provider/curator/moderator payout splits are not productized.
.video / .music / .podcast remain mostly local scaffold/product UX, not full backend publishing flows.
```

---

## 12. Recommended next high-impact milestone

The next highest-impact milestone is CrabLink consumption of the new gateway-only profile routes.

Target CrabLink route usage:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

Important frontend rule:

```text
CrabLink must call svc-gateway only.
Do not call omnigate directly.
Do not call svc-passport directly.
Do not locally mark usernames as confirmed unless backend says username_status="confirmed".
```

Likely files to inspect/update in CrabLink next:

```text
extensions/chrome/src/ronClient.js
extensions/chrome/src/page-profile*.js
extensions/chrome/src/page.js
extensions/chrome/src/storage.js
extensions/chrome/src/page-settings*.js
shared/schemas/identity-me.schema.json
shared/fixtures/identity-me.ready.sample.json
scripts/check-chrome.sh
scripts/smoke-local-gateway.sh
```

Likely new ronClient methods:

```text
claimPassportProfile(payload, options)
getPassportProfile(usernameOrHandle, options)
```

Likely UI behavior:

```text
When creating first passport, user chooses username.
CrabLink sends POST /identity/passport/profile/claim through gateway.
If response username_status="confirmed", UI displays confirmed @username.
If 409 username_unavailable, UI shows user-friendly "username unavailable".
If 400 reserved_username, UI shows user-friendly "reserved username".
Profile page can GET /identity/passport/profile/:username.
Settings/current identity card can show confirmed profile state when available.
```

Likely smoke additions:

```text
Gateway profile claim smoke
Gateway profile lookup smoke
Conflict/unavailable username smoke
Reserved username smoke
No direct svc-passport URL token in CrabLink code
```

---

## 13. Recommended next backend milestone after CrabLink profile UI

After CrabLink consumes profile claim/read routes, the next backend milestone should be:

```text
crab://@username resolver integration
```

Target public route:

```text
GET /crab/resolve?url=crab://@skinnycrabby
```

Expected chain:

```text
CrabLink address bar
→ svc-gateway /crab/resolve?url=crab://@skinnycrabby
→ omnigate /v1/crab/resolve?url=crab://@skinnycrabby
→ profile resolver
→ svc-passport public profile read
→ return public profile page / manifest-backed profile response
```

Potential Omnigate work:

```text
Update crab resolver to recognize profile CrabLink routes.
Use ron-naming CrabLink parser profile route.
Call svc-passport profile lookup.
Return profile page DTO / public profile response.
Preserve no wallet/ledger mutation.
```

Potential gateway work:

```text
Probably already covered by /crab/resolve proxy if Omnigate resolver supports it.
Add tests for crab://@username if needed.
```

Potential CrabLink work:

```text
Address bar can open crab://@username.
Profile page renderer displays public profile response.
Creator links can point to crab://@username.
```

---

## 14. Important technical lessons from this session

### Rust test ownership

We hit the same partial-move issue twice:

```rust
matches!(err, Error::Variant { name } if name == "x")
err.code()
```

This moves `name` out of `err`. Correct pattern:

```rust
assert_eq!(err.code(), "some_code");
assert!(matches!(
    &err,
    Error::Variant { name } if name == "x"
));
```

### Base64 contract in svc-passport

`svc-passport` currently uses standard Base64, not URL-safe Base64, for `sig_b64`.

Correct test import:

```rust
use base64::{engine::general_purpose::STANDARD, Engine as _};
```

### Audience requirement in svc-passport verify

Current config requires `aud`. Verify requests need top-level:

```json
{
  "aud": "svc-passport"
}
```

or whatever `cfg.passport.issuer` resolves to in tests.

### Gateway query forwarding

`/identity/me` originally dropped query strings. It now uses:

```rust
let upstream_path = with_query("/v1/identity/me", uri.query());
```

This should be remembered for future gateway proxy routes.

### Disk space

`cargo clean` was needed after target growth caused temp directory creation failures. User prefers safe cleanup commands and does not want `rm` recommendations.

Use:

```text
cargo clean
```

Do not recommend destructive `rm` cleanup.

---

## 15. Current “known good” route chain summary

The current profile claim route chain is:

```text
CrabLink next target:
POST http://127.0.0.1:8090/identity/passport/profile/claim

svc-gateway:
POST /identity/passport/profile/claim

Omnigate:
POST /v1/identity/passport/profile/claim

svc-passport:
POST /v1/passport/profile/claim
```

The current profile read route chain is:

```text
CrabLink next target:
GET http://127.0.0.1:8090/identity/passport/profile/skinnycrabby

svc-gateway:
GET /identity/passport/profile/:username

Omnigate:
GET /v1/identity/passport/profile/:username

svc-passport:
GET /v1/passport/profile/:username
```

---

## 16. Recommended next commands for next instance

Before next code changes, generate/upload fresh bundles if possible:

```text
RustyOnions:
scripts/make_codebundle.sh or equivalent current bundle script if available

CrabLink:
scripts/make_codebundle.sh
```

Then verify key green gates:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-passport --all-targets
cargo test -p omnigate --test passport_profile_routes
cargo test -p svc-gateway --test identity_routes_proxy
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Then switch to CrabLink:

```bash
cd /Users/mymac/Desktop/crablink

scripts/check-chrome.sh
scripts/package-chrome.sh
scripts/make_codebundle.sh
```

Then implement CrabLink profile client/UI wiring.

---

## 17. Recommended next implementation batch

Next batch should be CrabLink-only unless a fresh codebundle reveals missing contracts.

Suggested batch name:

```text
CrabLink gateway-only profile claim/read integration
```

Likely deliverables:

```text
extensions/chrome/src/ronClient.js
- add claimPassportProfile()
- add getPassportProfile()
- ensure gateway-only base URL
- no direct svc-passport / omnigate internal URL tokens

extensions/chrome/src/page-profile*.js
- add confirmed backend profile hydration
- display username_status accurately
- show warnings for null rep/mod scores
- show profile_crab_url when present

extensions/chrome/src/page-settings*.js or passport drawer file
- show confirmed @username from backend
- provide claim username action if no confirmed username

shared/schemas/profile/public-profile.schema.json or update existing identity schema
- add expected shape for svc-passport.public-profile.v1 response if CrabLink validates it

scripts/check-chrome.sh
- enforce no direct svc-passport URL usage
- enforce gateway profile route tokens
- maybe require profile claim/read helpers

scripts/smoke-local-gateway.sh
- optional profile claim smoke
- optional profile lookup smoke
```

Acceptance gate:

```text
CrabLink can claim @username through gateway.
CrabLink can read profile through gateway.
CrabLink displays confirmed only if backend returns confirmed.
CrabLink does not fake rep/mod.
CrabLink does not call svc-passport directly.
Chrome checks pass.
Package builds.
Codebundle generated.
```

---

## 18. Do not lose these project-specific decisions

```text
Canonical public crab asset URLs use:
crab://<64 lowercase hex>.<asset_kind>

Internal content IDs remain:
b3:<64 lowercase hex>

Names/usernames are human pointers, not canonical object identity.

@username is a discovery/profile pointer, not wallet authority.

Main passport can create sites.
Alt anonymous passports should browse/surf/comment later but should not create sites.

Profile/public passport DTOs must not expose private alt mapping.

Reputation/moderation scores can display as null/placeholder only until backend computes them.

Every content object still eventually needs BLAKE3 canonical ID.

Paid actions must use hold/capture/release semantics.

No fake ROC balances.

No direct ledger mutation outside wallet/ledger path.

No ROX/Solana/external settlement until internal ROC loop is proven.
```

---

## 19. Suggested crate-note order after this master note

Now that the master note is written, create unique crate notes in this order:

```text
1. ron-naming crate notes
2. ron-proto crate notes
3. svc-passport crate notes
4. omnigate crate notes
5. svc-gateway crate notes
6. CrabLink next-step notes
```

### END NOTE - MAY 6 2026 - 17:45 CST


### BEGIN NOTE - MAY 16 2026 - 14:35 CST

# RustyOnions / CrabLink Carryover Notes — End of Session

These notes are intended to let the next instance continue immediately without rediscovering context.

## 0. Current strategic status

We are continuing **RustyOnions + CrabLink NEXT_LEVEL / WEB3_2 product-layer work**.

The active north stars remain:

```text
CRABLINK_REFACTOR.MD
NEXT_LEVEL.MD
WEB3.MD
WEB3_2.MD
QUICKCHAIN.MD only as future/preflight blueprint
```

The current mission is still:

```text
Build and prove the internal ROC economy first.
No ROX.
No Solana.
No external settlement.
No bridge.
No staking.
No liquidity.
No exchange-facing logic.
No QuickChain implementation yet.
```

The project has now moved beyond “can the internal economy work?” into “make the working economy visible, repeatable, and productized.”

Current estimated completion:

```text
Overall RustyOnions + CrabLink internal WEB3 project:
  92–94%

NEXT_LEVEL.MD implementation:
  79–84%

QuickChain readiness:
  32–38%

QuickChain implementation:
  0–3%
```

QuickChain remains correctly locked. The latest preflight gates explicitly ended with “QuickChain preflight passed,” “QuickChain remains locked,” and “No chain/ROX/Solana bridge implementation was introduced.” 

---

# 1. The biggest wins from this session

## 1.1 Paid named-site visit is now real and repeatable

We proved the core creator-economy loop:

```text
Visitor B
→ opens/visits a creator-owned crab site
→ pays 10 ROC
→ Creator A receives 10 ROC
→ svc-wallet commits transfer
→ ron-ledger returns receipt/root
→ CrabLink displays receipt
```

This is no longer a fake UI proof. It is real wallet/ledger movement.

The latest live proofs repeatedly showed:

```text
acct_visitor_b decreases by 10 ROC
acct_dev increases by 10 ROC
txid returned
receipt_hash returned
ledger_root returned
```

The current green gate stack includes:

```text
omnigate site_visit contract tests
svc-gateway site_visit proxy tests
svc-wallet site_visit receipt replay tests
live paid site_visit smoke
```

The paid creator loop gate locks Omnigate quote/pay, Gateway proxy-only routing, and Wallet-level site_visit receipt replay. 

## 1.2 Paid storage economics mode is now green with `configs/roc-economics.dev.toml`

We fixed the paid-storage smoke path to use the correct project dev economics file:

```text
configs/roc-economics.dev.toml
```

The earlier script wrongly preferred:

```text
configs/roc-economics.toml
```

Then we fixed the paid-storage smoke parser to support:

```text
pricing_kind = "flat"
```

for:

```text
paid_storage_put
```

The current economics-mode paid-storage live proof is green:

```text
pricing mode: economics:paid_storage_put:flat
capture amount: 25
release amount: 0
WEB3 paid-storage live smoke with settlement + real accounting export green
```

It also proves accounting export and replay rejection:

```text
ron-accounting.snapshot.v1 rows present
same hold cannot be replayed for different bytes
wallet receipt paid storage context idem mismatch is rejected
```

The latest terminal proof shows flat pricing, 25 ROC capture, 0 release, accounting export, and replay-attack rejection all passing. 

## 1.3 Accounting → rewarder → wallet → ledger loop is green

The rewarder smoke now proves:

```text
ron-accounting reward snapshot vector generated
svc-rewarder computes reward epoch
replay compute is deterministic
settlement batch can be fetched
svc-rewarder emits payouts through svc-wallet
replay emit does not double issue
```

The output showed:

```text
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
acct_a=356 ROC minor units
acct_b=643 ROC minor units
payout_total=999 ROC minor units
replay=no double issue
```

This is a major internal-economy preflight proof. 

## 1.4 QuickChain preflight is green

The QuickChain preflight gate now passes with:

```text
RON_PREFLIGHT_RUN_LIVE=1
RON_PREFLIGHT_RUN_HEAVY=1
```

and with paid-storage economics mode enabled through:

```text
configs/roc-economics.dev.toml
```

Important: this is **not QuickChain implementation**. It is only proof that the internal ROC economy is becoming strong enough to eventually support QuickChain design work.

QuickChain still remains locked. 

## 1.5 Paid b3 asset `content_view` backend loop is now green

This was the final major backend addition of this session.

We added backend support for generic paid asset views:

```text
POST /content/view/quote
POST /content/view/pay
```

Public path:

```text
CrabLink / client
→ svc-gateway /content/view/quote or /content/view/pay
→ omnigate /v1/content/view/quote or /v1/content/view/pay
→ svc-index asset manifest pointer
→ svc-storage manifest object
→ manifest-derived payout recipient
→ svc-wallet /v1/transfer
→ ron-ledger receipt/root
```

The paid `content_view` green gate now passes with Clippy enabled:

```text
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

The latest proof locked:

```text
Omnigate b3 asset quote/pay contract
Manifest-derived payout recipient
Wallet transfer front-door
Nonce recovery
Wallet receipt return
Gateway proxy-only route contract
```

The uploaded terminal output confirms the `content_view` gate passed after warnings were cleaned up and the gateway doc comment was fixed. 

---

# 2. Files added or changed during this session

## 2.1 Site-visit / creator-loop backend proof files

Added or modified:

```text
scripts/web3_paid_site_visit_smoke.sh
crates/omnigate/tests/site_visit.rs
crates/svc-gateway/tests/site_visit_routes_proxy.rs
crates/svc-wallet/tests/i_14_site_visit_receipt_replay.rs
scripts/web3_nextlevel_creator_loop_green_gate.sh
```

Purpose:

```text
Make paid site_visit a repeatable backend and live-stack proof.
```

Locked behavior:

```text
Visitor B pays Creator A
payer balance decreases
creator balance increases
wallet transfer receipt returned
ledger root returned
gateway remains proxy-only
omnigate coordinates only
svc-wallet remains mutation front door
ron-ledger remains durable truth
```

## 2.2 QuickChain preflight files

Added or modified:

```text
scripts/web3_quickchain_preflight_gate.sh
```

Purpose:

```text
Verify internal ROC prerequisites before QuickChain is allowed to begin.
```

Important behaviors:

```text
QuickChain remains locked
scan avoids false-positive self-match
paid-storage economics path prefers configs/roc-economics.dev.toml
auto falls back safely when config is absent
live preflight can run while main dev stack is running
```

## 2.3 Paid-storage smoke file

Modified:

```text
scripts/web3_paid_storage_live_smoke.sh
```

Purpose:

```text
Support dev economics flat pricing in addition to legacy and per-byte modes.
```

Now supports:

```text
legacy:max(bytes,1)
economics:<action>:flat
economics:<action>:per_byte
economics:<action>:per_byte_plus_minimum
```

Current validated dev mode:

```text
economics:paid_storage_put:flat
hold amount: 25
capture amount: 25
release amount: 0
```

## 2.4 Product-route documentation

Added:

```text
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
```

Purpose:

```text
Document stable public route surfaces and invariants.
```

This doc should be updated as new product routes become live. It should not become aspirational fiction; keep it tied to tested routes.

## 2.5 Paid content-view backend files

Added or modified:

```text
crates/omnigate/src/routes/v1/content_view.rs
crates/omnigate/src/routes/v1/mod.rs
crates/svc-gateway/src/routes/product.rs
crates/omnigate/tests/content_view.rs
crates/svc-gateway/tests/content_view_routes_proxy.rs
scripts/web3_nextlevel_content_view_green_gate.sh
```

Purpose:

```text
Add backend quote/pay support for paid b3 asset views.
```

Routes added:

```text
omnigate:
POST /v1/content/view/quote
POST /v1/content/view/pay

svc-gateway:
POST /content/view/quote
POST /content/view/pay
```

Current test proof:

```text
cargo test -p omnigate --test content_view
cargo test -p svc-gateway --test content_view_routes_proxy
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

---

# 3. Important implementation details from this session

## 3.1 Content-view route shape

The new `content_view` backend route expects b3-backed assets.

Accepted request shape examples:

```json
{
  "asset_crab_url": "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
  "payer_account": "acct_visitor_b",
  "viewer_wallet_account": "acct_visitor_b",
  "viewer_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_creator",
  "max_amount_minor": "5",
  "client_idempotency_key": "content-view-quote-test"
}
```

Payment example:

```json
{
  "asset_crab_url": "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
  "payer_account": "acct_visitor_b",
  "viewer_wallet_account": "acct_visitor_b",
  "viewer_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_creator",
  "amount_minor": "5",
  "asset": "roc",
  "quote_id": "content-view-test-quote",
  "quote_hash": "quotehash",
  "client_idempotency_key": "content-view-pay-test"
}
```

Current default dev price:

```text
OMNIGATE_CONTENT_VIEW_PRICE_MINOR=5
```

Default asset:

```text
roc
```

Default action:

```text
content_view
```

Accepted payout action variants in manifest:

```text
content_view
asset_view
view
<asset_kind>_view
```

Examples:

```text
article_view
post_view
comment_view
image_view
```

## 3.2 Content-view manifest tolerance

We discovered the first version of `AssetManifestDocument` was too strict.

Problem:

```text
serde(deny_unknown_fields)
```

on manifest DTO caused real/evolved manifest fields such as:

```text
receipts
receipt_refs
provenance
storage
payout.splits
```

to trigger bad gateway responses.

Fix:

```text
Request DTOs remain strict.
Stored manifest documents are tolerant.
Critical economic fields are explicitly validated.
Unknown manifest fields are ignored by Serde.
```

Do not reintroduce `deny_unknown_fields` on stored manifest DTOs unless every real manifest field is accounted for.

## 3.3 Content-view JSON macro recursion issue

The first `content_view` response used one very large nested `serde_json::json!` call.

Problem:

```text
recursion limit reached while expanding json_internal!
```

Fix:

```text
Split response construction into smaller json! chunks:
policy
asset_page
quote
next
payment
receipt
response
```

Do not solve this by raising the crate recursion limit.

## 3.4 Gateway content-view route

`svc-gateway` remains proxy-only.

Added public routes:

```text
POST /content/view/quote
POST /content/view/pay
```

Proxied to:

```text
POST /v1/content/view/quote
POST /v1/content/view/pay
```

Gateway must not:

```text
parse manifests
select payout recipients
calculate prices
call wallet directly
mutate ledger
store assets
perform accounting
compute rewards
```

Gateway should only:

```text
preserve selected auth/idempotency/x-ron headers
preserve body
filter hop-by-hop inbound headers
return upstream response
```

The proxy test confirms this behavior.

---

# 4. Current green gates

## 4.1 Paid creator loop

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions
bash scripts/web3_nextlevel_creator_loop_green_gate.sh
```

Optional live stack proof:

```bash
RON_RUN_LIVE_SITE_VISIT=1 \
SITE_NAME=ron7 \
PAYER_ACCOUNT=acct_visitor_b \
RECIPIENT_ACCOUNT=acct_dev \
bash scripts/web3_nextlevel_creator_loop_green_gate.sh
```

Direct live proof:

```bash
SITE_NAME=ron7 \
PAYER_ACCOUNT=acct_visitor_b \
RECIPIENT_ACCOUNT=acct_dev \
bash scripts/web3_paid_site_visit_smoke.sh
```

Important side effect:

```text
Every live site_visit smoke spends 10 ROC from acct_visitor_b to acct_dev.
```

## 4.2 QuickChain preflight

Run base preflight:

```bash
cd /Users/mymac/Desktop/RustyOnions
bash scripts/web3_quickchain_preflight_gate.sh
```

Run live preflight with stack running:

```bash
RON_PREFLIGHT_RUN_LIVE=1 \
bash scripts/web3_quickchain_preflight_gate.sh
```

Run full heavy + live preflight:

```bash
RON_PREFLIGHT_RUN_HEAVY=1 \
RON_PREFLIGHT_RUN_LIVE=1 \
bash scripts/web3_quickchain_preflight_gate.sh
```

Run forced paid-storage economics mode:

```bash
RON_PREFLIGHT_RUN_LIVE=1 \
RON_PREFLIGHT_PAID_STORAGE_USE_ECONOMICS=1 \
bash scripts/web3_quickchain_preflight_gate.sh
```

Expected economics path:

```text
/Users/mymac/Desktop/RustyOnions/configs/roc-economics.dev.toml
```

Expected paid-storage proof:

```text
pricing mode: economics:paid_storage_put:flat
capture amount: 25
release amount: 0
```

## 4.3 Paid content-view backend gate

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p omnigate --test content_view
cargo test -p svc-gateway --test content_view_routes_proxy
bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Strict scoped Clippy gate:

```bash
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Latest status:

```text
PASS
```

The latest output confirms:

```text
clippy omnigate content_view target only: pass
clippy svc-gateway content_view proxy target only: pass
omnigate content_view contract tests: 3 passed
svc-gateway content_view proxy tests: 1 passed
paid content_view gate: passed
```

## 4.4 CrabLink gate

Usual gate:

```bash
cd /Users/mymac/Desktop/crablink
scripts/check-chrome.sh
scripts/package-chrome.sh
```

Do not assume exact script names if repo changed; verify scripts in the CrabLink repo before running.

---

# 5. Stack-running policy

The main dev stack can remain running while running most of these gates.

This is important because restarting the stack can wipe transient local assets/sites and force recreation.

Safe to run while the main stack is up:

```text
cargo tests
content_view backend tests
creator-loop contract tests
QuickChain preflight
paid site_visit live smoke
paid storage smoke
rewarder smoke
CrabLink check/package scripts
```

Important nuance:

```text
site_visit live smoke uses the running main stack and spends real local ROC.
paid-storage smoke starts isolated temporary services on separate ports.
rewarder smoke starts isolated temporary services on separate ports.
```

Temporary smoke ports observed:

```text
ron-accounting: 127.0.0.1:19600
svc-wallet:     127.0.0.1:18088
svc-storage:    127.0.0.1:15303
svc-rewarder:   127.0.0.1:18090
```

Do not run two live smoke scripts at the exact same time, because temp ports can collide.

The `curl: (7)` messages during paid-storage smoke startup are normal polling while temporary services boot, as long as the service later becomes healthy and the script passes.

---

# 6. Hard invariants to carry forward

## 6.1 Do not touch `ron-kernel` for product work

This is very important.

We saw `ron-kernel` appear during Clippy because Cargo still checks dependencies enough to typecheck them. That does **not** mean we should edit kernel.

The standing rule remains:

```text
Do not touch ron-kernel in NEXT_LEVEL / CrabLink / content_view product work.
```

Existing notes already warn that kernel cleanup is real work but requires a dedicated kernel-only session. 

If Clippy surfaces `ron-kernel` during a product gate, do not patch kernel. Scope the product gate instead.

Good scoped examples:

```bash
cargo clippy -p omnigate --test content_view --no-deps -- -D warnings
cargo clippy -p svc-gateway --test content_view_routes_proxy --no-deps -- -D warnings
```

Bad broad example for this product session:

```bash
cargo clippy -p omnigate --all-targets -- -D warnings
```

## 6.2 CrabLink must remain a thin client

CrabLink must not become a backend.

Do not add to CrabLink:

```text
ledger logic
wallet accounting truth
storage internals
index internals
reward calculation
policy calculation truth
private settlement logic
```

CrabLink should display backend-derived truth and submit explicit user-approved actions. 

## 6.3 No silent ROC spending

Paid actions must require explicit user confirmation.

CrabLink must not silently:

```text
hold ROC
capture ROC
release held ROC
issue ROC
burn ROC
transfer ROC
```

All real mutations must go through backend wallet/ledger surfaces. 

## 6.4 Integer money only

Money remains integer minor units.

Backend truth should remain:

```text
available_minor_units
held_minor_units
amount_minor
```

Do not use floating-point math for monetary truth. 

## 6.5 Gateway-only boundary for CrabLink

CrabLink should call:

```text
svc-gateway public route
```

not internal services.

Avoid direct CrabLink calls to:

```text
svc-storage
svc-index
svc-wallet
svc-passport
ron-ledger
ron-accounting
svc-rewarder
```

Preferred flow:

```text
CrabLink
→ svc-gateway public route
→ omnigate/internal service boundary
```

This remains a core product/security invariant. 

---

# 7. Current known issues and cautions

## 7.1 `ron-kernel` Clippy debt exists but is out of scope

A broad Clippy run can expose many `ron-kernel` warnings/errors because `ron-kernel` has strict crate-level lints.

Do not mix this into NEXT_LEVEL product work.

Future dedicated kernel-only session may clean:

```text
doc_markdown
must_use_candidate
missing_errors_doc
missing_panics_doc
needless_continue
new_without_default
unnecessary_cast
manual_clamp
inline_always
unused_async
too_many_lines
```

But that must be its own session.

## 7.2 Main stack assets may be transient

Do not restart the stack unless needed.

The user prefers keeping the stack running because restarting can lose assets/sites, requiring recreation.

The next instance should preserve this workflow.

## 7.3 Paid site_visit smoke spends ROC every time

Repeated live smoke runs have progressively reduced:

```text
acct_visitor_b
```

and increased:

```text
acct_dev
```

by 10 ROC per run.

This is expected and useful, but next instance should remember it is real local wallet movement.

## 7.4 Content-view backend is green, but live CrabLink UI is not wired yet

Current content-view status:

```text
Backend route contract: green
Gateway proxy route: green
Clippy scoped gate: green
CrabLink UI integration: not yet done
Live stack content_view smoke: not yet added
```

The next product step is CrabLink UI wiring.

## 7.5 Site rendering notes from earlier still matter, but content-view is now the priority

Earlier notes said the next CrabLink work should include schema-aware rendering for `omnigate.site-page.v1`, with Developer JSON preserved. 

However, this session moved the backend forward to `content_view`. The next session should decide whether to:

```text
A. Wire paid content_view UI first
B. Polish named site-page rendering first
```

My recommendation is to do **content_view UI first**, because the backend route is fresh, green, and waiting for CrabLink integration.

---

# 8. Recommended next-session opening prompt

Use this framing at the start of the next session:

```text
We are continuing RustyOnions + CrabLink NEXT_LEVEL product-layer work.

Do not touch ron-kernel.

QuickChain remains locked.

The latest RustyOnions backend gates are green:
- paid site_visit creator loop
- paid storage economics flat mode using configs/roc-economics.dev.toml
- accounting → rewarder → wallet emit with no double issue
- QuickChain preflight heavy/live
- paid content_view backend quote/pay
- gateway content_view proxy
- scoped Clippy content_view gate

The next task is to wire CrabLink paid content_view UI:
- article/post/comment/image asset pages should call /content/view/quote
- display price, recipient, and asset metadata
- require explicit user click to pay
- call /content/view/pay
- unlock full content only after wallet receipt
- refresh balance after payment
- display txid, receipt_hash, and ledger_root
- preserve Developer JSON
- do not call internal services directly
```

---

# 9. Recommended next-session order

## Step 1 — Regenerate/upload fresh references

Before coding, refresh both repos.

RustyOnions:

```bash
cd /Users/mymac/Desktop/RustyOnions

scripts/make_codebundle.sh
scripts/collect_notes.sh
scripts/collect_alldocs.sh
```

CrabLink:

```bash
cd /Users/mymac/Desktop/crablink

scripts/check-chrome.sh
scripts/package-chrome.sh
scripts/make_codebundle.sh
```

If script names differ, inspect `scripts/` and use the current actual script names.

## Step 2 — Re-run the current Rust backend gate if needed

RustyOnions:

```bash
cd /Users/mymac/Desktop/RustyOnions

RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Optional broader preflight:

```bash
RON_PREFLIGHT_RUN_HEAVY=1 \
RON_PREFLIGHT_RUN_LIVE=1 \
bash scripts/web3_quickchain_preflight_gate.sh
```

Do not run broad kernel Clippy.

## Step 3 — Inspect CrabLink current asset-page code

Find the files responsible for asset page rendering and paid site access.

Likely files from recent work:

```text
CODEBUNDLE_CHROME_EXTENSION.MD
extensions/chrome/src/*
siteVisitClient.js
SiteVisitAccess.jsx
SiteRender.jsx
article/post/comment/image page components
gateway client helpers
balance chip hooks
receipt display components
```

The exact current filenames must be confirmed from the latest CrabLink codebundle.

## Step 4 — Add a CrabLink `contentViewClient`

Target public gateway endpoints:

```text
POST /content/view/quote
POST /content/view/pay
```

Do not call:

```text
/v1/content/view/quote
/v1/content/view/pay
svc-wallet directly
svc-index directly
svc-storage directly
```

Client should probably expose:

```js
quoteContentView({ assetCrabUrl, payerAccount, viewerPassportSubject, recipientAccount, maxAmountMinor })
payContentView({ assetCrabUrl, payerAccount, recipientAccount, amountMinor, quoteId, quoteHash })
```

Headers should include:

```text
authorization / dev bearer if existing client uses it
content-type: application/json
accept: application/json
x-ron-passport
x-ron-wallet-account
idempotency-key
x-correlation-id if existing helper supports it
```

## Step 5 — Add paid content-view UI component

Suggested component:

```text
ContentViewAccess.jsx
```

Responsibilities:

```text
show paid-view quote card
show price, recipient, asset kind, asset CID
show "Pay 5 ROC" button
disable button while paying
show backend error if quote/pay fails
unlock content only after payment response has wallet receipt
display txid / receipt_hash / ledger_root
refresh balance after success
```

No silent spend.

No automatic pay on page load.

## Step 6 — Wire asset pages

Targets:

```text
article asset page
post asset page
comment asset page
image asset page
```

The first practical target should probably be:

```text
article
```

Then extend to:

```text
post
comment
image
```

Do not try to wire every asset kind if the batch gets too large. It is better to prove article first.

## Step 7 — Add local CrabLink validation

Run:

```bash
cd /Users/mymac/Desktop/crablink

scripts/check-chrome.sh
scripts/package-chrome.sh
```

Then reload Chrome extension.

Manual test:

```text
1. Keep RustyOnions main stack running.
2. Open CrabLink.
3. Navigate to a known b3 article/post/comment asset.
4. Confirm quote appears.
5. Confirm no payment happens automatically.
6. Click Pay.
7. Confirm full content unlocks only after receipt.
8. Confirm balance refreshes.
9. Confirm receipt displays.
10. Confirm Developer JSON remains available.
```

---

# 10. Acceptance criteria for the next CrabLink `content_view` batch

The next batch should not be considered complete until:

```text
scripts/check-chrome.sh passes
scripts/package-chrome.sh passes
CrabLink loads normally
Known b3 asset page loads
Content-view quote appears
Quote amount is integer ROC minor units / display amount
Recipient is manifest/backend-derived
No silent payment happens
Pay button calls /content/view/pay through gateway
Wallet receipt is displayed
txid is displayed
receipt_hash is displayed
ledger_root is displayed if present
Balance chip refreshes after payment
Full/protected content unlocks only after payment success
Errors are visible and understandable
Developer JSON remains available
No direct calls to svc-wallet/svc-index/svc-storage/ron-ledger are added
QuickChain remains untouched
ron-kernel remains untouched
```

---

# 11. Suggested future Rust backend follow-ups after CrabLink UI

Do these after CrabLink can use `content_view`.

## 11.1 Add live `content_view` smoke

Add something like:

```text
scripts/web3_paid_content_view_smoke.sh
```

Purpose:

```text
Use a real manifest-backed asset on the running stack.
Quote 5 ROC.
Pay from visitor account to creator account.
Assert payer -5 and creator +5.
Assert wallet receipt/hash/root.
```

Potential challenge:

```text
Need a reliable known asset manifest pointer in svc-index and manifest in svc-storage.
```

This might require a helper that creates or repairs a test article asset, similar to the site-visit smoke creating/repairing `crab://ron7`.

## 11.2 Add content_view to QuickChain preflight

Only after live content_view smoke exists:

```text
scripts/web3_quickchain_preflight_gate.sh
```

could include:

```text
live paid content_view smoke
```

But do not add it before the live smoke is stable.

## 11.3 Add wallet receipt replay test for content_view

Could mirror:

```text
crates/svc-wallet/tests/i_14_site_visit_receipt_replay.rs
```

with:

```text
i_15_content_view_receipt_replay.rs
```

But since content_view is just a wallet transfer like site_visit, this is lower priority than UI wiring.

## 11.4 Evolve pricing from fixed env var to economics config

Current content_view pricing is:

```text
OMNIGATE_CONTENT_VIEW_PRICE_MINOR=5
```

Future goal:

```text
configs/roc-economics.dev.toml
[actions.content_view]
```

But do not overcomplicate the next UI batch. Fixed 5 ROC is acceptable for first proof.

---

# 12. Suggested future CrabLink UX polish after content_view

After paid content_view UI works:

```text
1. Polish b3 asset pages as full product pages.
2. Add beautiful article/post/comment readers.
3. Add receipt drawer/page instead of cluttering passport dropdown.
4. Keep passport dropdown minimal.
5. Add dedicated receipts page.
6. Add library page for created/visited assets.
7. Improve profile page display and creator ownership links.
8. Improve named site rendering.
9. Build guided create-site UI.
10. Support content embeds like crab images/articles/comments.
```

Earlier notes already emphasized that CrabLink should preserve Developer JSON and add schema-aware site rendering for `omnigate.site-page.v1`. 

---

# 13. Product architecture checkpoint

Current proven product loops:

```text
Paid storage/pinning:
  hold → storage write → capture/release → accounting export

Paid named-site visit:
  quote → pay → wallet transfer → ledger receipt → UI receipt

Accounting/rewarder:
  accounting snapshot → rewarder plan → wallet issue batch → no double issue

Paid b3 content_view:
  backend quote/pay → manifest payout recipient → wallet transfer → receipt
```

What is still missing for “full product feel”:

```text
CrabLink paid content_view UI
live content_view smoke
durable local asset/site persistence across restarts
full asset-page UX
public profile/passport polish
payout split UX/config maturation
moderation/reputation surfaces
receipts page
library/catalog page
site creation UI polish
```

---

# 14. Do not regress checklist

Do not regress:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
POST /content/view/quote
POST /content/view/pay
POST /paid/o with economics flat mode
configs/roc-economics.dev.toml preference
gateway proxy-only boundary
wallet mutation front door
ron-ledger as economic truth
integer minor units only
nonce/idempotency handling
receipt_hash b3 format
ledger_root return
accounting export rows
rewarder no-double-issue replay
CrabLink no-silent-spend rule
Developer JSON availability
ron-kernel untouched
QuickChain locked
```

---

# 15. Best immediate next task

The next session should begin with:

```text
Wire CrabLink paid content_view UI for article asset pages first.
```

Why article first:

```text
article is the clearest paid-reader product
less visual complexity than image/video/music
good foundation for post/comment
directly proves creator content monetization
```

Ideal first UI behavior:

```text
Open crab://<hash>.article
CrabLink resolves asset
CrabLink sees backend asset metadata
CrabLink shows "Paid content view — 5 ROC"
User clicks Pay
CrabLink calls /content/view/pay
Backend returns wallet receipt
CrabLink unlocks article content
CrabLink shows txid/receipt_hash/ledger_root
Balance refreshes
```

Do not start QuickChain yet.


### END NOTE - MAY 16 2026 - 14:35 CST



