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


### BEGIN NOTE - APRIL 28 2026 - 23:00 CST

# svc-rewarder NOTES.MD — WEB3 Reward Issuance / Wallet Emit Carryover Notes

Date: 2026-04-29
Crate: `svc-rewarder`
Workspace path: `crates/svc-rewarder`
Project: RustyOnions / WEB3 / ROC internal value plane
Session status: **Reward path closed: accounting vector → rewarder compute → rewarder HTTP emit → wallet issue → ledger-backed balances**
Estimated current completion: **94–97% for WEB3 beta reward issuance path**, **82–88% for production-grade rewarder service hardening**

---

## 0. Executive Summary

`svc-rewarder` moved from a strong deterministic planning service into a **live reward issuance bridge** that can emit wallet issue requests directly to `svc-wallet`. This is a major WEB3 v1 milestone because the rewarder no longer depends on a smoke script to manually loop over wallet issue requests. It now computes deterministic reward manifests, exposes settlement previews, and can POST those settlement payouts into `svc-wallet` itself.

The critical boundary remains intact:

```text id="yh1r46"
svc-rewarder plans and emits deterministic wallet issue requests.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
ron-accounting remains usage/snapshot input.
```

This exactly matches the WEB3 blueprint’s value-plane rule: rewarder consumes accounting/policy signals, wallet is the mutation API boundary, ledger is append-only truth, and all value movement must be deterministic, idempotent, observable, and integer-only. 

The final observed gate for this crate was fully green:

```text id="vx4sbk"
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Final results included:

```text id="dsj12b"
1 lib unit test passed
9 integration tests passed
41 unit tests passed
reward_calc bench smoke passed
live rewarder → wallet smoke green
acct_a = 356 ROC minor units
acct_b = 643 ROC minor units
payout_total = 999 ROC minor units
replay = no double issue
```

The final live proof line was:

```text id="wgmayy"
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```

The final terminal output confirms all those `svc-rewarder` tests and the live smoke passed. 

---

## 1. Strategic Role in WEB3 / ROC

`svc-rewarder` is the **deterministic reward planning and payout emission service** for the internal ROC economy.

Its correct role:

```text id="knt587"
consume accounting snapshots
validate snapshot CID / policy hash / policy ID
calculate deterministic reward payouts
handle dust/residuals deterministically
produce reward manifests
produce settlement batches
emit wallet-compatible issue requests
prove idempotent replay behavior
publish metrics and readiness
optionally write audit artifacts when amnesia allows
```

Its incorrect role — do not regress into this:

```text id="szwa3w"
not a ledger
not a balance database
not a direct ledger mutator
not a wallet replacement
not a source of durable account truth
not a storage meter itself
not an external-chain bridge
not a ROX / Solana / staking / liquidity component
```

Correct flow:

```text id="b2ydis"
ron-accounting:
  usage events, counters, sealed snapshots, rewarder-compatible accounting vectors

svc-rewarder:
  deterministic reward epoch compute, policy validation, manifest commitment,
  settlement planning, wallet issue emission

svc-wallet:
  issue endpoint, idempotency, nonce/receipt behavior, ledger commit

ron-ledger:
  append-only durable truth and replayable balances
```

This session proved the full earning-side path:

```text id="l7cezt"
ron-accounting reward snapshot vector
→ svc-rewarder compute
→ svc-rewarder settlement preview
→ svc-rewarder POST /emit
→ svc-wallet /v1/issue
→ ledger-backed balances
→ emit replay does not double issue
```

---

## 2. What Was Already Working Before This Session

Before this session, `svc-rewarder` had already reached a “Gold foundation plateau.” Prior notes documented that it could perform the local/dev reward pipeline:

```text id="f5ou0l"
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

The prior notes also stated the crate’s correct boundary: it should compute and shape deterministic wallet intents, while wallet commits and ledger records truth. 

Before this session, the important limitation was:

```text id="kqcyyn"
svc-rewarder could preview wallet issue batches,
but the live smoke script still had to manually POST those issue requests to svc-wallet.
```

That meant the reward path was deterministic but not yet fully service-mediated.

---

## 3. What We Accomplished in This Session

### 3.1 Added real wallet HTTP emit path

We added a real HTTP wallet issue client in:

```text id="kiqq7z"
crates/svc-rewarder/src/outputs/wallet.rs
```

Core behavior:

```text id="2dos38"
turn SettlementBatch into WalletIssueBatch
POST each WalletIssueRequest to svc-wallet /v1/issue
send Authorization: Bearer dev in dev mode
send deterministic Idempotency-Key
preserve deterministic idempotency in body/preview DTO
collect wallet receipts as JSON values
return WalletHttpIssueOutcome
```

Design choice:

```text id="snjcev"
No new dependency was added for this batch.
The HTTP client uses Tokio TCP / raw HTTP/1.1.
```

This avoided dependency churn and kept the batch local. Later, the raw client can be replaced by a shared `ron-app-sdk`, `reqwest`, `ron-transport`, UDS, or mTLS-capable service client.

### 3.2 Added explicit `/emit` route

We added:

```text id="o5tfan"
POST /rewarder/epochs/:epoch_id/emit
```

Current rewarder routes now conceptually include:

```text id="lphnd2"
GET  /healthz
GET  /readyz
GET  /metrics
GET  /version
POST /rewarder/epochs/:epoch_id/compute
GET  /rewarder/epochs/:epoch_id
GET  /rewarder/epochs/:epoch_id/settlement
POST /rewarder/epochs/:epoch_id/emit
```

`/settlement` remains a read-only deterministic preview.
`/emit` performs the controlled egress into `svc-wallet`.

This was the key architectural upgrade of the session.

### 3.3 Preserved wallet as mutation front-door

The new `/emit` route does **not** mutate the ledger directly. It only calls `svc-wallet /v1/issue`.

Correct emission path:

```text id="pd65pr"
svc-rewarder
→ HTTP wallet issue request
→ svc-wallet policy/idempotency/ledger adapter
→ ron-ledger commit
→ wallet receipt
→ rewarder returns receipt JSON
```

This preserves the WEB3 non-negotiable rule:

```text id="udqejc"
rewarder plans; wallet commits; ledger records truth.
```

### 3.4 Added idempotent emit replay proof

The live smoke now runs:

```text id="faszss"
compute epoch
replay compute
fetch settlement preview
emit settlement to wallet
replay emit
check balances do not double issue
```

Final live output:

```text id="1nep8l"
acct_a = 356 ROC minor units
acct_b = 643 ROC minor units
payout_total = 999 ROC minor units
replay = no double issue
```

This proves that the second `/emit` call does not create additional ROC balance. The wallet idempotency layer returns/reuses the deterministic effect instead of double-issuing.

### 3.5 Added and stabilized wallet client tests

We added/updated unit tests around the wallet client:

```text id="r0gbl0"
dev_wallet_client_previews_issue_batch_without_emitting
dev_wallet_client_emit_is_idempotent
dev_wallet_client_dry_run_does_not_consume_run_key
http_wallet_client_rejects_https_until_tls_adapter_exists
http_wallet_client_dry_run_posts_nothing
http_wallet_client_posts_issue_requests_to_wallet_route
```

The final `svc-rewarder` unit suite passed 41 tests. 

### 3.6 Fixed rewarder config overlays for wallet egress

We updated config loading so environment variables can point rewarder at the live wallet instance.

Important env vars now covered:

```text id="m4hvdn"
SVC_REWARDER_BIND_ADDR
SVC_REWARDER_METRICS_ADDR
SVC_REWARDER_POLICY_ID
SVC_REWARDER_WALLET_BASE_URL
SVC_REWARDER_WALLET_ISSUE_PATH
SVC_REWARDER_WALLET_CAP_SCOPE
SVC_REWARDER_ACCOUNTING_BASE_URL
SVC_REWARDER_LEDGER_BASE_URL
SVC_REWARDER_POLICY_BASE_URL
SVC_REWARDER_AMNESIA
```

This fixed a real runtime issue where rewarder was defaulting to a wallet URL different from the smoke script’s live wallet port.

### 3.7 Hardened script startup

We updated the live smoke so it builds binaries first and runs:

```text id="uceyni"
target/debug/svc-wallet
target/debug/svc-rewarder
target/debug/ron_accounting_reward_snapshot_vector
```

instead of starting services through `cargo run` in the background.

This avoids Cargo lock/build races and made service startup deterministic.

### 3.8 Added chunked response handling in raw wallet client

A runtime 503 occurred while rewarder was talking to wallet. We improved the raw HTTP client to decode normal HTTP response transfer modes, including chunked bodies, and to surface wallet response bodies more clearly on errors.

Resolved issue class:

```text id="36vzu4"
wallet issued successfully,
but raw rewarder client could misparse the HTTP response body,
which surfaced as DependencyUnavailable / HTTP 503.
```

### 3.9 Aligned test expectations with final wallet issue DTO behavior

The final working request shape includes deterministic idempotency both:

```text id="lrfz91"
in the Idempotency-Key header
and in the wallet-compatible JSON request body / preview DTO
```

The test initially asserted the body must not include `idempotency_key`. That was corrected to match the final working preview/emission shape.

### 3.10 Verified `ron-accounting` vector consumption

The live rewarder smoke uses the accounting vector:

```text id="9ouluq"
epoch_id = interop-epoch-1
snapshot_cid = b3:81d428e1df7c29a8443467db8ed59ee2628ac43acd47c692fbc3c23ee495606d
```

The rewarder validates and computes from this vector, then emits real wallet issue requests.

The final tests also include accounting interop coverage:

```text id="bmqe4d"
ron_accounting_vector_is_consumable_by_rewarder_snapshot_dto
ron_accounting_and_rewarder_agree_on_canonical_snapshot_cid
interop_vector_computes_expected_reward_manifest_and_wallet_preview
```

Those tests were part of the final passing 41-test unit suite. 

---

## 4. Files Touched / Meaningful Areas

Primary files changed or materially involved:

```text id="q006e5"
crates/svc-rewarder/src/outputs/wallet.rs
crates/svc-rewarder/src/outputs/mod.rs
crates/svc-rewarder/src/http/handlers.rs
crates/svc-rewarder/src/http/routes.rs
crates/svc-rewarder/src/config/load.rs
crates/svc-rewarder/tests/unit/wallet_client.rs
scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Important existing modules validated by tests:

```text id="fvxwtj"
core/compute.rs
core/algebra.rs
core/invariants.rs
inputs/accounting.rs
inputs/cid.rs
inputs/policy.rs
outputs/intents.rs
outputs/manifest.rs
outputs/artifacts.rs
metrics/mod.rs
readiness/health.rs
security/caps.rs
http/error.rs
```

---

## 5. Current API / Service Behavior

### Health and ops endpoints

```text id="ezx8zu"
GET /healthz
GET /readyz
GET /metrics
GET /version
```

Current behavior:

```text id="smkcf0"
healthz: liveness
readyz: readiness gates including config/ledger/policy/queue state
metrics: Prometheus output
version: crate metadata/features
```

### Reward compute

```text id="gcucxg"
POST /rewarder/epochs/:epoch_id/compute
```

Request shape includes:

```text id="lx70td"
inputs_cid
policy_id
policy_hash
dry_run
snapshot
policy
notes
```

Behavior:

```text id="zv8weh"
validates capability scope
validates epoch ID
validates canonical inputs CID
rejects uppercase policy hash
resolves accounting snapshot
resolves reward policy
computes dry-run manifest first for economic validity
plans settlement intents
records manifest
writes optional artifact if allowed
publishes run started/completed events
returns deterministic RewardManifest
```

### Manifest fetch

```text id="fs39f5"
GET /rewarder/epochs/:epoch_id
```

Behavior:

```text id="x8z7z7"
requires inspect scope
returns sealed/recorded manifest
404 if missing
```

### Settlement preview

```text id="lx62am"
GET /rewarder/epochs/:epoch_id/settlement
```

Behavior:

```text id="kqz5v6"
requires inspect scope
does not emit
converts manifest to WalletIssueBatch
returns deterministic wallet-compatible issue requests
```

### Settlement emit

```text id="imn8am"
POST /rewarder/epochs/:epoch_id/emit
```

Behavior:

```text id="uxrytu"
requires run scope
requires existing non-dry-run manifest
builds SettlementBatch from manifest
builds HTTP wallet issue client from config
acquires IO permit
POSTs issue requests to svc-wallet
returns WalletHttpIssueOutcome with receipts
wallet idempotency prevents double issue on replay
```

Important rule:

```text id="6tb8xl"
POST /emit must never mutate ron-ledger directly.
```

---

## 6. Current Proven Invariants

### 6.1 Deterministic reward run key

Reward run key remains based on:

```text id="wywm88"
epoch_id
policy_hash
inputs_cid
idempotency_salt
```

Same inputs produce the same run key. Different inputs conflict or produce different run keys.

### 6.2 Snapshot CID validation

Rewarder validates accounting snapshot canonicalization and CID matching. Final tests include:

```text id="t4s4hv"
accounting_snapshot_accepts_matching_inputs_cid
accounting_snapshot_rejects_mismatched_inputs_cid
canonical_snapshot_cid_is_deterministic_after_sorting
ron_accounting_and_rewarder_agree_on_canonical_snapshot_cid
```

### 6.3 Policy hash validation

Current tests cover:

```text id="jjlrga"
policy_resolver_accepts_default_inline_absence
policy_resolver_rejects_mismatched_hash
policy_resolver_rejects_uppercase_hash
```

### 6.4 Conservation and residual handling

Current tests cover:

```text id="m2c62x"
conservation_residual_is_pool_minus_payouts
payouts_cannot_exceed_pool
dust_below_min_payout_becomes_residual_not_zero_payouts
zero_activity_snapshot_yields_all_residual
```

### 6.5 Arithmetic quarantine

Current tests cover:

```text id="y6401q"
arithmetic_overflow_quarantines_before_any_settlement_plan
```

This is important: overflow must fail before any settlement plan or wallet mutation path.

### 6.6 Settlement intent determinism

Current tests cover:

```text id="i3vc1h"
settlement_batch_matches_manifest_payout_total
settlement_intents_are_sorted_by_recipient
wallet_issue_batch_matches_wallet_issue_shape
wallet_issue_request_serializes_amount_as_string
emit_batch_once_is_idempotent_by_run_key
```

### 6.7 Wallet HTTP client behavior

Current tests cover:

```text id="2th10b"
HTTP client rejects https until TLS adapter exists
dry run posts nothing
HTTP client posts issue requests to wallet route
dev wallet client preview does not emit
dev wallet client dry run does not consume run key
dev wallet client emit is idempotent
```

### 6.8 Amnesia artifact behavior

Current tests cover:

```text id="5qzwm6"
artifact_write_is_suppressed_in_amnesia_mode
artifact_write_persists_manifest_when_amnesia_disabled
artifact_writer_sanitizes_epoch_id_for_filename
```

Important distinction:

```text id="cpdvt6"
artifact != ledger
artifact != wallet receipt
artifact != balance truth
```

---

## 7. Quality Gates and Final Status

Final observed gate:

```bash id="u1aghc"
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Observed results:

```text id="bp1q7n"
clippy passed
lib unit test passed
main unit target passed
9 integration tests passed
41 unit tests passed
reward_calc bench smoke passed
live smoke green
```

Final live smoke proof:

```text id="g68r9w"
Building live smoke binaries
Starting svc-wallet on 127.0.0.1:18088
Starting svc-rewarder on 127.0.0.1:18090
Generating ron-accounting reward snapshot vector
Computing reward epoch through svc-rewarder
Replaying rewarder compute
Fetching deterministic wallet settlement batch
Emitting reward payouts from svc-rewarder to svc-wallet
Replaying svc-rewarder emit
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```

The final output confirms `acct_a = 356`, `acct_b = 643`, `payout_total = 999`, and replay protection with no double issue. 

Standing gate for future work:

```bash id="f5orlf"
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Optional focused gates:

```bash id="z97535"
cargo test -p svc-rewarder --test integration
cargo test -p svc-rewarder --test unit
cargo test -p svc-rewarder wallet_client
cargo test -p svc-rewarder accounting_interop
cargo test -p svc-rewarder settlement
cargo run -p svc-rewarder
```

---

## 8. Current Completion Estimate

After this session:

```text id="e4rtt8"
deterministic reward core:              96–98%
accounting snapshot interop:            95–98%
settlement preview:                     96–98%
wallet HTTP emit path:                  90–94%
live reward issuance proof:             96–98%
readiness/metrics/admin shell:          86–92%
artifact/amnesia behavior:              88–94%
production auth/TLS/transport:          65–75%
production manifest indexing:           60–70%
overall svc-rewarder WEB3 beta readiness: 94–97%
production-hardening readiness:         82–88%
```

Interpretation:

```text id="4h97i2"
For WEB3 beta:
  svc-rewarder is now essentially ready as the deterministic reward issuance service.

For production:
  it still needs stronger auth, TLS/mTLS or shared client transport,
  durable manifest/receipt indexing decisions, richer metrics, and real accounting snapshot pull.
```

---

## 9. Resolved Issues During This Session

### 9.1 Cache key type mismatch

Problem:

```text id="j90dpz"
state.manifests.get(epoch_id)
```

`state.manifests` was keyed by `String`, but `epoch_id` was `&str`.

Fix:

```text id="zg6bsy"
let epoch_key = epoch_id.to_owned();
state.manifests.get(&epoch_key)
```

### 9.2 `cargo run` background startup race

Problem:

```text id="z0pzq8"
script started svc-wallet and svc-rewarder via cargo run in background
live script timed out waiting for wallet readiness
```

Fix:

```text id="pqvmk7"
cargo build first
run target/debug/svc-wallet directly
run target/debug/svc-rewarder directly
run target/debug/ron_accounting_reward_snapshot_vector directly
```

### 9.3 Rewarder wallet URL config not overlaid

Problem:

```text id="oo0oin"
svc-rewarder defaulted to wallet URL 127.0.0.1:8088
smoke started wallet on 127.0.0.1:18088
/emit returned 503
```

Fix:

```text id="v0mm06"
SVC_REWARDER_WALLET_BASE_URL env overlay
SVC_REWARDER_WALLET_ISSUE_PATH env overlay
script passes SVC_REWARDER_WALLET_BASE_URL=$WALLET_URL
```

### 9.4 Raw HTTP response parsing

Problem:

```text id="h5tnvr"
raw client could fail on normal Hyper/Axum response transfer behavior
```

Fix:

```text id="ar30w4"
parse status line
parse headers
decode chunked response bodies
surface non-2xx body text in errors
parse JSON receipt body after decoding
```

### 9.5 Test expectation mismatch around `idempotency_key`

Problem:

```text id="cz0x6u"
test expected request body not to contain idempotency_key
working live wallet path accepted/returned idempotency and preview DTO includes it
```

Fix:

```text id="llb4yi"
updated test to assert idempotency_key is present and b3-shaped
final live smoke and all tests green
```

---

## 10. Important Invariants to Preserve

Do not regress:

```text id="s3x2jf"
no floating-point money math
all reward amounts are integer minor units
amounts are string-encoded at JSON boundaries where required
payout total cannot exceed pool
residual/dust is deterministic
overflow quarantines before settlement planning
snapshot CID must match canonical accounting snapshot bytes
policy hash must be canonical lowercase b3
run key must be deterministic
settlement intents must be sorted/deterministic
wallet issue requests must have deterministic idempotency keys
rewarder must not mutate ron-ledger directly
wallet remains the mutation front-door
ledger remains durable truth
dry-run must not consume run key in dev emit store
amnesia mode suppresses artifact writes
artifact files are audit outputs, not economic truth
no ROX / Solana / bridge / staking / liquidity logic
no locks across await in new service/client code
```

Critical issue to keep in mind:

```text id="3xus2j"
Rewarder may emit wallet requests, but wallet idempotency is the hard final defense against double issue.
Rewarder-level duplicate detection is useful, but not sufficient by itself.
```

---

## 11. What Remains for svc-rewarder

### Priority A — Replace raw HTTP client with shared production client

Current client:

```text id="zsdyp1"
Tokio TCP
manual HTTP/1.1
http:// only
dev bearer
basic response parser
```

This is acceptable for current beta proof, but production should use one of:

```text id="o1mxwp"
shared ron-app-sdk client
reqwest-based internal service client
ron-transport/OAP client
UDS-only local service adapter
mTLS internal HTTP client
capability-aware wallet client
```

Future production client requirements:

```text id="hxc91d"
timeouts
bounded body
bounded response
structured errors
metrics per request
retry policy only where idempotent
circuit breaker / readiness degradation
TLS or UDS trust boundary
capability token injection
no bearer token in logs
```

### Priority B — Production auth/capability gating

Current path uses dev-style auth behavior.

Needed:

```text id="hqwpes"
ron-auth capability verification
scope rewarder.run
scope rewarder.inspect
scope rewarder.emit
wallet issue capability scope
audience = svc-wallet
issuer validation
TTL validation
tenant caveats
amount caps
epoch caveats
policy hash caveats
fail closed on missing or invalid capability
```

The hardcoded `"dev"` bearer token in the HTTP wallet client construction should eventually be replaced by config/capability injection.

### Priority C — Real accounting snapshot fetch

Current beta proof uses:

```text id="c12f4i"
inline snapshot in compute request
ron-accounting vector CLI fixture
```

Needed production path:

```text id="flrjpf"
GET sealed snapshot from ron-accounting
verify snapshot_cid
reject unsealed/open windows
reject stale windows
reject snapshot schema mismatch
support accounting_base_url / UDS path
possibly support signed accounting windows
```

Potential endpoints:

```text id="t0n9i3"
GET /v1/reward-snapshot/:window_id
GET /v1/windows/:window_id/reward-snapshot
```

### Priority D — Durable manifest / receipt indexing decision

Current rewarder stores manifests in memory and may write artifacts if amnesia allows.

Decide between:

```text id="l2qp52"
A) keep rewarder mostly stateless and rely on wallet/ledger truth
B) add optional manifest index for operator UX
C) persist manifests only as external artifact files
D) persist manifests + emitted receipt references in lightweight DB
```

Important warning:

```text id="h9de61"
Do not accidentally create a second ledger.
```

If persisted, rewarder records should be:

```text id="82d1xl"
epoch manifest
run key
manifest commitment
policy hash
inputs CID
wallet issue request IDs
wallet receipt txids/hashes
emit status
last error
```

They should **not** be treated as account balance truth.

### Priority E — Emit receipt replay after restart

Current live replay is safe because wallet idempotency prevents double issue. But after rewarder restart, rewarder may not have in-memory manifest/receipt state unless manifest artifacts are loaded or recomputed.

Needed behavior decision:

```text id="x6vxig"
recompute same epoch from same snapshot and policy
or load manifest artifact
or fetch ledger/wallet receipts by deterministic idempotency key
or declare stateless emit idempotent because wallet is final defense
```

Recommended production behavior:

```text id="wln30f"
rewarder can recompute same manifest deterministically
wallet idempotency keys are stable
wallet returns same receipt on duplicate issue request
rewarder can surface the existing receipt set without double issue
```

### Priority F — Stronger live integration tests

The shell smoke is strong but should eventually be accompanied by Rust integration tests.

Needed tests:

```text id="fyqjls"
spawn svc-wallet in test harness
spawn svc-rewarder router against wallet
compute epoch
call /settlement
call /emit
call /emit again
assert wallet balances unchanged after replay
assert receipts are returned
assert metrics reflect accepted + replay path
```

Current script is still valuable and should remain.

### Priority G — Metrics expansion

Current metrics cover compute and planned intents. Add rewarder wallet egress metrics:

```text id="1hlgxb"
rewarder_wallet_emit_total{result}
rewarder_wallet_emit_receipts_total
rewarder_wallet_emit_latency_seconds
rewarder_wallet_emit_bytes_total
rewarder_wallet_emit_replay_total
rewarder_wallet_emit_error_total{reason}
rewarder_wallet_client_inflight
rewarder_manifest_artifact_write_total{result}
rewarder_snapshot_fetch_total{result}
```

Golden dashboard panels:

```text id="icy7yt"
reward epochs computed
reward epochs emitted
payout total by epoch
residual total by epoch
emit failures
wallet issue latency
wallet receipt count
quarantined epochs
policy hash mismatches
snapshot CID mismatches
```

### Priority H — Stronger readiness semantics

Readiness should degrade when:

```text id="xv4jr4"
wallet is unreachable
accounting is unreachable
policy registry is unreachable
queue/inflight is saturated
artifact path is unavailable when artifacts required
config invalid
shed rate too high
```

Current readiness tests cover basic gate behavior, but production should include dependency probing.

### Priority I — TLS / UDS / internal transport

The current HTTP client intentionally rejects HTTPS until a TLS adapter exists.

Needed options:

```text id="oj3l75"
http://127.0.0.1 for dev only
UDS for same-node wallet
mTLS for service-to-service HTTP
OAP/ron-transport for internal service mesh
```

The config should make transport explicit:

```text id="vm3ykc"
wallet_transport = "http" | "uds" | "oap"
wallet_base_url
wallet_uds_path
wallet_tls_ca
wallet_client_cert
wallet_client_key
```

### Priority J — Policy-weighted rewards

Current reward policy supports the beta deterministic fixture path. Future policy should include richer anti-gaming:

```text id="yi56jq"
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
```

Do this carefully and keep deterministic integer math.

### Priority K — Admin/debug endpoints

Potential endpoints:

```text id="8e5k3r"
GET /rewarder/epochs
GET /rewarder/epochs/:epoch_id/status
GET /rewarder/epochs/:epoch_id/receipts
GET /rewarder/config/effective
GET /rewarder/policy/:policy_id
POST /rewarder/epochs/:epoch_id/retry-emit
```

Guard all of them with capabilities.

---

## 12. Known Commands

### Full gate

```bash id="sk793b"
cargo fmt
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo test -p svc-rewarder --all-targets
```

### Live earning-side WEB3 smoke

```bash id="qx34xt"
bash scripts/web3_accounting_rewarder_wallet_smoke.sh
```

Expected final line:

```text id="debu6r"
WEB3 accounting → rewarder HTTP emit → wallet → ledger smoke green
```

### Run service manually

```bash id="3ssmvd"
SVC_REWARDER_BIND_ADDR=127.0.0.1:18090 \
SVC_REWARDER_WALLET_BASE_URL=http://127.0.0.1:18088 \
SVC_REWARDER_WALLET_ISSUE_PATH=/v1/issue \
cargo run -p svc-rewarder
```

### Probe endpoints

```bash id="mpfasm"
curl -fsS http://127.0.0.1:18090/healthz
curl -fsS http://127.0.0.1:18090/readyz | jq .
curl -fsS http://127.0.0.1:18090/version | jq .
curl -fsS http://127.0.0.1:18090/metrics | head
```

### Fetch settlement preview

```bash id="f30uk5"
curl -fsS \
  http://127.0.0.1:18090/rewarder/epochs/interop-epoch-1/settlement \
  -H 'Authorization: Bearer dev' \
  | jq .
