
### BEGIN NOTE - APRIL 27 2026 - 15:40 CST

# ron-accounting NOTES.MD — Gold Foundation / Carryover Notes

Date: 2026-04-27  
Crate: `ron-accounting`  
Workspace path: `crates/ron-accounting`  
Project: RustyOnions / WEB3 / ROC internal value plane  
Current status: **Gold foundation reached**  
Estimated crate completion: **88–91% complete**  
Recommended next WEB3 focus: **value-plane integration test: accounting → rewarder → wallet → ledger**

---

# 0. Executive Summary

`ron-accounting` has been taken from scaffold/stub state into a functioning transient metering and sealed-snapshot library for the internal ROC economy.

The crate now supports the core intended accounting pipeline:

```text
usage events
→ normalized labels
→ transient counters
→ deterministic windowed sealed slices
→ ordered export lanes
→ ACK/NACK-safe exporter routing
→ optional WAL feature path
→ rewarder-compatible snapshot DTO
→ deterministic reward projection
→ stable accounting↔rewarder interop vector
→ canonical JSON + b3 CID proof
````

The crate passed the full local quality gate:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal
```

Final observed test status:

```text
strict clippy passed
normal tests passed
WAL-feature tests passed
examples built and ran as test targets
bench smoke passed

normal mode:
  2 loom/model tests passed
  3 property tests passed
  37 unit tests passed
  record bench smoke passed
  seal bench smoke passed
  examples/minimal.rs built
  examples/export_to_mock.rs built

wal feature mode:
  2 loom/model tests passed
  3 property tests passed
  38 unit tests passed
  record bench smoke passed
  seal bench smoke passed
  examples/minimal.rs built
  examples/export_to_mock.rs built
```

The terminal output confirms the current gate is green, including clippy, normal test suite, WAL-feature test suite, 37 normal unit tests, 38 WAL-feature unit tests, 3 prop tests, 2 model tests, examples, and bench smoke. 

This crate is now a **Gold foundation** crate: internally coherent, test-covered, deterministic, and aligned with the WEB3 value-plane architecture. It is not yet 100% production-complete because the remaining work is mostly inter-service integration and production adapters, not core accounting logic.

---

# 1. Strategic Role in WEB3 / ROC

`ron-accounting` belongs to the RustyOnions internal ROC value plane.

Its role is:

```text
collect usage/metering signals
normalize and bound labels
aggregate transient counters
seal deterministic time-window snapshots
export snapshots/counters safely
feed svc-rewarder and pricing/enforcement logic
```

Its role is **not**:

```text
not a wallet
not a ledger
not a settlement engine
not a source of durable economic truth
not an external-chain bridge
not a Solana/ROX component
```

The WEB3 blueprint explicitly defines `ron-accounting` as metering and pricing input, while `ron-ledger` remains append-only truth and `svc-rewarder` consumes accounting/policy signals. 

Correct boundary:

```text
ron-accounting:
  transient counters
  time windows
  sealed usage/contribution snapshots
  interop DTOs for rewarder/pricing

svc-rewarder:
  deterministic reward epochs
  policy validation
  payout calculation
  settlement intent planning
  wallet-compatible issue requests

svc-wallet:
  mutation front-door
  auth/policy
  nonce/idempotency
  issue/transfer/burn/hold/capture/release
  receipts and balances

ron-ledger:
  durable append-only economic truth
  replay
  conservation
  balances
```

The correct production flow remains:

```text
usage events / counters
→ ron-accounting window
→ sealed accounting snapshot
→ svc-rewarder compute
→ wallet-compatible issue requests
→ svc-wallet /v1/issue
→ ron-ledger durable commit
```

This flow matches the prior `svc-rewarder` carryover notes and preserves the rule that rewarder/wallet/ledger own reward settlement truth, not accounting. 

---

# 2. Design Principles Locked In

## 2.1 ROC-only

This work is for the internal ROC value plane only.

Explicitly not included:

```text
ROX
Solana
external chain settlement
staking
liquidity pools
exchange-facing bridge logic
public governance
on-chain mint/burn
```

External ROX/Solana integration remains deferred until the very end of the broader WEB3 plan.

## 2.2 Accounting is transient

Accounting data is metering input, not economic finality.

The crate can seal snapshots, compute CIDs, export DTOs, and feed rewarder, but it must not become a ledger. It must not hold balances, enforce account non-negativity, issue tokens, burn tokens, or mutate durable value state.

## 2.3 Determinism first

The same inputs should produce the same sealed slice digest, reward snapshot canonical bytes, and interop vector CID.

Determinism currently exists across:

```text
label normalization
component normalization
route normalization
window alignment
sealed slice ordering
slice digest calculation
reward snapshot canonicalization
account ordering
reward projection aggregation
interop vector canonical JSON
b3 CID generation
```

## 2.4 Integer-only accounting

No floating point was introduced.

All counter values are integer values:

```text
u64 counter values
u64 bytes_stored
u64 bytes_served
u64 uptime_seconds
u64 request/cpu counters
u128 pool_minor_units parsing for reward snapshot validation
string-encoded amount/pool fields at JSON boundary
```

## 2.5 Boundedness and safety

The crate is designed around bounded/checked behavior:

```text
bounded labels
normalized labels
checked arithmetic
overflow rejection
bounded exporter lanes
bounded ACK LRU
feature-gated WAL quotas
explicit readiness gates
no unsafe code
no locks across .await lint enforced
```

## 2.6 Amnesia compatibility

The crate supports the amnesia posture conceptually by keeping core accounting logic library-first and not silently persisting unless the WAL feature and WAL config are used.

Remaining production work should preserve:

```text
amnesia mode = no silent disk persistence
WAL off by default unless explicitly enabled
any persistence must be explicit/configurable
```

## 2.7 Library-first

This crate currently remains a library-first accounting engine.

That is correct. The docs mention future HTTP/UDS/OAP adapter surfaces, but adapters should not corrupt the core library boundary or turn the crate into a service runner.

---

# 3. What We Built — Batch by Batch

## 3.1 Batch 1 — Core foundation

Batch 1 filled the original scaffold with working core files.

Major functional areas established:

```text
src/lib.rs
src/errors.rs
src/metrics.rs
src/readiness.rs
src/normalize.rs
src/utils/time.rs
src/utils/hashing.rs
src/utils/encode.rs
src/accounting/*
src/exporter/*
src/wal/*
src/config/*
tests/unit/*
tests/prop/*
tests/loom/*
benches/*
examples/*
```

Batch 1 created:

```text
error taxonomy
normalization helpers
readiness gates
metrics handle foundation
config schema/load/validate
dimension model
label model
recorder/counter model
window model
sealed slice model
rollover model
exporter trait
exporter router foundation
export lane foundation
ACK LRU foundation
WAL feature stubs/foundation
unit tests
property tests
loom/model placeholders
bench smoke
examples
```

Initial compile/clippy issues were fixed:

```text
too_many_arguments in rollover
derivable_impls in config schema
proptest import mismatch
ownership move in labels property test
constant assert clippy failures in placeholder tests
route normalization idempotence bug
```

Important bug fixed:

```text
normalize_route("0!") initially became "/0"
second normalization then became "/:id"
```

We fixed route normalization so normalization is idempotent.

## 3.2 Batch 2 — Exporter correctness

Batch 2 strengthened exporter routing.

Major files improved:

```text
src/exporter/ack_lru.rs
src/exporter/lane.rs
src/exporter/router.rs
src/exporter/worker.rs
src/exporter/mod.rs
tests/unit/exporter_ordering_tests.rs
```

Batch 2 added or improved:

```text
AckKey
AckLru insert/contains/eviction behavior
ExportLane stream key validation
per-stream sequence continuity
bounded per-stream queueing
lease_next instead of destructive pop
ack(seq)
nack(seq)
in-flight tracking
router deterministic lane iteration
router backlog_len
router lane_count
router inflight_len
export_one
export_next
export_until_blocked
ExportRetryPolicy
ExportReport
```

Important design correction:

```text
Batch 1 pop_next was destructive.
Batch 2 made export leasing ACK/NACK-safe.
```

Why this matters:

```text
A failed downstream export must not lose a sealed slice.
The slice is leased, then ACKed only after successful export.
On failure it is NACKed so it can be retried.
Ack::Duplicate is treated as successful export completion.
```

Tests added/proved:

```text
sequence gaps rejected
deterministic backlog retained
lease/ack/nack retry behavior
empty lanes retain sequence continuity
router can lease other stream while one stream is in-flight
ACK LRU dedupes and evicts oldest
retry policy caps exponential backoff
```

## 3.3 Batch 3 — Reward snapshot DTO and canonical CID

Batch 3 added the rewarder-facing DTO surface.

Major files added/improved:

```text
src/accounting/reward_snapshot.rs
src/accounting/mod.rs
src/lib.rs
tests/unit/reward_snapshot_tests.rs
```

Batch 3 added:

```text
RewardSnapshotExport
RewardContributionExport
canonical_snapshot_bytes
canonical_snapshot_cid
canonical JSON serialization
b3 CID over canonical bytes
pool_minor_units string validation
pool_minor_units u128 parser
account validation
duplicate account rejection
canonical contribution ordering
reward_score helper
total_score helper
contribution_count helper
```

Reward snapshot shape:

```text
RewardSnapshotExport
  produced_at_millis: u64
  pool_minor_units: String
  contributions: Vec<RewardContributionExport>

RewardContributionExport
  account: String
  bytes_stored: u64
  bytes_served: u64
  uptime_seconds: u64
```

This exactly targets the needed `svc-rewarder` accounting input shape:

```text
produced_at_millis
pool_minor_units
contributions:
  account
  bytes_stored
  bytes_served
  uptime_seconds
```

Earlier rewarder carryover notes specifically said `ron-accounting` should output that kind of sealed snapshot, including canonical CID enforcement. 

Batch 3 tests proved:

```text
canonical account order
canonical JSON shape
CID stable after canonicalization
duplicate accounts rejected
non-integer pool rejected
score arithmetic checked
invalid account chars rejected
```

## 3.4 Batch 4 — Reward projection from sealed slices

Batch 4 connected sealed accounting slices to rewarder snapshots.

Major files added/improved:

```text
src/accounting/reward_projection.rs
src/accounting/mod.rs
src/lib.rs
tests/unit/reward_projection_tests.rs
```

Batch 4 added:

```text
RewardAccountMode
RewardProjectionConfig
RewardProjectionReport
ProjectedRewardSnapshot
project_reward_snapshot_from_slices
account_from_labels
classification of stored bytes vs served bytes vs uptime seconds
checked aggregate accumulation
deterministic BTreeMap account ordering
snapshot CID generation
projection report
```

Reward projection behavior:

```text
Dimension::Bytes + GET/HEAD/served/download/egress/read route
  → bytes_served

Dimension::Bytes + everything else
  → bytes_stored

Dimension::Requests + UPTIME/uptime/heartbeat/health
  → uptime_seconds

Other request rows
  → ignored / no projected contribution
```

Supported account grouping:

```text
RewardAccountMode::TenantService
  account = "t:<tenant>/svc:<service>"

RewardAccountMode::TenantServiceRegion
  account = "t:<tenant>/svc:<service>/r:<region>"
```

Tests proved:

```text
PUT bytes map to bytes_stored
GET bytes map to bytes_served
grouping by tenant/service/region
CID stable when input slice order changes
uptime request rows map to uptime_seconds
unmapped request rows do not produce invalid reward snapshots
overflowing aggregates reject
```

This gives `ron-accounting` the ability to turn sealed usage windows into rewarder-compatible snapshots without manually constructing reward snapshot DTOs.

## 3.5 Batch 5 — Usage event ingest schema

Batch 5 added the WEB3 usage-event intake model.

Major files added/improved:

```text
src/accounting/events.rs
src/accounting/mod.rs
src/lib.rs
tests/unit/event_ingest_tests.rs
```

Batch 5 added:

```text
MetricKind
UsageEvent
UsageCounterInput
EventSubjectMode
EventIngestPolicy
EventIngestReport
record_usage_event
record_usage_events
```

Supported event metric kinds:

```text
bytes_stored
bytes_served
pin_seconds
request_ok
uptime_seconds
cpu_units
custom(...)
```

Usage event shape:

```text
UsageEvent
  timestamp_ms
  tenant
  subject
  metric_kind
  value
  source_service?
  region?
  route?
```

This matches the WEB3 direction that core services should produce metering signals and the value plane should consume them. The WEB3 blueprint describes the value plane as ledger/wallet/accounting/rewarder/enforcement and states that the core plane produces metering signals and economic authorization requests. 

Batch 5 behavior:

```text
BytesStored → Dimension::Bytes, method PUT, route /usage/bytes-stored
BytesServed → Dimension::Bytes, method GET, route /usage/bytes-served
UptimeSeconds → Dimension::Requests, method UPTIME, route /usage/uptime
RequestOk → Dimension::Requests, method REQ_OK
CpuUnits → Dimension::Cpu, method CPU
Custom("cache-hit") → Dimension::Requests, method CACHE_HIT, route /usage/custom/cache-hit
```

Event ingest policy:

```text
EventSubjectMode::Subject
  label service = event.subject

EventSubjectMode::SourceService
  label service = event.source_service or default source service
```

Tests proved:

```text
bytes_stored event records byte counter
bytes_served event records GET byte counter
source_service attribution works
zero-value event is no-op
batch ingest reports inspected/recorded/skipped
empty subject rejected
zero timestamp rejected
custom metric kind normalized safely
```

Important fixes during Batch 5:

```text
redundant closure clippy fix:
  .map(|value| normalize_route(value))
  became
  .map(normalize_route)

custom metric method label fix:
  cache-hit initially became UNKNOWN through method normalization
  changed custom method label to CACHE_HIT
```

## 3.6 Final Batch — Accounting ↔ Rewarder interop vector

Final batch added a deterministic interop vector.

Major files added/improved:

```text
src/accounting/interop.rs
src/accounting/mod.rs
src/lib.rs
tests/unit/interop_vector_tests.rs
```

Final batch added:

