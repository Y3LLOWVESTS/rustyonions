### BEGIN NOTE - APRIL 26 2026 - 14:00 CST

# svc-rewarder NOTES.MD — Gold Foundation / Carryover Notes

Date: 2026-04-27
Crate: `svc-rewarder`
Workspace path: `crates/svc-rewarder`
Project: RustyOnions / WEB3 / ROC internal value plane
Current status: **Gold foundation plateau reached**
Completion estimate: **75–80% complete**
Recommended next WEB3 crate/session focus: **`ron-accounting`**, then integration path into `svc-wallet` and paid storage/pinning.

---

# 0. Executive Summary

`svc-rewarder` has been taken from scaffold/stub state into a functioning deterministic reward computation service for the internal ROC economy.

The crate now has a working local/dev reward pipeline:

```text
inline accounting snapshot
→ canonical validation
→ policy validation
→ deterministic reward calculation
→ dust/residual handling
→ manifest commitment
→ settlement batch planning
→ wallet-compatible issue request preview
→ dry-run-safe production promotion
→ metrics/readiness/artifact behavior
```

The crate passed the full local quality gate:

```bash
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
cargo run -p svc-rewarder
```

Final observed test/running status:

```text
1 lib unit test passed
7 integration tests passed
33 unit tests passed
bench smoke passed
runtime launch passed
manual HTTP smoke passed
strict clippy passed
```

The service launched successfully:

```text
svc-rewarder listening local_addr=127.0.0.1:8090
```

Manual HTTP smoke also succeeded:

```text
/healthz  -> ok
/readyz   -> all readiness gates true
/version  -> svc-rewarder 0.1.0
/metrics  -> Prometheus output present
```

This is not yet fully production-complete because it still needs real inter-service adapters, but the deterministic core, HTTP shell, readiness, metrics, wallet-preview seam, and test foundation are strong.

---

# 1. Strategic Role in WEB3 / ROC

`svc-rewarder` belongs to the RustyOnions internal value plane.

Its role is **not** to be the durable economic truth engine. That belongs to `ron-ledger`.

Its role is **not** to be the wallet mutation front-door. That belongs to `svc-wallet`.

Its role is **not** to be the transient usage counter engine. That belongs to `ron-accounting`.

The correct boundaries are:

```text
ron-accounting:
  transient usage counters, time windows, sealed contribution snapshots

svc-rewarder:
  deterministic reward epochs, policy validation, reward manifests,
  settlement intent planning, wallet issue request shaping

svc-wallet:
  mutation front-door, auth/policy, idempotency, issue/transfer/burn,
  receipts, account balances

ron-ledger:
  durable append-only economic truth, replay, conservation, balances
```

The correct production flow should eventually be:

```text
usage events / counters
→ ron-accounting window
→ sealed accounting snapshot
→ svc-rewarder compute
→ wallet-compatible issue requests
→ svc-wallet /v1/issue
→ ron-ledger durable commit
```

The key boundary rule we preserved:

```text
svc-rewarder must not mutate ron-ledger directly in normal operation.
```

It computes and emits/shapes deterministic wallet intents. The wallet commits. The ledger records truth.

---

# 2. Design Principles Locked In

## 2.1 ROC-only

This crate currently works only on the internal ROC economy.

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

This is correct. External ROX integration remains deferred.

## 2.2 Determinism first

The same key tuple should produce the same reward run key:

```text
epoch_id
policy_hash
inputs_cid
idempotency_salt
```

The reward computation is deterministic, integer-only, and test-covered.

## 2.3 No floating point

All economic values are integer minor units.

DTO values for amounts are string-encoded at the JSON boundary where appropriate.

This matches the established wallet/ledger design rule: no floats in money-like math.

## 2.4 Rewarder does not own durable truth

Rewarder can write optional manifest artifacts when amnesia is disabled, but those artifacts are audit/convenience outputs, not a second ledger.

This distinction matters:

```text
manifest artifact != ledger
manifest artifact != wallet receipt
manifest artifact != source of account balance truth
```

## 2.5 Amnesia mode is respected

Default config uses amnesia mode:

```toml
[amnesia]
enabled = true
```

When amnesia is on, manifest artifacts are not written to disk. Tests prove this.

When amnesia is off, manifests can be written as JSON artifacts.

## 2.6 Dry-run promotion is supported