```

### Emit settlement

```bash id="jjrmvn"
curl -fsS -X POST \
  http://127.0.0.1:18090/rewarder/epochs/interop-epoch-1/emit \
  -H 'Authorization: Bearer dev' \
  | jq .
```

---

## 13. Suggested NOTES.MD Commit Summary

```text id="qyo088"
svc-rewarder: document wallet HTTP emit and live ROC earning-side proof

- record rewarder move from preview-only settlement to real /emit path
- document HTTP wallet issue client and deterministic idempotency behavior
- document accounting vector → rewarder compute → wallet issue live smoke
- preserve boundary: rewarder emits wallet intents, wallet commits, ledger is truth
- record final green gates: clippy, 9 integration tests, 41 unit tests, bench smoke, live smoke
- list remaining production hardening: auth, TLS/UDS, accounting fetch, manifest/receipt index, metrics
```

Longer body:

```text id="u8ccfe"
svc-rewarder now closes the earning side of the WEB3 ROC loop. It consumes a
ron-accounting reward snapshot vector, computes a deterministic reward manifest,
previews wallet-compatible settlement requests, and emits them directly to
svc-wallet through the new /rewarder/epochs/:epoch_id/emit path. Wallet
idempotency proves replay does not double issue.

The crate still obeys the core architecture: rewarder does not mutate ron-ledger
directly and does not own balances. svc-wallet remains the mutation front-door
and ron-ledger remains durable truth.
```

---

## 14. Final Status

`svc-rewarder` should now be considered:

```text id="f841b3"
Status: WEB3 beta reward issuance path essentially complete
WEB3 beta readiness: 94–97%
Production hardening readiness: 82–88%
Main remaining work: production wallet client, auth/capability wiring,
TLS/UDS transport, real accounting snapshot fetch, manifest/receipt indexing,
dependency readiness probes, and richer metrics.
```

The most important achievement is that the reward path is no longer theoretical and no longer manually script-mediated. It is now:

```text id="mbvkkw"
accounting vector
→ rewarder deterministic compute
→ rewarder HTTP emit
→ wallet issue
→ ledger-backed balances
→ replay-safe
```

That is a major WEB3 v1 beta milestone.


### END NOTE - APRIL 28 2026 - 23:00 CST



### BEGIN NOTE - JUNE 14 2026 - 16:30 CST

## 0. Executive summary

The `svc-rewarder` crate is now parked for the currently allowed QuickChain Phase-0 / preflight scope.

Current estimate:

```
svc-rewarder QuickChain Phase-0/preflight slice: 94–96% complete
svc-rewarder full future QuickChain role: 70–78% complete
overall QuickChain project: ~50–53% complete
```

This crate is now safe in the most important Phase-0 sense:

```
svc-rewarder is a deterministic ROC payout planner.
svc-rewarder is not a chain runtime.
svc-rewarder is not a validator.
svc-rewarder is not a bridge.
svc-rewarder is not a checkpoint writer.
svc-rewarder is not a root producer.
svc-rewarder is not a ledger mutation authority.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
ron-accounting remains snapshot/metering infrastructure, not balance truth.
QuickChain root/proof/validator/checkpoint work remains parked until future gates open.
```

The latest gate passed:

```
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

The script reached:

```
== svc-rewarder QuickChain preflight gate passed ==
```

The latest verification included:

```
cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder --test quickchain_preflight_boundary
cargo test -p svc-rewarder --test quickchain_preflight_raw_engagement
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_docs
cargo test -p svc-rewarder --all-targets
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

The latest full crate test inventory passed:

```
docs test: 5/5
boundary test: 4/4
raw engagement test: 4/4
replay/no-double-issue test: 4/4
funding-source test: 6/6
no-direct-mutation test: 4/4
integration tests: 9/9
unit tests: 41/41
bench smoke: reward_calc_100 Success
Clippy: clean with -D warnings
```

A fresh codebundle was generated afterward:

```
bash scripts/make_crate_codex.sh -c svc-rewarder
```

Latest result:

```
Wrote 73 files to crates/svc-rewarder/CODEBUNDLE.md
```

## 1. Crate role after this work

`svc-rewarder` now occupies a clean and narrow role in the internal ROC value loop.

Current role:

```
Consume sealed accounting/reward snapshots.
Validate strict reward policy inputs.
Compute deterministic ROC payout plans.
Produce deterministic reward manifests.
Produce wallet-shaped issue request previews.
Emit planned issue requests only through svc-wallet.
Never mutate ron-ledger directly.
Never claim balances, receipts, finality, roots, checkpoints, validator authority, bridge authority, or external settlement authority.
```

Value-loop position:

```
ron-proto economic DTOs
  -> ron-ledger durable economic truth
  -> svc-wallet mutation front-door
  -> ron-accounting metering/snapshots
  -> svc-rewarder payout planning
  -> svc-wallet issue requests / receipts
  -> ron-ledger truth
```

The most important doctrine now locked into this crate:

```
Rewarder plans.
Wallet mutates.
Ledger is truth.
Accounting snapshots are inputs, not balances.
QuickChain is future settlement infrastructure, not current runtime.
```

## 2. High-level changes completed

The work completed in `svc-rewarder` can be grouped into these areas:

```
1. QuickChain Phase-0 boundary tests.
2. Raw engagement rejection tests.
3. Replay and no-double-issue tests.
4. Funding-source provenance and policy validation.
5. No-direct-mutation tests.
6. Crate-local QuickChain docs preflight.
7. Preflight script expansion.
8. ron-accounting interop reinforcement.
9. wallet-front-door proof.
10. final gate verification and codebundle regeneration.
```

The crate now has a strong Phase-0 safety cage around the reward pipeline.

## 3. Files changed or materially affected

### 3.1 scripts/dev-quickchain-preflight.sh

File:

```
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Purpose:

```
One crate-local gate for all svc-rewarder QuickChain Phase-0/preflight checks.
```

Current behavior:

```
Resolves repo root from the script location.
Checks docs/quickchain-preflight.md exists.
Runs cargo fmt check for svc-rewarder.
Runs focused QuickChain preflight suites.
Runs cargo test -p svc-rewarder --all-targets.
Runs cargo clippy -p svc-rewarder --all-targets -- -D warnings.
Prints a green success marker only if all gates pass.
```

Focused suites now included:

```
quickchain_preflight_boundary
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_docs
```

Expected success marker:

```
== svc-rewarder QuickChain preflight gate passed ==
```

This script is now the command to run at the start of any future svc-rewarder QuickChain session.

Resume command:

```
cd /Users/mymac/Desktop/RustyOnions
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

### 3.2 docs/quickchain-preflight.md

File:

```
crates/svc-rewarder/docs/quickchain-preflight.md
```

Purpose:

```
Crate-local QuickChain boundary runbook.
Prevents future sessions from relying on memory.
Documents exactly what svc-rewarder is and is not allowed to become.
Documents focused test suites and future parked work.
Documents rewarder/wallet/ledger/accounting boundaries.
```

Important exact doctrine now captured in docs:

```
svc-rewarder is a deterministic ROC payout planner.
svc-rewarder is not a chain runtime.
svc-rewarder is not a validator.
svc-rewarder is not a bridge.
svc-rewarder is not a checkpoint writer.
svc-rewarder is not a root producer.
svc-rewarder is not a ledger mutation authority.
svc-wallet is the mutation front-door.
ron-ledger is durable economic truth.
```

Important allowed current work listed in docs:

```
strict serde DTOs
integer minor-unit money strings only
canonical lowercase b3 handles
explicit funding provenance
wallet issue request planning
deterministic payout planning
replay/dedupe safety
docs hardening
preflight tests
```

Important forbidden current work listed in docs:

```
no root-producing code
no checkpoint-producing code
no validator code
no bridge or external settlement code
no direct ledger mutation
no fake balances
no fake receipts
no fake finality
no raw engagement protocol ROC authority
no CrabLink chain authority
no gateway/omnigate/rewarder ledger mutation
```

Parked future work listed in docs:

```
canonical bytes and locked vectors
state/account Merkle roots
receipt roots
validator-set logic
checkpoint signing
external DA
public anchors
bridges
staking or liquidity
CrabLink chain authority
gateway/omnigate/rewarder ledger mutation
```

### 3.3 tests/quickchain_preflight_docs.rs

File:

```
crates/svc-rewarder/tests/quickchain_preflight_docs.rs
```

Purpose:

```
Enforces that the QuickChain Phase-0 boundary docs exist and keep the right doctrine.
Prevents future patches from deleting or softening the crate-local safety rules.
```

Important implementation detail:

```
Uses env!("CARGO_MANIFEST_DIR") to locate:
    docs/quickchain-preflight.md
```

This was necessary because the first version hardcoded:

```
crates/svc-rewarder/docs/quickchain-preflight.md
```

That path failed when Cargo ran the integration test from the crate context. The final version resolves the path from the crate manifest directory and is now green.

Tests now passing:

```
docs_state_rewarder_is_planning_only_not_chain_runtime
docs_name_allowed_and_forbidden_phase_zero_scope
docs_name_raw_engagement_replay_and_funding_boundaries
docs_list_every_focused_preflight_suite
docs_keep_future_quickchain_work_parked_outside_rewarder
```

The final wording fixes included ensuring the docs contain exact plain-text phrases:

```
not a chain runtime
no fake receipts
svc-wallet is the mutation front-door
ron-ledger is durable economic truth
```

### 3.4 tests/quickchain_preflight_boundary.rs

File:

```
crates/svc-rewarder/tests/quickchain_preflight_boundary.rs
```

Purpose:

```
Locks the basic QuickChain Phase-0 DTO/output safety boundary.
```

Tests passing:

```
compute_request_rejects_smuggled_quickchain_authority_fields
reward_manifest_does_not_expose_roots_receipts_balances_or_finality
wallet_preview_is_issue_request_shape_not_receipt_or_balance_truth
json_number_money_is_rejected_at_snapshot_wire_boundary
```

What this proves:

```
Compute request DTOs reject unknown authority-smuggling fields.
Reward manifests do not expose roots, receipts, balances, checkpoints, finality, anchors, or validators.
Wallet preview output is wallet issue request shaped, not receipt/balance truth.
Money must be JSON strings at the wire boundary, not numbers.
```

Smuggled fields rejected include examples like:

```
state_root
receipt_root
checkpoint_hash
validator_signature
settlement_status
finalized
ledger_receipt
wallet_balance
payout_authorized
```

Manifest/output forbidden terms checked include examples like:

```
state_root
receipt_root
accounting_root
reward_root
checkpoint_hash
validator
signature
receipt_hash
txid
balance_minor
available_minor
held_minor
finalized
anchored
```

### 3.5 tests/quickchain_preflight_raw_engagement.rs

File:

```
crates/svc-rewarder/tests/quickchain_preflight_raw_engagement.rs
```

Purpose:

```
Prevents raw engagement from becoming protocol ROC payout authority.
```

Tests passing:

```
accounting_snapshot_rejects_raw_engagement_contribution_fields
compute_request_rejects_top_level_raw_engagement_payout_fields
reward_policy_rejects_raw_engagement_formula_fields
current_allowed_contribution_counters_are_storage_egress_and_uptime_only
```

What this proves:

```
Raw views, likes, comments, impressions, watch seconds, clicks, and active users cannot be smuggled into accounting contribution rows.
Top-level compute requests cannot carry raw engagement payout fields.
Reward policies cannot carry raw engagement formulas or view-to-ROC payout ratios.
Current allowed contribution counters are limited to storage/egress/uptime style counters.
```

Allowed contribution fields:

```
account
bytes_stored
bytes_served
uptime_seconds
```

Rejected raw engagement fields include:

```
raw_views
raw_likes
raw_comments
raw_impressions
raw_watch_seconds
raw_clicks
raw_active_users
engagement_reward_minor_units
mint_from_views
reward_formula
raw_engagement_weight
watch_seconds_weight
views_to_roc_ratio
mint_authorized
payout_authorized
```

Important doctrine:

```
Raw engagement can be analytics.
Raw engagement can be metering.
Raw engagement can inform future non-protocol analytics.
Raw engagement must not directly mint or allocate protocol ROC.
```

### 3.6 tests/quickchain_preflight_replay_no_double_issue.rs

File:

```
crates/svc-rewarder/tests/quickchain_preflight_replay_no_double_issue.rs
```

Purpose:

```
Proves deterministic planning and replay/dedupe behavior without granting repeated payout authority.
```

Tests passing:

```
same_snapshot_policy_and_epoch_produce_same_plan_commitment
reordered_snapshot_rows_produce_same_plan
duplicate_epoch_replay_is_dedupe_not_second_payout_authority
idempotency_keys_are_retry_dedupe_not_operation_identity
```

What this proves:

```
Same epoch, policy, snapshot, and CID produce same plan commitment.
Reordered snapshot rows canonicalize to the same plan.
Duplicate epoch replay does not become a second payout authority.
Idempotency keys are retry/dedupe tools only.
```

Important doctrine:

```
idempotency_key is not operation_id.
idempotency_key is not account_sequence.
idempotency_key is not consensus authority.
idempotency_key is not validator finality.
operation_id remains backend-assigned durable ledger-operation identity in the ledger/wallet path.
account_sequence remains ledger-assigned, not rewarder-assigned.
```

### 3.7 tests/quickchain_preflight_funding_source.rs

File:

```
crates/svc-rewarder/tests/quickchain_preflight_funding_source.rs
```

Purpose:

```
Locks explicit funding provenance without letting funding metadata become settlement/finality authority.
```

Tests passing:

```
policy_requires_explicit_funding_source_on_wire
current_policy_accepts_explicit_protocol_pool_and_rejects_smuggled_authority_fields
unsigned_protocol_pool_policy_is_rejected_by_validator
compute_request_rejects_top_level_funding_authority_smuggling
manifest_carries_funding_provenance_but_not_funding_finality
wallet_preview_carries_batch_provenance_but_requests_remain_wallet_issue_shape
```

What this proves:

```
RewardPolicy must include explicit funding_source.
protocol_pool and governance_budget require signed policy.
Funding provenance may appear on the manifest and batch preview.
Individual wallet issue requests do not receive rewarder funding metadata.
Funding provenance is not settlement finality.
Funding provenance is not proof of mint authorization.
Funding provenance is not a receipt.
Funding provenance is not a bridge/anchor claim.
```

Current funding sources:

```
protocol_pool
advertiser_budget
creator_pool
sponsor_budget
governance_budget
```

Signed-policy requirement:

```
protocol_pool requires signed policy
governance_budget requires signed policy
```

Important files involved:

```
src/inputs/policy.rs
src/outputs/manifest.rs
src/outputs/intents.rs
tests/quickchain_preflight_funding_source.rs
```

### 3.8 tests/quickchain_preflight_no_direct_mutation.rs

File:

```
crates/svc-rewarder/tests/quickchain_preflight_no_direct_mutation.rs
```

Purpose:

```
Proves rewarder does not expose direct wallet/ledger/QuickChain/bridge mutation authority.
```

Tests passing:

```
config_rejects_external_settlement_bridge_anchor_validator_and_root_knobs
router_does_not_expose_direct_wallet_ledger_quickchain_or_bridge_mutation_routes
planning_outputs_do_not_claim_receipts_balances_operation_truth_roots_or_finality
compute_request_still_rejects_direct_mutation_authority_smuggling
```

What this proves:

```
Config rejects unknown external settlement/chain authority knobs.
Router does not expose direct wallet or ledger mutation routes.
Router does not expose QuickChain root/checkpoint/validator routes.
Router does not expose bridge/anchor settlement routes.
Planning outputs do not claim receipts, balances, operation truth, roots, or finality.
Compute request rejects direct mutation authority smuggling.
```

Rejected config examples include:

```
external_settlement
bridge_enabled
anchor_base_url
validator_set
root_production_enabled
checkpoint_writer_enabled
bridge_base_url
validator_rpc_url
validators
```

Forbidden routes checked include:

```
/v1/issue
/wallet/issue
/wallet/transfer
/wallet/burn
/ledger/issue
/ledger/transfer
/ledger/burn
/ledger/hold
/ledger/capture
/ledger/release
/ledger/append
/quickchain/root
/quickchain/checkpoint
/quickchain/validator
/quickchain/settle
/bridge/anchor
/bridge/settle
/anchors
/validators
```

Important note:

```
svc-rewarder does expose:
    /rewarder/epochs/:epoch_id/compute
    /rewarder/epochs/:epoch_id
    /rewarder/epochs/:epoch_id/settlement
    /rewarder/epochs/:epoch_id/emit

But `/emit` still emits through svc-wallet. It is not a direct ledger mutation route.
```

### 3.9 tests/integration/web3_roc_loop.rs

File:

```
crates/svc-rewarder/tests/integration/web3_roc_loop.rs
```

Purpose:

```
Proves the closed internal ROC loop:

    ron-accounting interop vector
      -> svc-rewarder deterministic settlement plan
      -> svc-wallet issue requests
      -> wallet balances changed exactly once
      -> idempotent replay does not double issue
```

What this test proves:

```
Rewarder can consume a ron-accounting reward snapshot interop vector.
Rewarder and accounting agree on canonical snapshot CID.
Rewarder computes expected payout totals.
Rewarder produces wallet issue requests.
svc-wallet accepts those issue requests.
svc-wallet returns receipts.
Balances change through wallet.
Replaying the same idempotency keys returns the same receipts.
Replay does not double balances.
```

Important expected values currently proven:

```
wallet_batch.wallet_path == "/v1/issue"
wallet_batch.total_minor_units == "999"
wallet_batch.requests.len() == 2
acct_a final amount == "356"
acct_b final amount == "643"
replay leaves acct_a at "356"
replay leaves acct_b at "643"
wallet_ops_total{op="issue"} 2
wallet_idempotency_replays_total 2
```

Important doctrine:

```
This is the strongest proof in svc-rewarder that reward planning can become real internal ROC movement only by passing through svc-wallet.
This does not make rewarder a ledger.
This does not make rewarder a wallet.
This does not make rewarder a QuickChain chain runtime.
```

## 4. Important source modules and current behavior

### 4.1 src/core/algebra.rs

Purpose:

```
Integer-only ROC minor-unit arithmetic.
```

Current key type:

```
AmountMinor(pub u128)
```

Important behavior:

```
Serializes as a decimal string.
Deserializes only from decimal string.
Rejects JSON-number money at DTO boundaries.
Uses checked add/subtract/multiply.
Avoids floats entirely.
```

Important helpers:

```
AmountMinor::checked_add
AmountMinor::checked_sub
AmountMinor::checked_mul_u128
checked_mul_div_floor
```

Important doctrine:

```
Money is integer minor-unit strings only.
No floats.
No wrapping arithmetic.
No negative amounts.
```

### 4.2 src/core/compute.rs

Purpose:

```
Pure deterministic reward calculation pipeline.
```

Important behavior:

```
Validates policy shape.
Requires canonical lowercase policy hash.
Canonicalizes snapshot contribution ordering.
Computes score from allowed counters.
Applies weight_bps using integer arithmetic.
Floors proportional allocation.
Drops dust below min payout into residual.
Sorts payouts by account.
Validates payout conservation.
Produces deterministic run_key.
Produces RewardManifest.
Does no IO.
```

Important fields:

```
ComputeInput.epoch_id
ComputeInput.inputs_cid
ComputeInput.policy
ComputeInput.snapshot
ComputeInput.dry_run
ComputeInput.idempotency_salt
```

Important helper:

```
run_key(epoch_id, policy_hash, inputs_cid, salt)
```

Important doctrine:

```
Same inputs must produce same plan.
Reordered snapshot rows must not affect output.
Compute must remain pure and deterministic.
IO must stay in HTTP/adapters, not core compute.
Compute cannot mutate ledger or wallet directly.
```

### 4.3 src/core/invariants.rs

Purpose:

```
Economic invariant checks.
```

Current invariants:

```
conservation: payouts do not exceed pool
overflow: arithmetic overflow must quarantine
idempotent: same inputs produce same run key and manifest commitment
```

Important behavior:

```
validate_payouts returns residual.
Payouts cannot exceed pool.
Zero payout entries cannot escape dust filtering.
Empty payout account cannot escape validation.
Arithmetic underflow/overflow leads to quarantine/error.
```

### 4.4 src/inputs/accounting.rs

Purpose:

```
Rewarder-side accounting snapshot DTO and canonical CID binding.
```

Current types:

```
AccountingSnapshot
AccountContribution
```

Current allowed contribution counters:

```
account
bytes_stored
bytes_served
uptime_seconds
```

Important behavior:

```
canonicalize trims account names and sorts contributions by account.
validate rejects empty account.
validate rejects duplicate accounts.
validate checks contribution score arithmetic.
canonical_snapshot_cid produces b3 hash over canonical JSON.
resolve_accounting_snapshot requires inline snapshot for now and checks inputs_cid matches canonical_snapshot_cid(snapshot).
```

Important future seam:

```
Inline snapshot is still required until a real ron-accounting fetch adapter is wired.
The future adapter must preserve the same integrity rule:

    inputs_cid == canonical_snapshot_cid(snapshot)
```

Important doctrine:

```
Rewarder consumes accounting snapshots.
Rewarder does not become accounting truth.
Accounting snapshots are not balances.
Snapshot CID is an artifact/input binding, not a QuickChain state root.
```

### 4.5 src/inputs/policy.rs

Purpose:

```
Reward policy DTO, resolver, validation helpers, and funding provenance.
```

Current types:

```
RewardPolicy
RewardFundingSource
```

Funding sources:

```
ProtocolPool
AdvertiserBudget
CreatorPool
SponsorBudget
GovernanceBudget
```

Wire labels:

```
protocol_pool
advertiser_budget
creator_pool
sponsor_budget
governance_budget
```

Important behavior:

```
RewardPolicy uses deny_unknown_fields.
funding_source is mandatory.
policy hash must be canonical b3:<64 lowercase hex>.
protocol_pool requires signed policy.
governance_budget requires signed policy.
weight_bps must be > 0 and <= 100000.
max_payout_minor_units must be >= min_payout_minor_units.
rounding must be floor.
```

Important current limitation:

```
signed: bool is a posture/seam, not real cryptographic verification yet.
Real policy registry/signature verification remains future work.
```

Important doctrine:

```
Funding source is provenance.
Funding source is not settlement finality.
Funding source is not mint authority.
Funding source is not bridge authority.
Funding source is not a wallet receipt.
```

### 4.6 src/outputs/manifest.rs

Purpose:

```
Reward manifest schema and commitment hashing.
```

Current important types:

```
RewardManifest
RewardPayout
RewardTotals
PolicySummary
LedgerSummary
ManifestStatus
```

Important behavior:

```
RewardManifest uses deny_unknown_fields.
Payouts are sorted by account before seal.
commitment_for_manifest hashes a CommitmentView that excludes the commitment field itself.
Commitment currently uses serde_json::to_vec over the view.
LedgerSummary records whether egress was emitted and the result label.
Manifest carries policy funding_source.
Manifest does not carry fake receipts, balances, roots, checkpoints, finality, validator claims, bridge claims, or anchor claims.
```

Important caveat:

```
Manifest commitment is deterministic within current serde_json struct ordering and current code.
This is not yet the full QuickChain locked canonical JSON v1 vector machinery.
Do not treat this commitment as a QuickChain state root or receipt root.
Future canonical bytes/locked vectors must be introduced before root-producing code.
```

### 4.7 src/outputs/intents.rs

Purpose:

```
Settlement intent DTOs and in-memory idempotent emitter seam.
```

Current important types:

```
SettlementIntent
SettlementBatch
WalletIssueRequest
WalletIssueBatch
IntentResult
IntentStore
```

Important constants:

```
ROC_ASSET = "roc"
WALLET_ISSUE_PATH = "/v1/issue"
```

Important behavior:

```
SettlementBatch::from_manifest builds deterministic issuance intents from manifest payouts.
Intents are sorted by recipient.
Intent totals must equal manifest payout totals.
WalletIssueBatch carries batch-level funding_source.
Individual WalletIssueRequest does not carry funding_source.
Wallet issue requests use:
    to
    asset
    amount_minor
    idempotency_key
    memo