```text
RewardSnapshotInteropVector
REWARD_SNAPSHOT_VECTOR_SCHEMA
REWARD_SNAPSHOT_VECTOR_EPOCH_ID
reward_snapshot_interop_vector_v1
canonical_json_for_snapshot
```

Stable vector schema:

```text
schema = "ron-accounting.reward-snapshot.interop.v1"
epoch_id = "interop-epoch-1"
```

Stable vector snapshot:

```json
{
  "produced_at_millis": 1,
  "pool_minor_units": "1000",
  "contributions": [
    {
      "account": "acct_a",
      "bytes_stored": 100,
      "bytes_served": 50,
      "uptime_seconds": 10
    },
    {
      "account": "acct_b",
      "bytes_stored": 200,
      "bytes_served": 0,
      "uptime_seconds": 20
    }
  ]
}
```

Expected total score:

```text
acct_a = 100 + (50 / 4) + 10 = 122
acct_b = 200 + (0 / 4) + 20 = 220
total  = 342
```

Final batch tests proved:

```text
interop vector validates
canonical JSON is stable
CID matches canonical bytes
snapshot has rewarder-compatible shape
expected score/count match
synthetic usage events are included
```

This is the key bridge artifact for the next WEB3 integration session.

---

# 4. Current Source Structure and Roles

Current meaningful source layout:

```text
src/lib.rs
  crate facade and public re-exports

src/errors.rs
  typed Error/Result taxonomy

src/metrics.rs
  accounting metric handles and metric foundation

src/readiness.rs
  readiness gate state

src/normalize.rs
  label/method/route normalization and cardinality protection

src/utils/time.rs
  timestamp/window helper utilities

src/utils/hashing.rs
  BLAKE3 b3:<hex> helper

src/utils/encode.rs
  canonical JSON/byte encoding helpers

src/accounting/dimensions.rs
  Dimension model: Bytes, Requests, Cpu

src/accounting/labels.rs
  TenantId, Namespace, AccountKey, LabelSet, Row

src/accounting/recorder.rs
  Recorder, CounterKey, CounterRow, RecorderConfig

src/accounting/window.rs
  UTC-aligned Window model

src/accounting/slice.rs
  SliceId, SliceMeta, SliceRow, SealedSlice

src/accounting/rollover.rs
  RolloverHandle, RolloverDecision, seal-if-due logic

src/accounting/events.rs
  UsageEvent ingest schema and record_usage_event(s)

src/accounting/reward_snapshot.rs
  RewardSnapshotExport and RewardContributionExport

src/accounting/reward_projection.rs
  sealed-slice → reward snapshot projection

src/accounting/interop.rs
  stable accounting↔rewarder interop vector

src/exporter/trait.rs
  Exporter trait and Ack type

src/exporter/trait_.rs
  compatibility facade for trait module naming

src/exporter/ack_lru.rs
  bounded ACK/idempotency cache

src/exporter/lane.rs
  per-stream ordered export lane

src/exporter/router.rs
  deterministic multi-stream export router

src/exporter/worker.rs
  async export_one/export_next/export_until_blocked helpers

src/wal/mod.rs
  feature-gated WAL facade

src/wal/segment.rs
  segment record and checks

src/wal/replay.rs
  replay verification helper

src/wal/fs.rs
  filesystem seam placeholder/foundation

src/config/schema.rs
  Config, AccountingConfig, ExporterConfig, WalConfig

src/config/load.rs
  config loading helpers

src/config/validate.rs
  fail-closed config validation

examples/minimal.rs
  minimal record/seal example

examples/export_to_mock.rs
  mock export example

benches/record.rs
  record smoke benchmark

benches/seal.rs
  seal smoke benchmark

tests/unit/*
  focused unit suites

tests/prop/*
  proptest suites

tests/loom/*
  current lightweight model tests/placeholders
```

Original scaffold expected these major areas: core library files, accounting modules, exporter modules, WAL feature modules, config modules, unit/prop/loom tests, benches, examples, vectors, and docs. 

We implemented the important core/library/test functionality and added additional modules not in the original scaffold because they became necessary for WEB3 interop:

```text
src/accounting/events.rs
src/accounting/reward_snapshot.rs
src/accounting/reward_projection.rs
src/accounting/interop.rs
tests/unit/event_ingest_tests.rs
tests/unit/reward_snapshot_tests.rs
tests/unit/reward_projection_tests.rs
tests/unit/interop_vector_tests.rs
```

These additions are intentional and aligned with the rewarder/accounting contract.

---

# 5. Current Public API Surface

Important public exports now include:

```rust
// core accounting
Recorder
RecorderConfig
CounterRow
Dimension
LabelSet
TenantId
Window
SealedSlice
SliceId
SliceMeta
SliceRow

// usage events
UsageEvent
MetricKind
UsageCounterInput
EventIngestPolicy
EventIngestReport
EventSubjectMode
record_usage_event
record_usage_events

// reward snapshot export
RewardSnapshotExport
RewardContributionExport
canonical_snapshot_bytes
canonical_snapshot_cid

// reward projection
RewardAccountMode
RewardProjectionConfig
RewardProjectionReport
ProjectedRewardSnapshot
project_reward_snapshot_from_slices
account_from_labels

// interop
RewardSnapshotInteropVector
reward_snapshot_interop_vector_v1
canonical_json_for_snapshot
REWARD_SNAPSHOT_VECTOR_SCHEMA
REWARD_SNAPSHOT_VECTOR_EPOCH_ID

// exporter
Exporter
Ack
BoxExportFuture
AckLru
ExporterRouter

// readiness
Readiness
ReadyKey

// WAL when feature enabled
Wal
WalConfig
WalStats
```

This is now enough for a next-session integration test to do:

```rust
let vector = reward_snapshot_interop_vector_v1()?;
let cid = vector.snapshot.canonical_cid()?;
assert_eq!(cid, vector.snapshot_cid);
```

And eventually for `svc-rewarder` to consume exactly the canonical snapshot shape emitted by `ron-accounting`.

---

# 6. Current Test Coverage

Final known green command set:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal
```

Final observed normal-mode tests:

```text
2 loom/model tests
3 property tests
37 unit tests
record bench smoke
seal bench smoke
examples/minimal.rs builds
examples/export_to_mock.rs builds
```

Final observed WAL-feature tests:

```text
2 loom/model tests
3 property tests
38 unit tests
record bench smoke
seal bench smoke
examples/minimal.rs builds
examples/export_to_mock.rs builds
```

The latest terminal output proves clippy and both normal/WAL test modes passed, including the final interop vector tests and WAL tests. 

## 6.1 Unit tests currently cover

```text
recording monotonicity
counter saturation behavior
sealing drains requested stream
sealed slice digest commitment
window UTC alignment
rollover once per boundary
exporter sequence gap rejection
exporter deterministic backlog
exporter lease/ACK/NACK retry safety
router sequence continuity
router multi-stream leasing
ACK LRU dedupe and eviction
retry policy backoff capping
WAL disabled placeholder
WAL append/drain when feature enabled
WAL replay skipping corrupt records
reward snapshot canonical order
reward snapshot canonical JSON
reward snapshot CID stability
reward snapshot duplicate rejection
reward snapshot pool validation
reward snapshot score validation
reward contribution account validation
reward projection grouping
reward projection CID stability
reward projection stored/served/uptime mapping
reward projection overflow rejection
event ingest bytes stored
event ingest bytes served
event ingest source attribution
event ingest zero no-op
event ingest batch summary
event ingest invalid subject/timestamp rejection
custom metric normalization
interop vector validation
interop vector canonical JSON stability
interop vector CID verification
interop vector rewarder shape
```

## 6.2 Property tests currently cover

```text
same slice input has same digest
component normalization is idempotent
route normalization is idempotent
```

## 6.3 Loom/model tests currently cover

```text
router model placeholder
shutdown model placeholder
```

Important caveat:

```text
The loom/model tests are currently lightweight model tests, not full loom interleaving proofs.
```

They prevent placeholder clippy failures and capture intended state transitions, but full concurrency model testing remains future work.

---

# 7. Current Quality / Completion Status

Current completion estimate:

```text
ron-accounting: 88–91% complete
```

Status label:

```text
Gold foundation
```

Why it is this high:

```text
core library compiles cleanly
strict clippy passes
normal tests pass
WAL-feature tests pass
usage event ingest exists
counter recording exists
window sealing exists
sealed slice digesting exists
ordered exporter exists
ACK/NACK retry-safe export exists
reward snapshot DTO exists
canonical b3 CID exists
reward projection exists
interop vector exists
examples and benches are green
```

Why it is not 100%:

```text
no production HTTP/UDS ingest adapter yet
no live svc-storage/svc-gateway/svc-edge event feed yet
WAL is not yet a full production segmented filesystem WAL
exporter is not wired to a live downstream service yet
loom tests are not full concurrency interleaving proofs
no end-to-end integration proof across accounting → rewarder → wallet → ledger yet
no dashboard/admin surfacing of accounting metrics yet
docs/openapi/schema/vector files are not finalized as checked external artifacts
```

The crate is good enough to move out of isolated coding and into WEB3 integration.

---

# 8. Remaining Work for ron-accounting

## 8.1 HTTP/UDS ingest adapter

Needed:

```text
POST /v1/usage
POST /v1/usage/batch
GET /healthz
GET /readyz
GET /metrics
GET /version
```

Possible DTO:

```json
{
  "timestamp_ms": 1,
  "tenant": 1,
  "subject": "provider-a",
  "metric_kind": "bytes_stored",
  "value": 1024,
  "source_service": "svc-storage",
  "region": "local",
  "route": "/objects"
}
```

Adapter must enforce:

```text
body cap ≤ 1 MiB
strict serde deny_unknown_fields
auth/capability checks
timeouts
structured errors
rate/concurrency limits
amnesia-aware persistence
```

## 8.2 Real service event feed

Need connect producers:

```text
svc-storage → bytes_stored, bytes_served, pin_seconds
svc-gateway → request_ok, bytes_served/ingress/egress counters
svc-edge    → bytes_served/cache-hit/custom
svc-overlay → uptime_seconds/session counters
micronode/macronode → profile-specific usage events
```

This is where the crate becomes useful inside the live node.

## 8.3 Production WAL

Current WAL feature tests prove the foundation, but production WAL still needs:

```text
real filesystem segment writer
segment rotation
max_bytes enforcement across segments
max_entries enforcement across segments
max_age_s retention
fsync_on_close
fsync_dir_on_create
crash recovery
partial-write detection
corruption quarantine
replay idempotence
metrics for replay/reject/quarantine
amnesia-mode disablement
```

## 8.4 Export adapter

Current exporter library is good. Remaining adapter work:

```text
export sealed slices to a downstream endpoint
export reward snapshots to svc-rewarder or artifact store
retry with jitter
timeout classification
duplicate ACK handling
NACK and retry loop
readiness degradation on exporter failure
metrics:
  accounting_export_total{result}
  accounting_export_latency_seconds
  accounting_export_backlog
  accounting_export_retries_total
```

## 8.5 Rewarder interop test

Next session should add a cross-crate test using the new vector.

Goal:

```text
ron-accounting reward_snapshot_interop_vector_v1()
→ canonical JSON
→ b3 CID
→ svc-rewarder canonical_snapshot_cid()
→ same CID
→ rewarder compute accepts snapshot
→ payout totals match expected score/policy
```

This directly addresses the previous rewarder note that `inputs_cid` should eventually equal `canonical_snapshot_cid(snapshot)`.

## 8.6 Real end-to-end ROC loop

Ultimate WEB3 integration proof:

```text
usage recorded
→ accounting window sealed
→ accounting reward snapshot generated
→ rewarder computes reward epoch
→ rewarder creates wallet issue requests
→ wallet issues ROC
→ ledger durable commit
→ replay does not double issue
→ dashboard/metrics show movement
```

The prior rewarder notes explicitly identified this as the eventual value-plane integration proof. 

## 8.7 Stronger observability

Add/use accounting-specific metrics:

```text
accounting_events_ingested_total{source,metric_kind,result}
accounting_event_reject_total{reason}
accounting_rows_active
accounting_windows_sealed_total
accounting_window_rollover_total{result}
accounting_snapshot_exports_total{result}
accounting_snapshot_bytes
accounting_counter_overflow_total
accounting_export_backlog
accounting_export_latency_seconds
accounting_wal_records_total{result}
accounting_wal_replay_total{result}
accounting_wal_bytes
accounting_ready{gate}
```

Docs had already anticipated metrics useful to rewarder such as sealed windows, snapshot exports, snapshot bytes, and counter overflow. 

## 8.8 Real concurrency tests

Current model tests are not enough for production confidence.

Needed:

```text
loom model for router ACK/NACK under interleavings
loom model for shutdown/drain behavior
loom model for exporter worker cancellation
test no lock across await in future async adapter
test no slice loss during failed export
test bounded queue behavior under pressure
```

## 8.9 Canonical test vector files

We added the code-level vector. Still useful to write checked test vector files:

```text
tests/vectors/ron-accounting/reward_snapshot_v1.json
tests/vectors/ron-accounting/reward_snapshot_v1.cid
tests/vectors/ron-accounting/usage_events_v1.json
tests/vectors/ron-accounting/projected_snapshot_v1.json
```

Future `svc-rewarder` can import/read these.

## 8.10 Documentation update

Update crate docs:

```text
README.md
NOTES.MD
docs/API.MD
docs/INTEROP.MD
docs/OBSERVABILITY.MD
docs/CONFIG.MD
docs/TESTS.MD
```

Needed content:

```text
usage event schema
reward snapshot DTO
projection rules
canonical CID rule
interop vector
WAL feature status
adapter TODOs
metrics list
golden commands
completion status
```

---

# 9. Important Invariants to Preserve

Do not regress these:

```text
ron-accounting is not a ledger
ron-accounting does not issue ROC
ron-accounting does not own balances
ron-accounting does not bypass svc-wallet
ron-accounting does not commit directly to ron-ledger
no floating point in value-plane math
no raw PII in labels
labels/routes must be normalized and bounded
sealed slices must be deterministic
canonical snapshot bytes must be stable
snapshot_cid must be b3:<hash(canonical_snapshot_bytes)>
export must be ACK/NACK-safe
failed export must not lose a sealed slice
WAL persistence must be explicit
amnesia mode must not silently write to disk
```

---

# 10. Known Commands

Use this as the standard crate gate:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal
```