A dry-run compute does not consume the live settlement run key.

Expected behavior:

```text
dry_run=true  → ledger.result = "dry_run"
dry_run=false → ledger.result = "accepted"
same triple   → same run_key
commitment    → changes because ledger result is part of the manifest commitment
```

This allows operators to preview a reward epoch before promoting it to production.

---

# 3. What We Built — Batch by Batch

## 3.1 Batch 1 — Service foundation

Batch 1 created the functional service foundation.

Major pieces added:

```text
src/main.rs
src/lib.rs
src/prelude.rs
src/config/*
src/http/*
src/core/*
src/inputs/*
src/outputs/*
src/metrics/mod.rs
src/readiness/*
src/bus/*
src/security/*
src/util/*
tests/unit/*
tests/integration/*
benches/reward_calc.rs
configs/svc-rewarder.toml
```

Batch 1 established:

* Axum HTTP service shell
* `/healthz`
* `/readyz`
* `/metrics`
* `/version`
* `POST /rewarder/epochs/:epoch_id/compute`
* initial config/load/validate
* deterministic reward compute path
* run key generation
* manifest generation
* settlement intent stub
* readiness state
* metrics registry
* unit/integration tests
* runnable service on `127.0.0.1:8090`

First issues fixed:

* A `&str` vs `&String` cache key mismatch in `handlers.rs`
* A Clippy `derivable_impls` issue for `TlsConfig`

After fixes, Batch 1 passed build/test/run.

## 3.2 Batch 2 — Accounting, policy, and settlement seams

Batch 2 strengthened the value-plane correctness.

Added or improved:

```text
inputs/accounting.rs
inputs/policy.rs
outputs/intents.rs
metrics/mod.rs
http/handlers.rs
tests/unit/accounting_policy.rs
tests/unit/settlement.rs
```

Batch 2 added:

* accounting snapshot validation
* canonical account ordering
* whitespace normalization for accounts
* duplicate account rejection
* canonical snapshot CID helper
* policy resolver
* canonical `b3:<64 lowercase hex>` policy hash validation
* policy mismatch rejection
* uppercase hash rejection
* deterministic settlement batch planning
* deterministic per-recipient idempotency keys
* metric for planned settlement intents

This proved:

```text
accounting snapshot → policy validation → reward manifest → settlement batch
```

## 3.3 Batch 3 — Wallet-compatible settlement preview

Batch 3 added the first real wallet-facing DTO seam.

Added or improved:

```text
outputs/intents.rs
outputs/mod.rs
http/handlers.rs
http/routes.rs
tests/unit/settlement.rs
```

Batch 3 added:

* `WalletIssueRequest`
* `WalletIssueBatch`
* wallet issue path constant
* `SettlementIntent::to_wallet_issue_request`
* `SettlementBatch::to_wallet_issue_batch`
* `GET /rewarder/epochs/:epoch_id/settlement`
* wallet-safe idempotency key length
* string-encoded `amount_minor`
* unit tests for wallet DTO shape

Important design correction:

```text
ROC asset string is lowercase "roc"
```

This matches the wallet-side convention.

## 3.4 Batch 4 — Wallet client seam and integration proof

Batch 4 added a more formal wallet egress seam without performing network mutation.

Added:

```text
outputs/wallet.rs
```

Expanded:

```text
outputs/mod.rs
config/types.rs
config/validate.rs
http/handlers.rs
tests/unit/config.rs
tests/unit/wallet_client.rs
tests/integration/http_compute.rs
```

Batch 4 added:

* `WalletIssueClient` trait
* `DevWalletIssueClient`
* `WalletIssueOutcome`
* wallet base URL config
* wallet issue path config
* wallet capability scope config
* config validation for wallet URL/path/scope
* integration test for settlement preview endpoint
* integration test for dry-run → production promotion
* integration test for metrics text assertions
* unit tests for wallet client preview/emission/idempotency

This proved:

```text
rewarder can shape wallet-compatible issue batches
rewarder can preview settlement without side effects
dev wallet seam can classify accepted/dup/dry_run
dry-run does not consume production run key
```

## 3.5 Final batch — Gold polish

Final batch added the remaining local correctness proof surfaces.

Added:

```text
tests/unit/artifacts.rs
tests/unit/quarantine_edges.rs
tests/unit/config_file.rs
configs/svc-rewarder.toml
README.md
NOTES.MD
```