idempotency_key is b3-tagged and capped to <=64 bytes.
IntentStore emits a run_key once and returns dup on replay.
dry_run emits nothing and does not consume the run key.
```

Important doctrine:

```
SettlementIntent is a plan, not a ledger operation.
WalletIssueRequest is the egress DTO shape, not a receipt.
IntentResult::Accepted is local/planning/egress posture, not consensus finality.
IntentStore is in-memory dev idempotency, not durable ledger operation truth.
```

### 4.8 src/outputs/wallet.rs

Purpose:

```
Wallet issue clients for turning reward settlement plans into svc-wallet issue requests.
```

Current clients:

```
DevWalletIssueClient
HttpWalletIssueClient
```

Important behavior:

```
DevWalletIssueClient previews issue batches without emitting.
DevWalletIssueClient emits through IntentStore and dedupes by run_key.
HttpWalletIssueClient posts each wallet issue request to svc-wallet `/v1/issue`.
HTTP client preserves idempotency key in both header and body.
HTTP client rejects HTTPS until a real TLS/shared transport adapter exists.
Dry-run posts nothing.
Direct bearer/cap values are not logged.
The current HTTP client is intentionally simple and dependency-light.
```

Important current limitation:

```
HTTP wallet client is local/simple HTTP/1.1 over Tokio TCP.
HTTPS is rejected until the TLS/shared transport adapter exists.
Bearer token is currently dev-style in tests.
Real egress authorization/capability/macaroon enforcement remains future work.
```

Important doctrine:

```
Rewarder targets wallet as mutation boundary.
Rewarder does not mutate ledger directly.
Every real economic effect must go through svc-wallet.
```

### 4.9 src/http/handlers.rs

Purpose:

```
HTTP handlers for compute, inspect, settlement preview, and explicit emit.
```

Important routes:

```
GET  /healthz
GET  /readyz
GET  /metrics
GET  /version
POST /rewarder/epochs/:epoch_id/compute
GET  /rewarder/epochs/:epoch_id
GET  /rewarder/epochs/:epoch_id/settlement
POST /rewarder/epochs/:epoch_id/emit
```

Important behavior:

```
compute_epoch requires Scope::Run.
get_epoch and get_settlement require Scope::Inspect.
emit_settlement requires Scope::Run.
epoch_id validation is bounded and allows only selected safe characters.
compute path resolves and validates CID, snapshot, and policy.
compute path first validates pure economic manifest with dry-run egress.
compute path plans settlement batch from validated manifest.
compute path emits only via DevWalletIssueClient for local/dev path.
emit path posts to svc-wallet via HttpWalletIssueClient.
dry-run manifest cannot be emitted; recompute with dry_run=false first.
metrics are updated for compute latency, planned intents, intent outcome, rejected reason, and run status.
```

Important doctrine:

```
`/settlement` is preview/read-only.
`/emit` is explicit and wallet-front-door-only.
No direct wallet/ledger mutation routes are exposed by rewarder.
No QuickChain root/checkpoint/validator/bridge routes are exposed.
```

### 4.10 src/config/types.rs and src/config/validate.rs

Purpose:

```
Strong typed config and fail-closed validation.
```

Important hardening:

```
serde deny_unknown_fields on config structs.
request body cap is bounded.
decompression ratio cap is bounded.
concurrency workers/queues must be nonzero.
TLS requires cert/key paths if enabled.
HTTP base URLs must start with http:// or https://.
wallet issue path must start with slash and contain no whitespace.
wallet capability scope must not be empty.
pq.mode must be off or hybrid.
shard strategy must be single, by_actor, or by_content.
Unknown config keys are rejected.
```

Important QuickChain safety:

```
Config does not support bridge, anchor, validator, root, checkpoint writer, external settlement, staking, liquidity, Solana, ROX, or public chain authority knobs.
Attempts to smuggle these knobs through TOML are covered by tests.
```

### 4.11 src/lib.rs

Important crate-level safety:

```
#![forbid(unsafe_code)]
#![deny(clippy::await_holding_lock)]
```

The crate remains safe Rust only.

## 5. Current test inventory

### 5.1 Focused QuickChain tests

Current focused tests:

```
quickchain_preflight_boundary.rs
quickchain_preflight_raw_engagement.rs
quickchain_preflight_replay_no_double_issue.rs
quickchain_preflight_funding_source.rs
quickchain_preflight_no_direct_mutation.rs
quickchain_preflight_docs.rs
```

Latest focused preflight result:

```
boundary: 4 passed
raw engagement: 4 passed
replay/no-double-issue: 4 passed
funding source: 6 passed
no-direct-mutation: 4 passed
docs: 5 passed
```

Total focused QuickChain preflight tests:

```
27 focused tests passing
```

### 5.2 Integration tests

Integration test file:

```
tests/integration.rs
```

Included modules:

```
egress_dedupe
http_compute
readiness
web3_roc_loop
```

Latest integration result:

```
9 passed
0 failed
```

Important integration coverage:

```
egress dedupe by run_key
HTTP compute happy path
deterministic replay
inputs_cid mismatch rejection
settlement preview shape
dry-run promotion to production
metrics include planned settlement intent counters
readiness happy/degraded behavior
ron-accounting -> rewarder -> svc-wallet ROC loop
```

### 5.3 Unit tests

Unit test modules:

```
accounting_interop
accounting_policy
artifacts
config
config_file
idempotency
invariants
quarantine_edges
settlement
wallet_client
```

Latest unit result:

```
41 passed
0 failed
```

Important unit coverage:

```
canonical snapshot order and whitespace handling
matching/mismatched inputs_cid
duplicate account rejection
ron-accounting vector compatibility
expected reward manifest and wallet preview from interop vector
policy resolver default inline absence
policy hash mismatch rejection
uppercase hash rejection
default config validation
TLS path validation
wallet base URL validation
wallet cap scope validation
wallet issue path validation
zero workers rejection
run_key determinism
payout conservation
payout cannot exceed pool
arithmetic overflow quarantine
dust below min payout becomes residual
zero activity snapshot yields residual
settlement batch total equals manifest payout total
settlement intents sorted by recipient
wallet issue request serializes amount as string
artifact writing suppressed in amnesia mode
artifact filename sanitization
artifact write persists when amnesia disabled
dev wallet dry run does not consume run key
dev wallet emit idempotent
dev wallet preview does not emit
HTTP wallet client rejects HTTPS until TLS adapter exists
HTTP wallet dry run posts nothing
HTTP wallet posts issue requests to wallet route
config fixture valid
unknown config keys rejected
partial config overlays defaults
```

### 5.4 Bench smoke

Bench file:

```
benches/reward_calc.rs
```

Latest result:

```
Testing reward_calc_100
Success
```

This is not a full performance certification, but it proves the bench target still builds and runs after QuickChain preflight additions.

## 6. What is now safe to say about svc-rewarder

Safe statements:

```
svc-rewarder is now Phase-0/preflight parked for the allowed QuickChain scope.
svc-rewarder now has explicit tests preventing direct ledger mutation authority.
svc-rewarder now has explicit tests preventing root/checkpoint/validator/bridge creep.
svc-rewarder now rejects raw engagement payout authority fields.
svc-rewarder now requires explicit funding provenance.
svc-rewarder now rejects unsigned protocol/governance funding policies.
svc-rewarder now keeps funding provenance out of individual wallet issue requests.
svc-rewarder now proves rewarder-planned wallet issue requests can mutate ROC only through svc-wallet.
svc-rewarder now has crate-local QuickChain docs that are themselves tested.
svc-rewarder now has a single local preflight gate for future sessions.
```

Do not overclaim:

```
Do not say svc-rewarder is a production-grade settlement system yet.
Do not say svc-rewarder has QuickChain roots.
Do not say svc-rewarder has checkpoint finality.
Do not say svc-rewarder has validator consensus.
Do not say svc-rewarder has durable idempotency across process restarts.
Do not say svc-rewarder has real policy signature verification.
Do not say svc-rewarder has real TLS/shared transport.
Do not say svc-rewarder is ready for external settlement.
Do not say svc-rewarder is allowed to interact with public anchors or bridges.
```

## 7. Remaining work for svc-rewarder

The remaining work is not urgent Phase-0 safety work. Most of it is production hardening or future-phase integration.

### 7.1 Real ron-accounting fetch adapter

Current state:

```
resolve_accounting_snapshot requires inline snapshot.
Inline snapshot must match inputs_cid.
ron-accounting interop vector is proven in tests.
```

Remaining work:

```
Add real adapter to fetch sealed reward snapshot from ron-accounting by CID or epoch.
Preserve canonical_snapshot_cid verification.
Preserve strict DTO rejection.
Add timeout/backpressure behavior.
Add dependency error mapping.
Add integration tests against ron-accounting service/router when ready.
```

Rules for future adapter:

```
Must not trust fetched bytes until CID verifies.
Must not accept balances as reward truth.
Must not accept QuickChain roots from accounting.
Must not let ron-accounting mutate rewarder plans.
Must keep accounting as metering/snapshot source only.
```

### 7.2 Real policy registry and signature verification

Current state:

```
RewardPolicy has signed: bool.
protocol_pool and governance_budget require signed == true.
This is a seam, not cryptographic enforcement.
```

Remaining work:

```
Add policy registry adapter.
Verify policy hash against fetched policy bytes.
Verify signatures/capabilities for protocol/governance funding sources.
Add failure modes for unsigned/stale/mismatched policy.
Keep the current validation helpers as shared enforcement.
```

Rules:

```
Policy registry can authorize policy validity.
Policy registry cannot mint ROC.
Policy registry cannot finalize settlement.
Policy registry cannot bypass svc-wallet.
Policy registry cannot introduce raw engagement payout formulas unless future doctrine explicitly permits a safe non-protocol class.
```

### 7.3 Durable idempotency across restarts

Current state:

```
IntentStore is in-memory.
It prevents duplicate emit by run_key during process lifetime.
svc-wallet idempotency protects actual wallet issue requests.
```

Remaining work:

```
Decide whether rewarder needs durable idempotency state.
If yes, add bounded durable store or rely exclusively on svc-wallet durable idempotency.
Add restart/replay tests.
Preserve rule that idempotency_key is retry/dedupe only, not ledger operation identity.
```

Rules:

```
Do not make rewarder the source of operation_id.
Do not assign account_sequence in rewarder.
Do not make rewarder durable ledger truth.
Do not treat rewarder idempotency as finality.
```

### 7.4 Stronger wallet egress auth/capability

Current state:

```
Tests use dev token.
Config has wallet_cap_scope and macaroon_path seam.
HttpWalletIssueClient currently uses explicit bearer token.
Secrets are not logged.
```

Remaining work:

```
Add real attenuated capability/macaroon loading.
Ensure tokens/caps are never logged.
Ensure caps are scoped only to rewarder issue egress.
Add negative tests for missing/invalid/overbroad capabilities.
Add redaction tests if logging grows.
```

Rules:

```
No uncapped spend authority.
No raw secrets in logs/errors.
No private keys in React or client layers.
No direct ledger mutation capability.
```

### 7.5 Shared transport / TLS adapter

Current state:

```
HTTP wallet client is dependency-light and HTTP-only.
HTTPS is rejected until adapter exists.
TLS runtime validation exists for service serving posture.
```

Remaining work:

```
Replace direct Tokio TCP client with shared hardened HTTP client/transport.
Add HTTPS support when TLS adapter is ready.
Preserve timeouts.
Preserve bounded body behavior.
Add better response size caps and redaction.
Add tests for TLS mode and failure mapping.
```

Rules:

```
Do not add ad hoc crypto shortcuts.
Do not log bearer tokens.
Do not silently downgrade TLS posture.
```

### 7.6 Artifact retention and audit integration

Current state:

```
maybe_write_manifest writes only when amnesia disabled.
Artifact filenames sanitize epoch_id.
Amnesia mode suppresses disk writes.
```

Remaining work:

```
Decide production retention policy.
Add audit bus integration if needed.
Add artifact cleanup/retention enforcement.
Add integrity checks for retained artifacts.
Keep artifacts as audit/planning records, not balance truth.
```

Rules:

```
Artifact commitment is not QuickChain root.
Artifact is not finality.
Artifact is not a wallet receipt.
```

### 7.7 Metrics and readiness expansion

Current state:

```
Metrics include reward runs, compute latency, ledger/wallet intent result, planned settlement intents, rejects, readyz degradation.
Readiness has config_loaded, ledger_ok, policy_registry_ok, queue_ok.
```

Remaining work:

```
Wire real dependency readiness for accounting, wallet, and policy registry.
Add cache hit/miss metrics if needed.
Add queue/backpressure metrics if real work queue is added.
Keep labels low cardinality.
Never use account IDs, policy hashes, run keys, or CIDs as metric labels.
```

### 7.8 Real worker/queue model

Current state:

```
ConcurrencyGates use semaphores for compute and IO.
No unbounded compute queue.
HTTP path uses try_acquire_owned.
```

Remaining work:

```
If heavier reward computation arrives, add bounded worker queue.
Add overload tests.
Add cancellation/shutdown tests.
Preserve no lock across await.
Preserve deterministic compute.
```

### 7.9 Canonical bytes and locked vectors

Current state:

```
Rewarder has deterministic JSON-based commitments for current manifests.
Tests prove deterministic behavior.
This is not yet full QuickChain canonical JSON v1 locked vector machinery.
```

Remaining work, future only:

```
Define canonical bytes.
Generate sketch vectors.
Lock bytes.
Lock hashes.
Only then consider root/proof machinery in the correct crate.
```

Rules:

```
No fake hashes/placeholders.
No root-producing code until canonical bytes and golden vectors are ready.
Preimage doctrine for future QuickChain remains:
    domain_separator_bytes || 0x00 || canonical_payload_bytes
Hash format remains:
    b3:<64 lowercase hex>
```

## 8. Explicitly forbidden future changes in svc-rewarder unless QuickChain gates change

Do not add these to svc-rewarder:

```
QuickChain validators
validator sets
checkpoint writer
checkpoint signing
state roots
account Merkle roots
receipt roots
proof generation
pruning
external DA
anchors
public bridges
Solana integration
ROX integration
staking
liquidity
exchange-facing logic
public settlement
direct ron-ledger mutation routes
direct wallet mutation routes
raw engagement protocol payout formulas
CrabLink chain authority
gateway/omnigate/rewarder ledger mutation authority
```

Do not add fields like:

```
operation_id
account_sequence
hold_id
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
bridge_authorized
anchor_authorized
external_settlement
funding_receipt
funding_finalized
ledger_receipt
wallet_receipt
receipt_hash
txid
balance_minor
available_minor
held_minor
finalized
anchored
protocol_minted
mint_authorized
```

unless a future gated phase explicitly introduces them in the correct crate with canonical bytes, vectors, and doctrine approval.

## 9. Current completion assessment

### 9.1 svc-rewarder Phase-0/preflight

Estimated completion:

```
94–96%
```

Why not 100%:

```
Real ron-accounting adapter is not wired.
Real policy registry/signature verification is not wired.
Durable rewarder-side idempotency across restarts is not decided.
Real attenuated wallet egress auth is not wired.
Shared transport/TLS adapter is not wired.
Future canonical bytes/locked vectors are not part of rewarder yet.
```

Why it is park-ready anyway:

```
These missing pieces are production/future integration seams.
The dangerous Phase-0 authority boundaries are now tested and documented.
The crate does not claim forbidden QuickChain authority.
The crate passes full preflight, all-targets, bench smoke, and Clippy.
```

### 9.2 svc-rewarder future QuickChain role

Estimated completion:

```
70–78%
```

Remaining future role work:

```
More robust policy/accounting/wallet adapters.
Stronger auth/capability.
Durable operational semantics if required.
Canonical vector alignment when future QuickChain gates open.
Better production transport/hardening.
```

### 9.3 Overall QuickChain project

Estimated completion:

```
~50–53%
```

Reason:

```
ron-proto, ron-ledger, svc-wallet, ron-accounting, and svc-rewarder now have substantial Phase-0 discipline.
The internal ROC value-plane chain is becoming coherent.
But full QuickChain remains future work:
    canonical bytes
    golden vectors
    roots
    proofs
    validators
    checkpoints
    DA/archive/challenge fallback
    pruning after proofs/DA
    anchors after internal ROC is proven
    no public settlement until later gates
```

## 10. Recommended next actions

### 10.1 If staying in svc-rewarder

Only do cleanup/hardening, not new QuickChain authority.

Safe next tasks:

```
Add final crate-local NOTES.md if not already present.
Add README reference to docs/quickchain-preflight.md.
Add comments around emit_settlement explaining wallet-front-door-only behavior.
Add TODO comments for real accounting adapter and policy registry.
Add a startup/runbook snippet for local dev.
Add low-risk docs for supported routes.
```

Avoid:

```
New roots.
New proof fields.
New validator fields.
New checkpoint fields.
New bridge/anchor settings.
New public settlement terms.
```

### 10.2 If moving to the next crate

Recommended next session target depends on broader project state.

If all current QuickChain value-loop crates have carryover notes, move toward:

```
svc-storage / svc-gateway / omnigate paid enforcement preflight
```

or run a workspace-level QuickChain status pass.

Likely next value-loop work:

```
Confirm paid enforcement services consume wallet/ledger truth only.
Confirm no fake receipts or cache-only unlocks.
Confirm gateway/omnigate do not mutate ledger.
Confirm offline cache cannot unlock paid content alone.
Confirm display-only receipt caches are backend-derived.
```

### 10.3 Session resume commands

Run these at the start of any future svc-rewarder session:

```
cd /Users/mymac/Desktop/RustyOnions
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Then regenerate bundle after any code changes:

```
bash scripts/make_crate_codex.sh -c svc-rewarder
```

Optional focused checks:

```
cargo test -p svc-rewarder --test quickchain_preflight_boundary
cargo test -p svc-rewarder --test quickchain_preflight_raw_engagement
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_docs
cargo test -p svc-rewarder --all-targets
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

## 11. Mental model for future maintainers

Remember this crate by one sentence:

```
svc-rewarder plans deterministic internal ROC payouts from sealed accounting snapshots and explicit policy, then hands wallet-shaped issue requests to svc-wallet; it never becomes ledger truth, chain truth, settlement finality, bridge authority, or raw-engagement mint authority.
```

Short doctrine:

```
DTOs before roots.
Determinism before distribution.
Rewarder plans.
Wallet mutates.
Ledger is truth.
Accounting is snapshot input.
Funding source is provenance, not finality.
Idempotency is dedupe, not operation identity.
Raw engagement is not protocol ROC authority.
No fake balances.
No fake receipts.
No fake finality.
No roots until canonical bytes and locked vectors.
No validators/checkpoints/bridges/anchors/staking/liquidity/public settlement in this crate.
```

## 12. Final status

`svc-rewarder` is now parked for QuickChain Phase-0/preflight.

Green proof:

```
docs preflight passed
boundary preflight passed
raw engagement preflight passed
replay/no-double-issue preflight passed
funding-source preflight passed
no-direct-mutation preflight passed
all-targets tests passed
unit tests passed
integration tests passed
bench smoke passed
Clippy -D warnings passed
codebundle regenerated
```

Do not reopen svc-rewarder for root/checkpoint/validator/bridge work. Reopen it only for safe adapter hardening, docs, transport/auth hardening, or future gated integration after QuickChain canonical bytes/vector doctrine is ready.

### END NOTE - JUNE 14 2026 - 16:30 CST


### BEGIN NOTE - JUNE 17 2026 - 13:10 CST

Confirmed: `svc-storage` is fully parked. The terminal shows the storage parking gate passed, then the forced codebundle regeneration wrote `75` files for `svc-rewarder` and `104` files for `svc-storage`. 

Here are the comprehensive notes to drop into the crate notes.

# QuickChain Phase-0 Crate Notes — `svc-rewarder` + `svc-storage`

## Status Summary

This notes entry records the completed QuickChain Phase-0 / preflight sweep for the paired crates:

* `crates/svc-rewarder`
* `crates/svc-storage`

Both crates have now been brought into the QuickChain safety/preflight doctrine for the current internal-ROC buildout.

This work does **not** make either crate a QuickChain runtime, validator, bridge, root producer, settlement layer, public-chain component, public finality source, or external-settlement participant.

The purpose of this sweep was to harden each crate’s current RustyOnions role before later QuickChain root/proof/checkpoint work begins elsewhere.

Current doctrine remains:

* `svc-wallet` is the economic mutation front-door.
* `ron-ledger` is durable replayable balance truth.
* `ron-accounting` produces accounting snapshots and reports, but is not balance truth.
* `svc-rewarder` plans deterministic payouts but must not mutate ledger truth directly.
* `svc-storage` stores and serves bytes by canonical b3, but must not become wallet, ledger, finality, root, bridge, validator, or paid-access authority.
* QuickChain is future settlement infrastructure, not current runtime authority.
* ROC remains internal.
* No ROX, Solana, bridge, staking, liquidity, external settlement, validators, anchors, or public-chain authority were added.

---

# Part I — `svc-rewarder`

## Crate Role

`svc-rewarder` is the deterministic reward planning service.

Its proper responsibility is to consume accounting/policy-style inputs, compute payout plans deterministically, expose manifests/intents, and route any actual economic mutation through `svc-wallet`.

It is **not** the ledger.

It is **not** the wallet.

It is **not** QuickChain.

It is **not** a validator.

It is **not** a settlement authority.

It must not directly issue, mint, transfer, burn, hold, capture, release, or mutate ROC balances outside the wallet/ledger path.

The essential boundary is:

`ron-accounting snapshots/reports -> svc-rewarder deterministic payout planning -> svc-wallet mutation front-door -> ron-ledger durable truth`

## QuickChain Phase-0 Purpose for `svc-rewarder`

The Phase-0 work for `svc-rewarder` was designed to prevent a dangerous architectural drift:

A reward service that sees “engagement,” “usage,” or “contribution” data could accidentally become a hidden minting path, a direct payout authority, or a proto-consensus authority. The preflight sweep prevents that.

The goal was to prove that `svc-rewarder` can participate in the internal ROC value loop without becoming:

* direct ledger mutation authority;
* raw engagement payout authority;
* fake protocol reward minting authority;
* settlement authority;
* root/checkpoint producer;
* validator;
* bridge;
* external-token integration point.

The crate is now framed as a deterministic payout planner with explicit boundaries.

## Key Doctrine Added / Enforced

The rewarder doctrine now makes the following clear:

* Raw engagement must never directly mint or allocate protocol ROC.
* Rewarder output is planning/intent material, not final ledger truth.
* Rewarder may prepare deterministic manifests and payout plans.
* Rewarder may emit wallet issue/capture/intents only through explicit wallet-facing seams.
* Wallet remains the mutation front-door.
* Ledger remains the durable replayable truth.
* Accounting remains upstream input/snapshot/report material, not balance truth.
* Funding provenance must be explicit.
* Protocol-pool and governance-style funding sources must not be accepted casually from unsigned/unverified policy material.
* Integer minor-unit accounting is required.
* Float money math is forbidden.
* Payout conservation must hold.
* Rewarder may not invent receipts, balances, roots, finality, checkpoints, validators, or bridges.
* Rewarder must not contain Solana, ROX, external settlement, staking, liquidity, validator-market, or public-chain logic.

## QuickChain Test Coverage Added / Verified

The focused QuickChain preflight suite for `svc-rewarder` consists of seven test targets:

1. `quickchain_preflight_boundary`
2. `quickchain_preflight_docs`
3. `quickchain_preflight_funding_source`
4. `quickchain_preflight_no_direct_mutation`
5. `quickchain_preflight_raw_engagement`
6. `quickchain_preflight_replay_no_double_issue`
7. `quickchain_tooling_boundary`

These tests collectively protect the crate’s intended role.

### `quickchain_preflight_boundary`

Purpose:

* Confirms rewarder routes and public shapes do not claim QuickChain authority.
* Guards against accidental exposure of chain/runtime/validator/finality behavior.
* Ensures rewarder remains a service that computes reward plans, not a settlement chain.

Boundary concepts covered:

* no state roots;
* no receipt roots;
* no checkpoints;
* no validators;
* no finality claims;
* no bridge claims;
* no external settlement claims;
* no balance truth claims;
* no direct ledger truth exposure.

### `quickchain_preflight_docs`

Purpose:

* Ensures the crate has explicit written QuickChain preflight doctrine.
* Prevents future work from silently deleting the boundary notes.
* Keeps the crate’s role readable to the next developer/session.

Docs are expected to explain:

* rewarder plans payouts;
* wallet mutates;
* ledger is truth;
* accounting is input/snapshot/report material;
* raw engagement is not direct protocol ROC authority;
* QuickChain is future infrastructure, not a current runtime path;
* no roots/checkpoints/validators/bridges/external settlement are allowed in this crate.

### `quickchain_preflight_funding_source`

Purpose:

* Guards the reward funding model.
* Ensures reward plans declare explicit funding provenance.
* Prevents silent protocol-pool allocation from arbitrary raw engagement.
* Prevents unsigned/unverified policy material from becoming protocol ROC authority.

Key protected idea:

`funding_source` is provenance, not mutation authority.

Funding-source examples:

* protocol pool;
* advertiser budget;
* creator pool;
* sponsor budget;
* governance budget.

Important doctrine:

* Protocol-pool and governance-budget style funding should require stronger policy/signed verification.
* Raw engagement events should not become direct issuance.
* Rewarder should plan payouts under declared policy, not invent an economic source.

### `quickchain_preflight_no_direct_mutation`

Purpose:

* Confirms rewarder does not mutate ledger balances directly.
* Confirms rewarder does not become a wallet replacement.
* Confirms wallet-facing mutation paths are explicit.
* Confirms direct ledger mutation symbols or routes do not creep into rewarder.

Protected boundary:

`svc-rewarder -> svc-wallet -> ron-ledger`

not:

`svc-rewarder -> ron-ledger direct mutation`

not:

`svc-rewarder -> hidden balance mutation`

not:

`svc-rewarder -> fake receipt`

### `quickchain_preflight_raw_engagement`

Purpose:

* Prevents raw usage/engagement data from directly producing protocol ROC payouts.
* Forces engagement to pass through accounting/policy/reward planning.
* Blocks bot-farm-friendly designs where views/clicks alone become protocol mint authority.
* Supports the longer-term doctrine that useful node/provider work can be rewardable, but only through deterministic policy and accounting.

Protected idea:

Raw engagement can be analytics, metering, or accounting input. It is not direct payout authority.

Event-class alignment:

* `analytics_only` events do not pay directly.
* `metering` events may feed accounting.
* `proof_eligible` events may become eligible only after policy/accounting verification.
* `ad_budgeted` events are budget-constrained, not protocol-mint authority.
* `economic_receipt` events reflect backend wallet/ledger truth, not rewarder invention.

### `quickchain_preflight_replay_no_double_issue`

Purpose:

* Ensures reward planning and settlement-intent behavior remains idempotent.
* Prevents replay from producing duplicate issue effects.
* Keeps epoch/policy/input combinations deterministic.
* Reinforces that retry keys and operation identity must not be confused.

Important identity doctrine:

* `operation_id` is durable backend-assigned ledger-operation identity.
* `idempotency_key` is retry protection, not authority.
* Rewarder run keys and manifests must be deterministic for the same sealed inputs.
* Replay must not cause double issuance.

### `quickchain_tooling_boundary`

Purpose:

* Confirms preflight tooling is Bash/cargo-only.
* Confirms no Python helper tooling is checked into the crate for this QuickChain sweep.
* Confirms the exhaustive preflight script discovers focused QuickChain tests dynamically.
* Confirms the parking script delegates to the exhaustive preflight gate.

This protects the workflow from stale hardcoded lists and tool drift.

## Scripts Added / Validated

### `crates/svc-rewarder/scripts/dev-quickchain-preflight.sh`

Purpose:

Runs the exhaustive QuickChain preflight gate for `svc-rewarder`.

The script verifies:

* required docs exist;
* no checked-in Python helper files exist under `crates/svc-rewarder`;
* formatting is clean with `cargo fmt -p svc-rewarder -- --check`;
* focused QuickChain tests are discovered dynamically from `crates/svc-rewarder/tests/quickchain*.rs`;
* every discovered focused QuickChain test target is run;
* `cargo test -p svc-rewarder --all-targets` passes;
* `cargo clippy -p svc-rewarder --all-targets -- -D warnings` passes;
* forbidden-scope marker is printed;
* final dynamic test-count marker is printed.

Important: the script should remain dynamic. Do not replace it with a stale hardcoded test list.

### `crates/svc-rewarder/scripts/dev-quickchain-park.sh`

Purpose:

Parking gate for the crate.

This script validates required preflight files exist, then delegates to:

`crates/svc-rewarder/scripts/dev-quickchain-preflight.sh`

Expected final parking doctrine:

If the parking gate passes, `svc-rewarder` is parked for the current QuickChain Phase-0 sweep.

## Code/Architecture Areas Now Covered

### Deterministic Reward Computation

Reward computation is treated as pure planning.

Important characteristics:

* no floats;
* checked arithmetic;
* integer minor units;
* deterministic score/payout ordering;
* canonical account sorting;
* conservation checks;
* residual calculation;
* dry-run support;
* explicit egress result modeling.

### Amount Handling

The rewarder uses integer minor units through an `AmountMinor` style.

Important properties:

* serialized as decimal strings at JSON boundaries;
* no floating-point ROC;
* checked add/sub/mul paths;
* overflow/underflow lead to quarantine-style errors;
* zero/negative payouts do not escape as valid payout entries.

### Payout Conservation

The rewarder validates:

* sum of payouts must not exceed pool;
* residual equals pool minus payouts;
* zero payout entries are filtered/rejected;
* empty account destinations are rejected;
* arithmetic overflow quarantines the run.

### Run Keys / Idempotency

Rewarder run identity uses deterministic input material such as:

* epoch id;
* policy hash;
* inputs CID;
* idempotency salt/domain separator.

The run key protects replay semantics and makes repeated computation stable for the same sealed inputs.

Important caution:

Run keys and idempotency keys are not ledger authority.

They are determinism/retry controls only.

### Policy and Funding Source

Reward policy now has stronger structure:

* policy id;
* policy hash;
* signed/verified flag;
* explicit funding source;
* maximum payout cap;
* minimum payout filter;
* basis-point weight;
* rounding mode.

Important doctrine:

* funding source is provenance, not mutation authority;
* protocol/governance-style funding requires stronger trust;
* rewarder must not infer protocol mint authority from engagement alone.

### Wallet Mutation Boundary

Rewarder may prepare wallet issue/intents, but must not mutate ledger directly.

Any real economic mutation must go through `svc-wallet`.

This preserves the value-plane boundary:

`svc-rewarder` plans.

`svc-wallet` mutates.

`ron-ledger` records truth.

### Accounting Input Boundary

Rewarder consumes accounting snapshots or accounting-derived input material.

Accounting is not balance truth.

Rewarder must not treat accounting counters as final wallet balance truth.

Accounting can say what happened in a metering/reporting sense. Wallet/ledger decides economic mutation.

### Public HTTP Boundary

Rewarder routes should not expose:

* balance truth;
* root truth;
* receipt root;
* state root;
* checkpoint;
* finality;
* validator signatures;
* bridge settlement;
* external anchor;
* staking/liquidity behavior.

Routes are for health/readiness/metrics/version, compute, inspect, settlement preview/emission through wallet-facing seams, and related operational behavior.

## `svc-rewarder` Validation Status

The crate has been validated through its focused QuickChain preflight gate.

The relevant gate includes:

* focused QuickChain test targets;
* all-targets test;
* clippy with warnings denied;
* Bash/cargo-only tooling boundary;
* no Python helper files;
* forbidden-scope marker.

Current state:

`svc-rewarder` is parked for the current QuickChain Phase-0 sweep.

## What Remains for `svc-rewarder`

The crate is parked for Phase-0/preflight, but not “done forever.”

Future work should stay ordered and gated.

### Near-Term Remaining Work

1. Keep rewarder aligned with future `ron-accounting` sealed snapshot format.

When accounting snapshots become more formal, rewarder should consume them without weakening the boundary.

2. Keep payout planning deterministic as schemas evolve.

If new contribution counters or scoring policies are added, they must preserve deterministic ordering, integer math, and bounded overflow behavior.

3. Add stronger policy registry integration later.

Current policy validation is enough for Phase-0. Future work may integrate signed policy registry material from `ron-policy`, but must not make rewarder itself policy authority.

4. Improve production wallet egress hardening later.

Rewarder can preview/emit through wallet-facing seams, but production egress should continue tightening:

* auth scopes;
* retry behavior;
* idempotency;
* failure classification;
* replay prevention;
* receipt handling;
* audit logging.

5. Add formal reward artifact vectors later.

Rewarder output vectors can be useful later, but must follow the vector doctrine:

`sketch -> locked_bytes -> locked_hash`

No fake hashes.

No placeholder commitments.

No roots until canonical bytes and root engines are approved.

### Long-Term Remaining Work

1. Reward roots are future work.

`reward_root` or reward manifest commitments should not be treated as QuickChain roots until:

* canonical bytes are frozen;
* domain separators are frozen;
* BLAKE3 preimage framing is frozen;
* golden vectors are locked;
* independent verifier can reproduce bytes and hashes.

2. No validator/committee behavior belongs here.

Even later, rewarder should not become the validator set. It can provide reward artifacts consumed by other systems.

3. No bridge/external settlement logic belongs here.

No ROX, Solana, L2, bridge, staking, liquidity, exchange-facing, or external anchor behavior should be added to rewarder during internal ROC proving.

4. Raw engagement must stay non-authoritative.

If CrabLink adds views, prompts, AI usage, creator engagement, ads, storage work, node uptime, or other signals, rewarder should only consume policy/accounting-validated inputs. It should never mint from raw activity alone.

---

# Part II — `svc-storage`

## Crate Role

`svc-storage` is the content-addressed byte/object service.

Its proper responsibility is:

* store bytes;
* derive canonical b3 content IDs from bytes;
* serve bytes by canonical b3;
* serve bounded ranges for media;
* support paid-write admission after backend-derived proof;
* produce usage/metering signals for accounting;
* expose observability without leaking authority;
* remain storage infrastructure, not economic truth.

`svc-storage` is not:

* wallet;
* ledger;
* accounting truth;
* rewarder;
* gateway authority;
* omnigate authority;
* QuickChain authority;
* root/checkpoint producer;
* validator;
* bridge;
* finality source;
* external-settlement participant.

## QuickChain Phase-0 Purpose for `svc-storage`

The Phase-0 work for `svc-storage` was designed to prevent storage from becoming a hidden economic authority.

Storage is near paid content, byte access, cache, and media. That makes it dangerous if boundaries are unclear.

The preflight sweep proves:

* storage only stores/serves bytes;
* b3 hashes identify bytes only;
* paid writes require backend-derived proof;
* cache cannot unlock paid content alone;
* storage does not invent wallet receipts;
* storage does not claim finality;
* storage does not mutate balances directly;
* settlement/capture/release, where present, is explicitly through wallet-facing adapter seams;
* accounting export is metering, not balance truth;
* observability does not leak high-cardinality or authority-bearing data;
* media reads are bounded and canonical-b3 based.

## Key Doctrine Added / Enforced

The storage doctrine now makes the following clear:

* `svc-storage` remains content-addressed byte/object infrastructure.
* b3 hashes identify bytes only.
* b3 hashes are not payment proofs.
* b3 hashes are not wallet receipts.
* b3 hashes are not account balances.
* b3 hashes are not ledger commitments.
* b3 hashes are not QuickChain roots.
* b3 hashes are not finality proofs.
* b3 hashes are not bridge proofs.
* `crab://` navigation is not storage authority.
* cache entries are not paid-access authority.
* client-side references are not economic truth.
* storage must not unlock paid content from cache alone.
* paid estimate is quote-only.
* paid write requires proof.
* accounting export is usage/metering only.
* settlement behavior is opt-in and must go through wallet front-door seams.
* no fake balances.
* no fake receipts.
* no fake finality.
* no roots.
* no validators.
* no bridges.
* no external settlement.