Optional targeted commands:

```bash
cargo test -p ron-accounting --test unit
cargo test -p ron-accounting --test prop
cargo test -p ron-accounting --test loom
cargo test -p ron-accounting --all-targets --features wal
```

Examples/benches are already included by the all-targets commands, but can also be run directly if needed:

```bash
cargo run -p ron-accounting --example minimal
cargo run -p ron-accounting --example export_to_mock
cargo bench -p ron-accounting
```

---

# 11. Recommended Next Session Plan

Do not keep adding isolated internal features unless a compile/test issue appears.

The next session should be a **WEB3 integration session**.

Recommended order:

## Step 1 — Capture fresh bundles

Attach/review:

```text
ron-accounting CODEBUNDLE
ron-accounting ALL_DOCS / NOTES
svc-rewarder CODEBUNDLE / NOTES
svc-wallet CODEBUNDLE / NOTES
ron-ledger CODEBUNDLE / NOTES
WEB3.MD
```

## Step 2 — Add shared accounting↔rewarder vector test

Goal:

```text
svc-rewarder imports/duplicates the ron-accounting interop vector
svc-rewarder validates canonical JSON and b3 CID
svc-rewarder accepts the snapshot shape
svc-rewarder produces deterministic reward totals
```

## Step 3 — Add minimal adapter seam

Choose one of:

```text
A) library-only adapter function first
B) HTTP/UDS event ingest adapter
C) file/vector-based integration smoke
```

Recommended first:

```text
library-only integration smoke
```

Because it is the least risky.

## Step 4 — End-to-end dev proof

Build:

```text
usage events
→ ron-accounting recorder
→ seal snapshot
→ project reward snapshot
→ svc-rewarder compute
→ wallet issue preview
```

Then later add:

```text
wallet issue actual dev path
ledger commit
ledger replay
```

## Step 5 — Paid storage/pinning proof

WEB3 v1 requires at least one real in-network use case, recommended paid storage/pinning. The WEB3 blueprint defines this as part of the done condition, alongside ledger durability, wallet mutation, rewarder issuance, and metrics/dashboard visibility. 

---

# 12. Current WEB3 Status Impact

After finishing this crate, the WEB3 value-plane status is approximately:

```text
ron-ledger      ~80–85%
svc-wallet      ~80–85%
svc-rewarder    ~78–82%
ron-accounting  ~88–91%
```

Overall WEB3 system status:

```text
~72–76%
```

Reason:

The individual crates are now strong, but WEB3 completion depends on the live loop:

```text
svc-storage / svc-gateway usage event
→ ron-accounting sealed snapshot
→ svc-rewarder deterministic reward batch
→ svc-wallet issue request
→ ron-ledger durable commit
→ dashboard/metrics prove ROC movement
```

The remaining work is mostly **interop and live flow proof**, not isolated crate foundations.

---

# 13. Short Carryover Summary

`ron-accounting` is now a Gold foundation crate. It records normalized usage events, maintains transient counters, seals deterministic slices, supports ordered ACK/NACK-safe export, has a WAL feature path, produces rewarder-compatible snapshots, projects sealed slices into reward snapshots, and exposes a stable accounting↔rewarder interop vector with canonical JSON and b3 CID proof. The full quality gate is green in normal and WAL modes. Remaining work is production adapters, stronger WAL, real service event feeds, full loom tests, and end-to-end WEB3 integration.

### END NOTE - APRIL 27 2026 - 15:40 CST

```
```
### BEGIN NOTE - APRIL 27 2026 - 19:55 CST


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

### END NOTE - APRIL 27 2026 - 19:55 CST


### BEGIN NOTE - APRIL 28 2026 - 20:10 CST


# ron-accounting NOTES.MD — WEB3 Usage Ingest, Reward Snapshot Foundation, and Live Storage Export Plateau

Date: 2026-04-28 / 2026-04-29  
Crate: `ron-accounting`  
Workspace path: `crates/ron-accounting`  
Project: RustyOnions / RON-CORE / WEB3 / ROC internal value plane  
Status: **Gold foundation + live HTTP ingest proof green**  

Current estimated completion:

```text
ron-accounting core library foundation:       ~90–94%
usage event ingest / normalization:           ~95–98%
sealed slices / reward projection:            ~88–93%
accounting ↔ rewarder interop vector:         ~90–94%
lightweight HTTP ingest adapter:              ~88–92%
live storage → accounting proof:              ~96–99%
production service/admin hardening:           ~70–80%
overall ron-accounting WEB3 readiness:        ~92–95%
````

Current project-level significance:

```text
ron-accounting is now no longer only a library proof.
It has a live local ingest process that receives real usage events from svc-storage,
records normalized counters, and exposes a snapshot proving the paid-storage
usage path reached accounting.
```

This note documents what `ron-accounting` does now, what we added, what has been proven, how to run it, and what remains.

---

# 0. Executive Summary

`ron-accounting` is the RustyOnions transient usage/metering layer for the internal ROC economy.

Its core pipeline is:

```text
usage events
→ normalized labels
→ transient counters
→ deterministic slices/windows
→ reward projection
→ canonical reward snapshot
→ svc-rewarder input
```

As of this plateau, it now also has a lightweight live HTTP ingest adapter:

```text
svc-storage /paid/o
→ usage_events
→ HTTP POST /v1/usage-events
→ ron-accounting ingest process
→ record_usage_events(...)
→ Recorder snapshot
→ GET /v1/snapshot
```

The latest three-process live smoke proved:

```text
ron-accounting ingest process starts
svc-wallet starts
svc-storage starts
svc-wallet issues ROC
svc-wallet creates a hold
svc-storage verifies wallet receipt over HTTP
svc-storage stores the b3 object
svc-storage captures/release ROC through wallet
svc-storage exports usage events to ron-accounting
ron-accounting accepts export with HTTP 202
ron-accounting snapshot records 3 rows
wallet balances remain conserved
object reads back
receipt replay for different bytes rejects
```

The latest live accounting snapshot proved:

```text
row_count = 3

bytes dimension:
  PUT /paid/o = 48

requests dimension:
  REQ_OK /paid/o = 1
  PIN_SECONDS /paid/o = 60
```

This closes a major WEB3 link:

```text
paid storage → real accounting ingest process
```

`ron-accounting` still remains within its correct boundary:

```text
It records usage.
It normalizes counters.
It prepares sealed/reward snapshots.
It does not hold user balances.
It does not issue ROC.
It does not mutate ron-ledger directly.
It does not replace svc-wallet.
It does not perform external settlement.
```

---

# 1. Strategic Role in WEB3 / ROC

`ron-accounting` belongs to the internal ROC value plane.

Its role:

```text
collect metering/usage signals
normalize labels and dimensions
aggregate transient counters
seal deterministic time/window slices
prepare rewarder-compatible snapshots
support reward projection
feed svc-rewarder/policy decisions
provide visibility into usage flow
```

Its non-role:

```text
not a wallet
not a ledger
not durable economic truth
not a settlement engine
not a bridge
not ROX/Solana/external-chain logic
not storage
not name resolution
not asset ownership
```

Correct service boundaries:

```text
svc-storage / svc-gateway / omnigate:
  produce usage events
  enforce paid access/storage
  call wallet for economic mutation when appropriate

ron-accounting:
  accepts usage events
  normalizes counters
  snapshots usage
  projects reward contribution inputs
  exposes accounting snapshots

svc-rewarder:
  consumes accounting/policy signals
  computes deterministic reward/payout plans
  does not directly mutate ledger

svc-wallet:
  sole normal mutation front-door for balances
  issues/captures/releases/transfers/burns
  commits receipts through ron-ledger

ron-ledger:
  append-only durable economic truth
  replayable, deterministic, crash-safe
```

Important WEB3 invariant:

```text
ron-accounting is transient counters/snapshots, not balance truth.
ron-ledger remains economic truth.
svc-wallet remains the mutation front-door.
```

---

# 2. Current Core Library Capabilities

Before the HTTP ingest adapter, `ron-accounting` was already a strong “Gold foundation” crate.

## 2.1 Usage event ingest

Current core ingest types/functions include:

```text
UsageEvent
MetricKind
UsageCounterInput
EventSubjectMode
EventIngestPolicy
EventIngestReport
record_usage_event(...)
record_usage_events(...)
```

Supported metric kinds include:

```text
bytes_stored
bytes_served
pin_seconds
request_ok
uptime_seconds
cpu_units
custom(...)
```

Current event shape:

```text
UsageEvent:
  timestamp_ms
  tenant
  subject
  metric_kind
  value
  source_service?
  region?
  route?
```

Core validation/tests prove:

```text
zero timestamp is rejected
empty subject is rejected
zero-value events are skipped/no-op
bytes_stored records into bytes dimension
bytes_served records into GET bytes
request_ok records into requests dimension
pin_seconds records as request/time-style counter
custom metric kinds normalize into request dimensions
source_service attribution works
batch ingest reports inspected/recorded/skipped counts
overflow in ingest counters rejects safely
```

## 2.2 Label normalization

`ron-accounting` normalizes labels to keep the counter space bounded and deterministic.

Important label fields:

```text
tenant
service
region
method
route
```

Important behavior:

```text
subject/source names are normalized
routes normalize deterministically
component normalization is idempotent
labelish fields are bounded
empty labelish fields reject
```

Property tests prove:

```text
component normalization is idempotent
route normalization is idempotent
same slice input has same digest
```

## 2.3 Recorder

The `Recorder` aggregates normalized rows.

It supports:

```text
record counter row
snapshot rows
row_count
monotone/saturating accumulation
```

The latest live ingest proof used a `Recorder` inside the HTTP ingest process and showed:

```text
row_count = 3
```

after receiving usage export from `svc-storage`.

## 2.4 Sealed slices and windows

Existing accounting foundation supports deterministic slice/window behavior.

Proven behavior:

```text
recording is monotone and saturating
sealing drains only requested stream
sealing commits digest
rollover happens once at boundary crossing
windows align to UTC boundary
canonical bytes are stable
canonical snapshot CID is stable
```

This matters because reward epochs must be deterministic and replayable.

## 2.5 Reward projection

`ron-accounting` can turn recorded/sealed usage into reward-oriented contribution shape.

Projection behavior:

```text
Dimension::Bytes + PUT-like rows
  → bytes_stored

Dimension::Bytes + GET/HEAD/read/download/egress-like rows
  → bytes_served

Dimension::Requests + UPTIME/heartbeat/health rows
  → uptime_seconds

Other request rows
  → ignored for reward contribution
```

Supported account grouping:

```text
RewardAccountMode::TenantService:
  account = t:<tenant>/svc:<service>

RewardAccountMode::TenantServiceRegion:
  account = t:<tenant>/svc:<service>/r:<region>
```

Tests prove:

```text
grouping by tenant/service/region works
PUT bytes map to bytes_stored
GET bytes map to bytes_served
uptime request rows map to uptime_seconds
unmapped request rows are ignored
projection CID is stable when input slice order changes
overflowing aggregates reject safely
```

## 2.6 Reward snapshot interop vector

The crate has a stable accounting ↔ rewarder interop vector.

Important exports/constants include:

```text
RewardSnapshotInteropVector
RewardSnapshotExport
RewardContributionExport
REWARD_SNAPSHOT_VECTOR_SCHEMA
REWARD_SNAPSHOT_VECTOR_EPOCH_ID
reward_snapshot_interop_vector_v1()
canonical_snapshot_bytes(...)
canonical_snapshot_cid(...)
```

This gives `svc-rewarder` a known-good accounting snapshot contract with:

```text
canonical JSON
canonical snapshot CID
stable epoch ID
stable contribution rows
known total score
rewarder-compatible shape
```

This is important because `svc-rewarder` should not invent contribution data. It should consume accounting snapshots.

---

# 3. New Live HTTP Ingest Adapter

This plateau added a lightweight HTTP ingest adapter for live WEB3 proof.

## 3.1 New files

Added:

```text
crates/ron-accounting/src/http_ingest.rs
crates/ron-accounting/src/bin/ron_accounting_ingest.rs
```

Updated:

```text
crates/ron-accounting/src/lib.rs
scripts/web3_paid_storage_live_smoke.sh
```

## 3.2 Why this adapter exists

Purpose:

```text
prove svc-storage can export real usage events to ron-accounting
avoid guessing or changing Cargo.toml dependencies
avoid introducing axum until final service shape is decided
create a real local process for WEB3 live smoke
provide immediate /healthz /readyz /v1/usage-events /v1/snapshot
```

Design choice:

```text
std-library blocking HTTP server
no new dependency required
good for beta proof and local integration
not necessarily the final production service architecture
```

## 3.3 Binary

Binary path:

```text
crates/ron-accounting/src/bin/ron_accounting_ingest.rs
```

Run manually:

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600 \
RON_ACC_BEARER=dev \
cargo run -p ron-accounting --bin ron_accounting_ingest
```

Compiled binary path after build:

```text
target/debug/ron_accounting_ingest
```