Final batch proved:

* artifact writing is suppressed in amnesia mode
* artifact writing persists manifest when amnesia is disabled
* artifact filenames are sanitized
* checked-in config fixture is valid
* partial config overlays defaults
* unknown config fields are rejected
* dust below `min_payout_minor_units` becomes residual
* zero-activity snapshots yield all residual
* arithmetic overflow quarantines before settlement planning

---

# 4. Current Source Structure

Current meaningful module roles:

```text
src/config/
  types.rs      — typed config model
  load.rs       — config file/env loading
  validate.rs   — fail-closed config validation
  mod.rs        — config facade

src/http/
  dto.rs        — request/response DTOs
  error.rs      — HTTP error mapping
  handlers.rs   — Axum handlers
  routes.rs     — router construction
  mod.rs        — HTTP state/facade

src/core/
  algebra.rs    — AmountMinor and checked math
  compute.rs    — deterministic reward calculation
  invariants.rs — conservation/invariant checks
  mod.rs        — core facade

src/inputs/
  accounting.rs       — accounting snapshot DTO/validation
  policy.rs           — reward policy DTO/validation
  cid.rs              — b3 CID parsing
  ledger_snapshot.rs  — read-only ledger snapshot seam
  cache.rs            — simple in-memory cache
  mod.rs              — input facade

src/outputs/
  manifest.rs     — reward manifest, totals, payouts, commitment
  intents.rs      — settlement intents and wallet issue batch DTOs
  wallet.rs       — wallet issue client trait/dev client
  artifacts.rs    — amnesia-aware artifact writer
  attestation.rs  — attestation DTO seam
  mod.rs          — output facade

src/metrics/
  mod.rs — Prometheus metrics registry

src/readiness/
  health.rs — readiness snapshot/gates
  mod.rs    — readiness facade

src/bus/
  events.rs — rewarder bus events
  mod.rs    — in-memory event sink

src/security/
  caps.rs — dev capability/scope checks
  tls.rs  — TLS seam
  pq.rs   — PQ posture seam
  mod.rs  — security facade

src/util/
  bytes.rs    — size parsing
  timeouts.rs — duration parsing
```

---

# 5. Current HTTP Surface

The service currently exposes:

```text
GET  /healthz
GET  /readyz
GET  /metrics
GET  /version

POST /rewarder/epochs/:epoch_id/compute
GET  /rewarder/epochs/:epoch_id
GET  /rewarder/epochs/:epoch_id/settlement
```

Current dev auth:

```text
Authorization: Bearer dev
```

This is intentionally temporary. Production capability/macaroons remain future work.

---

# 6. Current Config Surface

Checked-in fixture:

```text
crates/svc-rewarder/configs/svc-rewarder.toml
```

Important config fields:

```toml
bind_addr = "127.0.0.1:8090"
metrics_addr = "127.0.0.1:0"
max_conns = 1024
read_timeout = "5s"
write_timeout = "5s"
idle_timeout = "60s"

[tls]
enabled = false

[limits]
max_body_bytes = "1MiB"
decompress_ratio_cap = 10

[rewarder]
epoch_duration = "1h"
policy_id = "policy:v1"
inputs_cache_ttl = "5m"
max_epoch_skew = "2m"
idempotency_salt = "svc-rewarder|v1"
artifact_dir = "/var/run/svc-rewarder/artifacts"
retain_runs = "24h"
enable_zk_proofs = false

[ingress]
accounting_base_url = "http://127.0.0.1:7101"
wallet_base_url = "http://127.0.0.1:8088"
wallet_issue_path = "/v1/issue"
wallet_cap_scope = "wallet.issue.rewarder"
ledger_base_url = "http://127.0.0.1:7201"
policy_base_url = "http://127.0.0.1:7301"
macaroon_path = ""

[concurrency]
compute_workers = 4
io_inflight = 64
work_queue_max = 512

[shard]
strategy = "single"
shards = 1

[amnesia]
enabled = true

[pq]
mode = "off"

[log]
format = "text"
level = "info"
```

Config tests prove:

* default config is valid
* zero compute workers reject
* TLS enabled requires cert/key paths
* wallet base URL must be HTTP/HTTPS
* wallet issue path must start with `/`
* wallet cap scope must not be empty
* checked-in fixture is valid
* partial config overlays defaults
* unknown config keys reject