## QuickChain Test Coverage Added / Verified

The focused QuickChain preflight suite for `svc-storage` consists of ten test targets:

1. `quickchain_preflight_b3_integrity`
2. `quickchain_preflight_boundary`
3. `quickchain_preflight_docs`
4. `quickchain_preflight_economics_quote`
5. `quickchain_preflight_no_direct_mutation`
6. `quickchain_preflight_observability`
7. `quickchain_preflight_paid_cache`
8. `quickchain_preflight_range_media`
9. `quickchain_preflight_settlement_boundary`
10. `quickchain_tooling_boundary`

The exhaustive preflight gate dynamically discovers and runs every `quickchain*.rs` test target.

### `quickchain_preflight_b3_integrity`

Purpose:

* Confirms object ingest derives canonical b3 from actual bytes.
* Confirms callers cannot retrieve bytes under fake or noncanonical CIDs.
* Protects the fundamental storage invariant: b3 hash truth is byte truth.

Boundary protected:

A b3 hash means “these bytes,” not “this payment,” “this balance,” “this proof,” or “this root.”

### `quickchain_preflight_boundary`

Purpose:

* Confirms the router exposes storage routes, not QuickChain authority routes.
* Confirms public response shapes do not claim balance, receipt root, state root, checkpoint, validator, bridge, settlement, or finality truth.

Protected principle:

Storage responses can describe storage admission and byte retrieval. They must not pretend to be wallet/ledger/QuickChain outputs.

### `quickchain_preflight_docs`

Purpose:

* Ensures the crate has explicit written QuickChain Phase-0 doctrine.
* Confirms the docs contain RO headers and a complete test contract.
* Confirms docs state storage’s plain boundary phrases.

Important docs content includes:

* `svc-storage remains content-addressed byte/object infrastructure`
* `b3 hashes identify bytes only`
* `svc-wallet = economic mutation front-door`
* `ron-ledger = durable replayable truth`
* `cache must not decide paid access by itself`
* `no fake balances`
* `no fake receipts`
* `no roots`
* `no validators`
* `no bridges`
* `no external settlement`

The “plain scanner boundary phrases” section was added because tests and safety scanners need exact plain text. This is intentional and should not be removed casually.

### `quickchain_preflight_economics_quote`

Purpose:

* Confirms paid estimate is quote-only.
* Confirms pricing uses integer minor units.
* Confirms checked-in ROC economics policy can quote paid storage without wallet or ledger mutation.
* Confirms quote/economics sources do not smuggle mutation or chain authority.

Protected principle:

A quote is not a hold.

A quote is not a capture.

A quote is not a receipt.

A quote is not finality.

A quote is not ledger mutation.

### `quickchain_preflight_no_direct_mutation`

Purpose:

* Confirms production dependencies do not include wallet or ledger mutation crates in the wrong way.
* Confirms storage router does not expose wallet or ledger mutation endpoints.
* Confirms accounting export is metering, not balance truth.
* Confirms wallet capture/release appears only inside explicit settlement adapter code.

Protected boundary:

`svc-storage` may verify/admit bytes under a paid proof and may call explicit wallet-facing settlement adapters where configured, but it must not become wallet or ledger itself.

### `quickchain_preflight_observability`

Purpose:

* Confirms metrics source keeps labels low-cardinality and non-authoritative.
* Confirms metrics do not expose CIDs, wallet receipts, accounts, balances, roots, validators, bridge data, anchors, or finality claims.

Protected principle:

Observability is for operational health. It is not a public ledger, explorer, receipt database, chain authority, or account-balance API.

### `quickchain_preflight_paid_cache`

Purpose:

* Confirms fake cache or fake paid headers do not unlock absent objects.
* Confirms paid write rejects without backend-derived proof and does not cache bytes.
* Confirms dev-header paid write response is labeled as storage admission, not finality.

Protected principle:

Cache can speed up storage. Cache cannot authorize paid access.

Storage admission is not finality.

Dev headers are dev/test admission material, not production economic truth.

### `quickchain_preflight_range_media`

Purpose:

* Confirms read path serves only canonical b3 and bounded ranges.
* Confirms free object read routes do not claim paid access or QuickChain authority.
* Protects the media boundary.

Important for CrabLink/Tauri:

Large media must be bounded and honest. The storage path should support range/segment access and should not pipe full large media through inappropriate command results. Each rendition should own its own b3.

### `quickchain_preflight_settlement_boundary`

Purpose:

* Confirms settlement plan rejects overcapture, zero capture, and escrow self-payee.
* Confirms settlement plan is integer-bounded and deterministic without roots or finality.
* Confirms settlement source uses wallet front-door only and no chain authority.

Protected principle:

Storage settlement is wallet-facing and bounded. It is not QuickChain finality, not bridge settlement, not external settlement, and not a root/checkpoint system.

### `quickchain_tooling_boundary`

Purpose:

* Confirms the storage preflight script is Bash/cargo-only.
* Confirms no Python helper files are checked into `svc-storage`.
* Confirms the full gate is preserved.
* Confirms the script dynamically discovers all `quickchain*.rs` targets.
* Confirms the park script delegates to the exhaustive preflight gate.

This protects the workflow from stale hardcoded lists and accidental tool drift.

## Scripts Added / Validated

### `crates/svc-storage/scripts/dev-quickchain-preflight.sh`

Purpose:

Runs the exhaustive QuickChain preflight gate for `svc-storage`.

The script verifies:

* docs exist;
* no checked-in Python helper files under `crates/svc-storage`;
* formatting is clean with `cargo fmt -p svc-storage -- --check`;
* focused QuickChain tests are discovered dynamically from `crates/svc-storage/tests/quickchain*.rs`;
* every discovered focused test is run;
* `cargo test -p svc-storage --all-targets` passes;
* `cargo clippy -p svc-storage --all-targets -- -D warnings` passes;
* forbidden-scope marker is printed;
* final dynamic test-count marker is printed.

Expected final marker:

`== svc-storage quickchain exhaustive preflight gate passed: tests=10 ==`

This marker has now been observed.

### `crates/svc-storage/scripts/dev-quickchain-park.sh`

Purpose:

Parking gate for the crate.

It validates required files exist and delegates to:

`crates/svc-storage/scripts/dev-quickchain-preflight.sh`

Expected final marker:

`== svc-storage QuickChain parking gate passed ==`

This marker has now been observed.

## Code/Architecture Areas Now Covered

### B3 Integrity

Storage derives b3 from bytes.

The canonical form is:

`b3:<64 lowercase hex>`

Storage must reject fake, malformed, or noncanonical CIDs where appropriate.

This protects content addressing and keeps b3 as truth for bytes only.

### Free CAS Object Routes

Free/dev object routes are still available for basic storage operations.

These routes must not claim paid unlock, wallet receipt, ledger truth, state root, receipt root, checkpoint, validator approval, bridge settlement, or finality.

### Paid Estimate

The paid estimate route is read-only.

It can compute side-effect-free pricing.

It must not:

* create a wallet hold;
* capture funds;
* release funds;
* mutate ledger;
* store bytes;
* export accounting events;
* claim a receipt;
* claim finality.

### Paid Write Admission

Paid write requires backend-derived proof.

The current architecture supports dev-header and wallet-receipt style verification seams, with explicit modes.

The doctrine is:

* dev-header mode is dev/test admission only;
* wallet-receipt mode is production-shaped;
* disabled mode fails closed;
* paid writes without proof reject and do not store bytes;
* fake headers cannot unlock absent objects;
* paid responses must be labeled as storage admission, not finality.

### Wallet Receipt Verifier

The storage tests cover wallet receipt contract behavior, including rejection of:

* wrong operation;
* wrong asset;
* wrong payer;
* wrong escrow;
* wrong amount;
* zero/non-integer amount;
* bad receipt hash;
* missing receipt;
* missing payer/escrow;
* malformed proof material.

This supports paid storage without allowing storage to invent receipts.

### Settlement Adapter Boundary

Storage contains explicit paid-storage settlement planning/adapter seams.

This is not direct ledger mutation.

Important boundaries:

* settlement mode is explicit;
* default is safe/disabled where applicable;
* wallet capture/release goes through wallet-facing HTTP client seams;
* overcapture rejects;
* zero capture rejects;
* escrow self-payee rejects;
* settlement plan is deterministic and integer-bounded;
* settlement does not claim roots/finality.

This is allowed as an explicit backend wallet path, not QuickChain authority.

### Accounting Export

Storage may export usage events to accounting.

Accounting export is metering/reporting, not balance truth.

Export failure does not become ledger mutation.

Usage events should remain bounded and not include secret or authority-bearing data.

Important exported concepts:

* bytes stored;
* request success;
* optional pin seconds;
* tenant/subject/region/route metadata;
* deterministic idempotency for accounting export batch.

The export must not carry:

* private keys;
* wallet secrets;
* full object body bytes;
* balances;
* chain roots;
* finality claims.

### Observability

Storage metrics must remain low-cardinality and non-authoritative.

Metrics must not expose:

* CIDs;
* accounts;
* wallet receipts;
* balances;
* roots;
* validators;
* bridges;
* anchors;
* finality claims.

Metrics can report operational status, accepted/rejected counts, byte totals, accounting export status, and similar bounded labels.

### Range Media

Storage supports bounded range media reads.

This aligns with CrabLink/Tauri media doctrine:

* no full-file large media through command-result style paths;
* prefer range/segment access;
* each rendition owns its own b3;
* no DRM or anti-rip claims;
* b3 verifies bytes;
* cache cannot unlock paid content alone.

## `svc-storage` Validation Status

The following have now passed:

* `quickchain_preflight_docs`
* `quickchain_tooling_boundary`
* all ten focused QuickChain preflight targets
* `cargo test -p svc-storage --all-targets`
* `cargo clippy -p svc-storage --all-targets -- -D warnings`
* `crates/svc-storage/scripts/dev-quickchain-preflight.sh`
* `crates/svc-storage/scripts/dev-quickchain-park.sh`

The final exhaustive marker was observed:

`== svc-storage quickchain exhaustive preflight gate passed: tests=10 ==`

The final parking marker was observed:

`== svc-storage QuickChain parking gate passed ==`

Current state:

`svc-storage` is parked for the current QuickChain Phase-0 sweep.

## What Remains for `svc-storage`

The crate is parked for Phase-0/preflight, but not “done forever.”

Future work should remain carefully bounded.

### Near-Term Remaining Work

1. Keep paid storage in the wallet-front-door path.

Any storage payment path should continue to use backend wallet/ledger truth.

2. Tighten wallet receipt production mode.

The wallet-receipt mode should remain the production-shaped path. Future hardening can improve auth, timeout behavior, replay protection, receipt lookup, and error classification.

3. Keep dev-header mode explicitly dev/test only.

Do not allow dev-header proof to become production economic truth.

4. Keep accounting export as metering.

If accounting ingestion evolves, storage should export usage/metering events only. It should not emit balances or mutate accounting truth.

5. Keep range/media behavior bounded.

As CrabLink Tauri media grows, storage should support safe segment/range access and content-b3 verification without pretending to provide DRM or uncopyable media.

6. Keep docs scanner phrases.

The plain scanner boundary phrases should remain unless the corresponding tests are intentionally updated.

### Long-Term Remaining Work

1. No root production in storage.

If future QuickChain roots are created, storage should not be the root-producing engine. It can store artifacts by b3, but root production belongs in approved deterministic QuickChain/ledger/proof components after canonical vectors are locked.

2. No validator behavior.

Storage nodes may eventually participate in availability, retrieval, or proof systems, but `svc-storage` itself should not quietly become a validator runtime.

3. No bridge/external settlement behavior.

Storage must not grow Solana/ROX/bridge/external-settlement code during the internal ROC proving phase.

4. No cache-only paid unlock.

Even if offline cache improves, paid content unlock must remain backend-derived. Cache can verify bytes and support UX, but cannot replace wallet/ledger truth.

5. No public finality claims.

Storage admission is not finality. Object availability is not consensus. b3 byte truth is not QuickChain finality.

---

# Pair-Level Summary — `svc-rewarder + svc-storage`

## What This Pair Achieved

Together, these two crates now enforce a critical middle section of the RustyOnions internal ROC value loop.

The intended value loop is:

`ron-proto econ DTOs -> ron-ledger truth -> svc-wallet issue/transfer/burn/hold/capture/release/receipt -> svc-storage/svc-gateway/omnigate paid enforcement -> ron-accounting snapshots -> svc-rewarder payout planning -> wallet/ledger receipts`

This pair specifically covers:

* storage paid admission and metering;
* storage quote-only economics;
* storage wallet-front-door settlement boundary;
* storage b3 integrity;
* storage cache boundary;
* storage range media boundary;
* storage observability boundary;
* rewarder deterministic payout planning;
* rewarder funding-source discipline;
* rewarder raw-engagement boundary;
* rewarder replay/no-double-issue discipline;
* rewarder no-direct-mutation boundary;
* rewarder tooling and docs boundary.

## Why This Pair Matters

This pair is important because it is where accidental inflation or fake economic authority could easily creep in.

Dangerous failure modes prevented:

* storage accepts fake paid headers and stores/unlocks content;
* storage cache unlocks paid content by itself;
* storage response pretends to be finality;
* storage treats b3 as payment proof;
* storage metrics leak wallet receipts/accounts/CIDs/roots;
* rewarder mints directly from raw views/clicks/engagement;
* rewarder double-issues on replay;
* rewarder directly mutates ledger;
* rewarder treats accounting counters as balance truth;
* rewarder invents protocol pool payouts without policy/funding provenance;
* either crate starts exposing roots, validators, bridges, settlement anchors, staking, liquidity, or external-chain logic.

## Gates Now Green

### `svc-rewarder`

Focused QuickChain preflight gate passed with seven focused QuickChain test targets.

The crate is parked for current Phase-0/preflight purposes.

### `svc-storage`

Focused QuickChain preflight gate passed with ten focused QuickChain test targets.

All-targets tests passed.

Clippy with `-D warnings` passed.

Parking gate passed.

The crate is parked for current Phase-0/preflight purposes.

## Regenerated Codebundles

The current working codebundle regeneration command format is:

`bash scripts/make_crate_codex.sh --force -c svc-rewarder`

`bash scripts/make_crate_codex.sh --force -c svc-storage`

The regenerated crate codebundles are now current after the parking gate.

## Current Pair Completion Estimate

For this specific pair’s current QuickChain Phase-0/preflight scope:

* `svc-rewarder`: approximately 95–100% parked for this sweep.
* `svc-storage`: approximately 95–100% parked for this sweep.
* Pair-level Phase-0/preflight status: effectively parked.

This does not mean the full QuickChain blueprint is complete.

It means this crate pair has completed the current preflight boundary sweep.

## What This Does Not Complete

This work does not complete:

* canonical locked hash vectors;
* state root engine;
* receipt root engine;
* accounting root engine;
* reward root engine;
* checkpoint production;
* validator sets;
* committee consensus;
* data availability/challenge/pruning;
* external anchors;
* Solana/ROX/bridge integration;
* public settlement;
* CrabLink chain authority;
* gateway/omnigate full paid enforcement sweep;
* final QuickChain beta runtime.

Those remain later phases and should remain gated.

## Next Crate Pair

The next planned crate pair is:

1. `svc-gateway`
2. `omnigate`

Reason:

After wallet/accounting/rewarder/storage boundaries, the next risk surface is paid enforcement and hydration at the public/client-facing boundary.

The next sweep should verify that:

* `svc-gateway` remains public boundary, not ledger/wallet/root authority;
* `omnigate` hydrates/enforces access without mutating ledger truth directly;
* neither service invents balances/receipts;
* neither service unlocks paid content from cache alone;
* both keep wallet/ledger receipts backend-derived/display-only;
* both remain QuickChain-aware only as future/parked doctrine, not runtime chain authority;
* both preserve Tauri/CrabLink paid-flow doctrine:
  prepare/quote -> explicit confirmation -> backend wallet path -> backend receipt -> unlock/render -> display-only receipt cache -> balance refresh.

## Carry-Forward Warnings

Future sessions should not undo these boundaries.

Do not add to `svc-rewarder` or `svc-storage`:

* public chain state;
* roots;
* checkpoints;
* validators;
* consensus;
* staking;
* liquidity;
* bridge code;
* Solana code;
* ROX code;
* external settlement;
* public anchor mutation;
* exchange-facing logic;
* hidden ledger mutation;
* fake balances;
* fake receipts;
* fake finality;
* cache-only paid unlock;
* raw engagement protocol payouts;
* DB-order roots;
* wall-clock roots;
* placeholder hashes;
* fake golden vectors.

## Safe Future Additions

Safe future additions may include:

* more docs hardening;
* more preflight tests;
* stronger policy validation;
* stronger wallet receipt verification;
* better idempotency/replay tests;
* better bounded media/range tests;
* better accounting export tests;
* better metrics label tests;
* stricter DTO deny-unknown-field checks;
* stricter integer minor-unit checks;
* canonical byte vector sketches;
* locked canonical bytes only after schemas are stable;
* locked hashes only after canonical bytes/domain separators/preimage framing are approved.

## Final Current State

`svc-rewarder + svc-storage` are now parked for the current QuickChain Phase-0/preflight sweep.

The internal ROC proof path is stronger because this pair now protects two major risk surfaces:

1. reward planning must not become unauthorized minting or direct ledger mutation;
2. storage/paid bytes must not become fake paid access, fake receipts, fake finality, or hidden settlement authority.

Next session can safely move forward into `svc-gateway + omnigate` with these boundaries established behind it.


### END NOTE - JUNE 17 2026 - 13:10 CST


### BEGIN NOTE - JUNE 19 2026 - 14:40 CST

The terminal output confirms both crates are now parkable: `svc-rewarder` passed its exhaustive gate with 8 QuickChain tests, and `svc-storage` passed its exhaustive gate with 11 QuickChain tests, including all-targets and clippy for both crates. 

Here are the paste-ready notes.

# QuickChain Phase-0 Notes — `svc-rewarder` + `svc-storage`

Date: 2026-06-19
Scope: QuickChain Phase-0 / QC-0A safety, preflight, and value-loop boundary hardening
Crates covered:

* `crates/svc-rewarder`
* `crates/svc-storage`

## Executive Summary

This session completed the QuickChain Phase-0 boundary hardening pass for the `svc-rewarder + svc-storage` crate pair.

The work did **not** add QuickChain roots, checkpoints, validators, bridge logic, staking, liquidity, public anchors, ROX, Solana, external settlement, public-chain authority, or chain-runtime behavior.

Instead, the changes locked both crates into their correct internal RustyOnions value-plane roles:

```text
svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

The key architectural result is that `svc-storage` remains byte/object infrastructure and metering input, while `svc-rewarder` remains deterministic payout planning only. Neither crate is allowed to become balance truth, receipt truth, ledger mutation authority, root authority, checkpoint authority, validator authority, bridge authority, external settlement authority, or finality authority.

Both crates now have explicit docs and tests enforcing these boundaries.

---

# 1. `svc-rewarder` Changes

## 1.1 Updated QuickChain preflight documentation

Updated:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

The document now explicitly records the crate’s QuickChain Phase-0 posture.

`svc-rewarder` is documented as:

```text
a deterministic ROC payout planner
```

It is explicitly documented as **not** being:

```text
a chain runtime
a validator
a bridge
a checkpoint writer
a root producer
a ledger mutation authority
wallet authority
balance truth
receipt truth
settlement finality
```

The document also names the two most important economic authority boundaries:

```text
svc-wallet is the mutation front-door
ron-ledger is durable economic truth
```

This makes clear that `svc-rewarder` may plan payouts, but it must not mutate balances or create authoritative receipts itself.

## 1.2 Locked `svc-rewarder` into the internal value loop

The docs now state the required internal value loop:

```text
svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

This matters because QuickChain must not drift into a design where reward planning becomes a silent money authority.

`svc-rewarder` is allowed to produce deterministic planning artifacts and wallet issue request planning payloads, but it must not bypass `svc-wallet`, must not treat planning artifacts as settlement finality, and must not treat funding provenance as finality.

The final phrase was made explicit for scanner compatibility:

```text
funding provenance is not settlement finality
```

That phrase is intentionally present because the docs test scans for it literally.

## 1.3 Documented allowed Phase-0 scope

The `svc-rewarder` docs now list allowed Phase-0 work:

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

This keeps the crate aligned with the QuickChain doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

For this crate, the only allowed “economic output” is planning and wallet-handoff DTO shape. The authoritative mutation path remains downstream in `svc-wallet` and `ron-ledger`.

## 1.4 Documented forbidden Phase-0 scope

The `svc-rewarder` docs now explicitly forbid:

```text
root-producing code
checkpoint-producing code
validator code
bridge or external settlement code
direct ledger mutation
direct wallet mutation outside explicit svc-wallet handoff
fake balances
fake receipts
fake finality
Solana
ROX
public bridge
external anchors
staking or liquidity
exchange-facing logic
```

The future parked work is also documented as out of scope until the proper prerequisites exist:

```text
canonical bytes and locked vectors
state/account Merkle roots
receipt roots
validator-set logic
checkpoint signing
external DA
public anchors
bridges
staking or liquidity
CrabLink chain authority
gateway/omnigate/rewarder ledger mutation
```