## 3.4 Environment variables

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600
RON_ACC_BEARER=dev
```

Defaults:

```text
RON_ACCOUNTING_ADDR default = 127.0.0.1:19600
RON_ACC_BEARER default = dev
```

Security note:

```text
The default dev bearer is for local smoke only.
Production should use real capability/auth integration.
```

---

# 4. HTTP Ingest API

The lightweight ingest adapter exposes:

```text
GET  /healthz
GET  /readyz
GET  /v1/snapshot
POST /v1/usage-events
```

## 4.1 `GET /healthz`

Purpose:

```text
process liveness
```

Latest live smoke response:

```json
{
  "ok": true,
  "service": "ron-accounting"
}
```

## 4.2 `GET /readyz`

Purpose:

```text
basic readiness + row count
```

Response shape:

```json
{
  "service": "ron-accounting",
  "ready": true,
  "rows": 0
}
```

Current limitations:

```text
This lightweight adapter does not yet check bus, ledger, policy, metrics, WAL, or downstream readiness.
It is a beta ingest proof, not the final production readiness contract.
```

## 4.3 `POST /v1/usage-events`

Purpose:

```text
ingest svc-storage usage event exports
```

Required headers:

```text
Authorization: Bearer <RON_ACC_BEARER>
Idempotency-Key: storage_acct:<hash>
Content-Type: application/json
```

Expected request schema:

```json
{
  "schema": "svc-storage.usage-events.v1",
  "cid": "b3:<64 lowercase hex>",
  "wallet_txid": "tx_...",
  "source_service": "svc-storage",
  "events": [
    {
      "timestamp_ms": 1777424552047,
      "tenant": 7,
      "subject": "svc_storage_provider",
      "metric_kind": "bytes_stored",
      "value": 48,
      "source_service": "svc-storage",
      "region": "local",
      "route": "/paid/o"
    }
  ]
}
```

Validation performed:

```text
Authorization bearer must match, if configured
Idempotency-Key must exist
Idempotency-Key must be non-empty and bounded
schema must equal svc-storage.usage-events.v1
cid must be canonical b3:<64 lowercase hex>
wallet_txid must not be empty
body must fit the small adapter cap
JSON must match expected request type
usage events must pass core record_usage_events validation
```

Success response shape:

```json
{
  "ok": true,
  "duplicate": false,
  "idempotency_key": "storage_acct:<hash>",
  "inspected": 3,
  "recorded": 3,
  "skipped_zero": 0,
  "row_count": 3
}
```

Duplicate behavior:

```text
Duplicate Idempotency-Key returns ok=true, duplicate=true.
Duplicate request does not record the same events again.
Dedupe is currently in-memory only.
```

Important limitation:

```text
Dedupe does not survive process restart yet.
Production needs durable idempotency/outbox/WAL behavior.
```

## 4.4 `GET /v1/snapshot`

Purpose:

```text
inspect current in-memory accounting rows
```

Latest live smoke snapshot:

```json
{
  "schema": "ron-accounting.snapshot.v1",
  "row_count": 3,
  "rows": [
    {
      "key": {
        "dimension": "requests",
        "labels": {
          "method": "PIN_SECONDS",
          "region": "local",
          "route": "/paid/o",
          "service": "svc_storage_provider",
          "tenant": 7
        }
      },
      "value": 60
    },
    {
      "key": {
        "dimension": "bytes",
        "labels": {
          "method": "PUT",
          "region": "local",
          "route": "/paid/o",
          "service": "svc_storage_provider",
          "tenant": 7
        }
      },
      "value": 48
    },
    {
      "key": {
        "dimension": "requests",
        "labels": {
          "method": "REQ_OK",
          "region": "local",
          "route": "/paid/o",
          "service": "svc_storage_provider",
          "tenant": 7
        }
      },
      "value": 1
    }
  ]
}
```

This proves:

```text
svc-storage emitted usage events
svc-storage exported them over HTTP
ron-accounting accepted them
record_usage_events mapped them into normalized rows
snapshot inspection confirmed expected accounting counters
```

---

# 5. Live Storage → Accounting Export Proof

The new live smoke now runs three services:

```text
ron-accounting ingest
svc-wallet
svc-storage
```

Script:

```text
scripts/web3_paid_storage_live_smoke.sh
```

Current default ports:

```text
ron-accounting:
  127.0.0.1:19600

svc-wallet:
  127.0.0.1:18088

svc-storage:
  127.0.0.1:15303
```

The live smoke proves:

```text
ron-accounting /healthz returns ok
svc-wallet /healthz returns ok
svc-storage /healthz returns ok

svc-wallet issues 100 ROC to acct_user
svc-wallet holds 70 ROC into escrow_paid_write
svc-storage verifies wallet receipt over HTTP
svc-storage stores the object by b3 CID
svc-storage captures 48 ROC to svc_storage
svc-storage releases 22 ROC to acct_user
svc-storage exports 3 usage events to ron-accounting
ron-accounting returns HTTP 202
ron-accounting snapshot row_count = 3
object reads back correctly
wallet balances conserve
replay with different bytes rejects
```

Latest paid write accounting export response:

```json
{
  "mode": "http",
  "status": "exported",
  "event_count": 3,
  "idempotency_key": "storage_acct:5b5e91f3af58126c7cd9a24a66e31d62",
  "http_status": 202,
  "reason": null
}
```

This is the strongest current proof that the accounting crate is no longer isolated:

```text
real service usage → real accounting process → normalized counters
```

---

# 6. Current Tests and What They Prove

## 6.1 Latest validated command group

Latest green validation:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets

cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets

chmod +x scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_storage_live_smoke.sh
```

Latest observed results:

```text
ron-accounting clippy passed
ron-accounting tests passed
ron_accounting_ingest binary compiled
svc-storage clippy passed
svc-storage tests passed
live three-process smoke passed
```

## 6.2 ron-accounting test inventory

Current test groups:

```text
tests/loom.rs
tests/prop.rs
tests/unit.rs
benches/record.rs
benches/seal.rs
examples/minimal.rs
examples/export_to_mock.rs
```

Latest terminal run showed:

```text
loom/model tests:
  2 passed

property tests:
  3 passed

unit tests:
  37 passed

bench smoke:
  accounting_record_one success
  accounting_seal_one_slice success

examples:
  minimal.rs built as test target
  export_to_mock.rs built as test target

binary:
  ron_accounting_ingest compiled
```

## 6.3 `tests/unit/event_ingest_tests.rs`

Proves:

```text
usage_event_rejects_zero_timestamp
usage_event_rejects_empty_subject
zero_value_event_is_noop
usage_event_records_bytes_stored_counter
usage_event_records_bytes_served_as_get_bytes
usage_event_records request_ok/counter data
usage_event_can_attribute_to_source_service
custom_metric_kind_is_normalized_to_requests_dimension
batch_ingest_reports_recorded_and_zero_skipped
```

## 6.4 `tests/unit/reward_projection_tests.rs`

Proves:

```text
projection_can_group_by_tenant_service_region
projection_ignores_unmapped_request_rows
projection_maps_put_bytes_to_stored_and_get_bytes_to_served
projection_maps_uptime_request_rows_to_uptime_seconds
projection_cid_is_stable_when_input_slice_order_changes
projection_rejects_overflowing_aggregates
```

## 6.5 `tests/unit/reward_snapshot_tests.rs`

Proves:

```text
reward_snapshot_canonical_bytes_match_expected_shape
reward_snapshot_canonicalizes_account_order
reward_snapshot_cid_is_stable_after_canonicalization
reward_snapshot_rejects_duplicate_accounts_after_trim
reward_snapshot_rejects_non_integer_pool
reward_snapshot_total_score_is_checked
reward_contribution_rejects_invalid_account_chars
```

## 6.6 `tests/unit/interop_vector_tests.rs`

Proves:

```text
reward_snapshot_interop_vector_has_stable_canonical_json
reward_snapshot_interop_vector_cid_matches_snapshot_bytes
reward_snapshot_interop_vector_snapshot_is_rewarder_shape
reward_snapshot_interop_vector_validates
```

## 6.7 `tests/unit/exporter_ordering_tests.rs`

Proves:

```text
ack_lru_dedupes_and_evicts_oldest
lane_lease_ack_and_nack_are_retry_safe
lane_rejects_sequence_gap
router_can_lease_other_stream_when_one_stream_is_inflight
router_retains_empty_lane_sequence_continuity
router_keeps_deterministic_backlog
retry_policy_caps_exponential_backoff
```

## 6.8 `tests/unit/recording_tests.rs`

Proves:

```text
recording_is_monotone_and_saturating
sealing_drains_only_requested_stream_and_commits_digest
```

## 6.9 `tests/unit/rollover_tests.rs`

Proves:

```text
rollover_happens_once_when_boundary_crosses
window_aligns_to_utc_boundary
```

## 6.10 WAL tests

Current note:

```text
The latest run shown in this session used normal all-targets tests.
Earlier crate notes recorded WAL-feature tests as green, including WAL append/drain and corrupt-record skip behavior.
```

Recommended periodic WAL gate:

```bash
cargo test -p ron-accounting --all-targets --features wal
```

---

# 7. Commands

## 7.1 Full ron-accounting local quality gate

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
```

## 7.2 WAL feature gate

```bash
cargo test -p ron-accounting --all-targets --features wal
```

## 7.3 Run the lightweight ingest server manually

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600 \
RON_ACC_BEARER=dev \
cargo run -p ron-accounting --bin ron_accounting_ingest
```

## 7.4 Probe health

```bash
curl -fsS http://127.0.0.1:19600/healthz | jq .
curl -fsS http://127.0.0.1:19600/readyz | jq .
```

## 7.5 Inspect snapshot

```bash
curl -fsS http://127.0.0.1:19600/v1/snapshot | jq .
```

## 7.6 Run full WEB3 live smoke

From repo root:

```bash
chmod +x scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_storage_live_smoke.sh
```

Expected final line:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```

---

# 8. Manual Ingest Example

Start server:

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600 \
RON_ACC_BEARER=dev \
cargo run -p ron-accounting --bin ron_accounting_ingest
```

Then send a minimal example:

```bash
curl -fsS -X POST http://127.0.0.1:19600/v1/usage-events \
  -H 'Authorization: Bearer dev' \
  -H 'Idempotency-Key: storage_acct:manual_test_0001' \
  -H 'Content-Type: application/json' \
  -d '{
    "schema":"svc-storage.usage-events.v1",
    "cid":"b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57",
    "wallet_txid":"tx_manual",
    "source_service":"svc-storage",
    "events":[
      {
        "timestamp_ms":1777424552047,
        "tenant":7,
        "subject":"svc_storage_provider",
        "metric_kind":"bytes_stored",
        "value":48,
        "source_service":"svc-storage",
        "region":"local",
        "route":"/paid/o"
      },
      {
        "timestamp_ms":1777424552047,
        "tenant":7,
        "subject":"svc_storage_provider",
        "metric_kind":"request_ok",
        "value":1,
        "source_service":"svc-storage",
        "region":"local",
        "route":"/paid/o"
      }
    ]
  }' | jq .
```

Inspect:

```bash
curl -fsS http://127.0.0.1:19600/v1/snapshot | jq .
```

Expected:

```text
row_count increments
bytes row appears
request row appears
```

---

# 9. Files Added / Updated

## 9.1 Added in this plateau

```text
crates/ron-accounting/src/http_ingest.rs
crates/ron-accounting/src/bin/ron_accounting_ingest.rs
```

## 9.2 Updated in this plateau

```text
crates/ron-accounting/src/lib.rs
scripts/web3_paid_storage_live_smoke.sh
```

## 9.3 Related svc-storage files involved in the cross-crate proof

```text
crates/svc-storage/src/accounting/mod.rs
crates/svc-storage/src/accounting/exporter.rs
crates/svc-storage/src/http/routes/paid_object.rs
crates/svc-storage/tests/paid_write_accounting_export.rs
scripts/web3_paid_storage_live_smoke.sh
```

---

# 10. Current API / Data Shapes

## 10.1 Ingest request

```text
UsageEventsIngestRequest
  schema: String
  cid: String
  wallet_txid: String
  source_service: String
  events: Vec<UsageEvent>
```

Expected schema:

```text
svc-storage.usage-events.v1
```

## 10.2 Ingest response

```text
UsageEventsIngestResponse
  ok: bool
  duplicate: bool
  idempotency_key: String
  inspected: usize
  recorded: usize
  skipped_zero: usize
  row_count: usize
```

## 10.3 Snapshot response

Current lightweight adapter response:

```text
schema: ron-accounting.snapshot.v1
row_count: usize
rows: Vec<Row>
```

## 10.4 UsageEvent mapping from svc-storage

`svc-storage` currently sends:

```text
bytes_stored:
  Dimension::Bytes
  method = PUT
  route = /paid/o
  value = object bytes

request_ok:
  Dimension::Requests
  method = REQ_OK
  route = /paid/o
  value = 1

pin_seconds:
  Dimension::Requests
  method = PIN_SECONDS
  route = /paid/o
  value = requested pin duration
```

Latest smoke proved:

```text
bytes_stored = 48
request_ok = 1
pin_seconds = 60
```

---

# 11. Invariants Now Proven

## 11.1 Accounting records usage only

Proven:

```text
ron-accounting accepted usage events
ron-accounting recorded normalized counters
ron-accounting did not mutate wallet
ron-accounting did not mutate ledger
```

## 11.2 Storage → accounting bridge works

Proven:

```text
svc-storage exported usage_events over HTTP
ron-accounting accepted export with HTTP 202
ron-accounting snapshot row_count became 3
expected rows appeared
```

## 11.3 Idempotency exists at ingest boundary

Proven by implementation:

```text
POST /v1/usage-events requires Idempotency-Key
duplicate keys are deduped in memory
duplicate key does not record again
```

Remaining hardening:

```text
durable idempotency across restarts
bounded persisted dedupe cache
WAL-backed ingest receipts
```

## 11.4 Label normalization and dimensions are deterministic

Proven by tests and snapshot:

```text
svc_storage_provider normalized into service label
bytes_stored mapped to bytes/PUT
request_ok mapped to requests/REQ_OK
pin_seconds mapped to requests/PIN_SECONDS
route preserved as /paid/o
region preserved as local
tenant preserved as 7
```

## 11.5 Reward projection foundation exists

Proven by tests:

```text
recorded/sealed usage can become rewarder-compatible contribution snapshots
canonical reward snapshot bytes/CID are stable
bad contribution shapes reject
overflow rejects safely
```

---

# 12. Known Limitations

## 12.1 HTTP ingest adapter is not final production service

Current:

```text
std-only blocking HTTP server
thread-per-connection
minimal health/readiness
in-memory recorder
in-memory idempotency key set
no Prometheus metrics
no formal admin plane
no TLS/mTLS
no ron-auth/capability gate
```

Why acceptable now:

```text
It proves the live WEB3 path without new dependencies or service architecture churn.
```

Needed later:

```text
canonical service/admin shape
likely axum-based if workspace dependency support is confirmed
/metrics
truthful /readyz
bounded concurrency
request size limit middleware
structured error taxonomy
proper auth/capability verification
graceful shutdown
config file/env support
```

## 12.2 Idempotency is memory-only

Current:

```text
seen Idempotency-Key values are kept in memory
restart loses dedupe state
```

Needed:

```text
durable idempotency store or WAL-backed accepted batch IDs
bounded LRU with persistence options
amnesia-mode-specific in-memory-only policy
```

## 12.3 Recorder state is memory-only in live adapter

Current:

```text
snapshot rows live in process memory
restart loses current snapshot
```

Needed depending on production posture:

```text
WAL replay integration
snapshot export/checkpoint
configured amnesia behavior
bounded in-memory mode for micronode/amnesia mode
```

## 12.4 No exporter to rewarder yet

Current:

```text
ron-accounting can record and project reward snapshots
svc-rewarder has interop vector compatibility
but live accounting → rewarder → wallet payout proof is not yet wired
```

Needed next:

```text
record usage
seal/project snapshot
feed svc-rewarder
compute deterministic reward plan
commit payout/issue through svc-wallet
verify ledger/wallet receipt
```

## 12.5 Metrics/admin visibility incomplete

Current:

```text
live adapter has /healthz /readyz /v1/snapshot
no Prometheus metrics in the adapter
```

Needed:

```text
accounting_ingest_total
accounting_ingest_rejected_total
accounting_ingest_duplicate_total
accounting_rows_total
accounting_snapshot_rows
accounting_export_latency_seconds
readiness state
queue/backlog metrics
```

---

# 13. Recommended Next Work

## Priority 1 — Accounting → rewarder → wallet proof

This is now the highest-impact WEB3 integration step.

Goal:

```text
usage events
→ ron-accounting recorder
→ sealed/accounting snapshot
→ reward projection
→ svc-rewarder deterministic reward plan
→ svc-wallet issue/payout
→ ron-ledger commit
→ receipt and balance verification
```

Acceptance criteria:

```text
no floating-point math
integer minor units only
basis points only if splits are involved
deterministic reward plan
deterministic account ordering
same snapshot does not double pay
wallet is mutation front-door
rewarder does not mutate ledger directly
accounting does not mutate ledger directly
receipt is retrievable
ledger/wallet balance agrees after replay
```

Suggested test/script path:

```text
tests/web3_accounting_rewarder_wallet_loop.rs
or scripts/web3_accounting_rewarder_wallet_smoke.sh
```

## Priority 2 — Canonical ron-accounting service shape

Decide whether to:

```text
keep ron_accounting_ingest as the beta service
or replace/upgrade it into the canonical ron-accounting service/admin plane
```

Canonical service should eventually include:

```text
/healthz
/readyz
/metrics
/v1/usage-events
/v1/snapshot
/v1/seal
/v1/reward-snapshot
bounded request bodies
auth/capability gate
structured errors
graceful shutdown
config validation
```

## Priority 3 — WAL-backed ingest/idempotency

Add:

```text
WAL append for accepted usage batches
WAL replay on restart
durable idempotency key replay
corrupt-record skip behavior for HTTP ingest
amnesia-mode in-memory-only alternative
```

## Priority 4 — Metrics and observability

Add low-cardinality metrics:

```text
ron_accounting_ingest_total{status}
ron_accounting_ingest_events_total{status}
ron_accounting_ingest_duplicate_total
ron_accounting_rows_total
ron_accounting_snapshot_requests_total
ron_accounting_ingest_request_bytes_total
ron_accounting_ingest_latency_seconds
```

Do not include in metrics labels:

```text
CID
wallet_txid
account IDs
receipt hashes
user IDs
raw route variants with high cardinality
```

## Priority 5 — Stronger tests for HTTP ingest

Add dedicated tests for:

```text
healthz response
readyz response
valid ingest
duplicate idempotency key dedupe
missing auth rejects
missing idempotency key rejects
bad schema rejects
bad CID rejects
bad JSON rejects
zero-value event skip report
snapshot rows after ingest
```

Current live smoke proves the path, but dedicated unit/integration tests would lock it down.

---

# 14. Current Completion Impact

Before this plateau, `ron-accounting` was already a gold foundation crate:

```text
usage ingest
normalized counters
sealed slices
reward projection
reward snapshot interop vector
exporter lanes
WAL feature path
```

After this plateau, it also has:

```text
real HTTP ingest adapter
live process health
real storage usage export
snapshot inspection
three-process WEB3 proof
```

Updated honest status:

```text
ron-accounting crate overall:        ~92–95%
HTTP ingest beta path:               ~88–92%
storage→accounting live bridge:      ~96–99%
production service hardening:        ~70–80%
```

Project-level impact:

```text
RustyOnions now has a real internal economy circuit up through accounting:

wallet issue/hold
→ paid storage enforcement
→ wallet capture/release
→ accounting usage export
→ accounting snapshot rows
```

Remaining project-level gap:

```text
accounting snapshot
→ rewarder deterministic payout plan
→ wallet issue/payout
→ ledger receipt/replay
```

---

# 15. Suggested Commit Summary

```text
ron-accounting: add lightweight HTTP usage ingest for WEB3 smoke

- add std-only ron_accounting_ingest binary
- add /healthz, /readyz, /v1/usage-events, and /v1/snapshot
- accept svc-storage.usage-events.v1 export payloads
- require Idempotency-Key on usage ingest
- dedupe duplicate ingest keys in memory
- record usage events through existing record_usage_events path
- expose snapshot rows for live WEB3 smoke verification
- update lib exports for http_ingest
- update paid-storage live smoke to run ron-accounting + svc-wallet + svc-storage
- prove storage→accounting export with HTTP 202 and snapshot row_count=3
```

Longer body:

```text
This links the paid-storage ROC loop to a real ron-accounting process.
The live smoke now proves issue→hold→paid storage→capture→release→usage export→
accounting snapshot. ron-accounting remains usage/metering only and does not
mutate wallet or ledger state.
```

---

# 16. Guardrails / Do Not Regress

Do not regress these:

```text
Do not make ron-accounting the source of balance truth.
Do not let ron-accounting issue ROC directly.
Do not let ron-accounting mutate ron-ledger directly.
Do not use floating-point money math.
Do not put wallet txids/CIDs/account IDs into Prometheus labels.
Do not remove normalized label bounds.
Do not remove deterministic reward snapshot canonicalization.
Do not break rewarder interop vector shape.
Do not let usage ingest accept unknown/malformed schema silently.
Do not let duplicate idempotency keys double-record events.
Do not make storage accounting export failure silent.
Do not add ROX/Solana/external settlement logic.
Do not persist forbidden state in amnesia mode.
Do not hold locks across async await points.
```

Important current limitation:

```text
The lightweight HTTP ingest adapter is useful and green, but it is not yet the
final hardened production service. Treat it as beta proof and iterate toward
canonical service/admin/metrics/WAL behavior.
```

---

# 17. Current Status in One Paragraph

`ron-accounting` is now a strong WEB3 foundation crate with a live integration proof. It records normalized usage events, maintains transient counters, supports deterministic sealing/windowing, projects rewarder-compatible snapshots, provides a stable accounting↔rewarder interop vector, and now runs a lightweight HTTP ingest process that accepts real `svc-storage` usage exports. The latest live smoke proved `svc-storage` exported three usage events to `ron-accounting`, `ron-accounting` accepted them with HTTP 202, and `/v1/snapshot` returned the expected bytes/request rows. The remaining work is to harden the ingest adapter into a canonical service, add durable idempotency/WAL-backed ingest where appropriate, expand metrics/readiness/auth, and wire the next major loop: accounting snapshot → svc-rewarder plan → svc-wallet payout → ron-ledger receipt/replay.

---



```
```


### END NOTE - APRIL 28 2026 - 20:10 CST



### BEGIN NOTE - APRIL 28 2026 - 23:00 CST

# ron-accounting NOTES.MD — WEB3 Value-Plane Carryover Notes

Date: 2026-04-29
Crate: `ron-accounting`
Workspace path: `crates/ron-accounting`
Project: RustyOnions / WEB3 / ROC internal value plane
Session status: **Gold foundation + live storage ingest proof + rewarder interop vector proven**
Estimated current completion: **93–96% for WEB3 beta usage/reward accounting**, **75–82% for production-grade accounting service hardening**

---

## 0. Executive Summary

`ron-accounting` is now one of the strongest WEB3 foundation crates in RustyOnions. Its role is to collect real usage/metering signals, normalize and aggregate them, produce deterministic rewarder-compatible snapshots, and feed `svc-rewarder` with stable accounting inputs. It is **not** a ledger, wallet, issuer, balance keeper, or settlement engine. That boundary remains intact.

The crate already had a strong “Gold foundation” before this session: usage events, normalized labels, transient counters, deterministic slices, ordered exporter lanes, ACK/NACK-safe export behavior, optional WAL feature path, reward snapshot DTOs, reward projection, and stable accounting↔rewarder interop vectors. Existing notes already recorded the key invariant set: `ron-accounting` does not issue ROC, does not own balances, does not bypass `svc-wallet`, does not commit directly to `ron-ledger`, uses no floating-point value-plane math, keeps labels normalized/bounded, keeps sealed slices deterministic, and keeps `snapshot_cid = b3:<hash(canonical_snapshot_bytes)>`. 

This session strengthened `ron-accounting` in two important ways:

```text
1. It became a live participant in the paid-storage WEB3 smoke:
   svc-storage /paid/o
   → usage events
   → ron-accounting ingest HTTP adapter
   → normalized snapshot rows

2. It became the upstream source for the earning-side reward proof:
   ron-accounting reward snapshot vector
   → svc-rewarder compute
   → svc-rewarder wallet emit
   → svc-wallet issue
   → ledger-backed balances
   → replay does not double issue
```

The live paid-storage smoke proved `ron-accounting` starts as an ingest process, reports health, accepts usage events from `svc-storage`, and exposes a snapshot with three rows: `bytes_stored = 48`, `request_ok = 1`, and `pin_seconds = 60`. The terminal output showed `ron-accounting is healthy`, then the final snapshot contained `row_count: 3` with the expected normalized dimensions and labels. 

---

## 1. Strategic Role in WEB3 / ROC

`ron-accounting` is the **usage and metering layer** of the internal ROC economy.

Its correct role:

```text
collect usage events
normalize labels/routes/services/regions
aggregate transient counters
seal deterministic slices/windows
produce reward projection inputs
produce canonical reward snapshots
export usage/snapshot data
feed svc-rewarder and policy/pricing consumers
support dashboard/admin visibility
```

Its incorrect role — do not regress into this:

```text
not a wallet
not a ledger
not a settlement engine
not an issuer
not a burner
not a source of durable economic truth
not an external-chain bridge
not a Solana/ROX component
not a balance authority
```

Correct WEB3 value-plane boundary:

```text
svc-storage / svc-gateway / omnigate:
  enforce paid use cases
  emit usage events

ron-accounting:
  transient counters
  normalized rows
  sealed usage snapshots
  rewarder-compatible accounting inputs

svc-rewarder:
  deterministic reward epochs
  reward/payout planning
  wallet-compatible issue requests

svc-wallet:
  sole normal mutation front-door
  nonce + idempotency
  issue / transfer / burn / hold / capture / release
  receipts / balances

ron-ledger:
  append-only durable economic truth
  deterministic replay
  conservation and balance state
```

This session preserved that boundary. `ron-accounting` did not mutate balances, issue ROC, or talk directly to `ron-ledger`; it stayed a usage/snapshot source.

---

## 2. What Was Already Working Before This Session

Before this session, `ron-accounting` already had a strong foundation:

```text
usage event DTOs
usage event validation
label normalization
route normalization
tenant/service/region dimensions
transient counter recording
window/slice model
deterministic sealed slice digesting
reward snapshot DTOs
reward projection
canonical snapshot JSON
snapshot CID generation
export lanes
ACK/NACK-safe exporter behavior
idempotency/dedupe support in export paths
optional WAL modules
property tests
model/loom placeholder tests
unit coverage for ingest, projection, snapshots, rollover, WAL, exporter behavior
```

The crate notes described the intended pipeline as:

```text
usage events
→ normalized labels
→ transient counters
→ deterministic windowed sealed slices
→ ordered export lanes
→ ACK/NACK-safe exporter routing
→ optional WAL feature path
→ rewarder-compatible snapshot DTO
→ deterministic reward projection
→ stable accounting↔rewarder interop vector
→ canonical JSON + b3 CID proof
```

That same prior note identified `ron-accounting` as a “Gold foundation” crate and placed the remaining work mostly in production adapters, inter-service integration, WAL decisions, dashboard visibility, and stronger runtime hardening rather than core accounting math. 

---

## 3. What We Accomplished for ron-accounting in This Session

### 3.1 Added and used the reward snapshot vector emitter

We added/used a dedicated vector CLI:

```text
crates/ron-accounting/src/bin/ron_accounting_reward_snapshot_vector.rs
```

Purpose:

```text
Emit a deterministic ron-accounting reward snapshot interop vector.
Let svc-rewarder consume an accounting-produced snapshot fixture.
Prove canonical JSON + b3 CID are stable.
Avoid manual JSON hand-copying between crates.
```

Observed vector output in the live rewarder smoke:

```text
epoch_id = interop-epoch-1
snapshot_cid = b3:81d428e1df7c29a8443467db8ed59ee2628ac43acd47c692fbc3c23ee495606d
```

This vector was then passed into `svc-rewarder`, which computed deterministic payouts and emitted wallet issue requests through `svc-wallet`.

The important effect is that the earning-side ROC loop now has a stable accounting input contract:

```text
ron-accounting canonical snapshot
→ svc-rewarder validates/uses snapshot
→ svc-wallet issues ROC
→ replay does not double issue
```

### 3.2 Proved `ron-accounting → svc-rewarder` snapshot compatibility

