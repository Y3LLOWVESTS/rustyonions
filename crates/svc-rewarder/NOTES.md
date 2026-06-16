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