
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