This is important because `svc-rewarder` is near the value loop and could otherwise become a tempting place to add payout authority too early. The docs now make that drift visibly forbidden.

## 1.5 Raw engagement boundary strengthened

The docs now clearly state that raw engagement fields must not become direct protocol ROC payout authority.

Examples documented as forbidden direct payout inputs include:

```text
raw views
raw watch seconds
raw clicks
raw impressions
likes
shares
follows
views-to-ROC formulas
watch-seconds-to-ROC formulas
```

Raw usage may still feed accounting, fraud analysis, or policy inputs after classification and validation, but raw engagement cannot directly mint, issue, allocate, or authorize protocol ROC.

This protects the internal economy from bot-farm style incentives and ensures payout authority remains policy/accounting/wallet/ledger mediated.

## 1.6 Replay and identity boundary documented

The docs now distinguish between retry keys and ledger identity:

```text
idempotency keys are replay/dedupe tools
idempotency keys are not ledger operation identity
idempotency keys are not validator consensus
idempotency keys are not settlement authority
```

This matters because QuickChain’s future event model distinguishes:

```text
operation_id      = backend-assigned durable ledger-operation identity
idempotency_key   = retry/dedupe key
account_sequence  = ledger-assigned sequence
hold_id           = one hold lifecycle identifier
```

For `svc-rewarder`, this means deterministic replay protection can exist without pretending to own durable ledger operation identity.

## 1.7 Added pair-level value-loop boundary test

Added:

```text
crates/svc-rewarder/tests/quickchain_preflight_value_loop_boundary.rs
```

This test is a source/docs boundary test. It does not create chain state. It does not create roots. It does not create checkpoints. It does not create validator logic.

It enforces that the rewarder docs name the correct value-loop sequence and that the source code remains in the proper role.

The test checks the docs for these required phrases:

```text
svc-storage/svc-gateway/omnigate paid enforcement
ron-accounting snapshots
svc-rewarder payout planning
explicit approved payout intent
svc-wallet
ron-ledger
deterministic roc payout planner
svc-wallet is the mutation front-door
ron-ledger is durable economic truth
```

The goal is to make the correct architecture machine-checked, not merely implied.

## 1.8 Settlement intent source boundary locked

The new rewarder test checks:

```text
src/outputs/intents.rs
```

It requires wallet-handoff DTO markers such as:

```text
SettlementIntent
SettlementBatch
WalletIssueRequest
WALLET_ISSUE_PATH
to_wallet_issue_request
#[serde(deny_unknown_fields)]
```

It also rejects signs that settlement intents are turning into receipt/root/finality authority.

Forbidden markers include:

```text
WalletReceipt
receipt_hash
ledger_root
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
finality
external_anchor
bridge_txid
```

This preserves the important distinction:

```text
rewarder settlement intent = wallet handoff planning DTO
wallet receipt             = backend wallet/ledger-derived truth
```

`svc-rewarder` must not generate wallet receipts itself.

## 1.9 Wallet client boundary locked

The new rewarder test checks:

```text
src/outputs/wallet.rs
```

It requires explicit wallet boundary markers:

```text
svc-wallet
preview_issue_batch
emit_issue_batch
dry_run
WalletHttpIssueOutcome
```

It also rejects direct ledger/chain authority markers such as:

```text
ron_ledger::
LedgerClient
ledger_commit
checkpoint_hash
validator_signature
state_root
receipt_root
external_anchor
bridge_txid
```

This protects the intended shape:

```text
rewarder -> svc-wallet -> ron-ledger
```

and prevents the accidental shape:

```text
rewarder -> ron-ledger
```

The rewarder may call wallet-facing seams. It must not directly commit to the ledger.

## 1.10 Core reward planning source scan added

The new rewarder test scans important reward planning files:

```text
src/core/compute.rs
src/inputs/accounting.rs
src/outputs/manifest.rs
src/outputs/intents.rs
```

It verifies they do not gain QuickChain runtime authority fields:

```text
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
settlement_finality
external_anchor
bridge_txid
staking
liquidity
```

This is a drift detector. If a future patch tries to put Phase-1+ chain authority into the rewarder prematurely, the test should fail.

## 1.11 Final `svc-rewarder` gate result

The focused docs failure from the missing literal phrase was fixed.

Final verified `svc-rewarder` status:

```text
cargo test -p svc-rewarder --test quickchain_preflight_docs
cargo test -p svc-rewarder --test quickchain_preflight_value_loop_boundary
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Final result:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=8
```

The exhaustive gate included:

```text
format check
forbidden Python helper scan
dynamic focused QuickChain test discovery
8 focused QuickChain tests
svc-rewarder all-targets test
svc-rewarder clippy --all-targets -- -D warnings
forbidden-scope marker
```

The crate is now parkable for this QuickChain Phase-0 pass.

---

# 2. `svc-storage` Changes

## 2.1 Updated QuickChain preflight documentation

Updated:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

The document now explicitly records that `svc-storage` remains:

```text
content-addressed byte/object infrastructure
```

It also explicitly states:

```text
b3 hashes identify bytes only
```

The docs clarify that b3 hashes are:

```text
content truth only
not payment proof
not receipt roots
not account state roots
not checkpoint roots
not settlement finality
```

This is one of the most important storage-side boundaries for QuickChain. A hash proves bytes, not payment, authorization, finality, or ledger truth.

## 2.2 Locked `svc-storage` into the internal value loop

The docs now state the storage-side value loop:

```text
svc-storage paid admission and b3 byte integrity
-> storage/access metering
-> ron-accounting derivative snapshots
-> svc-rewarder deterministic payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

This establishes `svc-storage` as the front part of the paid-content and metering path, not as an economic truth source.

`svc-storage` may:

```text
store bytes
serve bytes
serve bounded ranges
verify b3
price/quote paid writes
verify backend wallet hold evidence
capture/release through configured wallet settlement path
emit usage events
export usage events to accounting
```

But `svc-storage` must not:

```text
mutate ledger directly
invent balances
invent receipts
claim finality
produce QuickChain roots
act as a validator
act as a bridge
act as settlement truth
turn raw metering into protocol payout authority
```

The distinction is subtle but critical: storage may have paid admission and wallet-facing settlement adapters, but it is not settlement authority and does not own ledger truth.

## 2.3 Paid access and cache boundary documented

The docs now explicitly state:

```text
cache must not decide paid access by itself
```

Additional documented boundaries:

```text
cache can verify b3 before trusted render
cache cannot unlock paid content alone
offline cache verifies b3 before trusted render
paid content requires backend-derived authorization
paid content requires backend-derived receipt/authorization
receipt cache is display-only
a storage CID, manifest CID, or b3 hash is not payment proof
```

This aligns `svc-storage` with the broader CrabLink/Tauri doctrine:

```text
cache is convenience
backend-derived authorization unlocks paid access
b3 verifies bytes
wallet/ledger own economic truth
```

## 2.4 Metering boundary documented

The docs now make storage metering explicitly derivative:

```text
storage metering is derivative accounting input only
```

Usage events are documented as:

```text
not balance updates
not wallet receipts
not payout authority
not ledger mutation
```

Storage/access metering can feed `ron-accounting`, and accounting snapshots can later feed `svc-rewarder`, but storage metering itself does not create money movement.

This prevents a future mistake where raw storage usage could directly allocate ROC.

## 2.5 Bounded media boundary documented

The docs now include large-media constraints:

```text
large media must stay bounded and honest
range/segment serving is preferred for large media
full-file unbounded command/result paths are not allowed
each rendition owns its own b3
no DRM or anti-rip guarantee is made
```

This is important for CrabLink because media is central to the product layer, and `svc-storage` must remain honest about what it can guarantee.

Storage can prove and serve bytes. It cannot claim DRM, anti-rip protection, or magical paid-content security once bytes are legitimately delivered.

## 2.6 Forbidden Phase-0 scope documented

The `svc-storage` docs now explicitly forbid:

```text
fake balances
fake receipts
silent spend
roots
validators
bridges
external settlement
checkpoints
anchors
external anchors
bridge or external settlement authority
staking
liquidity
Solana
ROX
exchange-facing logic
root-producing code
checkpoint-producing code
validator code
```

This prevents the storage crate from becoming a stealth chain boundary.

`svc-storage` is allowed to perform byte storage, paid admission checks, wallet-hold verification, wallet capture/release through configured wallet paths, and accounting export. It is not allowed to become a QuickChain runtime component or finality source.

## 2.7 Added pair-level value-loop boundary test

Added:

```text
crates/svc-storage/tests/quickchain_preflight_value_loop_boundary.rs
```

This test enforces the storage side of the value loop through docs/source scanning.

It checks the docs for required phrases:

```text
svc-storage paid admission and b3 byte integrity
storage/access metering
ron-accounting derivative snapshots
svc-rewarder deterministic payout planning
explicit approved payout intent
svc-wallet
ron-ledger
b3 hashes identify bytes only
cache must not decide paid access by itself
storage metering is derivative accounting input only
```

This ensures the architectural role of storage remains visible and machine-checked.

## 2.8 Pricing and metering source boundary locked

The new storage test checks:

```text
src/policy/economics.rs
src/accounting/mod.rs
src/accounting/exporter.rs
```

It requires markers proving these paths are planning/metering only:

```text
PaidStoragePriceEstimate
side-effect free
does not call wallet, ledger
UsageEventDto
usage only; no balances; no ledger mutation
export failure never mutates ledger or wallet state
no wallet receipt/body bytes exported
```

This protects the quote and accounting-export paths from becoming mutation paths.

Important distinction:

```text
paid estimate = quote only
usage event   = metering only
accounting export = derivative reporting only
wallet/ledger = economic truth
```

## 2.9 Paid-write proof boundary locked

The new storage test checks:

```text
src/policy/paid_write.rs
```

It requires wallet-hold evidence markers:

```text
WalletReceipt
validate_as_paid_write_hold
PaidWriteProof
paid_storage_context_idem
wallet receipt hash must be b3:<64 lowercase hex>
paid proof must reference a wallet hold receipt
```

This keeps paid-write admission tied to backend wallet evidence.

The test also rejects QuickChain runtime/finality markers in paid-write verification:

```text
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
settlement_finality
external_anchor
bridge_txid
staking
liquidity
```

This protects the intended meaning:

```text
paid-write proof = wallet hold evidence for storage admission
paid-write proof != QuickChain finality
paid-write proof != receipt root
paid-write proof != validator proof
```

## 2.10 Wallet capture/release settlement seam locked

The new storage test checks:

```text
src/policy/settlement.rs
```

It requires explicit wallet-settlement seam markers:

```text
SETTLEMENT_MODE_WALLET_CAPTURE
PaidStorageSettlementPlan
WalletSettlementHttpClient
capture_idem
release_idem
failed_write_release_idem
```

This confirms that storage’s post-write settlement behavior is a wallet-facing adapter, not direct ledger mutation and not chain settlement.

The test rejects direct ledger/chain authority markers:

```text
ron_ledger::
LedgerClient
ledger_commit
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
external_anchor
bridge_txid
staking
liquidity
```

This protects the intended path:

```text
svc-storage -> svc-wallet capture/release path -> ron-ledger
```

and rejects the forbidden path:

```text
svc-storage -> ron-ledger direct commit
```

## 2.11 Storage value-loop source scan added

The new storage test scans:

```text
src/policy/economics.rs
src/policy/paid_write.rs
src/policy/settlement.rs
src/accounting/mod.rs
src/accounting/exporter.rs
```

It rejects QuickChain runtime authority fields:

```text
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
settlement_finality
external_anchor
bridge_txid
staking
liquidity
```

This is a future drift detector. If later code tries to push roots, validator signatures, anchors, staking, liquidity, or bridge IDs into storage’s economic path, the test should fail.

## 2.12 Final `svc-storage` gate result

Final verified `svc-storage` status:

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final result:

```text
svc-storage quickchain exhaustive preflight gate passed: tests=11
```

The exhaustive gate included:

```text
format check
forbidden Python helper scan
dynamic focused QuickChain test discovery
11 focused QuickChain tests
svc-storage all-targets test
svc-storage clippy --all-targets -- -D warnings
forbidden-scope marker
```

The crate is now parkable for this QuickChain Phase-0 pass.

---

# 3. Cross-Crate Value-Loop Result

This session’s main contribution was not just adding isolated tests. It locked the relationship between `svc-storage` and `svc-rewarder` inside the larger RustyOnions internal ROC economy.

The resulting internal flow is now documented and tested as:

```text
paid storage/content admission
-> bounded byte serving and b3 verification
-> usage/metering events
-> accounting snapshots
-> deterministic reward planning
-> explicit wallet issue/capture/release intents
-> svc-wallet mutation front-door
-> ron-ledger durable economic truth
```

No crate in this pair is allowed to skip forward and become ledger truth.

## Correct role split

### `svc-storage`

Owns:

```text
bytes
CAS
b3 verification
bounded range serving
paid-write admission
wallet-hold proof verification
wallet capture/release adapter
usage event generation
accounting export
```

Does not own:

```text
balances
receipts
ledger mutation
QuickChain roots
checkpoint roots
validator signatures
bridge state
external settlement
staking
liquidity
finality
```

### `svc-rewarder`

Owns:

```text
deterministic payout planning
policy-aware reward calculation
funding provenance display
wallet issue request planning
idempotent preview/egress behavior
manifest planning artifacts
```

Does not own:

```text
wallet receipts
balance truth
ledger operation truth
direct ledger commits
QuickChain roots
checkpoint roots
validator signatures
external anchors
bridges
staking
liquidity
settlement finality
```

### `svc-wallet`

Still owns:

```text
economic mutation front-door
issue
transfer
burn
hold
capture
release
receipt generation path
```

### `ron-ledger`

Still owns:

```text
durable economic truth
ledger replay
accepted operation history
balance truth
operation identity
```

---

# 4. Why These Changes Matter

These changes protect the project against the most dangerous QuickChain Phase-0 failure mode: accidentally adding chain authority before the internal ROC economy is proven.

Without these tests, it would be easy for future code to drift toward one of these unsafe shapes:

```text
svc-storage creates payment truth from b3 or cache state
svc-storage treats paid-write headers as finality
svc-storage mutates ledger directly
svc-rewarder creates receipts
svc-rewarder treats funding provenance as finality
svc-rewarder emits direct ledger commits
svc-rewarder turns raw engagement into protocol ROC
either crate starts carrying roots/checkpoints/validators/anchors
```

The new docs/tests make those mistakes loud.

This keeps the current mission intact:

```text
prove ROC as a fully internal token/accounting/value plane first
```

No ROX.
No Solana.
No external settlement.
No public bridge.
No staking.
No liquidity.
No exchange-facing logic.
No public-chain authority.
No fake balances.
No fake receipts.
No silent spend.

---

# 5. Tests Added or Strengthened

## `svc-rewarder`

Added:

```text
crates/svc-rewarder/tests/quickchain_preflight_value_loop_boundary.rs
```

Strengthened by docs update:

```text
crates/svc-rewarder/tests/quickchain_preflight_docs.rs
```

Focused QuickChain suite now includes:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Final focused count:

```text
8 focused QuickChain tests
```

## `svc-storage`

Added:

```text
crates/svc-storage/tests/quickchain_preflight_value_loop_boundary.rs
```

Focused QuickChain suite now includes:

```text
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Final focused count:

```text
11 focused QuickChain tests
```

---

# 6. Final Verification Commands Run

## `svc-rewarder`

```text
cargo fmt -p svc-rewarder
cargo test -p svc-rewarder --test quickchain_preflight_docs
cargo test -p svc-rewarder --test quickchain_preflight_value_loop_boundary
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Final gate result:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=8
```

## `svc-storage`

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final gate result:

```text
svc-storage quickchain exhaustive preflight gate passed: tests=11
```

Both crates passed:

```text
cargo fmt check
focused QuickChain tests
all-targets tests
clippy --all-targets -- -D warnings
forbidden helper scan
forbidden scope marker
```

---

# 7. What Remains for These Crates Later

These crates are parkable for this Phase-0 pass, but future work remains once QuickChain advances beyond preflight/boundary hardening.

## Future `svc-rewarder` work

Later, after canonical bytes and locked vectors exist, possible future work may include:

```text
stronger canonical payout vector alignment
formal reward manifest vector fixtures
richer accounting snapshot validation
more interop vectors with ron-accounting
production wallet issue egress hardening
signed policy enforcement improvements
audit/event export hardening
reward planning replay corpus
```

Still forbidden until the correct phase gates are green:

```text
root-producing reward manifests
receipt roots
state roots
validator participation
checkpoint signing
external anchors
bridge IDs
staking
liquidity
direct ledger mutation
raw engagement protocol payouts
```

## Future `svc-storage` work

Later, after internal ROC paid access is more fully proven, possible future work may include:

```text
stronger wallet receipt lookup hardening
paid range-access policy hardening
storage accounting export interop vectors
more paid-write fraud/failure-mode tests
better bounded-media transport proofs
cache verification proofs
offline cache UX rules
gateway/omnigate integration regression tests
```

Still forbidden until the correct phase gates are green:

```text
cache-only paid unlock
b3-as-payment-proof
storage-generated receipts
storage-generated roots
storage-generated finality
direct ledger mutation
validator signatures
bridge settlement
external anchors
staking
liquidity
```

---

# 8. Parked Status

`svc-rewarder` is parkable for this QuickChain Phase-0 pass.

`svc-storage` is parkable for this QuickChain Phase-0 pass.

This pair can now be treated as complete for the current boundary-hardening sweep, subject to final regenerated codebundles being saved after the green run.

Recommended regeneration command:

```text
bash scripts/make_crate_codex.sh --force -c svc-rewarder
bash scripts/make_crate_codex.sh --force -c svc-storage
```

Next crate pair in the coherent QuickChain build order:

```text
svc-gateway + omnigate
```

The next pair should continue the same doctrine:

```text
backend owns truth
wallet mutates
ledger is durable truth
accounting is derivative
rewarder plans
storage serves bytes
gateway/omnigate enforce paid hydration without becoming ledger, wallet, root, checkpoint, validator, bridge, or finality authority
```


### END NOTE - JUNE 19 2026 - 14:40 CST




### BEGIN NOTE - JUNE 20 2026 - 11:30 CST

The latest terminal output confirms the pair is green: `svc-rewarder` parked with **9 focused QuickChain tests**, and `svc-storage` parked with **12 focused QuickChain tests**. The new source-authority scan tests also passed for both crates before the park scripts ran. 

Drop these into the crate-local notes.

### BEGIN NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-rewarder

# svc-rewarder QuickChain Phase-0 / QC-1A Notes

## Current Status

```text
svc-rewarder: GREEN / QuickChain Phase-0 boundary parked / QC-1A foundation slice complete for this pass
```

`svc-rewarder` is now parked for the current QuickChain Phase-0 / QC-1A boundary sweep.

The crate’s QuickChain posture is:

```text
svc-rewarder plans deterministic ROC payout batches.
svc-rewarder does not mutate balances.
svc-rewarder does not mint, issue, transfer, burn, hold, capture, or release ROC.
svc-rewarder does not produce wallet receipts.
svc-rewarder does not produce QuickChain roots.
svc-rewarder does not claim finality or settlement.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
QuickChain remains future verification/settlement infrastructure.
```

This is exactly the correct position for `svc-rewarder` inside the internal ROC value loop.

The crate now proves that reward planning is deterministic, funding-source-gated, raw-engagement-safe, replay-safe, and routed toward wallet/ledger authority instead of becoming authority itself.

---

## Intended Role

`svc-rewarder` is a payout planning service.

It may:

```text
- consume normalized accounting snapshots
- consume approved reward policy/config
- consume explicit funding-source information
- compute deterministic reward manifests
- compute deterministic settlement/payout intent batches
- create wallet issue request shapes for handoff
- expose preview/dry-run behavior
- enforce idempotency at the planning layer
- preserve deterministic ordering of recipients/intents
- quarantine invalid, unsafe, or over-budget planning inputs
- emit metrics about planned intents
```

It must not:

```text
- mutate wallet state directly
- mutate ledger state directly
- mint/issue/transfer/burn/hold/capture/release ROC directly
- produce backend wallet receipts
- claim balance truth
- claim receipt truth
- claim operation truth
- claim account_sequence truth
- treat idempotency_key as operation authority
- treat reward planning as finality
- treat accounting snapshots as balance truth
- treat raw engagement as protocol payout authority
- expose QuickChain roots
- expose checkpoint authority
- expose validator behavior
- expose settlement authority
- expose external anchors
- expose public bridge behavior
- expose staking or liquidity logic
- expose Solana/ROX/external settlement paths
```

The short doctrine is:

```text
ron-accounting measures.
svc-rewarder plans.
svc-wallet mutates.
ron-ledger is truth.
QuickChain later verifies roots/proofs.
```

---

## Why This Crate Matters

`svc-rewarder` is one of the highest-risk crates for accidental economic authority creep.

Without strong boundaries, the dangerous shape would be:

```text
raw views/clicks/engagement
-> rewarder computes payout
-> rewarder directly mutates ledger
-> rewarder emits receipt-looking object
-> gateway/omnigate/CrabLink treat it as spend authority
```

That shape is forbidden.

The safe shape is:

```text
ron-accounting snapshot
-> svc-rewarder deterministic planning
-> explicit funding-source policy
-> wallet issue request / settlement intent handoff
-> svc-wallet mutation path
-> ron-ledger durable truth
-> backend-derived receipt
```

This crate’s current test/documentation posture makes the dangerous shape difficult to introduce accidentally.

---

## Files Added or Hardened

### Documentation

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

Purpose:

```text
- Defines svc-rewarder as deterministic payout planning only.
- States svc-wallet is the mutation front-door.
- States ron-ledger is durable economic truth.
- States ron-accounting snapshots are planning inputs, not balance truth.
- Names the allowed Phase-0 scope.
- Names the forbidden QuickChain/runtime scope.
- Keeps future roots/checkpoints/validators/settlement parked outside the crate.
- Documents funding-source, raw-engagement, replay, value-loop, and no-direct-mutation boundaries.
```

### Scripts

```text
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The preflight script now:

```text
- uses Bash strict mode
- runs from repo root
- verifies quickchain-preflight docs exist
- rejects checked-in Python helper drift under the crate
- runs cargo fmt check
- dynamically discovers every tests/quickchain*.rs target
- runs every discovered focused QuickChain test target
- runs cargo test -p svc-rewarder --all-targets
- runs cargo clippy -p svc-rewarder --all-targets -- -D warnings
- prints the forbidden-scope marker
- prints the final exhaustive preflight success marker
```

The park script now:

```text
- verifies the docs file exists
- verifies the preflight script exists
- verifies the tooling boundary test exists
- delegates to the exhaustive preflight script
- prints the final parking-gate success marker
```

Important low-disk workflow note:

```text
Do not run both dev-quickchain-preflight.sh and dev-quickchain-park.sh for the same crate unless deliberately requesting duplicate full passes.

The park script delegates to preflight.
```

### Focused QuickChain Tests

Current focused QuickChain targets:

```text
crates/svc-rewarder/tests/quickchain_preflight_boundary.rs
crates/svc-rewarder/tests/quickchain_preflight_docs.rs
crates/svc-rewarder/tests/quickchain_preflight_funding_source.rs
crates/svc-rewarder/tests/quickchain_preflight_no_direct_mutation.rs
crates/svc-rewarder/tests/quickchain_preflight_raw_engagement.rs
crates/svc-rewarder/tests/quickchain_preflight_replay_no_double_issue.rs
crates/svc-rewarder/tests/quickchain_preflight_source_authority_scan.rs
crates/svc-rewarder/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-rewarder/tests/quickchain_tooling_boundary.rs
```

This gives `svc-rewarder` nine focused QuickChain test targets in the current pass.

---

## What Each QuickChain Test Target Proves

### `quickchain_preflight_boundary.rs`

This target proves the outer rewarder DTO and response boundary.

It verifies:

```text
- JSON number money is rejected at the snapshot wire boundary.
- Reward manifests do not expose roots, receipts, balances, or finality.
- Wallet preview output remains an issue-request shape, not receipt or balance truth.
- Compute requests reject smuggled QuickChain authority fields.
```

This is important because rewarder output can look economic. These tests keep it as planning output only.

### `quickchain_preflight_docs.rs`

This target proves the documentation stays synchronized with the crate’s intended QuickChain posture.

It verifies that docs:

```text
- list every focused preflight suite
- state rewarder is planning-only and not chain runtime
- name allowed Phase-0 scope
- name forbidden Phase-0 scope
- keep future QuickChain work parked outside rewarder
- name raw-engagement, replay, and funding-source boundaries
```

This matters because the crate boundary is not only code-level. Future contributors need the doctrine written down locally.

### `quickchain_preflight_funding_source.rs`

This target proves payout planning cannot silently invent funding authority.

It verifies:

```text
- unsigned protocol-pool policy is rejected by validator
- policy requires explicit funding source on wire
- current policy accepts explicit protocol-pool provenance only when authority fields are not smuggled
- reward manifests carry funding provenance but not funding finality
- wallet previews carry batch provenance but remain wallet issue request shapes
- compute requests reject top-level funding-authority smuggling
```

This blocks a dangerous future where rewarder could act like a protocol mint.

### `quickchain_preflight_no_direct_mutation.rs`

This target proves rewarder does not expose or accept direct wallet/ledger mutation authority.

It verifies:

```text
- planning outputs do not claim receipts, balances, operation truth, roots, or finality
- config rejects external settlement, bridge, anchor, validator, and root knobs
- compute requests reject direct mutation authority smuggling
- router does not expose direct wallet, ledger, QuickChain, or bridge mutation routes
```

This keeps `svc-rewarder` from becoming a hidden wallet, ledger, or settlement service.

### `quickchain_preflight_raw_engagement.rs`

This target proves raw Web2-style engagement cannot directly become protocol payout authority.

It verifies:

```text
- allowed contribution counters are storage egress and uptime only
- reward policy rejects raw engagement formula fields
- accounting snapshot rejects raw engagement contribution fields
- compute request rejects top-level raw engagement payout fields
```

This is critical because fakeable metrics like views, clicks, likes, or visits must never directly mint or allocate protocol ROC.

### `quickchain_preflight_replay_no_double_issue.rs`

This target proves replay behavior remains deterministic and does not imply duplicate payout authority.

It verifies:

```text
- duplicate epoch replay is dedupe, not second payout authority
- idempotency keys are retry dedupe, not operation identity
- same snapshot, policy, and epoch produce same plan commitment
- reordered snapshot rows produce the same plan
```

This protects the reward loop from replay/double-issue bugs.

### `quickchain_preflight_source_authority_scan.rs`

This target was added in the current pass.

It proves the source tree does not quietly grow runtime authority or QuickChain implementation surfaces.

It verifies:

```text
- svc-rewarder source tree does not define QuickChain runtime modules
- svc-rewarder source does not contain direct chain or ledger authority tokens after stripping comments/string literals
```

This is a guardrail against future accidental imports, modules, or authority-shaped code paths.

The source scan is intentionally code-only: it strips comments and string literals before checking forbidden runtime tokens so docs/tests can still describe forbidden concepts.

### `quickchain_preflight_value_loop_boundary.rs`

This target proves rewarder stays in the correct place in the internal ROC value loop.

It verifies:

```text
- docs lock rewarder position in the internal value loop
- settlement intents are wallet handoff DTOs, not receipts or roots
- wallet client boundary targets wallet without direct ledger or chain authority
- core rewarder sources do not gain QuickChain runtime authority fields
```

This is the cross-crate boundary test that keeps rewarder from replacing wallet/ledger truth.

### `quickchain_tooling_boundary.rs`

This target proves the crate-local QuickChain tooling remains safe and repeatable.

It verifies:

```text
- park script delegates to exhaustive preflight
- preflight script is Bash/cargo-only and keeps the full gate
- preflight script discovers all QuickChain tests dynamically
- no Python helpers are checked into the rewarder crate
```

This supports the current project workflow and prevents stale manual test lists.

---

## Current Verified Gate Results

The final parking run verified:

```text
Focused QuickChain tests discovered: 9