The rewarder smoke consumed the accounting vector and proved:

```text
accounting-produced snapshot shape is accepted by svc-rewarder
snapshot CID is stable
epoch ID is stable
rewarder can compute deterministic payouts from the accounting vector
rewarder can preview and emit wallet issue requests
```

This closes a major gap that earlier notes called out as future work: “accounting snapshot → rewarder deterministic payout plan → wallet issue → ledger receipt/replay.” Earlier carryover notes explicitly named that as the missing project-level gap. 

### 3.3 Proved live storage usage export into ron-accounting

The paid-storage live smoke started a real `ron-accounting` ingest process:

```text
ron-accounting ingest on 127.0.0.1:19600
/healthz returns ok
svc-storage exports accounting events to it
/v1/snapshot exposes normalized rows
```

The live output proved:

```text
row_count = 3

rows:
  PIN_SECONDS / requests = 60
  PUT / bytes = 48
  REQ_OK / requests = 1
```

That means `svc-storage` is not merely simulating accounting. It is now exporting real usage to a live `ron-accounting` process, and `ron-accounting` is recording it into a queryable snapshot.

### 3.4 Preserved accounting as transient usage, not truth

In the live smoke, the economic mutation path was:

```text
svc-wallet issue
→ svc-wallet hold
→ svc-storage verifies paid context
→ svc-wallet capture/release
```

The accounting path was separate:

```text
svc-storage usage_events
→ ron-accounting ingest
→ ron-accounting snapshot rows
```

This is architecturally correct. `ron-accounting` observed usage and provided metering output; it did not decide balances, issue rewards, or settle payment.

### 3.5 Strengthened WEB3 closed-loop confidence

After this session, `ron-accounting` participates in both directions of the internal value plane:

Spend/use side:

```text
paid storage operation
→ usage exported to ron-accounting
→ snapshot rows prove network use happened
```

Earn/reward side:

```text
ron-accounting reward snapshot vector
→ svc-rewarder computes payout plan
→ svc-wallet issues ROC
```

This is a big milestone because WEB3 v1 requires at least one real in-network paid use case plus deterministic reward planning from accounting/policy signals.

---

## 4. Current Proven Capabilities

### 4.1 Usage event ingestion

Current status:

```text
usage events can be created by svc-storage
usage events include timestamp, tenant, subject, metric_kind, value, source_service, region, route
usage events are normalized into dimensions/labels
zero-value events can be skipped/reported
malformed events are rejected by validation paths
```

Live proof:

```text
svc-storage emitted:
  bytes_stored = 48
  request_ok = 1
  pin_seconds = 60

ron-accounting snapshot showed:
  dimension=bytes, method=PUT, value=48
  dimension=requests, method=REQ_OK, value=1
  dimension=requests, method=PIN_SECONDS, value=60
```

### 4.2 HTTP ingest adapter

Current live path:

```text
RON_ACCOUNTING_ADDR=127.0.0.1:19600
RON_ACC_BEARER=dev
cargo run -p ron-accounting --bin ron_accounting_ingest
```

Known HTTP surfaces from current notes:

```text
GET  /healthz
GET  /readyz
POST /v1/usage-events
GET  /v1/snapshot
```

The crate notes already recorded manual ingest and snapshot commands for this lightweight server. 

### 4.3 Snapshot inspection

The live process exposes a snapshot view for operator/dev proof:

```text
GET /v1/snapshot
```

The smoke output proved it returns normalized rows with:

```text
row_count
rows[]
schema = ron-accounting.snapshot.v1
```

This is enough for current beta proof and developer visibility, but not yet a full production admin/dashboard API.

### 4.4 Reward snapshot / rewarder interop

Current capabilities:

```text
reward snapshot DTO exists
canonical snapshot JSON exists
snapshot CID uses canonical b3 hash
interop vector exists
rewarder can consume the vector
rewarder and accounting agree on canonical snapshot CID
```

Prior notes already called out the interop vector and canonical CID rule as a stable contract. 

### 4.5 Exporter lane / idempotency foundation

The existing foundation includes:

```text
ordered export lanes
lease/ack/nack behavior
ACK LRU dedupe
gap rejection
retry policy
deterministic router backlog
other-stream leasing while one stream is inflight
```

This remains important for future production adapters where accounting exports to rewarder, dashboard, or durable systems.

### 4.6 WAL feature path

WAL mode exists and prior notes record it as green in earlier gates. Current notes say periodic WAL testing should remain part of the accounting quality discipline:

```bash
cargo test -p ron-accounting --all-targets --features wal
```

WAL is still not the source of economic truth. It is only optional persistence/replay support for accounting data. The ledger remains truth.

---

## 5. Quality Gates Observed This Session

The session included a full `ron-accounting` test pass early in the work. The observed run included:

```text
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
```

Observed results included:

```text
ron-accounting clippy passed
ron_accounting_reward_snapshot_vector binary built as a test target
2 loom/model placeholder tests passed
3 property tests passed
37 unit tests passed
record bench smoke passed
seal bench smoke passed
examples built as test targets
```

The live paid-storage smoke also started `ron-accounting`, confirmed health, exported real events, and queried snapshot rows successfully. 

Recommended standing gate for this crate remains:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
cargo test -p ron-accounting --all-targets --features wal
```

Optional targeted gates:

```bash
cargo test -p ron-accounting --test unit
cargo test -p ron-accounting --test prop
cargo test -p ron-accounting --test loom
cargo run -p ron-accounting --example minimal
cargo run -p ron-accounting --example export_to_mock
cargo bench -p ron-accounting
```

---

## 6. Current Completion Estimate

Updated estimate after this session:

```text
ron-accounting core library foundation:       92–95%
usage event ingest / normalization:           96–98%
reward snapshot / canonical CID contract:     94–97%
accounting ↔ rewarder interop vector:         95–98%
lightweight HTTP ingest adapter:              88–92%
live storage → accounting proof:              97–99%
WAL production posture:                       72–80%
dashboard/admin/API production hardening:     70–78%
overall ron-accounting WEB3 beta readiness:   93–96%
```

Interpretation:

```text
For WEB3 beta:
  ron-accounting is basically ready as a usage/snapshot substrate.

For production:
  it still needs stronger durability policy, richer admin surfaces,
  production authentication/capability wiring, deeper chaos/loom tests,
  dashboard DTOs, and operational hardening.
```

---

## 7. Important Invariants to Preserve

Do not regress any of these:

```text
ron-accounting is not a ledger
ron-accounting does not issue ROC
ron-accounting does not burn ROC
ron-accounting does not hold balances
ron-accounting does not mutate ron-ledger
ron-accounting does not bypass svc-wallet
ron-accounting remains usage/snapshot input only
no ROX / Solana / external settlement logic
no floating-point value-plane math
all counters and quantities remain integers
labels must remain normalized and bounded
routes must remain normalized and bounded
no raw PII in labels
no wallet txids / CIDs / receipt hashes / account IDs in Prometheus labels
sealed snapshots must be deterministic
canonical snapshot bytes must remain stable
snapshot_cid must remain b3:<64 lowercase hex>
failed export must not silently lose a sealed slice
duplicate idempotency keys must not double-record usage
WAL persistence must be explicit
amnesia mode must not silently write forbidden durable state
no locks across async await in future adapters
```

Important high-cardinality labels to avoid in metrics:

```text
CID
wallet_txid
account IDs
receipt hashes
user IDs
raw unbounded routes
raw object names
raw passport IDs
```

---

## 8. What Remains for ron-accounting

### Priority A — Production-grade ingest tests

The live smoke proves the HTTP ingest path, but dedicated tests should lock it down.

Needed tests:

```text
/healthz returns expected shape
/readyz returns expected shape
valid usage ingest returns 202
duplicate Idempotency-Key dedupes
missing Authorization rejects
missing Idempotency-Key rejects
bad schema rejects
bad CID rejects
bad wallet_txid shape rejects if enforced
bad JSON rejects
zero-value event skip report is stable
snapshot rows after ingest are deterministic
snapshot route preserves schema
```

Current live proof is strong, but crate-local tests should make regressions impossible.

### Priority B — Snapshot export API/command

Current snapshot inspection is good for dev proof. The next production step is a stable snapshot export surface for rewarder.

Possible forms:

```text
GET /v1/reward-snapshot
GET /v1/windows/:window_id/reward-snapshot
cargo run -p ron-accounting --bin ron_accounting_snapshot_export
library function: export_reward_snapshot(window_id)
```

The output should include:

```text
produced_at_millis
pool_minor_units
contributions[]
canonical_snapshot_json
snapshot_cid
schema/version
```

### Priority C — Sealed window lifecycle

Accounting needs a stronger distinction between:

```text
open/current counters
sealed slices
sealed reward snapshots
exported snapshots
replayed snapshots
```

Needed statuses:

```text
open
sealed
export_pending
exported
export_failed
replayed
expired/compacted
```

Important: once a reward snapshot is sealed, it should be immutable and deterministic for that window.

### Priority D — Durable WAL production decisions

WAL exists, but production policy remains unresolved.

Decisions needed:

```text
Is WAL enabled by default in macronode?
Is WAL disabled by default in amnesia/micronode?
Where do WAL files live?
What retention policy applies?
What compaction policy applies?
How are corrupt records reported?
Does startup replay happen automatically?
What metrics expose replay status?
What happens if replay partially succeeds?
```

WAL must never be treated as ledger truth.

### Priority E — Stronger concurrency and failure modeling

Existing model tests are not enough for production confidence.

Needed deeper tests:

```text
loom model for ACK/NACK interleavings
loom model for shutdown while export is inflight
loom model for exporter worker cancellation
bounded queue under load
export retry under partial failure
idempotent duplicate event ingest under concurrent requests
no lock across await in HTTP ingest path
failed export does not lose slice
shutdown drains or explicitly preserves in-flight usage
```

### Priority F — Dashboard/admin visibility

Needed DTOs/endpoints:

```text
usage by service
usage by tenant
usage by route
usage by metric kind
storage bytes by provider
served bytes by provider
pin seconds by provider
active window status
sealed snapshot status
snapshot CID
last export result
export backlog
ingest reject counts
dedupe/replay counts
```

Dashboard-friendly summary shape:

```text
GET /v1/accounting/summary
GET /v1/accounting/windows
GET /v1/accounting/windows/:id
GET /v1/accounting/provider/:id
```

### Priority G — Production auth/capability model

The current local ingest server uses dev bearer-style auth. Production should use the project’s capability model.

Needed:

```text
ron-auth capability verification
scope: accounting.ingest
scope: accounting.snapshot.read
scope: accounting.admin
short TTL tokens
issuer/audience validation
tenant caveats
service caveats
body size caveats
route/method caveats
fail-closed behavior
```

### Priority H — Production service integration

Current local ingest is enough for beta smoke. Production should decide how `ron-accounting` is hosted:

```text
standalone service binary
macronode-managed service
micronode optional local service
library embedded in service
UDS-only internal adapter
HTTP internal adapter
OAP adapter later
```

### Priority I — Canonical test vector files

The code-level vector exists and is proven. Still useful to write checked-in files:

```text
crates/ron-accounting/tests/vectors/reward_snapshot_v1.json
crates/ron-accounting/tests/vectors/reward_snapshot_v1.cid
crates/ron-accounting/tests/vectors/usage_events_v1.json
crates/ron-accounting/tests/vectors/projected_snapshot_v1.json
```

These can be imported by `svc-rewarder` tests to prevent drift.

### Priority J — Reward policy / anti-gaming inputs

Accounting will eventually need richer input signals for rewarder anti-gaming:

```text
per-account caps
per-node caps
per-tenant caps
minimum uptime thresholds
storage proof weighting
served-byte weighting
request quality weighting
geographic/regional balancing
Sybil-resistant passport hooks
quarantine suspicious concentration
rate-of-change caps
signed accounting windows
provider identity binding
```

Keep this as signal generation only; rewarder/policy should decide payouts.

---

## 9. Known Commands

### Full local quality gate

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
```

### WAL feature gate

```bash
cargo test -p ron-accounting --all-targets --features wal
```

### Run lightweight ingest server manually

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600 \
RON_ACC_BEARER=dev \
cargo run -p ron-accounting --bin ron_accounting_ingest
```

### Probe health

```bash
curl -fsS http://127.0.0.1:19600/healthz | jq .
curl -fsS http://127.0.0.1:19600/readyz | jq .
```

### Inspect snapshot

```bash
curl -fsS http://127.0.0.1:19600/v1/snapshot | jq .
```

### Emit reward snapshot vector

```bash
cargo run -p ron-accounting --bin ron_accounting_reward_snapshot_vector -- --json-pretty
cargo run -p ron-accounting --bin ron_accounting_reward_snapshot_vector -- --json
cargo run -p ron-accounting --bin ron_accounting_reward_snapshot_vector -- --snapshot-json
cargo run -p ron-accounting --bin ron_accounting_reward_snapshot_vector -- --snapshot-cid
cargo run -p ron-accounting --bin ron_accounting_reward_snapshot_vector -- --epoch-id
```

### Run paid-storage live smoke involving ron-accounting

```bash
bash scripts/web3_paid_storage_live_smoke.sh
```

Expected final line:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```

### Run rewarder/accounting/wallet live smoke

```bash
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Expected final line:

```text
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```

---

## 10. Suggested NOTES.MD Commit Summary

```text
ron-accounting: document WEB3 live usage + reward interop plateau

- preserve accounting as transient usage/snapshot substrate, not ledger truth
- record current usage ingest, normalization, sealed snapshot, reward projection status
- document reward snapshot interop vector used by svc-rewarder
- document live paid-storage export into ron-accounting ingest
- document live snapshot rows for bytes_stored/request_ok/pin_seconds
- document remaining production work: ingest tests, sealed windows, WAL policy,
  dashboard DTOs, auth/capability gating, and stronger concurrency tests
```

Longer body:

```text
ron-accounting now participates in both sides of the WEB3 ROC loop. On the spend/use
side, svc-storage exports paid-storage usage events into the live accounting ingest
adapter and /v1/snapshot shows normalized rows. On the earn/reward side, the
ron_accounting_reward_snapshot_vector binary emits a canonical rewarder-compatible
snapshot consumed by svc-rewarder, which then emits wallet issues through svc-wallet.