---

# 7. Important DTOs / Types

## 7.1 Accounting snapshot

```text
AccountingSnapshot
  produced_at_millis
  pool_minor_units
  contributions: Vec<AccountContribution>

AccountContribution
  account
  bytes_stored
  bytes_served
  uptime_seconds
```

Current deterministic score formula:

```text
score = bytes_stored + (bytes_served / 4) + uptime_seconds
```

This is intentionally simple and deterministic. It is not final anti-gaming policy.

## 7.2 Reward policy

```text
RewardPolicy
  id
  hash
  signed
  max_payout_minor_units
  min_payout_minor_units
  weight_bps
  rounding
```

Current supported rounding:

```text
floor
```

Current hash requirement:

```text
b3:<64 lowercase hex chars>
```

## 7.3 Reward manifest

The manifest includes:

```text
epoch_id
run_key
commitment
status
inputs_cid
policy summary
totals
payouts
invariants
ledger summary
attestation
```

Manifest commitment changes if ledger result changes. That is why dry-run and production manifests can share run key but have different commitments.

## 7.4 Settlement intent

```text
SettlementIntent
  run_key
  idempotency_key
  epoch_id
  manifest_commitment
  to
  asset = "roc"
  amount_minor_units
  memo
```

## 7.5 Wallet issue request

Wallet-compatible DTO shape:

```text
WalletIssueRequest
  to
  asset
  amount_minor
  idempotency_key
  memo
```

Important:

```text
amount_minor is serialized as a string
idempotency_key is <=64 bytes
asset is "roc"
```

## 7.6 Wallet issue batch

```text
WalletIssueBatch
  run_key
  epoch_id
  manifest_commitment
  wallet_path
  total_minor_units
  requests
```

---

# 8. Current Tests

## 8.1 Integration tests

Current integration tests: **7**

They cover:

```text
egress_dedupe::settlement_intent_egress_is_idempotent_by_run_key
readiness::readyz_degrades_when_queue_gate_false
readiness::readyz_is_ok_after_state_initialization
http_compute::compute_happy_path_and_replay_are_deterministic
http_compute::metrics_include_planned_settlement_intents_after_compute
http_compute::settlement_preview_endpoint_returns_wallet_issue_batch
http_compute::dry_run_can_promote_to_production_without_consuming_run_key
```

These prove:

* HTTP compute works
* replay is deterministic
* settlement preview endpoint works
* dry-run can promote to production
* readiness gates work
* metrics expose planned settlement counters
* egress dedupe is stable

## 8.2 Unit tests

Current unit tests: **33**

They cover:

```text
accounting canonicalization
duplicate account rejection
snapshot CID determinism
policy resolver acceptance/rejection
config defaults
TLS validation
wallet URL/path/scope validation
artifact amnesia suppression
artifact persistence
artifact filename sanitization
run key determinism
intent store idempotency
conservation
payout cannot exceed pool
overflow quarantine
dust/residual behavior
zero activity residual behavior
settlement batch total matching
settlement sorting
wallet issue DTO shape
wallet issue serialization
wallet client preview
wallet client idempotent emit
wallet client dry-run behavior
config fixture validity
unknown config key rejection
partial config defaults
```

## 8.3 Bench smoke

Current bench smoke:

```text
benches/reward_calc.rs
Testing reward_calc_100
Success
```

This is not a full performance benchmark yet, but it proves the bench target builds/runs.

---

# 9. Manual Smoke Commands

Run service:

```bash
cargo run -p svc-rewarder
```

Health smoke:

```bash
curl -s http://127.0.0.1:8090/healthz
curl -s http://127.0.0.1:8090/readyz
curl -s http://127.0.0.1:8090/version
curl -s http://127.0.0.1:8090/metrics | head
```

Example compute:

```bash
curl -s -X POST http://127.0.0.1:8090/rewarder/epochs/demo-epoch-1/compute \
  -H 'Authorization: Bearer dev' \
  -H 'Content-Type: application/json' \
  -d '{
    "inputs_cid":"b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "policy_id":"policy:v1",
    "policy_hash":"b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "dry_run":false,
    "snapshot":{
      "produced_at_millis":1,
      "pool_minor_units":"1000",
      "contributions":[
        {"account":"acct_a","bytes_stored":100,"bytes_served":50,"uptime_seconds":10},
        {"account":"acct_b","bytes_stored":200,"bytes_served":0,"uptime_seconds":20}
      ]
    },
    "policy":{
      "id":"policy:v1",
      "hash":"b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "signed":true,
      "max_payout_minor_units":"1000",
      "min_payout_minor_units":"1",
      "weight_bps":10000,
      "rounding":"floor"
    }
  }' | jq .
```