quickchain_preflight_boundary.rs:              passed
quickchain_preflight_docs.rs:                  passed
quickchain_preflight_funding_source.rs:        passed
quickchain_preflight_no_direct_mutation.rs:    passed
quickchain_preflight_raw_engagement.rs:        passed
quickchain_preflight_replay_no_double_issue.rs: passed
quickchain_preflight_source_authority_scan.rs: passed
quickchain_preflight_value_loop_boundary.rs:   passed
quickchain_tooling_boundary.rs:                passed

cargo test -p svc-rewarder --all-targets:      passed
cargo clippy -p svc-rewarder --all-targets -- -D warnings: passed

Final marker:
== svc-rewarder quickchain exhaustive preflight gate passed: tests=9 ==
== svc-rewarder QuickChain parking gate passed ==
```

All-targets coverage included:

```text
src/lib.rs unit tests
src/main.rs tests
tests/integration.rs
tests/unit.rs
benches/reward_calc.rs
all focused quickchain*.rs targets
```

Important successful existing tests include:

```text
- rewarder settlement issues through wallet once without double issue
- settlement intent egress is idempotent by run key
- compute happy path and replay are deterministic
- settlement preview endpoint returns wallet issue batch
- metrics include planned settlement intents after compute
- dry run can promote to production without consuming run key
- accounting interop vector is consumable by rewarder snapshot DTO
- rewarder and accounting agree on canonical snapshot CID
- wallet issue request serializes amount as string
- wallet client dry run posts nothing
- wallet client posts issue requests to wallet route
```

---

## What svc-rewarder Now Proves

`svc-rewarder` now proves:

```text
- rewarder output is plan/intent only
- rewarder planning is deterministic for same input
- rewarder replay is dedupe/idempotency, not second payout authority
- idempotency_key is retry dedupe, not operation identity
- rewarder rejects smuggled authority fields
- rewarder rejects raw engagement protocol-payout basis
- rewarder requires explicit funding-source provenance
- rewarder uses integer minor-unit string money at the wire boundary
- rewarder does not expose balance truth
- rewarder does not expose receipt truth
- rewarder does not expose operation truth
- rewarder does not expose roots/finality/checkpoints
- rewarder does not mutate ledger directly
- rewarder does not bypass svc-wallet
- rewarder routes mutation requests toward svc-wallet only
- rewarder docs preserve future QuickChain scope without implementing it
- rewarder tooling discovers and runs QuickChain gates dynamically
```

---

## Current Forbidden Scope Locked Out

`svc-rewarder` must continue to reject:

```text
roots
receipt roots
account state roots
hold roots
checkpoint roots
epoch roots
checkpoint production
validator behavior
settlement authority
finality claims
external anchors
bridge behavior
public-chain settlement
staking
liquidity
Solana path
ROX path
external L2/DA mutation path
direct ledger mutation
direct wallet mutation
protocol payout from raw engagement
fake balances
fake receipts
silent spend
```

---

## Cross-Crate Boundary After This Work

The intended value-loop position is now clearer:

```text
ron-proto:
  DTOs and strict wire shapes

ron-ledger:
  durable economic truth and replay

svc-wallet:
  mutation front-door and receipt production

svc-storage:
  b3 bytes, paid access enforcement hooks, metering input

ron-accounting:
  derivative snapshots and planning inputs

svc-rewarder:
  deterministic payout planning and wallet handoff intents

future QuickChain:
  eventually verifies canonical roots/proofs/checkpoints after locked vectors
```

The wrong shape remains forbidden:

```text
ron-accounting pays directly.
svc-rewarder pays directly.
svc-storage pays directly.
svc-gateway pays directly.
omnigate pays directly.
CrabLink pays directly.
raw engagement pays directly.
```

---

## Completion Estimate For This Slice

For this crate’s current QuickChain Phase-0 / QC-1A boundary scope:

```text
svc-rewarder: 95–100% parked for this sweep
```

This does not mean the full future reward system is finished.

Remaining future work, after explicit later gates, may include:

```text
- deeper production policy integration
- hardened production wallet transport
- stronger accounting snapshot windows
- more complete payout planning economics
- production reward scheduling
- future root material emitted by upstream canonical systems
- future proof verification paths
```

But those are not required for this current boundary pass.

---

## Do Not Reopen Unless

Do not reopen this crate during the current pass unless:

```text
- terminal output shows a regression
- a later crate-pair exposes a missing rewarder boundary
- svc-wallet/ron-ledger DTO changes require rewarder handoff updates
- ron-accounting snapshot format changes require rewarder parser/interop updates
- QuickChain vector/root gates are explicitly opened later
```

---

## Regeneration Command

After this pass, regenerate the crate codebundle with:

```bash
bash scripts/make_crate_codex.sh --force -c svc-rewarder
```

---

### END NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-rewarder

### BEGIN NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-storage

# svc-storage QuickChain Phase-0 / QC-1A Notes

## Current Status

```text
svc-storage: GREEN / QuickChain Phase-0 boundary parked / QC-1A foundation slice complete for this pass
```

`svc-storage` is now parked for the current QuickChain Phase-0 / QC-1A boundary sweep.

The crate’s QuickChain posture is:

```text
svc-storage stores and serves bytes by canonical b3 identity.
svc-storage may enforce paid-access admission through backend-derived authorization.
svc-storage may provide metering/quote inputs.
svc-storage does not mutate wallet state.
svc-storage does not mutate ledger state.
svc-storage does not mint, issue, transfer, burn, hold, capture, or release ROC.
svc-storage does not produce wallet receipts.
svc-storage does not treat b3 as payment proof.
svc-storage does not let cache unlock paid content alone.
svc-storage does not produce QuickChain roots.
svc-storage does not claim finality or settlement.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
QuickChain remains future verification/settlement infrastructure.
```

This is the correct role for `svc-storage` inside the internal ROC value loop.

The crate now proves b3 integrity, paid/cache boundaries, range-media boundaries, quote-only economics, observability redaction, and no-direct-mutation posture.

---

## Intended Role

`svc-storage` is content-addressed storage and byte-serving infrastructure.

It may:

```text
- ingest bytes
- derive canonical b3 IDs from bytes
- verify b3 before trusted reads
- store objects/manifests/renditions
- serve objects by b3
- serve ranges/segments for bounded media
- quote storage or retrieval cost shapes
- require backend-derived paid authorization for paid content
- expose admission/metering signals
- emit derivative accounting inputs
- participate in paid enforcement by checking authorization/receipt-shaped backend proof
```

It must not:

```text
- decide balances
- create receipts
- create wallet holds
- capture wallet holds
- release wallet holds
- mint/issue/transfer/burn ROC
- mutate ron-ledger
- bypass svc-wallet
- treat b3 byte identity as payment proof
- treat cached bytes as paid unlock authority
- treat display receipt cache as spend authority
- expose QuickChain roots
- expose checkpoint authority
- expose validator behavior
- expose settlement authority
- expose external anchors
- expose public bridge behavior
- expose staking or liquidity logic
- expose Solana/ROX/external settlement paths
```

The short doctrine is:

```text
b3 proves bytes.
b3 does not prove payment.
cache proves possession.
cache does not prove entitlement.
storage admission is not finality.
wallet/ledger truth unlocks paid economics.
```

---

## Why This Crate Matters

`svc-storage` sits directly in the paid content path. That makes it a dangerous place for accidental authority creep.

Without guardrails, the wrong shape would be:

```text
client has bytes in cache
-> storage sees b3
-> storage treats b3/cache as payment proof
-> paid content unlocks without backend wallet/ledger truth
```

Another wrong shape would be:

```text
storage quote/admission
-> storage creates receipt-looking object
-> gateway/omnigate/CrabLink treat storage response as paid finality
```

Both are forbidden.

The safe shape is:

```text
user requests paid content
-> prepare/quote
-> explicit confirmation
-> backend wallet path
-> backend-derived receipt/authorization
-> storage/gateway/omnigate enforce paid access
-> bytes unlock/render
-> receipt cache remains display-only
```

This crate’s current test/documentation posture keeps storage in the safe shape.

---

## Files Added or Hardened

### Documentation

```text
crates/svc-storage/docs/quickchain-preflight.md
```

Purpose:

```text
- Defines svc-storage as bytes/b3/content infrastructure.
- States b3 is content truth only.
- States b3 is not payment proof.
- States cache cannot unlock paid content alone.
- States paid content requires backend-derived authorization/receipt.
- States receipt cache is display-only.
- States storage metering is derivative accounting input only.
- States svc-wallet is the mutation front-door.
- States ron-ledger is durable economic truth.
- Documents bounded media/range/segment posture.
- Documents that each rendition owns its own b3.
- Names forbidden QuickChain/runtime scope.
```

### Scripts

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The preflight script now:

```text
- uses Bash strict mode
- runs from repo root
- verifies quickchain-preflight docs exist
- rejects checked-in Python helper drift under the crate
- runs cargo fmt check
- dynamically discovers every tests/quickchain*.rs target
- runs every discovered focused QuickChain test target
- runs cargo test -p svc-storage --all-targets
- runs cargo clippy -p svc-storage --all-targets --no-deps -- -D warnings
- prints the forbidden-scope marker
- prints the final exhaustive preflight success marker
```

The park script now:

```text
- verifies the docs file exists
- verifies the preflight script exists
- verifies the tooling boundary test exists
- delegates to the exhaustive preflight script
- prints the final parking-gate success marker
```

Important low-disk workflow note:

```text
Do not run both dev-quickchain-preflight.sh and dev-quickchain-park.sh for the same crate unless deliberately requesting duplicate full passes.

The park script delegates to preflight.
```

### Focused QuickChain Tests

Current focused QuickChain targets:

```text
crates/svc-storage/tests/quickchain_preflight_b3_integrity.rs
crates/svc-storage/tests/quickchain_preflight_boundary.rs
crates/svc-storage/tests/quickchain_preflight_docs.rs
crates/svc-storage/tests/quickchain_preflight_economics_quote.rs
crates/svc-storage/tests/quickchain_preflight_no_direct_mutation.rs
crates/svc-storage/tests/quickchain_preflight_observability.rs
crates/svc-storage/tests/quickchain_preflight_paid_cache.rs
crates/svc-storage/tests/quickchain_preflight_range_media.rs
crates/svc-storage/tests/quickchain_preflight_settlement_boundary.rs
crates/svc-storage/tests/quickchain_preflight_source_authority_scan.rs
crates/svc-storage/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-storage/tests/quickchain_tooling_boundary.rs
```

This gives `svc-storage` twelve focused QuickChain test targets in the current pass.

---

## What Each QuickChain Test Target Proves

### `quickchain_preflight_b3_integrity.rs`

This target proves byte identity remains canonical and content-derived.

It verifies:

```text
- caller cannot retrieve bytes under fake or noncanonical CID
- object ingest derives canonical b3 from bytes
```

This keeps b3 as content truth, not caller-provided authority.

### `quickchain_preflight_boundary.rs`

This target proves storage route/response shapes remain storage-shaped, not economic or QuickChain authority-shaped.

It verifies:

```text
- router exposes storage routes, not QuickChain authority routes
- public response shapes do not claim balance, receipt, root, or finality truth
```

This blocks storage from exposing runtime/settlement surfaces.

### `quickchain_preflight_docs.rs`

This target proves documentation stays synchronized with the crate’s intended QuickChain posture.

It verifies docs preserve the critical storage doctrine:

```text
- storage is bytes/b3/content infrastructure
- b3 is content truth only
- b3 is not payment proof
- cache cannot unlock paid content alone
- paid content requires backend-derived authorization/receipt
- receipt cache is display-only
- storage does not mutate wallet or ledger
- future QuickChain roots/checkpoints/validators/settlement remain parked
```

This matters because paid storage is a subtle boundary that future contributors could easily misunderstand.

### `quickchain_preflight_economics_quote.rs`

This target proves storage economics remain quote/admission/meters only, not mutation authority.

It verifies:

```text
- quote fields do not become wallet/ledger mutation fields
- storage quote behavior does not produce receipt truth
- storage cost/admission shapes do not imply final settlement
- money remains integer/minor-unit shaped where applicable
```

Storage can help price or enforce paid content, but it cannot become the wallet.

### `quickchain_preflight_no_direct_mutation.rs`

This target proves storage cannot directly mutate wallet/ledger state.

It verifies:

```text
- storage does not expose direct wallet or ledger mutation routes
- storage does not expose mint/issue/transfer/burn/hold/capture/release authority
- storage does not claim receipt/balance/finality/root truth
- storage config does not grow direct external settlement/bridge/anchor knobs
```

This keeps storage out of the economic mutation path.

### `quickchain_preflight_observability.rs`

This target proves metrics/logging/observability stay safe.

It verifies:

```text
- storage observability does not leak authority-shaped values
- metrics do not claim receipts, balances, roots, or finality
- observability remains derivative and operational
```

Storage metrics can inform accounting, debugging, and operations. They cannot become payout authority or economic proof.

### `quickchain_preflight_paid_cache.rs`

This target proves paid content cannot be unlocked by cache alone.

It verifies:

```text
- cache hit is not paid authorization
- display receipt cache is not spend authority
- paid unlock requires backend-derived authorization/receipt path
- offline/cache posture verifies b3 before trusted render but cannot replace wallet/ledger truth
```

This is one of the most important storage boundary tests because CrabLink will eventually rely on offline/cache UX.

### `quickchain_preflight_range_media.rs`

This target proves media serving remains bounded and honest.

It verifies:

```text
- range/segment media posture is bounded
- large media is not shoved through unbounded command/result paths
- each rendition owns its own b3 identity
- media delivery does not imply payment/finality truth
```

This preserves the CrabLink media doctrine: bounded/honest media, no DRM/anti-rip claims, and each rendition has its own content identity.

### `quickchain_preflight_settlement_boundary.rs`

This target proves storage settlement-shaped concepts do not become settlement authority.

It verifies:

```text
- storage may verify or carry backend-derived settlement/authorization references where appropriate
- storage does not produce settlement
- storage does not produce finality
- storage does not produce ledger roots
- storage does not replace wallet/ledger truth
```

This is intentionally careful: storage may need to interact with receipt/authorization metadata for paid content, but it must not author that truth.

### `quickchain_preflight_source_authority_scan.rs`

This target was added in the current pass.

It proves the source tree does not quietly grow runtime authority or external settlement surfaces.

It verifies:

```text
- svc-storage source tree does not define QuickChain runtime modules
- svc-storage source does not contain direct chain or external settlement tokens after stripping comments/string literals
```

This is a guardrail against future accidental imports, modules, or authority-shaped code paths.

The source scan is intentionally code-only: it strips comments and string literals before checking forbidden runtime tokens so docs/tests can still describe forbidden concepts.

### `quickchain_preflight_value_loop_boundary.rs`

This target proves storage stays in the correct place in the internal ROC value loop.

It verifies:

```text
- storage participates in paid enforcement and metering without becoming economic truth
- storage does not bypass svc-wallet
- storage does not replace ron-ledger
- storage outputs remain admission/storage/byte-serving shaped
```

This keeps storage as content infrastructure, not wallet/ledger authority.

### `quickchain_tooling_boundary.rs`

This target proves the crate-local QuickChain tooling remains safe and repeatable.

It verifies:

```text
- park script delegates to exhaustive preflight
- preflight script is Bash/cargo-only and keeps the full gate
- preflight script discovers all QuickChain tests dynamically
- no Python helpers are checked into the storage crate
```

This supports the current project workflow and prevents stale manual test lists.

---

## Current Verified Gate Results

The final parking run verified:

```text
Focused QuickChain tests discovered: 12

quickchain_preflight_b3_integrity.rs:         passed
quickchain_preflight_boundary.rs:             passed
quickchain_preflight_docs.rs:                 passed
quickchain_preflight_economics_quote.rs:      passed
quickchain_preflight_no_direct_mutation.rs:   passed
quickchain_preflight_observability.rs:        passed
quickchain_preflight_paid_cache.rs:           passed
quickchain_preflight_range_media.rs:          passed
quickchain_preflight_settlement_boundary.rs:  passed
quickchain_preflight_source_authority_scan.rs: passed
quickchain_preflight_value_loop_boundary.rs:  passed
quickchain_tooling_boundary.rs:               passed

cargo test -p svc-storage --all-targets:      passed
cargo clippy -p svc-storage --all-targets --no-deps -- -D warnings: passed

Final marker:
== svc-storage quickchain exhaustive preflight gate passed: tests=12 ==
== svc-storage QuickChain parking gate passed ==
```

The focused source-authority scan also passed before the park run:

```text
storage_source_tree_does_not_define_quickchain_runtime_modules: ok
storage_source_has_no_direct_chain_or_external_settlement_tokens: ok
```

---

## What svc-storage Now Proves

`svc-storage` now proves:

```text
- b3 identity is derived from bytes
- fake or noncanonical CIDs cannot retrieve bytes
- storage routes remain storage routes
- storage public responses do not claim balances, receipts, roots, or finality
- storage quotes remain quote/admission shapes
- storage does not expose direct wallet mutation
- storage does not expose direct ledger mutation
- storage observability remains operational/derivative
- paid cache cannot unlock content by itself
- offline/cache posture cannot replace backend authorization
- bounded range/media posture remains explicit
- each rendition owns its own b3
- storage settlement boundary remains verification/enforcement only, not authority
- source tree does not define QuickChain runtime modules
- source tree does not grow external settlement/bridge/validator/staking/liquidity logic
- tooling discovers and runs QuickChain gates dynamically
```

---

## Current Forbidden Scope Locked Out

`svc-storage` must continue to reject:

```text
roots
receipt roots
account state roots
hold roots
checkpoint roots
epoch roots
checkpoint production
validator behavior
settlement authority
finality claims
external anchors
bridge behavior
public-chain settlement
staking
liquidity
Solana path
ROX path
external L2/DA mutation path
direct ledger mutation
direct wallet mutation
fake balances
fake receipts
silent spend
cache-only paid unlock
b3-as-payment-proof
manifest-as-payment-proof
storage-admission-as-finality
```

---

## Cross-Crate Boundary After This Work

The intended value-loop position is now clearer:

```text
ron-proto:
  DTOs and strict wire shapes

ron-ledger:
  durable economic truth and replay

svc-wallet:
  mutation front-door and receipt production

svc-storage:
  b3 byte storage, paid admission/enforcement support, bounded media, metering input

ron-accounting:
  derivative snapshots and planning inputs

svc-rewarder:
  deterministic payout planning and wallet handoff intents

future QuickChain:
  eventually verifies canonical roots/proofs/checkpoints after locked vectors
```

The wrong shape remains forbidden:

```text
storage treats b3 as receipt.
storage treats cache as entitlement.
storage treats quote as payment.
storage treats admission as finality.
storage mutates wallet directly.
storage mutates ledger directly.
gateway/omnigate unlock paid content from cache alone.
CrabLink treats display receipt cache as spend authority.
```

---

## Completion Estimate For This Slice

For this crate’s current QuickChain Phase-0 / QC-1A boundary scope:

```text
svc-storage: 95–100% parked for this sweep
```

This does not mean all future storage economics/media/availability work is complete.

Remaining future work, after explicit later gates, may include:

```text
- deeper production paid-storage flows
- stronger storage/provider availability proofs
- refined metering feeds into accounting
- hardened large-media delivery paths
- future proof systems for availability/retrieval
- future canonical root/proof integration after locked vectors
```

But those are not required for this current boundary pass.

---

## Do Not Reopen Unless

Do not reopen this crate during the current pass unless:

```text
- terminal output shows a regression
- a later crate-pair exposes a missing storage boundary
- svc-wallet/ron-ledger paid authorization DTOs change
- gateway/omnigate paid access contracts change
- CrabLink cache/offline behavior needs a stricter backend signal
- QuickChain vector/root gates are explicitly opened later
```

---

## Regeneration Command

After this pass, regenerate the crate codebundle with:

```bash
bash scripts/make_crate_codex.sh --force -c svc-storage
```

---

### END NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-storage

The pair is now cleanly documented as parked. Next is the cross-session carryover for **`svc-gateway + omnigate`**.


### END NOTE - JUNE 20 2026 - 11:30 CST


### BEGIN NOTE - JUNE 22 2026 - 11:40 CST

Below are paste-ready crate notes for `svc-rewarder + svc-storage`. The latest terminal confirms `svc-rewarder` parked with **10 focused QuickChain tests**, all-targets, clippy, and the parking gate passed; `svc-storage` parked with **13 focused QuickChain tests**, all-targets, clippy, and the parking gate passed.  

# QuickChain Crate Notes — `svc-rewarder + svc-storage`

Date: 2026-06-22
Project: RustyOnions / CrabLink
Track: QuickChain Phase 1 / QC-1A Round 1 foundation
Crate pair: `svc-rewarder + svc-storage`
Status: GREEN / parked for this QuickChain slice

---

## Summary

This session completed the `svc-rewarder + svc-storage` QuickChain QC-1A / Phase 1 Round 1 pair pass.

The goal of this pass was not to implement roots, checkpoints, validators, settlement, anchors, bridges, staking, liquidity, ROX, Solana, public-chain state, or external settlement logic.

The goal was to harden the service boundaries so:

```text
svc-rewarder remains deterministic payout planning only.
svc-storage remains bytes, b3 integrity, paid admission, range/media, and metering only.
svc-wallet remains the ROC mutation front-door.
ron-ledger remains durable economic truth.
ron-accounting remains derivative snapshot/accounting input, not balance truth.
QuickChain remains future settlement infrastructure.
```

Both crates are now parked green under their crate-local QuickChain parking gates.

---

## Files Added / Updated

### `svc-rewarder`

Added:

```text
crates/svc-rewarder/tests/quickchain_preflight_phase1_pair_interlock.rs
```

This new test target locks the QC-1A pair boundary for `svc-rewarder`.

It verifies that:

```text
- rewarder docs preserve rewarder as planner, not root authority
- manifests and settlement batches remain planning artifacts, not roots
- accounting and storage inputs cannot smuggle payout execution or root material
- backend wallet receipts may pass through wallet HTTP outcomes, but rewarder does not become receipt authority
- rewarder source tree does not gain root producer, validator, bridge, Solana, ROX, staking, liquidity, or external settlement runtime
```

No `Cargo.toml` changes were needed.

### `svc-storage`

Added / updated:

```text
crates/svc-storage/tests/quickchain_preflight_phase1_pair_interlock.rs
```

This new test target locks the QC-1A pair boundary for `svc-storage`.

It verifies that:

```text
- storage docs preserve storage as paid admission and metering only
- paid object responses remain storage admission + metering output, not reward/root/finality authority
- settlement adapter targets svc-wallet capture/release only and does not become ledger or chain authority
- accounting export remains metering only, not payout authorization
- storage runtime source tree does not gain root producer, validator, bridge, Solana, ROX, staking, liquidity, or external settlement runtime
```

A follow-up patch fixed one false positive in the `svc-storage` source authority scan.

The failure was caused by the string `solana` appearing inside a defensive negative unit-test fixture, not in production runtime code. The test was corrected so the runtime authority scan strips ordinary `#[cfg(test)] mod tests { ... }` blocks before scanning production source for forbidden runtime markers.

This keeps the test strict against real runtime authority creep while allowing unit tests to contain hostile/forbidden strings used as rejection fixtures.

No `Cargo.toml` changes were needed.

---

## `svc-rewarder` Boundary Locked In

`svc-rewarder` is now explicitly guarded as:

```text
deterministic payout planner
reward manifest producer
wallet-facing intent planner
policy/accounting consumer
idempotent replay/dedupe surface
```

It is explicitly not:

```text
ledger truth
wallet truth
receipt authority
balance authority
operation authority
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
staking/liquidity authority
public-chain authority
```

The crate now has a focused Phase 1 pair-interlock test proving rewarder cannot accidentally replace wallet/ledger truth.

Important protected concepts:

```text
rewarder output is plan/intent only
rewarder planning is deterministic for the same input
rewarder replay is dedupe/idempotency, not second payout authority
idempotency keys are retry dedupe, not durable operation identity
rewarder rejects smuggled authority fields
rewarder rejects raw engagement protocol-payout basis
rewarder requires explicit funding-source provenance
rewarder uses integer minor-unit money at the wire boundary
rewarder does not expose balance truth
rewarder does not expose receipt truth
rewarder does not expose operation truth
rewarder does not expose roots, finality, checkpoints, or validator claims
rewarder does not mutate ledger directly
rewarder does not bypass svc-wallet
rewarder routes mutation requests toward svc-wallet only
```

---

## `svc-storage` Boundary Locked In

`svc-storage` is now explicitly guarded as:

```text
canonical b3 byte storage
paid storage admission surface
wallet receipt evidence verifier
optional wallet capture/release adapter through svc-wallet
range/media byte serving surface
usage/metering source
accounting export source
```

It is explicitly not:

```text
ledger truth
wallet truth
receipt truth
balance truth
reward authority
payout authority
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
cache entitlement authority
staking/liquidity authority
public-chain authority
```

Important protected concepts:

```text
b3 hashes identify bytes only
cache is not paid-access authority
storage cannot unlock paid content from cache alone
storage paid-write responses are admission/metering evidence, not finality
wallet receipts are backend evidence, not storage-created truth
settlement adapter calls wallet capture/release only
storage never calls ron-ledger directly for economic mutation
accounting export is usage metering only
accounting export failure does not mutate wallet/ledger state
storage does not mint, allocate, or pay ROC
storage does not create reward roots or payout plans
storage does not create QuickChain roots/checkpoints/validator state
```

---

## Important False Positive Fixed

Initial `svc-storage` focused test result:

```text
storage_src_has_no_root_producer_validator_bridge_or_external_settlement_runtime ... FAILED
```

Cause:

```text
The broad runtime source scan matched the word "solana" inside a negative unit-test fixture.
```

Resolution:

```text
The scan now strips ordinary #[cfg(test)] mod tests { ... } blocks before checking runtime source text.
```

Why this is correct:

```text
The test is intended to detect production/runtime authority creep.
Defensive test fixtures may contain forbidden words to prove parser rejection.
Runtime source must remain free of Solana/ROX/bridge/external-settlement authority.
```