The crate remains correctly bounded: it records usage and produces snapshots only.
It does not issue ROC, mutate ron-ledger, own balances, or bypass svc-wallet.
```

---

## 11. Final Status

`ron-accounting` should currently be considered:

```text
Status: Gold foundation + beta-ready WEB3 accounting substrate
WEB3 beta readiness: 93–96%
Production hardening readiness: 75–82%
Main remaining work: service hardening, sealed-window API, WAL policy,
auth/capability enforcement, dashboard/admin surfaces, and deeper concurrency tests.
```




### END NOTE - APRIL 28 2026 - 23:00 CST


### BEGIN NOTE - JUNE 14 2026 - 12:40 CST

Your latest terminal output confirms the `ron-accounting` parking gate passed, the crate-local runbook exists, workflow package names are clean, and the updated codebundle was generated with 73 files. 

# ron-accounting QuickChain Carryover Notes

RO:WHAT — Carryover notes for the `ron-accounting` crate after completing the current QuickChain Phase-0/preflight hardening pass.

RO:WHY — Preserve the exact state of the crate, what was accomplished, what remains, and what the next session should do without re-opening unsafe QuickChain scope.

RO:INTERACTS — `ron-accounting`, `svc-rewarder`, `svc-wallet`, `ron-ledger`, `ron-proto`, QuickChain QC-0A docs, future reward manifests, future accounting/reward roots.

RO:INVARIANTS — `ron-accounting` is derivative metering/snapshot infrastructure only; it is not balance truth, not wallet authority, not ledger authority, not payout authority, not root-producing code, and not settlement/finality infrastructure.

RO:SECURITY — No fake balances, no fake receipts, no wallet mutation, no ledger mutation, no QuickChain roots, no validators, no checkpoints, no settlement, no anchors, no pruning, no raw engagement protocol payouts.

RO:TEST — Current parking gate: `crates/ron-accounting/scripts/dev-quickchain-park.sh`.

---

## 0. Current crate status

`ron-accounting` is parked for the current QuickChain Phase-0 pass.

Status estimate:

* `ron-accounting` QuickChain Phase-0 slice: 95–96% complete.
* Future QuickChain root/accounting-root/reward-root role: intentionally not started.
* Next active crate: `svc-rewarder`.

The crate now has a strong boundary posture:

* It records bounded usage/metering.
* It normalizes usage labels.
* It seals deterministic accounting artifacts.
* It exports reward-planning-compatible snapshots.
* It rejects authority drift at DTO boundaries.
* It rejects raw engagement becoming protocol ROC reward authority.
* It documents that accounting artifacts are not roots.
* It documents that reward snapshots are planning inputs only.
* It has a final parking script that proves all current gates are green.

The final parking command passed:

```
cargo fmt -p ron-accounting
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

The output ended with:

```
== ron-accounting QuickChain parking gate passed ==
```

An updated crate codebundle was generated afterward:

```
bash scripts/make_crate_codex.sh -c ron-accounting --force
```

The codebundle output reported:

```
Wrote 73 files to: /Users/mymac/Desktop/RustyOnions/crates/ron-accounting/CODEBUNDLE.md
```

---

## 1. What ron-accounting is responsible for

`ron-accounting` is the metering and usage-snapshot layer.

It may:

* accept strict usage-event DTOs;
* validate schema, b3 CID shapes, labels, timestamps, and batch size;
* normalize labels and routes;
* record bounded counters;
* maintain in-memory recorder rows;
* seal usage slices deterministically;
* export sealed slices through ordered lanes;
* optionally persist/replay WAL slices when the WAL feature is enabled;
* build rewarder-compatible contribution snapshots;
* produce deterministic artifact CIDs over canonical snapshot bytes;
* provide interop vectors for `svc-rewarder`.

It may not:

* create balances;
* display balances as truth;
* create receipts;
* mint ROC;
* burn ROC;
* transfer ROC;
* hold ROC;
* capture holds;
* release holds;
* mutate `svc-wallet`;
* mutate `ron-ledger`;
* mark finality;
* produce QuickChain roots;
* produce accounting roots;
* produce reward roots;
* produce checkpoint hashes;
* sign checkpoints;
* validate checkpoints;
* anchor externally;
* prune history;
* treat raw engagement as protocol reward authority.

The correct pipeline remains:

```
usage / metering
→ ron-accounting counters
→ sealed accounting slices
→ reward-planning snapshot artifact
→ svc-rewarder payout plan
→ svc-wallet explicit mutation path
→ ron-ledger durable truth
```

---

## 2. Relationship to the internal ROC value loop

The intended internal ROC value loop is:

```
ron-proto econ DTOs
→ ron-ledger truth
→ svc-wallet issue / transfer / burn / hold / capture / release / receipt
→ svc-storage / svc-gateway / omnigate paid enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ wallet / ledger receipts
```

`ron-accounting` sits after paid enforcement and before reward planning.

This crate does not decide who gets paid. It only produces deterministic, bounded, validated metering inputs that later policy/rewarder code may consume.

`svc-rewarder` plans payouts.

`svc-wallet` commits approved economic mutations.

`ron-ledger` remains durable economic truth.

---

## 3. Major accomplishments in this pass

### 3.1 QuickChain preflight script added

File:

```
crates/ron-accounting/scripts/dev-quickchain-preflight.sh
```

Purpose:

* Runs crate formatting check.
* Runs strict clippy for `ron-accounting`.
* Runs all-target tests.
* Runs all-target tests with WAL feature enabled.
* Runs focused QuickChain preflight test suites.

Current focused test list:

```
quickchain_preflight_boundary
quickchain_preflight_ingest_poisoning
quickchain_preflight_snapshot_non_authority
quickchain_preflight_reward_dto_strictness
quickchain_preflight_reward_projection_boundary
quickchain_preflight_event_class_boundary
```

Expected success marker:

```
== ron-accounting QuickChain preflight gate passed ==
```

This script is the normal local gate before claiming the crate is still safe for the current QuickChain Phase-0 slice.

---

### 3.2 Final parking script added

File:

```
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

Purpose:

* Runs the full crate preflight script.
* Verifies stale `ron-accounting2` workflow package references are gone.
* Verifies the crate-local QuickChain preflight runbook exists.
* Verifies the runbook contains key boundary sections.
* Runs `cargo check --workspace`.
* Marks the crate parkable when all checks pass.

Expected success marker:

```
== ron-accounting QuickChain parking gate passed ==
```

This script is now the final “safe to leave this crate” gate.

---

### 3.3 Crate-local QuickChain runbook added

File:

```
crates/ron-accounting/docs/quickchain-preflight.md
```

Purpose:

* Documents that `ron-accounting` is not a chain, ledger, wallet, settlement service, root producer, validator, or payout executor.
* Documents its exact role in the RustyOnions internal ROC value loop.
* Documents “accounting is not balance truth.”
* Documents artifact CID versus QuickChain root boundaries.
* Documents event-class doctrine.
* Documents ingest boundary.
* Documents reward projection boundary.
* Documents current QuickChain preflight test inventory.
* Documents no-go checklist for parking.
* Documents handoff to `svc-rewarder`.

This file should be treated as the crate-local safety contract for current QuickChain work.

---

### 3.4 Stale crate workflow references fixed

The crate-local GitHub workflow files previously referenced:

```
ron-accounting2
```

They were changed to:

```
ron-accounting
```

Files fixed:

```
crates/ron-accounting/.github/workflows/clippy.yml
crates/ron-accounting/.github/workflows/coverage.yml
crates/ron-accounting/.github/workflows/perf-gate.yml
crates/ron-accounting/.github/workflows/public-api.yml
```

Current parking script checks for stale `ron-accounting2` references and fails if they reappear.

---

## 4. QuickChain boundary tests completed

### 4.1 General boundary tests

File:

```
crates/ron-accounting/tests/quickchain_preflight_boundary.rs
```

Purpose:

* Proves usage DTOs reject smuggled QuickChain authority fields.
* Proves ingest DTOs reject smuggled QuickChain authority fields.
* Proves raw engagement usage events are metering, not protocol ROC authority.
* Proves reward snapshots serialize as planning artifacts, not balance/root objects.
* Proves projected reward snapshots are artifact CIDs, not QuickChain roots.

Important completed tests:

* `usage_event_dto_rejects_smuggled_quickchain_authority_fields`
* `ingest_request_dto_rejects_smuggled_quickchain_authority_fields`
* `raw_engagement_usage_event_is_metering_not_protocol_roc_authority`
* `reward_snapshot_serializes_as_planning_artifact_not_balance_or_root`
* `projected_reward_snapshot_is_artifact_cid_not_quickchain_root`

Status: green.

---

### 4.2 Ingest poisoning tests

File:

```
crates/ron-accounting/tests/quickchain_preflight_ingest_poisoning.rs
```

Purpose:

* Proves strict ingest validation.
* Rejects invalid nested usage events before recording.
* Rejects noncanonical CIDs before dedupe authority.
* Rejects schema drift and empty required fields.
* Rejects body-supplied idempotency or operation identity.
* Rejects QuickChain root/finality poison fields.
* Rejects oversized batches.

Important completed tests:

* `strict_ingest_request_validates_minimal_storage_export`
* `ingest_request_rejects_invalid_nested_usage_events_before_recording`
* `ingest_request_rejects_noncanonical_cid_shapes_before_dedupe_authority`
* `ingest_request_rejects_schema_drift_and_empty_required_fields`
* `ingest_request_body_cannot_smuggle_idempotency_or_operation_identity`
* `ingest_request_rejects_quickchain_root_and_finality_poison_fields`
* `ingest_request_rejects_oversized_batches`

Status: green.

---

### 4.3 Snapshot non-authority tests

File:

```
crates/ron-accounting/tests/quickchain_preflight_snapshot_non_authority.rs
```

Purpose:

* Proves canonical reward snapshots are contribution artifacts only.
* Proves snapshot CIDs are artifact hashes, not root fields.
* Proves canonical bytes do not contain finality/mutation vocabulary.
* Proves `pool_minor_units` is an integer string, not a float or JSON number.
* Proves duplicate accounts and invalid account strings reject.

Important completed tests:

* `reward_snapshot_json_shape_is_contribution_artifact_only`
* `canonical_snapshot_cid_is_artifact_hash_not_root_field`
* `canonical_snapshot_bytes_do_not_contain_finality_or_mutation_vocabulary`
* `pool_minor_units_is_integer_string_not_float_or_json_number`
* `duplicate_accounts_and_invalid_account_strings_reject`

Status: green.

---

### 4.4 Reward DTO strictness tests

File:

```
crates/ron-accounting/tests/quickchain_preflight_reward_dto_strictness.rs
```

Purpose:

* Proves reward-related DTOs reject unknown authority fields.
* Proves nested reward-contribution drift rejects.
* Proves projection config/report/result shapes are strict.
* Proves valid current shapes still roundtrip.

Important completed tests:

* `reward_snapshot_export_rejects_unknown_authority_fields`
* `reward_contribution_export_rejects_unknown_authority_fields`
* `nested_reward_contribution_rejects_unknown_authority_fields`
* `reward_projection_config_rejects_unknown_authority_fields`
* `reward_projection_report_rejects_unknown_authority_fields`
* `projected_reward_snapshot_rejects_unknown_top_level_authority_fields`
* `projected_reward_snapshot_rejects_unknown_nested_snapshot_authority_fields`
* `strict_reward_dtos_still_roundtrip_valid_current_shapes`

Status: green.

---

### 4.5 Reward projection boundary tests

File:

```
crates/ron-accounting/tests/quickchain_preflight_reward_projection_boundary.rs
```

Purpose:

* Proves projection accepts only storage/served/uptime-style eligible metering inputs.
* Proves analytics/raw engagement metrics are ignored.
* Proves non-reward metrics alone cannot project rewards.
* Proves projection artifacts do not expose payout or chain authority fields.
* Proves deterministic ordering without pretending to be a root.
* Proves float money and empty reward inputs reject.

Important completed tests:

* `projection_accepts_storage_served_and_uptime_but_ignores_analytics`
* `raw_engagement_and_non_reward_metrics_alone_cannot_project_rewards`
* `current_event_class_doctrine_is_narrow_and_explicit`
* `projection_artifact_does_not_expose_payout_or_chain_authority_fields`
* `projection_order_is_deterministic_but_still_not_a_root`
* `projection_rejects_float_money_and_empty_reward_inputs`

Status: green.

---

### 4.6 Event-class boundary tests

File:

```
crates/ron-accounting/tests/quickchain_preflight_event_class_boundary.rs
```

Purpose:

* Proves client-supplied `event_class` and reward-authority fields reject.
* Proves ingest request rejects top-level and nested event-class fields.
* Proves only explicit reward-projection metric kinds are currently eligible.
* Proves eligible metering events project but remain planning-only.
* Proves analytics/raw engagement metrics remain request counters and do not project rewards.
* Proves ad-budgeted style events do not become protocol reward authority.
* Proves pin, CPU, and plain request metrics do not project rewards.
* Proves attribution mode cannot turn raw engagement into reward authority.
* Proves route wording cannot smuggle reward authority into request metrics.

Important completed tests:

* `usage_event_rejects_client_supplied_event_class_and_reward_authority`
* `ingest_request_rejects_top_level_and_nested_event_class_fields`
* `only_explicit_reward_projection_metric_kinds_are_currently_eligible`
* `reward_eligible_metering_events_project_but_remain_planning_only`
* `analytics_engagement_metrics_remain_requests_and_do_not_project_rewards`
* `ad_budgeted_style_events_do_not_become_protocol_reward_authority`
* `pin_cpu_and_plain_request_metrics_do_not_project_rewards`
* `attribution_mode_does_not_turn_raw_engagement_into_reward_authority`
* `route_wording_cannot_smuggle_reward_authority_into_request_metrics`

Status: green.

---

## 5. Core code boundaries currently established

### 5.1 UsageEvent

Relevant file:

```
crates/ron-accounting/src/accounting/events.rs
```

Current important properties:

* `UsageEvent` uses strict serde unknown-field rejection.
* `timestamp_ms` must be nonzero.
* `subject` must be nonempty and bounded.
* `source_service` must be nonempty and bounded if present.
* `region` must be nonempty and bounded if present.
* custom metric names must be nonempty and bounded.
* `to_counter()` normalizes route and labels before recording.
* zero-value events are skipped as no-ops.
* usage events are counters, not balances and not ledger entries.

Current metric reward-projection eligibility:

* `BytesStored`: eligible.
* `BytesServed`: eligible.
* `UptimeSeconds`: eligible.
* `PinSeconds`: not directly eligible.
* `RequestOk`: not directly eligible.
* `CpuUnits`: not directly eligible.
* `Custom(...)`: not directly eligible.

Important note:

Do not add new reward-eligible metric kinds casually. Any new eligible metric kind must come with explicit tests proving it cannot be raw engagement protocol payout authority.

---

### 5.2 HTTP ingest

Relevant file:

```
crates/ron-accounting/src/http_ingest.rs
```

Current important properties:

* `UsageEventsIngestRequest` uses strict serde unknown-field rejection.
* schema must match the expected storage usage-events schema.
* `cid` must be canonical lowercase `b3:<64hex>`.
* wallet/source fields must be nonempty and bounded.
* event batches are capped.
* every nested event validates before recording.
* `Idempotency-Key` belongs in the header.
* body-supplied idempotency/operation identity is rejected by unknown-field strictness tests.
* malformed/poisoned payloads reject before dedupe/recording.

Important note:

The ingest adapter is not an economic command surface. It must remain usage-only.

---

### 5.3 RewardSnapshotExport

Relevant file:

```
crates/ron-accounting/src/accounting/reward_snapshot.rs
```

Current important properties:

* `RewardSnapshotExport` uses strict serde unknown-field rejection.
* `RewardContributionExport` uses strict serde unknown-field rejection.
* `pool_minor_units` is string-encoded integer money.
* floats reject.
* JSON numbers reject at the boundary.
* account strings are trimmed/canonicalized and sorted.
* duplicate accounts reject.
* contribution score arithmetic is checked.
* total score arithmetic is checked.
* canonical bytes are deterministic.
* canonical CID is `b3:<64 lowercase hex>` over canonical bytes.

Important note:

`canonical_cid()` is an artifact CID, not a QuickChain root.

---

### 5.4 Reward projection

Relevant file:

```
crates/ron-accounting/src/accounting/reward_projection.rs
```

Current important properties:

* projection config/report/result DTOs use strict serde unknown-field rejection.
* projection is deterministic over sorted account outputs.
* projection groups by tenant/service or tenant/service/region depending on account mode.
* projection maps bytes stored, bytes served, and uptime-like rows into contribution counters.
* non-mapped rows are ignored and counted in the report.
* projection rejects empty contribution sets through snapshot validation.
* projection rejects overflow.
* projection produces `ProjectedRewardSnapshot`.
* `ProjectedRewardSnapshot.snapshot_cid` is an artifact CID.

Important note:

Projection is not payout. Projection is not reward execution. Projection does not mutate ledger or wallet.

---

### 5.5 Recorder and sealed slices

Relevant files:

```
crates/ron-accounting/src/accounting/recorder.rs
crates/ron-accounting/src/accounting/slice.rs
```

Current important properties:

* recorder is sharded and bounded.
* recorder uses non-negative integer counters.
* zero increments are no-ops.
* rows are sorted deterministically for snapshots and sealed slices.
* sealing drains only the requested stream.
* sealed slices include canonical digest.
* sealed slices are immutable artifact snapshots.
* slice digest is not a QuickChain root.
* sequence handling remains caller/exporter responsibility.

Important note:

The sealed-slice digest is useful for artifact integrity and ordered export. Do not rename it to `accounting_root`.

---

### 5.6 Interop vector

Relevant file:

```
crates/ron-accounting/src/accounting/interop.rs
```

Current important properties:

* `reward_snapshot_interop_vector_v1()` produces a deterministic accounting → rewarder fixture.
* vector includes sample usage events.
* vector includes a canonical rewarder-compatible snapshot.
* vector includes snapshot CID.
* vector includes canonical snapshot JSON.
* vector validates internal consistency.
* unit tests prove vector stability.

Important note:

This interop vector is very useful for `svc-rewarder`. It should be consumed or mirrored in `svc-rewarder` tests next.

---

### 5.7 WAL status

Relevant files:

```
crates/ron-accounting/src/wal/*.rs
crates/ron-accounting/tests/unit/wal_tests.rs
```

Current status:

* WAL feature tests pass.
* With default/no WAL feature, placeholder test confirms disabled posture.
* With `--features wal`, WAL append/drain and corrupt-record replay tests pass.
* WAL is good enough for current crate parking.

Future WAL work may still be desirable:

* stronger crash/restart simulation;
* bounded quota exhaustion tests;
* replay ordering across multiple segments;
* partial-write simulation;
* fsync behavior review;
* integration with exporter ack/retry lifecycle;
* metrics for WAL replay and corruption.

Important note:

WAL work is not currently blocking QuickChain Phase-0 parking.

---

## 6. What the final parking gate proves

The final parking script proves:

1. Formatting is stable.
2. `ron-accounting` clippy is green with `-D warnings`.
3. All-target tests pass.
4. WAL-feature tests pass.
5. Focused QuickChain preflight tests pass.
6. Stale `ron-accounting2` workflow references are gone.
7. Crate-local QuickChain runbook exists.
8. Workspace check passes.

Known unrelated warnings from other crates may still appear:

* `micronode`: unused import `UpstreamSpec`.
* `ryker`: field `max_msg_bytes` is never read.
* `omnigate`: constant `MAX_ACCOUNT_BYTES` is never used.

These are not `ron-accounting` blockers.

---

## 7. What remains for ron-accounting later

The crate is parkable now, but not permanently finished. Future work should be done only when it fits the broader QuickChain timeline.

### 7.1 Future docs polish

Possible future docs additions:

* `README.md` update linking to `docs/quickchain-preflight.md`.
* operator runbook for ingest adapter.
* WAL operator notes.
* rewarder interop notes.
* examples showing accounting → rewarder handoff.

Current docs are enough to park the crate.

---

### 7.2 Future CI cleanup

Possible future cleanup:

* decide whether crate-local `.github/workflows` should remain inside crate or be moved to repo root;
* add `dev-quickchain-park.sh` to a repo-level CI matrix;
* add workspace-wide warning-deny policy only after unrelated crates are cleaned;
* consider a CI job that runs the parking script directly.

Current workflow package-name hygiene is fixed.

---

### 7.3 Future ingest hardening

Possible future improvements:

* stronger HTTP adapter tests if this becomes production-facing;
* auth/bearer test coverage;
* replay/dedupe storage review;
* request body hard-limit tests;
* malformed JSON body tests;
* route normalization edge cases;
* service/source label cardinality controls;
* more explicit error mapping for public gateway use.

Do not add economic authority to ingest.

---

### 7.4 Future reward projection hardening

Possible future improvements:

* make projection classification policy more explicit and versioned;
* move reward-eligible metric policy to `ron-policy` or a strict config object;
* add funding-source handoff fields only in `svc-rewarder`, not as payout authority here;
* add more projection fixtures from realistic storage/gateway slices;
* add regression tests for route/method edge cases;
* add explicit rejection for ambiguous future metric names if needed.

Do not make projection execute payouts.

---

### 7.5 Future WAL/export durability

Possible future improvements:

* crash/restart tests around WAL replay;
* exporter ack lifecycle integration tests;
* multiple-lane replay ordering tests;
* quota exhaustion tests;
* WAL compaction tests;
* corruption metrics;
* fsync/failure-mode review.

Do not let WAL replay become ledger replay. WAL replay is accounting artifact recovery only.

---

### 7.6 Future QuickChain root integration

This is intentionally not started.

Future QuickChain work may eventually need:

* canonical accounting-window DTOs;
* accounting-window root preimage definitions;
* `accounting_root` test vectors;
* reward-root test vectors;
* canonical bytes for event batches;
* event-root and aggregate-root formats;
* fixture vectors connecting `ron-accounting` artifacts to future QuickChain epoch artifacts.

However, do not implement this until the broader QuickChain vector gates are explicitly green.

Forbidden until then:

* no accounting roots;
* no reward roots;
* no checkpoint hashes;
* no validator-executable transition;
* no root-producing code;
* no settlement labels;
* no pruning behavior;
* no external anchors.

---

## 8. What not to do when returning to ron-accounting

Do not add fields like these to current DTOs unless a future gated QuickChain phase explicitly requires them and tests them:

* `accounting_root`
* `reward_root`
* `state_root`
* `receipt_root`
* `checkpoint_root`
* `checkpoint_hash`
* `settlement_status`
* `finalized`
* `validator`
* `validator_signature`
* `anchor`
* `bridge`
* `stake`
* `liquidity`
* `wallet_mutation`
* `ledger_mutation`
* `operation_id`
* `account_sequence`
* `receipt_id`
* `payout_authorized`
* `mint_authorized`

Do not add direct calls from `ron-accounting` to:

* `svc-wallet` mutation routes;
* `ron-ledger` mutation APIs;
* settlement/checkpoint services;
* external anchor services;
* validator services.

Do not rename artifact CIDs or slice digests as roots.

Do not allow raw engagement metrics to become primary protocol ROC reward denominators.

Do not add floating-point money math.

Do not accept JSON-number money values where string integer minor units are required.

---

## 9. Recommended next crate: svc-rewarder

The next crate should be:

```
crates/svc-rewarder
```

Reason:

`ron-accounting` now emits deterministic reward-planning inputs. The next safety boundary is making sure `svc-rewarder` consumes those inputs as planning data only and cannot mutate economic truth directly.

The expected handoff is:

```
ron-accounting RewardSnapshotExport / ProjectedRewardSnapshot
→ svc-rewarder deterministic payout plan
→ explicit funding/policy gates
→ svc-wallet approved mutation request
→ ron-ledger durable receipt/balance truth
```

`svc-rewarder` must prove:

* it consumes accounting snapshots as planning input only;
* it does not mutate `ron-ledger` directly;
* it does not mutate `svc-wallet` directly except through approved wallet front-door APIs if that is how the service is designed;
* it emits deterministic payout plans/intents only;
* it requires explicit funding source for reward plans;
* it rejects raw engagement as primary protocol ROC denominator;
* it cannot double-plan or double-issue across replay;
* it routes economic mutation through `svc-wallet`;
* it never fabricates receipts;
* it never fabricates balances.

Recommended first `svc-rewarder` QuickChain preflight tests:

* `quickchain_preflight_boundary.rs`
* `quickchain_preflight_raw_engagement_rejection.rs`
* `quickchain_preflight_funding_source_required.rs`
* `quickchain_preflight_no_wallet_ledger_mutation.rs`
* `quickchain_preflight_replay_no_double_issue.rs`
* `quickchain_preflight_accounting_interop.rs`

Likely first `svc-rewarder` goals:

1. Read the current `svc-rewarder` codebundle.
2. Find current snapshot/payout plan DTOs.
3. Verify whether it already consumes `ron-accounting` interop vector shape.
4. Add strict unknown-field rejection to rewarder-facing DTOs where safe.
5. Add tests proving rewarder has no direct ledger/wallet mutation authority.
6. Add tests proving raw engagement cannot become protocol ROC payout basis.
7. Add funding-source requirement if missing.
8. Add deterministic replay/no-double-plan test.

---

## 10. Completion estimate after this pass

Current estimate:

```
ron-accounting QuickChain Phase-0 slice: 95–96%
ron-accounting park status: parked
future accounting_root / reward_root role: 0% by design
future QuickChain Phase-1+ root-producing code in this crate: blocked until vectors
```

The missing 4–5% is not an immediate blocker. It represents future polish and later integration after adjacent crates are hardened.

Remaining non-blocking areas:

* deeper WAL crash durability tests;
* more ingest HTTP edge-case tests;
* more operator docs;
* more CI integration;
* future canonical accounting-root vectors after QuickChain gates permit root work.

The current crate has achieved the main goal for this phase:

```
ron-accounting can safely provide deterministic accounting/reward-planning artifacts
without becoming balance truth, payout authority, or QuickChain settlement authority.
```

---

## 11. Current safe parking summary

`ron-accounting` is safe to leave in this state.

When returning later, first run:

```
crates/ron-accounting/scripts/dev-quickchain-park.sh
```

If it passes, do not reopen architecture unless a specific future requirement exists.

If future work tries to add authority to this crate, stop and re-check:

* Does this belong in `svc-wallet`?
* Does this belong in `ron-ledger`?
* Does this belong in `svc-rewarder`?
* Does this require QuickChain locked vectors first?
* Is this root-producing code?
* Is this settlement/finality code?
* Is this raw engagement payout logic?
* Is this creating fake receipts or fake balances?

The default answer should be:

```
ron-accounting measures and snapshots usage.
It does not own economic truth.
```

---

## 12. One-paragraph handoff summary

`ron-accounting` is now parked for the QuickChain Phase-0/preflight pass. It has strict usage-event and ingest DTO boundaries, strict reward snapshot/projection DTOs, deterministic artifact CIDs, reward-planning snapshots, event-class/raw-engagement guardrails, ingest poisoning tests, snapshot non-authority tests, reward DTO strictness tests, reward projection boundary tests, event-class boundary tests, a crate-local QuickChain runbook, fixed crate-local workflow package names, and a final parking script that passed. It remains derivative metering infrastructure only: no balances, no receipts, no wallet mutation, no ledger mutation, no roots, no validators, no settlement, no anchors, no pruning, and no raw engagement protocol payout authority. The next correct crate is `svc-rewarder`, where the project must prove reward planning stays deterministic, funding-source-gated, raw-engagement-safe, replay-safe, and routed through `svc-wallet`/`ron-ledger` for actual economic truth.


### END NOTE - JUNE 14 2026 - 12:40 CST