Fetch manifest:

```bash
curl -s http://127.0.0.1:8090/rewarder/epochs/demo-epoch-1 \
  -H 'Authorization: Bearer dev' | jq .
```

Preview settlement/wallet issue requests:

```bash
curl -s http://127.0.0.1:8090/rewarder/epochs/demo-epoch-1/settlement \
  -H 'Authorization: Bearer dev' | jq .
```

---

# 10. Current Metrics

Current Prometheus metrics include:

```text
svc_rewarder_readyz_degraded{cause="config_loaded"}
svc_rewarder_readyz_degraded{cause="ledger_ok"}
svc_rewarder_readyz_degraded{cause="policy_registry_ok"}
svc_rewarder_readyz_degraded{cause="queue_ok"}

svc_rewarder_reward_compute_latency_seconds
svc_rewarder_reward_runs_total
svc_rewarder_ledger_intents_total
svc_rewarder_settlement_intents_planned_total
svc_rewarder_rejected_total
```

Important metric meaning:

```text
reward_runs_total:
  count by final manifest status

ledger_intents_total:
  accepted / dup / dry_run / error egress classifications

settlement_intents_planned_total:
  total number of wallet issue request intents planned from manifests

readyz_degraded:
  current readiness missing/degraded cause gauges

rejected_total:
  bad_request / conflict / invariant / auth / busy / not_found / config / internal
```

No account IDs are placed into metric labels. This is correct.

---

# 11. Known Remaining Work

The crate is now strong locally, but not yet fully production complete.

## 11.1 Real `ron-accounting` adapter

Current input snapshots are inline/dev.

Needed:

```text
fetch sealed snapshot by inputs_cid
verify snapshot CID against actual canonical bytes
validate schema
handle unavailable accounting service
handle stale snapshot
handle oversized/decompression limits if fetched remotely
metrics:
  accounting_fetch_latency_seconds
  accounting_fetch_total{result}
  accounting_snapshot_reject_total{reason}
```

Important next-crate implication:

```text
ron-accounting should output exactly the kind of sealed snapshot svc-rewarder expects:
AccountingSnapshot {
  produced_at_millis,
  pool_minor_units,
  contributions: [
    account,
    bytes_stored,
    bytes_served,
    uptime_seconds
  ]
}
```

The next session should strongly consider making `ron-accounting` export a stable reward snapshot DTO or adapter that can feed `svc-rewarder`.

## 11.2 Real `svc-wallet` HTTP adapter

Current wallet client is a dev stub.

Needed:

```text
HTTP POST to wallet_base_url + wallet_issue_path
request body = WalletIssueRequest
authorization = attenuated rewarder wallet capability
idempotency key = per-recipient deterministic key
timeout
retry with jitter
classify wallet result:
  accepted
  dup
  rejected
  unavailable
  timeout
```

Important integration rule:

```text
Do not bypass svc-wallet to write directly to ron-ledger.
```

## 11.3 Production capability/macaroons

Current dev auth is:

```text
Authorization: Bearer dev
```

Needed scopes:

```text
rewarder.run
rewarder.inspect
wallet.issue.rewarder
```

Future caveats:

```text
tenant
method
path
epoch
ttl
max_amount
audience = svc-wallet
```

## 11.4 OpenAPI/schema finalization

Docs mention machine-readable HTTP API schemas, but they are not fully finalized.

Needed:

```text
docs/openapi/svc-rewarder.json
docs/schemas/compute.request.v1.json
docs/schemas/manifest.v1.json
docs/schemas/settlement.preview.v1.json
contract tests for schemas
```

## 11.5 Real policy registry integration

Current policy can be inline/default.

Needed:

```text
fetch signed policy by policy_id
verify policy_hash
verify signature
cache policy
reject stale/revoked policy
metrics for policy fetch/verify
```

## 11.6 Stronger anti-gaming rules

Current reward formula is intentionally simple:

```text
score = bytes_stored + bytes_served/4 + uptime_seconds
```

Future reward policy should likely include:

```text
per-account caps
per-node caps
per-tenant caps
minimum uptime thresholds
storage proof weighting
served-byte weighting
request quality weighting
geographic/regional balancing
Sybil-resistant identity hooks
quarantine suspicious concentration
rate-of-change caps
signed accounting windows
```

## 11.7 Durable manifest indexing decision

Current artifact writer is optional and not a database.

Need decide:

```text
A) keep rewarder stateless and rely on wallet/ledger truth
B) add optional manifest index for operator UX
C) store manifests only as external artifacts
```

Do not accidentally create a second ledger.

## 11.8 End-to-end ROC loop test

Eventually needed:

```text
usage recorded
→ accounting window sealed
→ rewarder computes
→ wallet issues ROC
→ ledger balance increases
→ replay does not double issue
```

This should become the WEB3 value-plane integration proof.

---

# 12. Recommended Next WEB3 Crate

The next best WEB3 crate to work on is likely:

```text
ron-accounting
```

Reason:

`svc-rewarder` is now ready to consume real sealed accounting snapshots, but currently uses inline dev snapshots. The biggest missing upstream dependency is a real accounting snapshot producer.

The next session should probably start by attaching/reviewing:

```text
ron-accounting CODEBUNDLE
ron-accounting ALL_DOCS / NOTES
svc-rewarder NOTES.MD
svc-wallet NOTES.MD
ron-ledger NOTES.MD
WEB3.MD
```

The goal for `ron-accounting` should be:

```text
turn raw usage events/counters into deterministic sealed snapshots that svc-rewarder can consume
```

Recommended `ron-accounting` batch plan:

## Batch A — Snapshot export DTO

Build/verify a reward snapshot export surface matching rewarder needs:

```text
produced_at_millis
pool_minor_units
contributions:
  account
  bytes_stored
  bytes_served
  uptime_seconds
```

Potential endpoint or function:

```text
GET /accounting/windows/:window_id/reward-snapshot
```

or library export:

```rust
RewardSnapshotExport
```

Important: use integer counters only.

## Batch B — Canonicalization and hashing

Add canonical JSON/bytes hashing:

```text
snapshot_cid = b3:<hash(canonical_snapshot_bytes)>
```

This should match `svc-rewarder::inputs::canonical_snapshot_cid`.

Potential future interop test:

```text
ron-accounting generated snapshot CID == svc-rewarder canonical_snapshot_cid
```

## Batch C — Window sealing

Accounting should distinguish:

```text
open/current counters
sealed reward snapshot
replayed/exported snapshot
```

Once a snapshot is sealed, it should be deterministic and immutable for that window.

## Batch D — Integration vector

Create a test vector shared with rewarder:

```text
accounting fixture snapshot JSON
expected b3 CID
expected rewarder payout totals
expected settlement preview shape
```

## Batch E — Export/error taxonomy

Add errors for:

```text
window not found
window not sealed
snapshot too large
counter overflow
invalid account id
stale window
```

## Batch F — Metrics

Add accounting metrics useful to rewarder:

```text
accounting_windows_sealed_total
accounting_snapshot_exports_total{result}
accounting_snapshot_bytes
accounting_counter_overflow_total
```

---

# 13. How svc-rewarder Should Be Used by the Next Crate

When building `ron-accounting`, treat `svc-rewarder` as the consumer contract.

The accounting crate should be able to produce this JSON shape:

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

Rewarder currently requires:

```text
account names non-empty
account names canonicalized/trimmed
no duplicate accounts
all counters unsigned
score arithmetic must not overflow
pool_minor_units must be valid AmountMinor
```

The eventual `inputs_cid` should refer to the canonical sealed accounting snapshot.

Current temporary behavior:

```text
inputs_cid is syntactically validated but not yet enforced against snapshot bytes
```

Future behavior should be:

```text
inputs_cid must equal canonical_snapshot_cid(snapshot)
```

This is probably one of the highest-value integration tasks after `ron-accounting` snapshot export is stable.

---

# 14. How svc-wallet Should Be Used Later

`svc-rewarder` now produces wallet-compatible issue requests.

Shape:

```json
{
  "to": "acct_a",
  "asset": "roc",
  "amount_minor": "123",
  "idempotency_key": "b3:...",
  "memo": "svc-rewarder:epoch-1:acct_a"
}
```

Current preview endpoint:

```text
GET /rewarder/epochs/:epoch_id/settlement
```

returns:

```json
{
  "run_key": "...",
  "epoch_id": "...",
  "manifest_commitment": "...",
  "wallet_path": "/v1/issue",
  "total_minor_units": "...",
  "requests": [...]
}
```

When wiring real wallet mutation:

```text
for each request:
  POST {wallet_base_url}{wallet_issue_path}
  Authorization: Bearer <rewarder wallet capability>
  Idempotency-Key: <request.idempotency_key>  // if wallet expects header
  Body: WalletIssueRequest
```

Need verify exact `svc-wallet` DTO/header expectations before wiring.

Critical invariant:

```text
replaying the same reward epoch must not issue twice
```

This should be protected by:

```text
rewarder run_key
per-recipient idempotency_key
svc-wallet idempotency store
ron-ledger idempotency/conservation
```

---

# 15. Completion Estimate

Current truthful status:

```text
Core deterministic compute: strong
Accounting input validation: good
Policy validation: good
Manifest commitments: good
Settlement planning: good
Wallet DTO preview: good
Dry-run promotion: good
Readiness/metrics: good
Artifact/amnesia behavior: good
Local test coverage: strong
Runtime smoke: good

Real accounting adapter: pending
Real wallet HTTP adapter: pending
Production capabilities: pending
OpenAPI/schema sync: pending
Anti-gaming policy depth: pending
End-to-end ROC loop: pending
```

Overall:

```text
75–80% complete
```

This is a strong Gold foundation, but not the final production service.

---

# 16. Recommended Commit Message

```text
svc-rewarder: implement deterministic ROC reward foundation
```

Suggested commit body:

```text
- Add svc-rewarder config, validation, and example config fixture
- Add deterministic integer-only reward compute path
- Add accounting snapshot validation and canonical snapshot hashing helper
- Add reward policy validation and canonical policy hash checks
- Add deterministic run_key and manifest commitment hashing
- Add settlement intent planning and wallet-compatible issue request previews
- Add dev wallet issue client seam with dry-run-safe idempotent behavior
- Add Axum routes for health, readiness, metrics, version, compute, manifest, and settlement preview
- Add Prometheus metrics for runs, rejects, planned settlement intents, and wallet/ledger egress results
- Add amnesia-aware artifact writer
- Add tests for conservation, residual/dust behavior, overflow quarantine, idempotency, config, config fixture, artifacts, readiness, HTTP compute, settlement preview, dry-run promotion, and wallet DTO shape
- Add README and NOTES carryover
```

---

# 17. Next Session Starting Prompt

Use this in the next session:

```text
We finished svc-rewarder to a Gold foundation plateau. It now passes fmt, clippy -D warnings, tests, bench smoke, runtime launch, and manual HTTP health/ready/version/metrics smoke. It has deterministic reward computation, accounting snapshot validation, policy validation, manifest commitments, settlement batch planning, wallet-compatible issue request previews, dry-run-safe production promotion, amnesia-aware artifacts, and 7 integration + 33 unit tests passing.

The next WEB3 crate should likely be ron-accounting. The goal is to make ron-accounting export deterministic sealed reward snapshots that svc-rewarder can consume. The snapshot shape should include produced_at_millis, pool_minor_units, and per-account contributions with account, bytes_stored, bytes_served, and uptime_seconds. We need canonical snapshot hashing so ron-accounting’s snapshot CID matches svc-rewarder’s canonical_snapshot_cid helper. After that, rewarder can enforce inputs_cid == canonical snapshot CID and eventually fetch snapshots from ron-accounting instead of inline request bodies.
```

---

# 18. Final Current State

`svc-rewarder` is now a credible deterministic reward engine for the ROC internal economy.

It proves the reward half of the value plane locally:

```text
measurements can become rewards
rewards can become manifests
manifests can become wallet issue requests
dry runs can be promoted safely
duplicates do not double-emit
amnesia mode is respected
metrics/readiness tell the truth
```

The next big project step is to supply it with real sealed accounting data from `ron-accounting`, then wire its wallet issue batch into `svc-wallet` for a true end-to-end ROC loop.

### END NOTE - APRIL 27 2026 - 14:00 CST


### BEGIN NOTE - APRIL 27 2026 - 19:55 CST


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

### END NOTE - APRIL 27 2026 - 19:55 CST