After the fix:

```text
cargo test -p svc-storage --test quickchain_preflight_phase1_pair_interlock
```

passed:

```text
5 passed; 0 failed
```

---

## Commands Run

Focused pair-interlock tests:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_phase1_pair_interlock
cargo test -p svc-storage --test quickchain_preflight_phase1_pair_interlock
```

Final parking gates:

```bash
crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The low-disk workflow was respected: focused tests first, then each park script once. We did not duplicate preflight + park unnecessarily.

---

## Final Verified Gate Results

### `svc-rewarder`

Final status:

```text
GREEN / parked
```

Focused QuickChain tests discovered:

```text
10
```

Discovered focused tests:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Additional gates:

```text
cargo test -p svc-rewarder --all-targets: passed
cargo clippy -p svc-rewarder --all-targets -- -D warnings: passed
```

Final markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=10 ==
== svc-rewarder QuickChain parking gate passed ==
```

Forbidden-scope marker preserved:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-rewarder remains deterministic payout planning only; svc-wallet remains mutation front-door; ron-ledger remains truth
```

### `svc-storage`

Final status:

```text
GREEN / parked
```

Focused QuickChain tests discovered:

```text
13
```

Discovered focused tests:

```text
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Additional gates:

```text
cargo test -p svc-storage --all-targets: passed
cargo clippy -p svc-storage --all-targets -- -D warnings: passed
```

Final markers:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=13 ==
== svc-storage QuickChain parking gate passed ==
```

Forbidden-scope marker preserved:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-storage remains bytes by canonical b3 only; cache is not paid-access authority; wallet/ledger truth stays backend-owned
```

---

## Why This Pair Matters

This pair is important because it protects the middle of the internal ROC value loop.

`svc-storage` creates paid storage/access/metering signals. Those signals are useful inputs for accounting and future reward planning, but they must never directly unlock paid access from cache alone and must never directly create payout authority.

`svc-rewarder` consumes accounting/policy-shaped information and produces deterministic payout plans. Those plans may become wallet-facing requests, but they are not receipts, not balance truth, not ledger truth, not finality, and not QuickChain roots.

The pair now proves the intended split:

```text
storage emits bytes + paid admission + metering evidence
accounting derives snapshots
rewarder plans payouts
wallet executes approved ROC mutations
ledger remains durable truth
```

---

## Current QuickChain Pair Progress

Current QC-1A / Phase 1 Round 1 pair order:

```text
1. ron-proto + ron-ledger          ✅ completed for this slice
2. svc-wallet + ron-accounting     ✅ completed for this slice
3. svc-rewarder + svc-storage      ✅ completed for this slice
4. svc-gateway + omnigate          ← next
5. svc-index + ron-policy
6. CrabLink Tauri + client adapters
```

The next crate pair is:

```text
svc-gateway + omnigate
```

The next pair should focus on public route/admission and hydration/access composition boundaries.

---

## Next Pair Preview: `svc-gateway + omnigate`

Next mission:

```text
Harden svc-gateway and omnigate so they can participate in paid access and hydration flows without becoming wallet truth, ledger truth, receipt truth, balance truth, storage truth, cache authority, root authority, checkpoint authority, validator authority, bridge authority, external settlement authority, finality authority, staking/liquidity authority, or public-chain authority.
```

Expected next-session focus:

```text
svc-gateway = public boundary / route / admission / proxy / fail-closed surface
omnigate = product hydration / access composition / backend-derived display surface
```

Do not add:

```text
roots
checkpoints
validators
committee/quorum logic
settlement finality
anchors
bridges
ROX
Solana
staking
liquidity
exchange-facing logic
public-chain state
cache-only paid unlock
fake receipts
fake balances
fake finality
direct wallet mutation from the client/product layer
direct ledger mutation from gateway/omnigate
```

---

## Recommended Cleanup Before Next Pair

Because the park gates compiled many tests and dependencies, this is a good cleanup point:

```bash
cargo clean
```

Then regenerate focused codebundles for the next pair:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Recommended files to attach for the next session:

```text
QUICKCHAIN_BUILDPLAN.MD
QUICKCHAIN_REVIEW_BUNDLE.MD
CODEBUNDLE_RS.md
latest carryover notes
latest terminal output
crates/svc-gateway/CODEBUNDLE.md
crates/omnigate/CODEBUNDLE.md
```

---

## One-Line Handoff

`svc-rewarder + svc-storage` are now green/parked for QuickChain QC-1A / Phase 1 Round 1: rewarder is locked as deterministic payout planning only, storage is locked as b3 bytes/paid admission/metering only, wallet/ledger remain economic truth, and the next crate pair is `svc-gateway + omnigate`.


### END NOTE - JUNE 22 2026 - 11:40 CST


### BEGIN NOTE - JUNE 22 2026 - 18:35 CST

The terminal evidence supports these notes: `svc-rewarder` passed its Phase 1 Round 2 confirmation and parking gate, and `svc-storage` passed the fixed confirmation test plus its full parking gate with 14 focused QuickChain tests.  

# QuickChain Phase 1 Crate Notes — `svc-rewarder` + `svc-storage`

Date: June 22, 2026
Scope: QuickChain Phase 1, Round 2
Crate pair:

```text
svc-rewarder + svc-storage
```

## Status Summary

`svc-rewarder` and `svc-storage` are now complete for QuickChain Phase 1.

Final pair status:

```text
svc-rewarder:
  Phase 1 Round 1: COMPLETE
  Phase 1 Round 2: COMPLETE
  Pair status: PHASE 1 COMPLETE / PARKED

svc-storage:
  Phase 1 Round 1: COMPLETE
  Phase 1 Round 2: COMPLETE
  Pair status: PHASE 1 COMPLETE / PARKED
```

This does not mean all of QuickChain Phase 1 is complete. It means this crate pair is done for Phase 1 and should not be touched again unless a later cross-crate audit exposes a real integration mismatch.

Remaining Phase 1 Round 2 pairs after this:

```text
svc-gateway + omnigate
svc-index + ron-policy
CrabLink Tauri + client adapters
```

## What This Session Accomplished

This session completed the downstream Phase 1 Round 2 confirmation pass for `svc-rewarder` and `svc-storage`.

The goal was not to make either crate a QuickChain runtime. The goal was to confirm that both crates can safely coexist with Phase 1 deterministic root/proof material while preserving strict service authority boundaries.

The pair now confirms:

```text
svc-rewarder remains deterministic payout planning only.
svc-storage remains canonical b3 byte/artifact storage only.
reward plans and manifests may be referenced as artifacts.
storage CIDs may reference vector/root/proof artifacts as bytes.
artifact CIDs are not QuickChain roots.
artifact storage is not finality.
rewarder planning is not ledger mutation.
storage admission is not paid-access authority by itself.
svc-wallet remains the economic mutation front-door.
ron-ledger remains durable economic truth.
```

## `svc-rewarder` Notes

### Role

`svc-rewarder` remains deterministic reward planning infrastructure.

Its valid role is:

```text
ron-accounting snapshot / metering material
  -> deterministic reward planning
  -> reward manifest / payout plan artifact
  -> explicit wallet path later if approved
```

It is not:

```text
a wallet
a ledger
a root producer
a checkpoint producer
a validator
a bridge
an external settlement adapter
a finality authority
a direct payout executor
```

### Phase 1 Round 2 Confirmation

The new/confirmed Round 2 test target is:

```text
crates/svc-rewarder/tests/quickchain_phase1_round2_confirmation.rs
```

It confirms:

```text
docs name the Phase 1 Round 2 downstream confirmation boundary
reward manifest commitments are referenceable artifacts, not root authority
accounting and policy inputs remain planning inputs, not balance/root truth
wallet egress remains explicit wallet issue handoff, not rewarder mutation truth
```

The focused confirmation test passed:

```text
cargo test -p svc-rewarder --test quickchain_phase1_round2_confirmation
```

Result:

```text
4 passed; 0 failed
```

The full rewarder park gate also passed:

```text
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The park gate confirmed:

```text
focused QuickChain tests discovered dynamically
all focused QuickChain tests passed
all-targets tests passed
clippy passed with -D warnings
forbidden-scope marker preserved
parking gate passed
```

### Protected Invariants

`svc-rewarder` now has Phase 1 protection for:

```text
no direct ledger mutation
no hidden wallet mutation
no fake balances
no fake receipts
no fake finality
no root-producing rewarder runtime
no checkpoint authority
no validator behavior
no bridge or external settlement
no staking
no liquidity
no ROX/Solana path
no raw engagement protocol ROC minting
```

### Important Design Boundary

Rewarder output can be treated as deterministic planning material or artifact material.

It must not be treated as:

```text
balance truth
receipt truth
ledger truth
QuickChain root truth
settlement finality
validator approval
protocol payout execution
```

The mental model remains:

```text
ron-accounting measures.
svc-rewarder plans.
svc-wallet mutates.
ron-ledger is truth.
QuickChain later verifies roots.
```

## `svc-storage` Notes

### Role

`svc-storage` remains byte/object infrastructure.

Its valid role is:

```text
canonical b3 object storage
bounded read/range media serving
paid write admission evidence handling
storage metering
artifact byte retention
```

It is not:

```text
a wallet
a ledger
a receipt authority
a paid-unlock authority by itself
a root producer
a checkpoint producer
a validator
a bridge
an external settlement adapter
a finality oracle
```

### Phase 1 Round 2 Confirmation

The new/confirmed Round 2 test target is:

```text
crates/svc-storage/tests/quickchain_phase1_round2_confirmation.rs
```

It confirms:

```text
docs name the Phase 1 Round 2 storage artifact boundary
storage can store and retrieve Phase 1 artifacts by canonical b3
object routes and storage remain content-addressed byte paths, not chain authority
paid/accounting storage sources do not turn artifacts into unlock or finality authority
```

The focused confirmation test passed after the marker-string fix:

```text
cargo test -p svc-storage --test quickchain_phase1_round2_confirmation
```

Result:

```text
4 passed; 0 failed
```

The full storage park gate also passed:

```text
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The park gate confirmed:

```text
14 focused QuickChain tests discovered dynamically
all focused QuickChain tests passed
all-targets tests passed
clippy passed with -D warnings
forbidden-scope marker preserved
parking gate passed
```

### Specific Fix Applied

The only follow-up fix needed was in the storage confirmation test.

The original marker expected the literal phrase:

```text
b3:<64 lowercase hex>
```

inside stripped source text. Since the source scanner stripped comments and the real enforcement lived in code-level guards, the test was corrected to check actual source guards instead:

```text
starts_with("b3:")
3 + 64
b'a'..=b'f'
```

This better reflects the runtime source boundary: canonical b3 is enforced by code, not by a comment-only marker.

### Protected Invariants

`svc-storage` now has Phase 1 protection for:

```text
canonical b3 object identity
content-addressed object reads
bounded range reads
storage artifact retrieval by b3
paid write proof requirement
wallet-derived paid evidence
accounting export as metering only
cache cannot unlock paid content alone
no balance mutation
no direct ledger mutation
no fake balances
no fake receipts
no fake finality
no root-producing storage runtime
no checkpoint authority
no validator behavior
no bridge or external settlement
no staking
no liquidity
no ROX/Solana path
```

### Important Design Boundary

Storage may retain bytes that represent future QuickChain vectors, roots, or proofs.

That does not make storage:

```text
the root authority
the verifier
the validator
the finality oracle
the wallet
the ledger
the paid-access authority
```

The correct model is:

```text
storage stores bytes by b3.
b3 identifies bytes.
QuickChain semantics live elsewhere.
wallet/ledger truth lives elsewhere.
paid unlock requires backend-derived receipt/proof flow.
```

## Files Changed / Added

The effective Phase 1 Round 2 work for this pair centered on:

```text
crates/svc-rewarder/tests/quickchain_phase1_round2_confirmation.rs
crates/svc-storage/tests/quickchain_phase1_round2_confirmation.rs
crates/svc-rewarder/docs/quickchain-preflight.md
crates/svc-storage/docs/quickchain-preflight.md
```

The changes were intentionally downstream-light and boundary-focused. They did not add root production, checkpoint production, validator logic, external settlement logic, bridge logic, staking, liquidity, ROX, Solana, fake receipts, fake balances, or direct ledger mutation.

## Validation Commands Run

Focused tests:

```bash
cargo test -p svc-rewarder --test quickchain_phase1_round2_confirmation
cargo test -p svc-storage --test quickchain_phase1_round2_confirmation
```

Parking gates:

```bash
crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

Final observed status:

```text
svc-rewarder QuickChain parking gate passed
svc-storage QuickChain parking gate passed
```

## Completion Decision

`svc-rewarder + svc-storage` should now be marked:

```text
QuickChain Phase 1 complete / parked for this crate pair.
```

Do not continue patching this pair during Phase 1 unless later work in `svc-gateway + omnigate`, `svc-index + ron-policy`, CrabLink Tauri, or the final cross-crate audit exposes a concrete mismatch.

## Next Crate Pair

The next QuickChain Phase 1 Round 2 pair should be:

```text
svc-gateway + omnigate
```

The next pair should prove:

```text
svc-gateway remains public boundary / paid enforcement boundary only.
omnigate remains hydration/enforcement orchestration only.
gateway and omnigate may surface backend-derived proof/root/vector references.
gateway and omnigate must not mutate ledger or wallet truth.
gateway and omnigate must not produce roots/checkpoints.
gateway and omnigate must not claim finality.
gateway and omnigate must not unlock paid content from cache alone.
wallet/ledger receipts remain backend truth.
```

Use `QUICKCHAIN_BUILDPLAN.MD` as the phase/round reference and `QUICKCHAIN_REVIEW_BUNDLE.MD` as the master blueprint before starting the next pair.


### END NOTE - JUNE 22 2026 - 18:35 CST



### BEGIN NOTE - JUNE 23 2026 - 19:30 CST

Below are paste-ready crate notes. The terminal evidence shows the Phase 2 Round 1 patch applied, both focused `quickchain_phase2_replay_boundary` tests passed, `svc-rewarder` discovered/passed **12** focused QuickChain tests, `svc-storage` discovered/passed **15**, and both parking gates passed.  The prior carryover target for this pair was exactly to prove read-only verifier artifact handling without rewarder/storage becoming verifier, finality, committee, payout execution, settlement, or paid-unlock authority. 

Add this to `crates/svc-rewarder/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-rewarder

# QuickChain Crate Notes — svc-rewarder

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-rewarder`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-rewarder` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-rewarder COMPLETE / PARKED
```

This means `svc-rewarder` has been hardened for the Phase 2 Round 1 verifier-artifact/read-only replication slice.

It does not mean all of Phase 2 is complete. It does not mean committee signing, quorum/fork-choice, validator readiness, staking, slashing, bridge behavior, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `svc-rewarder` can safely coexist with Phase 2 read-only verifier artifacts without becoming QuickChain authority.

The crate remains:

```text
deterministic payout planning only
reward manifest / planning artifact producer
accounting and policy input consumer
non-mutating payout planner
wallet handoff planner only
```

The crate does not become:

```text
verifier authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
ledger mutation authority
wallet mutation authority
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
svc-rewarder plans payouts; svc-wallet commits approved payout intents; ron-ledger records durable economic truth.
```

---

## 2. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-rewarder/tests/quickchain_phase2_replay_boundary.rs
```

This test locks the Phase 2 Round 1 boundary:

```text
reward manifests may become read-only verifier artifact inputs
reward manifests remain planning artifacts
rewarder does not sign committee votes
rewarder does not decide quorum
rewarder does not claim fork choice
rewarder does not claim finality
rewarder still cannot mutate ledger truth
svc-wallet commits approved payout intents
ron-ledger remains durable economic truth
```

The focused test passed:

```text
cargo test -p svc-rewarder --test quickchain_phase2_replay_boundary

running 3 tests
docs_name_phase2_round1_read_only_verifier_boundary ... ok
reward_outputs_are_replay_artifact_inputs_not_committee_authority ... ok
rewarder_does_not_turn_replay_artifacts_into_wallet_or_ledger_mutation_truth ... ok

test result: ok. 3 passed; 0 failed
```

---

## 3. Documentation Updated

Updated:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

The doc now explicitly names the Phase 2 Round 1 status:

```text
Phase 2 Round 1 verifier artifact / read-only replication
```

The doc now preserves these boundaries:

```text
reward manifests may become read-only verifier artifact inputs
reward manifests are not committee votes
reward manifests are not quorum decisions
reward manifests are not fork choice
reward manifests are not finality
reward manifests are not validator signatures
reward manifests are not balance truth
reward manifests are not direct payout execution
```

The doc also keeps the forbidden scope clear:

```text
no committee signing
no quorum/fork-choice
no validator signatures
no staking
no slashing
no public bridge
no external settlement
no ROX
no Solana
no direct ledger mutation
no fake receipts
no fake balances
no fake finality
```

---

## 4. Final Gate Evidence

The final local gate discovered 12 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=12 ==
== svc-rewarder QuickChain parking gate passed ==
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

The all-targets test gate completed cleanly:

```text
cargo test -p svc-rewarder --all-targets
```

---

## 5. Important Existing Coverage Still Preserved

This pass did not replace older rewarder safety coverage. It extended it.

Still protected:

```text
reward manifest does not expose roots, receipts, balances, or finality
wallet preview is issue-request shape, not receipt or balance truth
compute request rejects smuggled QuickChain authority fields
funding source is explicit
raw engagement cannot directly allocate protocol ROC
duplicate epoch replay is dedupe, not second payout authority
idempotency keys are retry dedupe, not operation identity
same snapshot/policy/epoch produces same plan commitment
reordered snapshot rows produce same plan
source tree does not define QuickChain runtime modules
source has no direct chain or ledger authority tokens
wallet client boundary targets wallet without direct ledger or chain authority
```

---

## 6. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-rewarder is green/parked for QuickChain Phase 2 Round 1.
It can reference or produce planning artifacts that may be consumed as read-only verifier evidence,
but it cannot become verifier, finality, committee, settlement, payout execution, wallet, or ledger authority.
```

---

## 7. What Not To Claim

Do not claim:

```text
Phase 2 complete
Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
reward execution live from verifier artifacts
rewarder can commit wallet payouts directly
rewarder can mutate ledger
external settlement ready
bridge ready
staking/slashing ready
```

---

## 8. Do Not Reopen Unless

Do not reopen `svc-rewarder` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in rewarder verifier-artifact boundaries
svc-wallet approved payout intent DTOs change
ron-ledger economic truth or receipt format changes
ron-accounting snapshot format changes
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source scanner catches new authority creep
```

Otherwise, `svc-rewarder` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 9. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-gateway + omnigate
```

Expected goal for that pair:

```text
public boundary / hydration / paid-enforcement surfaces may display or route backend-derived verifier/readiness artifacts,
but cannot become verifier, finality, settlement, wallet mutation, paid unlock, or chain authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-rewarder

Add this to `crates/svc-storage/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-storage

# QuickChain Crate Notes — svc-storage

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-storage`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-storage` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-storage COMPLETE / PARKED
```

This means `svc-storage` has been hardened for the Phase 2 Round 1 verifier-artifact/read-only replication slice.

It does not mean all of Phase 2 is complete. It does not mean storage is a verifier, finality system, payment truth system, committee participant, settlement layer, bridge, or public-chain runtime.

---

## 1. What This Pass Proved

This pass proved that `svc-storage` can safely retain and retrieve Phase 2 read-only verifier artifacts as bytes by canonical b3 without becoming QuickChain authority.

The crate remains:

```text
bytes/artifacts/manifests storage
b3 integrity surface
paid admission support surface
metering source
artifact persistence layer
bounded range/media byte path
```

The crate does not become:

```text
verifier authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
payment truth
wallet mutation authority
ledger mutation authority
paid-unlock authority by cache alone
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
b3 hashes prove bytes; they do not prove payment, finality, settlement, or access authority by themselves.
```

---

## 2. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-storage/tests/quickchain_phase2_replay_boundary.rs
```

This test locks the Phase 2 Round 1 boundary:

```text
storage may retain read-only verifier artifact bytes by canonical b3
storage may retrieve read-only verifier artifact bytes by canonical b3
artifact cids are byte references, not verifier authority
b3 proves bytes, not balance truth
b3 proves bytes, not paid access
b3 proves bytes, not finality
storage cannot decide quorum
storage cannot sign committee votes
storage cannot claim fork choice
storage cannot claim finality
storage cannot mutate replay outcomes
storage cannot unlock paid content from cache alone
wallet/ledger receipts remain backend truth
```

The focused test passed:

```text
cargo test -p svc-storage --test quickchain_phase2_replay_boundary

running 3 tests
docs_name_phase2_round1_storage_verifier_artifact_boundary ... ok
storage_can_archive_and_retrieve_read_only_verifier_artifacts_by_b3 ... ok
storage_replay_artifact_paths_do_not_become_verifier_or_paid_unlock_authority ... ok

test result: ok. 3 passed; 0 failed
```

---

## 3. Documentation Updated

Updated:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

The doc now explicitly names the Phase 2 Round 1 status:

```text
Phase 2 Round 1 verifier artifact / read-only replication
```

The doc now preserves these boundaries:

```text
storage may retain read-only verifier artifact bytes by canonical b3
storage may retrieve read-only verifier artifact bytes by canonical b3
artifact cids are byte references, not verifier authority
b3 proves bytes, not balance truth
b3 proves bytes, not paid access
b3 proves bytes, not finality
storage cannot decide quorum
storage cannot sign committee votes
storage cannot claim fork choice
storage cannot claim finality
storage cannot mutate replay outcomes
storage cannot unlock paid content from cache alone
wallet/ledger receipts remain backend truth
```

The doc also keeps the forbidden scope clear:

```text
no committee signing
no quorum/fork-choice
no validator signatures
no staking
no slashing
no public bridge
no external settlement
no ROX
no Solana
no direct wallet mutation
no direct ledger mutation
no fake receipts
no fake balances
no fake finality
no cache-only paid unlock
```

---

## 4. Final Gate Evidence

The final local gate discovered 15 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=15 ==
== svc-storage QuickChain parking gate passed ==
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings
```

The all-targets test gate completed cleanly:

```text
cargo test -p svc-storage --all-targets
```

---

## 5. Important Existing Coverage Still Preserved

This pass did not replace older storage safety coverage. It extended it.

Still protected:

```text
storage stores/retrieves Phase 1 artifacts by canonical b3
object routes and store remain content-addressed byte paths, not chain authority
paid/accounting storage sources do not turn artifacts into unlock or finality authority
object ingest derives canonical b3 from bytes
caller cannot retrieve bytes under fake or noncanonical cid
public response shapes do not claim balance, receipt, root, or finality truth
paid estimate is quote-only and integer minor units
storage does not mutate wallet or ledger directly
accounting export is metering, not balance truth
metrics stay low-cardinality and non-authoritative
fake cache or paid headers do not unlock absent objects
paid write rejects without backend-derived proof and does not cache bytes
range media path serves canonical b3 and bounded ranges only
settlement plan is integer, bounded, and deterministic without roots or finality
settlement source uses wallet front door only and no chain authority
source tree does not define QuickChain runtime modules
source has no direct chain or external settlement tokens
```

---

## 6. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-storage is green/parked for QuickChain Phase 2 Round 1.
It can retain and retrieve read-only verifier artifacts as canonical b3 bytes,
but those bytes do not become verifier, finality, committee, payment, settlement, wallet, ledger, or paid-unlock truth.
```

---

## 7. What Not To Claim

Do not claim:

```text
Phase 2 complete
Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
storage proves payment
storage proves settlement
storage unlocks paid content from cache
storage can mutate wallet
storage can mutate ledger
external settlement ready
bridge ready
staking/slashing ready
```

---

## 8. Do Not Reopen Unless

Do not reopen `svc-storage` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in storage verifier-artifact boundaries
svc-wallet receipt / paid-admission DTOs change
ron-ledger economic truth or receipt format changes
svc-gateway / omnigate paid enforcement contracts change
CrabLink cache/offline behavior needs a stricter backend signal
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source scanner catches new authority creep
```

Otherwise, `svc-storage` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 9. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-gateway + omnigate
```

Expected goal for that pair:

```text
public boundary / hydration / paid-enforcement surfaces may display or route backend-derived verifier/readiness artifacts,
but cannot become verifier, finality, settlement, wallet mutation, paid unlock, or chain authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-storage


### END NOTE - JUNE 23 2026 - 19:30 CST


### BEGIN NOTE - JUNE 24 2026 - 01:00 CST

The terminal confirms both crate-local park gates passed after the Phase 2 Round 2 committee-boundary patch, so these notes can be dropped into the two crate `NOTES.MD` files. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-rewarder — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-rewarder` is **parked for QuickChain Phase 2 Round 2**.

This round added the Phase 2 Round 2 **committee-readiness boundary** for `svc-rewarder` without implementing committee runtime, validator runtime, quorum logic, fork choice, finality, roots, checkpoints, bridge, staking, slashing, liquidity, or external settlement.

The crate remains exactly where it belongs in the RustyOnions/QuickChain value loop:

```text
ron-accounting snapshots / policy inputs
→ svc-rewarder deterministic payout planning
→ explicit svc-wallet issue-request handoff previews
→ svc-wallet commits approved payout intents
→ ron-ledger remains durable economic truth
```

`svc-rewarder` is still a deterministic payout planner only. It does not mutate ledger truth and does not become committee, finality, settlement, or validator-economy authority.

## What changed in this round

Added the new focused Phase 2 Round 2 committee-boundary test:

```text
crates/svc-rewarder/tests/quickchain_phase2_committee_boundary.rs
```

Updated the QuickChain preflight documentation with a new Phase 2 Round 2 marker section:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

No `Cargo.toml` changes were required.

## Boundary locked by this patch

The new test locks the following `svc-rewarder` rules:

```text
- reward manifests remain payout planning artifacts
- wallet issue requests remain explicit svc-wallet handoff previews
- svc-rewarder is not a committee member
- svc-rewarder does not produce signed verification attestations
- svc-rewarder does not decide quorum
- svc-rewarder cannot claim fork choice
- svc-rewarder cannot claim finality
- svc-rewarder cannot create validator rewards from raw engagement
- svc-rewarder cannot mutate ledger truth
- svc-wallet commits approved payout intents
- ron-ledger remains durable economic truth
```

## New test coverage added

The new test suite verifies:

```text
docs_name_phase2_round2_committee_readiness_boundary
rewarder_wire_edges_reject_committee_attestation_poison_fields
rewarder_preserves_planning_and_wallet_handoff_seams_without_committee_authority
rewarder_source_does_not_implement_committee_or_validator_economy_runtime
serialized_rewarder_handoff_dtos_have_no_committee_authority_keys
```

Important coverage details:

```text
- WalletIssueRequest continues to reject unknown committee/quorum/finality/validator fields.
- Rewarder manifest Attestation continues to reject unknown committee/quorum/finality/validator fields.
- Rewarder output and wallet handoff seams remain explicit planning/egress DTO seams.
- Source scans prevent committee authority fields from creeping into svc-rewarder runtime code.
- Serialized wallet issue handoff DTOs expose no committee-authority keys.
```

## Commands proven green

The following focused command passed:

```bash
cargo test -p svc-rewarder --test quickchain_phase2_committee_boundary
```

Result:

```text
5 passed; 0 failed
```

The crate-local parking gate also passed:

```bash
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The park gate discovered and ran 13 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The park gate also ran:

```text
svc-rewarder all-targets test
svc-rewarder clippy
forbidden-scope marker check
```

All passed.

## Final forbidden-scope marker

The park gate confirmed:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-rewarder remains deterministic payout planning only; svc-wallet remains mutation front-door; ron-ledger remains truth
```

## Phase 2 Round 2 verdict

`svc-rewarder` is complete for this Phase 2 Round 2 pass.

The crate now has explicit committee-readiness boundary coverage proving that future committee/verifier artifacts may use rewarder outputs as deterministic replay/planning inputs, but `svc-rewarder` itself remains non-authoritative over:

```text
- committee membership
- signed verification attestations
- quorum
- fork choice
- finality
- validator rewards
- raw engagement protocol payouts
- ledger mutation
- roots/checkpoints
- external settlement
```

## Next time this crate is touched

Do not add committee runtime here.

Future safe work for later phases may include:

```text
- making rewarder artifacts easier for independent verifiers to consume
- stronger deterministic artifact shape checks
- tighter replay package compatibility checks
- additional anti-smuggling tests around payout policy and accounting snapshots
```

Unsafe or out-of-scope work remains forbidden:

```text
- no direct ledger mutation
- no direct wallet mutation outside explicit wallet handoff path
- no root/checkpoint production
- no quorum certificates
- no validator signatures
- no fork-choice/finality authority
- no staking/slashing/bonded economics
- no bridge/external settlement
- no protocol ROC payouts from raw engagement
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-storage — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-storage` is **parked for QuickChain Phase 2 Round 2**.

This round added the Phase 2 Round 2 **committee-readiness boundary** for `svc-storage` without implementing committee runtime, validator runtime, quorum logic, fork choice, finality, roots, checkpoints, bridge, staking, slashing, liquidity, or external settlement.

The crate remains exactly where it belongs in the RustyOnions/QuickChain value loop:

```text
svc-storage
→ stores bytes/artifacts by canonical b3
→ supports paid storage admission and metering
→ may retain replay/committee-readiness artifact bytes
→ does not decide verifier agreement or paid unlock truth
→ wallet/ledger receipts remain backend truth
```

`svc-storage` remains byte/artifact infrastructure only. It is not payment truth, balance truth, committee truth, finality truth, or validator authority.

## What changed in this round

Added the new focused Phase 2 Round 2 committee-boundary test:

```text
crates/svc-storage/tests/quickchain_phase2_committee_boundary.rs
```

Updated the QuickChain preflight documentation with a new Phase 2 Round 2 marker section:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

No `Cargo.toml` changes were required.

## Boundary locked by this patch

The new test locks the following `svc-storage` rules:

```text
- svc-storage stores committee/replay artifacts as bytes only
- storage is not a committee member
- storage does not produce signed verification attestations
- storage does not decide quorum
- storage does not claim fork choice
- storage does not claim finality
- b3 proves byte integrity, not committee agreement
- artifact cids are byte references, not verifier authority
- cache cannot unlock paid content alone
- wallet/ledger receipts remain backend truth
```

## New test coverage added

The new test suite verifies:

```text
docs_name_phase2_round2_committee_readiness_boundary
storage_can_hold_committee_readiness_artifact_bytes_without_interpreting_authority
storage_source_preserves_byte_storage_and_paid_boundary_seams
storage_source_does_not_implement_committee_or_validator_economy_authority
storage_boundary_has_no_committee_authority_keys_in_paid_or_policy_runtime_source
```

Important coverage details:

```text
- MemoryStorage can store and retrieve committee-readiness artifact bytes by canonical b3.
- The stored artifact payload may contain opaque committee-looking text, but storage treats it only as bytes.
- b3 integrity is checked as byte truth only, not committee agreement.
- Storage source seams preserve canonical b3, get_full, get_range, wallet settlement adapter, and accounting export boundaries.
- Source scans prevent committee/quorum/finality/validator-economy authority markers from entering storage runtime code.
- Paid, policy, and accounting source paths do not accept committee-authority keys.
```

## Commands proven green

The following focused command passed:

```bash
cargo test -p svc-storage --test quickchain_phase2_committee_boundary
```

Result:

```text
5 passed; 0 failed
```

The crate-local parking gate also passed:

```bash
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The park gate discovered and ran 16 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The park gate also ran:

```text
svc-storage all-targets test
svc-storage clippy
forbidden-scope marker check
```

All passed.

## Final forbidden-scope marker

The park gate confirmed:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-storage remains bytes by canonical b3 only; cache is not paid-access authority; wallet/ledger truth stays backend-owned
```

## Phase 2 Round 2 verdict

`svc-storage` is complete for this Phase 2 Round 2 pass.

The crate now has explicit committee-readiness boundary coverage proving that it may store replay/committee-readiness artifacts as canonical b3-addressed bytes, but it must never become authoritative over:

```text
- committee membership
- signed verification attestations
- quorum
- fork choice
- finality
- paid unlock
- wallet receipts
- ledger receipts
- validator rewards
- roots/checkpoints
- external settlement
```

## Next time this crate is touched

Do not add committee runtime here.

Future safe work for later phases may include:

```text
- storing verifier/replay artifacts as opaque content-addressed bytes
- improving artifact metadata readability without granting authority
- stronger b3/range/integrity checks around replay artifact storage
- additional paid-cache regression tests
- clearer archive/retention policy once DA/challenge/archive fallback is explicitly in scope
```

Unsafe or out-of-scope work remains forbidden:

```text
- no direct wallet mutation
- no direct ledger mutation
- no cache-only paid unlock
- no root/checkpoint production
- no quorum certificates
- no validator signatures
- no fork-choice/finality authority
- no staking/slashing/bonded economics
- no bridge/external settlement
- no pruning before DA/challenge/archive fallback is proven
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2


### END NOTE - JUNE 24 2026 - 01:00 CST


### BEGIN NOTE - JUNE 24 2026 - 12:55 CST

Here are paste-ready crate notes for `svc-rewarder + svc-storage`. The terminal output confirms both crate-local exhaustive preflight gates passed: `svc-rewarder` with 14 focused QuickChain tests and `svc-storage` with 17 focused QuickChain tests, plus all-target tests and clippy. 

## QuickChain Phase 3 Round 1 Notes — svc-rewarder

### Status

`svc-rewarder` is complete for QuickChain Phase 3 Round 1.

The crate now has an explicit Phase 3 validator/passport boundary test layer proving that `svc-rewarder` remains deterministic payout planning only. It can compute reward manifests, generate planning artifacts, and shape wallet handoff previews/intents, but it does not become validator authority, passport registry authority, wallet authority, ledger authority, staking authority, slashing authority, settlement authority, or QuickChain runtime authority.

### Phase 3 Scope Completed

Added Phase 3 validator/passport boundary coverage for:

* compute requests rejecting validator/passport authority poison fields
* nested accounting snapshots rejecting validator/passport authority poison fields
* nested reward policy DTOs rejecting validator/passport authority poison fields
* reward manifests remaining planning artifacts, not validator membership or passport authority
* reward manifests rejecting validator/passport authority poison fields
* wallet handoff preview staying wallet issue-request shape only
* source tree avoiding validator admission, revocation, rotation, staking, slashing, registry proof handling, direct ledger mutation, and validator-economy runtime
* manifest avoiding `svc-passport`, `svc-registry`, `ron-auth`, and `ron-ledger` authority dependencies

### Files Touched

* `crates/svc-rewarder/tests/quickchain_phase3_validator_boundary.rs`
* `crates/svc-rewarder/docs/quickchain-preflight.md`

### Important Boundaries Preserved

`svc-rewarder` remains:

* deterministic reward planner
* payout manifest producer
* wallet handoff planner
* funding-source-aware policy consumer
* accounting snapshot consumer
* non-authoritative planning layer

`svc-rewarder` does not become:

* wallet mutation authority
* direct ledger mutation authority
* balance truth
* receipt truth
* validator identity authority
* passport registry authority
* validator capability authority
* validator-set authority
* staking/bonding/slashing authority
* QuickChain root/checkpoint/finality producer
* bridge/external settlement authority

### Phase 3 Design Decision

For Phase 3 Round 1, validator/passport material is rejected at rewarder boundaries instead of being accepted as metadata.

That means fields such as validator set hash, validator capability, passport subject, registry proof, bond requirement, bonded economics, staking power, or slash evidence cannot be smuggled into rewarder compute requests, reward policies, accounting snapshots, reward manifests, or wallet handoff preview shapes.

This keeps reward planning separate from validator membership and prevents validator identity from becoming payout execution authority.

### Test Results

Focused Phase 3 tests passed:

* `quickchain_phase3_validator_boundary` — 7 passed

Full preflight passed:

* discovered focused QuickChain tests: 14
* all 14 focused QuickChain tests passed
* all-target tests passed
* unit tests passed
* integration tests passed
* bench smoke passed
* clippy passed

Final gate:

`svc-rewarder quickchain exhaustive preflight gate passed: tests=14`

### Remaining Risks / Deferred Work

No passport, registry, auth, validator lifecycle, bond, staking, slashing, bridge, external settlement, or root-producing runtime was added.

That is intentional.

Future Phase 3 Round 2 work may harden validator lifecycle visibility or evidence handling elsewhere, but `svc-rewarder` should continue to treat validator/passport material as non-authoritative unless a later phase explicitly defines a safe planning-only artifact role.

## QuickChain Phase 3 Round 1 Notes — svc-storage

### Status

`svc-storage` is complete for QuickChain Phase 3 Round 1.

The crate now has an explicit Phase 3 validator/passport boundary test layer proving that `svc-storage` remains bytes-by-canonical-b3 infrastructure. It may store opaque validator/readiness artifact bytes by b3, but it does not interpret those bytes as validator membership, passport authority, registry truth, capability truth, paid-unlock truth, wallet truth, ledger truth, staking authority, slashing authority, settlement authority, or QuickChain runtime authority.

### Phase 3 Scope Completed

Added Phase 3 validator/passport boundary coverage for:

* opaque validator/readiness artifacts stored and retrieved by canonical b3 only
* b3 remaining byte-integrity truth only
* storage not interpreting validator/passport artifacts as authority
* usage event DTOs rejecting validator/passport authority poison fields
* accounting export DTOs rejecting validator/passport authority poison fields
* selected paid/policy/accounting/storage source paths avoiding Phase 3 authority fields
* source tree avoiding validator admission, revocation, rotation, staking, slashing, registry proof handling, cache unlock authority, wallet authority, and ledger authority
* manifest avoiding `svc-passport`, `svc-registry`, `ron-auth`, and `ron-ledger` authority dependencies

### Files Touched

* `crates/svc-storage/tests/quickchain_phase3_validator_boundary.rs`
* `crates/svc-storage/src/accounting/mod.rs`
* `crates/svc-storage/src/accounting/exporter.rs`
* `crates/svc-storage/docs/quickchain-preflight.md`

### Important Boundaries Preserved

`svc-storage` remains:

* content-addressed byte storage
* canonical b3 verifier
* range/media-serving infrastructure
* paid-storage admission participant
* accounting usage-event exporter
* non-authoritative artifact holder

`svc-storage` does not become:

* payment truth
* paid-access truth
* cache-only unlock authority
* wallet mutation authority
* direct ledger mutation authority
* balance truth
* receipt truth
* validator identity authority
* passport registry authority
* validator capability authority
* validator-set authority
* staking/bonding/slashing authority
* QuickChain root/checkpoint/finality producer
* bridge/external settlement authority

### Phase 3 Design Decision

For Phase 3 Round 1, storage may retain validator/passport-related artifact bytes only as opaque b3-addressed objects.

That means:

* b3 proves the bytes are the bytes
* b3 does not prove validator membership
* b3 does not prove passport admission
* b3 does not prove registry truth
* b3 does not prove payment
* b3 does not unlock paid content
* b3 does not create settlement finality

The patch also hardened storage accounting DTOs with unknown-field rejection so validator/passport authority cannot be smuggled into usage events or accounting export requests.

### Test Results

Focused Phase 3 tests passed:

* `quickchain_phase3_validator_boundary` — 5 passed

Full preflight passed:

* discovered focused QuickChain tests: 17
* all 17 focused QuickChain tests passed
* all-target tests passed
* paid-write tests passed
* wallet receipt verifier tests passed
* b3/range/media tests passed
* web3 paid storage loop passed
* clippy passed

Final gate:

`svc-storage quickchain exhaustive preflight gate passed: tests=17`

### Remaining Risks / Deferred Work

No passport, registry, auth, validator lifecycle, bond, staking, slashing, bridge, external settlement, root-producing runtime, or cache-only paid unlock behavior was added.

That is intentional.

Future work may route or display validator/readiness artifacts by b3, but storage must continue to treat them as opaque bytes. Interpretation, authorization, validator lifecycle, and payment truth must remain outside storage.


### END NOTE - JUNE 24 2026 - 12:55 CST


### BEGIN NOTE - JUNE 24 2026 - 20:10 CST

Here are the carry-forward crate notes for **`svc-rewarder + svc-storage`**. The terminal output confirms both lifecycle tests, both Round 1 validator tests, all-target tests, clippy, and exhaustive QuickChain gates are green: `svc-rewarder ... tests=15` and `svc-storage ... tests=18`.  This matches the Phase 3 Round 2 lifecycle-hardening scope: rotation/revocation/evidence/downtime/parameter updates, while still forbidding staking, slashing, liquidity, and public bridge behavior. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-rewarder + svc-storage

# QuickChain Phase 3 Round 2 Crate Notes — `svc-rewarder + svc-storage`

## 0. Status

Crate pair:

```text
svc-rewarder + svc-storage
```

Phase / round:

```text
QuickChain Phase 3 — passport-gated validator set
Round 2 — validator lifecycle / evidence / operation hardening
```

Status:

```text
COMPLETE / PARKED for QuickChain Phase 3
```

Because Phase 3 has only two rounds, and this pair now has both Round 1 validator/passport boundary coverage and Round 2 validator lifecycle coverage, the safe status label is:

```text
QuickChain Phase 3 — svc-rewarder + svc-storage: 100% COMPLETE / PARKED
```

Important precision:

```text
svc-rewarder + svc-storage are complete for QuickChain Phase 3.
They are not Phase 4 staking/slashing/bonding complete.
They should not be patched again for Phase 3 unless a regression appears.
```

Current Phase 3 progress:

```text
1. ron-proto + ron-ledger        ✅ Phase 3 complete / parked
2. svc-wallet + ron-accounting   ✅ Phase 3 complete / parked
3. svc-rewarder + svc-storage    ✅ Phase 3 complete / parked
4. svc-gateway + omnigate        ← next active pair
5. svc-index + ron-policy        pending
6. CrabLink Tauri + adapters     pending
```

---

## 1. Why this pair mattered

`svc-rewarder` is payout planning infrastructure.

`svc-storage` is bytes/artifacts/paid-storage infrastructure.

Phase 3 Round 2 introduced validator lifecycle hardening upstream, including:

```text
validator rotation
validator revocation
equivocation evidence
double-attestation evidence
split-brain evidence
replay challenge evidence
invalid-attestation evidence
downtime/degraded status
governance-gated parameter updates
```

This pair’s job was to prove that lifecycle/evidence metadata can pass through the downstream system only as safe data or opaque bytes, and cannot become:

```text
payout authority
wallet mutation authority
ledger mutation authority
paid unlock authority
validator runtime authority
staking authority
slashing authority
bridge authority
external settlement authority
```

The patch intentionally did not add runtime validator logic.

No staking, slashing, bonding, bridge, public settlement, external settlement, liquidity, ROX, Solana, or public-chain behavior was added.

---

## 2. Files added or changed

New `svc-rewarder` test:

```text
crates/svc-rewarder/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

New `svc-storage` test:

```text
crates/svc-storage/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

Follow-up test-only fix:

```text
crates/svc-rewarder/tests/quickchain_phase3_validator_lifecycle_boundary.rs
crates/svc-storage/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

The follow-up fix did two things:

```text
removed an unused RewardFundingSource import from the svc-rewarder test
fixed svc-storage test-only lifetime handling for DTOs with &'static str fields
```

No production runtime files were changed.

No `Cargo.toml` dependencies were changed.

No Python helper scripts were added.

No validator runtime logic was added.

No wallet, ledger, bridge, staking, slashing, or settlement authority was added.

---

## 3. `svc-rewarder` Phase 3 Round 2 coverage

The new `svc-rewarder` lifecycle-boundary test proves:

```text
ComputeEpochRequest rejects validator lifecycle authority fields
AccountingSnapshot rejects validator lifecycle authority fields
AccountingSnapshot contribution rows reject validator lifecycle authority fields
RewardPolicy rejects validator lifecycle authority fields
RewardManifest rejects validator lifecycle authority fields
WalletIssueRequest handoff rejects validator lifecycle authority fields
Rewarder outputs remain planning artifacts only
Rewarder wallet preview remains wallet issue-request shape only
Rewarder source does not construct validator lifecycle runtime authority
Rewarder source does not construct staking/slashing/bridge/external-settlement authority
```

The lifecycle poison-field matrix included:

```text
validator_rotation
validator_rotation_epoch
validator_rotation_decision
validator_revocation
validator_revocation_reason
validator_revocation_decision
validator_lifecycle_decision
validator_lifecycle_status
validator_lifecycle_rejection_code
equivocation_evidence
double_attestation_evidence
split_brain_evidence
replay_challenge_evidence
invalid_attestation_evidence
validator_downtime_status
validator_degraded_status
downtime_report
governance_parameter_update
validator_set_parameter_update
quorum_parameter_update
checkpoint_parameter_update
slash_evidence
slashing
staking_power
validator_bond
bonded_economics
validator_reward
bridge_settlement
external_settlement
```

The output-boundary checks also confirmed rewarder artifacts do not expose:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payout_executed
paid_unlock
settlement_finality
finalized
anchored
```

`svc-rewarder` remains:

```text
deterministic payout planning only
```

It does not become:

```text
wallet
ledger
validator runtime
settlement authority
staking/slashing authority
bridge authority
```

---

## 4. `svc-rewarder` final test status

Focused Round 2 lifecycle test passed:

```bash
cargo test -p svc-rewarder --test quickchain_phase3_validator_lifecycle_boundary
```

Result:

```text
4 passed; 0 failed
```

Covered tests:

```text
rewarder_compute_request_rejects_validator_lifecycle_authority_fields
rewarder_input_dtos_reject_validator_lifecycle_authority_fields
rewarder_outputs_remain_planning_artifacts_not_lifecycle_or_payout_execution_authority
rewarder_source_does_not_construct_validator_lifecycle_runtime_or_economic_authority
```

Existing Phase 3 Round 1 validator-boundary test passed:

```bash
cargo test -p svc-rewarder --test quickchain_phase3_validator_boundary
```

Result:

```text
7 passed; 0 failed
```

Covered tests:

```text
rewarder_wallet_handoff_preview_has_no_phase3_validator_authority_keys
rewarder_manifest_keeps_passport_registry_and_auth_crates_out_of_planning_path
reward_manifest_remains_planning_artifact_not_validator_membership_or_passport_authority
rewarder_nested_snapshot_and_policy_reject_phase3_validator_authority_fields
rewarder_compute_request_rejects_phase3_validator_passport_authority_fields
reward_manifest_rejects_phase3_validator_passport_authority_fields
rewarder_source_does_not_implement_phase3_validator_or_passport_authority
```

Final exhaustive gate:

```bash
bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=15 ==
```

Additional green gates:

```text
all-targets test passed
clippy -D warnings passed
no checked-in Python helpers under svc-rewarder
format check passed
dynamic QuickChain test discovery found 15 tests
forbidden-scope marker preserved
```

Therefore:

```text
svc-rewarder is complete / parked for QuickChain Phase 3.
```

---

## 5. `svc-storage` Phase 3 Round 2 coverage

The new `svc-storage` lifecycle-boundary test proves:

```text
AccountingExportRequest rejects validator lifecycle authority fields
UsageEventDto rejects nested validator lifecycle authority fields
storage usage events remain metering only
storage accounting export remains derivative metering only
lifecycle evidence bytes can be stored by canonical b3
lifecycle evidence bytes can be read by exact b3
lifecycle evidence bytes can be range-read safely
stored lifecycle evidence does not unlock paid content
stored lifecycle evidence does not become validator authority
storage source does not construct lifecycle/cache/economic authority
```

The lifecycle poison-field matrix included:

```text
validator_rotation
validator_rotation_epoch
validator_rotation_decision
validator_revocation
validator_revocation_reason
validator_revocation_decision
validator_lifecycle_decision
validator_lifecycle_status
validator_lifecycle_rejection_code
equivocation_evidence
double_attestation_evidence
split_brain_evidence
replay_challenge_evidence
invalid_attestation_evidence
validator_downtime_status
validator_degraded_status
downtime_report
governance_parameter_update
validator_set_parameter_update
quorum_parameter_update
checkpoint_parameter_update
slash_evidence
slashing
staking_power
validator_bond
bonded_economics
validator_reward
bridge_settlement
external_settlement
```

The usage/accounting export checks also confirmed storage usage events do not expose:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payout_executed
paid_unlock
settlement_finality
finalized
anchored
cache_only_unlock
```

`svc-storage` can store validator lifecycle evidence only as:

```text
opaque bytes
canonical b3-addressed artifacts
retrievable byte objects
range-readable byte objects
```

It cannot interpret those artifacts as:

```text
paid unlock authority
wallet truth
ledger truth
validator truth
settlement truth
staking/slashing truth
bridge truth
```

---

## 6. `svc-storage` final test status

Focused Round 2 lifecycle test passed:

```bash
cargo test -p svc-storage --test quickchain_phase3_validator_lifecycle_boundary
```

Result:

```text
4 passed; 0 failed
```

Covered tests:

```text
storage_accounting_export_rejects_validator_lifecycle_authority_fields
storage_usage_events_remain_metering_not_lifecycle_or_paid_unlock_authority
lifecycle_evidence_bytes_store_by_b3_without_unlock_or_validator_authority
storage_source_does_not_construct_validator_lifecycle_or_cache_unlock_authority
```

Existing Phase 3 Round 1 validator-boundary test passed:

```bash
cargo test -p svc-storage --test quickchain_phase3_validator_boundary
```

Result:

```text
5 passed; 0 failed
```

Covered tests:

```text
storage_accounting_usage_dtos_reject_phase3_validator_authority_fields
storage_manifest_keeps_passport_registry_auth_wallet_and_ledger_authority_out_of_runtime_deps
storage_can_retain_phase3_validator_readiness_artifacts_as_opaque_b3_bytes_only
storage_paid_policy_and_accounting_sources_have_no_phase3_authority_keys
storage_source_does_not_implement_phase3_validator_or_passport_authority
```

Final exhaustive gate:

```bash
bash crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=18 ==
```

Additional green gates:

```text
all-targets test passed
clippy -D warnings passed
no checked-in Python helpers under svc-storage
format check passed
dynamic QuickChain test discovery found 18 tests
forbidden-scope marker preserved
```

Therefore:

```text
svc-storage is complete / parked for QuickChain Phase 3.
```

---

## 7. Test-only failure and fix

Initial `svc-storage` failure:

```text
error[E0597]: `top_level` does not live long enough
error[E0597]: `nested` does not live long enough
```

Cause:

```text
AccountingExportRequest and UsageEventDto contain &'static str fields.
The test dynamically built JSON Strings and tried to deserialize them into DTOs requiring static string fields.
```

Resolution:

```text
Converted those dynamic test JSON strings into leaked static test fixtures using Box::leak(...into_boxed_str()).
```

This was test-only.

It did not weaken production behavior.

It did not change runtime code.

It did not change DTO shape.

It preserved the intended lifecycle poison-field rejection coverage.

`svc-rewarder` also had a clippy-blocking unused import:

```text
RewardFundingSource
```

Resolution:

```text
Removed unused RewardFundingSource import from the new lifecycle test.
```

---

## 8. What remains forbidden after this pair

This pair did not add and must not be treated as adding:

```text
staking
slashing
bonding
validator rewards as live economics
validator payout execution
public validator economy
public chain runtime
public bridge
ROX
Solana
external settlement
liquidity
exchange-facing logic
rewarder ledger mutation
storage ledger mutation
storage paid-unlock truth
storage cache paid-unlock truth
storage validator authority
raw engagement direct ROC allocation
gateway/omnigate wallet mutation
accounting as balance truth
fake balances
fake receipts
silent spend
placeholder roots
fake finality
```

`svc-rewarder` remains:

```text
deterministic payout planning only
```

`svc-storage` remains:

```text
bytes/artifacts by canonical b3
paid storage admission/enforcement only
usage metering/export only
not payment truth
not validator truth
```

---

## 9. Boundary state after completion

The architecture remains intact:

```text
ron-proto = DTOs / strict shapes
ron-ledger = economic truth
svc-wallet = mutation front-door
ron-accounting = snapshots / metering / read-model artifacts
svc-rewarder = payout planning only
svc-storage = bytes/artifacts/paid-write enforcement only
svc-gateway = public boundary / enforcement
omnigate = hydration / product coordinator
svc-index = lookup / pointer service
ron-policy = declarative policy
CrabLink Tauri = display / user intent only
```

Validator lifecycle evidence now remains safely contained downstream as:

```text
strict rejected poison fields in reward planning DTOs
strict rejected poison fields in storage accounting export DTOs
opaque b3-addressed bytes in storage
non-authoritative metering labels
non-authoritative artifacts
```

It cannot become:

```text
wallet spend authority
wallet receipt authority
balance truth
accounting truth
rewarder payout execution
storage paid unlock authority
validator membership authority
validator lifecycle authority
staking authority
slashing authority
bridge authority
external settlement authority
```

---

## 10. Retest commands for this completed pair

Only rerun if these crates are modified again.

Focused retests:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-rewarder --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p svc-rewarder --test quickchain_phase3_validator_boundary

cargo test -p svc-storage --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p svc-storage --test quickchain_phase3_validator_boundary
```

Crate-local exhaustive gates:

```bash
bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
bash crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=15 ==
== svc-storage quickchain exhaustive preflight gate passed: tests=18 ==
```

---

## 11. Completion judgment

Final judgment:

```text
QuickChain Phase 3 — svc-rewarder + svc-storage: 100% COMPLETE / PARKED
```

This pair now has:

```text
Phase 3 Round 1 validator/passport boundary coverage
Phase 3 Round 2 validator lifecycle hardening coverage
full crate-local QuickChain preflight green
all-target tests green
clippy -D warnings green
no Python helper drift
no runtime authority creep
no wallet/ledger mutation creep
no staking/slashing/bridge/external settlement creep
```

Next active pair:

```text
svc-gateway + omnigate
```

Recommended next codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-rewarder + svc-storage


### END NOTE - JUNE 24 2026 - 20:10 CST